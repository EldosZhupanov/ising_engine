use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use crate::core::hubo::HuboModel;
use crate::solver::types::QuantumField;
use crate::solver::engine::step;

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
    pub fn new(temp_max: f64, temp_min: f64, sweeps: usize, exchanges: usize, seed: Option<u64>) -> Self {
        Self {
            num_replicas: 64, // MSC locks to 64
            temp_max,
            temp_min,
            sweeps_per_exchange: sweeps,
            total_exchanges: exchanges,
            seed,
            num_slices: 1, // Default to 1 for classical, can be configured
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
        // Convert QuboModel to HuboModel internally to maintain backward compatibility for
        // the 50+ files referencing the old solver, while unlocking 5D engine capability.
        let mut hubo_model = HuboModel::new(model.num_vars);
        hubo_model.linear = model.linear.clone();
        for i in 0..model.num_vars {
            for (j, weight) in model.quadratic.get_row(i) {
                // CSR matrix stores undirected graph as directed edges, adding i->j
                hubo_model.edges2[i].push(crate::core::hubo::Edge2 { j, weight });
            }
        }
        let n = model.num_vars;
        let base_seed = self.seed.unwrap_or_else(rand::random);
        let mut rng = ChaCha8Rng::seed_from_u64(base_seed);

        let mut is_clamped = vec![false; n];
        let mut clamped_val = vec![0; n];
        for &(idx, val) in clamped {
            is_clamped[idx] = true;
            clamped_val[idx] = val;
        }

        // Initialize 5D Field
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

        // Initialize States
        for p in 0..self.num_pops {
            for t in 0..self.num_temps {
                for s in 0..self.num_slices {
                    for v in 0..n {
                        if is_clamped[v] {
                            let val = if clamped_val[v] == 1 { u64::MAX } else { 0 };
                            *field.get_mut(v, s, t, p) = val;
                        } else {
                            if let Some(ref probs) = self.gnn_heuristic_probs {
                                let prob_one = probs[v];
                                let mut bitmask = 0u64;
                                for bit in 0..64 {
                                    if rng.gen_range(0.0..1.0) < prob_one {
                                        bitmask |= 1 << bit;
                                    }
                                }
                                *field.get_mut(v, s, t, p) = bitmask;
                            } else {
                                *field.get_mut(v, s, t, p) = rng.gen::<u64>();
                            }
                        }
                    }
                }
            }
        }

        let j_tau = 1.0; // Coupling constant for quantum slices

        // Interaction Kernel Execution
        for _ in 0..self.total_exchanges {
            for _ in 0..self.sweeps_per_exchange {
                step(&mut field, &hubo_model, &temps, j_tau, &mut rng);
            }
        }

        // Extract best replica result (simplification: extracting replica 0 of temp 0, pop 0, slice 0)
        let mut final_state = vec![0i8; n];
        for (i, item) in final_state.iter_mut().enumerate().take(n) {
            *item = (field.get(i, 0, 0, 0) & 1) as i8;
        }
        final_state
    }
}
