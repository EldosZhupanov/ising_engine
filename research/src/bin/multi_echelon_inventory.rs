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
// 2. БИЗНЕС-ЛОГИКА: SUPPLY CHAIN COMPILER
// ==========================================
// Структура дерева: F -> (D, E); D -> (A, B); E -> (C)
// Узлы: 0=F, 1=D, 2=E, 3=A, 4=B, 5=C

pub struct SupplyChainModeler {
    pub t_periods: usize,
    pub setup_costs: Vec<f64>,   // c0
    pub holding_costs: Vec<f64>, // c
    pub demands: Vec<Vec<f64>>,  // Спрос для листьев (A, B, C)
}

impl SupplyChainModeler {
    pub fn new() -> Self {
        Self {
            t_periods: 3, // Планируем на 3 периода
            // Setup Cost: У корня дороже всего заказать фуру, у листьев дешевле
            setup_costs: vec![150.0, 80.0, 80.0, 30.0, 30.0, 30.0],
            // Holding Cost: На главном складе хранить дешево, в магазинах - дорого
            holding_costs: vec![1.0, 2.0, 2.0, 4.0, 4.0, 4.0],
            // Спрос: Заполнен только для листьев [Узел 3 (A), Узел 4 (B), Узел 5 (C)]
            demands: vec![
                vec![0.0, 0.0, 0.0],    // 0: F
                vec![0.0, 0.0, 0.0],    // 1: D
                vec![0.0, 0.0, 0.0],    // 2: E
                vec![10.0, 20.0, 15.0], // 3: A
                vec![15.0, 0.0, 10.0],  // 4: B
                vec![20.0, 20.0, 20.0], // 5: C
            ],
        }
    }

    pub fn build_qubo(&self) -> (QuboModel, Vec<String>) {
        let num_nodes = 6;
        let mut linear = vec![];
        let mut quadratic_edges = vec![];
        let mut energy_offset = 0.0;
        let gamma = 10_000.0; // Жесткий штраф за логистические ошибки
        let mut var_names = vec![];

        // 1. Переменные Открытия Заказа O_{node, t}. Индексы: 0 .. (6 * 3) = 18 штук
        for node in 0..num_nodes {
            for t in 0..self.t_periods {
                linear.push(self.setup_costs[node]); // Если заказываем, платим Setup Cost
                var_names.push(format!("Заказ: Узел {}, День {}", node_name(node), t + 1));
            }
        }

        // 2. Переменные Путей (Demand Paths)
        let get_o_idx = |node: usize, t: usize| node * self.t_periods + t;
        let mut current_var_idx = num_nodes * self.t_periods;

        let leaves = vec![(3, 1), (4, 1), (5, 2)]; // (Node, Parent). Root is always 0.

        for (leaf, parent) in leaves {
            for k in 0..self.t_periods {
                let demand = self.demands[leaf][k];
                if demand == 0.0 {
                    continue;
                } // Нет спроса - нет путей

                let mut path_vars_for_demand = vec![];

                // Генерируем все возможные пути во времени: t_root <= t_parent <= t_leaf <= k
                for t_root in 0..=k {
                    for t_parent in t_root..=k {
                        for t_leaf in t_parent..=k {
                            let path_idx = current_var_idx;
                            current_var_idx += 1;
                            path_vars_for_demand.push(path_idx);

                            // Считаем стоимость хранения товара на всех складах по этому пути
                            let hold_root = self.holding_costs[0] * (t_parent - t_root) as f64;
                            let hold_parent =
                                self.holding_costs[parent] * (t_leaf - t_parent) as f64;
                            let hold_leaf = self.holding_costs[leaf] * (k - t_leaf) as f64;
                            let total_hold_cost = (hold_root + hold_parent + hold_leaf) * demand;

                            // QUBO: Стоимость пути
                            linear.push(total_hold_cost);
                            var_names.push(format!(
                                "Путь: Спрос {} ед. для {} в д.{}. (F:д{}, {}:д{}, {}:д{})",
                                demand,
                                node_name(leaf),
                                k + 1,
                                t_root + 1,
                                node_name(parent),
                                t_parent + 1,
                                node_name(leaf),
                                t_leaf + 1
                            ));

                            // ПРИВЯЗКА ПУТИ К ЗАКАЗАМ (Y_path -> O_node_t)
                            // Штраф: Y * (1 - O) = Y - Y*O -> Linear: +gamma, Quad: -gamma
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
                                get_o_idx(0, t_root),
                                path_idx,
                            );
                            add_activation_penalty(
                                &mut linear,
                                &mut quadratic_edges,
                                get_o_idx(parent, t_parent),
                                path_idx,
                            );
                            add_activation_penalty(
                                &mut linear,
                                &mut quadratic_edges,
                                get_o_idx(leaf, t_leaf),
                                path_idx,
                            );
                        }
                    }
                }

                // ОГРАНИЧЕНИЕ: Ровно 1 путь должен быть выбран для удовлетворения этого спроса
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

fn node_name(idx: usize) -> &'static str {
    match idx {
        0 => "F(Root)",
        1 => "D(L2)",
        2 => "E(L2)",
        3 => "A(L1)",
        4 => "B(L1)",
        5 => "C(L1)",
        _ => "?",
    }
}

// ==========================================
// 3. BAYESIAN ENGINE V3.0
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
// 4. MAIN - ЗАПУСК МОДЕЛИРОВАНИЯ
// ==========================================
fn main() {
    println!("📦 МНОГОЭШЕЛОННАЯ ОПТИМИЗАЦИЯ ЗАПАСОВ (Wagner-Whitin Multi-Echelon)");
    println!("Генерация графа путей спроса и логистических узлов...");

    let modeler = SupplyChainModeler::new();
    let (qubo_model, var_names) = modeler.build_qubo();

    println!(
        "Размер QUBO-модели: {} переменных (Открытия складов + Возможные Пути).",
        qubo_model.num_vars
    );
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

    println!("\n================ РЕЗУЛЬТАТЫ ПЛАНИРОВАНИЯ ================");
    println!("Время расчета: {:.2?}", duration);

    if best_energy >= 10_000.0 {
        println!(
            "⚠️ Алгоритм не нашел легального решения (штраф не снят). Энергия: {}",
            best_energy
        );
    } else {
        println!("✅ Идеальная стратегия поставок найдена!");
        println!("💰 Общие затраты (Хранение + Логистика): ${}", best_energy);
        println!("---------------------------------------------------------");
        println!("РАСПИСАНИЕ ЗАКАЗОВ ФУР (Setup Costs):");
        for i in 0..(6 * modeler.t_periods) {
            if best_state[i] == 1 {
                println!("  🚚 {}", var_names[i]);
            }
        }
        println!("\nВЫБРАННЫЕ МАРШРУТЫ УДОВЛЕТВОРЕНИЯ СПРОСА:");
        for i in (6 * modeler.t_periods)..qubo_model.num_vars {
            if best_state[i] == 1 {
                println!("  📦 {}", var_names[i]);
            }
        }
    }
    println!("=========================================================");
}
