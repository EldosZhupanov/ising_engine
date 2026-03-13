//! 5D Memory Layout for Quantum Population-Annealing Hypergraph Solver
//!
//! Multi-Spin Coding (MSC) stores 64 parallel replicas per `u64` word,
//! flattened into a 5-dimensional structure.
//!
//! D1: Variable Index (Fastest moving)
//! D2: Multi-Spin Coding (u64, 64 replicas per word)
//! D3: Trotter Slices (Quantum dimension for tunneling)
//! D4: Temperature Replicas (Parallel Tempering)
//! D5: Population Ensemble (For Population Annealing)

/// 64 bit-parallel replicas stored in a single word
pub type SpinWord = u64;

/// Single-Object-of-Arrays (SoA) layout for cache locality and contiguous memory sweeps
pub struct QuantumField {
    pub spins: Vec<SpinWord>,
    pub num_vars: usize,
    pub num_slices: usize,
    pub num_temps: usize,
    pub num_pops: usize,
}

impl QuantumField {
    /// Creates a new QuantumField initialized to 0.
    pub fn new(num_vars: usize, num_slices: usize, num_temps: usize, num_pops: usize) -> Self {
        let total_size = num_vars * num_slices * num_temps * num_pops;
        Self {
            spins: vec![0; total_size],
            num_vars,
            num_slices,
            num_temps,
            num_pops,
        }
    }

    /// Computes the 1D index for a 5D address: [Pop][Temp][Slice][Var]
    ///
    /// Variable Index (v) is the fastest moving (contiguous in memory) to maximize
    /// L1 Cache Line hits (64 bytes = 8 SpinWords).
    #[inline(always)]
    pub fn get_idx(&self, v: usize, s: usize, t: usize, p: usize) -> usize {
        v + (self.num_vars * s)
            + (self.num_vars * self.num_slices * t)
            + (self.num_vars * self.num_slices * self.num_temps * p)
    }

    /// Retrieves an immutable reference to the SpinWord at the given 5D address.
    #[inline(always)]
    pub fn get(&self, v: usize, s: usize, t: usize, p: usize) -> SpinWord {
        self.spins[self.get_idx(v, s, t, p)]
    }

    /// Retrieves a mutable reference to the SpinWord at the given 5D address.
    #[inline(always)]
    pub fn get_mut(&mut self, v: usize, s: usize, t: usize, p: usize) -> &mut SpinWord {
        let idx = self.get_idx(v, s, t, p);
        &mut self.spins[idx]
    }
}
