//! Deterministic local-search polish.
//!
//! Steepest-descent 1-opt with incrementally maintained flip gains — the
//! standard finishing pass of UBQP heuristics (the descent core of tabu
//! search: Glover, Lü & Hao, 4OR 8, 239 (2010); Palubeckis, Ann. OR 131,
//! 259 (2004)). Guarantees that every returned solution is a 1-opt local
//! minimum over the FREE variables: no single flip of an unclamped variable
//! can lower the energy. Deterministic: steepest gain, lowest index on ties.

use crate::core::QuboModel;

/// Polishes `state` in place to a 1-opt local minimum (free variables only).
/// Returns the total energy improvement (≥ 0).
///
/// Gains g_i = ΔE of flipping x_i are maintained incrementally: a flip of i
/// updates g_j for neighbors j in O(deg(i)) via the exact identity
/// g_j ← g_j + s_j·s_i·w_ij after the flip (s = ±1 in flip direction), and
/// g_i ← −g_i. Total cost O(nnz) per descent pass.
pub fn steepest_descent_1opt(model: &QuboModel, state: &mut [i8], is_clamped: &[bool]) -> f64 {
    let n = model.num_vars;
    // g[i] = E(flip i) − E(current) = (1 − 2x_i)(h_i + Σ_j w_ij x_j)
    let mut gains = vec![0.0f64; n];
    for i in 0..n {
        let mut field = model.linear[i];
        for (j, w) in model.quadratic.get_row(i) {
            field += w * (state[j] as f64);
        }
        gains[i] = (1.0 - 2.0 * (state[i] as f64)) * field;
    }

    let mut total_improvement = 0.0;
    loop {
        // Steepest improving flip among free variables (lowest index ties).
        let mut best = (usize::MAX, -1e-12);
        for i in 0..n {
            if !is_clamped[i] && gains[i] < best.1 {
                best = (i, gains[i]);
            }
        }
        let (i, gain) = best;
        if i == usize::MAX {
            return total_improvement;
        }
        total_improvement -= gain;

        // Flip i; update its own gain and neighbors' gains incrementally.
        // For neighbor j: g_j depends on field_j, which changes by ±w_ij.
        // sign = +w if i flipped 0→1, −w if 1→0; g_j changes by
        // (1 − 2x_j)·(±w_ij).
        let delta_dir = 1.0 - 2.0 * (state[i] as f64); // +1 if 0→1, −1 if 1→0
        state[i] = 1 - state[i];
        gains[i] = -gain;
        for (j, w) in model.quadratic.get_row(i) {
            gains[j] += (1.0 - 2.0 * (state[j] as f64)) * w * delta_dir;
        }
    }
}

/// Computes the exact single-flip gains g_i = E(flip i) − E(current) for all variables.
#[inline]
pub fn compute_single_flip_gains(model: &QuboModel, state: &[i8]) -> Vec<f64> {
    let n = model.num_vars;
    let mut gains = vec![0.0f64; n];
    for i in 0..n {
        let mut field = model.linear[i];
        for (j, w) in model.quadratic.get_row(i) {
            field += w * (state[j] as f64);
        }
        gains[i] = (1.0 - 2.0 * (state[i] as f64)) * field;
    }
    gains
}

/// Finds the steepest improving 2-flip along graph edges (u, v) ∈ E with u < v.
///
/// By Theorem 1 (Edge-Restricted 2-Opt), when a state is at a 1-opt local minimum
/// (where all g_i ≥ 0), any non-adjacent pair (u, v) ∉ E has
/// ΔE(u, v) = g_u + g_v ≥ 0. Therefore, non-edges can NEVER improve energy.
/// Scanning strictly the |E| graph edges is provably 100% complete.
///
/// Returns Some((u, v, delta_e)) with lowest-index tie-breaking if an improving
/// 2-flip exists (delta_e < -1e-12), or None if no improving 2-flip exists.
pub fn find_steepest_2opt_edge_flip(
    model: &QuboModel,
    state: &[i8],
    gains: &[f64],
    is_clamped: &[bool],
) -> Option<(usize, usize, f64)> {
    let n = model.num_vars;
    let mut best: Option<(usize, usize, f64)> = None;

    for u in 0..n {
        if is_clamped.get(u).copied().unwrap_or(false) {
            continue;
        }
        let s_u = 1.0 - 2.0 * (state[u] as f64);
        for (v, w) in model.quadratic.get_row(u) {
            if v > u && !is_clamped.get(v).copied().unwrap_or(false) {
                let s_v = 1.0 - 2.0 * (state[v] as f64);
                let delta_e = gains[u] + gains[v] + w * s_u * s_v;
                if delta_e < -1e-12 {
                    match best {
                        None => best = Some((u, v, delta_e)),
                        Some((_, _, best_delta)) => {
                            if delta_e < best_delta {
                                best = Some((u, v, delta_e));
                            }
                        }
                    }
                }
            }
        }
    }
    best
}

/// Escapes 1-opt local minima by repeatedly applying the steepest improving edge-restricted
/// 2-flip and descending to a new 1-opt minimum, until no improving 1-flip or 2-flip exists.
///
/// By Theorem 1, this guarantees that the resulting configuration is a true (1+2)-opt
/// local optimum across all single flips and all N(N-1)/2 pairwise flips.
/// Returns the total additional energy improvement achieved beyond the initial state (≥ 0).
pub fn steepest_descent_2opt_escapes(
    model: &QuboModel,
    state: &mut [i8],
    is_clamped: &[bool],
) -> f64 {
    let e_start = model.calculate_total_energy(state);
    loop {
        steepest_descent_1opt(model, state, is_clamped);
        let gains = compute_single_flip_gains(model, state);
        if let Some((u, v, _delta_e)) =
            find_steepest_2opt_edge_flip(model, state, &gains, is_clamped)
        {
            state[u] = 1 - state[u];
            state[v] = 1 - state[v];
        } else {
            break;
        }
    }
    let e_end = model.calculate_total_energy(state);
    (e_start - e_end).max(0.0)
}
