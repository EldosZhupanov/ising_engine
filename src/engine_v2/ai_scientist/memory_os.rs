//! Scientific Memory + Memory Manager (Stage 8, Pillars II).
//!
//! The DB *stores* experiments; nobody *manages* the store, and nobody can ask
//! it the human question — "*have I seen a structure like this before, and what
//! worked?*". This module adds both faces of a managed memory:
//!
//!   - SCIENTIFIC MEMORY (`recall`) — semantic, structure-keyed recall. Given a
//!     new instance's structural signature, it retrieves what happened on the
//!     structurally-SIMILAR instances the platform has ever seen (across every
//!     family and campaign) and the conditional knowledge that applies, and
//!     narrates it: "seen 14 similar instances; parallel tempering worked best
//!     there, and only where clustering was high."
//!   - MEMORY MANAGER (`MemoryManager::analyze`) — indexes the store by
//!     structural bucket, reports what is well-characterized enough to COMPACT
//!     into a summary, and what remains unknown. It NEVER deletes — the
//!     append-only stream is the source of truth (§12); compaction produces a
//!     summary that can stand in for detail in a bounded LLM context, while the
//!     raw record is kept forever.

use super::db::ExperimentDb;
use super::graph::{condition_holds, KnowledgeGraph};
use super::predictor::InstanceSignature;
use std::collections::{BTreeMap, BTreeSet};

/// regime label → (experiment count, distinct instances, per-operator (Σ rel, n)).
type RegimeAgg = BTreeMap<&'static str, (usize, BTreeSet<String>, BTreeMap<String, (f64, usize)>)>;

/// The 5-feature structural encoding shared with the learned models, so
/// "similarity" here means the same thing it means to the Policy/Predictor.
fn feats(n: usize, density: f64, clustering: f64, mean_degree: f64, degree_cv: f64) -> [f64; 5] {
    [
        ((n as f64) + 1.0).ln() / 10.0,
        density,
        clustering,
        mean_degree / 10.0,
        degree_cv,
    ]
}

fn sig_feats(s: &InstanceSignature) -> [f64; 5] {
    feats(s.n, s.density, s.clustering, s.mean_degree, s.degree_cv)
}

fn distance(a: &[f64; 5], b: &[f64; 5]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// What the platform recalls about a structurally-similar situation.
#[derive(Debug, Clone)]
pub struct Recollection {
    /// Distinct instances within the similarity radius.
    pub similar_instances: Vec<String>,
    /// Operators that worked best on those instances, mean rel improvement first.
    pub best_operators: Vec<(String, f64)>,
    /// Conditional knowledge (graph facts) whose condition applies to this
    /// signature — rules AND survived theories.
    pub applicable_facts: Vec<String>,
    /// The human narrative — "I've seen this before…".
    pub narrative: String,
}

/// SCIENTIFIC MEMORY: recall what happened on instances structurally similar to
/// `sig`, from the entire history (`radius` in feature-distance units; instances
/// within it are "similar"). Draws on the append-only DB and the knowledge graph.
pub fn recall(
    db: &ExperimentDb,
    graph: &KnowledgeGraph,
    sig: &InstanceSignature,
    radius: f64,
) -> Recollection {
    let target = sig_feats(sig);

    // Group records by instance; each instance's feature vector from any record.
    let mut inst_feats: BTreeMap<String, [f64; 5]> = BTreeMap::new();
    for r in db.all() {
        if !r.instance_id.is_empty() {
            inst_feats
                .entry(r.instance_id.clone())
                .or_insert_with(|| feats(r.n, r.density, r.clustering, r.mean_degree, r.degree_cv));
        }
    }
    let similar: Vec<String> = inst_feats
        .iter()
        .filter(|(_, f)| distance(&target, f) <= radius)
        .map(|(name, _)| name.clone())
        .collect();

    // Best operators on the similar instances (mean rel improvement per op).
    let mut op_acc: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for r in db.all() {
        if !similar.contains(&r.instance_id) {
            continue;
        }
        for op in &r.sequence {
            let e = op_acc.entry(op.clone()).or_insert((0.0, 0));
            e.0 += r.rel_improvement();
            e.1 += 1;
        }
    }
    let mut best_operators: Vec<(String, f64)> = op_acc
        .into_iter()
        .map(|(op, (s, n))| (op, s / n.max(1) as f64))
        .collect();
    best_operators.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));

    // Conditional knowledge that applies to this structure.
    let mut applicable_facts: Vec<String> = graph
        .triples()
        .iter()
        .filter(|t| condition_holds(&t.condition, sig.density, sig.clustering))
        .filter(|t| !t.condition.is_empty()) // only the CONDITIONAL knowledge
        .map(|t| {
            format!(
                "{} --{}--> {} IF {} (w {:+.2}, conf {:.2})",
                t.subject,
                t.predicate,
                t.object,
                t.condition,
                t.weight,
                t.confidence()
            )
        })
        .collect();
    applicable_facts.sort();
    applicable_facts.dedup();
    applicable_facts.truncate(8);

    // Narrative.
    let mut narrative = String::new();
    if similar.is_empty() {
        narrative.push_str(
            "No structurally-similar instance in memory yet — this regime is new; explore it.",
        );
    } else {
        narrative.push_str(&format!(
            "Seen {} structurally-similar instance(s). ",
            similar.len()
        ));
        if let Some((op, imp)) = best_operators.first() {
            narrative.push_str(&format!(
                "There, `{op}` worked best (mean {:+.1}% of baseline). ",
                imp * 100.0
            ));
        }
        if let Some(fact) = applicable_facts.first() {
            narrative.push_str(&format!("Applicable rule: {fact}."));
        }
    }

    Recollection {
        similar_instances: similar,
        best_operators,
        applicable_facts,
        narrative,
    }
}

/// A structural bucket of the memory and how well-characterized it is.
#[derive(Debug, Clone)]
pub struct Bucket {
    pub label: String,
    pub experiments: usize,
    pub instances: usize,
    /// True if there is enough data + a stable dominant operator that the raw
    /// detail could be COMPACTED into a summary (the summary stands in; the raw
    /// stream is still kept — nothing is deleted).
    pub compactable: bool,
    pub dominant_operator: Option<String>,
}

/// The Memory Manager's report over the whole store.
#[derive(Debug, Clone)]
pub struct MemoryReport {
    pub total_experiments: usize,
    pub distinct_instances: usize,
    pub buckets: Vec<Bucket>,
    pub note: String,
}

/// MEMORY MANAGER: index + retention analysis over the append-only store. It
/// buckets by structural regime, flags what is well-enough characterized to
/// compact, and — critically — never proposes deletion.
pub struct MemoryManager {
    /// Experiments in a bucket beyond which it is a compaction candidate.
    pub compact_threshold: usize,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            compact_threshold: 200,
        }
    }
}

/// The canonical structural-regime label for an instance's edge density. This is
/// the SINGLE source of truth for the density bands, shared by the Memory Manager
/// (bucketing) and the Research Executive (per-situation model selection), so the
/// two never disagree about what "sparse" means.
pub(crate) fn regime_label(density: f64) -> &'static str {
    if density < 0.01 {
        "very-sparse"
    } else if density < 0.05 {
        "sparse"
    } else if density < 0.2 {
        "medium"
    } else {
        "dense"
    }
}

impl MemoryManager {
    fn bucket_label(density: f64) -> &'static str {
        regime_label(density)
    }

    pub fn analyze(&self, db: &ExperimentDb) -> MemoryReport {
        // regime -> (experiments, instances set, per-op improvement)
        let mut agg: RegimeAgg = BTreeMap::new();
        for r in db.all() {
            let label = Self::bucket_label(r.density);
            let e = agg.entry(label).or_default();
            e.0 += 1;
            if !r.instance_id.is_empty() {
                e.1.insert(r.instance_id.clone());
            }
            for op in &r.sequence {
                let o = e.2.entry(op.clone()).or_insert((0.0, 0));
                o.0 += r.rel_improvement();
                o.1 += 1;
            }
        }
        let mut buckets = Vec::new();
        for (label, (exps, insts, ops)) in &agg {
            let dominant = ops
                .iter()
                .map(|(op, (s, n))| (op.clone(), s / *n as f64))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(op, _)| op);
            buckets.push(Bucket {
                label: label.to_string(),
                experiments: *exps,
                instances: insts.len(),
                compactable: *exps >= self.compact_threshold && dominant.is_some(),
                dominant_operator: dominant,
            });
        }
        buckets.sort_by_key(|b| std::cmp::Reverse(b.experiments));
        let distinct_instances = db
            .all()
            .iter()
            .filter(|r| !r.instance_id.is_empty())
            .map(|r| r.instance_id.clone())
            .collect::<BTreeSet<_>>()
            .len();

        MemoryReport {
            total_experiments: db.len(),
            distinct_instances,
            buckets,
            note: "Append-only: records are never deleted. 'Compactable' regimes can be \
SUMMARIZED into a rule for bounded LLM context, but the raw stream is retained forever \
and every summary is recomputable from it."
                .into(),
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

    fn sig(density: f64) -> InstanceSignature {
        InstanceSignature {
            n: 800,
            density,
            clustering: 0.3,
            mean_degree: 4.0,
            degree_cv: 0.0,
        }
    }

    #[test]
    fn recall_finds_similar_instances_and_their_best_operators() {
        // Two sparse instances (near the query) where "metropolis" shines, and
        // one dense instance (far) that should be excluded.
        let mut recs = Vec::new();
        for _ in 0..6 {
            recs.push(rec("Gsparse1", 0.01, &["metropolis_sweep"], -0.6));
            recs.push(rec("Gsparse2", 0.012, &["metropolis_sweep"], -0.5));
            recs.push(rec("Gsparse1", 0.01, &["random_flip_sweep"], -0.05));
        }
        let mut dense = rec("Kdense", 0.5, &["metropolis_sweep"], -0.9);
        dense.clustering = 0.3;
        recs.push(dense);
        let db = ExperimentDb::from_records(recs);
        let graph = KnowledgeGraph::new();

        let r = recall(&db, &graph, &sig(0.011), 0.1);
        assert!(
            r.similar_instances.contains(&"Gsparse1".to_string())
                && r.similar_instances.contains(&"Gsparse2".to_string()),
            "should recall the sparse instances: {:?}",
            r.similar_instances
        );
        assert!(
            !r.similar_instances.contains(&"Kdense".to_string()),
            "the dense instance is not structurally similar"
        );
        assert_eq!(
            r.best_operators.first().map(|(op, _)| op.as_str()),
            Some("metropolis_sweep"),
            "metropolis worked best on similar instances: {:?}",
            r.best_operators
        );
        assert!(r.narrative.contains("similar"));
    }

    #[test]
    fn recall_applies_only_matching_conditional_knowledge() {
        let db = ExperimentDb::from_records(vec![rec("g", 0.01, &["op"], -0.5)]);
        let mut graph = KnowledgeGraph::new();
        graph.observe_if(
            "cluster",
            "effective-on",
            "graph",
            "density<0.05",
            0.4,
            "proof",
        );
        graph.observe_if(
            "dense_op",
            "effective-on",
            "graph",
            "density>=0.05",
            0.4,
            "proof",
        );
        let r = recall(&db, &graph, &sig(0.01), 0.2);
        assert!(
            r.applicable_facts.iter().any(|f| f.contains("cluster")),
            "the sparse condition applies: {:?}",
            r.applicable_facts
        );
        assert!(
            !r.applicable_facts.iter().any(|f| f.contains("dense_op")),
            "the dense condition does not apply here"
        );
    }

    #[test]
    fn memory_manager_buckets_and_never_deletes() {
        let mut recs = Vec::new();
        for _ in 0..250 {
            recs.push(rec("s", 0.01, &["metropolis_sweep"], -0.6));
        }
        for _ in 0..10 {
            recs.push(rec("d", 0.5, &["gibbs_color_sweep"], -0.5));
        }
        let db = ExperimentDb::from_records(recs);
        let report = MemoryManager::default().analyze(&db);
        assert_eq!(report.total_experiments, 260);
        let sparse = report.buckets.iter().find(|b| b.label == "sparse").unwrap();
        assert!(
            sparse.compactable,
            "well-sampled sparse regime is compactable"
        );
        assert_eq!(
            sparse.dominant_operator.as_deref(),
            Some("metropolis_sweep")
        );
        let dense = report.buckets.iter().find(|b| b.label == "dense").unwrap();
        assert!(
            !dense.compactable,
            "thin dense regime is not yet compactable"
        );
        assert!(
            report.note.contains("never"),
            "retention note must state append-only"
        );
    }
}
