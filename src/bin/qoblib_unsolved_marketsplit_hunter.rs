//! QOBLIB 2026 Problem Class 01: Market Split experimental search.
//!
//! Tests search on 41 known-feasible instances in QOBLIB 01-marketsplit
//! (Cornuéjols & Dawande 1998; Nature Computational Science 2026 / ZIB-AOPT).
//!
//! Features:
//! - Adaptive Constraint Weighting (Breakout Method / Dynamic Local Search)
//! - Fine-grained 2-opt swap neighborhood (O(m) delta updates preserving bit balance)
//! - Elite Pool with Uniform Recombination and Variable Neighborhood Perturbations
//! - Multi-armed parallel search across Rayon worker threads
//! - Independent verification via official ZIB verifier `check_marketsplit`

#![allow(clippy::needless_range_loop)]
#![allow(clippy::inconsistent_digit_grouping)]

#[path = "common/marketsplit_certificate.rs"]
mod marketsplit_certificate;

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

#[derive(Clone)]
pub struct MSState {
    pub n: usize,
    pub m: usize,
    pub sol: Vec<i8>,
    pub residuals: Vec<i64>,
    pub energy: i64, // Exact sum_{k=0..m-1} r_k^2
}

impl MSState {
    pub fn new(inst: &MarketSplitInstance, sol: Vec<i8>) -> Self {
        let n = inst.num_vars;
        let m = inst.num_cons;
        let mut residuals = Vec::with_capacity(m);
        let mut energy = 0i64;

        for k in 0..m {
            let mut sum = 0i64;
            for j in 0..n {
                if sol[j] == 1 {
                    sum += inst.matrix[k][j];
                }
            }
            let r = sum - inst.rhs[k];
            residuals.push(r);
            energy += r * r;
        }

        Self {
            n,
            m,
            sol,
            residuals,
            energy,
        }
    }

    #[inline(always)]
    pub fn delta_flip_1_weighted(&self, matrix: &[Vec<i64>], weights: &[i64], p: usize) -> i64 {
        let dx_p = 1 - 2 * (self.sol[p] as i64);
        let mut delta = 0i64;
        for k in 0..self.m {
            let dr = dx_p * matrix[k][p];
            delta += weights[k] * (2 * self.residuals[k] * dr + dr * dr);
        }
        delta
    }

    #[inline(always)]
    pub fn apply_flip_1(&mut self, matrix: &[Vec<i64>], p: usize) {
        let dx_p = 1 - 2 * (self.sol[p] as i64);
        self.sol[p] ^= 1;
        let mut true_e = 0i64;
        for k in 0..self.m {
            self.residuals[k] += dx_p * matrix[k][p];
            true_e += self.residuals[k] * self.residuals[k];
        }
        self.energy = true_e;
    }

    #[inline(always)]
    pub fn delta_flip_2_weighted(
        &self,
        matrix: &[Vec<i64>],
        weights: &[i64],
        p: usize,
        q: usize,
    ) -> i64 {
        let dx_p = 1 - 2 * (self.sol[p] as i64);
        let dx_q = 1 - 2 * (self.sol[q] as i64);
        let mut delta = 0i64;
        for k in 0..self.m {
            let dr = dx_p * matrix[k][p] + dx_q * matrix[k][q];
            delta += weights[k] * (2 * self.residuals[k] * dr + dr * dr);
        }
        delta
    }

    #[inline(always)]
    pub fn apply_flip_2(&mut self, matrix: &[Vec<i64>], p: usize, q: usize) {
        let dx_p = 1 - 2 * (self.sol[p] as i64);
        let dx_q = 1 - 2 * (self.sol[q] as i64);
        self.sol[p] ^= 1;
        self.sol[q] ^= 1;
        let mut true_e = 0i64;
        for k in 0..self.m {
            self.residuals[k] += dx_p * matrix[k][p] + dx_q * matrix[k][q];
            true_e += self.residuals[k] * self.residuals[k];
        }
        self.energy = true_e;
    }
}

/// 1-opt local quench on unweighted energy.
pub fn local_search_1opt(inst: &MarketSplitInstance, state: &mut MSState) {
    let unit_weights = vec![1i64; inst.num_cons];
    loop {
        if state.energy == 0 {
            break;
        }
        let mut best_delta = 0i64;
        let mut best_p = None;

        for p in 0..state.n {
            let d = state.delta_flip_1_weighted(&inst.matrix, &unit_weights, p);
            if d < best_delta {
                best_delta = d;
                best_p = Some(p);
            }
        }

        if let Some(p) = best_p {
            state.apply_flip_1(&inst.matrix, p);
        } else {
            break;
        }
    }
}

/// 2-opt swap quench (exchanging variable with 1 and variable with 0).
pub fn local_search_swap_2opt(inst: &MarketSplitInstance, state: &mut MSState) {
    let unit_weights = vec![1i64; inst.num_cons];
    loop {
        local_search_1opt(inst, state);
        if state.energy == 0 {
            break;
        }

        let mut best_delta = 0i64;
        let mut best_move: Option<(usize, usize)> = None;

        // Partition indices by current spin value
        let ones: Vec<usize> = (0..state.n).filter(|&p| state.sol[p] == 1).collect();
        let zeros: Vec<usize> = (0..state.n).filter(|&p| state.sol[p] == 0).collect();

        'scan: for &p in &ones {
            for &q in &zeros {
                let d = state.delta_flip_2_weighted(&inst.matrix, &unit_weights, p, q);
                if d < best_delta {
                    best_delta = d;
                    best_move = Some((p, q));
                    if d <= -8 {
                        break 'scan;
                    }
                }
            }
        }

        if let Some((p, q)) = best_move {
            state.apply_flip_2(&inst.matrix, p, q);
        } else {
            break;
        }
    }
}

/// Adaptive Constraint Weighting (Breakout / DLM) search.
pub fn breakout_search(
    inst: &MarketSplitInstance,
    state: &mut MSState,
    max_steps: usize,
    rng: &mut ChaCha8Rng,
    solved_flag: &AtomicBool,
) -> bool {
    let m = inst.num_cons;
    let n = inst.num_vars;
    let mut weights = vec![1i64; m];
    let mut tabu = vec![0usize; n];
    let tenure = (n / 10).max(4);

    let mut best_seen_energy = state.energy;

    for step in 1..=max_steps {
        if state.energy == 0 || solved_flag.load(Ordering::Relaxed) {
            return state.energy == 0;
        }

        // 1. Check all 1-opt flips
        let mut best_1_delta = i64::MAX;
        let mut best_1_p = None;
        for p in 0..n {
            let is_allowed = tabu[p] < step;
            let d = state.delta_flip_1_weighted(&inst.matrix, &weights, p);
            let aspiration = state.energy + d < best_seen_energy;

            if (is_allowed || aspiration) && d < best_1_delta {
                best_1_delta = d;
                best_1_p = Some(p);
            }
        }

        // 2. Sample swap moves (one is 1, one is 0)
        let mut best_swap_delta = i64::MAX;
        let mut best_swap = None;
        let num_swap_samples = (n * 3).min(3000);

        for _ in 0..num_swap_samples {
            let p = rng.gen_range(0..n);
            let q = rng.gen_range(0..n);
            if p == q || state.sol[p] == state.sol[q] {
                continue;
            }
            let is_allowed = tabu[p] < step && tabu[q] < step;
            let d = state.delta_flip_2_weighted(&inst.matrix, &weights, p, q);
            let aspiration = state.energy + d < best_seen_energy;

            if (is_allowed || aspiration) && d < best_swap_delta {
                best_swap_delta = d;
                best_swap = Some((p, q));
            }
        }

        // Perform best improving move
        if best_1_delta < 0 && best_1_delta <= best_swap_delta {
            let p = best_1_p.unwrap();
            state.apply_flip_1(&inst.matrix, p);
            tabu[p] = step + tenure;
            if state.energy < best_seen_energy {
                best_seen_energy = state.energy;
            }
        } else if best_swap_delta < 0 {
            let (p, q) = best_swap.unwrap();
            state.apply_flip_2(&inst.matrix, p, q);
            tabu[p] = step + tenure;
            tabu[q] = step + tenure;
            if state.energy < best_seen_energy {
                best_seen_energy = state.energy;
            }
        } else {
            // LOCAL MINIMUM REACHED!
            // BREAKOUT: Increase weights of violated constraints
            for k in 0..m {
                if state.residuals[k] != 0 {
                    weights[k] += 1 + (state.residuals[k].abs() / 4).max(1);
                }
            }

            // Rescale weights if too large
            let max_w = *weights.iter().max().unwrap_or(&1);
            if max_w > 5000 {
                for w in weights.iter_mut() {
                    *w = (*w * 3 / 4).max(1);
                }
            }

            // Forced escape: take best available move even if delta >= 0
            if best_1_delta <= best_swap_delta {
                if let Some(p) = best_1_p {
                    state.apply_flip_1(&inst.matrix, p);
                    tabu[p] = step + tenure;
                }
            } else if let Some((p, q)) = best_swap {
                state.apply_flip_2(&inst.matrix, p, q);
                tabu[p] = step + tenure;
                tabu[q] = step + tenure;
            }
        }
    }

    state.energy == 0
}

/// Perturbs an elite state by flipping k random bits.
pub fn perturb_state(
    inst: &MarketSplitInstance,
    base_sol: &[i8],
    flips: usize,
    rng: &mut ChaCha8Rng,
) -> MSState {
    let mut sol = base_sol.to_vec();
    let n = sol.len();
    for _ in 0..flips {
        let p = rng.gen_range(0..n);
        sol[p] ^= 1;
    }
    let mut state = MSState::new(inst, sol);
    local_search_swap_2opt(inst, &mut state);
    state
}

/// Recombines two elite states with uniform crossover.
pub fn crossover_states(
    inst: &MarketSplitInstance,
    parent_a: &[i8],
    parent_b: &[i8],
    rng: &mut ChaCha8Rng,
) -> MSState {
    let n = parent_a.len();
    let mut child = vec![0i8; n];
    for i in 0..n {
        if parent_a[i] == parent_b[i] {
            child[i] = parent_a[i];
        } else {
            child[i] = if rng.gen_bool(0.5) { 1 } else { 0 };
        }
    }
    let mut state = MSState::new(inst, child);
    local_search_swap_2opt(inst, &mut state);
    state
}

pub struct HuntOutcome {
    pub solved: Option<Vec<i8>>,
    pub best_energy: i64,
    pub best_sol: Vec<i8>,
    pub violated_constraints: usize,
}

/// Solves a single Market Split instance using parallel worker threads.
pub fn hunt_instance(inst: &MarketSplitInstance, timeout_per_instance: Duration) -> HuntOutcome {
    let t0 = Instant::now();

    // Phase 1: Lattice Basis Reduction (Aardal-Hurkens-Lenstra / LLL) Pre-solve
    let lll_seed = if let Ok(aardal_res) = ising_engine::core::lattice::aardal_diophantine_reduction(
        &inst.matrix,
        &inst.rhs,
        10_000,
        10,
    ) {
        if let Some(exact_sol) = aardal_res.exact_boolean_solution {
            return HuntOutcome {
                solved: Some(exact_sol.clone()),
                best_energy: 0,
                best_sol: exact_sol,
                violated_constraints: 0,
            };
        }
        if let Some(part) = aardal_res.particular_solution {
            let clamped: Vec<i8> = part.iter().map(|&v| if v > 0 { 1 } else { 0 }).collect();
            let mut s = MSState::new(inst, clamped);
            local_search_swap_2opt(inst, &mut s);
            if s.energy == 0 {
                return HuntOutcome {
                    solved: Some(s.sol.clone()),
                    best_energy: 0,
                    best_sol: s.sol,
                    violated_constraints: 0,
                };
            }
            Some((s.energy, s.sol))
        } else {
            None
        }
    } else {
        None
    };

    let solved_flag = Arc::new(AtomicBool::new(false));
    let initial_best = lll_seed
        .clone()
        .unwrap_or((i64::MAX, vec![0i8; inst.num_vars]));
    let global_best = Arc::new(Mutex::new(initial_best));
    let num_threads = rayon::current_num_threads();

    let solved_sol = (0..num_threads).into_par_iter().find_map_any(|tid| {
        let mut rng = ChaCha8Rng::seed_from_u64(20260925_100000u64 + (tid as u64) * 65537);

        // Elite pool of (energy, solution)
        let mut elite_pool: Vec<(i64, Vec<i8>)> = Vec::with_capacity(8);
        if let Some(ref seed) = lll_seed {
            elite_pool.push(seed.clone());
        }

        while t0.elapsed() < timeout_per_instance && !solved_flag.load(Ordering::Relaxed) {
            let mut state = if elite_pool.len() >= 2 && rng.gen_bool(0.35) {
                // Recombine two elite solutions
                let idx_a = rng.gen_range(0..elite_pool.len());
                let idx_b = rng.gen_range(0..elite_pool.len());
                crossover_states(inst, &elite_pool[idx_a].1, &elite_pool[idx_b].1, &mut rng)
            } else if !elite_pool.is_empty() && rng.gen_bool(0.50) {
                // Perturb an elite solution
                let idx = rng.gen_range(0..elite_pool.len());
                let flips = rng.gen_range(2..=5);
                perturb_state(inst, &elite_pool[idx].1, flips, &mut rng)
            } else {
                // Random start
                let mut sol = vec![0i8; inst.num_vars];
                for val in sol.iter_mut() {
                    *val = if rng.gen_bool(0.5) { 1 } else { 0 };
                }
                let mut s = MSState::new(inst, sol);
                local_search_swap_2opt(inst, &mut s);
                s
            };

            // Update elite pool and global best
            {
                let mut lock = global_best.lock().unwrap();
                if state.energy < lock.0 {
                    lock.0 = state.energy;
                    lock.1 = state.sol.clone();
                }
            }
            if elite_pool.len() < 8 {
                elite_pool.push((state.energy, state.sol.clone()));
                elite_pool.sort_by_key(|e| e.0);
            } else if state.energy < elite_pool.last().unwrap().0 {
                elite_pool.pop();
                elite_pool.push((state.energy, state.sol.clone()));
                elite_pool.sort_by_key(|e| e.0);
            }

            if state.energy == 0 {
                solved_flag.store(true, Ordering::SeqCst);
                return Some(state.sol);
            }

            // Run Adaptive Constraint Weighting (Breakout) search
            let max_breakout_steps = (inst.num_vars * 15).max(1000);
            let is_solved =
                breakout_search(inst, &mut state, max_breakout_steps, &mut rng, &solved_flag);

            {
                let mut lock = global_best.lock().unwrap();
                if state.energy < lock.0 {
                    lock.0 = state.energy;
                    lock.1 = state.sol.clone();
                }
            }

            if state.energy < elite_pool.last().map(|e| e.0).unwrap_or(i64::MAX) {
                if elite_pool.len() < 8 {
                    elite_pool.push((state.energy, state.sol.clone()));
                } else {
                    elite_pool.pop();
                    elite_pool.push((state.energy, state.sol.clone()));
                }
                elite_pool.sort_by_key(|e| e.0);
            }

            if is_solved || state.energy == 0 {
                solved_flag.store(true, Ordering::SeqCst);
                return Some(state.sol);
            }
        }
        None
    });

    let (best_energy, best_sol) = {
        let lock = global_best.lock().unwrap();
        (lock.0, lock.1.clone())
    };

    let mut viols = 0usize;
    for k in 0..inst.num_cons {
        let mut sum = 0i64;
        for j in 0..inst.num_vars {
            if best_sol[j] == 1 {
                sum += inst.matrix[k][j];
            }
        }
        if sum != inst.rhs[k] {
            viols += 1;
        }
    }

    HuntOutcome {
        solved: solved_sol,
        best_energy,
        best_sol,
        violated_constraints: viols,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!(
        "     QOBLIB 2026: EXPERIMENTAL SEARCH ON 41 MARKET SPLIT INSTANCES                       "
    );
    println!("==========================================================================================");
    println!(
        "Problem Class: 01-marketsplit (Cornuéjols & Dawande 1998, Nature Comp. Science 2026)"
    );
    println!("Status: known feasible; official input comments publish witness assignments");
    println!(
        "Algorithm: Adaptive Constraint Weighting (Breakout / DLM) + Elite Recombination & Swaps"
    );
    println!("Verification: Independent check with official ZIB verifier `check_marketsplit`");
    println!(
        "Threads: {} Rayon worker threads",
        rayon::current_num_threads()
    );
    println!("------------------------------------------------------------------------------------------\n");

    let args: Vec<String> = std::env::args().collect();
    let mut single_instance: Option<PathBuf> = None;
    let mut timeout_secs = 30u64;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--instance" && i + 1 < args.len() {
            single_instance = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else if args[i] == "--timeout" && i + 1 < args.len() {
            timeout_secs = args[i + 1].parse().unwrap_or(30);
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
        "Targeting {} benchmark instance(s). Timeout per instance: {}s.",
        entries.len(),
        timeout_secs
    );

    let mut total_solved = 0usize;
    let timeout_per_inst = Duration::from_secs(timeout_secs);

    println!(
        "\n{:<16} | {:>4} | {:>4} | {:>14} | {:>14} | {:>9}",
        "Instance", "Cons", "Vars", "Search Result", "Official ZIB", "Time"
    );
    println!(
        "{:-<16}-+-{:-<4}-+-{:-<4}-+-{:-<14}-+-{:-<14}-+-{:-<9}",
        "", "", "", "", "", ""
    );

    for path in &entries {
        let inst = parse_marketsplit_dat(path)?;
        let t0 = Instant::now();

        let outcome = hunt_instance(&inst, timeout_per_inst);
        let elapsed = t0.elapsed();

        if let Some(ref sol) = outcome.solved {
            let checker = Path::new("target/release/check_marketsplit");
            let rejected_dir = Path::new("benchmarks/qoblib/marketsplit/rejected_candidates");
            let certificate = marketsplit_certificate::certify(
                path,
                &inst.name,
                sol,
                checker,
                sol_dir,
                Path::new("benchmarks/qoblib/marketsplit/unsolved_candidates"),
                rejected_dir,
            )?;
            let (search_result, checker_status) = match certificate {
                marketsplit_certificate::CertificateStatus::Verified(ref file) => {
                    println!("    Verified certificate: {}", file.display());
                    total_solved += 1;
                    ("VERIFIED", "VALID (0 viol)")
                }
                marketsplit_certificate::CertificateStatus::Rejected(ref file) => {
                    println!("    Rejected candidate retained: {}", file.display());
                    ("REJECTED", "INVALID")
                }
            };
            println!(
                "🔥 {:<14} | {:>4} | {:>4} | {:>14} | {:>14} | {:>8.2?}",
                inst.name, inst.num_cons, inst.num_vars, search_result, checker_status, elapsed
            );
        } else {
            let info = format!(
                "TIMEOUT (E:{}, V:{}/{})",
                outcome.best_energy, outcome.violated_constraints, inst.num_cons
            );
            println!(
                "   {:<14} | {:>4} | {:>4} | {:>22} | {:>14} | {:>8.2?}",
                inst.name, inst.num_cons, inst.num_vars, info, "—", elapsed
            );
        }
    }

    println!("\n==========================================================================================");
    println!(
        "Campaign complete. Checker-verified candidates: {} / {} instances.",
        total_solved,
        entries.len()
    );
    println!("==========================================================================================");

    Ok(())
}
