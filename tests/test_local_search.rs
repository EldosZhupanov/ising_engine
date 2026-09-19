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

/// Verifies that steepest_descent_2opt_escapes never worsens energy,
/// and produces a true (1+2)-opt local optimum across all single flips
/// and all pairwise flips.
#[test]
fn test_2opt_edge_escape_never_worsens_and_reaches_2opt() {
    use ising_engine::solver::local_search::steepest_descent_2opt_escapes;
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let n = 16;
    for _ in 0..20 {
        let model = random_qubo(n, &mut rng);
        let mut state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let e0 = model.calculate_total_energy(&state);
        let no_clamps = vec![false; n];
        let improvement = steepest_descent_2opt_escapes(&model, &mut state, &no_clamps);
        let e1 = model.calculate_total_energy(&state);

        assert!(improvement >= -1e-9, "improvement must be non-negative");
        assert!(
            e1 <= e0 + 1e-9,
            "energy must never increase: e1 {} > e0 {}",
            e1,
            e0
        );
        assert!(
            (e0 - e1 - improvement).abs() < 1e-6,
            "reported improvement {} != actual {}",
            improvement,
            e0 - e1
        );

        // Verify 1-opt local optimality: no single flip can improve
        for i in 0..n {
            let mut flipped = state.clone();
            flipped[i] = 1 - flipped[i];
            let ef = model.calculate_total_energy(&flipped);
            assert!(
                ef >= e1 - 1e-9,
                "single flip on var {} improves energy from {} to {}",
                i,
                e1,
                ef
            );
        }

        // Verify 2-opt local optimality: no pair flip anywhere in the graph can improve
        for i in 0..n {
            for j in (i + 1)..n {
                let mut flipped = state.clone();
                flipped[i] = 1 - flipped[i];
                flipped[j] = 1 - flipped[j];
                let ef = model.calculate_total_energy(&flipped);
                assert!(
                    ef >= e1 - 1e-9,
                    "pair flip on ({}, {}) improves energy from {} to {}",
                    i,
                    j,
                    e1,
                    ef
                );
            }
        }
    }
}

/// Verifies that 2-opt escapes strictly respect clamped variables.
#[test]
fn test_2opt_respects_clamps() {
    use ising_engine::solver::local_search::steepest_descent_2opt_escapes;
    let mut rng = ChaCha8Rng::seed_from_u64(99);
    let n = 16;
    let model = random_qubo(n, &mut rng);
    let mut state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
    let mut is_clamped = vec![false; n];
    is_clamped[2] = true;
    is_clamped[7] = true;
    let (c2, c7) = (state[2], state[7]);
    steepest_descent_2opt_escapes(&model, &mut state, &is_clamped);
    assert_eq!(state[2], c2, "clamped var 2 moved");
    assert_eq!(state[7], c7, "clamped var 7 moved");
}

/// Verifies Theorem 1 directly: at a 1-opt local minimum, any improving 2-flip
/// across all N(N-1)/2 pairs MUST be an edge in the graph. Non-edges can NEVER improve.
#[test]
fn test_theorem1_non_edges_never_improve_at_1opt() {
    let mut rng = ChaCha8Rng::seed_from_u64(12345);
    let n = 20;

    for _ in 0..30 {
        // Create a sparse QUBO with edge probability ~ 0.3
        let mut upper = vec![vec![0.0f64; n]; n];
        let mut has_edge = vec![vec![false; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                if rng.gen_bool(0.3) {
                    let w = rng.gen_range(-3.0..3.0);
                    upper[i][j] = w;
                    has_edge[i][j] = true;
                    has_edge[j][i] = true;
                }
            }
        }
        let mut values = Vec::new();
        let mut col_indices = Vec::new();
        let mut row_offsets = vec![0];
        for i in 0..n {
            for j in 0..n {
                if i != j && has_edge[i][j] {
                    let w = if i < j { upper[i][j] } else { upper[j][i] };
                    col_indices.push(j);
                    values.push(w);
                }
            }
            row_offsets.push(col_indices.len());
        }
        let model = QuboModel {
            num_vars: n,
            linear: (0..n).map(|_| rng.gen_range(-2.0..2.0)).collect(),
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
            energy_offset: 0.0,
        };

        let mut state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let no_clamps = vec![false; n];
        steepest_descent_1opt(&model, &mut state, &no_clamps);
        let e_1opt = model.calculate_total_energy(&state);

        // Brute-force scan all pairs in the entire graph
        for i in 0..n {
            for j in (i + 1)..n {
                let mut flipped = state.clone();
                flipped[i] = 1 - flipped[i];
                flipped[j] = 1 - flipped[j];
                let ef = model.calculate_total_energy(&flipped);
                let delta = ef - e_1opt;
                if !has_edge[i][j] {
                    // Non-edge: MUST NOT IMPROVE (Theorem 1)
                    assert!(
                        delta >= -1e-9,
                        "Violation of Theorem 1: non-edge ({}, {}) improved energy by {}",
                        i,
                        j,
                        delta
                    );
                }
            }
        }
    }
}
