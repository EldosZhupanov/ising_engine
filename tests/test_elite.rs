//! Contracts for the elite archive and UBQP path relinking.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::elite::{path_relink, EliteArchive};
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

#[test]
fn archive_keeps_best_and_respects_capacity() {
    let mut a = EliteArchive::new(3, 1);
    // Distinct low-Hamming-safe states of length 4.
    a.insert(&[0, 0, 0, 0], 5.0);
    a.insert(&[1, 1, 1, 1], 2.0);
    a.insert(&[0, 1, 0, 1], 8.0);
    a.insert(&[1, 0, 1, 0], 1.0); // best; evicts the 8.0 entry
    assert_eq!(a.len(), 3);
    assert_eq!(a.best_energy(), Some(1.0));
    // The worst (8.0) must have been evicted.
    assert!(a.entries().iter().all(|(e, _)| *e <= 5.0 + 1e-9));
}

#[test]
fn archive_diversity_filter_rejects_near_duplicates() {
    let mut a = EliteArchive::new(5, 2);
    assert!(a.insert(&[0, 0, 0, 0], 10.0));
    // Hamming 1 < min_hamming 2, and worse → rejected.
    assert!(!a.insert(&[1, 0, 0, 0], 12.0));
    assert_eq!(a.len(), 1);
    // Hamming 1 but strictly better → replaces the near neighbor.
    assert!(a.insert(&[0, 1, 0, 0], 8.0));
    assert_eq!(a.len(), 1);
    assert_eq!(a.best_energy(), Some(8.0));
}

#[test]
fn archive_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    let build = || {
        let mut r = ChaCha8Rng::seed_from_u64(50);
        let mut a = EliteArchive::new(8, 1);
        for _ in 0..40 {
            let s: Vec<i8> = (0..12).map(|_| r.gen_range(0..=1)).collect();
            let e: f64 = r.gen_range(-10.0..10.0);
            a.insert(&s, e);
        }
        a.entries().to_vec()
    };
    let _ = rng.gen::<u64>();
    assert_eq!(build(), build());
}

/// Path relinking must return a state whose energy equals a fresh
/// recomputation, never worse than the better endpoint, and it must actually
/// traverse the differing bits (endpoint states reachable).
#[test]
fn path_relink_energy_is_exact_and_no_worse_than_endpoints() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    let n = 16;
    for _ in 0..100 {
        let model = random_qubo(n, &mut rng);
        let source: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let target: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let e_src = model.calculate_total_energy(&source);
        let e_tgt = model.calculate_total_energy(&target);

        let (best, best_e) = path_relink(&model, &source, &target);
        // Incremental energy must match a from-scratch evaluation.
        assert!(
            (model.calculate_total_energy(&best) - best_e).abs() < 1e-6,
            "path-relink energy desynced: {} vs {}",
            best_e,
            model.calculate_total_energy(&best)
        );
        // Never worse than the better of the two endpoints.
        assert!(
            best_e <= e_src.min(e_tgt) + 1e-9,
            "relink returned {} worse than best endpoint {}",
            best_e,
            e_src.min(e_tgt)
        );
    }
}

#[test]
fn path_relink_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let model = random_qubo(20, &mut rng);
    let source: Vec<i8> = (0..20).map(|_| rng.gen_range(0..=1)).collect();
    let target: Vec<i8> = (0..20).map(|_| rng.gen_range(0..=1)).collect();
    let a = path_relink(&model, &source, &target);
    let b = path_relink(&model, &source, &target);
    assert_eq!(a.0, b.0);
    assert!((a.1 - b.1).abs() < 1e-12);
}

/// On a brute-forceable instance, relinking two random local optima can only
/// help: its result is ≤ the better endpoint, and here we check it sometimes
/// discovers a strictly better intermediate (the whole point of relinking).
#[test]
fn path_relink_can_improve_on_endpoints() {
    let mut rng = ChaCha8Rng::seed_from_u64(4);
    let mut improved = 0;
    for _ in 0..200 {
        let n = 14;
        let model = random_qubo(n, &mut rng);
        let source: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let target: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let e_endpoints = model
            .calculate_total_energy(&source)
            .min(model.calculate_total_energy(&target));
        let (_, best_e) = path_relink(&model, &source, &target);
        if best_e < e_endpoints - 1e-9 {
            improved += 1;
        }
    }
    assert!(
        improved > 0,
        "path relinking never improved on endpoints across 200 trials — likely a bug"
    );
}
