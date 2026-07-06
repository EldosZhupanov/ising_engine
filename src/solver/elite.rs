//! Elite archive and UBQP path relinking.
//!
//! References:
//! - Elite pool / diversity management: Glover & Laguna, "Tabu Search"
//!   (1997); scatter-search reference set methodology.
//! - Path relinking for UBQP: Wang, Lü, Glover & Hao, "Path relinking for
//!   unconstrained binary quadratic programming", EJOR 223, 595 (2012),
//!   which holds best-known results on standard UBQP benchmarks.
//!
//! Path relinking walks from a source solution to a target by flipping, one
//! at a time, the variables on which they differ — at each step choosing the
//! differing variable whose flip yields the lowest energy (greedy relinking)
//! — and returns the best solution encountered along the path, which can be
//! strictly better than either endpoint. Energy is maintained incrementally
//! via single-flip gains (the same O(deg) update as the 1-opt finisher), so
//! a full relink costs O(nnz). Deterministic: ties break on lowest index.

use crate::core::QuboModel;

/// A bounded archive of high-quality, mutually-diverse solutions, kept sorted
/// by ascending energy.
#[derive(Clone)]
pub struct EliteArchive {
    capacity: usize,
    /// Minimum Hamming distance between any two archived solutions.
    min_hamming: usize,
    entries: Vec<(f64, Vec<i8>)>,
}

impl EliteArchive {
    pub fn new(capacity: usize, min_hamming: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            min_hamming,
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn best_energy(&self) -> Option<f64> {
        self.entries.first().map(|(e, _)| *e)
    }

    pub fn entries(&self) -> &[(f64, Vec<i8>)] {
        &self.entries
    }

    /// Mean pairwise Hamming distance across the archive (0 if < 2 entries) —
    /// a diversity statistic.
    pub fn mean_diversity(&self) -> f64 {
        let m = self.entries.len();
        if m < 2 {
            return 0.0;
        }
        let mut sum = 0usize;
        let mut pairs = 0usize;
        for i in 0..m {
            for j in (i + 1)..m {
                sum += hamming(&self.entries[i].1, &self.entries[j].1);
                pairs += 1;
            }
        }
        sum as f64 / pairs as f64
    }

    /// Inserts `state` (with `energy`). Returns true if archived.
    ///
    /// Diversity rule: if a stored entry is within `min_hamming` of the
    /// candidate, the candidate replaces it only if strictly better;
    /// otherwise it is rejected (keeps the pool diverse). If distinct from
    /// all entries, it is inserted and the worst is evicted past capacity.
    /// Deterministic.
    pub fn insert(&mut self, state: &[i8], energy: f64) -> bool {
        // Find the nearest archived entry within the diversity radius.
        let mut near: Option<usize> = None;
        for (i, (_, s)) in self.entries.iter().enumerate() {
            if hamming(s, state) < self.min_hamming.max(1) {
                near = Some(i);
                break;
            }
        }
        if let Some(i) = near {
            if energy < self.entries[i].0 - 1e-12 {
                self.entries[i] = (energy, state.to_vec());
                self.sort_and_trim();
                return true;
            }
            return false;
        }
        // Distinct: insert if there is room or it beats the current worst.
        if self.entries.len() < self.capacity {
            self.entries.push((energy, state.to_vec()));
            self.sort_and_trim();
            return true;
        }
        let worst = self
            .entries
            .last()
            .map(|(e, _)| *e)
            .unwrap_or(f64::INFINITY);
        if energy < worst - 1e-12 {
            self.entries.push((energy, state.to_vec()));
            self.sort_and_trim();
            return true;
        }
        false
    }

    fn sort_and_trim(&mut self) {
        self.entries
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        self.entries.truncate(self.capacity);
    }
}

fn hamming(a: &[i8], b: &[i8]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

/// Greedy path relinking from `source` toward `target`. Returns the best
/// (state, energy) encountered on the path (endpoints included).
///
/// At each step the differing variable whose flip gives the lowest energy is
/// flipped (moving strictly toward the target); energy is tracked
/// incrementally. O(nnz) total. Deterministic (lowest index on ties).
pub fn path_relink(model: &QuboModel, source: &[i8], target: &[i8]) -> (Vec<i8>, f64) {
    let n = model.num_vars;
    let mut state = source.to_vec();
    let mut energy = model.calculate_total_energy(&state);
    let mut best_state = state.clone();
    let mut best_energy = energy;

    // Incremental flip gains g_i = ΔE of flipping x_i.
    let mut gains = vec![0.0f64; n];
    for i in 0..n {
        let mut field = model.linear[i];
        for (j, w) in model.quadratic.get_row(i) {
            field += w * (state[j] as f64);
        }
        gains[i] = (1.0 - 2.0 * (state[i] as f64)) * field;
    }

    // Differing variables define the moving set.
    let mut differing: Vec<usize> = (0..n).filter(|&i| source[i] != target[i]).collect();

    while !differing.is_empty() {
        // Pick the differing variable with the lowest flip gain (lowest index
        // on ties) — greedy relinking toward the target.
        let mut pick = 0usize;
        let mut best_gain = f64::INFINITY;
        for (k, &i) in differing.iter().enumerate() {
            if gains[i] < best_gain - 1e-15 {
                best_gain = gains[i];
                pick = k;
            }
        }
        let i = differing.swap_remove(pick);

        // Apply the flip and update energy + neighbor gains.
        let delta_dir = 1.0 - 2.0 * (state[i] as f64);
        energy += gains[i];
        state[i] = 1 - state[i];
        gains[i] = -gains[i];
        for (j, w) in model.quadratic.get_row(i) {
            gains[j] += (1.0 - 2.0 * (state[j] as f64)) * w * delta_dir;
        }

        if energy < best_energy {
            best_energy = energy;
            best_state.copy_from_slice(&state);
        }
    }
    (best_state, best_energy)
}
