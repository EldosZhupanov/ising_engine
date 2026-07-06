use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use super::replica::{build_clamped_set, Replica};
use crate::core::QuboModel;

/// Adaptive Parallel Tempering solver with runtime temperature optimization.
///
/// Tracks swap acceptance rates between adjacent replicas and adjusts
/// temperatures to target an optimal acceptance rate (~0.23).
pub struct AdaptiveTemperingSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub total_exchanges: usize,
    /// How often to adjust temperatures (in exchange steps).
    pub adaptation_interval: usize,
    pub seed: Option<u64>,
}

impl AdaptiveTemperingSolver {
    fn calculate_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
        let flip_multiplier = 1.0 - 2.0 * (state[var_idx] as f64);
        let mut sum_j = 0.0;
        for (col, weight) in model.quadratic.get_row(var_idx) {
            sum_j += weight * (state[col] as f64);
        }
        flip_multiplier * (model.linear[var_idx] + sum_j)
    }

    pub fn solve(&self, model: &QuboModel, clamped: &[(usize, i8)]) -> Vec<i8> {
        let base_seed = self.seed.unwrap_or_else(rand::random);
        let mut engine_rng = ChaCha8Rng::seed_from_u64(base_seed + self.num_replicas as u64);

        let mut replica_rngs: Vec<ChaCha8Rng> = (0..self.num_replicas)
            .map(|i| ChaCha8Rng::seed_from_u64(base_seed + i as u64))
            .collect();

        let clamped_set = build_clamped_set(model.num_vars, clamped);

        let mut replicas: Vec<Replica> = replica_rngs
            .iter_mut()
            .enumerate()
            .map(|(i, local_rng)| {
                let mut state: Vec<i8> = (0..model.num_vars)
                    .map(|_| local_rng.gen_range(0..=1))
                    .collect();
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

        // Swap acceptance counters
        let mut swap_accepts = vec![0; self.num_replicas - 1];

        for exchange_step in 0..self.total_exchanges {
            // Parallel Metropolis sweep (rayon)
            replicas
                .par_iter_mut()
                .zip(replica_rngs.par_iter_mut())
                .for_each(|(replica, local_rng)| {
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
                                && local_rng.gen_range(0.0..1.0) < exponent.exp())
                        {
                            replica.state[var_idx] = 1 - replica.state[var_idx];
                        }
                    }
                    replica.energy = model.calculate_total_energy(&replica.state);
                });

            // Replica exchange with statistics tracking
            for i in 0..(self.num_replicas - 1) {
                let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
                let delta_energy = replicas[i].energy - replicas[i + 1].energy;
                let swap_prob = (delta_beta * delta_energy).exp();

                if swap_prob >= 1.0 || engine_rng.gen_range(0.0..1.0) < swap_prob {
                    swap_accepts[i] += 1;
                    let (left, right) = replicas.split_at_mut(i + 1);
                    let ri = &mut left[i];
                    let rj = &mut right[0];
                    std::mem::swap(&mut ri.state, &mut rj.state);
                    for &(idx, val) in clamped {
                        ri.state[idx] = val;
                        rj.state[idx] = val;
                    }
                    std::mem::swap(&mut ri.energy, &mut rj.energy);
                }
            }

            // Temperature adaptation
            if (exchange_step + 1) % self.adaptation_interval == 0 {
                let target_rate = 0.23; // Optimal acceptance rate

                if exchange_step == self.adaptation_interval - 1 {
                    println!("--- Temperature Adaptation In Progress ---");
                }

                for i in 1..(self.num_replicas - 1) {
                    let rate = swap_accepts[i] as f64 / self.adaptation_interval as f64;

                    let adjustment = (target_rate - rate) * 0.05; // Learning rate
                    let mut new_temp = replicas[i].temp * (1.0 - adjustment);

                    // Guard: temperature must stay between neighbors.
                    // On ladders where adjacent temperatures lie within
                    // 0.002, the padded bounds invert and f64::clamp would
                    // panic — fall back to the neighbors' midpoint, which
                    // keeps strict monotonicity without a panic path.
                    let temp_hotter = replicas[i - 1].temp;
                    let temp_colder = replicas[i + 1].temp;
                    let lo = temp_colder + 0.001;
                    let hi = temp_hotter - 0.001;
                    new_temp = if lo <= hi {
                        new_temp.clamp(lo, hi)
                    } else {
                        0.5 * (temp_colder + temp_hotter)
                    };

                    replicas[i].temp = new_temp;
                    swap_accepts[i] = 0;
                }
                swap_accepts[0] = 0;
            }
        }

        replicas
            .into_iter()
            .min_by(|a, b| a.energy.partial_cmp(&b.energy).unwrap())
            .unwrap()
            .state
    }
}
