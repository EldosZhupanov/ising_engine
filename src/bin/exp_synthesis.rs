//! RC-003 — falsification harness for H-C′ (runtime move synthesis).
//!
//! Pre-registered in `research/RC003_ARCHITECTURE_AUDIT.md`.
//!
//! Three arms at identical sweep budget:
//!   N  metropolis_sweep    single-flip only (the `synth_null` path is unit-tested
//!                          bit-identical to it)
//!   P  synth_population    moves mined from cross-replica covariance — the KNOWN
//!                          method (LTGA / GOMEA / EDA linkage learning)
//!   D  synth_dynamics      moves mined from temporal co-flip — H-C′
//!
//! The decisive quantity is **D − P**, not either against N. By the ergodic
//! theorem the two mined structures coincide at equilibrium, so H-C′ has content
//! only out of equilibrium and predicts:
//!
//!   * D > P under fast mixing-limited conditions (cold ladder), and
//!   * the D − P gap SHRINKING monotonically as mixing improves (hotter ladder).
//!
//! A flat gap refutes H-C′ even if D beats N — that would just be a lucky
//! neighborhood, not an architecture. An energy win under 1% is not claimed as
//! success unless the mixing-rate pattern holds.
//!
//! ```text
//! cargo run --release --bin exp_synthesis -- \
//!     --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 --seeds 5
//! ```

use ising_engine::benchmark::stats::{bootstrap_mean_ci, wilcoxon_signed_rank};
use ising_engine::engine_v2::ai_scientist::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == flag)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Paired contrast of `a` against `b`: mean relative gain, bootstrap CI, W/L, p.
fn contrast(a: &[f64], b: &[f64]) -> (f64, f64, f64, usize, usize, f64) {
    let rel: Vec<f64> = a
        .iter()
        .zip(b)
        .map(|(x, y)| {
            if y.abs() > 1e-12 {
                (y - x) / y.abs()
            } else {
                0.0
            }
        })
        .collect();
    let wins = a.iter().zip(b).filter(|(x, y)| **x < **y - 1e-9).count();
    let losses = a.iter().zip(b).filter(|(x, y)| **x > **y + 1e-9).count();
    let (lo, hi) = bootstrap_mean_ci(&rel, 2000, 0.95, 4242);
    let z = vec![0.0; rel.len()];
    let p = wilcoxon_signed_rank(&rel, &z).p_value;
    (
        rel.iter().sum::<f64>() / rel.len() as f64,
        lo,
        hi,
        wins,
        losses,
        p,
    )
}

fn main() {
    let dir: String = arg("--dir", "benchmark_suite/data/gset".to_string());
    let sweeps: u32 = arg("--sweeps", 100);
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
        let (Ok(t), Some(id)) = (
            std::fs::read_to_string(p),
            p.file_name().and_then(|s| s.to_str()),
        ) else {
            continue;
        };
        if let Ok(ir) = rudy_maxcut_ir(&t) {
            instances.push((id.to_string(), ir));
        }
    }
    if instances.is_empty() {
        eprintln!("no parsable instances in {dir}");
        std::process::exit(1);
    }

    let reg = OperatorRegistry::standard();
    let exec = RuntimeExecutor::auto();
    let arms = ["metropolis_sweep", "synth_population", "synth_dynamics"];
    // Mixing rate proxy: a hotter ladder mixes faster and sits closer to
    // equilibrium, where the ergodic theorem forces D and P to coincide.
    let temps = [0.1f64, 0.5, 1.0, 2.0, 4.0];

    println!(
        "# RC-003 runtime move synthesis · {} instances × {} seeds · sweeps={} replicas={}",
        instances.len(),
        nseeds,
        sweeps,
        replicas
    );
    println!("# positive rel = first arm found LOWER energy. D-P is the falsification test.\n");
    println!(
        "{:<8} {:<12} {:>10} {:>20} {:>10} {:>11}",
        "temp_hi", "contrast", "mean rel", "95% CI", "W/L", "wilcoxon p"
    );

    for &temp_hi in &temps {
        let mut sc: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
        for (_, ir) in &instances {
            for seed in 1..=nseeds {
                for (ai, op) in arms.iter().enumerate() {
                    let task = ExperimentTask {
                        schedule: Schedule {
                            ops: vec![(*op).to_string()],
                            sweeps: vec![sweeps],
                            temp_hi,
                            temp_lo,
                        },
                        num_replicas: replicas,
                        seed,
                    };
                    let out = exec.run_batch(ir, &reg, std::slice::from_ref(&task));
                    sc[ai].push(out.first().map_or(f64::NAN, |o| o.score));
                }
            }
        }
        for (label, a, b) in [
            ("P vs N", 1usize, 0usize),
            ("D vs N", 2, 0),
            ("D vs P", 2, 1),
        ] {
            let (m, lo, hi, w, l, p) = contrast(&sc[a], &sc[b]);
            println!(
                "{temp_hi:<8} {label:<12} {:>9.4}% [{:>7.4}%,{:>7.4}%] {:>5}/{:<4} {:>11.3e}",
                m * 100.0,
                lo * 100.0,
                hi * 100.0,
                w,
                l,
                p
            );
        }
        println!();
    }
    println!(
        "# n pairs per contrast = {}",
        instances.len() as u64 * nseeds
    );
    println!("# H-C' requires the D-P gap to SHRINK monotonically as temp_hi rises.");
    println!("# A flat D-P gap refutes H-C' regardless of whether D beats N.");
}
