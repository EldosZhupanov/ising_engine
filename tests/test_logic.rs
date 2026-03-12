use ising_engine::compiler::LogicBuilder;
use ising_engine::solver::ParallelTemperingSolver;

#[test]
fn test_not_gate() {
    let mut builder = LogicBuilder::new();
    let x = builder.add_var();
    let y = builder.add_var();
    builder.add_not_gate(x, y);
    let model = builder.build();

    let solver = ParallelTemperingSolver {
        num_replicas: 16, temp_max: 100.0, temp_min: 0.1,
        sweeps_per_exchange: 1000, total_exchanges: 100, seed: None,
    };
    
    let res1 = solver.solve(&model, &[(x, 1)]);
    assert_eq!(res1[y], 0);

    let res2 = solver.solve(&model, &[(x, 0)]);
    assert_eq!(res2[y], 1);
}

#[test]
fn test_or_gate() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_or_gate(a, b, z);
    let model = builder.build();

    let solver = ParallelTemperingSolver {
        num_replicas: 16, temp_max: 100.0, temp_min: 0.1, sweeps_per_exchange: 500, total_exchanges: 50, seed: None,
    };

    assert_eq!(solver.solve(&model, &[(a, 0), (b, 0)])[z], 0);
    assert_eq!(solver.solve(&model, &[(a, 1), (b, 0)])[z], 1);
    assert_eq!(solver.solve(&model, &[(a, 0), (b, 1)])[z], 1);
    assert_eq!(solver.solve(&model, &[(a, 1), (b, 1)])[z], 1);
}

#[test]
fn test_mux_gate() {
    let mut builder = LogicBuilder::new();
    let s = builder.add_var(); 
    let a = builder.add_var(); 
    let b = builder.add_var(); 
    let z = builder.add_var(); 
    builder.add_mux_gate(s, a, b, z);
    let model = builder.build();

    let solver = ParallelTemperingSolver {
        num_replicas: 16, temp_max: 100.0, temp_min: 0.1, sweeps_per_exchange: 500, total_exchanges: 50, seed: None,
    };

    assert_eq!(solver.solve(&model, &[(s, 1), (a, 1), (b, 0)])[z], 1);
    assert_eq!(solver.solve(&model, &[(s, 1), (a, 0), (b, 1)])[z], 0);
    
    assert_eq!(solver.solve(&model, &[(s, 0), (a, 1), (b, 0)])[z], 0);
    assert_eq!(solver.solve(&model, &[(s, 0), (a, 0), (b, 1)])[z], 1);
}

#[test]
fn test_comparator_equal() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let eq = builder.add_var();
    builder.add_equal_gate(a, b, eq);
    let model = builder.build();

    let solver = ParallelTemperingSolver {
        num_replicas: 16, temp_max: 100.0, temp_min: 0.1, sweeps_per_exchange: 500, total_exchanges: 50, seed: None,
    };

    assert_eq!(solver.solve(&model, &[(a, 0), (b, 0)])[eq], 1);
    assert_eq!(solver.solve(&model, &[(a, 1), (b, 1)])[eq], 1);
    assert_eq!(solver.solve(&model, &[(a, 1), (b, 0)])[eq], 0);
    assert_eq!(solver.solve(&model, &[(a, 0), (b, 1)])[eq], 0);
}
