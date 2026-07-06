//! Benchmark suite.
//!
//! Design rule: every benchmark states what it measures, and kernel
//! benchmarks assert at setup that presolve fixes NOTHING on their instance —
//! otherwise they would silently measure overhead instead of annealing.
//! (This exact failure happened when first-order persistency landed: the old
//! uniform-positive instances were fully presolved and the "kernel" benchmark
//! dropped 59% without a single kernel change.)

use criterion::{criterion_group, criterion_main, Criterion};
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::presolve::{connected_components, fix_persistent_variables};
use ising_engine::solver::parallel_tempering::ParallelTemperingSolver;
use ising_engine::solver::ultimate::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Duration;

/// Dense mixed-sign QUBO (weights and fields in [-1, 1], seeded).
/// Mixed signs keep every persistency bound L_i/U_i far from zero, so the
/// instance is presolve-immune and exercises the full annealing kernel.
#[allow(clippy::needless_range_loop)]
fn mixed_sign_qubo(n: usize, seed: u64) -> QuboModel {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
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

/// Uniform-positive QUBO: every variable is provably fixed to 0 by
/// first-order persistency. Used to measure presolve + non-sweep overheads.
fn uniform_positive_qubo(n: usize, w: f64, h: f64) -> QuboModel {
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                col_indices.push(j);
                values.push(w);
            }
        }
        row_offsets.push(col_indices.len());
    }
    QuboModel {
        energy_offset: 0.0,
        num_vars: n,
        linear: vec![h; n],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

fn bench_scalar_pt(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar");
    group.measurement_time(Duration::from_secs(10));

    let num_vars = 10;
    let model = QuboModel {
        energy_offset: 0.0,
        num_vars,
        linear: vec![0.1; num_vars],
        quadratic: CsrMatrix::empty(num_vars),
    };
    let solver = ParallelTemperingSolver {
        num_replicas: 4,
        temp_max: 10.0,
        temp_min: 0.1,
        sweeps_per_exchange: 10,
        total_exchanges: 50,
        seed: None,
    };
    group.bench_function("parallel_tempering_small", |b| {
        b.iter(|| solver.solve(&model, &[]))
    });
    group.finish();
}

/// Kernel benchmarks: full annealing on presolve-immune instances.
fn bench_ultimate_kernel(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultimate_kernel");
    group.measurement_time(Duration::from_secs(10));

    let model_50 = mixed_sign_qubo(50, 0xA5);
    assert!(
        fix_persistent_variables(&model_50, &[]).is_empty(),
        "kernel benchmark instance (n=50) must be presolve-immune"
    );
    assert_eq!(
        connected_components(&model_50, &[true; 50]).len(),
        1,
        "kernel benchmark instance (n=50) must be connected"
    );
    let solver_50 = UltimateSolver::new(10.0, 0.1, 10, 20, Some(42)).with_quantum_dims(1, 10, 1);
    group.bench_function("ultimate_solver_50_vars_mixed", |b| {
        b.iter(|| solver_50.solve(&model_50, &[]))
    });

    let model_200 = mixed_sign_qubo(200, 0xBE42C4);
    assert!(
        fix_persistent_variables(&model_200, &[]).is_empty(),
        "kernel benchmark instance (n=200) must be presolve-immune"
    );
    assert_eq!(
        connected_components(&model_200, &[true; 200]).len(),
        1,
        "kernel benchmark instance (n=200) must be connected"
    );
    let solver_200 = UltimateSolver::new(10.0, 0.1, 5, 10, Some(42)).with_quantum_dims(1, 12, 1);
    group.bench_function("ultimate_solver_200_vars_mixed", |b| {
        b.iter(|| solver_200.solve(&model_200, &[]))
    });
    group.finish();
}

/// Overhead benchmarks on the fully-presolvable uniform instance:
/// `presolve_short_circuit` runs the whole solver with zero sweep work
/// (per-step RNG-buffer fill + PT bookkeeping remain); `presolve_only`
/// times the O(nnz) persistency pass itself.
fn bench_overheads(c: &mut Criterion) {
    let mut group = c.benchmark_group("overheads");
    group.measurement_time(Duration::from_secs(10));

    let model = uniform_positive_qubo(200, 0.01, 0.1);
    let derived = fix_persistent_variables(&model, &[]);
    assert_eq!(
        derived.len(),
        200,
        "overhead benchmark instance must be fully presolved"
    );

    let solver = UltimateSolver::new(10.0, 0.1, 5, 10, Some(42)).with_quantum_dims(1, 12, 1);
    group.bench_function("presolve_short_circuit_200_vars", |b| {
        b.iter(|| solver.solve(&model, &[]))
    });
    group.bench_function("presolve_only_200_vars", |b| {
        b.iter(|| fix_persistent_variables(&model, &[]))
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_scalar_pt,
    bench_ultimate_kernel,
    bench_overheads
);
criterion_main!(benches);
