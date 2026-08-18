//! Problem IR — the single canonical representation every frontend lowers
//! into and every backend consumes (Constitution §7, ADR-0005).
//!
//! Objective (QUBO, x ∈ {0,1}):
//!   E(x) = offset + Σ_i `linear[i]`·x_i + Σ_{i<j} q_ij·x_i·x_j
//!
//! The quadratic part is stored as symmetric CSR with BOTH triangles
//! materialized: `row(i)` holds every neighbor j with the full pair
//! coefficient q_ij. Local field h_i = `linear[i]` + Σ_j q_ij·x_j is then a
//! single row scan, and ΔE(flip i) = (1 − 2·x_i)·h_i.

/// Canonical lowered problem. Immutable after construction.
#[derive(Debug, Clone)]
pub struct ProblemIR {
    pub n: usize,
    pub offset: f64,
    pub linear: Vec<f64>,
    /// CSR row pointers, len n+1.
    pub row_ptr: Vec<u32>,
    /// CSR column indices (both triangles), len 2m.
    pub col_idx: Vec<u32>,
    /// Full pair coefficient q_ij per entry, len 2m.
    pub weights: Vec<f64>,
}

impl ProblemIR {
    /// Build from an upper-triangle pair list (i < j, full coefficient q_ij).
    /// Duplicate pairs are accumulated. Deterministic: CSR rows are ordered
    /// by ascending column index regardless of input order (ADR-0005).
    pub fn from_pairs(n: usize, offset: f64, linear: Vec<f64>, pairs: &[(u32, u32, f64)]) -> Self {
        assert_eq!(linear.len(), n, "linear length must equal n");
        // Accumulate duplicates in a deterministic dense-per-row pass.
        let mut adj: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
        for &(i, j, q) in pairs {
            let (i, j) = (i as usize, j as usize);
            assert!(i < j && j < n, "pairs must satisfy i < j < n");
            adj[i].push((j as u32, q));
            adj[j].push((i as u32, q));
        }
        let mut row_ptr = Vec::with_capacity(n + 1);
        let mut col_idx = Vec::new();
        let mut weights = Vec::new();
        row_ptr.push(0u32);
        for row in &mut adj {
            row.sort_unstable_by_key(|&(j, _)| j);
            let mut k = 0;
            while k < row.len() {
                let j = row[k].0;
                let mut q = 0.0;
                while k < row.len() && row[k].0 == j {
                    q += row[k].1;
                    k += 1;
                }
                if q != 0.0 {
                    col_idx.push(j);
                    weights.push(q);
                }
            }
            row_ptr.push(col_idx.len() as u32);
        }
        Self {
            n,
            offset,
            linear,
            row_ptr,
            col_idx,
            weights,
        }
    }

    /// Number of nonzero pairs (each stored twice in CSR).
    pub fn num_pairs(&self) -> usize {
        self.col_idx.len() / 2
    }

    /// Neighbors of site i as parallel (col, weight) slices.
    pub fn row(&self, i: usize) -> (&[u32], &[f64]) {
        let (a, b) = (self.row_ptr[i] as usize, self.row_ptr[i + 1] as usize);
        (&self.col_idx[a..b], &self.weights[a..b])
    }

    /// Canonical scorer: evaluate E(x) from scratch against the bare model.
    /// The last line of defense (ADR-0005): every claimed answer is re-scored
    /// through this function, never through a backend's incremental ledger.
    pub fn energy(&self, x: &[u8]) -> f64 {
        assert_eq!(x.len(), self.n);
        let mut e = self.offset;
        for i in 0..self.n {
            if x[i] == 0 {
                continue;
            }
            e += self.linear[i];
            let (cols, ws) = self.row(i);
            let mut pair = 0.0;
            for (&j, &w) in cols.iter().zip(ws) {
                if x[j as usize] != 0 {
                    pair += w;
                }
            }
            e += 0.5 * pair; // both triangles stored → halve
        }
        e
    }

    /// Greedy vertex coloring of the interaction graph: `colors[i]` is site i's
    /// color, and edge-sharing sites always get different colors. Sites are
    /// colored in ascending index order, each taking the smallest color unused
    /// by its already-colored neighbors — fully deterministic (ADR-0004).
    ///
    /// Same-color sites form an independent set, so an operator may update a
    /// whole color class as one synchronous (chromatic) step without any
    /// site's move disturbing another's local field. Operators consume this
    /// through `SpinState::coloring`; the coupling weights never leak to them.
    pub fn greedy_coloring(&self) -> Vec<u32> {
        let mut colors = vec![u32::MAX; self.n];
        let mut used: Vec<u32> = Vec::new(); // neighbor colors seen for this site
        for i in 0..self.n {
            used.clear();
            let (cols, _) = self.row(i);
            for &j in cols {
                let c = colors[j as usize];
                if c != u32::MAX {
                    used.push(c);
                }
            }
            used.sort_unstable();
            // Smallest color not present among colored neighbors.
            let mut c = 0u32;
            for &u in &used {
                if u == c {
                    c += 1;
                } else if u > c {
                    break;
                }
            }
            colors[i] = c;
        }
        colors
    }

    /// True iff every coefficient is integral — eligibility for the integer
    /// substrate without an ε-certificate (ADR-0002, ADR-0005).
    pub fn is_integral(&self) -> bool {
        let intish = |v: f64| v.fract() == 0.0;
        intish(self.offset)
            && self.linear.iter().all(|&v| intish(v))
            && self.weights.iter().all(|&v| intish(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute_force_min(ir: &ProblemIR) -> f64 {
        let mut best = f64::INFINITY;
        for bits in 0u32..(1 << ir.n) {
            let x: Vec<u8> = (0..ir.n).map(|i| ((bits >> i) & 1) as u8).collect();
            best = best.min(ir.energy(&x));
        }
        best
    }

    #[test]
    fn energy_matches_brute_force_definition() {
        // E = 1 + 2x0 - 3x1 + x2 - 4x0x1 + 2x1x2
        let ir = ProblemIR::from_pairs(3, 1.0, vec![2.0, -3.0, 1.0], &[(0, 1, -4.0), (1, 2, 2.0)]);
        assert_eq!(ir.energy(&[0, 0, 0]), 1.0);
        assert_eq!(ir.energy(&[1, 1, 0]), 1.0 + 2.0 - 3.0 - 4.0);
        assert_eq!(ir.energy(&[1, 1, 1]), 1.0 + 2.0 - 3.0 + 1.0 - 4.0 + 2.0);
        assert_eq!(brute_force_min(&ir), -4.0);
    }

    #[test]
    fn duplicate_pairs_accumulate_deterministically() {
        let a = ProblemIR::from_pairs(2, 0.0, vec![0.0; 2], &[(0, 1, 1.5), (0, 1, 0.5)]);
        let b = ProblemIR::from_pairs(2, 0.0, vec![0.0; 2], &[(0, 1, 2.0)]);
        assert_eq!(a.energy(&[1, 1]), b.energy(&[1, 1]));
        assert_eq!(a.col_idx, b.col_idx);
    }

    #[test]
    fn greedy_coloring_is_conflict_free() {
        // Path + a triangle: 0-1-2-0, 2-3, 3-4.
        let ir = ProblemIR::from_pairs(
            5,
            0.0,
            vec![0.0; 5],
            &[
                (0, 1, 1.0),
                (1, 2, 1.0),
                (0, 2, 1.0),
                (2, 3, 1.0),
                (3, 4, 1.0),
            ],
        );
        let colors = ir.greedy_coloring();
        for i in 0..ir.n {
            let (cols, _) = ir.row(i);
            for &j in cols {
                assert_ne!(
                    colors[i], colors[j as usize],
                    "sites {i} and {j} share an edge and a color"
                );
            }
        }
        // A triangle needs 3 colors; greedy in index order uses exactly that.
        assert_eq!(colors[0], 0);
        assert_eq!(colors[1], 1);
        assert_eq!(colors[2], 2);
    }

    #[test]
    fn integrality_detection() {
        let ir = ProblemIR::from_pairs(2, 0.0, vec![1.0, 2.0], &[(0, 1, -3.0)]);
        assert!(ir.is_integral());
        let ir2 = ProblemIR::from_pairs(2, 0.0, vec![1.0, 2.5], &[(0, 1, -3.0)]);
        assert!(!ir2.is_integral());
    }
}
