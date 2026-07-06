//! Regression tests for the adaptive-temperature guard in the scalar family.
//!
//! The neighbor guard `new_temp.clamp(colder + 0.001, hotter - 0.001)` uses an
//! ABSOLUTE epsilon on a GEOMETRIC ladder: whenever two adjacent temperatures
//! lie within 0.002 of each other (a perfectly legal configuration, e.g.
//! temp_max = 0.03, temp_min = 0.02, 32 replicas → adjacent gaps ≈ 4·10⁻⁴),
//! the clamp bounds invert and `f64::clamp` panics by contract (min > max).
//! A solver must never panic on valid hyperparameters.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::{AdaptiveTemperingSolver, ClusterSolver};

fn tiny_model() -> QuboModel {
    QuboModel {
        energy_offset: 0.0,
        num_vars: 4,
        linear: vec![0.1; 4],
        quadratic: CsrMatrix::empty(4),
    }
}

#[test]
fn adaptive_solver_survives_tight_temperature_ladder() {
    let solver = AdaptiveTemperingSolver {
        num_replicas: 32,
        temp_max: 0.03,
        temp_min: 0.02,
        sweeps_per_exchange: 5,
        total_exchanges: 2,
        adaptation_interval: 1,
        seed: Some(1),
    };
    let state = solver.solve(&tiny_model(), &[]);
    assert_eq!(state.len(), 4);
    assert!(state.iter().all(|&x| x == 0 || x == 1));
}

#[test]
fn cluster_solver_survives_tight_temperature_ladder() {
    let solver = ClusterSolver {
        num_replicas: 32,
        temp_max: 0.03,
        temp_min: 0.02,
        sweeps_per_exchange: 5,
        total_exchanges: 2,
        adaptation_interval: 1,
        seed: Some(1),
    };
    let state = solver.solve(&tiny_model(), &[]);
    assert_eq!(state.len(), 4);
    assert!(state.iter().all(|&x| x == 0 || x == 1));
}
