//! `portability` — does the platform generalize BEYOND the family it learned
//! on? This is the test that decides whether "universal optimization system"
//! is a real claim or marketing.
//!
//! The neural operator policy is trained ONLY on G-Set MaxCut experiment
//! history (`experiments/platform_gset`). It is then evaluated, cold, on
//! instances from OTHER problem families — BiqMac, OR-Library BQP, QPLIB —
//! parsed by the benchmark front-ends and bridged to the engine_v2 IR (the
//! bridge is energy-exact, proven by `frontend` tests). On each held-out
//! instance the policy's greedy schedule is run on the REAL Runtime against
//! two references at identical seeds / replicas / budget:
//!   - default: `DecisionEngine::default_plan` (rule-based, known weak);
//!   - random:  a same-length operator sequence from the capability pool.
//!
//! Lower raw Runtime energy is better (the family sign conventions are already
//! baked into the bridge). Transfer is claimed ONLY to the extent the table
//! shows it — a family where the policy does not beat baselines is reported as
//! a non-transfer, not hidden.
//!
//!   cargo run --release --bin portability -- \
//!       --db experiments/platform_gset/ai_experiments.txt \
//!       --data benchmark_suite/data --per-family 4 --seeds 3

use ising_engine::benchmark::instances::{
    parse_biqmac_sparse, parse_orlib_bqp, parse_qplib, parse_rudy_maxcut, Instance,
};
use ising_engine::engine_v2::ai_scientist::{
    BatchExecutor, ExperimentDb, ExperimentTask, InstanceSignature, OperatorPolicy, RuntimeExecutor,
};
use ising_engine::engine_v2::decision::DecisionEngine;
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::qubo_model_to_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
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

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}
fn min(v: &[f64]) -> f64 {
    v.iter().fold(f64::INFINITY, |a, &b| a.min(b))
}

/// Mean/best raw energy of `schedule` over `seeds` on the real Runtime.
fn run(
    exec: &RuntimeExecutor,
    ir: &ProblemIR,
    reg: &OperatorRegistry,
    schedule: &Schedule,
    replicas: usize,
    seeds: &[u64],
) -> Vec<f64> {
    let tasks: Vec<ExperimentTask> = seeds
        .iter()
        .map(|&seed| ExperimentTask {
            schedule: schedule.clone(),
            num_replicas: replicas,
            seed,
        })
        .collect();
    exec.run_batch(ir, reg, &tasks)
        .iter()
        .map(|o| o.score)
        .collect()
}

struct FamilyResult {
    family: String,
    instances: usize,
    policy_wins: usize,
    default_wins: usize,
    random_wins: usize,
    /// Mean (policy − default) energy margin; negative ⇒ policy better.
    mean_vs_default: f64,
}

#[allow(clippy::too_many_arguments)]
fn eval_family(
    family: &str,
    irs: &[(String, ProblemIR)],
    policy: &OperatorPolicy,
    reg: &OperatorRegistry,
    exec: &RuntimeExecutor,
    replicas: usize,
    sweeps: u32,
    seeds: &[u64],
) -> FamilyResult {
    let (mut pw, mut dw, mut rw) = (0, 0, 0);
    let mut margins = Vec::new();
    for (name, ir) in irs {
        let stats = DecisionEngine::analyze(ir);
        let sig = InstanceSignature {
            n: stats.n,
            density: stats.density,
            clustering: stats.clustering,
            mean_degree: stats.mean_degree,
            degree_cv: stats.degree_cv,
        };
        let policy_ops = policy.generate_greedy(&sig);
        let policy_sched = Schedule {
            ops: policy_ops.clone(),
            sweeps: vec![sweeps; policy_ops.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        let default_plan = match DecisionEngine::default_plan(ir, reg, replicas, sweeps, seeds[0]) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let default_sched = Schedule {
            ops: default_plan
                .steps
                .iter()
                .map(|s| s.operator.clone())
                .collect(),
            sweeps: default_plan.steps.iter().map(|s| s.sweeps).collect(),
            temp_hi: *default_plan.temperatures.first().unwrap_or(&4.0),
            temp_lo: *default_plan.temperatures.last().unwrap_or(&0.1),
        };
        let pool = DecisionEngine::operator_pool(&stats, stats.select_backend(), reg);
        let mut rng = ChaCha8Rng::seed_from_u64(0x50_0000 ^ name.len() as u64);
        let random_ops: Vec<String> = (0..policy_ops.len().max(1))
            .map(|_| pool[rng.gen_range(0..pool.len())].to_string())
            .collect();
        let random_sched = Schedule {
            ops: random_ops,
            sweeps: vec![sweeps; policy_ops.len().max(1)],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };

        let pe = run(exec, ir, reg, &policy_sched, replicas, seeds);
        let de = run(exec, ir, reg, &default_sched, replicas, seeds);
        let re = run(exec, ir, reg, &random_sched, replicas, seeds);
        let (pm, dm, rm) = (mean(&pe), mean(&de), mean(&re));
        margins.push(pm - dm);
        let winner = [("p", pm), ("d", dm), ("r", rm)]
            .into_iter()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0;
        match winner {
            "p" => pw += 1,
            "d" => dw += 1,
            _ => rw += 1,
        }
        println!(
            "  [{family}] {name} (n={}): policy {policy_ops:?} best={:.1}/mean={:.1} | default best={:.1}/mean={:.1} | random best={:.1}/mean={:.1} -> {}",
            stats.n, min(&pe), pm, min(&de), dm, min(&re), rm,
            match winner { "p" => "policy", "d" => "default", _ => "random" }
        );
    }
    FamilyResult {
        family: family.to_string(),
        instances: pw + dw + rw,
        policy_wins: pw,
        default_wins: dw,
        random_wins: rw,
        mean_vs_default: mean(&margins),
    }
}

/// Take up to `k` instances from a family, bridging each to the IR.
fn bridge_all(insts: Vec<Instance>, k: usize) -> Vec<(String, ProblemIR)> {
    insts
        .into_iter()
        .take(k)
        .map(|i| (i.name.clone(), qubo_model_to_ir(&i.model)))
        .collect()
}

fn main() {
    let db_path =
        arg("--db").unwrap_or_else(|| "experiments/platform_gset/ai_experiments.txt".into());
    let data = arg("--data").unwrap_or_else(|| "benchmark_suite/data".into());
    let per_family = argn("--per-family", 4);
    let epochs = argn("--epochs", 80);
    let sweeps = argn("--sweeps", 20) as u32;
    let replicas = argn("--replicas", 16);
    let n_seeds = argn("--seeds", 3) as u64;
    let out =
        arg("--out").unwrap_or_else(|| "experiments/platform_gset/portability_report.md".into());

    let db = ExperimentDb::load(&db_path).unwrap_or_else(|e| {
        eprintln!("cannot load {db_path}: {e}");
        exit(1);
    });
    if db.is_empty() {
        eprintln!("empty database at {db_path}; run research_platform first");
        exit(1);
    }
    let vocab: std::collections::BTreeSet<String> = db
        .all()
        .iter()
        .flat_map(|r| r.sequence.iter().cloned())
        .collect();
    let mut policy = OperatorPolicy::new(vocab.into_iter().collect(), 1);
    let (lb, la) = policy.train_supervised(&db, epochs, 0.05);
    println!(
        "policy trained on {} MaxCut experiments (loss {lb:.3} -> {la:.3})\n",
        db.len()
    );

    let reg = OperatorRegistry::standard();
    let exec = RuntimeExecutor::auto();
    let seeds: Vec<u64> = (0..n_seeds).map(|s| 0xDA7A + 7919 * s).collect();

    // Assemble the transfer families from whatever is present locally.
    let mut families: Vec<(String, Vec<(String, ProblemIR)>)> = Vec::new();

    // BiqMac: several small dense instances.
    let mut biqmac = Vec::new();
    if let Ok(rd) = std::fs::read_dir(format!("{data}/biqmac")) {
        let mut files: Vec<_> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "sparse"))
            .collect();
        files.sort();
        for p in files.into_iter().take(per_family) {
            if let Ok(t) = std::fs::read_to_string(&p) {
                let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
                if let Ok(inst) = parse_biqmac_sparse(&t, name) {
                    biqmac.push((inst.name.clone(), qubo_model_to_ir(&inst.model)));
                }
            }
        }
    }
    if !biqmac.is_empty() {
        families.push(("biqmac".into(), biqmac));
    }

    // OR-Library BQP: first file holds several instances.
    if let Ok(t) = std::fs::read_to_string(format!("{data}/orlib/bqp100.txt")) {
        if let Ok(insts) = parse_orlib_bqp(&t, "bqp100") {
            let v = bridge_all(insts, per_family);
            if !v.is_empty() {
                families.push(("orlib_bqp".into(), v));
            }
        }
    }

    // QPLIB: individual files (only binary unconstrained ones parse).
    let mut qplib = Vec::new();
    if let Ok(rd) = std::fs::read_dir(format!("{data}/qplib")) {
        let mut files: Vec<_> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "qplib"))
            .collect();
        files.sort();
        for p in files {
            if qplib.len() >= per_family {
                break;
            }
            if let Ok(t) = std::fs::read_to_string(&p) {
                let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
                if let Ok(inst) = parse_qplib(&t, name) {
                    qplib.push((inst.name.clone(), qubo_model_to_ir(&inst.model)));
                }
            }
        }
    }
    if !qplib.is_empty() {
        families.push(("qplib".into(), qplib));
    }

    // A MaxCut reference row (in-distribution family) for calibration.
    let mut maxcut = Vec::new();
    for g in ["G1", "G14", "G22"] {
        if let Ok(t) = std::fs::read_to_string(format!("{data}/gset/{g}")) {
            if let Ok(inst) = parse_rudy_maxcut(&t, g) {
                maxcut.push((inst.name.clone(), qubo_model_to_ir(&inst.model)));
            }
        }
    }
    if !maxcut.is_empty() {
        families.push(("maxcut(ref)".into(), maxcut));
    }

    if families.is_empty() {
        eprintln!("no benchmark family data found under {data}; nothing to test");
        exit(1);
    }

    let mut results = Vec::new();
    for (fam, irs) in &families {
        println!("=== {fam} ({} instances) ===", irs.len());
        results.push(eval_family(
            fam, irs, &policy, &reg, &exec, replicas, sweeps, &seeds,
        ));
    }

    // Report.
    let mut report = String::new();
    report.push_str(&format!(
        "# Cross-Family Portability\n\nPolicy trained ONLY on {} G-Set MaxCut experiments \
(supervised loss {lb:.3} -> {la:.3}). Evaluated cold on other families; energy-exact \
IR bridge; equal {replicas} replicas / {sweeps} sweeps / {n_seeds} seeds; lower raw energy \
is better.\n\n| family | instances | policy wins | default wins | random wins | mean policy−default energy |\n|---|---|---|---|---|---|\n",
        db.len()
    ));
    println!("\n=== summary (mean-energy winner per instance) ===");
    for r in &results {
        println!(
            "{:<14} {:>2} inst | policy {} | default {} | random {} | Δvs-default {:+.1}",
            r.family, r.instances, r.policy_wins, r.default_wins, r.random_wins, r.mean_vs_default
        );
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:+.1} |\n",
            r.family, r.instances, r.policy_wins, r.default_wins, r.random_wins, r.mean_vs_default
        ));
    }
    let transfer: usize = results
        .iter()
        .filter(|r| r.family != "maxcut(ref)")
        .map(|r| r.policy_wins)
        .sum();
    let transfer_total: usize = results
        .iter()
        .filter(|r| r.family != "maxcut(ref)")
        .map(|r| r.instances)
        .sum();
    report.push_str(&format!(
        "\n**Transfer**: the MaxCut-trained policy won {transfer}/{transfer_total} of the \
non-MaxCut instances against the (weak) default and random baselines. This measures \
generalization of the LEARNED operator choice across problem families — negative \
Δ means the policy's schedule reached lower energy than the rule-based default. No claim \
beyond this table; a family the policy does not win is a real limit, not hidden.\n\n\
Caveats: `default_plan` is a known-weak first-alphabetical baseline; `random` can idle at \
0 energy when a lone cluster/replica move has no disagreement to act on from the all-zero \
start (operator contract, not a bug). Budgets are small (fast smoke-scale), not a full \
benchmarking run.\n"
    ));
    if let Err(e) = std::fs::write(&out, &report) {
        eprintln!("failed to write {out}: {e}");
    } else {
        println!("\nreport: {out}");
    }
    println!("transfer: policy won {transfer}/{transfer_total} non-MaxCut instances");
}
