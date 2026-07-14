//! Meta-Learning Layer (the "next level" — where the models share knowledge).
//!
//! The platform now has four learners that each see the problem differently:
//!   - the Meta-Learner mines symbolic rules from the experiment history;
//!   - the Policy learns which operator to reach for on a given regime;
//!   - the World model predicts each operator's yield without running it;
//!   - the Dynamics model learns when a run plateaus.
//!
//! On their own they are siloed. This layer is the BUS that consolidates their
//! per-operator opinions into one shared verdict, publishes it (with full
//! source attribution) into the Knowledge Graph so the LLM scientist and the
//! reports see the consensus, and biases the Evolution ideator toward what the
//! models jointly endorse. That closes the user's loop:
//!
//!   Dynamics says op plateaus early  ─┐
//!   World says op has low yield       ├─▶ Meta-Learning Layer ─▶ shared verdict
//!   Policy stops preferring op        ─┘        │
//!                                               ├─▶ Knowledge Graph ─▶ LLM report
//!                                               └─▶ biased ideator ─▶ Evolution ─▶ Runtime ─▶ (more data) ─▶ …
//!
//! Everything here is a CONSUMER of already-trained models — it computes no
//! energies itself and it never overrides the Runtime's verdict. It only
//! reweights which candidates are worth the Runtime's time, and records why.

use super::super::evolution::Schedule;
use super::super::ir::ProblemIR;
use super::super::registry::OperatorRegistry;
use super::db::ExperimentDb;
use super::dynamics::{capture_trajectory, DynamicsModel};
use super::graph::KnowledgeGraph;
use super::lab::{Ideator, ResearchBrief};
use super::meta_learner::{MetaLearner, RuleKind};
use super::policy::OperatorPolicy;
use super::predictor::InstanceSignature;
use super::scientist::Hypothesis;
use super::world::WorldModel;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeMap;

/// Which model produced a per-operator signal (for attribution in the graph).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Meta,
    Policy,
    World,
    Dynamics,
}

impl Source {
    fn tag(self) -> &'static str {
        match self {
            Source::Meta => "meta",
            Source::Policy => "policy",
            Source::World => "world",
            Source::Dynamics => "dynamics",
        }
    }
}

/// The raw per-operator signals gathered from the models — the shared bus each
/// model writes into. Missing entries mean "that model has no opinion here".
#[derive(Debug, Clone, Default)]
pub struct OperatorSignals {
    /// Symbolic rule bias: + for dominant operators, − for anti-patterns/useless.
    pub meta_bias: BTreeMap<String, f64>,
    /// Policy's first-operator preference probability (0..1).
    pub policy_pref: BTreeMap<String, f64>,
    /// World model's predicted single-step yield (raw energy improvement, ≥ 0).
    pub world_yield: BTreeMap<String, f64>,
    /// Dynamics plateau fraction (0..1): low ⇒ the operator front-loads and a
    /// switch-early is warranted.
    pub dynamics_plateau: BTreeMap<String, f64>,
}

impl OperatorSignals {
    /// Extract symbolic bias from the Meta-Learner's mined rules: dominance
    /// adds +confidence, antipattern/useless subtract confidence, accumulated
    /// per operator (the operator is the rule statement's leading token, which
    /// is how the miner formats every rule).
    pub fn from_meta(meta: &MetaLearner, db: &ExperimentDb) -> Self {
        let mut s = Self::default();
        for rule in meta.mine(db) {
            let Some(op) = rule.statement.split_whitespace().next() else {
                continue;
            };
            let sign = match rule.kind {
                RuleKind::Dominance => 1.0,
                RuleKind::Antipattern | RuleKind::Useless => -1.0,
                _ => 0.0,
            };
            if sign != 0.0 {
                *s.meta_bias.entry(op.to_string()).or_insert(0.0) += sign * rule.confidence;
            }
        }
        s
    }

    /// Add the Policy's first-operator preferences on this instance regime.
    pub fn with_policy(mut self, policy: &OperatorPolicy, sig: &InstanceSignature) -> Self {
        for (op, p) in policy.first_op_probs(sig) {
            self.policy_pref.insert(op, p);
        }
        self
    }

    /// Add the World model's imagined per-operator yields on this instance.
    pub fn with_world(
        mut self,
        world: &WorldModel,
        ir: &ProblemIR,
        temp_hi: f64,
        sweeps: u32,
    ) -> Self {
        for op in world.vocab() {
            self.world_yield
                .insert(op.clone(), world.predicted_yield(ir, op, temp_hi, sweeps));
        }
        self
    }

    /// Add the Dynamics model's per-operator plateau fractions by capturing one
    /// short single-operator trajectory per operator on `ir` (bounded cost —
    /// `ops.len()` runs). Operators absent from the run vocabulary are skipped.
    #[allow(clippy::too_many_arguments)]
    pub fn with_dynamics(
        mut self,
        model: &DynamicsModel,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        ops: &[String],
        num_replicas: usize,
        sweeps_each: u32,
        repeats: usize,
        seed: u64,
        epsilon: f64,
    ) -> Self {
        for op in ops {
            let sched = Schedule {
                ops: vec![op.clone(); repeats.max(2)],
                sweeps: vec![sweeps_each; repeats.max(2)],
                temp_hi: 4.0,
                temp_lo: 0.1,
            };
            let tr = capture_trajectory(ir, registry, &sched, num_replicas, seed);
            if tr.steps.len() >= 2 {
                self.dynamics_plateau
                    .insert(op.clone(), model.plateau_frac(&tr, epsilon));
            }
        }
        self
    }

    fn all_operators(&self) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();
        for m in [
            &self.meta_bias,
            &self.policy_pref,
            &self.world_yield,
            &self.dynamics_plateau,
        ] {
            set.extend(m.keys().cloned());
        }
        set.into_iter().collect()
    }
}

/// Relative weight of each model in the consolidated score. Dynamics does not
/// enter the score (front-loading is not "good/bad", it drives switch-early).
#[derive(Debug, Clone, Copy)]
pub struct MetaWeights {
    pub meta: f64,
    pub policy: f64,
    pub world: f64,
    /// Score threshold (in combined z-units) above which an operator is
    /// PREFERRED and below whose negation it is AVOIDED.
    pub decision: f64,
    /// Plateau fraction below which an operator is flagged switch-early.
    pub switch_frac: f64,
}

impl Default for MetaWeights {
    fn default() -> Self {
        Self {
            meta: 1.0,
            policy: 0.7,
            world: 0.8,
            decision: 0.5,
            switch_frac: 0.4,
        }
    }
}

/// The consolidated, source-attributed verdict on one operator.
#[derive(Debug, Clone)]
pub struct OperatorVerdict {
    pub op: String,
    /// Combined z-score; higher ⇒ the models jointly endorse it.
    pub score: f64,
    pub prefer: bool,
    pub avoid: bool,
    pub switch_early: bool,
    /// (source, contribution) for auditability.
    pub sources: Vec<(Source, f64)>,
    pub rationale: String,
}

/// z-normalize a map's values → per-key z (mean 0, unit std). Empty/degenerate
/// input yields all-zero, so a source with no spread contributes nothing.
fn znorm(m: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
    let n = m.len();
    if n == 0 {
        return BTreeMap::new();
    }
    let mean = m.values().sum::<f64>() / n as f64;
    let var = m.values().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let sd = var.sqrt();
    m.iter()
        .map(|(k, v)| (k.clone(), if sd > 1e-12 { (v - mean) / sd } else { 0.0 }))
        .collect()
}

/// The shared knowledge produced by consolidating all models' signals for a
/// given instance regime (identified by `condition`).
#[derive(Debug, Clone)]
pub struct MetaKnowledge {
    verdicts: Vec<OperatorVerdict>,
    condition: String,
}

impl MetaKnowledge {
    /// Consolidate the shared bus into per-operator verdicts. `condition` is the
    /// structural regime the verdicts hold under (e.g. "density<0.05"), stamped
    /// onto every published fact so the knowledge stays conditional.
    pub fn consolidate(signals: &OperatorSignals, weights: &MetaWeights, condition: &str) -> Self {
        let zm = znorm(&signals.meta_bias);
        let zp = znorm(&signals.policy_pref);
        let zw = znorm(&signals.world_yield);
        let mut verdicts = Vec::new();
        for op in signals.all_operators() {
            let mut sources = Vec::new();
            let mut score = 0.0;
            if let Some(v) = zm.get(&op) {
                score += weights.meta * v;
                sources.push((Source::Meta, *v));
            }
            if let Some(v) = zp.get(&op) {
                score += weights.policy * v;
                sources.push((Source::Policy, *v));
            }
            if let Some(v) = zw.get(&op) {
                score += weights.world * v;
                sources.push((Source::World, *v));
            }
            let plateau = signals.dynamics_plateau.get(&op).copied();
            if let Some(p) = plateau {
                sources.push((Source::Dynamics, p));
            }
            let switch_early = plateau.is_some_and(|p| p < weights.switch_frac);
            let prefer = score > weights.decision;
            let avoid = score < -weights.decision;
            let rationale = {
                let parts: Vec<String> = sources
                    .iter()
                    .map(|(s, v)| format!("{}={:+.2}", s.tag(), v))
                    .collect();
                let tag = if prefer {
                    "PREFER"
                } else if avoid {
                    "AVOID"
                } else {
                    "neutral"
                };
                let sw = if switch_early { " +switch-early" } else { "" };
                format!("{tag}{sw} (score {score:+.2}: {})", parts.join(", "))
            };
            verdicts.push(OperatorVerdict {
                op,
                score,
                prefer,
                avoid,
                switch_early,
                sources,
                rationale,
            });
        }
        // Strongest endorsement first (deterministic tie-break by name).
        verdicts.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.op.cmp(&b.op)));
        Self {
            verdicts,
            condition: condition.to_string(),
        }
    }

    pub fn verdicts(&self) -> &[OperatorVerdict] {
        &self.verdicts
    }

    pub fn verdict(&self, op: &str) -> Option<&OperatorVerdict> {
        self.verdicts.iter().find(|v| v.op == op)
    }

    pub fn preferred(&self) -> Vec<&str> {
        self.verdicts
            .iter()
            .filter(|v| v.prefer)
            .map(|v| v.op.as_str())
            .collect()
    }

    pub fn avoided(&self) -> Vec<&str> {
        self.verdicts
            .iter()
            .filter(|v| v.avoid)
            .map(|v| v.op.as_str())
            .collect()
    }

    pub fn switch_early(&self) -> Vec<&str> {
        self.verdicts
            .iter()
            .filter(|v| v.switch_early)
            .map(|v| v.op.as_str())
            .collect()
    }

    /// Publish the consensus into the Knowledge Graph as CONDITIONAL,
    /// source-attributed facts, so the LLM scientist and the reports read what
    /// the models jointly concluded — with the proof of which models agreed.
    /// Returns the number of facts written.
    pub fn publish(&self, graph: &mut KnowledgeGraph) -> usize {
        let mut n = 0;
        for v in &self.verdicts {
            if !v.prefer && !v.avoid && !v.switch_early {
                continue;
            }
            let proof = format!("meta-layer consensus: {}", v.rationale);
            if v.prefer {
                graph.observe_if(
                    &v.op,
                    "consensus-prefer",
                    "schedule",
                    &self.condition,
                    v.score,
                    &proof,
                );
                n += 1;
            }
            if v.avoid {
                graph.observe_if(
                    &v.op,
                    "consensus-avoid",
                    "schedule",
                    &self.condition,
                    v.score,
                    &proof,
                );
                n += 1;
            }
            if v.switch_early {
                graph.observe_if(
                    &v.op,
                    "plateaus-early",
                    "true",
                    &self.condition,
                    1.0,
                    &proof,
                );
                n += 1;
            }
        }
        n
    }

    /// A compact consensus for the LLM prompt / research report — the story the
    /// user described ("Dynamics says X plateaus → avoid long X").
    pub fn consensus_report(&self) -> String {
        let mut s = String::from("## Meta-Learning Layer — cross-model consensus\n");
        let pref = self.preferred();
        let avoid = self.avoided();
        let switch = self.switch_early();
        if pref.is_empty() && avoid.is_empty() && switch.is_empty() {
            s.push_str("- (no operator crossed the consensus threshold yet)\n");
            return s;
        }
        if !pref.is_empty() {
            s.push_str(&format!("- PREFER (models agree): {}\n", pref.join(", ")));
        }
        if !avoid.is_empty() {
            s.push_str(&format!("- AVOID (models agree): {}\n", avoid.join(", ")));
        }
        if !switch.is_empty() {
            s.push_str(&format!(
                "- SWITCH EARLY (front-loads, Dynamics): {}\n",
                switch.join(", ")
            ));
        }
        s.push_str(&format!("- (conditioned on: {})\n", self.condition));
        s
    }

    /// Consolidated bias for one operator (score; 0 if unknown), for ideators.
    pub fn bias(&self, op: &str) -> f64 {
        self.verdict(op).map(|v| v.score).unwrap_or(0.0)
    }
}

/// An `Ideator` that re-weights a base ideator's proposals by the shared
/// consensus: it oversamples, scores each candidate by the mean operator bias,
/// drops any proposal containing an AVOIDED operator when a better one exists,
/// and keeps the top `n`. The Runtime still judges every survivor — this only
/// spends the search budget on what the models jointly endorse.
pub struct MetaBiasedIdeator {
    inner: Box<dyn Ideator>,
    knowledge: MetaKnowledge,
    oversample: usize,
}

impl MetaBiasedIdeator {
    pub fn new(inner: Box<dyn Ideator>, knowledge: MetaKnowledge, oversample: usize) -> Self {
        Self {
            inner,
            knowledge,
            oversample: oversample.max(1),
        }
    }

    fn proposal_score(&self, ops: &[String]) -> f64 {
        if ops.is_empty() {
            return f64::NEG_INFINITY;
        }
        ops.iter().map(|op| self.knowledge.bias(op)).sum::<f64>() / ops.len() as f64
    }
}

impl Ideator for MetaBiasedIdeator {
    fn propose(
        &mut self,
        brief: &ResearchBrief,
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        let mut cand = self.inner.propose(brief, n * self.oversample, rng);
        if cand.len() <= n {
            return cand;
        }
        // Rank by consensus score; stable so equal-score keeps proposal order.
        cand.sort_by(|a, b| {
            self.proposal_score(&b.operators)
                .total_cmp(&self.proposal_score(&a.operators))
        });
        cand.truncate(n);
        cand
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals() -> OperatorSignals {
        let mut s = OperatorSignals::default();
        // greedy: mildly good rule, high policy pref, high world yield.
        s.meta_bias.insert("greedy_descent".into(), 0.6);
        s.policy_pref.insert("greedy_descent".into(), 0.5);
        s.world_yield.insert("greedy_descent".into(), 50.0);
        // metropolis: neutral rule, medium pref, medium yield, front-loads.
        s.meta_bias.insert("metropolis_sweep".into(), 0.1);
        s.policy_pref.insert("metropolis_sweep".into(), 0.35);
        s.world_yield.insert("metropolis_sweep".into(), 30.0);
        s.dynamics_plateau.insert("metropolis_sweep".into(), 0.2); // plateaus early
                                                                   // random_flip: anti-pattern, ignored by policy, ~0 yield.
        s.meta_bias.insert("random_flip_sweep".into(), -0.9);
        s.policy_pref.insert("random_flip_sweep".into(), 0.15);
        s.world_yield.insert("random_flip_sweep".into(), 2.0);
        s
    }

    #[test]
    fn consolidation_separates_prefer_avoid_and_switch_early() {
        let k = MetaKnowledge::consolidate(&signals(), &MetaWeights::default(), "density<0.05");
        assert!(
            k.preferred().contains(&"greedy_descent"),
            "greedy should be preferred: {:?}",
            k.verdicts()
        );
        assert!(
            k.avoided().contains(&"random_flip_sweep"),
            "random_flip should be avoided: {:?}",
            k.verdicts()
        );
        assert!(
            k.switch_early().contains(&"metropolis_sweep"),
            "metropolis front-loads → switch early: {:?}",
            k.switch_early()
        );
        // Every verdict carries source attribution.
        for v in k.verdicts() {
            assert!(!v.sources.is_empty(), "{} has no sources", v.op);
        }
    }

    #[test]
    fn publish_writes_conditional_source_attributed_facts() {
        let k = MetaKnowledge::consolidate(&signals(), &MetaWeights::default(), "density<0.05");
        let mut g = KnowledgeGraph::new();
        let wrote = k.publish(&mut g);
        assert!(
            wrote >= 3,
            "expected prefer+avoid+switch facts, got {wrote}"
        );
        // The avoid fact is conditional and attributed.
        let avoid = g.query_applicable("random_flip_sweep", "consensus-avoid", 0.01, 0.3);
        assert_eq!(avoid.len(), 1);
        assert_eq!(avoid[0].condition, "density<0.05");
        assert!(avoid[0].proof.contains("meta-layer consensus"));
        // The switch-early fact is present for metropolis.
        assert_eq!(
            g.query_applicable("metropolis_sweep", "plateaus-early", 0.01, 0.3)
                .len(),
            1
        );
        // The consensus report reads like the user's story.
        let rep = k.consensus_report();
        assert!(rep.contains("PREFER"));
        assert!(rep.contains("AVOID"));
        assert!(rep.contains("SWITCH EARLY"));
    }

    // A trivial inner ideator that always proposes the same fixed set, so the
    // meta bias — not the inner logic — decides what survives.
    struct FixedIdeator {
        proposals: Vec<Vec<String>>,
        next: u64,
    }
    impl Ideator for FixedIdeator {
        fn propose(
            &mut self,
            _brief: &ResearchBrief,
            n: usize,
            _rng: &mut ChaCha8Rng,
        ) -> Vec<Hypothesis> {
            (0..n)
                .map(|i| {
                    let id = self.next;
                    self.next += 1;
                    Hypothesis {
                        id,
                        operators: self.proposals[i % self.proposals.len()].clone(),
                        rationale: "fixed".into(),
                        reasoning: "fixed".into(),
                        predicted_improvement: 0.0,
                        status: super::super::scientist::HypothesisStatus::Proposed,
                        observed_improvement: 0.0,
                        confidence: 0.0,
                    }
                })
                .collect()
        }
    }

    #[test]
    fn biased_ideator_prefers_consensus_endorsed_proposals() {
        use crate::engine_v2::decision::DecisionEngine;
        use crate::engine_v2::ir::ProblemIR;
        use crate::engine_v2::registry::OperatorRegistry;

        let k = MetaKnowledge::consolidate(&signals(), &MetaWeights::default(), "");
        // Two proposal templates: one all-greedy (endorsed), one all-random (avoided).
        let inner = Box::new(FixedIdeator {
            proposals: vec![
                vec!["random_flip_sweep".into()],
                vec!["greedy_descent".into()],
            ],
            next: 0,
        });
        let mut biased = MetaBiasedIdeator::new(inner, k, 3);

        // Build a minimal brief.
        let ir = ProblemIR::from_pairs(4, 0.0, vec![0.0; 4], &[(0, 1, 1.0), (2, 3, -1.0)]);
        let reg = OperatorRegistry::standard();
        let stats = DecisionEngine::analyze(&ir);
        let db = ExperimentDb::new();
        let analysis = super::super::scientist::AIScientist::analyze(&db);
        let pool = DecisionEngine::operator_pool(&stats, stats.select_backend(), &reg);
        let graph = KnowledgeGraph::new();
        let brief = ResearchBrief {
            stats: &stats,
            analysis: &analysis,
            graph: &graph,
            pool: &pool,
            registry: &reg,
            context: "test",
        };
        let mut rng = rand_chacha::ChaCha8Rng::from_seed([3u8; 32]);
        use rand::SeedableRng;
        let out = biased.propose(&brief, 2, &mut rng);
        // With oversampling, the kept proposals should skew to greedy (endorsed)
        // over random_flip (avoided).
        let greedy = out
            .iter()
            .filter(|h| h.operators == vec!["greedy_descent".to_string()])
            .count();
        assert!(greedy >= 1, "endorsed proposal should survive: {out:?}");
    }
}
