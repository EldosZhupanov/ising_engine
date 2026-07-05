use criterion::{criterion_group, criterion_main, Criterion};
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::parallel_tempering::ParallelTemperingSolver;
use std::time::Duration;

fn bench_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimizer");
    group.measurement_time(Duration::from_secs(10));

    // Create a small mock model
    let num_vars = 10;
    let mut model = QuboModel {
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

fn bench_ultimate_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultimate_solver");
    group.measurement_time(Duration::from_secs(5));

    let num_vars = 50;
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..num_vars {
        for j in 0..num_vars {
            if i != j {
                col_indices.push(j);
                values.push(0.05);
            }
        }
        row_offsets.push(col_indices.len());
    }

    let model = QuboModel {
        num_vars,
        linear: vec![0.1; num_vars],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };

    use ising_engine::solver::ultimate::UltimateSolver;
    let solver = UltimateSolver::new(10.0, 0.1, 10, 20, Some(42)).with_quantum_dims(1, 10, 1);

    group.bench_function("ultimate_solver_50_vars", |b| {
        b.iter(|| solver.solve(&model, &[]))
    });
    group.finish();
}

fn bench_ultimate_solver_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultimate_solver_large");
    group.measurement_time(Duration::from_secs(10));

    let num_vars = 200;
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..num_vars {
        for j in 0..num_vars {
            if i != j {
                col_indices.push(j);
                values.push(0.01);
            }
        }
        row_offsets.push(col_indices.len());
    }

    let model = QuboModel {
        num_vars,
        linear: vec![0.1; num_vars],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    };

    use ising_engine::solver::ultimate::UltimateSolver;
    let solver = UltimateSolver::new(10.0, 0.1, 5, 10, Some(42)).with_quantum_dims(1, 12, 1);

    group.bench_function("ultimate_solver_200_vars", |b| {
        b.iter(|| solver.solve(&model, &[]))
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_solver,
    bench_ultimate_solver,
    bench_ultimate_solver_large
);
criterion_main!(benches);
