//! Canonical-basis contract for the MSC engine.
//!
//! The engine must evaluate the same product-basis Hamiltonian as the rest of
//! the library (x ∈ {0,1}):
//!
//!   E(x) = Σ h_i·x_i + Σ w_ij·x_i·x_j + Σ w_ijk·x_i·x_j·x_k + Σ w_ijkl·x_i·x_j·x_k·x_l
//!
//! All engine calls here pass j_tau = 0.0 to isolate the classical Hamiltonian
//! from the Trotter inter-slice coupling, which is a separate concern.

use ising_engine::compiler::LogicBuilder;
use ising_engine::core::hubo::{Edge2, FlatHuboModel, HuboModel};
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::engine;
use ising_engine::solver::types::QuantumField;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Mirrors the QuboModel → HuboModel conversion in UltimateSolver::solve().
fn flat_from_qubo(model: &QuboModel) -> FlatHuboModel {
    let mut hubo = HuboModel::new(model.num_vars);
    hubo.linear = model.linear.clone();
    for i in 0..model.num_vars {
        for (j, weight) in model.quadratic.get_row(i) {
            hubo.edges2[i].push(Edge2 { j, weight });
        }
    }
    FlatHuboModel::from_hubo(&hubo)
}

/// Writes `state` into replica 0 of a fresh 1-slice/1-temp/1-pop field.
fn field_from_state(state: &[i8]) -> QuantumField {
    let mut field = QuantumField::new(state.len(), 1, 1, 1);
    for (v, &x) in state.iter().enumerate() {
        field.set_replica(v, 0, 0, 0, 0, x);
    }
    field
}

fn state_from_bits(bits: u32, n: usize) -> Vec<i8> {
    (0..n).map(|v| ((bits >> v) & 1) as i8).collect()
}

/// Dense symmetric random QUBO with weights in [-1, 1].
// Two-sided matrix indexing (upper[i][j] vs upper[j][i]) is clearer as written.
#[allow(clippy::needless_range_loop)]
fn random_qubo(n: usize, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            upper[i][j] = rng.gen_range(-1.0..1.0);
        }
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let w = if i < j { upper[i][j] } else { upper[j][i] };
                col_indices.push(j);
                values.push(w);
            }
        }
        row_offsets.push(col_indices.len());
    }
    QuboModel {
        energy_offset: 0.0,
        num_vars: n,
        linear: (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

#[test]
fn engine_energy_matches_qubo_energy_and_gate_exhaustive() {
    let mut builder = LogicBuilder::new();
    let a = builder.add_var();
    let b = builder.add_var();
    let z = builder.add_var();
    builder.add_and_gate(a, b, z);
    let model = builder.build();
    let flat = flat_from_qubo(&model);

    for bits in 0..8u32 {
        let state = state_from_bits(bits, 3);
        let field = field_from_state(&state);
        let internal = engine::calculate_replica_energies(&flat, &field, 0, 0, 0.0)[0];
        let reference = model.calculate_total_energy(&state);
        assert!(
            (internal - reference).abs() < 1e-9,
            "state {:?}: engine energy {} != QUBO energy {}",
            state,
            internal,
            reference
        );
    }
}

#[test]
fn engine_energy_matches_qubo_energy_random_instances() {
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    let n = 8;
    for _ in 0..20 {
        let model = random_qubo(n, &mut rng);
        let flat = flat_from_qubo(&model);
        for bits in 0..(1u32 << n) {
            let state = state_from_bits(bits, n);
            let field = field_from_state(&state);
            let internal = engine::calculate_replica_energies(&flat, &field, 0, 0, 0.0)[0];
            let reference = model.calculate_total_energy(&state);
            assert!(
                (internal - reference).abs() < 1e-9,
                "state {:?}: engine energy {} != QUBO energy {}",
                state,
                internal,
                reference
            );
        }
    }
}

#[test]
fn engine_delta_matches_full_recompute() {
    let mut rng = ChaCha8Rng::seed_from_u64(11);
    let n = 8;
    for _ in 0..20 {
        let model = random_qubo(n, &mut rng);
        let flat = flat_from_qubo(&model);
        for _ in 0..50 {
            let bits: u32 = rng.gen_range(0..(1u32 << n));
            let state = state_from_bits(bits, n);
            let field = field_from_state(&state);
            for v in 0..n {
                let delta = engine::calculate_delta_e(&flat, &field, v, 0, 0, 0)[0];
                let mut flipped = state.clone();
                flipped[v] = 1 - flipped[v];
                let flipped_field = field_from_state(&flipped);
                let e_before = engine::calculate_replica_energies(&flat, &field, 0, 0, 0.0)[0];
                let e_after =
                    engine::calculate_replica_energies(&flat, &flipped_field, 0, 0, 0.0)[0];
                assert!(
                    (delta - (e_after - e_before)).abs() < 1e-9,
                    "flip var {} in state {:?}: delta {} != recomputed {}",
                    v,
                    state,
                    delta,
                    e_after - e_before
                );
            }
        }
    }
}

#[test]
fn engine_hubo_energy_matches_product_semantics() {
    // Contract for higher-order terms, matching the documented intent
    // (e.g. "penalty if x1, x2, x3 are all 1"): each term contributes
    // w · Π x_i, i.e. only when ALL member variables are 1.
    let mut builder = LogicBuilder::new();
    let x1 = builder.add_var();
    let x2 = builder.add_var();
    let x3 = builder.add_var();
    let x4 = builder.add_var();
    builder.linear[x1] = 1.5;
    builder.linear[x4] = -0.5;
    builder.quadratic.push((x1, x2, 2.0));
    builder.add_edge3(x1, x2, x3, 5.0);
    builder.add_edge4(x1, x2, x3, x4, -10.0);
    let hubo = builder.build_hubo();
    let flat = FlatHuboModel::from_hubo(&hubo);

    for bits in 0..16u32 {
        let s = state_from_bits(bits, 4);
        let (f1, f2, f3, f4) = (s[0] as f64, s[1] as f64, s[2] as f64, s[3] as f64);
        let reference =
            1.5 * f1 - 0.5 * f4 + 2.0 * f1 * f2 + 5.0 * f1 * f2 * f3 - 10.0 * f1 * f2 * f3 * f4;
        let field = field_from_state(&s);
        let internal = engine::calculate_replica_energies(&flat, &field, 0, 0, 0.0)[0];
        assert!(
            (internal - reference).abs() < 1e-9,
            "state {:?}: engine energy {} != product-basis energy {}",
            s,
            internal,
            reference
        );
    }
}
