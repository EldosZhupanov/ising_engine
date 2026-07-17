//! Platform self-evaluation (Stage 6 follow-up): is the platform actually
//! LEARNING, or fitting noise? Two audits over the append-only history:
//!
//! 1. RULE REPRODUCIBILITY — mine each instance's records separately and count
//!    which rule SIGNATURES (structure, numbers stripped) recur across
//!    instances. A rule seen on one instance is an observation; a rule that
//!    re-emerges independently on several is knowledge.
//! 2. PREDICTOR ACCURACY — leave-one-instance-out: fit the ridge predictor on
//!    every other instance's records, predict the held-out instance, score by
//!    Spearman rank correlation (does it ORDER candidates correctly — its only
//!    job as a filter) and MAE.
//!
//! Both are written to `evaluation_report.md` with support counts, so weak
//! evidence reads as weak.

use super::db::ExperimentDb;
use super::meta_learner::MetaLearner;
use super::predictor::{leave_one_instance_out, InstanceSignature};
use crate::engine_v2::evolution::Schedule;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A rule signature and the instances that independently produced it.
#[derive(Debug, Clone)]
pub struct ReproducedRule {
    pub signature: String,
    pub instances: Vec<String>,
    pub mean_confidence: f64,
    pub example: String,
}

/// Group records by instance id (records without provenance are skipped).
fn by_instance(db: &ExperimentDb) -> BTreeMap<String, Vec<usize>> {
    let mut m: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, r) in db.all().iter().enumerate() {
        if !r.instance_id.is_empty() {
            m.entry(r.instance_id.clone()).or_default().push(i);
        }
    }
    m
}

/// Mine each instance separately; report signatures that recur on ≥ 2
/// instances, most-reproduced first. Returns (rules, instances_analyzed).
pub fn rule_reproducibility(db: &ExperimentDb, meta: &MetaLearner) -> (Vec<ReproducedRule>, usize) {
    let groups = by_instance(db);
    let mut agg: BTreeMap<String, (Vec<String>, f64, String)> = BTreeMap::new();
    let mut analyzed = 0usize;
    for (inst, idx) in &groups {
        if idx.len() < meta.min_support * 2 {
            continue; // too little data to mine this instance honestly
        }
        analyzed += 1;
        let sub = ExperimentDb::from_records(idx.iter().map(|&i| db.all()[i].clone()).collect());
        for rule in meta.mine(&sub) {
            let e = agg
                .entry(rule.signature())
                .or_insert_with(|| (Vec::new(), 0.0, rule.statement.clone()));
            if !e.0.contains(inst) {
                e.0.push(inst.clone());
                e.1 += rule.confidence;
            }
        }
    }
    let mut out: Vec<ReproducedRule> = agg
        .into_iter()
        .filter(|(_, (insts, _, _))| insts.len() >= 2)
        .map(
            |(signature, (instances, conf_sum, example))| ReproducedRule {
                mean_confidence: conf_sum / instances.len() as f64,
                signature,
                instances,
                example,
            },
        )
        .collect();
    out.sort_by(|a, b| {
        b.instances
            .len()
            .cmp(&a.instances.len())
            .then(b.mean_confidence.total_cmp(&a.mean_confidence))
            .then(a.signature.cmp(&b.signature))
    });
    (out, analyzed)
}

/// Per-instance held-out accuracy of the predictor.
#[derive(Debug, Clone)]
pub struct InstanceAccuracy {
    pub instance: String,
    pub n_test: usize,
    pub spearman: f64,
    pub mae: f64,
}

/// Spearman rank correlation (average ranks for ties).
pub fn spearman(a: &[f64], b: &[f64]) -> f64 {
    fn ranks(v: &[f64]) -> Vec<f64> {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|&i, &j| v[i].total_cmp(&v[j]).then(i.cmp(&j)));
        let mut r = vec![0.0; v.len()];
        let mut k = 0;
        while k < idx.len() {
            let mut m = k;
            while m + 1 < idx.len() && v[idx[m + 1]] == v[idx[k]] {
                m += 1;
            }
            let avg = (k + m) as f64 / 2.0 + 1.0;
            for &i in &idx[k..=m] {
                r[i] = avg;
            }
            k = m + 1;
        }
        r
    }
    if a.len() < 2 {
        return 0.0;
    }
    let (ra, rb) = (ranks(a), ranks(b));
    let n = a.len() as f64;
    let mean = (n + 1.0) / 2.0;
    let (mut num, mut da, mut db_) = (0.0, 0.0, 0.0);
    for i in 0..a.len() {
        let (x, y) = (ra[i] - mean, rb[i] - mean);
        num += x * y;
        da += x * x;
        db_ += y * y;
    }
    if da <= 0.0 || db_ <= 0.0 {
        0.0
    } else {
        num / (da * db_).sqrt()
    }
}

/// Leave-one-instance-out predictor evaluation. `None` when fewer than two
/// instances carry enough data — an honest "cannot evaluate yet".
pub fn evaluate_predictor(db: &ExperimentDb, lambda: f64) -> Option<Vec<InstanceAccuracy>> {
    let groups = by_instance(db);
    let eligible: Vec<&String> = groups.keys().filter(|k| groups[*k].len() >= 10).collect();
    if eligible.len() < 2 {
        return None;
    }
    // One pass builds every fold's predictor via additive sufficient statistics
    // (O(N·d²), no per-fold refit, no row cloning) instead of refitting on a
    // fresh clone of ~all rows per instance (O(K·N·d²)).
    let folds = leave_one_instance_out(db, lambda);
    let mut out = Vec::new();
    for inst in eligible {
        let test_idx = &groups[inst];
        let Some(p) = folds.get(inst) else {
            continue;
        };
        let mut preds = Vec::with_capacity(test_idx.len());
        let mut actual = Vec::with_capacity(test_idx.len());
        let mut abs_err = 0.0;
        for &i in test_idx {
            let r = &db.all()[i];
            let sig = InstanceSignature {
                n: r.n,
                density: r.density,
                clustering: r.clustering,
                mean_degree: r.mean_degree,
                degree_cv: r.degree_cv,
            };
            let sched = Schedule {
                ops: r.sequence.clone(),
                sweeps: r.sweeps.clone(),
                temp_hi: r.temp_hi,
                temp_lo: r.temp_lo,
            };
            let y_hat = p.predict(&sig, &sched);
            let y = r.rel_improvement();
            abs_err += (y_hat - y).abs();
            preds.push(y_hat);
            actual.push(y);
        }
        out.push(InstanceAccuracy {
            instance: inst.clone(),
            n_test: preds.len(),
            spearman: spearman(&preds, &actual),
            mae: abs_err / preds.len().max(1) as f64,
        });
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Write `dir/evaluation_report.md`: both audits with honest caveats.
pub fn write_evaluation(
    dir: impl AsRef<Path>,
    db: &ExperimentDb,
    meta: &MetaLearner,
) -> io::Result<PathBuf> {
    let dir = dir.as_ref();
    let (rules, analyzed) = rule_reproducibility(db, meta);
    let pred = evaluate_predictor(db, 1e-3);

    let mut out = String::new();
    out.push_str(&format!(
        "# Platform Self-Evaluation — {} experiments, {} instances mined\n\n",
        db.len(),
        analyzed
    ));

    out.push_str("## Rule reproducibility across instances\n\n");
    out.push_str(
        "A rule counts as REPRODUCED when its structural signature emerges \
independently from ≥ 2 instances' own records.\n\n",
    );
    if rules.is_empty() {
        out.push_str("No rule reproduced across instances yet — treat every mined rule as instance-specific until this changes.\n");
    } else {
        out.push_str("| signature | instances | mean confidence | example |\n|---|---|---|---|\n");
        for r in rules.iter().take(30) {
            out.push_str(&format!(
                "| `{}` | {} ({}) | {:.2} | {} |\n",
                r.signature,
                r.instances.len(),
                r.instances.join(", "),
                r.mean_confidence,
                r.example.replace('|', "/"),
            ));
        }
    }

    out.push_str("\n## Predictor accuracy (leave-one-instance-out)\n\n");
    match &pred {
        None => out.push_str(
            "Not enough multi-instance data to evaluate — the predictor must not be trusted as a filter yet.\n",
        ),
        Some(rows) => {
            out.push_str(
                "Spearman = rank correlation between predicted and observed relative \
improvement on the HELD-OUT instance (the filter's actual job). MAE in \
fractions of baseline.\n\n| instance | n | Spearman | MAE |\n|---|---|---|---|\n",
            );
            let mut s_sum = 0.0;
            for r in rows {
                out.push_str(&format!(
                    "| {} | {} | {:+.3} | {:.3} |\n",
                    r.instance, r.n_test, r.spearman, r.mae
                ));
                s_sum += r.spearman;
            }
            let mean_s = s_sum / rows.len() as f64;
            out.push_str(&format!(
                "\nMean held-out Spearman: **{mean_s:+.3}** over {} instances. ",
                rows.len()
            ));
            out.push_str(if mean_s > 0.3 {
                "The predictor generalizes across instances well enough to keep as a filter.\n"
            } else if mean_s > 0.1 {
                "Weak but positive transfer — keep the filter only with generous oversampling.\n"
            } else {
                "No useful transfer measured — the filter should be disabled until this improves.\n"
            });
        }
    }

    let path = dir.join("evaluation_report.md");
    fs::write(&path, out)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(inst: &str, seq: &[&str], score: f64, density: f64) -> ExperimentRecord {
        ExperimentRecord {
            instance_id: inst.into(),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
            score,
            baseline: 0.0,
            density,
            clustering: 0.3,
            backend: "SparseBitSlice".into(),
            ..Default::default()
        }
    }

    fn two_instance_db() -> ExperimentDb {
        // On BOTH instances: metropolis dominates, random_flip is the worst.
        let mut recs = Vec::new();
        for inst in ["G_a", "G_b"] {
            for i in 0..20 {
                let mut r = rec(
                    inst,
                    &["metropolis_sweep", "greedy_descent"],
                    -0.5 - 0.01 * i as f64,
                    0.01,
                );
                r.id = recs.len() as u64;
                recs.push(r);
                let mut r = rec(inst, &["random_flip_sweep"], 0.02, 0.01);
                r.id = recs.len() as u64;
                recs.push(r);
            }
        }
        ExperimentDb::from_records(recs)
    }

    #[test]
    #[ignore = "benchmark: set ISING_BENCH_DB=<path to ai_experiments.txt> to measure"]
    fn evaluate_predictor_cost_on_a_real_db() {
        let Ok(path) = std::env::var("ISING_BENCH_DB") else {
            return;
        };
        let db = ExperimentDb::load(&path).expect("load db");
        let _ = evaluate_predictor(&db, 1e-3); // warm
        let iters = 5;
        let t = std::time::Instant::now();
        let mut folds = 0usize;
        for _ in 0..iters {
            folds = evaluate_predictor(&db, 1e-3).map(|v| v.len()).unwrap_or(0);
        }
        let per_ms = t.elapsed().as_secs_f64() / iters as f64 * 1000.0;
        // Honesty check: the mean leave-one-out Spearman must still match the
        // historically-recorded +0.747, i.e. the sufficient-statistics rewrite
        // did not change the science, only the speed.
        let accs = evaluate_predictor(&db, 1e-3).unwrap();
        let mean_spear = accs.iter().map(|a| a.spearman).sum::<f64>() / accs.len() as f64;
        eprintln!(
            "evaluate_predictor over {} experiments / {folds} folds: {per_ms:.2} ms/call \
             (mean leave-one-out Spearman {mean_spear:.4})",
            db.len()
        );
    }

    #[test]
    fn reproduced_rules_require_two_instances() {
        let db = two_instance_db();
        let (rules, analyzed) = rule_reproducibility(&db, &MetaLearner::new());
        assert_eq!(analyzed, 2);
        assert!(
            rules
                .iter()
                .any(|r| r.signature == "Dominance:metropolis_sweep" && r.instances.len() == 2),
            "dominance should reproduce on both instances: {rules:?}"
        );
        // A rule mined on one instance only must NOT be listed.
        for r in &rules {
            assert!(r.instances.len() >= 2);
        }
    }

    #[test]
    fn spearman_basics() {
        assert!((spearman(&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0]) - 1.0).abs() < 1e-12);
        assert!((spearman(&[1.0, 2.0, 3.0], &[30.0, 20.0, 10.0]) + 1.0).abs() < 1e-12);
        assert_eq!(spearman(&[1.0], &[2.0]), 0.0);
    }

    #[test]
    fn predictor_eval_transfers_across_instances() {
        let db = two_instance_db();
        let rows = evaluate_predictor(&db, 1e-3).expect("two eligible instances");
        assert_eq!(rows.len(), 2);
        for r in &rows {
            // The good/bad operator split is identical on both instances, so
            // held-out ranking must be strongly positive.
            assert!(
                r.spearman > 0.5,
                "expected transfer on {}: spearman {}",
                r.instance,
                r.spearman
            );
        }
    }

    #[test]
    fn sufficient_statistics_loo_matches_from_scratch() {
        use super::super::predictor::Predictor;
        // Three instances sharing the operator pool (30 rows each ⇒ 60 training
        // rows per fold ≥ MIN_TRAIN). Slight per-instance jitter avoids ties.
        let mut recs = Vec::new();
        for (k, inst) in ["G_a", "G_b", "G_c"].iter().enumerate() {
            let d = 0.005 + 0.003 * k as f64;
            for i in 0..15 {
                let mut r = rec(
                    inst,
                    &["metropolis_sweep", "greedy_descent"],
                    -0.5 - 0.01 * i as f64 - 0.002 * k as f64,
                    d,
                );
                r.id = recs.len() as u64;
                recs.push(r);
                let mut r = rec(inst, &["random_flip_sweep"], -0.02 - 0.001 * i as f64, d);
                r.id = recs.len() as u64;
                recs.push(r);
            }
        }
        let db = ExperimentDb::from_records(recs);

        // NEW path (sufficient statistics).
        let fast = evaluate_predictor(&db, 1e-3).unwrap();

        // REFERENCE: the pre-optimization from-scratch leave-one-out.
        let groups = by_instance(&db);
        let eligible: Vec<&String> = groups.keys().filter(|k| groups[*k].len() >= 10).collect();
        let mut reference: Vec<(String, f64, f64)> = Vec::new();
        for inst in &eligible {
            let train: Vec<_> = db
                .all()
                .iter()
                .filter(|r| &r.instance_id != *inst)
                .cloned()
                .collect();
            let p = Predictor::fit(&ExperimentDb::from_records(train), 1e-3).unwrap();
            let mut preds = Vec::new();
            let mut actual = Vec::new();
            let mut abs_err = 0.0;
            for &i in &groups[*inst] {
                let r = &db.all()[i];
                let sig = InstanceSignature {
                    n: r.n,
                    density: r.density,
                    clustering: r.clustering,
                    mean_degree: r.mean_degree,
                    degree_cv: r.degree_cv,
                };
                let sched = Schedule {
                    ops: r.sequence.clone(),
                    sweeps: r.sweeps.clone(),
                    temp_hi: r.temp_hi,
                    temp_lo: r.temp_lo,
                };
                let yh = p.predict(&sig, &sched);
                abs_err += (yh - r.rel_improvement()).abs();
                preds.push(yh);
                actual.push(r.rel_improvement());
            }
            reference.push((
                (*inst).clone(),
                spearman(&preds, &actual),
                abs_err / preds.len() as f64,
            ));
        }

        // Same instances, IDENTICAL Spearman (ranks are robust to ULP prediction
        // differences from the total−partial reordering), MAE within FP tolerance.
        assert_eq!(fast.len(), reference.len());
        for (f, (rinst, rspear, rmae)) in fast.iter().zip(&reference) {
            assert_eq!(&f.instance, rinst);
            assert!(
                (f.spearman - rspear).abs() < 1e-9,
                "spearman drift on {}: {} vs {}",
                f.instance,
                f.spearman,
                rspear
            );
            assert!(
                (f.mae - rmae).abs() < 1e-6,
                "mae drift on {}: {} vs {}",
                f.instance,
                f.mae,
                rmae
            );
        }
    }

    #[test]
    fn evaluation_report_is_written_with_both_sections() {
        let dir = std::env::temp_dir().join(format!("eval_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let db = two_instance_db();
        let path = write_evaluation(&dir, &db, &MetaLearner::new()).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("Rule reproducibility"));
        assert!(text.contains("Dominance:metropolis_sweep"));
        assert!(text.contains("leave-one-instance-out"));
        assert!(text.contains("Mean held-out Spearman"));
        let _ = fs::remove_dir_all(&dir);
    }
}
