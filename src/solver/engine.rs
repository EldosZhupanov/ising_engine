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
    let p = 1.0 + r + 0.5 * r2 + (1.0 / 6.0) * r3
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
    // Each edge: load spin_j[r], XOR with spin_v[r], convert, FMA
    // Inner loop is pure contiguous arithmetic — vectorizable.
    for idx in model.edge2_offsets[v]..model.edge2_offsets[v + 1] {
        let j = model.edge2_targets[idx];
        let w = model.edge2_weights[idx];
        let base_j = (j + num_vars * s) * NUM_REPLICAS;
        for r in 0..NUM_REPLICAS {
            let parity = (spins[base_v + r] ^ spins[base_j + r]) as f64;
            delta[r] += w * (2.0 * parity - 1.0);
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
            let parity = (spins[base_v + r] ^ spins[base_j + r] ^ spins[base_k + r]) as f64;
            delta[r] += w * (2.0 * parity - 1.0);
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
            let parity =
                (spins[base_v + r] ^ spins[base_j + r] ^ spins[base_k + r] ^ spins[base_l + r])
                    as f64;
            delta[r] += w * (2.0 * parity - 1.0);
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
                        let parity = (spins[base_v + r] ^ spins[base_j + r]) as f64;
                        energies[r] += w * (1.0 - parity);
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
                        let parity =
                            (spins[base_v + r] ^ spins[base_j + r] ^ spins[base_k + r]) as f64;
                        energies[r] += w * (1.0 - parity);
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
                        let parity = (spins[base_v + r]
                            ^ spins[base_j + r]
                            ^ spins[base_k + r]
                            ^ spins[base_l + r]) as f64;
                        energies[r] += w * (1.0 - parity);
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

/// 5D Interaction Kernel: Single Monte Carlo Step
///
/// Architecture:
/// 1. Pre-generates ALL random values for the entire sweep (no RNG calls in hot loop)
/// 2. Delta energy computation uses contiguous memory layout (vectorizable)
/// 3. Metropolis acceptance uses branchless mask computation (vectorizable)
/// 4. Parallel Tempering uses byte-level swap (no bit manipulation)
#[allow(clippy::needless_range_loop)]
pub fn step<R: Rng>(
    field: &mut QuantumField,
    model: &FlatHuboModel,
    temps: &[f64],
    j_tau: f64,
    rng: &mut R,
) {
    let num_vars = field.num_vars;
    let num_slices = field.num_slices;
    let num_temps = field.num_temps;
    let num_pops = field.num_pops;

    let base_seed = rng.gen::<u64>();
    let chunk_size = num_vars * num_slices * NUM_REPLICAS;

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
            let mut cell_rng = Xoshiro256PlusPlus::new(base_seed.wrapping_add(cell_idx as u64));

            // Pre-generate ALL random values for entire sweep.
            // Hot loop only LOADS from this buffer — no RNG calls inside.
            let rng_total = num_vars * num_slices * NUM_REPLICAS;
            let mut rng_buf = vec![0.0f64; rng_total];
            cell_rng.fill_f64(&mut rng_buf);

            for s in 0..num_slices {
                for v in 0..num_vars {
                    // --- Compute classical delta energy (VECTORIZED) ---
                    let delta_classical =
                        calculate_delta_e_local(model, spins_chunk, num_vars, v, s);

                    // --- Compute Trotter delta + acceptance (VECTORIZED) ---
                    let base_v = (v + num_vars * s) * NUM_REPLICAS;
                    let s_prev = if s == 0 { num_slices - 1 } else { s - 1 };
                    let s_next = if s == num_slices - 1 { 0 } else { s + 1 };
                    let base_prev = (v + num_vars * s_prev) * NUM_REPLICAS;
                    let base_next = (v + num_vars * s_next) * NUM_REPLICAS;
                    let rng_offset = (v + num_vars * s) * NUM_REPLICAS;

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
                        let accept =
                            (total_delta <= 0.0) | (rng_buf[rng_offset + r] < exp_val);

                        // Branchless spin flip: XOR with 0 (no flip) or 1 (flip)
                        let flip = accept as i8;
                        spins_chunk[base_v + r] ^= flip;

                        // Branchless energy tracking: multiply by 0.0 or 1.0
                        energy_slot[0][r] += total_delta * (flip as f64);
                    }
                }
            }
        });

    // ========================================================================
    // PHASE 2: Parallel Tempering — Replica Exchange
    // ========================================================================
    for p in 0..num_pops {
        for t in 0..(num_temps - 1) {
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
            if any_swap {
                let base1_cell = (t + num_temps * p) * num_vars * num_slices * NUM_REPLICAS;
                let base2_cell =
                    ((t + 1) + num_temps * p) * num_vars * num_slices * NUM_REPLICAS;

                for s in 0..num_slices {
                    for v in 0..num_vars {
                        let offset = (v + num_vars * s) * NUM_REPLICAS;
                        for r in 0..NUM_REPLICAS {
                            if swap_mask[r] {
                                field
                                    .spins
                                    .swap(base1_cell + offset + r, base2_cell + offset + r);
                            }
                        }
                    }
                }

                // Swap tracked energy values
                let idx1 = t + num_temps * p;
                let idx2 = (t + 1) + num_temps * p;
                for r in 0..NUM_REPLICAS {
                    if swap_mask[r] {
                        let tmp = field.energies[idx1][r];
                        field.energies[idx1][r] = field.energies[idx2][r];
                        field.energies[idx2][r] = tmp;
                    }
                }
            }
        }
    }

    // ========================================================================
    // PHASE 3: Population Resampling
    // ========================================================================
    if num_pops > 1 {
        let t_target = 0;
        let mut min_e = [f64::MAX; NUM_REPLICAS];
        let mut max_e = [f64::MIN; NUM_REPLICAS];
        let mut best_p = [0usize; NUM_REPLICAS];
        let mut worst_p = [0usize; NUM_REPLICAS];

        for p in 0..num_pops {
            let energies = field.energies[t_target + num_temps * p];
            for r in 0..NUM_REPLICAS {
                if energies[r] < min_e[r] {
                    min_e[r] = energies[r];
                    best_p[r] = p;
                }
                if energies[r] > max_e[r] {
                    max_e[r] = energies[r];
                    worst_p[r] = p;
                }
            }
        }

        // Determine which replicas need overwriting in each population
        let mut overwrite_mask = vec![[false; NUM_REPLICAS]; num_pops];
        for r in 0..NUM_REPLICAS {
            let wp = worst_p[r];
            let bp = best_p[r];
            if wp != bp {
                overwrite_mask[wp][r] = true;
            }
        }

        // Flat traversal: copy best population's replica values to worst
        let stride = num_vars * num_slices * num_temps * NUM_REPLICAS;
        let var_stride = NUM_REPLICAS;
        for s in 0..num_slices {
            for v in 0..num_vars {
                let offset = (v + num_vars * s) * var_stride;
                // Gather best state for each replica
                let mut best_state = [0i8; NUM_REPLICAS];
                for r in 0..NUM_REPLICAS {
                    let bp = best_p[r];
                    let src_base = bp * stride + (t_target * num_vars * num_slices * NUM_REPLICAS);
                    best_state[r] = field.spins[src_base + offset + r];
                }

                // Overwrite worst populations
                for p in 0..num_pops {
                    let dst_base = p * stride + (t_target * num_vars * num_slices * NUM_REPLICAS);
                    for r in 0..NUM_REPLICAS {
                        if overwrite_mask[p][r] {
                            field.spins[dst_base + offset + r] = best_state[r];
                        }
                    }
                }
            }
        }
    }
}
