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

    group.bench_function("parallel_tempering_small", |b| b.iter(|| solver.solve(&model, &[])));
    group.finish();
}

criterion_group!(benches, bench_solver);
criterion_main!(benches);
