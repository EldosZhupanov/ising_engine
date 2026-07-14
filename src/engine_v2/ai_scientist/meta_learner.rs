//! Meta-Learner (Stage 5+++) — the system studies its own ALGORITHMS, not the
//! task. It mines the append-only experiment database for statistically-supported
//! rules about operators, conditioned on graph structure:
//!
//!   "metropolis_sweep appears in 95% of the best solutions"
//!   "greedy_descent helps far more AFTER a thermal pass than first"
//!   "houdayer_cluster helps on sparse (+12) but hurts on dense (-3)"
//!   "random_flip_sweep is almost never in a top solution — avoid as first op"
//!
//! Then it writes a RESEARCH REPORT the LLM scientist reads to form the next
//! generation of hypotheses. Pure analysis over recorded history — it touches no
//! core component and changes nothing (Constitution §12).

use super::super::evolution::Schedule;
use super::super::knowledge::{InstanceFeatures, KnowledgeBase};
use super::db::ExperimentDb;
use super::graph::KnowledgeGraph;
use std::collections::{HashMap, HashSet};

/// The category of a mined rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    /// An operator dominates the best solutions.
    Dominance,
    /// An operator's effect depends on instance structure.
    Conditional,
    /// An operator works better in a particular ORDER.
    Ordering,
    /// An operator that consistently fails to help.
    Antipattern,
    /// An operator whose presence makes no measurable difference.
    Useless,
    /// The starting temperature changes an operator's effect.
    Temperature,
    /// More budget stops paying (or keeps paying) for an operator.
    Budget,
    /// One backend consistently outperforms another.
    Backend,
}

/// A learned, human-readable rule ABOUT ALGORITHMS with statistical backing.
#[derive(Debug, Clone)]
pub struct Rule {
    pub kind: RuleKind,
    pub statement: String,
    pub support: usize,
    pub confidence: f64,
}

impl Rule {
    /// Structural identity of a rule with the instance-specific numbers
    /// stripped — the unit of CROSS-INSTANCE reproducibility (Stage 6 eval).
    /// Parses the statement formats defined in this same file.
    pub fn signature(&self) -> String {
        let first = self.statement.split_whitespace().next().unwrap_or("?");
        match self.kind {
            RuleKind::Dominance => format!("Dominance:{first}"),
            RuleKind::Conditional => format!("Conditional:{first}"),
            RuleKind::Antipattern => format!("Antipattern:{first}"),
            RuleKind::Useless => format!("Useless:{first}"),
            RuleKind::Ordering => {
                let after = self
                    .statement
                    .split(" AFTER ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .unwrap_or("?");
                format!("Ordering:{first}<-{after}")
            }
            RuleKind::Temperature => {
                let dir = if self.statement.contains(" hotter") {
                    "hotter"
                } else {
                    "colder"
                };
                format!("Temperature:{first}:{dir}")
            }
            RuleKind::Budget => {
                let dir = if self.statement.contains("keeps paying") {
                    "scales"
                } else {
                    "saturates"
                };
                format!("Budget:{first}:{dir}")
            }
            RuleKind::Backend => {
                // "backend {win} outperforms {lose} ..."
                let mut it = self.statement.split_whitespace().skip(1);
                let win = it.next().unwrap_or("?");
                let lose = it.nth(1).unwrap_or("?");
                format!("Backend:{win}>{lose}")
            }
        }
    }
}

/// A hypothesized MISSING operator between two that frequently co-occur — the
/// seed of operator discovery (the scientist proposes; synthesis is future work).
#[derive(Debug, Clone)]
pub struct OperatorGap {
    pub predecessor: String,
    pub successor: String,
    pub suggested_name: String,
    pub rationale: String,
    pub support: usize,
}

/// Mines rules and writes reports from the experiment history.
#[derive(Debug, Clone)]
pub struct MetaLearner {
    /// Fraction of best/worst records that count as "top"/"bottom".
    pub top_fraction: f64,
    /// Minimum observations for a rule to be reported (reproducibility).
    pub min_support: usize,
}

impl Default for MetaLearner {
    fn default() -> Self {
        Self {
            top_fraction: 0.25,
            min_support: 5,
        }
    }
}

impl MetaLearner {
    pub fn new() -> Self {
        Self::default()
    }

    fn all_operators(db: &ExperimentDb) -> Vec<String> {
        let mut set: HashSet<&str> = HashSet::new();
        for r in db.all() {
            for op in &r.sequence {
                set.insert(op);
            }
        }
        let mut v: Vec<String> = set.into_iter().map(|s| s.to_string()).collect();
        v.sort();
        v
    }

    /// Mine all rule kinds, strongest first.
    pub fn mine(&self, db: &ExperimentDb) -> Vec<Rule> {
        let mut rules = Vec::new();
        rules.extend(self.dominance_rules(db));
        rules.extend(self.conditional_rules(db));
        rules.extend(self.ordering_rules(db));
        rules.extend(self.antipattern_rules(db));
        rules.extend(self.useless_rules(db));
        rules.extend(self.temperature_rules(db));
        rules.extend(self.budget_rules(db));
        rules.extend(self.backend_rules(db));
        // Strongest evidence first (support × confidence), deterministic tie-break.
        rules.sort_by(|a, b| {
            let sa = a.support as f64 * a.confidence;
            let sb = b.support as f64 * b.confidence;
            sb.total_cmp(&sa).then(a.statement.cmp(&b.statement))
        });
        rules
    }

    fn dominance_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        let n = db.len();
        if n < self.min_support * 2 {
            return Vec::new();
        }
        let mut sorted: Vec<&super::db::ExperimentRecord> = db.all().iter().collect();
        sorted.sort_by(|a, b| a.score.total_cmp(&b.score)); // best (lowest) first
        let k = ((n as f64 * self.top_fraction) as usize).max(self.min_support);
        let top = &sorted[..k.min(n)];
        let mut out = Vec::new();
        for op in Self::all_operators(db) {
            let in_top = top.iter().filter(|r| r.sequence.contains(&op)).count();
            let overall = db.all().iter().filter(|r| r.sequence.contains(&op)).count();
            let top_freq = in_top as f64 / top.len().max(1) as f64;
            let overall_freq = overall as f64 / n as f64;
            // Dominant if common in the best AND over-represented there.
            if in_top >= self.min_support && top_freq >= 0.6 && top_freq > overall_freq + 0.1 {
                out.push(Rule {
                    kind: RuleKind::Dominance,
                    statement: format!(
                        "{op} appears in {:.0}% of the best solutions (vs {:.0}% overall)",
                        top_freq * 100.0,
                        overall_freq * 100.0
                    ),
                    support: in_top,
                    confidence: top_freq,
                });
            }
        }
        out
    }

    /// Per-operator (sparse effect, dense effect, support) where both sides have
    /// enough observations — the shared basis for conditional rules and for
    /// publishing conditional facts into the knowledge graph.
    fn conditional_effects(&self, db: &ExperimentDb) -> Vec<(String, f64, f64, usize)> {
        // Bucket by density; rules emerge from CROSS-instance history.
        let sparse: Vec<_> = db.all().iter().filter(|r| r.density < 0.05).collect();
        let dense: Vec<_> = db.all().iter().filter(|r| r.density >= 0.05).collect();
        if sparse.len() < self.min_support || dense.len() < self.min_support {
            return Vec::new();
        }
        // Use RELATIVE improvement (fraction of baseline) so instances of very
        // different sizes/energy scales are comparable.
        let mean_rel = |recs: &[&super::db::ExperimentRecord], op: &str| -> Option<f64> {
            let vals: Vec<f64> = recs
                .iter()
                .filter(|r| r.sequence.iter().any(|o| o == op))
                .map(|r| r.rel_improvement())
                .collect();
            if vals.len() >= self.min_support {
                Some(vals.iter().sum::<f64>() / vals.len() as f64)
            } else {
                None
            }
        };
        let mut out = Vec::new();
        for op in Self::all_operators(db) {
            if let (Some(s), Some(d)) = (mean_rel(&sparse, &op), mean_rel(&dense, &op)) {
                let sup = sparse
                    .iter()
                    .chain(dense.iter())
                    .filter(|r| r.sequence.contains(&op))
                    .count();
                out.push((op, s, d, sup));
            }
        }
        out
    }

    fn conditional_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        let mut out = Vec::new();
        for (op, s, d, sup) in self.conditional_effects(db) {
            // Rule fires when the relative effect differs by ≥ 10 points.
            if (s - d).abs() > 0.10 {
                out.push(Rule {
                    kind: RuleKind::Conditional,
                    statement: format!(
                        "{op} changes energy by {:+.0}% of baseline on sparse vs {:+.0}% on dense",
                        s * 100.0,
                        d * 100.0
                    ),
                    support: sup,
                    confidence: (1.0 - (-((s - d).abs()) / 0.3).exp()).clamp(0.0, 1.0),
                });
            }
        }
        out
    }

    /// Publish CONSISTENT conditional findings into the knowledge graph as
    /// auditable facts ("op effective-on graph IF density<0.05", weight = mean
    /// relative improvement, proof = the experiment population). Only findings
    /// that pass the same significance gates as `mine` are written.
    pub fn publish(&self, db: &ExperimentDb, graph: &mut KnowledgeGraph) -> usize {
        let mut published = 0;
        for (op, s, d, sup) in self.conditional_effects(db) {
            if (s - d).abs() <= 0.10 {
                continue; // not a consistent structural difference
            }
            let proof = format!("meta-learner over {} experiments (support {sup})", db.len());
            graph.observe_if(&op, "effective-on", "graph", "density<0.05", s, &proof);
            graph.observe_if(&op, "effective-on", "graph", "density>=0.05", d, &proof);
            published += 2;
        }
        published
    }

    /// Runs of `db` containing `op`, as relative improvements.
    fn rel_with_op<'a>(
        db: &'a ExperimentDb,
        op: &str,
    ) -> impl Iterator<Item = &'a super::db::ExperimentRecord> {
        let op = op.to_string();
        db.all().iter().filter(move |r| r.sequence.contains(&op))
    }

    fn useless_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        let mut out = Vec::new();
        for op in Self::all_operators(db) {
            let with: Vec<f64> = Self::rel_with_op(db, &op)
                .map(|r| r.rel_improvement())
                .collect();
            let without: Vec<f64> = db
                .all()
                .iter()
                .filter(|r| !r.sequence.contains(&op))
                .map(|r| r.rel_improvement())
                .collect();
            if with.len() < self.min_support * 2 || without.len() < self.min_support * 2 {
                continue;
            }
            let mw = mean_of(&with);
            let mo = mean_of(&without);
            // Presence moves the outcome by < 2 points of baseline ⇒ useless.
            if (mw - mo).abs() < 0.02 {
                out.push(Rule {
                    kind: RuleKind::Useless,
                    statement: format!(
                        "{op} makes no measurable difference ({:+.1}% with vs {:+.1}% without)",
                        mw * 100.0,
                        mo * 100.0
                    ),
                    support: with.len(),
                    confidence: (1.0 - (-(with.len() as f64) / 20.0).exp()).clamp(0.0, 1.0),
                });
            }
        }
        out
    }

    fn temperature_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        let mut out = Vec::new();
        for op in Self::all_operators(db) {
            let mut pairs: Vec<(f64, f64)> = Self::rel_with_op(db, &op)
                .map(|r| (r.temp_hi, r.rel_improvement()))
                .collect();
            if pairs.len() < self.min_support * 2 {
                continue;
            }
            pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mid = pairs.len() / 2;
            let split = pairs[mid].0;
            let cold: Vec<f64> = pairs[..mid].iter().map(|p| p.1).collect();
            let hot: Vec<f64> = pairs[mid..].iter().map(|p| p.1).collect();
            if cold.len() < self.min_support || hot.len() < self.min_support {
                continue;
            }
            let (mc, mh) = (mean_of(&cold), mean_of(&hot));
            if (mh - mc).abs() > 0.05 {
                let dir = if mh > mc { "hotter" } else { "colder" };
                out.push(Rule {
                    kind: RuleKind::Temperature,
                    statement: format!(
                        "{op} works better {dir} (t_hi≥{split:.2}: {:+.1}% vs {:+.1}%)",
                        mh * 100.0,
                        mc * 100.0
                    ),
                    support: pairs.len(),
                    confidence: (1.0 - (-((mh - mc).abs()) / 0.15).exp()).clamp(0.0, 1.0),
                });
            }
        }
        out
    }

    fn budget_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        let mut out = Vec::new();
        for op in Self::all_operators(db) {
            let mut pairs: Vec<(f64, f64)> = Self::rel_with_op(db, &op)
                .map(|r| (r.sweeps.iter().sum::<u32>() as f64, r.rel_improvement()))
                .collect();
            if pairs.len() < self.min_support * 2 {
                continue;
            }
            pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mid = pairs.len() / 2;
            let split = pairs[mid].0;
            if pairs[0].0 == pairs[pairs.len() - 1].0 {
                continue; // no budget variation to learn from
            }
            let low: Vec<f64> = pairs[..mid].iter().map(|p| p.1).collect();
            let high: Vec<f64> = pairs[mid..].iter().map(|p| p.1).collect();
            if low.len() < self.min_support || high.len() < self.min_support {
                continue;
            }
            let (ml, mh) = (mean_of(&low), mean_of(&high));
            if mh > ml + 0.05 {
                out.push(Rule {
                    kind: RuleKind::Budget,
                    statement: format!(
                        "{op} keeps paying with budget (≥{split:.0} sweeps: {:+.1}% vs {:+.1}%)",
                        mh * 100.0,
                        ml * 100.0
                    ),
                    support: pairs.len(),
                    confidence: (1.0 - (-((mh - ml).abs()) / 0.15).exp()).clamp(0.0, 1.0),
                });
            } else if mh < ml + 0.01 {
                out.push(Rule {
                    kind: RuleKind::Budget,
                    statement: format!(
                        "{op} shows diminishing returns past {split:.0} sweeps ({:+.1}% vs {:+.1}%)",
                        mh * 100.0,
                        ml * 100.0
                    ),
                    support: pairs.len(),
                    confidence: (1.0 - (-(pairs.len() as f64) / 20.0).exp()).clamp(0.0, 1.0),
                });
            }
        }
        out
    }

    fn backend_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        let mut by_backend: HashMap<&str, Vec<f64>> = HashMap::new();
        for r in db.all() {
            by_backend
                .entry(r.backend.as_str())
                .or_default()
                .push(r.rel_improvement());
        }
        let mut names: Vec<&&str> = by_backend.keys().collect();
        names.sort();
        let mut out = Vec::new();
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                let (a, b) = (*names[i], *names[j]);
                let (va, vb) = (&by_backend[a], &by_backend[b]);
                if va.len() < self.min_support || vb.len() < self.min_support {
                    continue;
                }
                let (ma, mb) = (mean_of(va), mean_of(vb));
                if (ma - mb).abs() > 0.05 {
                    let (win, lose, mw, ml) = if ma > mb {
                        (a, b, ma, mb)
                    } else {
                        (b, a, mb, ma)
                    };
                    out.push(Rule {
                        kind: RuleKind::Backend,
                        statement: format!(
                            "backend {win} outperforms {lose} ({:+.1}% vs {:+.1}% of baseline)",
                            mw * 100.0,
                            ml * 100.0
                        ),
                        support: va.len().min(vb.len()),
                        confidence: (1.0 - (-((ma - mb).abs()) / 0.15).exp()).clamp(0.0, 1.0),
                    });
                }
            }
        }
        out
    }

    fn ordering_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        // Mean improvement when X is FIRST, vs when X immediately follows some Y.
        let mut first_imp: HashMap<String, Vec<f64>> = HashMap::new();
        let mut after_imp: HashMap<(String, String), Vec<f64>> = HashMap::new();
        for r in db.all() {
            if let Some(first) = r.sequence.first() {
                first_imp
                    .entry(first.clone())
                    .or_default()
                    .push(r.improvement());
            }
            for w in r.sequence.windows(2) {
                after_imp
                    .entry((w[1].clone(), w[0].clone())) // (X, Y): X after Y
                    .or_default()
                    .push(r.improvement());
            }
        }
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
        let mut out = Vec::new();
        for ((x, y), after) in &after_imp {
            if after.len() < self.min_support {
                continue;
            }
            let first = first_imp.get(x);
            let first_mean = first.map(|v| mean(v)).unwrap_or(0.0);
            let after_mean = mean(after);
            if after_mean > first_mean + 3.0 {
                out.push(Rule {
                    kind: RuleKind::Ordering,
                    statement: format!(
                        "{x} works better AFTER {y} ({after_mean:+.1}) than as the first operator ({first_mean:+.1})"
                    ),
                    support: after.len(),
                    confidence: (1.0 - (-(after_mean - first_mean) / 8.0).exp()).clamp(0.0, 1.0),
                });
            }
        }
        // Deterministic order.
        out.sort_by(|a, b| a.statement.cmp(&b.statement));
        out
    }

    fn antipattern_rules(&self, db: &ExperimentDb) -> Vec<Rule> {
        let n = db.len();
        if n < self.min_support * 2 {
            return Vec::new();
        }
        let mut sorted: Vec<&super::db::ExperimentRecord> = db.all().iter().collect();
        sorted.sort_by(|a, b| a.score.total_cmp(&b.score));
        let k = ((n as f64 * self.top_fraction) as usize).max(self.min_support);
        let top = &sorted[..k.min(n)];
        let bottom = &sorted[n.saturating_sub(k)..];
        let mut out = Vec::new();
        for op in Self::all_operators(db) {
            let in_top = top.iter().filter(|r| r.sequence.contains(&op)).count();
            let in_bottom = bottom.iter().filter(|r| r.sequence.contains(&op)).count();
            // Appears often in the worst solutions, rarely in the best.
            if in_bottom >= self.min_support && in_bottom >= in_top * 2 && in_top * 4 < top.len() {
                out.push(Rule {
                    kind: RuleKind::Antipattern,
                    statement: format!(
                        "{op} appears in the worst solutions {}x more than the best — a weak choice here",
                        if in_top == 0 { in_bottom } else { in_bottom / in_top.max(1) }
                    ),
                    support: in_bottom,
                    confidence: in_bottom as f64 / (in_bottom + in_top).max(1) as f64,
                });
            }
        }
        out
    }

    /// Suggest a missing intermediate operator between the most-common adjacent
    /// pair among the best solutions (operator-discovery seed).
    pub fn suggest_operator_gap(&self, db: &ExperimentDb) -> Option<OperatorGap> {
        let n = db.len();
        if n < self.min_support * 2 {
            return None;
        }
        let mut sorted: Vec<&super::db::ExperimentRecord> = db.all().iter().collect();
        sorted.sort_by(|a, b| a.score.total_cmp(&b.score));
        let k = ((n as f64 * self.top_fraction) as usize).max(self.min_support);
        let mut pair_count: HashMap<(String, String), usize> = HashMap::new();
        for r in &sorted[..k.min(n)] {
            for w in r.sequence.windows(2) {
                *pair_count.entry((w[0].clone(), w[1].clone())).or_insert(0) += 1;
            }
        }
        let (pair, count) = pair_count
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))?;
        if count < self.min_support {
            return None;
        }
        let (pre, suc) = pair;
        let short = |s: &str| s.split('_').next().unwrap_or(s).to_string();
        Some(OperatorGap {
            suggested_name: format!("{}_{}", short(&pre), short(&suc)),
            rationale: format!(
                "{pre}→{suc} is the most frequent transition in the best solutions ({count}x); \
a fused operator bridging them may capture the effect in one cheaper step"
            ),
            predecessor: pre,
            successor: suc,
            support: count,
        })
    }

    /// Predict a good schedule for a NEW instance by transferring the best-known
    /// algorithm from the most similar past instances (a learned predictor over
    /// recorded performance — the realistic version of "a net that predicts
    /// algorithms"). `None` if the knowledge base has nothing comparable.
    pub fn recommend(&self, kb: &KnowledgeBase, features: &InstanceFeatures) -> Option<Schedule> {
        kb.similar(features, 5)
            .into_iter()
            .min_by(|a, b| a.best_energy.total_cmp(&b.best_energy))
            .map(|e| Schedule {
                ops: e.sequence.clone(),
                sweeps: e.sweeps.clone(),
                temp_hi: e.temp_hi,
                temp_lo: e.temp_lo,
            })
    }

    /// Write a research report: the findings, the strongest knowledge-graph
    /// edges, and a suggested new direction. This is what the LLM scientist reads
    /// to reason about the NEXT generation of ideas.
    pub fn report(&self, db: &ExperimentDb, graph: &KnowledgeGraph) -> String {
        let mut r = String::new();
        r.push_str("# Research Report\n\n");
        r.push_str(&format!("Experiments analyzed: {}\n", db.len()));
        if let Some(best) = db.best() {
            r.push_str(&format!(
                "Best solution: {:?}  (score {:.1}, baseline {:.1})\n",
                best.sequence, best.score, best.baseline
            ));
        }
        r.push_str("\n## Findings (statistically supported)\n");
        let rules = self.mine(db);
        if rules.is_empty() {
            r.push_str("- (not enough data yet to draw supported conclusions)\n");
        }
        for rule in rules.iter().take(12) {
            r.push_str(&format!(
                "- [{:?}] {} (support {}, confidence {:.2})\n",
                rule.kind, rule.statement, rule.support, rule.confidence
            ));
        }
        if !graph.is_empty() {
            r.push_str("\n## Knowledge graph — strongest learned transitions\n");
            let mut edges: Vec<_> = graph
                .triples()
                .iter()
                .filter(|t| t.predicate == "precedes-well")
                .collect();
            edges.sort_by(|a, b| b.weight.total_cmp(&a.weight));
            for t in edges.iter().take(6) {
                r.push_str(&format!(
                    "- {} → {} (weight {:+.1}, support {})\n",
                    t.subject, t.object, t.weight, t.support
                ));
            }
        }
        if let Some(gap) = self.suggest_operator_gap(db) {
            r.push_str("\n## Suggested new direction (operator discovery)\n");
            r.push_str(&format!(
                "- Missing operator `{}` between {} and {}: {}\n",
                gap.suggested_name, gap.predecessor, gap.successor, gap.rationale
            ));
        }
        r
    }
}

fn mean_of(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(seq: &[&str], score: f64, density: f64) -> ExperimentRecord {
        ExperimentRecord {
            id: 0,
            hypothesis_id: None,
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
            num_replicas: 8,
            seed: 1,
            backend: "SparseBitSlice".into(),
            work: 1000.0,
            score,
            baseline: 0.0,
            density,
            clustering: 0.3,
            ..Default::default()
        }
    }

    #[test]
    fn mines_dominance_of_a_winning_operator() {
        let mut db = ExperimentDb::new();
        // metropolis is in every strong solution; random_flip only in weak ones.
        for i in 0..20 {
            db.record(rec(
                &["metropolis_sweep", "greedy_descent"],
                -100.0 - i as f64,
                0.01,
            ));
        }
        for i in 0..20 {
            db.record(rec(&["random_flip_sweep"], -10.0 + i as f64, 0.01));
        }
        let rules = MetaLearner::new().mine(&db);
        assert!(
            rules
                .iter()
                .any(|r| r.kind == RuleKind::Dominance && r.statement.contains("metropolis_sweep")),
            "should discover metropolis dominance: {rules:?}"
        );
        // And flag random_flip as an antipattern (worst solutions).
        assert!(rules
            .iter()
            .any(|r| r.kind == RuleKind::Antipattern && r.statement.contains("random_flip_sweep")));
    }

    #[test]
    fn mines_structure_conditional_rule() {
        let mut db = ExperimentDb::new();
        // houdayer great on sparse, poor on dense.
        for _ in 0..8 {
            db.record(rec(&["houdayer_cluster"], -50.0, 0.01)); // improvement +50
            db.record(rec(&["houdayer_cluster"], -3.0, 0.20)); // improvement +3
        }
        let rules = MetaLearner::new().mine(&db);
        assert!(
            rules.iter().any(
                |r| r.kind == RuleKind::Conditional && r.statement.contains("houdayer_cluster")
            ),
            "should discover the sparse/dense conditional: {rules:?}"
        );
    }

    #[test]
    fn mines_useless_operator() {
        let mut db = ExperimentDb::new();
        for _ in 0..12 {
            db.record(rec(&["metropolis_sweep", "noop_shuffle"], -0.10, 0.01));
            db.record(rec(&["greedy_descent"], -0.10, 0.01));
        }
        let rules = MetaLearner::new().mine(&db);
        assert!(
            rules
                .iter()
                .any(|r| r.kind == RuleKind::Useless && r.statement.contains("noop_shuffle")),
            "noop_shuffle should be flagged useless: {rules:?}"
        );
    }

    #[test]
    fn mines_temperature_and_budget_influence() {
        let mut db = ExperimentDb::new();
        for i in 0..12 {
            // Hot runs help much more than cold ones; long runs beat short.
            let hot = i % 2 == 0;
            db.record(ExperimentRecord {
                temp_hi: if hot { 8.0 } else { 0.5 },
                sweeps: vec![if hot { 40 } else { 10 }],
                score: if hot { -0.30 } else { -0.05 },
                ..rec(&["anneal_op"], 0.0, 0.01)
            });
        }
        let rules = MetaLearner::new().mine(&db);
        assert!(
            rules.iter().any(|r| r.kind == RuleKind::Temperature
                && r.statement.contains("anneal_op")
                && r.statement.contains("hotter")),
            "temperature rule missing: {rules:?}"
        );
        assert!(
            rules.iter().any(|r| r.kind == RuleKind::Budget
                && r.statement.contains("anneal_op")
                && r.statement.contains("keeps paying")),
            "budget rule missing: {rules:?}"
        );
    }

    #[test]
    fn mines_backend_difference() {
        let mut db = ExperimentDb::new();
        for _ in 0..8 {
            db.record(ExperimentRecord {
                backend: "SparseBitSlice".into(),
                ..rec(&["op_a"], -0.20, 0.01)
            });
            db.record(ExperimentRecord {
                backend: "Reference".into(),
                ..rec(&["op_a"], -0.02, 0.01)
            });
        }
        let rules = MetaLearner::new().mine(&db);
        assert!(
            rules.iter().any(|r| r.kind == RuleKind::Backend
                && r.statement.contains("SparseBitSlice outperforms Reference")),
            "backend rule missing: {rules:?}"
        );
    }

    #[test]
    fn publish_writes_conditional_facts_with_proof() {
        let mut db = ExperimentDb::new();
        for _ in 0..8 {
            db.record(rec(&["houdayer_cluster"], -0.50, 0.01));
            db.record(rec(&["houdayer_cluster"], -0.03, 0.20));
        }
        let mut graph = KnowledgeGraph::new();
        let n = MetaLearner::new().publish(&db, &mut graph);
        assert_eq!(n, 2);
        let sparse = graph.query_applicable("houdayer_cluster", "effective-on", 0.01, 0.3);
        assert_eq!(sparse.len(), 1);
        assert_eq!(sparse[0].condition, "density<0.05");
        assert!(sparse[0].proof.contains("meta-learner over 16 experiments"));
        assert!(sparse[0].weight > 0.4);
        let dense = graph.query_applicable("houdayer_cluster", "effective-on", 0.2, 0.3);
        assert!(dense[0].weight < 0.1);
    }

    #[test]
    fn report_and_gap_are_produced() {
        let mut db = ExperimentDb::new();
        for i in 0..24 {
            db.record(rec(
                &["gibbs_color_sweep", "greedy_descent"],
                -100.0 - i as f64,
                0.01,
            ));
        }
        let report = MetaLearner::new().report(&db, &KnowledgeGraph::new());
        assert!(report.contains("# Research Report"));
        assert!(report.contains("Findings"));
        let gap = MetaLearner::new().suggest_operator_gap(&db).unwrap();
        assert_eq!(gap.predecessor, "gibbs_color_sweep");
        assert_eq!(gap.successor, "greedy_descent");
        assert_eq!(gap.suggested_name, "gibbs_greedy");
    }
}
