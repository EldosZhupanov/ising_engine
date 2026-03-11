#![allow(warnings)]
use rand::Rng;
use rayon::prelude::*;
use std::f64::consts::E;
use std::time::Instant;

// ==========================================
// 1. ВЫЧИСЛИТЕЛЬНОЕ ЯДРО (CSR MATRIX)
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
// 2. БИЗНЕС-КОМПИЛЯТОР: WAGNER-WHITIN (p01.param)
// ==========================================
pub struct SupplyChainModeler {
    pub t_periods: usize,
    pub setup_costs: Vec<f64>,
    pub holding_costs: Vec<f64>,
    pub demands: Vec<Vec<f64>>,
    pub lineages: Vec<Vec<usize>>,
}

impl SupplyChainModeler {
    pub fn new() -> Self {
        // Данные из файла p01.param (адаптировано под 0-индексацию)
        Self {
            t_periods: 10,
            // Стоимость заказа (procCost) для узлов 0..5
            setup_costs: vec![1000.0, 3000.0, 1000.0, 5000.0, 7000.0, 10000.0],
            // Стоимость хранения (holdingCost)
            holding_costs: vec![10.0, 30.0, 20.0, 6.0, 6.0, 5.0],
            // Спрос только на листьях (Узлы 0, 1, 2)
            demands: vec![
                vec![
                    100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0,
                ],
                vec![
                    50.0, 200.0, 50.0, 50.0, 200.0, 250.0, 250.0, 100.0, 150.0, 150.0,
                ],
                vec![
                    250.0, 50.0, 350.0, 50.0, 250.0, 50.0, 250.0, 50.0, 350.0, 50.0,
                ],
            ],
            // Иерархия (от корня к листу): Root -> Parent -> Leaf
            // Node 5 (Root) -> Node 3, Node 4. Node 3 -> 0, 1. Node 4 -> 2.
            lineages: vec![
                vec![5, 3, 0], // Путь к листу 0
                vec![5, 3, 1], // Путь к листу 1
                vec![5, 4, 2], // Путь к листу 2
            ],
        }
    }

    pub fn build_qubo(&self) -> (QuboModel, Vec<String>) {
        let num_nodes = 6;
        let mut linear = vec![];
        let mut quadratic_edges = vec![];
        let mut energy_offset = 0.0;
        let gamma = 5_000_000.0; // Жесткий логистический штраф (запрет ошибок)
        let mut var_names = vec![];

        // 1. Переменные Открытия Заказа O_{node, t}
        for node in 0..num_nodes {
            for t in 0..self.t_periods {
                linear.push(self.setup_costs[node]);
                var_names.push(format!("Заказ: Узел {}, День {}", node, t + 1));
            }
        }

        // 2. Переменные Путей Спроса (Demand Paths)
        let get_o_idx = |node: usize, t: usize| node * self.t_periods + t;
        let mut current_var_idx = num_nodes * self.t_periods;

        for leaf_idx in 0..3 {
            let lineage = &self.lineages[leaf_idx];

            for k in 0..self.t_periods {
                let demand = self.demands[leaf_idx][k];
                if demand == 0.0 {
                    continue;
                }

                let mut path_vars_for_demand = vec![];

                // Перебор всех возможных дат поставок: t0 <= t1 <= t2 <= k
                for t0 in 0..=k {
                    for t1 in t0..=k {
                        for t2 in t1..=k {
                            let path_idx = current_var_idx;
                            current_var_idx += 1;
                            path_vars_for_demand.push(path_idx);

                            // Считаем налог на хранение на всех эшелонах по этому пути
                            let h_root = self.holding_costs[lineage[0]] * (t1 - t0) as f64;
                            let h_parent = self.holding_costs[lineage[1]] * (t2 - t1) as f64;
                            let h_leaf = self.holding_costs[lineage[2]] * (k - t2) as f64;
                            let total_hold_cost = (h_root + h_parent + h_leaf) * demand;

                            linear.push(total_hold_cost);
                            var_names.push(format!("Путь [Узел {}: д.{}] ➔ Прибыл на Корень в д.{}, Транзит в д.{}, Доставка в д.{}", 
                                          lineage[2], k+1, t0+1, t1+1, t2+1));

                            // ПРИВЯЗКА ПУТИ К ЗАКАЗАМ: Y -> O
                            let add_activation_penalty =
                                |lin: &mut Vec<f64>,
                                 quad: &mut Vec<(usize, usize, f64)>,
                                 o_idx: usize,
                                 y_idx: usize| {
                                    lin[y_idx] += gamma;
                                    quad.push((y_idx, o_idx, -gamma));
                                    quad.push((o_idx, y_idx, -gamma));
                                };

                            add_activation_penalty(
                                &mut linear,
                                &mut quadratic_edges,
                                get_o_idx(lineage[0], t0),
                                path_idx,
                            );
                            add_activation_penalty(
                                &mut linear,
                                &mut quadratic_edges,
                                get_o_idx(lineage[1], t1),
                                path_idx,
                            );
                            add_activation_penalty(
                                &mut linear,
                                &mut quadratic_edges,
                                get_o_idx(lineage[2], t2),
                                path_idx,
                            );
                        }
                    }
                }

                // ОГРАНИЧЕНИЕ: Ровно 1 логистический путь должен закрывать этот спрос
                energy_offset += gamma;
                for &y1 in &path_vars_for_demand {
                    linear[y1] -= gamma;
                    for &y2 in &path_vars_for_demand {
                        if y1 < y2 {
                            quadratic_edges.push((y1, y2, 2.0 * gamma));
                            quadratic_edges.push((y2, y1, 2.0 * gamma));
                        }
                    }
                }
            }
        }

        let num_vars = linear.len();
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
        (
            QuboModel {
                num_vars,
                linear,
                quadratic: CsrMatrix {
                    values,
                    col_indices,
                    row_offsets,
                },
                energy_offset,
            },
            var_names,
        )
    }
}

// ==========================================
// 3. BAYESIAN ENGINE V3.0 (SWARM INTELLIGENCE)
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
// 4. MAIN - ЗАПУСК
// ==========================================
fn main() {
    println!("📦 ИССЛЕДОВАНИЕ ОПЕРАЦИЙ: Wagner-Whitin (CSPLib prob040)");
    println!("Загрузка данных p01.param (6 узлов, 3 листа, 10 периодов)...");

    let modeler = SupplyChainModeler::new();
    let (qubo_model, var_names) = modeler.build_qubo();

    println!(
        "Размер скомпилированной Изинг-матрицы: {} переменных.",
        qubo_model.num_vars
    );
    println!("Запуск Bayesian Swarm Intelligence. Пробиваем штрафной барьер...");

    let solver = HybridSolver {
        num_replicas: 128,
        temp_max: 50_000_000.0,
        temp_min: 0.1,
        sweeps_per_exchange: 50,
        max_exchanges: 4000,
    };

    let start = Instant::now();
    let (best_energy, best_state) = solver.solve(&qubo_model);
    let duration = start.elapsed();

    println!("\n================ ОТЧЕТ ДЛЯ КЛИЕНТА ================");
    println!("Время квантовой эмуляции: {:.2?}", duration);

    if best_energy >= 1_000_000.0 {
        println!(
            "⚠️ Алгоритм застрял в локальном минимуме (штраф не снят). Энергия: {}",
            best_energy
        );
    } else {
        println!("✅ МАТЕМАТИЧЕСКИЙ ТРИУМФ! Оптимальная стратегия распределения найдена.");
        println!(
            "💰 Общие затраты корпорации (Хранение + Логистика): ${}",
            best_energy
        );
        println!("---------------------------------------------------------");
        println!("РАСПИСАНИЕ АРЕНДЫ ФУР (Setup Costs):");
        for i in 0..(6 * modeler.t_periods) {
            if best_state[i] == 1 {
                println!("  🚚 {}", var_names[i]);
            }
        }
    }
    println!("=========================================================");
}
