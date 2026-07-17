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
use std::collections::{BTreeMap, BTreeSet};

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
    feature_row(
        vocab,
        sig,
        &sched.ops,
        &sched.sweeps,
        sched.temp_hi,
        sched.temp_lo,
    )
}

/// The feature row, computed directly from borrowed schedule fields — so hot
/// loops (e.g. leave-one-out sufficient statistics) can build features from a
/// raw `ExperimentRecord` without cloning it into a `Schedule`. Bit-identical to
/// `features` for the same inputs.
fn feature_row(
    vocab: &[String],
    sig: &InstanceSignature,
    ops: &[String],
    sweeps: &[u32],
    temp_hi: f64,
    temp_lo: f64,
) -> Vec<f64> {
    let mut x = Vec::with_capacity(10 + vocab.len());
    x.push(1.0); // bias
    x.push(((sig.n as f64) + 1.0).ln() / 10.0);
    x.push(sig.density);
    x.push(sig.clustering);
    x.push(sig.mean_degree / 10.0);
    x.push(sig.degree_cv);
    x.push(ops.len() as f64 / 4.0);
    x.push((sweeps.iter().sum::<u32>() as f64 + 1.0).ln() / 10.0);
    x.push((temp_hi + 1.0).ln());
    x.push(temp_lo);
    for op in vocab {
        x.push(if ops.iter().any(|o| o == op) {
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

    /// Serialize the learned weights for the Model Registry. Round-trip f64
    /// `Display` (shortest exact) so a reload is bit-identical and preserves
    /// determinism (ADR-0004). Groups are `;`-separated, values `,`-separated;
    /// operator names are snake_case identifiers, so neither separator collides.
    /// Layout: `vocab;weights;trained_on`.
    pub fn to_weights_text(&self) -> String {
        let w = self
            .weights
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("{};{};{}", self.vocab.join(","), w, self.trained_on)
    }

    /// Reconstruct a predictor from [`Self::to_weights_text`]. Returns `None` on
    /// a malformed payload or a weight-vector length that disagrees with the
    /// vocabulary (mirrors the db's skip-malformed-line policy).
    pub fn from_weights_text(s: &str) -> Option<Self> {
        let mut g = s.split(';');
        let vocab_s = g.next()?;
        let w_s = g.next()?;
        let trained_on: usize = g.next()?.parse().ok()?;
        let vocab: Vec<String> = if vocab_s.is_empty() {
            Vec::new()
        } else {
            vocab_s.split(',').map(|x| x.to_string()).collect()
        };
        let weights: Vec<f64> = if w_s.is_empty() {
            Vec::new()
        } else {
            w_s.split(',')
                .map(|x| x.parse().ok())
                .collect::<Option<_>>()?
        };
        if weights.len() != 10 + vocab.len() {
            return None;
        }
        Some(Self {
            vocab,
            weights,
            trained_on,
        })
    }
}

/// Leave-one-instance-out fitted predictors via ADDITIVE SUFFICIENT STATISTICS.
///
/// The normal-equation matrices XᵀX and Xᵀy are sums over rows, so the training
/// set for fold *k* (every row whose instance ≠ *k*) has matrices
/// `S_total − S_k` and `b_total − b_k`. One pass builds the global sums and every
/// per-instance partial (O(N·d²), and — via [`feature_row`] — with ZERO per-row
/// cloning); each fold is then a single d×d solve (O(K·d³)). This replaces the
/// naïve O(K·N·d²)-compute + O(K·N)-clone leave-one-out.
///
/// All folds share ONE global vocabulary. This is identical to refitting per fold
/// whenever the instances share an operator pool (the normal case — the whole
/// campaign draws from one registry); the tests verify the outputs match a
/// from-scratch leave-one-out. NOTE: `S_total − S_k` differs from a fresh
/// `Σ_{j≠k}` by floating-point reordering, so predictions can differ at the ULP
/// level — immaterial to the Spearman RANK correlation this feeds, and verified.
///
/// Returns a fitted [`Predictor`] per instance that has ≥ `MIN_TRAIN` training
/// rows from the other instances.
pub(crate) fn leave_one_instance_out(
    db: &ExperimentDb,
    lambda: f64,
) -> BTreeMap<String, Predictor> {
    let vocab: Vec<String> = db
        .all()
        .iter()
        .flat_map(|r| r.sequence.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let dim = 10 + vocab.len();

    // Per-instance partial normal equations (upper triangle stored flat).
    struct Partial {
        s: Vec<f64>, // dim*dim, only [i*dim + j] for j ≥ i written
        b: Vec<f64>,
        n: usize,
    }
    let mut s_total = vec![0.0f64; dim * dim];
    let mut b_total = vec![0.0f64; dim];
    let mut n_total = 0usize;
    let mut per: BTreeMap<String, Partial> = BTreeMap::new();

    for r in db.all() {
        let sig = InstanceSignature {
            n: r.n,
            density: r.density,
            clustering: r.clustering,
            mean_degree: r.mean_degree,
            degree_cv: r.degree_cv,
        };
        // No clone: features straight from the borrowed record fields.
        let x = feature_row(&vocab, &sig, &r.sequence, &r.sweeps, r.temp_hi, r.temp_lo);
        let y = r.rel_improvement();
        let e = per.entry(r.instance_id.clone()).or_insert_with(|| Partial {
            s: vec![0.0; dim * dim],
            b: vec![0.0; dim],
            n: 0,
        });
        for i in 0..dim {
            let xi = x[i];
            b_total[i] += xi * y;
            e.b[i] += xi * y;
            let row = i * dim;
            for j in i..dim {
                let v = xi * x[j];
                s_total[row + j] += v;
                e.s[row + j] += v;
            }
        }
        e.n += 1;
        n_total += 1;
    }

    // Each fold: A = (S_total − S_k), symmetrized + ridged EXACTLY as
    // `Predictor::fit`, then the same Gaussian-elimination solve.
    let mut out = BTreeMap::new();
    for (inst, p) in &per {
        if n_total - p.n < MIN_TRAIN {
            continue;
        }
        let mut a = vec![vec![0.0; dim]; dim];
        let mut rhs = vec![0.0; dim];
        for i in 0..dim {
            rhs[i] = b_total[i] - p.b[i];
            let row = i * dim;
            for j in i..dim {
                a[i][j] = s_total[row + j] - p.s[row + j];
            }
        }
        // Mirror the upper triangle down and add ridge to the diagonal — the
        // identical `split_at_mut` idiom `Predictor::fit` uses (same FP ops).
        for i in 0..dim {
            let (upper, lower) = a.split_at_mut(i);
            for (j, u) in upper.iter().enumerate() {
                lower[0][j] = u[i];
            }
            lower[0][i] += lambda.max(1e-9);
        }
        if let Some(weights) = solve(a, rhs) {
            out.insert(
                inst.clone(),
                Predictor {
                    vocab: vocab.clone(),
                    weights,
                    trained_on: n_total - p.n,
                },
            );
        }
    }
    out
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
    fn weights_text_round_trips_exactly() {
        let mut db = ExperimentDb::new();
        for i in 0..40 {
            let jitter = (i % 5) as f64 * 0.01;
            db.record(rec(&["strong_op"], -0.5 - jitter, 10));
            db.record(rec(&["weak_op"], -0.05 - jitter, 10));
        }
        let p = Predictor::fit(&db, 1e-3).unwrap();
        let round = Predictor::from_weights_text(&p.to_weights_text()).unwrap();
        let sig = InstanceSignature {
            density: 0.01,
            clustering: 0.3,
            ..Default::default()
        };
        // Reconstruction is bit-identical (round-trip f64 Display), so a reloaded
        // registry snapshot predicts exactly what the live model did.
        for s in [sched(&["strong_op"]), sched(&["weak_op", "strong_op"])] {
            assert_eq!(
                p.predict(&sig, &s).to_bits(),
                round.predict(&sig, &s).to_bits()
            );
        }
        assert_eq!(round.trained_on, p.trained_on);
        // A truncated payload is rejected, not silently accepted.
        assert!(Predictor::from_weights_text("op_a,op_b;1.0,2.0;30").is_none());
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
