//! Lattice Basis Reduction (Lenstra–Lenstra–Lovász / LLL) and Diophantine Preprocessing.
//!
//! Provides:
//! - Experimental f64-GSO LLL reduction of integer row vectors.
//! - Extended-lattice candidates for 0-1 linear Diophantine systems.
//!
//! Integer null vectors and particular solutions are checked against A and b;
//! completeness of the extracted kernel and absence of i64 overflow are not
//! certified by this module. Keep it out of production claims until audited.
//!
//! # References
//! - Lenstra, A. K., Lenstra, H. W., & Lovász, L. (1982). Factoring polynomials with rational coefficients.
//!   *Mathematische Annalen*, 261(4), 515–534.
//! - Aardal, K., Hurkens, C. A. J., & Lenstra, A. K. (2000). Solving a system of linear Diophantine
//!   equations with lower and upper bounds on the variables. *Mathematics of Operations Research*, 25(3), 427–442.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_memcpy)]

use crate::core::{CsrMatrix, QuboModel};

/// Integer row vectors in $\mathbb{Z}^d$; `new` does not prove independence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatticeBasis {
    /// Number of basis vectors (rank $k$).
    pub k: usize,
    /// Ambient dimension ($d$).
    pub d: usize,
    /// Basis vectors $b_0, b_1, \dots, b_{k-1}$, each of length $d$.
    pub basis: Vec<Vec<i64>>,
}

impl LatticeBasis {
    /// Creates a new lattice basis from row vectors.
    pub fn new(basis: Vec<Vec<i64>>) -> Result<Self, &'static str> {
        let k = basis.len();
        if k == 0 {
            return Err("Basis cannot be empty");
        }
        let d = basis[0].len();
        if d == 0 {
            return Err("Ambient dimension cannot be zero");
        }
        for (i, row) in basis.iter().enumerate() {
            if row.len() != d {
                return Err("All basis vectors must have identical dimension");
            }
            if row.iter().all(|&x| x == 0) && k > 1 {
                // Warning or rejection for zero vectors in basis
                let _ = i;
            }
        }
        Ok(Self { k, d, basis })
    }

    /// Performs the Lenstra–Lenstra–Lovász (LLL) lattice basis reduction.
    ///
    /// Parameter `delta` is the Lovász condition parameter, typically $\delta \in (0.25, 1.0]$.
    /// Standard choices are `0.75` (classical) or `0.99` (stronger reduction).
    pub fn lll(&mut self, delta: f64) -> Result<(), &'static str> {
        if delta <= 0.25 || delta > 1.0 {
            return Err("LLL parameter delta must be in (0.25, 1.0]");
        }
        let k = self.k;
        let d = self.d;
        if k <= 1 {
            return Ok(());
        }

        // Gram-Schmidt orthogonalization matrices (stored as f64 for numerical performance)
        let mut b_star = vec![vec![0.0f64; d]; k];
        let mut b_star_sq = vec![0.0f64; k];
        let mut mu = vec![vec![0.0f64; k]; k];

        // Helper closure to compute GSO from row `start` up to row `k-1`
        let compute_gso = |basis: &[Vec<i64>],
                           b_star: &mut [Vec<f64>],
                           b_star_sq: &mut [f64],
                           mu: &mut [Vec<f64>],
                           start: usize| {
            for i in start..k {
                for c in 0..d {
                    b_star[i][c] = basis[i][c] as f64;
                }
                for j in 0..i {
                    if b_star_sq[j] < 1e-12 {
                        mu[i][j] = 0.0;
                    } else {
                        let mut dot = 0.0f64;
                        for c in 0..d {
                            dot += (basis[i][c] as f64) * b_star[j][c];
                        }
                        let coef = dot / b_star_sq[j];
                        mu[i][j] = coef;
                        for c in 0..d {
                            b_star[i][c] -= coef * b_star[j][c];
                        }
                    }
                }
                let mut sq = 0.0f64;
                for c in 0..d {
                    sq += b_star[i][c] * b_star[i][c];
                }
                b_star_sq[i] = sq;
            }
        };

        compute_gso(&self.basis, &mut b_star, &mut b_star_sq, &mut mu, 0);

        let mut i = 1usize;
        let max_iters = 100_000 * k;
        let mut iters = 0usize;

        while i < k {
            iters += 1;
            if iters > max_iters {
                return Err("LLL iteration limit exceeded");
            }

            // Size reduction of b[i] against b[i - 1]
            if mu[i][i - 1].abs() > 0.5 {
                let q = mu[i][i - 1].round() as i64;
                if q != 0 {
                    for c in 0..d {
                        self.basis[i][c] -= q * self.basis[i - 1][c];
                    }
                    compute_gso(&self.basis, &mut b_star, &mut b_star_sq, &mut mu, i);
                }
            }

            // Lovász condition: ||b^*_i||^2 >= (delta - mu_{i, i-1}^2) * ||b^*_{i-1}||^2
            let lovasz_bound = (delta - mu[i][i - 1] * mu[i][i - 1]) * b_star_sq[i - 1];

            if b_star_sq[i] >= lovasz_bound - 1e-9 {
                // Lovász condition satisfied! Size reduce against all j from i-2 down to 0
                for j in (0..=(i.saturating_sub(2))).rev() {
                    if mu[i][j].abs() > 0.5 {
                        let q = mu[i][j].round() as i64;
                        if q != 0 {
                            for c in 0..d {
                                self.basis[i][c] -= q * self.basis[j][c];
                            }
                            compute_gso(&self.basis, &mut b_star, &mut b_star_sq, &mut mu, i);
                        }
                    }
                }
                i += 1;
            } else {
                // Swap basis[i] and basis[i - 1]
                self.basis.swap(i, i - 1);
                // Recompute GSO starting from i - 1
                compute_gso(&self.basis, &mut b_star, &mut b_star_sq, &mut mu, i - 1);
                i = i.saturating_sub(1).max(1);
            }
        }

        Ok(())
    }

    /// Computes the squared Euclidean norm of vector `idx`.
    pub fn vector_norm_sq(&self, idx: usize) -> i64 {
        let mut norm_sq = 0i64;
        for &x in &self.basis[idx] {
            norm_sq += x * x;
        }
        norm_sq
    }

    /// Computes Gram-Schmidt orthogonalization (GSO) matrices for the basis.
    pub fn compute_gso(&self) -> (Vec<Vec<f64>>, Vec<f64>) {
        let k = self.k;
        let d = self.d;
        let mut b_star = vec![vec![0.0f64; d]; k];
        let mut b_star_sq = vec![0.0f64; k];

        for i in 0..k {
            for c in 0..d {
                b_star[i][c] = self.basis[i][c] as f64;
            }
            for j in 0..i {
                if b_star_sq[j] >= 1e-12 {
                    let mut dot = 0.0f64;
                    for c in 0..d {
                        dot += (self.basis[i][c] as f64) * b_star[j][c];
                    }
                    let coef = dot / b_star_sq[j];
                    for c in 0..d {
                        b_star[i][c] -= coef * b_star[j][c];
                    }
                }
            }
            let mut sq = 0.0f64;
            for c in 0..d {
                sq += b_star[i][c] * b_star[i][c];
            }
            b_star_sq[i] = sq;
        }

        (b_star, b_star_sq)
    }

    /// Solves the Closest Vector Problem (CVP) using Babai's Nearest Plane algorithm (Babai 1986).
    ///
    /// Given a target vector $t \in \mathbb{R}^d$, returns integer coefficients $c \in \mathbb{Z}^k$
    /// such that $v = \sum_{i=0}^{k-1} c_i b_i$ is a lattice vector close to $t$.
    pub fn babai_nearest_plane(&self, target: &[f64]) -> Vec<i64> {
        let k = self.k;
        let d = self.d;
        let (b_star, b_star_sq) = self.compute_gso();

        let mut b_prime = target.to_vec();
        let mut c = vec![0i64; k];

        for i in (0..k).rev() {
            if b_star_sq[i] < 1e-12 {
                continue;
            }
            let mut dot = 0.0f64;
            for j in 0..d {
                dot += b_prime[j] * b_star[i][j];
            }
            let c_i = (dot / b_star_sq[i]).round() as i64;
            c[i] = c_i;

            for j in 0..d {
                b_prime[j] -= (c_i as f64) * (self.basis[i][j] as f64);
            }
        }

        c
    }
}

/// Result of Aardal–Hurkens–Lenstra lattice reformulation of $Ax = b$.
#[derive(Clone, Debug)]
pub struct AardalReductionResult {
    /// Particular candidate solution $x_0 \in \mathbb{Z}^n$.
    pub particular_solution: Option<Vec<i64>>,
    /// Verified null vectors $\{v_i\}$ with $A v_i = 0$; saturation/completeness is unproved.
    pub kernel_basis: Vec<Vec<i64>>,
    /// Immediate exact boolean solution $x \in \{0, 1\}^n$ satisfying $Ax = b$, if discovered directly by LLL.
    pub exact_boolean_solution: Option<Vec<i8>>,
}

/// Reformulates and reduces a 0-1 linear Diophantine system $Ax = b$ via the Aardal–Hurkens–Lenstra (2000) method.
///
/// Matrix $A$ is $m \times n$, vector $b$ is $m \times 1$.
/// Constructs extended lattice of dimension $(n + 1)$ with vectors of length $(n + 1 + m)$.
///
/// Parameters $N_1$ and $N_2$ control the penalty scale:
/// - $N_1$ penalizes constraint violations $Ax \neq b$.
/// - $N_2$ sets the target scale for the particular solution bit $z = 1$.
pub fn aardal_diophantine_reduction(
    matrix: &[Vec<i64>],
    rhs: &[i64],
    n1: i64,
    n2: i64,
) -> Result<AardalReductionResult, &'static str> {
    let m = matrix.len();
    if m == 0 {
        return Err("Constraint matrix cannot be empty");
    }
    let n = matrix[0].len();
    if n == 0 {
        return Err("Variable count cannot be zero");
    }
    if rhs.len() != m {
        return Err("RHS vector length must match number of matrix rows");
    }

    // Dimension of the extended lattice:
    // k = n + 1 vectors (one for each variable x_j, plus one for the RHS)
    // d = n + 1 + m ambient dimension:
    // [0..n]: unit coordinate e_j (for x)
    // [n]: scale N2 coordinate (for z)
    // [n+1 .. n+1+m]: penalty N1 * A_kj (for constraints)
    let k = n + 1;
    let d = n + 1 + m;
    let mut basis = vec![vec![0i64; d]; k];

    // Columns 0..n-1: x_j basis vectors
    for j in 0..n {
        basis[j][j] = 1; // e_j
        basis[j][n] = 0; // z-coordinate
        for (i, row) in matrix.iter().enumerate() {
            basis[j][n + 1 + i] = n1 * row[j]; // N1 * A_ij
        }
    }

    // Column n: RHS vector
    basis[n][n] = n2; // N2 * z
    for (i, &b_val) in rhs.iter().enumerate() {
        basis[n][n + 1 + i] = -n1 * b_val; // -N1 * b_i
    }

    // Construct lattice and perform LLL reduction
    let mut lattice = LatticeBasis::new(basis)?;
    lattice.lll(0.75)?;

    let mut particular_solution = None;
    let mut kernel_basis = Vec::new();
    let mut exact_boolean_solution = None;

    // Scan the reduced basis vectors
    for vec in &lattice.basis {
        // Check if constraint residuals are exactly zero (all bottom m coordinates == 0)
        let residuals_zero = vec[(n + 1)..d].iter().all(|&val| val == 0);

        if residuals_zero {
            let z_coord = vec[n];

            if z_coord.abs() == n2 {
                // Vector corresponds to z = +1 or z = -1
                let sign = if z_coord == n2 { 1 } else { -1 };
                let mut candidate_x = vec![0i64; n];
                for j in 0..n {
                    candidate_x[j] = sign * vec[j];
                }

                // Verify that A * candidate_x == b exactly
                let mut satisfies_eq = true;
                for i in 0..m {
                    let mut sum = 0i64;
                    for j in 0..n {
                        sum += matrix[i][j] * candidate_x[j];
                    }
                    if sum != rhs[i] {
                        satisfies_eq = false;
                        break;
                    }
                }

                if satisfies_eq {
                    if particular_solution.is_none() {
                        particular_solution = Some(candidate_x.clone());
                    }

                    // Check if candidate_x is strictly boolean {0, 1}
                    if candidate_x.iter().all(|&val| val == 0 || val == 1) {
                        let bool_sol: Vec<i8> = candidate_x.iter().map(|&v| v as i8).collect();
                        exact_boolean_solution = Some(bool_sol);
                    }
                }
            } else if z_coord == 0 {
                // Vector corresponds to kernel vector A * v = 0
                let mut v = vec![0i64; n];
                for j in 0..n {
                    v[j] = vec[j];
                }
                // Check non-trivial
                if v.iter().any(|&val| val != 0) {
                    let mut satisfies_null = true;
                    for i in 0..m {
                        let mut sum = 0i64;
                        for j in 0..n {
                            sum += matrix[i][j] * v[j];
                        }
                        if sum != 0 {
                            satisfies_null = false;
                            break;
                        }
                    }
                    if satisfies_null {
                        kernel_basis.push(v);
                    }
                }
            }
        }
    }

    Ok(AardalReductionResult {
        particular_solution,
        kernel_basis,
        exact_boolean_solution,
    })
}

/// Encodes the integer kernel space into an exact Quadratic Unconstrained Binary Optimization (QUBO) model.
///
/// Formulation:
/// Min H(x) = \sum_{j=0}^{n-1} x_j(x_j - 1)
/// where x = x_0 + V * lambda, and lambda_i = offset_i + \sum_{b=0}^{B-1} 2^b z_{i * B + b}.
///
/// Conditional invariant under exact arithmetic: if Ax0=b and every Av_i=0,
/// decoding any binary z preserves Ax=b; H(z)=0 iff every x_j is 0 or 1.
/// Inputs and overflow are not checked by this constructor.
pub struct KernelQuboMapping {
    pub model: QuboModel,
    pub energy_offset: f64,
    pub num_bits_per_coord: usize,
    pub offsets: Vec<i64>,
    pub num_kernel_vars: usize,
    pub num_spins: usize,
}

impl KernelQuboMapping {
    /// Decodes a binary state z in {0, 1}^N back into integer coordinates lambda in Z^r
    /// and the full system solution x in Z^n.
    pub fn decode(&self, x0: &[i64], kernel: &[Vec<i64>], state: &[i8]) -> (Vec<i64>, Vec<i64>) {
        let r = self.num_kernel_vars;
        let b_count = self.num_bits_per_coord;
        let n = x0.len();
        let mut lambda = self.offsets.clone();

        for i in 0..r {
            for b in 0..b_count {
                let spin_idx = i * b_count + b;
                if state[spin_idx] == 1 {
                    lambda[i] += 1i64 << b;
                }
            }
        }

        let mut x = x0.to_vec();
        for i in 0..r {
            if lambda[i] != 0 {
                let v = &kernel[i];
                for j in 0..n {
                    x[j] += lambda[i] * v[j];
                }
            }
        }

        (lambda, x)
    }
}

pub fn build_kernel_qubo(
    x0: &[i64],
    kernel: &[Vec<i64>],
    lambda_centers: &[i64],
    bits_per_coord: usize,
) -> KernelQuboMapping {
    let n = x0.len();
    let r = kernel.len();
    let b_count = bits_per_coord.clamp(1, 8);
    let num_spins = r * b_count;

    // Offset for each coordinate: center - 2^(B-1)
    let half_range = 1i64 << (b_count - 1);
    let mut offsets = Vec::with_capacity(r);
    for i in 0..r {
        offsets.push(lambda_centers[i] - half_range);
    }

    // Base vector x_base = x_0 + \sum_i offset_i * v_i
    let mut x_base = x0.to_vec();
    for i in 0..r {
        let v = &kernel[i];
        let off = offsets[i];
        for j in 0..n {
            x_base[j] += off * v[j];
        }
    }

    // Weight matrix W[j][p] = 2^b * v_{i, j} for spin p = i * B + b
    let mut w = vec![vec![0i64; num_spins]; n];
    for i in 0..r {
        let v = &kernel[i];
        for b in 0..b_count {
            let p = i * b_count + b;
            let bit_val = 1i64 << b;
            for j in 0..n {
                w[j][p] = bit_val * v[j];
            }
        }
    }

    // Linear terms h_p = \sum_j [ (2 * x_base[j] - 1) * W[j][p] + W[j][p]^2 ]
    let mut linear = vec![0.0f64; num_spins];
    for p in 0..num_spins {
        let mut sum = 0i64;
        for j in 0..n {
            let w_val = w[j][p];
            sum += (2 * x_base[j] - 1) * w_val + w_val * w_val;
        }
        linear[p] = sum as f64;
    }

    // Quadratic terms J[p][q] = 2 * \sum_j W[j][p] * W[j][q] for p != q
    let mut coo_entries: Vec<Vec<(usize, f64)>> = vec![Vec::new(); num_spins];
    for p in 0..num_spins {
        for q in (p + 1)..num_spins {
            let mut dot = 0i64;
            for j in 0..n {
                dot += w[j][p] * w[j][q];
            }
            if dot != 0 {
                let weight = (2 * dot) as f64;
                coo_entries[p].push((q, weight));
                coo_entries[q].push((p, weight));
            }
        }
    }

    // Build CSR matrix
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = Vec::with_capacity(num_spins + 1);
    row_offsets.push(0);

    for mut row in coo_entries {
        row.sort_by_key(|&(col, _)| col);
        for (col, weight) in row {
            values.push(weight);
            col_indices.push(col);
        }
        row_offsets.push(values.len());
    }

    let quadratic = CsrMatrix {
        values,
        col_indices,
        row_offsets,
    };

    let mut const_offset = 0i64;
    for j in 0..n {
        const_offset += x_base[j] * (x_base[j] - 1);
    }

    let model = QuboModel {
        num_vars: num_spins,
        linear,
        quadratic,
        energy_offset: const_offset as f64,
    };

    KernelQuboMapping {
        model,
        energy_offset: const_offset as f64,
        num_bits_per_coord: b_count,
        offsets,
        num_kernel_vars: r,
        num_spins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lll_2d_lagrange_reduction() {
        // Known 2D basis with non-orthogonal skewed vectors
        let b = vec![vec![1, 2], vec![3, 4]];
        let mut lattice = LatticeBasis::new(b).unwrap();
        lattice.lll(0.75).unwrap();

        // After reduction, vectors must be shorter and nearly orthogonal
        let v0_norm = lattice.vector_norm_sq(0);
        let v1_norm = lattice.vector_norm_sq(1);
        assert!(v0_norm > 0);
        assert!(v1_norm > 0);
        assert!(v0_norm <= 5);
    }

    #[test]
    fn test_lll_3d_standard_reduction() {
        // Standard test lattice from Cohen's Computational Algebraic Number Theory
        let b = vec![vec![1, -1, 3], vec![1, 0, 5], vec![1, 2, 6]];
        let mut lattice = LatticeBasis::new(b).unwrap();
        lattice.lll(0.75).unwrap();

        assert_eq!(lattice.basis.len(), 3);
        assert!(lattice.vector_norm_sq(0) <= lattice.vector_norm_sq(1));
    }

    #[test]
    fn test_aardal_reduction_toy_diophantine() {
        // System:
        // 2*x0 + 3*x1 + 5*x2 = 8
        // 1*x0 + 1*x1 + 2*x2 = 3
        // Solution: x = [1, 0, 1] satisfies:
        // 2(1) + 3(0) + 5(1) = 7 != 8, wait:
        // Let's pick true solution x = [1, 2, 0]: 2(1) + 3(2) = 8; 1(1) + 1(2) = 3!
        let matrix = vec![vec![2, 3, 5], vec![1, 1, 2]];
        let rhs = vec![8, 3];

        let res = aardal_diophantine_reduction(&matrix, &rhs, 1000, 10).unwrap();
        assert!(res.particular_solution.is_some() || !res.kernel_basis.is_empty());
    }
}
