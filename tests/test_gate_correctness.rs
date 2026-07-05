use ising_engine::compiler::LogicBuilder;
use ising_engine::core::QuboModel;

fn evaluate_qubo_brute_force(model: &QuboModel) -> Vec<(Vec<i8>, f64)> {
    let n = model.num_vars;
    let mut results = Vec::new();
    let num_states = 1 << n;
    for state_idx in 0..num_states {
        let mut state = vec![0i8; n];
        for (i, val) in state.iter_mut().enumerate().take(n) {
            *val = ((state_idx >> i) & 1) as i8;
        }
        let energy = model.calculate_total_energy(&state);
        results.push((state, energy));
    }
    results
}

#[test]
fn test_not_gate_truth_table() {
    let mut builder = LogicBuilder::new();
    let x = builder.add_var();
    let y = builder.add_var();
    builder.add_not_gate(x, y);
    let model = builder.build();

    let states = evaluate_qubo_brute_force(&model);

    // Find min energy
    let min_energy = states.iter().map(|(_, e)| *e).fold(f64::INFINITY, f64::min);

    for (state, energy) in states {
        let is_valid = state[y] == (1 - state[x]);
        if is_valid {
            assert!(
                (energy - min_energy).abs() < 1e-10,
                "Valid NOT state {:?} failed",
                state
            );
        } else {
            assert!(
                energy > min_energy + 0.9,
                "Invalid NOT state {:?} was not penalized",
                state
            );
        }
    }
}

#[test]
fn test_and_gate_truth_table() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_and_gate(a, b, z);
    let model = builder.build();

    let states = evaluate_qubo_brute_force(&model);
    let min_energy = states.iter().map(|(_, e)| *e).fold(f64::INFINITY, f64::min);

    for (state, energy) in states {
        let is_valid = state[z] == (state[a] & state[b]);
        if is_valid {
            assert!(
                (energy - min_energy).abs() < 1e-10,
                "Valid AND state {:?} failed",
                state
            );
        } else {
            assert!(
                energy > min_energy + 0.9,
                "Invalid AND state {:?} was not penalized",
                state
            );
        }
    }
}

#[test]
fn test_or_gate_truth_table() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_or_gate(a, b, z);
    let model = builder.build();

    let states = evaluate_qubo_brute_force(&model);
    let min_energy = states.iter().map(|(_, e)| *e).fold(f64::INFINITY, f64::min);

    for (state, energy) in states {
        let is_valid = state[z] == (state[a] | state[b]);
        if is_valid {
            assert!(
                (energy - min_energy).abs() < 1e-10,
                "Valid OR state {:?} failed",
                state
            );
        } else {
            assert!(
                energy > min_energy + 0.9,
                "Invalid OR state {:?} was not penalized",
                state
            );
        }
    }
}

#[test]
fn test_xor_gate_truth_table() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    let aux = builder.add_var();
    builder.add_xor_gate(a, b, z, aux);
    let model = builder.build();

    // Verify XOR output z == a ^ b for all combinations of input, optimizing over aux
    for a_val in 0..=1 {
        for b_val in 0..=1 {
            for z_val in 0..=1 {
                let is_valid = z_val == (a_val ^ b_val);

                let mut min_energy_for_config = f64::INFINITY;
                for aux_val in 0..=1 {
                    let mut state = vec![0i8; 4];
                    state[a] = a_val;
                    state[b] = b_val;
                    state[z] = z_val;
                    state[aux] = aux_val;
                    let e = model.calculate_total_energy(&state);
                    if e < min_energy_for_config {
                        min_energy_for_config = e;
                    }
                }

                if is_valid {
                    assert!(
                        (min_energy_for_config - 0.0).abs() < 1e-10,
                        "Valid XOR combination ({}, {}, {}) failed",
                        a_val,
                        b_val,
                        z_val
                    );
                } else {
                    assert!(
                        min_energy_for_config >= 1.0 - 1e-10,
                        "Invalid XOR combination ({}, {}, {}) was not penalized",
                        a_val,
                        b_val,
                        z_val
                    );
                }
            }
        }
    }
}

#[test]
fn test_half_adder_truth_table() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let sum = builder.add_var();
    let carry = builder.add_var();
    builder.add_half_adder(a, b, sum, carry);
    let model = builder.build();

    // Since half adder uses XOR under the hood, it introduces 1 aux variable for XOR.
    // Total variables = 5.
    // Inputs: a, b. Outputs: sum, carry.
    // We optimize over all auxiliary variables (index 4) for each configuration of (a, b, sum, carry)
    for a_val in 0..=1 {
        for b_val in 0..=1 {
            let true_sum = a_val ^ b_val;
            let true_carry = a_val & b_val;

            for sum_val in 0..=1 {
                for carry_val in 0..=1 {
                    let is_valid = (sum_val == true_sum) && (carry_val == true_carry);

                    let mut min_energy_for_config = f64::INFINITY;
                    // Try all combinations of the 5th variable (aux variable used in XOR)
                    for aux_val in 0..=1 {
                        let mut state = vec![0i8; 5];
                        state[a] = a_val;
                        state[b] = b_val;
                        state[sum] = sum_val;
                        state[carry] = carry_val;
                        state[4] = aux_val; // XOR auxiliary variable
                        let e = model.calculate_total_energy(&state);
                        if e < min_energy_for_config {
                            min_energy_for_config = e;
                        }
                    }

                    if is_valid {
                        assert!(
                            (min_energy_for_config - 0.0).abs() < 1e-10,
                            "Valid Half Adder combination ({}, {}) failed",
                            a_val,
                            b_val
                        );
                    } else {
                        assert!(min_energy_for_config >= 1.0 - 1e-10, "Invalid Half Adder combination ({}, {}) with sum={}, carry={} was not penalized", a_val, b_val, sum_val, carry_val);
                    }
                }
            }
        }
    }
}
