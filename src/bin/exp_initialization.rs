//! RC-002 — the Initialization Erasure Law.
//!
//! Design pre-registered in `research/RC002_INITIALIZATION_ERASURE.md`.
//!
//! Every recorded experiment in this platform starts from `vec![0u8; n]`
//! (`executor.rs:124`) with all replicas identical. This binary asks whether that
//! choice matters — and if it stops mattering above some temperature T*.
//!
//! Three arms at IDENTICAL total sweep budget, separating the two things an
//! initialization can supply:
//!
//!   A  metropolis(100)                     poor quality, ZERO diversity  (status quo)
//!   B  random_flip(1) → metropolis(99)     poor quality, MAX  diversity
//!   C  greedy_descent(1) → metropolis(99)  GOOD quality, zero diversity
//!
//! `random_flip_sweep` flips each (site, replica) independently with p = ½, so on
//! the all-zeros state one sweep is exactly an independent uniform draw per
//! replica. Every component here is an already-verified operator: no new code
//! enters the measurement.
//!
//! Erasure is NOT claimed from p > 0.05 (absence of evidence). It is claimed only
//! when the bootstrap 95% CI on the mean relative difference lies entirely inside
//! the pre-registered ±0.1% equivalence bound.
//!
//! ```text
//! cargo run --release --bin exp_initialization -- \
//!     --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 --seeds 5
//! ```

use ising_engine::benchmark::stats::{bootstrap_mean_ci, wilcoxon_signed_rank};
use ising_engine::engine_v2::ai_scientist::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// The pre-registered equivalence bound: ±0.1% on the mean relative difference.
const EQUIV: f64 = 0.001;

fn main() {
    let dir: String = arg("--dir", "benchmark_suite/data/gset".to_string());
    let total: u32 = arg("--sweeps", 100);
    let replicas: usize = arg("--replicas", 32);
    let nseeds: u64 = arg("--seeds", 5);
    let temp_lo: f64 = arg("--temp-lo", 0.1);

    let mut paths: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(e) => {
            eprintln!("cannot read {dir}: {e}");
            std::process::exit(1);
        }
    };
    paths.sort();

    let mut instances: Vec<(String, ProblemIR)> = Vec::new();
    for p in &paths {
        let (Ok(text), Some(id)) = (
            std::fs::read_to_string(p),
            p.file_name().and_then(|s| s.to_str()),
        ) else {
            continue;
        };
        if let Ok(ir) = rudy_maxcut_ir(&text) {
            instances.push((id.to_string(), ir));
        }
    }
    if instances.is_empty() {
        eprintln!("no parsable instances in {dir}");
        std::process::exit(1);
    }

    let reg = OperatorRegistry::standard();
    let exec = RuntimeExecutor::auto();
    let warm = total.saturating_sub(1);

    // `--deep` runs the confirmatory 2×2: is the quality axis still dead with a
    // 10× deeper warm start, and does quality add anything ON TOP of diversity?
    // If the dissociation is real then B ≈ D and A ≈ C, whatever the depth.
    let deep = std::env::args().any(|a| a == "--deep");
    let d = 10u32;
    // (label, ops, per-op sweeps) — every arm sums to `total`.
    let arms: Vec<(&str, Vec<&str>, Vec<u32>)> = if deep {
        vec![
            (
                "A zeros (status quo)",
                vec!["metropolis_sweep"],
                vec![total],
            ),
            (
                "B random only",
                vec!["random_flip_sweep", "metropolis_sweep"],
                vec![1, warm],
            ),
            (
                "C greedy x10 only",
                vec!["greedy_descent", "metropolis_sweep"],
                vec![d, total - d],
            ),
            (
                "D random+greedy x9",
                vec!["random_flip_sweep", "greedy_descent", "metropolis_sweep"],
                vec![1, d - 1, total - d],
            ),
        ]
    } else {
        vec![
            (
                "A zeros (status quo)",
                vec!["metropolis_sweep"],
                vec![total],
            ),
            (
                "B random (diverse)",
                vec!["random_flip_sweep", "metropolis_sweep"],
                vec![1, warm],
            ),
            (
                "C greedy (informed)",
                vec!["greedy_descent", "metropolis_sweep"],
                vec![1, warm],
            ),
        ]
    };

    let temps: Vec<f64> = if deep {
        vec![0.1, 4.0]
    } else {
        vec![0.1, 0.25, 0.5, 1.0, 2.0, 4.0]
    };

    println!(
        "# RC-002 initialization erasure · {} instances × {} seeds · \
         total sweeps={} replicas={} temp_lo={}",
        instances.len(),
        nseeds,
        total,
        replicas,
        temp_lo
    );
    println!("# paired on (instance, seed); positive rel = arm BEAT the zeros baseline");
    println!(
        "# erasure requires the 95% CI to sit entirely inside ±{:.2}%\n",
        EQUIV * 100.0
    );
    println!(
        "{:<8} {:<22} {:>10} {:>20} {:>9} {:>11} {:>8}",
        "temp_hi", "arm", "mean rel", "95% CI (bootstrap)", "W/L", "wilcoxon p", "verdict"
    );

    for &temp_hi in &temps {
        let mut scores: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
        for (_, ir) in &instances {
            for seed in 1..=nseeds {
                for (ai, (_, ops, sw)) in arms.iter().enumerate() {
                    let task = ExperimentTask {
                        schedule: Schedule {
                            ops: ops.iter().map(|s| s.to_string()).collect(),
                            sweeps: sw.clone(),
                            temp_hi,
                            temp_lo,
                        },
                        num_replicas: replicas,
                        seed,
                    };
                    let out = exec.run_batch(ir, &reg, std::slice::from_ref(&task));
                    scores[ai].push(out.first().map_or(f64::NAN, |o| o.score));
                }
            }
        }

        let base = scores[0].clone();
        for (ai, (label, _, _)) in arms.iter().enumerate().skip(1) {
            let s = &scores[ai];
            let rel: Vec<f64> = s
                .iter()
                .zip(&base)
                .map(|(a, n)| {
                    if n.abs() > 1e-12 {
                        (n - a) / n.abs()
                    } else {
                        0.0
                    }
                })
                .collect();
            let wins = s
                .iter()
                .zip(&base)
                .filter(|(a, n)| **a < **n - 1e-9)
                .count();
            let losses = s
                .iter()
                .zip(&base)
                .filter(|(a, n)| **a > **n + 1e-9)
                .count();
            let zeros = vec![0.0; rel.len()];
            let t = wilcoxon_signed_rank(&rel, &zeros);
            let (lo, hi) = bootstrap_mean_ci(&rel, 2000, 0.95, 12345);
            let mean_rel = rel.iter().sum::<f64>() / rel.len() as f64;

            // Equivalent = CI entirely inside the bound. Differs = significant
            // AND the CI excludes zero. Anything else is underpowered.
            let verdict = if lo > -EQUIV && hi < EQUIV {
                "ERASED"
            } else if t.p_value < 0.05 && (lo > 0.0 || hi < 0.0) {
                "DIFFERS"
            } else {
                "unclear"
            };
            println!(
                "{temp_hi:<8} {label:<22} {:>9.4}% [{:>7.4}%,{:>7.4}%] {:>4}/{:<4} {:>11.3e} {:>8}",
                mean_rel * 100.0,
                lo * 100.0,
                hi * 100.0,
                wins,
                losses,
                t.p_value,
                verdict
            );
        }
    }

    println!("\n# n pairs per cell = {}", instances.len() as u64 * nseeds);
    println!("# ERASED  = 95% CI inside ±0.1% (equivalence demonstrated)");
    println!("# DIFFERS = p<0.05 and CI excludes 0");
    println!("# unclear = underpowered; neither equivalence nor difference shown");
}
