#![allow(warnings)]
use ising_engine::compiler::LogicBuilder;
use ising_engine::solver::ParallelTemperingSolver;

fn main() {
    println!("🦀 QUBO Engine: 2-bit Reversible Adder");
    println!("A (2-bit) + B (2-bit) = Sum (3-bit)\n");

    let mut compiler = LogicBuilder::new();
    let a0 = compiler.add_var();
    let a1 = compiler.add_var();
    let b0 = compiler.add_var();
    let b1 = compiler.add_var();
    let s0 = compiler.add_var();
    let s1 = compiler.add_var();
    let s2 = compiler.add_var();
    let carry0 = compiler.add_var();

    compiler.add_half_adder(a0, b0, s0, carry0);
    compiler.add_full_adder(a1, b1, carry0, s1, s2);

    let model = compiler.build();
    let solver = ParallelTemperingSolver {
        num_replicas: 32,
        temp_max: 200.0,
        temp_min: 0.01,
        sweeps_per_exchange: 200,
        total_exchanges: 200,
        seed: None,
    };

    let clamped = vec![(s2, 1), (s1, 0), (s0, 1)]; // Sum = 5 (101)
    println!("Clamped output: Sum = 5. Solving for inputs...\n");

    let result = solver.solve(&model, &clamped);
    let energy = model.calculate_total_energy(&result);
    let val_a = (result[a1] << 1) | result[a0];
    let val_b = (result[b1] << 1) | result[b0];

    println!(
        "A = {} ({}{}), B = {} ({}{})",
        val_a, result[a1], result[a0], val_b, result[b1], result[b0]
    );
    println!("Energy: {}", energy);

    if energy.abs() < 1e-10 && val_a + val_b == 5 {
        println!("✅ Found {} + {} = 5!", val_a, val_b);
    } else {
        println!("❌ Local minimum.");
    }
}
