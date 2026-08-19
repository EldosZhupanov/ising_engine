//! RC-020 — direct identification of marginal wall cost.
//!
//! Implements `research/PREREG_RC020_MARGINAL_WALL_COST.md` §11 and nothing
//! else. Every constant here is frozen by that document.
//!
//! WALL TIME ONLY (§1). No energy, cut or quality is computed, stored or
//! printed. `cost_model` is forbidden as a measure of cost (RC-005) and is never
//! read.
//!
//! The estimand is measured DIRECTLY: a window of exactly `W` sweeps is timed
//! inside a running trajectory and `b_hat = window_ms / W`. The k-independent
//! startup floor lies outside every timed window, so there is no regression, no
//! intercept, and no separability problem.
//!
//! ```text
//! cargo run --release --bin exp_marginal_cost -- --controls
//! ```

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{Budget, Operator};
use ising_engine::engine_v2::operators::{GibbsColorSweep, MetropolisSweep};
use ising_engine::engine_v2::runtime::RuntimeView;
use ising_engine::engine_v2::state::{SpinState, StateDigest};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

// ============================================================ FROZEN (§2)

const REPLICAS: usize = 32;
const TEMPS: [f64; 3] = [0.1, 0.5, 2.0];
const PREFIX_SWEEPS: u32 = 4;
const WINDOWS_A: [u32; 3] = [4, 8, 16];
const WINDOWS_B: [u32; 3] = [16, 8, 4];
const REPETITIONS: usize = 9;
const K_REF: f64 = 16.0;

const PILOT: [u64; 8] = [10001, 10002, 10003, 10004, 10005, 10006, 10007, 10008];
const HELD_IN: [u64; 8] = [11001, 11002, 11003, 11004, 11005, 11006, 11007, 11008];
const HELD_OUT: [u64; 8] = [12001, 12002, 12003, 12004, 12005, 12006, 12007, 12008];

/// §2.1. Every prior block, refused by value. `5001-5008` are burned by the
/// RC-017 abort and `7001-7008` by the RC-018 pilot; `6001-6008`, `8001-8008`
/// and `9001-9008` are reserved to other cycles and must not be opened here.
const FORBIDDEN_LO: [u64; 9] = [1001, 2001, 3001, 4001, 5001, 6001, 7001, 8001, 9001];

/// Amendment 1 §A1. §2 named no seed for the controls; this value is now frozen
/// by `PREREG_RC020_AMENDMENT_1.md` and may not be changed, extended into a
/// block, or supplemented without a further amendment. Amendment 1 §A3: it seeds
/// the Runtime-facing controls and the warmup only, never a recorded observation.
const CONTROL_SEED: u64 = 20001;

/// Amendment 1 §A1 — control-internal deterministic seeds, frozen alongside it.
const SYNTHETIC_CONTROL_SEED: u64 = 991;
const CONTROL_BOOTSTRAP_SEED: u64 = 993;

const CI_BASE_SEED: u64 = 20261201;
const PERM_BASE_SEED: u64 = 20261301;
const BOOTSTRAP_REPS: usize = 100_000;

/// §5, frozen numerically before any run. `PERF.md` same-code drift for this
/// host class.
const DRIFT_BOUND: f64 = 0.09;
const DEGENERATE_RESOLUTION_MULT: f64 = 100.0;
const LOAD_BOUND: f64 = 2.0;
const MAX_DISCARDED_REPS: usize = 2;
/// §7.1 A1 precision and A2 stationarity.
const PRECISION_BOUND: f64 = 0.10;
const STATIONARITY_BOUND: f64 = 0.10;
/// §4 P1 synthetic recovery tolerance.
const SYNTHETIC_TOL: f64 = 0.05;
/// §3.7. Four logical CPUs, pinned for the whole session.
const CPU_SET: &str = "0-3";

/// §3.6. The sentinel: `metropolis_sweep` on G11 at T = 0.5, W = 8.
const SENTINEL_INSTANCE: usize = 1;
const SENTINEL_TEMP: f64 = 0.5;
const SENTINEL_WINDOW: u32 = 8;

const PREREG_PATH: &str = "research/PREREG_RC020_MARGINAL_WALL_COST.md";
const DESCENDANT_HEADING: &str = "## 9. Falsifiable descendant";
/// §9 is frozen blank. While this marker is present, held-in and held-out are
/// unreachable (§10.7).
const DESCENDANT_BLANK_MARKER: &str =
    "**This section is intentionally empty and is frozen in that state.**";

struct Inst {
    name: &'static str,
    path: &'static str,
    index: usize,
    sha12: &'static str,
}

const CORPUS: [Inst; 6] = [
    Inst {
        name: "G1",
        path: "benchmark_suite/data/gset/G1",
        index: 0,
        sha12: "73bf704d8ffc",
    },
    Inst {
        name: "G11",
        path: "benchmark_suite/data/gset/G11",
        index: 3,
        sha12: "c2a760d2926d",
    },
    Inst {
        name: "G14",
        path: "benchmark_suite/data/gset/G14",
        index: 6,
        sha12: "dc769b978a40",
    },
    Inst {
        name: "G22",
        path: "benchmark_suite/data/gset/G22",
        index: 9,
        sha12: "9baeee06eb14",
    },
    Inst {
        name: "G32",
        path: "benchmark_suite/data/gset/G32",
        index: 12,
        sha12: "9760fce6b601",
    },
    Inst {
        name: "G43",
        path: "benchmark_suite/data/gset/G43",
        index: 17,
        sha12: "9af5445b4b06",
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    Metropolis,
    Gibbs,
}

impl Op {
    fn tag(self) -> &'static str {
        match self {
            Op::Metropolis => "metropolis_sweep",
            Op::Gibbs => "gibbs_color_sweep",
        }
    }
    const ALL: [Op; 2] = [Op::Metropolis, Op::Gibbs];
}

/// §2. Window-order arms; `code` feeds the CI seed formula.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WinArm {
    A,
    B,
}

impl WinArm {
    fn tag(self) -> &'static str {
        match self {
            WinArm::A => "A",
            WinArm::B => "B",
        }
    }
    fn windows(self) -> [u32; 3] {
        match self {
            WinArm::A => WINDOWS_A,
            WinArm::B => WINDOWS_B,
        }
    }
    fn code(self) -> u64 {
        match self {
            WinArm::A => 0,
            WinArm::B => 1,
        }
    }
    const ALL: [WinArm; 2] = [WinArm::A, WinArm::B];
}

// ==================================================================== CLI

fn flag(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}

fn arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

// ================================================================= SHA-256
// Minimal local implementation; no dependency is added. Pinned by a KAT test.

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bitlen = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    let mut w = [0u32; 64];
    for chunk in msg.chunks_exact(64) {
        for (i, wi) in w.iter_mut().enumerate().take(16) {
            let j = i * 4;
            *wi = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        let (mut e, mut f, mut g, mut hh) = (h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K256[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (hv, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *hv = hv.wrapping_add(v);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

// =========================================================== PURE NUMERICS

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = v.len() / 2;
    if v.len().is_multiple_of(2) {
        (v[m - 1] + v[m]) / 2.0
    } else {
        v[m]
    }
}

/// §5. `spread = max/min - 1`, the sentinel drift statistic. Non-finite or
/// non-positive inputs yield infinity so they can only fail, never pass.
fn spread(a: f64, b: f64) -> f64 {
    if !(a.is_finite() && b.is_finite()) || a <= 0.0 || b <= 0.0 {
        return f64::INFINITY;
    }
    a.max(b) / a.min(b) - 1.0
}

/// §5. The sentinel rule: a repetition is discarded when drift exceeds the
/// frozen bound.
fn drift_exceeded(first_ms: f64, last_ms: f64) -> bool {
    spread(first_ms, last_ms) > DRIFT_BOUND
}

/// §5. Both guards, and no clamping. NaN lands in the degenerate branch.
fn degenerate_timing(window_ms: f64, resolution_ms: f64) -> bool {
    !window_ms.is_finite()
        || window_ms <= 0.0
        || window_ms < DEGENERATE_RESOLUTION_MULT * resolution_ms
}

/// §6.1. The directly measured marginal cost. No intercept is estimated.
fn b_hat(window_ms: f64, w: u32) -> f64 {
    window_ms / w as f64
}

/// §6.5. `k'_real = (b_M / b_G) * k_ref`.
fn k_real(b_m: f64, b_g: f64) -> Option<f64> {
    if !(b_m.is_finite() && b_g.is_finite()) || b_g <= 0.0 {
        return None;
    }
    Some((b_m / b_g) * K_REF)
}

/// §2. `CI_BASE_SEED + 2*corpus_index + arm_code`.
fn ci_seed(corpus_index: usize, arm: WinArm) -> u64 {
    CI_BASE_SEED + 2 * corpus_index as u64 + arm.code()
}

/// §6.3. Deterministic percentile 95% bootstrap.
fn bootstrap_ci(xs: &[f64], reps: usize, seed: u64) -> (f64, f64) {
    if xs.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut means = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut s = 0.0;
        for _ in 0..xs.len() {
            s += xs[rng.gen_range(0..xs.len())];
        }
        means.push(s / xs.len() as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (means[reps / 40], means[reps - 1 - reps / 40])
}

/// §3.5. Deterministic Fisher-Yates.
fn permutation(len: usize, seed: u64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..len).collect();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for i in (1..len).rev() {
        idx.swap(i, rng.gen_range(0..=i));
    }
    idx
}

// ================================================================= ROUTING

/// §7.1 / §5. The three outcomes are distinct and are never conflated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CellRoute {
    Qualifies,
    /// §5: window-length estimates disagree. Neither qualifying nor a null.
    Unstable,
    /// §7.1 A1 or A3 failed.
    NotIdentified,
}

/// §7.1. A1 precision, A2 stationarity, A3 positivity, evaluated in that order.
fn route_cell(b: f64, ci: (f64, f64), by_window: &[f64]) -> CellRoute {
    let a3 = b > 0.0 && ci.0 > 0.0;
    let a1 = b.is_finite() && b != 0.0 && ((ci.1 - ci.0) / 2.0) / b.abs() <= PRECISION_BOUND;
    if !(a1 && a3) {
        return CellRoute::NotIdentified;
    }
    let mut v = by_window.to_vec();
    let m = median(&mut v);
    let stationary = m > 0.0
        && by_window
            .iter()
            .all(|x| (x - m).abs() / m <= STATIONARITY_BOUND);
    if stationary {
        CellRoute::Qualifies
    } else {
        CellRoute::Unstable
    }
}

/// §7.3. Stated on the continuous width, deliberately never on integer
/// stability (§0 lesson 1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Feasibility {
    Feasible,
    ReducedResolution,
    NotFeasible,
}

fn feasibility(ci_width_sweeps: f64) -> Feasibility {
    if !ci_width_sweeps.is_finite() || ci_width_sweeps >= 3.0 {
        Feasibility::NotFeasible
    } else if ci_width_sweeps < 1.0 {
        Feasibility::Feasible
    } else {
        Feasibility::ReducedResolution
    }
}

// ============================================================== ENVIRONMENT

/// §3.7. Smallest observable non-zero clock delta, in ms.
fn timer_resolution_ms() -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..64 {
        let t0 = Instant::now();
        loop {
            let d = t0.elapsed();
            if !d.is_zero() {
                best = best.min(d.as_secs_f64() * 1000.0);
                break;
            }
        }
    }
    if best.is_finite() {
        best
    } else {
        0.0
    }
}

/// §3.7. 1-minute load average.
fn load_avg() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| s.split_whitespace().next().and_then(|v| v.parse().ok()))
        .unwrap_or(f64::NAN)
}

fn thread_count() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Threads:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
        })
        .unwrap_or(0)
}

fn cpus_allowed() -> String {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Cpus_allowed_list:"))
                .map(|l| l.split_whitespace().nth(1).unwrap_or("?").to_string())
        })
        .unwrap_or_else(|| "?".to_string())
}

/// §3.7. Pin this process for the whole session. `taskset` sets the affinity of
/// a running pid, so no dependency is added.
fn pin_cpus(set: &str) -> bool {
    let pid = std::process::id().to_string();
    let ok = std::process::Command::new("taskset")
        .args(["-cp", set, &pid])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    ok && cpus_allowed() == set
}

// ================================================================ EXECUTION

fn isothermal(t: f64) -> Vec<f64> {
    vec![t; REPLICAS]
}

fn make_op(op: Op) -> Box<dyn Operator> {
    match op {
        Op::Metropolis => Box::new(MetropolisSweep::new()),
        Op::Gibbs => Box::new(GibbsColorSweep::new()),
    }
}

fn view(temps: &[f64]) -> RuntimeView<'_> {
    RuntimeView {
        iteration: 0,
        temperatures: temps,
        num_replicas: REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    }
}

fn fresh_state(ir: &ProblemIR) -> Option<SparseBitSlice<'_>> {
    let init = vec![0u8; ir.n];
    SparseBitSlice::new(ir, REPLICAS, &init).ok()
}

fn load(path: &str) -> Result<ProblemIR, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    rudy_maxcut_ir(&text)
}

/// §3.1-§3.2. One trajectory: one state, one RNG stream. The prefix and all
/// three windows are consecutive `apply` calls, each window bracketed by its own
/// clock reads. Nothing here subtracts timings from different executions.
struct Trajectory {
    /// Directly measured, in the arm's window order.
    window_ms: [f64; 3],
}

fn run_trajectory(ir: &ProblemIR, op: Op, seed: u64, temp: f64, arm: WinArm) -> Option<Trajectory> {
    let temps = isothermal(temp);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut o = make_op(op);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Prefix: untimed and discarded (§2).
    let _ = o.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: PREFIX_SWEEPS,
        },
    );

    let mut window_ms = [0.0f64; 3];
    for (slot, &w) in arm.windows().iter().enumerate() {
        let t0 = Instant::now();
        let _ = o.apply(&mut st, &v, &mut rng, Budget { sweeps: w });
        window_ms[slot] = t0.elapsed().as_secs_f64() * 1000.0;
    }
    Some(Trajectory { window_ms })
}

struct Probes {
    state: StateDigest,
    energy: Vec<u64>,
    rng: u64,
}

fn energy_bits(st: &SparseBitSlice<'_>) -> Vec<u64> {
    let mut e = vec![0.0f64; REPLICAS];
    st.energies_into(&mut e);
    e.iter().map(|v| v.to_bits()).collect()
}

/// N3: the instrumented trajectory's probes.
fn instrumented_probes(
    ir: &ProblemIR,
    op: Op,
    seed: u64,
    temp: f64,
    arm: WinArm,
) -> Option<Probes> {
    let temps = isothermal(temp);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut o = make_op(op);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let _ = o.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: PREFIX_SWEEPS,
        },
    );
    for &w in arm.windows().iter() {
        let _ = o.apply(&mut st, &v, &mut rng, Budget { sweeps: w });
    }
    Some(Probes {
        state: st.digest(),
        energy: energy_bits(&st),
        rng: rng.r#gen::<u64>(),
    })
}

/// N3: the same total sweep count executed as ONE uninterrupted call.
fn uninterrupted_probes(ir: &ProblemIR, op: Op, seed: u64, temp: f64) -> Option<Probes> {
    let total = PREFIX_SWEEPS + WINDOWS_A.iter().sum::<u32>();
    let temps = isothermal(temp);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut o = make_op(op);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let _ = o.apply(&mut st, &v, &mut rng, Budget { sweeps: total });
    Some(Probes {
        state: st.digest(),
        energy: energy_bits(&st),
        rng: rng.r#gen::<u64>(),
    })
}

/// §3.6. The sentinel, executed first and last in every repetition.
/// `inject_extra_sweeps` exists solely so P3 can make the guard fail on demand:
/// a guard that cannot fail is not a guard (§4 P3).
fn run_sentinel(ir: &ProblemIR, seed: u64, inject_extra_sweeps: u32) -> Option<f64> {
    let temps = isothermal(SENTINEL_TEMP);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut o = make_op(Op::Metropolis);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let _ = o.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: PREFIX_SWEEPS,
        },
    );
    let t0 = Instant::now();
    let _ = o.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: SENTINEL_WINDOW + inject_extra_sweeps,
        },
    );
    Some(t0.elapsed().as_secs_f64() * 1000.0)
}

/// §3.3. One complete trajectory per (instance, operator), discarded.
fn warmup(ir: &ProblemIR, op: Op, seed: u64) {
    let _ = run_trajectory(ir, op, seed, TEMPS[1], WinArm::A);
}

// ========================================================= PROVENANCE (§10)

/// `output()` rather than `status()`: `status` inherits stdout, so `ls-files`
/// and `log` would print into the control record and pollute the evidence.
fn git_ok(args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Facts gathered separately so the decision is pure and testable without git.
#[derive(Clone, Copy, Debug)]
struct Facts {
    exists: bool,
    tracked: bool,
    clean: bool,
    committed: bool,
    descendant_blank: bool,
}

/// §10.1-§10.2. Pure: same facts -> same verdict, naming the failed condition.
fn prereg_gate(f: &Facts) -> Result<(), &'static str> {
    if !f.exists {
        return Err("pre-registration file is missing");
    }
    if !f.tracked {
        return Err("pre-registration is not tracked by git");
    }
    if !f.clean {
        return Err("pre-registration has uncommitted modifications");
    }
    if !f.committed {
        return Err("pre-registration has never been committed");
    }
    Ok(())
}

/// §10.7. While §9 is blank, held-in and held-out are unreachable.
fn reserved_blocks_locked(f: &Facts) -> bool {
    f.descendant_blank
}

fn facts() -> Facts {
    let text = std::fs::read_to_string(PREREG_PATH).unwrap_or_default();
    Facts {
        exists: std::path::Path::new(PREREG_PATH).exists(),
        tracked: git_ok(&["ls-files", "--error-unmatch", PREREG_PATH]),
        clean: git_ok(&["diff", "--quiet", "HEAD", "--", PREREG_PATH]),
        committed: git_ok(&["log", "-1", "--format=%ct", "--", PREREG_PATH]),
        descendant_blank: text.contains(DESCENDANT_HEADING)
            && text.contains(DESCENDANT_BLANK_MARKER),
    }
}

/// §2.1 / §10.4. Refusal by value, so no forbidden seed can reach execution.
fn seed_forbidden(seed: u64) -> bool {
    FORBIDDEN_LO.iter().any(|&lo| seed >= lo && seed <= lo + 7)
}

/// Amendment 1 §A2 — every seed-valued family RC-020 names, enumerated so the
/// disjointness claim is checked rather than asserted.
fn named_seed_values() -> Vec<u64> {
    let mut v = Vec::new();
    for lo in FORBIDDEN_LO {
        v.extend(lo..=lo + 7);
    }
    v.extend(PILOT);
    v.extend(HELD_IN);
    v.extend(HELD_OUT);
    for i in CORPUS.iter() {
        for a in WinArm::ALL {
            v.push(ci_seed(i.index, a));
        }
    }
    for r in 0..REPETITIONS {
        v.push(PERM_BASE_SEED + r as u64);
    }
    v
}

/// Amendment 1 §A4.8. A collision is a **Class I** condition — instrument
/// invalid — never a Class II null. Checked before any control executes.
fn control_seeds_disjoint() -> Result<(), String> {
    let named = named_seed_values();
    for (tag, s) in [
        ("CONTROL_SEED", CONTROL_SEED),
        ("SYNTHETIC_CONTROL_SEED", SYNTHETIC_CONTROL_SEED),
        ("CONTROL_BOOTSTRAP_SEED", CONTROL_BOOTSTRAP_SEED),
    ] {
        if named.contains(&s) {
            return Err(format!("{tag} = {s} collides with a named seed value"));
        }
    }
    if CONTROL_SEED == SYNTHETIC_CONTROL_SEED
        || CONTROL_SEED == CONTROL_BOOTSTRAP_SEED
        || SYNTHETIC_CONTROL_SEED == CONTROL_BOOTSTRAP_SEED
    {
        return Err("the three control seeds are not pairwise distinct".into());
    }
    Ok(())
}

fn verify_corpus() -> Result<(), String> {
    for i in CORPUS.iter() {
        let bytes = std::fs::read(i.path).map_err(|e| format!("{}: {e}", i.path))?;
        let got = sha256_hex(&bytes);
        if got[..12] != *i.sha12 {
            return Err(format!(
                "{} hash mismatch: expected {} got {}",
                i.name,
                i.sha12,
                &got[..12]
            ));
        }
    }
    Ok(())
}

// ================================================================ SCHEMA (§10.1)

/// Exactly the 22 columns of §10.1, in that order. No outcome column exists and
/// none may be added. Every guard of §5 has a column, so a guard that did not
/// run is visible in the data and not only in the code.
const SCHEMA: [&str; 22] = [
    "instance",
    "corpus_index",
    "operator",
    "temp",
    "arm",
    "window_w",
    "seed",
    "repetition",
    "window_ms",
    "b_hat",
    "sentinel_first_ms",
    "sentinel_last_ms",
    "sentinel_spread",
    "timer_resolution_ms",
    "load_avg_start",
    "load_avg_end",
    "cpu_set",
    "thread_count",
    "discarded",
    "discard_reason",
    "degenerate",
    "ci_seed",
];

fn header() -> String {
    let mut s = SCHEMA.join("\t");
    s.push('\n');
    s
}

// ================================================================= CONTROLS

/// P1 / N2 operate on constructed data with a KNOWN per-unit cost, so they
/// validate the estimator independently of the Runtime and cannot flake.
fn synthetic_windows(per_sweep: f64, jitter: f64, seed: u64) -> Vec<(u32, f64)> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut v = Vec::new();
    for _ in 0..REPETITIONS {
        for &w in WINDOWS_A.iter() {
            let j = (rng.r#gen::<f64>() - 0.5) * 2.0 * jitter;
            v.push((w, per_sweep * w as f64 * (1.0 + j)));
        }
    }
    v
}

/// §5/§7 — print the frozen decision rules and exercise each one, so the guards
/// that the pilot body will apply are demonstrably live and inspectable at
/// controls time rather than only at pilot time. This is the answer to the
/// RC-018 failure mode: a rule that is never exercised is indistinguishable
/// from a rule that does not exist.
fn print_frozen_protocol(resolution_ms: f64) {
    println!("\n[§5/§7] frozen decision rules, exercised");

    let dmin = DEGENERATE_RESOLUTION_MULT * resolution_ms;
    println!(
        "    degenerate below {dmin:.6} ms: at {:.6} ms -> {}, at {:.6} ms -> {}",
        dmin * 0.5,
        degenerate_timing(dmin * 0.5, resolution_ms),
        dmin * 2.0,
        degenerate_timing(dmin * 2.0, resolution_ms)
    );
    println!(
        "    drift bound {DRIFT_BOUND}: 5% -> flagged {}, 10% -> flagged {}",
        drift_exceeded(100.0, 105.0),
        drift_exceeded(100.0, 110.0)
    );
    println!(
        "    session aborts above {MAX_DISCARDED_REPS} discarded of {REPETITIONS} repetitions"
    );
    println!(
        "    A1 precision <= {PRECISION_BOUND}, A2 stationarity <= {STATIONARITY_BOUND}: \
         tight+stationary -> {:?}, tight+drifting -> {:?}, imprecise -> {:?}",
        route_cell(1.0, (0.98, 1.02), &[1.0, 1.02, 0.98]),
        route_cell(1.0, (0.98, 1.02), &[1.0, 1.5, 0.6]),
        route_cell(1.0, (0.5, 1.5), &[1.0, 1.0, 1.0])
    );
    println!(
        "    feasibility by CI width in sweeps: 0.5 -> {:?}, 2.0 -> {:?}, 3.0 -> {:?}",
        feasibility(0.5),
        feasibility(2.0),
        feasibility(3.0)
    );
    println!(
        "    CI seeds {}..{} over {} instances x {} arms",
        ci_seed(CORPUS[0].index, WinArm::A),
        ci_seed(CORPUS[CORPUS.len() - 1].index, WinArm::B),
        CORPUS.len(),
        WinArm::ALL.len()
    );
    let conds = CORPUS.len() * Op::ALL.len() * TEMPS.len() * WinArm::ALL.len();
    println!(
        "    interleave: {} conditions per repetition, permuted from seed {}",
        conds, PERM_BASE_SEED
    );
    let perm = permutation(conds, PERM_BASE_SEED);
    println!(
        "    repetition 0 order begins {:?}",
        &perm[..perm.len().min(6)]
    );
    println!(
        "    schema header: {} columns, {} tab separators",
        SCHEMA.len(),
        header().matches('\t').count()
    );
}

struct Controls {
    resolution_ms: f64,
    all_pass: bool,
}

fn run_controls() -> Controls {
    println!("RC-020 controls — PREREG §4, guards §5, environment §3.7\n");
    let mut ok = true;

    // Amendment 1 §A4.8 — before anything else, and Class I on failure.
    match control_seeds_disjoint() {
        Ok(()) => println!(
            "[A1] control seeds {CONTROL_SEED}/{SYNTHETIC_CONTROL_SEED}/{CONTROL_BOOTSTRAP_SEED} \
             disjoint from all {} named values",
            named_seed_values().len()
        ),
        Err(e) => {
            println!("[A1] control-seed disjointness: FAIL (Class I) — {e}");
            ok = false;
        }
    }

    // ---- environment (§3.7) -----------------------------------------------
    let pinned = pin_cpus(CPU_SET);
    println!(
        "[env] cpu pin {CPU_SET}: {} (allowed now: {})",
        if pinned { "ok" } else { "FAILED" },
        cpus_allowed()
    );
    ok &= pinned;
    let resolution_ms = timer_resolution_ms();
    println!(
        "[env] timer resolution: {resolution_ms:.6} ms (degenerate below {:.6} ms)",
        DEGENERATE_RESOLUTION_MULT * resolution_ms
    );
    let la = load_avg();
    let load_ok = la.is_finite() && la < LOAD_BOUND;
    println!(
        "[env] load average {la:.2} < {LOAD_BOUND}: {}",
        if load_ok { "ok" } else { "HOST-LOADED" }
    );
    ok &= load_ok;
    println!("[env] threads: {}", thread_count());

    // ---- P2: corpus hashes -------------------------------------------------
    match verify_corpus() {
        Ok(()) => println!(
            "[P2] corpus hashes: all {} match PREREG_RC016 §4",
            CORPUS.len()
        ),
        Err(e) => {
            println!("[P2] corpus hashes: FAIL — {e}");
            ok = false;
        }
    }

    // ---- P1: synthetic marginal recovery ----------------------------------
    let truth = 0.75_f64;
    let obs = synthetic_windows(truth, 0.01, SYNTHETIC_CONTROL_SEED);
    let mut bs: Vec<f64> = obs.iter().map(|&(w, ms)| b_hat(ms, w)).collect();
    let got = median(&mut bs);
    let err = (got - truth).abs() / truth;
    let kr = k_real(truth * 1.5, truth).unwrap_or(f64::NAN);
    let kr_got = k_real(got * 1.5, got).unwrap_or(f64::NAN);
    let kr_err = (kr_got - kr).abs() / kr;
    let p1 = err <= SYNTHETIC_TOL && kr_err <= SYNTHETIC_TOL;
    println!("[P1] synthetic marginal: b={got:.6} (truth {truth}, err {err:.4}); k'_real err {kr_err:.6} -> {}",
        if p1 { "PASS" } else { "FAIL" });
    ok &= p1;

    // ---- N2: zero dependence of b_hat on the window label -----------------
    let per_rep: Vec<f64> = (0..REPETITIONS)
        .map(|r| {
            let lo = r * WINDOWS_A.len();
            let big = b_hat(obs[lo + 2].1, obs[lo + 2].0);
            let small = b_hat(obs[lo].1, obs[lo].0);
            big - small
        })
        .collect();
    let (l, u) = bootstrap_ci(&per_rep, BOOTSTRAP_REPS, CONTROL_BOOTSTRAP_SEED);
    let n2 = l <= 0.0 && u >= 0.0;
    println!(
        "[N2] window-label null: CI [{l:.6}, {u:.6}] contains 0 -> {}",
        if n2 { "PASS" } else { "FAIL" }
    );
    ok &= n2;

    // ---- Runtime-facing controls ------------------------------------------
    let probe = &CORPUS[SENTINEL_INSTANCE];
    match load(probe.path) {
        Err(e) => {
            println!("[N1/N3/P3/P4] FAIL — cannot load {}: {e}", probe.path);
            ok = false;
        }
        Ok(ir) => {
            warmup(&ir, Op::Metropolis, CONTROL_SEED);

            // N1: same code, two labels.
            let a = run_trajectory(&ir, Op::Metropolis, CONTROL_SEED, TEMPS[1], WinArm::A);
            let b = run_trajectory(&ir, Op::Metropolis, CONTROL_SEED, TEMPS[1], WinArm::A);
            match (a, b) {
                (Some(x), Some(y)) => {
                    let s = spread(x.window_ms[1], y.window_ms[1]);
                    let pass = s <= DRIFT_BOUND;
                    println!("[N1] same-code null: {:.3} vs {:.3} ms, spread {s:.4} <= {DRIFT_BOUND} -> {}",
                        x.window_ms[1], y.window_ms[1], if pass { "PASS" } else { "HOST-UNSTABLE" });
                    ok &= pass;
                }
                _ => {
                    println!("[N1] same-code null: FAIL — backend rejected the instance");
                    ok = false;
                }
            }

            // N3: instrumented vs uninterrupted, all three probes.
            let mut n3 = true;
            for op in Op::ALL {
                let i = instrumented_probes(&ir, op, CONTROL_SEED, TEMPS[1], WinArm::A);
                let u = uninterrupted_probes(&ir, op, CONTROL_SEED, TEMPS[1]);
                match (i, u) {
                    (Some(p), Some(q)) => {
                        let (sd, eb, rp) =
                            (p.state == q.state, p.energy == q.energy, p.rng == q.rng);
                        let same = sd && eb && rp;
                        println!(
                            "[N3] {:<18} state {} energy {} rng {} -> {}",
                            op.tag(),
                            if sd { "match" } else { "DIFFER" },
                            if eb { "match" } else { "DIFFER" },
                            if rp { "match" } else { "DIFFER" },
                            if same { "PASS" } else { "FAIL" }
                        );
                        n3 &= same;
                    }
                    _ => {
                        println!("[N3] {:<18} FAIL — backend rejected", op.tag());
                        n3 = false;
                    }
                }
            }
            ok &= n3;

            // P3: the sentinel guard must FAIL ON DEMAND. A clean pair must pass
            // the drift rule and an injected slowdown must trip it.
            let clean_a = run_sentinel(&ir, CONTROL_SEED, 0);
            let clean_b = run_sentinel(&ir, CONTROL_SEED, 0);
            let slowed = run_sentinel(&ir, CONTROL_SEED, SENTINEL_WINDOW);
            match (clean_a, clean_b, slowed) {
                (Some(x), Some(y), Some(z)) => {
                    let clean_ok = !drift_exceeded(x, y);
                    let detected = drift_exceeded(x, z);
                    let pass = clean_ok && detected;
                    println!("[P3] sentinel liveness: clean {:.3}/{:.3} spread {:.4} not flagged={clean_ok}; \
                              injected {:.3} spread {:.4} flagged={detected} -> {}",
                        x, y, spread(x, y), z, spread(x, z), if pass { "PASS" } else { "FAIL" });
                    ok &= pass;
                }
                _ => {
                    println!("[P3] sentinel liveness: FAIL — sentinel could not execute");
                    ok = false;
                }
            }

            // P4: strict inequality only; no ratio threshold (§4 P4).
            let mut p4 = true;
            for arm in WinArm::ALL {
                let Some(t) = run_trajectory(&ir, Op::Metropolis, CONTROL_SEED, TEMPS[1], arm)
                else {
                    p4 = false;
                    continue;
                };
                let w = arm.windows();
                let big = w.iter().position(|&x| x == 16).unwrap_or(0);
                let small = w.iter().position(|&x| x == 4).unwrap_or(0);
                let pass = t.window_ms[big] > t.window_ms[small];
                println!(
                    "[P4] arm {}: W=16 {:.3} ms > W=4 {:.3} ms -> {}",
                    arm.tag(),
                    t.window_ms[big],
                    t.window_ms[small],
                    if pass { "PASS" } else { "FAIL" }
                );
                p4 &= pass;
            }
            ok &= p4;
        }
    }

    // ---- provenance (§10) --------------------------------------------------
    let f = facts();
    match prereg_gate(&f) {
        Ok(()) => println!("[§10] pre-registration tracked, clean and committed"),
        Err(e) => {
            println!("[§10] provenance: FAIL — {e}");
            ok = false;
        }
    }
    println!(
        "[§10.7] descendant §9 blank, reserved blocks locked: {}",
        reserved_blocks_locked(&f)
    );
    println!("[§10.1] artifact schema: {} columns", SCHEMA.len());

    print_frozen_protocol(resolution_ms);

    println!(
        "\nCONTROLS: {}",
        if ok {
            "ALL PASS"
        } else {
            "FAILED — instrument invalid (K1), no datum may be retained"
        }
    );
    Controls {
        resolution_ms,
        all_pass: ok,
    }
}

// ============================================================ PILOT (gated)

/// §10. Every condition is a gate, not a message. Nothing is retained unless all
/// of them hold — fail closed before any datum.
fn pilot_gates(dir: &str) -> Result<Env, String> {
    let f = facts();
    prereg_gate(&f).map_err(|e| e.to_string())?;
    control_seeds_disjoint()?;
    verify_corpus()?;
    for &s in PILOT.iter() {
        if seed_forbidden(s) {
            return Err(format!("seed {s} is in a forbidden prior block"));
        }
    }
    if !reserved_blocks_locked(&f) {
        return Err(
            "descendant §9 is no longer blank; RC-020's scope is fixed by this file".into(),
        );
    }
    let c = run_controls();
    if !c.all_pass {
        return Err("controls failed (K1): instrument invalid, no datum retained".into());
    }
    if !(c.resolution_ms.is_finite() && c.resolution_ms > 0.0) {
        return Err("timer resolution unmeasurable; §5 degeneracy bound undefined".into());
    }
    if std::path::Path::new(&format!("{dir}/rc020_pilot.tsv")).exists() {
        return Err(
            "a pilot artifact already exists; refusing to overwrite frozen evidence".into(),
        );
    }
    if !pin_cpus(CPU_SET) {
        return Err(format!("§3.7 requires the session pinned to {CPU_SET}"));
    }
    Ok(Env {
        resolution_ms: c.resolution_ms,
        cpu_set: cpus_allowed(),
        threads: thread_count(),
    })
}

// ============================================================== PILOT BODY

/// §3. The 72 conditions of one repetition, before permutation.
fn conditions() -> Vec<(usize, Op, f64, WinArm)> {
    let mut v = Vec::new();
    for (ii, _) in CORPUS.iter().enumerate() {
        for op in Op::ALL {
            for &t in TEMPS.iter() {
                for arm in WinArm::ALL {
                    v.push((ii, op, t, arm));
                }
            }
        }
    }
    v
}

/// One directly measured window, before the repetition-level fields are known.
struct Obs {
    inst: usize,
    op: Op,
    temp: f64,
    arm: WinArm,
    w: u32,
    seed: u64,
    window_ms: f64,
}

struct Env {
    resolution_ms: f64,
    cpu_set: String,
    threads: usize,
}

fn fmt_row(o: &Obs, rep: usize, sf: f64, sl: f64, las: f64, lae: f64, env: &Env) -> String {
    let inst = &CORPUS[o.inst];
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.8}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.2}\t{:.2}\t{}\t{}\tfalse\t-\t{}\t{}\n",
        inst.name,
        inst.index,
        o.op.tag(),
        o.temp,
        o.arm.tag(),
        o.w,
        o.seed,
        rep,
        o.window_ms,
        b_hat(o.window_ms, o.w),
        sf,
        sl,
        spread(sf, sl),
        env.resolution_ms,
        las,
        lae,
        env.cpu_set,
        env.threads,
        degenerate_timing(o.window_ms, env.resolution_ms),
        ci_seed(inst.index, o.arm)
    )
}

/// §5. A discarded repetition is written to the artifact with its reason; it is
/// never silently dropped.
fn fmt_discard(
    rep: usize,
    reason: &str,
    sf: f64,
    sl: f64,
    las: f64,
    lae: f64,
    env: &Env,
) -> String {
    format!(
        "-\t-\t-\t-\t-\t0\t0\t{}\t0.0\t0.0\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.2}\t{:.2}\t{}\t{}\ttrue\t{}\tfalse\t0\n",
        rep, sf, sl, spread(sf, sl), env.resolution_ms, las, lae, env.cpu_set, env.threads, reason
    )
}

/// One repetition: sentinel first, the permuted conditions, sentinel last.
/// Returns its rows, or the reason it must be discarded (§5).
fn run_repetition(
    irs: &[ProblemIR],
    rep: usize,
    env: &Env,
) -> Result<Vec<String>, (String, f64, f64, f64, f64)> {
    let las = load_avg();
    let sentinel_ir = &irs[SENTINEL_INSTANCE];

    // §5 HOST-LOADED is decided BEFORE execution, not after.
    if !(las.is_finite() && las < LOAD_BOUND) {
        return Err(("HOST-LOADED".into(), 0.0, 0.0, las, f64::NAN));
    }

    let Some(sf) = run_sentinel(sentinel_ir, CONTROL_SEED, 0) else {
        return Err(("SENTINEL-FAILED".into(), 0.0, 0.0, las, f64::NAN));
    };

    let conds = conditions();
    let order = permutation(conds.len(), PERM_BASE_SEED + rep as u64);
    let mut obs = Vec::with_capacity(conds.len() * PILOT.len() * WINDOWS_A.len());
    for &ci in order.iter() {
        let (ii, op, temp, arm) = conds[ci];
        for &seed in PILOT.iter() {
            let Some(t) = run_trajectory(&irs[ii], op, seed, temp, arm) else {
                return Err(("BACKEND-REJECTED".into(), sf, 0.0, las, f64::NAN));
            };
            for (slot, &w) in arm.windows().iter().enumerate() {
                obs.push(Obs {
                    inst: ii,
                    op,
                    temp,
                    arm,
                    w,
                    seed,
                    window_ms: t.window_ms[slot],
                });
            }
        }
    }

    let Some(sl) = run_sentinel(sentinel_ir, CONTROL_SEED, 0) else {
        return Err(("SENTINEL-FAILED".into(), sf, 0.0, las, f64::NAN));
    };
    let lae = load_avg();

    // §5 the sentinel rule.
    if drift_exceeded(sf, sl) {
        return Err(("SENTINEL-DRIFT".into(), sf, sl, las, lae));
    }

    Ok(obs
        .iter()
        .map(|o| fmt_row(o, rep, sf, sl, las, lae, env))
        .collect())
}

/// The pilot session. Every gate has already passed; this only executes.
fn run_pilot(dir: &str, env: &Env) -> Result<(String, usize, usize), String> {
    let irs: Vec<ProblemIR> = CORPUS
        .iter()
        .map(|i| load(i.path))
        .collect::<Result<_, _>>()?;

    // §3.3 warmup: one complete trajectory per (instance, operator), discarded.
    for (ii, _) in CORPUS.iter().enumerate() {
        for op in Op::ALL {
            warmup(&irs[ii], op, CONTROL_SEED);
        }
    }

    let mut out = header();
    let mut discarded = 0usize;

    for rep in 0..REPETITIONS {
        let mut attempt = 0;
        loop {
            match run_repetition(&irs, rep, env) {
                Ok(rows) => {
                    for r in rows {
                        out.push_str(&r);
                    }
                    break;
                }
                Err((reason, sf, sl, las, lae)) => {
                    discarded += 1;
                    out.push_str(&fmt_discard(rep, &reason, sf, sl, las, lae, env));
                    println!("  repetition {rep} discarded: {reason} ({discarded} so far)");
                    // §5 HOST-UNSTABLE overrides the retry.
                    if discarded > MAX_DISCARDED_REPS {
                        return Err(format!(
                            "HOST-UNSTABLE: {discarded} of {REPETITIONS} repetitions discarded, \
                             bound is {MAX_DISCARDED_REPS}; aborting before any verdict"
                        ));
                    }
                    attempt += 1;
                    // §5 re-executed at most once.
                    if attempt > 1 {
                        return Err(format!(
                            "session discarded: repetition {rep} exceeded the bound twice ({reason})"
                        ));
                    }
                }
            }
        }
    }

    // §10.1 written ONLY on successful completion.
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = format!("{dir}/rc020_pilot.tsv");
    std::fs::write(&path, &out).map_err(|e| e.to_string())?;

    // Amendment 1 §A4.10 — session-level provenance, kept OUT of the frozen
    // 22-column schema, which is unchanged.
    let rows = out.lines().count() - 1;
    let prov = format!(
        "RC-020 pilot provenance\n\
         pre-registration: {PREREG_PATH}\n\
         amendment: research/PREREG_RC020_AMENDMENT_1.md\n\
         CONTROL_SEED: {CONTROL_SEED}\n\
         SYNTHETIC_CONTROL_SEED: {SYNTHETIC_CONTROL_SEED}\n\
         CONTROL_BOOTSTRAP_SEED: {CONTROL_BOOTSTRAP_SEED}\n\
         CI_BASE_SEED: {CI_BASE_SEED}\n\
         PERM_BASE_SEED: {PERM_BASE_SEED}\n\
         pilot seeds: {PILOT:?}\n\
         repetitions: {REPETITIONS}\n\
         discarded repetitions: {discarded}\n\
         cpu_set: {}\n\
         threads: {}\n\
         timer_resolution_ms: {:.6}\n\
         artifact: {path}\n\
         artifact_sha256: {}\n\
         rows: {rows}\n",
        env.cpu_set,
        env.threads,
        env.resolution_ms,
        sha256_hex(out.as_bytes())
    );
    std::fs::write(format!("{dir}/rc020_pilot_provenance.txt"), prov).map_err(|e| e.to_string())?;

    Ok((path, rows, discarded))
}

fn main() {
    let dir: String = arg("--dir", "experiments/rc020".to_string());

    if flag("--controls") {
        let c = run_controls();
        std::process::exit(if c.all_pass { 0 } else { 1 });
    }

    if flag("--held-in") || flag("--held-out") {
        eprintln!(
            "REFUSED — §9's descendant is frozen blank, so {HELD_IN:?} and {HELD_OUT:?}\n\
             are unreachable. RC-020 is calibration-only (§1)."
        );
        std::process::exit(4);
    }

    if flag("--pilot") {
        let env = match pilot_gates(&dir) {
            Ok(env) => env,
            Err(e) => {
                eprintln!("PILOT REFUSED — {e}");
                std::process::exit(2);
            }
        };
        println!("\nRC-020 pilot — seeds {PILOT:?}, {REPETITIONS} repetitions\n");
        match run_pilot(&dir, &env) {
            Ok((path, rows, discarded)) => {
                println!("\n  frozen: {path} ({rows} rows, {discarded} discarded repetitions)");
                println!("  provenance: {dir}/rc020_pilot_provenance.txt");
                std::process::exit(0);
            }
            Err(e) => {
                // §10.1: nothing is written unless the session completes.
                eprintln!("\n  PILOT ABORTED — {e}");
                eprintln!("  No artifact written. Record this discard in the results document.");
                std::process::exit(5);
            }
        }
    }

    println!(
        "RC-020 marginal wall cost — calibration instrument\n\
         \n  --controls   run every control and exit\
         \n  --pilot      seeds {PILOT:?} (fully gated; writes the §10.1 artifact)\
         \n  --dir <path> artifact directory (default experiments/rc020)\n\
         \nWall time only. No energy or quality is computed or stored.\n\
         Equal-cost and efficacy claims are out of scope (§1)."
    );
}

// ==================================================================== TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answers() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// §2.1. Hard refusal by value, and the RC-020 blocks are themselves clean.
    #[test]
    fn every_prior_block_is_refused() {
        for lo in FORBIDDEN_LO {
            for s in lo..=lo + 7 {
                assert!(seed_forbidden(s), "seed {s} must be refused");
            }
        }
        for s in PILOT.iter().chain(&HELD_IN).chain(&HELD_OUT) {
            assert!(!seed_forbidden(*s), "RC-020 seed {s} must not be forbidden");
        }
    }

    /// The burned blocks specifically: RC-017's 5001-5008 and RC-018's
    /// 7001-7008 can never be reached again.
    #[test]
    fn burned_blocks_are_unreachable() {
        for s in (5001..=5008).chain(7001..=7008) {
            assert!(seed_forbidden(s), "burned seed {s} must be refused");
        }
    }

    #[test]
    fn rc020_blocks_are_pairwise_disjoint() {
        let all: Vec<u64> = PILOT
            .iter()
            .chain(&HELD_IN)
            .chain(&HELD_OUT)
            .copied()
            .collect();
        let mut u = all.clone();
        u.sort_unstable();
        u.dedup();
        assert_eq!(u.len(), all.len());
    }

    /// The control seed must lie outside every named block (see its doc comment).
    #[test]
    fn control_seed_is_outside_every_block() {
        assert!(!seed_forbidden(CONTROL_SEED));
        for s in PILOT.iter().chain(&HELD_IN).chain(&HELD_OUT) {
            assert_ne!(*s, CONTROL_SEED);
        }
    }

    /// §10.1. Exactly 22 columns, in order, with no outcome column.
    #[test]
    fn schema_is_exactly_the_preregistered_22_columns() {
        assert_eq!(SCHEMA.len(), 22);
        assert_eq!(SCHEMA[0], "instance");
        assert_eq!(SCHEMA[21], "ci_seed");
        for c in SCHEMA {
            for bad in ["energy", "quality", "cut", "objective", "best", "delta"] {
                assert!(!c.contains(bad), "outcome column {c}");
            }
        }
        // Every §5 guard has a column, so a guard that did not run is visible.
        for needed in [
            "sentinel_first_ms",
            "sentinel_last_ms",
            "sentinel_spread",
            "timer_resolution_ms",
            "load_avg_start",
            "load_avg_end",
            "discarded",
            "discard_reason",
            "degenerate",
        ] {
            assert!(SCHEMA.contains(&needed), "guard column {needed} absent");
        }
        assert_eq!(header().matches('\t').count(), 21);
    }

    /// §5. Two-sided, and NaN can only fail.
    #[test]
    fn degeneracy_and_drift_guards_are_two_sided() {
        let res = 0.001;
        assert!(degenerate_timing(0.0, res));
        assert!(degenerate_timing(-1.0, res));
        assert!(degenerate_timing(f64::NAN, res));
        assert!(degenerate_timing(0.05, res));
        assert!(!degenerate_timing(0.5, res));

        assert!(!drift_exceeded(100.0, 105.0)); // 5% within bound
        assert!(drift_exceeded(100.0, 110.0)); // 10% exceeds
        assert!(drift_exceeded(100.0, 0.0)); // impossible value cannot pass
        assert!(drift_exceeded(100.0, f64::NAN));
    }

    /// §6.1. Directly measured, no intercept anywhere.
    #[test]
    fn b_hat_is_a_direct_quotient() {
        assert_eq!(b_hat(8.0, 4), 2.0);
        assert_eq!(b_hat(32.0, 16), 2.0);
        // A pure per-sweep cost gives the SAME b_hat at every window length,
        // which is what §7.1 A2 tests for.
        for w in WINDOWS_A {
            assert!((b_hat(0.75 * w as f64, w) - 0.75).abs() < 1e-12);
        }
    }

    #[test]
    fn k_real_is_a_ratio_of_marginals() {
        assert_eq!(k_real(2.0, 1.0), Some(32.0));
        assert_eq!(k_real(1.0, 2.0), Some(8.0));
        assert_eq!(k_real(1.0, 0.0), None);
        assert_eq!(k_real(f64::NAN, 1.0), None);
    }

    /// §7.3. Stated on the continuous width; no integer stability requirement.
    #[test]
    fn feasibility_is_continuous_not_integer() {
        assert_eq!(feasibility(0.4), Feasibility::Feasible);
        assert_eq!(feasibility(1.0), Feasibility::ReducedResolution);
        assert_eq!(feasibility(2.99), Feasibility::ReducedResolution);
        assert_eq!(feasibility(3.0), Feasibility::NotFeasible);
        assert_eq!(feasibility(f64::NAN), Feasibility::NotFeasible);
        // A value sitting exactly on a half-integer is NOT penalised: this is
        // the RC-018 brittleness the design deliberately avoids (§0).
        assert_eq!(feasibility(0.5), Feasibility::Feasible);
    }

    /// §5/§7.1. UNSTABLE is neither qualifying nor a null.
    #[test]
    fn routing_separates_unstable_from_not_identified() {
        let tight = (0.98, 1.02);
        assert_eq!(
            route_cell(1.0, tight, &[1.0, 1.02, 0.98]),
            CellRoute::Qualifies
        );
        assert_eq!(
            route_cell(1.0, tight, &[1.0, 1.5, 0.6]),
            CellRoute::Unstable
        );
        // Imprecise: A1 fails regardless of stationarity.
        assert_eq!(
            route_cell(1.0, (0.5, 1.5), &[1.0, 1.0, 1.0]),
            CellRoute::NotIdentified
        );
        // Non-positive: A3 fails.
        assert_eq!(
            route_cell(-1.0, (-2.0, -0.5), &[-1.0, -1.0, -1.0]),
            CellRoute::NotIdentified
        );
    }

    #[test]
    fn provenance_refuses_each_missing_condition() {
        let good = Facts {
            exists: true,
            tracked: true,
            clean: true,
            committed: true,
            descendant_blank: true,
        };
        assert!(prereg_gate(&good).is_ok());
        for (f, want) in [
            (
                Facts {
                    exists: false,
                    ..good
                },
                "missing",
            ),
            (
                Facts {
                    tracked: false,
                    ..good
                },
                "tracked",
            ),
            (
                Facts {
                    clean: false,
                    ..good
                },
                "uncommitted",
            ),
            (
                Facts {
                    committed: false,
                    ..good
                },
                "never been committed",
            ),
        ] {
            let e = prereg_gate(&f).expect_err("must refuse");
            assert!(e.contains(want), "message {e:?} should name {want:?}");
        }
        assert!(reserved_blocks_locked(&good));
        assert!(!reserved_blocks_locked(&Facts {
            descendant_blank: false,
            ..good
        }));
    }

    /// §9 must actually still be blank in the committed file, and the markers
    /// the gate greps for must exist — otherwise the lock is vacuous.
    #[test]
    fn descendant_section_is_blank_in_the_preregistration() {
        let t = std::fs::read_to_string(PREREG_PATH).expect("pre-registration readable");
        assert!(t.contains(DESCENDANT_HEADING));
        assert!(t.contains(DESCENDANT_BLANK_MARKER));
    }

    #[test]
    fn ci_seeds_are_distinct_across_instances_and_arms() {
        let mut seen = std::collections::BTreeSet::new();
        for i in CORPUS.iter() {
            for a in WinArm::ALL {
                assert!(seen.insert(ci_seed(i.index, a)), "CI seed collision");
            }
        }
    }

    #[test]
    fn permutation_is_deterministic_and_total() {
        let a = permutation(36, PERM_BASE_SEED);
        assert_eq!(a, permutation(36, PERM_BASE_SEED));
        let mut s = a.clone();
        s.sort_unstable();
        assert_eq!(s, (0..36).collect::<Vec<_>>());
        assert_ne!(permutation(36, PERM_BASE_SEED + 1), a);
    }

    /// Amendment 1 §A4.9 — the frozen-constant test covers the seed set, so a
    /// change to any control seed fails the suite instead of silently altering
    /// what the controls did.
    #[test]
    fn control_seeds_are_frozen_and_disjoint() {
        assert_eq!(CONTROL_SEED, 20001);
        assert_eq!(SYNTHETIC_CONTROL_SEED, 991);
        assert_eq!(CONTROL_BOOTSTRAP_SEED, 993);
        control_seeds_disjoint().expect("Amendment 1 §A2 disjointness must hold");

        // Amendment 1 §A2: 72 forbidden + 8 pilot + 8 held-in + 8 held-out
        // + 12 CI + 9 permutation = 117 enumerated here; the two control
        // literals of §A1 bring the document's total to 119.
        let named = named_seed_values();
        assert_eq!(named.len(), 117);
        assert_eq!(named.len() + 2, 119);

        // The margins the amendment computed.
        let below = named
            .iter()
            .filter(|&&x| x < CONTROL_SEED)
            .max()
            .copied()
            .unwrap();
        let above = named
            .iter()
            .filter(|&&x| x > CONTROL_SEED)
            .min()
            .copied()
            .unwrap();
        assert_eq!(below, 12008);
        assert_eq!(above, 20261201);
        assert_eq!(CONTROL_SEED - below, 7_993);
        assert_eq!(above - CONTROL_SEED, 20_241_200);
    }

    /// The gate must actually be able to fail, or it is decoration.
    #[test]
    fn disjointness_gate_catches_a_collision() {
        let named = named_seed_values();
        for s in [
            PILOT[0],
            HELD_IN[0],
            HELD_OUT[0],
            5001,
            7001,
            CI_BASE_SEED,
            PERM_BASE_SEED,
        ] {
            assert!(
                named.contains(&s),
                "{s} must be a named value the gate would catch"
            );
        }
        assert!(!named.contains(&CONTROL_SEED));
    }

    /// §3: 72 conditions per repetition, and the artifact row arithmetic.
    #[test]
    fn pilot_grid_matches_the_preregistration() {
        let c = conditions();
        assert_eq!(
            c.len(),
            CORPUS.len() * Op::ALL.len() * TEMPS.len() * WinArm::ALL.len()
        );
        assert_eq!(c.len(), 72);
        // Rows per repetition = conditions x pilot seeds x windows.
        assert_eq!(c.len() * PILOT.len() * WINDOWS_A.len(), 1_728);
        assert_eq!(
            c.len() * PILOT.len() * WINDOWS_A.len() * REPETITIONS,
            15_552
        );
        // Trajectories actually executed per session, excluding sentinels/warmup.
        assert_eq!(c.len() * PILOT.len() * REPETITIONS, 5_184);
    }

    /// Every emitted row must have exactly the 22 frozen fields, discards too.
    #[test]
    fn emitted_rows_have_exactly_22_fields() {
        let env = Env {
            resolution_ms: 1e-5,
            cpu_set: "0-3".into(),
            threads: 1,
        };
        let o = Obs {
            inst: 0,
            op: Op::Metropolis,
            temp: 0.5,
            arm: WinArm::A,
            w: 8,
            seed: PILOT[0],
            window_ms: 2.0,
        };
        let row = fmt_row(&o, 0, 1.0, 1.01, 0.5, 0.6, &env);
        assert_eq!(row.trim_end().split('\t').count(), SCHEMA.len());
        let d = fmt_discard(3, "SENTINEL-DRIFT", 1.0, 2.0, 0.5, 0.6, &env);
        assert_eq!(d.trim_end().split('\t').count(), SCHEMA.len());
        // The discard row must carry its reason and the flag, in their columns.
        let f: Vec<&str> = d.trim_end().split('\t').collect();
        assert_eq!(f[18], "true");
        assert_eq!(f[19], "SENTINEL-DRIFT");
        // A recorded observation is never marked discarded.
        let g: Vec<&str> = row.trim_end().split('\t').collect();
        assert_eq!(g[18], "false");
        assert_eq!(g[19], "-");
    }

    #[test]
    fn frozen_constants_match_the_preregistration() {
        assert_eq!(REPLICAS, 32);
        assert_eq!(TEMPS, [0.1, 0.5, 2.0]);
        assert_eq!(PREFIX_SWEEPS, 4);
        assert_eq!(WINDOWS_A, [4, 8, 16]);
        assert_eq!(WINDOWS_B, [16, 8, 4]);
        assert_eq!(REPETITIONS, 9);
        assert_eq!(CORPUS.len(), 6);
        assert_eq!(PILOT[0], 10001);
        assert_eq!(HELD_IN[0], 11001);
        assert_eq!(HELD_OUT[0], 12001);
        assert_eq!(CI_BASE_SEED, 20261201);
        assert_eq!(PERM_BASE_SEED, 20261301);
        assert_eq!(DRIFT_BOUND, 0.09);
        assert_eq!(LOAD_BOUND, 2.0);
        assert_eq!(MAX_DISCARDED_REPS, 2);
        assert_eq!(CPU_SET, "0-3");
        assert_eq!((SENTINEL_TEMP, SENTINEL_WINDOW), (0.5, 8));
        assert_eq!(CORPUS[SENTINEL_INSTANCE].name, "G11");
    }

    /// §3.1-§3.2: no arithmetic combines timings from different executions.
    #[test]
    fn no_cross_execution_timing_arithmetic() {
        let src = include_str!("exp_marginal_cost.rs");
        let minus = "-";
        for (l, r) in [
            ("total12", "total4"),
            ("t_end", "t_start_other"),
            ("ms_b", "ms_a"),
        ] {
            assert!(
                !src.contains(&format!("{l} {minus} {r}")),
                "cross-execution subtraction {l}/{r}"
            );
        }
        assert!(
            src.contains("window_ms[slot] = t0.elapsed()"),
            "windows must be timed directly"
        );
    }
}
