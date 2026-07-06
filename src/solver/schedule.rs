//! Data-driven temperature-range selection.
//!
//! Standard simulated-annealing practice (White, "Concepts of scale in
//! simulated annealing", IEEE ICCD 1984; the same construction is used as
//! the default β-range heuristic of D-Wave's `neal` SA sampler):
//!
//! - T_max: the largest possible single-flip |ΔE| — the maximum local field
//!   M_i = |h_i| + Σ_j |w_ij| — divided by ln 2, so even the worst uphill
//!   move is initially accepted with probability ≈ 1/2 (hot start).
//! - T_min: the smallest nonzero single-coefficient scale divided by ln 100,
//!   so the weakest interaction is resolved with acceptance ≈ 1% (cold end).
//!
//! This removes the hidden assumption that coefficients are O(1): scaling
//! every coefficient by 10^6 scales the returned range by 10^6 and leaves
//! the acceptance profile invariant.

use crate::core::QuboModel;

/// Suggested (temp_max, temp_min) for `model`. Falls back to (1.0, 0.1) for
/// degenerate all-zero models.
pub fn suggested_temp_range(model: &QuboModel) -> (f64, f64) {
    let mut max_local = 0.0f64;
    let mut min_nonzero = f64::INFINITY;
    for i in 0..model.num_vars {
        let mut local = model.linear[i].abs();
        if model.linear[i] != 0.0 {
            min_nonzero = min_nonzero.min(model.linear[i].abs());
        }
        for (_, w) in model.quadratic.get_row(i) {
            local += w.abs();
            if w != 0.0 {
                min_nonzero = min_nonzero.min(w.abs());
            }
        }
        max_local = max_local.max(local);
    }
    if max_local == 0.0 || !min_nonzero.is_finite() {
        return (1.0, 0.1);
    }
    let t_max = max_local / std::f64::consts::LN_2;
    let t_min = (min_nonzero / 100.0f64.ln()).min(t_max * 0.5);
    (t_max, t_min)
}
