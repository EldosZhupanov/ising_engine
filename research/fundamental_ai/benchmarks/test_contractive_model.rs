//! Diagnostic test: Contractive Non-Energy Neural Model (Family C) vs Energy Relaxation
//!
//! Evaluates whether a contractive non-energy update:
//! s_v^{(t+1)} = (1 - alpha) s_v^{(t)} + alpha * tanh( W_self s_v^{(t)} + sum W_r s_u )
//! can learn to perform active error correction (Net Rescue > +30%) on the permutation task.

#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    unused_variables,
    dead_code
)]

use fundamental_ai::learned_energy::PermutationWorld;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

struct ContractiveModel {
    dim: usize,
    num_relations: usize,
    alpha: f64,
    w_rel: Vec<f64>,  // [num_relations * dim * dim]
    w_self: Vec<f64>, // [dim * dim]
}

impl ContractiveModel {
    fn new(dim: usize, num_relations: usize, alpha: f64, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = (1.0 / (dim as f64)).sqrt();
        let w_rel = (0..(num_relations * dim * dim))
            .map(|_| rng.gen_range(-scale..scale))
            .collect();
        let mut w_self = vec![0.0; dim * dim];
        for i in 0..dim {
            w_self[i * dim + i] = 1.0; // Identity self-loop
        }
        Self {
            dim,
            num_relations,
            alpha,
            w_rel,
            w_self,
        }
    }

    fn step(
        &self,
        s: &[Vec<f64>],
        edges: &[fundamental_ai::learned_energy::ConstraintEdge],
        n: usize,
    ) -> Vec<Vec<f64>> {
        let mut next_s = vec![vec![0.0; self.dim]; n];
        for v in 0..n {
            let mut field = vec![0.0; self.dim];
            // Self contribution
            for i in 0..self.dim {
                for j in 0..self.dim {
                    field[i] += self.w_self[i * self.dim + j] * s[v][j];
                }
            }
            // Incoming message contributions
            for edge in edges {
                if edge.target == v {
                    let u = edge.source;
                    let r = edge.relation;
                    for i in 0..self.dim {
                        for j in 0..self.dim {
                            field[i] +=
                                self.w_rel[r * self.dim * self.dim + i * self.dim + j] * s[u][j];
                        }
                    }
                }
                // Also reverse edge (if undirected message passing)
                if edge.source == v {
                    let u = edge.target;
                    let r = edge.relation;
                    // Transpose relation
                    for i in 0..self.dim {
                        for j in 0..self.dim {
                            field[i] +=
                                self.w_rel[r * self.dim * self.dim + j * self.dim + i] * s[u][j];
                        }
                    }
                }
            }
            for i in 0..self.dim {
                next_s[v][i] = (1.0 - self.alpha) * s[v][i] + self.alpha * field[i].tanh();
            }
        }
        next_s
    }

    fn unroll(
        &self,
        s_init: &[Vec<f64>],
        edges: &[fundamental_ai::learned_energy::ConstraintEdge],
        n: usize,
        steps: usize,
    ) -> Vec<Vec<f64>> {
        let mut s = s_init.to_vec();
        for _ in 0..steps {
            s = self.step(&s, edges, n);
        }
        s
    }
}

fn main() {
    let dim = 16;
    let num_relations = 4;
    let world = PermutationWorld::new(dim, num_relations, 1001);
    let mut rng_train = ChaCha8Rng::seed_from_u64(2002);
    let mut rng_test = ChaCha8Rng::seed_from_u64(3003);

    let train_graphs: Vec<_> = (0..40)
        .map(|i| world.generate_consistent_cycle(3 + (i % 3), 0.20, &mut rng_train))
        .collect();
    let test_graphs: Vec<_> = (0..40)
        .map(|i| world.generate_consistent_cycle(3 + (i % 3), 0.20, &mut rng_test))
        .collect();

    // Initialize with Oracle Weights to test theoretical upper bound of Family C
    let mut oracle_model = ContractiveModel::new(dim, num_relations, 0.25, 42);
    oracle_model.w_rel.fill(0.0);
    for r in 0..num_relations {
        for i in 0..dim {
            let j = world.permutations[r][i];
            // Forward edge message
            oracle_model.w_rel[r * dim * dim + i * dim + j] = 1.0;
        }
    }

    println!("--- Family C: Contractive Model with Oracle Relation Weights ---");
    for &t in &[0, 1, 2, 4, 8, 16] {
        let mut m_acc = 0.0;
        let mut m_rescue = 0.0;
        let mut m_damage = 0.0;
        for g in &test_graphs {
            let s = oracle_model.unroll(&g.noisy_states, &g.edges, g.num_nodes, t);
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
            m_acc += corr as f64 / total as f64;
            m_rescue += rescued as f64 / init_corr as f64;
            m_damage += damaged as f64 / init_clean as f64;
        }
        let n_test = test_graphs.len() as f64;
        m_acc /= n_test;
        m_rescue /= n_test;
        m_damage /= n_test;
        println!("Oracle Contractive T={:2}: Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
            t, m_acc * 100.0, m_rescue * 100.0, m_damage * 100.0, (m_rescue - m_damage) * 100.0);
    }
}
