//! Learning rules and memory landscape construction for tensor energy models.

use crate::models::{
    DenseTensor3Hopfield, LowRankCPMemory, PairwiseHopfield, SparseHyperedgeMemory,
};
use crate::types::{Hyperedge3, Hyperedge4, SpinState};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

/// L1: Standard Pairwise Hebbian rule: J_ij = (1/N) * sum_mu xi_i^mu * xi_j^mu.
pub fn train_pairwise_hebbian(patterns: &[SpinState]) -> PairwiseHopfield {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let mut model = PairwiseHopfield::new(n);
    let scale = 1.0 / (n as f64);

    for i in 0..n {
        for j in (i + 1)..n {
            let mut sum_corr = 0.0;
            for pat in patterns {
                sum_corr += (pat.get(i) as f64) * (pat.get(j) as f64);
            }
            let val = sum_corr * scale;
            model.set_j(i, j, val);
        }
    }
    model
}

/// L2: Dense Third-Order Hebbian rule: T_ijk = (1/N^2) * sum_mu xi_i^mu * xi_j^mu * xi_k^mu.
pub fn train_dense_tensor3_hebbian(patterns: &[SpinState]) -> DenseTensor3Hopfield {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let mut model = DenseTensor3Hopfield::new(n);
    let scale = 1.0 / ((n * n) as f64);

    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let mut sum_corr = 0.0;
                for pat in patterns {
                    sum_corr += (pat.get(i) as f64) * (pat.get(j) as f64) * (pat.get(k) as f64);
                }
                model.set_t(i, j, k, sum_corr * scale);
            }
        }
    }
    model
}

/// L3: Sparse 3-body hyperedge memory with exact parameter budget.
/// Selects hyperedges based on correlation magnitude with degree balance.
pub fn train_sparse_hyperedge3_budgeted(
    patterns: &[SpinState],
    budget: usize,
    seed: u64,
) -> SparseHyperedgeMemory {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let mut model = SparseHyperedgeMemory::new(n);
    let scale = 1.0 / ((n * n) as f64);

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut selected_set: HashSet<(u32, u32, u32)> = HashSet::with_capacity(budget * 2);

    // Degree-balanced hyperedge generation: each node gets a quota of hyperedges
    // to ensure no node is left isolated.
    let per_node_quota = ((budget * 3) / n).max(1);
    for i in 0..n {
        let mut count = 0;
        let mut attempts = 0;
        while count < per_node_quota
            && attempts < per_node_quota * 20
            && selected_set.len() < budget
        {
            attempts += 1;
            let j = rng.gen_range(0..n);
            let k = rng.gen_range(0..n);
            if i != j && j != k && i != k {
                let mut idx = [i as u32, j as u32, k as u32];
                idx.sort_unstable();
                let key = (idx[0], idx[1], idx[2]);
                if selected_set.insert(key) {
                    count += 1;
                }
            }
        }
    }

    // Fill remaining budget if needed
    let mut attempts = 0;
    while selected_set.len() < budget && attempts < budget * 50 {
        attempts += 1;
        let i = rng.gen_range(0..n);
        let j = rng.gen_range(0..n);
        let k = rng.gen_range(0..n);
        if i != j && j != k && i != k {
            let mut idx = [i as u32, j as u32, k as u32];
            idx.sort_unstable();
            selected_set.insert((idx[0], idx[1], idx[2]));
        }
    }

    // Compute Hebbian weights for selected hyperedges
    for &(i, j, k) in &selected_set {
        let mut sum_corr = 0.0;
        for pat in patterns {
            sum_corr += (pat.get(i as usize) as f64)
                * (pat.get(j as usize) as f64)
                * (pat.get(k as usize) as f64);
        }
        let weight = sum_corr * scale;
        model.add_hyperedge3(Hyperedge3 { i, j, k, weight });
    }

    model
}

/// L3b: Sparse 4-body hyperedge memory with exact parameter budget.
pub fn train_sparse_hyperedge4_budgeted(
    patterns: &[SpinState],
    budget: usize,
    seed: u64,
) -> SparseHyperedgeMemory {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let mut model = SparseHyperedgeMemory::new(n);
    let scale = 1.0 / ((n * n * n) as f64);

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut selected_set: HashSet<(u32, u32, u32, u32)> = HashSet::with_capacity(budget * 2);

    let per_node_quota = ((budget * 4) / n).max(1);
    for i in 0..n {
        let mut count = 0;
        let mut attempts = 0;
        while count < per_node_quota
            && attempts < per_node_quota * 20
            && selected_set.len() < budget
        {
            attempts += 1;
            let j = rng.gen_range(0..n);
            let k = rng.gen_range(0..n);
            let l = rng.gen_range(0..n);
            if i != j && j != k && k != l && i != k && i != l && j != l {
                let mut idx = [i as u32, j as u32, k as u32, l as u32];
                idx.sort_unstable();
                let key = (idx[0], idx[1], idx[2], idx[3]);
                if selected_set.insert(key) {
                    count += 1;
                }
            }
        }
    }

    let mut attempts = 0;
    while selected_set.len() < budget && attempts < budget * 50 {
        attempts += 1;
        let i = rng.gen_range(0..n);
        let j = rng.gen_range(0..n);
        let k = rng.gen_range(0..n);
        let l = rng.gen_range(0..n);
        if i != j && j != k && k != l && i != k && i != l && j != l {
            let mut idx = [i as u32, j as u32, k as u32, l as u32];
            idx.sort_unstable();
            selected_set.insert((idx[0], idx[1], idx[2], idx[3]));
        }
    }

    for &(i, j, k, l) in &selected_set {
        let mut sum_corr = 0.0;
        for pat in patterns {
            sum_corr += (pat.get(i as usize) as f64)
                * (pat.get(j as usize) as f64)
                * (pat.get(k as usize) as f64)
                * (pat.get(l as usize) as f64);
        }
        let weight = sum_corr * scale;
        model.add_hyperedge4(Hyperedge4 { i, j, k, l, weight });
    }

    model
}

/// L5/L6: Low-Rank Symmetric CP Factorized Tensor Learning.
/// For P patterns, rank R = P, with factor a_{ir} = xi_i^r and lambda_r = 1 / N^2.
pub fn train_low_rank_cp(patterns: &[SpinState]) -> LowRankCPMemory {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let p = patterns.len();
    let mut model = LowRankCPMemory::new(n, p);
    let scale = 1.0 / ((n * n) as f64);

    for r in 0..p {
        model.cp.lambda[r] = scale;
        for i in 0..n {
            model.cp.set_factor(i, r, patterns[r].get(i) as f64);
        }
    }
    model.cp.update_norm_sq();
    model
}

/// Helper: Compute top-R eigenvectors and eigenvalues of empirical covariance matrix C = (1/P) X X^T.
pub fn compute_covariance_eigenvectors(
    patterns: &[SpinState],
    rank: usize,
) -> (Vec<f64>, Vec<f64>) {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let p = patterns.len();
    let r_eff = rank.min(n).min(p);

    // 1. Compute empirical covariance matrix C (n x n)
    let mut cov = vec![0.0; n * n];
    let inv_p = 1.0 / (p as f64);
    for pat in patterns {
        for i in 0..n {
            let si = pat.get(i) as f64;
            let row_offset = i * n;
            for j in 0..n {
                let sj = pat.get(j) as f64;
                cov[row_offset + j] += si * sj * inv_p;
            }
        }
    }

    // 2. Subspace iteration with modified Gram-Schmidt
    let mut v = vec![0.0; n * r_eff];
    // Deterministic pseudo-random initialization
    for i in 0..n {
        for r in 0..r_eff {
            let angle = ((i * 13 + r * 37 + 7) as f64).sin();
            v[i * r_eff + r] = angle;
        }
    }

    let mut y = vec![0.0; n * r_eff];
    for _ in 0..40 {
        // Y = C * V
        y.fill(0.0);
        for i in 0..n {
            let c_row = i * n;
            for j in 0..n {
                let cij = cov[c_row + j];
                for r in 0..r_eff {
                    y[i * r_eff + r] += cij * v[j * r_eff + r];
                }
            }
        }

        // Modified Gram-Schmidt orthogonalization
        for r in 0..r_eff {
            for prev in 0..r {
                let mut dot = 0.0;
                for i in 0..n {
                    dot += y[i * r_eff + r] * y[i * r_eff + prev];
                }
                for i in 0..n {
                    y[i * r_eff + r] -= dot * y[i * r_eff + prev];
                }
            }
            let mut norm_sq = 0.0;
            for i in 0..n {
                let val = y[i * r_eff + r];
                norm_sq += val * val;
            }
            let norm = norm_sq.sqrt().max(1e-12);
            for i in 0..n {
                y[i * r_eff + r] /= norm;
            }
        }
        v.copy_from_slice(&y);
    }

    // 3. Compute eigenvalues: lambda_r = v_r^T * C * v_r
    let mut lambdas = vec![0.0; r_eff];
    for r in 0..r_eff {
        let mut val = 0.0;
        for i in 0..n {
            let mut cv_i = 0.0;
            for j in 0..n {
                cv_i += cov[i * n + j] * v[j * r_eff + r];
            }
            val += v[i * r_eff + r] * cv_i;
        }
        lambdas[r] = val.max(0.0);
    }

    (v, lambdas)
}

/// Train Low-Rank CP-3 Memory from top-R SVD/PCA components of pattern covariance.
pub fn train_low_rank_cp_svd(patterns: &[SpinState], rank: usize) -> LowRankCPMemory {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let r_eff = rank.min(n).min(patterns.len());
    let (basis, lambdas) = compute_covariance_eigenvectors(patterns, r_eff);

    let mut model = LowRankCPMemory::new(n, r_eff);
    let sqrt_n = (n as f64).sqrt();
    let scale = 1.0 / ((n * n) as f64);

    for r in 0..r_eff {
        model.cp.lambda[r] = lambdas[r] * scale;
        for i in 0..n {
            // Scale basis vector to norm sqrt(N)
            model.cp.set_factor(i, r, basis[i * r_eff + r] * sqrt_n);
        }
    }
    model.cp.update_norm_sq();
    model
}

/// Train Low-Rank CP-3 Memory using Anandkumar et al.'s robust symmetric tensor power method.
pub fn train_low_rank_cp_tensor_power(
    patterns: &[SpinState],
    rank: usize,
    num_iters: usize,
    seed: u64,
) -> LowRankCPMemory {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let p = patterns.len();
    let r_eff = rank.min(p);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut factors = vec![0.0; n * r_eff];
    let mut lambdas = vec![0.0; r_eff];
    let inv_p = 1.0 / (p as f64);

    for k in 0..r_eff {
        // Random unit initialization
        let mut v: Vec<f64> = (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect();
        let norm = (v.iter().map(|x| x * x).sum::<f64>()).sqrt().max(1e-10);
        for x in &mut v {
            *x /= norm;
        }

        let mut w = vec![0.0; n];
        for _ in 0..num_iters {
            w.fill(0.0);
            // Empirical 3rd-order moment contraction: T(v, v, .)
            for pat in patterns {
                let mut dot = 0.0;
                for i in 0..n {
                    dot += (pat.get(i) as f64) * v[i];
                }
                let dot_sq = dot * dot * inv_p;
                for i in 0..n {
                    w[i] += dot_sq * (pat.get(i) as f64);
                }
            }

            // Deflate against already extracted factors
            for prev in 0..k {
                let mut dot_prev = 0.0;
                for i in 0..n {
                    dot_prev += factors[i * r_eff + prev] * v[i];
                }
                let term = lambdas[prev] * dot_prev * dot_prev;
                for i in 0..n {
                    w[i] -= term * factors[i * r_eff + prev];
                }
            }

            let norm_w = (w.iter().map(|x| x * x).sum::<f64>()).sqrt().max(1e-12);
            for i in 0..n {
                v[i] = w[i] / norm_w;
            }
        }

        // Compute lambda_k = v^T * w
        let mut lambda_k = 0.0;
        for i in 0..n {
            lambda_k += v[i] * w[i];
        }
        lambdas[k] = lambda_k.max(1e-6);
        for i in 0..n {
            factors[i * r_eff + k] = v[i];
        }
    }

    let mut model = LowRankCPMemory::new(n, r_eff);
    let sqrt_n = (n as f64).sqrt();
    let scale = 1.0 / ((n * n) as f64);
    for k in 0..r_eff {
        model.cp.lambda[k] = lambdas[k] * scale;
        for i in 0..n {
            model.cp.set_factor(i, k, factors[i * r_eff + k] * sqrt_n);
        }
    }
    model.cp.update_norm_sq();
    model
}

/// Pairwise Hopfield memory trained from top-R SVD components: W = sum_r lambda_r v_r v_r^T with W_ii = 0.
pub fn train_pairwise_low_rank_svd(patterns: &[SpinState], rank: usize) -> PairwiseHopfield {
    assert!(!patterns.is_empty());
    let n = patterns[0].len();
    let r_eff = rank.min(n).min(patterns.len());
    let (basis, lambdas) = compute_covariance_eigenvectors(patterns, r_eff);

    let mut model = PairwiseHopfield::new(n);
    let scale = 1.0 / (n as f64);
    for i in 0..n {
        for j in (i + 1)..n {
            let mut w = 0.0;
            for r in 0..r_eff {
                w += lambdas[r] * basis[i * r_eff + r] * basis[j * r_eff + r];
            }
            model.set_j(i, j, w * scale);
        }
    }
    model
}

/// Linear SVD Subspace feedforward projection baseline: s_hat = sign(V V^T s).
#[derive(Debug, Clone)]
pub struct LinearSVDSubspace {
    pub n: usize,
    pub rank: usize,
    pub basis: Vec<f64>,
}

impl LinearSVDSubspace {
    pub fn from_patterns(patterns: &[SpinState], rank: usize) -> Self {
        assert!(!patterns.is_empty());
        let n = patterns[0].len();
        let r_eff = rank.min(n).min(patterns.len());
        let (basis, _) = compute_covariance_eigenvectors(patterns, r_eff);
        Self {
            n,
            rank: r_eff,
            basis,
        }
    }

    pub fn project(&self, s: &SpinState) -> SpinState {
        let mut coords = vec![0.0; self.rank];
        for i in 0..self.n {
            let si = s.get(i) as f64;
            for r in 0..self.rank {
                coords[r] += self.basis[i * self.rank + r] * si;
            }
        }
        let mut spins = Vec::with_capacity(self.n);
        for i in 0..self.n {
            let mut rec = 0.0;
            for r in 0..self.rank {
                rec += self.basis[i * self.rank + r] * coords[r];
            }
            spins.push(if rec >= 0.0 { 1i8 } else { -1i8 });
        }
        SpinState::from_slice(&spins)
    }
}

/// Factor Diagnostics for EXP-TEN-003:
/// 1. Maximum mutual cosine similarity between each factor and the training memories:
///    MaxSim(r) = max_mu |u^r . xi^mu| / (||u^r|| * ||xi^mu||).
pub fn compute_factor_max_similarity(model: &LowRankCPMemory, patterns: &[SpinState]) -> Vec<f64> {
    let rank = model.cp.rank;
    let n = model.n;
    let mut max_sims = vec![0.0; rank];

    for r in 0..rank {
        let mut factor_norm_sq = 0.0;
        for i in 0..n {
            let val = model.cp.get_factor(i, r);
            factor_norm_sq += val * val;
        }
        let factor_norm = factor_norm_sq.sqrt().max(1e-12);
        let pat_norm = (n as f64).sqrt();

        let mut max_cos = 0.0;
        for pat in patterns {
            let mut dot = 0.0;
            for i in 0..n {
                dot += model.cp.get_factor(i, r) * (pat.get(i) as f64);
            }
            let cos_sim = (dot.abs()) / (factor_norm * pat_norm);
            if cos_sim > max_cos {
                max_cos = cos_sim;
            }
        }
        max_sims[r] = max_cos;
    }
    max_sims
}

/// Factor Diagnostics for EXP-TEN-003:
/// 2. Participation Ratio of each factor across memories:
///    PR_r = 1 / sum_mu (p_{r, mu})^2 where p_{r, mu} is normalized squared overlap.
pub fn compute_factor_participation_ratios(
    model: &LowRankCPMemory,
    patterns: &[SpinState],
) -> Vec<f64> {
    let rank = model.cp.rank;
    let n = model.n;
    let p = patterns.len();
    let mut prs = vec![0.0; rank];

    for r in 0..rank {
        let mut overlaps_sq = Vec::with_capacity(p);
        let mut total_overlap = 0.0;
        for pat in patterns {
            let mut dot = 0.0;
            for i in 0..n {
                dot += model.cp.get_factor(i, r) * (pat.get(i) as f64);
            }
            let val = dot * dot;
            overlaps_sq.push(val);
            total_overlap += val;
        }

        if total_overlap < 1e-12 {
            prs[r] = 1.0;
            continue;
        }

        let mut sum_p_sq = 0.0;
        for &ov in &overlaps_sq {
            let p_mu = ov / total_overlap;
            sum_p_sq += p_mu * p_mu;
        }
        prs[r] = 1.0 / sum_p_sq.max(1e-12);
    }
    prs
}

/// Factor Diagnostics for EXP-TEN-003:
/// 3. Normalized Factor Usage Entropy across memories:
///    H = (1 / (P * ln(R))) * sum_mu H(q_mu), where q_{mu, r} is the factor probability on pattern mu.
pub fn compute_factor_entropy(model: &LowRankCPMemory, patterns: &[SpinState]) -> f64 {
    let rank = model.cp.rank;
    let n = model.n;
    let p = patterns.len();
    if rank <= 1 || p == 0 {
        return 0.0;
    }

    let mut total_entropy = 0.0;
    for pat in patterns {
        let mut activations = Vec::with_capacity(rank);
        let mut sum_act = 0.0;
        for r in 0..rank {
            let mut dot = 0.0;
            for i in 0..n {
                dot += model.cp.get_factor(i, r) * (pat.get(i) as f64);
            }
            let a = dot * dot;
            activations.push(a);
            sum_act += a;
        }

        if sum_act < 1e-12 {
            continue;
        }

        let mut h_mu = 0.0;
        for &a in &activations {
            let q = a / sum_act;
            if q > 1e-12 {
                h_mu -= q * q.ln();
            }
        }
        total_entropy += h_mu;
    }

    let max_entropy = (rank as f64).ln();
    (total_entropy / (p as f64)) / max_entropy
}
