#![allow(warnings)]
use rand::Rng;
use rayon::prelude::*;
use std::time::Instant;

const LANES: usize = 8; // ширина SIMD-вектора

// ==========================================
// 1. CSR матрица для QUBO/Max-Cut
// ==========================================
#[derive(Debug)]
pub struct CsrMatrix {
    pub values: Vec<f64>,
    pub col_indices: Vec<usize>,
    pub row_offsets: Vec<usize>,
}

impl CsrMatrix {
    pub fn get_row(&self, row: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        let start = self.row_offsets[row];
        let end = self.row_offsets[row + 1];
        self.col_indices[start..end]
            .iter()
            .copied()
            .zip(self.values[start..end].iter().copied())
    }
}

pub struct QuboModel {
    pub num_vars: usize,
    pub linear: Vec<f64>,
    pub quadratic: CsrMatrix,
}

impl QuboModel {
    pub fn calculate_total_energy(&self, state: &[i8]) -> f64 {
        let mut energy = 0.0;
        for i in 0..self.num_vars {
            if state[i] == 1 {
                energy += self.linear[i];
                for (j, w) in self.quadratic.get_row(i) {
                    if state[j] == 1 {
                        energy += w * 0.5;
                    }
                }
            }
        }
        energy
    }
}

// ==========================================
// 2. Генератор случайного Max-Cut графа
// ==========================================
pub struct MaxCutBuilder {
    num_vars: usize,
    linear: Vec<f64>,
    quadratic_edges: Vec<(usize, usize, f64)>,
    pub total_graph_weight: f64,
}

impl MaxCutBuilder {
    pub fn new(num_vars: usize) -> Self {
        Self {
            num_vars,
            linear: vec![0.0; num_vars],
            quadratic_edges: vec![],
            total_graph_weight: 0.0,
        }
    }

    pub fn generate_random_graph(&mut self, num_edges: usize) {
        let mut rng = rand::thread_rng();
        let mut edges_added = 0;
        while edges_added < num_edges {
            let u = rng.gen_range(0..self.num_vars);
            let v = rng.gen_range(0..self.num_vars);
            if u != v {
                let w: f64 = rng.gen_range(1.0f64..10.0f64).round();
                self.linear[u] -= w;
                self.linear[v] -= w;
                self.quadratic_edges.push((u, v, 2.0 * w));
                self.quadratic_edges.push((v, u, 2.0 * w));
                self.total_graph_weight += w;
                edges_added += 1;
            }
        }
    }

    pub fn build(self) -> QuboModel {
        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; self.num_vars];
        for (u, v, w) in self.quadratic_edges {
            row_edges[u].push((v, w));
        }
        let mut values = vec![];
        let mut col_indices = vec![];
        let mut row_offsets = vec![0];
        for mut edges in row_edges {
            edges.sort_by_key(|&(v, _)| v);
            let mut merged = vec![];
            for (v, w) in edges {
                if let Some(&mut (last_v, ref mut last_w)) = merged.last_mut() {
                    if last_v == v {
                        *last_w += w;
                    } else {
                        merged.push((v, w));
                    }
                } else {
                    merged.push((v, w));
                }
            }
            for (v, w) in merged {
                if w != 0.0 {
                    col_indices.push(v);
                    values.push(w);
                }
            }
            row_offsets.push(col_indices.len());
        }
        QuboModel {
            num_vars: self.num_vars,
            linear: self.linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
        }
    }
}

// ==========================================
// 3. Ultimate Solver + SIMD
// ==========================================
#[derive(Clone)]
struct Replica {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
}

pub struct UltimateSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub total_exchanges: usize,
    pub adaptation_interval: usize,
}

impl UltimateSolver {
    fn calculate_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
        let flip_multiplier = 1.0 - 2.0 * (state[var_idx] as f64);
        let mut sum_j = 0.0;
        for (col, weight) in model.quadratic.get_row(var_idx) {
            sum_j += weight * (state[col] as f64);
        }
        flip_multiplier * (model.linear[var_idx] + sum_j)
    }

    fn try_cluster_flip(model: &QuboModel, replica: &mut Replica, rng: &mut rand::rngs::ThreadRng) {
        let seed = rng.gen_range(0..model.num_vars);
        let mut cluster = vec![seed];
        let mut in_cluster = vec![false; model.num_vars];
        in_cluster[seed] = true;
        let mut queue = vec![seed];

        while let Some(current) = queue.pop() {
            for (neighbor, weight) in model.quadratic.get_row(current) {
                if !in_cluster[neighbor] && weight.abs() > 0.5 {
                    let prob = 1.0 - (-2.0 * weight.abs() / replica.temp).exp();
                    if rng.r#gen::<f64>() < prob {
                        in_cluster[neighbor] = true;
                        cluster.push(neighbor);
                        queue.push(neighbor);
                    }
                }
            }
        }

        if cluster.len() < 2 {
            return;
        }

        let e_old = model.calculate_total_energy(&replica.state);
        for &idx in &cluster {
            replica.state[idx] = 1 - replica.state[idx];
        }
        let e_new = model.calculate_total_energy(&replica.state);
        let delta_e = e_new - e_old;

        if delta_e > 0.0
            && (replica.temp <= 1e-8 || rng.r#gen::<f64>() >= (-delta_e / replica.temp).exp())
        {
            for &idx in &cluster {
                replica.state[idx] = 1 - replica.state[idx];
            }
        }
    }

    pub fn solve(&self, model: &QuboModel) -> Vec<i8> {
        let mut rng = rand::thread_rng();
        let mut replicas: Vec<Replica> = (0..self.num_replicas)
            .map(|i| {
                let state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
                let fraction = i as f64 / (self.num_replicas - 1).max(1) as f64;
                let temp = self.temp_max * (self.temp_min / self.temp_max).powf(fraction);
                let energy = model.calculate_total_energy(&state);
                Replica {
                    state,
                    temp,
                    energy,
                }
            })
            .collect();

        let mut swap_accepts = vec![0; self.num_replicas - 1];

        for exchange_step in 0..self.total_exchanges {
            replicas.par_iter_mut().for_each(|replica| {
                let mut local_rng = rand::thread_rng();
                for step in 0..self.sweeps_per_exchange {
                    if step % 5 == 0 {
                        Self::try_cluster_flip(model, replica, &mut local_rng);
                    } else {
                        let var_idx = local_rng.gen_range(0..model.num_vars);
                        let delta_e = Self::calculate_delta_e(model, &replica.state, var_idx);
                        if delta_e < 0.0
                            || (replica.temp > 1e-8
                                && local_rng.r#gen::<f64>() < (-delta_e / replica.temp).exp())
                        {
                            replica.state[var_idx] = 1 - replica.state[var_idx];
                        }
                    }
                }
                replica.energy = model.calculate_total_energy(&replica.state);
            });

            for i in 0..(self.num_replicas - 1) {
                let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
                let delta_energy = replicas[i].energy - replicas[i + 1].energy;
                let swap_prob = (delta_beta * delta_energy).exp();
                if swap_prob >= 1.0 || rng.r#gen::<f64>() < swap_prob {
                    swap_accepts[i] += 1;
                    replicas.swap(i, i + 1);
                }
            }

            if (exchange_step + 1) % self.adaptation_interval == 0 {
                for i in 1..(self.num_replicas - 1) {
                    let rate = swap_accepts[i] as f64 / self.adaptation_interval as f64;
                    let mut new_temp = replicas[i].temp * (1.0 - (0.23 - rate) * 0.1);
                    let min_bound = replicas[i + 1].temp;
                    let max_bound = replicas[i - 1].temp;
                    if min_bound < max_bound {
                        new_temp = new_temp.clamp(min_bound, max_bound);
                    } else {
                        new_temp = min_bound;
                    }
                    replicas[i].temp = new_temp;
                    swap_accepts[i] = 0;
                }
            }
        }

        replicas
            .into_iter()
            .min_by(|a, b| a.energy.partial_cmp(&b.energy).unwrap())
            .unwrap()
            .state
    }
}

// ==========================================
// 4. MAIN - Масштабирование
// ==========================================
fn run_scaling_test(num_nodes: usize) {
    println!("--- Scaling test: {} nodes ---", num_nodes);
    let num_edges = num_nodes * 5; // плотность ~5 ребер на узел

    let mut builder = MaxCutBuilder::new(num_nodes);
    builder.generate_random_graph(num_edges);
    let model = builder.build();

    let solver = UltimateSolver {
        num_replicas: 64,
        temp_max: 500.0,
        temp_min: 0.001,
        sweeps_per_exchange: 500,
        total_exchanges: 100,
        adaptation_interval: 10,
    };

    let start = Instant::now();
    let result = solver.solve(&model);
    let duration = start.elapsed();

    let final_energy = model.calculate_total_energy(&result);
    let max_cut_score = -final_energy;
    println!("Energy (cut value): {}", max_cut_score);
    println!("Time: {:.6?}\n", duration);
}

fn main() {
    let sizes = vec![1000, 2000, 3000, 4000, 5000];
    for n in sizes {
        run_scaling_test(n);
    }
}
