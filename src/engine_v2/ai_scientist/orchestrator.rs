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
use super::executor::BatchExecutor;
use super::memory_os::{recall, MemoryManager};
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
    /// A one-line memory-manager health summary.
    pub memory: String,
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
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            campaign: CampaignConfig::default(),
            theory: true,
            export_dataset: true,
            recall_radius: 0.15,
            theory_cfg: TheoryConfig::default(),
            planner_driven: false,
        }
    }
}

/// The conductor. Owns the persistent `CampaignManager` (and through it every
/// store) and drives the lifecycle.
pub struct ResearchOrchestrator {
    mgr: CampaignManager,
    dir: PathBuf,
    tick_count: usize,
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
        })
    }

    pub fn manager(&self) -> &CampaignManager {
        &self.mgr
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

        // ── analyze → update_all_models(): the campaign runs the bulk loop ──
        report.campaign = self
            .mgr
            .run(ir, registry, evolver, executor, &cfg.campaign)?;

        // ── write_knowledge(): Theory Engine on the best schedule's operators ──
        if cfg.theory && !report.campaign.best_schedule.is_empty() {
            let condition = if stats.density < 0.05 {
                "density<0.05"
            } else {
                "density>=0.05"
            };
            let schedule = Schedule {
                sweeps: vec![16; report.campaign.best_schedule.len()],
                ops: report.campaign.best_schedule.clone(),
                temp_hi: 4.0,
                temp_lo: 0.1,
            };
            let engine = TheoryEngine::new(cfg.theory_cfg);
            let mut seen = std::collections::BTreeSet::new();
            for op in &report.campaign.best_schedule {
                if !seen.insert(op.clone()) {
                    continue; // one theory per distinct operator
                }
                let theory: Theory =
                    engine.explain(ir, registry, &schedule, op, condition, &[7, 8, 9]);
                report.theories.push(format!(
                    "{op}: {:?} (conf {:.2}) — {}",
                    theory.status, theory.confidence, theory.hypothesis.claim
                ));
                if theory.status != TheoryStatus::Hypothesis {
                    engine.publish(&theory, &mut self.mgr.graph);
                    report.theories_published += 1;
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
        if cfg.export_dataset {
            report.dataset_rows =
                FoundationDataset::export(&self.mgr.db, self.dir.join("dataset")).unwrap_or(0);
        }

        Ok(report)
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
        let mut reports = Vec::new();
        for t in 0..ticks {
            // The loop sets its own task: the Planner picks the least-known
            // instance by expected new knowledge; otherwise cycle round-robin.
            let chosen = if cfg.planner_driven {
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
                    .unwrap_or(t % instances.len())
            } else {
                t % instances.len()
            };
            let (id, ir) = &instances[chosen];
            let mut tick_cfg = cfg.clone();
            tick_cfg.campaign.instance_id = id.clone();
            reports.push(self.tick(ir, registry, evolver, executor, &tick_cfg)?);
        }
        Ok(reports)
    }
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
            planner_driven: false,
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
}
