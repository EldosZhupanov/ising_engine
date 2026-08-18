//! Research Orchestrator (Stage 8 — the heartbeat of the platform).
//!
//! Everything else is an organ; this is the life cycle that keeps them beating
//! together. One `tick` walks the full research loop the platform was designed
//! around:
//!
//!   loop {
//!     observe()             ← Scientific Memory: have we seen this structure?
//!     analyze() · learn()   ┐
//!     detect_uncertainty()  ├ CampaignManager: meta-learner rules, model
//!     plan()                │ training, Curiosity Engine, Meta-Layer consensus,
//!     generate() · run()    │ ideation → Evolution → Runtime → append-only DB
//!     evaluate()            │ → Statistician → knowledge graph
//!     update_all_models()   ┘
//!     write_knowledge()     ← Theory Engine (ablation → theory) + Foundation
//!                             Dataset export + Memory Manager index
//!     repeat()
//!   }
//!
//! The Orchestrator adds the stages the campaign alone does not: it OBSERVES
//! memory before acting, and after each campaign it WRITES KNOWLEDGE — it runs
//! the Theory Engine on the operators of the best schedule (ablating each to
//! test whether it is causal), publishes the surviving theories into the graph,
//! and re-exports the training corpus. It computes no energy and overrides
//! nothing; it is the conductor, not a player.

use super::super::decision::DecisionEngine;
use super::super::evolution::{Evolver, Schedule};
use super::super::ir::ProblemIR;
use super::super::registry::OperatorRegistry;
use super::campaign::{CampaignConfig, CampaignManager, CampaignSummary};
use super::dataset::FoundationDataset;
use super::executive::{Action, ExecutiveConfig, ResearchExecutive, ResourceState};
use super::executor::BatchExecutor;
use super::memory_os::{recall, MemoryManager};
use super::monitor::{HealthReport, Monitor, MonitorConfig};
use super::predictor::InstanceSignature;
use super::theory::{Theory, TheoryConfig, TheoryEngine, TheoryStatus};
use std::path::PathBuf;

/// What one lifecycle tick produced — one entry per stage, for the operator log.
#[derive(Debug, Default)]
pub struct TickReport {
    pub tick: usize,
    pub instance: String,
    /// observe(): what Scientific Memory recalled about this structure.
    pub recall: String,
    /// the campaign that ran the analyze→update stages.
    pub campaign: CampaignSummary,
    /// write_knowledge(): theory verdicts on the best schedule's operators.
    pub theories: Vec<String>,
    /// write_knowledge(): supported theories published into the graph.
    pub theories_published: usize,
    /// write_knowledge(): rows in the re-exported Foundation Dataset.
    pub dataset_rows: usize,
    /// learn(): concepts the Concept Discovery Engine admitted into the shared
    /// vocabulary this tick (the scientist expanding its own representation).
    pub concepts: Vec<String>,
    /// A one-line memory-manager health summary.
    pub memory: String,
    /// The Monitor's one-line health summary for this tick (gates the loop).
    pub health: String,
    /// The Chief Scientist's directives this tick (and, when executive-driven,
    /// what the loop actually did about them).
    pub directives: Vec<String>,
}

/// Configuration of the orchestrated loop.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub campaign: CampaignConfig,
    /// Run the Theory Engine on the best schedule each tick (write_knowledge).
    pub theory: bool,
    /// Re-export the Foundation Dataset each tick.
    pub export_dataset: bool,
    /// Memory recall radius (feature-distance) for the observe() stage.
    pub recall_radius: f64,
    /// Ablation replicas/threshold for the Theory Engine.
    pub theory_cfg: TheoryConfig,
    /// When true, the Research Planner CHOOSES the next instance each tick by
    /// expected new knowledge (the loop sets its own task) instead of cycling
    /// the instance list round-robin.
    pub planner_driven: bool,
    /// When true (default), the Monitor runs health gates each tick and PAUSES
    /// the loop if a hard invariant fails (e.g. the append-only DB is corrupted).
    /// Warnings are surfaced but never halt the loop.
    pub monitor: bool,
    /// When true, the loop OBEYS the Research Executive (Chief Scientist): each
    /// tick it consults the executive and ACTS on its directives — trains the
    /// models it says are stale, investigates the theories it flags, and ROUTES
    /// idea generation to the local or cloud LLM as it directs. The executive
    /// brief is always shown on the dashboard; this makes the loop follow it,
    /// turning the manager from an advisor into the driver.
    pub executive_driven: bool,
    /// Local LLM model (Ollama, e.g. `qwen2.5-coder:7b`) the executive may route
    /// idea generation to. `None` ⇒ no local tier available to the executive.
    pub local_llm: Option<String>,
    /// Cloud LLM model the executive may escalate to (rarely). `None` ⇒ no
    /// cloud tier.
    pub cloud_llm: Option<String>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            campaign: CampaignConfig::default(),
            theory: true,
            export_dataset: true,
            recall_radius: 0.15,
            theory_cfg: TheoryConfig::default(),
            monitor: true,
            planner_driven: false,
            executive_driven: false,
            local_llm: None,
            cloud_llm: None,
        }
    }
}

/// Probe a local Ollama server (localhost:11434) and return the first available
/// model name, or `None` if it is not running. Cheap, bounded, no dependency —
/// so the platform routes to a local LLM only when one is genuinely up.
pub fn detect_local_llm() -> Option<String> {
    let out = std::process::Command::new("curl")
        .args(["-s", "--max-time", "3", "http://localhost:11434/api/tags"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let i = s.find("\"name\":\"")? + 8;
    let rest = &s[i..];
    let j = rest.find('"')?;
    Some(rest[..j].to_string())
}

/// The conductor. Owns the persistent `CampaignManager` (and through it every
/// store) and drives the lifecycle.
pub struct ResearchOrchestrator {
    mgr: CampaignManager,
    dir: PathBuf,
    tick_count: usize,
    /// The executive's view of model freshness — watermarks advanced as the
    /// loop retrains, so its staleness judgments stay accurate across ticks.
    resources: ResourceState,
    /// Gate result of the most recent tick's Monitor pass. The health check runs
    /// ONCE per tick (it includes a leave-one-out predictor evaluation, which is
    /// not free); the run loops read this cached flag instead of re-running it.
    last_health_ok: bool,
}

impl ResearchOrchestrator {
    /// Open (or resume) the platform under `dir`.
    pub fn open(dir: impl Into<PathBuf>, report_every: usize) -> std::io::Result<Self> {
        let dir = dir.into();
        let mgr = CampaignManager::open(&dir, report_every)?;
        Ok(Self {
            mgr,
            dir,
            tick_count: 0,
            resources: ResourceState {
                local_available: true,
                cloud_available: std::env::var("ANTHROPIC_API_KEY").is_ok(),
                ..Default::default()
            },
            last_health_ok: true,
        })
    }

    pub fn manager(&self) -> &CampaignManager {
        &self.mgr
    }

    /// A cheap Monitor pass over the current stores — the health gates the loop
    /// consults each tick. `healthy()` is false iff a hard invariant failed.
    pub fn health(&self) -> HealthReport {
        Monitor::new(MonitorConfig::default()).check(&self.mgr.db, &self.mgr.graph)
    }

    /// One full lifecycle tick on one instance.
    pub fn tick(
        &mut self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        evolver: &Evolver,
        executor: &dyn BatchExecutor,
        cfg: &OrchestratorConfig,
    ) -> std::io::Result<TickReport> {
        self.tick_count += 1;
        let stats = DecisionEngine::analyze(ir);
        let sig = InstanceSignature {
            n: stats.n,
            density: stats.density,
            clustering: stats.clustering,
            mean_degree: stats.mean_degree,
            degree_cv: stats.degree_cv,
        };
        let mut report = TickReport {
            tick: self.tick_count,
            instance: cfg.campaign.instance_id.clone(),
            ..Default::default()
        };

        // ── observe(): Scientific Memory — have we seen this structure? ──
        report.recall = recall(&self.mgr.db, &self.mgr.graph, &sig, cfg.recall_radius).narrative;

        // ── the Chief Scientist directs the cycle: consult, then OBEY. ──
        // Availability reflects reality: a tier exists only if configured.
        self.resources.local_available = cfg.local_llm.is_some();
        self.resources.cloud_available =
            cfg.cloud_llm.is_some() || std::env::var("ANTHROPIC_API_KEY").is_ok();
        let brief = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &self.mgr.db,
            &self.mgr.graph,
            &self.resources,
            Some(&sig),
        );
        let mut campaign_cfg = cfg.campaign.clone();
        let mut investigate: Vec<String> = Vec::new();
        let (mut consult_local, mut consult_cloud) = (false, false);
        if cfg.executive_driven {
            // Start from a clean LLM slate; the executive routes idea generation.
            campaign_cfg.llm_model = None;
            campaign_cfg.cloud_model = None;
            for d in &brief.decisions {
                report
                    .directives
                    .push(format!("{} — {}", d.title(), d.reason));
                match &d.action {
                    // "retrain the World/Dynamics model" ⇒ turn on the shared-
                    // knowledge path this tick, which trains them.
                    Action::TrainModel { model } if model == "World" || model == "Dynamics" => {
                        campaign_cfg.shared_knowledge = true;
                    }
                    // "gather evidence for X" ⇒ queue X for the Theory Engine.
                    Action::GatherTheoryEvidence { operator }
                        if investigate.len() < 2 && !investigate.contains(operator) =>
                    {
                        investigate.push(operator.clone());
                    }
                    // Route idea generation to the tier the executive chose.
                    Action::ConsultLocalLLM => consult_local = true,
                    Action::ConsultCloudLLM => consult_cloud = true,
                    // Per-situation model selection: record which model the loop
                    // trusts for this regime this tick, provenance-stamped and
                    // append-only, so its accuracy can be scored over time.
                    Action::UseModel { model, regime } => {
                        self.mgr.graph.observe_if(
                            model,
                            "model-selected-in",
                            regime,
                            "",
                            1.0,
                            "executive",
                        );
                    }
                    _ => {}
                }
            }
            if consult_local {
                campaign_cfg.llm_model = cfg.local_llm.clone();
            }
            if consult_cloud {
                campaign_cfg.cloud_model = cfg.cloud_llm.clone();
            }
        }

        // ── analyze → update_all_models(): the campaign runs the bulk loop ──
        report.campaign = self
            .mgr
            .run(ir, registry, evolver, executor, &campaign_cfg)?;
        // The Predictor refits every run; the World/Dynamics models were (re)trained
        // iff the shared-knowledge path ran. Advance the executive's watermarks.
        self.resources.predictor_trained_at = self.mgr.db.len();
        if campaign_cfg.shared_knowledge {
            // The shared-knowledge path (re)trained Policy + World + Dynamics.
            self.resources.world_trained_at = self.mgr.db.len();
            self.resources.dynamics_trained_at = self.mgr.db.len();
            self.resources.policy_trained_at = self.mgr.db.len();
        }
        if consult_cloud {
            self.resources.last_cloud_consult_at = self.mgr.db.len();
        }

        // ── Discover Concepts (learn): the scientist EXPANDS ITS OWN VOCABULARY.
        // The Concept Discovery Engine tests candidate structural concepts against
        // the accumulated evidence (out-of-sample leave-one-out + Occam) and admits
        // the ones that improve the representation, publishing every verdict
        // (admitted AND rejected) to the graph. Admitted concepts extend the shared
        // FeatureRegistry, so faculties reading the live vocabulary (Scientific
        // Memory today; the models as they refit) get them automatically. ──
        {
            let base = (*super::feature_registry::current()).clone();
            let (grown, verdicts) = super::concept::discover(
                &self.mgr.db,
                base,
                &super::concept::default_candidates(),
                &super::concept::ConceptConfig::default(),
                &mut self.mgr.graph,
            );
            report.concepts = verdicts
                .iter()
                .filter(|v| v.admitted)
                .map(|v| v.name.clone())
                .collect();
            if grown.len() > super::feature_registry::current().len() {
                super::feature_registry::set_current(grown);
            }
        }

        // ── write_knowledge(): Theory Engine on the best schedule's operators,
        // plus any operator the Chief Scientist flagged for investigation. ──
        if cfg.theory && (!report.campaign.best_schedule.is_empty() || !investigate.is_empty()) {
            let condition = if stats.density < 0.05 {
                "density<0.05"
            } else {
                "density>=0.05"
            };
            let schedule = Schedule {
                sweeps: vec![16; report.campaign.best_schedule.len().max(1)],
                ops: report.campaign.best_schedule.clone(),
                temp_hi: 4.0,
                temp_lo: 0.1,
            };
            let engine = TheoryEngine::new(cfg.theory_cfg);
            let mut seen = std::collections::BTreeSet::new();
            let run_theory =
                |op: &str,
                 sched: &Schedule,
                 directed: bool,
                 report: &mut TickReport,
                 graph: &mut super::graph::KnowledgeGraph| {
                    let theory: Theory =
                        engine.explain(ir, registry, sched, op, condition, &[7, 8, 9]);
                    report.theories.push(format!(
                        "{op}{}: {:?} (conf {:.2}) — {}",
                        if directed {
                            " (executive-directed)"
                        } else {
                            ""
                        },
                        theory.status,
                        theory.confidence,
                        theory.explanation
                    ));
                    if theory.status != TheoryStatus::Hypothesis {
                        engine.publish(&theory, graph);
                        report.theories_published += 1;
                    }
                };
            for op in report.campaign.best_schedule.clone() {
                if seen.insert(op.clone()) {
                    run_theory(&op, &schedule, false, &mut report, &mut self.mgr.graph);
                }
            }
            // Executive-directed: does this dominant-but-unexplained operator do
            // causal work? Test it standalone (ablation ⇒ nothing).
            for op in &investigate {
                if seen.insert(op.clone()) {
                    let solo = Schedule {
                        ops: vec![op.clone()],
                        sweeps: vec![16],
                        temp_hi: 4.0,
                        temp_lo: 0.1,
                    };
                    run_theory(op, &solo, true, &mut report, &mut self.mgr.graph);
                }
            }
            // Persist the graph now that new theories landed in it.
            let _ = self.mgr.graph.save(self.dir.join("knowledge_graph.txt"));
        }

        // ── write_knowledge(): Memory Manager index + Foundation Dataset ──
        let mem = MemoryManager::default().analyze(&self.mgr.db);
        report.memory = format!(
            "{} experiments across {} regimes; {} compactable",
            mem.total_experiments,
            mem.buckets.len(),
            mem.buckets.iter().filter(|b| b.compactable).count()
        );
        // Monitor: health gates over the freshly-updated stores. Computed ONCE
        // here (not free — includes a predictor evaluation); the run loops read
        // the cached `last_health_ok` rather than re-running the pass.
        let health = self.health();
        report.health = health.summary();
        self.last_health_ok = health.healthy();
        if cfg.export_dataset {
            let ds = self.dir.join("dataset");
            report.dataset_rows = FoundationDataset::export(&self.mgr.db, &ds).unwrap_or(0);
            // Decision history: every graph fact with its provenance (which
            // agent/model decided it, and why) → the "reasons for action".
            let _ = FoundationDataset::export_decision_log(&self.mgr.graph, &ds);
            // Search-trajectory digest: how the ensemble MOVED under the best
            // schedule (best/mean energy, entropy, diversity, acceptance per
            // step) — the "solution search trajectories" corpus.
            if !report.campaign.best_schedule.is_empty() {
                let schedule = Schedule {
                    sweeps: vec![16; report.campaign.best_schedule.len()],
                    ops: report.campaign.best_schedule.clone(),
                    temp_hi: 4.0,
                    temp_lo: 0.1,
                };
                let tr = super::dynamics::capture_trajectory(ir, registry, &schedule, 16, 12345);
                let captures = vec![(
                    cfg.campaign.instance_id.clone(),
                    report.campaign.best_schedule.clone(),
                    tr,
                )];
                let _ = FoundationDataset::export_trajectories(&ds, &captures);
            }
        }

        // Refresh the dashboard with a FRESH executive brief (state changed this
        // tick), so the single pane shows the Chief Scientist's current directives.
        let brief_now = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &self.mgr.db,
            &self.mgr.graph,
            &self.resources,
            Some(&sig),
        );
        let _ = super::dashboard::write_dashboard_with(
            &self.dir,
            &self.mgr.db,
            &self.mgr.graph,
            &self.mgr.archive,
            &report.campaign.notes,
            Some(&brief_now),
        );

        Ok(report)
    }

    /// Choose the next instance to study: the Planner picks the least-known
    /// instance by expected new knowledge; otherwise cycle round-robin.
    fn choose_instance(
        &self,
        instances: &[(String, ProblemIR)],
        cfg: &OrchestratorConfig,
        tick: usize,
    ) -> usize {
        if cfg.planner_driven {
            let sigs: Vec<(String, InstanceSignature)> = instances
                .iter()
                .map(|(id, ir)| {
                    let s = DecisionEngine::analyze(ir);
                    (
                        id.clone(),
                        InstanceSignature {
                            n: s.n,
                            density: s.density,
                            clustering: s.clustering,
                            mean_degree: s.mean_degree,
                            degree_cv: s.degree_cv,
                        },
                    )
                })
                .collect();
            super::planner::ResearchPlanner::default()
                .next_target(&sigs, &self.mgr.db)
                .unwrap_or(tick % instances.len())
        } else {
            tick % instances.len()
        }
    }

    /// Run the lifecycle for `ticks` iterations, cycling through `instances`
    /// (each a `(instance_id, ir)`). This is the platform's main loop.
    pub fn run(
        &mut self,
        instances: &[(String, ProblemIR)],
        registry: &OperatorRegistry,
        evolver: &Evolver,
        executor: &dyn BatchExecutor,
        cfg: &OrchestratorConfig,
        ticks: usize,
    ) -> std::io::Result<Vec<TickReport>> {
        if instances.is_empty() {
            return Ok(Vec::new());
        }
        let mut reports = Vec::new();
        for t in 0..ticks {
            let chosen = self.choose_instance(instances, cfg, t);
            let (id, ir) = &instances[chosen];
            let mut tick_cfg = cfg.clone();
            tick_cfg.campaign.instance_id = id.clone();
            reports.push(self.tick(ir, registry, evolver, executor, &tick_cfg)?);

            // Monitor gate: if a hard invariant broke (e.g. the append-only DB
            // was corrupted), PAUSE the loop rather than compound the fault. The
            // pass already ran inside tick(); read its cached result.
            if cfg.monitor && !self.last_health_ok {
                let summary = reports.last().map(|r| r.health.clone()).unwrap_or_default();
                if let Some(last) = reports.last_mut() {
                    last.directives.push(format!("MONITOR HALT — {summary}"));
                }
                break;
            }
        }
        Ok(reports)
    }

    /// Run as a persistent SERVICE: keep ticking (planner-picking its own tasks)
    /// until any budget cap is hit — a tick count, an experiment count, or a wall
    /// clock. `0` on a field means "no cap on this axis"; a service with all
    /// three zero would run until the Monitor halts it or the agent cap trips, so
    /// callers should always set at least one. Everything persists after each
    /// tick (via the campaign), so the service is interruptible and resumable.
    ///
    /// The wall-clock cap is a STOPPING condition only — it never feeds any model
    /// input or seed, so per-experiment determinism (ADR-0004) is unaffected;
    /// only the NUMBER of ticks a run completes may vary with machine speed.
    pub fn run_service(
        &mut self,
        instances: &[(String, ProblemIR)],
        registry: &OperatorRegistry,
        evolver: &Evolver,
        executor: &dyn BatchExecutor,
        cfg: &OrchestratorConfig,
        budget: ServiceBudget,
    ) -> std::io::Result<Vec<TickReport>> {
        if instances.is_empty() {
            return Ok(Vec::new());
        }
        let mut reports = Vec::new();
        let start = std::time::Instant::now();
        let mut t = 0usize;
        loop {
            if budget.max_ticks != 0 && t >= budget.max_ticks {
                break;
            }
            if budget.max_experiments != 0 && self.mgr.db.len() >= budget.max_experiments {
                break;
            }
            if budget.max_wall_secs != 0 && start.elapsed().as_secs() >= budget.max_wall_secs {
                break;
            }
            let chosen = self.choose_instance(instances, cfg, t);
            let (id, ir) = &instances[chosen];
            let mut tick_cfg = cfg.clone();
            tick_cfg.campaign.instance_id = id.clone();
            reports.push(self.tick(ir, registry, evolver, executor, &tick_cfg)?);
            t += 1;

            // Health gate (cached from the tick's single Monitor pass).
            if cfg.monitor && !self.last_health_ok {
                let summary = reports.last().map(|r| r.health.clone()).unwrap_or_default();
                if let Some(last) = reports.last_mut() {
                    last.directives.push(format!("MONITOR HALT — {summary}"));
                }
                break;
            }
        }
        Ok(reports)
    }

    /// Multi-instance theory INVESTIGATION: run the Theory Engine's ablation of
    /// `operator` across EVERY given instance and aggregate the trials into one
    /// theory. This is how a mechanism earns real Popperian confidence — a
    /// single-instance ablation is weak, but surviving refutation on many
    /// instances is strong. The resulting theory (supported OR refuted) is
    /// published into the graph. Returns the aggregated theory.
    ///
    /// The operator is tested for its MARGINAL contribution inside a real
    /// explore→quench pipeline `[operator, quench]`, NOT in isolation. Ablating a
    /// solo operator leaves an EMPTY schedule that runs nothing, so *any*
    /// operator would look "causal" — a vacuous test. Ablating `operator` from
    /// `[operator, quench]` instead leaves `[quench]`, so the engine measures
    /// whether `operator` genuinely helps the following descent reach a lower
    /// energy (causal) or is a spurious passenger (the quench does the work).
    pub fn investigate_operator(
        &mut self,
        instances: &[(String, ProblemIR)],
        registry: &OperatorRegistry,
        operator: &str,
        theory_cfg: TheoryConfig,
        seeds: &[u64],
    ) -> Option<Theory> {
        if instances.is_empty() {
            return None;
        }
        let irs: Vec<&ProblemIR> = instances.iter().map(|(_, ir)| ir).collect();
        // Quench must differ from `operator` (the ablation removes ALL matching
        // tokens, so `[greedy, greedy]` would ablate to empty). Both descents are
        // in the standard registry.
        let quench = if operator == "greedy_descent" {
            "steepest_descent"
        } else {
            "greedy_descent"
        };
        let schedule = Schedule {
            ops: vec![operator.to_string(), quench.to_string()],
            sweeps: vec![16, 16],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        // Condition label from the first instance's regime band (most --family /
        // --file batches share a regime); honest and consistent with the buckets.
        let d0 = DecisionEngine::analyze(&instances[0].1).density;
        let condition = if d0 < 0.05 {
            "density<0.05"
        } else {
            "density>=0.05"
        };
        let engine = TheoryEngine::new(theory_cfg);
        let theory = engine.investigate(&irs, registry, &schedule, operator, condition, seeds)?;
        engine.publish(&theory, &mut self.mgr.graph);
        let _ = self.mgr.graph.save(self.dir.join("knowledge_graph.txt"));
        Some(theory)
    }
}

/// Budget caps for a persistent [`ResearchOrchestrator::run_service`] loop. A
/// field set to `0` imposes no cap on that axis.
#[derive(Debug, Clone, Copy, Default)]
pub struct ServiceBudget {
    pub max_ticks: usize,
    pub max_experiments: usize,
    pub max_wall_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::super::executor::RuntimeExecutor;
    use super::super::lab::LabConfig;
    use super::*;

    fn ring(n: usize) -> ProblemIR {
        let mut pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
            .map(|i| (i, i + 1, if i % 2 == 0 { -1.0 } else { 1.0 }))
            .collect();
        pairs.push((0, n as u32 - 1, -1.0));
        ProblemIR::from_pairs(n, 0.0, vec![0.0; n], &pairs)
    }

    fn small_orch_cfg() -> OrchestratorConfig {
        OrchestratorConfig {
            campaign: CampaignConfig {
                instance_id: "ring16".into(),
                generations: 1,
                lab: LabConfig {
                    rounds: 1,
                    hypotheses_per_round: 4,
                    batch_size: 8,
                    seeds_per_hypothesis: 2,
                    num_replicas: 8,
                    base_seed: 5,
                    ..Default::default()
                },
                report_every: 10,
                cloud_every: 1_000_000,
                predictor_oversample: 2,
                base_seed: 5,
                ..Default::default()
            },
            theory: true,
            export_dataset: true,
            recall_radius: 0.2,
            theory_cfg: TheoryConfig {
                num_replicas: 8,
                min_degradation: 0.005,
            },
            monitor: true,
            planner_driven: false,
            executive_driven: false,
            local_llm: None,
            cloud_llm: None,
        }
    }

    #[test]
    fn one_tick_runs_the_whole_lifecycle() {
        let dir = std::env::temp_dir().join(format!("orch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ir = ring(16);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        let mut orch = ResearchOrchestrator::open(&dir, 10).unwrap();
        let insts = vec![("ring16".to_string(), ir)];
        let reports = orch
            .run(&insts, &reg, &evolver, &exec, &small_orch_cfg(), 2)
            .unwrap();

        assert_eq!(reports.len(), 2);
        // Tick 1: memory is empty → recall says the regime is new.
        assert!(
            reports[0].recall.contains("new") || reports[0].recall.contains("similar"),
            "recall narrative: {}",
            reports[0].recall
        );
        // The campaign ran and produced experiments.
        assert!(reports[1].campaign.experiments_after > reports[0].campaign.experiments_before);
        // write_knowledge produced theory verdicts on the best schedule.
        assert!(
            !reports[1].theories.is_empty(),
            "theories should be written: {:?}",
            reports[1].theories
        );
        // The Foundation Dataset was exported.
        assert!(reports[1].dataset_rows > 0);
        assert!(std::path::Path::new(&dir)
            .join("dataset/foundation_dataset.tsv")
            .exists());
        // Tick 2: memory now recalls the (same) instance seen in tick 1.
        assert!(
            reports[1].recall.contains("similar"),
            "second tick should recall memory: {}",
            reports[1].recall
        );
        assert!(reports[1].memory.contains("experiments"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn planner_driven_loop_studies_the_least_known_instance() {
        let dir = std::env::temp_dir().join(format!("orch_plan_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        let mut cfg = small_orch_cfg();
        cfg.planner_driven = true;
        cfg.theory = false; // keep the test fast; the loop mechanics are the point

        // Two distinct instances; the planner should spread attention across
        // both (after studying one, the other becomes the least-known).
        let insts = vec![
            ("ringA".to_string(), ring(16)),
            ("ringB".to_string(), ring(20)),
        ];
        let mut orch = ResearchOrchestrator::open(&dir, 10).unwrap();
        let reports = orch.run(&insts, &reg, &evolver, &exec, &cfg, 2).unwrap();

        assert_eq!(reports.len(), 2);
        let studied: std::collections::BTreeSet<&str> = orch
            .manager()
            .db
            .all()
            .iter()
            .map(|r| r.instance_id.as_str())
            .collect();
        assert!(
            studied.contains("ringA") && studied.contains("ringB"),
            "planner should have studied BOTH instances, got {studied:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn service_loop_stops_at_the_experiment_budget() {
        let dir = std::env::temp_dir().join(format!("orch_svc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        let mut cfg = small_orch_cfg();
        cfg.theory = false; // fast; the budget mechanics are the point

        let insts = vec![("ring16".to_string(), ring(16))];
        let mut orch = ResearchOrchestrator::open(&dir, 10).unwrap();
        // Cap by experiments: stop once the DB reaches >= 20 records.
        let budget = ServiceBudget {
            max_experiments: 20,
            ..Default::default()
        };
        let reports = orch
            .run_service(&insts, &reg, &evolver, &exec, &cfg, budget)
            .unwrap();
        assert!(
            !reports.is_empty(),
            "the service must run at least one tick"
        );
        assert!(
            orch.manager().db.len() >= 20,
            "service must run until the experiment budget is met: {}",
            orch.manager().db.len()
        );
        // It must STOP shortly after crossing the budget, not run forever.
        assert!(
            reports.len() <= 20,
            "service ran too long past its budget: {} ticks",
            reports.len()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_instance_set_does_not_panic() {
        let dir = std::env::temp_dir().join(format!("orch_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(1);
        let cfg = small_orch_cfg();
        let mut orch = ResearchOrchestrator::open(&dir, 10).unwrap();
        // No instances: both loops must return empty, not divide-by-zero panic.
        let r1 = orch.run(&[], &reg, &evolver, &exec, &cfg, 3).unwrap();
        let budget = ServiceBudget {
            max_ticks: 3,
            ..Default::default()
        };
        let r2 = orch
            .run_service(&[], &reg, &evolver, &exec, &cfg, budget)
            .unwrap();
        assert!(r1.is_empty() && r2.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn multi_instance_investigate_aggregates_trials_across_instances() {
        let dir = std::env::temp_dir().join(format!("orch_inv_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let reg = OperatorRegistry::standard();

        let insts = vec![
            ("ringA".to_string(), ring(16)),
            ("ringB".to_string(), ring(20)),
            ("ringC".to_string(), ring(24)),
        ];
        let mut orch = ResearchOrchestrator::open(&dir, 10).unwrap();
        let tcfg = TheoryConfig {
            num_replicas: 8,
            min_degradation: 0.005,
        };
        let theory = orch
            .investigate_operator(&insts, &reg, "metropolis_sweep", tcfg, &[7, 8])
            .expect("a theory should be produced across instances");
        // Aggregation: trials span all three instances × two seeds each, so more
        // than a single-instance ablation would give — that is what earns
        // Popperian confidence.
        assert!(
            theory.trials >= 3,
            "multi-instance investigate must aggregate trials: {}",
            theory.trials
        );
        // The theory (supported or refuted) was published to the graph.
        assert!(
            orch.manager()
                .graph
                .triples()
                .iter()
                .any(|t| t.subject == "metropolis_sweep"),
            "the investigated theory must be published"
        );

        // Edge case: investigating greedy_descent itself must NOT panic or
        // produce a vacuous vs-nothing test. The quench falls back to
        // steepest_descent (differs from the ablated op), so the ablated schedule
        // is [steepest_descent] — non-empty — and the marginal test is real.
        let g = orch
            .investigate_operator(&insts, &reg, "greedy_descent", tcfg, &[7, 8])
            .expect("greedy_descent investigation should also produce a theory");
        assert!(g.trials >= 3, "aggregation must hold for the quench op too");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn executive_driven_loop_obeys_the_chief_scientist() {
        let dir = std::env::temp_dir().join(format!("orch_exec_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        let mut cfg = small_orch_cfg();
        cfg.executive_driven = true;

        let ir = ring(16);
        let insts = vec![("ring16".to_string(), ir)];
        let mut orch = ResearchOrchestrator::open(&dir, 10).unwrap();
        // First tick seeds the DB; second tick the executive has real state to
        // direct (stale models, saturation, unexplained operators).
        let reports = orch.run(&insts, &reg, &evolver, &exec, &cfg, 2).unwrap();

        // The Chief Scientist issued directives and they were recorded.
        assert!(
            !reports[1].directives.is_empty(),
            "the executive must produce directives when executive-driven"
        );
        // Obedience: the loop advanced the model watermarks (it retrained), so
        // the executive's staleness view stays current — no infinite "retrain"
        // loop. The dashboard was refreshed with the brief.
        assert!(std::path::Path::new(&dir).join("dashboard.html").exists());
        let html = std::fs::read_to_string(dir.join("dashboard.html")).unwrap();
        assert!(
            html.contains("Chief Scientist"),
            "dashboard must show the executive"
        );
        // Valid results throughout.
        assert!(reports[1].campaign.best_score <= reports[1].campaign.baseline + 1e-9);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
