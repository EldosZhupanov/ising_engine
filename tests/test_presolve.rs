//! Correctness contract for first-order persistency presolve.
//!
//! The guarantee under test (weak persistency; Hammer-Hansen-Simeone 1984,
//! Boros-Hammer 2002): applying every derived fixing preserves at least one
//! globally optimal solution — i.e. the optimal ENERGY of the restricted
//! problem equals the optimal energy of the original (clamped) problem.
//! Verified by exhaustive enumeration on every instance.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::presolve::fix_persistent_variables;
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Exhaustive optimum over all states consistent with `fixed`.
fn brute_force_optimum(model: &QuboModel, fixed: &[(usize, i8)]) -> f64 {
    let n = model.num_vars;
    assert!(n <= 16, "exhaustive enumeration guard");
    let mut best = f64::INFINITY;
    'outer: for bits in 0..(1u32 << n) {
        let state: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        for &(idx, val) in fixed {
            if state[idx] != val {
                continue 'outer;
            }
        }
        best = best.min(model.calculate_total_energy(&state));
    }
    best
}

/// Random sparse QUBO with mixed-sign weights.
#[allow(clippy::needless_range_loop)]
fn random_sparse_qubo(n: usize, density: f64, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < density {
                upper[i][j] = rng.gen_range(-2.0..2.0);
            }
        }
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let w = if i < j { upper[i][j] } else { upper[j][i] };
                if w != 0.0 {
                    col_indices.push(j);
                    values.push(w);
                }
            }
        }
        row_offsets.push(col_indices.len());
    }
    QuboModel {
        energy_offset: 0.0,
        num_vars: n,
        linear: (0..n).map(|_| rng.gen_range(-2.0..2.0)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

#[test]
fn presolve_preserves_optimal_energy_exhaustive() {
    let mut rng = ChaCha8Rng::seed_from_u64(41);
    let mut total_fixed = 0usize;
    for _ in 0..100 {
        let model = random_sparse_qubo(10, 0.3, &mut rng);
        let derived = fix_persistent_variables(&model, &[]);
        total_fixed += derived.len();
        let e_full = brute_force_optimum(&model, &[]);
        let e_restricted = brute_force_optimum(&model, &derived);
        assert!(
            (e_full - e_restricted).abs() < 1e-9,
            "presolve lost the optimum: full {} vs restricted {} (fixed {:?})",
            e_full,
            e_restricted,
            derived
        );
    }
    // The test must actually exercise the rules: sparse mixed-sign instances
    // at n=10 fix many variables in aggregate.
    assert!(
        total_fixed > 50,
        "rules never triggered ({} fixings over 100 instances) — test is vacuous",
        total_fixed
    );
}

#[test]
fn presolve_preserves_optimal_energy_under_clamps() {
    let mut rng = ChaCha8Rng::seed_from_u64(43);
    for _ in 0..100 {
        let model = random_sparse_qubo(10, 0.3, &mut rng);
        let clamp_idx = rng.gen_range(0..10);
        let clamp_val = rng.gen_range(0..=1) as i8;
        let clamps = vec![(clamp_idx, clamp_val)];
        let derived = fix_persistent_variables(&model, &clamps);
        // Derived fixings must never touch the clamped index.
        assert!(derived.iter().all(|&(i, _)| i != clamp_idx));
        let mut all_fixed = clamps.clone();
        all_fixed.extend_from_slice(&derived);
        let e_clamped = brute_force_optimum(&model, &clamps);
        let e_restricted = brute_force_optimum(&model, &all_fixed);
        assert!(
            (e_clamped - e_restricted).abs() < 1e-9,
            "presolve lost the clamped optimum: {} vs {}",
            e_clamped,
            e_restricted
        );
    }
}

#[test]
fn all_nonnegative_couplings_and_positive_fields_fix_everything_to_zero() {
    // h_i > 0, w_ij ≥ 0 ⇒ L_i = h_i > 0 for every i ⇒ all fixed to 0 in pass 1.
    let n = 6;
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                col_indices.push(j);
                values.push(0.7);
            }
        }
        row_offsets.push(col_indices.len());
    }
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: n,
        linear: vec![0.5; n],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };
    let derived = fix_persistent_variables(&model, &[]);
    assert_eq!(derived.len(), n);
    assert!(derived.iter().all(|&(_, v)| v == 0));
}

#[test]
fn cascade_requires_iteration_to_fixpoint() {
    // x0: h=1, single coupling w01=-5 ⇒ L_0 = 1-5 = -4 < 0, U_0 = 1 > 0 —
    // NOT fixable while x1 is free.
    // x1: h=-6, couplings w01=-5, w12=+3 ⇒ U_1 = -6+3 = -3 ≤ 0 ⇒ fix x1=1.
    // Folding x1=1: h0_eff = 1-5 = -4 ⇒ U_0 = -4 ≤ 0 ⇒ fix x0=1 (2nd pass).
    //               h2_eff = 1+3 = 4  ⇒ L_2 = 4 ≥ 0  ⇒ fix x2=0.
    let quads = [(0usize, 1usize, -5.0f64), (1, 2, 3.0)];
    let mut rows: Vec<Vec<(usize, f64)>> = vec![vec![]; 3];
    for &(u, v, w) in &quads {
        rows[u].push((v, w));
        rows[v].push((u, w));
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for r in &rows {
        for &(j, w) in r {
            col_indices.push(j);
            values.push(w);
        }
        row_offsets.push(col_indices.len());
    }
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 3,
        linear: vec![1.0, -6.0, 1.0],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };
    let mut derived = fix_persistent_variables(&model, &[]);
    derived.sort_unstable();
    assert_eq!(derived, vec![(0, 1), (1, 1), (2, 0)]);
    // Cross-check by enumeration.
    let e_full = brute_force_optimum(&model, &[]);
    let e_restricted = brute_force_optimum(&model, &derived);
    assert!((e_full - e_restricted).abs() < 1e-9);
}

#[test]
fn user_clamp_propagates_into_fixings() {
    // Clamp x0 = 1 with w01 = +10, h1 = -0.5: effective field of x1 becomes
    // 9.5 ⇒ L_1 = 9.5 ≥ 0 ⇒ x1 fixed to 0. Without the clamp, x1 is free
    // (L_1 = -0.5 < 0 < 9.5 = U_1) and in fact the global optimum sets x1=1.
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 2,
        linear: vec![0.0, -0.5],
        quadratic: CsrMatrix {
            values: vec![10.0, 10.0],
            col_indices: vec![1, 0],
            row_offsets: vec![0, 1, 2],
        },
    };
    let no_clamp = fix_persistent_variables(&model, &[]);
    assert!(
        !no_clamp.iter().any(|&(i, v)| i == 1 && v == 0),
        "x1 must not be fixed to 0 without the clamp"
    );
    let derived = fix_persistent_variables(&model, &[(0, 1)]);
    assert_eq!(derived, vec![(1, 0)]);
}

#[test]
fn solver_with_presolve_finds_exhaustive_optimum() {
    let mut rng = ChaCha8Rng::seed_from_u64(47);
    for trial in 0..5 {
        let model = random_sparse_qubo(10, 0.3, &mut rng);
        let e_opt = brute_force_optimum(&model, &[]);
        let solver = UltimateSolver::new(20.0, 0.05, 30, 40, Some(500 + trial));
        let state = solver.solve(&model, &[]);
        let e_found = model.calculate_total_energy(&state);
        assert!(
            (e_found - e_opt).abs() < 1e-9,
            "trial {}: solver+presolve returned {} but optimum is {}",
            trial,
            e_found,
            e_opt
        );
    }
}
