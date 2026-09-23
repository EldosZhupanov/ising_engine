//! QOBLIB 2026 Problem Class 01: Market Split Problem Benchmark Runner.
//!
//! Evaluates UltimateSolver on the official QOBLIB 01-marketsplit problem class
//! (Nature Computational Science, 2026; IBM Quantum & Zuse Institute Berlin / ZIB).
//!
//! Reference: Cornuéjols & Dawande (1998) "A Class of Hard Small 0-1 Programs".
//! Verification: Official ZIB solution checker (`check_marketsplit.rs`, Thorsten Koch).

#![allow(clippy::needless_range_loop)]

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

pub struct MarketSplitInstance {
    pub name: String,
    pub num_cons: usize,
    pub num_vars: usize,
    pub matrix: Vec<Vec<i64>>,
    pub rhs: Vec<i64>,
}

pub fn parse_marketsplit_dat<P: AsRef<Path>>(
    path: P,
) -> Result<MarketSplitInstance, Box<dyn std::error::Error>> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);

    let name = path
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let mut num_cons = 0;
    let mut num_vars = 0;
    let mut matrix = Vec::new();
    let mut rhs = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if num_cons == 0 && num_vars == 0 {
            if parts.len() >= 2 {
                num_cons = parts[0].parse::<usize>()?;
                num_vars = parts[1].parse::<usize>()?;
            }
        } else if parts.len() > num_vars {
            let mut row = Vec::with_capacity(num_vars);
            for i in 0..num_vars {
                row.push(parts[i].parse::<i64>()?);
            }
            let b = parts[num_vars].parse::<i64>()?;
            matrix.push(row);
            rhs.push(b);
        }
    }

    Ok(MarketSplitInstance {
        name,
        num_cons,
        num_vars,
        matrix,
        rhs,
    })
}

/// Converts a Market Split instance Ax = b into an exact QUBO model:
/// Min H(x) = sum_k ( sum_j A_kj x_j - b_k )^2 >= 0
///
/// H(x) == 0 <=> all constraints Ax = b are satisfied.
pub fn marketsplit_to_qubo(inst: &MarketSplitInstance) -> (QuboModel, f64) {
    let n = inst.num_vars;
    let m = inst.num_cons;

    let mut linear = vec![0.0f64; n];
    let mut offset = 0.0f64;

    for k in 0..m {
        let bk = inst.rhs[k] as f64;
        offset += bk * bk;
        for j in 0..n {
            let a_kj = inst.matrix[k][j] as f64;
            // x_j^2 = x_j for binary x_j
            linear[j] += a_kj * a_kj - 2.0 * bk * a_kj;
        }
    }

    // Dense quadratic interaction J_ij = 2 * sum_k A_ki * A_kj
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = Vec::with_capacity(n + 1);
    row_offsets.push(0);

    for i in 0..n {
        for j in 0..n {
            if i != j {
                let mut sum_prod = 0.0f64;
                for k in 0..m {
                    sum_prod += (inst.matrix[k][i] as f64) * (inst.matrix[k][j] as f64);
                }
                let weight = 2.0 * sum_prod;
                if weight.abs() > 1e-9 {
                    col_indices.push(j);
                    values.push(weight);
                }
            }
        }
        row_offsets.push(col_indices.len());
    }

    let model = QuboModel {
        energy_offset: offset,
        num_vars: n,
        linear,
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };

    (model, offset)
}

/// Verifies whether `solution` satisfies all m subset sum constraints.
pub fn verify_marketsplit_solution(
    inst: &MarketSplitInstance,
    solution: &[i8],
) -> (bool, Vec<i64>, i64) {
    let mut residuals = Vec::with_capacity(inst.num_cons);
    let mut total_sq_error = 0i64;
    let mut is_feasible = true;

    for k in 0..inst.num_cons {
        let mut lhs = 0i64;
        for j in 0..inst.num_vars {
            if solution[j] == 1 {
                lhs += inst.matrix[k][j];
            }
        }
        let diff = lhs - inst.rhs[k];
        residuals.push(diff);
        total_sq_error += diff * diff;
        if diff != 0 {
            is_feasible = false;
        }
    }

    (is_feasible, residuals, total_sq_error)
}

/// Fast O(m * N^2) 1-opt and 2-opt exact quench for Market Split.
pub fn local_search_2opt_marketsplit(inst: &MarketSplitInstance, solution: &mut [i8]) -> i64 {
    let n = inst.num_vars;
    let m = inst.num_cons;
    let mut residuals: Vec<i64> = (0..m)
        .map(|k| {
            let mut sum = 0i64;
            for j in 0..n {
                if solution[j] == 1 {
                    sum += inst.matrix[k][j];
                }
            }
            sum - inst.rhs[k]
        })
        .collect();
    let mut current_energy: i64 = residuals.iter().map(|&r| r * r).sum();

    loop {
        if current_energy == 0 {
            break;
        }

        let mut best_delta = 0i64;
        let mut best_move: Option<(usize, Option<usize>)> = None;

        // 1-flip moves
        for p in 0..n {
            let dx_p = 1 - 2 * (solution[p] as i64);
            let mut delta = 0i64;
            for k in 0..m {
                let dr = dx_p * inst.matrix[k][p];
                delta += 2 * residuals[k] * dr + dr * dr;
            }
            if delta < best_delta {
                best_delta = delta;
                best_move = Some((p, None));
            }
        }

        // 2-flip moves (including swaps)
        for p in 0..n {
            let dx_p = 1 - 2 * (solution[p] as i64);
            for q in (p + 1)..n {
                let dx_q = 1 - 2 * (solution[q] as i64);
                let mut delta = 0i64;
                for k in 0..m {
                    let dr = dx_p * inst.matrix[k][p] + dx_q * inst.matrix[k][q];
                    delta += 2 * residuals[k] * dr + dr * dr;
                }
                if delta < best_delta {
                    best_delta = delta;
                    best_move = Some((p, Some(q)));
                }
            }
        }

        if let Some((p, maybe_q)) = best_move {
            let dx_p = 1 - 2 * (solution[p] as i64);
            solution[p] ^= 1;
            for k in 0..m {
                residuals[k] += dx_p * inst.matrix[k][p];
            }
            if let Some(q) = maybe_q {
                let dx_q = 1 - 2 * (solution[q] as i64);
                solution[q] ^= 1;
                for k in 0..m {
                    residuals[k] += dx_q * inst.matrix[k][q];
                }
            }
            current_energy += best_delta;
        } else {
            break;
        }
    }

    current_energy
}

/// Dynamic Tabu Search with 1-opt/2-opt moves and aspiration criterion for Market Split.
pub fn tabu_search_marketsplit(
    inst: &MarketSplitInstance,
    solution: &mut [i8],
    max_iters: usize,
) -> i64 {
    let n = inst.num_vars;
    let m = inst.num_cons;
    let mut residuals: Vec<i64> = (0..m)
        .map(|k| {
            let mut sum = 0i64;
            for j in 0..n {
                if solution[j] == 1 {
                    sum += inst.matrix[k][j];
                }
            }
            sum - inst.rhs[k]
        })
        .collect();
    let mut current_energy: i64 = residuals.iter().map(|&r| r * r).sum();
    let mut best_energy = current_energy;
    let mut best_solution = solution.to_vec();

    if best_energy == 0 {
        return 0;
    }

    let tenure = (n / 6).max(3);
    let mut tabu = vec![0usize; n];
    let mut no_improve = 0;

    for iter in 1..=max_iters {
        let mut best_move_delta = i64::MAX;
        let mut best_move: Option<(usize, Option<usize>)> = None;

        // 1-flip moves
        for p in 0..n {
            let is_allowed = tabu[p] < iter;
            let dx_p = 1 - 2 * (solution[p] as i64);
            let mut delta = 0i64;
            for k in 0..m {
                let dr = dx_p * inst.matrix[k][p];
                delta += 2 * residuals[k] * dr + dr * dr;
            }
            let cand_energy = current_energy + delta;
            let aspiration = cand_energy < best_energy;

            if (is_allowed || aspiration) && delta < best_move_delta {
                best_move_delta = delta;
                best_move = Some((p, None));
            }
        }

        // 2-flip moves
        for p in 0..n {
            let dx_p = 1 - 2 * (solution[p] as i64);
            for q in (p + 1)..n {
                let is_allowed = tabu[p] < iter && tabu[q] < iter;
                let dx_q = 1 - 2 * (solution[q] as i64);
                let mut delta = 0i64;
                for k in 0..m {
                    let dr = dx_p * inst.matrix[k][p] + dx_q * inst.matrix[k][q];
                    delta += 2 * residuals[k] * dr + dr * dr;
                }
                let cand_energy = current_energy + delta;
                let aspiration = cand_energy < best_energy;

                if (is_allowed || aspiration) && delta < best_move_delta {
                    best_move_delta = delta;
                    best_move = Some((p, Some(q)));
                }
            }
        }

        if let Some((p, maybe_q)) = best_move {
            let dx_p = 1 - 2 * (solution[p] as i64);
            solution[p] ^= 1;
            tabu[p] = iter + tenure;
            for k in 0..m {
                residuals[k] += dx_p * inst.matrix[k][p];
            }
            if let Some(q) = maybe_q {
                let dx_q = 1 - 2 * (solution[q] as i64);
                solution[q] ^= 1;
                tabu[q] = iter + tenure;
                for k in 0..m {
                    residuals[k] += dx_q * inst.matrix[k][q];
                }
            }
            current_energy += best_move_delta;

            if current_energy < best_energy {
                best_energy = current_energy;
                best_solution = solution.to_vec();
                no_improve = 0;
                if best_energy == 0 {
                    break;
                }
            } else {
                no_improve += 1;
                if no_improve > 50 {
                    // Perturbation: flip 2 pseudorandom spins to escape the basin
                    let p1 = (iter * 13 + 5) % n;
                    let p2 = (iter * 29 + 17) % n;
                    if p1 != p2 {
                        let dx1 = 1 - 2 * (solution[p1] as i64);
                        solution[p1] ^= 1;
                        for k in 0..m {
                            residuals[k] += dx1 * inst.matrix[k][p1];
                        }
                        let dx2 = 1 - 2 * (solution[p2] as i64);
                        solution[p2] ^= 1;
                        for k in 0..m {
                            residuals[k] += dx2 * inst.matrix[k][p2];
                        }
                        current_energy = residuals.iter().map(|&r| r * r).sum();
                        tabu.fill(0);
                        no_improve = 0;
                    }
                }
            }
        } else {
            break;
        }
    }

    solution.copy_from_slice(&best_solution);
    best_energy
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!("     QOBLIB 2026: MARKET SPLIT PROBLEM BENCHMARK (NATURE COMPUTATIONAL SCIENCE 2026)      ");
    println!("==========================================================================================");
    println!("Problem Class: 01-marketsplit (Multi-Dimensional Subset Sum / Exact Equality QUBO)");
    println!("Framework: IBM Quantum & Zuse Institute Berlin (ZIB) — The Intractable Decathlon");
    println!(
        "Validation: Official ZIB solution checker (check_marketsplit by Prof. Thorsten Koch)"
    );
    println!("------------------------------------------------------------------------------------------\n");

    let instance_paths = [
        "benchmarks/qoblib/marketsplit/instances/ms_03_050_002.dat",
        "benchmarks/qoblib/marketsplit/instances/ms_03_050_005.dat",
        "benchmarks/qoblib/marketsplit/instances/ms_03_050_007.dat",
        "benchmarks/qoblib/marketsplit/instances/ms_03_050_009.dat",
        "benchmarks/qoblib/marketsplit/instances/ms_03_100_001.dat",
        "benchmarks/qoblib/marketsplit/instances/ms_03_100_012.dat",
    ];

    println!(
        "{:<18} | {:>4} | {:>4} | {:>14} | {:>10} | {:>12} | {:>10}",
        "Instance", "Cons", "Vars", "QUBO Energy", "Violations", "Status", "Time"
    );
    println!(
        "{:-<18}-+-{:-<4}-+-{:-<4}-+-{:-<14}-+-{:-<10}-+-{:-<12}-+-{:-<10}",
        "", "", "", "", "", "", ""
    );

    for &path_str in &instance_paths {
        let path = Path::new(path_str);
        if !path.exists() {
            eprintln!("Warning: instance file {} not found, skipping", path_str);
            continue;
        }

        let inst = parse_marketsplit_dat(path)?;
        let (model, _offset) = marketsplit_to_qubo(&inst);

        let t0 = Instant::now();

        // Run search with UltimateSolver + 2-opt quenches + Tabu search
        let mut best_solution = vec![0i8; inst.num_vars];
        let mut best_error = i64::MAX;

        let seeds = [42u64, 101, 2026, 777, 999, 1234, 5555, 8888, 9999, 1337];

        for &seed in &seeds {
            let solver = UltimateSolver::new(500.0, 0.1, 40, 50, Some(seed)).with_2opt(true);
            let mut candidate = solver.solve(&model, &[]);
            let mut sq_err = local_search_2opt_marketsplit(&inst, &mut candidate);

            if sq_err > 0 {
                sq_err = tabu_search_marketsplit(&inst, &mut candidate, 1000);
            }

            if sq_err < best_error {
                best_error = sq_err;
                best_solution = candidate;
            }

            if sq_err == 0 {
                break;
            }
        }

        let elapsed = t0.elapsed();
        let (is_feas, residuals, _sq_err) = verify_marketsplit_solution(&inst, &best_solution);
        let violations = residuals.iter().filter(|&&r| r != 0).count();

        // Calculate model energy
        let qubo_energy = model.calculate_total_energy(&best_solution);

        let status_str = if is_feas {
            "VALID (FEAS)"
        } else {
            "INFEASIBLE"
        };

        println!(
            "{:<18} | {:>4} | {:>4} | {:>14.1} | {:>10} | {:>12} | {:>9.2?}",
            inst.name, inst.num_cons, inst.num_vars, qubo_energy, violations, status_str, elapsed
        );

        // Save solution
        let sol_dir = Path::new("benchmarks/qoblib/marketsplit/solutions");
        std::fs::create_dir_all(sol_dir)?;
        let sol_path = sol_dir.join(format!("{}.sol", inst.name));
        let sol_str: String = best_solution
            .iter()
            .map(|&v| if v == 1 { "1" } else { "0" })
            .collect::<Vec<&str>>()
            .join(" ");
        std::fs::write(&sol_path, format!("{}\n", sol_str))?;

        // Verify with official checker
        let checker_path = Path::new("target/release/check_marketsplit");
        if checker_path.exists() {
            let output = std::process::Command::new(checker_path)
                .arg(path)
                .arg(&sol_path)
                .output()?;

            let out_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                println!("   => [Official ZIB Checker]: VALID (Exit code 0)");
            } else {
                println!(
                    "   => [Official ZIB Checker]: Exit code {} | stdout: {} | stderr: {}",
                    output.status.code().unwrap_or(-1),
                    out_str.trim(),
                    err_str.trim()
                );
            }
        }
    }

    println!("\n==========================================================================================");
    println!("Benchmark Complete. Solutions saved in benchmarks/qoblib/marketsplit/solutions/");
    println!("==========================================================================================");

    Ok(())
}
