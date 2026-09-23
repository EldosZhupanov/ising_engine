//! Isolated JSONL adapter: production solver + production presolve + tiny exact oracle.
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::presolve::{
    connected_components, extract_component, fix_persistent_variables, full_presolve,
};
use ising_engine::solver::UltimateSolver;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    linear: Vec<f64>,
    pairs: Vec<(usize, usize, f64)>,
    offset: f64,
    seed: u64,
}

fn model(req: &Request) -> Result<QuboModel, String> {
    let n = req.linear.len();
    if !(1..=16).contains(&n)
        || !req.offset.is_finite()
        || req.linear.iter().any(|v| !v.is_finite())
    {
        return Err("require 1..16 variables and finite coefficients".into());
    }
    let mut rows = vec![Vec::new(); n];
    let mut seen = std::collections::HashSet::new();
    for &(i, j, w) in &req.pairs {
        if i >= j || j >= n || !w.is_finite() || !seen.insert((i, j)) {
            return Err("pairs must be unique finite upper-triangular entries".into());
        }
        if w != 0.0 {
            rows[i].push((j, w));
            rows[j].push((i, w));
        }
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
    Ok(QuboModel {
        num_vars: n,
        linear: req.linear.clone(),
        quadratic: csr,
        energy_offset: req.offset,
    })
}

fn assignment(n: usize, mask: usize) -> Vec<i8> {
    (0..n).map(|i| ((mask >> i) & 1) as i8).collect()
}

fn exact(m: &QuboModel) -> (Vec<i8>, f64) {
    let mut best = (vec![0; m.num_vars], f64::INFINITY);
    for mask in 0..(1usize << m.num_vars) {
        let x = assignment(m.num_vars, mask);
        let e = m.calculate_total_energy(&x);
        if e < best.1 {
            best = (x, e);
        }
    }
    best
}

fn reduced(m: &QuboModel) -> Value {
    let first = fix_persistent_variables(m, &[]);
    let fixed = full_presolve(m, &[]);
    let mut free = vec![true; m.num_vars];
    let mut ones = vec![false; m.num_vars];
    let mut state = vec![0; m.num_vars];
    for &(i, v) in &fixed {
        free[i] = false;
        ones[i] = v == 1;
        state[i] = v;
    }
    let components = connected_components(m, &free);
    let mut evaluations = 0usize;
    for comp in &components {
        let (sub, map) = extract_component(m, comp, &ones);
        let (x, _) = exact(&sub);
        evaluations += 1usize << comp.len();
        for (local, &global) in map.iter().enumerate() {
            state[global] = x[local];
        }
    }
    json!({"state": state, "energy": m.calculate_total_energy(&state),
        "first_order_fixed": first, "fixed": fixed, "components": components,
        "enumerated_states": evaluations})
}

fn solve(req: Request) -> Result<Value, String> {
    let m = model(&req)?;
    let (x, e) = exact(&m);
    let reduced = reduced(&m);
    if (reduced["energy"].as_f64().unwrap() - e).abs() > 1e-8 {
        return Err("presolve optimum preservation failed".into());
    }
    let solver = UltimateSolver::new(10.0, 0.05, 10, 20, Some(req.seed)).with_2opt(true);
    let y = solver.solve(&m, &[]);
    if y.len() != m.num_vars || y.iter().any(|&v| v != 0 && v != 1) {
        return Err("solver returned invalid state".into());
    }
    let spectrum: Vec<_> = (0..(1usize << m.num_vars))
        .map(|mask| m.calculate_total_energy(&assignment(m.num_vars, mask)))
        .collect();
    Ok(
        json!({"exact": {"state":x,"energy":e}, "reduced_exact":reduced,
        "ultimate":{"state":y,"energy":m.calculate_total_energy(&y)},
        "energy_spectrum":spectrum,"full_enumerated_states":1usize << m.num_vars}),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let req: Request = serde_json::from_str(&line?)?;
        let result = solve(req).map_err(io::Error::other)?;
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_csr_matches_independent_polynomial() {
        let req = Request {
            linear: vec![-2.0, 3.0, -1.0],
            pairs: vec![(0, 1, 4.0), (1, 2, -5.0)],
            offset: 7.0,
            seed: 1,
        };
        let m = model(&req).unwrap();
        for mask in 0..8 {
            let x = assignment(3, mask);
            let direct = 7.0 - 2.0 * f64::from(x[0]) + 3.0 * f64::from(x[1]) - f64::from(x[2])
                + 4.0 * f64::from(x[0] * x[1])
                - 5.0 * f64::from(x[1] * x[2]);
            assert_eq!(m.calculate_total_energy(&x), direct);
        }
    }

    #[test]
    fn rejects_bad_indices_duplicates_and_dimensions() {
        for pairs in [
            vec![(1, 0, 1.0)],
            vec![(0, 2, 1.0)],
            vec![(0, 1, 1.0), (0, 1, 2.0)],
        ] {
            assert!(model(&Request {
                linear: vec![0.0; 2],
                pairs,
                offset: 0.0,
                seed: 1
            })
            .is_err());
        }
        assert!(model(&Request {
            linear: vec![],
            pairs: vec![],
            offset: 0.0,
            seed: 1
        })
        .is_err());
    }

    #[test]
    fn presolve_and_components_preserve_exact_optimum() {
        // Deterministic mixed signs, ties, offsets and disconnected cases; no pilot inputs.
        for seed in 0..80usize {
            let req = Request {
                linear: (0..6)
                    .map(|i| ((seed * 7 + i * 3) % 9) as f64 - 4.0)
                    .collect(),
                pairs: (0..6)
                    .flat_map(|i| {
                        ((i + 1)..6).filter_map(move |j| {
                            if (seed + i + j) % 3 == 0 {
                                None
                            } else {
                                Some((i, j, ((seed * 3 + i * 5 + j) % 7) as f64 - 3.0))
                            }
                        })
                    })
                    .collect(),
                offset: 13.25,
                seed: seed as u64,
            };
            let m = model(&req).unwrap();
            let r = reduced(&m);
            assert!((r["energy"].as_f64().unwrap() - exact(&m).1).abs() < 1e-8);
        }
    }

    #[test]
    fn fixed_seed_solver_replays_and_preserves_energy_contract() {
        let make = || Request {
            linear: vec![-1.0, -2.0, -3.0],
            pairs: vec![(0, 1, 7.0), (1, 2, 7.0)],
            offset: 4.0,
            seed: 123,
        };
        let a = solve(make()).unwrap();
        let b = solve(make()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a["ultimate"]["energy"], a["exact"]["energy"]);
    }
}
