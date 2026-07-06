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
