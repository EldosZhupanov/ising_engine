use ising_engine::compiler::LogicBuilder;
use ising_engine::core::{CsrMatrix, QuboModel};

/// Helper: compute delta-E by brute force (flip, recompute, compare).
fn brute_force_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
    let e_before = model.calculate_total_energy(state);
    let mut flipped = state.to_vec();
    flipped[var_idx] = 1 - flipped[var_idx];
    let e_after = model.calculate_total_energy(&flipped);
    e_after - e_before
}

/// Incremental delta-E (same formula as the solver).
fn incremental_delta_e(model: &QuboModel, state: &[i8], var_idx: usize) -> f64 {
    let current_val = state[var_idx] as f64;
    let flip_multiplier = 1.0 - 2.0 * current_val;
    let mut sum_j = 0.0;
    for (col, weight) in model.quadratic.get_row(var_idx) {
        sum_j += weight * (state[col] as f64);
    }
    flip_multiplier * (model.linear[var_idx] + sum_j)
}

// ========================================================================
// Energy correctness tests
// ========================================================================

#[test]
fn test_empty_model_energy() {
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 3,
        linear: vec![0.0; 3],
        quadratic: CsrMatrix::empty(3),
    };
    let state = vec![1, 0, 1];
    assert!((model.calculate_total_energy(&state)).abs() < 1e-10);
}

#[test]
fn test_linear_only_energy() {
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 3,
        linear: vec![2.0, -3.0, 5.0],
        quadratic: CsrMatrix::empty(3),
    };
    // x = [1, 0, 1] → E = 2*1 + (-3)*0 + 5*1 = 7
    assert!((model.calculate_total_energy(&[1, 0, 1]) - 7.0).abs() < 1e-10);
    // x = [0, 1, 0] → E = -3
    assert!((model.calculate_total_energy(&[0, 1, 0]) - (-3.0)).abs() < 1e-10);
}

#[test]
fn test_and_gate_energy() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_and_gate(a, b, z);
    let model = builder.build();

    // Correct: Z = A AND B → energy = 0
    assert!((model.calculate_total_energy(&[0, 0, 0])).abs() < 1e-10);
    assert!((model.calculate_total_energy(&[1, 0, 0])).abs() < 1e-10);
    assert!((model.calculate_total_energy(&[0, 1, 0])).abs() < 1e-10);
    assert!((model.calculate_total_energy(&[1, 1, 1])).abs() < 1e-10);

    // Incorrect: energy > 0
    assert!(model.calculate_total_energy(&[0, 0, 1]) > 0.0);
    assert!(model.calculate_total_energy(&[1, 0, 1]) > 0.0);
    assert!(model.calculate_total_energy(&[0, 1, 1]) > 0.0);
    assert!(model.calculate_total_energy(&[1, 1, 0]) > 0.0);
}

// ========================================================================
// Delta-E consistency tests
// ========================================================================

#[test]
fn test_delta_e_matches_bruteforce_and_gate() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_and_gate(a, b, z);
    let model = builder.build();

    let states: Vec<Vec<i8>> = vec![
        vec![0, 0, 0],
        vec![1, 0, 0],
        vec![0, 1, 0],
        vec![1, 1, 1],
        vec![0, 0, 1],
        vec![1, 1, 0],
    ];

    for state in &states {
        for var_idx in [a, b, z] {
            let brute = brute_force_delta_e(&model, state, var_idx);
            let incremental = incremental_delta_e(&model, state, var_idx);
            assert!(
                (brute - incremental).abs() < 1e-10,
                "delta-E mismatch for state {:?}, flip var {}: brute={}, incremental={}",
                state,
                var_idx,
                brute,
                incremental
            );
        }
    }
}

#[test]
fn test_delta_e_matches_bruteforce_xor_gate() {
    let mut builder = LogicBuilder::new();
    let x = builder.add_var();
    let y = builder.add_var();
    let z = builder.add_var();
    let w = builder.add_var();
    builder.add_xor_gate(x, y, z, w);
    let model = builder.build();

    // Test all 16 states
    for bits in 0..16u8 {
        let state = vec![
            (bits & 1) as i8,
            ((bits >> 1) & 1) as i8,
            ((bits >> 2) & 1) as i8,
            ((bits >> 3) & 1) as i8,
        ];
        for var_idx in 0..4 {
            let brute = brute_force_delta_e(&model, &state, var_idx);
            let incremental = incremental_delta_e(&model, &state, var_idx);
            assert!(
                (brute - incremental).abs() < 1e-10,
                "delta-E mismatch for XOR state {:?}, flip var {}: brute={}, incremental={}",
                state,
                var_idx,
                brute,
                incremental
            );
        }
    }
}

#[test]
fn test_sk_energy_exhaustive_equivalence() {
    // Exact verification of the mathematical equivalence between the physical SK Hamiltonian
    // H = - sum_{i < j} J_ij sigma_i sigma_j  (sigma in {-1, +1})
    // and the mapped QuboModel evaluated via QuboModel::calculate_total_energy.
    let n = 10;
    let mut rng = rand::thread_rng();
    use rand::Rng;

    let mut j_mat = vec![vec![0.0f64; n]; n];
    let mut sum_j_all = 0.0;
    for i in 0..n {
        for j in (i + 1)..n {
            let j_val = rng.gen_range(-1.5..1.5);
            j_mat[i][j] = j_val;
            j_mat[j][i] = j_val;
            sum_j_all += j_val;
        }
    }

    let energy_offset = -sum_j_all;
    let mut linear = vec![0.0f64; n];
    for i in 0..n {
        let mut row_sum = 0.0;
        for j in 0..n {
            if i != j {
                row_sum += j_mat[i][j];
            }
        }
        linear[i] = 2.0 * row_sum;
    }

    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                col_indices.push(j);
                values.push(-4.0 * j_mat[i][j]);
            }
        }
        row_offsets.push(col_indices.len());
    }

    let model = QuboModel {
        energy_offset,
        num_vars: n,
        linear,
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };

    // Exhaustive test across all 2^10 = 1024 states
    for state_idx in 0..(1 << n) {
        let x: Vec<i8> = (0..n).map(|bit| ((state_idx >> bit) & 1) as i8).collect();
        let sigma: Vec<f64> = x
            .iter()
            .map(|&bit| if bit == 0 { 1.0 } else { -1.0 })
            .collect();

        // 1. Direct SK Hamiltonian
        let mut e_sk = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                e_sk -= j_mat[i][j] * sigma[i] * sigma[j];
            }
        }

        // 2. QuboModel calculate_total_energy
        let e_qubo = model.calculate_total_energy(&x);

        assert!(
            (e_qubo - e_sk).abs() < 1e-11,
            "SK energy mismatch at state {}: E_QUBO = {}, E_SK = {}, diff = {}",
            state_idx,
            e_qubo,
            e_sk,
            (e_qubo - e_sk).abs()
        );

        // 3. Single-flip delta E equivalence for every variable
        for var in 0..n {
            let mut x_flipped = x.clone();
            x_flipped[var] = 1 - x_flipped[var];
            let delta_qubo = model.calculate_total_energy(&x_flipped) - e_qubo;

            // Physical delta: flip sigma[var] -> -sigma[var]
            let mut delta_sk = 0.0;
            for j in 0..n {
                if j != var {
                    // term was - J_v,j * sigma[v] * sigma[j]
                    // changes to + J_v,j * sigma[v] * sigma[j]
                    // so delta = + 2 * J_v,j * sigma[v] * sigma[j]
                    delta_sk += 2.0 * j_mat[var][j] * sigma[var] * sigma[j];
                }
            }

            assert!(
                (delta_qubo - delta_sk).abs() < 1e-11,
                "Delta E mismatch at state {}, var {}: delta_qubo={}, delta_sk={}",
                state_idx,
                var,
                delta_qubo,
                delta_sk
            );
        }
    }
}
