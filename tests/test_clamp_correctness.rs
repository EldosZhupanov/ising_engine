//! Clamp (conditional-optimization) contract for the MSC solver path.
//!
//! solve(&model, &clamped) must minimize E over the FREE variables with the
//! clamped ones held fixed — i.e. sample/optimize the conditional distribution
//! π(x_free | x_clamped). The standard exact MCMC treatment is that Metropolis
//! proposals never touch conditioned ("quenched") variables.

use ising_engine::compiler::LogicBuilder;
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;

/// A clamp must survive maximal energetic pressure: h = -5 makes x = 1 the
/// unconstrained optimum (E = -5), but the clamp fixes x = 0.
#[test]
fn clamped_variable_survives_energetic_pressure() {
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 1,
        linear: vec![-5.0],
        quadratic: CsrMatrix::empty(1),
    };
    let solver = UltimateSolver::new(10.0, 0.1, 20, 20, Some(1234));
    let result = solver.solve(&model, &[(0, 0)]);
    assert_eq!(
        result[0], 0,
        "clamp x0=0 violated: solver flipped an energetically favorable clamped variable"
    );
}

/// Clamps pin exactly the clamped variables; free variables still reach the
/// (trivial) optimum. h_i = -1 for all i favors x_i = 1 everywhere.
#[test]
fn clamps_pin_only_the_clamped_variables() {
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars: 4,
        linear: vec![-1.0; 4],
        quadratic: CsrMatrix::empty(4),
    };
    let solver = UltimateSolver::new(10.0, 0.1, 50, 50, Some(99));
    let result = solver.solve(&model, &[(1, 0), (3, 0)]);
    assert_eq!(result[1], 0, "clamp x1=0 violated");
    assert_eq!(result[3], 0, "clamp x3=0 violated");
    assert_eq!(result[0], 1, "free variable x0 failed to reach its optimum");
    assert_eq!(result[2], 1, "free variable x2 failed to reach its optimum");
}

/// End-to-end reverse Toffoli (the flagship demo, seeded): clamping A=1, B=1,
/// Cout=0 leaves a unique energy-0 completion — x = A AND B = 1, and
/// Cin = Cout XOR x = 1. The solver must return the clamps verbatim and find
/// that completion.
#[test]
fn toffoli_clamps_are_honored_end_to_end() {
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

    let solver = UltimateSolver::new(100.0, 0.1, 50, 100, Some(7));
    let clamped = vec![(a, 1), (b, 1), (c_out, 0)];
    let result = solver.solve(&model, &clamped);

    assert_eq!(result[a], 1, "clamp A=1 violated");
    assert_eq!(result[b], 1, "clamp B=1 violated");
    assert_eq!(result[c_out], 0, "clamp Cout=0 violated");
    assert_eq!(result[x], 1, "x = A AND B must be 1");
    assert_eq!(result[c_in], 1, "unique consistent completion is Cin = 1");
    let _ = w; // XOR auxiliary — value implied by the others, not asserted
}
