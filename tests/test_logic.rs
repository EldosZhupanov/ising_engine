use ising_engine::compiler::LogicBuilder;

// ========================================================================
// AND gate truth table
// ========================================================================

#[test]
fn test_and_gate_truth_table() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_and_gate(a, b, z);
    let model = builder.build();

    let truth_table: [(i8, i8, i8); 4] = [
        (0, 0, 0), // 0 AND 0 = 0
        (0, 1, 0), // 0 AND 1 = 0
        (1, 0, 0), // 1 AND 0 = 0
        (1, 1, 1), // 1 AND 1 = 1
    ];

    for &(a_val, b_val, z_expected) in &truth_table {
        let state = vec![a_val, b_val, z_expected];
        let energy = model.calculate_total_energy(&state);
        assert!(
            energy.abs() < 1e-10,
            "AND({}, {}) = {} should have zero energy, got {}",
            a_val,
            b_val,
            z_expected,
            energy
        );
    }

    // Verify incorrect outputs have positive penalty energy
    let wrong_states: [(i8, i8, i8); 4] = [(0, 0, 1), (0, 1, 1), (1, 0, 1), (1, 1, 0)];

    for &(a_val, b_val, z_wrong) in &wrong_states {
        let state = vec![a_val, b_val, z_wrong];
        let energy = model.calculate_total_energy(&state);
        assert!(
            energy > 0.0,
            "AND({}, {}) = {} (wrong!) should have positive energy, got {}",
            a_val,
            b_val,
            z_wrong,
            energy
        );
    }
}

// ========================================================================
// XOR gate truth table
// ========================================================================

#[test]
fn test_xor_gate_truth_table() {
    let mut builder = LogicBuilder::new();
    let x = builder.add_var();
    let y = builder.add_var();
    let z = builder.add_var();
    let w = builder.add_var(); // ancilla
    builder.add_xor_gate(x, y, z, w);
    let model = builder.build();

    // For each XOR input combo, there should exist an ancilla value
    // such that energy = 0 when z = x XOR y.
    let xor_table: [(i8, i8, i8); 4] = [(0, 0, 0), (0, 1, 1), (1, 0, 1), (1, 1, 0)];

    for &(x_val, y_val, z_expected) in &xor_table {
        // Try both ancilla values to find the ground state
        let mut found_zero_energy = false;
        for w_val in 0..=1_i8 {
            let state = vec![x_val, y_val, z_expected, w_val];
            let energy = model.calculate_total_energy(&state);
            if energy.abs() < 1e-10 {
                found_zero_energy = true;
                break;
            }
        }
        assert!(
            found_zero_energy,
            "XOR({}, {}) = {} should have a zero-energy state for some ancilla value",
            x_val, y_val, z_expected
        );
    }

    // Verify incorrect outputs have positive energy for ALL ancilla values
    let wrong_table: [(i8, i8, i8); 4] = [(0, 0, 1), (0, 1, 0), (1, 0, 0), (1, 1, 1)];

    for &(x_val, y_val, z_wrong) in &wrong_table {
        for w_val in 0..=1_i8 {
            let state = vec![x_val, y_val, z_wrong, w_val];
            let energy = model.calculate_total_energy(&state);
            assert!(
                energy > 0.0,
                "XOR({}, {}) = {} (wrong!) w={} should have positive energy, got {}",
                x_val,
                y_val,
                z_wrong,
                w_val,
                energy
            );
        }
    }
}

// ========================================================================
// Half adder truth table
// ========================================================================

#[test]
fn test_half_adder_truth_table() {
    let truth_table: [(i8, i8, i8, i8); 4] = [
        (0, 0, 0, 0), // 0+0 = sum=0, carry=0
        (0, 1, 1, 0), // 0+1 = sum=1, carry=0
        (1, 0, 1, 0), // 1+0 = sum=1, carry=0
        (1, 1, 0, 1), // 1+1 = sum=0, carry=1
    ];

    for &(a_val, b_val, sum_expected, carry_expected) in &truth_table {
        let mut builder = LogicBuilder::new();
        let a = builder.add_var();
        let b = builder.add_var();
        let sum = builder.add_var();
        let carry = builder.add_var();
        builder.add_half_adder(a, b, sum, carry);
        let model = builder.build();

        // Clamp inputs and expected outputs, check if correct state has 0 energy
        let mut best_energy = f64::MAX;
        let ancilla_count = model.num_vars - 4;
        let ancilla_combos = 1u32 << ancilla_count;

        for ancilla_bits in 0..ancilla_combos {
            let mut state = vec![a_val, b_val, sum_expected, carry_expected];
            for bit_idx in 0..ancilla_count {
                state.push(((ancilla_bits >> bit_idx) & 1) as i8);
            }
            let energy = model.calculate_total_energy(&state);
            if energy < best_energy {
                best_energy = energy;
            }
        }

        assert!(
            best_energy.abs() < 1e-10,
            "HalfAdder({}, {}) = sum={}, carry={} should have zero energy, best={}",
            a_val,
            b_val,
            sum_expected,
            carry_expected,
            best_energy
        );
    }
}

// ========================================================================
// Full adder truth table
// ========================================================================

#[test]
fn test_full_adder_truth_table() {
    let truth_table: [(i8, i8, i8, i8, i8); 8] = [
        (0, 0, 0, 0, 0),
        (0, 0, 1, 1, 0),
        (0, 1, 0, 1, 0),
        (0, 1, 1, 0, 1),
        (1, 0, 0, 1, 0),
        (1, 0, 1, 0, 1),
        (1, 1, 0, 0, 1),
        (1, 1, 1, 1, 1),
    ];

    for &(a_val, b_val, cin_val, sum_expected, cout_expected) in &truth_table {
        let mut builder = LogicBuilder::new();
        let a = builder.add_var();
        let b = builder.add_var();
        let c_in = builder.add_var();
        let sum = builder.add_var();
        let c_out = builder.add_var();
        builder.add_full_adder(a, b, c_in, sum, c_out);
        let model = builder.build();

        let mut best_energy = f64::MAX;
        let ancilla_count = model.num_vars - 5;
        let ancilla_combos = 1u32 << ancilla_count;

        for ancilla_bits in 0..ancilla_combos {
            let mut state = vec![a_val, b_val, cin_val, sum_expected, cout_expected];
            for bit_idx in 0..ancilla_count {
                state.push(((ancilla_bits >> bit_idx) & 1) as i8);
            }
            let energy = model.calculate_total_energy(&state);
            if energy < best_energy {
                best_energy = energy;
            }
        }

        assert!(
            best_energy.abs() < 1e-10,
            "FullAdder({}, {}, {}) = sum={}, cout={} should have zero energy, best={}",
            a_val,
            b_val,
            cin_val,
            sum_expected,
            cout_expected,
            best_energy
        );
    }
}
