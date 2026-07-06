// Demo binary: index-based loops over generated matrices are clearer here.
#![allow(clippy::needless_range_loop)]

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use rand::Rng;
use std::time::Instant;

fn generate_dense_qubo(n: usize, density: f64) -> QuboModel {
    let mut rng = rand::thread_rng();
    let mut linear = vec![0.0; n];
    let mut quadratic = vec![];

    for i in 0..n {
        linear[i] = rng.gen_range(-10.0..10.0);
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < density {
                let weight = rng.gen_range(-10.0..10.0);
                quadratic.push((i, j, weight));
            }
        }
    }

    let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; n];
    for (u, v, w) in quadratic {
        row_edges[u].push((v, w));
        row_edges[v].push((u, w));
    }

    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];

    for edges in row_edges.iter_mut() {
        edges.sort_by_key(|&(v, _)| v);
        for &(v, w) in edges.iter() {
            col_indices.push(v);
            values.push(w);
        }
        row_offsets.push(col_indices.len());
    }

    QuboModel {
        energy_offset: 0.0,
        num_vars: n,
        linear,
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

fn calculate_energy(model: &QuboModel, state: &[i8]) -> f64 {
    let mut e = 0.0;
    for i in 0..model.num_vars {
        if state[i] == 1 {
            e += model.linear[i];
            for (j, w) in model.quadratic.get_row(i) {
                if state[j] == 1 {
                    e += w * 0.5; // avoid double counting
                }
            }
        }
    }
    e
}

fn main() {
    println!("🔥 ISING ENGINE: ENTERPRISE STRESS TEST SUITE 🔥");
    println!("============================================================");

    // We will test various sizes: from MEV scale (N=50) to Logistics scale (N=1000)
    let tests = vec![
        ("MEV Arbitrage Matrix", 50, 0.8, 100, 50),
        ("Standard Portfolio", 200, 0.5, 500, 100),
        ("Complex Logistics VRP", 500, 0.2, 1000, 200),
        ("Enterprise Gurobi-Killer", 1000, 0.1, 2000, 400),
    ];

    for (name, n, density, sweeps, exchanges) in tests {
        println!(
            "\n▶️ TEST: {} (Variables: {}, Density: {:.0}%)",
            name,
            n,
            density * 100.0
        );
        let model = generate_dense_qubo(n, density);

        let solver = UltimateSolver::new(
            1000.0,
            0.01,
            sweeps,
            exchanges,
            Some(42), // Fixed seed for reproducibility in tests
        );

        let start = Instant::now();
        let state = solver.solve(&model, &[]);
        let duration = start.elapsed();

        let final_energy = calculate_energy(&model, &state);

        println!("   ⏱️ Execution Time: {:?}", duration);
        println!("   ⚡ Ground State Energy: {:.2}", final_energy);

        // Calculate throughput
        let total_flips = (n as u128) * (sweeps as u128) * (exchanges as u128) * 64; // 64 replicas in MSC
        let flips_per_sec = (total_flips as f64) / duration.as_secs_f64();
        println!(
            "   🚀 Throughput: {:.2} Million Flips/sec",
            flips_per_sec / 1_000_000.0
        );
    }
    println!("\n============================================================");
    println!("✅ STRESS TEST COMPLETE.");
}
