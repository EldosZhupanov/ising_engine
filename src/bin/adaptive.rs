#![allow(warnings)]
use ising_engine::compiler::{LogicBuilder, Multiplier2x2};
use ising_engine::solver::AdaptiveTemperingSolver;

fn main() {
    println!("🦀 Adaptive Parallel Tempering");
    println!("Runtime temperature balancing enabled\n");

    let mut compiler = LogicBuilder::new();
    let a0 = compiler.add_var();
    let a1 = compiler.add_var();
    let b0 = compiler.add_var();
    let b1 = compiler.add_var();
    let p0 = compiler.add_var();
    let p1 = compiler.add_var();
    let p2 = compiler.add_var();
    let p3 = compiler.add_var();

    compiler.add_multiplier_2x2(Multiplier2x2 {
        a: [a0, a1],
        b: [b0, b1],
        p: [p0, p1, p2, p3],
    });
    let model = compiler.build();

    let solver = AdaptiveTemperingSolver {
        num_replicas: 64,
        temp_max: 300.0,
        temp_min: 0.01,
        sweeps_per_exchange: 2000,
        total_exchanges: 400,
        adaptation_interval: 40,
        seed: None,
    };

    let clamped = vec![(p3, 0), (p2, 1), (p1, 1), (p0, 0)]; // Product = 6
    println!("Clamped output: Product = 6. Searching...");
    let result = solver.solve(&model, &clamped);
    let energy = model.calculate_total_energy(&result);
    let val_a = (result[a1] << 1) | result[a0];
    let val_b = (result[b1] << 1) | result[b0];

    println!("\n[ ADAPTIVE RESULT ]");
    println!("A={}, B={}, Energy={}", val_a, val_b, energy);
    if energy.abs() < 1e-10 && val_a * val_b == 6 {
        println!("✅ Adaptive solver found the answer!");
    } else {
        println!("❌ Local minimum.");
    }
}
