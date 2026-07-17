//! `SparseBitSlice` — the integer bit-sliced state backend (ADR-0002, ADR-0003).
//!
//! One 64-bit word packs 64 replicas of ONE spin (site-major bit planes), so a
//! 64-byte cache line holds 512 replicas of a site — bandwidth becomes ensemble
//! width. Couplings and local fields are INTEGERS (ADR-0004: exact, no rounding,
//! trajectories reproducible by construction). Eligible only for integral
//! instances; the Decision Engine routes here (sparse + integral).
//!
//! This is state MECHANICS, not an algorithm: it maintains fields and energies
//! under flips. Optimization dynamics are operators, layered above.
//!
//! Correctness is anchored to `ReferenceState`: `sparse_matches_reference`
//! replays identical flip sequences on both and asserts equal energies, ΔE,
//! and overlaps, with zero field-ledger drift.

use super::super::ir::ProblemIR;
use super::super::state::{fnv1a, ReplicaMask, SpinState, StateDigest};

pub struct SparseBitSlice<'a> {
    ir: &'a ProblemIR,
    r: usize,
    /// words per site = ceil(r / 64)
    w: usize,
    /// spins[site * w + word]: bit b == replica (word*64 + b)
    spins: Vec<u64>,
    /// integer local fields, fields[site * r + replica]
    fields: Vec<i64>,
    /// integer energy per replica
    energies: Vec<i64>,
    ilinear: Vec<i64>,
    iweights: Vec<i64>,
    /// Reusable per-replica flip-sign scratch (len r), invariant: all zero
    /// between `apply_flips` calls. Lets the field update run neighbor-major over
    /// contiguous, vectorizable replica blocks instead of scattered stride-r
    /// writes. Never affects the ledger (sign 0 ⇒ +0).
    flip_sign: Vec<i64>,
}

impl<'a> SparseBitSlice<'a> {
    /// Build with `num_replicas` copies of `init` (len n, 0/1). Errors unless
    /// the instance is integral (the substrate is exact integer arithmetic).
    pub fn new(ir: &'a ProblemIR, num_replicas: usize, init: &[u8]) -> Result<Self, String> {
        if !ir.is_integral() {
            return Err("SparseBitSlice requires integral couplings".into());
        }
        assert_eq!(init.len(), ir.n);
        let r = num_replicas;
        let w = r.div_ceil(64);
        let ilinear: Vec<i64> = ir.linear.iter().map(|&v| v as i64).collect();
        let iweights: Vec<i64> = ir.weights.iter().map(|&v| v as i64).collect();

        let mut spins = vec![0u64; ir.n * w];
        for (site, &bit) in init.iter().enumerate() {
            if bit & 1 == 1 {
                for word in 0..w {
                    let lo = word * 64;
                    let valid = (r - lo).min(64);
                    let m = if valid == 64 {
                        u64::MAX
                    } else {
                        (1u64 << valid) - 1
                    };
                    spins[site * w + word] = m;
                }
            }
        }

        let mut s = Self {
            ir,
            r,
            w,
            spins,
            fields: vec![0; ir.n * r],
            energies: vec![0; r],
            ilinear,
            iweights,
            flip_sign: vec![0; r],
        };
        s.rebuild();
        Ok(s)
    }

    #[inline]
    fn spin_bit(&self, site: usize, rep: usize) -> u64 {
        (self.spins[site * self.w + rep / 64] >> (rep % 64)) & 1
    }

    /// Recompute all fields and energies from scratch (init + audit reference).
    fn rebuild(&mut self) {
        let (n, r) = (self.ir.n, self.r);
        for (site, f) in self.fields.chunks_mut(r).enumerate() {
            f.fill(self.ilinear[site]);
        }
        for site in 0..n {
            let (a, b) = (
                self.ir.row_ptr[site] as usize,
                self.ir.row_ptr[site + 1] as usize,
            );
            for k in a..b {
                let j = self.ir.col_idx[k] as usize;
                let wt = self.iweights[k];
                for rep in 0..r {
                    if self.spin_bit(site, rep) == 1 {
                        self.fields[j * r + rep] += wt;
                    }
                }
            }
        }
        for rep in 0..r {
            self.energies[rep] = self.energy_of(rep);
        }
    }

    /// Exact integer energy of one replica from scratch. Both CSR triangles
    /// are stored, so the quadratic sum counts each pair twice — halve it.
    fn energy_of(&self, rep: usize) -> i64 {
        let n = self.ir.n;
        let mut lin = 0i64;
        let mut quad = 0i64;
        for site in 0..n {
            if self.spin_bit(site, rep) == 0 {
                continue;
            }
            lin += self.ilinear[site];
            let (a, b) = (
                self.ir.row_ptr[site] as usize,
                self.ir.row_ptr[site + 1] as usize,
            );
            for k in a..b {
                let j = self.ir.col_idx[k] as usize;
                if self.spin_bit(j, rep) == 1 {
                    quad += self.iweights[k];
                }
            }
        }
        self.ir.offset as i64 + lin + quad / 2
    }

    /// Field update for SPARSE flips: replica-outer, neighbor-inner. Does
    /// `O(neighbors · flips)` scattered stride-`r` writes — the right choice when
    /// few replicas flip (the writes are few, so their scatter is cheap and it
    /// avoids touching non-flipped replicas). Selected by `apply_flips` when the
    /// flip count is below the measured crossover (~`r/3`).
    #[inline]
    fn apply_flips_sparse(&mut self, site: usize, mask: &ReplicaMask) {
        let (r, w) = (self.r, self.w);
        let (a, b) = (
            self.ir.row_ptr[site] as usize,
            self.ir.row_ptr[site + 1] as usize,
        );
        let sbase = site * w;
        for word in 0..w {
            let mbits = mask.words()[word];
            if mbits == 0 {
                continue;
            }
            let spin_word = self.spins[sbase + word];
            let mut bits = mbits;
            while bits != 0 {
                let bpos = bits.trailing_zeros() as usize;
                let rep = word * 64 + bpos;
                let x = (spin_word >> bpos) & 1;
                let h = self.fields[site * r + rep];
                self.energies[rep] += if x == 0 { h } else { -h };
                let sign: i64 = if x == 0 { 1 } else { -1 };
                for k in a..b {
                    let j = self.ir.col_idx[k] as usize;
                    self.fields[j * r + rep] += sign * self.iweights[k];
                }
                bits &= bits - 1;
            }
            self.spins[sbase + word] ^= mbits;
        }
    }

    /// Field update for DENSE flips: neighbor-outer, replica-inner. Updates each
    /// neighbor's CONTIGUOUS replica block `fields[j*r .. j*r+r]` in one burst — a
    /// vectorizable SAXPY over the flip-sign scratch — giving temporal locality
    /// instead of stride-`r` cache thrash. Does `O(neighbors·r)` work regardless
    /// of flip count, so it wins only above the ~`r/3` crossover. Bit-identical to
    /// `apply_flips_sparse`: each `(j,rep)` cell is written exactly once, integer
    /// add is exact, and a non-flipped replica has `flip_sign == 0` ⇒ it adds 0.
    #[inline]
    fn apply_flips_dense(&mut self, site: usize, mask: &ReplicaMask) {
        let (r, w) = (self.r, self.w);
        let (a, b) = (
            self.ir.row_ptr[site] as usize,
            self.ir.row_ptr[site + 1] as usize,
        );
        let sbase = site * w;

        // Pass 1: on-site energy delta + record flip sign; flip the spin bits.
        for word in 0..w {
            let mbits = mask.words()[word];
            if mbits == 0 {
                continue;
            }
            let spin_word = self.spins[sbase + word]; // pre-flip snapshot
            let mut bits = mbits;
            while bits != 0 {
                let bpos = bits.trailing_zeros() as usize;
                let rep = word * 64 + bpos;
                let x = (spin_word >> bpos) & 1;
                let h = self.fields[site * r + rep];
                self.energies[rep] += if x == 0 { h } else { -h };
                self.flip_sign[rep] = if x == 0 { 1 } else { -1 };
                bits &= bits - 1;
            }
            self.spins[sbase + word] ^= mbits;
        }

        // Pass 2: contiguous, vectorizable per-neighbor block update.
        let fields = &mut self.fields;
        let signs = &self.flip_sign;
        let iweights = &self.iweights;
        let col_idx = &self.ir.col_idx;
        for k in a..b {
            let j = col_idx[k] as usize;
            let wt = iweights[k];
            let block = &mut fields[j * r..j * r + r];
            for (f, &s) in block.iter_mut().zip(signs.iter()) {
                *f += s * wt;
            }
        }

        // Restore the scratch invariant (all zero) — clear only the flipped reps.
        for word in 0..w {
            let mut bits = mask.words()[word];
            while bits != 0 {
                let bpos = bits.trailing_zeros() as usize;
                self.flip_sign[word * 64 + bpos] = 0;
                bits &= bits - 1;
            }
        }
    }
}

impl SpinState for SparseBitSlice<'_> {
    fn num_vars(&self) -> usize {
        self.ir.n
    }

    fn num_replicas(&self) -> usize {
        self.r
    }

    fn delta_e_into(&self, site: usize, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.r);
        let base = site * self.r;
        for (rep, o) in out.iter_mut().enumerate() {
            let h = self.fields[base + rep];
            let de = if self.spin_bit(site, rep) == 0 { h } else { -h };
            *o = de as f64;
        }
    }

    fn apply_flips(&mut self, site: usize, mask: &ReplicaMask) {
        debug_assert_eq!(mask.len(), self.r);
        // The field update has two regimes: the SCATTERED path is O(neighbors·
        // flips); the DENSE path is O(neighbors·r) but contiguous + vectorizable.
        // A/B on real instances puts the crossover at ≈ r/3 flips. Dispatch per
        // call by the actual flip count — so a single sweep automatically uses
        // the dense path on hot (high-acceptance) sites and the scattered path on
        // cold ones. Both are bit-identical (integer ledger, each cell once).
        let flips: u32 = mask.words().iter().map(|x| x.count_ones()).sum();
        if (flips as usize) * 3 >= self.r {
            self.apply_flips_dense(site, mask);
        } else {
            self.apply_flips_sparse(site, mask);
        }
    }

    fn energies_into(&self, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.r);
        for (o, &e) in out.iter_mut().zip(&self.energies) {
            *o = e as f64;
        }
    }

    fn spin(&self, site: usize, replica: usize) -> bool {
        self.spin_bit(site, replica) == 1
    }

    fn extract_into(&self, replica: usize, out: &mut [u8]) {
        debug_assert_eq!(out.len(), self.ir.n);
        for (site, o) in out.iter_mut().enumerate() {
            *o = self.spin_bit(site, replica) as u8;
        }
    }

    fn overlap(&self, a: usize, b: usize) -> f64 {
        let n = self.ir.n;
        if n == 0 {
            return 1.0;
        }
        let mut acc = 0i64;
        for site in 0..n {
            let sa = self.spin_bit(site, a) as i64;
            let sb = self.spin_bit(site, b) as i64;
            acc += if sa == sb { 1 } else { -1 };
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
        fnv1a(self.spins.iter().flat_map(|w| w.to_le_bytes()))
    }

    fn audit(&self) -> f64 {
        let (n, r) = (self.ir.n, self.r);
        let mut fresh = vec![0i64; n * r];
        for (site, f) in fresh.chunks_mut(r).enumerate() {
            f.fill(self.ilinear[site]);
        }
        for site in 0..n {
            let (a, b) = (
                self.ir.row_ptr[site] as usize,
                self.ir.row_ptr[site + 1] as usize,
            );
            for k in a..b {
                let j = self.ir.col_idx[k] as usize;
                let wt = self.iweights[k];
                for rep in 0..r {
                    if self.spin_bit(site, rep) == 1 {
                        fresh[j * r + rep] += wt;
                    }
                }
            }
        }
        let mut drift = 0i64;
        for (a, b) in self.fields.iter().zip(&fresh) {
            drift = drift.max((a - b).abs());
        }
        for rep in 0..r {
            drift = drift.max((self.energies[rep] - self.energy_of(rep)).abs());
        }
        drift as f64
    }
}

#[cfg(test)]
mod tests {
    use super::super::ReferenceState;
    use super::*;
    use crate::engine_v2::state::SpinState;

    #[test]
    #[ignore = "profiling: ISING_PROFILE_RUDY=<gset file> cargo test apply_flips_kernel_ab -- --ignored --nocapture"]
    fn apply_flips_kernel_ab_by_flip_count() {
        use crate::engine_v2::frontend::rudy_maxcut_ir;
        use crate::engine_v2::state::ReplicaMask;
        let Ok(path) = std::env::var("ISING_PROFILE_RUDY") else {
            return;
        };
        let ir = rudy_maxcut_ir(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let (r, sweeps) = (64usize, 40usize);
        eprintln!(
            "apply_flips A/B on n={}, r={r}, {sweeps} sweeps — old(scattered) vs new(neighbor-major), min-of-3",
            ir.n
        );
        eprintln!("   f (flips) | sparse ms |  dense ms | dispatch ms | dispatch/sparse");
        for f in [4usize, 8, 16, 32, 64] {
            let mut mask = ReplicaMask::new(r);
            let step = (r / f).max(1);
            let (mut set, mut rep) = (0usize, 0usize);
            while set < f && rep < r {
                mask.set(rep);
                rep += step;
                set += 1;
            }
            // path: 0 = sparse (forced), 1 = dense (forced), 2 = dispatch (prod).
            let time = |path: u8| -> f64 {
                let mut bs = SparseBitSlice::new(&ir, r, &vec![0u8; ir.n]).unwrap();
                let call = |bs: &mut SparseBitSlice, s: usize| match path {
                    0 => bs.apply_flips_sparse(s, &mask),
                    1 => bs.apply_flips_dense(s, &mask),
                    _ => bs.apply_flips(s, &mask), // the production dispatcher
                };
                for site in 0..ir.n {
                    call(&mut bs, site); // warm
                }
                let t = std::time::Instant::now();
                for _ in 0..sweeps {
                    for site in 0..ir.n {
                        call(std::hint::black_box(&mut bs), site);
                    }
                }
                t.elapsed().as_secs_f64() * 1000.0
            };
            let sparse = (0..3).map(|_| time(0)).fold(f64::INFINITY, f64::min);
            let dense = (0..3).map(|_| time(1)).fold(f64::INFINITY, f64::min);
            let disp = (0..3).map(|_| time(2)).fold(f64::INFINITY, f64::min);
            eprintln!(
                "   {f:>8} | {sparse:9.2} | {dense:9.2} | {disp:11.2} | {:.3}",
                disp / sparse
            );
        }
    }
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    fn sample_ir() -> ProblemIR {
        // Frustrated integral instance, n=7.
        let pairs = [
            (0u32, 1u32, -1.0),
            (1, 2, 2.0),
            (2, 3, -3.0),
            (0, 3, 1.0),
            (3, 4, -1.0),
            (4, 5, 2.0),
            (5, 6, -2.0),
            (2, 6, 1.0),
        ];
        ProblemIR::from_pairs(7, -2.0, vec![1.0, -2.0, 3.0, 0.0, -1.0, 2.0, 1.0], &pairs)
    }

    #[test]
    fn sparse_matches_reference_under_identical_flips() {
        let ir = sample_ir();
        let r = 100; // > 64 → exercises 2 words per site
        let init = [1u8, 0, 1, 0, 0, 1, 0];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();

        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        let mut d_ref = vec![0.0; r];
        let mut d_bs = vec![0.0; r];
        let mut rng = ChaCha8Rng::seed_from_u64(9);

        for _ in 0..300 {
            let site = rng.gen_range(0..ir.n);
            // ΔE must agree before the flip.
            refs.delta_e_into(site, &mut d_ref);
            bs.delta_e_into(site, &mut d_bs);
            assert_eq!(d_ref, d_bs, "delta_e mismatch at site {site}");

            // Identical random replica mask.
            let mut m = ReplicaMask::new(r);
            for rep in 0..r {
                if rng.gen::<bool>() {
                    m.set(rep);
                }
            }
            refs.apply_flips(site, &m);
            bs.apply_flips(site, &m);

            refs.energies_into(&mut e_ref);
            bs.energies_into(&mut e_bs);
            assert_eq!(e_ref, e_bs, "energy mismatch after flipping site {site}");
        }

        assert_eq!(bs.audit(), 0.0, "bitslice field ledger drifted");
        assert_eq!(refs.audit(), 0.0);
        // Overlap agrees too.
        assert_eq!(bs.overlap(0, 1), refs.overlap(0, 1));
        // Canonical scorer agrees with the ledger for a sampled replica.
        let mut x = vec![0u8; ir.n];
        bs.extract_into(7 % r, &mut x);
        assert_eq!(bs.energy_of(7 % r) as f64, ir.energy(&x));
    }

    #[test]
    fn copy_and_swap_replicas_match_reference() {
        let ir = sample_ir();
        let r = 8;
        let init = [1u8, 0, 1, 0, 0, 1, 0];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        // Diverge the replicas first with a few flips.
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        for _ in 0..40 {
            let site = rng.gen_range(0..ir.n);
            let mut m = ReplicaMask::new(r);
            for rep in 0..r {
                if rng.gen::<bool>() {
                    m.set(rep);
                }
            }
            refs.apply_flips(site, &m);
            bs.apply_flips(site, &m);
        }
        // copy 0 -> 5, swap 1 <-> 6, on both backends.
        refs.copy_replica(0, 5);
        bs.copy_replica(0, 5);
        refs.swap_replicas(1, 6);
        bs.swap_replicas(1, 6);

        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs);
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
        // Copied replica equals its source; extracted configs agree.
        for rep in 0..r {
            let mut xr = vec![0u8; ir.n];
            let mut xb = vec![0u8; ir.n];
            refs.extract_into(rep, &mut xr);
            bs.extract_into(rep, &mut xb);
            assert_eq!(xr, xb);
        }
        let mut src = vec![0u8; ir.n];
        let mut dst = vec![0u8; ir.n];
        bs.extract_into(0, &mut src);
        bs.extract_into(5, &mut dst);
        assert_eq!(src, dst, "copy_replica must duplicate the source config");
    }

    #[test]
    fn rejects_non_integral() {
        let ir = ProblemIR::from_pairs(2, 0.0, vec![0.5, 0.0], &[(0, 1, -1.0)]);
        assert!(SparseBitSlice::new(&ir, 8, &[0, 0]).is_err());
    }
}
