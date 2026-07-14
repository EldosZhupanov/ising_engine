//! Reference `SpinState` — the correctness oracle (Constitution §9).
//!
//! Straightforward, obviously-correct field maintenance in f64. Every fast
//! backend must reproduce this backend's ΔE, energies, and flip semantics
//! exactly (`audit()` returns 0.0 drift here by construction). It carries no
//! optimization dynamics — those are operators.
//!
//! Layout: spins and fields are stored replica-major within each site so a
//! single-site update touches contiguous per-replica lanes.

use super::super::ir::ProblemIR;
use super::super::state::{fnv1a, ReplicaMask, SpinState, StateDigest};

pub struct ReferenceState<'a> {
    ir: &'a ProblemIR,
    r: usize,
    /// spins[site * r + replica] ∈ {0, 1}
    spins: Vec<u8>,
    /// fields[site * r + replica] = linear[site] + Σ_j q · x_j
    fields: Vec<f64>,
    /// energies[replica]
    energies: Vec<f64>,
}

impl<'a> ReferenceState<'a> {
    /// Build with `num_replicas` copies of the given initial configuration
    /// (len n, 0/1). Fields and energies are computed from scratch once.
    pub fn new(ir: &'a ProblemIR, num_replicas: usize, init: &[u8]) -> Self {
        assert_eq!(init.len(), ir.n);
        let n = ir.n;
        let r = num_replicas;
        let mut spins = vec![0u8; n * r];
        for site in 0..n {
            for rep in 0..r {
                spins[site * r + rep] = init[site] & 1;
            }
        }
        let mut s = Self {
            ir,
            r,
            spins,
            fields: vec![0.0; n * r],
            energies: vec![0.0; r],
        };
        s.rebuild();
        s
    }

    #[inline]
    fn idx(&self, site: usize, rep: usize) -> usize {
        site * self.r + rep
    }

    /// Recompute every field and energy from scratch (used at init and audit).
    fn rebuild(&mut self) {
        let n = self.ir.n;
        for v in self.fields.iter_mut() {
            *v = 0.0;
        }
        for site in 0..n {
            for rep in 0..self.r {
                self.fields[site * self.r + rep] = self.ir.linear[site];
            }
        }
        for site in 0..n {
            let (cols, ws) = self.ir.row(site);
            for (&j, &w) in cols.iter().zip(ws) {
                let j = j as usize;
                for rep in 0..self.r {
                    if self.spins[site * self.r + rep] != 0 {
                        self.fields[j * self.r + rep] += w;
                    }
                }
            }
        }
        for rep in 0..self.r {
            let mut x = vec![0u8; n];
            for (site, xi) in x.iter_mut().enumerate() {
                *xi = self.spins[site * self.r + rep];
            }
            self.energies[rep] = self.ir.energy(&x);
        }
    }
}

impl SpinState for ReferenceState<'_> {
    fn num_vars(&self) -> usize {
        self.ir.n
    }

    fn num_replicas(&self) -> usize {
        self.r
    }

    fn delta_e_into(&self, site: usize, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.r);
        for (rep, o) in out.iter_mut().enumerate() {
            let x = self.spins[self.idx(site, rep)];
            let h = self.fields[self.idx(site, rep)];
            // (1 - 2x) · h : +h if flipping 0→1, −h if flipping 1→0.
            *o = if x == 0 { h } else { -h };
        }
    }

    fn apply_flips(&mut self, site: usize, mask: &ReplicaMask) {
        debug_assert_eq!(mask.len(), self.r);
        // Snapshot per-replica ΔE and the sign of the change before mutating.
        for rep in 0..self.r {
            if !mask.get(rep) {
                continue;
            }
            let si = self.idx(site, rep);
            let x = self.spins[si];
            let h = self.fields[si];
            let de = if x == 0 { h } else { -h };
            self.energies[rep] += de;
            // sign of (new x - old x): +1 if 0→1, −1 if 1→0.
            let sign = if x == 0 { 1.0 } else { -1.0 };
            self.spins[si] ^= 1;
            // Scatter into neighbor fields for this replica.
            let (cols, ws) = self.ir.row(site);
            for (&j, &w) in cols.iter().zip(ws) {
                self.fields[(j as usize) * self.r + rep] += sign * w;
            }
        }
    }

    fn energies_into(&self, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.r);
        out.copy_from_slice(&self.energies);
    }

    fn spin(&self, site: usize, replica: usize) -> bool {
        self.spins[self.idx(site, replica)] != 0
    }

    fn extract_into(&self, replica: usize, out: &mut [u8]) {
        debug_assert_eq!(out.len(), self.ir.n);
        for (site, o) in out.iter_mut().enumerate() {
            *o = self.spins[self.idx(site, replica)];
        }
    }

    fn overlap(&self, a: usize, b: usize) -> f64 {
        let n = self.ir.n;
        if n == 0 {
            return 1.0;
        }
        let mut acc = 0i64;
        for site in 0..n {
            let sa = 2 * self.spins[self.idx(site, a)] as i64 - 1;
            let sb = 2 * self.spins[self.idx(site, b)] as i64 - 1;
            acc += sa * sb;
        }
        acc as f64 / n as f64
    }

    fn coloring(&self) -> Vec<u32> {
        self.ir.greedy_coloring()
    }

    fn neighbors(&self, site: usize) -> &[u32] {
        self.ir.row(site).0
    }

    fn digest(&self) -> StateDigest {
        fnv1a(self.spins.iter().copied())
    }

    fn audit(&self) -> f64 {
        let n = self.ir.n;
        let mut fresh_fields = vec![0.0f64; n * self.r];
        for site in 0..n {
            for rep in 0..self.r {
                fresh_fields[site * self.r + rep] = self.ir.linear[site];
            }
        }
        for site in 0..n {
            let (cols, ws) = self.ir.row(site);
            for (&j, &w) in cols.iter().zip(ws) {
                let j = j as usize;
                for rep in 0..self.r {
                    if self.spins[site * self.r + rep] != 0 {
                        fresh_fields[j * self.r + rep] += w;
                    }
                }
            }
        }
        let mut drift = 0.0f64;
        for (a, b) in self.fields.iter().zip(&fresh_fields) {
            drift = drift.max((a - b).abs());
        }
        for rep in 0..self.r {
            let mut x = vec![0u8; n];
            for (site, xi) in x.iter_mut().enumerate() {
                *xi = self.spins[site * self.r + rep];
            }
            drift = drift.max((self.energies[rep] - self.ir.energy(&x)).abs());
        }
        drift
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri() -> ProblemIR {
        // E = 1 + 2x0 - 3x1 + x2 - 4x0x1 + 2x1x2
        ProblemIR::from_pairs(3, 1.0, vec![2.0, -3.0, 1.0], &[(0, 1, -4.0), (1, 2, 2.0)])
    }

    #[test]
    fn energies_track_flips_and_match_canonical() {
        let ir = tri();
        let mut st = ReferenceState::new(&ir, 4, &[0, 0, 0]);
        let mut e = vec![0.0; 4];
        st.energies_into(&mut e);
        assert!(e.iter().all(|&v| v == 1.0));

        // Flip site 1 in replicas 0 and 2 only.
        let mut m = ReplicaMask::new(4);
        m.set(0);
        m.set(2);
        st.apply_flips(1, &m);
        st.energies_into(&mut e);
        assert_eq!(e[0], ir.energy(&[0, 1, 0]));
        assert_eq!(e[1], ir.energy(&[0, 0, 0]));
        assert_eq!(e[2], ir.energy(&[0, 1, 0]));
        assert_eq!(st.audit(), 0.0, "ledger must never drift on the oracle");
    }

    #[test]
    fn delta_e_matches_recompute() {
        let ir = tri();
        let st = ReferenceState::new(&ir, 2, &[1, 0, 1]);
        let mut de = vec![0.0; 2];
        st.delta_e_into(1, &mut de);
        let base = ir.energy(&[1, 0, 1]);
        let flipped = ir.energy(&[1, 1, 1]);
        assert_eq!(de[0], flipped - base);
    }

    #[test]
    fn overlap_extremes() {
        let ir = tri();
        let st = ReferenceState::new(&ir, 2, &[1, 0, 1]);
        assert_eq!(st.overlap(0, 1), 1.0); // identical replicas
    }
}
