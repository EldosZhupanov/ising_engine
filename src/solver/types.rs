//! 5D Memory Layout for Quantum Population-Annealing Hypergraph Solver
//!
//! SIMD-First Architecture: Byte-per-replica layout replaces packed u64 Multi-Spin Coding.
//! Each replica's spin state is stored as a separate i8 (0 or 1) in contiguous memory,
//! enabling AVX2 vectorization of the inner replica loop.
//!
//! Memory layout per variable: [rep0, rep1, rep2, ..., rep63] — 64 contiguous bytes.
//! This maps directly to 8× AVX2 ymm loads (4 × f64 per ymm register).
//!
//! D1: Variable Index
//! D2: Replica Index (64 replicas, contiguous for SIMD)
//! D3: Trotter Slices
//! D4: Temperature Replicas
//! D5: Population Ensemble

/// Number of parallel replicas. Must be 64 for backward compatibility.
pub const NUM_REPLICAS: usize = 64;

/// Single-Object-of-Arrays (SoA) layout for cache locality and SIMD vectorization.
///
/// Spin storage uses byte-per-replica layout: each spin is a single i8 (0 or 1).
/// 64 replicas per variable are contiguous in memory, enabling direct AVX2 vector loads.
pub struct QuantumField {
    /// Byte-per-replica spin storage.
    /// Layout: spins[(v + num_vars * (s + num_slices * (t + num_temps * p))) * NUM_REPLICAS + r]
    /// where r is the replica index 0..63.
    pub spins: Vec<i8>,
    /// Tracked energies per (temperature, population) cell for incremental PT.
    /// Indexed by `t + num_temps * p`. Eliminates O(N×E) full recalculation.
    pub energies: Vec<[f64; NUM_REPLICAS]>,
    pub num_vars: usize,
    pub num_slices: usize,
    pub num_temps: usize,
    pub num_pops: usize,
}

impl QuantumField {
    /// Creates a new QuantumField with byte-per-replica layout, initialized to 0.
    pub fn new(num_vars: usize, num_slices: usize, num_temps: usize, num_pops: usize) -> Self {
        let total_vars = num_vars * num_slices * num_temps * num_pops;
        let total_size = total_vars * NUM_REPLICAS;
        let energy_cells = num_temps * num_pops;
        Self {
            spins: vec![0i8; total_size],
            energies: vec![[0.0; NUM_REPLICAS]; energy_cells],
            num_vars,
            num_slices,
            num_temps,
            num_pops,
        }
    }

    /// Computes the base index for variable v's replica block at (s, t, p).
    /// The NUM_REPLICAS replicas are at indices base..base+NUM_REPLICAS.
    #[inline(always)]
    pub fn var_base(&self, v: usize, s: usize, t: usize, p: usize) -> usize {
        (v + self.num_vars * (s + self.num_slices * (t + self.num_temps * p))) * NUM_REPLICAS
    }

    /// Gets a single replica value (backward compat for tests).
    /// Returns the i8 spin value (0 or 1) for replica `r` of variable (v, s, t, p).
    #[inline(always)]
    pub fn get_replica(&self, v: usize, s: usize, t: usize, p: usize, r: usize) -> i8 {
        self.spins[self.var_base(v, s, t, p) + r]
    }

    /// Sets a single replica value (backward compat for tests).
    #[inline(always)]
    pub fn set_replica(&mut self, v: usize, s: usize, t: usize, p: usize, r: usize, val: i8) {
        let base = self.var_base(v, s, t, p);
        self.spins[base + r] = val;
    }

    /// Gets immutable slice of all NUM_REPLICAS replicas for variable (v, s, t, p).
    #[inline(always)]
    pub fn get_replicas(&self, v: usize, s: usize, t: usize, p: usize) -> &[i8] {
        let base = self.var_base(v, s, t, p);
        &self.spins[base..base + NUM_REPLICAS]
    }

    /// Gets mutable slice of all NUM_REPLICAS replicas for variable (v, s, t, p).
    #[inline(always)]
    pub fn get_replicas_mut(&mut self, v: usize, s: usize, t: usize, p: usize) -> &mut [i8] {
        let base = self.var_base(v, s, t, p);
        &mut self.spins[base..base + NUM_REPLICAS]
    }
}
