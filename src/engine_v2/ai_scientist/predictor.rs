//! Performance predictor (Stage 6, Task 8) — a small, dependency-free ridge
//! regression trained on the experiment database. Input: instance features +
//! operator sequence + temperature + budget. Output: expected RELATIVE
//! improvement over baseline.
//!
//! The predictor is ONLY a filter: it ranks candidate schedules so the campaign
//! spends its budget on the promising ones. Every surviving candidate is still
//! executed and scored by the Runtime — a prediction is never treated as a
//! result. Fully deterministic (normal equations, Gaussian elimination).

use super::super::evolution::Schedule;
use super::db::ExperimentDb;
use std::collections::BTreeSet;

/// Instance features the predictor conditions on.
#[derive(Debug, Clone, Copy, Default)]
pub struct InstanceSignature {
    pub n: usize,
    pub density: f64,
    pub clustering: f64,
    pub mean_degree: f64,
    pub degree_cv: f64,
}

/// Ridge-regression predictor over (instance, schedule) features.
#[derive(Debug, Clone)]
pub struct Predictor {
    /// Operator vocabulary seen at fit time (sorted ⇒ deterministic layout).
    vocab: Vec<String>,
    /// Learned weights; layout: [bias, instance(5), schedule(4), vocab...].
    weights: Vec<f64>,
    pub trained_on: usize,
}

const MIN_TRAIN: usize = 30;

fn features(vocab: &[String], sig: &InstanceSignature, sched: &Schedule) -> Vec<f64> {
    let mut x = Vec::with_capacity(10 + vocab.len());
    x.push(1.0); // bias
    x.push(((sig.n as f64) + 1.0).ln() / 10.0);
    x.push(sig.density);
    x.push(sig.clustering);
    x.push(sig.mean_degree / 10.0);
    x.push(sig.degree_cv);
    x.push(sched.ops.len() as f64 / 4.0);
    x.push((sched.sweeps.iter().sum::<u32>() as f64 + 1.0).ln() / 10.0);
    x.push((sched.temp_hi + 1.0).ln());
    x.push(sched.temp_lo);
    for op in vocab {
        x.push(if sched.ops.iter().any(|o| o == op) {
            1.0
        } else {
            0.0
        });
    }
    x
}

/// Solve (A)w = b by Gaussian elimination with partial pivoting. A is square,
/// Ridge regression by normal equations: fit `w` minimizing ‖Xw − y‖² + λ‖w‖²
/// over the given feature `rows` and targets `y`. Deterministic (Gaussian
/// elimination on XᵀX + λI). Shared by the Dynamics and World models. Returns
/// `None` when the system is singular (should not happen with λ > 0).
pub(crate) fn ridge_fit(rows: &[Vec<f64>], y: &[f64], lambda: f64) -> Option<Vec<f64>> {
    if rows.is_empty() {
        return None;
    }
    let dim = rows[0].len();
    let mut xtx = vec![vec![0.0; dim]; dim];
    let mut xty = vec![0.0; dim];
    for (x, &yi) in rows.iter().zip(y) {
        for i in 0..dim {
            xty[i] += x[i] * yi;
            for j in i..dim {
                xtx[i][j] += x[i] * x[j];
            }
        }
    }
    for i in 0..dim {
        let (upper, lower) = xtx.split_at_mut(i);
        for (j, u) in upper.iter().enumerate() {
            lower[0][j] = u[i];
        }
        lower[0][i] += lambda.max(1e-9);
    }
    solve(xtx, xty)
}

/// symmetric positive definite here (XᵀX + λI), so this always succeeds.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in (col + 1)..n {
            let f = a[row][col] / a[col][col];
            let (head, tail) = a.split_at_mut(row);
            let pivot_row = &head[col];
            for (rk, pk) in tail[0].iter_mut().zip(pivot_row).skip(col) {
                *rk -= f * pk;
            }
            b[row] -= f * b[col];
        }
    }
    let mut w = vec![0.0; n];
    for col in (0..n).rev() {
        let mut s = b[col];
        for k in (col + 1)..n {
            s -= a[col][k] * w[k];
        }
        w[col] = s / a[col][col];
    }
    Some(w)
}

impl Predictor {
    /// Fit on the whole experiment history. `None` when there is not enough
    /// data to learn anything trustworthy (< MIN_TRAIN records) — callers must
    /// then skip filtering rather than filter on noise.
    pub fn fit(db: &ExperimentDb, lambda: f64) -> Option<Self> {
        if db.len() < MIN_TRAIN {
            return None;
        }
        let vocab: Vec<String> = db
            .all()
            .iter()
            .flat_map(|r| r.sequence.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let dim = 10 + vocab.len();
        let mut xtx = vec![vec![0.0; dim]; dim];
        let mut xty = vec![0.0; dim];
        for r in db.all() {
            let sched = Schedule {
                ops: r.sequence.clone(),
                sweeps: r.sweeps.clone(),
                temp_hi: r.temp_hi,
                temp_lo: r.temp_lo,
            };
            let sig = InstanceSignature {
                n: r.n,
                density: r.density,
                clustering: r.clustering,
                mean_degree: r.mean_degree,
                degree_cv: r.degree_cv,
            };
            let x = features(&vocab, &sig, &sched);
            let y = r.rel_improvement();
            for i in 0..dim {
                xty[i] += x[i] * y;
                for j in i..dim {
                    xtx[i][j] += x[i] * x[j];
                }
            }
        }
        // Symmetrize + ridge.
        for i in 0..dim {
            let (upper, lower) = xtx.split_at_mut(i);
            for (j, u) in upper.iter().enumerate() {
                lower[0][j] = u[i];
            }
            lower[0][i] += lambda.max(1e-9);
        }
        let weights = solve(xtx, xty)?;
        Some(Self {
            vocab,
            weights,
            trained_on: db.len(),
        })
    }

    /// Expected relative improvement of `sched` on an instance with `sig`.
    pub fn predict(&self, sig: &InstanceSignature, sched: &Schedule) -> f64 {
        features(&self.vocab, sig, sched)
            .iter()
            .zip(&self.weights)
            .map(|(x, w)| x * w)
            .sum()
    }

    /// FILTER: keep the `keep` most promising candidates (order: predicted
    /// improvement, best first; deterministic tie-break by index). Everything
    /// kept is still executed by the Runtime — the predictor never scores a
    /// result, it only spends the budget better.
    pub fn filter(
        &self,
        sig: &InstanceSignature,
        candidates: &[Schedule],
        keep: usize,
    ) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..candidates.len()).collect();
        let preds: Vec<f64> = candidates.iter().map(|s| self.predict(sig, s)).collect();
        idx.sort_by(|&a, &b| preds[b].total_cmp(&preds[a]).then(a.cmp(&b)));
        idx.truncate(keep.max(1));
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(seq: &[&str], score: f64, sweeps: u32) -> ExperimentRecord {
        ExperimentRecord {
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![sweeps; seq.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
            num_replicas: 8,
            backend: "SparseBitSlice".into(),
            score,
            baseline: 0.0,
            density: 0.01,
            clustering: 0.3,
            ..Default::default()
        }
    }

    fn sched(ops: &[&str]) -> Schedule {
        Schedule {
            ops: ops.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; ops.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
        }
    }

    #[test]
    fn refuses_to_fit_on_too_little_data() {
        let mut db = ExperimentDb::new();
        for _ in 0..10 {
            db.record(rec(&["a"], -0.5, 10));
        }
        assert!(Predictor::fit(&db, 1e-3).is_none());
    }

    #[test]
    fn learns_which_operator_helps_and_filters_accordingly() {
        let mut db = ExperimentDb::new();
        // "strong_op" reliably improves by 0.5; "weak_op" by 0.05.
        for i in 0..40 {
            let jitter = (i % 5) as f64 * 0.01;
            db.record(rec(&["strong_op"], -0.5 - jitter, 10));
            db.record(rec(&["weak_op"], -0.05 - jitter, 10));
        }
        let p = Predictor::fit(&db, 1e-3).unwrap();
        let sig = InstanceSignature {
            n: 0,
            density: 0.01,
            clustering: 0.3,
            mean_degree: 0.0,
            degree_cv: 0.0,
        };
        let strong = p.predict(&sig, &sched(&["strong_op"]));
        let weak = p.predict(&sig, &sched(&["weak_op"]));
        assert!(
            strong > weak + 0.2,
            "predictor must separate: strong {strong}, weak {weak}"
        );
        // Filter keeps the promising candidate first and truncates.
        let candidates = vec![
            sched(&["weak_op"]),
            sched(&["strong_op"]),
            sched(&["weak_op", "strong_op"]),
        ];
        let kept = p.filter(&sig, &candidates, 2);
        assert_eq!(kept.len(), 2);
        assert!(
            kept.contains(&1),
            "strong_op schedule must survive: {kept:?}"
        );
        assert!(!kept.contains(&0), "weak-only schedule must be filtered");
    }

    #[test]
    fn deterministic_fit_and_predict() {
        let mut db = ExperimentDb::new();
        for i in 0..35 {
            db.record(rec(&["a", "b"], -0.1 * (i % 7) as f64, 10 + i));
        }
        let p1 = Predictor::fit(&db, 1e-3).unwrap();
        let p2 = Predictor::fit(&db, 1e-3).unwrap();
        let sig = InstanceSignature::default();
        let s = sched(&["a"]);
        assert_eq!(
            p1.predict(&sig, &s).to_bits(),
            p2.predict(&sig, &s).to_bits()
        );
    }
}
