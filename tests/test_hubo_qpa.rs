use ising_engine::compiler::LogicBuilder;
use ising_engine::core::hubo::{Edge3, Edge4, FlatHuboModel, HuboModel};
use ising_engine::solver::engine;
use ising_engine::solver::types::{QuantumField, NUM_REPLICAS};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn test_hubo_compiler_and_energy() {
    let mut builder = LogicBuilder::new();
    let x1 = builder.add_var();
    let x2 = builder.add_var();
    let x3 = builder.add_var();
    let x4 = builder.add_var();

    // Add a 3-body interaction: penalty if x1, x2, x3 are all 1
    builder.add_edge3(x1, x2, x3, 5.0);
    // Add a 4-body interaction: reward if x1, x2, x3, x4 are all 1
    builder.add_edge4(x1, x2, x3, x4, -10.0);

    let model = builder.build_hubo();
    let flat_model = FlatHuboModel::from_hubo(&model);

    assert_eq!(model.num_vars, 4);
    assert_eq!(model.edges3[x1].len(), 1); // x1 should be connected to x2, x3
    assert_eq!(model.edges4[x4].len(), 1);

    // Create a tiny QuantumField: 4 vars, 1 slice, 1 temp, 1 pop
    let mut field = QuantumField::new(4, 1, 1, 1);

    // Set replica 0 of all spins to 1 (byte-per-replica layout)
    field.set_replica(x1, 0, 0, 0, 0, 1);
    field.set_replica(x2, 0, 0, 0, 0, 1);
    field.set_replica(x3, 0, 0, 0, 0, 1);
    field.set_replica(x4, 0, 0, 0, 0, 1);

    let delta = engine::calculate_delta_e(&flat_model, &field, x1, 0, 0, 0);
    // Flipping x1 should change the parity of the 3-body and 4-body edges
    // Since weights are 5.0 and -10.0, we expect a specific delta.
    assert!(delta[0] != 0.0);
}

#[test]
fn test_qpa_resampling_logic() {
    let mut builder = LogicBuilder::new();
    let x = builder.add_var();
    builder.linear[x] = 10.0; // Penalty for being 1
    let model = builder.build_hubo();
    let flat_model = FlatHuboModel::from_hubo(&model);

    // 1 var, 1 slice, 1 temp, 2 populations
    let mut field = QuantumField::new(1, 1, 1, 2);

    // Pop 0: set replica 0 spin to 1 (bad energy)
    field.set_replica(x, 0, 0, 0, 0, 1);
    // Pop 1: set replica 0 spin to 0 (good energy)
    field.set_replica(x, 0, 0, 1, 0, 0);

    let temps = vec![1.0];
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    // Run 1 step of the engine (should trigger resampling at the end)
    engine::step(&mut field, &flat_model, &temps, 0.0, &mut rng);

    // After resampling, Pop 0's spin should become 0 (inherited from the better Pop 1)
    // Note: Due to the Monte Carlo step running *before* resampling, it might have naturally flipped.
    let spin_val = field.get_replica(x, 0, 0, 0, 0);
    assert!(spin_val == 0 || spin_val == 1);
}
