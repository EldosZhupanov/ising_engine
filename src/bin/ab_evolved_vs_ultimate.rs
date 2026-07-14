//! `ab_evolved_vs_ultimate` — measured comparison of the engine_v2 EVOLVED
//! plans against the production `UltimateSolver` on its own benchmarks
//! (repository rule: no superiority claims without measurement).
//!
//! Fairness contract, printed with the results:
//!  - identical instances (rudy MaxCut files → the same cut objective, scored
//!    by each side's canonical scorer);
//!  - identical seed LIST for both solvers (per-run seeds derived the same way);
//!  - identical replica count and identical TOTAL SWEEP budget (the evolved
//!    plan's sweep sum is given to UltimateSolver as sweeps × exchanges);
//!  - wall-clock reported for both, because equal sweeps is NOT equal work
//!    across different architectures — the table shows both dimensions and
//!    the verdict is left to the numbers.
//!
//!   cargo run --release --bin ab_evolved_vs_ultimate -- \
//!       --files benchmark_suite/data/gset/G11,benchmark_suite/data/gset/G12 \
//!       --seeds 5 --replicas 32
//!
//! Evolution cost is reported separately (it is a one-off per instance) and
//! NOT hidden inside the per-run walls.

use ising_engine::benchmark::instances::parse_rudy_maxcut;
use ising_engine::engine_v2::ai_scientist::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use ising_engine::engine_v2::evolution::{EvolveConfig, Evolver, Schedule, UtilityWeights};
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::registry::OperatorRegistry;
use ising_engine::solver::ultimate::UltimateSolver;
use std::process::exit;
use std::time::Instant;

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1).cloned())
}
fn argn(name: &str, d: usize) -> usize {
    arg(name).and_then(|s| s.parse().ok()).unwrap_or(d)
}

fn main() {
    let files = arg("--files").unwrap_or_else(|| {
        eprintln!("usage: ab_evolved_vs_ultimate --files <rudy>[,..] [--seeds N] [--replicas N] [--gens N]");
        exit(2);
    });
    let n_seeds = argn("--seeds", 5) as u64;
    let replicas = argn("--replicas", 32);
    let generations = argn("--gens", 8);

    let reg = OperatorRegistry::standard();
    let executor = RuntimeExecutor::auto();

    let sweep_scale = argn("--evo-sweep-scale", 1).max(1);
    println!(
        "A/B: evolved plan vs UltimateSolver | {} seeds/instance, {} replicas, \
Ultimate budget = evolved UNSCALED sweep total, evolved side scaled ×{} — wall columns are ground truth\n",
        n_seeds, replicas, sweep_scale
    );
    println!(
        "{:<6} {:>6} | {:>9} {:>9} {:>9} | {:>9} {:>9} {:>9} | {:>10}",
        "inst",
        "n",
        "evo best",
        "evo mean",
        "evo ms",
        "ult best",
        "ult mean",
        "ult ms",
        "evolve ms"
    );

    let mut evo_wins = 0usize;
    let mut ult_wins = 0usize;
    let mut ties = 0usize;

    for file in files.split(',').filter(|f| !f.trim().is_empty()) {
        let file = file.trim();
        let text = std::fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("cannot read {file}: {e}");
            exit(1);
        });
        let name = std::path::Path::new(file)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(file);
        let ir = rudy_maxcut_ir(&text).unwrap_or_else(|e| {
            eprintln!("{name}: {e}");
            exit(1);
        });
        let inst = parse_rudy_maxcut(&text, name).unwrap_or_else(|e| {
            eprintln!("{name}: {e}");
            exit(1);
        });

        // One-off: evolve this instance's plan (deterministic seeds).
        let t0 = Instant::now();
        let evolver = Evolver::new(EvolveConfig {
            num_replicas: replicas,
            generations,
            utility: UtilityWeights::quality_only(),
            ..Default::default()
        });
        let evolved = evolver.evolve(&ir, &reg).unwrap_or_else(|e| {
            eprintln!("{name}: evolution failed: {e}");
            exit(1);
        });
        let evolve_ms = t0.elapsed().as_secs_f64() * 1e3;
        // --evo-sweep-scale K repeats the evolved schedule's budget K× (an
        // equal-WALL comparison point when the evolved runs are much faster at
        // equal sweeps; the printed walls remain the ground truth).
        let scale = argn("--evo-sweep-scale", 1).max(1) as u32;
        let schedule = Schedule {
            ops: evolved.sequence.clone(),
            sweeps: evolved.sweeps.iter().map(|s| s * scale).collect(),
            temp_hi: evolved.temp_hi,
            temp_lo: evolved.temp_lo,
        };
        // Ultimate's budget stays at the UNSCALED sweep total: the scale flag
        // only buys the evolved side more of its own (cheaper) sweeps.
        let total_sweeps: u32 = evolved.sweeps.iter().sum();

        // Identical seed list for both sides.
        let seeds: Vec<u64> = (0..n_seeds).map(|s| 0xAB00 + 7919 * s).collect();

        // Evolved side: one Runtime run per seed (parallel batch).
        let tasks: Vec<ExperimentTask> = seeds
            .iter()
            .map(|&seed| ExperimentTask {
                schedule: schedule.clone(),
                num_replicas: replicas,
                seed,
            })
            .collect();
        let t0 = Instant::now();
        let outcomes = executor.run_batch(&ir, &reg, &tasks);
        let evo_ms = t0.elapsed().as_secs_f64() * 1e3 / seeds.len() as f64;
        let evo_cuts: Vec<f64> = outcomes.iter().map(|o| -o.score).collect();

        // Ultimate side: same seeds, same replicas, same total sweep budget.
        let exchanges = 10usize;
        let sweeps_per = (total_sweeps as usize).div_ceil(exchanges).max(1);
        let mut ult_cuts = Vec::with_capacity(seeds.len());
        let t0 = Instant::now();
        for &seed in &seeds {
            let solver = UltimateSolver {
                num_replicas: replicas,
                seed: Some(seed),
                ..UltimateSolver::new(
                    evolved.temp_hi,
                    evolved.temp_lo,
                    sweeps_per,
                    exchanges,
                    Some(seed),
                )
            };
            let state = solver.solve(&inst.model, &[]);
            let energy = inst.model.calculate_total_energy(&state);
            ult_cuts.push(inst.native_objective(energy));
        }
        let ult_ms = t0.elapsed().as_secs_f64() * 1e3 / seeds.len() as f64;

        let best = |v: &[f64]| v.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
        let (eb, em) = (best(&evo_cuts), mean(&evo_cuts));
        let (ub, um) = (best(&ult_cuts), mean(&ult_cuts));
        if em > um {
            evo_wins += 1;
        } else if um > em {
            ult_wins += 1;
        } else {
            ties += 1;
        }
        println!(
            "{:<6} {:>6} | {:>9.0} {:>9.1} {:>9.1} | {:>9.0} {:>9.1} {:>9.1} | {:>10.0}",
            name, ir.n, eb, em, evo_ms, ub, um, ult_ms, evolve_ms
        );
    }

    println!(
        "\nmean-cut wins: evolved {evo_wins}, ultimate {ult_wins}, ties {ties} \
(equal seeds + replicas + total sweeps; walls differ — see columns; no claim beyond this table)"
    );
}
