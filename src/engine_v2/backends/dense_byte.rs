//! `DenseByteState` — the dense float workhorse (Blueprint v0.2, ADR-0002).
//!
//! Production-shaped kernel under the same `SpinState` contract: weights are
//! stored as CONTIGUOUS dense rows (`w[site·n ..][j]`, zero where no edge) and
//! spins/fields as per-replica PLANES (`fields[rep·n + j]`), so the hot path —
//! scattering one flipped site into every neighbor field — is a single
//! contiguous `f[j] += sign · wrow[j]` pass the compiler auto-vectorizes,
//! exactly the loop shape of the production dense engine. On dense instances
//! this replaces `ReferenceState`'s per-edge indexed scatter.
//!
//! Semantics are IDENTICAL to `ReferenceState` by construction: the dense row
//! carries 0.0 where the CSR has no entry, and `x + 0.0` leaves every f64
//! unchanged (fields start from `linear`, which is finite), so ΔE, energies,
//! digests, and trajectories match bit-for-bit — proven by
//! `tests/test_dense_byte_verification.rs`.
//!
//! Memory: the dense matrix costs n²·8 bytes. `boxed_state` only routes here
//! below [`DENSE_N_LIMIT`]; larger instances keep the reference oracle.

use super::super::ir::ProblemIR;
use super::super::state::{fnv1a, ReplicaMask, SpinState, StateDigest};

/// Largest n for which the n² dense matrix is worth it (4096² ⇒ 128 MiB).
pub const DENSE_N_LIMIT: usize = 4096;

pub struct DenseByteState<'a> {
    ir: &'a ProblemIR,
    n: usize,
    r: usize,
    /// Dense symmetric weight rows: `w[site * n + j]`, 0.0 off-support.
    w: Vec<f64>,
    /// spins[rep * n + site] ∈ {0, 1} — one contiguous plane per replica.
    spins: Vec<u8>,
    /// fields[rep * n + site] = linear[site] + Σ_j w·x_j — plane per replica.
    fields: Vec<f64>,
    energies: Vec<f64>,
}

impl<'a> DenseByteState<'a> {
    /// Build with `num_replicas` copies of `init` (len n, 0/1).
    pub fn new(ir: &'a ProblemIR, num_replicas: usize, init: &[u8]) -> Self {
        assert_eq!(init.len(), ir.n);
        let n = ir.n;
        let r = num_replicas;
        let mut w = vec![0.0f64; n * n];
        for site in 0..n {
            let (cols, ws) = ir.row(site);
            for (&j, &wt) in cols.iter().zip(ws) {
                w[site * n + j as usize] = wt;
            }
        }
        let mut spins = vec![0u8; r * n];
        for rep in 0..r {
            for site in 0..n {
                spins[rep * n + site] = init[site] & 1;
            }
        }
        let mut s = Self {
            ir,
            n,
            r,
            w,
            spins,
            fields: vec![0.0; r * n],
            energies: vec![0.0; r],
        };
        s.rebuild();
        s
    }

    /// Recompute every field and energy from scratch (init and audit).
    fn rebuild(&mut self) {
        let n = self.n;
        for rep in 0..self.r {
            let plane = &mut self.fields[rep * n..(rep + 1) * n];
            plane.copy_from_slice(&self.ir.linear);
            for site in 0..n {
                if self.spins[rep * n + site] != 0 {
                    let wrow = &self.w[site * n..(site + 1) * n];
                    for (f, &wj) in plane.iter_mut().zip(wrow) {
                        *f += wj;
                    }
                }
            }
            let x = &self.spins[rep * n..(rep + 1) * n];
            self.energies[rep] = self.ir.energy(x);
        }
    }
}

impl SpinState for DenseByteState<'_> {
    fn num_vars(&self) -> usize {
        self.n
    }

    fn num_replicas(&self) -> usize {
        self.r
    }

    fn delta_e_into(&self, site: usize, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.r);
        for (rep, o) in out.iter_mut().enumerate() {
            let x = self.spins[rep * self.n + site];
            let h = self.fields[rep * self.n + site];
            *o = if x == 0 { h } else { -h };
        }
    }

    fn apply_flips(&mut self, site: usize, mask: &ReplicaMask) {
        debug_assert_eq!(mask.len(), self.r);
        let n = self.n;
        let wrow = &self.w[site * n..(site + 1) * n];
        for rep in 0..self.r {
            if !mask.get(rep) {
                continue;
            }
            let si = rep * n + site;
            let x = self.spins[si];
            let h = self.fields[si];
            self.energies[rep] += if x == 0 { h } else { -h };
            self.spins[si] ^= 1;
            let plane = &mut self.fields[rep * n..(rep + 1) * n];
            // The production-shaped kernel: one contiguous FMA-able pass.
            if x == 0 {
                for (f, &wj) in plane.iter_mut().zip(wrow) {
                    *f += wj;
                }
            } else {
                for (f, &wj) in plane.iter_mut().zip(wrow) {
                    *f -= wj;
                }
            }
        }
    }

    fn energies_into(&self, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.r);
        out.copy_from_slice(&self.energies);
    }

    fn spin(&self, site: usize, replica: usize) -> bool {
        self.spins[replica * self.n + site] != 0
    }

    fn extract_into(&self, replica: usize, out: &mut [u8]) {
        debug_assert_eq!(out.len(), self.n);
        out.copy_from_slice(&self.spins[replica * self.n..(replica + 1) * self.n]);
    }

    fn overlap(&self, a: usize, b: usize) -> f64 {
        if self.n == 0 {
            return 1.0;
        }
        let pa = &self.spins[a * self.n..(a + 1) * self.n];
        let pb = &self.spins[b * self.n..(b + 1) * self.n];
        let agree: i64 = pa
            .iter()
            .zip(pb)
            .map(|(&x, &y)| if x == y { 1i64 } else { -1 })
            .sum();
        agree as f64 / self.n as f64
    }

    fn coloring(&self) -> Vec<u32> {
        self.ir.greedy_coloring()
    }

    fn neighbors(&self, site: usize) -> &[u32] {
        self.ir.row(site).0
    }

    fn digest(&self) -> StateDigest {
        // Same site-major byte order as ReferenceState, so identical
        // configurations produce identical digests across backends.
        let (n, r) = (self.n, self.r);
        fnv1a((0..n * r).map(|k| self.spins[(k % r) * n + k / r]))
    }

    fn audit(&self) -> f64 {
        let n = self.n;
        let mut drift = 0.0f64;
        for rep in 0..self.r {
            let mut fresh = self.ir.linear.clone();
            for site in 0..n {
                if self.spins[rep * n + site] != 0 {
                    let wrow = &self.w[site * n..(site + 1) * n];
                    for (f, &wj) in fresh.iter_mut().zip(wrow) {
                        *f += wj;
                    }
                }
            }
            let plane = &self.fields[rep * n..(rep + 1) * n];
            for (a, b) in plane.iter().zip(&fresh) {
                drift = drift.max((a - b).abs());
            }
            let x = &self.spins[rep * n..(rep + 1) * n];
            drift = drift.max((self.energies[rep] - self.ir.energy(x)).abs());
        }
        drift
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri() -> ProblemIR {
        ProblemIR::from_pairs(3, 1.0, vec![2.0, -3.0, 1.0], &[(0, 1, -4.0), (1, 2, 2.0)])
    }

    #[test]
    fn energies_track_flips_and_match_canonical() {
        let ir = tri();
        let mut st = DenseByteState::new(&ir, 4, &[0, 0, 0]);
        let mut e = vec![0.0; 4];
        st.energies_into(&mut e);
        assert!(e.iter().all(|&v| v == 1.0));
        let mut m = ReplicaMask::new(4);
        m.set(0);
        m.set(2);
        st.apply_flips(1, &m);
        st.energies_into(&mut e);
        assert_eq!(e[0], ir.energy(&[0, 1, 0]));
        assert_eq!(e[1], ir.energy(&[0, 0, 0]));
        assert_eq!(e[2], ir.energy(&[0, 1, 0]));
        assert_eq!(st.audit(), 0.0);
    }

    #[test]
    fn extract_and_overlap_use_replica_planes() {
        let ir = tri();
        let mut st = DenseByteState::new(&ir, 2, &[1, 0, 1]);
        let mut out = vec![0u8; 3];
        st.extract_into(0, &mut out);
        assert_eq!(out, vec![1, 0, 1]);
        assert_eq!(st.overlap(0, 1), 1.0);
        let mut m = ReplicaMask::new(2);
        m.set(1);
        st.apply_flips(0, &m);
        // replicas now differ at one of three sites: (2 - 1)/3
        assert!((st.overlap(0, 1) - 1.0 / 3.0).abs() < 1e-12);
    }
}
