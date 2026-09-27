//! MQ-NATIVE-001: model preparation and fixed-state validation only.
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::engine_v2::{OperatorRegistry, ProblemIR};
use serde::Deserialize;
use serde_json::json;
use std::io::{self, Write};
use std::time::{Duration, Instant};

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
    let start = Instant::now();
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("expected MODEL DELAY_NS".into());
    }
    let m: Model = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let delay: u64 = args[2].parse()?;
    let (model, ir, _) = prepare(&m)?;
    let registry = OperatorRegistry::standard();
    std::hint::black_box(&registry);
    let n = model.num_vars;
    let probes: Vec<Vec<i8>> = vec![
        vec![0; n],
        vec![1; n],
        (0..n).map(|i| (i % 2) as i8).collect(),
        (0..n).map(|i| i8::from(i % 3 == 0)).collect(),
    ];
    let energies: Vec<f64> = probes
        .iter()
        .map(|x| model.calculate_total_energy(x))
        .collect();
    for (x, &energy) in probes.iter().zip(&energies) {
        let bits: Vec<u8> = x.iter().map(|&v| v as u8).collect();
        if ir.energy(&bits) != energy {
            return Err("model representation energy mismatch".into());
        }
    }
    let status = std::fs::read_to_string("/proc/self/status")?;
    let affinity = status
        .lines()
        .find_map(|s| s.strip_prefix("Cpus_allowed_list:"))
        .ok_or("affinity unavailable")?
        .trim();
    let ready_ns = start.elapsed().as_nanos();
    let sleep_start = Instant::now();
    std::thread::sleep(Duration::from_nanos(delay));
    let sleep_ns = sleep_start.elapsed().as_nanos();
    let state = &probes[0];
    let energy = energies[0];
    let chunk = 0;
    let mut out = io::stdout().lock();
    writeln!(
        out,
        "{}",
        json!({"kind":"inc", "state":state, "energy":energy,"chunk":chunk})
    )?;
    out.flush()?;
    writeln!(
        out,
        "{}",
        json!({"kind":"diagnostic", "energies":energies,
        "ready_ns":ready_ns,"sleep_ns":sleep_ns,"affinity":affinity})
    )?;
    out.flush()?;
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
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
