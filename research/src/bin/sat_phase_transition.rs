#![allow(warnings)]
use rand::Rng;
use rayon::prelude::*;
use std::f64::consts::E;

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
// 2. 3-SAT КОМПИЛЯТОР (Редукция в MIS QUBO)
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
        let linear = vec![-1.0; qubo_vars]; // Награда за выполнение условия
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
// 3. PURE PARALLEL TEMPERING (Без жадного ML)
// ==========================================
#[derive(Clone)]
struct Replica {
    state: Vec<i8>,
    temp: f64,
    energy: f64,
}

pub struct PTSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub max_exchanges: usize,
}
impl PTSolver {
    fn calculate_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
        let flip_multiplier = 1.0 - 2.0 * (state[var_idx] as f64);
        let mut sum_j = 0.0;
        for (col, weight) in model.quadratic.get_row(var_idx) {
            sum_j += weight * (state[col] as f64);
        }
        flip_multiplier * (model.linear[var_idx] + sum_j)
    }

    pub fn solve_until_target(&self, model: &QuboModel, target_energy: f64) -> (bool, usize) {
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

        for exchange_step in 0..self.max_exchanges {
            // Проверка: нашли ли мы глобальный минимум (все скобки решены)
            let current_min = replicas
                .iter()
                .map(|r| r.energy)
                .fold(f64::INFINITY, f64::min);
            if current_min <= target_energy + 1e-6 {
                return (true, exchange_step);
            }

            // ЧИСТАЯ ФИЗИКА: Нет ML-отсечений, исследуем всё
            replicas.par_iter_mut().for_each(|replica| {
                let mut local_rng = rand::thread_rng();
                for _ in 0..self.sweeps_per_exchange {
                    let var_idx = local_rng.gen_range(0..model.num_vars);
                    let delta_e = Self::calculate_delta_e(model, &replica.state, var_idx);

                    if delta_e < 0.0
                        || (replica.temp > 1e-8
                            && local_rng.gen_range(0.0_f64..1.0_f64)
                                < E.powf(-delta_e / replica.temp))
                    {
                        replica.state[var_idx] = 1 - replica.state[var_idx];
                    }
                }
                replica.energy = model.calculate_total_energy(&replica.state);
            });

            // Обмен температурами
            for i in 0..(self.num_replicas - 1) {
                let delta_beta = (1.0 / replicas[i].temp) - (1.0 / replicas[i + 1].temp);
                let delta_energy = replicas[i].energy - replicas[i + 1].energy;
                let swap_prob = (delta_beta * delta_energy).exp();
                if swap_prob >= 1.0 || rng.gen_range(0.0_f64..1.0_f64) < swap_prob {
                    let temp_state = replicas[i].state.clone();
                    replicas[i].state = replicas[i + 1].state.clone();
                    replicas[i + 1].state = temp_state;
                    let temp_energy = replicas[i].energy;
                    replicas[i].energy = replicas[i + 1].energy;
                    replicas[i + 1].energy = temp_energy;
                }
            }
        }
        (false, self.max_exchanges)
    }
}

// ==========================================
// 4. MAIN - PHASE TRANSITION LAB
// ==========================================
fn main() {
    println!("🔬 3-SAT PHASE TRANSITION LAB: В поисках Края Хаоса (4.26)");
    println!("N = 25 переменных. Сканируем отношение Clauses/Vars (α) от 3.0 до 5.0");
    println!("Используем чистый термодинамический движок (Parallel Tempering)");
    println!("----------------------------------------------------------------------------------");
    println!(
        "{:>10} | {:>10} | {:>15} | {:>15} | {:>15}",
        "Ratio (α)", "Clauses(M)", "% Solved", "Avg Exchanges", "Hardness Factor"
    );
    println!("----------------------------------------------------------------------------------");

    let num_vars = 25;
    let instances_per_ratio = 50; // Увеличили выборку до 50 для красоты графика
    let alpha_ratios = vec![3.0, 3.4, 3.8, 4.0, 4.26, 4.5, 5.0];

    let solver = PTSolver {
        num_replicas: 64,
        temp_max: 100.0, // SAT требует более горячего старта
        temp_min: 0.01,
        sweeps_per_exchange: 50,
        max_exchanges: 500, // Лимит 500 обменов
    };

    for &alpha in &alpha_ratios {
        let num_clauses = (num_vars as f64 * alpha).round() as usize;
        let target_energy = -(num_clauses as f64);

        let mut solved_count = 0;
        let mut total_exchanges_used = 0;

        for _ in 0..instances_per_ratio {
            let builder = Sat3Builder::generate_random(num_vars, num_clauses);
            let qubo_model = builder.build_qubo();

            let (solved, exchanges) = solver.solve_until_target(&qubo_model, target_energy);

            if solved {
                solved_count += 1;
            }
            total_exchanges_used += exchanges;
        }

        let solved_percent = (solved_count as f64 / instances_per_ratio as f64) * 100.0;
        let avg_exchanges = total_exchanges_used as f64 / instances_per_ratio as f64;
        let marker = if alpha == 4.26 {
            "🔥 (Край Хаоса)"
        } else {
            ""
        };

        println!(
            "{:>10.2} | {:>10} | {:>14.1}% | {:>15.1} | {:>10.1} {}",
            alpha, num_clauses, solved_percent, avg_exchanges, avg_exchanges, marker
        );
    }
}
