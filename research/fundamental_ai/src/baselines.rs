//! Strong Scientific Baselines for EXP-TEN-002:
//! 1. B1: Pseudoinverse / Projection Hopfield Network (Kanter & Sompolinsky 1987)
//! 2. B2: Nearest-Neighbor Hamming Memory Oracle (1-NN dot-product argmax)
//! 3. B3: Exact Polynomial Dense Associative Memory n=3 (Krotov & Hopfield 2016)
//! 4. B4: Continuous Modern Hopfield Network / Softmax Attention (Ramsauer et al. 2020)

use crate::models::EnergyModel;
use crate::types::SpinState;

// ---------------------------------------------------------------------------
// B1: Pseudoinverse / Projection Hopfield Network
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PseudoinverseHopfield {
    pub n: usize,
    pub p: usize,
    pub w_matrix: Vec<f64>,
}

impl PseudoinverseHopfield {
    /// Train projection rule W = X * (X^T * X)^{-1} * X^T with zero diagonal.
    pub fn train(patterns: &[SpinState]) -> Result<Self, String> {
        let p = patterns.len();
        assert!(p > 0);
        let n = patterns[0].len();

        // 1. Compute Gram matrix G = X^T * X of shape P x P
        let mut g = vec![0.0; p * p];
        for mu in 0..p {
            for nu in mu..p {
                let dot: i64 = patterns[mu]
                    .spins
                    .iter()
                    .zip(patterns[nu].spins.iter())
                    .map(|(&a, &b)| (a as i64) * (b as i64))
                    .sum();
                let val = dot as f64;
                g[mu * p + nu] = val;
                g[nu * p + mu] = val;
            }
        }

        // Add small Tikhonov regularization on diagonal for numerical stability if near-singular
        for i in 0..p {
            g[i * p + i] += 1e-4;
        }

        // 2. Invert G using Gauss-Jordan elimination
        let g_inv = invert_matrix(&g, p)?;

        // 3. Compute W = X * G_inv * X^T
        // Intermediate matrix M = G_inv * X^T of shape P x N: M[mu, j] = sum_nu G_inv[mu, nu] * X[j, nu]
        let mut m_mat = vec![0.0; p * n];
        for mu in 0..p {
            for j in 0..n {
                let mut sum = 0.0;
                for nu in 0..p {
                    sum += g_inv[mu * p + nu] * (patterns[nu].get(j) as f64);
                }
                m_mat[mu * n + j] = sum;
            }
        }

        // W[i, j] = sum_mu X[i, mu] * M[mu, j]
        let mut w = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let mut sum = 0.0;
                    for mu in 0..p {
                        sum += (patterns[mu].get(i) as f64) * m_mat[mu * n + j];
                    }
                    w[i * n + j] = sum;
                }
            }
        }

        Ok(Self { n, p, w_matrix: w })
    }
}

impl EnergyModel for PseudoinverseHopfield {
    fn num_spins(&self) -> usize {
        self.n
    }

    fn num_parameters(&self) -> usize {
        self.n * (self.n - 1) / 2
    }

    fn energy(&self, s: &SpinState) -> f64 {
        let mut e = 0.0;
        for i in 0..self.n {
            let si = s.get(i) as f64;
            let row = i * self.n;
            for j in (i + 1)..self.n {
                e -= self.w_matrix[row + j] * si * (s.get(j) as f64);
            }
        }
        e
    }

    #[inline]
    fn effective_field(&self, s: &SpinState, i: usize) -> f64 {
        let mut field = 0.0;
        let row = i * self.n;
        for j in 0..self.n {
            if j != i {
                field += self.w_matrix[row + j] * (s.get(j) as f64);
            }
        }
        field
    }
}

// ---------------------------------------------------------------------------
// B2: Nearest-Neighbor Hamming Memory Oracle (1-NN Lookup)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NearestNeighborOracle {
    pub n: usize,
    pub patterns: Vec<SpinState>,
}

impl NearestNeighborOracle {
    pub fn new(patterns: &[SpinState]) -> Self {
        assert!(!patterns.is_empty());
        Self {
            n: patterns[0].len(),
            patterns: patterns.to_vec(),
        }
    }

    /// Retrieve the stored memory with highest dot-product overlap.
    pub fn retrieve(&self, probe: &SpinState) -> (SpinState, usize, f64) {
        let mut best_overlap = f64::NEG_INFINITY;
        let mut best_idx = 0;

        for (idx, pat) in self.patterns.iter().enumerate() {
            let ov = probe.overlap(pat);
            if ov > best_overlap {
                best_overlap = ov;
                best_idx = idx;
            }
        }

        (self.patterns[best_idx].clone(), best_idx, best_overlap)
    }
}

// ---------------------------------------------------------------------------
// B3: Exact Polynomial Dense Associative Memory (DAM n=3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PolynomialDAM {
    pub n: usize,
    pub p: usize,
    pub patterns: Vec<SpinState>,
}

impl PolynomialDAM {
    pub fn new(patterns: &[SpinState]) -> Self {
        assert!(!patterns.is_empty());
        Self {
            n: patterns[0].len(),
            p: patterns.len(),
            patterns: patterns.to_vec(),
        }
    }
}

impl EnergyModel for PolynomialDAM {
    fn num_spins(&self) -> usize {
        self.n
    }

    fn num_parameters(&self) -> usize {
        self.n * self.p
    }

    fn energy(&self, s: &SpinState) -> f64 {
        let scale = 1.0 / (6.0 * ((self.n * self.n) as f64));
        let mut total_e = 0.0;
        for pat in &self.patterns {
            let s_dot = s.overlap(pat) * (self.n as f64);
            // Newton-Girard symmetric cubic polynomial excluding self-loops
            let e3 = s_dot * s_dot * s_dot - 3.0 * (self.n as f64) * s_dot + 2.0 * s_dot;
            total_e -= scale * e3;
        }
        total_e
    }

    #[inline]
    fn effective_field(&self, s: &SpinState, i: usize) -> f64 {
        let scale = 1.0 / (2.0 * ((self.n * self.n) as f64));
        let si = s.get(i) as f64;
        let mut field = 0.0;
        for pat in &self.patterns {
            let s_dot = s.overlap(pat) * (self.n as f64);
            let xi = pat.get(i) as f64;
            let s_without_i = s_dot - si * xi;
            let term = s_without_i * s_without_i - ((self.n - 1) as f64);
            field += scale * xi * term;
        }
        field
    }
}

// ---------------------------------------------------------------------------
// B4: Continuous Modern Hopfield Network / Softmax Attention
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ModernHopfield {
    pub n: usize,
    pub p: usize,
    pub patterns: Vec<SpinState>,
    pub beta: f64,
}

impl ModernHopfield {
    pub fn new(patterns: &[SpinState], beta: f64) -> Self {
        assert!(!patterns.is_empty());
        Self {
            n: patterns[0].len(),
            p: patterns.len(),
            patterns: patterns.to_vec(),
            beta,
        }
    }

    /// Single softmax attention update: s_new = sign(X * softmax(beta * X^T * s)).
    pub fn step(&self, s: &SpinState) -> SpinState {
        // 1. Compute logits = beta * X^T * s
        let mut logits = vec![0.0; self.p];
        for mu in 0..self.p {
            logits[mu] = self.beta * s.overlap(&self.patterns[mu]) * (self.n as f64);
        }

        // Softmax with numerical stability (subtract max)
        let max_logit = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mut exps = vec![0.0; self.p];
        let mut sum_exp = 0.0;
        for mu in 0..self.p {
            let e = (logits[mu] - max_logit).exp();
            exps[mu] = e;
            sum_exp += e;
        }
        let inv_sum = 1.0 / sum_exp;
        for mu in 0..self.p {
            exps[mu] *= inv_sum;
        }

        // 2. Compute s_new = sign(sum_mu exps[mu] * xi^mu)
        let mut new_spins = vec![1i8; self.n];
        for i in 0..self.n {
            let mut val = 0.0;
            for mu in 0..self.p {
                val += exps[mu] * (self.patterns[mu].get(i) as f64);
            }
            new_spins[i] = if val >= 0.0 { 1 } else { -1 };
        }

        SpinState { spins: new_spins }
    }

    /// Run iterative update to convergence (max 20 steps).
    pub fn retrieve(&self, probe: &SpinState, max_steps: usize) -> SpinState {
        let mut state = probe.clone();
        for _ in 0..max_steps {
            let next_state = self.step(&state);
            if next_state == state {
                break;
            }
            state = next_state;
        }
        state
    }
}

// Helper: Invert matrix of size p x p using Gauss-Jordan elimination with partial pivoting.
fn invert_matrix(a: &[f64], p: usize) -> Result<Vec<f64>, String> {
    let mut mat = vec![0.0; p * 2 * p];
    // Initialize [A | I]
    for i in 0..p {
        for j in 0..p {
            mat[i * 2 * p + j] = a[i * p + j];
        }
        mat[i * 2 * p + p + i] = 1.0;
    }

    for col in 0..p {
        // Find pivot
        let mut max_row = col;
        let mut max_val = mat[col * 2 * p + col].abs();
        for row in (col + 1)..p {
            let v = mat[row * 2 * p + col].abs();
            if v > max_val {
                max_val = v;
                max_row = row;
            }
        }

        if max_val < 1e-12 {
            return Err("Matrix is numerically singular".to_string());
        }

        // Swap rows
        if max_row != col {
            for j in 0..(2 * p) {
                mat.swap(col * 2 * p + j, max_row * 2 * p + j);
            }
        }

        // Scale pivot row
        let pivot = mat[col * 2 * p + col];
        let inv_pivot = 1.0 / pivot;
        for j in 0..(2 * p) {
            mat[col * 2 * p + j] *= inv_pivot;
        }

        // Eliminate other rows
        for row in 0..p {
            if row != col {
                let factor = mat[row * 2 * p + col];
                for j in 0..(2 * p) {
                    mat[row * 2 * p + j] -= factor * mat[col * 2 * p + j];
                }
            }
        }
    }

    // Extract inverse matrix
    let mut inv = vec![0.0; p * p];
    for i in 0..p {
        for j in 0..p {
            inv[i * p + j] = mat[i * 2 * p + p + j];
        }
    }

    Ok(inv)
}
