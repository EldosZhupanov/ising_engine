//! Statistical comparison (Stage 5, responsibility #7). Compares two samples of
//! scores (e.g. a candidate's per-seed scores vs the baseline's) and reports the
//! effect size and an approximate two-sided p-value from Welch's t-test. Pure,
//! deterministic, dependency-free (a rational approximation of the normal CDF
//! stands in for the t-distribution — honest for the moderate samples used here,
//! and labeled "approx" everywhere it surfaces).

/// Outcome of comparing sample `a` (candidate) against sample `b` (baseline).
/// Lower score is better, so `a` is an improvement when `mean_a < mean_b`.
#[derive(Debug, Clone, Copy)]
pub struct Comparison {
    pub mean_a: f64,
    pub mean_b: f64,
    pub std_a: f64,
    pub std_b: f64,
    pub n_a: usize,
    pub n_b: usize,
    /// Cohen's d effect size (pooled); positive ⇒ `a` scores lower (better).
    pub effect_size: f64,
    pub t_statistic: f64,
    /// Approximate two-sided p-value (normal approximation).
    pub p_value_approx: f64,
}

impl Comparison {
    /// `a` is a statistically significant improvement over `b` at `alpha`.
    pub fn is_significant_improvement(&self, alpha: f64) -> bool {
        self.mean_a < self.mean_b && self.p_value_approx < alpha
    }
    /// Confidence that `a` differs from `b` (1 − p), clamped to [0, 1].
    pub fn confidence(&self) -> f64 {
        (1.0 - self.p_value_approx).clamp(0.0, 1.0)
    }
}

fn mean(x: &[f64]) -> f64 {
    if x.is_empty() {
        0.0
    } else {
        x.iter().sum::<f64>() / x.len() as f64
    }
}

fn variance(x: &[f64], m: f64) -> f64 {
    if x.len() < 2 {
        0.0
    } else {
        x.iter().map(|&v| (v - m) * (v - m)).sum::<f64>() / (x.len() as f64 - 1.0)
    }
}

/// Abramowitz & Stegun 7.1.26 approximation of erf.
fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    sign * y
}

/// Standard normal CDF.
fn normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// Welch's two-sample comparison (unequal variances). `a` = candidate scores,
/// `b` = baseline scores. Lower is better.
pub fn compare(a: &[f64], b: &[f64]) -> Comparison {
    let (ma, mb) = (mean(a), mean(b));
    let (va, vb) = (variance(a, ma), variance(b, mb));
    let (na, nb) = (a.len(), b.len());
    let se = (va / na.max(1) as f64 + vb / nb.max(1) as f64).sqrt();
    let t = if se > 0.0 { (ma - mb) / se } else { 0.0 };
    // Pooled std for effect size.
    let pooled = ((va + vb) / 2.0).sqrt();
    let effect = if pooled > 0.0 {
        (mb - ma) / pooled
    } else {
        0.0
    };
    // Two-sided p via normal approximation of |t|.
    let p = if se > 0.0 {
        2.0 * (1.0 - normal_cdf(t.abs()))
    } else if (ma - mb).abs() > 0.0 {
        0.0 // means differ with zero variance ⇒ certainly different
    } else {
        1.0
    };
    Comparison {
        mean_a: ma,
        mean_b: mb,
        std_a: va.sqrt(),
        std_b: vb.sqrt(),
        n_a: na,
        n_b: nb,
        effect_size: effect,
        t_statistic: t,
        p_value_approx: p.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clearly_better_sample_is_significant() {
        let a = [-100.0, -101.0, -99.0, -100.5]; // candidate: much lower
        let b = [-10.0, -9.0, -11.0, -10.5]; // baseline
        let c = compare(&a, &b);
        assert!(c.mean_a < c.mean_b);
        assert!(c.effect_size > 0.0);
        assert!(c.is_significant_improvement(0.05));
        assert!(c.confidence() > 0.95);
    }

    #[test]
    fn identical_samples_are_not_significant() {
        let a = [-10.0, -10.0, -10.0];
        let b = [-10.0, -10.0, -10.0];
        let c = compare(&a, &b);
        assert!(!c.is_significant_improvement(0.05));
        assert!((c.p_value_approx - 1.0).abs() < 1e-9);
    }

    #[test]
    fn erf_is_reasonable() {
        assert!((erf(0.0)).abs() < 1e-9);
        assert!((erf(10.0) - 1.0).abs() < 1e-3);
        assert!((normal_cdf(0.0) - 0.5).abs() < 1e-6);
    }
}
