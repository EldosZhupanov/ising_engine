//! Contracts for data-driven temperature range and anytime incumbent/polish.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::schedule::suggested_temp_range;
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[allow(clippy::needless_range_loop)]
fn random_qubo(n: usize, scale: f64, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            upper[i][j] = rng.gen_range(-2.0..2.0) * scale;
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
        num_vars: n,
        linear: (0..n).map(|_| rng.gen_range(-2.0..2.0) * scale).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
        energy_offset: 0.0,
    }
}

/// Temperature range must scale linearly with coefficient magnitude: an
/// instance scaled by C returns a range scaled by exactly C (acceptance
/// profile is scale-invariant).
#[test]
fn temp_range_scales_with_coefficients() {
    let mut rng = ChaCha8Rng::seed_from_u64(10);
    let base = random_qubo(20, 1.0, &mut rng);
    let mut rng = ChaCha8Rng::seed_from_u64(10);
    let scaled = random_qubo(20, 1e6, &mut rng);

    let (bmax, bmin) = suggested_temp_range(&base);
    let (smax, smin) = suggested_temp_range(&scaled);
    assert!(
        (smax / bmax - 1e6).abs() / 1e6 < 1e-9,
        "T_max must scale by C"
    );
    assert!(
        (smin / bmin - 1e6).abs() / 1e6 < 1e-9,
        "T_min must scale by C"
    );
    assert!(bmax > bmin && smax > smin, "range must be ordered");
}

#[test]
fn temp_range_handles_degenerate_model() {
    let model = QuboModel {
        num_vars: 4,
        linear: vec![0.0; 4],
        quadratic: CsrMatrix::empty(4),
        energy_offset: 0.0,
    };
    let (tmax, tmin) = suggested_temp_range(&model);
    assert!(
        tmax > tmin && tmin > 0.0,
        "degenerate model must give a valid fallback"
    );
}

/// The returned solution must be a 1-opt local minimum (finisher runs inside
/// solve) — no single free flip improves it. This is the anytime + polish
/// guarantee observed at the public API.
#[test]
fn solver_output_is_locally_optimal() {
    let mut rng = ChaCha8Rng::seed_from_u64(11);
    for _ in 0..5 {
        let model = random_qubo(30, 1.0, &mut rng);
        let state = UltimateSolver::new(5.0, 0.05, 5, 10, Some(77)).solve(&model, &[]);
        let e = model.calculate_total_energy(&state);
        for v in 0..model.num_vars {
            let mut flipped = state.clone();
            flipped[v] = 1 - flipped[v];
            assert!(
                model.calculate_total_energy(&flipped) >= e - 1e-9,
                "solver returned a non-1-opt solution (var {} improves)",
                v
            );
        }
    }
}

/// Determinism must survive DEO + incumbent + polish: identical seeds give
/// identical results.
#[test]
fn solver_deterministic_with_all_features() {
    let mut rng = ChaCha8Rng::seed_from_u64(12);
    let model = random_qubo(24, 1.0, &mut rng);
    let s1 = UltimateSolver::new(5.0, 0.05, 10, 20, Some(999)).solve(&model, &[]);
    let s2 = UltimateSolver::new(5.0, 0.05, 10, 20, Some(999)).solve(&model, &[]);
    assert_eq!(s1, s2);
}
