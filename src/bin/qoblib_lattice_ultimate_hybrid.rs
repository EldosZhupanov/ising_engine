//! QOBLIB 2026 Problem Class 01: Lattice-Ultimate Hybrid Solver.
//!
//! Experimental combination:
//! 1. LLL Basis Reduction extracts verified null vectors and a particular x0.
//! 2. Under exact arithmetic, Ax = b is preserved for verified x0 and null vectors.
//! 3. Objective is exact quadratic Hamiltonian: Phi(lambda) = lambda^T G lambda + c^T lambda + C0.
//!    Global min Phi == 0 iff all x_j in {0, 1}.
//! 4. SIMD UltimateSolver explores trust-region QUBOs with 64 replicas, ICM, and 2-opt.
//! 5. Integer Gram Solver evaluates a coordinate delta in O(1) after gradient maintenance.
//! 6. Independent validation via official ZIB verifier `check_marketsplit`.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::inconsistent_digit_grouping)]

#[path = "common/marketsplit_certificate.rs"]
mod marketsplit_certificate;

use ising_engine::core::lattice::{aardal_diophantine_reduction, build_kernel_qubo, LatticeBasis};
use ising_engine::solver::UltimateSolver;
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

/// Exact Gram Matrix Representation of the Quadratic Hamiltonian:
/// Phi(lambda) = sum_{j=0}^{n-1} x_j (x_j - 1)
/// where x = x0 + sum_i lambda_i * v_i
pub struct GramHamiltonian {
    pub n: usize,
    pub r: usize,
    pub g_matrix: Vec<Vec<i64>>, // G_{i,k} = <v_i, v_k>
    pub c_vector: Vec<i64>,      // c_i = sum_j (2 x_{0,j} - 1) v_{i,j}
    pub c0: i64,                 // c0 = sum_j x_{0,j} (x_{0,j} - 1)
}

impl GramHamiltonian {
    pub fn new(x0: &[i64], kernel: &[Vec<i64>]) -> Self {
        let n = x0.len();
        let r = kernel.len();

        let mut g_matrix = vec![vec![0i64; r]; r];
        for i in 0..r {
            for k in 0..=i {
                let mut dot = 0i64;
                for j in 0..n {
                    dot += kernel[i][j] * kernel[k][j];
                }
                g_matrix[i][k] = dot;
                g_matrix[k][i] = dot;
            }
        }

        let mut c_vector = vec![0i64; r];
        for i in 0..r {
            let mut sum = 0i64;
            for j in 0..n {
                sum += (2 * x0[j] - 1) * kernel[i][j];
            }
            c_vector[i] = sum;
        }

        let mut c0 = 0i64;
        for j in 0..n {
            c0 += x0[j] * (x0[j] - 1);
        }

        Self {
            n,
            r,
            g_matrix,
            c_vector,
            c0,
        }
    }
}

/// Ultra-Fast Gram Integer State with O(1) Move Evaluation
#[derive(Clone)]
pub struct GramState {
    pub lambda: Vec<i64>,
    pub gradient: Vec<i64>, // g_i = 2 [G lambda]_i + c_i
    pub phi: i64,           // Phi = lambda^T G lambda + c^T lambda + C0
    pub x: Vec<i64>,
    pub out_of_bounds: usize,
}

impl GramState {
    pub fn from_lambda(
        ham: &GramHamiltonian,
        x0: &[i64],
        kernel: &[Vec<i64>],
        lambda: Vec<i64>,
    ) -> Self {
        let r = ham.r;
        let n = ham.n;

        let mut gradient = ham.c_vector.clone();
        for i in 0..r {
            if lambda[i] != 0 {
                let li = lambda[i];
                for k in 0..r {
                    gradient[k] += 2 * li * ham.g_matrix[k][i];
                }
            }
        }

        let mut phi = ham.c0;
        for i in 0..r {
            phi += ham.c_vector[i] * lambda[i];
            for k in 0..r {
                phi += ham.g_matrix[i][k] * lambda[i] * lambda[k];
            }
        }

        let mut x = x0.to_vec();
        for i in 0..r {
            if lambda[i] != 0 {
                let li = lambda[i];
                let v = &kernel[i];
                for j in 0..n {
                    x[j] += li * v[j];
                }
            }
        }

        let mut out_of_bounds = 0usize;
        for &val in &x {
            if val != 0 && val != 1 {
                out_of_bounds += 1;
            }
        }

        Self {
            lambda,
            gradient,
            phi,
            x,
            out_of_bounds,
        }
    }

    #[inline(always)]
    pub fn is_solved(&self) -> bool {
        self.out_of_bounds == 0 && self.phi == 0 && self.x.iter().all(|&v| v == 0 || v == 1)
    }

    /// O(1) Delta Evaluation: Delta Phi = delta * g_i + delta^2 * G_{i,i}
    #[inline(always)]
    pub fn eval_delta(&self, ham: &GramHamiltonian, i: usize, delta: i64) -> i64 {
        delta * self.gradient[i] + delta * delta * ham.g_matrix[i][i]
    }

    /// O(r) Move Application
    #[inline(always)]
    pub fn apply_move(&mut self, ham: &GramHamiltonian, kernel: &[Vec<i64>], i: usize, delta: i64) {
        let d_phi = self.eval_delta(ham, i, delta);
        self.phi += d_phi;
        self.lambda[i] += delta;

        // Update gradient: g_k <- g_k + 2 * delta * G_{k, i}
        let col = &ham.g_matrix[i];
        for k in 0..ham.r {
            self.gradient[k] += 2 * delta * col[k];
        }

        // Update x vector and out_of_bounds count
        let v = &kernel[i];
        let mut new_oob = 0usize;
        for j in 0..ham.n {
            self.x[j] += delta * v[j];
            if self.x[j] != 0 && self.x[j] != 1 {
                new_oob += 1;
            }
        }
        self.out_of_bounds = new_oob;
    }

    /// Quenches to the nearest integer local minimum using O(1) evaluations
    pub fn quench(&mut self, ham: &GramHamiltonian, kernel: &[Vec<i64>], max_steps: usize) {
        for _ in 0..max_steps {
            let mut best_i = None;
            let mut best_delta = 0i64;
            let mut best_dphi = 0i64;

            for i in 0..ham.r {
                let d_plus = self.eval_delta(ham, i, 1);
                if d_plus < best_dphi {
                    best_dphi = d_plus;
                    best_i = Some(i);
                    best_delta = 1;
                }
                let d_minus = self.eval_delta(ham, i, -1);
                if d_minus < best_dphi {
                    best_dphi = d_minus;
                    best_i = Some(i);
                    best_delta = -1;
                }
            }

            if let Some(i) = best_i {
                self.apply_move(ham, kernel, i, best_delta);
                if self.is_solved() {
                    break;
                }
            } else {
                break; // Local minimum reached
            }
        }
    }
}

#[inline(always)]
pub fn verify_solution(inst: &MarketSplitInstance, x: &[i64]) -> bool {
    if x.len() != inst.num_vars || x.iter().any(|&v| v != 0 && v != 1) {
        return false;
    }
    for k in 0..inst.num_cons {
        let mut sum = 0i64;
        for j in 0..inst.num_vars {
            if x[j] == 1 {
                sum += inst.matrix[k][j];
            }
        }
        if sum != inst.rhs[k] {
            return false;
        }
    }
    true
}

fn get_active_bad_coords(x: &[i64], kernel: &[Vec<i64>]) -> Vec<usize> {
    let r = kernel.len();
    let n = x.len();
    let mut bad_indices = Vec::new();
    for j in 0..n {
        if x[j] != 0 && x[j] != 1 {
            bad_indices.push(j);
        }
    }
    let mut active = Vec::new();
    for i in 0..r {
        let v = &kernel[i];
        if bad_indices.iter().any(|&j| v[j] != 0) {
            active.push(i);
        }
    }
    if active.is_empty() {
        (0..r).collect()
    } else {
        active
    }
}

fn print_x_histogram(x: &[i64]) {
    let mut counts = std::collections::BTreeMap::new();
    for &val in x {
        *counts.entry(val).or_insert(0) += 1;
    }
    print!("       [Histogram: ");
    for (val, count) in counts {
        print!("{}x({}) ", count, val);
    }
    println!("]");
}

pub fn solve_hybrid(
    inst: &MarketSplitInstance,
    x0: &[i64],
    kernel: &[Vec<i64>],
    timeout: Option<Duration>,
) -> Option<Vec<i8>> {
    let t0 = Instant::now();
    let r = kernel.len();
    let n = x0.len();
    let ham = Arc::new(GramHamiltonian::new(x0, kernel));
    let solved_flag = Arc::new(AtomicBool::new(false));

    // Evaluate start points
    let raw_lambda = vec![0i64; r];
    let cvp_lambda = project_target_onto_kernel(x0, kernel);

    let target: Vec<f64> = x0.iter().map(|&x| 0.5 - (x as f64)).collect();
    let mut lattice = LatticeBasis::new(kernel.to_vec()).unwrap();
    let _ = lattice.lll(0.99);
    let babai_lambda = lattice.babai_nearest_plane(&target);

    let mut raw_state = GramState::from_lambda(&ham, x0, kernel, raw_lambda);
    raw_state.quench(&ham, kernel, 500);

    let mut cvp_state = GramState::from_lambda(&ham, x0, kernel, cvp_lambda);
    cvp_state.quench(&ham, kernel, 500);

    let mut babai_state = GramState::from_lambda(&ham, x0, kernel, babai_lambda);
    babai_state.quench(&ham, kernel, 500);

    println!(
        "   [INIT] Raw: OOB {}/{} (Phi={}) | CVP: OOB {}/{} (Phi={}) | Babai: OOB {}/{} (Phi={})",
        raw_state.out_of_bounds,
        n,
        raw_state.phi,
        cvp_state.out_of_bounds,
        n,
        cvp_state.phi,
        babai_state.out_of_bounds,
        n,
        babai_state.phi,
    );

    if raw_state.is_solved() && verify_solution(inst, &raw_state.x) {
        return Some(raw_state.x.iter().map(|&v| v as i8).collect());
    }
    if cvp_state.is_solved() && verify_solution(inst, &cvp_state.x) {
        return Some(cvp_state.x.iter().map(|&v| v as i8).collect());
    }
    if babai_state.is_solved() && verify_solution(inst, &babai_state.x) {
        return Some(babai_state.x.iter().map(|&v| v as i8).collect());
    }

    let mut initial_incumbent = raw_state;
    if cvp_state.out_of_bounds < initial_incumbent.out_of_bounds
        || (cvp_state.out_of_bounds == initial_incumbent.out_of_bounds
            && cvp_state.phi < initial_incumbent.phi)
    {
        initial_incumbent = cvp_state;
    }
    if babai_state.out_of_bounds < initial_incumbent.out_of_bounds
        || (babai_state.out_of_bounds == initial_incumbent.out_of_bounds
            && babai_state.phi < initial_incumbent.phi)
    {
        initial_incumbent = babai_state;
    }

    let shared_incumbent = Arc::new(Mutex::new(initial_incumbent));
    let num_threads = rayon::current_num_threads();

    (0..num_threads).into_par_iter().find_map_any(|tid| {
        let mut rng = ChaCha8Rng::seed_from_u64(20260925_500000u64 + (tid as u64) * 274177);
        let ham_ref = &ham;

        let is_timeout = |start: Instant| -> bool {
            if let Some(limit) = timeout {
                start.elapsed() >= limit
            } else {
                false
            }
        };

        match tid % 4 {
            0 => {
                // ARM 0: Agile SIMD UltimateSolver QUBO Trust-Region Hunter
                // Solves successive focused trust regions around the current incumbent
                let mut round = 0usize;
                while !is_timeout(t0) && !solved_flag.load(Ordering::Relaxed) {
                    round += 1;
                    let current_center = {
                        let inc = shared_incumbent.lock().unwrap();
                        inc.lambda.clone()
                    };

                    // Trust region with 2 or 3 bits per coordinate (range: center - 2..1 or center - 4..3)
                    let bits = if round.is_multiple_of(2) { 2 } else { 3 };
                    let mapping = build_kernel_qubo(x0, kernel, &current_center, bits);

                    // Focused subspace clamping: clamp all coordinates that do not touch bad variables
                    let inc_x = {
                        let inc = shared_incumbent.lock().unwrap();
                        inc.x.clone()
                    };
                    let active = get_active_bad_coords(&inc_x, kernel);
                    let mut clamped = Vec::new();
                    // When not in full-exploration round, clamp inactive coordinates
                    if !round.is_multiple_of(5) && active.len() < r {
                        for i in 0..r {
                            if !active.contains(&i) {
                                for b in 0..bits {
                                    let p = i * bits + b;
                                    let val = if b == bits - 1 { 1i8 } else { 0i8 };
                                    clamped.push((p, val));
                                }
                            }
                        }
                    }

                    // Agile UltimateSolver: 20 sweeps, 10 exchanges with 2-opt escape
                    let solver = UltimateSolver::new(5.0, 0.05, 20, 10, Some(rng.gen::<u64>()))
                        .with_2opt(true);

                    let spins = solver.solve(&mapping.model, &clamped);
                    let (cand_lambda, _) = mapping.decode(x0, kernel, &spins);

                    let mut cand_state = GramState::from_lambda(ham_ref, x0, kernel, cand_lambda);
                    cand_state.quench(ham_ref, kernel, 300);

                    if cand_state.is_solved() && verify_solution(inst, &cand_state.x) {
                        solved_flag.store(true, Ordering::SeqCst);
                        println!("[ARM 0 UltimateSolver] internally feasible candidate; checker pending");
                        return Some(cand_state.x.iter().map(|&v| v as i8).collect());
                    }

                    let mut inc = shared_incumbent.lock().unwrap();
                    if cand_state.out_of_bounds < inc.out_of_bounds
                        || (cand_state.out_of_bounds == inc.out_of_bounds && cand_state.phi < inc.phi)
                    {
                        println!(
                            "   🔥 [ARM 0 UltimateSolver] New Incumbent: OOB {}/{} | Phi {} ({:.2}s, Round {})",
                            cand_state.out_of_bounds, n, cand_state.phi, t0.elapsed().as_secs_f64(), round
                        );
                        *inc = cand_state;
                    } else if round.is_multiple_of(5) {
                        println!(
                            "   [ARM 0 UltimateSolver] Round {} | Best OOB {}/{} | Latest cand OOB {}/{} ({:.2}s)",
                            round, inc.out_of_bounds, n, cand_state.out_of_bounds, n, t0.elapsed().as_secs_f64()
                        );
                    }
                }
            }
            1 => {
                // ARM 1: Ultra-Fast Integer Tabu Search with O(1) Gradient Descent
                let mut tabu = vec![0usize; r];
                let mut step = 0usize;
                let mut no_improve = 0usize;
                let mut state = {
                    let inc = shared_incumbent.lock().unwrap();
                    inc.clone()
                };
                let mut best_state = state.clone();

                while !is_timeout(t0) && !solved_flag.load(Ordering::Relaxed) {
                    step += 1;

                    // Periodic sync with shared incumbent every 200k steps (~4ms)
                    if step.is_multiple_of(200_000) {
                        let inc = shared_incumbent.lock().unwrap();
                        if inc.out_of_bounds < best_state.out_of_bounds || (inc.out_of_bounds == best_state.out_of_bounds && inc.phi < best_state.phi) {
                            state = inc.clone();
                            best_state = inc.clone();
                        }
                    }

                    // Best non-tabu coordinate move
                    let mut best_i = None;
                    let mut best_delta = 0i64;
                    let mut best_dphi = i64::MAX;
                    let tenure = rng.gen_range(5..=25);

                    for i in 0..r {
                        let is_allowed = tabu[i] < step;
                        for &delta in &[-1i64, 1i64] {
                            let dphi = state.eval_delta(ham_ref, i, delta);
                            let cand_phi = state.phi + dphi;
                            let aspiration = cand_phi < best_state.phi;

                            if (is_allowed || aspiration) && dphi < best_dphi {
                                best_dphi = dphi;
                                best_i = Some(i);
                                best_delta = delta;
                            }
                        }
                    }

                    if let Some(i) = best_i {
                        state.apply_move(ham_ref, kernel, i, best_delta);
                        tabu[i] = step + tenure;

                        if state.is_solved() && verify_solution(inst, &state.x) {
                            solved_flag.store(true, Ordering::SeqCst);
                            println!("[ARM 1 Tabu] internally feasible candidate; checker pending");
                            return Some(state.x.iter().map(|&v| v as i8).collect());
                        }

                        if state.out_of_bounds < best_state.out_of_bounds || (state.out_of_bounds == best_state.out_of_bounds && state.phi < best_state.phi) {
                            best_state = state.clone();
                            no_improve = 0;

                            let mut inc = shared_incumbent.lock().unwrap();
                            if state.out_of_bounds < inc.out_of_bounds
                                || (state.out_of_bounds == inc.out_of_bounds && state.phi < inc.phi)
                            {
                                println!(
                                    "   🔥 [ARM 1 Tabu]           New Incumbent: OOB {}/{} | Phi {} ({:.2}s)",
                                    state.out_of_bounds, n, state.phi, t0.elapsed().as_secs_f64()
                                );
                                print_x_histogram(&state.x);
                                *inc = state.clone();
                            }
                        } else {
                            no_improve += 1;
                            if no_improve > 100_000 {
                                state = best_state.clone();
                                let active = get_active_bad_coords(&state.x, kernel);
                                let kick_i = active[rng.gen_range(0..active.len())];
                                let kick_d = if rng.gen_bool(0.5) { 1 } else { -1 };
                                state.apply_move(ham_ref, kernel, kick_i, kick_d);
                                no_improve = 0;
                            }
                        }
                    } else {
                        // Targeted shake when trapped
                        let active = get_active_bad_coords(&state.x, kernel);
                        let kick_i = active[rng.gen_range(0..active.len())];
                        let kick_d = if rng.gen_bool(0.5) { 1 } else { -1 };
                        state.apply_move(ham_ref, kernel, kick_i, kick_d);
                    }
                }
            }
            2 => {
                // ARM 2: Targeted Variable Neighborhood Search (VNS with Bad-Coordinate Focus)
                let mut state = {
                    let inc = shared_incumbent.lock().unwrap();
                    inc.clone()
                };

                let mut k = 1;
                while !is_timeout(t0) && !solved_flag.load(Ordering::Relaxed) {
                    // Shake: apply k targeted perturbations on coordinates that touch bad variables
                    let active = get_active_bad_coords(&state.x, kernel);
                    let mut cand = state.clone();
                    for _ in 0..k {
                        let i = active[rng.gen_range(0..active.len())];
                        let delta = if rng.gen_bool(0.5) { 1 } else { -1 };
                        cand.apply_move(ham_ref, kernel, i, delta);
                    }

                    // Local quench
                    cand.quench(ham_ref, kernel, 300);

                    if cand.is_solved() && verify_solution(inst, &cand.x) {
                        solved_flag.store(true, Ordering::SeqCst);
                        println!("[ARM 2 VNS] internally feasible candidate; checker pending");
                        return Some(cand.x.iter().map(|&v| v as i8).collect());
                    }

                    if cand.out_of_bounds < state.out_of_bounds
                        || (cand.out_of_bounds == state.out_of_bounds && cand.phi < state.phi)
                    {
                        state = cand.clone();
                        k = 1;

                        let mut inc = shared_incumbent.lock().unwrap();
                        if state.out_of_bounds < inc.out_of_bounds
                            || (state.out_of_bounds == inc.out_of_bounds && state.phi < inc.phi)
                        {
                            println!(
                                "   🔥 [ARM 2 VNS]            New Incumbent: OOB {}/{} | Phi {} ({:.2}s, k={})",
                                state.out_of_bounds, n, state.phi, t0.elapsed().as_secs_f64(), k
                            );
                            print_x_histogram(&state.x);
                            *inc = state.clone();
                        }
                    } else {
                        k = (k % 8) + 1; // Expand neighborhood up to 8 steps
                    }
                }
            }
            _ => {
                // ARM 3: Simulated Annealing on Integer Hamiltonian with Temperature Decay
                let mut state = {
                    let inc = shared_incumbent.lock().unwrap();
                    inc.clone()
                };
                let mut temp = 50.0f64;

                while !is_timeout(t0) && !solved_flag.load(Ordering::Relaxed) {
                    temp *= 0.99998;
                    if temp < 0.1 {
                        temp = 40.0;
                        let inc = shared_incumbent.lock().unwrap();
                        state = inc.clone();
                    }

                    let i = rng.gen_range(0..r);
                    let delta = if rng.gen_bool(0.5) { 1i64 } else { -1i64 };
                    let dphi = state.eval_delta(ham_ref, i, delta);

                    if dphi <= 0 || rng.gen_bool((-(dphi as f64) / temp).exp().clamp(0.0, 1.0)) {
                        state.apply_move(ham_ref, kernel, i, delta);

                        if state.is_solved() && verify_solution(inst, &state.x) {
                            solved_flag.store(true, Ordering::SeqCst);
                            println!("[ARM 3 Anneal] internally feasible candidate; checker pending");
                            return Some(state.x.iter().map(|&v| v as i8).collect());
                        }

                        let mut inc = shared_incumbent.lock().unwrap();
                        if state.out_of_bounds < inc.out_of_bounds
                            || (state.out_of_bounds == inc.out_of_bounds && state.phi < inc.phi)
                        {
                            println!(
                                "   🔥 [ARM 3 Anneal]         New Incumbent: OOB {}/{} | Phi {} ({:.2}s)",
                                state.out_of_bounds, n, state.phi, t0.elapsed().as_secs_f64()
                            );
                            *inc = state.clone();
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
    println!("     QOBLIB 2026: LATTICE-ULTIMATE HYBRID SOLVER                                          ");
    println!("==========================================================================================");
    println!(
        "Architecture: Aardal-Hurkens-Lenstra LLL + SIMD UltimateSolver + Gram Integer Quench"
    );
    println!("Invariant under exact arithmetic: A x = b for verified x0 and null vectors");
    println!("Objective: Exact Hamiltonian Phi(lambda) = sum_j x_j(x_j - 1) == 0 iff binary");
    println!(
        "Threads: {} Rayon worker threads",
        rayon::current_num_threads()
    );
    println!("------------------------------------------------------------------------------------------\n");

    let args: Vec<String> = std::env::args().collect();
    let mut single_instance: Option<PathBuf> = None;
    let mut timeout_secs: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--instance" && i + 1 < args.len() {
            single_instance = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else if args[i] == "--timeout" && i + 1 < args.len() {
            let t = args[i + 1].parse().unwrap_or(0);
            if t > 0 {
                timeout_secs = Some(t);
            }
            i += 2;
        } else {
            i += 1;
        }
    }

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
                    let solved = solve_hybrid(&inst, x0, &res.kernel_basis, budget);

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
                                println!("Verified hybrid certificate: {}", file.display());
                            }
                            marketsplit_certificate::CertificateStatus::Rejected(file) => {
                                eprintln!(
                                    "Hybrid candidate rejected and retained: {}",
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
