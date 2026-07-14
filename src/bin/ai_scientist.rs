//! `ai_scientist` — run an autonomous discovery campaign with the multi-agent
//! Scientific Lab (Stage 5+).
//!
//! Four agents (Hypothesis Generator, Experiment Designer, Statistician,
//! Knowledge Manager) collaborate: they reason from operator capabilities and a
//! persistent knowledge graph, design novelty-filtered experiments, run them on
//! the read-only core via a parallel executor, judge significance, and store
//! only reproducible patterns. Fitness is cost-aware. Everything persists
//! (experiment DB, knowledge base, knowledge graph), so the lab keeps learning.
//!
//!   cargo run --release --bin ai_scientist -- --file benchmark_suite/data/gset/G11 \
//!       --rounds 4 --batch 60 --replicas 32

use ising_engine::engine_v2::ai_scientist::{
    ExperimentDb, KnowledgeGraph, LabConfig, LlmHypothesisGenerator, MetaLearner, RuntimeExecutor,
    ScientificLab,
};
use ising_engine::engine_v2::evolution::Evolver;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::knowledge::KnowledgeBase;
use ising_engine::engine_v2::registry::OperatorRegistry;
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

fn main() {
    let file = arg("--file").unwrap_or_else(|| {
        eprintln!(
            "usage: ai_scientist --file <rudy> [--rounds N] [--batch N] [--replicas N] [--seed N]"
        );
        exit(2);
    });
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("cannot read {file}: {e}");
        exit(1);
    });
    let ir = rudy_maxcut_ir(&text).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        exit(1);
    });

    let reg = OperatorRegistry::standard();
    let evolver = Evolver::new(Default::default());
    let executor = RuntimeExecutor::auto();

    // Persistent stores — the lab resumes and keeps learning across campaigns.
    let dir = arg("--dir").unwrap_or_else(|| "experiments/results".into());
    let db_path = format!("{dir}/ai_experiments.txt");
    let kb_path = format!("{dir}/knowledge.txt");
    let graph_path = format!("{dir}/knowledge_graph.txt");
    let mut db = ExperimentDb::load(&db_path).unwrap_or_default();
    let mut kb = KnowledgeBase::load(&kb_path).unwrap_or_default();
    let graph = KnowledgeGraph::load(&graph_path).unwrap_or_default();

    let cfg = LabConfig {
        rounds: argn("--rounds", 4),
        hypotheses_per_round: argn("--hypotheses", 12),
        batch_size: argn("--batch", 60),
        seeds_per_hypothesis: argn("--seeds", 4),
        num_replicas: argn("--replicas", 32),
        max_ops: argn("--max-ops", 4),
        base_seed: arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(1),
        ..Default::default()
    };

    // --llm <model> drives the scientist with a local LLM (Ollama); the LLM only
    // proposes ideas, the deterministic core does all computation. Without it,
    // the deterministic heuristic generator is used.
    let llm_model = arg("--llm");
    println!(
        "Scientific Lab on {file}: {} rounds, batch {}, {} cores, scientist = {}",
        cfg.rounds,
        cfg.batch_size,
        executor.threads,
        llm_model.as_deref().unwrap_or("heuristic")
    );
    println!(
        "resuming: {} experiments, {} knowledge-graph facts\n",
        db.len(),
        graph.len()
    );

    let mut lab = match &llm_model {
        Some(model) => ScientificLab::with_ideator(
            cfg.base_seed,
            graph,
            Box::new(LlmHypothesisGenerator::new(model.clone())),
        ),
        None => ScientificLab::with_graph(cfg.base_seed, graph),
    };
    let report = lab.run(&ir, &reg, &evolver, &executor, &mut db, &mut kb, &cfg);

    for d in lab.decisions() {
        println!("  {d}");
    }
    println!();
    println!("baseline cut         : {}", -report.baseline as i64);
    println!("best discovered cut  : {}", -report.best_score as i64);
    println!("best schedule        : {:?}", report.best_schedule.ops);
    println!("  cost-aware fitness : {:.2}", report.best_fitness);
    println!("experiments run      : {}", report.experiments_run);
    println!("architectures tried  : {}", report.archive_size);
    println!("knowledge-graph facts: {}", report.graph_facts);
    println!("confirmed hypotheses : {}", report.confirmed.len());
    for h in report.confirmed.iter().take(6) {
        println!(
            "  [{}] {:?}  imp {:+.1}  conf {:.2}",
            h.id, h.operators, h.observed_improvement, h.confidence
        );
        println!("        WHY: {}", h.reasoning);
    }
    println!("\nlearned patterns (knowledge graph, strongest first):");
    let mut facts: Vec<_> = lab.graph().triples().to_vec();
    facts.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    for t in facts.iter().take(8) {
        println!(
            "  {} --{}--> {}   (weight {:+.1}, support {})",
            t.subject, t.predicate, t.object, t.weight, t.support
        );
    }

    // Meta-Learner: mine the accumulated history for rules ABOUT ALGORITHMS and
    // write a research report (what the LLM scientist reads next).
    let report = MetaLearner::new().report(&db, lab.graph());
    println!("\n{}", "=".repeat(60));
    print!("{report}");

    // Persist the growing stores + the report.
    let _ = std::fs::create_dir_all(&dir);
    let _ = db.save(&db_path);
    let _ = kb.save(&kb_path);
    let _ = lab.graph().save(&graph_path);
    let _ = std::fs::write(format!("{dir}/research_report.md"), &report);
    println!(
        "\npersisted DB ({} records) + knowledge graph ({} facts) + research report to {dir}/",
        db.len(),
        lab.graph().len()
    );
}
