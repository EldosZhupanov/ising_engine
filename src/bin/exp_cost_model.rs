//! Vector-2 probe: is the operator cost model a function of the right variables?
//!
//! Every `cost_model` in the operator library is a pure function of instance
//! SHAPE — `(n, num_pairs, num_replicas)`. None depends on state, temperature or
//! acceptance. But `SparseBitSlice::apply_flips` dispatches on **flip count**:
//!
//! ```text
//! let flips = mask.words().iter().map(|x| x.count_ones()).sum();
//! if flips * 3 >= r { apply_flips_dense(..) }   // O(deg·r)
//! else              { apply_flips_scattered(..) } // O(deg·flips)
//! ```
//!
//! So the true cost carries a hidden complexity parameter — the **flip density**
//! φ = flips/r — with a regime change at φ = 1/3. The declared model is blind to
//! it. `runtime.rs:298` feeds the declared work to the profiler, and the
//! scheduler's utility model divides quality by that cost, so the error is not
//! cosmetic: it biases budget allocation as a function of temperature.
//!
//! PRE-REGISTERED PREDICTION
//!   - declared work: IDENTICAL at every temperature (shape-only)
//!   - actual ms:     rises monotonically with acceptance, varying > 2x
//!   - cost-model error ms/work therefore varies > 2x, maximal in the cold regime
//!
//! REFUTED if actual ms is flat in temperature (< 20% spread), since this
//! host's same-code drift is ~9%.
//!
//! Temperatures are INTERLEAVED across repetitions and the MEDIAN is taken, so
//! monotonic host drift cannot manufacture a monotonic trend.
//!
//! ```text
//! cargo run --release --bin exp_cost_model -- --file benchmark_suite/data/gset/G1
//! ```

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::operator::{Budget, InstanceShape, Operator};
use ising_engine::engine_v2::operators::MetropolisSweep;
use ising_engine::engine_v2::runtime::RuntimeView;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == flag)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = v.len() / 2;
    if v.len().is_multiple_of(2) {
        (v[m - 1] + v[m]) / 2.0
    } else {
        v[m]
    }
}

fn main() {
    let file: String = arg("--file", "benchmark_suite/data/gset/G1".to_string());
    let r: usize = arg("--replicas", 32);
    let sweeps: u32 = arg("--sweeps", 20);
    let reps: usize = arg("--reps", 7);

    let Ok(text) = std::fs::read_to_string(&file) else {
        eprintln!("cannot read {file}");
        std::process::exit(1);
    };
    let Ok(ir) = rudy_maxcut_ir(&text) else {
        eprintln!("cannot parse {file}");
        std::process::exit(1);
    };

    // The declared cost — shape only, so it is a single constant for this run.
    let declared = MetropolisSweep::new()
        .cost_model(InstanceShape {
            n: ir.n,
            num_pairs: ir.col_idx.len() / 2,
            num_replicas: r,
        })
        .work_per_sweep
        * sweeps as f64;

    let temps_hi = [0.05f64, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
    let mut samples: Vec<Vec<f64>> = vec![Vec::new(); temps_hi.len()];
    let mut acc: Vec<f64> = vec![0.0; temps_hi.len()];

    println!(
        "# cost-model probe · {} · n={} pairs={} r={} sweeps={} reps={}",
        file,
        ir.n,
        ir.col_idx.len() / 2,
        r,
        sweeps,
        reps
    );
    println!("# declared work (shape-only, constant by construction) = {declared:.0}");
    println!("# temperatures INTERLEAVED per repetition; median reported\n");

    // Interleave: rep-major, temperature-minor. Monotonic host drift then lands
    // on every temperature equally and cannot fake a monotonic trend.
    for _rep in 0..reps {
        for (ti, &t_hi) in temps_hi.iter().enumerate() {
            let temps: Vec<f64> = (0..r)
                .map(|k| {
                    if r <= 1 {
                        t_hi
                    } else {
                        t_hi * (0.1f64 / t_hi).powf(k as f64 / (r - 1) as f64)
                    }
                })
                .collect();
            let init = vec![0u8; ir.n];
            let Ok(mut st) = SparseBitSlice::new(&ir, r, &init) else {
                eprintln!("SparseBitSlice rejected this instance (non-integral weights?)");
                std::process::exit(1);
            };
            let view = RuntimeView {
                iteration: 0,
                temperatures: &temps,
                num_replicas: r,
                recent_acceptance: 0.0,
                remaining_ms: f64::INFINITY,
            };
            let mut op = MetropolisSweep::new();
            let mut rng = ChaCha8Rng::seed_from_u64(7);

            let t0 = Instant::now();
            let rep_out = op.apply(&mut st, &view, &mut rng, Budget { sweeps });
            let ms = t0.elapsed().as_secs_f64() * 1000.0;

            samples[ti].push(ms);
            acc[ti] = rep_out.acceptance_rate();
        }
    }

    println!(
        "{:>8} {:>10} {:>12} {:>12} {:>14} {:>10}",
        "temp_hi", "accept", "phi=flips/r", "median ms", "ms per Mwork", "regime"
    );
    let mut ratios = Vec::new();
    for (ti, &t_hi) in temps_hi.iter().enumerate() {
        let ms = median(&mut samples[ti]);
        // Acceptance IS the expected flip density: each replica flips
        // independently with probability = acceptance, so E[flips]/r = accept.
        let phi = acc[ti];
        let ratio = ms / (declared / 1.0e6);
        ratios.push(ratio);
        println!(
            "{t_hi:>8} {:>10.4} {phi:>12.4} {ms:>12.3} {ratio:>14.4} {:>10}",
            acc[ti],
            if phi * 3.0 >= 1.0 {
                "DENSE"
            } else {
                "scattered"
            }
        );
    }

    let lo = ratios.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = ratios.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!("\n# declared work is CONSTANT across every row above, by construction.");
    println!(
        "# cost-model error spread = {:.2}x  (min {:.4}, max {:.4} ms per Mwork)",
        hi / lo,
        lo,
        hi
    );
    println!("# PREDICTION: >2x spread confirms the model omits a real cost variable.");
    println!("#             <1.2x refutes it (host same-code drift is ~9%).");
}
