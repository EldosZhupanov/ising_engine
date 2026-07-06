//! Round-trip / replica-flow tracking (Katzgraber-Trebst-Huse-Troyer 2006).
//!
//! Verifies that replica identity follows configurations through the physical
//! swaps: the flow f(T) = n_up/(n_up+n_down) is monotone across the ladder
//! (≈1 at the cold end, ≈0 at the hot end), round trips are counted, and the
//! instrumentation is deterministic and zero-overhead when disabled.

use ising_engine::core::hubo::{Edge2, FlatHuboModel, HuboModel};
use ising_engine::solver::engine::{calculate_replica_energies, step, StepScratch};
use ising_engine::solver::types::{QuantumField, NUM_REPLICAS};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn random_pairwise(n: usize, rng: &mut ChaCha8Rng) -> FlatHuboModel {
    let mut hubo = HuboModel::new(n);
    for i in 0..n {
        hubo.linear[i] = rng.gen_range(-1.0..1.0);
        for j in (i + 1)..n {
            let w = rng.gen_range(-1.0..1.0);
            hubo.edges2[i].push(Edge2 { j, weight: w });
            hubo.edges2[j].push(Edge2 { j: i, weight: w });
        }
    }
    FlatHuboModel::from_hubo(&hubo)
}

fn run_tracking(seed: u64) -> (Vec<f64>, u64) {
    let n = 20;
    let nt = 8;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let flat = random_pairwise(n, &mut rng);
    let mut field = QuantumField::new(n, 1, nt, 1);
    for t in 0..nt {
        for v in 0..n {
            for r in 0..NUM_REPLICAS {
                field.set_replica(v, 0, t, 0, r, rng.gen_range(0..=1));
            }
        }
    }
    for t in 0..nt {
        field.energies[t] = calculate_replica_energies(&flat, &field, t, 0, 1.0);
    }
    // Geometric ladder.
    let (tmax, tmin) = (5.0f64, 0.05f64);
    let f = (tmin / tmax).powf(1.0 / (nt - 1) as f64);
    let temps: Vec<f64> = (0..nt).map(|t| tmax * f.powi(t as i32)).collect();
    let no_clamps = vec![false; n];

    let mut scratch = StepScratch::for_field(&field);
    scratch.enable_roundtrip_tracking();
    for _ in 0..400 {
        step(
            &mut field,
            &flat,
            &temps,
            1.0,
            &no_clamps,
            &mut scratch,
            &mut rng,
        );
    }
    (scratch.roundtrip_flow(), scratch.roundtrip_count())
}

#[test]
fn flow_is_monotone_across_the_ladder() {
    let (flow, rt) = run_tracking(1);
    // By construction the hot end (t=0) is labeled down (f≈0) and the cold
    // end (t=nt-1) up (f≈1); the profile must be non-decreasing overall.
    assert!(flow[0] < 0.5, "hot-end flow should be low, got {}", flow[0]);
    assert!(
        *flow.last().unwrap() > 0.5,
        "cold-end flow should be high, got {}",
        flow.last().unwrap()
    );
    // Monotone up to small sampling noise (allow tiny non-monotonic dips).
    let violations = flow.windows(2).filter(|w| w[1] < w[0] - 0.05).count();
    assert!(
        violations <= 1,
        "flow should be ~monotone increasing, got {:?}",
        flow
    );
    // Some round trips must have completed over 400 steps.
    assert!(rt > 0, "no round trips counted");
}

#[test]
fn tracking_is_deterministic() {
    let a = run_tracking(42);
    let b = run_tracking(42);
    assert_eq!(a.1, b.1);
    assert_eq!(a.0.len(), b.0.len());
    for (x, y) in a.0.iter().zip(&b.0) {
        assert!((x - y).abs() < 1e-12);
    }
}

/// With tracking disabled, the field evolves identically to a run that never
/// had tracking — instrumentation must not perturb the dynamics.
#[test]
fn tracking_disabled_does_not_change_dynamics() {
    let n = 12;
    let nt = 6;
    let mut rng0 = ChaCha8Rng::seed_from_u64(7);
    let flat = random_pairwise(n, &mut rng0);
    let build = || {
        let mut r = ChaCha8Rng::seed_from_u64(123);
        let mut f = QuantumField::new(n, 1, nt, 1);
        for t in 0..nt {
            for v in 0..n {
                for lane in 0..NUM_REPLICAS {
                    f.set_replica(v, 0, t, 0, lane, r.gen_range(0..=1));
                }
            }
        }
        for t in 0..nt {
            f.energies[t] = calculate_replica_energies(&flat, &f, t, 0, 1.0);
        }
        f
    };
    let temps: Vec<f64> = (0..nt).map(|t| 5.0 * 0.5f64.powi(t as i32)).collect();
    let no_clamps = vec![false; n];

    let mut f1 = build();
    let mut s1 = StepScratch::for_field(&f1);
    let mut rng1 = ChaCha8Rng::seed_from_u64(99);
    for _ in 0..50 {
        step(&mut f1, &flat, &temps, 1.0, &no_clamps, &mut s1, &mut rng1);
    }

    let mut f2 = build();
    let mut s2 = StepScratch::for_field(&f2);
    s2.enable_roundtrip_tracking(); // enabled — must not alter spins/RNG
    let mut rng2 = ChaCha8Rng::seed_from_u64(99);
    for _ in 0..50 {
        step(&mut f2, &flat, &temps, 1.0, &no_clamps, &mut s2, &mut rng2);
    }

    assert_eq!(f1.spins, f2.spins, "tracking must not perturb the dynamics");
}
