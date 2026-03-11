use ising_engine::compiler::{LogicBuilder, Multiplier2x2};
use ising_engine::solver::ParallelTemperingSolver;

// ========================================================================
// Solver convergence tests
// ========================================================================

#[test]
fn test_solver_finds_and_preimage() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_and_gate(a, b, z);
    let model = builder.build();

    let solver = ParallelTemperingSolver {
        num_replicas: 8,
        temp_max: 50.0,
        temp_min: 0.01,
        sweeps_per_exchange: 200,
        total_exchanges: 50,
        seed: None,
    };

    // Clamp Z=1, solver should find A=1, B=1
    let clamped = vec![(z, 1)];

    // Run multiple times for statistical confidence
    let mut successes = 0;
    let trials = 10;

    for _ in 0..trials {
        let result = solver.solve(&model, &clamped);
        let energy = model.calculate_total_energy(&result);
        if energy.abs() < 1e-10 && result[a] == 1 && result[b] == 1 {
            successes += 1;
        }
    }

    // Should succeed at least 70% of the time for this trivial problem
    assert!(
        successes >= 7,
        "Solver only found AND preimage {}/{} times (expected ≥7)",
        successes,
        trials
    );
}

#[test]
fn test_solver_finds_factorization() {
    let mut builder = LogicBuilder::new();
    let a0 = builder.add_var();
    let a1 = builder.add_var();
    let b0 = builder.add_var();
    let b1 = builder.add_var();
    let p0 = builder.add_var();
    let p1 = builder.add_var();
    let p2 = builder.add_var();
    let p3 = builder.add_var();

    builder.add_multiplier_2x2(Multiplier2x2 {
        a: [a0, a1],
        b: [b0, b1],
        p: [p0, p1, p2, p3],
    });
    let model = builder.build();

    let solver = ParallelTemperingSolver {
        num_replicas: 32,
        temp_max: 200.0,
        temp_min: 0.01,
        sweeps_per_exchange: 1000,
        total_exchanges: 200,
        seed: None,
    };

    // 6 = 0110 in binary → p3=0, p2=1, p1=1, p0=0
    let clamped = vec![(p3, 0), (p2, 1), (p1, 1), (p0, 0)];

    let mut successes = 0;
    let trials = 5;

    for _ in 0..trials {
        let result = solver.solve(&model, &clamped);
        let energy = model.calculate_total_energy(&result);
        let val_a = (result[a1] << 1) | result[a0];
        let val_b = (result[b1] << 1) | result[b0];

        if energy.abs() < 1e-10 && val_a * val_b == 6 {
            successes += 1;
        }
    }

    assert!(
        successes >= 3,
        "Solver only factored 6 correctly {}/{} times (expected ≥3)",
        successes,
        trials
    );
}

#[test]
fn test_reproducibility() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    let aux = builder.add_var();
    builder.add_xor_gate(a, b, z, aux);
    let model = builder.build();

    let solver1 = ParallelTemperingSolver {
        num_replicas: 4,
        temp_max: 10.0,
        temp_min: 0.1,
        sweeps_per_exchange: 50,
        total_exchanges: 50,
        seed: Some(1337),
    };

    let solver2 = ParallelTemperingSolver {
        num_replicas: 4,
        temp_max: 10.0,
        temp_min: 0.1,
        sweeps_per_exchange: 50,
        total_exchanges: 50,
        seed: Some(1337),
    };

    let clamped = vec![(z, 1)];
    let res1 = solver1.solve(&model, &clamped);
    let res2 = solver2.solve(&model, &clamped);

    assert_eq!(res1, res2, "Deterministic solvers strictly returned differing states!");
}
