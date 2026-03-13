use crate::core::QuboModel;
use rand::Rng;

pub struct TabuSolver {
    pub max_iterations: usize,
    pub tabu_tenure: usize,
}

impl TabuSolver {
    pub fn solve(&self, model: &QuboModel) -> (f64, Vec<i8>) {
        let n = model.num_vars;
        let mut rng = rand::thread_rng();
        
        let mut current_state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
        let mut best_state = current_state.clone();
        
        let mut current_energy = Self::calculate_energy(model, &current_state);
        let mut best_energy = current_energy;
        
        // Tabu list stores the iteration number when a flip is allowed again
        let mut tabu_list = vec![0; n];

        for iter in 0..self.max_iterations {
            let mut best_delta = f64::INFINITY;
            let mut best_flip_idx = n; // Invalid initial index

            for i in 0..n {
                // If it's not tabu (or we allow aspiration override, but keeping simple here)
                if tabu_list[i] <= iter {
                    let flip_mult = if current_state[i] == 1 { -1.0 } else { 1.0 };
                    
                    let mut sum_j = 0.0;
                    for (col, weight) in model.quadratic.get_row(i) { 
                        sum_j += weight * (current_state[col] as f64); 
                    }
                    let delta_e = flip_mult * (model.linear[i] + sum_j);

                    if delta_e < best_delta {
                        best_delta = delta_e;
                        best_flip_idx = i;
                    }
                }
            }

            if best_flip_idx < n {
                // Apply the flip
                current_state[best_flip_idx] = 1 - current_state[best_flip_idx];
                current_energy += best_delta;
                
                // Add to tabu list
                tabu_list[best_flip_idx] = iter + self.tabu_tenure;

                // Update global best
                if current_energy < best_energy {
                    best_energy = current_energy;
                    best_state = current_state.clone();
                }
            } else {
                // All improving/neutral moves are tabu. Break or wait.
                break;
            }
        }

        (best_energy, best_state)
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
