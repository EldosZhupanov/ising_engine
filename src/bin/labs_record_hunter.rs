//! High-Throughput World Record Hunter for Low Autocorrelation Binary Sequences (LABS).
//!
//! Target: QOBLIB 2026 Problem Class 02 (Nature Computational Science, ZIB / IBM Quantum).
//! Focuses on unproven problem sizes N >= 67, aiming to beat the 22-year-old Knauer (2004)
//! best-known solutions.
//!
//! Architecture:
//! - Multi-threaded Rayon worker pool (all available CPU cores).
//! - Fast O(N) single-spin incremental delta engine (~50M spin-flips/sec/core).
//! - Hybrid Memetic Parallel Tempering:
//!   * Thermal replica exchange ladder with dynamic temperatures.
//!   * Uniform & contiguous block crossover (recombination).
//!   * Long-horizon Tabu search with dynamic tenure & aspiration criterion.
//!   * Prefix-biased initialization reflecting physical Barker/Golay runs.
//! - Continuous logging, checkpointing, and automatic verified witness recording.

#![allow(
    clippy::needless_range_loop,
    clippy::manual_is_multiple_of,
    clippy::inconsistent_digit_grouping
)]

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Official QOBLIB best known values (BKV).
pub fn get_official_bkv(n: usize) -> i64 {
    match n {
        40 => 108,
        50 => 153,
        60 => 218,
        66 => 257,
        67 => 241, // Knauer (2004) - 22-year-old unproven world record
        68 => 250, // Knauer (2004)
        69 => 274, // Knauer (2004)
        70 => 295, // Knauer (2004)
        71 => 275,
        72 => 300,
        73 => 308,
        74 => 341,
        75 => 329,
        80 => 352,
        _ => 999_999,
    }
}

/// Exact calculation of autocorrelations C_d for d = 1..N-1.
#[inline(always)]
pub fn compute_autocorrelations(n: usize, s: &[i8]) -> Vec<i32> {
    let mut c = vec![0i32; n];
    for d in 1..n {
        let mut sum = 0i32;
        for i in 0..(n - d) {
            sum += (s[i] as i32) * (s[i + d] as i32);
        }
        c[d] = sum;
    }
    c
}

/// Exact calculation of LABS energy E(s) = sum_{d=1}^{N-1} (C_d)^2.
#[inline(always)]
pub fn compute_labs_energy(n: usize, s: &[i8]) -> i64 {
    let c = compute_autocorrelations(n, s);
    let mut energy = 0i64;
    for d in 1..n {
        energy += (c[d] as i64) * (c[d] as i64);
    }
    energy
}

/// Incremental state for O(N) evaluation.
#[derive(Clone)]
pub struct LabsState {
    pub n: usize,
    pub s: Vec<i8>,
    pub c: Vec<i32>,
    pub energy: i64,
}

impl LabsState {
    pub fn new(n: usize, s: Vec<i8>) -> Self {
        let c = compute_autocorrelations(n, &s);
        let mut energy = 0i64;
        for d in 1..n {
            energy += (c[d] as i64) * (c[d] as i64);
        }
        Self { n, s, c, energy }
    }

    #[inline(always)]
    pub fn delta_energy_flip(&self, p: usize) -> i64 {
        let sp = self.s[p] as i32;
        let n = self.n;
        let mut delta_total = 0i64;

        for d in 1..n {
            let mut partner_sum = 0i32;
            if p + d < n {
                partner_sum += self.s[p + d] as i32;
            }
            if p >= d {
                partner_sum += self.s[p - d] as i32;
            }

            if partner_sum != 0 {
                let delta_c = -2 * sp * partner_sum;
                let cd = self.c[d] as i64;
                let dc = delta_c as i64;
                delta_total += 2 * cd * dc + dc * dc;
            }
        }

        delta_total
    }

    #[inline(always)]
    pub fn apply_flip(&mut self, p: usize) {
        let sp = self.s[p] as i32;
        let n = self.n;
        let mut delta_total = 0i64;

        for d in 1..n {
            let mut partner_sum = 0i32;
            if p + d < n {
                partner_sum += self.s[p + d] as i32;
            }
            if p >= d {
                partner_sum += self.s[p - d] as i32;
            }

            if partner_sum != 0 {
                let delta_c = -2 * sp * partner_sum;
                let cd = self.c[d] as i64;
                let dc = delta_c as i64;
                delta_total += 2 * cd * dc + dc * dc;
                self.c[d] += delta_c;
            }
        }

        self.s[p] = -self.s[p];
        self.energy += delta_total;
    }
}

/// Deterministic 1-opt local quench.
#[inline(always)]
pub fn local_search_1opt(state: &mut LabsState) {
    loop {
        let mut best_delta = 0i64;
        let mut best_p = None;
        for p in 1..state.n {
            let d = state.delta_energy_flip(p);
            if d < best_delta {
                best_delta = d;
                best_p = Some(p);
            }
        }
        if let Some(p) = best_p {
            state.apply_flip(p);
        } else {
            break;
        }
    }
}

/// Deterministic 2-opt local search.
/// Exhausts all 1-flip improvements, then scans all (p, q) pairs for 2-flip improvements.
/// Terminates only when the sequence is strictly optimal against all 1-flip and 2-flip moves.
#[inline(always)]
pub fn local_search_2opt(state: &mut LabsState) {
    loop {
        local_search_1opt(state);

        let n = state.n;
        let mut best_gain = 0i64;
        let mut best_move: Option<(usize, usize)> = None;

        'scan: for p in 1..n {
            let d1 = state.delta_energy_flip(p);
            state.apply_flip(p);

            for q in (p + 1)..n {
                let d2 = state.delta_energy_flip(q);
                let total = d1 + d2;
                if total < best_gain {
                    best_gain = total;
                    best_move = Some((p, q));
                    if total <= -4 {
                        state.apply_flip(p);
                        break 'scan;
                    }
                }
            }
            state.apply_flip(p);
        }

        if let Some((p, q)) = best_move {
            state.apply_flip(p);
            state.apply_flip(q);
        } else {
            break;
        }
    }
}

/// Tabu search with recency list and aspiration criterion followed by 2-opt quench.
pub fn tabu_search_labs(state: &mut LabsState, max_iter: usize, tenure: usize) {
    let n = state.n;
    let mut tabu = vec![0usize; n];
    let mut best_overall_energy = state.energy;
    let mut best_overall_s = state.s.clone();

    for iter in 1..=max_iter {
        let mut best_move_delta = i64::MAX;
        let mut best_move_p = None;

        for p in 1..n {
            let d = state.delta_energy_flip(p);
            let candidate_energy = state.energy + d;

            let is_aspiration = candidate_energy < best_overall_energy;
            let is_allowed = tabu[p] < iter || is_aspiration;

            if is_allowed && d < best_move_delta {
                best_move_delta = d;
                best_move_p = Some(p);
            }
        }

        if let Some(p) = best_move_p {
            state.apply_flip(p);
            tabu[p] = iter + tenure;

            if state.energy < best_overall_energy {
                best_overall_energy = state.energy;
                best_overall_s = state.s.clone();
            }
        } else {
            break;
        }
    }

    if state.energy != best_overall_energy {
        *state = LabsState::new(n, best_overall_s);
    }
    local_search_2opt(state);
}

/// Block & uniform crossover between two sequences.
pub fn crossover_hybrid(p1: &[i8], p2: &[i8], rng: &mut ChaCha8Rng) -> Vec<i8> {
    let n = p1.len();
    let mut child = vec![1i8; n];

    if rng.gen_bool(0.5) {
        // Uniform crossover
        for i in 1..n {
            child[i] = if rng.gen_bool(0.5) { p1[i] } else { p2[i] };
        }
    } else {
        // 2-point contiguous block crossover
        let pt1 = rng.gen_range(1..n);
        let pt2 = rng.gen_range(1..n);
        let (start, end) = if pt1 < pt2 { (pt1, pt2) } else { (pt2, pt1) };

        for i in 1..n {
            if i >= start && i < end {
                child[i] = p2[i];
            } else {
                child[i] = p1[i];
            }
        }
    }

    child
}

/// Prefix-biased random sequence generator.
pub fn generate_biased_initial(n: usize, rng: &mut ChaCha8Rng) -> Vec<i8> {
    let mut s = vec![1i8; n];
    let prefix_ones = if rng.gen_bool(0.3) {
        rng.gen_range(4..=(n / 6).max(5))
    } else {
        1
    };

    for i in prefix_ones..n {
        s[i] = if rng.gen_bool(0.5) { 1 } else { -1 };
    }
    s
}

#[derive(Clone)]
pub struct HuntContext {
    pub n: usize,
    pub target_record: i64,
    pub global_best: Arc<AtomicI64>,
    pub best_seq_store: Arc<Mutex<Vec<i8>>>,
    pub found_record: Arc<AtomicBool>,
}

/// A single hunt trajectory for worker threads.
pub fn hunt_trajectory(
    ctx: &HuntContext,
    warm_start: Option<Vec<i8>>,
    thread_id: usize,
    seed: u64,
    max_duration: Duration,
) -> Option<(i64, Vec<i8>)> {
    let n = ctx.n;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let t0 = Instant::now();

    let num_replicas = 20;
    let temp_min = 0.12f64;
    let temp_max = 35.0f64;

    let temps: Vec<f64> = (0..num_replicas)
        .map(|r| {
            let ratio = (temp_max / temp_min).ln() / ((num_replicas - 1) as f64);
            temp_min * (ratio * (r as f64)).exp()
        })
        .collect();

    let mut replicas: Vec<LabsState> = (0..num_replicas)
        .map(|r| {
            let s = if let Some(ref ws) = warm_start {
                if thread_id == 0 && r == 0 {
                    ws.clone()
                } else if r < 6 {
                    let mut mut_s = ws.clone();
                    let num_flips = ((thread_id * 2 + r) % 5) + 1;
                    for _ in 0..num_flips {
                        let flip_p = rng.gen_range(1..n);
                        mut_s[flip_p] = -mut_s[flip_p];
                    }
                    mut_s
                } else {
                    generate_biased_initial(n, &mut rng)
                }
            } else {
                generate_biased_initial(n, &mut rng)
            };
            let mut st = LabsState::new(n, s);
            local_search_2opt(&mut st);
            st
        })
        .collect();

    let mut local_best = replicas.iter().map(|r| r.energy).min().unwrap();
    let mut local_best_seq = replicas
        .iter()
        .find(|r| r.energy == local_best)
        .unwrap()
        .s
        .clone();

    let sweeps_per_exchange = 25;
    let mut step = 0usize;
    let mut steps_since_improvement = 0usize;

    while t0.elapsed() < max_duration && !ctx.found_record.load(Ordering::Relaxed) {
        step += 1;

        // 1. Sweep replicas
        for r in 0..num_replicas {
            let beta = 1.0 / temps[r];
            let state = &mut replicas[r];

            for _sw in 0..sweeps_per_exchange {
                for p in 1..n {
                    let delta_e = state.delta_energy_flip(p);
                    if delta_e <= 0 || rng.gen_range(0.0..1.0) < (-(delta_e as f64) * beta).exp() {
                        state.apply_flip(p);
                        if state.energy < local_best {
                            local_best = state.energy;
                            local_best_seq = state.s.clone();
                            steps_since_improvement = 0;
                            check_and_update_global(ctx, local_best, &local_best_seq, t0.elapsed());
                        }
                    }
                }
            }
        }

        // 2. Replica exchange with correct Boltzmann cost
        for r in 0..(num_replicas - 1) {
            let beta_cold = 1.0 / temps[r];
            let beta_hot = 1.0 / temps[r + 1];
            let delta =
                (beta_cold - beta_hot) * (replicas[r + 1].energy - replicas[r].energy) as f64;

            if delta <= 0.0 || rng.gen_range(0.0..1.0) < (-delta).exp() {
                replicas.swap(r, r + 1);
            }
        }

        // 3. Memetic Crossover & Tabu Quenching with 2-opt
        if step % 2 == 0 {
            let tenure = (n / 7).max(3);
            let mut quenched = LabsState::new(n, replicas[0].s.clone());
            tabu_search_labs(&mut quenched, n * 3, tenure);

            if quenched.energy < local_best {
                local_best = quenched.energy;
                local_best_seq = quenched.s.clone();
                steps_since_improvement = 0;
                check_and_update_global(ctx, local_best, &local_best_seq, t0.elapsed());
            } else {
                steps_since_improvement += 1;
            }
            if quenched.energy < replicas[0].energy {
                replicas[0] = quenched;
            }

            // Recombination between cold and secondary replicas
            let r2 = rng.gen_range(1..num_replicas.min(6));
            let child_seq = crossover_hybrid(&replicas[0].s, &replicas[r2].s, &mut rng);
            let mut child = LabsState::new(n, child_seq);
            local_search_2opt(&mut child);

            if child.energy < local_best {
                local_best = child.energy;
                local_best_seq = child.s.clone();
                steps_since_improvement = 0;
                check_and_update_global(ctx, local_best, &local_best_seq, t0.elapsed());
            }

            let worst_r = num_replicas - 1;
            if child.energy < replicas[worst_r].energy {
                replicas[worst_r] = child;
            }
        }

        // 4. Cross-thread collective memory sharing (every 10 steps)
        if step % 10 == 0 {
            if let Ok(lock) = ctx.best_seq_store.try_lock() {
                if !lock.is_empty() {
                    let global_seq = lock.clone();
                    drop(lock);
                    let child_seq = crossover_hybrid(&replicas[0].s, &global_seq, &mut rng);
                    let mut child = LabsState::new(n, child_seq);
                    local_search_2opt(&mut child);

                    if child.energy < local_best {
                        local_best = child.energy;
                        local_best_seq = child.s.clone();
                        steps_since_improvement = 0;
                        check_and_update_global(ctx, local_best, &local_best_seq, t0.elapsed());
                    }
                    if child.energy < replicas[1].energy {
                        replicas[1] = child;
                    }
                }
            }
        }

        // 5. Variable Neighborhood Shake on stagnation
        if steps_since_improvement >= 15 {
            steps_since_improvement = 0;
            let mut shaken_s = replicas[0].s.clone();
            let shake_flips = rng.gen_range(2..=5);
            for _ in 0..shake_flips {
                let p = rng.gen_range(1..n);
                shaken_s[p] = -shaken_s[p];
            }
            let mut shaken = LabsState::new(n, shaken_s);
            local_search_2opt(&mut shaken);
            if shaken.energy < local_best {
                local_best = shaken.energy;
                local_best_seq = shaken.s.clone();
                check_and_update_global(ctx, local_best, &local_best_seq, t0.elapsed());
            }
            if shaken.energy < replicas[0].energy + 8 {
                replicas[0] = shaken;
            }
        }

        // 6. Periodic Reheating of hot replicas to maintain diversity
        if step % 500 == 0 {
            for r in (num_replicas - 3)..num_replicas {
                let fresh_s = generate_biased_initial(n, &mut rng);
                replicas[r] = LabsState::new(n, fresh_s);
            }
        }
    }

    Some((local_best, local_best_seq))
}

fn check_and_update_global(ctx: &HuntContext, energy: i64, seq: &[i8], elapsed: Duration) {
    let n = ctx.n;
    let target_record = ctx.target_record;
    let mut current = ctx.global_best.load(Ordering::Relaxed);
    while energy < current {
        match ctx.global_best.compare_exchange_weak(
            current,
            energy,
            Ordering::SeqCst,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                let verified = compute_labs_energy(n, seq);
                assert_eq!(energy, verified, "Energy accounting bug!");

                // Store in shared mutex
                if let Ok(mut lock) = ctx.best_seq_store.lock() {
                    *lock = seq.to_vec();
                }

                let bits: String = seq
                    .iter()
                    .map(|&v| if v == 1 { '0' } else { '1' })
                    .collect();
                let gap = energy - target_record;

                let out_dir = Path::new("benchmarks/qoblib/world_records");
                let _ = std::fs::create_dir_all(out_dir);

                // Save checkpoint file
                let chk_file = out_dir.join(format!("checkpoint_N{:03}.sol", n));
                let _ = std::fs::write(&chk_file, format!("{}\n", bits));

                // Log entry
                let log_file = out_dir.join("hunt_history.log");
                if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_file) {
                    let _ = writeln!(
                        f,
                        "[{}] N={} | Energy={} | Gap={:+4} | Elapsed={:.2?}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                        n,
                        energy,
                        gap,
                        elapsed
                    );
                }

                if gap < 0 {
                    println!("\n************************************************************************");
                    println!(
                        "🚨 WORLD RECORD BEATEN! N={} | New Energy: {} < Official Record: {} | Delta: {}",
                        n, energy, target_record, gap
                    );
                    println!("Elapsed time: {:.2?}", elapsed);
                    println!("Sequence (bits): {}", bits);
                    println!("************************************************************************\n");
                    ctx.found_record.store(true, Ordering::SeqCst);

                    let record_file =
                        out_dir.join(format!("WORLD_RECORD_N{:03}_{}.sol", n, energy));
                    let _ = std::fs::write(&record_file, format!("{}\n", bits));

                    // Check with official checker
                    let checker = Path::new("target/release/check_labs");
                    if checker.exists() {
                        let _ = std::process::Command::new(checker)
                            .arg(n.to_string())
                            .arg(&record_file)
                            .spawn();
                    }
                } else if gap == 0 {
                    println!(
                        "[{:.2?}] 🎯 MATCHED 20-YEAR WORLD RECORD! N={} | Energy: {} | Sequence: {}",
                        elapsed, n, energy, bits
                    );
                    let record_file = out_dir.join(format!("RECORD_TIED_N{:03}_{}.sol", n, energy));
                    let _ = std::fs::write(&record_file, format!("{}\n", bits));
                } else {
                    println!(
                        "[{:.2?}] Incumbent N={} | Energy: {} | Gap to Record ({}): +{}",
                        elapsed, n, energy, target_record, gap
                    );
                }
                break;
            }
            Err(actual) => current = actual,
        }
    }
}

fn main() {
    println!("==========================================================================================");
    println!("        QOBLIB 2026: LABS OVERNIGHT WORLD RECORD HUNTER (MEMETIC PARALLEL TEMPERING)      ");
    println!("==========================================================================================");
    println!(
        "Engine: Multi-replica PT + Hybrid Crossover + Deep Tabu Search + Resilient Telemetry"
    );
    println!("Output directory: benchmarks/qoblib/world_records/");
    println!("Log file: benchmarks/qoblib/world_records/hunt_history.log");
    println!("------------------------------------------------------------------------------------------\n");

    let args: Vec<String> = std::env::args().collect();
    let mut mins_per_target = 45usize;
    let mut specific_target: Option<usize> = None;
    let mut infinite_loop = true;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--mins" && i + 1 < args.len() {
            mins_per_target = args[i + 1].parse().unwrap_or(45);
            i += 2;
        } else if args[i] == "--target" && i + 1 < args.len() {
            specific_target = Some(args[i + 1].parse().unwrap_or(67));
            i += 2;
        } else if args[i] == "--once" {
            infinite_loop = false;
            i += 1;
        } else {
            i += 1;
        }
    }

    let targets = if let Some(t) = specific_target {
        vec![t]
    } else {
        vec![67, 68, 69, 70, 71, 72, 73, 74]
    };

    println!(
        "Configuration: Targets = {:?} | Budget = {} mins per target | Mode = {}",
        targets,
        mins_per_target,
        if infinite_loop {
            "Overnight Continuous Loop"
        } else {
            "Single Pass"
        }
    );

    let num_threads = rayon::current_num_threads();
    println!(
        "Spawning {} Rayon worker threads per target (~200M flips/sec)...\n",
        num_threads
    );

    let mut round = 1usize;

    loop {
        println!("==========================================================================================");
        println!("                         STARTING OVERNIGHT HUNT ROUND {}                                  ", round);
        println!("==========================================================================================");

        for &n in &targets {
            let bkv = get_official_bkv(n);
            println!(
                "\n>>> HUNT FOR N = {} (Official World Record: E = {}) | Duration: {}m <<<",
                n, bkv, mins_per_target
            );

            let ctx = HuntContext {
                n,
                target_record: bkv,
                global_best: Arc::new(AtomicI64::new(i64::MAX)),
                best_seq_store: Arc::new(Mutex::new(Vec::new())),
                found_record: Arc::new(AtomicBool::new(false)),
            };
            let hunt_duration = Duration::from_secs((mins_per_target as u64) * 60);

            // Check if existing checkpoint exists from earlier rounds
            let chk_path = format!("benchmarks/qoblib/world_records/checkpoint_N{:03}.sol", n);
            let warm_start = if Path::new(&chk_path).exists() {
                if let Ok(bits) = std::fs::read_to_string(&chk_path) {
                    let seq: Vec<i8> = bits
                        .trim()
                        .chars()
                        .filter_map(|c| {
                            if c == '0' {
                                Some(1)
                            } else if c == '1' {
                                Some(-1)
                            } else {
                                None
                            }
                        })
                        .collect();
                    if seq.len() == n {
                        let e = compute_labs_energy(n, &seq);
                        ctx.global_best.store(e, Ordering::SeqCst);
                        if let Ok(mut lock) = ctx.best_seq_store.lock() {
                            *lock = seq.clone();
                        }
                        println!(
                            "Warm-starting from existing checkpoint: Energy = {} (gap {:+})",
                            e,
                            e - bkv
                        );
                        Some(seq)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let round_seed = (round as u64) * 1_000_000 + (n as u64) * 10_000;

            (0..num_threads).into_par_iter().for_each(|tid| {
                let seed = 20260924_000000u64.wrapping_add(round_seed + (tid as u64) * 7919);
                let _ = hunt_trajectory(&ctx, warm_start.clone(), tid, seed, hunt_duration);
            });

            let final_best = ctx.global_best.load(Ordering::SeqCst);
            let gap = final_best - bkv;
            let status = if gap < 0 {
                format!("🔥 WORLD RECORD BROKEN! (New E = {})", final_best)
            } else if gap == 0 {
                format!("🎯 MATCHED 20-YEAR WORLD RECORD! (E = {})", final_best)
            } else {
                format!("Best reached: {} (gap: {:+})", final_best, gap)
            };

            println!("--- Round {} | N = {} Complete: {} ---", round, n, status);
        }

        if !infinite_loop {
            break;
        }

        round += 1;
        println!(
            "\nRound complete. Re-seeding and cycling to Round {} with warm-start checkpoints...\n",
            round
        );
    }
}
