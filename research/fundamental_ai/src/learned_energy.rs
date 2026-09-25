//! Learned Dual-Variable Energy Networks and Baselines for Cyclic Constraint Graphs (EXP-TEN-006).
//!
//! Evaluates whether a parameter-shared local energy interaction can be learned from data
//! and transferred to solve unseen larger cyclic constraint topologies through iterative relaxation.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// A directed constraint edge in a cyclic factor graph: source -relation-> target.
#[derive(Debug, Clone)]
pub struct ConstraintEdge {
    pub source: usize,
    pub target: usize,
    pub relation: usize,
}

/// A cyclic constraint graph with N nodes, each with ground-truth, noisy observation, and edges.
#[derive(Debug, Clone)]
pub struct CyclicGraph {
    pub num_nodes: usize,
    pub dim: usize,
    pub clean_states: Vec<Vec<f64>>, // [num_nodes][dim] in {-1.0, +1.0}
    pub noisy_states: Vec<Vec<f64>>, // [num_nodes][dim]
    pub edges: Vec<ConstraintEdge>,
}

/// Generate a dataset of cyclic permutation graphs.
/// Relations are fixed permutations pi_0, ..., pi_{num_relations-1} of dimension `dim`.
pub struct PermutationWorld {
    pub dim: usize,
    pub num_relations: usize,
    pub permutations: Vec<Vec<usize>>, // [num_relations][dim]
}

impl PermutationWorld {
    pub fn new(dim: usize, num_relations: usize, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut permutations = Vec::with_capacity(num_relations);
        for _ in 0..num_relations {
            let mut perm: Vec<usize> = (0..dim).collect();
            // Knuth shuffle
            for i in (1..dim).rev() {
                let j = rng.gen_range(0..=i);
                perm.swap(i, j);
            }
            permutations.push(perm);
        }
        Self {
            dim,
            num_relations,
            permutations,
        }
    }

    /// Generate a consistent cycle of length K with noise level `noise`.
    pub fn generate_consistent_cycle(
        &self,
        k: usize,
        noise: f64,
        rng: &mut ChaCha8Rng,
    ) -> CyclicGraph {
        assert!(k >= 3);
        // 1. Initial random spin state for node 0
        let mut clean = Vec::with_capacity(k);
        let mut current_spins: Vec<f64> = (0..self.dim)
            .map(|_| if rng.gen_bool(0.5) { 1.0 } else { -1.0 })
            .collect();
        clean.push(current_spins.clone());

        // 2. Sample random relations for hops 0 -> 1, ..., K-2 -> K-1
        let mut edges = Vec::with_capacity(k);
        for u in 0..(k - 1) {
            let rel = rng.gen_range(0..self.num_relations);
            let mut next_spins = vec![0.0; self.dim];
            for i in 0..self.dim {
                next_spins[self.permutations[rel][i]] = current_spins[i];
            }
            edges.push(ConstraintEdge {
                source: u,
                target: u + 1,
                relation: rel,
            });
            clean.push(next_spins.clone());
            current_spins = next_spins;
        }

        // 3. For the closing hop (K-1 -> 0), pick the relation that best or exactly closes the cycle
        // In this permutation world, let hop K-1 -> 0 use a relation closing the cycle
        let closing_rel = rng.gen_range(0..self.num_relations);
        // Force ground-truth closure: clean[0] is aligned with closing_rel applied to clean[K-1]
        // to guarantee strict mathematical consistency around the cycle
        let mut closed_node0 = vec![0.0; self.dim];
        for i in 0..self.dim {
            closed_node0[self.permutations[closing_rel][i]] = current_spins[i];
        }
        clean[0] = closed_node0; // Cycle is strictly consistent: 0 -> 1 -> ... -> K-1 -> 0
        edges.push(ConstraintEdge {
            source: k - 1,
            target: 0,
            relation: closing_rel,
        });

        // 4. Generate noisy observed states
        let mut noisy = Vec::with_capacity(k);
        for v in 0..k {
            let mut nv = clean[v].clone();
            for val in nv.iter_mut() {
                if rng.gen_bool(noise) {
                    *val = -*val;
                }
            }
            noisy.push(nv);
        }

        CyclicGraph {
            num_nodes: k,
            dim: self.dim,
            clean_states: clean,
            noisy_states: noisy,
            edges,
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Candidate: Learned Dual-Variable Energy Network (EXP-TEN-006)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LearnedDualEnergyNetwork {
    pub dim: usize,
    pub latent_dim: usize,
    pub num_relations: usize,
    pub beta: f64,
    pub lambda_obs: f64,
    // Weights:
    // w1[r * latent_dim * dim + k * dim + i]
    pub w1: Vec<f64>,
    // w2[r * latent_dim * dim + k * dim + i]
    pub w2: Vec<f64>,
    // u_lat[k * latent_dim + l]
    pub u_lat: Vec<f64>,
    // vz[k * dim + i]
    pub vz: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct RelaxationResult {
    pub states: Vec<Vec<f64>>,
    pub latents: Vec<Vec<f64>>,
    pub energies: Vec<f64>,
    pub limit_cycle: bool,
}

impl LearnedDualEnergyNetwork {
    pub fn new(dim: usize, latent_dim: usize, num_relations: usize, beta: f64, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = (2.0 / (dim as f64)).sqrt();

        let mut sample_weights = |count: usize| -> Vec<f64> {
            (0..count).map(|_| rng.gen_range(-scale..scale)).collect()
        };

        Self {
            dim,
            latent_dim,
            num_relations,
            beta,
            lambda_obs: 1.0,
            w1: sample_weights(num_relations * latent_dim * dim),
            w2: sample_weights(num_relations * latent_dim * dim),
            u_lat: sample_weights(latent_dim * latent_dim),
            vz: sample_weights(latent_dim * dim),
        }
    }

    pub fn num_parameters(&self) -> usize {
        self.w1.len() + self.w2.len() + self.u_lat.len() + self.vz.len()
    }

    /// Compute total system energy E(s, z; x, G).
    pub fn energy(&self, s: &[Vec<f64>], z: &[Vec<f64>], graph: &CyclicGraph) -> f64 {
        let mut total_e = 0.0;

        // 1. Observation anchoring energy
        for v in 0..graph.num_nodes {
            for i in 0..self.dim {
                let diff = s[v][i] - graph.noisy_states[v][i];
                total_e += 0.5 * self.lambda_obs * diff * diff;
            }
        }

        // 2. Latent confinement
        for v in 0..graph.num_nodes {
            for k in 0..self.latent_dim {
                total_e += 0.5 * z[v][k] * z[v][k];
            }
        }

        // 3. Relational interaction energy over edges
        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;
            let rel_offset = r * self.latent_dim * self.dim;

            for k in 0..self.latent_dim {
                let row_offset = rel_offset + k * self.dim;
                let mut dot1 = 0.0;
                let mut dot2 = 0.0;
                for i in 0..self.dim {
                    dot1 += self.w1[row_offset + i] * s[u][i];
                    dot2 += self.w2[row_offset + i] * s[v][i];
                }

                let mut lat_coupling = 0.0;
                let u_row = k * self.latent_dim;
                for l in 0..self.latent_dim {
                    lat_coupling += self.u_lat[u_row + l] * z[u][l];
                }
                let arg = self.beta * (dot1 + dot2 + lat_coupling * z[v][k]);
                // - (1/beta) * ln cosh(arg)
                let abs_arg = arg.abs();
                let ln_cosh = if abs_arg > 20.0 {
                    abs_arg - std::f64::consts::LN_2
                } else {
                    arg.cosh().ln()
                };
                total_e -= ln_cosh / self.beta;
            }
        }

        // 4. Observable-latent coupling
        for v in 0..graph.num_nodes {
            for k in 0..self.latent_dim {
                let row = k * self.dim;
                let mut dot = 0.0;
                for i in 0..self.dim {
                    dot += self.vz[row + i] * s[v][i];
                }
                total_e -= dot * z[v][k];
            }
        }

        total_e
    }

    /// Compute exact analytical gradients: grad_s[v] = nabla_{s_v} E, grad_z[v] = nabla_{z_v} E.
    pub fn compute_gradients(
        &self,
        s: &[Vec<f64>],
        z: &[Vec<f64>],
        graph: &CyclicGraph,
        grad_s: &mut [Vec<f64>],
        grad_z: &mut [Vec<f64>],
    ) {
        let n = graph.num_nodes;
        for v in 0..n {
            grad_s[v].fill(0.0);
            grad_z[v].fill(0.0);
        }

        // 1. Anchoring gradient: lambda_obs * (s_v - x_v)
        for v in 0..n {
            for i in 0..self.dim {
                grad_s[v][i] += self.lambda_obs * (s[v][i] - graph.noisy_states[v][i]);
            }
            for k in 0..self.latent_dim {
                grad_z[v][k] += z[v][k];
            }
        }

        // 2. Observable-latent coupling: - V_z^T z_v to s_v, - V_z s_v to z_v
        for v in 0..n {
            for k in 0..self.latent_dim {
                let row = k * self.dim;
                let zk = z[v][k];
                for i in 0..self.dim {
                    grad_s[v][i] -= self.vz[row + i] * zk;
                    grad_z[v][k] -= self.vz[row + i] * s[v][i];
                }
            }
        }

        // 3. Relational potential gradient over edges
        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;
            let rel_offset = r * self.latent_dim * self.dim;

            for k in 0..self.latent_dim {
                let row_offset = rel_offset + k * self.dim;
                let mut dot1 = 0.0;
                let mut dot2 = 0.0;
                for i in 0..self.dim {
                    dot1 += self.w1[row_offset + i] * s[u][i];
                    dot2 += self.w2[row_offset + i] * s[v][i];
                }

                let mut lat_coupling = 0.0;
                let u_row = k * self.latent_dim;
                for l in 0..self.latent_dim {
                    lat_coupling += self.u_lat[u_row + l] * z[u][l];
                }

                let arg = self.beta * (dot1 + dot2 + lat_coupling * z[v][k]);
                // Derivative of - (1/beta) ln cosh(beta * y) w.r.t y is - tanh(beta * y)
                let factor = -arg.tanh();

                // To s_u via w1: factor * w1[k, i]
                for i in 0..self.dim {
                    grad_s[u][i] += factor * self.w1[row_offset + i];
                }
                // To s_v via w2: factor * w2[k, i]
                for i in 0..self.dim {
                    grad_s[v][i] += factor * self.w2[row_offset + i];
                }
                // To z_v: factor * lat_coupling
                grad_z[v][k] += factor * lat_coupling;
                // To z_u: factor * z_v[k] * u_lat[k, l]
                let zk_v = z[v][k];
                for l in 0..self.latent_dim {
                    grad_z[u][l] += factor * zk_v * self.u_lat[u_row + l];
                }
            }
        }
    }

    /// Inference: Relax network for `steps` iterations.
    /// Returns RelaxationResult.
    pub fn relax(&self, graph: &CyclicGraph, steps: usize, lr: f64) -> RelaxationResult {
        let n = graph.num_nodes;
        let mut s = graph.noisy_states.clone();
        let mut z = vec![vec![0.0; self.latent_dim]; n];

        let mut energies = Vec::with_capacity(steps + 1);
        energies.push(self.energy(&s, &z, graph));

        let mut grad_s = vec![vec![0.0; self.dim]; n];
        let mut grad_z = vec![vec![0.0; self.latent_dim]; n];

        let mut trajectory = Vec::with_capacity(steps);

        for _ in 0..steps {
            self.compute_gradients(&s, &z, graph, &mut grad_s, &mut grad_z);

            // Gradient descent step: s <- clamp(s - lr * grad_s, -1, 1)
            for v in 0..n {
                for i in 0..self.dim {
                    s[v][i] = (s[v][i] - lr * grad_s[v][i]).clamp(-1.0, 1.0);
                }
                for k in 0..self.latent_dim {
                    z[v][k] -= lr * grad_z[v][k];
                }
            }

            let e = self.energy(&s, &z, graph);
            energies.push(e);
            trajectory.push(s.clone());
        }

        // Limit cycle detection: check if state repeats with period 2, 3, or 4
        let mut limit_cycle = false;
        if steps >= 8 {
            let last = &trajectory[steps - 1];
            for p in 2..=4 {
                if steps > p {
                    let prev = &trajectory[steps - 1 - p];
                    let mut diff = 0.0;
                    for v in 0..n {
                        for i in 0..self.dim {
                            diff += (last[v][i] - prev[v][i]).abs();
                        }
                    }
                    if diff < 1e-4 {
                        limit_cycle = true;
                        break;
                    }
                }
            }
        }

        RelaxationResult {
            states: s,
            latents: z,
            energies,
            limit_cycle,
        }
    }

    pub fn get_params(&self) -> Vec<f64> {
        let mut p = Vec::with_capacity(self.num_parameters());
        p.extend_from_slice(&self.w1);
        p.extend_from_slice(&self.w2);
        p.extend_from_slice(&self.u_lat);
        p.extend_from_slice(&self.vz);
        p
    }

    pub fn set_params(&mut self, p: &[f64]) {
        assert_eq!(p.len(), self.num_parameters());
        let mut offset = 0;
        let l1 = self.w1.len();
        self.w1.copy_from_slice(&p[offset..offset + l1]);
        offset += l1;
        let l2 = self.w2.len();
        self.w2.copy_from_slice(&p[offset..offset + l2]);
        offset += l2;
        let lu = self.u_lat.len();
        self.u_lat.copy_from_slice(&p[offset..offset + lu]);
        offset += lu;
        let lv = self.vz.len();
        self.vz.copy_from_slice(&p[offset..offset + lv]);
    }

    /// Single training step with Backpropagation Through Time (unrolled 5 steps).
    pub fn train_step_bptt(
        &mut self,
        graphs: &[CyclicGraph],
        lr_train: f64,
        unroll_steps: usize,
        relax_lr: f64,
    ) -> f64 {
        let eps = 1e-4;
        let mut params = self.get_params();

        let compute_batch_loss = |model: &LearnedDualEnergyNetwork| -> f64 {
            let mut loss = 0.0;
            for g in graphs {
                let res = model.relax(g, unroll_steps, relax_lr);
                for v in 0..g.num_nodes {
                    for i in 0..g.dim {
                        let diff = res.states[v][i] - g.clean_states[v][i];
                        loss += diff * diff;
                    }
                }
            }
            loss / (graphs.len() * graphs[0].num_nodes * graphs[0].dim) as f64
        };

        let base_loss = compute_batch_loss(self);
        let mut grads = vec![0.0; params.len()];

        for idx in 0..params.len() {
            let orig = params[idx];
            params[idx] = orig + eps;
            self.set_params(&params);
            let loss_plus = compute_batch_loss(self);
            params[idx] = orig;
            grads[idx] = ((loss_plus - base_loss) / eps).clamp(-5.0, 5.0);
        }

        for idx in 0..params.len() {
            params[idx] -= lr_train * grads[idx];
        }
        self.set_params(&params);

        base_loss
    }
}

// ---------------------------------------------------------------------------
// 2. Baseline B1: Recurrent Attention Network (Directed Cascade)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RecurrentAttentionGraph {
    pub dim: usize,
    pub num_relations: usize,
    pub w_q: Vec<f64>,   // dim x dim
    pub w_k: Vec<f64>,   // dim x dim
    pub w_v: Vec<f64>,   // dim x dim
    pub w_rel: Vec<f64>, // num_relations x dim
}

impl RecurrentAttentionGraph {
    pub fn new(dim: usize, num_relations: usize, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = (2.0 / (dim as f64)).sqrt();
        let mut init = |n: usize| (0..n).map(|_| rng.gen_range(-scale..scale)).collect();

        Self {
            dim,
            num_relations,
            w_q: init(dim * dim),
            w_k: init(dim * dim),
            w_v: init(dim * dim),
            w_rel: init(num_relations * dim),
        }
    }

    pub fn num_parameters(&self) -> usize {
        self.w_q.len() + self.w_k.len() + self.w_v.len() + self.w_rel.len()
    }

    pub fn step(&self, s: &[Vec<f64>], graph: &CyclicGraph) -> Vec<Vec<f64>> {
        let _n = graph.num_nodes;
        let mut next_s = s.to_vec();

        // Edge-conditioned attention update
        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;

            let mut q = vec![0.0; self.dim];
            let mut k = vec![0.0; self.dim];
            let mut val = vec![0.0; self.dim];

            for i in 0..self.dim {
                for j in 0..self.dim {
                    q[i] += self.w_q[i * self.dim + j] * s[v][j];
                    k[i] += self.w_k[i * self.dim + j] * s[u][j];
                    val[i] += self.w_v[i * self.dim + j] * s[u][j];
                }
                k[i] += self.w_rel[r * self.dim + i];
            }

            let mut dot = 0.0;
            for i in 0..self.dim {
                dot += q[i] * k[i];
            }
            let gate = (dot / (self.dim as f64).sqrt()).tanh();

            for i in 0..self.dim {
                next_s[v][i] = (next_s[v][i] + gate * val[i]).clamp(-1.0, 1.0);
            }
        }
        next_s
    }

    pub fn unroll(&self, graph: &CyclicGraph, steps: usize) -> (Vec<Vec<f64>>, bool) {
        let mut s = graph.noisy_states.clone();
        let mut trajectory = Vec::with_capacity(steps);

        for _ in 0..steps {
            s = self.step(&s, graph);
            trajectory.push(s.clone());
        }

        let mut limit_cycle = false;
        if steps >= 8 {
            let last = &trajectory[steps - 1];
            for p in 2..=4 {
                if steps > p {
                    let prev = &trajectory[steps - 1 - p];
                    let mut diff = 0.0;
                    for v in 0..graph.num_nodes {
                        for i in 0..self.dim {
                            diff += (last[v][i] - prev[v][i]).abs();
                        }
                    }
                    if diff < 1e-4 {
                        limit_cycle = true;
                        break;
                    }
                }
            }
        }

        (s, limit_cycle)
    }

    pub fn get_params(&self) -> Vec<f64> {
        let mut p = Vec::with_capacity(self.num_parameters());
        p.extend_from_slice(&self.w_q);
        p.extend_from_slice(&self.w_k);
        p.extend_from_slice(&self.w_v);
        p.extend_from_slice(&self.w_rel);
        p
    }

    pub fn set_params(&mut self, p: &[f64]) {
        assert_eq!(p.len(), self.num_parameters());
        let mut offset = 0;
        let lq = self.w_q.len();
        self.w_q.copy_from_slice(&p[offset..offset + lq]);
        offset += lq;
        let lk = self.w_k.len();
        self.w_k.copy_from_slice(&p[offset..offset + lk]);
        offset += lk;
        let lv = self.w_v.len();
        self.w_v.copy_from_slice(&p[offset..offset + lv]);
        offset += lv;
        let lr = self.w_rel.len();
        self.w_rel.copy_from_slice(&p[offset..offset + lr]);
    }

    pub fn train_step(&mut self, graphs: &[CyclicGraph], lr: f64, steps: usize) -> f64 {
        let eps = 1e-4;
        let mut params = self.get_params();

        let compute_loss = |model: &RecurrentAttentionGraph| -> f64 {
            let mut l = 0.0;
            for g in graphs {
                let (final_s, _) = model.unroll(g, steps);
                for v in 0..g.num_nodes {
                    for i in 0..g.dim {
                        let d = final_s[v][i] - g.clean_states[v][i];
                        l += d * d;
                    }
                }
            }
            l / (graphs.len() * graphs[0].num_nodes * graphs[0].dim) as f64
        };

        let base_loss = compute_loss(self);
        let mut grads = vec![0.0; params.len()];

        for i in 0..params.len() {
            let orig = params[i];
            params[i] = orig + eps;
            self.set_params(&params);
            let l_plus = compute_loss(self);
            params[i] = orig;
            grads[i] = ((l_plus - base_loss) / eps).clamp(-5.0, 5.0);
        }

        for i in 0..params.len() {
            params[i] -= lr * grads[i];
        }
        self.set_params(&params);

        base_loss
    }
}

// ---------------------------------------------------------------------------
// 3. Baseline B2: Graph Neural Network / Message Passing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GNNMessagePassing {
    pub dim: usize,
    pub num_relations: usize,
    pub w_msg: Vec<f64>,  // dim x dim
    pub w_rel: Vec<f64>,  // num_relations x dim
    pub w_node: Vec<f64>, // dim x dim
}

impl GNNMessagePassing {
    pub fn new(dim: usize, num_relations: usize, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = (2.0 / (dim as f64)).sqrt();
        let mut init = |n: usize| (0..n).map(|_| rng.gen_range(-scale..scale)).collect();

        Self {
            dim,
            num_relations,
            w_msg: init(dim * dim),
            w_rel: init(num_relations * dim),
            w_node: init(dim * dim),
        }
    }

    pub fn num_parameters(&self) -> usize {
        self.w_msg.len() + self.w_rel.len() + self.w_node.len()
    }

    pub fn step(&self, s: &[Vec<f64>], graph: &CyclicGraph) -> Vec<Vec<f64>> {
        let n = graph.num_nodes;
        let mut messages = vec![vec![0.0; self.dim]; n];

        for edge in &graph.edges {
            let u = edge.source;
            let v = edge.target;
            let r = edge.relation;

            for i in 0..self.dim {
                let mut m = 0.0;
                for j in 0..self.dim {
                    m += self.w_msg[i * self.dim + j] * s[u][j];
                }
                m += self.w_rel[r * self.dim + i];
                messages[v][i] += m.tanh();
            }
        }

        let mut next_s = vec![vec![0.0; self.dim]; n];
        for v in 0..n {
            for i in 0..self.dim {
                let mut upd = 0.0;
                for j in 0..self.dim {
                    upd += self.w_node[i * self.dim + j] * s[v][j];
                }
                next_s[v][i] = (upd + messages[v][i]).clamp(-1.0, 1.0);
            }
        }
        next_s
    }

    pub fn unroll(&self, graph: &CyclicGraph, steps: usize) -> (Vec<Vec<f64>>, bool) {
        let mut s = graph.noisy_states.clone();
        let mut trajectory = Vec::with_capacity(steps);

        for _ in 0..steps {
            s = self.step(&s, graph);
            trajectory.push(s.clone());
        }

        let mut limit_cycle = false;
        if steps >= 8 {
            let last = &trajectory[steps - 1];
            for p in 2..=4 {
                if steps > p {
                    let prev = &trajectory[steps - 1 - p];
                    let mut diff = 0.0;
                    for v in 0..graph.num_nodes {
                        for i in 0..self.dim {
                            diff += (last[v][i] - prev[v][i]).abs();
                        }
                    }
                    if diff < 1e-4 {
                        limit_cycle = true;
                        break;
                    }
                }
            }
        }

        (s, limit_cycle)
    }

    pub fn get_params(&self) -> Vec<f64> {
        let mut p = Vec::with_capacity(self.num_parameters());
        p.extend_from_slice(&self.w_msg);
        p.extend_from_slice(&self.w_rel);
        p.extend_from_slice(&self.w_node);
        p
    }

    pub fn set_params(&mut self, p: &[f64]) {
        assert_eq!(p.len(), self.num_parameters());
        let mut offset = 0;
        let lm = self.w_msg.len();
        self.w_msg.copy_from_slice(&p[offset..offset + lm]);
        offset += lm;
        let lr = self.w_rel.len();
        self.w_rel.copy_from_slice(&p[offset..offset + lr]);
        offset += lr;
        let ln = self.w_node.len();
        self.w_node.copy_from_slice(&p[offset..offset + ln]);
    }

    pub fn train_step(&mut self, graphs: &[CyclicGraph], lr: f64, steps: usize) -> f64 {
        let eps = 1e-4;
        let mut params = self.get_params();

        let compute_loss = |model: &GNNMessagePassing| -> f64 {
            let mut l = 0.0;
            for g in graphs {
                let (final_s, _) = model.unroll(g, steps);
                for v in 0..g.num_nodes {
                    for i in 0..g.dim {
                        let d = final_s[v][i] - g.clean_states[v][i];
                        l += d * d;
                    }
                }
            }
            l / (graphs.len() * graphs[0].num_nodes * graphs[0].dim) as f64
        };

        let base_loss = compute_loss(self);
        let mut grads = vec![0.0; params.len()];

        for i in 0..params.len() {
            let orig = params[i];
            params[i] = orig + eps;
            self.set_params(&params);
            let l_plus = compute_loss(self);
            params[i] = orig;
            grads[i] = ((l_plus - base_loss) / eps).clamp(-5.0, 5.0);
        }

        for i in 0..params.len() {
            params[i] -= lr * grads[i];
        }
        self.set_params(&params);

        base_loss
    }
}
