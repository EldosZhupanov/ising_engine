//! Correctness contract for Isoenergetic Cluster Moves (Houdayer / ICM).
//!
//! The defining property (Houdayer 2001; Zhu, Ochoa & Katzgraber, PRL 115,
//! 077201 (2015)): flipping a disagreement cluster in BOTH replicas of a
//! same-temperature pair conserves the pair's total energy exactly, so the
//! move is rejection-free. These tests verify:
//!   1. pair-total energy conservation (isoenergetic),
//!   2. tracked energies stay synchronized with spins after the move,
//!   3. applicability gating on higher-order models,
//!   4. determinism,
//!   5. end-to-end solver quality with ICM enabled (exhaustive optimum).

use ising_engine::core::hubo::{Edge2, Edge3, FlatHuboModel, HuboModel};
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::engine::calculate_replica_energies;
use ising_engine::solver::icm::{icm_pair_move, icm_sweep, is_icm_applicable};
use ising_engine::solver::types::{QuantumField, NUM_REPLICAS};
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn random_pairwise(n: usize, rng: &mut ChaCha8Rng) -> FlatHuboModel {
    let mut hubo = HuboModel::new(n);
    for i in 0..n {
        hubo.linear[i] = rng.gen_range(-1.5..1.5);
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < 0.5 {
                let w = rng.gen_range(-2.0..2.0);
                hubo.edges2[i].push(Edge2 { j, weight: w });
                hubo.edges2[j].push(Edge2 { j: i, weight: w });
            }
        }
    }
    FlatHuboModel::from_hubo(&hubo)
}

/// Sum of the two paired lanes' energies must be invariant under an ICM move,
/// and each lane's tracked energy must match a full recomputation.
#[test]
fn icm_move_is_isoenergetic_and_keeps_energies_synced() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    let n = 12;
    for _ in 0..100 {
        let flat = random_pairwise(n, &mut rng);
        let mut field = QuantumField::new(n, 1, 1, 1);
        // Two independent random configs in lanes 0 and 32.
        for v in 0..n {
            field.set_replica(v, 0, 0, 0, 0, rng.gen_range(0..=1));
            field.set_replica(v, 0, 0, 0, 32, rng.gen_range(0..=1));
        }
        field.energies[0] = calculate_replica_energies(&flat, &field, 0, 0, 0.0);
        let total_before = field.energies[0][0] + field.energies[0][32];

        let mut in_cluster = vec![false; n];
        let mut visited = vec![false; n];
        let mut stack = Vec::new();
        icm_pair_move(
            &mut field,
            &flat,
            0,
            0,
            0,
            32,
            &mut in_cluster,
            &mut visited,
            &mut stack,
            &mut rng,
        );

        // Pair total conserved (isoenergetic).
        let total_after = field.energies[0][0] + field.energies[0][32];
        assert!(
            (total_before - total_after).abs() < 1e-9,
            "ICM not isoenergetic: {} -> {}",
            total_before,
            total_after
        );
        // Tracked energies match a from-scratch recomputation.
        let recomputed = calculate_replica_energies(&flat, &field, 0, 0, 0.0);
        assert!(
            (field.energies[0][0] - recomputed[0]).abs() < 1e-9,
            "lane 0 tracked energy desynchronized"
        );
        assert!(
            (field.energies[0][32] - recomputed[32]).abs() < 1e-9,
            "lane 32 tracked energy desynchronized"
        );
    }
}

/// A full sweep over all 32 pairs and multiple cells keeps every lane's
/// tracked energy synchronized.
#[test]
fn icm_sweep_keeps_all_lanes_synced() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    let n = 10;
    let flat = random_pairwise(n, &mut rng);
    let mut field = QuantumField::new(n, 1, 3, 2);
    for p in 0..2 {
        for t in 0..3 {
            for v in 0..n {
                for r in 0..NUM_REPLICAS {
                    field.set_replica(v, 0, t, p, r, rng.gen_range(0..=1));
                }
            }
        }
    }
    for p in 0..2 {
        for t in 0..3 {
            field.energies[t + 3 * p] = calculate_replica_energies(&flat, &field, t, p, 0.0);
        }
    }
    icm_sweep(&mut field, &flat, &mut rng);
    for p in 0..2 {
        for t in 0..3 {
            let recomputed = calculate_replica_energies(&flat, &field, t, p, 0.0);
            for (r, &e) in recomputed.iter().enumerate() {
                assert!(
                    (field.energies[t + 3 * p][r] - e).abs() < 1e-9,
                    "cell (t={}, p={}) lane {} desynced after ICM sweep",
                    t,
                    p,
                    r
                );
            }
        }
    }
}

#[test]
fn icm_applicability_gates_on_higher_order_terms() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let pairwise = random_pairwise(6, &mut rng);
    assert!(is_icm_applicable(&pairwise));

    let mut hubo = HuboModel::new(4);
    hubo.edges3[0].push(Edge3 {
        j: 1,
        k: 2,
        weight: 1.0,
    });
    let with_cubic = FlatHuboModel::from_hubo(&hubo);
    assert!(
        !is_icm_applicable(&with_cubic),
        "ICM must be disabled on models with 3-body terms"
    );
}

#[test]
fn icm_move_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    let n = 12;
    let flat = random_pairwise(n, &mut rng);
    let make_field = || {
        let mut f = QuantumField::new(n, 1, 1, 1);
        let mut r = ChaCha8Rng::seed_from_u64(99);
        for v in 0..n {
            f.set_replica(v, 0, 0, 0, 0, r.gen_range(0..=1));
            f.set_replica(v, 0, 0, 0, 32, r.gen_range(0..=1));
        }
        f.energies[0] = calculate_replica_energies(&flat, &f, 0, 0, 0.0);
        f
    };
    let run = || {
        let mut f = make_field();
        let (mut a, mut b, mut s) = (vec![false; n], vec![false; n], Vec::new());
        let mut mrng = ChaCha8Rng::seed_from_u64(7);
        icm_pair_move(
            &mut f, &flat, 0, 0, 0, 32, &mut a, &mut b, &mut s, &mut mrng,
        );
        f.spins.clone()
    };
    assert_eq!(run(), run(), "ICM move must be deterministic given a seed");
}

fn random_qubo(n: usize, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut hubo = HuboModel::new(n);
    for i in 0..n {
        hubo.linear[i] = rng.gen_range(-2.0..2.0);
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < 0.5 {
                hubo.edges2[i].push(Edge2 {
                    j,
                    weight: rng.gen_range(-2.0..2.0),
                });
            }
        }
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for e in &hubo.edges2[i] {
            col_indices.push(e.j);
            values.push(e.weight);
            // also symmetric partner
        }
        row_offsets.push(col_indices.len());
    }
    // Build a symmetric CSR from the upper triangle.
    let mut rows: Vec<Vec<(usize, f64)>> = vec![vec![]; n];
    for i in 0..n {
        for e in &hubo.edges2[i] {
            rows[i].push((e.j, e.weight));
            rows[e.j].push((i, e.weight));
        }
    }
    let (mut v2, mut c2, mut r2) = (Vec::new(), Vec::new(), vec![0]);
    for row in &mut rows {
        row.sort_by_key(|&(j, _)| j);
        for &(j, w) in row.iter() {
            c2.push(j);
            v2.push(w);
        }
        r2.push(c2.len());
    }
    QuboModel {
        num_vars: n,
        linear: hubo.linear,
        quadratic: CsrMatrix {
            values: v2,
            col_indices: c2,
            row_offsets: r2,
        },
        energy_offset: 0.0,
    }
}

fn brute(model: &QuboModel) -> f64 {
    let n = model.num_vars;
    let mut best = f64::INFINITY;
    for bits in 0..(1u32 << n) {
        let s: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        best = best.min(model.calculate_total_energy(&s));
    }
    best
}

/// End-to-end: with ICM enabled the solver stays correct and reaches the
/// exhaustive optimum; results remain deterministic.
#[test]
fn solver_with_icm_reaches_optimum_and_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(5);
    for trial in 0..5 {
        let model = random_qubo(14, &mut rng);
        let e_opt = brute(&model);
        let mut s = UltimateSolver::new(10.0, 0.05, 10, 20, Some(300 + trial));
        s.use_icm = true;
        let out1 = s.solve(&model, &[]);
        let mut s2 = UltimateSolver::new(10.0, 0.05, 10, 20, Some(300 + trial));
        s2.use_icm = true;
        let out2 = s2.solve(&model, &[]);
        assert_eq!(out1, out2, "ICM solve must be deterministic");
        let e = model.calculate_total_energy(&out1);
        assert!(
            (e - e_opt).abs() < 1e-9,
            "trial {}: ICM solver returned {} but optimum is {}",
            trial,
            e,
            e_opt
        );
    }
}
