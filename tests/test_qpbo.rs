//! Exhaustive validation of roof-duality / QPBO weak persistency.
//!
//! The load-bearing safety property (Hammer-Hansen-Simeone 1984): roof
//! duality yields STRONG persistency — a variable it fixes takes that value
//! in EVERY global optimum. These tests verify against brute force that
//! (1) every fixed variable is strongly persistent, (2) the reduced problem
//! has the identical global optimum, (3) submodular instances are solved
//! completely and exactly, (4) it never fixes a free variable, plus fuzz,
//! determinism, and edge cases.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::presolve::qpbo::roof_duality_persistencies;
use ising_engine::presolve::{fix_persistent_variables_probing, full_presolve};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Random QUBO. `min_w..max_w` sets the coupling sign mix; density controls
/// sparsity. Linear terms in [-2, 2].
#[allow(clippy::needless_range_loop)]
fn random_qubo(n: usize, density: f64, wlo: f64, whi: f64, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < density {
                upper[i][j] = rng.gen_range(wlo..whi);
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

/// All global optima (as bit vectors).
fn all_optima(m: &QuboModel) -> Vec<Vec<i8>> {
    let n = m.num_vars;
    let mut best = f64::INFINITY;
    let mut set = Vec::new();
    for bits in 0..(1u32 << n) {
        let s: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        let e = m.calculate_total_energy(&s);
        if e < best - 1e-9 {
            best = e;
            set = vec![s];
        } else if (e - best).abs() < 1e-9 {
            set.push(s);
        }
    }
    set
}

fn restricted_optimum(m: &QuboModel, fixed: &[(usize, i8)]) -> f64 {
    let n = m.num_vars;
    let mut best = f64::INFINITY;
    'outer: for bits in 0..(1u32 << n) {
        let s: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        for &(i, v) in fixed {
            if s[i] != v {
                continue 'outer;
            }
        }
        best = best.min(m.calculate_total_energy(&s));
    }
    best
}

#[test]
fn qpbo_fixings_are_strongly_persistent() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    let mut total_fixed = 0usize;
    let mut mismatches = 0usize;
    for trial in 0..4000 {
        let n = 6 + (trial % 5); // 6..10
        let density = 0.3 + 0.5 * ((trial % 3) as f64 / 3.0);
        let model = random_qubo(n, density, -2.0, 2.0, &mut rng);
        let optima = all_optima(&model);
        let fix = roof_duality_persistencies(&model);
        for i in 0..n {
            if let Some(v) = fix[i] {
                total_fixed += 1;
                // STRONG persistency: every optimum has x_i = v.
                if !optima.iter().all(|s| s[i] == v) {
                    mismatches += 1;
                }
            }
        }
    }
    assert_eq!(
        mismatches, 0,
        "{} of {} QPBO fixings were NOT strongly persistent",
        mismatches, total_fixed
    );
    assert!(
        total_fixed > 1000,
        "test too weak: only {} fixings",
        total_fixed
    );
}

#[test]
fn qpbo_preserves_global_optimum() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    for trial in 0..3000 {
        let n = 6 + (trial % 6); // 6..11
        let model = random_qubo(n, 0.5, -2.0, 2.0, &mut rng);
        let fix: Vec<(usize, i8)> = roof_duality_persistencies(&model)
            .into_iter()
            .enumerate()
            .filter_map(|(i, v)| v.map(|x| (i, x)))
            .collect();
        let e_full = restricted_optimum(&model, &[]);
        let e_restricted = restricted_optimum(&model, &fix);
        assert!(
            (e_full - e_restricted).abs() < 1e-9,
            "QPBO changed the optimum: {} vs {} (fixed {:?})",
            e_full,
            e_restricted,
            fix
        );
    }
}

/// Submodular QUBO (all couplings ≤ 0) is exactly minimizable by graph cuts:
/// roof duality must fix EVERY variable, and the fixed assignment must be a
/// global optimum.
#[test]
fn qpbo_solves_submodular_exactly() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    for _ in 0..1500 {
        let n = 6 + rng.gen_range(0..5);
        // All couplings in [-2, 0] ⇒ submodular.
        let model = random_qubo(n, 0.6, -2.0, 0.0, &mut rng);
        let optima = all_optima(&model);
        let fix = roof_duality_persistencies(&model);
        // Submodular roof duality is exact: it labels every variable that is
        // the same across all optima. When the optimum is UNIQUE, that is
        // every variable, and the labeling equals the global optimum.
        if optima.len() == 1 {
            let assigned: Vec<i8> = (0..n)
                .map(|i| fix[i].expect("unique-optimum submodular: every var must be fixed"))
                .collect();
            assert_eq!(assigned, optima[0], "submodular labeling != unique optimum");
        }
        // In all cases every fixing is strongly persistent.
        for i in 0..n {
            if let Some(v) = fix[i] {
                assert!(optima.iter().all(|s| s[i] == v));
            }
        }
    }
}

#[test]
fn qpbo_never_fixes_a_free_variable() {
    // A variable that takes BOTH values across the optima set must NOT be
    // fixed. (Directly implied by strong persistency, tested separately for
    // clarity on instances with degenerate optima.)
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    for _ in 0..3000 {
        let n = 6 + rng.gen_range(0..5);
        let model = random_qubo(n, 0.5, -2.0, 2.0, &mut rng);
        let optima = all_optima(&model);
        let fix = roof_duality_persistencies(&model);
        for i in 0..n {
            let v0 = optima.iter().any(|s| s[i] == 0);
            let v1 = optima.iter().any(|s| s[i] == 1);
            if v0 && v1 {
                assert!(
                    fix[i].is_none(),
                    "var {} varies across optima but QPBO fixed it to {:?}",
                    i,
                    fix[i]
                );
            }
        }
    }
}

#[test]
fn qpbo_agrees_with_probing_where_both_fix() {
    let mut rng = ChaCha8Rng::seed_from_u64(5);
    for _ in 0..2000 {
        let n = 6 + rng.gen_range(0..6);
        let model = random_qubo(n, 0.5, -2.0, 2.0, &mut rng);
        let qpbo = roof_duality_persistencies(&model);
        let probing = fix_persistent_variables_probing(&model, &[]);
        for &(i, v) in &probing {
            if let Some(qv) = qpbo[i] {
                assert_eq!(qv, v, "QPBO and probing disagree on var {}", i);
            }
        }
    }
}

#[test]
fn qpbo_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(6);
    let model = random_qubo(14, 0.4, -2.0, 2.0, &mut rng);
    assert_eq!(
        roof_duality_persistencies(&model),
        roof_duality_persistencies(&model)
    );
}

#[test]
fn qpbo_fuzz_extremes_and_edge_cases() {
    // n = 1.
    let m1 = QuboModel {
        num_vars: 1,
        linear: vec![-3.0],
        quadratic: CsrMatrix::empty(1),
        energy_offset: 0.0,
    };
    assert_eq!(roof_duality_persistencies(&m1), vec![Some(1)]);
    let m0 = QuboModel {
        num_vars: 1,
        linear: vec![3.0],
        quadratic: CsrMatrix::empty(1),
        energy_offset: 0.0,
    };
    assert_eq!(roof_duality_persistencies(&m0), vec![Some(0)]);

    // All-zero model: nothing is strictly persistent → all free.
    let mz = QuboModel {
        num_vars: 4,
        linear: vec![0.0; 4],
        quadratic: CsrMatrix::empty(4),
        energy_offset: 0.0,
    };
    assert!(roof_duality_persistencies(&mz).iter().all(|x| x.is_none()));

    // Extreme magnitudes + many zeros, validated against brute force.
    let mut rng = ChaCha8Rng::seed_from_u64(77);
    for _ in 0..1000 {
        let n = 5 + rng.gen_range(0..5);
        let model = random_qubo(n, 0.4, -1e6, 1e6, &mut rng);
        let fix: Vec<(usize, i8)> = roof_duality_persistencies(&model)
            .into_iter()
            .enumerate()
            .filter_map(|(i, v)| v.map(|x| (i, x)))
            .collect();
        let e_full = restricted_optimum(&model, &[]);
        let e_restricted = restricted_optimum(&model, &fix);
        assert!(
            (e_full - e_restricted).abs() < 1e-3 * e_full.abs().max(1.0),
            "extreme-magnitude QPBO changed the optimum: {} vs {}",
            e_full,
            e_restricted
        );
    }
}

/// The integrated production presolve (QPBO ∪ probing) must preserve the
/// optimum, respect clamps, and be at least as strong as probing alone.
#[test]
fn full_presolve_preserves_optimum_and_dominates() {
    let mut rng = ChaCha8Rng::seed_from_u64(8);
    let mut total_full = 0usize;
    let mut total_probing = 0usize;
    for _ in 0..2000 {
        let n = 6 + rng.gen_range(0..6);
        let model = random_qubo(n, 0.45, -2.0, 2.0, &mut rng);
        let full = full_presolve(&model, &[]);
        let probing = fix_persistent_variables_probing(&model, &[]);
        total_full += full.len();
        total_probing += probing.len();
        let e_full = restricted_optimum(&model, &[]);
        let e_restricted = restricted_optimum(&model, &full);
        assert!(
            (e_full - e_restricted).abs() < 1e-9,
            "full presolve changed the optimum: {} vs {}",
            e_full,
            e_restricted
        );
        // Superset of probing (QPBO strong-persistent + probing weak, unioned).
        for &(i, v) in &probing {
            assert!(
                full.iter().any(|&(k, w)| k == i && w == v),
                "full presolve dropped a probing fixing"
            );
        }
    }
    assert!(
        total_full >= total_probing,
        "full presolve ({}) weaker than probing ({})",
        total_full,
        total_probing
    );
}

#[test]
fn full_presolve_respects_clamps() {
    let mut rng = ChaCha8Rng::seed_from_u64(9);
    for _ in 0..1000 {
        let n = 8 + rng.gen_range(0..4);
        let model = random_qubo(n, 0.4, -2.0, 2.0, &mut rng);
        let ci = rng.gen_range(0..n);
        let cv = rng.gen_range(0..=1) as i8;
        let clamps = vec![(ci, cv)];
        let full = full_presolve(&model, &clamps);
        assert!(full.iter().all(|&(i, _)| i != ci));
        let mut all = clamps.clone();
        all.extend_from_slice(&full);
        let e_clamped = restricted_optimum(&model, &clamps);
        let e_restricted = restricted_optimum(&model, &all);
        assert!(
            (e_clamped - e_restricted).abs() < 1e-9,
            "full presolve lost the clamped optimum: {} vs {}",
            e_clamped,
            e_restricted
        );
    }
}
