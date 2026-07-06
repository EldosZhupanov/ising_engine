//! Correctness contract for the steepest-descent 1-opt finisher.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::local_search::steepest_descent_1opt;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[allow(clippy::needless_range_loop)]
fn random_qubo(n: usize, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            upper[i][j] = rng.gen_range(-2.0..2.0);
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
        num_vars: n,
        linear: (0..n).map(|_| rng.gen_range(-2.0..2.0)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
        energy_offset: 0.0,
    }
}

/// After the finisher, no single free-variable flip can lower the energy
/// (1-opt local minimum), and the energy never increased.
#[test]
fn descent_reaches_1opt_local_minimum() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    let n = 20;
    for _ in 0..50 {
        let model = random_qubo(n, &mut rng);
        let mut state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let e0 = model.calculate_total_energy(&state);
        let no_clamps = vec![false; n];
        let improvement = steepest_descent_1opt(&model, &mut state, &no_clamps);
        let e1 = model.calculate_total_energy(&state);

        assert!(improvement >= -1e-9, "improvement must be non-negative");
        assert!(
            (e0 - e1 - improvement).abs() < 1e-6,
            "reported improvement {} != actual {}",
            improvement,
            e0 - e1
        );
        // No improving single flip remains.
        for v in 0..n {
            let mut flipped = state.clone();
            flipped[v] = 1 - flipped[v];
            assert!(
                model.calculate_total_energy(&flipped) >= e1 - 1e-9,
                "flipping var {} still improves — not a 1-opt minimum",
                v
            );
        }
    }
}

/// The finisher must never move a clamped variable.
#[test]
fn descent_respects_clamps() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    let n = 16;
    let model = random_qubo(n, &mut rng);
    let mut state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
    let mut is_clamped = vec![false; n];
    is_clamped[3] = true;
    is_clamped[10] = true;
    let (c3, c10) = (state[3], state[10]);
    steepest_descent_1opt(&model, &mut state, &is_clamped);
    assert_eq!(state[3], c3, "clamped var 3 moved");
    assert_eq!(state[10], c10, "clamped var 10 moved");
}

/// Incremental gain bookkeeping must agree with from-scratch recomputation:
/// run the descent, then verify all gains would recompute to ≥ 0 for free
/// vars (already covered by the 1-opt check) — here we cross-check that a
/// single descent is idempotent (re-running changes nothing).
#[test]
fn descent_is_idempotent_at_local_minimum() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let n = 18;
    let model = random_qubo(n, &mut rng);
    let mut state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
    let no_clamps = vec![false; n];
    steepest_descent_1opt(&model, &mut state, &no_clamps);
    let second = steepest_descent_1opt(&model, &mut state, &no_clamps);
    assert!(
        second.abs() < 1e-9,
        "already at a minimum: second pass gained {}",
        second
    );
}
