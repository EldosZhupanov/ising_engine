//! Research Executive (Stage 8 — the Chief Scientist).
//!
//! Every other component is an organ. This is the ONE agent that manages them
//! all — the executive that, each cycle, answers the meta-questions no single
//! model can:
//!
//!   • What should I research next, and why?
//!   • Is there enough data here, or should I run another 1,000 experiments?
//!   • Which model needs attention — retrain the Predictor, or update the World?
//!   • Should I ask the cheap local model (Qwen) first, or is there enough new
//!     distilled knowledge to spend a deep review from the cloud (Claude)?
//!   • Is a theory backed by enough evidence to publish, or does it need more
//!     ablation trials?
//!
//! It answers each with a MEASURED reason drawn from the platform's own state —
//! coverage, model staleness, theory confidence, budget. It is a deterministic,
//! auditable policy (the LLM proposes; the executive decides), so every action
//! it takes is a reproducible, explainable choice, never a black box.

use super::db::ExperimentDb;
use super::graph::KnowledgeGraph;
use super::memory_os::{regime_label, MemoryManager};
use super::predictor::InstanceSignature;
use std::collections::BTreeMap;

/// When each model was last trained (as an experiment-count watermark) plus the
/// availability of the two LLM tiers — the executive's view of its resources.
#[derive(Debug, Clone, Default)]
pub struct ResourceState {
    pub predictor_trained_at: usize,
    pub world_trained_at: usize,
    pub dynamics_trained_at: usize,
    /// The Policy has no `trained_on` field of its own, so its freshness is
    /// tracked here (advanced by the loop when it retrains the Policy).
    pub policy_trained_at: usize,
    pub last_cloud_consult_at: usize,
    pub cloud_available: bool,
    pub local_available: bool,
}

/// Thresholds governing the executive's judgment.
#[derive(Debug, Clone, Copy)]
pub struct ExecutiveConfig {
    /// Retrain the Predictor once the DB has grown this much since it last ran.
    pub predictor_refresh: usize,
    /// The World/Dynamics models are costlier — refresh them less often.
    pub world_refresh: usize,
    /// Consult the cloud model at most once per this many new experiments.
    pub cloud_cadence: usize,
    /// A regime with at least this many experiments AND a stable dominant
    /// operator counts as sufficiently characterized ("enough data").
    pub regime_saturation: usize,
    /// A theory at or above this confidence is publishable; below, it needs
    /// more ablation evidence.
    pub theory_publish_conf: f64,
}

impl Default for ExecutiveConfig {
    fn default() -> Self {
        Self {
            predictor_refresh: 200,
            world_refresh: 600,
            cloud_cadence: 10_000,
            regime_saturation: 300,
            theory_publish_conf: 0.5,
        }
    }
}

/// A single executive action, with its measured justification and urgency.
#[derive(Debug, Clone)]
pub struct Decision {
    pub action: Action,
    pub reason: String,
    /// 0..1, higher = do sooner. Decisions are returned urgency-sorted.
    pub urgency: f64,
}

impl Decision {
    /// The human-readable directive for this decision's action.
    pub fn title(&self) -> String {
        label(&self.action)
    }
}

/// The kinds of decision the executive makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Study this regime next (it is the least characterized).
    Research { regime: String },
    /// Run more experiments here — the regime is under-sampled.
    GatherMoreData { regime: String },
    /// Enough data here — this regime is well characterized; move on.
    RegimeSaturated { regime: String },
    /// Retrain a model (`Predictor` / `World` / `Dynamics`).
    TrainModel { model: String },
    /// Trust a specific learned model (`Predictor` / `Policy` / `World` /
    /// `Dynamics`) for the next decision on this regime — per-situation model
    /// selection, conditioned on how well the regime is characterized.
    UseModel { model: String, regime: String },
    /// Generate more ideas cheaply with the local model (Qwen) first.
    ConsultLocalLLM,
    /// Spend a rare, deep review from the cloud model (Claude).
    ConsultCloudLLM,
    /// A theory has enough evidence — publish it.
    PublishTheory { operator: String },
    /// A candidate mechanism needs more ablation trials before it is a theory.
    GatherTheoryEvidence { operator: String },
}

/// The executive's report for one cycle: the state of the research and the
/// prioritized decisions, each explained.
#[derive(Debug, Clone)]
pub struct ExecutiveBrief {
    pub headline: String,
    pub decisions: Vec<Decision>,
}

impl ExecutiveBrief {
    /// The single most urgent thing to do right now, if any.
    pub fn top(&self) -> Option<&Decision> {
        self.decisions.first()
    }

    /// A compact narrative for the dashboard / report / LLM prompt.
    pub fn narrative(&self) -> String {
        let mut s = format!("{}\n", self.headline);
        for d in &self.decisions {
            s.push_str(&format!("• {} — {}\n", label(&d.action), d.reason));
        }
        s
    }
}

fn label(a: &Action) -> String {
    match a {
        Action::Research { regime } => format!("RESEARCH the {regime} regime next"),
        Action::GatherMoreData { regime } => format!("RUN MORE experiments on {regime}"),
        Action::RegimeSaturated { regime } => format!("ENOUGH data on {regime} — move on"),
        Action::TrainModel { model } => format!("RETRAIN the {model} model"),
        Action::UseModel { model, regime } => {
            format!("TRUST the {model} model for the next {regime} decision")
        }
        Action::ConsultLocalLLM => "ASK the local model (Qwen) for ideas first".into(),
        Action::ConsultCloudLLM => "CONSULT the cloud model (Claude) for a deep review".into(),
        Action::PublishTheory { operator } => format!("PUBLISH the theory for {operator}"),
        Action::GatherTheoryEvidence { operator } => {
            format!("GATHER more ablation evidence for {operator}")
        }
    }
}

/// The Chief Scientist.
pub struct ResearchExecutive {
    cfg: ExecutiveConfig,
}

impl ResearchExecutive {
    pub fn new(cfg: ExecutiveConfig) -> Self {
        Self { cfg }
    }

    /// Survey the whole platform state and decide what to do this cycle. When
    /// `sig` is `Some`, the executive ALSO chooses which learned model to trust
    /// for the next decision on that instance's regime (per-situation model
    /// selection); render-context callers with no current instance pass `None`.
    pub fn assess(
        &self,
        db: &ExperimentDb,
        graph: &KnowledgeGraph,
        resources: &ResourceState,
        sig: Option<&InstanceSignature>,
    ) -> ExecutiveBrief {
        let n = db.len();
        let mem = MemoryManager::default().analyze(db);
        let best = db.best().map(|b| b.rel_improvement()).unwrap_or(0.0);
        let headline = format!(
            "State of research: {n} experiments across {} regimes, {} instances; best improvement {:+.1}%; {} knowledge facts.",
            mem.buckets.len(),
            mem.distinct_instances,
            best * 100.0,
            graph.len()
        );

        let mut decisions: Vec<Decision> = Vec::new();

        // ── What to research next? The least-characterized regime. ──
        if let Some(thin) = mem.buckets.iter().min_by_key(|b| b.experiments) {
            let deficit = 1.0 / (1.0 + thin.experiments as f64);
            if thin.experiments < self.cfg.regime_saturation {
                decisions.push(Decision {
                    action: Action::Research {
                        regime: thin.label.clone(),
                    },
                    reason: format!(
                        "the {} regime has only {} experiments — the least characterized, so the most to learn",
                        thin.label, thin.experiments
                    ),
                    urgency: 0.6 + 0.4 * deficit,
                });
                decisions.push(Decision {
                    action: Action::GatherMoreData {
                        regime: thin.label.clone(),
                    },
                    reason: format!(
                        "{} experiments < saturation ({}); run more before drawing conclusions",
                        thin.experiments, self.cfg.regime_saturation
                    ),
                    urgency: 0.55 + 0.3 * deficit,
                });
            }
        }
        // ── Is there enough data anywhere? Note saturated regimes. ──
        for b in &mem.buckets {
            if b.experiments >= self.cfg.regime_saturation && b.dominant_operator.is_some() {
                decisions.push(Decision {
                    action: Action::RegimeSaturated {
                        regime: b.label.clone(),
                    },
                    reason: format!(
                        "{} experiments with a stable dominant operator ({}) — well characterized; stop spending here",
                        b.experiments,
                        b.dominant_operator.as_deref().unwrap_or("?")
                    ),
                    urgency: 0.35,
                });
            }
        }

        // ── Which model needs attention? Retrain by staleness. ──
        let stale = |trained_at: usize, refresh: usize| -> Option<f64> {
            let grown = n.saturating_sub(trained_at);
            if grown >= refresh {
                Some((grown as f64 / refresh as f64).min(3.0) / 3.0) // → up to 1.0
            } else {
                None
            }
        };
        if let Some(u) = stale(resources.predictor_trained_at, self.cfg.predictor_refresh) {
            decisions.push(Decision {
                action: Action::TrainModel {
                    model: "Predictor".into(),
                },
                reason: format!(
                    "{} new experiments since the Predictor last trained (refresh every {}) — its forecasts are going stale",
                    n.saturating_sub(resources.predictor_trained_at),
                    self.cfg.predictor_refresh
                ),
                urgency: 0.5 + 0.4 * u,
            });
        }
        if let Some(u) = stale(resources.world_trained_at, self.cfg.world_refresh) {
            decisions.push(Decision {
                action: Action::TrainModel {
                    model: "World".into(),
                },
                reason: format!(
                    "the World model is {} experiments behind — its imagined rollouts no longer reflect the current dynamics",
                    n.saturating_sub(resources.world_trained_at)
                ),
                urgency: 0.45 + 0.35 * u,
            });
        }

        // ── Per-situation model selection: which learned model to TRUST for the
        // NEXT decision on THIS instance's regime? Different models answer
        // different questions, so the choice is conditioned on how well the
        // regime is characterized (Predictor is reliable only in-distribution)
        // and on each model's freshness — never a bare "pick the newest". ──
        if let Some(sig) = sig {
            let regime = regime_label(sig.density);
            let here = mem.buckets.iter().find(|b| b.label == regime);
            let experiments = here.map(|b| b.experiments).unwrap_or(0);
            let dominant = here.and_then(|b| b.dominant_operator.clone());
            let world_stale = n.saturating_sub(resources.world_trained_at);
            let policy_stale = n.saturating_sub(resources.policy_trained_at);

            let (model, reason) = if experiments >= self.cfg.regime_saturation && dominant.is_some()
            {
                (
                    "Predictor",
                    format!(
                        "the {regime} regime is well characterized ({experiments} experiments, dominant {}) — the Predictor's in-distribution score forecast is the reliable guide",
                        dominant.as_deref().unwrap_or("?")
                    ),
                )
            } else if experiments < self.cfg.predictor_refresh {
                // Under-sampled: the Predictor would extrapolate off-distribution.
                // Prefer the World model's imagined rollouts, unless it is the
                // staler of the two — then fall back to the Policy's proposals.
                if world_stale <= policy_stale {
                    (
                        "World",
                        format!(
                            "the {regime} regime is under-sampled ({experiments} experiments) — trust the World model's imagined rollouts rather than the Predictor extrapolating off-distribution"
                        ),
                    )
                } else {
                    (
                        "Policy",
                        format!(
                            "the {regime} regime is under-sampled ({experiments} experiments) and the World model is {world_stale} experiments stale — trust the Policy's distilled operator proposals instead"
                        ),
                    )
                }
            } else {
                (
                    "Policy",
                    format!(
                        "the {regime} regime is partly characterized ({experiments} experiments, no stable dominant operator) — trust the Policy to propose the next operator"
                    ),
                )
            };
            decisions.push(Decision {
                action: Action::UseModel {
                    model: model.into(),
                    regime: regime.into(),
                },
                reason,
                urgency: 0.5,
            });
        }

        // ── Local first, cloud rarely. ──
        let since_cloud = n.saturating_sub(resources.last_cloud_consult_at);
        if resources.cloud_available && since_cloud >= self.cfg.cloud_cadence && n > 0 {
            decisions.push(Decision {
                action: Action::ConsultCloudLLM,
                reason: format!(
                    "{since_cloud} new experiments distilled since the last cloud review (cadence {}) — enough new knowledge to justify the expense",
                    self.cfg.cloud_cadence
                ),
                urgency: 0.5,
            });
        } else if resources.local_available {
            decisions.push(Decision {
                action: Action::ConsultLocalLLM,
                reason: "cheap idea generation; not enough new distilled knowledge to warrant the cloud yet".into(),
                urgency: 0.3,
            });
        }

        // ── Theories: publish the well-evidenced, investigate the rest. ──
        // Existing theory facts, keyed by operator.
        let mut theory_conf: BTreeMap<&str, f64> = BTreeMap::new();
        for t in graph.triples() {
            if t.predicate == "theory-explains" {
                let e = theory_conf.entry(t.subject.as_str()).or_insert(0.0);
                *e = e.max(t.confidence());
            }
        }
        for (op, conf) in &theory_conf {
            if *conf >= self.cfg.theory_publish_conf {
                decisions.push(Decision {
                    action: Action::PublishTheory {
                        operator: (*op).to_string(),
                    },
                    reason: format!("theory for {op} reached confidence {conf:.2} — enough evidence to stand as knowledge"),
                    urgency: 0.4,
                });
            } else {
                decisions.push(Decision {
                    action: Action::GatherTheoryEvidence {
                        operator: (*op).to_string(),
                    },
                    reason: format!(
                        "theory for {op} is only {conf:.2} confident (< {}) — run more ablation trials on other instances",
                        self.cfg.theory_publish_conf
                    ),
                    urgency: 0.35,
                });
            }
        }
        // A dominant operator with NO theory yet is worth explaining.
        for b in &mem.buckets {
            if let Some(op) = &b.dominant_operator {
                if !theory_conf.contains_key(op.as_str())
                    && b.experiments >= self.cfg.predictor_refresh
                {
                    decisions.push(Decision {
                        action: Action::GatherTheoryEvidence {
                            operator: op.clone(),
                        },
                        reason: format!(
                            "{op} dominates the {} regime but has no explanation yet — run the Theory Engine to find WHY",
                            b.label
                        ),
                        urgency: 0.42,
                    });
                }
            }
        }

        decisions.sort_by(|a, b| b.urgency.total_cmp(&a.urgency));
        decisions.dedup_by(|a, b| a.action == b.action);
        ExecutiveBrief {
            headline,
            decisions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(inst: &str, density: f64, seq: &[&str], score: f64) -> ExperimentRecord {
        ExperimentRecord {
            instance_id: inst.into(),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            score,
            baseline: 0.0,
            density,
            n: 800,
            clustering: 0.3,
            mean_degree: 4.0,
            ..Default::default()
        }
    }

    #[test]
    fn recommends_researching_the_thinnest_regime_and_gathering_data() {
        // A sparse regime with 5 experiments (thin) and nothing else.
        let db = ExperimentDb::from_records(
            (0..5)
                .map(|_| rec("g", 0.01, &["metropolis_sweep"], -0.5))
                .collect(),
        );
        let exec = ResearchExecutive::new(ExecutiveConfig::default());
        let brief = exec.assess(&db, &KnowledgeGraph::new(), &ResourceState::default(), None);
        assert!(
            brief
                .decisions
                .iter()
                .any(|d| matches!(&d.action, Action::Research { regime } if regime == "sparse")),
            "should choose to research the thin regime: {}",
            brief.narrative()
        );
        assert!(brief
            .decisions
            .iter()
            .any(|d| matches!(d.action, Action::GatherMoreData { .. })));
    }

    #[test]
    fn recommends_retraining_a_stale_predictor() {
        // 300 experiments, predictor trained at 0 ⇒ very stale.
        let db =
            ExperimentDb::from_records((0..300).map(|_| rec("g", 0.01, &["op"], -0.5)).collect());
        let res = ResourceState {
            predictor_trained_at: 0,
            world_trained_at: 300,
            local_available: true,
            ..Default::default()
        };
        let brief = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &db,
            &KnowledgeGraph::new(),
            &res,
            None,
        );
        let train = brief
            .decisions
            .iter()
            .find(|d| matches!(&d.action, Action::TrainModel { model } if model == "Predictor"));
        assert!(
            train.is_some(),
            "stale predictor must be flagged: {}",
            brief.narrative()
        );
        // World was just trained ⇒ NOT flagged.
        assert!(!brief
            .decisions
            .iter()
            .any(|d| matches!(&d.action, Action::TrainModel { model } if model == "World")));
    }

    #[test]
    fn local_first_cloud_only_when_enough_new_knowledge() {
        let db =
            ExperimentDb::from_records((0..50).map(|_| rec("g", 0.01, &["op"], -0.5)).collect());
        // Little new knowledge ⇒ local first, not cloud.
        let res = ResourceState {
            cloud_available: true,
            local_available: true,
            last_cloud_consult_at: 0,
            ..Default::default()
        };
        let brief = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &db,
            &KnowledgeGraph::new(),
            &res,
            None,
        );
        assert!(brief
            .decisions
            .iter()
            .any(|d| d.action == Action::ConsultLocalLLM));
        assert!(!brief
            .decisions
            .iter()
            .any(|d| d.action == Action::ConsultCloudLLM));
    }

    #[test]
    fn selects_the_predictor_for_a_well_characterized_regime() {
        // 300 very-sparse experiments with a stable operator ⇒ saturated regime
        // ⇒ trust the in-distribution Predictor's score forecast.
        let db = ExperimentDb::from_records(
            (0..300)
                .map(|_| rec("g", 0.005, &["metropolis_sweep"], -0.5))
                .collect(),
        );
        let sig = InstanceSignature {
            n: 800,
            density: 0.005,
            clustering: 0.3,
            mean_degree: 4.0,
            degree_cv: 0.5,
        };
        let brief = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &db,
            &KnowledgeGraph::new(),
            &ResourceState::default(),
            Some(&sig),
        );
        assert!(
            brief.decisions.iter().any(|d| matches!(&d.action,
                Action::UseModel { model, regime } if model == "Predictor" && regime == "very-sparse")),
            "well-characterized regime should trust the Predictor: {}",
            brief.narrative()
        );
    }

    #[test]
    fn selects_an_exploratory_model_for_an_under_sampled_regime() {
        // Only 4 experiments in this regime ⇒ under-sampled ⇒ trust the World
        // model's imagined rollouts, not the Predictor extrapolating.
        let db = ExperimentDb::from_records(
            (0..4)
                .map(|_| rec("g", 0.005, &["metropolis_sweep"], -0.5))
                .collect(),
        );
        let sig = InstanceSignature {
            n: 800,
            density: 0.005,
            ..Default::default()
        };
        let brief = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &db,
            &KnowledgeGraph::new(),
            &ResourceState::default(),
            Some(&sig),
        );
        let picked = brief.decisions.iter().find_map(|d| match &d.action {
            Action::UseModel { model, regime } => Some((model.clone(), regime.clone())),
            _ => None,
        });
        assert_eq!(
            picked,
            Some(("World".into(), "very-sparse".into())),
            "under-sampled regime should trust the World model: {}",
            brief.narrative()
        );
        // With no current instance there is NO per-situation model selection.
        let none_brief = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &db,
            &KnowledgeGraph::new(),
            &ResourceState::default(),
            None,
        );
        assert!(!none_brief
            .decisions
            .iter()
            .any(|d| matches!(d.action, Action::UseModel { .. })));
    }

    #[test]
    fn publishes_a_confident_theory_and_investigates_a_weak_one() {
        let db =
            ExperimentDb::from_records((0..10).map(|_| rec("g", 0.01, &["op"], -0.5)).collect());
        let mut g = KnowledgeGraph::new();
        // A confident theory (published many trials) and a weak one.
        for _ in 0..8 {
            g.observe_if(
                "greedy_descent",
                "theory-explains",
                "exploitation",
                "density<0.05",
                0.9,
                "strong",
            );
        }
        g.observe_if(
            "random_flip_sweep",
            "theory-explains",
            "exploration",
            "density<0.05",
            0.1,
            "weak",
        );
        let brief = ResearchExecutive::new(ExecutiveConfig::default()).assess(
            &db,
            &g,
            &ResourceState::default(),
            None,
        );
        assert!(brief.decisions.iter().any(
            |d| matches!(&d.action, Action::PublishTheory { operator } if operator == "greedy_descent")
        ), "confident theory should be published: {}", brief.narrative());
        assert!(brief.decisions.iter().any(
            |d| matches!(&d.action, Action::GatherTheoryEvidence { operator } if operator == "random_flip_sweep")
        ), "weak theory needs more evidence: {}", brief.narrative());
    }
}
