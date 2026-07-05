use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use std::time::Instant;

fn main() {
    println!("🤖 ISING ENGINE: GNN HYBRID SOLVER DEMO");
    println!("============================================================");

    // We create a simple QUBO matrix (e.g. 10 variables)
    let n = 10;
    let mut linear = vec![0.0; n];
    let mut quadratic: Vec<(usize, usize, f64)> = vec![];

    // Let's assume the GNN analyzed this matrix and output a probability
    // that certain nodes MUST be 1.
    // E.g., nodes 0, 2, 4 are highly likely to be 1 (95% chance).
    // Nodes 1, 3, 5 are highly likely to be 0 (5% chance to be 1).
    let gnn_predictions = vec![
        0.95, 0.05, 0.95, 0.05, 0.95, 0.05, 0.50, 0.50, 0.50, 0.50, // Last 4 are uncertain
    ];

    // Build dummy CSR matrix
    let mut row_offsets = vec![0; n + 1];
    let model = QuboModel {
        num_vars: n,
        linear,
        quadratic: CsrMatrix {
            values: vec![],
            col_indices: vec![],
            row_offsets,
        },
    };

    println!("🧠 AI Pre-computation Complete.");
    println!("📊 GNN Probability Injection Vector:");
    for (i, p) in gnn_predictions.iter().enumerate() {
        println!("   Node {}: {:.0}% chance to be 1", i, p * 100.0);
    }

    let solver =
        UltimateSolver::new(100.0, 0.1, 1000, 100, None).with_gnn_heuristic(gnn_predictions); // <-- THE MAGIC HAPPENS HERE

    let start = Instant::now();
    let state = solver.solve(&model, &[]);
    let duration = start.elapsed();

    println!("\n⚡ Annealing finished in {:?}", duration);
    println!("🚀 Ground State (Warm Started by AI): {:?}", state);
    println!("============================================================");
}
