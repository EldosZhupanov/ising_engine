//! `SpinState` — the single interface between physics and hardware
//! (Constitution §9, ADR-0002). Operators are written against this trait
//! only; layouts (byte-per-replica, bit-sliced, future GPU/FPGA) live below.
//!
//! Energies cross the interface as f64. Integer backends remain exact:
//! every i32-representable value is exact in f64.

/// Per-replica flip mask: bit r set ⇒ replica r flips. Word-packed so
/// bit-sliced backends apply it with a single XOR per word.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplicaMask {
    words: Vec<u64>,
    len: usize,
}

impl ReplicaMask {
    pub fn new(num_replicas: usize) -> Self {
        Self {
            words: vec![0; num_replicas.div_ceil(64)],
            len: num_replicas,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    pub fn set(&mut self, r: usize) {
        debug_assert!(r < self.len);
        self.words[r / 64] |= 1 << (r % 64);
    }

    pub fn get(&self, r: usize) -> bool {
        debug_assert!(r < self.len);
        (self.words[r / 64] >> (r % 64)) & 1 == 1
    }

    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    pub fn words(&self) -> &[u64] {
        &self.words
    }

    pub fn count(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }
}

/// Content digest of a state — the replay/checkpoint anchor (ADR-0004).
/// FNV-1a over the spin planes; equal digests ⇔ identical configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateDigest(pub u64);

pub fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> StateDigest {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    StateDigest(h)
}

/// The backend contract. All methods are deterministic; none allocate.
pub trait SpinState {
    fn num_vars(&self) -> usize;
    fn num_replicas(&self) -> usize;

    /// ΔE of flipping `site` in each replica, written into `out`
    /// (len == num_replicas). O(1) per replica: reads the field ledger.
    fn delta_e_into(&self, site: usize, out: &mut [f64]);

    /// Flip `site` in the masked replicas and maintain all field ledgers.
    fn apply_flips(&mut self, site: usize, mask: &ReplicaMask);

    /// Current energy per replica from the ledger (never a recompute).
    fn energies_into(&self, out: &mut [f64]);

    /// Spin of (site, replica) as a bit.
    fn spin(&self, site: usize, replica: usize) -> bool;

    /// Write replica `r`'s configuration as 0/1 bytes into `out` (len n).
    fn extract_into(&self, replica: usize, out: &mut [u8]);

    /// Overlap q_ab ∈ [−1, 1] between two replicas (P0-cheap on bit planes).
    fn overlap(&self, a: usize, b: usize) -> f64;

    /// Conflict-free coloring of the interaction graph: `coloring()[site]` is
    /// its color, and edge-sharing sites differ. Lets operators run chromatic
    /// (synchronous) sweeps without ever seeing the coupling structure. Cheap
    /// to call but not free — operators cache it on first use. Deterministic.
    fn coloring(&self) -> Vec<u32>;

    /// Neighbors of `site` in the interaction graph (TOPOLOGY only, no weights).
    /// Cluster operators (Houdayer, ICM) walk this to grow disagreement
    /// components without ever seeing the coupling values.
    fn neighbors(&self, site: usize) -> &[u32];

    /// Overwrite replica `to` with a copy of replica `from`, maintaining every
    /// field/energy ledger. Default impl flips only the differing sites through
    /// the (verified) `apply_flips`, so any backend inherits it correctly.
    /// Population operators (Population Annealing, elitism) rely on it.
    fn copy_replica(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        let (n, r) = (self.num_vars(), self.num_replicas());
        let mut mask = ReplicaMask::new(r);
        for site in 0..n {
            if self.spin(site, from) != self.spin(site, to) {
                mask.clear();
                mask.set(to);
                self.apply_flips(site, &mask);
            }
        }
    }

    /// Exchange the configurations of replicas `a` and `b` (used by replica
    /// exchange / parallel tempering). Default impl flips both at the sites
    /// where they differ, through `apply_flips`, so it stays exact everywhere.
    fn swap_replicas(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        let (n, r) = (self.num_vars(), self.num_replicas());
        let mut mask = ReplicaMask::new(r);
        for site in 0..n {
            if self.spin(site, a) != self.spin(site, b) {
                mask.clear();
                mask.set(a);
                mask.set(b);
                self.apply_flips(site, &mask);
            }
        }
    }

    /// Content digest over all spin planes (replay anchor).
    fn digest(&self) -> StateDigest;

    /// Audit: rebuild fields/energies from scratch and compare with ledgers.
    /// Returns the max absolute ledger drift (must be 0.0 on integer
    /// substrates; used by tests and the Verification Track).
    fn audit(&self) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replica_mask_roundtrip() {
        let mut m = ReplicaMask::new(130);
        assert!(m.is_empty());
        for r in [0, 63, 64, 129] {
            m.set(r);
            assert!(m.get(r));
        }
        assert_eq!(m.count(), 4);
        assert!(!m.get(1));
        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn digest_distinguishes_configurations() {
        assert_ne!(fnv1a([0u8, 1, 0]), fnv1a([0u8, 0, 1]));
        assert_eq!(fnv1a([1u8, 2, 3]), fnv1a([1u8, 2, 3]));
    }
}
