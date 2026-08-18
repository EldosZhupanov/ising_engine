//! Vector-2 probe: does the chain mix, and were this session's measurements
//! ever equilibrated?
//!
//! RC-003 argued that temporal and ensemble linkage statistics coincide *at
//! equilibrium* (ergodic theorem), and used that to sharpen a hypothesis. RC-002
//! separately found that initialization DIVERSITY survives to `temp_hi = 4.0`.
//! Those two facts are in tension: surviving memory of the initial condition is
//! precisely a statement that the chain has NOT mixed. The ergodic premise was
//! never tested.
//!
//! Standard two-chain coupling diagnostic. Start two ensembles from INDEPENDENT
//! random configurations, run identical dynamics, and watch whether they become
//! statistically indistinguishable:
//!
//!   - `dE`   = |mean energy of A − mean energy of B|, scale-normalised
//!   - `q_AB` = mean cross-ensemble overlap (A replica vs B replica)
//!   - `q_AA` = mean within-A overlap — the reference an equilibrated chain
//!     must converge to, since at equilibrium "which chain" is meaningless and
//!     cross- and within-overlap must agree
//!
//! MIXED  ⇔ the two ensemble means agree within 2 sigma of their sampling error
//! (NOT an absolute threshold — see `mean_energy_se`) AND q_AB → q_AA.
//! If q_AB stays below q_AA the chains still remember where they started, so the
//! ensemble is out of equilibrium and any equilibrium argument about it is
//! inapplicable — including RC-003's.
//!
//! PRE-REGISTERED: at low temperature the two chains never converge within
//! budget (broken ergodicity); at high temperature they converge quickly. The
//! crossover locates the ergodicity boundary and therefore bounds which of this
//! session's measurements could have been equilibrium measurements at all.
//! Refuted if q_AB ≈ q_AA at every temperature by ~100 sweeps, which would mean
//! the chain mixes fast and the ergodic premise was safe.
//!
//! ```text
//! cargo run --release --bin exp_mixing_time -- --file benchmark_suite/data/gset/G11
//! ```

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{Budget, Operator};
use ising_engine::engine_v2::operators::MetropolisSweep;
use ising_engine::engine_v2::runtime::RuntimeView;
use ising_engine::engine_v2::state::SpinState;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == flag)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Deterministic independent random configuration.
fn rand_state(n: usize, seed: u64) -> Vec<u8> {
    let mut h = seed;
    (0..n)
        .map(|_| {
            h = h
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((h >> 33) & 1) as u8
        })
        .collect()
}

/// Mean |overlap| between every replica of `a` and every replica of `b`.
/// Absolute value because zero-field Ising has an exact Z2 gauge symmetry: a
/// replica and its global flip are the same physical state, so signed overlap
/// would average to zero for reasons that have nothing to do with mixing.
fn cross_overlap(a: &dyn SpinState, b: &dyn SpinState, n: usize) -> f64 {
    let (ra, rb) = (a.num_replicas(), b.num_replicas());
    let (mut xa, mut xb) = (vec![0u8; n], vec![0u8; n]);
    let mut sum = 0.0;
    let mut cnt = 0usize;
    for i in 0..ra {
        a.extract_into(i, &mut xa);
        for j in 0..rb {
            b.extract_into(j, &mut xb);
            let agree = xa.iter().zip(&xb).filter(|(p, q)| p == q).count();
            sum += ((2 * agree) as f64 / n as f64 - 1.0).abs();
            cnt += 1;
        }
    }
    sum / cnt.max(1) as f64
}

/// Mean |overlap| over distinct replica pairs inside one ensemble.
fn within_overlap(a: &dyn SpinState, n: usize) -> f64 {
    let r = a.num_replicas();
    let (mut xi, mut xj) = (vec![0u8; n], vec![0u8; n]);
    let mut sum = 0.0;
    let mut cnt = 0usize;
    for i in 0..r {
        a.extract_into(i, &mut xi);
        for j in (i + 1)..r {
            a.extract_into(j, &mut xj);
            let agree = xi.iter().zip(&xj).filter(|(p, q)| p == q).count();
            sum += ((2 * agree) as f64 / n as f64 - 1.0).abs();
            cnt += 1;
        }
    }
    sum / cnt.max(1) as f64
}

/// Mean energy AND the standard error of that mean. The SE is the correct null
/// for comparing two independent ensembles: at finite R the two means differ by
/// O(sigma/sqrt(R)) from sampling alone, so an absolute threshold below that
/// floor reports "not mixed" for every chain no matter how well it mixes.
fn mean_energy_se(s: &dyn SpinState) -> (f64, f64) {
    let r = s.num_replicas();
    let mut e = vec![0.0; r];
    s.energies_into(&mut e);
    let m = e.iter().sum::<f64>() / r as f64;
    if r < 2 {
        return (m, 0.0);
    }
    let var = e.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (r - 1) as f64;
    (m, (var / r as f64).sqrt())
}

fn ladder(hi: f64, lo: f64, r: usize) -> Vec<f64> {
    if r <= 1 {
        return vec![lo];
    }
    (0..r)
        .map(|k| hi * (lo / hi).powf(k as f64 / (r - 1) as f64))
        .collect()
}

fn main() {
    let file: String = arg("--file", "benchmark_suite/data/gset/G11".to_string());
    let r: usize = arg("--replicas", 16);
    let total: u32 = arg("--sweeps", 800);
    let step: u32 = arg("--step", 50);

    let Ok(text) = std::fs::read_to_string(&file) else {
        eprintln!("cannot read {file}");
        std::process::exit(1);
    };
    let Ok(ir): Result<ProblemIR, _> = rudy_maxcut_ir(&text) else {
        eprintln!("cannot parse {file}");
        std::process::exit(1);
    };

    println!(
        "# two-chain mixing diagnostic · {} · n={} r={} up to {} sweeps",
        file, ir.n, r, total
    );
    println!("# MIXED iff dE -> 0 AND q_AB -> q_AA.  q_AB < q_AA => still remembers init.\n");

    for &t_hi in &[0.1f64, 0.5, 1.0, 2.0, 4.0, 8.0] {
        // Flat ladder: a single temperature, so "mixing" is unambiguous. A PT
        // ladder would confound chain mixing with replica exchange.
        let temps = ladder(t_hi, t_hi, r);
        let xa = rand_state(ir.n, 0xA11CE);
        let xb = rand_state(ir.n, 0xB0B0B);
        let Ok(mut a) = SparseBitSlice::new(&ir, r, &xa) else {
            eprintln!("backend rejected instance");
            std::process::exit(1);
        };
        let Ok(mut b) = SparseBitSlice::new(&ir, r, &xb) else {
            eprintln!("backend rejected instance");
            std::process::exit(1);
        };
        // Distinct RNG streams: these are genuinely independent chains.
        let mut rng_a = ChaCha8Rng::seed_from_u64(11);
        let mut rng_b = ChaCha8Rng::seed_from_u64(22);
        let (mut op_a, mut op_b) = (MetropolisSweep::new(), MetropolisSweep::new());
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };

        println!("T = {t_hi} (flat ladder)");
        println!(
            "{:>8} {:>12} {:>10} {:>10} {:>10} {:>12}",
            "sweeps", "dE (rel)", "E agree?", "q_AB", "q_AA", "verdict"
        );

        let mut done = 0u32;
        while done < total {
            let n_this = step.min(total - done);
            op_a.apply(&mut a, &view, &mut rng_a, Budget { sweeps: n_this });
            op_b.apply(&mut b, &view, &mut rng_b, Budget { sweeps: n_this });
            done += n_this;

            let ((ea, sea), (eb, seb)) = (mean_energy_se(&a), mean_energy_se(&b));
            let de = (ea - eb).abs() / ea.abs().max(1e-9);
            // 2-sigma on the difference of two independent means.
            let tol = 2.0 * (sea * sea + seb * seb).sqrt();
            let e_agree = (ea - eb).abs() <= tol;
            let qab = cross_overlap(&a, &b, ir.n);
            let qaa = within_overlap(&a, ir.n);
            // Mixed: energies agree to 0.1% AND cross-overlap has reached
            // within 10% of the within-ensemble reference.
            let mixed = e_agree && (qaa - qab).abs() <= 0.1 * qaa.max(1e-9);
            if done.is_multiple_of(step * 4) || done == total {
                println!(
                    "{done:>8} {de:>12.6} {:>10} {qab:>10.4} {qaa:>10.4} {:>12}",
                    if e_agree { "yes" } else { "NO" },
                    if mixed { "MIXED" } else { "not mixed" }
                );
            }
        }
        println!();
    }

    println!("# If q_AB stays materially below q_AA, the ensemble is OUT OF EQUILIBRIUM");
    println!("# and equilibrium arguments about it (e.g. RC-003's ergodic premise)");
    println!("# do not apply at these budgets.");
}
