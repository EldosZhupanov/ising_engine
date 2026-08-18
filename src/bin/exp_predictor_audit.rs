//! RC-011 — auditing "predictor leave-one-instance-out Spearman +0.747".
//!
//! `ROADMAP.md` lists this under *Confirmed:* **cross-family transfer is real**
//! (portability 9/12, predictor leave-one-instance-out Spearman +0.747).
//!
//! The fold structure is sound — `leave_one_instance_out` excludes the whole
//! instance, so there is no row-level leakage. The problem is what the metric
//! can detect.
//!
//! ## The analytic result
//!
//! `predictor::feature_row` builds
//!
//! ```text
//!   [ bias | instance features (feature_registry v0) | schedule features | op indicators ]
//! ```
//!
//! with **no interaction terms** between instance and schedule features. The
//! model is therefore purely additive:
//!
//! ```text
//!   ŷ(inst, sched) = w·x_instance(inst)  +  w·x_schedule(sched)
//! ```
//!
//! `evaluate_predictor` scores each fold by `spearman(preds, actual)` over the
//! rows of the **single** held-out instance. Every such row shares that
//! instance's signature, so `w·x_instance` is one constant across the whole test
//! set. Spearman is rank-based, and **a constant offset cannot change a ranking**.
//!
//! Therefore the reported 0.747 is *mathematically invariant* to the instance
//! features and to every weight attached to them. It measures schedule-quality
//! ranking, and it **cannot** measure transfer of instance-conditional
//! knowledge — a model that ignored the instance entirely would score the same.
//!
//! ## What this binary proves empirically
//!
//! Two load-bearing facts, so the argument does not rest on reading code:
//!
//! 1. **No interaction:** `predict(sigA, s) − predict(sigB, s)` is the SAME
//!    constant for every schedule `s`. (If interactions existed, the difference
//!    would vary with `s`.)
//! 2. **Constant within a fold:** every row of a given instance in the real
//!    18,570-run DB carries an identical instance signature.
//!
//! 1 + 2 ⇒ the offset is constant within each fold ⇒ Spearman is unchanged. QED.
//!
//! REFUTED IF either the difference in (1) varies with the schedule, or (2) finds
//! an instance whose rows disagree on their signature.
//!
//! ```text
//! cargo run --release --bin exp_predictor_audit -- \
//!     --db experiments/platform_gset/ai_experiments.txt
//! ```

use ising_engine::engine_v2::ai_scientist::db::ExperimentDb;
use ising_engine::engine_v2::ai_scientist::evaluation::evaluate_predictor;
use ising_engine::engine_v2::ai_scientist::predictor::{InstanceSignature, Predictor};
use ising_engine::engine_v2::evolution::Schedule;
use std::collections::HashMap;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == flag)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn sched(ops: &[&str], sweeps: u32, hi: f64, lo: f64) -> Schedule {
    Schedule {
        ops: ops.iter().map(|s| s.to_string()).collect(),
        sweeps: vec![sweeps; ops.len()],
        temp_hi: hi,
        temp_lo: lo,
    }
}

fn main() {
    let path: String = arg(
        "--db",
        "experiments/platform_gset/ai_experiments.txt".to_string(),
    );
    let Ok(db) = ExperimentDb::load(&path) else {
        eprintln!("cannot load DB at {path}");
        std::process::exit(1);
    };
    println!("# RC-011 predictor audit · DB rows = {}", db.all().len());

    // ---- Fact 2: is the instance signature constant within each instance? ----
    let mut sigs: HashMap<String, (f64, f64, f64, f64, f64)> = HashMap::new();
    let mut violations = 0usize;
    let mut instances = 0usize;
    for r in db.all() {
        let k = (
            r.n as f64,
            r.density,
            r.clustering,
            r.mean_degree,
            r.degree_cv,
        );
        match sigs.get(&r.instance_id) {
            None => {
                sigs.insert(r.instance_id.clone(), k);
                instances += 1;
            }
            Some(prev) if *prev != k => violations += 1,
            _ => {}
        }
    }
    println!(
        "# distinct instances = {instances}; rows whose signature disagrees with \
         their instance = {violations}"
    );

    // ---- Fact 1: is the model additive (no instance x schedule interaction)? ----
    let Some(p) = Predictor::fit(&db, 1e-3) else {
        eprintln!("predictor did not fit");
        std::process::exit(1);
    };
    let sig_a = InstanceSignature {
        n: 800,
        density: 0.06,
        clustering: 0.10,
        mean_degree: 12.0,
        degree_cv: 0.30,
    };
    let sig_b = InstanceSignature {
        n: 2000,
        density: 0.02,
        clustering: 0.02,
        mean_degree: 4.0,
        degree_cv: 0.05,
    };
    let probes = [
        sched(&["metropolis_sweep"], 50, 4.0, 0.1),
        sched(&["greedy_descent"], 10, 1.0, 0.1),
        sched(&["metropolis_sweep", "greedy_descent"], 30, 2.0, 0.5),
        sched(&["gibbs_color_sweep", "replica_exchange"], 80, 8.0, 0.05),
        sched(&["random_flip_sweep", "steepest_descent"], 5, 0.5, 0.2),
        sched(&["extremal_optimization"], 120, 3.0, 0.3),
        sched(&["houdayer_cluster", "metropolis_sweep"], 40, 1.5, 0.1),
    ];
    let deltas: Vec<f64> = probes
        .iter()
        .map(|s| p.predict(&sig_a, s) - p.predict(&sig_b, s))
        .collect();
    let d0 = deltas[0];
    let max_dev = deltas.iter().map(|d| (d - d0).abs()).fold(0.0, f64::max);
    println!(
        "# predict(sigA,s) - predict(sigB,s) over {} very different schedules:",
        probes.len()
    );
    println!("#   first = {d0:.12}   max deviation from it = {max_dev:.3e}");

    // ---- The reported metric ----
    if let Some(acc) = evaluate_predictor(&db, 1e-3) {
        let mean = acc.iter().map(|a| a.spearman).sum::<f64>() / acc.len() as f64;
        println!(
            "# evaluate_predictor: {} folds, mean leave-one-instance-out Spearman = {mean:.4}",
            acc.len()
        );
    }

    println!();
    if violations == 0 && max_dev < 1e-9 {
        println!("VERIFIED: the model is additive in (instance, schedule) and the instance");
        println!("signature is constant within every instance. So w.x_instance is a CONSTANT");
        println!("offset across each fold's test set, and Spearman — being rank-based — is");
        println!("mathematically invariant to it.");
        println!();
        println!("=> The reported leave-one-instance-out Spearman measures SCHEDULE-QUALITY");
        println!("   RANKING only. It cannot evidence transfer of instance-conditional");
        println!("   knowledge: a predictor that ignored the instance entirely would score");
        println!("   identically. ROADMAP's 'cross-family transfer is real' is not supported");
        println!("   by THIS metric (the portability 9/12 result is a separate claim).");
    } else {
        println!("REFUTED: additivity or within-instance constancy does not hold");
        println!("  (signature violations = {violations}, max deviation = {max_dev:.3e})");
        std::process::exit(1);
    }
}
