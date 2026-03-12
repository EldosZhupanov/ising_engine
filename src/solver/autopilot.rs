use crate::core::QuboModel;
use crate::solver::UltimateSolver;
use rand::Rng;
use std::time::Instant;

/// Represents a configuration of hyperparameters for the Ising Solver
#[derive(Debug, Clone, Copy)]
pub struct SolverConfig {
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps: usize,
}

/// The autonomous orchestrator that uses a lightweight Bayesian/Surrogate heuristic
/// to find the optimal hyperparameters for a specific QUBO instance before full execution.
pub struct BayesianOrchestrator {
    pub micro_exchanges: usize, // How deep to go during exploration (e.g., 10)
    pub exploration_samples: usize, // How many random points to sample (e.g., 20)
    pub exploitation_steps: usize, // How many gradient steps to take toward the best zone
}

impl Default for BayesianOrchestrator {
    fn default() -> Self {
        Self {
            micro_exchanges: 20,
            exploration_samples: 15,
            exploitation_steps: 5,
        }
    }
}

impl BayesianOrchestrator {
    /// Autonomously tune parameters and return the best state found during the final deep run
    pub fn solve_auto(&self, model: &QuboModel) -> (f64, Vec<i8>, SolverConfig) {
        println!("🤖 [Autopilot] Initiating Phase 1: Parameter Exploration ({} samples)...", self.exploration_samples);
        let mut rng = rand::thread_rng();
        
        let mut best_config = SolverConfig { temp_max: 1000.0, temp_min: 0.01, sweeps: 100 };
        let mut best_energy = f64::INFINITY;

        // 1. EXPLORATION (Random Uniform Sampling across the space)
        let mut history = Vec::new();
        
        for _ in 0..self.exploration_samples {
            let config = SolverConfig {
                temp_max: rng.gen_range(10.0..5000.0),
                // Log-uniform sampling for temp_min is better, but we approximate
                temp_min: rng.gen_range(0.001..0.5),
                sweeps: rng.gen_range(10..200),
            };

            let solver = UltimateSolver::new(
                config.temp_max,
                config.temp_min,
                config.sweeps,
                self.micro_exchanges,
                None, // Random seed
            );

            let state = solver.solve(model, &[]);
            let energy = Self::calculate_energy(model, &state);
            
            history.push((config, energy));

            if energy < best_energy {
                best_energy = energy;
                best_config = config;
            }
        }

        // 2. EXPLOITATION (Surrogate Hill-Climbing / Expected Improvement proxy)
        // We take the best config found and "wiggle" the parameters to find the exact peak.
        println!("🤖 [Autopilot] Initiating Phase 2: Exploitation (Refining T_max={:.1}, Sweeps={})...", best_config.temp_max, best_config.sweeps);
        
        for _ in 0..self.exploitation_steps {
            // Create a neighborhood mutation of the best config
            let mut config = best_config;
            config.temp_max *= rng.gen_range(0.8..1.2); // +/- 20%
            config.temp_min *= rng.gen_range(0.5..2.0);
            config.sweeps = (config.sweeps as f64 * rng.gen_range(0.8..1.2)) as usize;
            
            let solver = UltimateSolver::new(
                config.temp_max, config.temp_min, config.sweeps, self.micro_exchanges, None
            );
            let state = solver.solve(model, &[]);
            let energy = Self::calculate_energy(model, &state);
            
            if energy < best_energy {
                best_energy = energy;
                best_config = config;
            }
        }

        println!("✅ [Autopilot] Tuning Complete. Optimal Config Found: T_max={:.1}, T_min={:.4}, Sweeps={}", 
                 best_config.temp_max, best_config.temp_min, best_config.sweeps);
                 
        // 3. FULL DEEP EXECUTION
        // Now that we know the exact physical properties of this matrix, we unleash the full power.
        println!("🚀 [Autopilot] Launching Deep Annealing with Optimal Config...");
        let final_exchanges = 300; // Deep run
        let solver = UltimateSolver::new(
            best_config.temp_max, best_config.temp_min, best_config.sweeps, final_exchanges, None
        );
        
        let start = Instant::now();
        let state = solver.solve(model, &[]);
        let final_energy = Self::calculate_energy(model, &state);
        
        println!("⏱️ Deep Run Execution Time: {:?}", start.elapsed());
        
        (final_energy, state, best_config)
    }

    fn calculate_energy(model: &QuboModel, state: &[i8]) -> f64 {
        let mut e = 0.0;
        for i in 0..model.num_vars {
            if state[i] == 1 {
                e += model.linear[i];
                for (j, w) in model.quadratic.get_row(i) {
                    if state[j] == 1 { e += w * 0.5; }
                }
            }
        }
        e
    }
}
