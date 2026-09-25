//! Diagnostic test: Linear/Bilinear Interaction Energy vs ln-cosh Barrier
//!
//! Compares:
//! 1. E_lncosh = - (1/beta) ln cosh(beta s_v^T W s_u) [Degree 2 inside tanh -> Force -> 0 at s_v=0]
//! 2. E_linear = - s_v^T W s_u                      [Force = W s_u != 0 at s_v=0]

#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    unused_variables,
    dead_code
)]

use fundamental_ai::learned_energy::PermutationWorld;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn main() {
    let dim = 16;
    let num_relations = 4;
    let world = PermutationWorld::new(dim, num_relations, 1001);
    let mut rng = ChaCha8Rng::seed_from_u64(2002);

    // Create ground-truth W matrices: W_r = Pi_r
    // where Pi_r is the exact permutation matrix for relation r
    let mut w_true = vec![vec![vec![0.0; dim]; dim]; num_relations];
    for r in 0..num_relations {
        for i in 0..dim {
            let j = world.permutations[r][i];
            w_true[r][i][j] = 1.0;
        }
    }

    let g = world.generate_consistent_cycle(4, 0.20, &mut rng);
    let n = g.num_nodes;
    let lambda_obs = 0.2;
    let lr = 0.05;

    // Test 1: Linear Interaction E = lambda/2 ||s - x||^2 - sum s_v^T W_r s_u
    println!("--- Testing Linear Interaction Energy with True Oracle W ---");
    let mut s_linear = g.noisy_states.clone();
    for step in 1..=16 {
        let mut grad_s = vec![vec![0.0; dim]; n];
        // Obs grad
        for v in 0..n {
            for i in 0..dim {
                grad_s[v][i] += lambda_obs * (s_linear[v][i] - g.noisy_states[v][i]);
            }
        }
        // Linear interaction grad: - sum_{u->v} W_r s_u - sum_{v->w} W_r^T s_w
        for edge in &g.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;
            for i in 0..dim {
                for j in 0..dim {
                    let w_val = w_true[r][i][j];
                    grad_s[v][i] -= w_val * s_linear[u][j];
                    grad_s[u][j] -= w_val * s_linear[v][i];
                }
            }
        }
        for v in 0..n {
            for i in 0..dim {
                s_linear[v][i] = (s_linear[v][i] - lr * grad_s[v][i]).clamp(-1.0, 1.0);
            }
        }

        // Evaluate rescue / damage
        let mut corr = 0;
        let mut rescued = 0;
        let mut damaged = 0;
        let mut init_corr = 0;
        let mut init_clean = 0;
        let total = n * dim;
        for v in 0..n {
            for i in 0..dim {
                let true_sign = if g.clean_states[v][i] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let init_sign = if g.noisy_states[v][i] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let fin_sign = if s_linear[v][i] >= 0.0 { 1.0 } else { -1.0 };
                if fin_sign == true_sign {
                    corr += 1;
                }
                if init_sign != true_sign {
                    init_corr += 1;
                    if fin_sign == true_sign {
                        rescued += 1;
                    }
                } else {
                    init_clean += 1;
                    if fin_sign != true_sign {
                        damaged += 1;
                    }
                }
            }
        }
        let rescue_rate = rescued as f64 / init_corr as f64;
        let damage_rate = damaged as f64 / init_clean as f64;
        println!(
            "Linear Step {:2}: Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
            step,
            corr as f64 / total as f64 * 100.0,
            rescue_rate * 100.0,
            damage_rate * 100.0,
            (rescue_rate - damage_rate) * 100.0
        );
    }

    // Test 2: Ln-Cosh Interaction E = lambda/2 ||s - x||^2 - (1/beta) sum ln cosh(beta s_v^T W_r s_u)
    println!("\n--- Testing Ln-Cosh Interaction Energy with True Oracle W (beta=1.0) ---");
    let mut s_lncosh = g.noisy_states.clone();
    let beta = 1.0;
    for step in 1..=16 {
        let mut grad_s = vec![vec![0.0; dim]; n];
        for v in 0..n {
            for i in 0..dim {
                grad_s[v][i] += lambda_obs * (s_lncosh[v][i] - g.noisy_states[v][i]);
            }
        }
        for edge in &g.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;
            let mut dot = 0.0;
            for i in 0..dim {
                for j in 0..dim {
                    dot += s_lncosh[v][i] * w_true[r][i][j] * s_lncosh[u][j];
                }
            }
            let factor = -(beta * dot).tanh();
            for i in 0..dim {
                for j in 0..dim {
                    let w_val = w_true[r][i][j];
                    grad_s[v][i] += factor * w_val * s_lncosh[u][j];
                    grad_s[u][j] += factor * w_val * s_lncosh[v][i];
                }
            }
        }
        for v in 0..n {
            for i in 0..dim {
                s_lncosh[v][i] = (s_lncosh[v][i] - lr * grad_s[v][i]).clamp(-1.0, 1.0);
            }
        }

        let mut corr = 0;
        let mut rescued = 0;
        let mut damaged = 0;
        let mut init_corr = 0;
        let mut init_clean = 0;
        let total = n * dim;
        for v in 0..n {
            for i in 0..dim {
                let true_sign = if g.clean_states[v][i] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let init_sign = if g.noisy_states[v][i] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let fin_sign = if s_lncosh[v][i] >= 0.0 { 1.0 } else { -1.0 };
                if fin_sign == true_sign {
                    corr += 1;
                }
                if init_sign != true_sign {
                    init_corr += 1;
                    if fin_sign == true_sign {
                        rescued += 1;
                    }
                } else {
                    init_clean += 1;
                    if fin_sign != true_sign {
                        damaged += 1;
                    }
                }
            }
        }
        let rescue_rate = rescued as f64 / init_corr as f64;
        let damage_rate = damaged as f64 / init_clean as f64;
        println!(
            "LnCosh Step {:2}: Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
            step,
            corr as f64 / total as f64 * 100.0,
            rescue_rate * 100.0,
            damage_rate * 100.0,
            (rescue_rate - damage_rate) * 100.0
        );
    }
}
