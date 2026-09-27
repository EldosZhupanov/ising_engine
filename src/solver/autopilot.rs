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

/// Random parameter search followed by local random perturbations.
///
/// This tuner has no Bayesian surrogate, posterior or acquisition function.
/// Its selected configuration is the best sampled one, not a certified optimum.
pub struct RandomSearchOrchestrator {
    pub micro_exchanges: usize, // How deep to go during exploration (e.g., 10)
    pub exploration_samples: usize, // How many random points to sample (e.g., 20)
    pub exploitation_steps: usize, // How many local random proposals to evaluate
}

/// Compatibility name for existing callers; this algorithm is not Bayesian.
pub type BayesianOrchestrator = RandomSearchOrchestrator;

impl Default for RandomSearchOrchestrator {
    fn default() -> Self {
        Self {
            micro_exchanges: 20,
            exploration_samples: 15,
            exploitation_steps: 5,
        }
    }
}

impl RandomSearchOrchestrator {
    /// Tune parameters and return the final solve's energy, state and selected configuration.
    pub fn solve_auto(&self, model: &QuboModel) -> (f64, Vec<i8>, SolverConfig) {
        println!(
            "🤖 [Autopilot] Initiating Phase 1: Parameter Exploration ({} samples)...",
            self.exploration_samples
        );
        let mut rng = rand::thread_rng();

        let mut best_config = SolverConfig {
            temp_max: 1000.0,
            temp_min: 0.01,
            sweeps: 100,
        };
        let mut best_energy = f64::INFINITY;

        // 1. EXPLORATION (Random Uniform Sampling across the space)

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
            let energy = model.calculate_total_energy(&state);

            if energy < best_energy {
                best_energy = energy;
                best_config = config;
            }
        }

        // 2. Local random search around the best sampled configuration.
        println!(
            "🤖 [Autopilot] Initiating Phase 2: Exploitation (Refining T_max={:.1}, Sweeps={})...",
            best_config.temp_max, best_config.sweeps
        );

        for _ in 0..self.exploitation_steps {
            // Create a neighborhood mutation of the best config
            let mut config = best_config;
            config.temp_max *= rng.gen_range(0.8..1.2); // +/- 20%
            config.temp_min *= rng.gen_range(0.5..2.0);
            config.sweeps = (config.sweeps as f64 * rng.gen_range(0.8..1.2)) as usize;

            let solver = UltimateSolver::new(
                config.temp_max,
                config.temp_min,
                config.sweeps,
                self.micro_exchanges,
                None,
            );
            let state = solver.solve(model, &[]);
            let energy = model.calculate_total_energy(&state);

            if energy < best_energy {
                best_energy = energy;
                best_config = config;
            }
        }

        println!(
            "[Autopilot] Best sampled config: T_max={:.1}, T_min={:.4}, Sweeps={}",
            best_config.temp_max, best_config.temp_min, best_config.sweeps
        );

        // 3. FULL DEEP EXECUTION
        // Evaluate the selected configuration with a larger exchange budget.
        println!("🚀 [Autopilot] Launching Deep Annealing with Selected Config...");
        let final_exchanges = 300; // Deep run
        let solver = UltimateSolver::new(
            best_config.temp_max,
            best_config.temp_min,
            best_config.sweeps,
            final_exchanges,
            None,
        );

        let start = Instant::now();
        let state = solver.solve(model, &[]);
        let final_energy = model.calculate_total_energy(&state);

        println!("⏱️ Deep Run Execution Time: {:?}", start.elapsed());

        (final_energy, state, best_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CsrMatrix;

    #[test]
    fn reported_energy_includes_model_offset_and_legacy_name_compiles() {
        let model = QuboModel {
            num_vars: 1,
            energy_offset: 7.0,
            linear: vec![-2.0],
            quadratic: CsrMatrix::empty(1),
        };
        let tuner: RandomSearchOrchestrator = BayesianOrchestrator {
            micro_exchanges: 1,
            exploration_samples: 0,
            exploitation_steps: 0,
        };
        let (energy, state, _) = tuner.solve_auto(&model);
        assert_eq!(state, vec![1]);
        assert_eq!(energy, 5.0);
        assert_eq!(energy, model.calculate_total_energy(&state));
    }
}
