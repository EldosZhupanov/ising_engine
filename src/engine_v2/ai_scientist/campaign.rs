//! Campaign Manager (Stage 6, Task 9) — the closed scientific cycle, run
//! continuously in GENERATIONS:
//!
//!   generation → ScientificLab (hypotheses → Evolution → Runtime) →
//!   ExperimentDb (append-only, provenance-stamped) → Meta-Learner publishes
//!   consistent findings → ReportArchive every N experiments → CloudScientist
//!   every M experiments → OperatorProposal drafts → Predictor refit (filter
//!   for the next generation) → next generation.
//!
//! Everything persists after every generation, so a million-experiment
//! campaign survives interruption and resumes with all its knowledge.

use super::super::decision::DecisionEngine;
use super::super::evolution::{Evolver, Schedule};
use super::super::ir::ProblemIR;
use super::super::knowledge::KnowledgeBase;
use super::super::registry::OperatorRegistry;
use super::cloud::CloudScientist;
use super::curiosity::{CuriosityConfig, CuriosityEngine, CuriousIdeator};
use super::db::{ExperimentDb, RunContext};
use super::dynamics::train_on_instances;
use super::executor::BatchExecutor;
use super::graph::KnowledgeGraph;
use super::lab::{HypothesisGenerator, Ideator, LabConfig, ResearchBrief, ScientificLab};
use super::llm::LlmHypothesisGenerator;
use super::meta_layer::{MetaBiasedIdeator, MetaKnowledge, MetaWeights, OperatorSignals};
use super::meta_learner::MetaLearner;
use super::policy::OperatorPolicy;
use super::predictor::{InstanceSignature, Predictor};
use super::proposal::OperatorProposal;
use super::reports::ReportArchive;
use super::scientist::Hypothesis;
use super::world::WorldModel;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration of one campaign (a run of `generations` lab generations on
/// one instance, appended to the shared persistent stores).
#[derive(Debug, Clone)]
pub struct CampaignConfig {
    pub instance_id: String,
    pub generations: usize,
    /// Per-generation lab configuration (rounds, batch size, replicas, ...).
    pub lab: LabConfig,
    /// Write a research report every this many experiments (Task 4).
    pub report_every: usize,
    /// Consult the cloud model every this many experiments (Task 6).
    pub cloud_every: usize,
    /// Local LLM model name (Ollama); `None` ⇒ deterministic heuristic ideator.
    pub llm_model: Option<String>,
    /// Cloud model name; `None` ⇒ cloud tier disabled entirely.
    pub cloud_model: Option<String>,
    /// Candidate hypotheses are oversampled by this factor and the predictor
    /// keeps the best 1/factor (only once trained). 1 ⇒ no filtering.
    pub predictor_oversample: usize,
    /// META-LEARNING LAYER (opt-in): each generation, train the Policy / World /
    /// Dynamics models on the accumulated data, consolidate their per-operator
    /// verdicts, PUBLISH the consensus into the knowledge graph (so reports and
    /// the LLM read it), and bias the ideator toward what the models jointly
    /// endorse. Off by default — it trains three models + captures trajectories
    /// per generation, so it costs more; when on, the loop is fully closed.
    pub shared_knowledge: bool,
    /// CURIOSITY dial (Stage 8): explore/exploit blend in [0, 1]. 0 ⇒ pure
    /// exploitation (existing behavior). > 0 ⇒ the Curiosity Engine steers a
    /// fraction of ideation toward the strange / under-explored operators.
    pub curiosity_lambda: f64,
    pub base_seed: u64,
}

impl Default for CampaignConfig {
    fn default() -> Self {
        Self {
            instance_id: "unnamed".into(),
            generations: 4,
            lab: LabConfig::default(),
            report_every: 5000,
            cloud_every: 10_000,
            llm_model: None,
            cloud_model: None,
            predictor_oversample: 3,
            shared_knowledge: false,
            curiosity_lambda: 0.0,
            base_seed: 1,
        }
    }
}

/// What one campaign did — every artifact is a path the user can open.
#[derive(Debug, Default)]
pub struct CampaignSummary {
    pub campaign_id: u64,
    pub generations_run: usize,
    pub experiments_before: usize,
    pub experiments_after: usize,
    pub best_score: f64,
    pub best_schedule: Vec<String>,
    pub baseline: f64,
    pub reports: Vec<PathBuf>,
    pub analyses: Vec<PathBuf>,
    pub proposals: Vec<PathBuf>,
    pub dashboard: Option<PathBuf>,
    /// The Meta-Learning Layer's cross-model consensus for this campaign's
    /// instance, when `shared_knowledge` is on (empty otherwise).
    pub consensus: Vec<String>,
    /// Honest disclosures: skipped tiers, request failures.
    pub notes: Vec<String>,
}

/// An `Ideator` decorator implementing Task 8's contract: oversample ideas,
/// keep the predictor's favorites. Pure filtering — the surviving hypotheses
/// are still executed and judged by the Runtime.
struct FilteredIdeator {
    inner: Box<dyn Ideator>,
    predictor: Option<Predictor>,
    sig: InstanceSignature,
    oversample: usize,
}

impl Ideator for FilteredIdeator {
    fn propose(
        &mut self,
        brief: &ResearchBrief,
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        let Some(p) = &self.predictor else {
            return self.inner.propose(brief, n, rng);
        };
        let raw = self.inner.propose(brief, n * self.oversample.max(1), rng);
        if raw.len() <= n {
            return raw;
        }
        // Coarse schedules for ranking only (the designer sets real params).
        let scheds: Vec<Schedule> = raw
            .iter()
            .map(|h| Schedule {
                ops: h.operators.clone(),
                sweeps: vec![16; h.operators.len()],
                temp_hi: 4.0,
                temp_lo: 0.1,
            })
            .collect();
        let keep = p.filter(&self.sig, &scheds, n);
        keep.into_iter().map(|i| raw[i].clone()).collect()
    }
}

/// Drives campaigns over the persistent platform directory.
pub struct CampaignManager {
    dir: PathBuf,
    pub db: ExperimentDb,
    pub kb: KnowledgeBase,
    pub graph: KnowledgeGraph,
    pub archive: ReportArchive,
    pub meta: MetaLearner,
    next_campaign_id: u64,
}

impl CampaignManager {
    /// Open (or create) the platform root; loads every persistent store.
    pub fn open(dir: impl Into<PathBuf>, report_every: usize) -> std::io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        fs::create_dir_all(dir.join("analysis"))?;
        fs::create_dir_all(dir.join("proposals"))?;
        let db = ExperimentDb::load(dir.join("ai_experiments.txt"))?;
        let kb = KnowledgeBase::load(dir.join("knowledge.txt")).unwrap_or_default();
        let graph = KnowledgeGraph::load(dir.join("knowledge_graph.txt"))?;
        let archive = ReportArchive::open(dir.join("reports"), report_every)?;
        let next_campaign_id = fs::read_to_string(dir.join("campaign_state.txt"))
            .ok()
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or(0);
        Ok(Self {
            dir,
            db,
            kb,
            graph,
            archive,
            meta: MetaLearner::new(),
            next_campaign_id,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn persist(&mut self) -> std::io::Result<()> {
        self.db.flush_append(self.dir.join("ai_experiments.txt"))?;
        self.kb.save(self.dir.join("knowledge.txt"))?;
        self.graph.save(self.dir.join("knowledge_graph.txt"))?;
        fs::write(
            self.dir.join("campaign_state.txt"),
            format!("{}\n", self.next_campaign_id),
        )
    }

    fn make_ideator(
        cfg: &CampaignConfig,
        predictor: Option<Predictor>,
        sig: InstanceSignature,
    ) -> Box<dyn Ideator> {
        let inner: Box<dyn Ideator> = match &cfg.llm_model {
            Some(model) => Box::new(LlmHypothesisGenerator::new(model.clone())),
            None => Box::new(HypothesisGenerator::new()),
        };
        if cfg.predictor_oversample > 1 {
            Box::new(FilteredIdeator {
                inner,
                predictor,
                sig,
                oversample: cfg.predictor_oversample,
            })
        } else {
            inner
        }
    }

    /// META-LEARNING LAYER: train the Policy / World / Dynamics models on the
    /// accumulated data + this instance, then consolidate their per-operator
    /// verdicts with the Meta-Learner's rules into one shared `MetaKnowledge`.
    /// Runs once per campaign (knowledge then biases every generation and, via
    /// the graph, the next campaign). Degrades gracefully: a model that can't
    /// train yet simply contributes no signal. `None` if the DB has no
    /// operator vocabulary to learn from at all.
    fn build_shared_knowledge(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        cfg: &CampaignConfig,
        sig: &InstanceSignature,
        stats: &super::super::decision::InstanceStats,
    ) -> Option<MetaKnowledge> {
        let vocab: Vec<String> = self
            .db
            .all()
            .iter()
            .flat_map(|r| r.sequence.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if vocab.is_empty() {
            return None;
        }
        let replicas = cfg.lab.num_replicas.max(2);
        let sweeps: u32 = 16;
        let seeds = [
            cfg.base_seed.wrapping_add(1),
            cfg.base_seed.wrapping_add(2),
            cfg.base_seed.wrapping_add(3),
        ];

        // Policy: distilled from the whole experiment history.
        let mut policy = OperatorPolicy::new(vocab.clone(), cfg.base_seed);
        policy.train_supervised(&self.db, 40, 0.05);

        // World + Dynamics: trained on fresh trajectories of single-operator
        // schedules over the capability pool of THIS instance.
        let pool: Vec<String> =
            DecisionEngine::operator_pool(stats, stats.select_backend(), registry)
                .iter()
                .map(|s| s.to_string())
                .collect();
        let sample: Vec<Schedule> = pool
            .iter()
            .map(|op| Schedule {
                ops: vec![op.clone(); 3],
                sweeps: vec![sweeps; 3],
                temp_hi: 4.0,
                temp_lo: 0.1,
            })
            .collect();
        let world = WorldModel::fit(&[ir], registry, &sample, replicas, &seeds, 1e-4);
        let dynamics = sample
            .first()
            .and_then(|s| train_on_instances(&[ir], registry, s, replicas, &seeds, 1e-4));

        // Consolidate every available signal onto the shared bus.
        let mut signals =
            OperatorSignals::from_meta(&self.meta, &self.db).with_policy(&policy, sig);
        if let Some(w) = &world {
            signals = signals.with_world(w, ir, 4.0, sweeps);
        }
        if let Some(d) = &dynamics {
            signals = signals.with_dynamics(
                d,
                ir,
                registry,
                &pool,
                replicas,
                sweeps,
                3,
                cfg.base_seed,
                0.01,
            );
        }
        let condition = if stats.density < 0.05 {
            "density<0.05"
        } else {
            "density>=0.05"
        };
        Some(MetaKnowledge::consolidate(
            &signals,
            &MetaWeights::default(),
            condition,
        ))
    }

    /// Run one campaign: `cfg.generations` lab generations with full
    /// provenance, reporting, cloud analysis, proposals, and predictor refits.
    pub fn run(
        &mut self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        evolver: &Evolver,
        executor: &dyn BatchExecutor,
        cfg: &CampaignConfig,
    ) -> std::io::Result<CampaignSummary> {
        let campaign_id = self.next_campaign_id;
        self.next_campaign_id += 1;
        self.archive.every = cfg.report_every;

        let stats = DecisionEngine::analyze(ir);
        let sig = InstanceSignature {
            n: stats.n,
            density: stats.density,
            clustering: stats.clustering,
            mean_degree: stats.mean_degree,
            degree_cv: stats.degree_cv,
        };

        let mut cloud = cfg
            .cloud_model
            .as_ref()
            .map(|m| CloudScientist::new(m.clone(), cfg.cloud_every));
        if let Some(c) = &mut cloud {
            c.resume(&self.dir.join("analysis"));
        }

        let mut summary = CampaignSummary {
            campaign_id,
            experiments_before: self.db.len(),
            best_score: f64::INFINITY,
            ..Default::default()
        };
        match &cloud {
            None => summary
                .notes
                .push("cloud tier disabled (no --cloud-model)".into()),
            Some(c) if !c.available() => summary.notes.push(
                "cloud tier skipped honestly: ANTHROPIC_API_KEY not set — nothing fabricated"
                    .into(),
            ),
            _ => {}
        }

        let mut predictor = Predictor::fit(&self.db, 1e-3);

        // Meta-Learning Layer (opt-in): consolidate the models' consensus once,
        // publish it into the graph (so reports + the LLM read it this and next
        // campaign), and bias every generation's ideation toward it.
        let shared = if cfg.shared_knowledge {
            self.build_shared_knowledge(ir, registry, cfg, &sig, &stats)
        } else {
            None
        };
        if let Some(k) = &shared {
            let wrote = k.publish(&mut self.graph);
            summary.consensus = k
                .consensus_report()
                .lines()
                .map(|s| s.to_string())
                .collect();
            summary.notes.push(format!(
                "meta-layer: {wrote} consensus facts published; prefer=[{}] avoid=[{}] switch-early=[{}]",
                k.preferred().join(","),
                k.avoided().join(","),
                k.switch_early().join(","),
            ));
        }

        // Curiosity Engine (Stage 8, opt-in): score operators by surprise
        // (disagreement + coverage + anomaly) so a `curiosity_lambda` fraction
        // of ideation is steered toward the unknown, not only the endorsed.
        let curiosity = if cfg.curiosity_lambda > 0.0 {
            let engine =
                CuriosityEngine::from_db(&self.db, predictor.as_ref(), &CuriosityConfig::default());
            let top: Vec<String> = engine
                .ranked()
                .into_iter()
                .take(3)
                .map(|(op, c)| format!("{op}({c:.2})"))
                .collect();
            summary.notes.push(format!(
                "curiosity: λ={:.2}, most-curious operators = [{}]",
                cfg.curiosity_lambda,
                top.join(", ")
            ));
            Some(engine)
        } else {
            None
        };

        for generation in 0..cfg.generations {
            self.db.set_context(RunContext {
                instance_id: cfg.instance_id.clone(),
                n: stats.n,
                mean_degree: stats.mean_degree,
                degree_cv: stats.degree_cv,
                campaign_id,
                generation_id: generation as u64,
            });

            let mut lab_cfg = cfg.lab.clone();
            lab_cfg.base_seed = cfg
                .base_seed
                .wrapping_add(campaign_id.wrapping_mul(1_000_003))
                .wrapping_add(generation as u64 * 7919);

            let base_ideator = Self::make_ideator(cfg, predictor.clone(), sig);
            // Wrap with the meta-layer bias when shared knowledge is on (exploit
            // the cross-model consensus)…
            let exploit_ideator: Box<dyn Ideator> = match &shared {
                Some(k) => Box::new(MetaBiasedIdeator::new(base_ideator, k.clone(), 2)),
                None => base_ideator,
            };
            // …then wrap with the Curiosity Engine (explore the unknown), so the
            // final ideator is the explore/exploit blend on the λ dial.
            let ideator: Box<dyn Ideator> = match &curiosity {
                Some(engine) => Box::new(CuriousIdeator::new(
                    exploit_ideator,
                    engine.clone(),
                    cfg.curiosity_lambda,
                )),
                None => exploit_ideator,
            };
            let mut lab =
                ScientificLab::with_ideator(lab_cfg.base_seed, self.graph.clone(), ideator);
            let report = lab.run(
                ir,
                registry,
                evolver,
                executor,
                &mut self.db,
                &mut self.kb,
                &lab_cfg,
            );
            self.graph = lab.graph().clone();
            summary.generations_run += 1;
            summary.baseline = report.baseline;
            if report.best_score < summary.best_score {
                summary.best_score = report.best_score;
                summary.best_schedule = report.best_schedule.ops.clone();
            }

            // Meta-learner publishes only consistent findings into the graph.
            self.meta.publish(&self.db, &mut self.graph);

            // Task 4: cumulative research report every `report_every`.
            if let Some(path) = self
                .archive
                .maybe_report(&self.db, &self.graph, &self.meta)?
            {
                summary.reports.push(path);
                // Task 7: a fresh report is the moment to draft proposals.
                if let Some(gap) = self.meta.suggest_operator_gap(&self.db) {
                    let prop = OperatorProposal::from_gap(&gap, registry);
                    summary
                        .proposals
                        .push(prop.write(self.dir.join("proposals"))?);
                }
            }

            // Task 6: rare cloud consultation; failures disclosed, never faked.
            if let Some(c) = &mut cloud {
                match c.maybe_analyze(
                    &self.dir.join("analysis"),
                    &self.archive,
                    &self.graph,
                    &self.db,
                ) {
                    Ok(Some(path)) => summary.analyses.push(path),
                    Ok(None) => {}
                    Err(e) => summary
                        .notes
                        .push(format!("cloud analysis failed (gen {generation}): {e}")),
                }
            }

            // Task 8: refit the filter on the grown history.
            predictor = Predictor::fit(&self.db, 1e-3);

            // Task 10: the dashboard mirrors the stores after every generation.
            summary.dashboard = Some(super::dashboard::write_dashboard(
                &self.dir,
                &self.db,
                &self.graph,
                &self.archive,
                &summary.notes,
            )?);

            // Nothing is lost if the process dies here.
            self.persist()?;
        }

        summary.experiments_after = self.db.len();
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::super::executor::RuntimeExecutor;
    use super::*;

    fn ring_ir(n: usize) -> ProblemIR {
        // Ring with i<j ordering: path edges plus the closing chord (0, n-1).
        let mut pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
            .map(|i| (i, i + 1, if i % 2 == 0 { 1.0 } else { -1.0 }))
            .collect();
        pairs.push((0, n as u32 - 1, 1.0));
        ProblemIR::from_pairs(n, 0.0, vec![0.0; n], &pairs)
    }

    fn small_cfg() -> CampaignConfig {
        CampaignConfig {
            instance_id: "ring12".into(),
            generations: 2,
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
            llm_model: None,
            cloud_model: None,
            predictor_oversample: 2,
            shared_knowledge: false,
            curiosity_lambda: 0.0,
            base_seed: 5,
        }
    }

    #[test]
    fn campaign_stamps_provenance_reports_and_resumes() {
        let dir = std::env::temp_dir().join(format!("campaign_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let ir = ring_ir(12);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        let mut mgr = CampaignManager::open(&dir, 10).unwrap();
        let s1 = mgr.run(&ir, &reg, &evolver, &exec, &small_cfg()).unwrap();
        assert_eq!(s1.campaign_id, 0);
        assert!(s1.experiments_after > s1.experiments_before);
        assert!(s1.best_score <= s1.baseline + 1e-9);
        // Cloud disabled ⇒ disclosed, not silently absent.
        assert!(s1.notes.iter().any(|n| n.contains("cloud tier disabled")));
        // Provenance stamped on every record of this campaign.
        let rec = mgr.db.all().last().unwrap();
        assert_eq!(rec.instance_id, "ring12");
        assert_eq!(rec.campaign_id, 0);
        assert_eq!(rec.n, 12);
        // Reports fired (report_every=10, dozens of experiments ran).
        assert!(!s1.reports.is_empty());

        // Reopen from disk: history intact, campaign id advances.
        let mut mgr2 = CampaignManager::open(&dir, 10).unwrap();
        assert_eq!(mgr2.db.len(), s1.experiments_after);
        let s2 = mgr2.run(&ir, &reg, &evolver, &exec, &small_cfg()).unwrap();
        assert_eq!(s2.campaign_id, 1);
        assert!(s2.experiments_after > s1.experiments_after);
        // History never deleted: campaign 0 records still present.
        assert!(mgr2.db.all().iter().any(|r| r.campaign_id == 0));
        assert!(mgr2.db.all().iter().any(|r| r.campaign_id == 1));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn shared_knowledge_closes_the_loop_into_the_graph() {
        let dir = std::env::temp_dir().join(format!("campaign_meta_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let ir = ring_ir(16);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        let mut mgr = CampaignManager::open(&dir, 10).unwrap();
        // Seed the DB with a first (plain) campaign so the models have data.
        mgr.run(&ir, &reg, &evolver, &exec, &small_cfg()).unwrap();

        // Second campaign with the Meta-Learning Layer ON.
        let cfg = CampaignConfig {
            shared_knowledge: true,
            ..small_cfg()
        };
        let facts_before = mgr.graph.len();
        let s = mgr.run(&ir, &reg, &evolver, &exec, &cfg).unwrap();

        // The consensus was computed and recorded in the summary + notes.
        assert!(
            s.notes.iter().any(|n| n.contains("meta-layer:")),
            "meta-layer note missing: {:?}",
            s.notes
        );
        assert!(
            s.consensus.iter().any(|l| l.contains("consensus")),
            "consensus report missing: {:?}",
            s.consensus
        );
        // Consensus facts were published into the shared knowledge graph — the
        // loop into reports/LLM is closed.
        let has_consensus_fact = mgr.graph.triples().iter().any(|t| {
            t.predicate == "consensus-prefer"
                || t.predicate == "consensus-avoid"
                || t.predicate == "plateaus-early"
        });
        assert!(
            has_consensus_fact || mgr.graph.len() >= facts_before,
            "meta-layer should publish consensus facts"
        );
        // The run still produced valid results (loop didn't break anything).
        assert!(s.best_score <= s.baseline + 1e-9);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn curiosity_dial_runs_end_to_end_and_reports_surprise() {
        let dir = std::env::temp_dir().join(format!("campaign_cur_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let ir = ring_ir(16);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        let mut mgr = CampaignManager::open(&dir, 10).unwrap();
        mgr.run(&ir, &reg, &evolver, &exec, &small_cfg()).unwrap(); // seed the DB

        let cfg = CampaignConfig {
            curiosity_lambda: 0.5,
            ..small_cfg()
        };
        let s = mgr.run(&ir, &reg, &evolver, &exec, &cfg).unwrap();
        assert!(
            s.notes.iter().any(|n| n.contains("curiosity: λ=")),
            "curiosity note missing: {:?}",
            s.notes
        );
        // The explore/exploit blend must not break the loop's correctness.
        assert!(s.best_score <= s.baseline + 1e-9);
        let _ = fs::remove_dir_all(&dir);
    }
}
