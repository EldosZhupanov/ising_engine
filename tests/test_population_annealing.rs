//! Contract tests for Population Annealing (Machta, PRE 82, 026704 (2010);
//! Wang, Machta & Katzgraber, PRE 92, 063307 (2015)).
//!
//! Unit level: Boltzmann weighting, log-sum-exp stability, ESS, systematic
//! resampling (size conservation, per-count variance bound, unbiasedness),
//! and spin/energy synchronization of the resampling application.
//! Integration level: determinism given a seed, clamp preservation, and
//! exhaustive-optimum quality of the PA mode (num_pops > 1).

use ising_engine::core::hubo::{Edge2, FlatHuboModel, HuboModel};
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::engine;
use ising_engine::solver::population_annealing::{
    apply_resample, effective_sample_size, normalized_weights, systematic_resample,
};
use ising_engine::solver::types::{QuantumField, NUM_REPLICAS};
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

// ========================================================================
// Weights
// ========================================================================

#[test]
fn weights_follow_boltzmann_ratios() {
    // w_i ∝ exp(−Δβ·E_i): for E = [0, 1, 2] and Δβ = 0.7, successive weight
    // ratios must equal exp(−0.7) exactly (up to float rounding).
    let delta_beta = 0.7;
    let energies = [0.0, 1.0, 2.0];
    let log_w: Vec<f64> = energies.iter().map(|e| -delta_beta * e).collect();
    let w = normalized_weights(&log_w);

    let sum: f64 = w.iter().sum();
    assert!((sum - 1.0).abs() < 1e-12, "weights must normalize to 1");
    let expected_ratio = (-delta_beta).exp();
    assert!(((w[1] / w[0]) - expected_ratio).abs() < 1e-12);
    assert!(((w[2] / w[1]) - expected_ratio).abs() < 1e-12);
}

#[test]
fn lse_normalization_is_stable_at_extreme_energies() {
    // Naive exp(−Δβ·E) overflows/underflows at |E| ~ 1e6; log-sum-exp must
    // return finite, normalized weights.
    let log_w = [-2.0e6, -2.0e6 + 1.0, -3.0e6];
    let w = normalized_weights(&log_w);
    assert!(w.iter().all(|x| x.is_finite()));
    let sum: f64 = w.iter().sum();
    assert!((sum - 1.0).abs() < 1e-12);
    // The −3e6 member is astronomically suppressed; the two close members
    // keep the exact ratio: log-weight −2e6 vs −2e6+1 → w[0]/w[1] = e^{−1}.
    assert!(w[2] < 1e-300);
    assert!(((w[0] / w[1]) - (-1.0f64).exp()).abs() < 1e-9);
}

// ========================================================================
// ESS
// ========================================================================

#[test]
fn ess_bounds_and_exact_values() {
    // Uniform weights: ESS = R.
    let r = 64;
    let uniform = vec![1.0 / r as f64; r];
    assert!((effective_sample_size(&uniform) - r as f64).abs() < 1e-9);

    // Fully degenerate: ESS = 1.
    let mut degenerate = vec![0.0; r];
    degenerate[3] = 1.0;
    assert!((effective_sample_size(&degenerate) - 1.0).abs() < 1e-12);

    // Two equal survivors: ESS = 2.
    let mut two = vec![0.0; r];
    two[0] = 0.5;
    two[1] = 0.5;
    assert!((effective_sample_size(&two) - 2.0).abs() < 1e-12);
}

// ========================================================================
// Systematic resampling
// ========================================================================

fn example_weights(r: usize, seed: u64) -> Vec<f64> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let raw: Vec<f64> = (0..r).map(|_| rng.gen_range(0.01..1.0)).collect();
    let s: f64 = raw.iter().sum();
    raw.into_iter().map(|x| x / s).collect()
}

#[test]
fn systematic_resample_conserves_size_and_bounds_counts() {
    let r = 128;
    let w = example_weights(r, 5);
    for m in 0..50 {
        let u = (m as f64 + 0.5) / 50.0;
        let counts = systematic_resample(&w, u);
        assert_eq!(
            counts.iter().sum::<usize>(),
            r,
            "population size must be conserved"
        );
        for (i, &c) in counts.iter().enumerate() {
            assert!(
                (c as f64 - r as f64 * w[i]).abs() < 1.0 + 1e-9,
                "systematic count {} deviates from R·w = {} by ≥ 1",
                c,
                r as f64 * w[i]
            );
        }
    }
}

#[test]
fn systematic_resample_is_unbiased_over_the_offset() {
    // E_u[count_i] = R·w_i exactly for u ~ U[0,1); a fine midpoint grid over
    // u must reproduce it to grid accuracy.
    let r = 64;
    let w = example_weights(r, 9);
    let m_grid = 2000;
    let mut mean = vec![0.0f64; r];
    for m in 0..m_grid {
        let u = (m as f64 + 0.5) / m_grid as f64;
        let counts = systematic_resample(&w, u);
        for (i, &c) in counts.iter().enumerate() {
            mean[i] += c as f64;
        }
    }
    for x in mean.iter_mut() {
        *x /= m_grid as f64;
    }
    for i in 0..r {
        assert!(
            (mean[i] - r as f64 * w[i]).abs() < 2e-3,
            "resampling biased at parent {}: mean {} vs R·w {}",
            i,
            mean[i],
            r as f64 * w[i]
        );
    }
}

// ========================================================================
// Resampling application: spins and energies stay synchronized
// ========================================================================

#[test]
fn apply_resample_keeps_energies_synchronized_with_spins() {
    // PA-PT hybrid member = lane r of population p across the WHOLE ladder:
    // 3 vars, 2 slices, 2 temperatures, 2 populations → R = 128 members.
    let n = 3;
    let mut rng = ChaCha8Rng::seed_from_u64(31);
    let mut hubo = HuboModel::new(n);
    for i in 0..n {
        hubo.linear[i] = rng.gen_range(-1.0..1.0);
        for j in (i + 1)..n {
            let w = rng.gen_range(-1.0..1.0);
            hubo.edges2[i].push(Edge2 { j, weight: w });
            hubo.edges2[j].push(Edge2 { j: i, weight: w });
        }
    }
    let flat = FlatHuboModel::from_hubo(&hubo);

    let nt = 2;
    let mut field = QuantumField::new(n, 2, nt, 2);
    for p in 0..2 {
        for t in 0..nt {
            for s in 0..2 {
                for v in 0..n {
                    for r in 0..NUM_REPLICAS {
                        field.set_replica(v, s, t, p, r, rng.gen_range(0..=1));
                    }
                }
            }
        }
    }
    let j_tau = 0.0;
    for p in 0..2 {
        for t in 0..nt {
            field.energies[t + nt * p] =
                engine::calculate_replica_energies(&flat, &field, t, p, j_tau);
        }
    }

    // A genuinely uneven plan: weights from the members' COLDEST-level
    // Boltzmann factors (the level whose β advances between stages).
    let r_total = 2 * NUM_REPLICAS;
    let cold = nt - 1;
    let mut log_w = Vec::with_capacity(r_total);
    for p in 0..2 {
        for r in 0..NUM_REPLICAS {
            log_w.push(-0.8 * field.energies[cold + nt * p][r]);
        }
    }
    let w = normalized_weights(&log_w);
    let counts = systematic_resample(&w, 0.37);
    assert_eq!(counts.iter().sum::<usize>(), r_total);

    apply_resample(&mut field, &counts, &mut rng);

    // Every member's tracked energy at EVERY temperature level must equal a
    // full recomputation of its (copied) configuration at that level.
    for p in 0..2 {
        for t in 0..nt {
            let recomputed = engine::calculate_replica_energies(&flat, &field, t, p, j_tau);
            for (r, &expected) in recomputed.iter().enumerate() {
                assert!(
                    (field.energies[t + nt * p][r] - expected).abs() < 1e-9,
                    "member (p={}, r={}) desynchronized at level {} after resampling",
                    p,
                    r,
                    t
                );
            }
        }
    }
}

// ========================================================================
// Integration: PA mode of UltimateSolver
// ========================================================================

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

fn brute_force_optimum(model: &QuboModel, fixed: &[(usize, i8)]) -> f64 {
    let n = model.num_vars;
    assert!(n <= 16);
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

fn pa_solver(seed: u64) -> UltimateSolver {
    let mut s = UltimateSolver::new(20.0, 0.05, 20, 40, Some(seed));
    s.num_pops = 2; // PA mode: population of 2 × 64 members
    s
}

#[test]
fn pa_mode_is_deterministic_given_seed() {
    let mut rng = ChaCha8Rng::seed_from_u64(51);
    let model = random_sparse_qubo(12, 0.4, &mut rng);
    let s1 = pa_solver(777).solve(&model, &[]);
    let s2 = pa_solver(777).solve(&model, &[]);
    assert_eq!(s1, s2, "PA mode must be deterministic given a seed");
}

#[test]
fn pa_mode_finds_exhaustive_optimum() {
    let mut rng = ChaCha8Rng::seed_from_u64(53);
    for trial in 0..5 {
        let model = random_sparse_qubo(10, 0.4, &mut rng);
        let e_opt = brute_force_optimum(&model, &[]);
        let state = pa_solver(600 + trial).solve(&model, &[]);
        let e_found = model.calculate_total_energy(&state);
        assert!(
            (e_found - e_opt).abs() < 1e-9,
            "trial {}: PA returned {} but optimum is {}",
            trial,
            e_found,
            e_opt
        );
    }
}

#[test]
fn pa_mode_honors_clamps() {
    let mut rng = ChaCha8Rng::seed_from_u64(57);
    let model = random_sparse_qubo(12, 0.4, &mut rng);
    let clamps = vec![(0usize, 1i8), (5, 0i8)];
    let state = pa_solver(4242).solve(&model, &clamps);
    assert_eq!(state[0], 1, "clamp x0=1 violated in PA mode");
    assert_eq!(state[5], 0, "clamp x5=0 violated in PA mode");
}

// ========================================================================
// Adaptive ladder (acceptance uniformization)
// ========================================================================

use ising_engine::solver::engine::StepScratch;
use ising_engine::solver::population_annealing::adapt_spacings;

#[test]
fn adapt_spacings_is_fixed_point_at_uniform_acceptance() {
    let spacings = [0.1, 0.3, 0.2, 0.4];
    let uniform = [0.3; 4];
    let out = adapt_spacings(&spacings, &uniform);
    for (a, b) in out.iter().zip(&spacings) {
        assert!(
            (a - b).abs() < 1e-12,
            "uniform acceptance must be a fixed point"
        );
    }
}

#[test]
fn adapt_spacings_narrows_low_acceptance_pairs() {
    // Equal spacings, one starved pair: it must narrow, the other widen,
    // total span preserved.
    let spacings = [0.5, 0.5];
    let rates = [0.9, 0.1];
    let out = adapt_spacings(&spacings, &rates);
    assert!(out[0] > 0.5 && out[1] < 0.5);
    assert!((out.iter().sum::<f64>() - 1.0).abs() < 1e-12);
}

/// Swap statistics must report exact Metropolis exchange rates. Frozen
/// (fully clamped) two-level fields: with the LOW energy at the hot level,
/// the exchange exponent is positive → every lane swaps (rate 1.0); with the
/// arrangement reversed, exp(Δβ·ΔE) = e^{-15} → rate ≈ 0.
#[test]
fn swap_statistics_measure_exact_rates() {
    let mut hubo = HuboModel::new(1);
    hubo.linear[0] = 10.0;
    let flat = FlatHuboModel::from_hubo(&hubo);
    let temps = vec![2.0, 0.5];
    let clamped = vec![true];

    // Case 1: hot x=0 (E=0), cold x=1 (E=10) → always accepted.
    let mut field = QuantumField::new(1, 1, 2, 1);
    for r in 0..NUM_REPLICAS {
        field.set_replica(0, 0, 0, 0, r, 0);
        field.set_replica(0, 0, 1, 0, r, 1);
    }
    for t in 0..2 {
        field.energies[t] = engine::calculate_replica_energies(&flat, &field, t, 0, 0.0);
    }
    let mut scratch = StepScratch::for_field(&field);
    scratch.enable_swap_stats();
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    engine::step(
        &mut field,
        &flat,
        &temps,
        0.0,
        &clamped,
        &mut scratch,
        &mut rng,
    );
    let rates = scratch.swap_acceptance_rates();
    assert!(
        (rates[0] - 1.0).abs() < 1e-12,
        "favorable swap must have rate 1.0"
    );

    // Case 2: hot x=1 (E=10), cold x=0 (E=0) → acceptance e^{-15} ≈ 3e-7.
    let mut field = QuantumField::new(1, 1, 2, 1);
    for r in 0..NUM_REPLICAS {
        field.set_replica(0, 0, 0, 0, r, 1);
        field.set_replica(0, 0, 1, 0, r, 0);
    }
    for t in 0..2 {
        field.energies[t] = engine::calculate_replica_energies(&flat, &field, t, 0, 0.0);
    }
    let mut scratch = StepScratch::for_field(&field);
    scratch.enable_swap_stats();
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    engine::step(
        &mut field,
        &flat,
        &temps,
        0.0,
        &clamped,
        &mut scratch,
        &mut rng,
    );
    let rates = scratch.swap_acceptance_rates();
    assert!(
        rates[0] < 0.01,
        "unfavorable swap rate must be ≈ 0, got {}",
        rates[0]
    );
}

/// End-to-end tuning behavior via the public pieces (field + swap stats +
/// adapt_spacings): iterating the acceptance-uniformization update on a real
/// dense instance must reduce the spread (max − min) of per-pair acceptance
/// rates relative to the initial geometric ladder.
#[test]
fn acceptance_uniformization_reduces_rate_spread() {
    let n = 16;
    let mut rng = ChaCha8Rng::seed_from_u64(101);
    let mut hubo = HuboModel::new(n);
    for i in 0..n {
        hubo.linear[i] = rng.gen_range(-1.0..1.0);
        for j in (i + 1)..n {
            let w = rng.gen_range(-2.0..2.0);
            hubo.edges2[i].push(Edge2 { j, weight: w });
            hubo.edges2[j].push(Edge2 { j: i, weight: w });
        }
    }
    let flat = FlatHuboModel::from_hubo(&hubo);

    let nt = 6;
    let (t_hot, t_cold) = (5.0f64, 0.05f64);
    let mut field = QuantumField::new(n, 1, nt, 1);
    for t in 0..nt {
        for v in 0..n {
            for r in 0..NUM_REPLICAS {
                field.set_replica(v, 0, t, 0, r, rng.gen_range(0..=1));
            }
        }
    }
    for t in 0..nt {
        field.energies[t] = engine::calculate_replica_energies(&flat, &field, t, 0, 0.0);
    }
    let mut scratch = StepScratch::for_field(&field);
    scratch.enable_swap_stats();
    let no_clamps = vec![false; n];

    let mut alphas: Vec<f64> = (0..nt).map(|t| t as f64 / (nt - 1) as f64).collect();
    let spread = |rates: &[f64]| {
        rates.iter().cloned().fold(f64::MIN, f64::max)
            - rates.iter().cloned().fold(f64::MAX, f64::min)
    };

    let measure = |alphas: &[f64],
                   field: &mut QuantumField,
                   scratch: &mut StepScratch,
                   rng: &mut ChaCha8Rng| {
        let temps: Vec<f64> = alphas
            .iter()
            .map(|&a| t_hot * (t_cold / t_hot).powf(a))
            .collect();
        scratch.reset_swap_stats();
        for _ in 0..25 {
            engine::step(field, &flat, &temps, 0.0, &no_clamps, scratch, rng);
        }
        scratch.swap_acceptance_rates()
    };

    let initial_rates = measure(&alphas, &mut field, &mut scratch, &mut rng);
    let initial_spread = spread(&initial_rates);

    let mut rates = initial_rates;
    for _ in 0..10 {
        let spacings: Vec<f64> = alphas.windows(2).map(|w| w[1] - w[0]).collect();
        let new = adapt_spacings(&spacings, &rates);
        let mut acc = 0.0;
        for (t, d) in new.iter().enumerate() {
            acc += d;
            alphas[t + 1] = acc;
        }
        alphas[nt - 1] = 1.0;
        rates = measure(&alphas, &mut field, &mut scratch, &mut rng);
    }
    let final_spread = spread(&rates);

    assert!(
        final_spread < initial_spread,
        "acceptance spread must shrink under tuning: initial {:.3}, final {:.3}",
        initial_spread,
        final_spread
    );
}

// ========================================================================
// Post-resampling relaxation
// ========================================================================

/// After a total-collapse resample (all offspring cloned from one member),
/// PT relaxation steps must decorrelate the clones: member configurations
/// diverge again.
#[test]
fn pt_steps_decorrelate_cloned_members() {
    let n = 8;
    let mut rng = ChaCha8Rng::seed_from_u64(211);
    let mut hubo = HuboModel::new(n);
    for i in 0..n {
        hubo.linear[i] = rng.gen_range(-1.0..1.0);
        for j in (i + 1)..n {
            let w = rng.gen_range(-1.0..1.0);
            hubo.edges2[i].push(Edge2 { j, weight: w });
            hubo.edges2[j].push(Edge2 { j: i, weight: w });
        }
    }
    let flat = FlatHuboModel::from_hubo(&hubo);

    let nt = 2;
    let mut field = QuantumField::new(n, 1, nt, 2);
    for p in 0..2 {
        for t in 0..nt {
            for v in 0..n {
                for r in 0..NUM_REPLICAS {
                    field.set_replica(v, 0, t, p, r, rng.gen_range(0..=1));
                }
            }
        }
    }
    for p in 0..2 {
        for t in 0..nt {
            field.energies[t + nt * p] =
                engine::calculate_replica_energies(&flat, &field, t, p, 0.0);
        }
    }

    // Total collapse: every offspring is a clone of member 0.
    let r_total = 2 * NUM_REPLICAS;
    let mut counts = vec![0usize; r_total];
    counts[0] = r_total;
    apply_resample(&mut field, &counts, &mut rng);

    let member_config = |field: &QuantumField, p: usize, r: usize| -> Vec<i8> {
        let mut cfg = Vec::new();
        for t in 0..nt {
            for v in 0..n {
                cfg.push(field.get_replica(v, 0, t, p, r));
            }
        }
        cfg
    };
    let reference = member_config(&field, 0, 0);
    for p in 0..2 {
        for r in 0..NUM_REPLICAS {
            assert_eq!(
                member_config(&field, p, r),
                reference,
                "all members must be identical clones right after total collapse"
            );
        }
    }

    // Relaxation: a few full-PT steps must re-diversify the population.
    let mut scratch = StepScratch::for_field(&field);
    let no_clamps = vec![false; n];
    let temps = vec![2.0, 1.0];
    for _ in 0..3 {
        engine::step(
            &mut field,
            &flat,
            &temps,
            0.0,
            &no_clamps,
            &mut scratch,
            &mut rng,
        );
    }
    let reference = member_config(&field, 0, 0);
    let diverged = (0..2)
        .flat_map(|p| (0..NUM_REPLICAS).map(move |r| (p, r)))
        .any(|(p, r)| member_config(&field, p, r) != reference);
    assert!(
        diverged,
        "PT relaxation steps failed to decorrelate cloned members"
    );
}

/// The relaxation path must actually execute inside the solver and alter the
/// trajectory: with resampling forced every stage (θ > 1) and a minimal
/// budget, runs differing only in `post_resample_relaxation` diverge, and
/// the relaxed run still reaches the exhaustive optimum.
#[test]
fn solver_relaxation_executes_and_preserves_quality() {
    let mut rng = ChaCha8Rng::seed_from_u64(223);
    // Probe design note: output-difference probes fail here for a physical
    // reason — after a forced resample the population collapses onto clones,
    // and near-greedy cold sweeps are CONFLUENT (descent from identical
    // states reaches identical minima for 1 or 9 sweeps). The honest
    // observable: with stage sweeps set to ZERO, relaxation is the ONLY
    // equilibration, so it must STRICTLY improve the returned energy over
    // the no-relaxation run (which can only return best-of-random-init).
    let model = random_sparse_qubo(20, 1.0, &mut rng);

    let configure = |relaxation: usize| {
        let mut s = UltimateSolver::new(20.0, 0.05, 0 /* stage sweeps */, 2, Some(31337));
        s.num_pops = 2;
        s.num_temps = 3;
        s.ess_threshold = 1.1; // ESS ≤ R < θ·R always → resample every stage
        s.ladder_tuning_iters = 0; // isolate the relaxation effect
        s.post_resample_relaxation = relaxation;
        s
    };
    let e_without = model.calculate_total_energy(&configure(0).solve(&model, &[]));
    let e_with = model.calculate_total_energy(&configure(8).solve(&model, &[]));

    assert!(
        e_with < e_without,
        "post-resampling relaxation must execute and improve the result when \
         it is the only equilibration: with = {}, without = {}",
        e_with,
        e_without
    );

    // Quality: on a brute-forceable instance, a full-budget run with
    // relaxation enabled must reach the exhaustive optimum.
    let model12 = random_sparse_qubo(12, 1.0, &mut rng);
    let e_opt = {
        let mut best = f64::INFINITY;
        for bits in 0..(1u32 << 12) {
            let state: Vec<i8> = (0..12).map(|v| ((bits >> v) & 1) as i8).collect();
            best = best.min(model12.calculate_total_energy(&state));
        }
        best
    };
    let mut full = UltimateSolver::new(20.0, 0.05, 20, 40, Some(31337));
    full.num_pops = 2;
    let state = full.solve(&model12, &[]);
    assert!(
        (model12.calculate_total_energy(&state) - e_opt).abs() < 1e-9,
        "PA-PT with relaxation failed to reach the optimum"
    );
}

// ========================================================================
// PA diagnostics (free energy, family entropy, ESS evolution)
// ========================================================================

use ising_engine::solver::population_annealing::{family_entropy, free_energy_increment};

#[test]
fn free_energy_increment_matches_closed_form() {
    // Uniform log-weights lw = c: mean reweighting factor = e^c, so
    // Δ(−βF) = c exactly, independent of R.
    let lw = vec![0.7; 64];
    assert!((free_energy_increment(&lw) - 0.7).abs() < 1e-12);

    // Two-member explicit case: lw = [0, ln 3]; mean factor = (1+3)/2 = 2.
    let lw2 = vec![0.0, 3.0_f64.ln()];
    assert!((free_energy_increment(&lw2) - 2.0_f64.ln()).abs() < 1e-12);

    // Numerically stable at extreme magnitudes.
    let big = vec![-1.0e6, -1.0e6];
    assert!((free_energy_increment(&big) - (-1.0e6)).abs() < 1e-3);
}

#[test]
fn family_entropy_bounds() {
    // All distinct families: ρ = R·Σ(1/R)² = R·R·(1/R²) = 1.
    let all_distinct: Vec<usize> = (0..64).collect();
    let (rho, fams) = family_entropy(&all_distinct);
    assert!((rho - 1.0).abs() < 1e-12);
    assert_eq!(fams, 64);

    // Single family: ρ = R·(R/R)² = R.
    let one = vec![7usize; 64];
    let (rho, fams) = family_entropy(&one);
    assert!((rho - 64.0).abs() < 1e-12);
    assert_eq!(fams, 1);

    // Two equal families: ρ = 64·2·(0.5)² = 32.
    let two: Vec<usize> = (0..64).map(|i| i % 2).collect();
    let (rho, fams) = family_entropy(&two);
    assert!((rho - 32.0).abs() < 1e-12);
    assert_eq!(fams, 2);
}

#[test]
fn pa_diagnostics_are_populated_and_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(2001);
    let model = random_sparse_qubo(16, 0.5, &mut rng);
    let mut solver = UltimateSolver::new(20.0, 0.05, 5, 20, Some(555));
    solver.num_pops = 3;
    solver.ess_threshold = 0.8; // ensure resampling fires

    let (s1, d1) = solver.solve_with_pa_diagnostics(&model, &[]);
    let (s2, d2) = solver.solve_with_pa_diagnostics(&model, &[]);
    assert_eq!(s1, s2, "PA-diagnostics solve must be deterministic");
    assert_eq!(d1.resample_events, d2.resample_events);
    assert_eq!(d1.ess_history.len(), d2.ess_history.len());
    assert!((d1.free_energy - d2.free_energy).abs() < 1e-12);

    // ESS history recorded once per reweighting stage (all but the first).
    assert!(!d1.ess_history.is_empty(), "ESS history must be recorded");
    // Every ESS lies in [1, R].
    let r_total = 3.0 * 64.0;
    for &ess in &d1.ess_history {
        assert!(
            ess >= 1.0 - 1e-9 && ess <= r_total + 1e-9,
            "ESS out of range: {}",
            ess
        );
    }
    // Family entropy recorded per resampling event, ρ ∈ [1, R], families ≥ 1.
    assert_eq!(d1.family_entropy_history.len(), d1.resample_events);
    for &(rho, fams) in &d1.family_entropy_history {
        assert!(
            rho >= 1.0 - 1e-6 && rho <= r_total + 1e-6,
            "rho out of range: {}",
            rho
        );
        assert!(fams >= 1);
    }
}
