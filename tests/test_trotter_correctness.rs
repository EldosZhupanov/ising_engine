//! Trotter-coupling correctness for the MSC engine.
//!
//! Suzuki–Trotter mapping (Martoňák, Santoro & Tosatti, PRB 66, 094203 (2002)):
//! the inter-slice coupling J_τ acts on an imaginary-time ring of L slices.
//! At L = 1 the ring degenerates to a self-coupling — a CONSTANT term — so it
//! must contribute exactly zero to every flip's ΔE. For all L, the engine
//! invariant must hold: incrementally tracked energies equal a full
//! recomputation within float tolerance.

use ising_engine::core::hubo::{Edge2, FlatHuboModel, HuboModel};
use ising_engine::solver::engine;
use ising_engine::solver::types::{QuantumField, NUM_REPLICAS};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Random symmetric pairwise HUBO with weights in [-1, 1].
fn random_hubo(n: usize, rng: &mut ChaCha8Rng) -> FlatHuboModel {
    let mut hubo = HuboModel::new(n);
    for i in 0..n {
        hubo.linear[i] = rng.gen_range(-1.0..1.0);
    }
    for i in 0..n {
        for j in (i + 1)..n {
            let w = rng.gen_range(-1.0..1.0);
            hubo.edges2[i].push(Edge2 { j, weight: w });
            hubo.edges2[j].push(Edge2 { j: i, weight: w });
        }
    }
    FlatHuboModel::from_hubo(&hubo)
}

/// Randomize all spins of the field, then sync tracked energies by recomputation.
fn init_random(field: &mut QuantumField, flat: &FlatHuboModel, j_tau: f64, rng: &mut ChaCha8Rng) {
    for p in 0..field.num_pops {
        for t in 0..field.num_temps {
            for s in 0..field.num_slices {
                for v in 0..field.num_vars {
                    for r in 0..NUM_REPLICAS {
                        field.set_replica(v, s, t, p, r, rng.gen_range(0..=1));
                    }
                }
            }
        }
    }
    for p in 0..field.num_pops {
        for t in 0..field.num_temps {
            field.energies[t + field.num_temps * p] =
                engine::calculate_replica_energies(flat, field, t, p, j_tau);
        }
    }
}

/// Max |tracked − recomputed| over all cells and replicas.
fn max_energy_drift(field: &QuantumField, flat: &FlatHuboModel, j_tau: f64) -> f64 {
    let mut worst: f64 = 0.0;
    for p in 0..field.num_pops {
        for t in 0..field.num_temps {
            let recomputed = engine::calculate_replica_energies(flat, field, t, p, j_tau);
            let tracked = field.energies[t + field.num_temps * p];
            for r in 0..NUM_REPLICAS {
                worst = worst.max((tracked[r] - recomputed[r]).abs());
            }
        }
    }
    worst
}

/// At L = 1 the Trotter term is constant, so with a strictly uphill landscape
/// (h_i = +1.5, no couplings) and T ≈ 0, Metropolis must keep the all-zeros
/// ground state: the acceptance probability of ΔE = +1.5 at T = 0.01 is
/// exp(-150) ≈ 10^-66. Any flip means a phantom energy entered the acceptance.
#[test]
fn single_slice_ground_state_stable_at_low_temperature() {
    let n = 16;
    let mut hubo = HuboModel::new(n);
    for v in 0..n {
        hubo.linear[v] = 1.5;
    }
    let flat = FlatHuboModel::from_hubo(&hubo);

    let mut field = QuantumField::new(n, 1, 1, 1); // all-zeros ground state
    let j_tau = 1.0;
    field.energies[0] = engine::calculate_replica_energies(&flat, &field, 0, 0, j_tau);

    let temps = vec![0.01];
    let mut rng = ChaCha8Rng::seed_from_u64(3);

    let no_clamps = vec![false; n];
    let mut scratch = engine::StepScratch::for_field(&field);
    for step_idx in 0..5 {
        engine::step(
            &mut field,
            &flat,
            &temps,
            j_tau,
            &no_clamps,
            &mut scratch,
            &mut rng,
        );
        let ones: usize = field.spins.iter().map(|&s| s as usize).sum();
        assert_eq!(
            ones, 0,
            "step {}: {} spins left the ground state at T=0.01 — phantom Trotter \
             energy is entering the Metropolis acceptance",
            step_idx, ones
        );
    }
}

/// Engine invariant at L = 1: tracked energies must match full recomputation.
#[test]
fn tracked_energy_matches_recomputation_single_slice() {
    let mut rng = ChaCha8Rng::seed_from_u64(17);
    let flat = random_hubo(12, &mut rng);
    let j_tau = 1.0;
    let mut field = QuantumField::new(12, 1, 2, 1);
    init_random(&mut field, &flat, j_tau, &mut rng);

    let temps = vec![2.0, 0.5];
    let no_clamps = vec![false; 12];
    let mut scratch = engine::StepScratch::for_field(&field);
    for _ in 0..50 {
        engine::step(
            &mut field,
            &flat,
            &temps,
            j_tau,
            &no_clamps,
            &mut scratch,
            &mut rng,
        );
    }

    let drift = max_energy_drift(&field, &flat, j_tau);
    assert!(
        drift < 1e-6,
        "tracked energy drifted {} from recomputation at num_slices=1",
        drift
    );
}

/// The same invariant must hold for L = 2 and L = 3 (ring with a doubled bond
/// at L = 2 is the standard PIMC convention and delta counts it identically).
/// This documents that the multi-slice path was and stays consistent.
#[test]
fn tracked_energy_matches_recomputation_multi_slice() {
    for num_slices in [2usize, 3] {
        let mut rng = ChaCha8Rng::seed_from_u64(23 + num_slices as u64);
        let flat = random_hubo(10, &mut rng);
        let j_tau = 1.0;
        let mut field = QuantumField::new(10, num_slices, 2, 1);
        init_random(&mut field, &flat, j_tau, &mut rng);

        let temps = vec![2.0, 0.5];
        let no_clamps = vec![false; 10];
        let mut scratch = engine::StepScratch::for_field(&field);
        for _ in 0..50 {
            engine::step(
                &mut field,
                &flat,
                &temps,
                j_tau,
                &no_clamps,
                &mut scratch,
                &mut rng,
            );
        }

        let drift = max_energy_drift(&field, &flat, j_tau);
        assert!(
            drift < 1e-6,
            "tracked energy drifted {} from recomputation at num_slices={}",
            drift,
            num_slices
        );
    }
}

/// The tracked-energy invariant must also hold when population resampling
/// is active (num_pops > 1): a replica that receives another population's
/// spins must also carry that state's energy, otherwise PT swap decisions
/// run on stale values until the next re-sync.
#[test]
fn tracked_energy_matches_recomputation_multi_population() {
    for num_pops in [2usize, 3] {
        let mut rng = ChaCha8Rng::seed_from_u64(2027 + num_pops as u64);
        let flat = random_hubo(10, &mut rng);
        let j_tau = 1.0;
        let mut field = QuantumField::new(10, 1, 2, num_pops);
        init_random(&mut field, &flat, j_tau, &mut rng);

        let temps = vec![2.0, 0.5];
        let no_clamps = vec![false; 10];
        let mut scratch = engine::StepScratch::for_field(&field);
        for _ in 0..50 {
            engine::step(
                &mut field,
                &flat,
                &temps,
                j_tau,
                &no_clamps,
                &mut scratch,
                &mut rng,
            );
        }

        let drift = max_energy_drift(&field, &flat, j_tau);
        assert!(
            drift < 1e-6,
            "tracked energy drifted {} from recomputation at num_pops={} — \
             resampling desynchronized energies from spins",
            drift,
            num_pops
        );
    }
}

/// At L = 1 the (degenerate) Trotter term must not appear in reported
/// energies at all: the classical all-zeros state has energy exactly 0.
#[test]
fn single_slice_energy_has_no_trotter_constant() {
    let n = 8;
    let hubo = HuboModel::new(n); // all-zero Hamiltonian
    let flat = FlatHuboModel::from_hubo(&hubo);
    let field = QuantumField::new(n, 1, 1, 1);
    let energies = engine::calculate_replica_energies(&flat, &field, 0, 0, 1.0);
    assert!(
        energies[0].abs() < 1e-12,
        "zero Hamiltonian, zero state, but energy = {} — Trotter constant leaked \
         into single-slice energies",
        energies[0]
    );
}
