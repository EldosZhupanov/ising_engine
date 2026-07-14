//! `evolve` — demonstrate the Evolution Engine discovering an algorithm.
//!
//! Given a rudy MaxCut instance, it lets the engine SEARCH the space of operator
//! sequences (drawn by capability, Task 4) and prints the discovered pipeline,
//! its cut, and the per-generation trajectory — then compares it to the
//! hand-written capability-default plan. This is the "search for an algorithm"
//! made visible (Stage 4).
//!
//!   cargo run --release --bin evolve -- --file benchmark_suite/data/gset/G11 \
//!       --generations 8 --population 16 --replicas 64

use ising_engine::engine_v2::backends::{ReferenceState, SparseBitSlice};
use ising_engine::engine_v2::context::RunContext;
use ising_engine::engine_v2::decision::DecisionEngine;
use ising_engine::engine_v2::evolution::{EvolveConfig, Evolver};
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::knowledge::KnowledgeBase;
use ising_engine::engine_v2::plan::{Backend, Plan};
use ising_engine::engine_v2::registry::OperatorRegistry;
use ising_engine::engine_v2::runtime::Runtime;
use ising_engine::engine_v2::state::SpinState;
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

fn run_plan(
    ir: &ising_engine::engine_v2::ir::ProblemIR,
    plan: &Plan,
    reg: &OperatorRegistry,
    r: usize,
) -> f64 {
    let init = vec![0u8; ir.n];
    let mut state: Box<dyn SpinState> = match plan.backend {
        Backend::SparseBitSlice => match SparseBitSlice::new(ir, r, &init) {
            Ok(s) => Box::new(s),
            Err(_) => Box::new(ReferenceState::new(ir, r, &init)),
        },
        Backend::DenseByte => Box::new(ReferenceState::new(ir, r, &init)),
    };
    let mut rt = Runtime::new(RunContext::new(plan.seed), plan);
    rt.run(plan, state.as_mut(), reg, ir)
        .map(|x| x.best_energy)
        .unwrap_or(f64::INFINITY)
}

fn main() {
    let file = arg("--file").unwrap_or_else(|| {
        eprintln!("usage: evolve --file <rudy> [--generations N] [--population N] [--replicas N] [--seed N]");
        exit(2);
    });
    let replicas = argn("--replicas", 64);
    let cfg = EvolveConfig {
        population: argn("--population", 16),
        generations: argn("--generations", 8),
        max_steps: argn("--max-steps", 5),
        num_replicas: replicas,
        max_sweeps_per_step: argn("--max-sweeps", 40) as u32,
        run_seed: arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(1),
        evolve_seed: arg("--evolve-seed")
            .and_then(|s| s.parse().ok())
            .unwrap_or(777),
        utility: Default::default(),
    };

    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("cannot read {file}: {e}");
        exit(1);
    });
    let ir = rudy_maxcut_ir(&text).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        exit(1);
    });

    let reg = OperatorRegistry::standard();
    let stats = DecisionEngine::analyze(&ir);
    let backend = stats.select_backend();
    let pool = DecisionEngine::operator_pool(&stats, backend, &reg);

    println!(
        "instance: n={} pairs={} density={:.4} integral={}",
        stats.n, stats.num_pairs, stats.density, stats.integral
    );
    println!("backend (by rule): {backend:?}");
    println!("operator pool (by capability): {pool:?}\n");

    // Baseline: the hand-written capability-default plan.
    let baseline =
        DecisionEngine::default_plan(&ir, &reg, replicas, cfg.max_sweeps_per_step, cfg.run_seed)
            .unwrap_or_else(|e| {
                eprintln!("default_plan: {e}");
                exit(1);
            });
    let base_energy = run_plan(&ir, &baseline, &reg, replicas);
    let base_steps: Vec<&str> = baseline.steps.iter().map(|s| s.operator.as_str()).collect();
    println!("default plan  : {base_steps:?}  -> cut {}", -base_energy);

    // Meta-learning: load the persistent knowledge base, seed the search from
    // similar past instances, evolve, then record + persist the outcome so the
    // next run is better informed.
    let kb_path = arg("--kb").unwrap_or_else(|| "experiments/results/knowledge.txt".into());
    let kb = KnowledgeBase::load(&kb_path).unwrap_or_default();
    let similar = kb.similar(
        &ising_engine::engine_v2::knowledge::InstanceFeatures::from_stats(&stats),
        3,
    );
    println!(
        "knowledge base: {} experiences ({} similar seeds)\n",
        kb.len(),
        similar.len()
    );

    // The evolved algorithm (seeded from the knowledge base).
    let res = Evolver::new(cfg)
        .evolve_with_kb(&ir, &reg, &kb)
        .unwrap_or_else(|e| {
            eprintln!("evolve: {e}");
            exit(1);
        });
    println!("evolved plan  : {:?}", res.sequence);
    println!("  sweeps      : {:?}", res.sweeps);
    println!("  ladder      : T {:.3} -> {:.3}", res.temp_hi, res.temp_lo);
    println!("  evaluations : {}", res.evaluations);
    println!(
        "  utility     : {:.2}  (work {:.2e}, mem {:.1} KB)",
        res.best_utility,
        res.best_work,
        res.best_memory_bytes / 1024.0
    );
    let traj: Vec<i64> = res.generation_best.iter().map(|&u| -u as i64).collect();
    println!("  utility/gen : {traj:?}");
    println!(
        "\nRESULT  default cut = {}   evolved cut = {}   Δ = {:+}",
        -base_energy as i64,
        -res.best_energy as i64,
        (-res.best_energy + base_energy) as i64
    );

    // Record this outcome and persist, so future runs learn from it.
    let mut kb = kb;
    kb.record(res.to_experience(&ir));
    if let Some(parent) = std::path::Path::new(&kb_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match kb.save(&kb_path) {
        Ok(()) => println!(
            "knowledge base updated: {kb_path} ({} experiences)",
            kb.len()
        ),
        Err(e) => eprintln!("warning: could not save knowledge base: {e}"),
    }
}
