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
// 2. ФОНДОВЫЙ РЫНОК (Генератор QUBO для Портфеля)
// ==========================================
pub struct PortfolioBuilder {
    num_assets: usize,
    target_k: usize,
    returns: Vec<f64>,
    covariance: Vec<Vec<f64>>,
}

impl PortfolioBuilder {
    pub fn new(num_assets: usize, target_k: usize) -> Self {
        let mut rng = rand::thread_rng();
        let returns: Vec<f64> = (0..num_assets).map(|_| rng.gen_range(2.0..15.0)).collect();
        let mut covariance = vec![vec![0.0; num_assets]; num_assets];
        for i in 0..num_assets {
            for j in i..num_assets {
                if i == j {
                    covariance[i][j] = rng.gen_range(1.0..5.0);
                } else {
                    let cov = rng.gen_range(-2.0..4.0);
                    covariance[i][j] = cov;
                    covariance[j][i] = cov;
                }
            }
        }
        Self {
            num_assets,
            target_k,
            returns,
            covariance,
        }
    }

    pub fn build_qubo(&self, alpha: f64, beta: f64, gamma: f64) -> QuboModel {
        let mut linear = vec![0.0; self.num_assets];
        let mut quadratic_edges = vec![];
        let k_f64 = self.target_k as f64;

        for i in 0..self.num_assets {
            linear[i] = -alpha * self.returns[i] + gamma * (1.0 - 2.0 * k_f64);
            for j in (i + 1)..self.num_assets {
                let weight = beta * self.covariance[i][j] + 2.0 * gamma;
                if weight.abs() > 1e-6 {
                    // ИСПРАВЛЕНИЕ: Убрали умножение на 2.0. Парсер и так симметричен!
                    quadratic_edges.push((i, j, weight));
                    quadratic_edges.push((j, i, weight));
                }
            }
        }

        let mut row_edges: Vec<Vec<(usize, f64)>> = vec![vec![]; self.num_assets];
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
            num_vars: self.num_assets,
            linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
        }
    }

    pub fn evaluate_portfolio(&self, state: &[i8]) -> (usize, f64, f64) {
        let mut selected = 0;
        let mut total_return = 0.0;
        let mut total_risk = 0.0;
        for i in 0..self.num_assets {
            if state[i] == 1 {
                selected += 1;
                total_return += self.returns[i];
                for j in 0..self.num_assets {
                    if state[j] == 1 {
                        total_risk += self.covariance[i][j];
                    }
                }
            }
        }
        (selected, total_return, total_risk)
    }
}

// ==========================================
// 3. PURE SIMULATED ANNEALING
// ==========================================
pub struct PureSASolver {
    pub total_sweeps: usize,
    pub temp_max: f64,
    pub temp_min: f64,
}
impl PureSASolver {
    fn calculate_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
        let flip_multiplier = 1.0 - 2.0 * (state[var_idx] as f64);
        let mut sum_j = 0.0;
        for (col, weight) in model.quadratic.get_row(var_idx) {
            sum_j += weight * (state[col] as f64);
        }
        flip_multiplier * (model.linear[var_idx] + sum_j)
    }
    pub fn solve(&self, model: &QuboModel) -> Vec<i8> {
        let mut rng = rand::thread_rng();
        let mut state: Vec<i8> = (0..model.num_vars).map(|_| rng.gen_range(0..=1)).collect();
        let mut current_temp = self.temp_max;
        let cooling_rate = (self.temp_min / self.temp_max).powf(1.0 / self.total_sweeps as f64);
        for _ in 0..self.total_sweeps {
            let var_idx = rng.gen_range(0..model.num_vars);
            let delta_e = Self::calculate_delta_e(model, &state, var_idx);
            if delta_e < 0.0
                || (current_temp > 1e-8
                    && rng.gen_range(0.0_f64..1.0_f64) < E.powf(-delta_e / current_temp))
            {
                state[var_idx] = 1 - state[var_idx];
            }
            current_temp *= cooling_rate;
        }
        state
    }
}

// ==========================================
// 4. TUNED HYBRID ENGINE (Для жестких ограничений)
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
    pub total_exchanges: usize,
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
                    priority_map: vec![1.0; model.num_vars],
                }
            })
            .collect();

        let mut swap_accepts = vec![0; self.num_replicas - 1];

        for exchange_step in 0..self.total_exchanges {
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
        replicas
            .into_iter()
            .min_by(|a, b| a.energy.partial_cmp(&b.energy).unwrap())
            .unwrap()
            .state
    }
}

// ==========================================
// 5. MAIN - PORTFOLIO OPTIMIZATION BENCHMARK
// ==========================================
fn main() {
    println!("🦀 B2B USE CASE: Markowitz Portfolio Optimization (Constrained QUBO)");
    println!("Выбираем идеальные активы в условиях жестких ограничений.");
    println!("------------------------------------------------------------------");

    let num_assets = 200;
    let target_k = 20; // Теперь график штрафов настроен идеально!

    let builder = PortfolioBuilder::new(num_assets, target_k);
    let model = builder.build_qubo(1.0, 0.5, 100.0);

    println!(
        "Рынок сгенерирован: {} активов. Задача: собрать портфель из {}.",
        num_assets, target_k
    );
    println!("------------------------------------------------------------------");

    let sa_solver = PureSASolver {
        total_sweeps: 2_000_000,
        temp_max: 1000.0,
        temp_min: 0.001,
    };
    println!("[1/2] Запуск Pure SA (Слепой спуск, 2M итераций)...");
    let sa_start = Instant::now();
    let sa_result = sa_solver.solve(&model);
    let sa_duration = sa_start.elapsed();
    let (sa_k, sa_ret, sa_risk) = builder.evaluate_portfolio(&sa_result);

    let hybrid_solver = HybridSolver {
        num_replicas: 128,
        temp_max: 1000.0,
        temp_min: 0.001,
        sweeps_per_exchange: 200,
        total_exchanges: 1000,
        adaptation_interval: 20,
    };
    println!("[2/2] Запуск Hybrid Engine (Туннелирование сквозь барьеры)...");
    let hybrid_start = Instant::now();
    let hybrid_result = hybrid_solver.solve(&model);
    let hybrid_duration = hybrid_start.elapsed();
    let (hyb_k, hyb_ret, hyb_risk) = builder.evaluate_portfolio(&hybrid_result);

    println!("\n================ РЕЗУЛЬТАТЫ ИНВЕСТИРОВАНИЯ ================");
    println!(
        "{:>15} | {:>10} | {:>12} | {:>10} | {:>10}",
        "Алгоритм", "Время", "Выбрано (K)", "Доходность", "Риск"
    );
    println!("---------------------------------------------------------------------------");
    println!(
        "{:>15} | {:>10} | {:>12} | {:>10.1} | {:>10.1}",
        "Pure SA",
        format!("{}ms", sa_duration.as_millis()),
        sa_k,
        sa_ret,
        sa_risk
    );
    println!(
        "{:>15} | {:>10} | {:>12} | {:>10.1} | {:>10.1}",
        "Hybrid Engine",
        format!("{}ms", hybrid_duration.as_millis()),
        hyb_k,
        hyb_ret,
        hyb_risk
    );
    println!("===========================================================================");

    if hyb_k == target_k && sa_k != target_k {
        println!(
            "✅ БЕЗОГОВОРОЧНАЯ ПОБЕДА! SA разбился о стену штрафов и собрал {} акций.",
            sa_k
        );
        println!(
            "✅ Наш Hybrid пробил энергетический барьер и точно собрал портфель из {} акций!",
            target_k
        );
    } else if hyb_k == target_k && sa_k == target_k {
        println!("Оба справились с ограничением. Смотрим на доходность/риск.");
    } else {
        println!(
            "❌ Гибрид собрал {} акций. Нужна дополнительная настройка температур.",
            hyb_k
        );
    }
}
