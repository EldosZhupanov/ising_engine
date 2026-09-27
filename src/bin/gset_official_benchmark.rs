// Demo binary: index-based loops over generated matrices are clearer here.
#![allow(clippy::needless_range_loop)]

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

// Historical executable name retained for compatibility. This generates a
// synthetic graph; matching G1's expected density does not make it a Gset input.
fn generate_synthetic_maxcut(n: usize, density: f64, seed: u64) -> QuboModel {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut linear = vec![0.0; n];
    let mut quadratic = vec![];

    // Max-Cut formulation: Q_ij = 2*W_ij, Q_ii = -sum(W_ij)
    for i in 0..n {
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < density {
                let weight = 1.0;
                quadratic.push((i, j, 2.0 * weight));
                // -w*(x_i + x_j - 2*x_i*x_j): both endpoints contribute.
                linear[i] -= weight;
                linear[j] -= weight;
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

fn calculate_cut(model: &QuboModel, state: &[i8]) -> f64 {
    // Convert QUBO energy back to Max-Cut value
    // Count crossing edges directly, independently of linear QUBO coefficients.
    let mut cut = 0.0;
    // Iterate through upper triangle to count edges cut
    for i in 0..model.num_vars {
        for (j, w) in model.quadratic.get_row(i) {
            if i < j && state[i] != state[j] {
                cut += w / 2.0; // reverse QUBO scaling
            }
        }
    }
    cut
}

fn main() {
    const GRAPH_SEED: u64 = 42;
    const SOLVER_SEED: u64 = 1337;
    println!("Synthetic Max-Cut demonstration (historical executable: gset_official_benchmark)");
    println!("This is not a Gset instance. No optimum or external solver timing is asserted.");
    let model = generate_synthetic_maxcut(800, 19176.0 / (800.0 * 799.0 / 2.0), GRAPH_SEED);
    println!(
        "vertices={} edges={} graph_seed={} solver_seed={}",
        model.num_vars,
        model.quadratic.values.len() / 2,
        GRAPH_SEED,
        SOLVER_SEED
    );

    let solver = UltimateSolver::new(
        1000.0,
        0.01,
        500, // Sweeps
        100, // Exchanges
        Some(SOLVER_SEED),
    );

    let start = Instant::now();
    let state = solver.solve(&model, &[]);
    let duration = start.elapsed();

    let cut_value = calculate_cut(&model, &state);

    let energy = model.calculate_total_energy(&state);
    assert_eq!(energy, -cut_value, "Max-Cut/QUBO energy mismatch");
    println!("solver_wall_seconds={:.6}", duration.as_secs_f64());
    println!("cut={cut_value} qubo_energy={energy}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_triangle_energy_equals_negative_cut_exhaustively() {
        let model = generate_synthetic_maxcut(3, 1.0, 42);
        for bits in 0u32..8 {
            let state: Vec<i8> = (0..3).map(|v| ((bits >> v) & 1) as i8).collect();
            let ones = bits.count_ones();
            let cut = (ones * (3 - ones)) as f64;
            assert_eq!(calculate_cut(&model, &state), cut);
            assert_eq!(model.calculate_total_energy(&state), -cut);
        }
    }

    #[test]
    fn seeded_sparse_graph_is_repeatable_and_energy_consistent() {
        let a = generate_synthetic_maxcut(6, 0.4, 19);
        let b = generate_synthetic_maxcut(6, 0.4, 19);
        assert_eq!(a.linear, b.linear);
        assert_eq!(a.quadratic.col_indices, b.quadratic.col_indices);
        assert_eq!(a.quadratic.row_offsets, b.quadratic.row_offsets);
        assert_eq!(a.quadratic.values, b.quadratic.values);
        for bits in 0..64 {
            let state: Vec<i8> = (0..6).map(|v| ((bits >> v) & 1) as i8).collect();
            assert_eq!(a.calculate_total_energy(&state), -calculate_cut(&a, &state));
        }
    }
}
