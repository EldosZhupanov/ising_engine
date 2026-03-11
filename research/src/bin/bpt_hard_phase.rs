#![allow(warnings)]
use rand::Rng;
use rayon::prelude::*;
use std::f64::consts::E;
use std::time::Instant;

// ==========================================
// 1. ВЫЧИСЛИТЕЛЬНОЕ ЯДРО (CSR Format)
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
    pub energy_offset: f64,
}
impl QuboModel {
    pub fn calculate_total_energy(&self, state: &[i8]) -> f64 {
        let mut energy = self.energy_offset;
        for i in 0..self.num_vars {
            if state[i] == 1 {
                energy += self.linear[i];
                for (j, weight) in self.quadratic.get_row(i) {
                    if state[j] == 1 {
                        energy += weight * 0.5;
                    }
                }
            }
        }
        energy
    }
}

// ==========================================
// 2. ГЕНЕРАТОР: BOOLEAN PYTHAGOREAN TRIPLES
// ==========================================
pub struct PythagoreanBuilder {
    pub num_vars: usize,
    pub triples: Vec<(usize, usize, usize)>,
}

impl PythagoreanBuilder {
    pub fn new(n: usize) -> Self {
        let mut triples = vec![];
        for a in 1..=n {
            for b in a..=n {
                let c_sq = (a * a + b * b) as f64;
                let c = c_sq.sqrt() as usize;
                if c > n {
                    break;
                }
                if c * c == a * a + b * b {
                    triples.push((a, b, c));
                }
            }
        }
        Self {
            num_vars: n,
            triples,
        }
    }

    pub fn build_qubo(&self) -> QuboModel {
        let mut linear = vec![0.0; self.num_vars];
        let mut quadratic_edges = vec![];
        let mut energy_offset = 0.0;

        for &(a, b, c) in &self.triples {
            let u = a - 1;
            let v = b - 1;
            let w = c - 1;

            linear[u] -= 1.0;
            linear[v] -= 1.0;
            linear[w] -= 1.0;

            quadratic_edges.push((u, v, 1.0));
            quadratic_edges.push((v, u, 1.0));
            quadratic_edges.push((v, w, 1.0));
            quadratic_edges.push((w, v, 1.0));
            quadratic_edges.push((u, w, 1.0));
            quadratic_edges.push((w, u, 1.0));

            energy_offset += 1.0;
        }

        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; self.num_vars];
        for (u, v, w) in quadratic_edges {
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
            linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
            energy_offset,
        }
    }
}

// ==========================================
// 3. RESEARCH SOLVER
// ==========================================
#[derive(Clone)]
struct Replica {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
    priority_map: Vec<f64>,
}

pub struct HybridSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub max_exchanges: usize,
    pub adaptation_interval: usize,
}
impl HybridSolver {
    fn calculate_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
        let flip_multiplier = 1.0 - 2.0 * (state[var_idx] as f64);
        let mut sum_j = 0.0;
        for (col, weight) in model.quadratic.get_row(var_idx) {
            sum_j += weight * (state[col] as f64);
        }
        flip_multiplier * (model.linear[var_idx] + sum_j)
    }

    pub fn solve_until_zero(&self, model: &QuboModel) -> (bool, f64, usize) {
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
                    priority_map: vec![1.0; model.num_vars],
                }
            })
            .collect();

        let mut swap_accepts = vec![0; self.num_replicas - 1];

        for exchange_step in 0..self.max_exchanges {
            if let Some(winner) = replicas.iter().find(|r| r.energy <= 0.0) {
                return (true, 0.0, exchange_step);
            }

            replicas.par_iter_mut().for_each(|replica| {
                let mut local_rng = rand::thread_rng();
                for _ in 0..self.sweeps_per_exchange {
                    let var_idx = local_rng.gen_range(0..model.num_vars);
                    if local_rng.gen_range(0.0_f64..1.0_f64) > replica.priority_map[var_idx] {
                        continue;
                    }

                    let delta_e = Self::calculate_delta_e(model, &replica.state, var_idx);
                    let mut accepted = false;

                    if delta_e < 0.0 {
                        accepted = true;
                        replica.priority_map[var_idx] =
                            (replica.priority_map[var_idx] + 0.15).min(1.0);
                    } else if replica.temp > 1e-8
                        && local_rng.gen_range(0.0_f64..1.0_f64) < E.powf(-delta_e / replica.temp)
                    {
                        accepted = true;
                        replica.priority_map[var_idx] =
                            (replica.priority_map[var_idx] + 0.02).min(1.0);
                    } else {
                        replica.priority_map[var_idx] =
                            (replica.priority_map[var_idx] - 0.01).max(0.3);
                    }

                    if accepted {
                        replica.state[var_idx] = 1 - replica.state[var_idx];
                        for (neighbor, _) in model.quadratic.get_row(var_idx) {
                            replica.priority_map[neighbor] =
                                (replica.priority_map[neighbor] + 0.1).min(1.0);
                        }
                    }
                }
                replica.energy = model.calculate_total_energy(&replica.state);
            });

            for i in 0..(self.num_replicas - 1) {
                let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
                let delta_energy = replicas[i].energy - replicas[i + 1].energy;
                let swap_prob = (delta_beta * delta_energy).exp();
                if swap_prob >= 1.0 || rng.gen_range(0.0_f64..1.0_f64) < swap_prob {
                    swap_accepts[i] += 1;
                    let temp_state = replicas[i].state.clone();
                    replicas[i].state = replicas[i + 1].state.clone();
                    replicas[i + 1].state = temp_state;
                    let temp_map = replicas[i].priority_map.clone();
                    replicas[i].priority_map = replicas[i + 1].priority_map.clone();
                    replicas[i + 1].priority_map = temp_map;
                    let temp_energy = replicas[i].energy;
                    replicas[i].energy = replicas[i + 1].energy;
                    replicas[i + 1].energy = temp_energy;
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

        let best_energy = replicas
            .iter()
            .map(|r| r.energy)
            .fold(f64::INFINITY, f64::min);
        (false, best_energy, self.max_exchanges)
    }
}

// ==========================================
// 4. MAIN - ЭКСПЕДИЦИЯ К 7825
// ==========================================
fn main() {
    println!("🌋 ЭКСПЕДИЦИЯ BPT: Поиск математического дна (7825)");
    println!("Ожидаем легкое решение в начале и тотальную фрустрацию в конце.");
    println!("----------------------------------------------------------------------------------");
    println!(
        "{:>6} | {:>10} | {:>10} | {:>12} | {:>15}",
        "N", "Троек(M)", "Время(ms)", "Обмены", "Конфликты(E)"
    );
    println!("----------------------------------------------------------------------------------");

    let checkpoints = vec![2500, 5000, 7000, 7500, 7800, 7824, 7825];

    // Жесткий, но конечный лимит, чтобы мы не ждали 4 года, как суперкомпьютер Stampede
    let solver = HybridSolver {
        num_replicas: 128,
        temp_max: 500.0,
        temp_min: 0.001,
        sweeps_per_exchange: 200,
        max_exchanges: 2000, // Лимит в 2000 шагов. Хватит ли этого для N=7500? Увидим.
        adaptation_interval: 20,
    };

    for &n in &checkpoints {
        let builder = PythagoreanBuilder::new(n);
        let model = builder.build_qubo();

        let start = Instant::now();
        let (success, final_energy, exchanges) = solver.solve_until_zero(&model);
        let duration = start.elapsed().as_millis();

        let marker = if success { "✅ SAT" } else { "❌ STUCK" };
        let ultimate_marker = if n == 7825 && !success {
            "💀 UNSAT (Математически)"
        } else {
            marker
        };

        println!(
            "{:>6} | {:>10} | {:>10} | {:>12} | {:>12} {}",
            n,
            builder.triples.len(),
            duration,
            exchanges,
            final_energy,
            ultimate_marker
        );
    }

    println!("----------------------------------------------------------------------------------");
    println!("Вывод: Мы увидим, как эвристика ломается по мере роста плотности троек.");
}
