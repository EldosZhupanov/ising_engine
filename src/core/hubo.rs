//! Hypergraph Unconstrained Binary Optimization (HUBO) Model
//!
//! Replaces standard QuboModel to support 3-body (Edge3) and 4-body (Edge4)
//! interactions natively.

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

// Preserve legacy QuboModel for backward compatibility across modules
use crate::core::CsrMatrix;

pub struct QuboModel {
    pub num_vars: usize,
    pub linear: Vec<f64>,
    pub quadratic: CsrMatrix,
}

impl QuboModel {
    pub fn calculate_total_energy(&self, state: &[i8]) -> f64 {
        let mut energy = 0.0;
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
