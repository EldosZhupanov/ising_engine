//! SIMD-First HPC Ising/HUBO Monte Carlo Kernel
//!
//! Architecture: Byte-per-replica layout with contiguous f64 delta arrays.
//! All inner replica loops (0..NUM_REPLICAS) operate on contiguous memory
//! and use pure arithmetic (no bit shifts, no branches, no function calls).
//! LLVM auto-vectorizes these loops to AVX2 packed f64 operations.

use super::types::{QuantumField, NUM_REPLICAS};
use crate::core::hubo::FlatHuboModel;
use rand::Rng;
use rayon::prelude::*;

/// Xoshiro256++ PRNG for fast batch RNG generation.
/// State is 4×u64 = 32 bytes, fits in registers.
pub struct Xoshiro256PlusPlus {
    s: [u64; 4],
}

impl Xoshiro256PlusPlus {
    #[inline(always)]
    pub fn new(seed: u64) -> Self {
        let mut state = seed;
        let mut next_u64 = || {
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        };
        Self {
            s: [next_u64(), next_u64(), next_u64(), next_u64()],
        }
    }

    #[inline(always)]
    pub fn next_f64(&mut self) -> f64 {
        let result = (self.s[0].wrapping_add(self.s[3]))
            .rotate_left(23)
            .wrapping_add(self.s[0]);

        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;

        self.s[3] = self.s[3].rotate_left(45);

        (result >> 11) as f64 * 1.1102230246251565e-16
    }

    #[inline(always)]
    pub fn fill_f64(&mut self, dest: &mut [f64]) {
        for val in dest.iter_mut() {
            *val = self.next_f64();
        }
    }
}

/// 8-lane interleaved Xoshiro256++ for vectorizable batch RNG generation.
///
/// Eight independent streams in structure-of-arrays layout: the update uses
/// only add/xor/shift/rotate on u64 lanes (no multiplies), so the 8-wide
/// inner loops compile to packed 64-bit AVX2 ops — and even without packed
/// codegen, eight independent dependency chains pipeline out-of-order,
/// unlike the strictly serial single-stream generator.
///
/// Seeding: per-lane state words drawn from a SplitMix64 sequence, following
/// the generator authors' recommendation (Blackman & Vigna, "Scrambled
/// Linear Pseudorandom Number Generators", ACM TOMS 2021). Distinct
/// SplitMix-derived states give independent streams (overlap probability is
/// negligible in a 2^256 state space).
///
/// Uniform output uses the mantissa bit-trick: set exponent to 1.0's,
/// fill the 52 mantissa bits with random bits, subtract 1.0 → [0, 1).
/// The bitcast is free and the whole conversion vectorizes on AVX2
/// (the plain u64→f64 cast has no packed AVX2 form).
pub struct Xoshiro256PlusPlusX8 {
    /// s[word][lane]: four xoshiro state words × eight lanes.
    s: [[u64; 8]; 4],
}

impl Xoshiro256PlusPlusX8 {
    pub fn new(seed: u64) -> Self {
        let mut state = seed;
        let mut next_u64 = || {
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        };
        let mut s = [[0u64; 8]; 4];
        for lane in 0..8 {
            for word in s.iter_mut() {
                word[lane] = next_u64();
            }
        }
        Self { s }
    }

    /// Advances all 8 lanes once and returns 8 uniform doubles in [0, 1).
    #[inline(always)]
    fn next_f64x8(&mut self) -> [f64; 8] {
        let mut out = [0.0f64; 8];
        for (l, slot) in out.iter_mut().enumerate() {
            let result = (self.s[0][l].wrapping_add(self.s[3][l]))
                .rotate_left(23)
                .wrapping_add(self.s[0][l]);
            *slot = f64::from_bits(0x3FF0000000000000 | (result >> 12)) - 1.0;
        }
        for l in 0..8 {
            let t = self.s[1][l] << 17;
            self.s[2][l] ^= self.s[0][l];
            self.s[3][l] ^= self.s[1][l];
            self.s[1][l] ^= self.s[2][l];
            self.s[0][l] ^= self.s[3][l];
            self.s[2][l] ^= t;
            self.s[3][l] = self.s[3][l].rotate_left(45);
        }
        out
    }

    pub fn fill_f64(&mut self, dest: &mut [f64]) {
        let mut chunks = dest.chunks_exact_mut(8);
        for chunk in &mut chunks {
            chunk.copy_from_slice(&self.next_f64x8());
        }
        let rem = chunks.into_remainder();
        if !rem.is_empty() {
            let vals = self.next_f64x8();
            rem.copy_from_slice(&vals[..rem.len()]);
        }
    }

    /// One replica-block (64 lanes) of uniforms — exactly the next 64 values
    /// of the same stream `fill_f64` would produce (8 × 8-lane draws, no
    /// remainder since NUM_REPLICAS = 64). Lets the sweep generate its
    /// acceptance randomness just-in-time in registers/L1 instead of
    /// round-tripping an 8-bytes-per-spin-site buffer through memory.
    #[inline(always)]
    pub fn next_f64x64(&mut self) -> [f64; NUM_REPLICAS] {
        let mut out = [0.0f64; NUM_REPLICAS];
        for block in out.chunks_exact_mut(8) {
            block.copy_from_slice(&self.next_f64x8());
        }
        out
    }
}

/// Fast exponential approximation using 6th-order Taylor polynomial.
/// Pure f64 arithmetic — no integer casts, no bitcasts, fully SIMD-vectorizable.
/// For x in [-20, 0], accuracy is sufficient for Metropolis acceptance.
///
/// Uses range reduction: exp(x) = exp(x/16)^16 via repeated squaring.
/// This keeps the polynomial argument small for better accuracy.
#[inline(always)]
fn fast_exp(x: f64) -> f64 {
    // Clamp to valid range — compiles to vmaxpd/vminpd
    let x = x.clamp(-20.0, 0.0);

    // Range reduction: compute exp(x/16) then square 4 times
    let r = x * 0.0625; // x / 16

    // Taylor series for exp(r) where |r| <= 1.25
    // exp(r) ≈ 1 + r + r²/2 + r³/6 + r⁴/24 + r⁵/120 + r⁶/720
    let r2 = r * r;
    let r3 = r2 * r;
    let p = 1.0
        + r
        + 0.5 * r2
        + (1.0 / 6.0) * r3
        + (1.0 / 24.0) * r2 * r2
        + (1.0 / 120.0) * r2 * r3
        + (1.0 / 720.0) * r3 * r3;

    // Square 4 times: p^16 = exp(x/16)^16 = exp(x)
    let p2 = p * p;
    let p4 = p2 * p2;
    let p8 = p4 * p4;
    p8 * p8
}

// ============================================================================
// DELTA ENERGY COMPUTATION — SIMD-VECTORIZABLE KERNEL
// ============================================================================

/// Computes delta energy for flipping variable `v` across all 64 replicas.
///
/// Architecture: All inner loops iterate over contiguous i8 arrays and
/// accumulate into a contiguous f64[64] array. No bit shifts, no branches,
/// no function calls inside the inner loop. LLVM emits AVX2 packed instructions.
#[inline(always)]
#[allow(clippy::needless_range_loop)]
pub fn calculate_delta_e_local(
    model: &FlatHuboModel,
    spins: &[i8],
    num_vars: usize,
    v: usize,
    s: usize,
) -> [f64; NUM_REPLICAS] {
    let mut delta = [0.0f64; NUM_REPLICAS];
    let base_v = (v + num_vars * s) * NUM_REPLICAS;

    // --- Linear term ---
    // delta[r] = w_lin * (1.0 - 2.0 * spin_v[r])
    // Compiles to: vmovd + vpmovsxbd + vcvtdq2pd + vfmadd231pd
    let w_lin = model.linear[v];
    for r in 0..NUM_REPLICAS {
        let state = spins[base_v + r] as f64;
        delta[r] = w_lin * (1.0 - 2.0 * state);
    }

    // --- Edge2 terms (CSR traversal) ---
    // Product basis: Δ(w·x_v·x_j) for flipping x_v is w·x_j·(1 − 2·x_v).
    // Inner loop is pure contiguous i8 arithmetic + one convert — vectorizable.
    for idx in model.edge2_offsets[v]..model.edge2_offsets[v + 1] {
        let j = model.edge2_targets[idx];
        let w = model.edge2_weights[idx];
        let base_j = (j + num_vars * s) * NUM_REPLICAS;
        for r in 0..NUM_REPLICAS {
            let t = spins[base_j + r] * (1 - 2 * spins[base_v + r]);
            delta[r] += w * (t as f64);
        }
    }

    // --- Edge3 terms (CSR traversal) ---
    for idx in model.edge3_offsets[v]..model.edge3_offsets[v + 1] {
        let j = model.edge3_j[idx];
        let k = model.edge3_k[idx];
        let w = model.edge3_weights[idx];
        let base_j = (j + num_vars * s) * NUM_REPLICAS;
        let base_k = (k + num_vars * s) * NUM_REPLICAS;
        for r in 0..NUM_REPLICAS {
            // Δ(w·x_v·x_j·x_k) = w·x_j·x_k·(1 − 2·x_v)
            let t = (spins[base_j + r] & spins[base_k + r]) * (1 - 2 * spins[base_v + r]);
            delta[r] += w * (t as f64);
        }
    }

    // --- Edge4 terms (CSR traversal) ---
    for idx in model.edge4_offsets[v]..model.edge4_offsets[v + 1] {
        let j = model.edge4_j[idx];
        let k = model.edge4_k[idx];
        let l = model.edge4_l[idx];
        let w = model.edge4_weights[idx];
        let base_j = (j + num_vars * s) * NUM_REPLICAS;
        let base_k = (k + num_vars * s) * NUM_REPLICAS;
        let base_l = (l + num_vars * s) * NUM_REPLICAS;
        for r in 0..NUM_REPLICAS {
            // Δ(w·x_v·x_j·x_k·x_l) = w·x_j·x_k·x_l·(1 − 2·x_v)
            let t = (spins[base_j + r] & spins[base_k + r] & spins[base_l + r])
                * (1 - 2 * spins[base_v + r]);
            delta[r] += w * (t as f64);
        }
    }

    delta
}

/// Wrapper: computes delta energy using QuantumField indexing.
#[inline(always)]
pub fn calculate_delta_e(
    model: &FlatHuboModel,
    field: &QuantumField,
    v: usize,
    s: usize,
    t: usize,
    p: usize,
) -> [f64; NUM_REPLICAS] {
    let cell_idx = t + field.num_temps * p;
    let chunk_size = field.num_vars * field.num_slices * NUM_REPLICAS;
    let start = cell_idx * chunk_size;
    let end = start + chunk_size;
    calculate_delta_e_local(model, &field.spins[start..end], field.num_vars, v, s)
}

// ============================================================================
// REPLICA ENERGIES — FULL ENERGY CALCULATION
// ============================================================================

/// Computes total energy per replica for a single cell.
/// Used for initial energy computation and periodic re-sync.
#[inline(always)]
#[allow(clippy::needless_range_loop)]
pub fn calculate_replica_energies_local(
    model: &FlatHuboModel,
    spins: &[i8],
    num_vars: usize,
    num_slices: usize,
    j_tau: f64,
) -> [f64; NUM_REPLICAS] {
    // Suzuki–Trotter coupling acts on a ring of L slices; at L = 1 the ring
    // degenerates to a self-coupling (a constant), which must not enter the
    // Hamiltonian at all.
    let j_tau = if num_slices > 1 { j_tau } else { 0.0 };
    let mut energies = [0.0f64; NUM_REPLICAS];

    for s in 0..num_slices {
        for v in 0..num_vars {
            let base_v = (v + num_vars * s) * NUM_REPLICAS;

            // Linear contribution
            let w_lin = model.linear[v];
            for r in 0..NUM_REPLICAS {
                energies[r] += w_lin * (spins[base_v + r] as f64);
            }

            // Edge2 (only count each edge once: v < j)
            for idx in model.edge2_offsets[v]..model.edge2_offsets[v + 1] {
                let j = model.edge2_targets[idx];
                if v < j {
                    let w = model.edge2_weights[idx];
                    let base_j = (j + num_vars * s) * NUM_REPLICAS;
                    for r in 0..NUM_REPLICAS {
                        // Product basis: w·x_v·x_j
                        let t = (spins[base_v + r] & spins[base_j + r]) as f64;
                        energies[r] += w * t;
                    }
                }
            }

            // Edge3 (only count each edge once: v < j && v < k)
            for idx in model.edge3_offsets[v]..model.edge3_offsets[v + 1] {
                let j = model.edge3_j[idx];
                let k = model.edge3_k[idx];
                if v < j && v < k {
                    let w = model.edge3_weights[idx];
                    let base_j = (j + num_vars * s) * NUM_REPLICAS;
                    let base_k = (k + num_vars * s) * NUM_REPLICAS;
                    for r in 0..NUM_REPLICAS {
                        // Product basis: w·x_v·x_j·x_k
                        let t = (spins[base_v + r] & spins[base_j + r] & spins[base_k + r]) as f64;
                        energies[r] += w * t;
                    }
                }
            }

            // Edge4 (only count each edge once: v < j && v < k && v < l)
            for idx in model.edge4_offsets[v]..model.edge4_offsets[v + 1] {
                let j = model.edge4_j[idx];
                let k = model.edge4_k[idx];
                let l = model.edge4_l[idx];
                if v < j && v < k && v < l {
                    let w = model.edge4_weights[idx];
                    let base_j = (j + num_vars * s) * NUM_REPLICAS;
                    let base_k = (k + num_vars * s) * NUM_REPLICAS;
                    let base_l = (l + num_vars * s) * NUM_REPLICAS;
                    for r in 0..NUM_REPLICAS {
                        // Product basis: w·x_v·x_j·x_k·x_l
                        let t = (spins[base_v + r]
                            & spins[base_j + r]
                            & spins[base_k + r]
                            & spins[base_l + r]) as f64;
                        energies[r] += w * t;
                    }
                }
            }

            // Quantum Trotter coupling
            let s_next = if s == num_slices - 1 { 0 } else { s + 1 };
            let base_next = (v + num_vars * s_next) * NUM_REPLICAS;
            for r in 0..NUM_REPLICAS {
                let parity = (spins[base_v + r] ^ spins[base_next + r]) as f64;
                energies[r] += j_tau * (1.0 - parity);
            }
        }
    }

    energies
}

/// Wrapper: computes replica energies using QuantumField indexing.
#[inline(always)]
pub fn calculate_replica_energies(
    model: &FlatHuboModel,
    field: &QuantumField,
    t: usize,
    p: usize,
    j_tau: f64,
) -> [f64; NUM_REPLICAS] {
    let cell_idx = t + field.num_temps * p;
    let chunk_size = field.num_vars * field.num_slices * NUM_REPLICAS;
    let start = cell_idx * chunk_size;
    let end = start + chunk_size;
    calculate_replica_energies_local(
        model,
        &field.spins[start..end],
        field.num_vars,
        field.num_slices,
        j_tau,
    )
}

// ============================================================================
// MONTE CARLO STEP — SIMD-FIRST DESIGN
// ============================================================================

/// Reusable scratch for `step()`, allocated once per solve.
///
/// Eliminates the per-step heap allocations in the hot path. Every region is
/// fully overwritten before it is read within a step, so reuse is
/// numerically identical to fresh buffers. (The former per-cell RNG buffer —
/// 8 bytes per spin-site per step, the sweep's dominant memory traffic — is
/// gone: acceptance randomness is now generated just-in-time per replica
/// block, producing the identical value stream; see `next_f64x64`.)
pub struct StepScratch {
    /// PT swap attempts per adjacent temperature pair (aggregated over
    /// populations and steps since the last reset). Purely observational:
    /// counting does not alter RNG consumption or swap decisions.
    swap_attempts: Vec<u64>,
    /// PT swap acceptances per adjacent temperature pair.
    swap_accepts: Vec<u64>,
    /// Statistics collection flag — off by default; enabled for ladder
    /// tuning via `enable_swap_stats`.
    collect_swap_stats: bool,
    /// DEO phase: alternates every step between even pairs (0,1),(2,3),…
    /// and odd pairs (1,2),(3,4),… — the non-reversible deterministic
    /// even-odd replica-exchange scheme (Okabe et al., Chem. Phys. Lett.
    /// 335, 435 (2001)), proven to dominate reversible sweeps in round-trip
    /// rate (Syed, Bouchard-Côté, Deligiannidis & Doucet, JRSS-B 84, 321
    /// (2022)).
    deo_odd_phase: bool,
    /// Round-trip tracking (opt-in, for feedback-ladder tuning). Each replica
    /// carries a direction label that follows its configuration through the
    /// physical swaps: -1 = last touched the hot end, +1 = last touched the
    /// cold end, 0 = neither yet. This gives persistent replica identity for
    /// flow estimation WITHOUT logical label-swapping (see the round-trip
    /// method group). Off by default → zero hot-path cost.
    track_roundtrips: bool,
    /// Per-cell direction labels (same index layout as `energies`).
    labels: Vec<[i8; NUM_REPLICAS]>,
    /// Accumulated up/down occupancy per temperature (for flow f(T)).
    rt_n_up: Vec<u64>,
    rt_n_down: Vec<u64>,
    /// Completed round trips (a cold-labeled replica returning to the hot end).
    rt_count: u64,
}

impl StepScratch {
    pub fn for_field(field: &QuantumField) -> Self {
        let num_pairs = field.num_temps.saturating_sub(1);
        Self {
            swap_attempts: vec![0; num_pairs],
            swap_accepts: vec![0; num_pairs],
            collect_swap_stats: false,
            deo_odd_phase: false,
            track_roundtrips: false,
            labels: vec![[0i8; NUM_REPLICAS]; field.num_temps * field.num_pops],
            rt_n_up: vec![0; field.num_temps],
            rt_n_down: vec![0; field.num_temps],
            rt_count: 0,
        }
    }

    /// Enables round-trip / replica-flow tracking (feedback-ladder tuning).
    pub fn enable_roundtrip_tracking(&mut self) {
        self.track_roundtrips = true;
    }

    /// Fraction of up-labeled replicas per temperature, f(T) = n_up/(n_up+n_down)
    /// (Katzgraber, Trebst, Huse & Troyer, JSTAT P03018 (2006)).
    pub fn roundtrip_flow(&self) -> Vec<f64> {
        self.rt_n_up
            .iter()
            .zip(&self.rt_n_down)
            .map(|(&u, &d)| {
                if u + d == 0 {
                    0.0
                } else {
                    u as f64 / (u + d) as f64
                }
            })
            .collect()
    }

    /// Number of completed round trips since the last reset.
    pub fn roundtrip_count(&self) -> u64 {
        self.rt_count
    }

    pub fn reset_roundtrip_stats(&mut self) {
        self.rt_n_up.iter_mut().for_each(|x| *x = 0);
        self.rt_n_down.iter_mut().for_each(|x| *x = 0);
        self.rt_count = 0;
        for l in self.labels.iter_mut() {
            *l = [0i8; NUM_REPLICAS];
        }
    }

    /// Enables per-pair swap-statistics collection (used by ladder tuning).
    pub fn enable_swap_stats(&mut self) {
        self.collect_swap_stats = true;
    }

    /// Swap-acceptance rate per adjacent temperature pair, aggregated over
    /// populations and over all steps since the last `reset_swap_stats`.
    pub fn swap_acceptance_rates(&self) -> Vec<f64> {
        self.swap_accepts
            .iter()
            .zip(&self.swap_attempts)
            .map(|(&a, &n)| if n == 0 { 0.0 } else { a as f64 / n as f64 })
            .collect()
    }

    pub fn reset_swap_stats(&mut self) {
        self.swap_attempts.iter_mut().for_each(|x| *x = 0);
        self.swap_accepts.iter_mut().for_each(|x| *x = 0);
    }
}

/// 5D Interaction Kernel: Single Monte Carlo Step
///
/// Architecture:
/// 1. Pre-generates ALL random values for the entire sweep (no RNG calls in hot loop)
/// 2. Delta energy computation uses contiguous memory layout (vectorizable)
/// 3. Metropolis acceptance uses branchless mask computation (vectorizable)
/// 4. Parallel Tempering uses byte-level swap (no bit manipulation)
///
/// `is_clamped` (length = num_vars) marks conditioned variables: Metropolis
/// proposals never touch them, which is the exact treatment of conditional
/// sampling π(x_free | x_clamped). Clamped spins must be initialized to the
/// same value in every slice/temperature/population cell; PT swaps then
/// preserve them automatically.
#[allow(clippy::needless_range_loop)]
pub fn step<R: Rng>(
    field: &mut QuantumField,
    model: &FlatHuboModel,
    temps: &[f64],
    j_tau: f64,
    is_clamped: &[bool],
    scratch: &mut StepScratch,
    rng: &mut R,
) {
    let num_vars = field.num_vars;
    let num_slices = field.num_slices;
    let num_temps = field.num_temps;
    let num_pops = field.num_pops;
    // At L = 1 the Trotter ring degenerates to a constant self-coupling whose
    // flip-delta is exactly zero; a nonzero j_tau here would inject a phantom
    // -2·j_tau into every Metropolis acceptance (see test_trotter_correctness).
    let j_tau = if num_slices > 1 { j_tau } else { 0.0 };
    assert_eq!(
        is_clamped.len(),
        num_vars,
        "is_clamped must have one entry per variable"
    );

    let base_seed = rng.gen::<u64>();
    let chunk_size = num_vars * num_slices * NUM_REPLICAS;
    assert_eq!(scratch.swap_attempts.len(), num_temps.saturating_sub(1));

    // ========================================================================
    // PHASE 1: Parallel Classical + Trotter Sweeps
    // ========================================================================
    field
        .spins
        .par_chunks_mut(chunk_size)
        .zip(field.energies.par_chunks_mut(1))
        .enumerate()
        .for_each(|(cell_idx, (spins_chunk, energy_slot))| {
            let t = cell_idx % num_temps;
            let beta = 1.0 / temps[t];
            let mut cell_rng = Xoshiro256PlusPlusX8::new(base_seed.wrapping_add(cell_idx as u64));

            // Acceptance randomness is generated JUST-IN-TIME per replica
            // block (Fix C, Rank 2). The old design pre-filled a buffer of
            // one f64 per spin-site per step — 8 bytes of RNG round-tripped
            // through memory per 1 byte of spin state, the sweep's dominant
            // memory traffic. SAFETY (bit-identical values): consumption
            // order (s asc, v asc, r asc) equals the old linear fill order,
            // NUM_REPLICAS = 64 = 8×8 exactly matches fill_f64's 8-lane
            // blocking with no remainder, and clamped variables still draw
            // (and discard) their block so the stream stays aligned — every
            // consumed value is identical to the buffered design's.
            for s in 0..num_slices {
                for v in 0..num_vars {
                    let lane_rng = cell_rng.next_f64x64();

                    // Conditioned variables are excluded from proposals.
                    // Branch is per-variable, outside the replica lanes —
                    // the vectorized inner loops are unaffected.
                    if is_clamped[v] {
                        continue;
                    }

                    // --- Compute classical delta energy (VECTORIZED) ---
                    let delta_classical =
                        calculate_delta_e_local(model, spins_chunk, num_vars, v, s);

                    // --- Acceptance (VECTORIZED) ---
                    let base_v = (v + num_vars * s) * NUM_REPLICAS;

                    if j_tau == 0.0 {
                        // FAST PATH (production default: num_slices == 1 forces
                        // j_tau = 0 above). The Trotter term is identically
                        // ±0.0 here, so it is dropped along with its 2 byte
                        // loads, 2 XORs, 2 int→f64 converts, and 6 FLOPs per
                        // lane. SAFETY (bit-identical decisions): the only
                        // arithmetic difference vs. adding delta_q = ±0.0 is
                        // the SIGN of a zero total_delta. IEEE-754 makes
                        // -0.0 <= 0.0, -0.0 >= 0.0, and x + (-0.0) == x + 0.0
                        // for the accept test, the PT swap test, and the
                        // energy accumulation alike, so no decision or tracked
                        // value can differ (golden regression enforces this).
                        for r in 0..NUM_REPLICAS {
                            let total_delta = delta_classical[r];
                            let exp_val = fast_exp(-total_delta * beta);
                            let accept = (total_delta <= 0.0) | (lane_rng[r] < exp_val);
                            let flip = accept as i8;
                            spins_chunk[base_v + r] ^= flip;
                            energy_slot[0][r] += total_delta * (flip as f64);
                        }
                    } else {
                        // Trotter path (num_slices > 1): original loop.
                        let s_prev = if s == 0 { num_slices - 1 } else { s - 1 };
                        let s_next = if s == num_slices - 1 { 0 } else { s + 1 };
                        let base_prev = (v + num_vars * s_prev) * NUM_REPLICAS;
                        let base_next = (v + num_vars * s_next) * NUM_REPLICAS;

                        // SIMD-friendly inner loop: contiguous loads, FMA, branchless accept
                        for r in 0..NUM_REPLICAS {
                            // Trotter parity (contiguous byte loads)
                            let pp = (spins_chunk[base_v + r] ^ spins_chunk[base_prev + r]) as f64;
                            let pn = (spins_chunk[base_v + r] ^ spins_chunk[base_next + r]) as f64;
                            let delta_q = j_tau * (2.0 * pp - 1.0) + j_tau * (2.0 * pn - 1.0);

                            let total_delta = delta_classical[r] + delta_q;

                            // Branchless Metropolis acceptance:
                            // accept = (delta <= 0) OR (rand < exp(-delta * beta))
                            // Compiles to: vcmppd + vcmppd + vorpd
                            let exp_val = fast_exp(-total_delta * beta);
                            let accept = (total_delta <= 0.0) | (lane_rng[r] < exp_val);

                            // Branchless spin flip: XOR with 0 (no flip) or 1 (flip)
                            let flip = accept as i8;
                            spins_chunk[base_v + r] ^= flip;

                            // Branchless energy tracking: multiply by 0.0 or 1.0
                            energy_slot[0][r] += total_delta * (flip as f64);
                        }
                    }
                }
            }
        });

    // ========================================================================
    // PHASE 2: Parallel Tempering — Replica Exchange
    // ========================================================================
    // Non-reversible DEO: even pairs on one step, odd pairs on the next.
    // Within a phase the pairs are disjoint, so the exchange is a valid
    // deterministic alternation (Syed et al. 2022); it replaces the previous
    // sequential same-order sweep, whose reversible dynamics diffuse
    // replicas through the ladder ~T times slower.
    let parity_start = usize::from(scratch.deo_odd_phase);
    for p in 0..num_pops {
        for t in (parity_start..num_temps.saturating_sub(1)).step_by(2) {
            let e1 = field.energies[t + num_temps * p];
            let e2 = field.energies[(t + 1) + num_temps * p];
            let beta1 = 1.0 / temps[t];
            let beta2 = 1.0 / temps[t + 1];
            let delta_beta = beta1 - beta2;

            // Compute swap mask: which replicas should swap between temp levels
            let mut swap_mask = [false; NUM_REPLICAS];
            for r in 0..NUM_REPLICAS {
                let delta_e = e1[r] - e2[r];
                let exponent = delta_beta * delta_e;
                swap_mask[r] = exponent >= 0.0 || rng.gen::<f64>() < exponent.exp();
            }
            // Apply swaps using byte-level operations (no bit manipulation)
            let any_swap = swap_mask.iter().any(|&s| s);
            // Swap statistics are opt-in (ladder tuning only): the default
            // path pays one untaken, predictable branch per pair and keeps
            // the original scan byte-for-byte.
            if scratch.collect_swap_stats {
                let accepted = swap_mask.iter().filter(|&&s| s).count() as u64;
                scratch.swap_attempts[t] += NUM_REPLICAS as u64;
                scratch.swap_accepts[t] += accepted;
            }
            if any_swap {
                let cell_len = num_vars * num_slices * NUM_REPLICAS;
                let base1_cell = (t + num_temps * p) * cell_len;
                let base2_cell = ((t + 1) + num_temps * p) * cell_len;

                // Branchless masked lane-group exchange (Fix B, Rank 5).
                // The old per-lane `Vec::swap` took an unpredictable branch
                // per replica and touched a 64-byte cache line per useful
                // byte. XOR-exchange with a per-lane 0x00/0xFF mask produces
                // byte-for-byte the same result (swap(a,b) ≡ a^=d, b^=d with
                // d = a^b), is branch-free, auto-vectorizes, and streams both
                // cells' lines at full utilization. Trajectories unchanged.
                let mut lane_mask = [0i8; NUM_REPLICAS];
                for r in 0..NUM_REPLICAS {
                    lane_mask[r] = (swap_mask[r] as i8).wrapping_neg(); // 0x00 / 0xFF
                }
                // Cells t and t+1 are adjacent in the flat layout, so a
                // single split yields disjoint &mut regions.
                let (left, right) = field.spins.split_at_mut(base2_cell);
                let cell1 = &mut left[base1_cell..base1_cell + cell_len];
                let cell2 = &mut right[..cell_len];
                for (block1, block2) in cell1
                    .chunks_exact_mut(NUM_REPLICAS)
                    .zip(cell2.chunks_exact_mut(NUM_REPLICAS))
                {
                    for r in 0..NUM_REPLICAS {
                        let d = (block1[r] ^ block2[r]) & lane_mask[r];
                        block1[r] ^= d;
                        block2[r] ^= d;
                    }
                }

                // Swap tracked energy values (and direction labels, if
                // tracking) so replica identity follows its configuration.
                let idx1 = t + num_temps * p;
                let idx2 = (t + 1) + num_temps * p;
                for r in 0..NUM_REPLICAS {
                    if swap_mask[r] {
                        let tmp = field.energies[idx1][r];
                        field.energies[idx1][r] = field.energies[idx2][r];
                        field.energies[idx2][r] = tmp;
                        if scratch.track_roundtrips {
                            let lt = scratch.labels[idx1][r];
                            scratch.labels[idx1][r] = scratch.labels[idx2][r];
                            scratch.labels[idx2][r] = lt;
                        }
                    }
                }
            }
        }
    }

    // Round-trip bookkeeping: relabel the extremes for the CURRENT
    // occupancy, count completed round trips (a cold-labeled replica arriving
    // back at the hot end), and accumulate the up/down occupancy histogram.
    if scratch.track_roundtrips && num_temps >= 2 {
        for p in 0..num_pops {
            let hot = num_temps * p; // t = 0 (T_max)
            let cold = (num_temps - 1) + num_temps * p; // t = nt-1 (T_min)
            for r in 0..NUM_REPLICAS {
                if scratch.labels[hot][r] == 1 {
                    scratch.rt_count += 1; // up replica returned to hot end
                }
                scratch.labels[hot][r] = -1; // now heading down
                scratch.labels[cold][r] = 1; // cold end: heading up
            }
            for t in 0..num_temps {
                let cell = t + num_temps * p;
                for r in 0..NUM_REPLICAS {
                    match scratch.labels[cell][r] {
                        1 => scratch.rt_n_up[t] += 1,
                        -1 => scratch.rt_n_down[t] += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    scratch.deo_odd_phase = !scratch.deo_odd_phase;
}
