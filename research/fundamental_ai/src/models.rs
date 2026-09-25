//! Energy-based associative memory models: M0 (Pairwise), M1/M2 (Dense Higher-Order),
//! M3 (Sparse Hyperedge), and M_CP (Low-Rank Factorized CP).

use crate::types::{CPTensor3, Hyperedge3, Hyperedge4, SpinState};

/// Common trait for energy models.
pub trait EnergyModel: Send + Sync {
    /// Dimension N.
    fn num_spins(&self) -> usize;

    /// Number of trainable or stored parameters.
    fn num_parameters(&self) -> usize;

    /// Total system energy for a given spin state: E(s).
    fn energy(&self, s: &SpinState) -> f64;

    /// Local effective field at spin index i: h_i^eff(s).
    fn effective_field(&self, s: &SpinState, i: usize) -> f64;

    /// Exact energy change if spin i were flipped: Delta E_i = 2 * s_i * h_i^eff(s).
    #[inline]
    fn flip_delta(&self, s: &SpinState, i: usize) -> f64 {
        2.0 * (s.get(i) as f64) * self.effective_field(s, i)
    }

    /// Compute all N effective fields into the provided buffer.
    fn compute_all_fields(&self, s: &SpinState, fields: &mut [f64]) {
        assert_eq!(fields.len(), self.num_spins());
        for i in 0..self.num_spins() {
            fields[i] = self.effective_field(s, i);
        }
    }
}

// ---------------------------------------------------------------------------
// M0: Pairwise Hopfield / Ising Memory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PairwiseHopfield {
    pub n: usize,
    /// Linear biases h_i.
    pub h: Vec<f64>,
    /// Dense symmetric coupling matrix J_ij with J_ii = 0.
    /// Stored as flat N x N matrix.
    pub j_matrix: Vec<f64>,
}

impl PairwiseHopfield {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            h: vec![0.0; n],
            j_matrix: vec![0.0; n * n],
        }
    }

    #[inline]
    pub fn get_j(&self, i: usize, j: usize) -> f64 {
        self.j_matrix[i * self.n + j]
    }

    #[inline]
    pub fn set_j(&mut self, i: usize, j: usize, val: f64) {
        self.j_matrix[i * self.n + j] = val;
        self.j_matrix[j * self.n + i] = val;
    }
}

impl EnergyModel for PairwiseHopfield {
    fn num_spins(&self) -> usize {
        self.n
    }

    fn num_parameters(&self) -> usize {
        self.n * (self.n - 1) / 2
    }

    fn energy(&self, s: &SpinState) -> f64 {
        assert_eq!(s.len(), self.n);
        let mut e = 0.0;
        // - sum_i h_i * s_i
        for i in 0..self.n {
            e -= self.h[i] * (s.get(i) as f64);
        }
        // - sum_{i < j} J_ij * s_i * s_j
        for i in 0..self.n {
            let si = s.get(i) as f64;
            let row_offset = i * self.n;
            for j in (i + 1)..self.n {
                e -= self.j_matrix[row_offset + j] * si * (s.get(j) as f64);
            }
        }
        e
    }

    #[inline]
    fn effective_field(&self, s: &SpinState, i: usize) -> f64 {
        assert_eq!(s.len(), self.n);
        let mut field = self.h[i];
        let row_offset = i * self.n;
        for j in 0..self.n {
            if j != i {
                field += self.j_matrix[row_offset + j] * (s.get(j) as f64);
            }
        }
        field
    }

    fn compute_all_fields(&self, s: &SpinState, fields: &mut [f64]) {
        assert_eq!(fields.len(), self.n);
        assert_eq!(s.len(), self.n);
        for i in 0..self.n {
            let mut field = self.h[i];
            let row_offset = i * self.n;
            for j in 0..self.n {
                if j != i {
                    field += self.j_matrix[row_offset + j] * (s.get(j) as f64);
                }
            }
            fields[i] = field;
        }
    }
}

// ---------------------------------------------------------------------------
// M1: Dense Third-Order Tensor Memory (for small N verification)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DenseTensor3Hopfield {
    pub n: usize,
    pub pairwise: PairwiseHopfield,
    /// Flat 3D tensor T[i, j, k] with i < j < k.
    pub t_tensor: Vec<f64>,
}

impl DenseTensor3Hopfield {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            pairwise: PairwiseHopfield::new(n),
            t_tensor: vec![0.0; n * n * n],
        }
    }

    #[inline]
    pub fn get_t(&self, i: usize, j: usize, k: usize) -> f64 {
        let mut idx = [i, j, k];
        idx.sort_unstable();
        self.t_tensor[idx[0] * self.n * self.n + idx[1] * self.n + idx[2]]
    }

    #[inline]
    pub fn set_t(&mut self, i: usize, j: usize, k: usize, val: f64) {
        let mut idx = [i, j, k];
        idx.sort_unstable();
        self.t_tensor[idx[0] * self.n * self.n + idx[1] * self.n + idx[2]] = val;
    }
}

impl EnergyModel for DenseTensor3Hopfield {
    fn num_spins(&self) -> usize {
        self.n
    }

    fn num_parameters(&self) -> usize {
        self.pairwise.num_parameters() + self.n * (self.n - 1) * (self.n - 2) / 6
    }

    fn energy(&self, s: &SpinState) -> f64 {
        let mut e = self.pairwise.energy(s);
        for i in 0..self.n {
            let si = s.get(i) as f64;
            for j in (i + 1)..self.n {
                let sj = s.get(j) as f64;
                for k in (j + 1)..self.n {
                    let sk = s.get(k) as f64;
                    let t = self.t_tensor[i * self.n * self.n + j * self.n + k];
                    e -= t * si * sj * sk;
                }
            }
        }
        e
    }

    fn effective_field(&self, s: &SpinState, m: usize) -> f64 {
        let mut field = self.pairwise.effective_field(s, m);
        for j in 0..self.n {
            if j == m {
                continue;
            }
            let sj = s.get(j) as f64;
            for k in (j + 1)..self.n {
                if k == m {
                    continue;
                }
                let sk = s.get(k) as f64;
                field += self.get_t(m, j, k) * sj * sk;
            }
        }
        field
    }
}

// ---------------------------------------------------------------------------
// M3: Sparse Higher-Order Hyperedge Memory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SparseHyperedgeMemory {
    pub n: usize,
    pub h: Vec<f64>,
    /// Optional sparse pairwise edges: (j, w) for node i.
    pub adj_pairs: Vec<Vec<(u32, f64)>>,
    /// Sparse 3-body hyperedges: (j, k, w) incident to node i.
    pub adj3: Vec<Vec<(u32, u32, f64)>>,
    /// Sparse 4-body hyperedges: (j, k, l, w) incident to node i.
    pub adj4: Vec<Vec<(u32, u32, u32, f64)>>,
    /// Stored hyperedges list for energy evaluation.
    pub edges3: Vec<Hyperedge3>,
    pub edges4: Vec<Hyperedge4>,
    pub num_pairs: usize,
}

impl SparseHyperedgeMemory {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            h: vec![0.0; n],
            adj_pairs: vec![Vec::new(); n],
            adj3: vec![Vec::new(); n],
            adj4: vec![Vec::new(); n],
            edges3: Vec::new(),
            edges4: Vec::new(),
            num_pairs: 0,
        }
    }

    /// Add a 3-body hyperedge.
    pub fn add_hyperedge3(&mut self, edge: Hyperedge3) {
        let (i, j, k, w) = (
            edge.i as usize,
            edge.j as usize,
            edge.k as usize,
            edge.weight,
        );
        self.adj3[i].push((edge.j, edge.k, w));
        self.adj3[j].push((edge.i, edge.k, w));
        self.adj3[k].push((edge.i, edge.j, w));
        self.edges3.push(edge);
    }

    /// Add a 4-body hyperedge.
    pub fn add_hyperedge4(&mut self, edge: Hyperedge4) {
        let (i, j, k, l, w) = (
            edge.i as usize,
            edge.j as usize,
            edge.k as usize,
            edge.l as usize,
            edge.weight,
        );
        self.adj4[i].push((edge.j, edge.k, edge.l, w));
        self.adj4[j].push((edge.i, edge.k, edge.l, w));
        self.adj4[k].push((edge.i, edge.j, edge.l, w));
        self.adj4[l].push((edge.i, edge.j, edge.k, w));
        self.edges4.push(edge);
    }
}

impl EnergyModel for SparseHyperedgeMemory {
    fn num_spins(&self) -> usize {
        self.n
    }

    fn num_parameters(&self) -> usize {
        self.num_pairs + self.edges3.len() + self.edges4.len()
    }

    fn energy(&self, s: &SpinState) -> f64 {
        assert_eq!(s.len(), self.n);
        let mut e = 0.0;
        // - sum_i h_i * s_i
        for i in 0..self.n {
            e -= self.h[i] * (s.get(i) as f64);
        }
        // - sum_e w_e * s_i * s_j * s_k
        for edge in &self.edges3 {
            let si = s.get(edge.i as usize) as f64;
            let sj = s.get(edge.j as usize) as f64;
            let sk = s.get(edge.k as usize) as f64;
            e -= edge.weight * si * sj * sk;
        }
        // - sum_e w_e * s_i * s_j * s_k * s_l
        for edge in &self.edges4 {
            let si = s.get(edge.i as usize) as f64;
            let sj = s.get(edge.j as usize) as f64;
            let sk = s.get(edge.k as usize) as f64;
            let sl = s.get(edge.l as usize) as f64;
            e -= edge.weight * si * sj * sk * sl;
        }
        e
    }

    #[inline]
    fn effective_field(&self, s: &SpinState, m: usize) -> f64 {
        assert_eq!(s.len(), self.n);
        let mut field = self.h[m];
        // 3-body hyperedges incident to m
        for &(j, k, w) in &self.adj3[m] {
            let sj = s.get(j as usize) as f64;
            let sk = s.get(k as usize) as f64;
            field += w * sj * sk;
        }
        // 4-body hyperedges incident to m
        for &(j, k, l, w) in &self.adj4[m] {
            let sj = s.get(j as usize) as f64;
            let sk = s.get(k as usize) as f64;
            let sl = s.get(l as usize) as f64;
            field += w * sj * sk * sl;
        }
        field
    }

    fn compute_all_fields(&self, s: &SpinState, fields: &mut [f64]) {
        for i in 0..self.n {
            fields[i] = self.effective_field(s, i);
        }
    }
}

// ---------------------------------------------------------------------------
// M_CP: Low-Rank Canonical Polyadic (CP) Factorized Tensor Memory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LowRankCPMemory {
    pub n: usize,
    pub cp: CPTensor3,
}

impl LowRankCPMemory {
    pub fn new(n: usize, rank: usize) -> Self {
        Self {
            n,
            cp: CPTensor3::new(n, rank),
        }
    }

    /// Compute pattern overlap vector S of length rank: S_r = sum_{j=1}^N A[j, r] * s_j.
    pub fn compute_overlap_vector(&self, s: &SpinState, s_vec: &mut [f64]) {
        assert_eq!(s_vec.len(), self.cp.rank);
        s_vec.fill(0.0);
        let rank = self.cp.rank;
        for j in 0..self.n {
            let sj = s.get(j) as f64;
            let row_offset = j * rank;
            for r in 0..rank {
                s_vec[r] += self.cp.factors[row_offset + r] * sj;
            }
        }
    }
}

impl EnergyModel for LowRankCPMemory {
    fn num_spins(&self) -> usize {
        self.n
    }

    fn num_parameters(&self) -> usize {
        self.n * self.cp.rank
    }

    fn energy(&self, s: &SpinState) -> f64 {
        assert_eq!(s.len(), self.n);
        let rank = self.cp.rank;
        let mut s_vec = vec![0.0; rank];
        self.compute_overlap_vector(s, &mut s_vec);

        // Compute p3_r = sum_i A[i, r]^3 * s_i for each r
        let mut p3 = vec![0.0; rank];
        for i in 0..self.n {
            let si = s.get(i) as f64;
            let row_offset = i * rank;
            for r in 0..rank {
                let a = self.cp.factors[row_offset + r];
                p3[r] += a * a * a * si;
            }
        }

        // Exact Newton-Girard polynomial evaluation:
        // sum_{i < j < k} z_i z_j z_k = (1/6) * (S_r^3 - 3 * S_r * norm_sq_r + 2 * p3_r)
        let mut total_e = 0.0;
        for r in 0..rank {
            let sr = s_vec[r];
            let n_sq = self.cp.norm_sq[r];
            let e3_r = (sr * sr * sr - 3.0 * sr * n_sq + 2.0 * p3[r]) / 6.0;
            total_e -= self.cp.lambda[r] * e3_r;
        }
        total_e
    }

    #[inline]
    fn effective_field(&self, s: &SpinState, m: usize) -> f64 {
        assert_eq!(s.len(), self.n);
        let rank = self.cp.rank;
        let sm = s.get(m) as f64;
        let mut s_vec = vec![0.0; rank];
        self.compute_overlap_vector(s, &mut s_vec);

        let mut field = 0.0;
        let m_offset = m * rank;
        for r in 0..rank {
            let amr = self.cp.factors[m_offset + r];
            let s_without_m = s_vec[r] - amr * sm;
            let norm_without_m = self.cp.norm_sq[r] - amr * amr;
            let term = 0.5 * (s_without_m * s_without_m - norm_without_m);
            field += self.cp.lambda[r] * amr * term;
        }
        field
    }

    fn compute_all_fields(&self, s: &SpinState, fields: &mut [f64]) {
        assert_eq!(fields.len(), self.n);
        let rank = self.cp.rank;
        let mut s_vec = vec![0.0; rank];
        self.compute_overlap_vector(s, &mut s_vec);

        fields.fill(0.0);
        for m in 0..self.n {
            let sm = s.get(m) as f64;
            let m_offset = m * rank;
            let mut field_m = 0.0;
            for r in 0..rank {
                let amr = self.cp.factors[m_offset + r];
                let s_without_m = s_vec[r] - amr * sm;
                let norm_without_m = self.cp.norm_sq[r] - amr * amr;
                let term = 0.5 * (s_without_m * s_without_m - norm_without_m);
                field_m += self.cp.lambda[r] * amr * term;
            }
            fields[m] = field_m;
        }
    }
}

// ---------------------------------------------------------------------------
// Parameterized Latent Energy Laws (EXP-TEN-005 Part 1)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnergyLaw {
    /// p = 3: Odd degree (erases sign because (u.s)^2 >= 0)
    Cubic,
    /// p = 4: Even degree (preserves sign: (u.s)^3 is odd function)
    Quartic,
    /// p = 6: High even degree (preserves sign: (u.s)^5 is odd function)
    Sextic,
    /// Non-polynomial smooth saturation: ln cosh(beta * x)
    LogCosh { beta: f64 },
    /// Non-polynomial rational saturation: x^2 / (1 + gamma * x^2)
    Rational { gamma: f64 },
}

impl EnergyLaw {
    /// Scalar potential F(x) where E(s) = - sum_r lambda_r F(u^r . s)
    #[inline]
    pub fn potential(&self, x: f64) -> f64 {
        match self {
            EnergyLaw::Cubic => (x * x * x) / 3.0,
            EnergyLaw::Quartic => (x * x * x * x) / 4.0,
            EnergyLaw::Sextic => (x * x * x * x * x * x) / 6.0,
            EnergyLaw::LogCosh { beta } => {
                let bx = (beta * x).abs();
                if bx > 20.0 {
                    (bx - std::f64::consts::LN_2) / beta
                } else {
                    (beta * x).cosh().ln() / beta
                }
            }
            EnergyLaw::Rational { gamma } => {
                let x2 = x * x;
                x2 / (1.0 + gamma * x2)
            }
        }
    }

    /// Derivative f(x) = F'(x) where effective field h_i = sum_r lambda_r u_i^r f(u^r . s)
    #[inline]
    pub fn derivative(&self, x: f64) -> f64 {
        match self {
            EnergyLaw::Cubic => x * x,
            EnergyLaw::Quartic => x * x * x,
            EnergyLaw::Sextic => x * x * x * x * x,
            EnergyLaw::LogCosh { beta } => (beta * x).tanh(),
            EnergyLaw::Rational { gamma } => {
                let denom = 1.0 + gamma * x * x;
                (2.0 * x) / (denom * denom)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LatentEnergyModel {
    pub n: usize,
    pub rank: usize,
    /// Basis vectors stored as row-major N x rank: basis[i * rank + r] = u_i^r
    pub basis: Vec<f64>,
    pub lambdas: Vec<f64>,
    pub law: EnergyLaw,
}

impl LatentEnergyModel {
    pub fn new(n: usize, rank: usize, basis: Vec<f64>, lambdas: Vec<f64>, law: EnergyLaw) -> Self {
        assert_eq!(basis.len(), n * rank);
        assert_eq!(lambdas.len(), rank);
        Self {
            n,
            rank,
            basis,
            lambdas,
            law,
        }
    }

    /// Compute overlap vector: a_r = sum_{i=1}^N basis[i * rank + r] * s_i
    pub fn compute_overlaps(&self, s: &SpinState, a: &mut [f64]) {
        assert_eq!(a.len(), self.rank);
        a.fill(0.0);
        for i in 0..self.n {
            let si = s.get(i) as f64;
            let offset = i * self.rank;
            for r in 0..self.rank {
                a[r] += self.basis[offset + r] * si;
            }
        }
    }

    /// Local field at spin i: h_i = sum_r lambda_r u_i^r f(u^r . s)
    pub fn effective_field_with_overlaps(&self, i: usize, a: &[f64]) -> f64 {
        let mut field = 0.0;
        let offset = i * self.rank;
        for r in 0..self.rank {
            let u_ir = self.basis[offset + r];
            field += self.lambdas[r] * u_ir * self.law.derivative(a[r]);
        }
        field
    }

    /// Single greedy descent relaxation sweep
    pub fn relax_greedy(&self, s: &SpinState, max_flips: usize) -> (SpinState, usize) {
        let mut current = s.clone();
        let mut a = vec![0.0; self.rank];
        self.compute_overlaps(&current, &mut a);

        let mut flips = 0;
        for _ in 0..max_flips {
            let mut best_idx = None;
            let mut best_delta = -1e-9;

            for i in 0..self.n {
                let si = current.get(i) as f64;
                let offset = i * self.rank;
                let mut delta_e = 0.0;
                for r in 0..self.rank {
                    let u_ir = self.basis[offset + r];
                    let delta_a = -2.0 * si * u_ir;
                    let diff_pot = self.law.potential(a[r] + delta_a) - self.law.potential(a[r]);
                    delta_e -= self.lambdas[r] * diff_pot;
                }
                if delta_e < best_delta {
                    best_delta = delta_e;
                    best_idx = Some(i);
                }
            }

            if let Some(i) = best_idx {
                let old_si = current.get(i) as f64;
                current.flip(i);
                let new_si = current.get(i) as f64;
                let d_si = new_si - old_si;
                let offset = i * self.rank;
                for r in 0..self.rank {
                    a[r] += self.basis[offset + r] * d_si;
                }
                flips += 1;
            } else {
                break;
            }
        }
        (current, flips)
    }

    /// Synchronous update step: s_i <- sign(h_i)
    pub fn step_synchronous(&self, s: &SpinState) -> SpinState {
        let mut a = vec![0.0; self.rank];
        self.compute_overlaps(s, &mut a);
        let mut new_spins = Vec::with_capacity(self.n);
        for i in 0..self.n {
            let h = self.effective_field_with_overlaps(i, &a);
            new_spins.push(if h >= 0.0 { 1i8 } else { -1i8 });
        }
        SpinState::from_slice(&new_spins)
    }
}

impl EnergyModel for LatentEnergyModel {
    fn num_spins(&self) -> usize {
        self.n
    }

    fn num_parameters(&self) -> usize {
        self.n * self.rank + self.rank
    }

    fn energy(&self, s: &SpinState) -> f64 {
        assert_eq!(s.len(), self.n);
        let mut a = vec![0.0; self.rank];
        self.compute_overlaps(s, &mut a);
        let mut e = 0.0;
        for r in 0..self.rank {
            e -= self.lambdas[r] * self.law.potential(a[r]);
        }
        e
    }

    fn effective_field(&self, s: &SpinState, i: usize) -> f64 {
        let mut a = vec![0.0; self.rank];
        self.compute_overlaps(s, &mut a);
        self.effective_field_with_overlaps(i, &a)
    }

    fn flip_delta(&self, s: &SpinState, i: usize) -> f64 {
        let mut a = vec![0.0; self.rank];
        self.compute_overlaps(s, &mut a);
        let si = s.get(i) as f64;
        let offset = i * self.rank;
        let mut delta_e = 0.0;
        for r in 0..self.rank {
            let u_ir = self.basis[offset + r];
            let delta_a = -2.0 * si * u_ir;
            let diff_pot = self.law.potential(a[r] + delta_a) - self.law.potential(a[r]);
            delta_e -= self.lambdas[r] * diff_pot;
        }
        delta_e
    }
}

#[derive(Debug, Clone)]
pub struct DualLatentModel {
    pub n: usize,
    pub rank: usize,
    pub basis: Vec<f64>,
    pub alpha: f64,
}

impl DualLatentModel {
    pub fn new(n: usize, rank: usize, basis: Vec<f64>, alpha: f64) -> Self {
        Self {
            n,
            rank,
            basis,
            alpha,
        }
    }

    pub fn reconstruct(&self, s: &SpinState, steps: usize) -> SpinState {
        let mut current = s.clone();
        for _ in 0..steps {
            let mut z = vec![0.0; self.rank];
            for i in 0..self.n {
                let si = current.get(i) as f64;
                let offset = i * self.rank;
                for r in 0..self.rank {
                    z[r] += self.basis[offset + r] * si;
                }
            }
            for r in 0..self.rank {
                let val = z[r];
                z[r] = if val > self.alpha {
                    val - self.alpha
                } else if val < -self.alpha {
                    val + self.alpha
                } else {
                    0.0
                };
            }
            let mut new_spins = Vec::with_capacity(self.n);
            for i in 0..self.n {
                let mut field = 0.0;
                let offset = i * self.rank;
                for r in 0..self.rank {
                    field += self.basis[offset + r] * z[r];
                }
                new_spins.push(if field >= 0.0 { 1i8 } else { -1i8 });
            }
            current = SpinState::from_slice(&new_spins);
        }
        current
    }
}

// ---------------------------------------------------------------------------
// Cyclic Relational Networks (EXP-TEN-005 Part 2)
// Closed triad: A -r1-> B -r2-> C -r3-> A
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ClosedTriadMemory {
    pub n_e: usize,
    pub n_r: usize,
    pub entities_a: Vec<SpinState>,
    pub entities_b: Vec<SpinState>,
    pub entities_c: Vec<SpinState>,
    pub relation_1: SpinState,
    pub relation_2: SpinState,
    pub relation_3: SpinState,
    pub beta: f64,
}

impl ClosedTriadMemory {
    pub fn new(
        n_e: usize,
        n_r: usize,
        entities_a: Vec<SpinState>,
        entities_b: Vec<SpinState>,
        entities_c: Vec<SpinState>,
        relation_1: SpinState,
        relation_2: SpinState,
        relation_3: SpinState,
        beta: f64,
    ) -> Self {
        assert_eq!(entities_a.len(), entities_b.len());
        assert_eq!(entities_a.len(), entities_c.len());
        Self {
            n_e,
            n_r,
            entities_a,
            entities_b,
            entities_c,
            relation_1,
            relation_2,
            relation_3,
            beta,
        }
    }

    pub fn num_triads(&self) -> usize {
        self.entities_a.len()
    }

    pub fn forward_attention_edge(
        x: &SpinState,
        r: &SpinState,
        keys_x: &[SpinState],
        rel: &SpinState,
        values_y: &[SpinState],
        beta: f64,
    ) -> SpinState {
        let m = keys_x.len();
        let n_e = x.len();
        let n_r = r.len();
        let scale = 1.0 / ((n_e as f64) * (n_r as f64).sqrt());

        let mut logits = Vec::with_capacity(m);
        let mut max_logit = f64::NEG_INFINITY;
        for mu in 0..m {
            let dot_x = x.dot(&keys_x[mu]) as f64;
            let dot_r = r.dot(rel) as f64;
            let logit = beta * dot_x * dot_r * scale;
            if logit > max_logit {
                max_logit = logit;
            }
            logits.push(logit);
        }

        let mut sum_exp = 0.0;
        let mut weights = Vec::with_capacity(m);
        for &l in &logits {
            let w = (l - max_logit).exp();
            weights.push(w);
            sum_exp += w;
        }

        let mut out_spins = Vec::with_capacity(n_e);
        let inv_sum = 1.0 / sum_exp.max(1e-12);
        for i in 0..n_e {
            let mut field = 0.0;
            for mu in 0..m {
                field += (weights[mu] * inv_sum) * (values_y[mu].get(i) as f64);
            }
            out_spins.push(if field >= 0.0 { 1i8 } else { -1i8 });
        }
        SpinState::from_slice(&out_spins)
    }

    pub fn feedforward_1pass(
        &self,
        s_a: &SpinState,
        _s_b: &SpinState,
        _s_c: &SpinState,
    ) -> (SpinState, SpinState, SpinState) {
        let new_b = Self::forward_attention_edge(
            s_a,
            &self.relation_1,
            &self.entities_a,
            &self.relation_1,
            &self.entities_b,
            self.beta,
        );
        let new_c = Self::forward_attention_edge(
            &new_b,
            &self.relation_2,
            &self.entities_b,
            &self.relation_2,
            &self.entities_c,
            self.beta,
        );
        let new_a = Self::forward_attention_edge(
            &new_c,
            &self.relation_3,
            &self.entities_c,
            &self.relation_3,
            &self.entities_a,
            self.beta,
        );
        (new_a, new_b, new_c)
    }

    pub fn feedforward_2pass(
        &self,
        s_a: &SpinState,
        s_b: &SpinState,
        s_c: &SpinState,
    ) -> (SpinState, SpinState, SpinState) {
        let (a1, b1, c1) = self.feedforward_1pass(s_a, s_b, s_c);
        self.feedforward_1pass(&a1, &b1, &c1)
    }

    pub fn recurrent_attention(
        &self,
        s_a: &SpinState,
        s_b: &SpinState,
        s_c: &SpinState,
        steps: usize,
    ) -> (SpinState, SpinState, SpinState) {
        let mut cur_a = s_a.clone();
        let mut cur_b = s_b.clone();
        let mut cur_c = s_c.clone();
        for _ in 0..steps {
            let (next_a, next_b, next_c) = self.feedforward_1pass(&cur_a, &cur_b, &cur_c);
            cur_a = next_a;
            cur_b = next_b;
            cur_c = next_c;
        }
        (cur_a, cur_b, cur_c)
    }

    pub fn energy_relaxation_step(
        &self,
        s_a: &SpinState,
        s_b: &SpinState,
        s_c: &SpinState,
    ) -> (SpinState, SpinState, SpinState) {
        let m = self.num_triads();
        let compute_slot_field = |slot_in_fwd: &SpinState,
                                  keys_fwd: &[SpinState],
                                  slot_in_bwd: &SpinState,
                                  keys_bwd: &[SpinState],
                                  target_exemplars: &[SpinState]|
         -> Vec<f64> {
            let mut l_fwd = Vec::with_capacity(m);
            let mut max_fwd = f64::NEG_INFINITY;
            for mu in 0..m {
                let dot = slot_in_fwd.dot(&keys_fwd[mu]) as f64;
                let l = self.beta * dot / (self.n_e as f64).sqrt();
                if l > max_fwd {
                    max_fwd = l;
                }
                l_fwd.push(l);
            }
            let mut sum_fwd = 0.0;
            let mut w_fwd = Vec::with_capacity(m);
            for &l in &l_fwd {
                let w = (l - max_fwd).exp();
                w_fwd.push(w);
                sum_fwd += w;
            }

            let mut l_bwd = Vec::with_capacity(m);
            let mut max_bwd = f64::NEG_INFINITY;
            for mu in 0..m {
                let dot = slot_in_bwd.dot(&keys_bwd[mu]) as f64;
                let l = self.beta * dot / (self.n_e as f64).sqrt();
                if l > max_bwd {
                    max_bwd = l;
                }
                l_bwd.push(l);
            }
            let mut sum_bwd = 0.0;
            let mut w_bwd = Vec::with_capacity(m);
            for &l in &l_bwd {
                let w = (l - max_bwd).exp();
                w_bwd.push(w);
                sum_bwd += w;
            }

            let mut fields = vec![0.0; self.n_e];
            for i in 0..self.n_e {
                let mut h = 0.0;
                for mu in 0..m {
                    let ex = target_exemplars[mu].get(i) as f64;
                    h += (w_fwd[mu] / sum_fwd.max(1e-12) + w_bwd[mu] / sum_bwd.max(1e-12)) * ex;
                }
                fields[i] = h;
            }
            fields
        };

        // A receives from B (backward constraint on A->B) and C (forward constraint on C->A)
        let fields_a = compute_slot_field(
            s_c,
            &self.entities_c,
            s_b,
            &self.entities_b,
            &self.entities_a,
        );
        // B receives from A (forward constraint on A->B) and C (backward constraint on B->C)
        let fields_b = compute_slot_field(
            s_a,
            &self.entities_a,
            s_c,
            &self.entities_c,
            &self.entities_b,
        );
        // C receives from B (forward constraint on B->C) and A (backward constraint on C->A)
        let fields_c = compute_slot_field(
            s_b,
            &self.entities_b,
            s_a,
            &self.entities_a,
            &self.entities_c,
        );

        let to_spins = |fields: &[f64]| -> SpinState {
            let mut spins = Vec::with_capacity(fields.len());
            for &h in fields {
                spins.push(if h >= 0.0 { 1i8 } else { -1i8 });
            }
            SpinState::from_slice(&spins)
        };

        (
            to_spins(&fields_a),
            to_spins(&fields_b),
            to_spins(&fields_c),
        )
    }

    pub fn relax_energy(
        &self,
        s_a: &SpinState,
        s_b: &SpinState,
        s_c: &SpinState,
        steps: usize,
    ) -> (SpinState, SpinState, SpinState) {
        let mut cur_a = s_a.clone();
        let mut cur_b = s_b.clone();
        let mut cur_c = s_c.clone();
        for _ in 0..steps {
            let (next_a, next_b, next_c) = self.energy_relaxation_step(&cur_a, &cur_b, &cur_c);
            cur_a = next_a;
            cur_b = next_b;
            cur_c = next_c;
        }
        (cur_a, cur_b, cur_c)
    }
}

#[derive(Debug, Clone)]
pub struct PairwiseCycleHopfield {
    pub n_e: usize,
    pub w_ab: Vec<f64>,
    pub w_bc: Vec<f64>,
    pub w_ca: Vec<f64>,
}

impl PairwiseCycleHopfield {
    pub fn train(
        entities_a: &[SpinState],
        entities_b: &[SpinState],
        entities_c: &[SpinState],
    ) -> Self {
        assert_eq!(entities_a.len(), entities_b.len());
        assert_eq!(entities_a.len(), entities_c.len());
        let m = entities_a.len();
        let n_e = entities_a[0].len();
        let scale = 1.0 / (m as f64 * n_e as f64);

        let mut w_ab = vec![0.0; n_e * n_e];
        let mut w_bc = vec![0.0; n_e * n_e];
        let mut w_ca = vec![0.0; n_e * n_e];

        for mu in 0..m {
            let ea = &entities_a[mu];
            let eb = &entities_b[mu];
            let ec = &entities_c[mu];
            for i in 0..n_e {
                let ai = ea.get(i) as f64;
                let bi = eb.get(i) as f64;
                let ci = ec.get(i) as f64;
                let row_offset = i * n_e;
                for j in 0..n_e {
                    w_ab[row_offset + j] += ai * (eb.get(j) as f64) * scale;
                    w_bc[row_offset + j] += bi * (ec.get(j) as f64) * scale;
                    w_ca[row_offset + j] += ci * (ea.get(j) as f64) * scale;
                }
            }
        }

        Self {
            n_e,
            w_ab,
            w_bc,
            w_ca,
        }
    }

    pub fn step(
        &self,
        s_a: &SpinState,
        s_b: &SpinState,
        s_c: &SpinState,
    ) -> (SpinState, SpinState, SpinState) {
        let n = self.n_e;
        let mut h_a = vec![0.0; n];
        let mut h_b = vec![0.0; n];
        let mut h_c = vec![0.0; n];

        for i in 0..n {
            for j in 0..n {
                let bj = s_b.get(j) as f64;
                let cj = s_c.get(j) as f64;
                let aj = s_a.get(j) as f64;
                // h_a = W_AB * s_B + W_CA^T * s_C
                h_a[i] += self.w_ab[i * n + j] * bj + self.w_ca[j * n + i] * cj;
                // h_b = W_BC * s_C + W_AB^T * s_A
                h_b[i] += self.w_bc[i * n + j] * cj + self.w_ab[j * n + i] * aj;
                // h_c = W_CA * s_A + W_BC^T * s_B
                h_c[i] += self.w_ca[i * n + j] * aj + self.w_bc[j * n + i] * bj;
            }
        }

        let to_spins = |fields: &[f64]| -> SpinState {
            let mut spins = Vec::with_capacity(fields.len());
            for &h in fields {
                spins.push(if h >= 0.0 { 1i8 } else { -1i8 });
            }
            SpinState::from_slice(&spins)
        };

        (to_spins(&h_a), to_spins(&h_b), to_spins(&h_c))
    }

    pub fn relax(
        &self,
        s_a: &SpinState,
        s_b: &SpinState,
        s_c: &SpinState,
        steps: usize,
    ) -> (SpinState, SpinState, SpinState) {
        let mut cur_a = s_a.clone();
        let mut cur_b = s_b.clone();
        let mut cur_c = s_c.clone();
        for _ in 0..steps {
            let (na, nb, nc) = self.step(&cur_a, &cur_b, &cur_c);
            cur_a = na;
            cur_b = nb;
            cur_c = nc;
        }
        (cur_a, cur_b, cur_c)
    }
}
