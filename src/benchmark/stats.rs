//! Statistical primitives for benchmark analysis, dependency-free.
//!
//! Provides bootstrap confidence intervals plus two paired significance tests
//! (Wilcoxon signed-rank and Student's paired t) so that "solver A beats
//! solver B" claims carry a p-value. p-values need a normal CDF (Wilcoxon,
//! via `erf`) and a Student-t CDF (paired t, via the regularized incomplete
//! beta function `betai`); both are implemented here from standard numerical
//! recipes rather than pulling in a stats crate.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

// --------------------------------------------------------------------------
// Special functions
// --------------------------------------------------------------------------

/// Natural log of the gamma function (Lanczos approximation, g=5, n=6).
/// Accurate to ~1e-10 for x > 0 — ample for p-values.
pub fn gammaln(x: f64) -> f64 {
    const C: [f64; 6] = [
        76.180_091_729_471_46,
        -86.505_320_329_416_77,
        24.014_098_240_830_91,
        -1.231_739_572_450_155,
        0.120_865_097_386_617_9e-2,
        -0.539_523_938_495_3e-5,
    ];
    let mut y = x;
    let tmp = x + 5.5 - (x + 0.5) * (x + 5.5).ln();
    let mut ser = 1.000_000_000_190_015;
    for c in C {
        y += 1.0;
        ser += c / y;
    }
    -tmp + (2.506_628_274_631_000_5 * ser / x).ln()
}

/// Error function (Abramowitz & Stegun 7.1.26), max error ~1.5e-7.
pub fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    if x >= 0.0 {
        y
    } else {
        -y
    }
}

/// Standard normal CDF Φ(z).
pub fn normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// Continued fraction for the incomplete beta (Numerical Recipes `betacf`).
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    const MAXIT: usize = 300;
    const EPS: f64 = 3e-12;
    const FPMIN: f64 = 1e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=MAXIT {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Regularized incomplete beta I_x(a, b).
pub fn betai(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (gammaln(a + b) - gammaln(a) - gammaln(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Two-sided p-value for Student's t with `dof` degrees of freedom.
pub fn student_t_sf_two_sided(t: f64, dof: f64) -> f64 {
    if dof <= 0.0 {
        return f64::NAN;
    }
    betai(0.5 * dof, 0.5, dof / (dof + t * t))
}

// --------------------------------------------------------------------------
// Bootstrap
// --------------------------------------------------------------------------

/// Percentile bootstrap CI for the mean of `data`. Returns (lo, hi) at the
/// given two-sided confidence. Deterministic in `seed`.
pub fn bootstrap_mean_ci(data: &[f64], n_boot: usize, confidence: f64, seed: u64) -> (f64, f64) {
    if data.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut means: Vec<f64> = Vec::with_capacity(n_boot);
    for _ in 0..n_boot {
        let mut s = 0.0;
        for _ in 0..data.len() {
            s += data[rng.gen_range(0..data.len())];
        }
        means.push(s / data.len() as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let alpha = 1.0 - confidence;
    let lo = percentile(&means, alpha / 2.0);
    let hi = percentile(&means, 1.0 - alpha / 2.0);
    (lo, hi)
}

/// Fraction of `true` in a boolean slice (empty ⇒ 0).
pub fn mean_bool(v: &[bool]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().filter(|b| **b).count() as f64 / v.len() as f64
    }
}

/// Linear-interpolated percentile of a pre-sorted slice; `q` in [0, 1].
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let pos = q.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

// --------------------------------------------------------------------------
// Paired significance tests
// --------------------------------------------------------------------------

/// Result of a paired significance test.
#[derive(Debug, Clone, Copy)]
pub struct PairedTest {
    /// Test statistic (t, or Wilcoxon z).
    pub statistic: f64,
    /// Two-sided p-value.
    pub p_value: f64,
    /// Number of usable pairs (non-tied for Wilcoxon).
    pub n: usize,
}

/// Student's paired t-test on `a - b` (two-sided). Requires ≥ 2 pairs.
pub fn paired_t_test(a: &[f64], b: &[f64]) -> PairedTest {
    assert_eq!(a.len(), b.len(), "paired samples must align");
    let n = a.len();
    if n < 2 {
        return PairedTest {
            statistic: f64::NAN,
            p_value: f64::NAN,
            n,
        };
    }
    let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    let mean = d.iter().sum::<f64>() / n as f64;
    let var = d.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0);
    if var <= 0.0 {
        // All differences identical: significant iff nonzero mean.
        let p = if mean.abs() > 0.0 { 0.0 } else { 1.0 };
        return PairedTest {
            statistic: if mean == 0.0 { 0.0 } else { f64::INFINITY },
            p_value: p,
            n,
        };
    }
    let se = (var / n as f64).sqrt();
    let t = mean / se;
    let p = student_t_sf_two_sided(t, n as f64 - 1.0);
    PairedTest {
        statistic: t,
        p_value: p,
        n,
    }
}

/// Wilcoxon signed-rank test on paired samples (two-sided, normal
/// approximation with continuity correction and tie handling). Zero
/// differences are dropped (Wilcoxon's original method).
pub fn wilcoxon_signed_rank(a: &[f64], b: &[f64]) -> PairedTest {
    assert_eq!(a.len(), b.len(), "paired samples must align");
    let diffs: Vec<f64> = a
        .iter()
        .zip(b)
        .map(|(x, y)| x - y)
        .filter(|d| *d != 0.0)
        .collect();
    let n = diffs.len();
    if n < 1 {
        return PairedTest {
            statistic: f64::NAN,
            p_value: 1.0,
            n,
        };
    }
    // Rank by |d| with average ranks for ties.
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&i, &j| diffs[i].abs().partial_cmp(&diffs[j].abs()).unwrap());
    let mut ranks = vec![0.0f64; n];
    let mut tie_sum = 0.0f64; // Σ(t³ − t) for the variance correction
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && (diffs[idx[j]].abs() - diffs[idx[i]].abs()).abs() < 1e-12 {
            j += 1;
        }
        let group = (j - i) as f64;
        let avg_rank = ((i + 1 + j) as f64) / 2.0; // mean of ranks i+1..=j (1-indexed)
        for &k in &idx[i..j] {
            ranks[k] = avg_rank;
        }
        tie_sum += group * group * group - group;
        i = j;
    }
    let w_plus: f64 = (0..n).filter(|&k| diffs[k] > 0.0).map(|k| ranks[k]).sum();
    let nf = n as f64;
    let mean = nf * (nf + 1.0) / 4.0;
    let var = nf * (nf + 1.0) * (2.0 * nf + 1.0) / 24.0 - tie_sum / 48.0;
    if var <= 0.0 {
        return PairedTest {
            statistic: 0.0,
            p_value: 1.0,
            n,
        };
    }
    // Continuity correction toward the mean.
    let diff = w_plus - mean;
    let z = (diff - diff.signum() * 0.5) / var.sqrt();
    let p = 2.0 * (1.0 - normal_cdf(z.abs()));
    PairedTest {
        statistic: z,
        p_value: p.clamp(0.0, 1.0),
        n,
    }
}
