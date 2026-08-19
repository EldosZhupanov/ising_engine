//! RC-018 — timing identification instrument.
//!
//! Implements `research/PREREG_RC018_COST_IDENTIFICATION.md` as amended by
//! Amendments 1 and 2, and nothing else. Every constant here is frozen by that
//! document; none may be changed without a committed amendment.
//!
//! WALL TIME ONLY. This binary computes, stores and prints no energy, no cut
//! value and no quality of any kind (prereg §1). `cost_model` is the object
//! under test and is never used as a measure of cost (RC-005).
//!
//! Two disjoint designs (Amendment 1 §A7):
//!   * Design L — geometric ladder 4.0 -> 0.1, budgets {4,8,16,24}, one operator
//!     execution per budget. Sole data source for Gate A.
//!   * Design T — isothermal cells, ONE paired trajectory per cell: sweeps 1-4
//!     as prefix, then `incremental_ms` timed DIRECTLY around sweeps 5-12 on the
//!     same state and the same RNG stream (Amendment 2 §B1). Sole data source
//!     for Gate B. No cross-execution subtraction exists anywhere in Design T.
//!
//! ```text
//! cargo run --release --bin exp_cost_identification -- --controls
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

// ============================================================ FROZEN CONSTANTS

/// Prereg §2 / Amendment 1 §A1. Design L budgets.
const BUDGETS_L: [u32; 4] = [4, 8, 16, 24];
/// Amendment 1 §A7. Design T isothermal cells.
const TEMPS_T: [f64; 8] = [0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0];
/// Amendment 2 §B1. The paired trajectory: prefix then directly timed window.
const PREFIX_SWEEPS: u32 = 4;
const WINDOW_SWEEPS: u32 = 8;
const REPLICAS: usize = 32;
const REPETITIONS: usize = 9;
const LADDER_HI: f64 = 4.0;
const LADDER_LO: f64 = 0.1;
/// Prereg §7 §A4. `k_ref` in the equal-total-time equation.
const K_REF: f64 = 16.0;
/// Amendment 1 §A2. The clip is part of the frozen model, not a fitting choice.
const PHI_CLIP: f64 = 1.0 / 3.0;

/// Prereg §2. Seed blocks, disjoint from every prior cycle.
const PILOT: [u64; 8] = [7001, 7002, 7003, 7004, 7005, 7006, 7007, 7008];
const HELD_IN: [u64; 8] = [8001, 8002, 8003, 8004, 8005, 8006, 8007, 8008];
const HELD_OUT: [u64; 8] = [9001, 9002, 9003, 9004, 9005, 9006, 9007, 9008];

/// Prereg §2 "Seed independence — binding". RC-016 and RC-017 blocks. `5001-5008`
/// are burned by the RC-017 abort; `6001-6008` are reserved to RC-017's successor.
/// No RC-017 datum may be reused (prereg §2), so the instrument refuses the seeds
/// outright rather than trusting a caller not to pass them.
const FORBIDDEN_SEEDS: [u64; 48] = [
    1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, // RC-016 held-in
    2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, // RC-016 held-out
    3001, 3002, 3003, 3004, 3005, 3006, 3007, 3008, // RC-016 pilot
    4001, 4002, 4003, 4004, 4005, 4006, 4007, 4008, // RC-017 pilot
    5001, 5002, 5003, 5004, 5005, 5006, 5007, 5008, // RC-017 held-in — BURNED
    6001, 6002, 6003, 6004, 6005, 6006, 6007, 6008, // RC-017 held-out — reserved
];

/// Prereg §2. Deterministic seeds for the descriptive bootstrap and for the
/// per-repetition order permutation.
const CI_BASE_SEED: u64 = 20261001;
const PERM_BASE_SEED: u64 = 20261101;
const BOOTSTRAP_REPS: usize = 100_000;

/// Prereg §5. This host's recorded same-code drift (`PERF.md`).
const DRIFT_BAR: f64 = 0.09;
/// Amendment 2 §B3 and prereg §5.
const DEGENERATE_RESOLUTION_MULT: f64 = 100.0;
/// Amendment 1 §A1.3. φ recovery tolerance.
const PHI_RECOVERY_TOL: f64 = 0.01;
/// Amendment 1 §A6. Synthetic recovery tolerance.
const SYNTHETIC_TOL: f64 = 0.05;

const PREREG_PATH: &str = "research/PREREG_RC018_COST_IDENTIFICATION.md";
const DESCENDANT_PATH: &str = "research/PREREG_RC018_DESCENDANT.md";
const CONFIRM_FLAG: &str = "--confirm-descendant-frozen";

/// Prereg §2. The frozen 6-instance subset. `index` is the zero-based corpus
/// index in the enumerated table of `PREREG_RC016.md` §4, which is the order the
/// `sha12` values are recorded in and which the CI seed formula binds to.
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

/// The two operators, each fitted separately (Amendment 1 §A2: pooling operators
/// is forbidden).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arm {
    Metropolis,
    Gibbs,
}

impl Arm {
    fn tag(self) -> &'static str {
        match self {
            Arm::Metropolis => "metropolis_sweep",
            Arm::Gibbs => "gibbs_color_sweep",
        }
    }
    const ALL: [Arm; 2] = [Arm::Metropolis, Arm::Gibbs];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Block {
    Pilot,
    HeldIn,
    HeldOut,
}

impl Block {
    fn tag(self) -> &'static str {
        match self {
            Block::Pilot => "pilot",
            Block::HeldIn => "heldin",
            Block::HeldOut => "heldout",
        }
    }
    /// Prereg §2. Exactly one block per mode; nothing else is reachable.
    fn seeds(self) -> [u64; 8] {
        match self {
            Block::Pilot => PILOT,
            Block::HeldIn => HELD_IN,
            Block::HeldOut => HELD_OUT,
        }
    }
    /// Amendment 1 §A2 / prereg §2: `0` held-in, `1` held-out; pilot is not a
    /// Gate block and takes the next code so its streams cannot collide.
    fn ci_offset(self) -> u64 {
        match self {
            Block::HeldIn => 0,
            Block::HeldOut => 1,
            Block::Pilot => 2,
        }
    }
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
//
// Minimal local implementation — the prereg requires content hashing and the
// project adds no dependency for it. Pinned by a known-answer test.

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

// ============================================================ SMALL NUMERICS

/// Prereg §6 / Amendment 1 §A2. Ordinary least squares of `y` on `x`.
/// Returns `(intercept, slope)`, or `None` when `x` has no spread — the defect
/// Amendment 1 §A5 found in the superseded N2.
fn ols(x: &[f64], y: &[f64]) -> Option<(f64, f64)> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let sxx: f64 = x.iter().map(|v| (v - mx) * (v - mx)).sum();
    if sxx <= 0.0 {
        return None;
    }
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let slope = sxy / sxx;
    Some((my - slope * mx, slope))
}

/// Amendment 1 §A4. Equal TOTAL time, not equal slope:
/// `a_M + b_M * K_REF = a_G + b_G * k'`.
fn k_prime(a_m: f64, b_m: f64, a_g: f64, b_g: f64) -> Option<i64> {
    if b_g <= 0.0 {
        return None;
    }
    let k = (a_m - a_g + K_REF * b_m) / b_g;
    if !k.is_finite() {
        return None;
    }
    let r = k.round() as i64;
    if r < 1 {
        None
    } else {
        Some(r)
    }
}

/// Prereg §6.4. Deterministic percentile 95% bootstrap over per-seed values.
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

/// Amendment 1 §A2. The shipped shape-only work, `S = (n + 2m) * r`.
fn shape_work(ir: &ProblemIR, replicas: usize) -> f64 {
    let m = ir.col_idx.len() / 2;
    ((ir.n + 2 * m) * replicas) as f64
}

/// Prereg §2. `CI_BASE_SEED + 2*corpus_index + block`, recorded per row so the
/// frozen estimator needs no out-of-band table to reproduce the bootstrap.
fn ci_seed(corpus_index: usize, block: Block) -> u64 {
    CI_BASE_SEED + 2 * corpus_index as u64 + block.ci_offset()
}

/// Prereg §3.2. Deterministic Fisher-Yates over condition indices.
fn permutation(len: usize, seed: u64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..len).collect();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for i in (1..len).rev() {
        idx.swap(i, rng.gen_range(0..=i));
    }
    idx
}

// ============================================================== EXECUTION

fn ladder(hi: f64, lo: f64, r: usize) -> Vec<f64> {
    (0..r)
        .map(|k| {
            if r <= 1 {
                hi
            } else {
                hi * (lo / hi).powf(k as f64 / (r - 1) as f64)
            }
        })
        .collect()
}

fn isothermal(t: f64, r: usize) -> Vec<f64> {
    vec![t; r]
}

fn make_op(arm: Arm) -> Box<dyn Operator> {
    match arm {
        Arm::Metropolis => Box::new(MetropolisSweep::new()),
        Arm::Gibbs => Box::new(GibbsColorSweep::new()),
    }
}

fn view<'a>(temps: &'a [f64]) -> RuntimeView<'a> {
    RuntimeView {
        iteration: 0,
        temperatures: temps,
        num_replicas: REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    }
}

fn fresh_state<'a>(ir: &'a ProblemIR) -> Option<SparseBitSlice<'a>> {
    let init = vec![0u8; ir.n];
    SparseBitSlice::new(ir, REPLICAS, &init).ok()
}

/// Design L (Gate A). One operator execution at one budget.
fn design_l(ir: &ProblemIR, arm: Arm, seed: u64, k: u32) -> Option<f64> {
    let temps = ladder(LADDER_HI, LADDER_LO, REPLICAS);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut op = make_op(arm);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let t0 = Instant::now();
    let _ = op.apply(&mut st, &v, &mut rng, Budget { sweeps: k });
    Some(t0.elapsed().as_secs_f64() * 1000.0)
}

/// One Design T observation (Gate B), Amendment 2 §B1.
struct Paired {
    /// Timed DIRECTLY around sweeps 5..12. Never a difference of two executions.
    incremental_ms: f64,
    /// Diagnostic only (§B1.4) — never a response, never subtracted.
    prefix_ms: f64,
    /// accepted/proposed over the measurement window ONLY (§B1.3).
    phi: f64,
    accepted: u64,
    proposed: u64,
}

/// Amendment 2 §B1. ONE Runtime execution: one state, one RNG stream. The prefix
/// and the window are consecutive `apply` calls on the SAME `st` and the SAME
/// `rng`, so the window continues the identical trajectory. There is deliberately
/// no arithmetic here combining timings from different executions.
fn design_t(ir: &ProblemIR, arm: Arm, seed: u64, t_cell: f64) -> Option<Paired> {
    let temps = isothermal(t_cell, REPLICAS);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut op = make_op(arm);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let p0 = Instant::now();
    let _ = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: PREFIX_SWEEPS,
        },
    );
    let prefix_ms = p0.elapsed().as_secs_f64() * 1000.0;

    let w0 = Instant::now();
    let rep = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: WINDOW_SWEEPS,
        },
    );
    let incremental_ms = w0.elapsed().as_secs_f64() * 1000.0;

    Some(Paired {
        incremental_ms,
        prefix_ms,
        phi: rep.acceptance_rate(),
        accepted: rep.accepted,
        proposed: rep.proposed,
    })
}

/// The three probes §B1.4 requires: state digest, energy bits, RNG position.
struct Probes {
    state: StateDigest,
    energy: Vec<u64>,
    rng: u64,
}

/// The same trajectory executed as ONE uninterrupted call, for N3' (§B1.4).
fn uninterrupted(ir: &ProblemIR, arm: Arm, seed: u64, t_cell: f64) -> Option<Probes> {
    let temps = isothermal(t_cell, REPLICAS);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut op = make_op(arm);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let _ = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: PREFIX_SWEEPS + WINDOW_SWEEPS,
        },
    );
    Some(Probes {
        state: st.digest(),
        energy: energy_bits(&st),
        rng: rng.r#gen::<u64>(),
    })
}

/// Paired variant returning the same three probes, for N3'.
fn paired_probes(ir: &ProblemIR, arm: Arm, seed: u64, t_cell: f64) -> Option<Probes> {
    let temps = isothermal(t_cell, REPLICAS);
    let mut st = fresh_state(ir)?;
    let v = view(&temps);
    let mut op = make_op(arm);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let _ = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: PREFIX_SWEEPS,
        },
    );
    let _ = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: WINDOW_SWEEPS,
        },
    );
    Some(Probes {
        state: st.digest(),
        energy: energy_bits(&st),
        rng: rng.r#gen::<u64>(),
    })
}

fn energy_bits(st: &SparseBitSlice<'_>) -> Vec<u64> {
    let mut e = vec![0.0f64; REPLICAS];
    st.energies_into(&mut e);
    e.iter().map(|v| v.to_bits()).collect()
}

// ============================================================ HOST GUARDS

/// Prereg §5. Smallest observable non-zero clock delta, in ms.
fn timer_resolution_ms() -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..64 {
        let t0 = Instant::now();
        loop {
            let d = t0.elapsed();
            if !d.is_zero() {
                let ms = d.as_secs_f64() * 1000.0;
                if ms < best {
                    best = ms;
                }
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

/// Amendment 2 §B3. Both guards, or the observation is degenerate.
fn degenerate_timing(incremental_ms: f64, resolution_ms: f64) -> bool {
    // NaN must land in the degenerate branch, so finiteness is tested explicitly
    // rather than relying on a comparison that is false for NaN either way.
    !incremental_ms.is_finite()
        || incremental_ms <= 0.0
        || incremental_ms < DEGENERATE_RESOLUTION_MULT * resolution_ms
}

fn rel_spread(a: f64, b: f64) -> f64 {
    let lo = a.min(b);
    let hi = a.max(b);
    if lo <= 0.0 {
        f64::INFINITY
    } else {
        hi / lo - 1.0
    }
}

// ========================================================= PROVENANCE GATES

fn git_ok(args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn git_out(args: &[&str]) -> Option<String> {
    let o = std::process::Command::new("git").args(args).output().ok()?;
    if !o.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn git_commit_secs(path: &str) -> Option<u64> {
    git_out(&["log", "-1", "--format=%ct", "--", path])?
        .parse()
        .ok()
}

fn mtime_secs(path: &str) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// The facts a provenance decision needs, gathered separately so the decision
/// itself is pure and unit-testable without a git tree.
#[derive(Clone, Copy, Debug)]
struct DocFacts {
    exists: bool,
    tracked: bool,
    clean: bool,
    commit_secs: Option<u64>,
}

/// Prereg chronology: the pre-registration must be committed, clean and older
/// than any artifact it governs. Pure.
fn prereg_gate(f: &DocFacts) -> Result<(), &'static str> {
    if !f.exists {
        return Err("pre-registration file is missing");
    }
    if !f.tracked {
        return Err("pre-registration is not tracked by git");
    }
    if !f.clean {
        return Err("pre-registration has uncommitted modifications");
    }
    if f.commit_secs.is_none() {
        return Err("pre-registration has never been committed");
    }
    Ok(())
}

/// Prereg §7 / §2: held-out additionally needs a committed, clean descendant
/// strictly NEWER than the held-in artifact, plus an explicit confirmation.
fn descendant_gate(
    f: &DocFacts,
    heldin_artifact_secs: Option<u64>,
    confirmed: bool,
) -> Result<(), &'static str> {
    if !confirmed {
        return Err("explicit confirmation flag absent");
    }
    let Some(heldin) = heldin_artifact_secs else {
        return Err("held-in artifact missing: held-out is not reachable first");
    };
    prereg_gate(f).map_err(|_| "descendant is missing, untracked, dirty or uncommitted")?;
    let Some(c) = f.commit_secs else {
        return Err("descendant has never been committed");
    };
    if c <= heldin {
        return Err("descendant was not committed strictly after the held-in artifact");
    }
    Ok(())
}

fn doc_facts(path: &str) -> DocFacts {
    DocFacts {
        exists: std::path::Path::new(path).exists(),
        tracked: git_ok(&["ls-files", "--error-unmatch", path]),
        clean: git_ok(&["diff", "--quiet", "HEAD", "--", path]),
        commit_secs: git_commit_secs(path),
    }
}

/// Prereg §2. A mode may touch exactly its own block, and no forbidden seed may
/// ever reach the Runtime.
fn seeds_allowed(block: Block) -> Result<[u64; 8], String> {
    let s = block.seeds();
    for x in s {
        if FORBIDDEN_SEEDS.contains(&x) {
            return Err(format!(
                "seed {x} belongs to a forbidden prior block (RC-016/RC-017); \
                 5001-5008 are burned and 6001-6008 are reserved"
            ));
        }
    }
    Ok(s)
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

fn load(path: &str) -> Result<ProblemIR, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    rudy_maxcut_ir(&text)
}

// ================================================================ CONTROLS

struct Controls {
    resolution_ms: f64,
    all_pass: bool,
}

/// N2' (Amendment 1 §A5) and P1'/P2 (§A6) operate on constructed data with a
/// KNOWN `(a, b)`, so they validate the estimator independently of the Runtime
/// and cannot flake on host noise.
fn synthetic_series(a: f64, b: f64, jitter: f64, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for rep in 0..REPETITIONS {
        for &k in BUDGETS_L.iter() {
            let j = (rng.r#gen::<f64>() - 0.5) * 2.0 * jitter;
            xs.push(k as f64);
            ys.push((a + b * k as f64) * (1.0 + j));
        }
        let _ = rep;
    }
    (xs, ys)
}

fn run_controls() -> Controls {
    println!("RC-018 controls — prereg §4, Amendment 1 §A1.3/§A5/§A6, Amendment 2 §B1.4\n");
    let mut ok = true;

    // ---- P3: corpus hashes -------------------------------------------------
    match verify_corpus() {
        Ok(()) => println!(
            "[P3] corpus hashes: all {} match PREREG_RC016 §4",
            CORPUS.len()
        ),
        Err(e) => {
            println!("[P3] corpus hashes: FAIL — {e}");
            ok = false;
        }
    }

    // ---- timer resolution --------------------------------------------------
    let resolution_ms = timer_resolution_ms();
    println!(
        "[--] timer resolution: {resolution_ms:.6} ms (degenerate below {:.6} ms)",
        DEGENERATE_RESOLUTION_MULT * resolution_ms
    );

    // ---- P2 / P1': estimator recovers known (a, b) and k' ------------------
    let (xs, ys) = synthetic_series(12.0, 3.0, 0.01, 991);
    match ols(&xs, &ys) {
        Some((a, b)) => {
            let ea = (a - 12.0).abs() / 12.0;
            let eb = (b - 3.0).abs() / 3.0;
            let pass = ea <= SYNTHETIC_TOL && eb <= SYNTHETIC_TOL;
            println!(
                "[P2] synthetic (a,b) recovery: a={a:.4} (err {:.3}) b={b:.4} (err {:.3}) -> {}",
                ea,
                eb,
                if pass { "PASS" } else { "FAIL" }
            );
            ok &= pass;
            // P1': the implied k' must be recovered EXACTLY on known inputs.
            let want = k_prime(12.0, 3.0, 6.0, 2.0);
            let got = k_prime(a, b, 6.0, 2.0);
            let kp = want.is_some() && want == got;
            println!(
                "[P1'] synthetic k' exactness: want {want:?} got {got:?} -> {}",
                if kp { "PASS" } else { "FAIL" }
            );
            ok &= kp;
        }
        None => {
            println!("[P2] synthetic recovery: FAIL — estimator returned None");
            ok = false;
        }
    }

    // ---- N2': zero-slope null across DISTINCT k labels ---------------------
    let (xs0, ys0) = synthetic_series(20.0, 0.0, 0.02, 992);
    match ols(&xs0, &ys0) {
        Some((_, b0)) => {
            // Per-repetition slopes, then the frozen percentile bootstrap.
            let mut slopes = Vec::new();
            for rep in 0..REPETITIONS {
                let lo = rep * BUDGETS_L.len();
                let hi = lo + BUDGETS_L.len();
                if let Some((_, s)) = ols(&xs0[lo..hi], &ys0[lo..hi]) {
                    slopes.push(s);
                }
            }
            let (l, u) = bootstrap_ci(&slopes, BOOTSTRAP_REPS, 993);
            let pass = l <= 0.0 && u >= 0.0;
            println!(
                "[N2'] zero-slope null: b={b0:.6} CI [{l:.6}, {u:.6}] contains 0 -> {}",
                if pass { "PASS" } else { "FAIL" }
            );
            ok &= pass;
        }
        None => {
            println!("[N2'] zero-slope null: FAIL — estimator returned None");
            ok = false;
        }
    }

    // ---- Runtime-facing controls, on the smallest corpus member ------------
    let probe = &CORPUS[1]; // G11: 800 vars, 1600 edges — cheapest
    match load(probe.path) {
        Err(e) => {
            println!("[N1/N3'/P4] FAIL — cannot load {}: {e}", probe.path);
            ok = false;
        }
        Ok(ir) => {
            // N1: same code, two labelled conditions, one seed.
            let a = design_l(&ir, Arm::Metropolis, PILOT[0], 8);
            let b = design_l(&ir, Arm::Metropolis, PILOT[0], 8);
            match (a, b) {
                (Some(x), Some(y)) => {
                    let s = rel_spread(x, y);
                    let pass = s <= DRIFT_BAR;
                    println!("[N1] same-code null: {x:.3} vs {y:.3} ms, spread {:.3} <= {DRIFT_BAR} -> {}",
                        s, if pass { "PASS" } else { "HOST-UNSTABLE" });
                    ok &= pass;
                }
                _ => {
                    println!("[N1] same-code null: FAIL — execution rejected the instance");
                    ok = false;
                }
            }

            // N3': paired instrumented trajectory vs uninterrupted 12 sweeps.
            let mut n3 = true;
            for &arm in Arm::ALL.iter() {
                let p = paired_probes(&ir, arm, PILOT[0], 1.0);
                let u = uninterrupted(&ir, arm, PILOT[0], 1.0);
                match (p, u) {
                    (Some(pp), Some(uu)) => {
                        let (sd, eb, rp) = (
                            pp.state == uu.state,
                            pp.energy == uu.energy,
                            pp.rng == uu.rng,
                        );
                        let same = sd && eb && rp;
                        println!(
                            "[N3'] {:<18} state {} energy {} rng {} -> {}",
                            arm.tag(),
                            if sd { "match" } else { "DIFFER" },
                            if eb { "match" } else { "DIFFER" },
                            if rp { "match" } else { "DIFFER" },
                            if same { "PASS" } else { "FAIL" }
                        );
                        n3 &= same;
                    }
                    _ => {
                        println!("[N3'] {:<18} FAIL — execution rejected", arm.tag());
                        n3 = false;
                    }
                }
            }
            ok &= n3;

            // P4: φ recovered independently. Over ONE sweep each site is proposed
            // once per replica, so accepted == net spin changes exactly; over more
            // sweeps a site may flip twice and the identity would not hold.
            let mut p4 = true;
            for &arm in Arm::ALL.iter() {
                let temps = isothermal(1.0, REPLICAS);
                let Some(mut st) = fresh_state(&ir) else {
                    p4 = false;
                    continue;
                };
                let mut before = vec![0u8; ir.n];
                let mut after = vec![0u8; ir.n];
                let v = view(&temps);
                let mut op = make_op(arm);
                let mut rng = ChaCha8Rng::seed_from_u64(PILOT[0]);
                let mut changed = 0u64;
                let mut befores: Vec<Vec<u8>> = Vec::with_capacity(REPLICAS);
                for r in 0..REPLICAS {
                    st.extract_into(r, &mut before);
                    befores.push(before.clone());
                }
                let rep = op.apply(&mut st, &v, &mut rng, Budget { sweeps: 1 });
                for (r, b) in befores.iter().enumerate() {
                    st.extract_into(r, &mut after);
                    changed += b.iter().zip(after.iter()).filter(|(x, y)| x != y).count() as u64;
                }
                let phi_report = rep.acceptance_rate();
                let phi_state = changed as f64 / (REPLICAS * ir.n) as f64;
                let d = (phi_report - phi_state).abs();
                let pass = d <= PHI_RECOVERY_TOL;
                println!("[P4] {:<18} phi_report={phi_report:.6} phi_state={phi_state:.6} |d|={d:.6} -> {}",
                    arm.tag(), if pass { "PASS" } else { "FAIL" });
                p4 &= pass;
            }
            ok &= p4;
        }
    }

    println!(
        "\nCONTROLS: {}",
        if ok {
            "ALL PASS"
        } else {
            "FAILED — instrument invalid, no science"
        }
    );
    Controls {
        resolution_ms,
        all_pass: ok,
    }
}

// ============================================================= SCIENCE MODES

/// Prereg §3. Every condition appears once per repetition, in a deterministic
/// per-repetition permutation, with a sentinel first and last.
fn conditions_l() -> Vec<(usize, Arm, u32)> {
    let mut v = Vec::new();
    for (ii, _) in CORPUS.iter().enumerate() {
        for &arm in Arm::ALL.iter() {
            for &k in BUDGETS_L.iter() {
                v.push((ii, arm, k));
            }
        }
    }
    v
}

fn conditions_t() -> Vec<(usize, Arm, f64)> {
    let mut v = Vec::new();
    for (ii, _) in CORPUS.iter().enumerate() {
        for &arm in Arm::ALL.iter() {
            for &t in TEMPS_T.iter() {
                v.push((ii, arm, t));
            }
        }
    }
    v
}

const HEADER_L: &str = "design\tinstance\tcorpus_index\toperator\tblock\tseed\trepetition\tk\tms\tshape_work\tci_seed\n";
const HEADER_T: &str = "design\tinstance\tcorpus_index\toperator\tblock\tseed\trepetition\ttemp\tincremental_ms\tprefix_ms_diagnostic\taccepted\tproposed\tphi\tphi_clipped\tshape_work\tdegenerate\tci_seed\n";

/// The one science entry point. Runs only after every gate has passed.
fn run_block(block: Block, dir: &str, resolution_ms: f64) -> Result<(), String> {
    let seeds = seeds_allowed(block)?;
    let mut out_l = String::from(HEADER_L);
    let mut out_t = String::from(HEADER_T);

    let irs: Vec<ProblemIR> = CORPUS
        .iter()
        .map(|i| load(i.path))
        .collect::<Result<_, _>>()?;

    for rep in 0..REPETITIONS {
        // Design L.
        let cl = conditions_l();
        for &ci in permutation(cl.len(), PERM_BASE_SEED + rep as u64).iter() {
            let (ii, arm, k) = cl[ci];
            for &s in seeds.iter() {
                let Some(ms) = design_l(&irs[ii], arm, s, k) else {
                    return Err(format!("{} rejected by the backend", CORPUS[ii].name));
                };
                out_l.push_str(&format!(
                    "L\t{}\t{}\t{}\t{}\t{s}\t{rep}\t{k}\t{ms:.6}\t{:.0}\t{}\n",
                    CORPUS[ii].name,
                    CORPUS[ii].index,
                    arm.tag(),
                    block.tag(),
                    shape_work(&irs[ii], REPLICAS),
                    ci_seed(CORPUS[ii].index, block)
                ));
            }
        }
        // Design T.
        let ct = conditions_t();
        for &ci in permutation(ct.len(), PERM_BASE_SEED + 1000 + rep as u64).iter() {
            let (ii, arm, t) = ct[ci];
            for &s in seeds.iter() {
                let Some(p) = design_t(&irs[ii], arm, s, t) else {
                    return Err(format!("{} rejected by the backend", CORPUS[ii].name));
                };
                let degen = degenerate_timing(p.incremental_ms, resolution_ms);
                out_t.push_str(&format!(
                    "T\t{}\t{}\t{}\t{}\t{s}\t{rep}\t{t}\t{:.6}\t{:.6}\t{}\t{}\t{:.8}\t{:.8}\t{:.0}\t{}\t{}\n",
                    CORPUS[ii].name,
                    CORPUS[ii].index,
                    arm.tag(),
                    block.tag(),
                    p.incremental_ms,
                    p.prefix_ms,
                    p.accepted,
                    p.proposed,
                    p.phi,
                    p.phi.min(PHI_CLIP),
                    shape_work(&irs[ii], REPLICAS),
                    degen,
                    ci_seed(CORPUS[ii].index, block)
                ));
            }
        }
    }

    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let pl = format!("{dir}/rc018_{}_designL.tsv", block.tag());
    let pt = format!("{dir}/rc018_{}_designT.tsv", block.tag());
    std::fs::write(&pl, out_l).map_err(|e| e.to_string())?;
    std::fs::write(&pt, out_t).map_err(|e| e.to_string())?;
    println!("  frozen: {pl}\n  frozen: {pt}");
    Ok(())
}

/// Every science mode passes through here. Nothing else may reach `run_block`.
fn gated_science(block: Block, dir: &str) -> i32 {
    println!("\nRC-018 {} block — provenance gates", block.tag());

    if let Err(e) = prereg_gate(&doc_facts(PREREG_PATH)) {
        eprintln!("  REFUSED — {e} ({PREREG_PATH})");
        return 2;
    }
    println!("  [1] pre-registration committed and clean");

    if let Err(e) = verify_corpus() {
        eprintln!("  REFUSED — corpus verification failed: {e}");
        return 3;
    }
    println!("  [2] corpus hashes verified");

    if let Err(e) = seeds_allowed(block) {
        eprintln!("  REFUSED — {e}");
        return 4;
    }
    println!(
        "  [3] seed block {:?} is allowed and forbidden-free",
        block.seeds()
    );

    if block == Block::HeldOut {
        let heldin = mtime_secs(&format!("{dir}/rc018_heldin_designT.tsv"));
        if let Err(e) = descendant_gate(&doc_facts(DESCENDANT_PATH), heldin, flag(CONFIRM_FLAG)) {
            eprintln!("  REFUSED — {e}");
            return 5;
        }
        println!("  [4] descendant committed strictly after held-in, run confirmed");
    }

    let c = run_controls();
    if !c.all_pass {
        eprintln!("\n  REFUSED — controls must pass before any science datum (§4, K1).");
        return 6;
    }

    match run_block(block, dir, c.resolution_ms) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("  block failed: {e}");
            7
        }
    }
}

fn main() {
    let dir: String = arg("--dir", "experiments/rc018".to_string());

    if flag("--controls") {
        let c = run_controls();
        std::process::exit(if c.all_pass { 0 } else { 1 });
    }

    let code = if flag("--pilot") {
        gated_science(Block::Pilot, &dir)
    } else if flag("--held-in") {
        gated_science(Block::HeldIn, &dir)
    } else if flag("--held-out") {
        gated_science(Block::HeldOut, &dir)
    } else {
        println!(
            "RC-018 timing identification instrument\n\
             \n  --controls    run every control and exit\
             \n  --pilot       seeds {PILOT:?}\
             \n  --held-in     seeds {HELD_IN:?}\
             \n  --held-out    seeds {HELD_OUT:?} (needs {CONFIRM_FLAG})\
             \n  --dir <path>  artifact directory (default experiments/rc018)\n\
             \nWall time only. No energy or quality is computed or stored."
        );
        0
    };
    std::process::exit(code);
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

    /// Prereg §2. No mode can reach a prior cycle's seeds.
    #[test]
    fn no_forbidden_seed_is_reachable() {
        for b in [Block::Pilot, Block::HeldIn, Block::HeldOut] {
            let s = seeds_allowed(b).expect("frozen block must be allowed");
            for x in s {
                assert!(!FORBIDDEN_SEEDS.contains(&x), "seed {x} is forbidden");
            }
        }
    }

    #[test]
    fn burned_and_reserved_rc017_seeds_are_forbidden() {
        for x in 5001..=5008u64 {
            assert!(
                FORBIDDEN_SEEDS.contains(&x),
                "burned seed {x} must be forbidden"
            );
        }
        for x in 6001..=6008u64 {
            assert!(
                FORBIDDEN_SEEDS.contains(&x),
                "reserved seed {x} must be forbidden"
            );
        }
    }

    #[test]
    fn seed_blocks_are_pairwise_disjoint() {
        let all: Vec<u64> = PILOT
            .iter()
            .chain(&HELD_IN)
            .chain(&HELD_OUT)
            .copied()
            .collect();
        let mut u = all.clone();
        u.sort_unstable();
        u.dedup();
        assert_eq!(u.len(), all.len(), "RC-018 blocks must be disjoint");
    }

    /// Amendment 2 §B1: the window must CONTINUE the prefix on one state and one
    /// RNG stream. If it were two independent executions this would fail.
    #[test]
    fn paired_trajectory_equals_uninterrupted() {
        let Ok(ir) = load(CORPUS[1].path) else {
            return; // corpus absent in this environment; hash control covers presence
        };
        for arm in Arm::ALL {
            let p = paired_probes(&ir, arm, PILOT[0], 1.0).expect("paired");
            let u = uninterrupted(&ir, arm, PILOT[0], 1.0).expect("uninterrupted");
            assert_eq!(
                p.state,
                u.state,
                "{}: paired state must equal uninterrupted",
                arm.tag()
            );
            assert_eq!(
                p.energy,
                u.energy,
                "{}: paired energy bits must match",
                arm.tag()
            );
            assert_eq!(
                p.rng,
                u.rng,
                "{}: paired RNG must equal uninterrupted",
                arm.tag()
            );
        }
    }

    /// Amendment 2 §B1: no cross-execution timing arithmetic may exist.
    #[test]
    fn no_independent_timing_subtraction() {
        let src = include_str!("exp_cost_identification.rs");
        // The needles are assembled at run time: written as literals they would
        // appear in this file and the scan would always trip over itself.
        let minus = "-";
        for (lhs, rhs) in [("t12", "t4"), ("ms12", "ms4"), ("T_meas(12)", "T_meas(4)")] {
            let bad = format!("{lhs} {minus} {rhs}");
            assert!(
                !src.contains(&bad),
                "superseded subtraction form present: {bad}"
            );
        }
        // The authoritative guarantee is behavioural - see
        // `paired_trajectory_equals_uninterrupted`. This scan only catches a
        // reintroduced textual form.
        assert!(
            src.contains("incremental_ms = w0.elapsed()"),
            "the window must be timed directly, not reconstructed"
        );
    }

    /// Prereg §1: wall time only.
    #[test]
    fn tsv_carries_no_outcome_columns() {
        for h in [HEADER_L, HEADER_T] {
            for bad in ["energy", "quality", "cut", "best", "delta", "objective"] {
                assert!(!h.contains(bad), "outcome column {bad} in {h}");
            }
        }
        assert!(HEADER_T.contains("incremental_ms"));
        assert!(HEADER_T.contains("prefix_ms_diagnostic"));
    }

    /// Amendment 2 §B2. The corrected accounting, per block and across blocks.
    #[test]
    fn run_counts_match_amendment_2() {
        let l = CORPUS.len() * Arm::ALL.len() * BUDGETS_L.len() * 8 * REPETITIONS;
        let t = CORPUS.len() * Arm::ALL.len() * TEMPS_T.len() * 8 * REPETITIONS;
        assert_eq!(l, 3_456, "Design L per block");
        assert_eq!(t, 6_912, "Design T per block");
        assert_eq!(l + t, 10_368, "per block");
        assert_eq!(3 * (l + t), 31_104, "across pilot + held-in + held-out");
    }

    #[test]
    fn condition_grids_have_the_frozen_size() {
        assert_eq!(conditions_l().len(), 6 * 2 * 4);
        assert_eq!(conditions_t().len(), 6 * 2 * 8);
    }

    /// Amendment 1 §A4: equal TOTAL time, not equal slope.
    #[test]
    fn k_prime_solves_equal_total_time() {
        // a_M + b_M*16 = a_G + b_G*k'  ->  10 + 2*16 = 42 ; 2 + 4*k' = 42 -> k'=10
        assert_eq!(k_prime(10.0, 2.0, 2.0, 4.0), Some(10));
        // Equal intercepts degenerate to the slope ratio, but only then.
        assert_eq!(k_prime(5.0, 4.0, 5.0, 2.0), Some(32));
        // The superseded ratio-of-slopes form would give 16*2/4 = 8 here; the
        // correct equation gives 10, so the two are genuinely different.
        assert_ne!(k_prime(10.0, 2.0, 2.0, 4.0), Some(8));
        // Non-positive or non-finite slope cannot define a budget.
        assert_eq!(k_prime(1.0, 1.0, 1.0, 0.0), None);
        assert_eq!(k_prime(0.0, 1.0, 100.0, 1.0), None); // k' would be < 1
    }

    #[test]
    fn ols_recovers_a_known_line_and_rejects_no_spread() {
        let xs = [4.0, 8.0, 16.0, 24.0];
        let ys = [
            12.0 + 3.0 * 4.0,
            12.0 + 3.0 * 8.0,
            12.0 + 3.0 * 16.0,
            12.0 + 3.0 * 24.0,
        ];
        let (a, b) = ols(&xs, &ys).expect("fit");
        assert!((a - 12.0).abs() < 1e-9 && (b - 3.0).abs() < 1e-9);
        // Amendment 1 §A5: a single repeated x has no spread and must not fit.
        assert!(ols(&[0.0, 0.0, 0.0], &[1.0, 2.0, 3.0]).is_none());
    }

    /// Amendment 1 §A2: S = (n + 2m) * r, exactly `work_per_sweep`'s form.
    #[test]
    fn shape_work_matches_the_shipped_form() {
        let Ok(ir) = load(CORPUS[1].path) else { return };
        let m = ir.col_idx.len() / 2;
        assert_eq!(
            shape_work(&ir, REPLICAS),
            ((ir.n + 2 * m) * REPLICAS) as f64
        );
    }

    /// Amendment 2 §B3: both guards, and no clamping.
    #[test]
    fn degeneracy_guards_are_two_sided() {
        let res = 0.001;
        assert!(degenerate_timing(0.0, res), "zero duration is degenerate");
        assert!(
            degenerate_timing(-1.0, res),
            "negative duration is degenerate"
        );
        assert!(
            degenerate_timing(0.05, res),
            "below 100x resolution is degenerate"
        );
        assert!(
            !degenerate_timing(0.5, res),
            "above 100x resolution is fine"
        );
    }

    #[test]
    fn provenance_refuses_each_missing_condition() {
        let good = DocFacts {
            exists: true,
            tracked: true,
            clean: true,
            commit_secs: Some(100),
        };
        assert!(prereg_gate(&good).is_ok());
        for (f, want) in [
            (
                DocFacts {
                    exists: false,
                    ..good
                },
                "missing",
            ),
            (
                DocFacts {
                    tracked: false,
                    ..good
                },
                "tracked",
            ),
            (
                DocFacts {
                    clean: false,
                    ..good
                },
                "uncommitted",
            ),
            (
                DocFacts {
                    commit_secs: None,
                    ..good
                },
                "never been committed",
            ),
        ] {
            let e = prereg_gate(&f).expect_err("must refuse");
            assert!(e.contains(want), "message {e:?} should name {want:?}");
        }
    }

    #[test]
    fn held_out_needs_confirmation_and_a_newer_descendant() {
        let d = DocFacts {
            exists: true,
            tracked: true,
            clean: true,
            commit_secs: Some(200),
        };
        assert!(descendant_gate(&d, Some(100), true).is_ok());
        assert!(
            descendant_gate(&d, Some(100), false).is_err(),
            "needs the flag"
        );
        assert!(
            descendant_gate(&d, None, true).is_err(),
            "needs held-in first"
        );
        assert!(
            descendant_gate(&d, Some(200), true).is_err(),
            "must be STRICTLY newer"
        );
        assert!(
            descendant_gate(&d, Some(300), true).is_err(),
            "older descendant refused"
        );
        let dirty = DocFacts { clean: false, ..d };
        assert!(
            descendant_gate(&dirty, Some(100), true).is_err(),
            "dirty descendant refused"
        );
    }

    #[test]
    fn permutation_is_deterministic_and_total() {
        let a = permutation(48, 7);
        let b = permutation(48, 7);
        assert_eq!(a, b, "same seed must give the same order");
        let mut s = a.clone();
        s.sort_unstable();
        assert_eq!(s, (0..48).collect::<Vec<_>>(), "must be a permutation");
        assert_ne!(permutation(48, 8), a, "different repetition must reorder");
    }

    #[test]
    fn frozen_constants_match_the_preregistration() {
        assert_eq!(BUDGETS_L, [4, 8, 16, 24]);
        assert_eq!(TEMPS_T, [0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0]);
        assert_eq!((PREFIX_SWEEPS, WINDOW_SWEEPS), (4, 8));
        assert_eq!((REPLICAS, REPETITIONS), (32, 9));
        assert_eq!(CORPUS.len(), 6);
        assert_eq!(CI_BASE_SEED, 20261001);
        assert_eq!(PERM_BASE_SEED, 20261101);
        assert!((PHI_CLIP - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn ci_seeds_are_distinct_across_instances_and_blocks() {
        let mut seen = std::collections::BTreeSet::new();
        for i in CORPUS.iter() {
            for b in [Block::HeldIn, Block::HeldOut, Block::Pilot] {
                let s = CI_BASE_SEED + 2 * i.index as u64 + b.ci_offset();
                assert!(seen.insert(s), "CI seed collision at {s}");
            }
        }
    }

    #[test]
    fn bootstrap_ci_is_deterministic_and_ordered() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let a = bootstrap_ci(&xs, 2_000, 42);
        let b = bootstrap_ci(&xs, 2_000, 42);
        assert_eq!(a, b);
        assert!(a.0 <= a.1);
    }
}
