//! QOBLIB 2026 Problem Class 01: Multi-Armed Lambda-Lattice Kernel Solver.
//!
//! Experimental Diophantine architecture:
//! 1. Uses LLL to extract verified null vectors and a candidate integer basis.
//!    and a particular solution x_0 satisfying A x_0 = b.
//! 2. Searches in the lambda-coordinate space x(lambda) = x_0 + sum_i lambda_i * v_i.
//! 3. Mathematically, A x = b is preserved for all integer lambda when Av=0 and Ax0=b.
//! 4. The objective function is purely boolean feasibility: min sum_j dist(x_j, {0, 1})^2.
//! 5. Employs 4 distinct parallel meta-heuristic search arms with exact recomputation:
//!    - Thread 0: Raw LLL Warm-Start with Breakout Coordinate Weighting
//!    - Thread 1: Least-Squares CVP Projection with Simulated Annealing
//!    - Thread 2: Variable Neighborhood Search (VNS with 1- and 2-step coordinate jumps)
//!    - Thread 3: Dynamic Tabu with adaptive tenure and aspiration
//! 6. Independent verification via official ZIB verifier `check_marketsplit`.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::inconsistent_digit_grouping)]

#[path = "common/marketsplit_args.rs"]
mod marketsplit_args;
#[path = "common/marketsplit_certificate.rs"]
mod marketsplit_certificate;

use ising_engine::core::lattice::aardal_diophantine_reduction;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct MarketSplitInstance {
    pub name: String,
    pub path: PathBuf,
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
        path: path.as_ref().to_path_buf(),
        num_cons,
        num_vars,
        matrix,
        rhs,
    })
}

#[inline(always)]
fn coordinate_dist(val: i64) -> i64 {
    if val < 0 {
        -val
    } else if val > 1 {
        val - 1
    } else {
        0
    }
}

/// Solves Least-Squares: min ||V * lambda - (0.5 - x0)||^2 to find initial lambda in Z^r
pub fn project_target_onto_kernel(x0: &[i64], kernel: &[Vec<i64>]) -> Vec<i64> {
    let n = x0.len();
    let r = kernel.len();
    let y: Vec<f64> = x0.iter().map(|&x| 0.5 - (x as f64)).collect();

    let mut g = vec![vec![0.0f64; r]; r];
    let mut rhs = vec![0.0f64; r];

    for i in 0..r {
        for j in 0..n {
            rhs[i] += (kernel[i][j] as f64) * y[j];
        }
        for k in 0..=i {
            let mut dot = 0.0f64;
            for j in 0..n {
                dot += (kernel[i][j] as f64) * (kernel[k][j] as f64);
            }
            g[i][k] = dot;
            g[k][i] = dot;
        }
        g[i][i] += 1e-4;
    }

    let mut lambda = vec![0.0f64; r];
    let mut res = rhs.clone();
    let mut p = res.clone();
    let mut rs_old: f64 = res.iter().map(|&x| x * x).sum();

    for _ in 0..300 {
        if rs_old < 1e-11 {
            break;
        }
        let mut ap = vec![0.0f64; r];
        for i in 0..r {
            for k in 0..r {
                ap[i] += g[i][k] * p[k];
            }
        }
        let p_dot_ap: f64 = p.iter().zip(ap.iter()).map(|(&a, &b)| a * b).sum();
        if p_dot_ap.abs() < 1e-14 {
            break;
        }
        let alpha = rs_old / p_dot_ap;
        for i in 0..r {
            lambda[i] += alpha * p[i];
            res[i] -= alpha * ap[i];
        }
        let rs_new: f64 = res.iter().map(|&x| x * x).sum();
        let beta = rs_new / rs_old;
        rs_old = rs_new;
        for i in 0..r {
            p[i] = res[i] + beta * p[i];
        }
    }

    lambda.iter().map(|&v| v.round() as i64).collect()
}

#[derive(Clone)]
pub struct LambdaState {
    pub n: usize,
    pub r: usize,
    pub lambda: Vec<i64>,
    pub x: Vec<i64>,
    pub cost: i64,
    pub out_of_bounds: usize,
}

impl LambdaState {
    pub fn from_lambda(x0: &[i64], kernel: &[Vec<i64>], lambda: Vec<i64>) -> Self {
        let n = x0.len();
        let r = kernel.len();
        let mut x = x0.to_vec();

        for i in 0..r {
            if lambda[i] != 0 {
                let v = &kernel[i];
                for j in 0..n {
                    x[j] += lambda[i] * v[j];
                }
            }
        }

        let mut cost = 0i64;
        let mut out_of_bounds = 0usize;
        for &val in &x {
            let d = coordinate_dist(val);
            cost += d * d;
            if d > 0 {
                out_of_bounds += 1;
            }
        }

        Self {
            n,
            r,
            lambda,
            x,
            cost,
            out_of_bounds,
        }
    }

    #[inline(always)]
    pub fn is_solved(&self) -> bool {
        self.out_of_bounds == 0 && self.cost == 0 && self.x.iter().all(|&v| v == 0 || v == 1)
    }

    #[inline(always)]
    pub fn eval_delta(&self, kernel: &[Vec<i64>], i: usize, delta: i64) -> (i64, i64) {
        let v = &kernel[i];
        let mut d_cost = 0i64;
        let mut d_oob = 0i64;

        for j in 0..self.n {
            let old_x = self.x[j];
            let new_x = old_x + delta * v[j];

            let old_d = coordinate_dist(old_x);
            let new_d = coordinate_dist(new_x);
            d_cost += new_d * new_d - old_d * old_d;

            let old_bad = if old_d > 0 { 1i64 } else { 0i64 };
            let new_bad = if new_d > 0 { 1i64 } else { 0i64 };
            d_oob += new_bad - old_bad;
        }

        (d_cost, d_oob)
    }

    #[inline(always)]
    pub fn apply_move(&mut self, kernel: &[Vec<i64>], i: usize, delta: i64) {
        self.lambda[i] += delta;
        let v = &kernel[i];
        let mut new_cost = 0i64;
        let mut new_oob = 0usize;

        for j in 0..self.n {
            self.x[j] += delta * v[j];
            let d = coordinate_dist(self.x[j]);
            new_cost += d * d;
            if d > 0 {
                new_oob += 1;
            }
        }
        self.cost = new_cost;
        self.out_of_bounds = new_oob;
    }
}

/// Solves the instance in lambda-kernel space using 4 distinct parallel meta-heuristic search arms.
pub fn solve_in_lambda_space(
    inst: &MarketSplitInstance,
    x0: &[i64],
    kernel: &[Vec<i64>],
    timeout: Option<Duration>,
) -> Option<Vec<i8>> {
    if kernel.is_empty() {
        return x0
            .iter()
            .all(|&value| value == 0 || value == 1)
            .then(|| x0.iter().map(|&value| value as i8).collect());
    }
    let t0 = Instant::now();
    let r = kernel.len();
    let n = x0.len();
    let solved_flag = Arc::new(AtomicBool::new(false));

    // Evaluate both start configurations
    let raw_state = LambdaState::from_lambda(x0, kernel, vec![0i64; r]);
    let cvp_lambda = project_target_onto_kernel(x0, kernel);
    let cvp_state = LambdaState::from_lambda(x0, kernel, cvp_lambda);

    println!(
        "   [LATTICE INIT] Raw LLL x0 OOB: {}/{} (Cost: {}) | CVP Projected OOB: {}/{} (Cost: {})",
        raw_state.out_of_bounds, n, raw_state.cost, cvp_state.out_of_bounds, n, cvp_state.cost
    );

    if raw_state.is_solved() {
        let bool_sol: Vec<i8> = raw_state.x.iter().map(|&v| v as i8).collect();
        return Some(bool_sol);
    }
    if cvp_state.is_solved() {
        let bool_sol: Vec<i8> = cvp_state.x.iter().map(|&v| v as i8).collect();
        return Some(bool_sol);
    }

    let best_initial = if raw_state.out_of_bounds <= cvp_state.out_of_bounds {
        raw_state
    } else {
        cvp_state
    };

    let global_best = Arc::new(Mutex::new((best_initial.cost, best_initial.out_of_bounds)));
    let num_threads = rayon::current_num_threads();

    (0..num_threads).into_par_iter().find_map_any(|tid| {
        let mut rng = ChaCha8Rng::seed_from_u64(20260925_400000u64 + (tid as u64) * 104729);
        let mut tabu = vec![0usize; r];

        // Seed threads with diversity
        let mut state = match tid % 4 {
            0 => LambdaState::from_lambda(x0, kernel, vec![0i64; r]),
            1 => LambdaState::from_lambda(x0, kernel, project_target_onto_kernel(x0, kernel)),
            _ => best_initial.clone(),
        };
        let mut best_state = state.clone();

        let mut step = 0usize;
        let mut no_improve = 0usize;
        let mut temp = 20.0f64;

        let is_timeout = |start: Instant| -> bool {
            if let Some(limit) = timeout {
                start.elapsed() >= limit
            } else {
                false
            }
        };

        while !is_timeout(t0) && !solved_flag.load(Ordering::Relaxed) {
            step += 1;

            if state.is_solved() {
                // Strict mathematical verification of Ax == b
                let mut valid = true;
                for k in 0..inst.num_cons {
                    let mut sum = 0i64;
                    for j in 0..inst.num_vars {
                        if state.x[j] == 1 {
                            sum += inst.matrix[k][j];
                        }
                    }
                    if sum != inst.rhs[k] {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    solved_flag.store(true, Ordering::SeqCst);
                    let bool_sol: Vec<i8> = state.x.iter().map(|&v| v as i8).collect();
                    return Some(bool_sol);
                }
            }

            match tid % 4 {
                0 => {
                    // Arm 0: Greedy coordinate descent with tabu
                    let mut best_move: Option<(usize, i64)> = None;
                    let mut best_eval_score = i64::MAX;

                    for i in 0..r {
                        if tabu[i] >= step { continue; }
                        for &delta in &[-1i64, 1i64, -2i64, 2i64] {
                            let (d_cost, d_oob) = state.eval_delta(kernel, i, delta);
                            let cand_oob = state.out_of_bounds as i64 + d_oob;
                            let cand_cost = state.cost + d_cost;
                            let score = cand_oob * 5000 + cand_cost;

                            if score < best_eval_score {
                                best_eval_score = score;
                                best_move = Some((i, delta));
                            }
                        }
                    }

                    if let Some((i, delta)) = best_move {
                        state.apply_move(kernel, i, delta);
                        tabu[i] = step + rng.gen_range(5..=20);

                        if state.out_of_bounds < best_state.out_of_bounds || (state.out_of_bounds == best_state.out_of_bounds && state.cost < best_state.cost) {
                            best_state = state.clone();
                            no_improve = 0;
                            let mut g = global_best.lock().unwrap();
                            if state.out_of_bounds < g.1 || (state.out_of_bounds == g.1 && state.cost < g.0) {
                                g.0 = state.cost; g.1 = state.out_of_bounds;
                                println!("   🔥 [ARM 0 Greedy]   Incumbent: OOB {}/{} | Cost {} ({:.2}s)", state.out_of_bounds, n, state.cost, t0.elapsed().as_secs_f64());
                            }
                        } else {
                            no_improve += 1;
                            if no_improve > 120 {
                                state = best_state.clone();
                                let kick_i = rng.gen_range(0..r);
                                let kick_d = if rng.gen_bool(0.5) { 1 } else { -1 };
                                state.apply_move(kernel, kick_i, kick_d);
                                no_improve = 0;
                            }
                        }
                    }
                }
                1 => {
                    // Arm 1: Simulated Annealing in Lambda-space
                    temp *= 0.99998;
                    if temp < 0.2 { temp = 25.0; }

                    let i = rng.gen_range(0..r);
                    let delta = if rng.gen_bool(0.5) { 1i64 } else { -1i64 };
                    let (d_cost, d_oob) = state.eval_delta(kernel, i, delta);
                    let delta_e = (d_oob * 1000 + d_cost) as f64;

                    if delta_e <= 0.0 || rng.gen_bool((-delta_e / temp).exp().clamp(0.0, 1.0)) {
                        state.apply_move(kernel, i, delta);

                        if state.out_of_bounds < best_state.out_of_bounds || (state.out_of_bounds == best_state.out_of_bounds && state.cost < best_state.cost) {
                            best_state = state.clone();
                            let mut g = global_best.lock().unwrap();
                            if state.out_of_bounds < g.1 || (state.out_of_bounds == g.1 && state.cost < g.0) {
                                g.0 = state.cost; g.1 = state.out_of_bounds;
                                println!("   🔥 [ARM 1 Anneal]   Incumbent: OOB {}/{} | Cost {} ({:.2}s)", state.out_of_bounds, n, state.cost, t0.elapsed().as_secs_f64());
                            }
                        }
                    }
                }
                2 => {
                    // Arm 2: Variable Neighborhood Search (sequential evaluations)
                    let i = rng.gen_range(0..r);
                    let delta = if rng.gen_bool(0.5) { 1i64 } else { -1i64 };
                    let (d_cost, d_oob) = state.eval_delta(kernel, i, delta);

                    if d_oob < 0 || (d_oob == 0 && d_cost < 0) || (no_improve > 150 && rng.gen_bool(0.05)) {
                        state.apply_move(kernel, i, delta);

                        if state.out_of_bounds < best_state.out_of_bounds || (state.out_of_bounds == best_state.out_of_bounds && state.cost < best_state.cost) {
                            best_state = state.clone();
                            no_improve = 0;
                            let mut g = global_best.lock().unwrap();
                            if state.out_of_bounds < g.1 || (state.out_of_bounds == g.1 && state.cost < g.0) {
                                g.0 = state.cost; g.1 = state.out_of_bounds;
                                println!("   🔥 [ARM 2 VNS]      Incumbent: OOB {}/{} | Cost {} ({:.2}s)", state.out_of_bounds, n, state.cost, t0.elapsed().as_secs_f64());
                            }
                        }
                    } else {
                        no_improve += 1;
                        if no_improve > 300 {
                            state = best_state.clone();
                            no_improve = 0;
                        }
                    }
                }
                _ => {
                    // Arm 3: Dynamic Tabu with random tenure
                    let tenure = rng.gen_range(5..=25);
                    let mut best_move: Option<(usize, i64)> = None;
                    let mut best_eval_score = i64::MAX;

                    for i in 0..r {
                        let is_allowed = tabu[i] < step;
                        for &delta in &[-1i64, 1i64] {
                            let (d_cost, d_oob) = state.eval_delta(kernel, i, delta);
                            let score = (state.out_of_bounds as i64 + d_oob) * 5000 + (state.cost + d_cost);
                            let aspiration = (state.out_of_bounds as i64 + d_oob) < (best_state.out_of_bounds as i64);

                            if (is_allowed || aspiration) && score < best_eval_score {
                                best_eval_score = score;
                                best_move = Some((i, delta));
                            }
                        }
                    }

                    if let Some((i, delta)) = best_move {
                        state.apply_move(kernel, i, delta);
                        tabu[i] = step + tenure;

                        if state.out_of_bounds < best_state.out_of_bounds || (state.out_of_bounds == best_state.out_of_bounds && state.cost < best_state.cost) {
                            best_state = state.clone();
                            let mut g = global_best.lock().unwrap();
                            if state.out_of_bounds < g.1 || (state.out_of_bounds == g.1 && state.cost < g.0) {
                                g.0 = state.cost; g.1 = state.out_of_bounds;
                                println!("   🔥 [ARM 3 Tabu]     Incumbent: OOB {}/{} | Cost {} ({:.2}s)", state.out_of_bounds, n, state.cost, t0.elapsed().as_secs_f64());
                            }
                        }
                    }
                }
            }
        }

        None
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!("     QOBLIB 2026: MULTI-ARMED LAMBDA-LATTICE SOLVER (CONTINUOUS HUNT)                     ");
    println!("==========================================================================================");
    println!("Method: LLL Basis Reduction + Kernel Optimization + Multi-Arm Meta-Heuristics");
    println!("Invariant under exact arithmetic: A x = b for verified x0 and null vectors");
    println!(
        "Threads: {} Rayon worker threads",
        rayon::current_num_threads()
    );
    println!("------------------------------------------------------------------------------------------\n");

    let args = marketsplit_args::parse(&std::env::args().collect::<Vec<_>>())?;
    let single_instance = args.instance;
    let timeout_secs = args.timeout_secs;

    let unsolved_dir = Path::new("benchmarks/qoblib/marketsplit/unsolved_instances");
    let sol_dir = Path::new("benchmarks/qoblib/marketsplit/unsolved_solutions");
    std::fs::create_dir_all(sol_dir)?;

    let entries: Vec<PathBuf> = if let Some(target) = single_instance {
        vec![target]
    } else {
        let mut list: Vec<PathBuf> = std::fs::read_dir(unsolved_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "dat"))
            .collect();
        list.sort();
        list
    };

    println!(
        "Targeting {} known-feasible benchmark instance(s). Continuous Search: {}.\n",
        entries.len(),
        timeout_secs.is_none()
    );

    for path in &entries {
        let inst = parse_marketsplit_dat(path)?;
        let t0 = Instant::now();

        println!(
            ">>> Attacking instance: {} (m={}, n={})...",
            inst.name, inst.num_cons, inst.num_vars
        );

        let lll_res = aardal_diophantine_reduction(&inst.matrix, &inst.rhs, 10_000, 10);
        let lll_time = t0.elapsed();

        match lll_res {
            Ok(res) => {
                let kernel_dim = res.kernel_basis.len();
                println!(
                    "    LLL Reduction complete in {:.2}s. Kernel dimension: {}.",
                    lll_time.as_secs_f64(),
                    kernel_dim
                );

                if let Some(ref exact_bool) = res.exact_boolean_solution {
                    let checker = Path::new("target/release/check_marketsplit");
                    let rejected_dir =
                        Path::new("benchmarks/qoblib/marketsplit/rejected_candidates");
                    match marketsplit_certificate::certify(
                        path,
                        &inst.name,
                        exact_bool,
                        checker,
                        sol_dir,
                        Path::new("benchmarks/qoblib/marketsplit/unsolved_candidates"),
                        rejected_dir,
                    )? {
                        marketsplit_certificate::CertificateStatus::Verified(file) => {
                            println!("Verified LLL certificate: {}", file.display());
                            continue;
                        }
                        marketsplit_certificate::CertificateStatus::Rejected(file) => {
                            eprintln!("LLL candidate rejected and retained: {}", file.display());
                        }
                    }
                }

                if let Some(ref x0) = res.particular_solution {
                    let budget = timeout_secs.map(Duration::from_secs);
                    let solved = solve_in_lambda_space(&inst, x0, &res.kernel_basis, budget);

                    if let Some(ref sol) = solved {
                        let checker = Path::new("target/release/check_marketsplit");
                        let rejected_dir =
                            Path::new("benchmarks/qoblib/marketsplit/rejected_candidates");
                        match marketsplit_certificate::certify(
                            path,
                            &inst.name,
                            sol,
                            checker,
                            sol_dir,
                            Path::new("benchmarks/qoblib/marketsplit/unsolved_candidates"),
                            rejected_dir,
                        )? {
                            marketsplit_certificate::CertificateStatus::Verified(file) => {
                                println!("Verified lattice certificate: {}", file.display());
                            }
                            marketsplit_certificate::CertificateStatus::Rejected(file) => {
                                eprintln!(
                                    "Lattice candidate rejected and retained: {}",
                                    file.display()
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("    LLL Reduction failed: {}", e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod lattice_guard_tests {
    use super::*;

    #[test]
    fn empty_extracted_kernel_does_not_start_integer_search() {
        let inst = MarketSplitInstance {
            name: "toy".to_string(),
            path: PathBuf::new(),
            num_cons: 1,
            num_vars: 2,
            matrix: vec![vec![1, 1]],
            rhs: vec![1],
        };
        assert_eq!(solve_in_lambda_space(&inst, &[2, -1], &[], None), None);
        assert_eq!(
            solve_in_lambda_space(&inst, &[1, 0], &[], None),
            Some(vec![1, 0])
        );
    }
}
