//! EXP-TEN-001: Associative Binary Pattern Completion Experiment Harness.

use crate::dynamics::greedy_descent;
use crate::models::EnergyModel;
use crate::types::SpinState;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Summary metrics for a single experimental condition across all seeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConditionResult {
    pub model_name: String,
    pub n: usize,
    pub p: usize,
    pub noise: f64,
    pub num_parameters: usize,
    pub exact_recovery_rate: f64,
    pub mean_ber: f64,
    pub mean_sweeps: f64,
    pub mean_flips: f64,
    pub mean_wall_time_us: f64,
    /// Capacity efficiency = (P * N * exact_recovery_rate) / num_parameters
    pub capacity_per_parameter: f64,
}

/// Generate P random binary patterns of length N with probability 0.5.
pub fn generate_random_patterns(n: usize, p: usize, seed: u64) -> Vec<SpinState> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }
    patterns
}

/// Corrupt a pattern by independently flipping each bit with probability `noise`.
pub fn corrupt_pattern(pattern: &SpinState, noise: f64, rng: &mut ChaCha8Rng) -> SpinState {
    let mut corrupted = pattern.clone();
    for i in 0..pattern.len() {
        if rng.gen::<f64>() < noise {
            corrupted.flip(i);
        }
    }
    corrupted
}

/// Run a single trial: train model, corrupt each pattern, perform relaxation, compute metrics.
pub fn evaluate_model_on_patterns(
    model: &dyn EnergyModel,
    model_name: &str,
    patterns: &[SpinState],
    noise: f64,
    seed: u64,
    max_sweeps: usize,
) -> ModelConditionResult {
    let n = model.num_spins();
    let p = patterns.len();
    let num_parameters = model.num_parameters();
    let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(9999));

    let mut exact_matches = 0;
    let mut total_ber = 0.0;
    let mut total_sweeps = 0;
    let mut total_flips = 0;
    let mut total_time_us = 0;

    for pat in patterns {
        let probe = corrupt_pattern(pat, noise, &mut rng);
        let res = greedy_descent(model, &probe, max_sweeps);

        if res.final_state == *pat {
            exact_matches += 1;
        }
        total_ber += res.final_state.bit_error_rate(pat);
        total_sweeps += res.sweeps;
        total_flips += res.flips;
        total_time_us += res.wall_time_us;
    }

    let exact_rate = (exact_matches as f64) / (p as f64);
    let mean_ber = total_ber / (p as f64);
    let mean_sweeps = (total_sweeps as f64) / (p as f64);
    let mean_flips = (total_flips as f64) / (p as f64);
    let mean_wall_time_us = (total_time_us as f64) / (p as f64);

    let capacity_per_parameter = if num_parameters > 0 {
        ((p * n) as f64 * exact_rate) / (num_parameters as f64)
    } else {
        0.0
    };

    ModelConditionResult {
        model_name: model_name.to_string(),
        n,
        p,
        noise,
        num_parameters,
        exact_recovery_rate: exact_rate,
        mean_ber,
        mean_sweeps,
        mean_flips,
        mean_wall_time_us,
        capacity_per_parameter,
    }
}
