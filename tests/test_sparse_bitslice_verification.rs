//! Verification Track (Constitution §4; ADR-0002, ADR-0004): `SparseBitSlice`
//! is the fast integer backend; `ReferenceState` is the correct-by-construction
//! f64 oracle. This harness proves the two agree bit-for-bit across THOUSANDS of
//! automated checks over randomly generated integral instances and random flip
//! sequences — every observable that crosses the `SpinState` interface.
//!
//! Nothing here is an algorithm. It is pure state mechanics under audit: for
//! each step we assert equal ΔE, equal per-replica energies, equal overlaps,
//! equal extracted configurations, and zero field-ledger drift on BOTH
//! backends, plus agreement with the canonical IR scorer.

use ising_engine::engine_v2::backends::{ReferenceState, SparseBitSlice};
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::state::{ReplicaMask, SpinState};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Deterministically generate a random INTEGRAL instance (couplings and linear
/// terms are integers, so `SparseBitSlice`'s exact arithmetic applies).
fn random_integral_ir(rng: &mut StdRng, n: usize, wmax: i64) -> ProblemIR {
    let linear: Vec<f64> = (0..n).map(|_| rng.gen_range(-wmax..=wmax) as f64).collect();
    let mut pairs = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            // ~35% edge density; skip zero weights (no edge).
            if rng.gen_bool(0.35) {
                let w = rng.gen_range(-wmax..=wmax);
                if w != 0 {
                    pairs.push((i as u32, j as u32, w as f64));
                }
            }
        }
    }
    let offset = rng.gen_range(-wmax..=wmax) as f64;
    ProblemIR::from_pairs(n, offset, linear, &pairs)
}

fn random_init(rng: &mut StdRng, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.gen::<bool>() as u8).collect()
}

#[test]
fn sparse_bitslice_matches_reference_over_thousands_of_checks() {
    // Replica counts deliberately straddle the 64-bit word boundary so the
    // multi-word bit-plane path (w > 1) and partial-tail masking are exercised.
    let replica_counts = [1usize, 8, 63, 64, 65, 100, 128, 200];
    let sizes = [2usize, 3, 5, 8, 13, 21, 34];
    let weight_ranges = [1i64, 3, 7, 50];

    let mut checks: u64 = 0;
    let mut audits: u64 = 0;
    // 400 independent instances × ~80 steps → well into the thousands of
    // vector-equality assertions, each spanning up to 200 replicas.
    for instance in 0..400u64 {
        let mut rng = StdRng::seed_from_u64(0x5B17_0000 ^ instance);
        let n = sizes[instance as usize % sizes.len()];
        let r = replica_counts[(instance as usize / sizes.len()) % replica_counts.len()];
        let wmax = weight_ranges[(instance as usize / 3) % weight_ranges.len()];

        let ir = random_integral_ir(&mut rng, n, wmax);
        let init = random_init(&mut rng, n);

        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init)
            .expect("integral instance must build a SparseBitSlice");

        // Fresh builds must already agree (init + rebuild path).
        assert_eq!(
            bs.audit(),
            0.0,
            "bitslice built with drift (instance {instance})"
        );
        assert_eq!(
            refs.audit(),
            0.0,
            "reference built with drift (instance {instance})"
        );
        audits += 2;

        let mut d_ref = vec![0.0; r];
        let mut d_bs = vec![0.0; r];
        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];

        for step in 0..80 {
            let site = rng.gen_range(0..n);

            // (1) ΔE must agree BEFORE the flip, for every replica.
            refs.delta_e_into(site, &mut d_ref);
            bs.delta_e_into(site, &mut d_bs);
            assert_eq!(
                d_ref, d_bs,
                "ΔE mismatch: instance {instance} step {step} site {site}"
            );
            checks += 1;

            // Identical random replica mask applied to both backends.
            let mut mask = ReplicaMask::new(r);
            for rep in 0..r {
                if rng.gen::<bool>() {
                    mask.set(rep);
                }
            }
            refs.apply_flips(site, &mask);
            bs.apply_flips(site, &mask);

            // (2) Per-replica energies must agree AFTER the flip.
            refs.energies_into(&mut e_ref);
            bs.energies_into(&mut e_bs);
            assert_eq!(
                e_ref, e_bs,
                "energy mismatch: instance {instance} step {step} site {site}"
            );
            checks += 1;

            // (3) Overlap between two random replicas must agree.
            if r >= 2 {
                let a = rng.gen_range(0..r);
                let b = rng.gen_range(0..r);
                assert_eq!(
                    refs.overlap(a, b),
                    bs.overlap(a, b),
                    "overlap mismatch: instance {instance} step {step} reps {a},{b}"
                );
                checks += 1;
            }

            // (4) Extracted configuration of a random replica must be identical,
            //     and the canonical IR scorer must equal the ledger energy.
            let rep = rng.gen_range(0..r);
            let mut x_ref = vec![0u8; n];
            let mut x_bs = vec![0u8; n];
            refs.extract_into(rep, &mut x_ref);
            bs.extract_into(rep, &mut x_bs);
            assert_eq!(
                x_ref, x_bs,
                "config mismatch: instance {instance} step {step} rep {rep}"
            );
            assert_eq!(
                ir.energy(&x_bs),
                e_bs[rep],
                "canonical scorer disagrees with ledger: instance {instance} rep {rep}"
            );
            checks += 2;

            // (5) Periodically re-derive both field ledgers from scratch: zero
            //     drift is the invariant the incremental update must preserve.
            if step % 20 == 0 {
                assert_eq!(
                    bs.audit(),
                    0.0,
                    "bitslice drift at instance {instance} step {step}"
                );
                assert_eq!(
                    refs.audit(),
                    0.0,
                    "reference drift at instance {instance} step {step}"
                );
                audits += 2;
            }
        }

        // Final full audit of every replica against the canonical scorer.
        assert_eq!(
            bs.audit(),
            0.0,
            "final bitslice drift (instance {instance})"
        );
        audits += 1;
        bs.energies_into(&mut e_bs);
        for (rep, &e) in e_bs.iter().enumerate() {
            let mut x = vec![0u8; n];
            bs.extract_into(rep, &mut x);
            assert_eq!(
                ir.energy(&x),
                e,
                "final canonical mismatch: instance {instance} rep {rep}"
            );
            checks += 1;
        }
    }

    // Guard: the harness must actually run the intended volume of checks.
    assert!(
        checks > 100_000,
        "verification harness under-covered: only {checks} checks"
    );
    eprintln!("sparse_bitslice verification: {checks} cross-checks, {audits} full audits");
}
