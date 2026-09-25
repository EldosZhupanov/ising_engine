//! Core types for higher-order tensor energy networks.

use serde::{Deserialize, Serialize};

/// Discrete binary spin state: s_i in {-1, +1}.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpinState {
    pub spins: Vec<i8>,
}

impl SpinState {
    /// Create a new spin state of length `n` initialized to `value` (+1 or -1).
    pub fn new(n: usize, value: i8) -> Self {
        assert!(value == 1 || value == -1, "Spins must be +1 or -1");
        Self {
            spins: vec![value; n],
        }
    }

    /// Create from a slice of i8 values (must all be +1 or -1).
    pub fn from_slice(slice: &[i8]) -> Self {
        for &s in slice {
            assert!(s == 1 || s == -1, "Spins must be +1 or -1, found {}", s);
        }
        Self {
            spins: slice.to_vec(),
        }
    }

    /// System dimension N.
    #[inline]
    pub fn len(&self) -> usize {
        self.spins.len()
    }

    /// Is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.spins.is_empty()
    }

    /// Access spin at index i.
    #[inline]
    pub fn get(&self, i: usize) -> i8 {
        self.spins[i]
    }

    /// Flip spin at index i.
    #[inline]
    pub fn flip(&mut self, i: usize) {
        self.spins[i] = -self.spins[i];
    }

    /// Set spin at index i.
    #[inline]
    pub fn set(&mut self, i: usize, val: i8) {
        debug_assert!(val == 1 || val == -1);
        self.spins[i] = val;
    }

    /// Compute Hamming distance to another state (number of differing bits).
    pub fn hamming_distance(&self, other: &Self) -> usize {
        assert_eq!(self.len(), other.len());
        self.spins
            .iter()
            .zip(other.spins.iter())
            .filter(|(&a, &b)| a != b)
            .count()
    }

    /// Inner product (dot product) with another spin state: sum_i s_i * other_i.
    pub fn dot(&self, other: &Self) -> i64 {
        assert_eq!(self.len(), other.len());
        self.spins
            .iter()
            .zip(other.spins.iter())
            .map(|(&a, &b)| (a as i64) * (b as i64))
            .sum()
    }

    /// Compute normalized overlap m in [-1.0, 1.0]: m = (1/N) * sum_i s_i * other_i.
    pub fn overlap(&self, other: &Self) -> f64 {
        assert_eq!(self.len(), other.len());
        let dot = self.dot(other);
        dot as f64 / self.len() as f64
    }

    /// Bit error rate (BER) in [0.0, 1.0]: BER = hamming_distance / N.
    pub fn bit_error_rate(&self, other: &Self) -> f64 {
        self.hamming_distance(other) as f64 / self.len() as f64
    }
}

/// Continuous relaxation state: x_i in [-1.0, 1.0].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousState {
    pub values: Vec<f64>,
}

impl ContinuousState {
    pub fn new(n: usize, val: f64) -> Self {
        Self {
            values: vec![val; n],
        }
    }

    pub fn from_spins(spins: &SpinState) -> Self {
        Self {
            values: spins.spins.iter().map(|&s| s as f64).collect(),
        }
    }

    pub fn to_spins(&self) -> SpinState {
        let spins: Vec<i8> = self
            .values
            .iter()
            .map(|&x| if x >= 0.0 { 1 } else { -1 })
            .collect();
        SpinState { spins }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Sparse 3-body hyperedge: indices (i, j, k) with i < j < k and weight w.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hyperedge3 {
    pub i: u32,
    pub j: u32,
    pub k: u32,
    pub weight: f64,
}

impl Hyperedge3 {
    pub fn new(i: usize, j: usize, k: usize, weight: f64) -> Self {
        let mut idx = [i as u32, j as u32, k as u32];
        idx.sort_unstable();
        debug_assert!(
            idx[0] < idx[1] && idx[1] < idx[2],
            "Indices must be distinct"
        );
        Self {
            i: idx[0],
            j: idx[1],
            k: idx[2],
            weight,
        }
    }
}

/// Sparse 4-body hyperedge: indices (i, j, k, l) with i < j < k < l and weight w.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hyperedge4 {
    pub i: u32,
    pub j: u32,
    pub k: u32,
    pub l: u32,
    pub weight: f64,
}

impl Hyperedge4 {
    pub fn new(i: usize, j: usize, k: usize, l: usize, weight: f64) -> Self {
        let mut idx = [i as u32, j as u32, k as u32, l as u32];
        idx.sort_unstable();
        debug_assert!(
            idx[0] < idx[1] && idx[1] < idx[2] && idx[2] < idx[3],
            "Indices must be distinct"
        );
        Self {
            i: idx[0],
            j: idx[1],
            k: idx[2],
            l: idx[3],
            weight,
        }
    }
}

/// Canonical Polyadic (CP) rank-R factorized tensor for 3-body interactions:
/// T_ijk = sum_{r=1}^R lambda_r * a_{ir} * a_{jr} * a_{kr}.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPTensor3 {
    pub n: usize,
    pub rank: usize,
    /// Factors matrix A of shape [n, rank], stored in row-major order: A[i, r] = factors[i * rank + r].
    pub factors: Vec<f64>,
    /// Tensor weights lambda of length rank.
    pub lambda: Vec<f64>,
    /// Precomputed norms squared: norm_sq[r] = sum_i A[i, r]^2.
    pub norm_sq: Vec<f64>,
}

impl CPTensor3 {
    pub fn new(n: usize, rank: usize) -> Self {
        Self {
            n,
            rank,
            factors: vec![0.0; n * rank],
            lambda: vec![1.0; rank],
            norm_sq: vec![0.0; rank],
        }
    }

    #[inline]
    pub fn get_factor(&self, i: usize, r: usize) -> f64 {
        self.factors[i * self.rank + r]
    }

    #[inline]
    pub fn set_factor(&mut self, i: usize, r: usize, val: f64) {
        self.factors[i * self.rank + r] = val;
    }

    /// Recompute the precomputed column norms squared.
    pub fn update_norm_sq(&mut self) {
        self.norm_sq.fill(0.0);
        for i in 0..self.n {
            for r in 0..self.rank {
                let v = self.factors[i * self.rank + r];
                self.norm_sq[r] += v * v;
            }
        }
    }
}
