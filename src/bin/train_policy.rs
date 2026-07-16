//! `train_policy` — roadmap Levels 3/6: distill the platform's accumulated
//! experiment database into the neural operator policy (`ai_scientist::policy`),
//! then measure it HONESTLY on instances it never trained on.
//!
//! Held out instances are excluded from `train_supervised` entirely; the
//! policy's greedy schedule for each is executed on the real Runtime
//! (identical seeds/replicas/budget) alongside two references:
//!   - "default"  — the rule-based `DecisionEngine::default_plan` (capability
//!     order, no learning);
//!   - "random"   — a uniformly sampled operator sequence of the SAME length
//!     as the policy's, from the same capability pool (isolates "did it learn
//!     a good CHOICE/ORDER" from "does more compute help").
//!
//! All three get the same seed list, replica count, and per-operator sweep
//! budget. No claim beyond what the table shows (repository rule).
//!
//!   cargo run --release --bin train_policy -- \
//!       --db experiments/platform_gset/ai_experiments.txt \
//!       --gset-dir benchmark_suite/data/gset --holdout 5 --epochs 80

use ising_engine::engine_v2::ai_scientist::{ExperimentDb, OperatorPolicy};
use ising_engine::engine_v2::context::RunContext;
use ising_engine::engine_v2::decision::DecisionEngine;
use ising_engine::engine_v2::evolution::boxed_state;
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::registry::OperatorRegistry;
use ising_engine::engine_v2::runtime::Runtime;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeSet;
use std::process::exit;

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1).cloned())
}
fn argn(name: &str, d: usize) -> usize {
    arg(name).and_then(|s| s.parse().ok()).unwrap_or(d)
}
fn argf(name: &str, d: f64) -> f64 {
    arg(name).and_then(|s| s.parse().ok()).unwrap_or(d)
}

/// Run one `Schedule` on the real Runtime for `seeds`, return per-seed best
/// energies (lower is better) — the same measure the training data uses.
fn run_schedule(
    ir: &ising_engine::engine_v2::ir::ProblemIR,
    registry: &OperatorRegistry,
    schedule: &Schedule,
    num_replicas: usize,
    seeds: &[u64],
) -> Vec<f64> {
    let backend = DecisionEngine::analyze(ir).select_backend();
    let temps = ising_engine::engine_v2::decision::geometric_ladder(
        num_replicas,
        schedule.temp_hi,
        schedule.temp_lo,
    );
    seeds
        .iter()
        .map(|&seed| {
            let steps = schedule
                .ops
                .iter()
                .zip(&schedule.sweeps)
                .map(|(op, &sw)| ising_engine::engine_v2::plan::PlanStep {
                    operator: op.clone(),
                    phase: ising_engine::engine_v2::plan::Phase::Exploit,
                    sweeps: sw.max(1),
                    repeat: 1,
                })
                .collect();
            let plan = ising_engine::engine_v2::plan::Plan {
                name: "train_policy_eval".into(),
                backend,
                num_replicas,
                temperatures: temps.clone(),
                steps,
                seed,
                rationale: Default::default(),
            };
            let init = vec![0u8; ir.n];
            let mut state = boxed_state(ir, backend, num_replicas, &init);
            let mut rt = Runtime::new(RunContext::new(seed), &plan);
            rt.run(&plan, state.as_mut(), registry, ir)
                .map(|r| r.best_energy)
                .unwrap_or(f64::INFINITY)
        })
        .collect()
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}
fn min(v: &[f64]) -> f64 {
    v.iter().fold(f64::INFINITY, |a, &b| a.min(b))
}

fn main() {
    let db_path =
        arg("--db").unwrap_or_else(|| "experiments/platform_gset/ai_experiments.txt".into());
    let gset_dir = arg("--gset-dir").unwrap_or_else(|| "benchmark_suite/data/gset".into());
    let holdout_n = argn("--holdout", 5);
    let epochs = argn("--epochs", 80);
    let lr = argf("--lr", 0.05);
    let sweeps = argn("--sweeps", 20) as u32;
    let replicas = argn("--replicas", 16);
    let n_seeds = argn("--seeds", 5) as u64;
    let out = arg("--out").unwrap_or_else(|| "experiments/platform_gset/policy_eval".into());

    let db = ExperimentDb::load(&db_path).unwrap_or_else(|e| {
        eprintln!("cannot load {db_path}: {e}");
        exit(1);
    });
    if db.is_empty() {
        eprintln!("database at {db_path} is empty — run research_platform first");
        exit(1);
    }

    let instances: BTreeSet<String> = db
        .all()
        .iter()
        .map(|r| r.instance_id.clone())
        .filter(|s| !s.is_empty())
        .collect();
    let instances: Vec<String> = instances.into_iter().collect();
    if instances.len() <= holdout_n {
        eprintln!(
            "only {} instances recorded, cannot hold out {holdout_n}",
            instances.len()
        );
        exit(1);
    }
    // Deterministic split: every Nth instance (by sorted name) is held out,
    // rather than a contiguous tail, so the holdout isn't all one size class.
    let stride = instances.len() / holdout_n;
    let holdout: BTreeSet<&String> = (0..holdout_n).map(|k| &instances[k * stride]).collect();
    let train: Vec<&String> = instances.iter().filter(|i| !holdout.contains(i)).collect();

    println!(
        "{} instances total: {} train, {} held out: {:?}",
        instances.len(),
        train.len(),
        holdout.len(),
        holdout.iter().collect::<Vec<_>>()
    );

    let train_records: Vec<_> = db
        .all()
        .iter()
        .filter(|r| train.iter().any(|t| **t == r.instance_id))
        .cloned()
        .collect();
    let train_db = ExperimentDb::from_records(train_records);

    let vocab: BTreeSet<String> = train_db
        .all()
        .iter()
        .flat_map(|r| r.sequence.iter().cloned())
        .collect();
    let vocab: Vec<String> = vocab.into_iter().collect();
    println!(
        "training on {} records, vocabulary: {:?}",
        train_db.len(),
        vocab
    );

    let mut policy = OperatorPolicy::new(vocab, 1);
    let (loss_before, loss_after) = policy.train_supervised(&train_db, epochs, lr);
    println!("supervised loss: {loss_before:.4} -> {loss_after:.4}");

    let reg = OperatorRegistry::standard();
    let mut report = String::new();
    report.push_str(&format!(
        "# Neural Policy Evaluation\n\n{} instances total: {} train, {} held out.\n\
Supervised cross-entropy: {loss_before:.4} -> {loss_after:.4}.\n\n\
Equal budget for all three columns: {replicas} replicas, {sweeps} sweeps/operator, \
{n_seeds} identical seeds. Lower energy is better (raw Runtime score, not rescaled).\n\n\
| instance | n | policy schedule | policy best/mean | default schedule | default best/mean | random schedule | random best/mean | winner |\n\
|---|---|---|---|---|---|---|---|---|\n",
        instances.len(),
        train.len(),
        holdout.len(),
    ));

    let mut policy_wins = 0usize;
    let mut default_wins = 0usize;
    let mut random_wins = 0usize;
    let seeds: Vec<u64> = (0..n_seeds).map(|s| 0xC0FFEE + 7919 * s).collect();

    for inst in &holdout {
        let path = format!("{gset_dir}/{inst}");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("skipping {inst}: cannot read {path}: {e}");
                continue;
            }
        };
        let ir = match rudy_maxcut_ir(&text) {
            Ok(ir) => ir,
            Err(e) => {
                eprintln!("skipping {inst}: {e}");
                continue;
            }
        };
        let stats = DecisionEngine::analyze(&ir);
        let sig = ising_engine::engine_v2::ai_scientist::InstanceSignature {
            n: stats.n,
            density: stats.density,
            clustering: stats.clustering,
            mean_degree: stats.mean_degree,
            degree_cv: stats.degree_cv,
        };

        // Policy schedule (greedy decode from the held-out instance's own
        // features — it never trained on this instance's records).
        let policy_ops = policy.generate_greedy(&sig);
        let policy_sched = Schedule {
            ops: policy_ops.clone(),
            sweeps: vec![sweeps; policy_ops.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        let policy_e = run_schedule(&ir, &reg, &policy_sched, replicas, &seeds);

        // Rule-based default.
        let default_plan =
            DecisionEngine::default_plan(&ir, &reg, replicas, sweeps, seeds[0]).unwrap();
        let default_ops: Vec<String> = default_plan
            .steps
            .iter()
            .map(|s| s.operator.clone())
            .collect();
        let default_sweeps: Vec<u32> = default_plan.steps.iter().map(|s| s.sweeps).collect();
        let default_sched = Schedule {
            ops: default_ops.clone(),
            sweeps: default_sweeps,
            temp_hi: *default_plan.temperatures.first().unwrap_or(&4.0),
            temp_lo: *default_plan.temperatures.last().unwrap_or(&0.1),
        };
        let default_e = run_schedule(&ir, &reg, &default_sched, replicas, &seeds);

        // Random baseline: same length as the policy schedule, drawn from the
        // same capability pool, deterministic per instance.
        let pool = DecisionEngine::operator_pool(&stats, stats.select_backend(), &reg);
        let mut rng = ChaCha8Rng::seed_from_u64(0x5EED_0000 ^ fnv(inst));
        let random_ops: Vec<String> = (0..policy_ops.len().max(1))
            .map(|_| pool[rng.gen_range(0..pool.len())].to_string())
            .collect();
        let random_sched = Schedule {
            ops: random_ops.clone(),
            sweeps: vec![sweeps; random_ops.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        let random_e = run_schedule(&ir, &reg, &random_sched, replicas, &seeds);

        let (pb, pm) = (min(&policy_e), mean(&policy_e));
        let (db_, dm) = (min(&default_e), mean(&default_e));
        let (rb, rm) = (min(&random_e), mean(&random_e));
        let winner = [("policy", pm), ("default", dm), ("random", rm)]
            .into_iter()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
        match winner {
            "policy" => policy_wins += 1,
            "default" => default_wins += 1,
            _ => random_wins += 1,
        }
        println!(
            "{inst}: policy {policy_ops:?} best={pb:.1} mean={pm:.1} | default {default_ops:?} best={db_:.1} mean={dm:.1} | random {random_ops:?} best={rb:.1} mean={rm:.1} -> {winner}"
        );
        report.push_str(&format!(
            "| {inst} | {} | {:?} | {pb:.1}/{pm:.1} | {:?} | {db_:.1}/{dm:.1} | {:?} | {rb:.1}/{rm:.1} | {winner} |\n",
            ir.n, policy_ops, default_ops, random_ops
        ));
    }

    report.push_str(&format!(
        "\nMean-energy wins: policy {policy_wins}, default {default_wins}, random {random_wins} \
(equal seeds/replicas/budget; no claim beyond this table).\n\n\
## Caveats (read before citing this table)\n\n\
- **`default` is a KNOWN-WEAK baseline**: `DecisionEngine::default_plan` picks the \
first operator alphabetically per required capability — it is not tuned. Beating \
it is a low bar; treat `default_wins` as a sanity floor, not evidence of skill.\n\
- **A `random` mean/best of exactly 0.0** means the sampled single operator never \
flipped anything from the all-zero start — this happens for cluster moves \
(`houdayer_cluster`, `isoenergetic_cluster`) and `replica_exchange`/`population_resample`, \
which act on DISAGREEMENT or DIVERSITY between replicas that does not exist yet \
when every replica starts identical and no thermal pass ran first. This is \
expected operator behavior (their `needs_replicas`/pairing contract), not a \
Runtime bug — but it means those random draws are not a fair adversary.\n\
- Held-out instances are still G-Set MaxCut; no claim is made about transfer to \
other problem families (BiqMac, QPLIB, ORLIB) — that would need its own run.\n"
    ));
    println!("\nwins: policy {policy_wins}, default {default_wins}, random {random_wins}");

    let _ = std::fs::create_dir_all(&out);
    let report_path = format!("{out}/policy_eval_report.md");
    if let Err(e) = std::fs::write(&report_path, &report) {
        eprintln!("failed to write {report_path}: {e}");
    } else {
        println!("report: {report_path}");
    }
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
