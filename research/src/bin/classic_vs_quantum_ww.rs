#![allow(warnings)]
use rand::Rng;
use rayon::prelude::*;
use std::f64::consts::E;
use std::time::Instant;

// ==========================================
// 1. КЛАССИЧЕСКИЙ АЛГОРИТМ (Dynamic Programming)
// ==========================================
// Решает классическую задачу Вагнера-Уитина для ОДНОГО склада
fn classic_wagner_whitin_dp(
    demands: &[f64],
    setup_cost: f64,
    holding_cost: f64,
) -> (f64, Vec<f64>) {
    let n = demands.len();
    let mut dp = vec![f64::INFINITY; n + 1];
    let mut choice = vec![0; n + 1];
    dp[n] = 0.0;

    for i in (0..n).rev() {
        let mut order_amount = 0.0;
        let mut hold_cost = 0.0;

        for j in i..n {
            order_amount += demands[j];
            hold_cost += demands[j] * holding_cost * ((j - i) as f64);

            // Если спроса нет, мы ничего не заказываем и не платим Setup Cost
            let cost = if order_amount > 0.0 {
                setup_cost + hold_cost + dp[j + 1]
            } else {
                dp[j + 1]
            };

            if cost < dp[i] {
                dp[i] = cost;
                choice[i] = j + 1;
            }
        }
    }

    let mut orders = vec![0.0; n];
    let mut curr = 0;
    while curr < n {
        let next = choice[curr];
        let mut amount = 0.0;
        for k in curr..next {
            amount += demands[k];
        }
        orders[curr] = amount;
        curr = next;
    }
    (dp[0], orders)
}

// ==========================================
// 2. ВЫЧИСЛИТЕЛЬНОЕ ЯДРО ИЗИНГА
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
// 3. БИЗНЕС-КОМПИЛЯТОР QUBO (p01.param)
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
        Self {
            t_periods: 10,
            setup_costs: vec![1000.0, 3000.0, 1000.0, 5000.0, 7000.0, 10000.0],
            holding_costs: vec![10.0, 30.0, 20.0, 6.0, 6.0, 5.0],
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
            lineages: vec![vec![5, 3, 0], vec![5, 3, 1], vec![5, 4, 2]],
        }
    }

    pub fn build_qubo(&self) -> QuboModel {
        let num_nodes = 6;
        let mut linear = vec![];
        let mut quadratic_edges = vec![];
        let mut energy_offset = 0.0;
        let gamma = 5_000_000.0;

        for node in 0..num_nodes {
            for _ in 0..self.t_periods {
                linear.push(self.setup_costs[node]);
            }
        }

        let get_o_idx = |node: usize, t: usize| node * self.t_periods + t;
        let mut current_var_idx = num_nodes * self.t_periods;

        for leaf_idx in 0..3 {
            let lineage = &self.lineages[leaf_idx];
            for k in 0..self.t_periods {
                let demand = self.demands[leaf_idx][k];
                if demand == 0.0 {
                    continue;
                }
                let mut path_vars = vec![];

                for t0 in 0..=k {
                    for t1 in t0..=k {
                        for t2 in t1..=k {
                            let path_idx = current_var_idx;
                            current_var_idx += 1;
                            path_vars.push(path_idx);

                            let h_root = self.holding_costs[lineage[0]] * (t1 - t0) as f64;
                            let h_parent = self.holding_costs[lineage[1]] * (t2 - t1) as f64;
                            let h_leaf = self.holding_costs[lineage[2]] * (k - t2) as f64;
                            linear.push((h_root + h_parent + h_leaf) * demand);

                            let mut add_pen = |o_idx| {
                                linear[path_idx] += gamma;
                                quadratic_edges.push((path_idx, o_idx, -gamma));
                                quadratic_edges.push((o_idx, path_idx, -gamma));
                            };
                            add_pen(get_o_idx(lineage[0], t0));
                            add_pen(get_o_idx(lineage[1], t1));
                            add_pen(get_o_idx(lineage[2], t2));
                        }
                    }
                }
                energy_offset += gamma;
                for &y1 in &path_vars {
                    linear[y1] -= gamma;
                    for &y2 in &path_vars {
                        if y1 < y2 {
                            quadratic_edges.push((y1, y2, 2.0 * gamma));
                            quadratic_edges.push((y2, y1, 2.0 * gamma));
                        }
                    }
                }
            }
        }

        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; linear.len()];
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
            num_vars: linear.len(),
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
// 4. BAYESIAN ENGINE V3.0
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
    pub fn solve(&self, model: &QuboModel) -> f64 {
        let burn_in = self.max_exchanges / 3;
        let mut rng = rand::thread_rng();
        let mut replicas: Vec<ReplicaBayes> = (0..self.num_replicas)
            .map(|i| {
                let state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
                let fraction = i as f64 / (self.num_replicas - 1).max(1) as f64;
                let temp = self.temp_max * (self.temp_min / self.temp_max).powf(fraction);
                ReplicaBayes {
                    state: state.clone(),
                    temp,
                    energy: model.calculate_total_energy(&state),
                    attempts: vec![0.0; model.num_vars],
                    successes: vec![0.0; model.num_vars],
                }
            })
            .collect();

        let mut global_best = f64::INFINITY;
        for step in 0..self.max_exchanges {
            let is_burn_in = step < burn_in;
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
                    let b = if te < replicas[i + 1].energy {
                        i
                    } else {
                        i + 1
                    };
                    let w = if b == i { i + 1 } else { i };
                    for k in 0..model.num_vars {
                        replicas[w].attempts[k] =
                            replicas[b].attempts[k] * 0.7 + replicas[w].attempts[k] * 0.3;
                        replicas[w].successes[k] =
                            replicas[b].successes[k] * 0.7 + replicas[w].successes[k] * 0.3;
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
        global_best
    }
}

// ==========================================
// 5. ГРАНД-ФИНАЛ: БИТВА АЛГОРИТМОВ
// ==========================================
fn main() {
    println!("⚔️ БИТВА АЛГОРИТМОВ: Классический Вагнер-Уитин (DP) vs Квантовый Изинг-Движок");
    let modeler = SupplyChainModeler::new();

    // --- 1. КЛАССИЧЕСКИЙ DP (Секвенциальный подход снизу-вверх) ---
    let start_dp = Instant::now();
    let (cost0, ord0) = classic_wagner_whitin_dp(
        &modeler.demands[0],
        modeler.setup_costs[0],
        modeler.holding_costs[0],
    );
    let (cost1, ord1) = classic_wagner_whitin_dp(
        &modeler.demands[1],
        modeler.setup_costs[1],
        modeler.holding_costs[1],
    );
    let (cost2, ord2) = classic_wagner_whitin_dp(
        &modeler.demands[2],
        modeler.setup_costs[2],
        modeler.holding_costs[2],
    );

    // Формируем спрос для родителей
    let dem3: Vec<f64> = ord0.iter().zip(ord1.iter()).map(|(a, b)| a + b).collect();
    let dem4 = ord2.clone();

    let (cost3, ord3) =
        classic_wagner_whitin_dp(&dem3, modeler.setup_costs[3], modeler.holding_costs[3]);
    let (cost4, ord4) =
        classic_wagner_whitin_dp(&dem4, modeler.setup_costs[4], modeler.holding_costs[4]);

    // Формируем спрос для корня
    let dem5: Vec<f64> = ord3.iter().zip(ord4.iter()).map(|(a, b)| a + b).collect();
    let (cost5, _ord5) =
        classic_wagner_whitin_dp(&dem5, modeler.setup_costs[5], modeler.holding_costs[5]);

    let total_classic_cost = cost0 + cost1 + cost2 + cost3 + cost4 + cost5;
    let time_dp = start_dp.elapsed();

    // --- 2. КВАНТОВЫЙ ДВИЖОК (Глобальная оптимизация) ---
    println!("Формируем QUBO-матрицу для глобальной оптимизации...");
    let qubo = modeler.build_qubo();
    let solver = HybridSolver {
        num_replicas: 128,
        temp_max: 50_000_000.0,
        temp_min: 0.1,
        sweeps_per_exchange: 50,
        max_exchanges: 6000, // Чуть больше шагов для гарантии
    };

    println!("Запускаем Роевой Интеллект...");
    let start_q = Instant::now();
    let total_quantum_cost = solver.solve(&qubo);
    let time_q = start_q.elapsed();

    // --- 3. ИТОГИ ---
    println!("\n================== ИТОГОВЫЙ ОТЧЕТ ==================");
    println!("1. Классический ERP-подход (Локальный DP Вагнера-Уитина):");
    println!("   Стоимость логистики: ${}", total_classic_cost);
    println!("   Время расчета:       {:.2?}", time_dp);
    println!("----------------------------------------------------");
    println!("2. Наш движок Bayesian Swarm (Глобальный QUBO):");
    println!("   Стоимость логистики: ${}", total_quantum_cost);
    println!("   Время расчета:       {:.2?}", time_q);
    println!("====================================================");

    let savings = total_classic_cost - total_quantum_cost;
    if savings > 0.0 {
        println!(
            "🔥 АБСОЛЮТНАЯ ПОБЕДА! Наш алгоритм сэкономил корпорации ${}!",
            savings
        );
        println!("Потому что Изинг избежал 'эффекта хлыста', синхронизировав поставки глобально.");
    } else {
        println!(
            "⚠️ Квантовый алгоритм не смог превзойти классику (застрял в локальном минимуме)."
        );
    }
}
