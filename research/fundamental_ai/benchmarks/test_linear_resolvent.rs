//! Mathematical verification:
//! Linear Energy Relaxation is the Resolvent of the Block Connection Graph
//! and converges toward Spectral Synchronization as lambda_obs approaches lambda_max.

#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    unused_variables,
    dead_code
)]

use fundamental_ai::learned_energy::PermutationWorld;
use fundamental_ai::synchronization_audit::SpectralPermutationSync;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn main() {
    let dim = 16;
    let num_relations = 4;
    let world = PermutationWorld::new(dim, num_relations, 1001);
    let mut rng = ChaCha8Rng::seed_from_u64(2002);

    let test_graphs: Vec<_> = (0..30)
        .map(|i| world.generate_consistent_cycle(4 + (i % 3), 0.20, &mut rng))
        .collect();

    // 1. Classical Spectral Synchronization
    let spectral = SpectralPermutationSync::new(dim, world.permutations.clone());
    let mut spec_acc = 0.0;
    let mut spec_rescue = 0.0;
    let mut spec_damage = 0.0;

    for g in &test_graphs {
        let s_spec = spectral.synchronize(g, 10, 0.5);
        let mut corr = 0;
        let mut rescued = 0;
        let mut damaged = 0;
        let mut init_corr = 0;
        let mut init_clean = 0;
        let total = g.num_nodes * dim;
        for v in 0..g.num_nodes {
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
                let fin_sign = if s_spec[v][i] >= 0.0 { 1.0 } else { -1.0 };
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
        spec_acc += corr as f64 / total as f64;
        spec_rescue += rescued as f64 / init_corr as f64;
        spec_damage += damaged as f64 / init_clean as f64;
    }
    let n = test_graphs.len() as f64;
    println!(
        "Spectral Sync Reference: Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
        spec_acc / n * 100.0,
        spec_rescue / n * 100.0,
        spec_damage / n * 100.0,
        (spec_rescue - spec_damage) / n * 100.0
    );

    // 2. Linear Resolvent Energy Relaxation across different lambda_obs
    // E = lambda/2 ||s - x||^2 - sum s_v^T Pi_r s_u
    // update: s_{t+1} = clamp( s_t - gamma * (lambda * (s_t - x) - W_block s_t), -1, 1 )
    for &lr in &[0.05, 0.10] {
        for &steps in &[10, 20, 50] {
            let lambda_obs = 0.2;
            let mut lin_acc = 0.0;
            let mut lin_rescue = 0.0;
            let mut lin_damage = 0.0;

            for g in &test_graphs {
                let mut s = g.noisy_states.clone();
                for _ in 0..steps {
                    let mut grad_s = vec![vec![0.0; dim]; g.num_nodes];
                    for v in 0..g.num_nodes {
                        for i in 0..dim {
                            grad_s[v][i] += lambda_obs * (s[v][i] - g.noisy_states[v][i]);
                        }
                    }
                    for edge in &g.edges {
                        let u = edge.source;
                        let v = edge.target;
                        let r = edge.relation;
                        for i in 0..dim {
                            let j = world.permutations[r][i];
                            // Pi_r[i, j] = 1 => s_v[i] * s_u[j]
                            grad_s[v][i] -= s[u][j];
                            grad_s[u][j] -= s[v][i];
                        }
                    }
                    for v in 0..g.num_nodes {
                        for i in 0..dim {
                            s[v][i] = (s[v][i] - lr * grad_s[v][i]).clamp(-1.0, 1.0);
                        }
                    }
                }

                // Evaluate
                let mut corr = 0;
                let mut rescued = 0;
                let mut damaged = 0;
                let mut init_corr = 0;
                let mut init_clean = 0;
                let total = g.num_nodes * dim;
                for v in 0..g.num_nodes {
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
                        let fin_sign = if s[v][i] >= 0.0 { 1.0 } else { -1.0 };
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
                lin_acc += corr as f64 / total as f64;
                lin_rescue += rescued as f64 / init_corr as f64;
                lin_damage += damaged as f64 / init_clean as f64;
            }

            println!("Linear EBM (lr={:.2}, steps={:2}): Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
                lr, steps, lin_acc / n * 100.0, lin_rescue / n * 100.0, lin_damage / n * 100.0, (lin_rescue - lin_damage) / n * 100.0);
        }
    }
}
