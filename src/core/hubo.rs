//! Hypergraph Unconstrained Binary Optimization (HUBO) Model
//!
//! Contains both the original HuboModel (`Vec<Vec<Edge>>`) for construction,
//! and FlatHuboModel (CSR-style contiguous arrays) for SIMD-optimized traversal.

// Preserve legacy QuboModel for backward compatibility across modules
use crate::core::CsrMatrix;

// ============================================================================
// Original Edge Types (used during construction)
// ============================================================================

#[derive(Clone)]
pub struct Edge2 {
    pub j: usize,
    pub weight: f64,
}

#[derive(Clone)]
pub struct Edge3 {
    pub j: usize,
    pub k: usize,
    pub weight: f64,
}

#[derive(Clone)]
pub struct Edge4 {
    pub j: usize,
    pub k: usize,
    pub l: usize,
    pub weight: f64,
}

/// Original HuboModel with `Vec<Vec<Edge>>` storage.
/// Used during construction (LogicBuilder::build_hubo).
/// Convert to FlatHuboModel before running the solver.
#[derive(Clone)]
pub struct HuboModel {
    pub num_vars: usize,
    pub linear: Vec<f64>,
    pub edges2: Vec<Vec<Edge2>>,
    pub edges3: Vec<Vec<Edge3>>,
    pub edges4: Vec<Vec<Edge4>>,
}

impl HuboModel {
    pub fn new(num_vars: usize) -> Self {
        Self {
            num_vars,
            linear: vec![0.0; num_vars],
            edges2: vec![Vec::new(); num_vars],
            edges3: vec![Vec::new(); num_vars],
            edges4: vec![Vec::new(); num_vars],
        }
    }
}

// ============================================================================
// CSR-Style Flat Edge Storage for SIMD-Optimized Traversal
// ============================================================================

/// Flattened HUBO model with CSR-style contiguous edge arrays.
///
/// Eliminates N separate heap allocations from `Vec<Vec<Edge>>`.
/// All edges for all variables are stored in single contiguous Vecs.
/// Access pattern: `for idx in offsets[v]..offsets[v+1]` — cache-line friendly,
/// hardware prefetcher compatible.
pub struct FlatHuboModel {
    pub num_vars: usize,
    pub linear: Vec<f64>,

    // 2-body edges (CSR layout)
    pub edge2_targets: Vec<usize>,
    pub edge2_weights: Vec<f64>,
    pub edge2_offsets: Vec<usize>, // length = num_vars + 1

    // 3-body edges (CSR layout)
    pub edge3_j: Vec<usize>,
    pub edge3_k: Vec<usize>,
    pub edge3_weights: Vec<f64>,
    pub edge3_offsets: Vec<usize>, // length = num_vars + 1

    // 4-body edges (CSR layout)
    pub edge4_j: Vec<usize>,
    pub edge4_k: Vec<usize>,
    pub edge4_l: Vec<usize>,
    pub edge4_weights: Vec<f64>,
    pub edge4_offsets: Vec<usize>, // length = num_vars + 1
}

impl FlatHuboModel {
    /// Convert from HuboModel to FlatHuboModel.
    /// This flattens `Vec<Vec<Edge>>` into contiguous CSR arrays.
    pub fn from_hubo(model: &HuboModel) -> Self {
        let n = model.num_vars;

        // Flatten edge2
        let mut edge2_targets = Vec::new();
        let mut edge2_weights = Vec::new();
        let mut edge2_offsets = Vec::with_capacity(n + 1);
        edge2_offsets.push(0);
        for v in 0..n {
            for edge in &model.edges2[v] {
                edge2_targets.push(edge.j);
                edge2_weights.push(edge.weight);
            }
            edge2_offsets.push(edge2_targets.len());
        }

        // Flatten edge3
        let mut edge3_j = Vec::new();
        let mut edge3_k = Vec::new();
        let mut edge3_weights = Vec::new();
        let mut edge3_offsets = Vec::with_capacity(n + 1);
        edge3_offsets.push(0);
        for v in 0..n {
            for edge in &model.edges3[v] {
                edge3_j.push(edge.j);
                edge3_k.push(edge.k);
                edge3_weights.push(edge.weight);
            }
            edge3_offsets.push(edge3_j.len());
        }

        // Flatten edge4
        let mut edge4_j = Vec::new();
        let mut edge4_k = Vec::new();
        let mut edge4_l = Vec::new();
        let mut edge4_weights = Vec::new();
        let mut edge4_offsets = Vec::with_capacity(n + 1);
        edge4_offsets.push(0);
        for v in 0..n {
            for edge in &model.edges4[v] {
                edge4_j.push(edge.j);
                edge4_k.push(edge.k);
                edge4_l.push(edge.l);
                edge4_weights.push(edge.weight);
            }
            edge4_offsets.push(edge4_j.len());
        }

        Self {
            num_vars: n,
            linear: model.linear.clone(),
            edge2_targets,
            edge2_weights,
            edge2_offsets,
            edge3_j,
            edge3_k,
            edge3_weights,
            edge3_offsets,
            edge4_j,
            edge4_k,
            edge4_l,
            edge4_weights,
            edge4_offsets,
        }
    }
}

// ============================================================================
// Legacy QuboModel (unchanged)
// ============================================================================

pub struct QuboModel {
    pub num_vars: usize,
    pub linear: Vec<f64>,
    pub quadratic: CsrMatrix,
    /// Constant term of the objective. Irrelevant for argmin, required for
    /// the "E = 0 iff satisfied" contract of logic-gate penalties (the NOT
    /// penalty 2xy − x − y + 1 has a +1 constant; Boros & Hammer 2002).
    pub energy_offset: f64,
}

impl QuboModel {
    pub fn calculate_total_energy(&self, state: &[i8]) -> f64 {
        let mut energy = self.energy_offset;
        for i in 0..self.num_vars {
            if state[i] == 1 {
                energy += self.linear[i];
                for (j, weight) in self.quadratic.get_row(i) {
                    if state[j] == 1 {
                        energy += weight * 0.5;
                    }
                }
            }
        }
        energy
    }
}
