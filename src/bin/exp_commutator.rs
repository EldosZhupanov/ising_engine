//! Vector-3 probe: is the operator monoid commutative?
//!
//! The Runtime executes operator SEQUENCES and the Evolution Engine searches over
//! them (`evolution.rs`, genetic search over operator orderings). That search is
//! only worth its cost if order carries information. Nobody has measured whether
//! it does.
//!
//! These are stochastic maps, so literal commutativity `A∘B = B∘A` is the wrong
//! question — `[A,B]` and `[B,A]` consume the shared RNG stream differently even
//! at an identical seed. The decidable question is whether order matters *more
//! than the seed does*:
//!
//! ```text
//!   d_order = mean over seeds of |E([A,B], seed) − E([B,A], seed)|
//!   d_seed  = mean over seed pairs of |E([A,B], sᵢ) − E([A,B], sⱼ)|
//!   rho     = d_order / d_seed
//! ```
//!
//! `d_seed` is the null: it is the spread the same schedule produces from seed
//! alone. rho ≈ 1 means swapping the operators perturbs the outcome no more than
//! re-seeding does — the pair commutes *in distribution*, and searching both
//! orderings is wasted compute. rho >> 1 means order is structural.
//!
//! PRE-REGISTERED PREDICTION: rho ≈ 1 for most pairs, but large wherever the two
//! operators differ in finishing quality (e.g. greedy_descent vs
//! random_flip_sweep), because the schedule is dominated by which operator runs
//! LAST. Refuted if rho is uniformly large (order matters generally) or
//! uniformly ≈ 1 (order never matters, including for the asymmetric pairs).
//!
//! ```text
//! cargo run --release --bin exp_commutator -- --dir benchmark_suite/data/gset
//! ```

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

/// Operators spanning the library's distinct families: thermal, greedy,
/// stochastic, replica-coupled, cluster, extremal.
const OPS: &[&str] = &[
    "metropolis_sweep",
    "gibbs_color_sweep",
    "greedy_descent",
    "steepest_descent",
    "random_flip_sweep",
    "replica_exchange",
    "houdayer_cluster",
    "extremal_optimization",
];

fn main() {
    let dir: String = arg("--dir", "benchmark_suite/data/gset".to_string());
    let sweeps: u32 = arg("--sweeps", 30);
    let replicas: usize = arg("--replicas", 32);
    let nseeds: u64 = arg("--seeds", 6);
    let max_inst: usize = arg("--instances", 6);

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
        if instances.len() >= max_inst {
            break;
        }
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

    // --diverse prepends one randomising sweep. From the all-zeros init every
    // replica is IDENTICAL (RC-002), which makes replica_exchange and
    // houdayer_cluster structurally inert — they have nothing to swap and no
    // overlap domains. Without this, their apparent commutativity is vacuity.
    let diverse = std::env::args().any(|x| x == "--diverse");
    let run = |ir: &ProblemIR, a: &str, b: &str, seed: u64| -> f64 {
        let (ops, sw) = if diverse {
            (
                vec![
                    "random_flip_sweep".to_string(),
                    a.to_string(),
                    b.to_string(),
                ],
                vec![1, sweeps, sweeps],
            )
        } else {
            (vec![a.to_string(), b.to_string()], vec![sweeps, sweeps])
        };
        let task = ExperimentTask {
            schedule: Schedule {
                ops,
                sweeps: sw,
                temp_hi: 2.0,
                temp_lo: 0.1,
            },
            num_replicas: replicas,
            seed,
        };
        exec.run_batch(ir, &reg, std::slice::from_ref(&task))
            .first()
            .map_or(f64::NAN, |o| o.score)
    };

    println!(
        "# operator commutator · {} instances × {} seeds · sweeps={} each · replicas={}",
        instances.len(),
        nseeds,
        sweeps,
        replicas
    );
    println!("# rho = d_order / d_seed;  rho ~ 1 => pair commutes in distribution\n");
    println!(
        "{:<24} {:<24} {:>12} {:>12} {:>8}",
        "A", "B", "d_order", "d_seed(null)", "rho"
    );

    let mut rows: Vec<(f64, String, String, f64, f64)> = Vec::new();

    for (i, &a) in OPS.iter().enumerate() {
        for &b in OPS.iter().skip(i + 1) {
            let (mut sum_order, mut n_order) = (0.0f64, 0usize);
            let (mut sum_seed, mut n_seed) = (0.0f64, 0usize);

            for (_, ir) in &instances {
                let mut ab = Vec::new();
                for s in 1..=nseeds {
                    let e_ab = run(ir, a, b, s);
                    let e_ba = run(ir, b, a, s);
                    // Scale-free: normalise by |E| so instances of different
                    // magnitude contribute comparably.
                    let scale = e_ab.abs().max(1e-9);
                    sum_order += (e_ab - e_ba).abs() / scale;
                    n_order += 1;
                    ab.push(e_ab);
                }
                // Null: same schedule, different seeds.
                for p in 0..ab.len() {
                    for q in (p + 1)..ab.len() {
                        let scale = ab[p].abs().max(1e-9);
                        sum_seed += (ab[p] - ab[q]).abs() / scale;
                        n_seed += 1;
                    }
                }
            }

            let d_order = sum_order / n_order.max(1) as f64;
            let d_seed = sum_seed / n_seed.max(1) as f64;
            let rho = if d_seed > 1e-12 {
                d_order / d_seed
            } else {
                f64::INFINITY
            };
            rows.push((rho, a.to_string(), b.to_string(), d_order, d_seed));
        }
    }

    rows.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal));
    for (rho, a, b, d_order, d_seed) in &rows {
        println!("{a:<24} {b:<24} {d_order:>12.6} {d_seed:>12.6} {rho:>8.2}");
    }

    let commuting = rows.iter().filter(|r| r.0 < 2.0).count();
    println!(
        "\n# pairs measured: {}   effectively commuting (rho < 2): {} ({:.0}%)",
        rows.len(),
        commuting,
        100.0 * commuting as f64 / rows.len() as f64
    );
    println!("# A high commuting fraction means the Evolution Engine's ordering search");
    println!("# explores distinctions that do not exist.");
}
