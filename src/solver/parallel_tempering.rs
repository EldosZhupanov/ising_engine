use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::f64::consts::E;

use super::replica::{build_clamped_set, Replica};
use crate::core::QuboModel;

/// Base Parallel Tempering solver with Metropolis-Hastings single-spin flips.
///
/// Uses logarithmically-spaced temperature replicas with replica exchange.
/// This is the standard solver used by most binaries.
pub struct ParallelTemperingSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub total_exchanges: usize,
    pub seed: Option<u64>,
}

impl ParallelTemperingSolver {
    /// Incremental energy change from flipping variable `var_idx`.
    ///
    /// δE = (1 - 2·x_i) · (h_i + Σ_j J_ij·x_j)
    fn calculate_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
        let current_val = state[var_idx] as f64;
        let flip_multiplier = 1.0 - 2.0 * current_val;
        let mut sum_j = 0.0;
        for (col, weight) in model.quadratic.get_row(var_idx) {
            sum_j += weight * (state[col] as f64);
        }
        flip_multiplier * (model.linear[var_idx] + sum_j)
    }

    /// Solves the QUBO problem with clamped (fixed) variables.
    pub fn solve(&self, model: &QuboModel, clamped: &[(usize, i8)]) -> Vec<i8> {
        let base_seed = self.seed.unwrap_or_else(rand::random);
        let mut engine_rng = ChaCha8Rng::seed_from_u64(base_seed + self.num_replicas as u64);
        
        let mut replica_rngs: Vec<ChaCha8Rng> = (0..self.num_replicas)
            .map(|i| ChaCha8Rng::seed_from_u64(base_seed + i as u64))
            .collect();

        let clamped_set = build_clamped_set(model.num_vars, clamped);

        // Initialize replicas with logarithmic temperature distribution
        let mut replicas: Vec<Replica> = replica_rngs
            .iter_mut()
            .enumerate()
            .map(|(i, local_rng)| {
                let mut state: Vec<i8> =
                    (0..model.num_vars).map(|_| local_rng.gen_range(0..=1)).collect();
                for &(idx, val) in clamped {
                    state[idx] = val;
                }
                let fraction = i as f64 / (self.num_replicas - 1).max(1) as f64;
                let temp = self.temp_max * (self.temp_min / self.temp_max).powf(fraction);
                let energy = model.calculate_total_energy(&state);
                Replica {
                    state,
                    temp,
                    energy,
                }
            })
            .collect();

        for _ in 0..self.total_exchanges {
            // Metropolis-Hastings sweep phase
            for (replica, local_rng) in replicas.iter_mut().zip(replica_rngs.iter_mut()) {
                for _ in 0..self.sweeps_per_exchange {
                    let var_idx = local_rng.gen_range(0..model.num_vars);
                    if clamped_set[var_idx] {
                        continue;
                    }

                    let delta_e = Self::calculate_delta_e(model, &replica.state, var_idx);

                    let exponent = -delta_e / replica.temp;
                    if delta_e < 0.0
                        || (replica.temp > 1e-8
                            && exponent > -20.0
                            && local_rng.gen_range(0.0..1.0) < E.powf(exponent))
                    {
                        replica.state[var_idx] = 1 - replica.state[var_idx];
                    }
                }
                replica.energy = model.calculate_total_energy(&replica.state);
            }

            // Replica exchange phase
            for i in 0..(self.num_replicas - 1) {
                let beta_i = 1.0 / replicas[i].temp;
                let beta_j = 1.0 / replicas[i + 1].temp;
                let delta_beta = beta_i - beta_j;
                let delta_energy = replicas[i].energy - replicas[i + 1].energy;

                let swap_prob = (delta_beta * delta_energy).exp();

                if swap_prob >= 1.0 || engine_rng.gen_range(0.0..1.0) < swap_prob {
                    let (left, right) = replicas.split_at_mut(i + 1);
                    let ri = &mut left[i];
                    let rj = &mut right[0];
                    std::mem::swap(&mut ri.state, &mut rj.state);

                    // Re-apply clamped values after swap
                    for &(idx, val) in clamped {
                        ri.state[idx] = val;
                        rj.state[idx] = val;
                    }

                    std::mem::swap(&mut ri.energy, &mut rj.energy);
                }
            }
        }

        // Return the state with minimum energy
        replicas
            .into_iter()
            .min_by(|a, b| a.energy.partial_cmp(&b.energy).unwrap())
            .unwrap()
            .state
    }
}
