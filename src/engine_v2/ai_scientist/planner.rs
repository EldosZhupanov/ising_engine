//! Research Planner (Stage 8, Pillar III — the loop sets its OWN tasks).
//!
//! Until now a human (or Claude Code) chose what to study; the Orchestrator
//! cycled instances round-robin. That is not a researcher — a researcher wakes
//! up and decides where its attention is best spent. This module is that
//! decision. It does NOT ask "what will yield the most energy"; it asks "what
//! will yield the most NEW KNOWLEDGE," and hands the Orchestrator a ranked
//! agenda with a reason for every item.
//!
//! Two decisions:
//!   - `next_target` — which INSTANCE / regime to study next, by expected new
//!     knowledge = coverage deficit (how little we have measured it) +
//!     structural novelty (how unlike anything we have already studied it is).
//!   - `agenda` — for a chosen instance, which SCHEDULES to try, ranked by the
//!     explore/exploit blend of the Predictor's expected value and the Curiosity
//!     Engine's information gain.
//!
//! It computes no energy and runs nothing — it only decides, and records why.

use super::super::evolution::Schedule;
use super::curiosity::{explore_exploit_priority, CuriosityEngine};
use super::db::ExperimentDb;
use super::predictor::{InstanceSignature, Predictor};
use std::collections::BTreeMap;

fn sig_feats(s: &InstanceSignature) -> [f64; 5] {
    [
        ((s.n as f64) + 1.0).ln() / 10.0,
        s.density,
        s.clustering,
        s.mean_degree / 10.0,
        s.degree_cv,
    ]
}

fn dist(a: &[f64; 5], b: &[f64; 5]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// A concrete research task the platform chose to do, with its justification.
#[derive(Debug, Clone)]
pub struct PlannedTask {
    pub instance_id: String,
    pub schedule: Schedule,
    /// Blended priority (higher = do sooner).
    pub priority: f64,
    /// Predictor's expected relative improvement (exploitation term).
    pub value: f64,
    /// Curiosity Engine's information gain (exploration term).
    pub info_gain: f64,
    pub reason: String,
}

/// How a candidate instance scored for "expected new knowledge".
#[derive(Debug, Clone)]
pub struct TargetScore {
    pub instance_id: String,
    pub coverage_deficit: f64,
    pub novelty: f64,
    pub score: f64,
    pub reason: String,
}

/// Chooses what to research next.
#[derive(Debug, Clone, Copy)]
pub struct ResearchPlanner {
    /// Explore/exploit blend for the schedule agenda (0 exploit, 1 explore).
    pub lambda: f64,
    /// Weight of coverage deficit vs structural novelty in target selection.
    pub w_coverage: f64,
    pub w_novelty: f64,
}

impl Default for ResearchPlanner {
    fn default() -> Self {
        Self {
            lambda: 0.5,
            w_coverage: 1.0,
            w_novelty: 1.0,
        }
    }
}

impl ResearchPlanner {
    /// Score every candidate instance by expected new knowledge and return them
    /// ranked (most-informative first). The platform's answer to "what should I
    /// study today, and why?".
    pub fn rank_targets(
        &self,
        candidates: &[(String, InstanceSignature)],
        db: &ExperimentDb,
    ) -> Vec<TargetScore> {
        // Coverage: experiments already recorded per instance id.
        let mut coverage: BTreeMap<&str, usize> = BTreeMap::new();
        for r in db.all() {
            if !r.instance_id.is_empty() {
                *coverage.entry(r.instance_id.as_str()).or_insert(0) += 1;
            }
        }
        // Feature vectors of everything already studied (for novelty).
        let mut studied: Vec<[f64; 5]> = Vec::new();
        let mut seen_inst: BTreeMap<&str, [f64; 5]> = BTreeMap::new();
        for r in db.all() {
            if !r.instance_id.is_empty() {
                seen_inst.entry(r.instance_id.as_str()).or_insert_with(|| {
                    [
                        ((r.n as f64) + 1.0).ln() / 10.0,
                        r.density,
                        r.clustering,
                        r.mean_degree / 10.0,
                        r.degree_cv,
                    ]
                });
            }
        }
        studied.extend(seen_inst.values().copied());

        let mut out: Vec<TargetScore> = candidates
            .iter()
            .map(|(id, sig)| {
                let cov = coverage.get(id.as_str()).copied().unwrap_or(0);
                let coverage_deficit = 1.0 / (1.0 + cov as f64);
                let f = sig_feats(sig);
                // Novelty = distance to the NEAREST already-studied structural
                // signature. A signature we have studied sits ~0 from itself ⇒
                // NOT novel; an unlike, unstudied regime sits far ⇒ novel. With
                // nothing studied yet, everything is new.
                let nearest = studied
                    .iter()
                    .map(|s| dist(&f, s))
                    .fold(f64::INFINITY, f64::min);
                let novelty = if nearest.is_finite() {
                    // squash to [0,1): far instances saturate toward 1.
                    nearest / (1.0 + nearest)
                } else {
                    1.0 // nothing studied ⇒ everything is new
                };
                let score = self.w_coverage * coverage_deficit + self.w_novelty * novelty;
                TargetScore {
                    instance_id: id.clone(),
                    coverage_deficit,
                    novelty,
                    score,
                    reason: format!(
                        "coverage deficit {coverage_deficit:.2} ({cov} experiments), structural novelty {novelty:.2}"
                    ),
                }
            })
            .collect();
        out.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.instance_id.cmp(&b.instance_id))
        });
        out
    }

    /// Pick the single next instance to study (index into `candidates`), or
    /// `None` if there are no candidates. This is the loop's self-set task.
    pub fn next_target(
        &self,
        candidates: &[(String, InstanceSignature)],
        db: &ExperimentDb,
    ) -> Option<usize> {
        if candidates.is_empty() {
            return None;
        }
        let ranked = self.rank_targets(candidates, db);
        let best = &ranked[0].instance_id;
        candidates.iter().position(|(id, _)| id == best)
    }

    /// Build the schedule agenda for a chosen instance: rank `candidates` by the
    /// explore/exploit blend of predicted value and curiosity, keep top `k`.
    /// This is "today: k hypotheses, and here is why each earns its slot."
    pub fn agenda(
        &self,
        instance_id: &str,
        sig: &InstanceSignature,
        candidates: &[Schedule],
        curiosity: &CuriosityEngine,
        predictor: Option<&Predictor>,
        k: usize,
    ) -> Vec<PlannedTask> {
        if candidates.is_empty() {
            return Vec::new();
        }
        // Raw value + info gain per candidate.
        let raw: Vec<(f64, f64)> = candidates
            .iter()
            .map(|s| {
                let value = predictor.map(|p| p.predict(sig, s)).unwrap_or(0.0);
                let info = curiosity.schedule_curiosity(&s.ops);
                (value, info)
            })
            .collect();
        // Min-max normalize value across candidates so λ is meaningful.
        let vlo = raw.iter().map(|(v, _)| *v).fold(f64::INFINITY, f64::min);
        let vhi = raw
            .iter()
            .map(|(v, _)| *v)
            .fold(f64::NEG_INFINITY, f64::max);
        let vspan = vhi - vlo;

        let mut tasks: Vec<PlannedTask> = candidates
            .iter()
            .zip(&raw)
            .map(|(s, &(value, info))| {
                let vnorm = if vspan > 1e-12 { (value - vlo) / vspan } else { 0.5 };
                let priority = explore_exploit_priority(vnorm, info, self.lambda);
                PlannedTask {
                    instance_id: instance_id.to_string(),
                    schedule: s.clone(),
                    priority,
                    value,
                    info_gain: info,
                    reason: format!(
                        "priority {priority:.2} = (1−λ)·value({vnorm:.2}) + λ·info-gain({info:.2}), λ={:.2}",
                        self.lambda
                    ),
                }
            })
            .collect();
        tasks.sort_by(|a, b| b.priority.total_cmp(&a.priority));
        tasks.truncate(k.max(1));
        tasks
    }

    /// Generate a bounded default candidate set from an operator pool: each
    /// operator alone, plus pairs of the two most-curious operators — enough
    /// for the agenda to have something to rank without exploding.
    pub fn default_candidates(
        pool: &[String],
        curiosity: &CuriosityEngine,
        sweeps: u32,
    ) -> Vec<Schedule> {
        let one = |ops: Vec<String>| Schedule {
            sweeps: vec![sweeps; ops.len()],
            ops,
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        let mut cands: Vec<Schedule> = pool.iter().map(|op| one(vec![op.clone()])).collect();
        // Two most-curious operators, paired.
        let mut by_cur: Vec<(&String, f64)> = pool
            .iter()
            .map(|op| (op, curiosity.operator_curiosity(op)))
            .collect();
        by_cur.sort_by(|a, b| b.1.total_cmp(&a.1));
        if by_cur.len() >= 2 {
            cands.push(one(vec![by_cur[0].0.clone(), by_cur[1].0.clone()]));
        }
        cands
    }
}

#[cfg(test)]
mod tests {
    use super::super::curiosity::CuriosityConfig;
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(inst: &str, n: usize, density: f64, seq: &[&str], score: f64) -> ExperimentRecord {
        ExperimentRecord {
            instance_id: inst.into(),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            score,
            baseline: 0.0,
            n,
            density,
            clustering: 0.3,
            mean_degree: 4.0,
            ..Default::default()
        }
    }

    fn sig(n: usize, density: f64) -> InstanceSignature {
        InstanceSignature {
            n,
            density,
            clustering: 0.3,
            mean_degree: 4.0,
            degree_cv: 0.0,
        }
    }

    #[test]
    fn planner_chooses_the_least_known_instance() {
        // "well_studied" has 40 experiments; "barely" has 1; "unseen" has 0 and
        // is structurally distinct. The planner should prefer the unknown.
        let mut recs: Vec<ExperimentRecord> = (0..40)
            .map(|_| rec("well_studied", 800, 0.01, &["metropolis_sweep"], -0.5))
            .collect();
        recs.push(rec("barely", 800, 0.01, &["greedy_descent"], -0.5));
        let db = ExperimentDb::from_records(recs);

        let candidates = vec![
            ("well_studied".to_string(), sig(800, 0.01)),
            ("barely".to_string(), sig(800, 0.011)),
            ("unseen".to_string(), sig(2000, 0.3)), // new size + density
        ];
        let planner = ResearchPlanner::default();
        let ranked = planner.rank_targets(&candidates, &db);
        assert_eq!(
            ranked[0].instance_id, "unseen",
            "the unseen, structurally-novel instance is the most informative: {ranked:?}"
        );
        assert!(
            ranked
                .iter()
                .position(|t| t.instance_id == "well_studied")
                .unwrap()
                > 0,
            "the well-studied instance should rank last-ish"
        );
        // next_target picks it.
        let idx = planner.next_target(&candidates, &db).unwrap();
        assert_eq!(candidates[idx].0, "unseen");
    }

    #[test]
    fn agenda_blends_value_and_info_gain_on_lambda() {
        // History: "safe" heavily explored, "novel" barely.
        let mut recs: Vec<ExperimentRecord> = (0..30)
            .map(|_| rec("g", 800, 0.01, &["safe"], -0.5))
            .collect();
        recs.push(rec("g", 800, 0.01, &["novel"], -0.5));
        let db = ExperimentDb::from_records(recs);
        let curiosity = CuriosityEngine::from_db(&db, None, &CuriosityConfig::default());

        let mk = |op: &str| Schedule {
            ops: vec![op.to_string()],
            sweeps: vec![10],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        let cands = vec![mk("safe"), mk("novel")];

        // λ=1 (pure explore) ⇒ the barely-tried "novel" tops the agenda.
        let explore = ResearchPlanner {
            lambda: 1.0,
            ..Default::default()
        };
        let ag = explore.agenda("g", &sig(800, 0.01), &cands, &curiosity, None, 2);
        assert_eq!(ag[0].schedule.ops, vec!["novel".to_string()]);
        assert!(ag[0].reason.contains("info-gain"));
    }

    #[test]
    fn default_candidates_covers_pool_plus_a_curious_pair() {
        let db = ExperimentDb::from_records(vec![rec("g", 800, 0.01, &["a"], -0.5)]);
        let cur = CuriosityEngine::from_db(&db, None, &CuriosityConfig::default());
        let pool = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let cands = ResearchPlanner::default_candidates(&pool, &cur, 16);
        // 3 singles + 1 pair.
        assert_eq!(cands.len(), 4);
        assert!(cands.iter().any(|s| s.ops.len() == 2));
    }
}
