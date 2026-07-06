//! Exhaustive correctness for exact probing persistency.
//!
//! Verifies against brute force that (a) probing never eliminates a variable
//! wrongly (the restricted optimum equals the true optimum), (b) probing is
//! at least as strong as first-order persistency, and (c) it strictly
//! dominates on some instances.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::presolve::{fix_persistent_variables, fix_persistent_variables_probing};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[allow(clippy::needless_range_loop)]
fn random_sparse(n: usize, density: f64, rng: &mut ChaCha8Rng) -> QuboModel {
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

fn brute_optimum(model: &QuboModel, fixed: &[(usize, i8)]) -> f64 {
    let n = model.num_vars;
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

#[test]
fn probing_preserves_optimum_exhaustive() {
    let mut rng = ChaCha8Rng::seed_from_u64(11);
    let mut total_probed = 0usize;
    let mut total_first_order = 0usize;
    for _ in 0..500 {
        let model = random_sparse(12, 0.35, &mut rng);
        let probed = fix_persistent_variables_probing(&model, &[]);
        let first = fix_persistent_variables(&model, &[]);
        total_probed += probed.len();
        total_first_order += first.len();

        let e_full = brute_optimum(&model, &[]);
        let e_probed = brute_optimum(&model, &probed);
        assert!(
            (e_full - e_probed).abs() < 1e-9,
            "probing changed the optimum: full {} vs restricted {} (fixed {:?})",
            e_full,
            e_probed,
            probed
        );
        // Probing must dominate first-order (fix a superset).
        for &(i, v) in &first {
            assert!(
                probed.iter().any(|&(k, w)| k == i && w == v),
                "probing missed a first-order fixing {:?}={}",
                i,
                v
            );
        }
    }
    // Probing must actually be STRICTLY stronger in aggregate.
    assert!(
        total_probed > total_first_order,
        "probing fixed {} vars vs first-order {} — no strict improvement",
        total_probed,
        total_first_order
    );
}

#[test]
fn probing_preserves_optimum_under_clamps() {
    let mut rng = ChaCha8Rng::seed_from_u64(13);
    for _ in 0..300 {
        let model = random_sparse(12, 0.35, &mut rng);
        let ci = rng.gen_range(0..12);
        let cv = rng.gen_range(0..=1) as i8;
        let clamps = vec![(ci, cv)];
        let probed = fix_persistent_variables_probing(&model, &clamps);
        assert!(probed.iter().all(|&(i, _)| i != ci));
        let mut all = clamps.clone();
        all.extend_from_slice(&probed);
        let e_clamped = brute_optimum(&model, &clamps);
        let e_restricted = brute_optimum(&model, &all);
        assert!(
            (e_clamped - e_restricted).abs() < 1e-9,
            "probing lost the clamped optimum: {} vs {}",
            e_clamped,
            e_restricted
        );
    }
}

#[test]
fn probing_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(17);
    let model = random_sparse(14, 0.4, &mut rng);
    let a = fix_persistent_variables_probing(&model, &[]);
    let b = fix_persistent_variables_probing(&model, &[]);
    assert_eq!(a, b);
}
