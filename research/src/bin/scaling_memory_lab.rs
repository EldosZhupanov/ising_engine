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
// 3. ДВА ДВИЖКА ДЛЯ СРАВНЕНИЯ
// ==========================================
#[derive(Clone)]
struct Replica {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
    priority_map: Vec<f64>,
}

pub struct EngineResult {
    pub solved: bool,
    pub exchanges: usize,
}

// ДВИЖОК 1: ЧИСТАЯ ФИЗИКА (Без памяти)
pub fn run_pure_pt(model: &QuboModel, target_energy: f64, max_exchanges: usize) -> EngineResult {
    let num_replicas = 64;
    let sweeps = 50;
    let mut rng = rand::thread_rng();
    let mut replicas: Vec<Replica> = (0..num_replicas)
        .map(|i| {
            let state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
            let fraction = i as f64 / (num_replicas - 1).max(1) as f64;
            let temp = 100.0 * (0.01_f64 / 100.0).powf(fraction);
            let energy = model.calculate_total_energy(&state);
            Replica {
                state,
                temp,
                energy,
                priority_map: vec![],
            }
        })
        .collect();

    for exchange_step in 0..max_exchanges {
        if replicas.iter().any(|r| r.energy <= target_energy + 1e-6) {
            return EngineResult {
                solved: true,
                exchanges: exchange_step,
            };
        }

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
    }
    EngineResult {
        solved: false,
        exchanges: max_exchanges,
    }
}

// ДВИЖОК 2: ФИЗИКА + ПАМЯТЬ (priority_map)
pub fn run_hybrid_ml(model: &QuboModel, target_energy: f64, max_exchanges: usize) -> EngineResult {
    let num_replicas = 64;
    let sweeps = 50;
    let mut rng = rand::thread_rng();
    let mut replicas: Vec<Replica> = (0..num_replicas)
        .map(|i| {
            let state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
            let fraction = i as f64 / (num_replicas - 1).max(1) as f64;
            let temp = 100.0 * (0.01_f64 / 100.0).powf(fraction);
            let energy = model.calculate_total_energy(&state);
            Replica {
                state,
                temp,
                energy,
                priority_map: vec![1.0; model.num_vars],
            }
        })
        .collect();

    for exchange_step in 0..max_exchanges {
        if replicas.iter().any(|r| r.energy <= target_energy + 1e-6) {
            return EngineResult {
                solved: true,
                exchanges: exchange_step,
            };
        }

        replicas.par_iter_mut().for_each(|replica| {
            let mut local_rng = rand::thread_rng();
            for _ in 0..sweeps {
                let var_idx = local_rng.gen_range(0..model.num_vars);
                if local_rng.gen_range(0.0..1.0) > replica.priority_map[var_idx] {
                    continue;
                }

                let flip_mult = 1.0 - 2.0 * (replica.state[var_idx] as f64);
                let mut sum_j = 0.0;
                for (col, weight) in model.quadratic.get_row(var_idx) {
                    sum_j += weight * (replica.state[col] as f64);
                }
                let delta_e = flip_mult * (model.linear[var_idx] + sum_j);

                let mut accepted = false;
                if delta_e < 0.0 {
                    accepted = true;
                    replica.priority_map[var_idx] = (replica.priority_map[var_idx] + 0.15).min(1.0);
                } else if replica.temp > 1e-8
                    && local_rng.gen_range(0.0..1.0) < E.powf(-delta_e / replica.temp)
                {
                    accepted = true;
                    replica.priority_map[var_idx] = (replica.priority_map[var_idx] + 0.02).min(1.0);
                } else {
                    replica.priority_map[var_idx] = (replica.priority_map[var_idx] - 0.05).max(0.2);
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

        for i in 0..(num_replicas - 1) {
            let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
            let swap_prob = (delta_beta * (replicas[i].energy - replicas[i + 1].energy)).exp();
            if swap_prob >= 1.0 || rng.gen_range(0.0..1.0) < swap_prob {
                let ts = replicas[i].state.clone();
                replicas[i].state = replicas[i + 1].state.clone();
                replicas[i + 1].state = ts;
                let tm = replicas[i].priority_map.clone();
                replicas[i].priority_map = replicas[i + 1].priority_map.clone();
                replicas[i + 1].priority_map = tm;
                let te = replicas[i].energy;
                replicas[i].energy = replicas[i + 1].energy;
                replicas[i + 1].energy = te;
            }
        }
    }
    EngineResult {
        solved: false,
        exchanges: max_exchanges,
    }
}

// ==========================================
// 4. MAIN - ИЗМЕРЕНИЕ ЁМКОСТИ ПАМЯТИ
// ==========================================
fn main() {
    println!("🧠 LAB: ИНФОРМАЦИОННАЯ ЁМКОСТЬ ЭВРИСТИКИ (Memory Limit)");
    println!("Фиксируем плотность α = 4.0 (Сложная, но решаемая фаза).");
    println!("Увеличиваем N. Ищем точку, где вектор приоритетов превращается в шум.");
    println!("----------------------------------------------------------------------------------");
    println!(
        "{:>5} | {:>10} | {:>15} | {:>15} | {:>15}",
        "N", "QUBO Узлов", "Exchanges (PT)", "Exchanges (ML)", "Ускорение (ML/PT)"
    );
    println!("----------------------------------------------------------------------------------");

    let n_values = vec![50, 75, 100, 125, 150, 175, 200];
    let alpha = 4.0;
    let instances = 5;
    let max_exchanges = 3000;

    for &n in &n_values {
        let m = (n as f64 * alpha).round() as usize;
        let target_e = -(m as f64);
        let qubo_vars = 3 * m;

        let mut pt_total = 0;
        let mut ml_total = 0;

        for _ in 0..instances {
            let builder = Sat3Builder::generate_random(n, m);
            let model = builder.build_qubo();

            let pt_res = run_pure_pt(&model, target_e, max_exchanges);
            let ml_res = run_hybrid_ml(&model, target_e, max_exchanges);

            pt_total += pt_res.exchanges;
            ml_total += ml_res.exchanges;
        }

        let pt_avg = pt_total as f64 / instances as f64;
        let ml_avg = ml_total as f64 / instances as f64;

        let speedup = pt_avg / ml_avg;

        let marker = if speedup < 1.0 {
            "⚠️ OVERFLOW"
        } else if speedup > 2.0 {
            "🔥 SUPER"
        } else {
            ""
        };

        println!(
            "{:>5} | {:>10} | {:>15.1} | {:>15.1} | {:>14.2}x {}",
            n, qubo_vars, pt_avg, ml_avg, speedup, marker
        );
    }

    println!("----------------------------------------------------------------------------------");
    println!(
        "Анализ: Если 'Ускорение' падает с ростом N, мы нашли предел топологической памяти движка."
    );
}
