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
