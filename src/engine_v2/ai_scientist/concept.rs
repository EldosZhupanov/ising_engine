//! Concept Discovery Engine — the upstream cognitive faculty that lets the
//! scientist expand its own vocabulary instead of only sharpening confidence
//! inside a fixed one.
//!
//! Pipeline: **propose → test → gate → admit**.
//!   * *propose* — a candidate concept is a deterministic `InstanceSignature → f64`
//!     (typically an interaction/nonlinearity the linear vocabulary cannot express,
//!     e.g. density×clustering).
//!   * *test* — does adding the concept improve **out-of-sample** prediction of
//!     experiment outcomes? Measured by leave-one-instance-out ridge RMSE, base
//!     vocabulary vs base+candidate. Reuses the shared `ridge_fit` solver.
//!   * *gate* — admit only if the OOD gain exceeds an **Occam** complexity penalty
//!     (the discipline that killed the naive 11-feature descriptor: in-sample fit
//!     is never enough).
//!   * *admit* — append to the shared [`FeatureRegistry`] (propagates to every
//!     faculty) and publish a source-attributed verdict to the [`KnowledgeGraph`];
//!     rejected concepts are recorded too (a dead end is knowledge).
//!
//! It emits **features only** — never rules (Meta-Learner), instance rankings
//! (Planner), or ablations (Theory Engine). One shared vocabulary; no duplication.

use std::collections::BTreeMap;

use super::db::ExperimentDb;
use super::feature_registry::{Feature, FeatureRegistry};
use super::graph::KnowledgeGraph;
use super::predictor::{ridge_fit, InstanceSignature};

/// Tunables for concept admission.
#[derive(Debug, Clone)]
pub struct ConceptConfig {
    pub lambda: f64,
    /// Occam penalty charged for the added dimension (fraction of base error).
    pub occam: f64,
    /// Net gain (OOD improvement − penalty) required to admit.
    pub min_gain: f64,
    /// Minimum distinct instances needed to run the OOD test.
    pub min_instances: usize,
}

impl Default for ConceptConfig {
    fn default() -> Self {
        Self {
            lambda: 1e-3,
            occam: 0.02,
            min_gain: 0.01,
            min_instances: 4,
        }
    }
}

/// The falsifiable record of testing one candidate concept.
#[derive(Debug, Clone)]
pub struct ConceptVerdict {
    pub name: String,
    pub admitted: bool,
    pub base_rmse: f64,
    pub candidate_rmse: f64,
    /// (base_rmse − candidate_rmse) / base_rmse, out-of-sample.
    pub predictive_gain: f64,
    pub complexity_penalty: f64,
    pub net: f64,
    pub reason: String,
}

/// The default candidate concept library — deterministic interactions and
/// nonlinearities the LINEAR base vocabulary (v0) cannot express. This is the
/// space the scientist searches each cycle; admitted concepts extend the shared
/// representation. Names are stable so the graph accrues confidence across ticks.
pub fn default_candidates() -> Vec<Feature> {
    vec![
        Feature {
            name: "density_x_clustering",
            extract: |s| s.density * s.clustering,
        },
        Feature {
            name: "density_x_degree_cv",
            extract: |s| s.density * s.degree_cv,
        },
        Feature {
            name: "clustering_x_degree_cv",
            extract: |s| s.clustering * s.degree_cv,
        },
        Feature {
            name: "density_sq",
            extract: |s| s.density * s.density,
        },
        Feature {
            name: "clustering_sq",
            extract: |s| s.clustering * s.clustering,
        },
        Feature {
            name: "mean_degree_x_density",
            extract: |s| (s.mean_degree / 10.0) * s.density,
        },
    ]
}

/// Per-instance (signature, target) built from the append-only DB. Target is the
/// instance's mean relative improvement — the thing worth predicting.
fn aggregate(db: &ExperimentDb) -> Vec<(InstanceSignature, f64)> {
    let mut acc: BTreeMap<String, (InstanceSignature, f64, usize)> = BTreeMap::new();
    for r in db.all() {
        if r.instance_id.is_empty() {
            continue;
        }
        let e = acc.entry(r.instance_id.clone()).or_insert_with(|| {
            (
                InstanceSignature {
                    n: r.n,
                    density: r.density,
                    clustering: r.clustering,
                    mean_degree: r.mean_degree,
                    degree_cv: r.degree_cv,
                },
                0.0,
                0,
            )
        });
        e.1 += r.rel_improvement();
        e.2 += 1;
    }
    acc.into_values()
        .filter(|(_, _, c)| *c > 0)
        .map(|(sig, sum, c)| (sig, sum / c as f64))
        .collect()
}

/// Leave-one-instance-out RMSE for a feature builder — the OOD skill measure.
fn loio_rmse<F>(data: &[(InstanceSignature, f64)], build: F, lambda: f64) -> Option<f64>
where
    F: Fn(&InstanceSignature) -> Vec<f64>,
{
    let n = data.len();
    if n < 3 {
        return None;
    }
    let rows: Vec<Vec<f64>> = data.iter().map(|(s, _)| build(s)).collect();
    let y: Vec<f64> = data.iter().map(|(_, t)| *t).collect();
    let (mut se, mut cnt) = (0.0, 0usize);
    for held in 0..n {
        let tr_rows: Vec<Vec<f64>> = (0..n)
            .filter(|&i| i != held)
            .map(|i| rows[i].clone())
            .collect();
        let tr_y: Vec<f64> = (0..n).filter(|&i| i != held).map(|i| y[i]).collect();
        if let Some(w) = ridge_fit(&tr_rows, &tr_y, lambda) {
            let pred: f64 = w.iter().zip(&rows[held]).map(|(a, b)| a * b).sum();
            se += (pred - y[held]).powi(2);
            cnt += 1;
        }
    }
    (cnt > 0).then(|| (se / cnt as f64).sqrt())
}

/// A feature row for the base vocabulary (with an intercept term).
fn base_row(base: &FeatureRegistry, s: &InstanceSignature) -> Vec<f64> {
    let mut r = Vec::with_capacity(base.len() + 1);
    r.push(1.0);
    base.encode_into(s, &mut r);
    r
}

/// Test one candidate concept against the accumulated evidence. `None` if there is
/// too little data to run an honest OOD test.
pub fn evaluate(
    db: &ExperimentDb,
    base: &FeatureRegistry,
    candidate: &Feature,
    cfg: &ConceptConfig,
) -> Option<ConceptVerdict> {
    let data = aggregate(db);
    if data.len() < cfg.min_instances {
        return None;
    }
    let base_rmse = loio_rmse(&data, |s| base_row(base, s), cfg.lambda)?;
    let extract = candidate.extract;
    let cand_rmse = loio_rmse(
        &data,
        |s| {
            let mut r = base_row(base, s);
            r.push(extract(s));
            r
        },
        cfg.lambda,
    )?;
    let predictive_gain = if base_rmse > 1e-12 {
        (base_rmse - cand_rmse) / base_rmse
    } else {
        0.0
    };
    let complexity_penalty = cfg.occam;
    let net = predictive_gain - complexity_penalty;
    let admitted = net >= cfg.min_gain;
    Some(ConceptVerdict {
        name: candidate.name.to_string(),
        admitted,
        base_rmse,
        candidate_rmse: cand_rmse,
        predictive_gain,
        complexity_penalty,
        net,
        reason: if admitted {
            format!(
                "OOD RMSE {base_rmse:.4}→{cand_rmse:.4} (gain {:.1}%) beats Occam {:.1}%",
                predictive_gain * 100.0,
                complexity_penalty * 100.0
            )
        } else {
            format!(
                "net {:.1}% below admit threshold {:.1}% (gain {:.1}%, penalty {:.1}%)",
                net * 100.0,
                cfg.min_gain * 100.0,
                predictive_gain * 100.0,
                complexity_penalty * 100.0
            )
        },
    })
}

/// Run a candidate library against the evidence, growing `base` with every concept
/// that earns admission and recording all verdicts (admitted AND rejected) in the
/// graph. Returns the grown registry and the verdicts. Deterministic.
pub fn discover(
    db: &ExperimentDb,
    mut base: FeatureRegistry,
    candidates: &[Feature],
    cfg: &ConceptConfig,
    graph: &mut KnowledgeGraph,
) -> (FeatureRegistry, Vec<ConceptVerdict>) {
    let mut verdicts = Vec::new();
    for cand in candidates {
        if base.contains(cand.name) {
            continue;
        }
        let Some(v) = evaluate(db, &base, cand, cfg) else {
            continue;
        };
        // Publish the verdict (kept forever, admitted or refuted).
        graph.observe_if(
            &format!("concept:{}", v.name),
            if v.admitted { "admitted" } else { "rejected" },
            "vocabulary",
            "",
            v.predictive_gain,
            "concept-discovery",
        );
        if v.admitted {
            base.admit(*cand);
        }
        verdicts.push(v);
    }
    (base, verdicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_v2::ai_scientist::db::{ExperimentRecord, RunContext};

    /// Build a DB where the target = density×clustering (an INTERACTION the linear
    /// base vocabulary cannot express), plus a decoy target-free structure.
    fn synthetic_db() -> ExperimentDb {
        let mut db = ExperimentDb::new();
        // 8 instances spanning the density×clustering grid.
        let grid = [
            (0.1, 0.1),
            (0.1, 0.9),
            (0.3, 0.5),
            (0.5, 0.2),
            (0.5, 0.8),
            (0.7, 0.4),
            (0.9, 0.1),
            (0.9, 0.9),
        ];
        for (k, &(density, clustering)) in grid.iter().enumerate() {
            db.set_context(RunContext {
                instance_id: format!("inst_{k}"),
                n: 100,
                mean_degree: density * 99.0,
                degree_cv: 0.1,
                campaign_id: 0,
                generation_id: 0,
            });
            // target rel_improvement = density*clustering, encoded via baseline/score.
            let target = density * clustering;
            for s in 0..3 {
                db.record(ExperimentRecord {
                    sequence: vec!["metropolis_sweep".into()],
                    sweeps: vec![10],
                    baseline: 100.0,
                    score: 100.0 - 100.0 * target, // improvement=100*target ⇒ rel≈target
                    density,
                    clustering,
                    seed: s,
                    ..Default::default()
                });
            }
        }
        db
    }

    #[test]
    fn admits_a_predictive_interaction_and_rejects_noise() {
        let db = synthetic_db();
        let cfg = ConceptConfig::default();

        // A genuinely predictive interaction the linear base cannot represent.
        let good = Feature {
            name: "density_x_clustering",
            extract: |s| s.density * s.clustering,
        };
        let v = evaluate(&db, &FeatureRegistry::v0(), &good, &cfg).expect("enough data");
        assert!(
            v.admitted,
            "a real interaction concept must be admitted: {}",
            v.reason
        );
        assert!(v.predictive_gain > cfg.occam, "gain must beat Occam: {v:?}");

        // A useless constant concept must be rejected.
        let noise = Feature {
            name: "constant",
            extract: |_| 1.0,
        };
        let vn = evaluate(&db, &FeatureRegistry::v0(), &noise, &cfg).expect("enough data");
        assert!(
            !vn.admitted,
            "a non-predictive concept must be rejected: {vn:?}"
        );
    }

    #[test]
    fn discover_grows_the_vocabulary_and_records_verdicts() {
        let db = synthetic_db();
        let mut graph = KnowledgeGraph::new();
        let candidates = [
            Feature {
                name: "density_x_clustering",
                extract: |s| s.density * s.clustering,
            },
            Feature {
                name: "constant",
                extract: |_| 1.0,
            },
        ];
        let (grown, verdicts) = discover(
            &db,
            FeatureRegistry::v0(),
            &candidates,
            &ConceptConfig::default(),
            &mut graph,
        );
        // The interaction is admitted (vocabulary grew from 5→6); the constant is not.
        assert_eq!(grown.len(), 6, "exactly one concept admitted");
        assert!(grown.contains("density_x_clustering"));
        assert!(!grown.contains("constant"));
        assert_eq!(verdicts.len(), 2, "both verdicts recorded");
        // Both verdicts are in the graph, kept forever.
        assert!(!graph
            .query("concept:density_x_clustering", "admitted")
            .is_empty());
        assert!(!graph.query("concept:constant", "rejected").is_empty());
    }
}
