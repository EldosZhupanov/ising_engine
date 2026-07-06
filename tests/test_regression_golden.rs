//! Fixed-seed regression goldens + determinism contract (AGENTS.md §6.3).
//!
//! The engine is deterministic by construction: a seeded ChaCha8 master RNG
//! drives per-cell counter-derived Xoshiro streams, so rayon scheduling cannot
//! affect results. These tests make that property enforceable:
//!
//! 1. Determinism: identical seed ⇒ bit-identical state, with and without clamps.
//! 2. Golden: a pinned (instance, seed) pair must reproduce the recorded output
//!    exactly. Any future refactor that changes this must update the golden
//!    EXPLICITLY and justify the behavioral change in review.
//!
//! Platform note: the golden was recorded on x86_64 with the repo's
//! `-Ctarget-cpu=native +avx2,+fma` flags. Float summation order is fixed by
//! codegen for a given target; on a different reference target the golden may
//! need re-recording (state asserted exactly, energy to 1e-9).

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Deterministic mixed-sign dense QUBO, n = 32, weights in [-1, 1].
// Two-sided matrix indexing (upper[i][j] vs upper[j][i]) is clearer as written.
#[allow(clippy::needless_range_loop)]
fn reference_instance() -> QuboModel {
    let n = 32;
    let mut rng = ChaCha8Rng::seed_from_u64(0xC0FFEE);
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            upper[i][j] = rng.gen_range(-1.0..1.0);
        }
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let w = if i < j { upper[i][j] } else { upper[j][i] };
                col_indices.push(j);
                values.push(w);
            }
        }
        row_offsets.push(col_indices.len());
    }
    QuboModel {
        energy_offset: 0.0,
        num_vars: n,
        linear: (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

fn reference_solver() -> UltimateSolver {
    UltimateSolver::new(50.0, 0.05, 30, 40, Some(2024))
}

#[test]
fn solver_is_deterministic_given_seed() {
    let model = reference_instance();
    let s1 = reference_solver().solve(&model, &[]);
    let s2 = reference_solver().solve(&model, &[]);
    assert_eq!(s1, s2, "identical seeds must produce identical states");
}

#[test]
fn solver_is_deterministic_given_seed_with_clamps() {
    let model = reference_instance();
    let clamps = vec![(0, 1i8), (7, 0i8), (31, 1i8)];
    let s1 = reference_solver().solve(&model, &clamps);
    let s2 = reference_solver().solve(&model, &clamps);
    assert_eq!(
        s1, s2,
        "identical seeds + clamps must produce identical states"
    );
    for &(idx, val) in &clamps {
        assert_eq!(s1[idx], val, "clamp on x{} not honored", idx);
    }
}

#[test]
fn fixed_seed_golden_regression() {
    let model = reference_instance();
    let state = reference_solver().solve(&model, &[]);
    let energy = model.calculate_total_energy(&state);

    // Recorded after the correctness fixes (basis, Trotter, clamps), 2026-07-05.
    const GOLDEN_STATE: [i8; 32] = [
        1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 0,
        1, 1,
    ];
    const GOLDEN_ENERGY: f64 = -2.499_621_900_069_755e1;

    assert_eq!(
        state.as_slice(),
        GOLDEN_STATE,
        "solver trajectory changed at fixed seed; energy now {:.17e}",
        energy
    );
    assert!(
        (energy - GOLDEN_ENERGY).abs() < 1e-9,
        "energy changed at fixed seed: got {:.17e}, golden {:.17e}",
        energy,
        GOLDEN_ENERGY
    );
}
