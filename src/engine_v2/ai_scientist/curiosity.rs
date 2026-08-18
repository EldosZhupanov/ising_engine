//! Curiosity Engine (Stage 8, Pillar IV — seek the unknown, not the best).
//!
//! Every other scorer in the platform is an EXPLOITER: the Predictor ranks by
//! expected improvement, the Meta-Layer by cross-model endorsement. Left alone
//! they grind the same productive furrow forever. The Curiosity Engine is the
//! counterweight — it scores a candidate by how much it would TEACH, not how
//! much it would win. It directs compute at the strange 5% where the platform's
//! own models are wrong, because that is where new physics hides.
//!
//! Three surprise signals, all computed from the append-only experiment record
//! (and, for the first, the Predictor):
//!   1. DISAGREEMENT — where the Predictor's forecast diverges from the realized
//!      outcome (high residual ⇒ the model is confused).
//!   2. COVERAGE — operators tried rarely (or never): thin data ⇒ unknown.
//!   3. ANOMALY — operators whose outcome varies wildly ACROSS SEEDS on the same
//!      instance: unpredictable ⇒ worth characterizing.
//!
//! These blend into a per-operator curiosity in [0, 1]. The explore/exploit
//! dial `λ` (`explore_exploit_priority`) mixes curiosity with any exploitation
//! value: `λ=0` is a pure optimizer, `λ=1` a pure scientist. Nothing here
//! computes an energy or overrides the Runtime — it only decides where the
//! Runtime's time is most informative to spend.

use super::super::evolution::Schedule;
use super::db::ExperimentDb;
use super::lab::{Ideator, ResearchBrief};
use super::predictor::{InstanceSignature, Predictor};
use super::scientist::Hypothesis;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeMap;

/// Weights for the three surprise signals and the explore/exploit dial.
#[derive(Debug, Clone, Copy)]
pub struct CuriosityConfig {
    pub w_disagreement: f64,
    pub w_coverage: f64,
    pub w_anomaly: f64,
    /// Explore/exploit blend used by [`explore_exploit_priority`]. 0 ⇒ pure
    /// exploit, 1 ⇒ pure explore.
    pub lambda: f64,
}

impl Default for CuriosityConfig {
    fn default() -> Self {
        Self {
            w_disagreement: 1.0,
            w_coverage: 0.8,
            w_anomaly: 1.0,
            lambda: 0.3,
        }
    }
}

/// Min-max normalize a map's values into [0, 1]. A degenerate (flat) map maps
/// everything to 0.0 — no spread, no surprise signal.
fn minmax(m: &BTreeMap<String, f64>) -> BTreeMap<String, f64> {
    let lo = m.values().cloned().fold(f64::INFINITY, f64::min);
    let hi = m.values().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = hi - lo;
    m.iter()
        .map(|(k, v)| (k.clone(), if span > 1e-12 { (v - lo) / span } else { 0.0 }))
        .collect()
}

fn record_sig(r: &super::db::ExperimentRecord) -> InstanceSignature {
    InstanceSignature {
        n: r.n,
        density: r.density,
        clustering: r.clustering,
        mean_degree: r.mean_degree,
        degree_cv: r.degree_cv,
    }
}

fn record_schedule(r: &super::db::ExperimentRecord) -> Schedule {
    Schedule {
        ops: r.sequence.clone(),
        sweeps: r.sweeps.clone(),
        temp_hi: r.temp_hi,
        temp_lo: r.temp_lo,
    }
}

/// The surprise map over operators, derived from the experiment history.
#[derive(Debug, Clone)]
pub struct CuriosityEngine {
    /// Combined per-operator curiosity in [0, 1] (already blended).
    curiosity: BTreeMap<String, f64>,
    /// Retained components for reporting / auditability.
    disagreement: BTreeMap<String, f64>,
    coverage: BTreeMap<String, f64>,
    anomaly: BTreeMap<String, f64>,
    /// Curiosity assigned to an operator never seen in the history — maximally
    /// interesting (an unexplored move is pure unknown).
    unseen: f64,
}

impl CuriosityEngine {
    /// Build from the DB. If a trained `predictor` is supplied, the
    /// disagreement signal (prediction residual) is included; otherwise that
    /// weight contributes nothing and curiosity rests on coverage + anomaly.
    pub fn from_db(
        db: &ExperimentDb,
        predictor: Option<&Predictor>,
        cfg: &CuriosityConfig,
    ) -> Self {
        use std::collections::BTreeSet;
        let ops: BTreeSet<String> = db
            .all()
            .iter()
            .flat_map(|r| r.sequence.iter().cloned())
            .collect();

        // --- coverage: rarely tried ⇒ curious. raw = 1/(1+count) ---
        let mut count: BTreeMap<String, f64> = BTreeMap::new();
        for r in db.all() {
            for op in &r.sequence {
                *count.entry(op.clone()).or_insert(0.0) += 1.0;
            }
        }
        let raw_cov: BTreeMap<String, f64> = ops
            .iter()
            .map(|op| {
                (
                    op.clone(),
                    1.0 / (1.0 + count.get(op).copied().unwrap_or(0.0)),
                )
            })
            .collect();

        // --- anomaly: cross-seed variance of the SAME schedule on the SAME
        //     instance, aggregated per operator ---
        let mut groups: BTreeMap<(String, String), Vec<f64>> = BTreeMap::new();
        for r in db.all() {
            if r.instance_id.is_empty() {
                continue;
            }
            groups
                .entry((r.instance_id.clone(), r.sequence.join(">")))
                .or_default()
                .push(r.rel_improvement());
        }
        let mut anom_acc: BTreeMap<String, (f64, f64)> = BTreeMap::new(); // (sum var, n groups)
        for ((_, seq), vals) in &groups {
            if vals.len() < 2 {
                continue;
            }
            let mean = vals.iter().sum::<f64>() / vals.len() as f64;
            let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
            for op in seq.split('>').filter(|s| !s.is_empty()) {
                let e = anom_acc.entry(op.to_string()).or_insert((0.0, 0.0));
                e.0 += var;
                e.1 += 1.0;
            }
        }
        let raw_anom: BTreeMap<String, f64> = ops
            .iter()
            .map(|op| {
                let v = anom_acc
                    .get(op)
                    .map(|(s, n)| if *n > 0.0 { s / n } else { 0.0 })
                    .unwrap_or(0.0);
                (op.clone(), v)
            })
            .collect();

        // --- disagreement: mean |predicted − actual| per operator ---
        let raw_dis: BTreeMap<String, f64> = if let Some(p) = predictor {
            let mut acc: BTreeMap<String, (f64, f64)> = BTreeMap::new();
            for r in db.all() {
                if r.sequence.is_empty() {
                    continue;
                }
                let resid =
                    (p.predict(&record_sig(r), &record_schedule(r)) - r.rel_improvement()).abs();
                for op in &r.sequence {
                    let e = acc.entry(op.clone()).or_insert((0.0, 0.0));
                    e.0 += resid;
                    e.1 += 1.0;
                }
            }
            ops.iter()
                .map(|op| {
                    let v = acc
                        .get(op)
                        .map(|(s, n)| if *n > 0.0 { s / n } else { 0.0 })
                        .unwrap_or(0.0);
                    (op.clone(), v)
                })
                .collect()
        } else {
            ops.iter().map(|op| (op.clone(), 0.0)).collect()
        };

        // Normalize each signal to [0,1] and blend.
        let cov = minmax(&raw_cov);
        let anom = minmax(&raw_anom);
        let dis = minmax(&raw_dis);
        let wsum = (cfg.w_coverage + cfg.w_anomaly + cfg.w_disagreement).max(1e-9);
        let curiosity: BTreeMap<String, f64> = ops
            .iter()
            .map(|op| {
                let c = cfg.w_coverage * cov.get(op).copied().unwrap_or(0.0)
                    + cfg.w_anomaly * anom.get(op).copied().unwrap_or(0.0)
                    + cfg.w_disagreement * dis.get(op).copied().unwrap_or(0.0);
                (op.clone(), c / wsum)
            })
            .collect();

        Self {
            curiosity,
            disagreement: dis,
            coverage: cov,
            anomaly: anom,
            unseen: 1.0,
        }
    }

    /// Curiosity of one operator in [0, 1]. An operator never seen in the
    /// history is maximally curious (`unseen`, default 1.0).
    pub fn operator_curiosity(&self, op: &str) -> f64 {
        self.curiosity.get(op).copied().unwrap_or(self.unseen)
    }

    /// Curiosity of a whole schedule: the mean over its operators. Empty ⇒ 0.
    pub fn schedule_curiosity(&self, ops: &[String]) -> f64 {
        if ops.is_empty() {
            return 0.0;
        }
        ops.iter()
            .map(|op| self.operator_curiosity(op))
            .sum::<f64>()
            / ops.len() as f64
    }

    /// The most-curious operators, strongest first — what the platform should go
    /// characterize (for the report / the planner).
    pub fn ranked(&self) -> Vec<(String, f64)> {
        let mut v: Vec<(String, f64)> = self
            .curiosity
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }

    /// Attribution for one operator: (coverage, anomaly, disagreement) after
    /// normalization — so a report can say WHY an operator is interesting.
    pub fn components(&self, op: &str) -> (f64, f64, f64) {
        (
            self.coverage.get(op).copied().unwrap_or(1.0),
            self.anomaly.get(op).copied().unwrap_or(0.0),
            self.disagreement.get(op).copied().unwrap_or(0.0),
        )
    }
}

/// The explore/exploit dial (§ Stage 8): blend an exploitation `value` (e.g. the
/// Predictor's expected improvement or the Meta-Layer's consensus score, both
/// pre-normalized to `[0,1]`) with a `curiosity` in `[0,1]`. `λ→0` optimizes,
/// `λ→1` explores.
pub fn explore_exploit_priority(value: f64, curiosity: f64, lambda: f64) -> f64 {
    let l = lambda.clamp(0.0, 1.0);
    (1.0 - l) * value + l * curiosity
}

/// An `Ideator` that steers the search toward the unknown. It oversamples the
/// inner ideator's proposals (whose ORDER encodes the exploitation preference —
/// e.g. after the predictor/meta filters), then re-ranks by
/// `(1−λ)·exploit_rank + λ·curiosity` and keeps the top `n`. `λ=0` preserves the
/// inner order (pure exploit); `λ=1` keeps the most-curious proposals.
pub struct CuriousIdeator {
    inner: Box<dyn Ideator>,
    engine: CuriosityEngine,
    lambda: f64,
    oversample: usize,
}

impl CuriousIdeator {
    pub fn new(inner: Box<dyn Ideator>, engine: CuriosityEngine, lambda: f64) -> Self {
        Self {
            inner,
            engine,
            lambda: lambda.clamp(0.0, 1.0),
            oversample: 3,
        }
    }
}

impl Ideator for CuriousIdeator {
    fn propose(
        &mut self,
        brief: &ResearchBrief,
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        let cand = self.inner.propose(brief, n * self.oversample, rng);
        if cand.len() <= n {
            return cand;
        }
        let m = cand.len();
        // Blend the inner ordering (exploit) with per-proposal curiosity.
        let mut scored: Vec<(f64, Hypothesis)> = cand
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                let exploit = if m > 1 {
                    1.0 - (i as f64) / (m as f64 - 1.0)
                } else {
                    1.0
                };
                let curiosity = self.engine.schedule_curiosity(&h.operators);
                (explore_exploit_priority(exploit, curiosity, self.lambda), h)
            })
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        scored.into_iter().take(n).map(|(_, h)| h).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(inst: &str, seq: &[&str], score: f64, seed: u64) -> ExperimentRecord {
        ExperimentRecord {
            instance_id: inst.into(),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
            seed,
            score,
            baseline: 0.0,
            density: 0.01,
            n: 100,
            clustering: 0.3,
            mean_degree: 4.0,
            ..Default::default()
        }
    }

    // Build a DB preserving each record's instance_id (record() would overwrite
    // it from the unset run context; from_records keeps it, like real data).
    fn db_of(recs: Vec<ExperimentRecord>) -> ExperimentDb {
        ExperimentDb::from_records(recs)
    }

    #[test]
    fn coverage_signal_favors_rarely_tried_operators() {
        // "common" appears 20×; "rare" appears once.
        let mut recs: Vec<ExperimentRecord> =
            (0..20).map(|i| rec("g", &["common"], -0.5, i)).collect();
        recs.push(rec("g", &["rare"], -0.5, 100));
        let db = db_of(recs);
        let e = CuriosityEngine::from_db(&db, None, &CuriosityConfig::default());
        assert!(
            e.operator_curiosity("rare") > e.operator_curiosity("common"),
            "rare {} should out-curious common {}",
            e.operator_curiosity("rare"),
            e.operator_curiosity("common")
        );
        // A never-seen operator is maximally curious.
        assert_eq!(e.operator_curiosity("never_tried"), 1.0);
    }

    #[test]
    fn anomaly_signal_favors_high_cross_seed_variance() {
        // "steady": same schedule/instance, identical outcome across seeds.
        // "wild":  same schedule/instance, wildly different outcome per seed.
        let mut recs: Vec<ExperimentRecord> =
            (0..8).map(|s| rec("inst", &["steady"], -0.5, s)).collect();
        for (s, sc) in [-0.1, -0.9, -0.2, -0.8, -0.3, -0.7, -0.15, -0.85]
            .iter()
            .enumerate()
        {
            recs.push(rec("inst", &["wild"], *sc, s as u64));
        }
        let db = db_of(recs);
        // Neutralize coverage/disagreement so anomaly is decisive.
        let cfg = CuriosityConfig {
            w_coverage: 0.0,
            w_disagreement: 0.0,
            w_anomaly: 1.0,
            lambda: 0.5,
        };
        let e = CuriosityEngine::from_db(&db, None, &cfg);
        assert!(
            e.operator_curiosity("wild") > e.operator_curiosity("steady"),
            "wild {} should out-curious steady {}",
            e.operator_curiosity("wild"),
            e.operator_curiosity("steady")
        );
    }

    #[test]
    fn explore_exploit_dial_interpolates() {
        // value high, curiosity low.
        assert!((explore_exploit_priority(1.0, 0.0, 0.0) - 1.0).abs() < 1e-12); // pure exploit
        assert!((explore_exploit_priority(1.0, 0.0, 1.0) - 0.0).abs() < 1e-12); // pure explore
        assert!((explore_exploit_priority(1.0, 0.0, 0.5) - 0.5).abs() < 1e-12); // blend
                                                                                // clamps out-of-range lambda.
        assert_eq!(explore_exploit_priority(1.0, 0.0, 2.0), 0.0);
    }

    // A deterministic inner ideator: proposal i uses operator ops[i], in a fixed
    // "exploit" order (best first).
    struct OrderedIdeator {
        templates: Vec<Vec<String>>,
        next: u64,
    }
    impl Ideator for OrderedIdeator {
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
                        operators: self.templates[i % self.templates.len()].clone(),
                        rationale: "ordered".into(),
                        reasoning: "ordered".into(),
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
    fn curious_ideator_lambda_controls_explore_vs_exploit() {
        use crate::engine_v2::decision::DecisionEngine;
        use crate::engine_v2::ir::ProblemIR;
        use crate::engine_v2::registry::OperatorRegistry;
        use rand::SeedableRng;

        // History: "safe" is heavily explored; "novel" barely.
        let mut recs: Vec<ExperimentRecord> =
            (0..30).map(|i| rec("g", &["safe"], -0.5, i)).collect();
        recs.push(rec("g", &["novel"], -0.5, 99));
        let db = db_of(recs);
        let engine = CuriosityEngine::from_db(&db, None, &CuriosityConfig::default());

        let ir = ProblemIR::from_pairs(4, 0.0, vec![0.0; 4], &[(0, 1, 1.0), (2, 3, -1.0)]);
        let reg = OperatorRegistry::standard();
        let stats = DecisionEngine::analyze(&ir);
        let edb = ExperimentDb::new();
        let analysis = super::super::scientist::AIScientist::analyze(&edb);
        let pool = DecisionEngine::operator_pool(&stats, stats.select_backend(), &reg);
        let graph = super::super::graph::KnowledgeGraph::new();
        let brief = ResearchBrief {
            stats: &stats,
            analysis: &analysis,
            graph: &graph,
            pool: &pool,
            registry: &reg,
            context: "t",
        };

        // Inner prefers "safe" (position 0) over "novel" (position 1).
        let templates = vec![vec!["safe".to_string()], vec!["novel".to_string()]];
        let mut rng = ChaCha8Rng::from_seed([1u8; 32]);

        // λ=0 (exploit): keeps the inner's favorite → "safe".
        let mut exploit = CuriousIdeator::new(
            Box::new(OrderedIdeator {
                templates: templates.clone(),
                next: 0,
            }),
            engine.clone(),
            0.0,
        );
        let out0 = exploit.propose(&brief, 1, &mut rng);
        assert_eq!(out0[0].operators, vec!["safe".to_string()]);

        // λ=1 (explore): the barely-tried "novel" wins on curiosity.
        let mut explore =
            CuriousIdeator::new(Box::new(OrderedIdeator { templates, next: 0 }), engine, 1.0);
        let out1 = explore.propose(&brief, 1, &mut rng);
        assert_eq!(
            out1[0].operators,
            vec!["novel".to_string()],
            "under full curiosity the unexplored operator should be chosen"
        );
    }
}
