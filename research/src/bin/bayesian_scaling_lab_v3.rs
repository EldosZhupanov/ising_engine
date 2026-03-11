#![allow(warnings)]
use rand::Rng;
use rayon::prelude::*;
use std::f64::consts::E;

// ==========================================
// 1. ВЫЧИСЛИТЕЛЬНОЕ ЯДРО
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
// 2. 3-SAT КОМПИЛЯТОР
// ==========================================
pub struct Sat3Builder {
    pub num_vars: usize,
    pub clauses: Vec<(i32, i32, i32)>,
}
impl Sat3Builder {
    pub fn generate_random(num_vars: usize, num_clauses: usize) -> Self {
        let mut rng = rand::thread_rng();
        let mut clauses = vec![];
        for _ in 0..num_clauses {
            let mut vars = vec![];
            while vars.len() < 3 {
                let v = rng.gen_range(1..=num_vars as i32);
                if !vars.contains(&v) {
                    vars.push(v);
                }
            }
            let l1 = if rng.gen_bool(0.5) { vars[0] } else { -vars[0] };
            let l2 = if rng.gen_bool(0.5) { vars[1] } else { -vars[1] };
            let l3 = if rng.gen_bool(0.5) { vars[2] } else { -vars[2] };
            clauses.push((l1, l2, l3));
        }
        Self { num_vars, clauses }
    }

    pub fn build_qubo(&self) -> QuboModel {
        let m = self.clauses.len();
        let qubo_vars = 3 * m;
        let linear = vec![-1.0; qubo_vars];
        let mut quadratic_edges = vec![];

        let get_literal = |node_idx: usize| -> i32 {
            let clause_idx = node_idx / 3;
            let lit_pos = node_idx % 3;
            let c = self.clauses[clause_idx];
            if lit_pos == 0 {
                c.0
            } else if lit_pos == 1 {
                c.1
            } else {
                c.2
            }
        };

        for c in 0..m {
            let base = 3 * c;
            let penalty = 2.0;
            quadratic_edges.push((base, base + 1, penalty));
            quadratic_edges.push((base + 1, base, penalty));
            quadratic_edges.push((base, base + 2, penalty));
            quadratic_edges.push((base + 2, base, penalty));
            quadratic_edges.push((base + 1, base + 2, penalty));
            quadratic_edges.push((base + 2, base + 1, penalty));
        }

        for i in 0..qubo_vars {
            for j in (i + 1)..qubo_vars {
                if get_literal(i) == -get_literal(j) {
                    quadratic_edges.push((i, j, 2.0));
                    quadratic_edges.push((j, i, 2.0));
                }
            }
        }

        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; qubo_vars];
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
            num_vars: qubo_vars,
            linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
        }
    }
}

// ==========================================
// 3. БАЙЕСОВСКИЕ ДВИЖКИ (v3.0)
// ==========================================
#[derive(Clone)]
struct ReplicaPT {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
}

#[derive(Clone)]
struct ReplicaBayes {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
    attempts: Vec<f64>,
    successes: Vec<f64>,
}

pub struct EngineResult {
    pub best_energy: f64,
}

// ДВИЖОК 1: ЧИСТАЯ ФИЗИКА (Pure PT)
pub fn run_pure_pt(model: &QuboModel, max_exchanges: usize) -> EngineResult {
    let num_replicas = 128;
    let sweeps = 50;
    let mut rng = rand::thread_rng();
    let mut replicas: Vec<ReplicaPT> = (0..num_replicas)
        .map(|i| {
            let state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
            let fraction = i as f64 / (num_replicas - 1).max(1) as f64;
            let temp = 100.0 * (0.01_f64 / 100.0).powf(fraction);
            let energy = model.calculate_total_energy(&state);
            ReplicaPT {
                state,
                temp,
                energy,
            }
        })
        .collect();

    let mut global_best = f64::INFINITY;

    for _ in 0..max_exchanges {
        replicas.par_iter_mut().for_each(|replica| {
            let mut local_rng = rand::thread_rng();
            for _ in 0..sweeps {
                let var_idx = local_rng.gen_range(0..model.num_vars);
                let flip_mult = 1.0 - 2.0 * (replica.state[var_idx] as f64);
                let mut sum_j = 0.0;
                for (col, weight) in model.quadratic.get_row(var_idx) {
                    sum_j += weight * (replica.state[col] as f64);
                }
                let delta_e = flip_mult * (model.linear[var_idx] + sum_j);

                if delta_e < 0.0
                    || (replica.temp > 1e-8
                        && local_rng.gen_range(0.0..1.0) < E.powf(-delta_e / replica.temp))
                {
                    replica.state[var_idx] = 1 - replica.state[var_idx];
                }
            }
            replica.energy = model.calculate_total_energy(&replica.state);
        });

        for i in 0..(num_replicas - 1) {
            let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
            let swap_prob = (delta_beta * (replicas[i].energy - replicas[i + 1].energy)).exp();
            if swap_prob >= 1.0 || rng.gen_range(0.0..1.0) < swap_prob {
                let ts = replicas[i].state.clone();
                replicas[i].state = replicas[i + 1].state.clone();
                replicas[i + 1].state = ts;
                let te = replicas[i].energy;
                replicas[i].energy = replicas[i + 1].energy;
                replicas[i + 1].energy = te;
            }
        }

        let current_min = replicas
            .iter()
            .map(|r| r.energy)
            .fold(f64::INFINITY, f64::min);
        if current_min < global_best {
            global_best = current_min;
        }
    }
    EngineResult {
        best_energy: global_best,
    }
}

// ДВИЖОК 2: BAYESIAN ISING ML v3.0 (Adaptive Blending)
pub fn run_bayesian_ml_v3(model: &QuboModel, max_exchanges: usize) -> EngineResult {
    let num_replicas = 128;
    let sweeps = 50;
    let burn_in_phase = max_exchanges / 3; // 33% времени учимся

    let mut rng = rand::thread_rng();
    let mut replicas: Vec<ReplicaBayes> = (0..num_replicas)
        .map(|i| {
            let state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
            let fraction = i as f64 / (num_replicas - 1).max(1) as f64;
            let temp = 100.0 * (0.01_f64 / 100.0).powf(fraction);
            let energy = model.calculate_total_energy(&state);
            ReplicaBayes {
                state,
                temp,
                energy,
                attempts: vec![0.0; model.num_vars],
                successes: vec![0.0; model.num_vars],
            }
        })
        .collect();

    let mut global_best = f64::INFINITY;

    for exchange_step in 0..max_exchanges {
        let is_burn_in = exchange_step < burn_in_phase;

        replicas.par_iter_mut().for_each(|replica| {
            let mut local_rng = rand::thread_rng();
            for _ in 0..sweeps {
                let var_idx = local_rng.gen_range(0..model.num_vars);

                // Смягчение Лапласа (0.1 / 1.0) для большей гибкости
                let priority =
                    (replica.successes[var_idx] + 0.1) / (replica.attempts[var_idx] + 1.0);

                // EPSILON-GREEDY: 0.5 (Оставляем огромный простор для исследования)
                if !is_burn_in && local_rng.gen_range(0.0..1.0) > priority.max(0.5) {
                    continue;
                }

                replica.attempts[var_idx] += 1.0;

                let flip_mult = 1.0 - 2.0 * (replica.state[var_idx] as f64);
                let mut sum_j = 0.0;
                for (col, weight) in model.quadratic.get_row(var_idx) {
                    sum_j += weight * (replica.state[col] as f64);
                }
                let delta_e = flip_mult * (model.linear[var_idx] + sum_j);

                let mut accepted = false;
                let mut real_improvement = false;

                if delta_e < 0.0 {
                    accepted = true;
                    real_improvement = true;
                    replica.successes[var_idx] += 1.0;
                } else if replica.temp > 1e-8
                    && local_rng.gen_range(0.0..1.0) < E.powf(-delta_e / replica.temp)
                {
                    accepted = true;
                }

                if accepted {
                    replica.state[var_idx] = 1 - replica.state[var_idx];

                    if real_improvement {
                        for (neighbor, _) in model.quadratic.get_row(var_idx) {
                            // ЖЕСТКИЙ NEIGHBOR BOOST: +1.0 к попыткам и успехам (будим соседей)
                            replica.attempts[neighbor] += 1.0;
                            replica.successes[neighbor] += 1.0;
                        }
                    }
                }
            }
            replica.energy = model.calculate_total_energy(&replica.state);
        });

        // СИНХРОНИЗАЦИЯ v3: ADAPTIVE BLENDING
        for i in 0..(num_replicas - 1) {
            let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
            let swap_prob = (delta_beta * (replicas[i].energy - replicas[i + 1].energy)).exp();
            if swap_prob >= 1.0 || rng.gen_range(0.0..1.0) < swap_prob {
                let ts = replicas[i].state.clone();
                replicas[i].state = replicas[i + 1].state.clone();
                replicas[i + 1].state = ts;
                let te = replicas[i].energy;
                replicas[i].energy = replicas[i + 1].energy;
                replicas[i + 1].energy = te;

                // Передаем память только от ЛУЧШЕЙ реплики к ХУДШЕЙ (с небольшим смешиванием)
                let better_idx = if te < replicas[i + 1].energy {
                    i
                } else {
                    i + 1
                };
                let worse_idx = if better_idx == i { i + 1 } else { i };

                for k in 0..model.num_vars {
                    // Худшая реплика перенимает 70% опыта лучшей и оставляет 30% своего
                    let new_a = replicas[better_idx].attempts[k] * 0.7
                        + replicas[worse_idx].attempts[k] * 0.3;
                    let new_s = replicas[better_idx].successes[k] * 0.7
                        + replicas[worse_idx].successes[k] * 0.3;

                    replicas[worse_idx].attempts[k] = new_a;
                    replicas[worse_idx].successes[k] = new_s;
                    // Лучшая реплика оставляет свой опыт нетронутым!
                }
            }
        }

        let current_min = replicas
            .iter()
            .map(|r| r.energy)
            .fold(f64::INFINITY, f64::min);
        if current_min < global_best {
            global_best = current_min;
        }
    }
    EngineResult {
        best_energy: global_best,
    }
}

// ==========================================
// 4. MAIN
// ==========================================
fn main() {
    println!("🧬 BAYESIAN SCALING LAB V3.0: The Final Cut");
    println!("Реплик: 128 | Burn-in: 33% | Epsilon: 0.5 | Adaptive Blending");
    println!("----------------------------------------------------------------------------------");
    println!(
        "{:>5} | {:>10} | {:>15} | {:>15} | {:>15}",
        "N", "QUBO Узлов", "Residual E (PT)", "Residual E (Bayes)", "Дельта (Выигрыш)"
    );
    println!("----------------------------------------------------------------------------------");

    let n_values = vec![50, 100, 150, 200, 250, 300, 400];
    let alpha = 3.5;
    let instances = 3;
    let max_exchanges = 5000;

    for &n in &n_values {
        let m = (n as f64 * alpha).round() as usize;
        let target_e = -(m as f64);
        let qubo_vars = 3 * m;

        let mut pt_residual_sum = 0.0;
        let mut ml_residual_sum = 0.0;

        for _ in 0..instances {
            let builder = Sat3Builder::generate_random(n, m);
            let model = builder.build_qubo();

            let pt_res = run_pure_pt(&model, max_exchanges);
            let ml_res = run_bayesian_ml_v3(&model, max_exchanges);

            pt_residual_sum += pt_res.best_energy - target_e;
            ml_residual_sum += ml_res.best_energy - target_e;
        }

        let pt_avg_res = pt_residual_sum / instances as f64;
        let ml_avg_res = ml_residual_sum / instances as f64;
        let delta = pt_avg_res - ml_avg_res;

        let marker = if delta > 0.0 {
            "🔥 Memory Wins!"
        } else if delta < 0.0 {
            "⚠️ Physics Wins"
        } else {
            "⚖️ Tie"
        };

        println!(
            "{:>5} | {:>10} | {:>15.1} | {:>15.1} | {:>14.1} {}",
            n, qubo_vars, pt_avg_res, ml_avg_res, delta, marker
        );
    }

    println!("----------------------------------------------------------------------------------");
}
