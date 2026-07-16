//! `research_platform` — Stage 6: the fully autonomous research platform, from
//! one command. Runs continuous campaigns of the closed scientific cycle
//! (hypotheses → evolution → Runtime → append-only DB → meta-learner →
//! knowledge graph → reports → optional local/cloud LLM tiers → operator
//! proposals → predictor filter → dashboard), persisting everything under
//! `--dir` so it can be interrupted and resumed indefinitely.
//!
//!   cargo run --release --bin research_platform -- \
//!       --file benchmark_suite/data/gset/G11 --campaigns 2 --generations 4 \
//!       [--llm qwen2.5-coder:7b] [--cloud-model claude-sonnet-5]
//!
//! The core (Runtime, Scheduler, SpinState, backends, Experiment Runner,
//! canonical scorer, Operator API) is read-only; this binary only orchestrates.

use ising_engine::engine_v2::ai_scientist::{
    train_on_instances, write_evaluation, CampaignConfig, CampaignManager, LabConfig, MetaLearner,
    RuntimeExecutor,
};
use ising_engine::engine_v2::evolution::{Evolver, Schedule};
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
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
    let mut instances = Vec::new();
    // --family {tsp|max2sat} SYNTHESIZES a runnable instance of another problem
    // family (same campaign/orchestrator loop, via the energy-exact families
    // frontend) instead of reading a MaxCut rudy file. Deterministic in --seed.
    if let Some(fam) = arg("--family") {
        use ising_engine::engine_v2::families::{max2sat_instance, tsp_instance};
        let seed = arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(1);
        match fam.as_str() {
            "tsp" => {
                let cities = argn("--cities", 8);
                println!("synthesizing TSP: {cities} cities, seed {seed}");
                instances.push(tsp_instance(cities, seed));
            }
            "max2sat" | "sat" => {
                let vars = argn("--vars", 40);
                let clauses = argn("--clauses", vars * 4);
                println!("synthesizing MAX-2-SAT: {vars} vars, {clauses} clauses, seed {seed}");
                instances.push(max2sat_instance(vars, clauses, seed));
            }
            other => {
                eprintln!("unknown --family '{other}' (use tsp | max2sat)");
                exit(2);
            }
        }
    } else {
        // One or more instances: --file accepts a comma-separated list, so a whole
        // benchmark family runs from one command (e.g. --file "$(ls d/G* | paste -sd,)").
        let files_arg = arg("--file").unwrap_or_else(|| {
            eprintln!(
                "usage: research_platform (--file <rudy>[,...] | --family tsp|max2sat) [--dir D]\n\
       [--campaigns N] [--generations N] [--rounds N] [--hypotheses N] [--batch N] [--seeds N]\n\
       [--replicas N] [--report-every N] [--cloud-every N] [--llm MODEL] [--cloud-model MODEL]\n\
       [--seed N] [--cities N] [--vars N] [--clauses N] [--early-stop]"
            );
            exit(2);
        });
        for file in files_arg.split(',').filter(|f| !f.trim().is_empty()) {
            let file = file.trim();
            let text = std::fs::read_to_string(file).unwrap_or_else(|e| {
                eprintln!("cannot read {file}: {e}");
                exit(1);
            });
            let ir = rudy_maxcut_ir(&text).unwrap_or_else(|e| {
                eprintln!("parse error in {file}: {e}");
                exit(1);
            });
            let id = std::path::Path::new(file)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unnamed")
                .to_string();
            instances.push((id, ir));
        }
    }

    let dir = arg("--dir").unwrap_or_else(|| "experiments/platform".into());
    let campaigns = argn("--campaigns", 1);
    let report_every = argn("--report-every", 5000);

    let base_cfg = CampaignConfig {
        instance_id: String::new(), // set per instance below
        generations: argn("--generations", 4),
        lab: LabConfig {
            rounds: argn("--rounds", 3),
            hypotheses_per_round: argn("--hypotheses", 12),
            batch_size: argn("--batch", 60),
            seeds_per_hypothesis: argn("--seeds", 4),
            num_replicas: argn("--replicas", 32),
            max_ops: argn("--max-ops", 4),
            ..Default::default()
        },
        report_every,
        cloud_every: argn("--cloud-every", 10_000),
        llm_model: arg("--llm"),
        cloud_model: arg("--cloud-model"),
        predictor_oversample: argn("--oversample", 3),
        // --shared-knowledge turns on the Meta-Learning Layer (Policy/World/
        // Dynamics consensus biasing ideation + published to the graph).
        shared_knowledge: std::env::args().any(|a| a == "--shared-knowledge"),
        // --curiosity <λ> turns on the Curiosity Engine (explore/exploit dial).
        curiosity_lambda: arg("--curiosity")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0),
        base_seed: arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(1),
    };

    let reg = OperatorRegistry::standard();
    let evolver = Evolver::new(Default::default());
    let mut executor = RuntimeExecutor::auto();
    // --early-stop (opt-in): attach a Dynamics early-stop controller to EVERY
    // run, so plateaued tasks stop instead of burning their full budget. We
    // bootstrap a Dynamics model from the first instance; if there is too little
    // trajectory data to fit one, we skip honestly rather than pretend.
    if std::env::args().any(|a| a == "--early-stop") {
        if let Some((_, ir0)) = instances.first() {
            let boot = Schedule {
                ops: vec!["metropolis_sweep".into(), "greedy_descent".into()],
                sweeps: vec![24, 24],
                temp_hi: 4.0,
                temp_lo: 0.1,
            };
            match train_on_instances(
                &[ir0],
                &reg,
                &boot,
                base_cfg.lab.num_replicas.max(2),
                &[1, 2, 3],
                1e-4,
            ) {
                Some(model) => {
                    executor = executor.with_early_stop_default(model);
                    println!("early-stop: Dynamics controller active on every run (opt-in)");
                }
                None => println!(
                    "early-stop requested but too little trajectory data to fit a Dynamics model — skipping honestly"
                ),
            }
        }
    }

    // --render-dashboard just (re)generates the dashboard from the persisted
    // stores — no experiments run — and prints its path. Use it to view the
    // accumulated platform state at any time.
    if std::env::args().any(|a| a == "--render-dashboard") {
        use ising_engine::engine_v2::ai_scientist::{
            write_dashboard_with, ResearchExecutive, ResourceState,
        };
        let mgr = CampaignManager::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        // The Chief Scientist surveys the accumulated state for the hero panel.
        let resources = ResourceState {
            cloud_available: std::env::var("ANTHROPIC_API_KEY").is_ok(),
            local_available: true,
            ..Default::default()
        };
        let brief = ResearchExecutive::new(Default::default())
            .assess(&mgr.db, &mgr.graph, &resources, None);
        match write_dashboard_with(&dir, &mgr.db, &mgr.graph, &mgr.archive, &[], Some(&brief)) {
            Ok(p) => {
                let abs = std::fs::canonicalize(&p).unwrap_or(p);
                println!(
                    "dashboard rendered from {} experiments, {} knowledge facts",
                    mgr.db.len(),
                    mgr.graph.len()
                );
                println!("open: {}", abs.display());
                println!("      file://{}", abs.display());
            }
            Err(e) => {
                eprintln!("failed to render dashboard: {e}");
                exit(1);
            }
        }
        return;
    }

    // --orchestrate <ticks> runs the Stage-8 Research Orchestrator: the full
    // life cycle (observe→analyze→learn→plan→run→evaluate→update→write-knowledge)
    // with the Theory Engine, Scientific Memory, and Foundation Dataset export.
    if let Some(ticks) = arg("--orchestrate").and_then(|s| s.parse::<usize>().ok()) {
        use ising_engine::engine_v2::ai_scientist::{OrchestratorConfig, ResearchOrchestrator};
        let mut orch = ResearchOrchestrator::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        let ocfg = OrchestratorConfig {
            campaign: base_cfg.clone(),
            // --planner lets the loop choose its own next instance by expected
            // new knowledge (autonomous task-setting) instead of round-robin.
            planner_driven: std::env::args().any(|a| a == "--planner"),
            // --executive makes the loop OBEY the Chief Scientist: it trains the
            // models the executive says are stale, investigates the theories it
            // flags, and ROUTES idea generation to the local/cloud LLM.
            executive_driven: std::env::args().any(|a| a == "--executive"),
            // Local tier the executive may route to: --llm, else auto-detect a
            // running Ollama server. Cloud tier: --cloud-model (honest skip
            // without an API key).
            local_llm: base_cfg
                .llm_model
                .clone()
                .or_else(ising_engine::engine_v2::ai_scientist::detect_local_llm),
            cloud_llm: base_cfg.cloud_model.clone(),
            ..Default::default()
        };
        if let Some(m) = &ocfg.local_llm {
            println!("local LLM available for the executive to route to: {m}");
        }
        println!(
            "Research Orchestrator: {ticks} lifecycle ticks over {} instance(s)\n",
            instances.len()
        );
        let reports = orch
            .run(&instances, &reg, &evolver, &executor, &ocfg, ticks)
            .unwrap_or_else(|e| {
                eprintln!("orchestrator failed: {e}");
                exit(1);
            });
        for r in &reports {
            println!("── tick {} [{}] ──", r.tick, r.instance);
            println!("  observe : {}", r.recall);
            for d in r.directives.iter().take(5) {
                println!("  chief   : {d}");
            }
            println!(
                "  run     : experiments {} → {}, best {:.1}",
                r.campaign.experiments_before, r.campaign.experiments_after, r.campaign.best_score
            );
            for t in &r.theories {
                println!("  theory  : {t}");
            }
            println!(
                "  write   : {} theories published, {} dataset rows, {}",
                r.theories_published, r.dataset_rows, r.memory
            );
        }
        println!(
            "\nplatform state: {} experiments, {} knowledge facts",
            orch.manager().db.len(),
            orch.manager().graph.len()
        );
        return;
    }

    // --service runs the orchestrator as a persistent, budget-capped loop (the
    // planner picks its own tasks). Caps: --max-ticks / --max-experiments /
    // --max-wall-secs (0 = uncapped on that axis). Interruptible & resumable —
    // everything persists after every tick.
    if std::env::args().any(|a| a == "--service") {
        use ising_engine::engine_v2::ai_scientist::{
            OrchestratorConfig, ResearchOrchestrator, ServiceBudget,
        };
        let mut orch = ResearchOrchestrator::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        let ocfg = OrchestratorConfig {
            campaign: base_cfg.clone(),
            // A service is autonomous by default: the planner sets its own tasks
            // unless --no-planner forces round-robin.
            planner_driven: !std::env::args().any(|a| a == "--no-planner"),
            executive_driven: std::env::args().any(|a| a == "--executive"),
            local_llm: base_cfg
                .llm_model
                .clone()
                .or_else(ising_engine::engine_v2::ai_scientist::detect_local_llm),
            cloud_llm: base_cfg.cloud_model.clone(),
            ..Default::default()
        };
        let budget = ServiceBudget {
            max_ticks: argn("--max-ticks", 0),
            max_experiments: argn("--max-experiments", 0),
            max_wall_secs: arg("--max-wall-secs")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        };
        if budget.max_ticks == 0 && budget.max_experiments == 0 && budget.max_wall_secs == 0 {
            eprintln!(
                "--service needs at least one cap: --max-ticks N / --max-experiments N / --max-wall-secs N"
            );
            exit(2);
        }
        println!(
            "Research service: budget ticks={} experiments={} wall_secs={} over {} instance(s)\n",
            budget.max_ticks,
            budget.max_experiments,
            budget.max_wall_secs,
            instances.len()
        );
        let reports = orch
            .run_service(&instances, &reg, &evolver, &executor, &ocfg, budget)
            .unwrap_or_else(|e| {
                eprintln!("service failed: {e}");
                exit(1);
            });
        println!(
            "service ran {} ticks; platform state: {} experiments, {} knowledge facts",
            reports.len(),
            orch.manager().db.len(),
            orch.manager().graph.len()
        );
        return;
    }

    // --investigate <operator> runs a MULTI-INSTANCE Theory-Engine ablation of
    // one operator across every loaded instance, aggregating the trials into a
    // single theory (Popperian confidence grows with cross-instance survival),
    // and publishes it (supported OR refuted — refutations are kept).
    if let Some(op) = arg("--investigate") {
        use ising_engine::engine_v2::ai_scientist::{
            ResearchOrchestrator, TheoryConfig, TheoryStatus,
        };
        let mut orch = ResearchOrchestrator::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        let nseeds = argn("--inv-seeds", 3).max(1) as u64;
        let seeds: Vec<u64> = (0..nseeds)
            .map(|i| base_cfg.base_seed.wrapping_add(i + 1))
            .collect();
        println!(
            "Investigating '{op}' across {} instance(s), {} seeds each…",
            instances.len(),
            seeds.len()
        );
        match orch.investigate_operator(&instances, &reg, &op, TheoryConfig::default(), &seeds) {
            Some(theory) => {
                println!("  status     : {:?}", theory.status);
                println!(
                    "  trials     : {} (survived {})",
                    theory.trials, theory.survived
                );
                println!("  confidence : {:.2}", theory.confidence);
                println!("  explanation: {}", theory.explanation);
                match theory.status {
                    TheoryStatus::Supported => {
                        println!("  → published: the mechanism survived ablation across instances")
                    }
                    _ => println!("  → recorded as a refutation (kept, not discarded)"),
                }
            }
            None => println!("  no theory produced (no instances?)"),
        }
        println!(
            "platform state: {} knowledge facts",
            orch.manager().graph.len()
        );
        return;
    }

    let mut mgr = CampaignManager::open(&dir, report_every).unwrap_or_else(|e| {
        eprintln!("cannot open platform dir {dir}: {e}");
        exit(1);
    });
    println!(
        "Research platform in {dir}: resuming with {} experiments, {} knowledge facts, {} reports",
        mgr.db.len(),
        mgr.graph.len(),
        mgr.archive.reports().len()
    );
    println!(
        "local scientist = {}, cloud scientist = {}, instances = {}\n",
        base_cfg.llm_model.as_deref().unwrap_or("heuristic"),
        base_cfg.cloud_model.as_deref().unwrap_or("disabled"),
        instances.len()
    );

    for (id, ir) in &instances {
        for c in 0..campaigns {
            let cfg = CampaignConfig {
                instance_id: id.clone(),
                ..base_cfg.clone()
            };
            let summary = mgr
                .run(ir, &reg, &evolver, &executor, &cfg)
                .unwrap_or_else(|e| {
                    eprintln!("campaign failed on {id}: {e}");
                    exit(1);
                });
            println!(
                "[{id}] campaign #{} ({} generations): experiments {} → {}",
                summary.campaign_id,
                summary.generations_run,
                summary.experiments_before,
                summary.experiments_after
            );
            println!(
                "  baseline {:.1} | best score {:.1} via {:?}",
                summary.baseline, summary.best_score, summary.best_schedule
            );
            for p in &summary.reports {
                println!("  report   : {}", p.display());
            }
            for p in &summary.analyses {
                println!("  analysis : {}", p.display());
            }
            for p in &summary.proposals {
                println!("  proposal : {}", p.display());
            }
            for line in &summary.consensus {
                if !line.trim().is_empty() {
                    println!("  meta     : {}", line.trim());
                }
            }
            for n in &summary.notes {
                println!("  note     : {n}");
            }
            if c + 1 < campaigns {
                println!();
            }
        }
    }
    // Self-audit over everything recorded so far: which rules reproduce across
    // instances, and whether the predictor transfers (honest "not yet" if the
    // history is too thin).
    match write_evaluation(std::path::Path::new(&dir), &mgr.db, &MetaLearner::new()) {
        Ok(p) => println!("\nevaluation: {}", p.display()),
        Err(e) => eprintln!("evaluation report failed: {e}"),
    }
    println!(
        "platform state persisted: {} experiments (append-only), {} knowledge facts",
        mgr.db.len(),
        mgr.graph.len()
    );
}
