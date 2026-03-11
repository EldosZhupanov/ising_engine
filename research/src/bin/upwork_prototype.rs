#![allow(warnings)]
use rand::Rng;
use rayon::prelude::*;
use std::f64::consts::E;
use std::time::Instant;

// ==========================================
// 1. ДАННЫЕ ЗАКАЗЧИКА
// ==========================================
const RAW_A: &str = "0 6 0 14 18 0 8 22 0 6 0 8 7 20 0 8 0 18 0 8 0 0 0 13 4 0 0 0 0 0 0 24 0 13 0 0 0 0 0 0 0 13 0 9 0 0 21 0 21 0 0 0 3 0 8 8 4 0 0 16 0 0 13 0 0 0 0 0 0 0 0 0 21 0 10 17 14 3 0 0 0 16 10 0 0 0 0 0 0 0";
const RAW_B: &str = "0 37 50000000 79 98 50000000 45 118 50000000 37 0 43 42 107 50000000 46 50000000 99 50000000 43 0 50000000 50000000 73 26 50000000 50000000 50000000 50000000 50000000 0 129 50000000 74 50000000 50000000 50000000 50000000 50000000 50000000 0 73 50000000 52 50000000 50000000 115 50000000 112 50000000 0 50000000 21 50000000 45 46 26 50000000 50000000 86 0 50000000 73 50000000 50000000 50000000 50000000 50000000 50000000 50000000 0 50000000 116 50000000 57 93 75 18 50000000 50000000 0 90 55 50000000 50000000 50000000 50000000 50000000 50000000 50000000";
const RAW_C: &str = "0 11100 0 23700 29400 0 13500 35400 0 11100 0 12900 12600 32100 0 13800 0 29700 0 12900 0 0 0 21900 7800 0 0 0 0 0 0 38700 0 22200 0 0 0 0 0 0 0 21900 0 15600 0 0 34500 0 33600 0 0 0 6300 0 13500 13800 7800 0 0 25800 0 0 21900 0 0 0 0 0 0 0 0 0 34800 0 17100 27900 22500 5400 0 0 0 27000 16500 0 0 0 0 0 0 0";

// ==========================================
// 2. ВЫЧИСЛИТЕЛЬНОЕ ЯДРО (CSR Format)
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
// 3. БИЗНЕС-КОМПИЛЯТОР (QUBO Modeler)
// ==========================================
pub struct BusinessModeler {
    pub n: usize,
    pub costs: Vec<f64>,
}

impl BusinessModeler {
    pub fn parse() -> Self {
        let parse_arr =
            |s: &str| -> Vec<f64> { s.split_whitespace().map(|x| x.parse().unwrap()).collect() };
        let a = parse_arr(RAW_A);
        let b = parse_arr(RAW_B);
        let c = parse_arr(RAW_C);

        let mut costs = vec![0.0; 100];
        for i in 0..a.len() {
            costs[i] = a[i] + b[i] + c[i];
        }

        Self { n: 10, costs }
    }

    pub fn build_qubo(&self) -> QuboModel {
        let num_vars = self.n * self.n;
        let mut linear = vec![0.0; num_vars];
        let mut quadratic_edges = vec![];
        let mut energy_offset = 0.0;

        let gamma = 1_000_000.0;

        for i in 0..self.n {
            for j in 0..self.n {
                let var_idx = i * self.n + j;
                linear[var_idx] += self.costs[var_idx];
            }
        }

        // БИЗНЕС-ОГРАНИЧЕНИЕ 1: ИСХОДЯЩИЕ (Исправленная алгебра: -1.0 * gamma)
        for i in 0..self.n {
            energy_offset += gamma;
            for j in 0..self.n {
                let v1 = i * self.n + j;
                linear[v1] -= gamma; // ИСПРАВЛЕНО! Было -2.0*gamma
                for k in (j + 1)..self.n {
                    let v2 = i * self.n + k;
                    quadratic_edges.push((v1, v2, 2.0 * gamma));
                    quadratic_edges.push((v2, v1, 2.0 * gamma));
                }
            }
        }

        // БИЗНЕС-ОГРАНИЧЕНИЕ 2: ВХОДЯЩИЕ (Исправленная алгебра: -1.0 * gamma)
        for j in 0..self.n {
            energy_offset += gamma;
            for i in 0..self.n {
                let v1 = i * self.n + j;
                linear[v1] -= gamma; // ИСПРАВЛЕНО! Было -2.0*gamma
                for k in (i + 1)..self.n {
                    let v2 = k * self.n + j;
                    quadratic_edges.push((v1, v2, 2.0 * gamma));
                    quadratic_edges.push((v2, v1, 2.0 * gamma));
                }
            }
        }

        // Запрет на доставку "самому себе" (По диагонали A_ii = 50_000_000 штраф)
        // Чтобы модель не сжульничала, выбрав нулевые стоимости по диагонали (хотя у нас там уже есть 0 в матрице, но мы запретим это жестко).
        for i in 0..self.n {
            let v_diag = i * self.n + i;
            linear[v_diag] += gamma * 10.0; // Жесточайший запрет на петли
        }

        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; num_vars];
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
            num_vars,
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
// 4. ТВОЙ BAYESIAN ISING ENGINE v3.0
// ==========================================
#[derive(Clone)]
struct ReplicaBayes {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
    attempts: Vec<f64>,
    successes: Vec<f64>,
}

pub struct HybridSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub max_exchanges: usize,
}

impl HybridSolver {
    pub fn solve(&self, model: &QuboModel) -> (f64, Vec<i8>) {
        let burn_in_phase = self.max_exchanges / 3;
        let mut rng = rand::thread_rng();
        let mut replicas: Vec<ReplicaBayes> = (0..self.num_replicas)
            .map(|i| {
                let state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
                let fraction = i as f64 / (self.num_replicas - 1).max(1) as f64;
                let temp = self.temp_max * (self.temp_min / self.temp_max).powf(fraction);
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

        let mut global_best_energy = f64::INFINITY;
        let mut global_best_state = vec![];

        for exchange_step in 0..self.max_exchanges {
            let is_burn_in = exchange_step < burn_in_phase;

            replicas.par_iter_mut().for_each(|replica| {
                let mut local_rng = rand::thread_rng();
                for _ in 0..self.sweeps_per_exchange {
                    let var_idx = local_rng.gen_range(0..model.num_vars);
                    let priority =
                        (replica.successes[var_idx] + 0.1) / (replica.attempts[var_idx] + 1.0);
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
                                replica.attempts[neighbor] += 1.0;
                                replica.successes[neighbor] += 1.0;
                            }
                        }
                    }
                }
                replica.energy = model.calculate_total_energy(&replica.state);
            });

            for i in 0..(self.num_replicas - 1) {
                let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
                let swap_prob = (delta_beta * (replicas[i].energy - replicas[i + 1].energy)).exp();
                if swap_prob >= 1.0 || rng.gen_range(0.0..1.0) < swap_prob {
                    let ts = replicas[i].state.clone();
                    replicas[i].state = replicas[i + 1].state.clone();
                    replicas[i + 1].state = ts;
                    let te = replicas[i].energy;
                    replicas[i].energy = replicas[i + 1].energy;
                    replicas[i + 1].energy = te;

                    let better_idx = if te < replicas[i + 1].energy {
                        i
                    } else {
                        i + 1
                    };
                    let worse_idx = if better_idx == i { i + 1 } else { i };

                    for k in 0..model.num_vars {
                        let new_a = replicas[better_idx].attempts[k] * 0.7
                            + replicas[worse_idx].attempts[k] * 0.3;
                        let new_s = replicas[better_idx].successes[k] * 0.7
                            + replicas[worse_idx].successes[k] * 0.3;
                        replicas[worse_idx].attempts[k] = new_a;
                        replicas[worse_idx].successes[k] = new_s;
                    }
                }
            }

            for r in &replicas {
                if r.energy < global_best_energy {
                    global_best_energy = r.energy;
                    global_best_state = r.state.clone();
                }
            }
        }
        (global_best_energy, global_best_state)
    }
}

// ==========================================
// 5. ИСПОЛНЕНИЕ ЗАКАЗА
// ==========================================
fn main() {
    println!("💼 UPWORK FREELANCE: Логистическая оптимизация (Assignment Problem)");
    println!("(Исправлена алгебра штрафов. Алгоритм больше не сможет нас обмануть!)");

    let modeler = BusinessModeler::parse();
    let qubo_model = modeler.build_qubo();

    println!("Запуск Bayesian ML Engine V3.0...");

    let solver = HybridSolver {
        num_replicas: 128,
        temp_max: 50_000.0,
        temp_min: 0.1,
        sweeps_per_exchange: 100,
        max_exchanges: 4000,
    };

    let start = Instant::now();
    let (best_energy, best_state) = solver.solve(&qubo_model);
    let duration = start.elapsed();

    println!("\n================ ОТЧЕТ ДЛЯ КЛИЕНТА ================");
    println!("Время расчета: {:.2?}", duration);

    if best_energy >= 100_000.0 {
        println!(
            "⚠️ ВНИМАНИЕ: Нарушены правила. Остаточная энергия: {}",
            best_energy
        );
    } else {
        println!("✅ Найдено ВАЛИДНОЕ решение! Все 10 заводов распределены 1 к 1.");
        println!("💰 Минимальные Затраты: ${}", best_energy);
        println!("--------------------------------------------------");

        for i in 0..modeler.n {
            for j in 0..modeler.n {
                let var_idx = i * modeler.n + j;
                if best_state[var_idx] == 1 {
                    println!(
                        "Завод {} ➔ Магазин {} (Затраты: ${})",
                        i + 1,
                        j + 1,
                        modeler.costs[var_idx]
                    );
                }
            }
        }
    }
    println!("===================================================");
}
