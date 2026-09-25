//! Synchronization Baselines, Stabilized Attention, and Trajectory Auditing for Cyclic Constraints.
//!
//! Provides:
//! 1. Classical Permutation Synchronization (Spectral Sync - Pachauri et al. 2013).
//! 2. Factor Graph Loopy Min-Sum Message Passing.
//! 3. Damped Recurrent Attention (Mann iteration).
//! 4. Symmetric Energy-Admitting Attention (W_Q = W_K).
//! 5. Exact Trajectory Auditor (T=0, Rescue Rate, Damage Rate, Net Rescue).
//! 6. McNemar and Paired Bootstrap Statistical Tests.

use crate::learned_energy::CyclicGraph;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

// ---------------------------------------------------------------------------
// 1. Classical Permutation Synchronization (Spectral Relaxation)
// ---------------------------------------------------------------------------
pub struct SpectralPermutationSync {
    pub dim: usize,
    pub permutations: Vec<Vec<usize>>, // per relation r
}

impl SpectralPermutationSync {
    pub fn new(dim: usize, permutations: Vec<Vec<usize>>) -> Self {
        Self { dim, permutations }
    }

    /// Synchronize noisy states by diffusing along the block permutation matrix.
    /// In each step: s_v <- sign( lambda_obs * x_v + sum_{u -> v} Pi_r s_u + sum_{v -> w} Pi_r^T s_w )
    pub fn synchronize(&self, graph: &CyclicGraph, steps: usize, alpha: f64) -> Vec<Vec<f64>> {
        let n = graph.num_nodes;
        let mut s = graph.noisy_states.clone();

        for _ in 0..steps {
            let mut next_s = vec![vec![0.0; self.dim]; n];

            // 1. Unary observation anchor
            for v in 0..n {
                for i in 0..self.dim {
                    next_s[v][i] += graph.noisy_states[v][i];
                }
            }

            // 2. Relative permutation constraints along edges
            for edge in &graph.edges {
                let u = edge.source;
                let v = edge.target;
                let r = edge.relation;
                let perm = &self.permutations[r];

                // Forward message u -> v: s_v[perm[i]] predicted from s_u[i]
                for i in 0..self.dim {
                    next_s[v][perm[i]] += s[u][i];
                }

                // Backward message v -> u: s_u[i] predicted from s_v[perm[i]]
                for i in 0..self.dim {
                    next_s[u][i] += s[v][perm[i]];
                }
            }

            // 3. Damped update and continuous relaxation
            for v in 0..n {
                for i in 0..self.dim {
                    let updated = next_s[v][i].clamp(-1.0, 1.0);
                    s[v][i] = (1.0 - alpha) * s[v][i] + alpha * updated;
                }
            }
        }

        s
    }
}

// ---------------------------------------------------------------------------
// 2. Factor Graph Loopy Min-Sum / Belief Propagation
// ---------------------------------------------------------------------------
pub struct LoopyMinSumSync {
    pub dim: usize,
    pub permutations: Vec<Vec<usize>>,
}

impl LoopyMinSumSync {
    pub fn new(dim: usize, permutations: Vec<Vec<usize>>) -> Self {
        Self { dim, permutations }
    }

    /// Min-sum message passing on the cyclic factor graph.
    pub fn solve(&self, graph: &CyclicGraph, steps: usize, damping: f64) -> Vec<Vec<f64>> {
        let n = graph.num_nodes;
        let num_edges = graph.edges.len();

        // Messages: forward[edge_idx] and backward[edge_idx]
        let mut fwd_msgs = vec![vec![0.0; self.dim]; num_edges];
        let mut bwd_msgs = vec![vec![0.0; self.dim]; num_edges];

        // Initialize messages from unary noisy states
        for (e_idx, edge) in graph.edges.iter().enumerate() {
            let u = edge.source;
            let v = edge.target;
            let perm = &self.permutations[edge.relation];
            for i in 0..self.dim {
                fwd_msgs[e_idx][perm[i]] = graph.noisy_states[u][i];
                bwd_msgs[e_idx][i] = graph.noisy_states[v][perm[i]];
            }
        }

        for _ in 0..steps {
            let mut new_fwd = fwd_msgs.clone();
            let mut new_bwd = bwd_msgs.clone();

            for (e_idx, edge) in graph.edges.iter().enumerate() {
                let u = edge.source;
                let v = edge.target;
                let perm = &self.permutations[edge.relation];

                // Message u -> v aggregates noisy_state[u] + incoming messages to u excluding edge e
                let mut cav_u = graph.noisy_states[u].clone();
                for (other_idx, other_e) in graph.edges.iter().enumerate() {
                    if other_idx != e_idx {
                        if other_e.target == u {
                            for i in 0..self.dim {
                                cav_u[i] += fwd_msgs[other_idx][i];
                            }
                        }
                        if other_e.source == u {
                            for i in 0..self.dim {
                                cav_u[i] += bwd_msgs[other_idx][i];
                            }
                        }
                    }
                }

                // Push through permutation
                for i in 0..self.dim {
                    let val = (1.0 - damping) * fwd_msgs[e_idx][perm[i]] + damping * cav_u[i];
                    new_fwd[e_idx][perm[i]] = val.clamp(-2.0, 2.0);
                }

                // Backward message v -> u aggregates noisy_state[v] + incoming messages to v excluding e
                let mut cav_v = graph.noisy_states[v].clone();
                for (other_idx, other_e) in graph.edges.iter().enumerate() {
                    if other_idx != e_idx {
                        if other_e.target == v {
                            for i in 0..self.dim {
                                cav_v[i] += fwd_msgs[other_idx][i];
                            }
                        }
                        if other_e.source == v {
                            for i in 0..self.dim {
                                cav_v[i] += bwd_msgs[other_idx][i];
                            }
                        }
                    }
                }

                for i in 0..self.dim {
                    let val = (1.0 - damping) * bwd_msgs[e_idx][i] + damping * cav_v[perm[i]];
                    new_bwd[e_idx][i] = val.clamp(-2.0, 2.0);
                }
            }

            fwd_msgs = new_fwd;
            bwd_msgs = new_bwd;
        }

        // Decode marginals
        let mut final_s = graph.noisy_states.clone();
        for (e_idx, edge) in graph.edges.iter().enumerate() {
            let u = edge.source;
            let v = edge.target;
            for i in 0..self.dim {
                final_s[v][i] += fwd_msgs[e_idx][i];
                final_s[u][i] += bwd_msgs[e_idx][i];
            }
        }

        for v in 0..n {
            for i in 0..self.dim {
                final_s[v][i] = final_s[v][i].clamp(-1.0, 1.0);
            }
        }

        final_s
    }
}

// ---------------------------------------------------------------------------
// 3. Damped Recurrent Attention Network (Mann Iteration)
// ---------------------------------------------------------------------------
pub struct DampedRecurrentAttention {
    pub dim: usize,
    pub num_relations: usize,
    pub alpha: f64, // Damping factor in (0, 1]
    pub w_q: Vec<f64>,
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_rel: Vec<f64>,
}

impl DampedRecurrentAttention {
    pub fn new(dim: usize, num_relations: usize, alpha: f64, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = (2.0 / (dim as f64)).sqrt();
        let mut init = |n: usize| (0..n).map(|_| rng.gen_range(-scale..scale)).collect();

        Self {
            dim,
            num_relations,
            alpha,
            w_q: init(dim * dim),
            w_k: init(dim * dim),
            w_v: init(dim * dim),
            w_rel: init(num_relations * dim),
        }
    }

    pub fn unroll(&self, graph: &CyclicGraph, steps: usize) -> Vec<Vec<f64>> {
        let mut s = graph.noisy_states.clone();
        let n = graph.num_nodes;

        for _ in 0..steps {
            let mut next_s = s.clone();

            for edge in &graph.edges {
                let u = edge.source;
                let v = edge.target;
                let r = edge.relation;

                let mut q = vec![0.0; self.dim];
                let mut k = vec![0.0; self.dim];
                let mut val = vec![0.0; self.dim];

                let rel_offset = r * self.dim;
                for i in 0..self.dim {
                    let row = i * self.dim;
                    let rel_i = self.w_rel[rel_offset + i];
                    for j in 0..self.dim {
                        q[i] += self.w_q[row + j] * (s[v][j] + rel_i);
                        k[i] += self.w_k[row + j] * s[u][j];
                        val[i] += self.w_v[row + j] * s[u][j];
                    }
                }

                let mut dot = 0.0;
                for i in 0..self.dim {
                    dot += q[i] * k[i];
                }
                let attn_weight = (dot / (self.dim as f64).sqrt()).tanh();

                for i in 0..self.dim {
                    next_s[v][i] += attn_weight * val[i];
                }
            }

            // Damped residual step: s <- (1 - alpha) * s + alpha * clamp(next_s, -1, 1)
            for v in 0..n {
                for i in 0..self.dim {
                    let updated = next_s[v][i].clamp(-1.0, 1.0);
                    s[v][i] = (1.0 - self.alpha) * s[v][i] + self.alpha * updated;
                }
            }
        }

        s
    }
}

// ---------------------------------------------------------------------------
// 4. Symmetric Energy-Admitting Attention (W_Q = W_K)
// ---------------------------------------------------------------------------
pub struct SymmetricEnergyAttention {
    pub dim: usize,
    pub num_relations: usize,
    pub alpha: f64,
    pub w_qk: Vec<f64>, // Shared W_Q = W_K, dimension dim x dim
    pub w_rel: Vec<f64>,
}

impl SymmetricEnergyAttention {
    pub fn new(dim: usize, num_relations: usize, alpha: f64, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = (2.0 / (dim as f64)).sqrt();
        let mut init = |n: usize| (0..n).map(|_| rng.gen_range(-scale..scale)).collect();

        Self {
            dim,
            num_relations,
            alpha,
            w_qk: init(dim * dim),
            w_rel: init(num_relations * dim),
        }
    }

    pub fn unroll(&self, graph: &CyclicGraph, steps: usize) -> Vec<Vec<f64>> {
        let mut s = graph.noisy_states.clone();
        let n = graph.num_nodes;

        for _ in 0..steps {
            let mut next_s = s.clone();

            for edge in &graph.edges {
                let u = edge.source;
                let v = edge.target;
                let r = edge.relation;

                let mut q = vec![0.0; self.dim];
                let mut k = vec![0.0; self.dim];

                let rel_offset = r * self.dim;
                for i in 0..self.dim {
                    let row = i * self.dim;
                    let rel_i = self.w_rel[rel_offset + i];
                    for j in 0..self.dim {
                        q[i] += self.w_qk[row + j] * (s[v][j] + rel_i);
                        k[i] += self.w_qk[row + j] * (s[u][j] + rel_i);
                    }
                }

                let mut dot = 0.0;
                for i in 0..self.dim {
                    dot += q[i] * k[i];
                }
                // Symmetric potential derivative
                let factor = (dot / (self.dim as f64).sqrt()).tanh();

                // Bidirectional symmetric force
                for i in 0..self.dim {
                    next_s[v][i] += factor * k[i];
                    next_s[u][i] += factor * q[i];
                }
            }

            for v in 0..n {
                for i in 0..self.dim {
                    let updated = next_s[v][i].clamp(-1.0, 1.0);
                    s[v][i] = (1.0 - self.alpha) * s[v][i] + self.alpha * updated;
                }
            }
        }

        s
    }
}

// ---------------------------------------------------------------------------
// 5. Exact Trajectory Auditor (EXP-TEN-006A-R Audit)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct StepAuditMetrics {
    pub step: usize,
    pub mean_accuracy: f64,
    pub exact_solve: bool,
    pub hamming_dist_to_clean: usize,
    pub hamming_dist_to_noisy: usize,
    pub rescue_rate: f64,
    pub damage_rate: f64,
    pub net_rescue: f64,
    pub delta_s_norm: f64,
    pub energy: f64,
}

pub struct TrajectoryAuditor;

impl TrajectoryAuditor {
    pub fn audit_state(
        current_s: &[Vec<f64>],
        prev_s: &[Vec<f64>],
        noisy_s: &[Vec<f64>],
        clean_s: &[Vec<f64>],
        step: usize,
        energy: f64,
    ) -> StepAuditMetrics {
        let n = current_s.len();
        let dim = current_s[0].len();
        let total_bits = n * dim;

        let mut correct_bits = 0;
        let mut dist_clean = 0;
        let mut dist_noisy = 0;
        let mut initial_wrong = 0;
        let mut initial_correct = 0;
        let mut rescued = 0;
        let mut damaged = 0;
        let mut delta_sq = 0.0;

        for v in 0..n {
            for i in 0..dim {
                let pred = if current_s[v][i] >= 0.0 { 1.0 } else { -1.0 };
                let cl = clean_s[v][i];
                let ny = noisy_s[v][i];

                if (pred - cl).abs() < 1e-3 {
                    correct_bits += 1;
                } else {
                    dist_clean += 1;
                }

                if (pred - ny).abs() >= 1e-3 {
                    dist_noisy += 1;
                }

                let was_initially_wrong = (ny - cl).abs() > 1e-3;
                if was_initially_wrong {
                    initial_wrong += 1;
                    if (pred - cl).abs() < 1e-3 {
                        rescued += 1;
                    }
                } else {
                    initial_correct += 1;
                    if (pred - cl).abs() > 1e-3 {
                        damaged += 1;
                    }
                }

                let diff = current_s[v][i] - prev_s[v][i];
                delta_sq += diff * diff;
            }
        }

        let rescue_rate = if initial_wrong > 0 {
            rescued as f64 / initial_wrong as f64
        } else {
            0.0
        };

        let damage_rate = if initial_correct > 0 {
            damaged as f64 / initial_correct as f64
        } else {
            0.0
        };

        StepAuditMetrics {
            step,
            mean_accuracy: correct_bits as f64 / total_bits as f64,
            exact_solve: dist_clean == 0,
            hamming_dist_to_clean: dist_clean,
            hamming_dist_to_noisy: dist_noisy,
            rescue_rate,
            damage_rate,
            net_rescue: rescue_rate - damage_rate,
            delta_s_norm: delta_sq.sqrt(),
            energy,
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Statistical Significance Tools: McNemar Exact Test & Bootstrap CI
// ---------------------------------------------------------------------------
pub struct StatisticalTests;

impl StatisticalTests {
    /// McNemar test for paired binary outcomes:
    /// Returns (b_discordant, c_discordant, chi_squared, p_value_approx)
    pub fn mcnemar(outcomes_a: &[bool], outcomes_b: &[bool]) -> (usize, usize, f64, f64) {
        assert_eq!(outcomes_a.len(), outcomes_b.len());
        let mut b = 0; // A correct, B incorrect
        let mut c = 0; // A incorrect, B correct

        for (&a, &b_val) in outcomes_a.iter().zip(outcomes_b.iter()) {
            if a && !b_val {
                b += 1;
            } else if !a && b_val {
                c += 1;
            }
        }

        let total_discordant = b + c;
        if total_discordant == 0 {
            return (b, c, 0.0, 1.0);
        }

        // Continuity-corrected chi-squared: (|b - c| - 1)^2 / (b + c)
        let num = ((b as f64 - c as f64).abs() - 1.0).max(0.0);
        let chi2 = (num * num) / total_discordant as f64;

        // Approximate p-value from chi2 with 1 degree of freedom: p = erfc(sqrt(chi2 / 2))
        // Using numerical complementary error function approximation
        let z = chi2.sqrt();
        let p_val = (-0.5 * chi2).exp() / (1.0 + 0.33267 * z);

        (b, c, chi2, p_val.clamp(0.0, 1.0))
    }

    /// Paired bootstrap confidence interval (95% CI) for mean difference: values_a - values_b
    pub fn paired_bootstrap_ci(
        values_a: &[f64],
        values_b: &[f64],
        num_resamples: usize,
        seed: u64,
    ) -> (f64, f64, f64) {
        assert_eq!(values_a.len(), values_b.len());
        let n = values_a.len();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let diffs: Vec<f64> = values_a
            .iter()
            .zip(values_b.iter())
            .map(|(&a, &b)| a - b)
            .collect();
        let mean_diff = diffs.iter().sum::<f64>() / n as f64;

        let mut bootstrap_means = Vec::with_capacity(num_resamples);
        for _ in 0..num_resamples {
            let mut sum = 0.0;
            for _ in 0..n {
                let idx = rng.gen_range(0..n);
                sum += diffs[idx];
            }
            bootstrap_means.push(sum / n as f64);
        }

        bootstrap_means.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let low_idx = (0.025 * num_resamples as f64) as usize;
        let high_idx = (0.975 * num_resamples as f64) as usize;

        (
            mean_diff,
            bootstrap_means[low_idx],
            bootstrap_means[high_idx],
        )
    }
}
