//! RC-010 — auditing the World model's "imagined-vs-real Spearman 0.975".
//!
//! `ROADMAP.md` reports the World model's imagined-vs-real schedule ranking at
//! **Spearman 0.975** as evidence it is a useful cheap filter. Forensics on the
//! source of that number:
//!
//!  * that test asserts only `rho >= 0.5`, so 0.975 is an observed value, never
//!    an enforced property.
//!  * **4 of its 5 candidate schedules are verbatim members of the training
//!    set** (the source comment says "held-out-ish", which is honest; the
//!    ROADMAP presentation is not).
//!  * the candidate set spans `random_flip_sweep` alone (pure noise, terrible)
//!    against schedules ending in `greedy_descent` (good). A model that learns
//!    only "greedy good, noise bad" already orders most of that set correctly.
//!
//! So the headline number may encode roughly ONE BIT of information — an obvious
//! quality gap — rather than fine-grained ranking skill.
//!
//! A FORENSIC STEP THAT WAS WRONG, recorded so it is not repeated: an earlier
//! version of this file argued 0.975 was unreachable at n = 5 (untied Spearman
//! gives 1.0, 0.95, 0.90, …) and therefore could not have come from that test.
//! `evaluation.rs:110` averages ranks over ties, which makes intermediate values
//! reachable; arm A below returns 0.9747. Never derive an attainability grid
//! without first checking tie handling.
//!
//! PRE-REGISTERED PREDICTION. Three candidate sets, same trained model:
//!
//!   A `reproduce`   — the original set (4/5 in training): high ρ expected.
//!   B `heldout`     — no candidate appears in training, still spanning the
//!                     noise-to-greedy quality range: ρ should stay high, since
//!                     the easy gap survives.
//!   C `homogeneous` — every candidate ends in `greedy_descent`, differing only
//!                     in its thermal prefix, so the easy gap is REMOVED and the
//!                     model must rank fine-grained differences.
//!
//! **ρ must collapse on C** if the correlation was carrying the easy gap. If ρ
//! stays high on C, the model genuinely ranks and the claim is sound — which
//! would refute this audit.
//!
//! ```text
//! cargo run --release --bin exp_world_model_audit
//! ```

use ising_engine::engine_v2::ai_scientist::world::{rank_correlation, WorldModel};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;

/// Same frustrated ring family the World model's own test trains on, so the
/// audit is a like-for-like comparison rather than a harder problem.
fn ring(n: usize, shift: f64) -> ProblemIR {
    let mut pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
        .map(|i| (i, i + 1, if i % 2 == 0 { -1.0 } else { 1.0 }))
        .collect();
    pairs.push((0, n as u32 - 1, -1.0));
    let linear: Vec<f64> = (0..n).map(|i| ((i as f64) * shift).cos() * 0.5).collect();
    ProblemIR::from_pairs(n, 0.0, linear, &pairs)
}

fn sched(ops: &[&str], sweeps: u32) -> Schedule {
    Schedule {
        ops: ops.iter().map(|s| s.to_string()).collect(),
        sweeps: vec![sweeps; ops.len()],
        temp_hi: 3.0,
        temp_lo: 0.1,
    }
}

fn key(s: &Schedule) -> String {
    format!("{}|{:?}", s.ops.join("+"), s.sweeps)
}

fn main() {
    let reg = OperatorRegistry::standard();

    // Training set: exactly as in world.rs's own test.
    let train_scheds = vec![
        sched(&["random_flip_sweep"], 4),
        sched(&["greedy_descent"], 4),
        sched(&["metropolis_sweep", "greedy_descent"], 4),
        sched(&["gibbs_color_sweep", "greedy_descent"], 4),
        sched(&["extremal_optimization", "greedy_descent"], 4),
    ];
    let train: Vec<ProblemIR> = (0..5).map(|i| ring(30, 0.2 + i as f64 * 0.13)).collect();
    let refs: Vec<&ProblemIR> = train.iter().collect();
    let Some(model) = WorldModel::fit(&refs, &reg, &train_scheds, 8, &[1, 2, 3], 1e-4) else {
        eprintln!("model did not fit (insufficient transitions)");
        std::process::exit(1);
    };
    let held = ring(30, 4.7);
    let train_keys: Vec<String> = train_scheds.iter().map(key).collect();

    // A: the original candidate set.
    let set_a = vec![
        sched(&["random_flip_sweep"], 4),
        sched(&["greedy_descent"], 4),
        sched(&["metropolis_sweep", "greedy_descent"], 4),
        sched(&["gibbs_color_sweep", "greedy_descent"], 4),
        sched(&["extremal_optimization", "metropolis_sweep"], 4),
    ];

    // B: genuinely held out (no member in training), still spanning the full
    // noise-to-greedy quality range so the easy gap is intact.
    let set_b = vec![
        sched(&["random_flip_sweep"], 6),
        sched(&["random_flip_sweep", "random_flip_sweep"], 5),
        sched(&["steepest_descent"], 6),
        sched(&["metropolis_sweep", "steepest_descent"], 6),
        sched(&["gibbs_color_sweep", "steepest_descent"], 6),
        sched(&["history_field", "steepest_descent"], 6),
        sched(&["extremal_optimization", "steepest_descent"], 6),
        sched(&["random_flip_sweep", "steepest_descent"], 6),
        sched(
            &["metropolis_sweep", "gibbs_color_sweep", "steepest_descent"],
            5,
        ),
    ];

    // C: quality-HOMOGENEOUS. Every candidate ends in greedy_descent, so the
    // trivial noise-vs-greedy distinction is gone and only fine-grained
    // prefix differences remain.
    let set_c = vec![
        sched(&["metropolis_sweep", "greedy_descent"], 6),
        sched(&["gibbs_color_sweep", "greedy_descent"], 6),
        sched(&["history_field", "greedy_descent"], 6),
        sched(&["extremal_optimization", "greedy_descent"], 6),
        sched(&["random_flip_sweep", "greedy_descent"], 6),
        sched(&["steepest_descent", "greedy_descent"], 6),
        sched(
            &["metropolis_sweep", "metropolis_sweep", "greedy_descent"],
            4,
        ),
        sched(
            &["gibbs_color_sweep", "metropolis_sweep", "greedy_descent"],
            4,
        ),
        sched(&["extremal_metropolis", "greedy_descent"], 6),
    ];

    println!("# RC-010 World-model rank-correlation audit");
    println!("# trained_on = {} transitions", model.trained_on);
    println!(
        "# NOTE world.rs's own test uses n=5 and gates only at rho >= 0.5, so \
         0.975 was observed, never enforced.\n"
    );
    println!(
        "{:<14} {:>3} {:>10} {:>10} {:>9}  note",
        "candidate set", "n", "in-train", "spearman", "spread"
    );

    for (label, set, note) in [
        ("A reproduce", &set_a, "original: easy gap present"),
        ("B heldout", &set_b, "no overlap; easy gap present"),
        ("C homogeneous", &set_c, "easy gap REMOVED"),
    ] {
        let overlap = set.iter().filter(|s| train_keys.contains(&key(s))).count();
        let (rho, pairs) = rank_correlation(&held, &reg, &model, set, 8, &[10, 11, 12]);
        // Spread of the REAL scores: if it is tiny, the ranking task is
        // near-degenerate and rho is dominated by noise either way.
        let reals: Vec<f64> = pairs.iter().map(|(_, r)| *r).collect();
        let (lo, hi) = reals
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)));
        let spread = (hi - lo) / hi.abs().max(1e-9);
        println!(
            "{label:<14} {:>3} {:>9}/{} {rho:>10.4} {:>8.2}%  {note}",
            set.len(),
            overlap,
            set.len(),
            spread * 100.0
        );
    }

    println!("\n# If rho collapses on C while staying high on A and B, the reported");
    println!("# correlation was carrying the noise-vs-greedy gap, not ranking skill.");
    println!("# If rho stays high on C, the model genuinely ranks and this audit fails.");
}
