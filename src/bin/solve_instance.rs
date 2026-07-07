//! Minimal benchmark bridge: solve ONE instance file with the production
//! `UltimateSolver` and print a single JSON line. Exists so external
//! benchmark drivers (benchmark_suite/scripts/compare_openjij.py) can run
//! the engine as a black box with an explicit seed and budget — the solver
//! itself is NOT modified in any way.
//!
//! Usage:
//!   solve_instance --file F --format rudy|orlib|qplib [--problem K]
//!                  --sweeps N --exchanges N --seed S
//!
//! Output (one line):
//!   {"energy": -11624.0, "wall_ms": 812.3, "n": 800,
//!    "sweeps": 20, "exchanges": 40, "state": "0110..."}
//!
//! `state` lets the driver re-score BOTH solvers with one canonical energy
//! function, eliminating any cross-language convention mismatch.

use ising_engine::benchmark::instances::{
    parse_biqmac_sparse, parse_orlib_bqp, parse_qplib, parse_rudy_maxcut,
};
use ising_engine::solver::UltimateSolver;
use std::time::Instant;

fn arg(name: &str) -> Option<String> {
    let argv: Vec<String> = std::env::args().collect();
    argv.iter()
        .position(|a| a == name)
        .and_then(|i| argv.get(i + 1).cloned())
}

fn main() {
    let file = arg("--file").expect("--file required");
    let format = arg("--format").expect("--format required");
    let problem: usize = arg("--problem").map(|s| s.parse().unwrap()).unwrap_or(1);
    let sweeps: usize = arg("--sweeps").map(|s| s.parse().unwrap()).unwrap_or(20);
    let exchanges: usize = arg("--exchanges").map(|s| s.parse().unwrap()).unwrap_or(20);
    let seed: u64 = arg("--seed").map(|s| s.parse().unwrap()).unwrap_or(1);

    let text = std::fs::read_to_string(&file).expect("read instance file");
    let name = std::path::Path::new(&file)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let inst = match format.as_str() {
        "rudy" => parse_rudy_maxcut(&text, &name).expect("parse rudy"),
        "orlib" => {
            let mut v = parse_orlib_bqp(&text, &name).expect("parse orlib");
            assert!(problem >= 1 && problem <= v.len(), "--problem out of range");
            v.swap_remove(problem - 1)
        }
        "biqmac" => parse_biqmac_sparse(&text, &name).expect("parse biqmac sparse"),
        "qplib" => parse_qplib(&text, &name).expect("parse qplib"),
        other => panic!("unknown --format {other}"),
    };

    let start = Instant::now();
    let state =
        UltimateSolver::new(5.0, 0.02, sweeps, exchanges, Some(seed)).solve(&inst.model, &[]);
    let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
    let energy = inst.model.calculate_total_energy(&state);

    let bits: String = state
        .iter()
        .map(|&b| if b == 1 { '1' } else { '0' })
        .collect();
    println!(
        "{{\"energy\": {energy}, \"wall_ms\": {wall_ms}, \"n\": {}, \"sweeps\": {sweeps}, \"exchanges\": {exchanges}, \"state\": \"{bits}\"}}",
        inst.model.num_vars
    );
}
