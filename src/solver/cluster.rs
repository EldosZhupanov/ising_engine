use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::f64::consts::E;

use super::replica::{build_clamped_set, Replica};
use crate::core::QuboModel;

/// Cluster-flip solver with adaptive tempering.
///
/// Combines Wolff-style cluster flips with single-spin Metropolis sweeps
/// and adaptive temperature optimization. Most powerful solver variant.
pub struct ClusterSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub total_exchanges: usize,
    pub adaptation_interval: usize,
    pub seed: Option<u64>,
}

impl ClusterSolver {
    fn calculate_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
        let flip_multiplier = 1.0 - 2.0 * (state[var_idx] as f64);
        let mut sum_j = 0.0;
        for (col, weight) in model.quadratic.get_row(var_idx) {
            sum_j += weight * (state[col] as f64);
        }
        flip_multiplier * (model.linear[var_idx] + sum_j)
    }

    /// Attempts to flip a cluster of strongly coupled variables.
    fn try_cluster_flip(
        model: &QuboModel,
        replica: &mut Replica,
        clamped_set: &[bool],
        rng: &mut ChaCha8Rng,
    ) {
        let seed = rng.gen_range(0..model.num_vars);
        if clamped_set[seed] {
            return;
        }

        let mut cluster = vec![seed];
        let mut in_cluster = vec![false; model.num_vars];
        in_cluster[seed] = true;
        let mut queue = vec![seed];

        while let Some(current) = queue.pop() {
            for (neighbor, weight) in model.quadratic.get_row(current) {
                if !in_cluster[neighbor] && weight.abs() > 1.0 {
                    if clamped_set[neighbor] {
                        continue;
                    }
                    let prob = 1.0 - (-2.0 * weight.abs() / replica.temp).exp();
                    if rng.gen_range(0.0..1.0) < prob {
                        in_cluster[neighbor] = true;
                        cluster.push(neighbor);
                        queue.push(neighbor);
                    }
                }
            }
        }

        if cluster.len() < 2 {
            return;
        }

        let e_old = model.calculate_total_energy(&replica.state);
        for &idx in &cluster {
            replica.state[idx] = 1 - replica.state[idx];
        }
        let e_new = model.calculate_total_energy(&replica.state);
        let delta_e = e_new - e_old;

        if delta_e > 0.0
            && (replica.temp <= 1e-8 || rng.gen_range(0.0..1.0) >= E.powf(-delta_e / replica.temp))
        {
            // Reject: revert
            for &idx in &cluster {
                replica.state[idx] = 1 - replica.state[idx];
            }
        }
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

        let mut swap_accepts = vec![0; self.num_replicas - 1];

        for exchange_step in 0..self.total_exchanges {
            replicas.par_iter_mut().zip(replica_rngs.par_iter_mut()).for_each(|(replica, local_rng)| {
                for step in 0..self.sweeps_per_exchange {
                    if step % 10 == 0 {
                        Self::try_cluster_flip(model, replica, &clamped_set, local_rng);
                    } else {
                        let var_idx = local_rng.gen_range(0..model.num_vars);
                        if clamped_set[var_idx] {
                            continue;
                        }
                        let delta_e = Self::calculate_delta_e(model, &replica.state, var_idx);
                        if delta_e < 0.0
                            || (replica.temp > 1e-8
                                && local_rng.gen_range(0.0..1.0) < E.powf(-delta_e / replica.temp))
                        {
                            replica.state[var_idx] = 1 - replica.state[var_idx];
                        }
                    }
                }
                replica.energy = model.calculate_total_energy(&replica.state);
            });

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

            if (exchange_step + 1) % self.adaptation_interval == 0 {
                for i in 1..(self.num_replicas - 1) {
                    let rate = swap_accepts[i] as f64 / self.adaptation_interval as f64;
                    let mut new_temp = replicas[i].temp * (1.0 - (0.23 - rate) * 0.05);
                    new_temp =
                        new_temp.clamp(replicas[i + 1].temp + 0.001, replicas[i - 1].temp - 0.001);
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
