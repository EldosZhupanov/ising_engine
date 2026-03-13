use super::types::QuantumField;
use crate::core::hubo::HuboModel;
use rand::Rng;

/// Computes the bit-parallel energy delta for flipping variable `i` across 64 replicas.
/// A positive weight favors the state where parity == 0 (all spins align to +1 product).
#[inline(always)]
pub fn calculate_delta_e(
    model: &HuboModel,
    field: &QuantumField,
    v: usize, s: usize, t: usize, p: usize
) -> [f64; 64] {
    let mut delta = [0.0; 64];
    let spin_i = field.get(v, s, t, p);

    // Linear
    let w_lin = model.linear[v];
    for bit in 0..64 {
        let state_i = (spin_i >> bit) & 1;
        // If state is 1, flipping to 0 changes energy by -w_lin.
        // If state is 0, flipping to 1 changes energy by +w_lin.
        delta[bit] += if state_i == 1 { -w_lin } else { w_lin };
    }

    // Edge2 (2-body: i, j)
    for edge in &model.edges2[v] {
        let spin_j = field.get(edge.j, s, t, p);
        let parity = spin_i ^ spin_j;
        for bit in 0..64 {
            let p_bit = (parity >> bit) & 1;
            // Energy contribution = weight * (if parity_bit == 0 { 1.0 } else { -1.0 })
            // Delta is the change in this contribution if bit `i` is flipped.
            // Flipping `i` flips the parity.
            delta[bit] += if p_bit == 0 { -edge.weight } else { edge.weight };
        }
    }

    // Edge3 (3-body: i, j, k)
    for edge in &model.edges3[v] {
        let spin_j = field.get(edge.j, s, t, p);
        let spin_k = field.get(edge.k, s, t, p);
        let parity = spin_i ^ spin_j ^ spin_k;
        for bit in 0..64 {
            let p_bit = (parity >> bit) & 1;
            delta[bit] += if p_bit == 0 { -edge.weight } else { edge.weight };
        }
    }

    // Edge4 (4-body: i, j, k, l)
    for edge in &model.edges4[v] {
        let spin_j = field.get(edge.j, s, t, p);
        let spin_k = field.get(edge.k, s, t, p);
        let spin_l = field.get(edge.l, s, t, p);
        let parity = spin_i ^ spin_j ^ spin_k ^ spin_l;
        for bit in 0..64 {
            let p_bit = (parity >> bit) & 1;
            delta[bit] += if p_bit == 0 { -edge.weight } else { edge.weight };
        }
    }

    delta
}

#[inline(always)]
pub fn calculate_replica_energies(
    model: &HuboModel,
    field: &QuantumField,
    t: usize,
    p: usize,
    j_tau: f64,
) -> [f64; 64] {
    let mut energies = [0.0; 64];
    let num_slices = field.num_slices;
    let num_vars = field.num_vars;

    for s in 0..num_slices {
        for v in 0..num_vars {
            let spin_v = field.get(v, s, t, p);

            // Linear
            let w_lin = model.linear[v];
            for bit in 0..64 {
                if ((spin_v >> bit) & 1) == 1 {
                    energies[bit] += w_lin;
                }
            }

            // Edge2
            for edge in &model.edges2[v] {
                if v < edge.j {
                    let spin_j = field.get(edge.j, s, t, p);
                    let parity = spin_v ^ spin_j;
                    for bit in 0..64 {
                        if ((parity >> bit) & 1) == 0 {
                            energies[bit] += edge.weight;
                        }
                    }
                }
            }

            // Edge3
            for edge in &model.edges3[v] {
                if v < edge.j && v < edge.k {
                    let spin_j = field.get(edge.j, s, t, p);
                    let spin_k = field.get(edge.k, s, t, p);
                    let parity = spin_v ^ spin_j ^ spin_k;
                    for bit in 0..64 {
                        if ((parity >> bit) & 1) == 0 {
                            energies[bit] += edge.weight;
                        }
                    }
                }
            }

            // Edge4
            for edge in &model.edges4[v] {
                if v < edge.j && v < edge.k && v < edge.l {
                    let spin_j = field.get(edge.j, s, t, p);
                    let spin_k = field.get(edge.k, s, t, p);
                    let spin_l = field.get(edge.l, s, t, p);
                    let parity = spin_v ^ spin_j ^ spin_k ^ spin_l;
                    for bit in 0..64 {
                        if ((parity >> bit) & 1) == 0 {
                            energies[bit] += edge.weight;
                        }
                    }
                }
            }

            // Quantum Trotter Energy
            let s_next = if s == num_slices - 1 { 0 } else { s + 1 };
            let spin_next = field.get(v, s_next, t, p);
            let parity_q = spin_v ^ spin_next;
            for bit in 0..64 {
                if ((parity_q >> bit) & 1) == 0 {
                    energies[bit] += j_tau;
                }
            }
        }
    }

    energies
}


/// 5D Interaction Kernel: Single Monte Carlo Step
pub fn step<R: Rng>(
    field: &mut QuantumField,
    model: &HuboModel,
    temps: &[f64],
    j_tau: f64,
    rng: &mut R
) {
    let num_vars = field.num_vars;
    let num_slices = field.num_slices;
    let num_temps = field.num_temps;
    let num_pops = field.num_pops;

    // 1. Classical Ising Update (D1 contiguous sweep)
    for p in 0..num_pops {
        for t in 0..num_temps {
            let beta = 1.0 / temps[t];
            for s in 0..num_slices {
                for v in 0..num_vars {
                    let delta_classical = calculate_delta_e(model, field, v, s, t, p);
                    
                    // Quantum Trotter Update: interaction with slice `s-1` and `s+1`
                    let spin_i = field.get(v, s, t, p);
                    let s_prev = if s == 0 { num_slices - 1 } else { s - 1 };
                    let s_next = if s == num_slices - 1 { 0 } else { s + 1 };
                    let spin_i_prev = field.get(v, s_prev, t, p);
                    let spin_i_next = field.get(v, s_next, t, p);

                    let parity_prev = spin_i ^ spin_i_prev;
                    let parity_next = spin_i ^ spin_i_next;

                    let mut flip_mask = 0u64;

                    for bit in 0..64 {
                        let p_prev = (parity_prev >> bit) & 1;
                        let p_next = (parity_next >> bit) & 1;

                        // Quantum energy change from flipping `i`
                        let mut delta_q = 0.0;
                        delta_q += if p_prev == 0 { -j_tau } else { j_tau };
                        delta_q += if p_next == 0 { -j_tau } else { j_tau };

                        let total_delta = delta_classical[bit] + delta_q;

                        // Metropolis Acceptance
                        if total_delta <= 0.0 || rng.gen::<f64>() < (-total_delta * beta).exp() {
                            flip_mask |= 1 << bit;
                        }
                    }

                    // Apply flips
                    let current_val = field.get(v, s, t, p);
                    *field.get_mut(v, s, t, p) = current_val ^ flip_mask;
                }
            }
        }
    }

    // 2. Parallel Tempering (Swap temps)
    for p in 0..num_pops {
        for t in 0..(num_temps - 1) {
            let e1 = calculate_replica_energies(model, field, t, p, j_tau);
            let e2 = calculate_replica_energies(model, field, t + 1, p, j_tau);
            let beta1 = 1.0 / temps[t];
            let beta2 = 1.0 / temps[t + 1];

            let mut swap_mask = 0u64;
            for bit in 0..64 {
                let delta_beta = beta1 - beta2;
                let delta_e = e1[bit] - e2[bit];
                let exponent = delta_beta * delta_e;
                
                if exponent >= 0.0 || rng.gen::<f64>() < exponent.exp() {
                    swap_mask |= 1 << bit;
                }
            }

            if swap_mask != 0 {
                for s in 0..num_slices {
                    for v in 0..num_vars {
                        let spin_t0 = field.get(v, s, t, p);
                        let spin_t1 = field.get(v, s, t + 1, p);
                        let new_t0 = (spin_t0 & !swap_mask) | (spin_t1 & swap_mask);
                        let new_t1 = (spin_t1 & !swap_mask) | (spin_t0 & swap_mask);
                        *field.get_mut(v, s, t, p) = new_t0;
                        *field.get_mut(v, s, t + 1, p) = new_t1;
                    }
                }
            }
        }
    }

    // 3. Population Resampling
    if num_pops > 1 {
        let t_target = 0;
        let mut min_e = [f64::MAX; 64];
        let mut max_e = [f64::MIN; 64];
        let mut best_p = [0; 64];
        let mut worst_p = [0; 64];

        for p in 0..num_pops {
            let energies = calculate_replica_energies(model, field, t_target, p, j_tau);
            for bit in 0..64 {
                if energies[bit] < min_e[bit] {
                    min_e[bit] = energies[bit];
                    best_p[bit] = p;
                }
                if energies[bit] > max_e[bit] {
                    max_e[bit] = energies[bit];
                    worst_p[bit] = p;
                }
            }
        }

        let mut overwrite_masks = vec![0u64; num_pops];
        for bit in 0..64 {
            let wp = worst_p[bit];
            let bp = best_p[bit];
            if wp != bp {
                overwrite_masks[wp] |= 1 << bit;
            }
        }

        for t in 0..num_temps {
            for s in 0..num_slices {
                for v in 0..num_vars {
                    let mut best_state = 0u64;
                    for bit in 0..64 {
                        let bp = best_p[bit];
                        best_state |= ((field.get(v, s, t, bp) >> bit) & 1) << bit;
                    }

                    for p in 0..num_pops {
                        let mask = overwrite_masks[p];
                        if mask != 0 {
                            let curr = field.get(v, s, t, p);
                            let merged = (curr & !mask) | (best_state & mask);
                            *field.get_mut(v, s, t, p) = merged;
                        }
                    }
                }
            }
        }
    }
}
