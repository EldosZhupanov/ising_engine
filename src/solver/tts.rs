//! Time-To-Solution (TTS) framework.
//!
//! References:
//! - Rønnow, Wang, Job, Boixo, Isakov, Wecker, Martinis, Lidar & Troyer,
//!   "Defining and detecting quantum speedup", Science 345, 420 (2014) —
//!   the optimal-stopping TTS metric used throughout the field.
//! - Luby, Sinclair & Zuckerman, "Optimal speedup of Las Vegas algorithms",
//!   Inf. Proc. Lett. 47, 173 (1993) — the universal restart schedule.
//!
//! TTS(q) is the wall-clock (or sweep) time needed so that, running
//! independent repetitions each of duration `t_run`, at least one reaches
//! the target with probability q:
//!
//!   TTS(q) = t_run · ln(1 − q) / ln(1 − p_s),
//!
//! where p_s is the single-run success probability. Bootstrap resampling
//! gives confidence intervals on p_s and hence on TTS.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Single-run success probability from Bernoulli trials.
pub fn success_probability(successes: &[bool]) -> f64 {
    if successes.is_empty() {
        return 0.0;
    }
    successes.iter().filter(|&&s| s).count() as f64 / successes.len() as f64
}

/// TTS(q) from a single-run success probability and per-run cost `t_run`.
///
/// Edge cases: p_s ≥ 1 ⇒ one run suffices ⇒ TTS = t_run; p_s ≤ 0 ⇒ +∞.
/// The standard quantile is q = 0.99.
pub fn tts(p_success: f64, t_run: f64, quantile: f64) -> f64 {
    assert!((0.0..1.0).contains(&quantile), "quantile must be in [0,1)");
    if p_success >= 1.0 {
        return t_run;
    }
    if p_success <= 0.0 {
        return f64::INFINITY;
    }
    t_run * (1.0 - quantile).ln() / (1.0 - p_success).ln()
}

/// Percentile bootstrap confidence interval for the success probability.
/// Returns (lower, upper) at the given two-sided `confidence` (e.g. 0.95).
/// Deterministic given `seed`.
pub fn bootstrap_success_ci(
    successes: &[bool],
    n_boot: usize,
    confidence: f64,
    seed: u64,
) -> (f64, f64) {
    let n = successes.len();
    if n == 0 || n_boot == 0 {
        return (0.0, 0.0);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut props: Vec<f64> = Vec::with_capacity(n_boot);
    for _ in 0..n_boot {
        let mut hits = 0usize;
        for _ in 0..n {
            if successes[rng.gen_range(0..n)] {
                hits += 1;
            }
        }
        props.push(hits as f64 / n as f64);
    }
    props.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let alpha = (1.0 - confidence) / 2.0;
    let lo = props[((alpha * n_boot as f64) as usize).min(n_boot - 1)];
    let hi = props[(((1.0 - alpha) * n_boot as f64) as usize).min(n_boot - 1)];
    (lo, hi)
}

/// TTS confidence interval, obtained by propagating the bootstrap CI on p_s
/// through the (monotone-decreasing in p_s) TTS formula: the lower p_s bound
/// gives the upper TTS bound and vice versa.
pub fn tts_ci(
    successes: &[bool],
    t_run: f64,
    quantile: f64,
    n_boot: usize,
    confidence: f64,
    seed: u64,
) -> (f64, f64) {
    let (p_lo, p_hi) = bootstrap_success_ci(successes, n_boot, confidence, seed);
    (tts(p_hi, t_run, quantile), tts(p_lo, t_run, quantile))
}

/// The Luby sequence (1-indexed): 1,1,2,1,1,2,4,1,1,2,1,1,2,4,8,…
/// Restart i uses a budget of `unit · luby(i)`; this schedule is within a
/// logarithmic factor of the optimal fixed cutoff for any Las Vegas
/// algorithm without knowing its runtime distribution.
pub fn luby(i: usize) -> usize {
    assert!(i >= 1, "luby is 1-indexed");
    // Find k with 2^(k-1) ≤ i ≤ 2^k − 1.
    let mut k = 1;
    let mut pow = 1usize; // 2^(k-1)
    loop {
        let seg_end = 2 * pow - 1; // 2^k − 1
        if i == seg_end {
            return pow;
        }
        if i < seg_end {
            // Recurse into the earlier identical block.
            return luby(i - pow + 1);
        }
        k += 1;
        pow = 1 << (k - 1);
    }
}

/// Plateau-based convergence detector: returns true if the best energy over
/// the last `window` observations improved by less than `tol`.
pub fn has_converged(best_energy_history: &[f64], window: usize, tol: f64) -> bool {
    let n = best_energy_history.len();
    if n < window || window == 0 {
        return false;
    }
    let recent = &best_energy_history[n - window..];
    let max = recent.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = recent.iter().cloned().fold(f64::INFINITY, f64::min);
    (max - min).abs() <= tol
}
