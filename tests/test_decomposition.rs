//! Correctness contract for connected-component decomposition.
//!
//! QUBO energy has no terms between different components of the
//! free-variable interaction graph, so E = Σ_c E_c (+ contribution of fixed
//! variables) and per-component optima compose into a global optimum.
//! Verified by exhaustive enumeration.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::presolve::{connected_components, extract_component};
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Builds a QUBO whose interaction graph is a disjoint union of blocks:
/// each block is a clique with mixed-sign weights (dense neighborhoods keep
/// the persistency bounds straddling zero, so blocks survive presolve).
/// No cross-block edges.
fn block_model(
    block_sizes: &[usize],
    rng: &mut ChaCha8Rng,
    weight_range: std::ops::Range<f64>,
) -> QuboModel {
    let n: usize = block_sizes.iter().sum();
    let mut quads: Vec<(usize, usize, f64)> = Vec::new();
    let mut offset = 0;
    for &size in block_sizes {
        for a in 0..size {
            for b in (a + 1)..size {
                quads.push((offset + a, offset + b, rng.gen_range(weight_range.clone())));
            }
        }
        offset += size;
    }
    let mut rows: Vec<Vec<(usize, f64)>> = vec![vec![]; n];
    for &(u, v, w) in &quads {
        rows[u].push((v, w));
        rows[v].push((u, w));
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for r in &mut rows {
        r.sort_by_key(|&(j, _)| j);
        for &(j, w) in r.iter() {
            col_indices.push(j);
            values.push(w);
        }
        row_offsets.push(col_indices.len());
    }
    QuboModel {
        energy_offset: 0.0,
        num_vars: n,
        linear: (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

fn brute_force_optimum(model: &QuboModel, fixed: &[(usize, i8)]) -> f64 {
    let n = model.num_vars;
    assert!(n <= 16);
    let mut best = f64::INFINITY;
    'outer: for bits in 0..(1u32 << n) {
        let state: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        for &(idx, val) in fixed {
            if state[idx] != val {
                continue 'outer;
            }
        }
        best = best.min(model.calculate_total_energy(&state));
    }
    best
}

#[test]
fn components_identified_exactly() {
    let mut rng = ChaCha8Rng::seed_from_u64(5);
    let model = block_model(&[3, 4, 2], &mut rng, -2.0..2.0);
    let active = vec![true; 9];
    let comps = connected_components(&model, &active);
    assert_eq!(
        comps,
        vec![vec![0, 1, 2], vec![3, 4, 5, 6], vec![7, 8]],
        "block structure must be recovered exactly"
    );
}

#[test]
fn clamped_vertex_splits_the_graph() {
    // Chain 0-1-2-3-4. Clamping the middle vertex (2) removes it from the
    // free graph: {0,1} and {3,4} become independent components.
    let chain = [(0usize, 1usize), (1, 2), (2, 3), (3, 4)];
    let mut rows: Vec<Vec<(usize, f64)>> = vec![vec![]; 5];
    for &(u, v) in &chain {
        rows[u].push((v, 1.0));
        rows[v].push((u, 1.0));
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for r in &mut rows {
        r.sort_by_key(|&(j, _)| j);
        for &(j, w) in r.iter() {
            col_indices.push(j);
            values.push(w);
        }
        row_offsets.push(col_indices.len());
    }
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 5,
        linear: vec![0.0; 5],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };
    let mut active = vec![true; 5];
    assert_eq!(connected_components(&model, &active).len(), 1);
    active[2] = false;
    let comps = connected_components(&model, &active);
    assert_eq!(comps, vec![vec![0, 1], vec![3, 4]]);
}

#[test]
fn extraction_folds_fixed_one_neighbors_into_fields() {
    // 0-1 edge w=2.0; 1-2 edge w=-3.0; variable 2 fixed at 1.
    // Sub-model on {0,1}: h0 unchanged, h1 += -3.0, single edge (0,1,2.0).
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 3,
        linear: vec![0.25, 0.5, 0.0],
        quadratic: CsrMatrix {
            values: vec![2.0, 2.0, -3.0, -3.0],
            col_indices: vec![1, 0, 2, 1],
            row_offsets: vec![0, 1, 3, 4],
        },
    };
    let fixed_one = vec![false, false, true];
    let (sub, map) = extract_component(&model, &[0, 1], &fixed_one);
    assert_eq!(map, vec![0, 1]);
    assert_eq!(sub.num_vars, 2);
    assert!((sub.linear[0] - 0.25).abs() < 1e-12);
    assert!((sub.linear[1] - (0.5 - 3.0)).abs() < 1e-12);
    let row0: Vec<(usize, f64)> = sub.quadratic.get_row(0).collect();
    let row1: Vec<(usize, f64)> = sub.quadratic.get_row(1).collect();
    assert_eq!(row0, vec![(1, 2.0)]);
    assert_eq!(row1, vec![(0, 2.0)]);
}

#[test]
fn multi_component_solve_matches_exhaustive_optimum() {
    let mut rng = ChaCha8Rng::seed_from_u64(71);
    let mut decompositions_exercised = 0;
    for trial in 0..10 {
        let model = block_model(&[4, 3, 3], &mut rng, -2.0..2.0);
        // Count how often the decomposition path actually triggers after
        // presolve (guards against a vacuous test).
        let derived = ising_engine::presolve::fix_persistent_variables(&model, &[]);
        let mut free = vec![true; model.num_vars];
        for &(i, _) in &derived {
            free[i] = false;
        }
        if connected_components(&model, &free).len() > 1 {
            decompositions_exercised += 1;
        }
        let e_opt = brute_force_optimum(&model, &[]);
        let solver = UltimateSolver::new(20.0, 0.05, 30, 40, Some(900 + trial));
        let state = solver.solve(&model, &[]);
        let e_found = model.calculate_total_energy(&state);
        assert!(
            (e_found - e_opt).abs() < 1e-9,
            "trial {}: solver returned {} but optimum is {}",
            trial,
            e_found,
            e_opt
        );
    }
    assert!(
        decompositions_exercised >= 5,
        "decomposition path exercised only {}/10 times — test is too weak",
        decompositions_exercised
    );
}

#[test]
fn clamp_bridge_decomposition_matches_clamped_optimum() {
    // Two strongly-coupled pairs {0,1} and {3,4} bridged by variable 2,
    // which is clamped to 1. Bridge couplings (w12 = 0.4, w23 = -0.4) fold
    // into the pair fields; the free graph splits into two components.
    let quads = [
        (0usize, 1usize, -2.0f64),
        (1, 2, 0.4),
        (2, 3, -0.4),
        (3, 4, -2.0),
    ];
    let mut rows: Vec<Vec<(usize, f64)>> = vec![vec![]; 5];
    for &(u, v, w) in &quads {
        rows[u].push((v, w));
        rows[v].push((u, w));
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for r in &mut rows {
        r.sort_by_key(|&(j, _)| j);
        for &(j, w) in r.iter() {
            col_indices.push(j);
            values.push(w);
        }
        row_offsets.push(col_indices.len());
    }
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 5,
        linear: vec![0.5, 0.5, -0.3, 0.5, 0.5],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };
    let clamps = vec![(2usize, 1i8)];

    // The free graph must split into two components once 2 is clamped.
    let derived = ising_engine::presolve::fix_persistent_variables(&model, &clamps);
    let mut free = vec![true; 5];
    free[2] = false;
    for &(i, _) in &derived {
        free[i] = false;
    }
    let n_comps = connected_components(&model, &free).len();
    assert!(
        n_comps == 2 || free.iter().filter(|&&f| f).count() == 0,
        "expected a split free graph, got {} components",
        n_comps
    );

    let e_opt = brute_force_optimum(&model, &clamps);
    let solver = UltimateSolver::new(20.0, 0.05, 30, 40, Some(1300));
    let state = solver.solve(&model, &clamps);
    assert_eq!(state[2], 1, "clamp violated");
    let e_found = model.calculate_total_energy(&state);
    assert!(
        (e_found - e_opt).abs() < 1e-9,
        "solver returned {} but clamped optimum is {}",
        e_found,
        e_opt
    );
}

#[test]
fn multi_component_solve_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(77);
    let model = block_model(&[4, 4], &mut rng, -2.0..2.0);
    let s1 = UltimateSolver::new(20.0, 0.05, 30, 40, Some(4242)).solve(&model, &[]);
    let s2 = UltimateSolver::new(20.0, 0.05, 30, 40, Some(4242)).solve(&model, &[]);
    assert_eq!(s1, s2);
}
