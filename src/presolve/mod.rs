//! Presolve: provably-safe problem reductions applied before annealing.
//!
//! Currently implements first-order persistency (variable fixing) from roof
//! duality: Hammer, Hansen & Simeone, "Roof duality, complementation and
//! persistency in quadratic 0-1 optimization", Math. Programming 28 (1984);
//! Boros & Hammer, "Pseudo-Boolean optimization", Discrete Appl. Math. 123
//! (2002). The same rules serve as the basic preprocessing pass in Glover,
//! Lewis & Kochenberger, EJOR 265 (2018).
//!
//! For E(x) = Σ h_i·x_i + Σ_{i<j} w_ij·x_i·x_j with x ∈ {0,1}^n, the discrete
//! derivative of variable i is δ_i(x) = E(x_i=1) − E(x_i=0) = h_i + Σ_j w_ij·x_j.
//! Bounding δ_i over all configurations of the free neighbors:
//!
//!   L_i = h_i + Σ_j min(0, w_ij)  ≤  δ_i  ≤  h_i + Σ_j max(0, w_ij) = U_i
//!
//! If L_i ≥ 0, setting x_i = 0 never increases E, so at least one optimal
//! solution has x_i = 0 (weak persistency). If U_i ≤ 0, symmetrically x_i = 1.
//! Fixing x_i = 1 folds w_ij into each neighbor's field h_j, which can make
//! neighbors fixable in turn — the rules iterate to a fixpoint. Each rule
//! application is valid in the reduced problem where prior fixings are
//! substituted, so by induction the final partial assignment extends to a
//! global optimum of the original (clamped) problem.

use crate::core::{CsrMatrix, QuboModel};

/// Derives provably-safe variable fixings via first-order persistency,
/// iterated to a fixpoint.
///
/// User clamps are treated as given constants: their values are folded into
/// the effective fields of their neighbors, and clamped indices never appear
/// in the returned list. Applying every returned fixing preserves at least
/// one globally optimal solution of the clamped problem.
///
/// Complexity: O(passes × nnz), at most n passes (each non-final pass fixes
/// at least one variable); deterministic.
pub fn fix_persistent_variables(model: &QuboModel, clamped: &[(usize, i8)]) -> Vec<(usize, i8)> {
    let status = first_order_fixpoint(model, clamped);
    let mut user_clamped = vec![false; model.num_vars];
    for &(idx, _) in clamped {
        user_clamped[idx] = true;
    }
    (0..model.num_vars)
        .filter(|&i| !user_clamped[i])
        .filter_map(|i| status[i].map(|v| (i, v)))
        .collect()
}

/// First-order persistency propagation to a fixpoint, given a set of
/// assumed assignments `assumptions`. Returns the full status vector
/// (`Some(v)` = fixed/assumed, `None` = free). This is the reusable core
/// shared by `fix_persistent_variables` and `fix_persistent_variables_probing`.
pub fn first_order_fixpoint(model: &QuboModel, assumptions: &[(usize, i8)]) -> Vec<Option<i8>> {
    let n = model.num_vars;
    let mut h = model.linear.clone();
    let mut status: Vec<Option<i8>> = vec![None; n];
    for &(idx, val) in assumptions {
        status[idx] = Some(val);
    }
    // Fold assumed-1 variables into neighbor fields.
    for &(idx, val) in assumptions {
        if val == 1 {
            for (j, w) in model.quadratic.get_row(idx) {
                h[j] += w;
            }
        }
    }

    loop {
        let mut changed = false;
        for i in 0..n {
            if status[i].is_some() {
                continue;
            }
            // Bounds over free neighbors only: fixed-to-1 neighbors are
            // already folded into h[i], fixed-to-0 neighbors contribute 0.
            let mut lo = h[i];
            let mut hi = h[i];
            for (j, w) in model.quadratic.get_row(i) {
                if status[j].is_none() {
                    if w < 0.0 {
                        lo += w;
                    } else {
                        hi += w;
                    }
                }
            }
            if lo >= 0.0 {
                status[i] = Some(0);
                changed = true;
            } else if hi <= 0.0 {
                status[i] = Some(1);
                for (j, w) in model.quadratic.get_row(i) {
                    h[j] += w;
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    status
}

/// Exact probing persistency: strictly stronger than first-order, provably
/// optimum-preserving.
///
/// For each free variable i, propagate first-order persistency separately
/// under the two case splits x_i = 0 and x_i = 1. If some variable j is
/// fixed to the SAME value v in BOTH branches, then j = v holds in a global
/// optimum regardless of x_i, so it is an unconditional fixing.
///
/// Correctness proof: first-order persistency in the x_i = a subproblem
/// yields an optimum OF THAT SUBPROBLEM with x_j = v_a. The global optimum
/// has x_i = 0 or x_i = 1 and is therefore an optimum of the corresponding
/// subproblem; if v_0 = v_1 = v, then in either case a global optimum with
/// x_j = v exists. QED. (This is the standard MIP "probing" reduction —
/// Savelsbergh, ORSA J. Comput. 6 (1994); Achterberg et al., INFORMS J.
/// Comput. 32 (2020) — specialized to unconstrained pseudo-Boolean
/// optimization, where every branch is feasible.)
///
/// User-clamped indices are respected and never returned. Iterates to a
/// fixpoint. Complexity O(rounds · n · nnz); deterministic.
pub fn fix_persistent_variables_probing(
    model: &QuboModel,
    clamped: &[(usize, i8)],
) -> Vec<(usize, i8)> {
    let n = model.num_vars;
    let mut user_clamped = vec![false; n];
    let mut assumptions: Vec<(usize, i8)> = clamped.to_vec();
    for &(idx, _) in clamped {
        user_clamped[idx] = true;
    }

    loop {
        // First-order fixpoint under current assumptions.
        let base = first_order_fixpoint(model, &assumptions);
        let mut newly: Vec<(usize, i8)> = Vec::new();
        let assumed: Vec<(usize, i8)> = (0..n).filter_map(|i| base[i].map(|v| (i, v))).collect();

        // Probe each still-free variable.
        for i in 0..n {
            if base[i].is_some() {
                continue;
            }
            let mut a0 = assumed.clone();
            a0.push((i, 0));
            let branch0 = first_order_fixpoint(model, &a0);
            let mut a1 = assumed.clone();
            a1.push((i, 1));
            let branch1 = first_order_fixpoint(model, &a1);

            for j in 0..n {
                if j == i || base[j].is_some() {
                    continue;
                }
                if let (Some(v0), Some(v1)) = (branch0[j], branch1[j]) {
                    if v0 == v1 && !newly.iter().any(|&(k, _)| k == j) {
                        newly.push((j, v0));
                    }
                }
            }
        }

        // Merge base fixings + newly probed fixings into assumptions.
        let before = assumptions.len();
        for (i, v) in assumed.into_iter().chain(newly) {
            if !assumptions.iter().any(|&(k, _)| k == i) {
                assumptions.push((i, v));
            }
        }
        if assumptions.len() == before {
            break;
        }
    }

    assumptions
        .into_iter()
        .filter(|&(i, _)| !user_clamped[i])
        .collect()
}

/// Connected components of the interaction graph restricted to `active`
/// vertices (an edge exists where w_ij ≠ 0 and both endpoints are active).
///
/// Because the QUBO energy has no terms between different components of the
/// active subgraph, the restricted problem decomposes additively and
/// per-component optima compose into a global optimum.
///
/// Deterministic: components are discovered in ascending order of their
/// smallest vertex, and each component's vertex list is sorted.
pub fn connected_components(model: &QuboModel, active: &[bool]) -> Vec<Vec<usize>> {
    let n = model.num_vars;
    let mut visited = vec![false; n];
    let mut components = Vec::new();
    let mut stack = Vec::new();
    for start in 0..n {
        if !active[start] || visited[start] {
            continue;
        }
        let mut comp = Vec::new();
        visited[start] = true;
        stack.push(start);
        while let Some(v) = stack.pop() {
            comp.push(v);
            for (j, w) in model.quadratic.get_row(v) {
                if w != 0.0 && active[j] && !visited[j] {
                    visited[j] = true;
                    stack.push(j);
                }
            }
        }
        comp.sort_unstable();
        components.push(comp);
    }
    components
}

/// Extracts the sub-QUBO induced by `comp` (sorted global indices of one
/// connected component of the free-variable graph).
///
/// Precondition: every neighbor of a component member is either itself a
/// member or a fixed variable. Couplings into variables fixed at 1 are folded
/// into the local linear fields (`fixed_one`); couplings into variables fixed
/// at 0 vanish. Returns the sub-model and the local→global index map.
pub fn extract_component(
    model: &QuboModel,
    comp: &[usize],
    fixed_one: &[bool],
) -> (QuboModel, Vec<usize>) {
    let mut global_to_local = vec![usize::MAX; model.num_vars];
    for (li, &gi) in comp.iter().enumerate() {
        global_to_local[gi] = li;
    }
    let mut linear = Vec::with_capacity(comp.len());
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for &gi in comp {
        let mut h = model.linear[gi];
        for (j, w) in model.quadratic.get_row(gi) {
            if global_to_local[j] != usize::MAX {
                col_indices.push(global_to_local[j]);
                values.push(w);
            } else if fixed_one[j] {
                h += w;
            }
        }
        linear.push(h);
        row_offsets.push(col_indices.len());
    }
    (
        QuboModel {
            num_vars: comp.len(),
            linear,
            quadratic: CsrMatrix {
                values,
                col_indices,
                row_offsets,
            },
            // Sub-model constants (fixed-variable contributions) are
            // deliberately dropped: sub-solves are argmin-only, and results
            // are re-scored on the ORIGINAL model, whose offset is intact.
            energy_offset: 0.0,
        },
        comp.to_vec(),
    )
}
