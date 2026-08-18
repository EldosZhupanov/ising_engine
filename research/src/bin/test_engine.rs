#![allow(warnings)]
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::ParallelTemperingSolver;

fn main() {
    println!("🦀 QUBO Engine: Parallel Tempering Test");
    println!("=======================================");

    let and_gate_model = QuboModel {
        num_vars: 3,
        linear: vec![0.0, 0.0, 3.0],
        quadratic: CsrMatrix {
            values: vec![1.0, -2.0, 1.0, -2.0, -2.0, -2.0],
            col_indices: vec![1, 2, 0, 2, 0, 1],
            row_offsets: vec![0, 2, 4, 6],
        },
        // This AND-gate penalty is written out by hand with no constant term,
        // so 0.0 reproduces the behaviour this binary had before `energy_offset`
        // was added to `QuboModel`.
        energy_offset: 0.0,
    };

    let solver = ParallelTemperingSolver {
        num_replicas: 8,
        temp_max: 50.0,
        temp_min: 0.01,
        sweeps_per_exchange: 200,
        total_exchanges: 50,
        seed: None,
    };

    let clamped = vec![(2, 1)]; // Z = 1
    let result = solver.solve(&and_gate_model, &clamped);

    println!("\n[ Inverse Compute: Z = 1 ]");
    println!("A = {}, B = {}", result[0], result[1]);
    if result[0] == 1 && result[1] == 1 {
        println!("✅ Success!");
    }
}
