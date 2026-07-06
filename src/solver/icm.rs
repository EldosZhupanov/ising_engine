//! Isoenergetic Cluster Moves (Houdayer / Zhu–Ochoa–Katzgraber ICM).
//!
//! References:
//! - Houdayer, "A cluster Monte Carlo algorithm for 2-dimensional spin
//!   glasses", Eur. Phys. J. B 22, 479 (2001).
//! - Zhu, Ochoa & Katzgraber, "Efficient cluster algorithm for spin
//!   glasses in any space dimension", PRL 115, 077201 (2015).
//!
//! ## The move
//!
//! Two replicas at the SAME temperature, configurations x (lane a) and
//! y (lane b). Consider the *overlap*: sites disagree where x_i ≠ y_i. The
//! disagreement sites, connected through the interaction graph, form
//! clusters. Flipping one whole cluster C in BOTH replicas
//! (x_i → 1−x_i and y_i → 1−y_i for i ∈ C) conserves the pair's total
//! energy exactly, so the move is rejection-free.
//!
//! ## Isoenergetic proof (product basis, x ∈ {0,1})
//!
//! For i ∈ C, y_i = 1 − x_i (they disagree). Then ΔE_a + ΔE_b = 0 term by
//! term:
//! - Linear i∈C: h_i(1−2x_i) + h_i(1−2y_i) = h_i(1−2x_i) − h_i(1−2x_i) = 0.
//! - Internal edge (i,j∈C): lane-b term = −(lane-a term) via y=1−x.
//! - Boundary edge (i∈C, j∉C): since C is a connected component of the
//!   disagreement graph, j∉C on an edge ⇒ j AGREES (x_j = y_j), giving
//!   lane-b term = −(lane-a term).
//! - External edges (neither in C): unchanged.
//!
//! Individually ΔE_a = −ΔE_b ≠ 0 in general (the replicas exchange energy);
//! the SUM is conserved. Tracked energies are updated by ±ΔE_a exactly.
//!
//! ## Applicability
//!
//! The cancellation holds only for pairwise (2-body) interactions, and the
//! move acts on a single classical configuration, so ICM is enabled only
//! when the model has NO 3/4-body edges and num_slices == 1. The solver
//! gates on this; `icm_pair_move` additionally asserts it.

use crate::core::hubo::FlatHuboModel;
use crate::solver::types::{QuantumField, NUM_REPLICAS};
use rand::Rng;

/// True iff the model is purely pairwise (no 3- or 4-body edges), the
/// precondition for the isoenergetic guarantee.
pub fn is_icm_applicable(model: &FlatHuboModel) -> bool {
    model.edge3_offsets.last().copied().unwrap_or(0) == 0
        && model.edge4_offsets.last().copied().unwrap_or(0) == 0
}

/// Exact energy change of flipping cluster `C` (given by `in_cluster`) in a
/// single lane whose spins are read via `spin(v)`. Product basis:
/// ΔE = Σ_{i∈C} h_i(1−2x_i)
///    + Σ_{i∈C, (i,j), j∉C} w_ij·x_j·(1−2x_i)          (boundary)
///    + Σ_{i<j, both∈C} w_ij·[(1−x_i)(1−x_j) − x_i x_j] (internal, once)
fn cluster_delta<F: Fn(usize) -> i8>(
    model: &FlatHuboModel,
    in_cluster: &[bool],
    cluster: &[usize],
    spin: F,
) -> f64 {
    let mut delta = 0.0;
    for &i in cluster {
        let xi = spin(i) as f64;
        delta += model.linear[i] * (1.0 - 2.0 * xi);
        for idx in model.edge2_offsets[i]..model.edge2_offsets[i + 1] {
            let j = model.edge2_targets[idx];
            let w = model.edge2_weights[idx];
            if in_cluster[j] {
                // Internal edge: count once (i < j).
                if i < j {
                    let xj = spin(j) as f64;
                    delta += w * ((1.0 - xi) * (1.0 - xj) - xi * xj);
                }
            } else {
                // Boundary edge.
                let xj = spin(j) as f64;
                delta += w * xj * (1.0 - 2.0 * xi);
            }
        }
    }
    delta
}

/// Performs one Houdayer/ICM move on the lane pair (a, b) within cell (t, p)
/// of a single-slice field. Returns `true` if a cluster was flipped.
///
/// Preconditions (asserted): num_slices == 1 and the model is pairwise.
/// Tracked energies `field.energies[t + num_temps*p][a|b]` are updated by
/// ±ΔE_a exactly, preserving the incremental-energy invariant.
#[allow(clippy::too_many_arguments)]
pub fn icm_pair_move<R: Rng>(
    field: &mut QuantumField,
    model: &FlatHuboModel,
    t: usize,
    p: usize,
    a: usize,
    b: usize,
    scratch_in_cluster: &mut [bool],
    scratch_visited: &mut [bool],
    scratch_stack: &mut Vec<usize>,
    rng: &mut R,
) -> bool {
    debug_assert_eq!(field.num_slices, 1);
    debug_assert!(is_icm_applicable(model));
    let n = field.num_vars;

    let spin_a = |v: usize| -> i8 { field.spins[field.var_base(v, 0, t, p) + a] };
    let spin_b = |v: usize| -> i8 { field.spins[field.var_base(v, 0, t, p) + b] };

    // Collect disagreement sites (x_i ≠ y_i ⇔ XOR == 1).
    for v in 0..n {
        scratch_visited[v] = spin_a(v) ^ spin_b(v) == 0; // agree ⇒ mark visited
        scratch_in_cluster[v] = false;
    }
    // Pick a random disagreeing (unvisited) seed.
    let disagreeing: Vec<usize> = (0..n).filter(|&v| !scratch_visited[v]).collect();
    if disagreeing.is_empty() {
        return false;
    }
    let seed = disagreeing[rng.gen_range(0..disagreeing.len())];

    // Grow the connected cluster of disagreeing sites via edge2 adjacency.
    let mut cluster = Vec::new();
    scratch_stack.clear();
    scratch_stack.push(seed);
    scratch_visited[seed] = true;
    scratch_in_cluster[seed] = true;
    while let Some(i) = scratch_stack.pop() {
        cluster.push(i);
        for idx in model.edge2_offsets[i]..model.edge2_offsets[i + 1] {
            let j = model.edge2_targets[idx];
            if !scratch_visited[j] {
                scratch_visited[j] = true;
                scratch_in_cluster[j] = true;
                scratch_stack.push(j);
            }
        }
    }

    // Exact ΔE for lane a; lane b changes by −ΔE_a (proven isoenergetic).
    let delta_a = cluster_delta(model, scratch_in_cluster, &cluster, spin_a);

    // Flip the cluster in both lanes.
    for &i in &cluster {
        let base = field.var_base(i, 0, t, p);
        field.spins[base + a] ^= 1;
        field.spins[base + b] ^= 1;
    }

    // Update tracked energies.
    let cell = t + field.num_temps * p;
    field.energies[cell][a] += delta_a;
    field.energies[cell][b] -= delta_a;
    true
}

/// One ICM sweep: pairs lanes (r, r+32) in every (temperature, population)
/// cell and attempts a Houdayer move on each of the 32 pairs. Deterministic
/// given `rng`.
#[allow(clippy::too_many_arguments)]
pub fn icm_sweep<R: Rng>(field: &mut QuantumField, model: &FlatHuboModel, rng: &mut R) {
    let n = field.num_vars;
    let half = NUM_REPLICAS / 2;
    let mut in_cluster = vec![false; n];
    let mut visited = vec![false; n];
    let mut stack = Vec::new();
    for p in 0..field.num_pops {
        for t in 0..field.num_temps {
            for r in 0..half {
                icm_pair_move(
                    field,
                    model,
                    t,
                    p,
                    r,
                    r + half,
                    &mut in_cluster,
                    &mut visited,
                    &mut stack,
                    rng,
                );
            }
        }
    }
}
