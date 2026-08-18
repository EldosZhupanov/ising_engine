//! RC-001 — the pre-registered trial for the Ensemble-Consensus Thermostat.
//!
//! Design (fixed in `research/RC001_ENSEMBLE_THERMOSTAT.md` before any run):
//! three arms at IDENTICAL budget, replica count, temperature ladder and seeds.
//!
//!   null   = `metropolis_sweep`     (λ = 0, verified bit-identical by unit test)
//!   freeze = `consensus_freeze`     (λ < 0, consensus sites colder)
//!   seek   = `consensus_seek`       (λ > 0, consensus sites hotter)
//!
//! Pairing is over (instance, seed), so each observation differs from its
//! control in exactly one thing: the λ term. Significance is a two-sided
//! Wilcoxon signed-rank test; the threshold p < 0.05 was fixed in advance.
//!
//! All computation goes through `RuntimeExecutor`, the canonical harness — this
//! binary only arranges tasks and applies statistics, so no bespoke measuring
//! device enters the comparison.
//!
//! ```text
//! cargo run --release --bin exp_consensus -- \
//!     --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 \
//!     --seeds 5 --temp-hi 4.0 --temp-lo 0.1
//! ```

use ising_engine::benchmark::stats::wilcoxon_signed_rank;
use ising_engine::engine_v2::ai_scientist::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operators::EnsembleThermostat;
use ising_engine::engine_v2::registry::OperatorRegistry;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Median of an unsorted slice (NaN-free by construction here).
fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = s.len() / 2;
    if s.len().is_multiple_of(2) {
        (s[m - 1] + s[m]) / 2.0
    } else {
        s[m]
    }
}

fn main() {
    let dir: String = arg("--dir", "benchmark_suite/data/gset".to_string());
    let sweeps: u32 = arg("--sweeps", 100);
    let replicas: usize = arg("--replicas", 32);
    let nseeds: u64 = arg("--seeds", 5);
    let temp_hi: f64 = arg("--temp-hi", 4.0);
    let temp_lo: f64 = arg("--temp-lo", 0.1);

    // Load every instance in the directory, sorted for a deterministic order.
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
        let Ok(text) = std::fs::read_to_string(p) else {
            continue;
        };
        let Ok(ir) = rudy_maxcut_ir(&text) else {
            continue;
        };
        let id = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        instances.push((id, ir));
    }
    if instances.is_empty() {
        eprintln!("no parsable instances in {dir}");
        std::process::exit(1);
    }

    let mut reg = OperatorRegistry::standard();
    let exec = RuntimeExecutor::auto();

    // `--dose` replaces the three-arm trial with a λ ladder. A single point is
    // not a mechanism: if the consensus term is real, effect size must move
    // with λ. A flat or non-monotone curve refutes the mechanism even when the
    // individual points are significant.
    let dose = std::env::args().any(|a| a == "--dose");
    const LADDER: &[(&str, f64)] = &[
        ("ect_m2.0", -2.0),
        ("ect_m1.0", -1.0),
        ("ect_m0.5", -0.5),
        ("ect_p0.5", 0.5),
        ("ect_p1.0", 1.0),
        ("ect_p2.0", 2.0),
        ("ect_p3.0", 3.0),
    ];
    let arms: Vec<(String, &str)> = if dose {
        for &(name, lambda) in LADDER {
            reg.register(move || Box::new(EnsembleThermostat::new(name, lambda)));
        }
        std::iter::once(("null λ=0".to_string(), "metropolis_sweep"))
            .chain(
                LADDER
                    .iter()
                    .map(|&(n, l)| (format!("λ={l:+.1}"), n as &str)),
            )
            .collect()
    } else {
        vec![
            ("null(metropolis)".to_string(), "metropolis_sweep"),
            ("freeze(λ<0)".to_string(), "consensus_freeze"),
            ("seek(λ>0)".to_string(), "consensus_seek"),
        ]
    };

    println!(
        "# RC-001 ensemble-consensus thermostat · {} instances × {} seeds \
         · sweeps={} replicas={} ladder={}→{}",
        instances.len(),
        nseeds,
        sweeps,
        replicas,
        temp_hi,
        temp_lo
    );
    println!("# paired on (instance, seed); lower energy is better\n");

    // scores[arm][k] over the flattened (instance, seed) pairs.
    let mut scores: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
    let mut labels: Vec<String> = Vec::new();

    for (id, ir) in &instances {
        for seed in 1..=nseeds {
            labels.push(format!("{id}/s{seed}"));
            for (ai, (_, op)) in arms.iter().enumerate() {
                let task = ExperimentTask {
                    schedule: Schedule {
                        ops: vec![op.to_string()],
                        sweeps: vec![sweeps],
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

    let null = &scores[0];
    println!(
        "{:<18} {:>14} {:>12} {:>12} {:>17} {:>6} {:>12}",
        "arm", "mean energy", "mean rel", "median rel", "win/loss/tie", "n_eff", "wilcoxon p"
    );
    for (ai, (label, _)) in arms.iter().enumerate() {
        let s = &scores[ai];
        let mean = s.iter().sum::<f64>() / s.len() as f64;
        if ai == 0 {
            println!(
                "{label:<18} {mean:>14.2} {:>12} {:>10} {:>12}",
                "—", "—", "—"
            );
            continue;
        }
        // Positive = arm found LOWER energy than its paired control.
        let rel: Vec<f64> = s
            .iter()
            .zip(null)
            .map(|(a, n)| {
                if n.abs() > 1e-12 {
                    (n - a) / n.abs()
                } else {
                    0.0
                }
            })
            .collect();
        let wins = s.iter().zip(null).filter(|(a, n)| **a < **n - 1e-9).count();
        let losses = s.iter().zip(null).filter(|(a, n)| **a > **n + 1e-9).count();
        let ties = s.len() - wins - losses;
        // Rank the SCALE-FREE relative differences, not the raw energies:
        // G-Set energies span an order of magnitude across instances, so a
        // signed-rank test on raw scores is dominated by the largest graphs.
        let zeros = vec![0.0; rel.len()];
        let t = wilcoxon_signed_rank(&rel, &zeros);
        let mean_rel = rel.iter().sum::<f64>() / rel.len() as f64;
        println!(
            "{label:<18} {mean:>14.2} {:>11.4}% {:>11.4}% {:>5}/{:<5}/{:<5} {:>6} {:>12.3e}",
            mean_rel * 100.0,
            median(&rel) * 100.0,
            wins,
            losses,
            ties,
            t.n,
            t.p_value
        );
    }

    println!("\n# n pairs = {}", null.len());
    println!("# win/loss/tie counted at 1e-9; n_eff = non-tied pairs entering Wilcoxon");
    println!("# pre-registered success criterion: p < 0.05 AND median rel > 0.1%");
}
