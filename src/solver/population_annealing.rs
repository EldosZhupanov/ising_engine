//! Population Annealing resampling primitives.
//!
//! Implements the standard Population Annealing (PA) reweighting/resampling
//! machinery of Machta, "Population annealing with weighted averages",
//! PRE 82, 026704 (2010), as characterized further in Wang, Machta &
//! Katzgraber, "Population annealing: Theory and application in spin
//! glasses", PRE 92, 063307 (2015):
//!
//! - When the inverse temperature moves β → β′, replica i acquires the
//!   Boltzmann reweighting factor  w_i ∝ exp(−(β′ − β)·E_i).
//! - Log-weights are accumulated across schedule steps and normalized with
//!   the log-sum-exp trick for numerical stability.
//! - Effective Sample Size ESS = 1 / Σ w̃_i² (w̃ normalized) measures weight
//!   degeneracy; resampling triggers when ESS drops below a configurable
//!   fraction of the population size (standard adaptive-resampling practice
//!   in sequential Monte Carlo).
//! - Resampling is SYSTEMATIC: offspring counts are determined by a single
//!   uniform offset u ∈ [0,1) via positions (j + u)/R against the cumulative
//!   weights. Systematic resampling is unbiased (E_u[count_i] = R·w̃_i) and
//!   has minimal variance: every count lies in {⌊R·w̃_i⌋, ⌈R·w̃_i⌉}.
//! - Offspring are placed at randomly shuffled destinations to avoid
//!   deterministic cloning patterns.
//!
//! In the PA-PT hybrid (see UltimateSolver::solve_population_annealing),
//! a population member owns a complete Parallel Tempering ladder; resampling
//! copies whole-ladder configurations, and the reweighting energies come
//! from the coldest level — the one whose β advances between stages.
//!
//! All randomness is caller-supplied, so runs are deterministic given a seed.

use crate::solver::types::{QuantumField, NUM_REPLICAS};
use rand::Rng;

/// Population-Annealing run diagnostics (Machta 2010; Wang, Machta &
/// Katzgraber 2015).
#[derive(Debug, Clone, Default)]
pub struct PaDiagnostics {
    /// Accumulated dimensionless free-energy change −β(F_final − F_init),
    /// summed over resampling events from the pre-reset log-weights.
    pub free_energy: f64,
    /// Effective sample size recorded at every annealing stage.
    pub ess_history: Vec<f64>,
    /// (family entropy ρ, surviving family count) after each resampling.
    pub family_entropy_history: Vec<(f64, usize)>,
    /// Number of resampling events triggered.
    pub resample_events: usize,
}

/// Normalizes accumulated log-weights into probabilities using log-sum-exp.
pub fn normalized_weights(log_w: &[f64]) -> Vec<f64> {
    let m = log_w.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = log_w.iter().map(|&lw| (lw - m).exp()).collect();
    let z: f64 = exps.iter().sum();
    exps.into_iter().map(|e| e / z).collect()
}

/// Effective Sample Size of a NORMALIZED weight vector: ESS = 1 / Σ w̃².
/// Equal weights give ESS = R; a single dominant weight gives ESS → 1.
pub fn effective_sample_size(w_norm: &[f64]) -> f64 {
    let s2: f64 = w_norm.iter().map(|w| w * w).sum();
    1.0 / s2
}

/// Free-energy increment for one annealing step (Machta 2010, "weighted
/// averages"): with cumulative log-weights `log_w` since the last resample,
/// Δ(−βF) = logsumexp(log_w) − ln R (the log of the mean reweighting factor).
/// Summing over resampling events yields −β(F_final − F_init) up to the
/// reference — the standard PA free-energy estimator.
pub fn free_energy_increment(log_w: &[f64]) -> f64 {
    let r = log_w.len() as f64;
    let m = log_w.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if !m.is_finite() {
        return 0.0;
    }
    let s: f64 = log_w.iter().map(|&lw| (lw - m).exp()).sum();
    m + s.ln() - r.ln()
}

/// Family entropy (Wang, Machta & Katzgraber, PRE 92, 063307 (2015)):
/// ρ_t = R · Σ_f (n_f / R)². ρ ≈ 1 ⇒ every member its own family
/// (well-equilibrated); large ρ ⇒ few ancestral families dominate.
/// Returns (rho, surviving_family_count).
pub fn family_entropy(family_ids: &[usize]) -> (f64, usize) {
    let r = family_ids.len();
    if r == 0 {
        return (0.0, 0);
    }
    let mut counts = std::collections::HashMap::new();
    for &f in family_ids {
        *counts.entry(f).or_insert(0usize) += 1;
    }
    let rho: f64 = counts
        .values()
        .map(|&n| {
            let frac = n as f64 / r as f64;
            frac * frac
        })
        .sum::<f64>()
        * r as f64;
    (rho, counts.len())
}

/// Systematic resampling: offspring counts per parent for population size
/// R = w_norm.len(), driven by a single uniform offset u ∈ [0, 1).
/// Guarantees Σ counts = R and |count_i − R·w̃_i| < 1.
pub fn systematic_resample(w_norm: &[f64], u: f64) -> Vec<usize> {
    let r_total = w_norm.len();
    let mut counts = vec![0usize; r_total];
    let mut cum = 0.0f64;
    let mut j = 0usize; // next sampling position index
    for (i, &w) in w_norm.iter().enumerate() {
        cum += w;
        // Positions (j + u)/R that fall below the cumulative weight belong
        // to parent i. The final parent absorbs any float-rounding residue.
        while j < r_total && ((j as f64 + u) / r_total as f64) < cum {
            counts[i] += 1;
            j += 1;
        }
    }
    // Float-rounding guard: assign any unplaced positions to the last parent.
    if j < r_total {
        counts[r_total - 1] += r_total - j;
    }
    counts
}

/// One acceptance-uniformization update of adjacent log-temperature ladder
/// spacings: Δ′_t ∝ Δ_t·√max(A_t, floor), renormalized to preserve the total
/// span. Pairs with high swap acceptance widen, pairs with low acceptance
/// narrow; the fixed point is uniform acceptance across all pairs.
///
/// This is the acceptance-based ladder-feedback variant (Rathore, Chopra &
/// de Pablo, J. Chem. Phys. 122, 024111 (2005); Kofke, J. Chem. Phys. 117,
/// 6911 (2002)); round-trip-flow optimization (Katzgraber, Trebst, Huse &
/// Troyer, JSTAT P03018 (2006)) is the stricter refinement of the same idea.
/// The √ damps the update for stable convergence over a few iterations.
pub fn adapt_spacings(spacings: &[f64], acceptance: &[f64]) -> Vec<f64> {
    assert_eq!(spacings.len(), acceptance.len());
    const ACCEPTANCE_FLOOR: f64 = 0.02;
    let raw: Vec<f64> = spacings
        .iter()
        .zip(acceptance)
        .map(|(&d, &a)| d * a.max(ACCEPTANCE_FLOOR).sqrt())
        .collect();
    let total_old: f64 = spacings.iter().sum();
    let total_new: f64 = raw.iter().sum();
    raw.into_iter().map(|d| d * total_old / total_new).collect()
}

/// Feedback-optimized ladder redistribution (Katzgraber, Trebst, Huse &
/// Troyer, "Feedback-optimized parallel tempering Monte Carlo", JSTAT
/// P03018 (2006)).
///
/// Given the current interior positions `alphas` (in [0,1] log-temperature
/// space, α[0]=0 hot, α[last]=1 cold) and the measured replica flow
/// `flow[k]` = fraction of up-moving replicas at ladder point k, the optimal
/// local temperature density is η(α) ∝ √( (df/dα) ), so the mass of an
/// interval is m_k = √( |Δf_k| · Δα_k ). New points equidistribute the
/// cumulative ∫η dα: point i is placed where the cumulative mass reaches
/// i/(M) of the total. Endpoints stay pinned. `damping` ∈ (0,1] blends the
/// new profile with the old for stable convergence.
///
/// The flow is first made monotone (cumulative max) since sampling noise can
/// produce small dips; f is theoretically monotone increasing hot→cold.
pub fn feedback_optimized_alphas(alphas: &[f64], flow: &[f64], damping: f64) -> Vec<f64> {
    let nt = alphas.len();
    assert_eq!(flow.len(), nt);
    if nt < 3 {
        return alphas.to_vec();
    }
    // Monotonize the flow (cumulative max from the hot end).
    let mut f = flow.to_vec();
    for k in 1..nt {
        if f[k] < f[k - 1] {
            f[k] = f[k - 1];
        }
    }
    // Interval masses m_k = √(|Δf_k| · Δα_k), with a floor so empty-flow
    // intervals still receive a minimal density (avoids collapse).
    const EPS: f64 = 1e-6;
    let mut mass = vec![0.0f64; nt - 1];
    for k in 0..nt - 1 {
        let df = (f[k + 1] - f[k]).abs().max(EPS);
        let da = (alphas[k + 1] - alphas[k]).abs().max(EPS);
        mass[k] = (df * da).sqrt();
    }
    let total: f64 = mass.iter().sum();
    if total <= 0.0 || !total.is_finite() {
        return alphas.to_vec();
    }
    // Cumulative mass at each ladder point.
    let mut cum = vec![0.0f64; nt];
    for k in 0..nt - 1 {
        cum[k + 1] = cum[k] + mass[k];
    }
    // Equidistribute: target cumulative level for point i is i/(nt-1)·total.
    let mut new_alphas = vec![0.0f64; nt];
    new_alphas[0] = alphas[0];
    new_alphas[nt - 1] = alphas[nt - 1];
    for i in 1..nt - 1 {
        let target = i as f64 / (nt - 1) as f64 * total;
        // Find interval k with cum[k] ≤ target ≤ cum[k+1].
        let mut k = 0;
        while k < nt - 2 && cum[k + 1] < target {
            k += 1;
        }
        let frac = if mass[k] > 0.0 {
            (target - cum[k]) / mass[k]
        } else {
            0.0
        };
        let interp = alphas[k] + frac * (alphas[k + 1] - alphas[k]);
        new_alphas[i] = damping * interp + (1.0 - damping) * alphas[i];
    }
    // Enforce strict monotonicity (numerical safety).
    for i in 1..nt {
        if new_alphas[i] <= new_alphas[i - 1] {
            new_alphas[i] = new_alphas[i - 1] + EPS;
        }
    }
    // Renormalize interior to keep endpoints exact.
    let base = new_alphas[0];
    let span = new_alphas[nt - 1] - new_alphas[0];
    let old_span = alphas[nt - 1] - alphas[0];
    if span > 0.0 {
        let a0 = alphas[0];
        for a in new_alphas.iter_mut() {
            *a = a0 + (*a - base) / span * old_span;
        }
    }
    new_alphas
}

/// Applies a resampling plan to the population field. A population MEMBER is
/// lane r of population p across the ENTIRE temperature ladder (PA-PT
/// hybrid): each offspring receives its parent's complete configuration at
/// every temperature level and slice, plus ALL of its tracked energies, so
/// spins and energies remain exactly synchronized at every level. Offspring
/// destinations are the population slots in randomly shuffled parent order.
pub fn apply_resample<R: Rng>(
    field: &mut QuantumField,
    counts: &[usize],
    rng: &mut R,
) -> Vec<usize> {
    let r_total = field.num_pops * NUM_REPLICAS;
    assert_eq!(counts.len(), r_total);
    debug_assert_eq!(counts.iter().sum::<usize>(), r_total);

    // Parent index for each offspring slot, then random redistribution.
    let mut parents = Vec::with_capacity(r_total);
    for (i, &c) in counts.iter().enumerate() {
        for _ in 0..c {
            parents.push(i);
        }
    }
    // Fisher-Yates shuffle with caller RNG (deterministic given seed).
    for k in (1..parents.len()).rev() {
        let j = rng.gen_range(0..=k);
        parents.swap(k, j);
    }

    // Snapshot, then scatter. (Driver-level code — not the sweep hot path.)
    let old_spins = field.spins.clone();
    let old_energies = field.energies.clone();
    let per_member = field.num_vars * field.num_slices;
    let num_temps = field.num_temps;

    for (dst, &src) in parents.iter().enumerate() {
        if dst == src {
            continue;
        }
        let (dp, dr) = (dst / NUM_REPLICAS, dst % NUM_REPLICAS);
        let (sp, sr) = (src / NUM_REPLICAS, src % NUM_REPLICAS);
        for t in 0..num_temps {
            // Cell (t, p) is contiguous: offset (t + num_temps·p)·per_member·64.
            let d_cell = (t + num_temps * dp) * per_member;
            let s_cell = (t + num_temps * sp) * per_member;
            for sv in 0..per_member {
                field.spins[(d_cell + sv) * NUM_REPLICAS + dr] =
                    old_spins[(s_cell + sv) * NUM_REPLICAS + sr];
            }
            field.energies[t + num_temps * dp][dr] = old_energies[t + num_temps * sp][sr];
        }
    }
    // parents[dst] = src parent slot, for ancestor/family tracking.
    parents
}
