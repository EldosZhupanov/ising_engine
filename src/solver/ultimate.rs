use crate::core::hubo::{FlatHuboModel, HuboModel};
use crate::solver::engine::{calculate_replica_energies, step};
use crate::solver::types::{QuantumField, NUM_REPLICAS};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct UltimateSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub total_exchanges: usize,
    pub seed: Option<u64>,
    pub num_slices: usize,
    pub num_temps: usize,
    pub num_pops: usize,
    pub gnn_heuristic_probs: Option<Vec<f64>>,
}

impl UltimateSolver {
    pub fn new(
        temp_max: f64,
        temp_min: f64,
        sweeps: usize,
        exchanges: usize,
        seed: Option<u64>,
    ) -> Self {
        Self {
            num_replicas: NUM_REPLICAS,
            temp_max,
            temp_min,
            sweeps_per_exchange: sweeps,
            total_exchanges: exchanges,
            seed,
            num_slices: 1,
            num_temps: 10,
            num_pops: 1,
            gnn_heuristic_probs: None,
        }
    }

    pub fn with_gnn_heuristic(mut self, probs: Vec<f64>) -> Self {
        self.gnn_heuristic_probs = Some(probs);
        self
    }

    pub fn with_quantum_dims(mut self, slices: usize, temps: usize, pops: usize) -> Self {
        self.num_slices = slices;
        self.num_temps = temps;
        self.num_pops = pops;
        self
    }

    pub fn solve(&self, model: &crate::core::QuboModel, clamped: &[(usize, i8)]) -> Vec<i8> {
        // Convert QuboModel → HuboModel → FlatHuboModel
        let mut hubo_model = HuboModel::new(model.num_vars);
        hubo_model.linear = model.linear.clone();
        for i in 0..model.num_vars {
            for (j, weight) in model.quadratic.get_row(i) {
                hubo_model.edges2[i].push(crate::core::hubo::Edge2 { j, weight });
            }
        }
        // Flatten to CSR layout for SIMD-optimized traversal
        let flat_model = FlatHuboModel::from_hubo(&hubo_model);

        let n = model.num_vars;
        let base_seed = self.seed.unwrap_or_else(rand::random);
        let mut rng = ChaCha8Rng::seed_from_u64(base_seed);

        let mut is_clamped = vec![false; n];
        let mut clamped_val = vec![0i8; n];
        for &(idx, val) in clamped {
            is_clamped[idx] = true;
            clamped_val[idx] = val;
        }

        // Initialize 5D Field with byte-per-replica layout
        let mut field = QuantumField::new(n, self.num_slices, self.num_temps, self.num_pops);

        // Generate temperature schedule (geometric)
        let mut temps = Vec::with_capacity(self.num_temps);
        let temp_factor = if self.num_temps > 1 {
            (self.temp_min / self.temp_max).powf(1.0 / (self.num_temps - 1) as f64)
        } else {
            1.0
        };
        for i in 0..self.num_temps {
            temps.push(self.temp_max * temp_factor.powi(i as i32));
        }

        // Initialize States — byte-per-replica layout
        for p in 0..self.num_pops {
            for t in 0..self.num_temps {
                for s in 0..self.num_slices {
                    for v in 0..n {
                        let base = field.var_base(v, s, t, p);
                        if is_clamped[v] {
                            // Set all replicas to clamped value
                            let val = if clamped_val[v] == 1 { 1i8 } else { 0i8 };
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = val;
                            }
                        } else if let Some(ref probs) = self.gnn_heuristic_probs {
                            // GNN heuristic: each replica independently sampled
                            let prob_one = probs[v];
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] =
                                    if rng.gen_range(0.0..1.0) < prob_one { 1 } else { 0 };
                            }
                        } else {
                            // Random initialization: extract bits from random u64
                            let random_word: u64 = rng.gen();
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = ((random_word >> r) & 1) as i8;
                            }
                        }
                    }
                }
            }
        }

        let j_tau = 1.0;

        // Compute initial energies for incremental tracking
        for pp in 0..self.num_pops {
            for tt in 0..self.num_temps {
                field.energies[tt + self.num_temps * pp] =
                    calculate_replica_energies(&flat_model, &field, tt, pp, j_tau);
            }
        }

        // Main solver loop with periodic energy re-sync
        let mut step_counter = 0u64;
        for _ in 0..self.total_exchanges {
            for _ in 0..self.sweeps_per_exchange {
                step(&mut field, &flat_model, &temps, j_tau, &mut rng);
                step_counter += 1;
                // Re-sync every 1000 steps to prevent floating-point drift
                if step_counter % 1000 == 0 {
                    for pp in 0..self.num_pops {
                        for tt in 0..self.num_temps {
                            field.energies[tt + self.num_temps * pp] =
                                calculate_replica_energies(&flat_model, &field, tt, pp, j_tau);
                        }
                    }
                }
            }
        }

        // Extract best replica — direct byte read (no bit extraction!)
        let mut best_state = vec![0i8; n];
        let mut best_energy = f64::INFINITY;
        let mut state_buf = vec![0i8; n];

        for p in 0..self.num_pops {
            for t in 0..self.num_temps {
                for s in 0..self.num_slices {
                    for r in 0..NUM_REPLICAS {
                        for v in 0..n {
                            // Direct byte read — no shifting, no masking
                            state_buf[v] = field.get_replica(v, s, t, p, r);
                        }
                        let energy = model.calculate_total_energy(&state_buf);
                        if energy < best_energy {
                            best_energy = energy;
                            best_state.copy_from_slice(&state_buf);
                        }
                    }
                }
            }
        }
        best_state
    }
}
