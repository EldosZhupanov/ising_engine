//! Verification Track (Constitution §4; ADR-0002, ADR-0004): `DenseByteState`
//! is the dense float workhorse; `ReferenceState` is the correct-by-construction
//! oracle. This harness proves the two agree BIT-FOR-BIT across thousands of
//! automated checks — random dense instances (float and integral weights),
//! random flip sequences, every observable that crosses the `SpinState`
//! interface: ΔE, per-replica energies, overlaps, extracted configurations,
//! digests, and zero field-ledger drift on both backends.

use ising_engine::engine_v2::backends::{DenseByteState, ReferenceState};
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::state::{ReplicaMask, SpinState};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Random DENSE instance; float weights (the dense backend is f64-exact against
/// the oracle because both do the identical additions — zeros add nothing).
fn random_dense_ir(rng: &mut StdRng, n: usize, integral: bool) -> ProblemIR {
    let linear: Vec<f64> = (0..n)
        .map(|_| {
            if integral {
                rng.gen_range(-9i64..=9) as f64
            } else {
                rng.gen_range(-4.0..4.0)
            }
        })
        .collect();
    let mut pairs = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            // ~70% edge density — firmly in DenseByte territory.
            if rng.gen_bool(0.7) {
                let w = if integral {
                    rng.gen_range(-9i64..=9) as f64
                } else {
                    rng.gen_range(-4.0..4.0)
                };
                if w != 0.0 {
                    pairs.push((i as u32, j as u32, w));
                }
            }
        }
    }
    let offset = if integral {
        rng.gen_range(-5i64..=5) as f64
    } else {
        rng.gen_range(-5.0..5.0)
    };
    ProblemIR::from_pairs(n, offset, linear, &pairs)
}

#[test]
fn dense_byte_matches_reference_over_thousands_of_checks() {
    let replica_counts = [1usize, 8, 63, 64, 65, 100];
    let sizes = [2usize, 3, 5, 8, 13, 21, 34];

    let mut checks: u64 = 0;
    for instance in 0..300u64 {
        let mut rng = StdRng::seed_from_u64(0xDE45_0000 ^ instance);
        let n = sizes[instance as usize % sizes.len()];
        let r = replica_counts[(instance as usize / sizes.len()) % replica_counts.len()];
        let integral = instance % 2 == 0;

        let ir = random_dense_ir(&mut rng, n, integral);
        let init: Vec<u8> = (0..n).map(|_| rng.gen::<bool>() as u8).collect();
        let mut oracle = ReferenceState::new(&ir, r, &init);
        let mut dense = DenseByteState::new(&ir, r, &init);

        let mut de_a = vec![0.0; r];
        let mut de_b = vec![0.0; r];
        let mut e_a = vec![0.0; r];
        let mut e_b = vec![0.0; r];
        let mut x_a = vec![0u8; n];
        let mut x_b = vec![0u8; n];

        for step in 0..60 {
            let site = rng.gen_range(0..n);
            // ΔE agreement BEFORE the flip (bitwise: to_bits equality).
            oracle.delta_e_into(site, &mut de_a);
            dense.delta_e_into(site, &mut de_b);
            for (a, b) in de_a.iter().zip(&de_b) {
                assert_eq!(a.to_bits(), b.to_bits(), "ΔE diverged at step {step}");
                checks += 1;
            }
            // Random masked flip applied identically to both.
            let mut mask = ReplicaMask::new(r);
            for rep in 0..r {
                if rng.gen_bool(0.5) {
                    mask.set(rep);
                }
            }
            oracle.apply_flips(site, &mask);
            dense.apply_flips(site, &mask);

            // Energies, digests, extraction, overlap agree after the flip.
            oracle.energies_into(&mut e_a);
            dense.energies_into(&mut e_b);
            for (a, b) in e_a.iter().zip(&e_b) {
                assert_eq!(a.to_bits(), b.to_bits(), "energy diverged at step {step}");
                checks += 1;
            }
            assert_eq!(oracle.digest(), dense.digest());
            let rep = rng.gen_range(0..r);
            oracle.extract_into(rep, &mut x_a);
            dense.extract_into(rep, &mut x_b);
            assert_eq!(x_a, x_b);
            // Canonical scorer agreement. Bit-exact on integral instances; on
            // float instances the incremental ledger may differ from a fresh
            // recompute by summation-order ULPs (true of the oracle itself),
            // so there we bound the drift instead.
            let canon = ir.energy(&x_a);
            if integral {
                assert_eq!(canon.to_bits(), e_a[rep].to_bits());
            } else {
                let tol = 1e-9 * canon.abs().max(1.0);
                assert!(
                    (canon - e_a[rep]).abs() <= tol,
                    "ledger drifted: {canon} vs {}",
                    e_a[rep]
                );
            }
            if r >= 2 {
                let a = rng.gen_range(0..r);
                let b = rng.gen_range(0..r);
                assert_eq!(
                    oracle.overlap(a, b).to_bits(),
                    dense.overlap(a, b).to_bits()
                );
            }
        }
        // Ledger audit: zero drift on integral instances; float instances may
        // accumulate representable-rounding differences vs a fresh recompute,
        // but both backends must drift IDENTICALLY (same additions).
        let (da, db) = (oracle.audit(), dense.audit());
        assert_eq!(da.to_bits(), db.to_bits(), "audit drift differs");
        if integral {
            assert_eq!(da, 0.0, "integral ledgers must not drift");
        }
        // Replica copy/swap defaults stay exact on the dense backend.
        if r >= 2 {
            oracle.copy_replica(0, r - 1);
            dense.copy_replica(0, r - 1);
            oracle.swap_replicas(0, r / 2);
            dense.swap_replicas(0, r / 2);
            assert_eq!(oracle.digest(), dense.digest());
        }
    }
    assert!(
        checks > 100_000,
        "harness must exercise >100k checks: {checks}"
    );
}

/// Measured throughput comparison (run manually, release mode):
///   cargo test --release --test test_dense_byte_verification -- --ignored --nocapture
/// Reports flips/second for oracle vs dense on a dense instance. Informational
/// only — no perf claim is made without running this.
#[test]
#[ignore]
fn timing_dense_vs_reference_on_dense_instance() {
    let mut rng = StdRng::seed_from_u64(0xBEEF);
    let n = 1024;
    let r = 64;
    let ir = random_dense_ir(&mut rng, n, false);
    let init: Vec<u8> = (0..n).map(|_| rng.gen::<bool>() as u8).collect();
    let mut mask = ReplicaMask::new(r);
    for rep in 0..r {
        mask.set(rep);
    }
    let flips = 200_000usize;
    let sites: Vec<usize> = (0..flips).map(|_| rng.gen_range(0..n)).collect();

    let mut oracle = ReferenceState::new(&ir, r, &init);
    let t0 = std::time::Instant::now();
    for &s in &sites {
        oracle.apply_flips(s, &mask);
    }
    let t_ref = t0.elapsed();

    let mut dense = DenseByteState::new(&ir, r, &init);
    let t0 = std::time::Instant::now();
    for &s in &sites {
        dense.apply_flips(s, &mask);
    }
    let t_dense = t0.elapsed();

    assert_eq!(oracle.digest(), dense.digest(), "same trajectory");
    println!(
        "n={n} r={r} density~0.7, {flips} masked flips: reference {:.2?} ({:.1} Mflip-lane/s) vs dense {:.2?} ({:.1} Mflip-lane/s) — {:.2}x",
        t_ref,
        (flips * r) as f64 / t_ref.as_secs_f64() / 1e6,
        t_dense,
        (flips * r) as f64 / t_dense.as_secs_f64() / 1e6,
        t_ref.as_secs_f64() / t_dense.as_secs_f64()
    );
}
