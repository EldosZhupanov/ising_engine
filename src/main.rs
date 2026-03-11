use ising_engine::compiler::LogicBuilder;
use ising_engine::solver::UltimateSolver;
use std::time::Instant;

fn main() {
    println!("🦀 QUBO Engine + Logic Compiler (v4 Ultimate MSC Edition)");
    println!("Toffoli gate: Cout = Cin XOR (A AND B)\n");

    let mut compiler = LogicBuilder::new();

    let a = compiler.add_var();
    let b = compiler.add_var();
    let c_in = compiler.add_var();
    let x = compiler.add_var();
    let w = compiler.add_var();
    let c_out = compiler.add_var();

    compiler.add_and_gate(a, b, x);
    compiler.add_xor_gate(c_in, x, c_out, w);

    let model = compiler.build();
    let solver = UltimateSolver::new(
        100.0, // temp_max
        0.1,   // temp_min
        50,    // sweeps_per_exchange
        100,   // total_exchanges
        None,  // seed
    );

    let clamped = vec![(a, 1), (b, 1), (c_out, 0)];
    println!("Annealing... A=1, B=1, Cout=0. Solving for Cin...");

    let start = Instant::now();
    let result = solver.solve(&model, &clamped);
    let duration = start.elapsed();

    println!("\n[ RESULT ]");
    println!(
        "A: {}, B: {}, Cin: {}, Cout: {}",
        result[a], result[b], result[c_in], result[c_out]
    );

    if result[c_in] == 1 {
        println!("✅ Cin = 1 recovered. Reverse Toffoli solved in {:?}!", duration);
    } else {
        println!("❌ Local minimum reached in {:?}.", duration);
    }
}
