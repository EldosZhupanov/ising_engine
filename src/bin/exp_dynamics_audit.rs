//! RC-012 — auditing the Dynamics early-stop controller at its deployed defaults.
//!
//! `Dynamics` is the last unaudited learned model, and the only one that ACTS:
//! `EarlyStopController::decide` converts `predict_remaining < epsilon` directly
//! into `RunControl::Stop` once past `min_frac` of the budget. A wrong prediction
//! does not mis-rank a candidate — it truncates a run and silently costs quality.
//! Deployed defaults (`executor.rs::EarlyStopConfig`): **epsilon = 0.005,
//! min_frac = 0.5**.
//!
//! ## Method — offline decision-rule evaluation
//!
//! Trajectories are captured WITHOUT any controller attached, then the exact
//! deployed rule is replayed offline over each trajectory's own step stream:
//! stop at the first step with `frac_elapsed >= min_frac` and predicted
//! remaining < epsilon. Because the full trajectory is known, the ACTUAL
//! remaining improvement at the stop point is known too:
//!
//! ```text
//! actual_k = (best_k − best_final) / max(|best_k|, 1)     [training's own scale]
//! ```
//!
//! A **false stop** is a firing with `actual_k >= epsilon`: the controller
//! discarded at least ε of reachable improvement. This measures the controller's
//! real cost with zero new runtime machinery and zero measurement contamination.
//!
//! ## Pre-registered prediction
//!
//! Training mirrors the module's own test (frustrated rings, greedy quench).
//! The controller will be well calibrated ON that distribution, and will fire
//! FALSELY on thermal schedules (late improvement is real there — RC-002), i.e.
//! missed improvement >= epsilon, because one shared ridge cannot carry both
//! trajectory shapes. REFUTED if missed improvement < epsilon in every condition.
//!
//! Conditions:
//!   A  held-out ring, greedy quench   (training distribution)
//!   B  held-out ring, thermal anneal  (schedule shift)
//!   C  G11,           greedy quench   (instance shift — the deployment path,
//!                                      which bootstraps from one instance)
//!
//! ```text
//! cargo run --release --bin exp_dynamics_audit
//! ```

use ising_engine::engine_v2::ai_scientist::dynamics::{
    capture_trajectory, train_on_instances, DynamicsModel, Trajectory,
};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;

const EPSILON: f64 = 0.005; // deployed default (executor.rs)
const MIN_FRAC: f64 = 0.5; // deployed default (executor.rs)

fn ring(n: usize, shift: f64) -> ProblemIR {
    let mut pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
        .map(|i| (i, i + 1, if i % 2 == 0 { -1.0 } else { 1.0 }))
        .collect();
    pairs.push((0, n as u32 - 1, -1.0));
    let linear: Vec<f64> = (0..n).map(|i| ((i as f64) * shift).sin()).collect();
    ProblemIR::from_pairs(n, 0.0, linear, &pairs)
}

fn quench(sweeps_each: u32, repeats: usize) -> Schedule {
    Schedule {
        ops: vec!["greedy_descent".to_string(); repeats],
        sweeps: vec![sweeps_each; repeats],
        temp_hi: 2.0,
        temp_lo: 0.05,
    }
}

fn thermal(sweeps_each: u32, repeats: usize) -> Schedule {
    Schedule {
        ops: vec!["metropolis_sweep".to_string(); repeats],
        sweeps: vec![sweeps_each; repeats],
        temp_hi: 2.0,
        temp_lo: 0.05,
    }
}

/// Replay the deployed decision rule offline. Returns
/// (fired, stop_frac, predicted_at_stop, actual_at_stop).
fn replay_rule(model: &DynamicsModel, tr: &Trajectory) -> (bool, f64, f64, f64) {
    let best: Vec<f64> = tr.steps.iter().map(|s| s.best_energy).collect();
    let final_best = *best.last().unwrap();
    for (k, s) in tr.steps.iter().enumerate() {
        let pred = model.predict_remaining(
            &tr.features,
            &best,
            k,
            s.mean_energy,
            s.entropy,
            s.diversity,
            s.acceptance,
            s.frac_elapsed,
        );
        if s.frac_elapsed >= MIN_FRAC && pred < EPSILON {
            let scale = best[k].abs().max(1.0);
            let actual = (best[k] - final_best) / scale;
            return (true, s.frac_elapsed, pred, actual);
        }
    }
    (false, 1.0, f64::NAN, 0.0)
}

fn main() {
    let reg = OperatorRegistry::standard();

    // Train exactly as the module's own test does: rings + greedy quench.
    let sched_train = quench(2, 12);
    let train: Vec<ProblemIR> = (0..4).map(|i| ring(24, 0.3 + i as f64 * 0.2)).collect();
    let refs: Vec<&ProblemIR> = train.iter().collect();
    let Some(model) = train_on_instances(&refs, &reg, &sched_train, 8, &[1, 2, 3], 1e-4) else {
        eprintln!("model did not fit");
        std::process::exit(1);
    };
    println!(
        "# RC-012 Dynamics early-stop audit · trained_on = {} rows · deployed rule: \
         stop when frac_elapsed >= {MIN_FRAC} and predicted remaining < {EPSILON}",
        model.trained_on
    );

    let held = ring(24, 5.0);
    let g11 = std::fs::read_to_string("benchmark_suite/data/gset/G11")
        .ok()
        .and_then(|t| rudy_maxcut_ir(&t).ok());

    let mut conditions: Vec<(&str, &ProblemIR, Schedule)> = vec![
        ("A ring+quench (in-dist)", &held, quench(2, 12)),
        ("B ring+thermal (sched shift)", &held, thermal(2, 12)),
    ];
    if let Some(ir) = g11.as_ref() {
        conditions.push(("C G11+quench (inst shift)", ir, quench(2, 12)));
        conditions.push(("D G11+thermal (both shifts)", ir, thermal(2, 12)));
    }

    println!(
        "\n{:<30} {:>6} {:>9} {:>10} {:>10} {:>11} {:>7}",
        "condition", "fired", "stop@", "predicted", "ACTUAL", "false-stop?", "saved"
    );

    let mut false_stops = 0usize;
    let mut firings = 0usize;
    for (label, ir, sched) in &conditions {
        for seed in [99u64, 100, 101, 102, 103] {
            let tr = capture_trajectory(ir, &reg, sched, 8, seed);
            let (fired, stop_frac, pred, actual) = replay_rule(&model, &tr);
            if fired {
                firings += 1;
                let false_stop = actual >= EPSILON;
                if false_stop {
                    false_stops += 1;
                }
                println!(
                    "{label:<30} {:>6} {stop_frac:>8.2} {pred:>10.5} {actual:>10.5} {:>11} {:>6.0}%",
                    "yes",
                    if false_stop { "FALSE" } else { "ok" },
                    (1.0 - stop_frac) * 100.0
                );
            } else {
                println!(
                    "{label:<30} {:>6} {:>8} {:>10} {:>10} {:>11} {:>7}",
                    "no", "-", "-", "-", "-", "-"
                );
            }
        }
    }

    println!("\n# firings: {firings} · false stops (actual >= {EPSILON} discarded): {false_stops}");
    println!("# PREDICTION: false stops concentrated on thermal / shifted conditions.");
    println!("# REFUTED if every firing's actual remaining < {EPSILON}.");

    // ── Q2: is the epsilon test doing ANY work, or is min_frac the whole rule? ──
    // Every firing above landed at exactly frac_elapsed = MIN_FRAC with a
    // predicted value of 0.00000. If the model predicts ~0 along the entire
    // trajectory, the "learned" controller is behaviourally the fixed rule
    // "truncate at min_frac", and epsilon is vacuous. Profile the prediction
    // and the ACTUAL remaining at every step for one seed per condition.
    println!("\n# Q2 — prediction profile (seed 99): predicted vs actual remaining per step");
    for (label, ir, sched) in &conditions {
        let tr = capture_trajectory(ir, &reg, sched, 8, 99);
        let best: Vec<f64> = tr.steps.iter().map(|s| s.best_energy).collect();
        let final_best = *best.last().unwrap();
        print!("{label:<30} pred:");
        for (k, s) in tr.steps.iter().enumerate() {
            let p = model.predict_remaining(
                &tr.features,
                &best,
                k,
                s.mean_energy,
                s.entropy,
                s.diversity,
                s.acceptance,
                s.frac_elapsed,
            );
            print!(" {p:.4}");
        }
        println!();
        print!("{:<30} real:", "");
        for bk in &best {
            let scale = bk.abs().max(1.0);
            print!(" {:.4}", (bk - final_best) / scale);
        }
        println!();
    }
    println!("\n# If pred is ~0 at every step in every condition, epsilon never gates");
    println!("# anything and the controller degenerates to 'stop at min_frac'.");

    // ── Q3: the DEPLOYED bootstrap, verbatim ──────────────────────────────────
    // `research_platform --early-stop` trains on instance[0] only, with the
    // 2-step schedule [metropolis(24), greedy(24)], seeds [1,2,3]. If each
    // trajectory contributes only 2 rows, that is 6 < 20 and `fit` returns None
    // — meaning the flag would ALWAYS "skip honestly". Reproduce it exactly.
    if let Some(ir) = g11.as_ref() {
        let boot = Schedule {
            ops: vec!["metropolis_sweep".into(), "greedy_descent".into()],
            sweeps: vec![24, 24],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        println!(
            "\n# Q3 — deployed bootstrap verbatim (instance[0] = G11, boot schedule, seeds 1-3)"
        );
        let tr0 = capture_trajectory(ir, &reg, &boot, 8, 1);
        println!("#   steps per trajectory = {}", tr0.steps.len());
        match train_on_instances(&[ir], &reg, &boot, 8, &[1, 2, 3], 1e-4) {
            Some(m) => {
                println!("#   fit SUCCEEDED: trained_on = {} rows", m.trained_on);
                // Profile the deployed model on a thermal run of the SAME instance.
                let tr = capture_trajectory(ir, &reg, &thermal(2, 12), 8, 99);
                let best: Vec<f64> = tr.steps.iter().map(|s| s.best_energy).collect();
                let fb = *best.last().unwrap();
                print!("#   deployed-model pred on G11+thermal:");
                for (k, s) in tr.steps.iter().enumerate() {
                    let p = m.predict_remaining(
                        &tr.features,
                        &best,
                        k,
                        s.mean_energy,
                        s.entropy,
                        s.diversity,
                        s.acceptance,
                        s.frac_elapsed,
                    );
                    print!(" {p:.4}");
                }
                println!();
                print!("#   actual remaining:                  ");
                for bk in &best {
                    print!(" {:.4}", (bk - fb) / bk.abs().max(1.0));
                }
                println!();
            }
            None => println!(
                "#   fit returned NONE (rows < 20) — the deployed --early-stop flag \
                 can never activate with this bootstrap"
            ),
        }
    }
}
