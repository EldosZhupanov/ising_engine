//! MQ-DIFFICULTY-001: frozen search policies plus readiness controls.
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::engine_v2::{
    boxed_state, DecisionEngine, OperatorRegistry, ProblemIR, RunContext, Runtime,
};
use ising_engine::solver::UltimateSolver;
use serde::Deserialize;
use serde_json::json;
use std::io::{self, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    offset: f64,
    linear: Vec<f64>,
    pairs: Vec<(usize, usize, f64)>,
}

fn prepare(m: &Model) -> Result<(QuboModel, ProblemIR, f64), &'static str> {
    let n = m.linear.len();
    if !(1..=4096).contains(&n) || !m.offset.is_finite() || m.linear.iter().any(|v| !v.is_finite())
    {
        return Err("invalid model");
    }
    let mut rows = vec![Vec::new(); n];
    let mut seen = std::collections::HashSet::new();
    let mut bounds: Vec<f64> = m.linear.iter().map(|v| v.abs()).collect();
    for &(i, j, w) in &m.pairs {
        if i >= j || j >= n || !w.is_finite() || !seen.insert((i, j)) {
            return Err("invalid pair");
        }
        rows[i].push((j, w));
        rows[j].push((i, w));
        bounds[i] += w.abs();
        bounds[j] += w.abs();
    }
    let mut csr = CsrMatrix::empty(n);
    csr.row_offsets.clear();
    csr.row_offsets.push(0);
    for row in &mut rows {
        row.sort_by_key(|&(j, _)| j);
        for &(j, w) in row.iter() {
            csr.col_indices.push(j);
            csr.values.push(w);
        }
        csr.row_offsets.push(csr.values.len());
    }
    let pairs: Vec<_> = m
        .pairs
        .iter()
        .map(|&(i, j, w)| (i as u32, j as u32, w))
        .collect();
    Ok((
        QuboModel {
            num_vars: n,
            linear: m.linear.clone(),
            quadratic: csr,
            energy_offset: m.offset,
        },
        ProblemIR::from_pairs(n, m.offset, m.linear.clone(), &pairs),
        bounds.into_iter().fold(1.0, f64::max),
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err("expected ARM MODEL SEED MODE".into());
    }
    let m: Model = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let seed: u64 = args[3].parse()?;
    let (model, ir, hot) = prepare(&m)?;
    let registry = OperatorRegistry::standard();
    let mut out = io::stdout().lock();
    let affinity_status = std::fs::read_to_string("/proc/self/status")?;
    let affinity = affinity_status
        .lines()
        .find_map(|s| s.strip_prefix("Cpus_allowed_list:"))
        .ok_or("affinity missing")?
        .trim();
    if affinity != "0" {
        return Err("affinity mismatch".into());
    }
    writeln!(out, "{}", json!({"kind":"ready", "affinity":affinity}))?;
    out.flush()?;
    if args[4] != "search" {
        if !["null_a", "null_b", "delay"].contains(&args[4].as_str()) {
            return Err("unknown mode".into());
        }
        let state = vec![0_i8; model.num_vars];
        let energy = model.calculate_total_energy(&state);
        let sleep_start = std::time::Instant::now();
        if args[4] == "delay" {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let sleep_ns = sleep_start.elapsed().as_nanos();
        writeln!(
            out,
            "{}",
            json!({"kind":"inc", "state":state, "energy":energy, "sleep_ns":sleep_ns})
        )?;
        out.flush()?;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    let mut best = f64::INFINITY;
    for chunk in 0_u64.. {
        let chunk_seed = seed.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(chunk);
        let (state, energy) = match args[1].as_str() {
            "ultimate" => {
                let solver = UltimateSolver::new(hot, 0.1, 8, 8, Some(chunk_seed)).with_2opt(true);
                let x = solver.solve(&model, &[]);
                let energy = model.calculate_total_energy(&x);
                (x.iter().map(|&v| v as u8).collect::<Vec<_>>(), energy)
            }
            "v2_default" => {
                let plan = DecisionEngine::default_plan(&ir, &registry, 32, 32, chunk_seed)?;
                if chunk == 0 {
                    writeln!(
                        out,
                        "{}",
                        json!({"kind":"config", "backend":format!("{:?}",plan.backend),
                            "operators":plan.steps.iter().map(|s| s.operator.as_str()).collect::<Vec<_>>(),
                            "temperatures":&plan.temperatures,"replicas":plan.num_replicas,"seed":plan.seed})
                    )?;
                    out.flush()?;
                }
                let mut state = boxed_state(&ir, plan.backend, plan.num_replicas, &vec![0; ir.n]);
                let mut runtime = Runtime::new(RunContext::new(chunk_seed), &plan);
                let record = runtime.run(&plan, state.as_mut(), &registry, &ir)?;
                (record.best_state, record.best_energy)
            }
            _ => return Err("unknown arm".into()),
        };
        if energy < best {
            best = energy;
            writeln!(
                out,
                "{}",
                json!({"kind":"inc", "state":state, "energy":energy,"chunk":chunk})
            )?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_representations_preserve_raw_polynomial_exhaustively() {
        let m = Model {
            offset: 3.25,
            linear: vec![0.5, -1.25, 0.0, 0.0],
            pairs: vec![(0, 1, 2.5), (1, 2, -0.75)],
        };
        let (q, ir, _) = prepare(&m).unwrap();
        for mask in 0..16 {
            let x: Vec<i8> = (0..4).map(|i| ((mask >> i) & 1) as i8).collect();
            let direct = m.offset
                + m.linear
                    .iter()
                    .zip(&x)
                    .map(|(h, v)| h * f64::from(*v))
                    .sum::<f64>()
                + m.pairs
                    .iter()
                    .map(|&(i, j, w)| w * f64::from(x[i]) * f64::from(x[j]))
                    .sum::<f64>();
            assert_eq!(q.calculate_total_energy(&x), direct);
            let y: Vec<u8> = x.iter().map(|&v| v as u8).collect();
            assert_eq!(ir.energy(&y), direct);
        }
    }
    #[test]
    fn rejects_duplicate_pair_and_nonfinite_offset() {
        let mut m = Model {
            offset: 0.0,
            linear: vec![0.0; 2],
            pairs: vec![(0, 1, 1.0), (0, 1, 2.0)],
        };
        assert!(prepare(&m).is_err());
        m.pairs.clear();
        m.offset = f64::NAN;
        assert!(prepare(&m).is_err());
    }
}
