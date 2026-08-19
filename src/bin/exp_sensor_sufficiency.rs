//! RC-016 — does the first-slot operator choice carry per-instance information?
//!
//! Pre-registered in `research/PREREG_RC016.md` as amended by Amendments 1, 2
//! and 3, all committed before this file existed. Where they differ the later
//! amendment controls.
//!
//! # What this measures
//!
//! ```text
//!   I_replace_work(instance) = Y(gibbs_color_sweep) − Y(metropolis_sweep)
//! ```
//!
//! substituted in the FIRST slot of `[X@16, greedy_descent@16]` at the deployed
//! ladder. Positive ⇒ the substitution was worse. Exact under the two RC-014
//! Phase-0 conditions: the pair is draw-identical, and the only downstream
//! operator (`greedy_descent`) draws nothing.
//!
//! # Scope — quotable verbatim, nothing beyond it
//!
//! One ordered pair · first slot · 16 sweeps · 16-sweep greedy finisher · one
//! geometric ladder 4.0→0.1 · 32 replicas · all-zeros init · unweighted G-Set ·
//! best-of-32-after-greedy · **`S₀` sufficiency only**. It licenses nothing about
//! the other 16 operators, capability selection (both operators declare the SAME
//! capabilities), the Evolution ordering search, the Policy operator head,
//! multi-slot schedules, other budgets, weighted instances, or `S₁`.
//!
//! ```text
//! cargo run --release --bin exp_sensor_sufficiency -- --controls
//! cargo run --release --bin exp_sensor_sufficiency -- --calibrate
//! cargo run --release --bin exp_sensor_sufficiency -- --science
//! cargo run --release --bin exp_sensor_sufficiency -- --holdout
//! cargo run --release --bin exp_sensor_sufficiency -- --verdict
//! ```

use ising_engine::engine_v2::context::RunContext;
use ising_engine::engine_v2::decision::{geometric_ladder, DecisionEngine};
use ising_engine::engine_v2::evolution::boxed_state;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::Budget;
use ising_engine::engine_v2::plan::{Phase, Plan, PlanStep};
use ising_engine::engine_v2::registry::OperatorRegistry;
use ising_engine::engine_v2::runtime::{Runtime, RuntimeView};
use ising_engine::engine_v2::state::SpinState;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

// ================================================================ STEP 1
// Frozen design. A change here is a protocol change, not a tuning knob.

const REPLICAS: usize = 32;
const TEMP_HI: f64 = 4.0;
const TEMP_LO: f64 = 0.1;
const SWEEPS: u32 = 16;
const OP_A: &str = "metropolis_sweep";
const OP_B: &str = "gibbs_color_sweep";

const HELD_IN: [u64; 8] = [1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008];
const HELD_OUT: [u64; 8] = [2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008];
/// Disjoint from both, so calibration cannot contaminate either block.
const PILOT: [u64; 8] = [3001, 3002, 3003, 3004, 3005, 3006, 3007, 3008];

/// Amendment 1 §A6.2.
const POWER_BOOTSTRAP_SEED: u64 = 20260819;
/// Amendment 3 §C1: `seed(instance, block) = BASE + 2·index + block`.
const CI_BOOTSTRAP_BASE_SEED: u64 = 20260820;
const BOOTSTRAP_REPS: usize = 100_000;

const ALPHA: f64 = 0.05;
const FDR_Q: f64 = 0.10;
const RHO_BAR: f64 = 0.5;
const REL_BAR: f64 = 0.001;
/// Amendment 2 §B4: `z ≥ 3` ⇒ `p_floor = 2^(z+1)/256 ≥ 0.0625 > α`.
const TIE_BLOCK_Z: usize = 3;
/// Amendment 1 §A3: SIGN CONSTANT needs ≥6 qualifying instances.
const MIN_QUALIFYING: usize = 6;
/// …spanning ≥3 matched groups and both structural families.
const MIN_GROUPS: usize = 3;
/// Amendment 1 §A4 kill criterion 2.
const DEGENERATE_FRACTION_KILL: f64 = 0.50;
/// Legacy Amendment 1 §A5.3 diagnostic band. Amendment 4 §D2 withdraws every
/// equal-cost inference from this ratio; the bounds remain only in the frozen
/// calibration record.
const COST_RATIO_LO: f64 = 0.95;
const COST_RATIO_HI: f64 = 1.05;
/// Amendment 1 §A6.2 step 6.
const POWER_BAR: f64 = 0.80;

/// The held-out block may not run until a falsifiable descendant has been
/// written AND committed, and committed AFTER the held-in results exist
/// (`PREREG_RC016.md` §5; the discipline RC-014 Gate A condition 6 established).
const DESCENDANT_PATH: &str = "research/PREREG_RC016_DESCENDANT.md";
/// …and the operator must say so explicitly, so `--holdout` cannot fire by
/// accident from a stale shell history.
const CONFIRM_FLAG: &str = "--confirm-descendant-frozen";
const AMENDMENT4_PATH: &str = "research/PREREG_RC016_AMENDMENT_4.md";

/// Structural family, recorded per instance because SIGN CONSTANT's coverage
/// requirement is stated over families, not only over matched groups.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Family {
    /// Random graph, all weights `+1`.
    RandomPlus,
    /// Toroidal grid, weights ±1.
    Toroidal,
}

/// The corpus, in the EXACT order of `PREREG_RC016.md` §4. The array position
/// IS the zero-based corpus index Amendment 3 §C1 binds the CI seed to, so the
/// index is fixed by construction and cannot drift.
///
/// `(name, path, matched-group id, family)` — group ids A..F match §4's table;
/// `None` means the instance is in no matched group.
/// One pre-registered instance. `sha12` is the first 12 hex chars of the file's
/// SHA-256, transcribed from `PREREG_RC016.md` §4 — recorded there because
/// Phase 0 §0.2 established that `ExperimentDb` stores `instance_id` as a NAME,
/// not a hash, so a changed benchmark file is otherwise undetectable.
struct Inst {
    name: &'static str,
    path: &'static str,
    group: Option<char>,
    family: Family,
    sha12: &'static str,
}

/// The corpus, in the EXACT order of `PREREG_RC016.md` §4. The array position
/// IS the zero-based corpus index Amendment 3 §C1 binds the CI seed to, so the
/// index is fixed by construction and cannot drift.
const CORPUS: &[Inst] = &[
    Inst {
        name: "G1",
        path: "benchmark_suite/data/gset/G1",
        group: Some('B'),
        family: Family::RandomPlus,
        sha12: "73bf704d8ffc",
    },
    Inst {
        name: "G2",
        path: "benchmark_suite/data/gset/G2",
        group: Some('B'),
        family: Family::RandomPlus,
        sha12: "732d57480a01",
    },
    Inst {
        name: "G3",
        path: "benchmark_suite/data/gset/G3",
        group: Some('B'),
        family: Family::RandomPlus,
        sha12: "999e49b5e093",
    },
    Inst {
        name: "G11",
        path: "benchmark_suite/data/gset/G11",
        group: Some('A'),
        family: Family::Toroidal,
        sha12: "c2a760d2926d",
    },
    Inst {
        name: "G12",
        path: "benchmark_suite/data/gset/G12",
        group: Some('A'),
        family: Family::Toroidal,
        sha12: "a8628108d95d",
    },
    Inst {
        name: "G13",
        path: "benchmark_suite/data/gset/G13",
        group: Some('A'),
        family: Family::Toroidal,
        sha12: "44af0d3aa232",
    },
    Inst {
        name: "G14",
        path: "benchmark_suite/data/gset/G14",
        group: None,
        family: Family::RandomPlus,
        sha12: "dc769b978a40",
    },
    Inst {
        name: "G15",
        path: "benchmark_suite/data/gset/G15",
        group: None,
        family: Family::RandomPlus,
        sha12: "2f1808f074bc",
    },
    Inst {
        name: "G16",
        path: "benchmark_suite/data/gset/G16",
        group: None,
        family: Family::RandomPlus,
        sha12: "5a70eec4649a",
    },
    Inst {
        name: "G22",
        path: "benchmark_suite/data/gset/G22",
        group: Some('E'),
        family: Family::RandomPlus,
        sha12: "9baeee06eb14",
    },
    Inst {
        name: "G23",
        path: "benchmark_suite/data/gset/G23",
        group: Some('E'),
        family: Family::RandomPlus,
        sha12: "3669c719ebbc",
    },
    Inst {
        name: "G24",
        path: "benchmark_suite/data/gset/G24",
        group: Some('E'),
        family: Family::RandomPlus,
        sha12: "9aff2abd74d1",
    },
    Inst {
        name: "G32",
        path: "benchmark_suite/data/gset/G32",
        group: Some('D'),
        family: Family::Toroidal,
        sha12: "9760fce6b601",
    },
    Inst {
        name: "G33",
        path: "benchmark_suite/data/gset/G33",
        group: Some('D'),
        family: Family::Toroidal,
        sha12: "4791e1bd9ac2",
    },
    Inst {
        name: "G34",
        path: "benchmark_suite/data/gset/G34",
        group: Some('D'),
        family: Family::Toroidal,
        sha12: "e84c77938fcd",
    },
    Inst {
        name: "G35",
        path: "benchmark_suite/data/gset/G35",
        group: None,
        family: Family::RandomPlus,
        sha12: "3df35a2abbe8",
    },
    Inst {
        name: "G36",
        path: "benchmark_suite/data/gset/G36",
        group: None,
        family: Family::RandomPlus,
        sha12: "022423749f4e",
    },
    Inst {
        name: "G43",
        path: "benchmark_suite/data/gset/G43",
        group: Some('C'),
        family: Family::RandomPlus,
        sha12: "9af5445b4b06",
    },
    Inst {
        name: "G44",
        path: "benchmark_suite/data/gset/G44",
        group: Some('C'),
        family: Family::RandomPlus,
        sha12: "929e7687b9a0",
    },
    Inst {
        name: "G45",
        path: "benchmark_suite/data/gset/G45",
        group: Some('C'),
        family: Family::RandomPlus,
        sha12: "e1514f22a23c",
    },
    Inst {
        name: "G48",
        path: "benchmark_suite/data/gset/G48",
        group: Some('F'),
        family: Family::RandomPlus,
        sha12: "2c2daba39d1f",
    },
    Inst {
        name: "G49",
        path: "benchmark_suite/data/gset/G49",
        group: Some('F'),
        family: Family::RandomPlus,
        sha12: "01733a64e25e",
    },
    Inst {
        name: "G50",
        path: "benchmark_suite/data/gset/G50",
        group: Some('F'),
        family: Family::RandomPlus,
        sha12: "d3f5f31c5089",
    },
    Inst {
        name: "G51",
        path: "benchmark_suite/data/gset/G51",
        group: None,
        family: Family::RandomPlus,
        sha12: "23b7111e929f",
    },
    Inst {
        name: "G52",
        path: "benchmark_suite/data/gset/G52",
        group: None,
        family: Family::RandomPlus,
        sha12: "48ab066f1a3b",
    },
    Inst {
        name: "G53",
        path: "benchmark_suite/data/gset/G53",
        group: None,
        family: Family::RandomPlus,
        sha12: "10ebfc718012",
    },
    Inst {
        name: "G55",
        path: "benchmark_suite/data/gset/G55",
        group: None,
        family: Family::RandomPlus,
        sha12: "7537bbb613a6",
    },
    Inst {
        name: "G60",
        path: "benchmark_suite/data/gset/G60",
        group: None,
        family: Family::RandomPlus,
        sha12: "b6480c1716ec",
    },
    Inst {
        name: "G63",
        path: "benchmark_suite/data/gset/G63",
        group: None,
        family: Family::RandomPlus,
        sha12: "a1d08e1eed7a",
    },
    Inst {
        name: "G70",
        path: "benchmark_suite/data/gset/G70",
        group: None,
        family: Family::RandomPlus,
        sha12: "0d965a2ff144",
    },
];

/// Held-in is block 0, held-out block 1 (Amendment 3 §C1).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    HeldIn,
    HeldOut,
}

impl Block {
    fn index(self) -> u64 {
        match self {
            Block::HeldIn => 0,
            Block::HeldOut => 1,
        }
    }
    fn seeds(self) -> &'static [u64; 8] {
        match self {
            Block::HeldIn => &HELD_IN,
            Block::HeldOut => &HELD_OUT,
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Block::HeldIn => "held-in",
            Block::HeldOut => "held-out",
        }
    }
}

// ================================================================ STEP 2 + 3
// Statistics, copied verbatim from the RC-014 instrument (validated there by the
// synthetic arithmetic positive), plus the zero-difference count and
// TIE-BLOCKED status that Amendment 2 §B4 adds.

struct Paired {
    /// Reference arm — `metropolis_sweep`. `d_seed` is this arm's sd (§6.1).
    base: Vec<f64>,
    other: Vec<f64>,
}

impl Paired {
    fn effects(&self) -> Vec<f64> {
        self.other
            .iter()
            .zip(&self.base)
            .map(|(a, b)| a - b)
            .collect()
    }

    fn mean_effect(&self) -> f64 {
        let e = self.effects();
        e.iter().sum::<f64>() / e.len() as f64
    }

    fn d_seed(&self) -> f64 {
        let n = self.base.len() as f64;
        if n < 2.0 {
            return 0.0;
        }
        let m = self.base.iter().sum::<f64>() / n;
        (self.base.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    }

    fn degenerate(&self) -> bool {
        self.d_seed() < 1e-9
    }

    fn rho(&self) -> Option<f64> {
        if self.degenerate() {
            None
        } else {
            Some(self.mean_effect().abs() / self.d_seed())
        }
    }

    fn rel(&self) -> f64 {
        let s = (self.base.iter().sum::<f64>() / self.base.len() as f64).abs();
        if s < 1e-12 {
            0.0
        } else {
            self.mean_effect().abs() / s
        }
    }

    /// Materiality is a CONJUNCTION (`PREREG_RC016.md` §6.1).
    fn material(&self) -> bool {
        !self.degenerate() && self.rho().is_some_and(|r| r >= RHO_BAR) && self.rel() >= REL_BAR
    }

    /// STEP 3 — Amendment 2 §B4. A zero paired difference is unchanged by a sign
    /// flip, so it ties pairs of assignments: `p_floor(z) = 2^(z+1)/256`.
    fn zeros(&self) -> usize {
        self.effects().iter().filter(|d| **d == 0.0).count()
    }

    /// `z ≥ 3` ⇒ `p_floor ≥ 0.0625 > α` ⇒ the instance CANNOT reject, whatever
    /// the magnitude of the remaining differences. It can never QUALIFY and is
    /// routed to Q-INCONCLUSIVE, never to NO MATERIAL EFFECT OBSERVED.
    fn tie_blocked(&self) -> bool {
        self.zeros() >= TIE_BLOCK_Z
    }

    /// Exact two-sided sign-flip over all `2^k` assignments. No approximation.
    fn signflip_p(&self) -> f64 {
        let e = self.effects();
        let k = e.len();
        assert!(k <= 20, "exact sign-flip enumeration is 2^k");
        let obs = (e.iter().sum::<f64>() / k as f64).abs();
        let total = 1usize << k;
        let mut ge = 0usize;
        for mask in 0..total {
            let mut s = 0.0;
            for (i, v) in e.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    s -= v;
                } else {
                    s += v;
                }
            }
            if (s / k as f64).abs() >= obs {
                ge += 1;
            }
        }
        ge as f64 / total as f64
    }
}

/// Benjamini–Hochberg step-up. Rejection flags in input order.
fn benjamini_hochberg(p: &[f64], q: f64) -> Vec<bool> {
    let m = p.len();
    let mut idx: Vec<usize> = (0..m).collect();
    idx.sort_by(|&a, &b| p[a].total_cmp(&p[b]));
    let mut cut = None;
    for (rank, &i) in idx.iter().enumerate() {
        if p[i] <= (rank + 1) as f64 / m as f64 * q {
            cut = Some(rank);
        }
    }
    let mut out = vec![false; m];
    if let Some(c) = cut {
        for &i in idx.iter().take(c + 1) {
            out[i] = true;
        }
    }
    out
}

/// Percentile bootstrap of the MEAN, seeded. Percentile indices `reps/40` and
/// `reps-1-reps/40` are the 2.5th and 97.5th (Amendment 3 §C1 clause 4).
fn bootstrap_ci(xs: &[f64], reps: usize, seed: u64) -> (f64, f64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut means: Vec<f64> = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut s = 0.0;
        for _ in 0..xs.len() {
            s += xs[rng.gen_range(0..xs.len())];
        }
        means.push(s / xs.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    (means[reps / 40], means[reps - 1 - reps / 40])
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

// ------------------------------------------------------------------ plumbing

fn flag(n: &str) -> bool {
    std::env::args().any(|a| a == n)
}

fn arg<T: std::str::FromStr>(name: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn load(path: &str) -> Result<ProblemIR, String> {
    rudy_maxcut_ir(&std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)
}

/// The pre-registered plan: `[X@sweeps, greedy_descent@16]`, deployed ladder,
/// legacy all-zeros init.
fn plan_for(ir: &ProblemIR, x: &str, sweeps: u32, seed: u64) -> Plan {
    Plan {
        name: "rc016".into(),
        backend: DecisionEngine::analyze(ir).select_backend(),
        num_replicas: REPLICAS,
        temperatures: geometric_ladder(REPLICAS, TEMP_HI, TEMP_LO),
        steps: vec![
            PlanStep {
                operator: x.into(),
                phase: Phase::Exploit,
                sweeps,
                repeat: 1,
            },
            PlanStep {
                operator: "greedy_descent".into(),
                phase: Phase::Exploit,
                sweeps: SWEEPS,
                repeat: 1,
            },
        ],
        seed,
        rationale: Default::default(),
    }
}

struct RunOut {
    y: f64,
    best_state: Vec<u8>,
    /// Wall time of the whole run, measured at the call site because
    /// `Profiler` aggregates by operator NAME, not per invocation.
    ms: f64,
}

fn execute(ir: &ProblemIR, reg: &OperatorRegistry, plan: &Plan) -> Result<RunOut, String> {
    let init = vec![0u8; ir.n];
    let mut state = boxed_state(ir, plan.backend, plan.num_replicas, &init);
    let mut rt = Runtime::new(RunContext::new(plan.seed), plan);
    let t0 = Instant::now();
    let rec = rt.run(plan, state.as_mut(), reg, ir)?;
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    Ok(RunOut {
        y: rec.best_energy,
        best_state: rec.best_state.clone(),
        ms,
    })
}

/// One contrast: reference arm `metropolis@16`, other arm `X@sweeps_b`.
fn contrast(
    ir: &ProblemIR,
    reg: &OperatorRegistry,
    seeds: &[u64],
    sweeps_b: u32,
) -> Option<Paired> {
    let (mut base, mut other) = (Vec::new(), Vec::new());
    for &s in seeds {
        let a = execute(ir, reg, &plan_for(ir, OP_A, SWEEPS, s)).ok()?;
        let b = execute(ir, reg, &plan_for(ir, OP_B, sweeps_b, s)).ok()?;
        base.push(a.y);
        other.push(b.y);
    }
    Some(Paired { base, other })
}

// --------------------------------------------------------------- sha256
//
// Implemented inline rather than added as a dependency: `sha2` is only a
// TRANSITIVE dep (via ethers) and `Cargo.toml` is out of scope, while shelling
// out to `sha256sum` would make a replayable research artifact depend on the
// environment. A known-answer test pins it, so it is verified rather than
// trusted.

const SHA_K: [u32; 64] = [
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

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
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
                .wrapping_add(SHA_K[i])
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
        for (slot, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(v);
        }
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}

// ================================================================ STEP 9
// Controls, cited from RC-014/RC-015 and RE-RUN here because corpus, instance
// sizes and code state differ. Any failure ⇒ kill criterion 3: the instrument is
// invalid and nothing measured is evidence about the hypothesis.

/// Draw-alignment probe at the operator boundary. The load-bearing measurement:
/// both operators must leave the generator at the same position, and an injected
/// one-word shift must be DETECTED so the probe cannot pass vacuously.
fn control_alignment(ir: &ProblemIR, reg: &OperatorRegistry) -> bool {
    let backend = DecisionEngine::analyze(ir).select_backend();
    let init = vec![0u8; ir.n];
    let temps = geometric_ladder(REPLICAS, TEMP_HI, TEMP_LO);
    let view = RuntimeView {
        iteration: 0,
        temperatures: &temps,
        num_replicas: REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    };
    let probe = |op: &str, shift: bool| -> u64 {
        let mut st = boxed_state(ir, backend, REPLICAS, &init);
        let mut rng = ChaCha8Rng::seed_from_u64(HELD_IN[0]);
        if shift {
            let _: u32 = rng.gen();
        }
        let mut o = reg.lookup(op).expect("registered");
        o.apply(st.as_mut(), &view, &mut rng, Budget { sweeps: SWEEPS });
        rng.gen()
    };
    let (pa, pb, ps) = (probe(OP_A, false), probe(OP_B, false), probe(OP_B, true));
    let aligned = pa == pb;
    let detected = ps != pa;
    println!(
        "    metropolis={pa:#018x} gibbs={pb:#018x} -> {}",
        if aligned {
            "EQUAL (pass)"
        } else {
            "DIFFER (FAIL)"
        }
    );
    println!(
        "    shifted gibbs={ps:#018x} -> {}",
        if detected {
            "DETECTED (pass)"
        } else {
            "MISSED (FAIL)"
        }
    );
    aligned && detected
}

/// Null: substituting the reference for itself must be bit-identical.
fn control_null(ir: &ProblemIR, reg: &OperatorRegistry) -> bool {
    let mut ok = true;
    for &s in HELD_IN.iter().take(3) {
        let (a, b) = match (
            execute(ir, reg, &plan_for(ir, OP_A, SWEEPS, s)),
            execute(ir, reg, &plan_for(ir, OP_A, SWEEPS, s)),
        ) {
            (Ok(a), Ok(b)) => (a, b),
            _ => return false,
        };
        let same = a.y.to_bits() == b.y.to_bits() && a.best_state == b.best_state;
        println!("    seed {s}: Y={:.4} replay-identical={}", a.y, same);
        ok &= same;
    }
    ok
}

/// Inert: substitution at zero budget consumes no draws and changes no state.
fn control_inert_zero_budget(ir: &ProblemIR, reg: &OperatorRegistry) -> bool {
    let (a, b) = match (
        execute(ir, reg, &plan_for(ir, OP_A, 0, HELD_IN[0])),
        execute(ir, reg, &plan_for(ir, OP_B, 0, HELD_IN[0])),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    let i = b.y - a.y;
    println!(
        "    I(sweeps=0) = {i:.17e} -> {}",
        if i == 0.0 {
            "exactly 0 (pass)"
        } else {
            "NONZERO (FAIL)"
        }
    );
    i == 0.0
}

/// Synthetic arithmetic positive: a fixed paired table with analytically known
/// effects, so the attribution / materiality / degenerate-null layer is checked
/// with no solver in the loop. An attribution bug and an absent effect look
/// identical in the output otherwise.
fn control_synthetic_arithmetic() -> bool {
    let p = Paired {
        base: vec![-10.0, -20.0, -30.0, -40.0],
        other: vec![-9.0, -18.0, -27.0, -36.0],
    };
    let eff = p.effects();
    let eff_ok = eff
        .iter()
        .zip([1.0, 2.0, 3.0, 4.0])
        .all(|(a, b)| (a - b).abs() < 1e-12);
    let mean_ok = (p.mean_effect() - 2.5).abs() < 1e-12;
    let sd_ok = (p.d_seed() - (500.0f64 / 3.0).sqrt()).abs() < 1e-12;
    let zeros_ok = p.zeros() == 0 && !p.tie_blocked();

    let flat = Paired {
        base: vec![-10.0; 4],
        other: vec![-9.0, -18.0, -27.0, -36.0],
    };
    let degen_ok = flat.degenerate() && flat.rho().is_none();

    // STEP 3 guard: three zeros must trip TIE-BLOCKED and the p-floor must be
    // 2^(z+1)/2^k, above alpha.
    let tied = Paired {
        base: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        other: vec![1.0, 2.0, 3.0, 5.0, 6.0, 7.0, 8.0, 9.0],
    };
    let tie_ok = tied.zeros() == 3 && tied.tie_blocked() && tied.signflip_p() >= ALPHA;

    println!(
        "    effects={eff:?} mean={:.12} d_seed={:.12}",
        p.mean_effect(),
        p.d_seed()
    );
    println!("    constant-base -> DEGENERATE_NULL: {degen_ok}");
    println!(
        "    z=3 fixture -> zeros={} tie_blocked={} p={:.4} (>= alpha {ALPHA})",
        tied.zeros(),
        tied.tie_blocked(),
        tied.signflip_p()
    );
    let ok = eff_ok && mean_ok && sd_ok && zeros_ok && degen_ok && tie_ok;
    println!(
        "    {}",
        if ok {
            "attribution arithmetic PASS"
        } else {
            "attribution arithmetic FAIL"
        }
    );
    ok
}

/// Seed uniqueness for the sensitivity CI (Amendment 3 §C1): 30 instances × 2
/// blocks must give 60 distinct seeds, none colliding with the power seed.
fn control_ci_seed_space() -> bool {
    let mut seeds: Vec<u64> = Vec::new();
    for i in 0..CORPUS.len() as u64 {
        for b in 0..2u64 {
            seeds.push(CI_BOOTSTRAP_BASE_SEED + 2 * i + b);
        }
    }
    let mut uniq = seeds.clone();
    uniq.sort_unstable();
    uniq.dedup();
    let distinct = uniq.len() == seeds.len();
    let no_collision = !seeds.contains(&POWER_BOOTSTRAP_SEED);
    println!(
        "    {} seeds, {} distinct, range {}..{}, collides with power seed: {}",
        seeds.len(),
        uniq.len(),
        uniq.first().copied().unwrap_or(0),
        uniq.last().copied().unwrap_or(0),
        !no_collision
    );
    distinct && no_collision
}

/// The corpus must be exactly the 30 enumerated files, all present, in §4 order.
fn control_corpus() -> bool {
    let mut bad: Vec<String> = Vec::new();
    for inst in CORPUS {
        match std::fs::read(inst.path) {
            Ok(bytes) => {
                let got = &sha256_hex(&bytes)[..12];
                if got != inst.sha12 {
                    bad.push(format!(
                        "{} sha {} != prereg {}",
                        inst.name, got, inst.sha12
                    ));
                }
            }
            Err(e) => bad.push(format!("{} unreadable: {e}", inst.name)),
        }
    }
    let count_ok = CORPUS.len() == 30;
    println!(
        "    {} instances declared, {} hash/read failures",
        CORPUS.len(),
        bad.len()
    );
    for b in &bad {
        println!("    FAIL: {b}");
    }
    count_ok && bad.is_empty()
}

/// Inert control B (RC-014): from all-zeros every replica is identical, so a
/// `replica_exchange` step can only permute identical states and is
/// STATE-INERT. Deleting it must leave best state and energy untouched — and it
/// can only do so because the sole downstream operator draws nothing, so the
/// stream shift the deletion causes reaches nobody.
fn control_inert_replica_exchange(ir: &ProblemIR, reg: &OperatorRegistry) -> bool {
    let mk = |with: bool| {
        let mut steps = Vec::new();
        if with {
            steps.push(PlanStep {
                operator: "replica_exchange".into(),
                phase: Phase::Exploit,
                sweeps: SWEEPS,
                repeat: 1,
            });
        }
        steps.push(PlanStep {
            operator: "greedy_descent".into(),
            phase: Phase::Exploit,
            sweeps: SWEEPS,
            repeat: 1,
        });
        Plan {
            name: "rc016-inertB".into(),
            backend: DecisionEngine::analyze(ir).select_backend(),
            num_replicas: REPLICAS,
            temperatures: geometric_ladder(REPLICAS, TEMP_HI, TEMP_LO),
            steps,
            seed: HELD_IN[0],
            rationale: Default::default(),
        }
    };
    let (a, b) = match (execute(ir, reg, &mk(true)), execute(ir, reg, &mk(false))) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    let ok = a.y.to_bits() == b.y.to_bits() && a.best_state == b.best_state;
    println!(
        "    with={:.4} without={:.4} state-identical={} -> {}",
        a.y,
        b.y,
        a.best_state == b.best_state,
        if ok { "pass" } else { "FAIL" }
    );
    ok
}

// --------------------------- synthetic operator positive (RC-014, ported)
//
// An INDEPENDENT reference written against `ProblemIR` alone, sharing no code
// with the operators, so agreement is evidence rather than tautology. This is
// the only control that can catch a wrong operator implementation.

fn small_instance(n: usize, seed: u64) -> ProblemIR {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut pairs = Vec::new();
    for i in 0..n as u32 {
        for j in (i + 1)..n as u32 {
            // Integer weights only: SparseBitSlice rejects non-integral couplings.
            let w: i32 = rng.gen_range(-2..=2);
            if w != 0 {
                pairs.push((i, j, w as f64));
            }
        }
    }
    let linear: Vec<f64> = (0..n).map(|_| rng.gen_range(-2..=2) as f64).collect();
    ProblemIR::from_pairs(n, 0.0, linear, &pairs)
}

/// The colour order both operators walk: colours ascending, sites ascending
/// within a colour. Derived from the IR only, never from the operator.
fn colour_order(state: &dyn SpinState) -> Vec<usize> {
    let colors = state.coloring();
    let ncolors = colors.iter().copied().max().map_or(0, |c| c as usize + 1);
    let mut order = Vec::with_capacity(colors.len());
    for c in 0..ncolors as u32 {
        for (site, &col) in colors.iter().enumerate() {
            if col == c {
                order.push(site);
            }
        }
    }
    order
}

#[allow(clippy::too_many_arguments)]
fn reference_apply(
    ir: &ProblemIR,
    order: &[usize],
    temps: &[f64],
    gibbs: bool,
    sweeps: u32,
    replicas: usize,
    rng: &mut ChaCha8Rng,
) -> f64 {
    let (n, r) = (ir.n, replicas);
    let mut x = vec![0u8; n * r]; // site-major: x[site*r + rep]
    for _ in 0..sweeps {
        for &site in order {
            let (cols, ws) = ir.row(site);
            let mut flip = vec![false; r];
            for (rep, f) in flip.iter_mut().enumerate() {
                // Local field h = l_i + sum_j q_ij x_j, recomputed from scratch.
                let mut h = ir.linear[site];
                for (&j, &w) in cols.iter().zip(ws) {
                    if x[j as usize * r + rep] != 0 {
                        h += w;
                    }
                }
                let xi = x[site * r + rep] != 0;
                let de = if xi { -h } else { h };
                let t = temps[rep % temps.len()];
                if gibbs {
                    let bh = if t > 0.0 {
                        h / t
                    } else if h > 0.0 {
                        f64::INFINITY
                    } else if h < 0.0 {
                        f64::NEG_INFINITY
                    } else {
                        0.0
                    };
                    let p1 = 1.0 / (1.0 + bh.exp());
                    let u: f64 = rng.gen();
                    *f = (u < p1) != xi;
                } else if de <= 0.0 {
                    let _u: f64 = rng.gen(); // alignment draw, discarded
                    *f = true;
                } else {
                    let p = if t > 0.0 { (-de / t).exp() } else { 0.0 };
                    let u: f64 = rng.gen();
                    *f = u < p;
                }
            }
            for (rep, &f) in flip.iter().enumerate() {
                if f {
                    x[site * r + rep] ^= 1;
                }
            }
        }
    }
    let mut best = f64::INFINITY;
    let mut buf = vec![0u8; n];
    for rep in 0..r {
        for (i, b) in buf.iter_mut().enumerate() {
            *b = x[i * r + rep];
        }
        let e = ir.energy(&buf);
        if e < best {
            best = e;
        }
    }
    best
}

/// Two separable claims, reported separately: AGREEMENT on every candidate
/// (the actual check, which no choice of fixture can make pass), and
/// NON-VACUITY — at least one fixture with a non-zero reference difference,
/// without which agreement is trivially satisfied by both operators landing on
/// the same optimum (the RC-009 lesson).
fn control_synthetic_operator(reg: &OperatorRegistry) -> bool {
    let mut checked = 0usize;
    let mut agreed = 0usize;
    let mut accepted: Option<(usize, u64, u32, usize, f64, f64, f64)> = None;
    let mut first_disagreement: Option<String> = None;

    'grid: for &replicas in &[2usize, 4, 8] {
        let temps = geometric_ladder(replicas, TEMP_HI, TEMP_LO);
        for &sweeps in &[1u32, 2, 4] {
            for n in [12usize, 16, 20] {
                for seed in 1..25u64 {
                    let ir = small_instance(n, seed);
                    if ir.num_pairs() == 0 {
                        continue;
                    }
                    let backend = DecisionEngine::analyze(&ir).select_backend();
                    let init = vec![0u8; ir.n];
                    let probe = boxed_state(&ir, backend, replicas, &init);
                    let order = colour_order(probe.as_ref());
                    drop(probe);

                    let mut r1 = ChaCha8Rng::seed_from_u64(seed);
                    let mut r2 = ChaCha8Rng::seed_from_u64(seed);
                    let ref_a =
                        reference_apply(&ir, &order, &temps, false, sweeps, replicas, &mut r1);
                    let ref_b =
                        reference_apply(&ir, &order, &temps, true, sweeps, replicas, &mut r2);

                    let single = |op: &str| Plan {
                        name: "rc016-synth".into(),
                        backend,
                        num_replicas: replicas,
                        temperatures: temps.clone(),
                        steps: vec![PlanStep {
                            operator: op.into(),
                            phase: Phase::Exploit,
                            sweeps,
                            repeat: 1,
                        }],
                        seed,
                        rationale: Default::default(),
                    };
                    let (pa, pb) = match (
                        execute(&ir, reg, &single(OP_A)),
                        execute(&ir, reg, &single(OP_B)),
                    ) {
                        (Ok(a), Ok(b)) => (a, b),
                        _ => continue,
                    };

                    checked += 1;
                    let ok = pa.y == ref_a && pb.y == ref_b;
                    if ok {
                        agreed += 1;
                    } else if first_disagreement.is_none() {
                        first_disagreement = Some(format!(
                            "n={n} seed={seed} sweeps={sweeps} r={replicas}: reference \
                             (m={ref_a:.6}, g={ref_b:.6}) vs production (m={:.6}, g={:.6})",
                            pa.y, pb.y
                        ));
                    }

                    let ref_i = ref_b - ref_a;
                    if accepted.is_none() && ref_i != 0.0 && ok && pb.y - pa.y == ref_i {
                        accepted = Some((n, seed, sweeps, replicas, ref_a, ref_b, ref_i));
                        if checked >= 40 {
                            break 'grid;
                        }
                    }
                }
            }
        }
    }

    println!("    agreement: {agreed}/{checked} fixtures reproduce the independent reference");
    if let Some(d) = &first_disagreement {
        println!("    FIRST DISAGREEMENT: {d}");
    }
    match accepted {
        Some((n, seed, sweeps, replicas, a, b, i)) => {
            println!(
                "    accepted non-vacuous fixture: n={n} seed={seed} sweeps={sweeps} \
                 replicas={replicas}  metropolis={a:.6} gibbs={b:.6} I={i:+.6}"
            );
            agreed == checked
        }
        None => {
            println!(
                "    no fixture with a non-zero reference difference — control VACUOUS (FAIL)"
            );
            false
        }
    }
}

fn run_controls(reg: &OperatorRegistry) -> bool {
    println!("RC-016 controls (PREREG_RC016.md §9 — cited from RC-014/RC-015, RE-RUN here)\n");
    let mut pass = true;

    println!("[corpus] 30 enumerated instances present, in §4 order");
    pass &= control_corpus();

    println!("\n[ci-seeds] Amendment 3 §C1 seed space is collision-free");
    pass &= control_ci_seed_space();

    println!("\n[arithmetic] synthetic arithmetic positive + TIE-BLOCKED guard");
    pass &= control_synthetic_arithmetic();

    let probe_ir = match load(CORPUS[0].path) {
        Ok(ir) => ir,
        Err(e) => {
            println!("\ncannot load {}: {e}", CORPUS[0].path);
            return false;
        }
    };

    println!("\n[alignment] both operators leave the generator aligned; shift detected");
    pass &= control_alignment(&probe_ir, reg);

    println!("\n[null] metropolis -> metropolis is bit-identical");
    pass &= control_null(&probe_ir, reg);

    println!("\n[inert A] substitution at sweeps=0 gives exactly 0");
    pass &= control_inert_zero_budget(&probe_ir, reg);

    println!("\n[inert B] structurally inert replica_exchange deletion");
    pass &= control_inert_replica_exchange(&probe_ir, reg);

    println!("\n[operator positive] production operators vs an independent reference");
    pass &= control_synthetic_operator(reg);

    println!(
        "\nCONTROLS: {}",
        if pass {
            "ALL PASS — calibration and science authorised"
        } else {
            "FAILED — kill criterion 3: instrument invalid; nothing is evidence about H-16"
        }
    );
    pass
}

// ================================================================ STEP 5
// Calibration. Pilot seeds only, interleaved conditions, medians. Freezes R,
// Y_ref and d_seed_pilot BEFORE any science seed runs. `cost_model` is forbidden
// as a cost measure (RC-005); only measured wall time is used.

#[derive(Clone)]
struct Calibration {
    index: usize,
    name: String,
    /// median_ms(gibbs) / median_ms(metropolis)
    r: f64,
    /// |median over pilot of Y(metropolis)| — Amendment 2 §B3 / Amendment 1 §A6.1
    y_ref: f64,
    /// Reference-arm sd over the pilot (the `d_seed` of §6.1, arm-level).
    d_seed_pilot: f64,
    /// Amendment 2 §B3: the BINDING threshold, not the rel component alone.
    delta: f64,
    /// Bootstrap rejection rate (Amendment 1 §A6.2 as corrected).
    power: f64,
    powered: bool,
    /// Whether the cost arm is needed at all (Amendment 1 §A5.3).
    cost_arm_needed: bool,
}

fn calibrate_one(idx: usize, reg: &OperatorRegistry) -> Option<Calibration> {
    let inst = &CORPUS[idx];
    let (name, path) = (inst.name, inst.path);
    let ir = load(path).ok()?;

    // Interleaved within each repetition, so monotonic host drift cannot
    // manufacture a monotonic trend (PERF.md; RC-005's rule).
    let (mut ms_a, mut ms_b) = (Vec::new(), Vec::new());
    let (mut y_a, mut y_b) = (Vec::new(), Vec::new());
    for &s in PILOT.iter() {
        let a = execute(&ir, reg, &plan_for(&ir, OP_A, SWEEPS, s)).ok()?;
        let b = execute(&ir, reg, &plan_for(&ir, OP_B, SWEEPS, s)).ok()?;
        ms_a.push(a.ms);
        ms_b.push(b.ms);
        y_a.push(a.y);
        y_b.push(b.y);
    }
    let r = median(&mut ms_b.clone()) / median(&mut ms_a.clone()).max(1e-12);
    let y_ref = median(&mut y_a.clone()).abs();

    let pilot = Paired {
        base: y_a.clone(),
        other: y_b.clone(),
    };
    let d_seed_pilot = pilot.d_seed();

    // Amendment 2 §B3 — materiality is a conjunction, so power for the BINDING
    // component, not the relative one alone.
    let delta = (REL_BAR * y_ref).max(RHO_BAR * d_seed_pilot);

    let power = power_rate(&pilot.effects(), delta);
    let powered = power >= POWER_BAR;
    let cost_arm_needed = !(COST_RATIO_LO..=COST_RATIO_HI).contains(&r);

    Some(Calibration {
        index: idx,
        name: name.to_string(),
        r,
        y_ref,
        d_seed_pilot,
        delta,
        power,
        powered,
        cost_arm_needed,
    })
}

// ================================================================ STEP 6
// Power bootstrap — MODEL-CONDITIONAL (Amendment 1 §A6.4, extended by
// Amendment 2 §B5 and scoped by Amendment 3 §C2).
//
// Assumptions, stated wherever a power verdict is reported:
//   1. empirical additive location shift;
//   2. pilot representativeness;
//   3. the bootstrap runs on a continuum while the data lie on the integer
//      lattice — optimistic WITH RESPECT TO TIE-BLOCKING specifically and
//      holding all else equal; other pilot-vs-science mismatch can move power in
//      EITHER direction, so no overall bound is claimed.

fn power_rate(pilot_diffs: &[f64], delta: f64) -> f64 {
    let k = pilot_diffs.len();
    let mean = pilot_diffs.iter().sum::<f64>() / k as f64;
    let resid: Vec<f64> = pilot_diffs.iter().map(|d| d - mean).collect();
    let mut rng = ChaCha8Rng::seed_from_u64(POWER_BOOTSTRAP_SEED);
    let mut rejects = 0usize;
    let mut sample = vec![0.0f64; k];
    for _ in 0..BOOTSTRAP_REPS {
        for slot in sample.iter_mut() {
            *slot = resid[rng.gen_range(0..k)] + delta;
        }
        if exact_signflip_p(&sample) < ALPHA {
            rejects += 1;
        }
    }
    rejects as f64 / BOOTSTRAP_REPS as f64
}

/// Exact two-sided sign-flip p-value on a raw difference vector.
fn exact_signflip_p(d: &[f64]) -> f64 {
    let k = d.len();
    let obs = (d.iter().sum::<f64>() / k as f64).abs();
    let total = 1usize << k;
    let mut ge = 0usize;
    for mask in 0..total {
        let mut s = 0.0;
        for (i, v) in d.iter().enumerate() {
            if mask >> i & 1 == 1 {
                s -= v;
            } else {
                s += v;
            }
        }
        if (s / k as f64).abs() >= obs {
            ge += 1;
        }
    }
    ge as f64 / total as f64
}

// ================================================================ STEP 7
// Sensitivity CI — Amendment 3 §C1. Descriptive only: never an input to
// QUALIFIES, never an equivalence test, never in the multiplicity family, never
// compared against the materiality bar to produce a verdict.

struct Sensitivity {
    lo: f64,
    hi: f64,
    /// Half-width relative to |mean Y of the REFERENCE (metropolis) arm| in this
    /// block, in percent. The denominator is named explicitly.
    half_width_pct: f64,
}

fn sensitivity_ci(p: &Paired, index: usize, block: Block) -> Sensitivity {
    let seed = CI_BOOTSTRAP_BASE_SEED + 2 * index as u64 + block.index();
    let (lo, hi) = bootstrap_ci(&p.effects(), BOOTSTRAP_REPS, seed);
    let denom = (p.base.iter().sum::<f64>() / p.base.len() as f64)
        .abs()
        .max(1e-12);
    Sensitivity {
        lo,
        hi,
        half_width_pct: 0.5 * (hi - lo) / denom * 100.0,
    }
}

// ================================================================ STEP 4 + 8
// Per-instance record, QUALIFIES, verdicts and kill criteria.

#[derive(Clone)]
struct Row {
    index: usize,
    name: String,
    group: Option<char>,
    family: Family,
    i: f64,
    rho: Option<f64>,
    rel: f64,
    p: f64,
    zeros: usize,
    degenerate: bool,
    tie_blocked: bool,
    material: bool,
    ci_lo: f64,
    ci_hi: f64,
    ci_hw_pct: f64,
}

fn measure_block(reg: &OperatorRegistry, block: Block) -> Vec<Row> {
    let mut rows = Vec::new();
    for (index, inst) in CORPUS.iter().enumerate() {
        let (name, path, group, family) = (inst.name, inst.path, inst.group, inst.family);
        // FAIL-CLOSED. A skipped instance would shrink `m` in the BH step-up,
        // and the threshold k/m·q RISES as m falls — a partial corpus silently
        // LOOSENS the multiplicity control. Abort instead.
        let ir = match load(path) {
            Ok(ir) => ir,
            Err(e) => {
                eprintln!("  {name}: FATAL load failure ({e}) — aborting the block");
                std::process::exit(4);
            }
        };
        let Some(p) = contrast(&ir, reg, block.seeds(), SWEEPS) else {
            eprintln!("  {name}: FATAL run failure — aborting the block");
            std::process::exit(4);
        };
        let s = sensitivity_ci(&p, index, block);
        let row = Row {
            index,
            name: (*name).to_string(),
            group,
            family,
            i: p.mean_effect(),
            rho: p.rho(),
            rel: p.rel(),
            p: p.signflip_p(),
            zeros: p.zeros(),
            degenerate: p.degenerate(),
            tie_blocked: p.tie_blocked(),
            material: p.material(),
            ci_lo: s.lo,
            ci_hi: s.hi,
            ci_hw_pct: s.half_width_pct,
        };
        println!(
            "  [{}] {:>5} I={:+10.4} rho={:<7} rel={:7.4}% p={:.4} z={} {}{}CI=[{:+.3},{:+.3}] hw={:.4}%",
            block.tag(),
            row.name,
            row.i,
            row.rho.map_or("DEGEN".to_string(), |r| format!("{r:.3}")),
            row.rel * 100.0,
            row.p,
            row.zeros,
            if row.tie_blocked { "TIE-BLOCKED " } else { "" },
            if row.degenerate { "DEGENERATE_NULL " } else { "" },
            row.ci_lo,
            row.ci_hi,
            row.ci_hw_pct
        );
        rows.push(row);
    }
    assert_eq!(
        rows.len(),
        CORPUS.len(),
        "a block must cover the whole enumerated corpus"
    );
    rows
}

/// RC-014 rule A6: matching sign, held-out independently material, ratio in
/// [0.5, 2.0]. A zero held-in effect or a degenerate arm fails replication.
fn a6_replicates(hi: &Row, ho: &Row) -> bool {
    if hi.i == 0.0 || hi.degenerate || ho.degenerate {
        return false;
    }
    hi.i.signum() == ho.i.signum() && ho.material && (0.5..=2.0).contains(&(ho.i / hi.i).abs())
}

/// STEP 4 — Amendment 1 §A2: BH-reject ∧ material ∧ A6 replication.
/// A TIE-BLOCKED instance can never satisfy this (Amendment 2 §B4).
fn qualifies(hi: &Row, ho: &Row, bh_reject: bool) -> bool {
    if hi.tie_blocked {
        return false;
    }
    bh_reject && hi.material && a6_replicates(hi, ho)
}

/// STEP 8 — the verdict space. Kept as data so routing is unit-testable; the
/// printing is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Kill criterion 2 — overrides the sign question entirely.
    BenchmarkValidity,
    SignVaries,
    SignConstant,
    /// Amendment 2 §B1 — DESCRIPTIVE only, never an equivalence claim.
    NoMaterialEffectObserved,
    QInconclusive,
}

struct Census {
    k: usize,
    k_plus: usize,
    k_minus: usize,
    material_unqualified: usize,
    degenerate_cells: usize,
    degenerate_fraction: f64,
    groups: usize,
    families: usize,
}

/// Pure routing: no I/O, so every branch is reachable from a unit test.
fn decide(held_in: &[Row], held_out: &[Row]) -> (Verdict, Census) {
    // No verdict may be computed from a partial corpus: BH's threshold k/m·q
    // rises as m falls, so a short block would loosen the multiplicity control.
    assert_eq!(held_in.len(), CORPUS.len(), "held-in must cover the corpus");
    assert_eq!(
        held_out.len(),
        CORPUS.len(),
        "held-out must cover the corpus"
    );

    let ps: Vec<f64> = held_in.iter().map(|r| r.p).collect();
    let rej = benjamini_hochberg(&ps, FDR_Q);

    let mut qual: Vec<&Row> = Vec::new();
    let mut material_unqualified = 0usize;
    for (k, hi) in held_in.iter().enumerate() {
        let Some(ho) = held_out.iter().find(|r| r.index == hi.index) else {
            continue;
        };
        if qualifies(hi, ho, rej[k]) {
            qual.push(hi);
        } else if hi.material || hi.tie_blocked {
            material_unqualified += 1;
        }
    }

    // Amendment 1 §A4 criterion 2 is stated over INSTANCES, not block-cells.
    // An instance counts as degenerate iff EITHER block is degenerate.
    let degenerate_cells = held_in
        .iter()
        .filter(|hi| {
            hi.degenerate
                || held_out
                    .iter()
                    .find(|ho| ho.index == hi.index)
                    .is_some_and(|ho| ho.degenerate)
        })
        .count();
    let degenerate_fraction = degenerate_cells as f64 / CORPUS.len() as f64;

    let k_plus = qual.iter().filter(|r| r.i > 0.0).count();
    let k_minus = qual.iter().filter(|r| r.i < 0.0).count();

    let mut groups: Vec<char> = qual.iter().filter_map(|r| r.group).collect();
    groups.sort_unstable();
    groups.dedup();
    let families: std::collections::BTreeSet<&str> = qual
        .iter()
        .map(|r| match r.family {
            Family::RandomPlus => "random+1",
            Family::Toroidal => "toroidal",
        })
        .collect();

    let census = Census {
        k: qual.len(),
        k_plus,
        k_minus,
        material_unqualified,
        degenerate_cells,
        degenerate_fraction,
        groups: groups.len(),
        families: families.len(),
    };

    // Kill criterion 2 is checked FIRST: saturation overrides the sign question.
    if degenerate_fraction >= DEGENERATE_FRACTION_KILL {
        return (Verdict::BenchmarkValidity, census);
    }
    // Amendment 4 §D4: SIGN VARIES is an existence claim and needs one qualified
    // witness of each sign. The K>=6 and coverage floors apply only to the
    // population-style SIGN CONSTANT claim.
    if k_plus >= 1 && k_minus >= 1 {
        return (Verdict::SignVaries, census);
    }
    if census.k >= MIN_QUALIFYING
        && (k_plus == 0 || k_minus == 0)
        && census.groups >= MIN_GROUPS
        && census.families >= 2
    {
        return (Verdict::SignConstant, census);
    }
    // Amendment 2 §B1/§B2: K = 0 is NOT sufficient. NO MATERIAL EFFECT OBSERVED
    // requires the POSITIVE condition over ALL instances in BOTH blocks.
    let all_clean = held_in.len() == CORPUS.len()
        && held_out.len() == CORPUS.len()
        && held_in
            .iter()
            .chain(held_out.iter())
            .all(|r| !r.degenerate && !r.material && !r.tie_blocked);
    if census.k == 0 && all_clean {
        return (Verdict::NoMaterialEffectObserved, census);
    }
    (Verdict::QInconclusive, census)
}

/// Presentation only. All routing lives in `decide`.
fn report_verdict(held_in: &[Row], held_out: &[Row]) {
    let (v, c) = decide(held_in, held_out);

    println!(
        "\n  --- corpus census (descriptive; NO cross-instance p-value is computed or licensed) ---"
    );
    println!(
        "  K = {} qualifying of {} enumerated   k+ = {}   k- = {}   K/30 = {:.3}",
        c.k,
        CORPUS.len(),
        c.k_plus,
        c.k_minus,
        c.k as f64 / CORPUS.len() as f64
    );
    println!(
        "  material-but-unqualified or tie-blocked: {}   degenerate cells: {} ({:.1}%)",
        c.material_unqualified,
        c.degenerate_cells,
        c.degenerate_fraction * 100.0
    );
    println!(
        "  qualifying coverage: {} matched groups, {} structural families",
        c.groups, c.families
    );

    match v {
        Verdict::BenchmarkValidity => {
            println!(
                "\n  VERDICT: BENCHMARK-VALIDITY FINDING — >= 50% of cells are DEGENERATE_NULL."
            );
            println!("  The configuration is saturated at this budget and cannot discriminate.");
            println!("  The sign question is NOT answered.");
        }
        Verdict::SignVaries => {
            println!("\n  VERDICT: SIGN VARIES");
            let ps: Vec<f64> = held_in.iter().map(|r| r.p).collect();
            let rej = benjamini_hochberg(&ps, FDR_Q);
            let qual: Vec<&Row> = held_in
                .iter()
                .enumerate()
                .filter(|(k, hi)| {
                    held_out
                        .iter()
                        .find(|r| r.index == hi.index)
                        .is_some_and(|ho| qualifies(hi, ho, rej[*k]))
                })
                .map(|(_, hi)| hi)
                .collect();
            for g in ['A', 'B', 'C', 'D', 'E', 'F'] {
                let m: Vec<&&Row> = qual.iter().filter(|r| r.group == Some(g)).collect();
                if m.iter().any(|r| r.i > 0.0) && m.iter().any(|r| r.i < 0.0) {
                    println!(
                        "    matched group {g}: opposite-signed qualifying members — S0 COUNTEREXAMPLE"
                    );
                    for x in &m {
                        println!("      {} I={:+.4}", x.name, x.i);
                    }
                }
            }
        }
        Verdict::SignConstant => {
            println!(
                "\n  VERDICT: SIGN CONSTANT — {} qualifying, {} matched groups, {} families",
                c.k, c.groups, c.families
            );
        }
        Verdict::NoMaterialEffectObserved => {
            println!("\n  VERDICT: NO MATERIAL EFFECT OBSERVED");
            println!(
                "  A DESCRIPTIVE statement about this design's sensitivity, and nothing more."
            );
            println!(
                "  FORBIDDEN under this verdict: any claim of equivalence, interchangeability"
            );
            println!("  or indistinguishability; any claim the effect is zero or negligible; any");
            println!("  claim bounding what a selector could gain.");
            let mut hw: Vec<f64> = held_in.iter().map(|r| r.ci_hw_pct).collect();
            hw.sort_by(f64::total_cmp);
            println!(
                "  Mandatory sensitivity: held-in CI half-width {:.4}%..{:.4}% of |mean Y(metropolis)|",
                hw.first().copied().unwrap_or(f64::NAN),
                hw.last().copied().unwrap_or(f64::NAN)
            );
        }
        Verdict::QInconclusive => {
            println!(
                "\n  VERDICT: Q-INCONCLUSIVE — K = {}; {} material-but-unqualified or tie-blocked; {} degenerate cells.",
                c.k, c.material_unqualified, c.degenerate_cells
            );
            println!("  No post-hoc relaxation of any bar is permitted.");
        }
    }
}

/// §8.1 requires the calibration table frozen "before any science seed runs".
/// Complete means: parses, and carries exactly one row per corpus index 0..29
/// with no non-finite field.
fn require_frozen_calibration(dir: &str) -> bool {
    let path = format!("{dir}/rc016_calibration.tsv");
    let Ok(text) = std::fs::read_to_string(&path) else {
        println!("    calibration artifact missing: {path}");
        return false;
    };
    let mut seen = vec![false; CORPUS.len()];
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 9 {
            println!("    malformed calibration row: {line}");
            return false;
        }
        let Ok(idx) = f[0].parse::<usize>() else {
            println!("    unparseable index: {}", f[0]);
            return false;
        };
        if idx >= CORPUS.len() || f[1] != CORPUS[idx].name {
            println!("    calibration row {idx} does not match the corpus order");
            return false;
        }
        for col in &f[2..7] {
            match col.parse::<f64>() {
                Ok(v) if v.is_finite() => {}
                _ => {
                    println!("    non-finite calibration value on row {idx}: {col}");
                    return false;
                }
            }
        }
        let degenerate_power =
            f[7] == "true" && f[4].parse::<f64>().is_ok_and(|d_seed| d_seed < 1e-9);
        if degenerate_power {
            println!("    DEGENERATE-POWER: {} (raw powered=true)", f[1]);
        }
        seen[idx] = true;
    }
    let missing: Vec<&str> = seen
        .iter()
        .enumerate()
        .filter(|(_, ok)| !**ok)
        .map(|(i, _)| CORPUS[i].name)
        .collect();
    if !missing.is_empty() {
        println!("    calibration incomplete, missing rows for: {missing:?}");
        return false;
    }
    println!("    frozen calibration complete: {} rows", CORPUS.len());
    true
}

// ------------------------------------------------------------- held-out gate

/// Commit time (unix seconds) of the newest commit touching `path`, or `None`
/// if the path is untracked / never committed.
fn git_commit_time(path: &str) -> Option<u64> {
    let out = std::process::Command::new("git")
        .args(["log", "-1", "--format=%ct", "--", path])
        .output()
        .ok()?;
    let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if t.is_empty() {
        None
    } else {
        t.parse().ok()
    }
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

/// Pure Amendment 4 §D6 Gate 1 predicate. Keeping policy separate from git and
/// filesystem I/O makes every refusal branch deterministic in unit tests.
fn amendment4_gate_ok(
    exists: bool,
    tracked: bool,
    clean: bool,
    commit_time: Option<u64>,
    calibration_mtime: Option<u64>,
) -> Result<(), &'static str> {
    if !exists {
        return Err("Amendment 4 file is missing");
    }
    if !tracked {
        return Err("Amendment 4 is not tracked by git");
    }
    if !clean {
        return Err("Amendment 4 has uncommitted modifications");
    }
    match (commit_time, calibration_mtime) {
        (Some(commit), Some(calibration)) if commit > calibration => Ok(()),
        (Some(_), Some(_)) => Err("Amendment 4 was not committed after calibration"),
        (None, _) => Err("Amendment 4 has no committing commit"),
        (_, None) => Err("calibration artifact mtime is unavailable"),
    }
}

fn git_succeeds(args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Hard precondition for held-in and held-out. Calibration and controls remain
/// reachable because they do not inspect either science seed block.
fn require_amendment4(dir: &str) -> bool {
    let calibration = format!("{dir}/rc016_calibration.tsv");
    let exists = std::path::Path::new(AMENDMENT4_PATH).exists();
    let tracked = git_succeeds(&["ls-files", "--error-unmatch", AMENDMENT4_PATH]);
    let clean = git_succeeds(&["diff", "--quiet", "HEAD", "--", AMENDMENT4_PATH]);
    match amendment4_gate_ok(
        exists,
        tracked,
        clean,
        git_commit_time(AMENDMENT4_PATH),
        mtime_secs(&calibration),
    ) {
        Ok(()) => {
            println!("    Amendment 4 provenance gate: PASS");
            true
        }
        Err(reason) => {
            eprintln!("    Amendment 4 provenance gate: FAIL — {reason}");
            false
        }
    }
}

/// Four independent conditions, all required. Held-out is the block that can
/// still falsify a held-in result, so it must not be reachable by accident.
fn descendant_gate(dir: &str) -> bool {
    let heldin = format!("{dir}/rc016_heldin.tsv");
    let mut ok = true;

    let has_confirm = flag(CONFIRM_FLAG);
    println!("    [1] explicit confirmation flag {CONFIRM_FLAG}: {has_confirm}");
    ok &= has_confirm;

    let heldin_done = std::path::Path::new(&heldin).exists();
    println!("    [2] held-in results present ({heldin}): {heldin_done}");
    ok &= heldin_done;

    let desc_exists = std::path::Path::new(DESCENDANT_PATH).exists();
    let desc_commit = git_commit_time(DESCENDANT_PATH);
    println!(
        "    [3] descendant {DESCENDANT_PATH} exists={desc_exists} committed={}",
        desc_commit.is_some()
    );
    ok &= desc_exists && desc_commit.is_some();

    match (desc_commit, mtime_secs(&heldin)) {
        (Some(dc), Some(hm)) => {
            let after = dc > hm;
            println!(
                "    [4] descendant committed AFTER held-in: {after} (commit {dc} vs held-in {hm})"
            );
            ok &= after;
        }
        _ => {
            println!("    [4] descendant committed AFTER held-in: cannot verify");
            ok = false;
        }
    }
    ok
}

// ------------------------------------------------------------------ persistence

fn write_rows(dir: &str, tag: &str, rows: &[Row]) {
    let mut s = String::from("index\tinstance\tgroup\tfamily\tI\trho\trel\tp\tzeros\tdegenerate\ttie_blocked\tmaterial\tci_lo\tci_hi\tci_hw_pct\n");
    for r in rows {
        s.push_str(&format!(
            "{}\t{}\t{}\t{:?}\t{:.6}\t{}\t{:.8}\t{:.8}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\n",
            r.index,
            r.name,
            r.group.map_or("-".to_string(), |g| g.to_string()),
            r.family,
            r.i,
            r.rho.map_or("NA".to_string(), |v| format!("{v:.6}")),
            r.rel,
            r.p,
            r.zeros,
            r.degenerate,
            r.tie_blocked,
            r.material,
            r.ci_lo,
            r.ci_hi,
            r.ci_hw_pct
        ));
    }
    let _ = std::fs::create_dir_all(dir);
    let path = format!("{dir}/rc016_{tag}.tsv");
    match std::fs::write(&path, s) {
        Ok(()) => println!("\n  written: {path}"),
        Err(e) => {
            eprintln!("\n  FATAL write failure for {path}: {e}");
            std::process::exit(5);
        }
    }
}

fn read_rows(dir: &str, tag: &str) -> Option<Vec<Row>> {
    let text = std::fs::read_to_string(format!("{dir}/rc016_{tag}.tsv")).ok()?;
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 15 {
            continue;
        }
        out.push(Row {
            index: f[0].parse().ok()?,
            name: f[1].to_string(),
            group: f[2].chars().next().filter(|c| *c != '-'),
            family: if f[3] == "Toroidal" {
                Family::Toroidal
            } else {
                Family::RandomPlus
            },
            i: f[4].parse().ok()?,
            rho: f[5].parse().ok(),
            rel: f[6].parse().ok()?,
            p: f[7].parse().ok()?,
            zeros: f[8].parse().ok()?,
            degenerate: f[9] == "true",
            tie_blocked: f[10] == "true",
            material: f[11] == "true",
            ci_lo: f[12].parse().ok()?,
            ci_hi: f[13].parse().ok()?,
            ci_hw_pct: f[14].parse().ok()?,
        });
    }
    Some(out)
}

// ---------------------------------------------------------------------- main

fn main() {
    let reg = OperatorRegistry::standard();
    let dir: String = arg("--dir", "experiments/rc016".to_string());

    let any_science =
        flag("--calibrate") || flag("--science") || flag("--holdout") || flag("--verdict");

    // STEP 9 — controls gate EVERYTHING, unconditionally. The earlier guard
    // `flag("--controls") || !any_science` skipped them whenever a science mode
    // was invoked without `--controls`, which is the exact inverse of the
    // pre-registration's requirement (§9).
    if !run_controls(&reg) {
        std::process::exit(1);
    }
    if !any_science {
        return;
    }

    if flag("--calibrate") {
        println!("\nRC-016 calibration — pilot seeds {PILOT:?}, interleaved, medians");
        println!("cost_model is FORBIDDEN as a cost measure (RC-005); measured wall time only.\n");
        println!("Power is MODEL-CONDITIONAL: additive location shift, pilot representativeness,");
        println!("and a continuum bootstrap against integer-lattice data — optimistic w.r.t.");
        println!("tie-blocking specifically; other mismatch can move power either way.\n");
        let mut out = String::from(
            "index\tinstance\tR\tY_ref\td_seed_pilot\tdelta\tpower\tpowered\tcost_arm_needed\n",
        );
        for (idx, entry) in CORPUS.iter().enumerate() {
            match calibrate_one(idx, &reg) {
                Some(c) => {
                    let power_label = if c.powered && c.d_seed_pilot < 1e-9 {
                        "DEGENERATE-POWER (raw powered=true)"
                    } else if c.powered {
                        "POWERED"
                    } else {
                        "UNDERPOWERED (cost arm NOT run)"
                    };
                    println!(
                        "  {:>5} R={:.4} Y_ref={:.2} d_seed={:.4} delta={:.4} power={:.4} {} {}",
                        c.name,
                        c.r,
                        c.y_ref,
                        c.d_seed_pilot,
                        c.delta,
                        c.power,
                        power_label,
                        if c.cost_arm_needed {
                            "cost-arm-needed"
                        } else {
                            "legacy R band; equal-cost inference withdrawn (Amendment 4)"
                        }
                    );
                    out.push_str(&format!(
                        "{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\n",
                        c.index,
                        c.name,
                        c.r,
                        c.y_ref,
                        c.d_seed_pilot,
                        c.delta,
                        c.power,
                        c.powered,
                        c.cost_arm_needed
                    ));
                }
                None => println!("  {:>5} calibration failed", entry.name),
            }
        }
        let _ = std::fs::create_dir_all(&dir);
        let path = format!("{dir}/rc016_calibration.tsv");
        match std::fs::write(&path, out) {
            Ok(()) => println!("\n  frozen: {path}"),
            Err(e) => {
                eprintln!("\n  FATAL write failure for {path}: {e}");
                std::process::exit(5);
            }
        }
    }

    if flag("--science") {
        println!("\nRC-016 held-in block — frozen-calibration precondition (§8.1)");
        if !require_amendment4(&dir) {
            eprintln!("\n  HELD-IN REFUSED — Amendment 4 §D6 Gate 1 failed");
            std::process::exit(2);
        }
        if !require_frozen_calibration(&dir) {
            eprintln!("\n  HELD-IN REFUSED — §8.1 requires the calibration table frozen");
            eprintln!("  before ANY science seed runs. Run --calibrate first.");
            std::process::exit(2);
        }
        println!("\nRC-016 held-in block, seeds {HELD_IN:?}\n");
        let rows = measure_block(&reg, Block::HeldIn);
        write_rows(&dir, "heldin", &rows);
    }

    if flag("--holdout") {
        println!("\nRC-016 held-out gate (PREREG §5: not inspected until controls and held-in are evaluated)");
        if !require_amendment4(&dir) {
            eprintln!("\n  HELD-OUT REFUSED — Amendment 4 §D6 Gate 1 failed");
            std::process::exit(2);
        }
        if !require_frozen_calibration(&dir) {
            eprintln!("\n  HELD-OUT REFUSED — no complete frozen calibration (§8.1).");
            std::process::exit(2);
        }
        if !descendant_gate(&dir) {
            println!("\n  HELD-OUT REFUSED — the falsifiable descendant must be written and");
            println!("  committed AFTER the held-in block, and the run explicitly confirmed.");
            println!("  Refusing rather than silently inspecting the held-out seeds.");
            std::process::exit(3);
        }
        println!("\nRC-016 held-out block, seeds {HELD_OUT:?}\n");
        let rows = measure_block(&reg, Block::HeldOut);
        write_rows(&dir, "heldout", &rows);
    }

    if flag("--verdict") {
        match (read_rows(&dir, "heldin"), read_rows(&dir, "heldout")) {
            (Some(hi), Some(ho)) => report_verdict(&hi, &ho),
            _ => println!("\n  verdict needs both rc016_heldin.tsv and rc016_heldout.tsv in {dir}"),
        }
    }
}

// ---------------------------------------------------------------------- tests
//
// These validate the analysis layer WITHOUT the Runtime, so an attribution bug
// and an absent effect cannot look alike, and every pre-registered predicate has
// a reachable failing case.

#[cfg(test)]
mod tests {
    use super::*;

    fn row(index: usize, i: f64, p: f64, material: bool, group: Option<char>, fam: Family) -> Row {
        Row {
            index,
            name: format!("I{index}"),
            group,
            family: fam,
            i,
            rho: Some(3.0),
            rel: 0.01,
            p,
            zeros: 0,
            degenerate: false,
            tie_blocked: false,
            material,
            ci_lo: i - 1.0,
            ci_hi: i + 1.0,
            ci_hw_pct: 0.05,
        }
    }

    /// The synthetic arithmetic positive: analytically known effects, so the
    /// attribution layer is checked with no solver in the loop.
    #[test]
    fn arithmetic_positive_recovers_known_effects() {
        let p = Paired {
            base: vec![-10.0, -20.0, -30.0, -40.0],
            other: vec![-9.0, -18.0, -27.0, -36.0],
        };
        assert_eq!(p.effects(), vec![1.0, 2.0, 3.0, 4.0]);
        assert!((p.mean_effect() - 2.5).abs() < 1e-12);
        assert!((p.d_seed() - (500.0f64 / 3.0).sqrt()).abs() < 1e-12);
        assert!(!p.degenerate());

        // A constant reference arm must FLAG, never divide by zero.
        let flat = Paired {
            base: vec![-10.0; 4],
            other: vec![-9.0, -18.0, -27.0, -36.0],
        };
        assert!(flat.degenerate());
        assert!(flat.rho().is_none());
    }

    /// Amendment 2 §B4: `p_floor(z) = 2^(z+1)/2^k`, and `z >= 3` at `k = 8`
    /// cannot reject at alpha whatever the other differences are.
    #[test]
    fn zero_ties_raise_the_p_floor_and_block_at_three() {
        for z in 0..=4usize {
            let mut other = vec![0.0; 8];
            for (j, o) in other.iter_mut().enumerate() {
                *o = if j < z { 0.0 } else { 10.0 };
            }
            let p = Paired {
                base: vec![0.0; 8],
                other,
            };
            assert_eq!(p.zeros(), z, "zero count");
            let floor = 2f64.powi(z as i32 + 1) / 256.0;
            assert!(
                (p.signflip_p() - floor).abs() < 1e-12,
                "z={z}: p={} expected floor {floor}",
                p.signflip_p()
            );
            assert_eq!(p.tie_blocked(), z >= TIE_BLOCK_Z);
            // The operational consequence: at z >= 3 rejection is impossible.
            if z >= TIE_BLOCK_Z {
                assert!(p.signflip_p() >= ALPHA);
            }
        }
    }

    /// A TIE-BLOCKED instance can never QUALIFY, even with a BH rejection and a
    /// perfect replication — the whole point of routing it to Q-INCONCLUSIVE.
    #[test]
    fn tie_blocked_never_qualifies() {
        let mut hi = row(0, 10.0, 0.0078, true, Some('A'), Family::Toroidal);
        let ho = row(0, 10.0, 0.0078, true, Some('A'), Family::Toroidal);
        assert!(qualifies(&hi, &ho, true));
        hi.tie_blocked = true;
        assert!(!qualifies(&hi, &ho, true));
    }

    #[test]
    fn benjamini_hochberg_step_up_is_monotone_and_correct() {
        // All tiny p-values reject; all large ones do not.
        assert_eq!(
            benjamini_hochberg(&[0.001, 0.002, 0.003], 0.10),
            vec![true; 3]
        );
        assert_eq!(benjamini_hochberg(&[0.9, 0.8, 0.7], 0.10), vec![false; 3]);
        // Step-up: the largest surviving rank pulls in every smaller p-value.
        let r = benjamini_hochberg(&[0.001, 0.06, 0.9], 0.10);
        assert_eq!(r, vec![true, true, false]);
        // Order of the input must not matter to WHICH entries reject.
        let r2 = benjamini_hochberg(&[0.9, 0.06, 0.001], 0.10);
        assert_eq!(r2, vec![false, true, true]);
    }

    /// QUALIFIES requires BH rejection — the hole Amendment 1 §A2 closed.
    #[test]
    fn qualification_requires_bh_rejection() {
        let hi = row(0, 10.0, 0.30, true, Some('A'), Family::Toroidal);
        let ho = row(0, 10.0, 0.30, true, Some('A'), Family::Toroidal);
        assert!(a6_replicates(&hi, &ho) && hi.material);
        assert!(
            !qualifies(&hi, &ho, false),
            "material + A6 must NOT suffice"
        );
        assert!(qualifies(&hi, &ho, true));
    }

    /// A6: sign match, held-out independently material, ratio in [0.5, 2.0].
    #[test]
    fn a6_replication_rule() {
        let hi = row(0, 10.0, 0.008, true, None, Family::RandomPlus);

        let same = row(0, 12.0, 0.008, true, None, Family::RandomPlus);
        assert!(a6_replicates(&hi, &same));

        let flipped = row(0, -12.0, 0.008, true, None, Family::RandomPlus);
        assert!(!a6_replicates(&hi, &flipped), "sign must match");

        let too_big = row(0, 30.0, 0.008, true, None, Family::RandomPlus);
        assert!(
            !a6_replicates(&hi, &too_big),
            "ratio 3.0 is outside [0.5,2]"
        );

        let too_small = row(0, 2.0, 0.008, true, None, Family::RandomPlus);
        assert!(
            !a6_replicates(&hi, &too_small),
            "ratio 0.2 is outside [0.5,2]"
        );

        let immaterial = row(0, 12.0, 0.008, false, None, Family::RandomPlus);
        assert!(
            !a6_replicates(&hi, &immaterial),
            "held-out must be independently material"
        );

        let mut degen = row(0, 12.0, 0.008, true, None, Family::RandomPlus);
        degen.degenerate = true;
        assert!(!a6_replicates(&hi, &degen));
    }

    fn clean_corpus() -> (Vec<Row>, Vec<Row>) {
        let mk = |_b: usize| {
            CORPUS
                .iter()
                .enumerate()
                .map(|(k, inst)| {
                    let mut r = row(k, 0.01, 0.9, false, inst.group, inst.family);
                    r.rho = Some(0.01);
                    r.rel = 0.0;
                    r
                })
                .collect::<Vec<Row>>()
        };
        (mk(0), mk(1))
    }

    /// Amendment 2 §B1/§B2 — the correction that mattered most: `K = 0` alone
    /// must NOT reach NO MATERIAL EFFECT OBSERVED.
    #[test]
    fn verdict_routing_k_zero_is_not_interchangeability() {
        let (hi, ho) = clean_corpus();
        assert_eq!(decide(&hi, &ho).0, Verdict::NoMaterialEffectObserved);

        // World 2: one material-but-unqualified instance ⇒ Q-INCONCLUSIVE.
        let (mut hi2, ho2) = clean_corpus();
        hi2[0].material = true;
        hi2[0].p = 0.9; // material, but nowhere near BH rejection
        assert_eq!(decide(&hi2, &ho2).0, Verdict::QInconclusive);

        // World 3: a single isolated DEGENERATE_NULL, far below the 50% kill.
        let (mut hi3, ho3) = clean_corpus();
        hi3[0].degenerate = true;
        assert_eq!(decide(&hi3, &ho3).0, Verdict::QInconclusive);

        // A single TIE-BLOCKED instance likewise blocks the descriptive verdict.
        let (mut hi4, ho4) = clean_corpus();
        hi4[0].tie_blocked = true;
        assert_eq!(decide(&hi4, &ho4).0, Verdict::QInconclusive);
    }

    #[test]
    fn verdict_routing_sign_varies_and_constant_and_saturation() {
        // SIGN VARIES: qualifying instances of BOTH signs inside one matched
        // group. FOUR are used, not two, because of the BH/p-floor interaction
        // proved in `bh_floor_needs_three_instances_to_reject_anything` below.
        let (mut hi, mut ho) = clean_corpus();
        for (k, sign) in [(17usize, 1.0f64), (18, -1.0), (19, 1.0), (0, -1.0)] {
            let (g, f) = (CORPUS[k].group, CORPUS[k].family);
            hi[k] = row(k, 10.0 * sign, 0.0078, true, g, f);
            ho[k] = row(k, 11.0 * sign, 0.0078, true, g, f);
        }
        let (v, c) = decide(&hi, &ho);
        assert!(c.k_plus >= 1 && c.k_minus >= 1, "both signs must qualify");
        // Amendment 4 §D4: opposite qualified signs are an existence proof and
        // carry no K>=6 or structural-coverage floor.
        assert_eq!(c.k, 4);
        assert_eq!(v, Verdict::SignVaries);

        // The minimum constructive case is one qualified witness of each sign.
        let (mut hi_min, mut ho_min) = clean_corpus();
        for (k, sign) in [(17usize, 1.0f64), (18, -1.0)] {
            let (g, f) = (CORPUS[k].group, CORPUS[k].family);
            hi_min[k] = row(k, 10.0 * sign, 0.001, true, g, f);
            ho_min[k] = row(k, 11.0 * sign, 0.001, true, g, f);
        }
        let (v_min, c_min) = decide(&hi_min, &ho_min);
        assert_eq!((c_min.k_plus, c_min.k_minus), (1, 1));
        assert_eq!(v_min, Verdict::SignVaries);

        // A one-sign pattern below K=6 remains inconclusive.
        let (mut hi_short, mut ho_short) = clean_corpus();
        for k in [17usize, 18] {
            let (g, f) = (CORPUS[k].group, CORPUS[k].family);
            hi_short[k] = row(k, 10.0, 0.001, true, g, f);
            ho_short[k] = row(k, 11.0, 0.001, true, g, f);
        }
        assert_eq!(decide(&hi_short, &ho_short).0, Verdict::QInconclusive);

        // The larger opposite-sign case remains SIGN VARIES as well.
        let (mut hi6, mut ho6) = clean_corpus();
        for (k, sign) in [
            (17usize, 1.0f64),
            (18, -1.0),
            (19, 1.0),
            (0, -1.0),
            (1, 1.0),
            (2, -1.0),
        ] {
            let (g, f) = (CORPUS[k].group, CORPUS[k].family);
            hi6[k] = row(k, 10.0 * sign, 0.0078, true, g, f);
            ho6[k] = row(k, 11.0 * sign, 0.0078, true, g, f);
        }
        let (v6, c6) = decide(&hi6, &ho6);
        assert_eq!(c6.k, 6);
        assert_eq!(v6, Verdict::SignVaries);

        // SIGN CONSTANT needs >= 6 qualifying across >= 3 groups AND both families.
        let (mut hi2, mut ho2) = clean_corpus();
        for k in [0usize, 1, 3, 4, 9, 10] {
            let (g, f) = (CORPUS[k].group, CORPUS[k].family);
            hi2[k] = row(k, 10.0, 0.0078, true, g, f);
            ho2[k] = row(k, 11.0, 0.0078, true, g, f);
        }
        let (v2, c2) = decide(&hi2, &ho2);
        assert_eq!(c2.k, 6);
        assert!(c2.groups >= MIN_GROUPS && c2.families >= 2);
        assert_eq!(v2, Verdict::SignConstant);

        // Same-sign but only ONE structural family ⇒ coverage fails.
        let (mut hi3, mut ho3) = clean_corpus();
        for k in [0usize, 1, 2, 9, 10, 11] {
            let (g, f) = (CORPUS[k].group, CORPUS[k].family);
            hi3[k] = row(k, 10.0, 0.0078, true, g, f);
            ho3[k] = row(k, 11.0, 0.0078, true, g, f);
        }
        let (v3, c3) = decide(&hi3, &ho3);
        assert_eq!(c3.families, 1);
        assert_eq!(v3, Verdict::QInconclusive);

        // Kill criterion 2 is checked FIRST and overrides everything.
        let (mut hi4, ho4) = clean_corpus();
        for r in hi4.iter_mut() {
            r.degenerate = true;
        }
        assert_eq!(decide(&hi4, &ho4).0, Verdict::BenchmarkValidity);
    }

    /// An operational consequence of the pre-registration, proved here so it is
    /// known BEFORE data rather than discovered during analysis: with 30
    /// instances at FDR 0.10, the exact sign-flip p-FLOOR of `2/256 = 0.0078125`
    /// only clears the BH threshold `k/30 · 0.10` from rank 3 onward. So **at
    /// least three instances must attain the floor before BH rejects any of
    /// them** — a lone maximally-significant instance cannot qualify.
    #[test]
    fn bh_floor_needs_three_instances_to_reject_anything() {
        let floor = 2.0 / 256.0;
        let mk = |n_at_floor: usize| {
            let mut ps = vec![0.9; CORPUS.len()];
            for slot in ps.iter_mut().take(n_at_floor) {
                *slot = floor;
            }
            benjamini_hochberg(&ps, FDR_Q)
        };
        assert_eq!(mk(1).iter().filter(|b| **b).count(), 0, "one is not enough");
        assert_eq!(mk(2).iter().filter(|b| **b).count(), 0, "two is not enough");
        assert_eq!(mk(3).iter().filter(|b| **b).count(), 3, "three rejects");
        // The arithmetic behind it.
        assert!(floor > 2.0 / 30.0 * FDR_Q);
        assert!(floor <= 3.0 / 30.0 * FDR_Q);
    }

    /// Fix 6: the benchmark-validity denominator is INSTANCES, not block-cells.
    /// An instance counts as degenerate iff EITHER block is degenerate.
    #[test]
    fn benchmark_validity_counts_instances_not_cells() {
        // 20 of 30 instances degenerate in the held-in block ONLY.
        // Cells: 20/60 = 33% (would NOT fire). Instances: 20/30 = 67% (fires).
        let (mut hi, ho) = clean_corpus();
        for r in hi.iter_mut().take(20) {
            r.degenerate = true;
        }
        let (v, c) = decide(&hi, &ho);
        assert_eq!(c.degenerate_cells, 20, "counted per instance");
        assert!((c.degenerate_fraction - 20.0 / 30.0).abs() < 1e-12);
        assert_eq!(v, Verdict::BenchmarkValidity);

        // 14 of 30 in both blocks: 28/60 = 47% of cells, 14/30 = 47% of
        // instances — below the bar either way, so it must NOT fire.
        let (mut hi2, mut ho2) = clean_corpus();
        for k in 0..14 {
            hi2[k].degenerate = true;
            ho2[k].degenerate = true;
        }
        let (v2, c2) = decide(&hi2, &ho2);
        assert_eq!(c2.degenerate_cells, 14);
        assert_ne!(v2, Verdict::BenchmarkValidity);
    }

    /// Fix 8's primitive, pinned against published vectors so the inline
    /// SHA-256 is verified rather than trusted.
    #[test]
    fn sha256_matches_known_answers() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // The corpus table stores the first 12 hex chars.
        assert_eq!(&sha256_hex(b"abc")[..12], "ba7816bf8f01");
    }

    /// Fix 8: every corpus entry carries a 12-hex-char pre-registered hash.
    #[test]
    fn corpus_carries_preregistered_hashes() {
        for inst in CORPUS {
            assert_eq!(inst.sha12.len(), 12, "{} hash length", inst.name);
            assert!(
                inst.sha12.chars().all(|c| c.is_ascii_hexdigit()),
                "{} hash is not hex",
                inst.name
            );
        }
        let mut hs: Vec<&str> = CORPUS.iter().map(|c| c.sha12).collect();
        let n = hs.len();
        hs.sort_unstable();
        hs.dedup();
        assert_eq!(hs.len(), n, "hashes must be distinct");
    }

    /// Fix 3: science refuses without a complete frozen calibration.
    #[test]
    fn science_requires_a_complete_frozen_calibration() {
        let dir = std::env::temp_dir().join("rc016_calib_missing");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::remove_file(dir.join("rc016_calibration.tsv"));
        assert!(!require_frozen_calibration(dir.to_str().unwrap()));

        // A partial table (one row) must also be refused.
        let dir2 = std::env::temp_dir().join("rc016_calib_partial");
        let _ = std::fs::create_dir_all(&dir2);
        std::fs::write(
            dir2.join("rc016_calibration.tsv"),
            "index\tinstance\tR\tY_ref\td_seed_pilot\tdelta\tpower\tpowered\tcost_arm_needed\n\
             0\tG1\t1.0\t100.0\t1.0\t0.5\t0.9\ttrue\tfalse\n",
        )
        .unwrap();
        assert!(!require_frozen_calibration(dir2.to_str().unwrap()));
    }

    #[test]
    fn amendment4_gate_reports_every_refusal_branch() {
        assert_eq!(
            amendment4_gate_ok(false, true, true, Some(20), Some(10)),
            Err("Amendment 4 file is missing")
        );
        assert_eq!(
            amendment4_gate_ok(true, false, true, Some(20), Some(10)),
            Err("Amendment 4 is not tracked by git")
        );
        assert_eq!(
            amendment4_gate_ok(true, true, false, Some(20), Some(10)),
            Err("Amendment 4 has uncommitted modifications")
        );
        assert_eq!(
            amendment4_gate_ok(true, true, true, Some(10), Some(10)),
            Err("Amendment 4 was not committed after calibration")
        );
        assert_eq!(
            amendment4_gate_ok(true, true, true, None, Some(10)),
            Err("Amendment 4 has no committing commit")
        );
        assert_eq!(
            amendment4_gate_ok(true, true, true, Some(20), None),
            Err("calibration artifact mtime is unavailable")
        );
        assert_eq!(
            amendment4_gate_ok(true, true, true, Some(20), Some(10)),
            Ok(())
        );
    }

    /// Amendment 4 §D2 withdraws the cost arm. This source-level invariant
    /// prevents a future CLI flag from silently making that arm reachable.
    #[test]
    fn rc016_exposes_no_cost_arm_mode() {
        let forbidden_flag = ["flag(\"--", "cost"].concat();
        let forbidden_equal_cost = ["--equal", "-cost"].concat();
        let forbidden_claim = ["equal sweeps IS", " equal cost"].concat();
        let source = include_str!("exp_sensor_sufficiency.rs");
        assert!(!source.contains(&forbidden_flag));
        assert!(!source.contains(&forbidden_equal_cost));
        assert!(!source.contains(&forbidden_claim));
    }

    /// Amendment 3 §C1: the seed formula must be collision-free across the
    /// corpus and must never meet the power seed.
    #[test]
    fn ci_seed_formula_is_injective_and_disjoint_from_power_seed() {
        let mut seeds = Vec::new();
        for i in 0..CORPUS.len() as u64 {
            for b in 0..2u64 {
                seeds.push(CI_BOOTSTRAP_BASE_SEED + 2 * i + b);
            }
        }
        assert_eq!(seeds.len(), 60);
        let mut u = seeds.clone();
        u.sort_unstable();
        u.dedup();
        assert_eq!(u.len(), 60, "seeds must be distinct");
        assert_eq!(*u.first().unwrap(), CI_BOOTSTRAP_BASE_SEED);
        assert!(!seeds.contains(&POWER_BOOTSTRAP_SEED));
        // Block index must actually change the stream.
        let xs = [1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0];
        assert_ne!(
            bootstrap_ci(&xs, 2000, CI_BOOTSTRAP_BASE_SEED),
            bootstrap_ci(&xs, 2000, CI_BOOTSTRAP_BASE_SEED + 1)
        );
    }

    /// Determinism: the same seed must reproduce the interval exactly, and the
    /// percentile indices must be the 2.5th/97.5th order statistics.
    #[test]
    fn bootstrap_is_deterministic_and_uses_the_declared_percentiles() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let a = bootstrap_ci(&xs, 4000, 12345);
        let b = bootstrap_ci(&xs, 4000, 12345);
        assert_eq!(a.0.to_bits(), b.0.to_bits());
        assert_eq!(a.1.to_bits(), b.1.to_bits());
        assert!(a.0 <= a.1);
        // reps/40 and reps-1-reps/40 are the 2.5% and 97.5% indices.
        let reps = 100_000usize;
        assert_eq!(reps / 40, 2_500);
        assert_eq!(reps - 1 - reps / 40, 97_499);
    }

    /// Power is model-conditional but must still be deterministic, monotone in
    /// the shift, and bounded.
    #[test]
    fn power_rate_is_deterministic_and_monotone_in_delta() {
        let resid = [-3.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0];
        let lo = power_rate(&resid, 0.1);
        let lo2 = power_rate(&resid, 0.1);
        let hi = power_rate(&resid, 50.0);
        assert_eq!(lo.to_bits(), lo2.to_bits(), "same seed ⇒ same rate");
        assert!((0.0..=1.0).contains(&lo) && (0.0..=1.0).contains(&hi));
        assert!(hi > lo, "a far larger shift must be easier to detect");
        assert!(hi > POWER_BAR);
    }

    /// Amendment 2 §B3: Delta is the BINDING component of a conjunctive bar,
    /// not the relative half alone.
    #[test]
    fn delta_is_the_binding_materiality_component() {
        // rel component binds
        let (y_ref, d_seed) = (1_000_000.0f64, 1.0f64);
        assert!((REL_BAR * y_ref).max(RHO_BAR * d_seed) - REL_BAR * y_ref < 1e-12);
        // rho component binds
        let (y_ref2, d_seed2) = (10.0f64, 1000.0f64);
        let delta2 = (REL_BAR * y_ref2).max(RHO_BAR * d_seed2);
        assert!((delta2 - RHO_BAR * d_seed2).abs() < 1e-12);
        assert!(delta2 > REL_BAR * y_ref2);
    }

    /// Mode guard: the held-out gate must refuse when nothing is in place.
    #[test]
    fn held_out_gate_refuses_without_a_frozen_descendant() {
        let dir = std::env::temp_dir().join("rc016_gate_test_empty");
        let _ = std::fs::create_dir_all(&dir);
        assert!(
            !descendant_gate(dir.to_str().unwrap()),
            "held-out must not be runnable without confirmation, held-in and a committed descendant"
        );
    }

    /// The corpus is the pre-registered one, in order, and the array position is
    /// the index the CI seed binds to.
    #[test]
    fn corpus_is_the_preregistered_thirty_in_order() {
        assert_eq!(CORPUS.len(), 30);
        assert_eq!(CORPUS[0].name, "G1");
        assert_eq!(CORPUS[29].name, "G70");
        let mut names: Vec<&str> = CORPUS.iter().map(|c| c.name).collect();
        let n = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "no duplicate instances");
        // Both structural families are represented, as SIGN CONSTANT requires.
        assert!(CORPUS.iter().any(|c| c.family == Family::Toroidal));
        assert!(CORPUS.iter().any(|c| c.family == Family::RandomPlus));
        // Six matched groups, three members each.
        for g in ['A', 'B', 'C', 'D', 'E', 'F'] {
            assert_eq!(
                CORPUS.iter().filter(|c| c.group == Some(g)).count(),
                3,
                "group {g} must have three members"
            );
        }
    }
}
