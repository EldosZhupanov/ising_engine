#![allow(warnings)]
use ising_engine::compiler::LogicBuilder;
use ising_engine::solver::ParallelTemperingSolver;

fn main() {
    println!("🦀 QUBO Engine: Reversing a One-Way Hash Function");
    println!("--------------------------------------------------");

    let mut compiler = LogicBuilder::new();

    // 4-bit input password
    let in0 = compiler.add_var();
    let in1 = compiler.add_var();
    let in2 = compiler.add_var();
    let in3 = compiler.add_var();

    // Layer 1: Mixing
    let x1 = compiler.add_var();
    let w1 = compiler.add_var();
    compiler.add_xor_gate(in0, in1, x1, w1);
    let x2 = compiler.add_var();
    compiler.add_and_gate(in2, in3, x2);
    let x3 = compiler.add_var();
    let w3 = compiler.add_var();
    compiler.add_xor_gate(in1, in2, x3, w3);
    let x4 = compiler.add_var();
    compiler.add_and_gate(in0, in3, x4);

    // Layer 2: Final hash
    let h0 = compiler.add_var();
    let wh0 = compiler.add_var();
    compiler.add_xor_gate(x1, x2, h0, wh0);
    let h1 = compiler.add_var();
    compiler.add_and_gate(x3, x4, h1);
    let h2 = compiler.add_var();
    compiler.add_and_gate(x1, x3, h2);
    let h3 = compiler.add_var();
    let wh3 = compiler.add_var();
    compiler.add_xor_gate(x2, x4, h3, wh3);

    let model = compiler.build();
    let solver = ParallelTemperingSolver {
        num_replicas: 64,
        temp_max: 300.0,
        temp_min: 0.01,
        sweeps_per_exchange: 500,
        total_exchanges: 500,
        seed: None,
    };

    // Target hash: [0, 1, 1, 0] (preimage: [1, 0, 1, 1])
    let clamped = vec![(h0, 0), (h1, 1), (h2, 1), (h3, 0)];
    println!("Target: recover password from hash [0, 1, 1, 0].");
    println!("Running thermodynamic search (64 replicas)...");

    let result = solver.solve(&model, &clamped);
    let energy = model.calculate_total_energy(&result);

    println!("\n[ RESULT ]");
    println!(
        "Password: [{}, {}, {}, {}]",
        result[in0], result[in1], result[in2], result[in3]
    );
    println!("Energy: {}", energy);

    if energy.abs() < 1e-10 {
        println!("✅ Found valid preimage!");
    } else {
        println!("❌ Local minimum.");
    }
}
