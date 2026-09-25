//! Analytic Energy-Based Models with Exact Parameter Gradients.
//!
//! Provides:
//! 1. AnalyticBilinearEnergyNet: E_theta(s) = (lambda_obs / 2) sum ||s_v - x_v||^2 - sum_{(u,v)} ln cosh(beta s_v^T W_r s_u).
//! 2. Exact analytical state gradients: nabla_s E.
//! 3. Exact analytical parameter gradients: nabla_{W_r} E.
//! 4. Analytical Equilibrium Propagation (TRAIN-C) without unrolling history.
//! 5. Exact Reverse-Mode Unrolled Backpropagation (TRAIN-A).

use crate::learned_energy::CyclicGraph;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Debug)]
pub struct AnalyticBilinearEnergyNet {
    pub dim: usize,
    pub num_relations: usize,
    pub beta: f64,
    pub lambda_obs: f64,
    // Weights: W[r * dim * dim + i * dim + j]
    pub w: Vec<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub accuracy: f64,
    pub rescue_rate: f64,
    pub damage_rate: f64,
    pub net_rescue: f64,
    pub constraint_satisfaction: f64,
    pub mean_squared_error: f64,
}

impl AnalyticBilinearEnergyNet {
    pub fn new(dim: usize, num_relations: usize, beta: f64, lambda_obs: f64, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = (2.0 / (dim as f64)).sqrt();
        let w = (0..(num_relations * dim * dim))
            .map(|_| rng.gen_range(-scale..scale))
            .collect();

        Self {
            dim,
            num_relations,
            beta,
            lambda_obs,
            w,
        }
    }

    pub fn num_parameters(&self) -> usize {
        self.w.len()
    }

    #[inline]
    fn w_idx(&self, r: usize, i: usize, j: usize) -> usize {
        r * self.dim * self.dim + i * self.dim + j
    }

    /// Compute total energy: E_theta(s; x)
    pub fn energy(&self, s: &[Vec<f64>], graph: &CyclicGraph) -> f64 {
        let n = graph.num_nodes;
        let mut e_obs = 0.0;
        for v in 0..n {
            for i in 0..self.dim {
                let diff = s[v][i] - graph.noisy_states[v][i];
                e_obs += 0.5 * self.lambda_obs * diff * diff;
            }
        }

        let mut e_rel = 0.0;
        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;

            // Compute dot = s_v^T W_r s_u
            let mut dot = 0.0;
            for i in 0..self.dim {
                let mut row_dot = 0.0;
                for j in 0..self.dim {
                    row_dot += self.w[self.w_idx(r, i, j)] * s[u][j];
                }
                dot += s[v][i] * row_dot;
            }

            let arg = self.beta * dot;
            // ln cosh(x) = |x| + ln((1 + exp(-2|x|)) / 2) for stability
            let ln_cosh = if arg.abs() > 20.0 {
                arg.abs() - 2.0_f64.ln()
            } else {
                arg.cosh().ln()
            };
            e_rel += (1.0 / self.beta) * ln_cosh;
        }

        e_obs - e_rel
    }

    /// Analytical gradient of energy with respect to states s_v: nabla_s E
    pub fn compute_state_gradient(
        &self,
        s: &[Vec<f64>],
        graph: &CyclicGraph,
        grad_s: &mut [Vec<f64>],
    ) {
        let n = graph.num_nodes;
        for v in 0..n {
            grad_s[v].fill(0.0);
        }

        // 1. Observation anchoring: lambda_obs * (s_v - x_v)
        for v in 0..n {
            for i in 0..self.dim {
                grad_s[v][i] += self.lambda_obs * (s[v][i] - graph.noisy_states[v][i]);
            }
        }

        // 2. Pairwise constraint forces
        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;

            // dot = s_v^T W_r s_u
            let mut dot = 0.0;
            let mut w_su = vec![0.0; self.dim];
            for i in 0..self.dim {
                for j in 0..self.dim {
                    w_su[i] += self.w[self.w_idx(r, i, j)] * s[u][j];
                }
                dot += s[v][i] * w_su[i];
            }

            let factor = -(self.beta * dot).tanh();

            // Force on s_v: factor * (W_r s_u)
            for i in 0..self.dim {
                grad_s[v][i] += factor * w_su[i];
            }

            // Force on s_u: factor * (W_r^T s_v)
            for j in 0..self.dim {
                let mut wt_sv = 0.0;
                for i in 0..self.dim {
                    wt_sv += self.w[self.w_idx(r, i, j)] * s[v][i];
                }
                grad_s[u][j] += factor * wt_sv;
            }
        }
    }

    /// Analytical gradient of energy with respect to parameters W: nabla_W E
    pub fn compute_param_gradient(&self, s: &[Vec<f64>], graph: &CyclicGraph, grad_w: &mut [f64]) {
        grad_w.fill(0.0);

        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;

            let mut dot = 0.0;
            for i in 0..self.dim {
                for j in 0..self.dim {
                    dot += s[v][i] * self.w[self.w_idx(r, i, j)] * s[u][j];
                }
            }

            let factor = -(self.beta * dot).tanh();

            // nabla_{W_r[i, j]} E = factor * s_v[i] * s_u[j]
            for i in 0..self.dim {
                let svi = s[v][i];
                for j in 0..self.dim {
                    let idx = self.w_idx(r, i, j);
                    grad_w[idx] += factor * svi * s[u][j];
                }
            }
        }
    }

    /// Inference relaxation: relax state for  iterations
    pub fn relax(&self, graph: &CyclicGraph, steps: usize, lr: f64) -> Vec<Vec<f64>> {
        let n = graph.num_nodes;
        let mut s = graph.noisy_states.clone();
        let mut grad_s = vec![vec![0.0; self.dim]; n];

        for _ in 0..steps {
            self.compute_state_gradient(&s, graph, &mut grad_s);
            for v in 0..n {
                for i in 0..self.dim {
                    s[v][i] = (s[v][i] - lr * grad_s[v][i]).clamp(-1.0, 1.0);
                }
            }
        }

        s
    }

    /// Nudged relaxation for Equilibrium Propagation:
    /// Minimizes E_theta(s; x) + (beta_nudge / (2 * N * d)) * sum ||s - s^*||^2
    pub fn relax_nudged(
        &self,
        graph: &CyclicGraph,
        steps: usize,
        lr: f64,
        beta_nudge: f64,
    ) -> Vec<Vec<f64>> {
        let n = graph.num_nodes;
        let nd_f = (n * self.dim) as f64;
        let nudge_scale = beta_nudge / nd_f;
        let mut s = graph.noisy_states.clone();
        let mut grad_s = vec![vec![0.0; self.dim]; n];

        for _ in 0..steps {
            self.compute_state_gradient(&s, graph, &mut grad_s);
            for v in 0..n {
                for i in 0..self.dim {
                    let nudge = nudge_scale * (s[v][i] - graph.clean_states[v][i]);
                    s[v][i] = (s[v][i] - lr * (grad_s[v][i] + nudge)).clamp(-1.0, 1.0);
                }
            }
        }

        s
    }

    /// Exact analytical Equilibrium Propagation gradient computation for a batch.
    /// Computes dLoss/dW = (nabla_W E(s_nudged) - nabla_W E(s_free)) / beta_nudge
    pub fn compute_eq_prop_grad(
        &self,
        graphs: &[CyclicGraph],
        steps_free: usize,
        steps_nudged: usize,
        relax_lr: f64,
        beta_nudge: f64,
        accum_grad_w: &mut [f64],
    ) -> f64 {
        accum_grad_w.fill(0.0);
        let mut grad_free = vec![0.0; self.w.len()];
        let mut grad_nudged = vec![0.0; self.w.len()];
        let mut total_loss = 0.0;

        for graph in graphs {
            let n = graph.num_nodes;
            let nd_f = (n * self.dim) as f64;

            // 1. Free phase
            let s_free = self.relax(graph, steps_free, relax_lr);
            self.compute_param_gradient(&s_free, graph, &mut grad_free);

            for v in 0..n {
                for i in 0..self.dim {
                    let diff = s_free[v][i] - graph.clean_states[v][i];
                    total_loss += diff * diff / nd_f;
                }
            }

            // 2. Nudged phase
            let s_nudged = self.relax_nudged(graph, steps_nudged, relax_lr, beta_nudge);
            self.compute_param_gradient(&s_nudged, graph, &mut grad_nudged);

            // EqProp gradient of MSE loss:
            // dL/dW = (nabla_W E(s_nudged) - nabla_W E(s_free)) / beta_nudge
            for idx in 0..self.w.len() {
                accum_grad_w[idx] += (grad_nudged[idx] - grad_free[idx]) / beta_nudge;
            }
        }

        let num_graphs = graphs.len() as f64;
        for idx in 0..self.w.len() {
            accum_grad_w[idx] /= num_graphs;
        }

        total_loss / num_graphs
    }

    /// Step using analytical Equilibrium Propagation
    pub fn train_step_eq_prop(
        &mut self,
        graphs: &[CyclicGraph],
        lr_train: f64,
        steps_free: usize,
        steps_nudged: usize,
        relax_lr: f64,
        beta_nudge: f64,
    ) -> f64 {
        let mut grad_w = vec![0.0; self.w.len()];
        let loss = self.compute_eq_prop_grad(
            graphs,
            steps_free,
            steps_nudged,
            relax_lr,
            beta_nudge,
            &mut grad_w,
        );

        for idx in 0..self.w.len() {
            let update = (lr_train * grad_w[idx]).clamp(-0.5, 0.5);
            self.w[idx] -= update;
        }

        loss
    }

    /// Compute exact analytical reverse-mode BPTT gradient for a single graph.
    /// Loss: L = (1 / (N * d)) sum_{v, i} (s_v^{(T)}[i] - s_v^*[i])^2
    pub fn compute_bptt_grad_single(
        &self,
        graph: &CyclicGraph,
        unroll_steps: usize,
        relax_lr: f64,
        grad_w: &mut [f64],
    ) -> f64 {
        grad_w.fill(0.0);
        let n = graph.num_nodes;
        let nd_f = (n * self.dim) as f64;

        // Forward trajectory
        let mut s_hist = Vec::with_capacity(unroll_steps + 1);
        let mut u_hist = Vec::with_capacity(unroll_steps);
        let mut s = graph.noisy_states.clone();
        s_hist.push(s.clone());

        let mut g_s = vec![vec![0.0; self.dim]; n];
        for _ in 0..unroll_steps {
            self.compute_state_gradient(&s, graph, &mut g_s);
            let mut u = vec![vec![0.0; self.dim]; n];
            let mut next_s = vec![vec![0.0; self.dim]; n];
            for v in 0..n {
                for i in 0..self.dim {
                    let u_val = s[v][i] - relax_lr * g_s[v][i];
                    u[v][i] = u_val;
                    next_s[v][i] = u_val.clamp(-1.0, 1.0);
                }
            }
            s = next_s;
            u_hist.push(u);
            s_hist.push(s.clone());
        }

        // Final loss and adjoint initialization at step T
        let s_final = &s_hist[unroll_steps];
        let mut loss = 0.0;
        let mut bar_s = vec![vec![0.0; self.dim]; n];
        for v in 0..n {
            for i in 0..self.dim {
                let diff = s_final[v][i] - graph.clean_states[v][i];
                loss += diff * diff / nd_f;
                bar_s[v][i] = (2.0 / nd_f) * diff;
            }
        }

        // Backward reverse pass from step T-1 down to 0
        for t in (0..unroll_steps).rev() {
            let s_t = &s_hist[t];
            let u_t = &u_hist[t];

            // 1. Backprop through clamp: bar_u = bar_s * I(-1 <= u <= 1)
            let mut bar_u = vec![vec![0.0; self.dim]; n];
            for v in 0..n {
                for i in 0..self.dim {
                    if u_t[v][i] >= -1.0 && u_t[v][i] <= 1.0 {
                        bar_u[v][i] = bar_s[v][i];
                    } else {
                        bar_u[v][i] = 0.0;
                    }
                }
            }

            // Directional multiplier: bar_g = -relax_lr * bar_u
            let mut bar_g = vec![vec![0.0; self.dim]; n];
            for v in 0..n {
                for i in 0..self.dim {
                    bar_g[v][i] = -relax_lr * bar_u[v][i];
                }
            }

            // Direct pass-through from s^{(t)}: bar_s_prev = bar_u
            let mut next_bar_s = bar_u;

            // 2. Add observation gradient contribution to state:
            // H_obs = lambda_obs * sum bar_g_v^T (s_v - x_v) => dH/ds_v = lambda_obs * bar_g_v
            for v in 0..n {
                for i in 0..self.dim {
                    next_bar_s[v][i] += self.lambda_obs * bar_g[v][i];
                }
            }

            // 3. Add edge contributions to parameters W and states s
            for edge in &graph.edges {
                let u = edge.source;
                let v = edge.target;
                let r = edge.relation;

                // Precompute W_r * s_u and W_r^T * s_v
                let mut w_su = vec![0.0; self.dim];
                let mut wt_sv = vec![0.0; self.dim];
                for i in 0..self.dim {
                    for j in 0..self.dim {
                        let w_val = self.w[self.w_idx(r, i, j)];
                        w_su[i] += w_val * s_t[u][j];
                        wt_sv[j] += w_val * s_t[v][i];
                    }
                }

                // z_e = beta * s_v^T W_r s_u
                let mut dot = 0.0;
                for i in 0..self.dim {
                    dot += s_t[v][i] * w_su[i];
                }
                let z_e = self.beta * dot;
                let t_e = z_e.tanh();
                let s_e = 1.0 - t_e * t_e; // sech^2(z_e)

                // Precompute W_r * bar_g_u and W_r^T * bar_g_v
                let mut w_gu = vec![0.0; self.dim];
                let mut wt_gv = vec![0.0; self.dim];
                for i in 0..self.dim {
                    for j in 0..self.dim {
                        let w_val = self.w[self.w_idx(r, i, j)];
                        w_gu[i] += w_val * bar_g[u][j];
                        wt_gv[j] += w_val * bar_g[v][i];
                    }
                }

                // Q_e = bar_g_v^T W_r s_u + s_v^T W_r bar_g_u
                let mut q_e = 0.0;
                for i in 0..self.dim {
                    q_e += bar_g[v][i] * w_su[i] + s_t[v][i] * w_gu[i];
                }

                // Parameter gradient contribution:
                // dH_e / dW_r[i, j] = -s_e * q_e * (beta * s_v[i] * s_u[j]) - t_e * (bar_g_v[i] * s_u[j] + s_v[i] * bar_g_u[j])
                let sq_beta = s_e * q_e * self.beta;
                for i in 0..self.dim {
                    let svi = s_t[v][i];
                    let gvi = bar_g[v][i];
                    for j in 0..self.dim {
                        let suj = s_t[u][j];
                        let guj = bar_g[u][j];
                        let idx = self.w_idx(r, i, j);
                        let dw = -sq_beta * (svi * suj) - t_e * (gvi * suj + svi * guj);
                        grad_w[idx] += dw;
                    }
                }

                // State gradient contribution:
                // dH_e / ds_v = -s_e * q_e * (beta * W_r s_u) - t_e * (W_r bar_g_u)
                for i in 0..self.dim {
                    next_bar_s[v][i] += -sq_beta * w_su[i] - t_e * w_gu[i];
                }

                // dH_e / ds_u = -s_e * q_e * (beta * W_r^T s_v) - t_e * (W_r^T bar_g_v)
                for j in 0..self.dim {
                    next_bar_s[u][j] += -sq_beta * wt_sv[j] - t_e * wt_gv[j];
                }
            }

            bar_s = next_bar_s;
        }

        loss
    }

    /// Step using exact analytical reverse-mode BPTT across a batch
    pub fn train_step_analytic_bptt(
        &mut self,
        graphs: &[CyclicGraph],
        lr_train: f64,
        unroll_steps: usize,
        relax_lr: f64,
    ) -> f64 {
        let mut total_loss = 0.0;
        let mut accum_grad_w = vec![0.0; self.w.len()];
        let mut single_grad_w = vec![0.0; self.w.len()];

        for graph in graphs {
            let loss =
                self.compute_bptt_grad_single(graph, unroll_steps, relax_lr, &mut single_grad_w);
            total_loss += loss;
            for idx in 0..self.w.len() {
                accum_grad_w[idx] += single_grad_w[idx];
            }
        }

        let num_graphs = graphs.len() as f64;
        for idx in 0..self.w.len() {
            let update = (lr_train * accum_grad_w[idx] / num_graphs).clamp(-0.5, 0.5);
            self.w[idx] -= update;
        }

        total_loss / num_graphs
    }

    /// Compute evaluation metrics on a graph
    pub fn evaluate_metrics(&self, graph: &CyclicGraph, s: &[Vec<f64>]) -> Metrics {
        let n = graph.num_nodes;
        let mut total_elements = 0;
        let mut correct = 0;
        let mut initial_corrupted = 0;
        let mut initial_clean = 0;
        let mut rescued = 0;
        let mut damaged = 0;
        let mut total_sq_err = 0.0;

        for v in 0..n {
            for i in 0..self.dim {
                total_elements += 1;
                let true_sign = if graph.clean_states[v][i] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let init_sign = if graph.noisy_states[v][i] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let final_sign = if s[v][i] >= 0.0 { 1.0 } else { -1.0 };

                if final_sign == true_sign {
                    correct += 1;
                }

                if init_sign != true_sign {
                    initial_corrupted += 1;
                    if final_sign == true_sign {
                        rescued += 1;
                    }
                } else {
                    initial_clean += 1;
                    if final_sign != true_sign {
                        damaged += 1;
                    }
                }

                let diff = s[v][i] - graph.clean_states[v][i];
                total_sq_err += diff * diff;
            }
        }

        let rescue_rate = if initial_corrupted > 0 {
            rescued as f64 / initial_corrupted as f64
        } else {
            0.0
        };

        let damage_rate = if initial_clean > 0 {
            damaged as f64 / initial_clean as f64
        } else {
            0.0
        };

        let mut satisfied_edges = 0;
        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;

            let mut su_sign = vec![0.0; self.dim];
            let mut sv_sign = vec![0.0; self.dim];
            for i in 0..self.dim {
                su_sign[i] = if s[u][i] >= 0.0 { 1.0 } else { -1.0 };
                sv_sign[i] = if s[v][i] >= 0.0 { 1.0 } else { -1.0 };
            }

            let mut dot = 0.0;
            for i in 0..self.dim {
                for j in 0..self.dim {
                    dot += sv_sign[i] * self.w[self.w_idx(r, i, j)] * su_sign[j];
                }
            }
            if dot > 0.0 {
                satisfied_edges += 1;
            }
        }

        let constraint_satisfaction = if !graph.edges.is_empty() {
            satisfied_edges as f64 / graph.edges.len() as f64
        } else {
            1.0
        };

        Metrics {
            accuracy: correct as f64 / total_elements as f64,
            rescue_rate,
            damage_rate,
            net_rescue: rescue_rate - damage_rate,
            constraint_satisfaction,
            mean_squared_error: total_sq_err / total_elements as f64,
        }
    }
}
