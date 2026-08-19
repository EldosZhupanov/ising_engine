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
/// Amendment 1 §A5.3: equal sweeps is equal cost inside this band.
const COST_RATIO_LO: f64 = 0.95;
const COST_RATIO_HI: f64 = 1.05;
/// Amendment 1 §A6.2 step 6.
const POWER_BAR: f64 = 0.80;

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
const CORPUS: &[(&str, &str, Option<char>, Family)] = &[
    (
        "G1",
        "benchmark_suite/data/gset/G1",
        Some('B'),
        Family::RandomPlus,
    ),
    (
        "G2",
        "benchmark_suite/data/gset/G2",
        Some('B'),
        Family::RandomPlus,
    ),
    (
        "G3",
        "benchmark_suite/data/gset/G3",
        Some('B'),
        Family::RandomPlus,
    ),
    (
        "G11",
        "benchmark_suite/data/gset/G11",
        Some('A'),
        Family::Toroidal,
    ),
    (
        "G12",
        "benchmark_suite/data/gset/G12",
        Some('A'),
        Family::Toroidal,
    ),
    (
        "G13",
        "benchmark_suite/data/gset/G13",
        Some('A'),
        Family::Toroidal,
    ),
    (
        "G14",
        "benchmark_suite/data/gset/G14",
        None,
        Family::RandomPlus,
    ),
    (
        "G15",
        "benchmark_suite/data/gset/G15",
        None,
        Family::RandomPlus,
    ),
    (
        "G16",
        "benchmark_suite/data/gset/G16",
        None,
        Family::RandomPlus,
    ),
    (
        "G22",
        "benchmark_suite/data/gset/G22",
        Some('E'),
        Family::RandomPlus,
    ),
    (
        "G23",
        "benchmark_suite/data/gset/G23",
        Some('E'),
        Family::RandomPlus,
    ),
    (
        "G24",
        "benchmark_suite/data/gset/G24",
        Some('E'),
        Family::RandomPlus,
    ),
    (
        "G32",
        "benchmark_suite/data/gset/G32",
        Some('D'),
        Family::Toroidal,
    ),
    (
        "G33",
        "benchmark_suite/data/gset/G33",
        Some('D'),
        Family::Toroidal,
    ),
    (
        "G34",
        "benchmark_suite/data/gset/G34",
        Some('D'),
        Family::Toroidal,
    ),
    (
        "G35",
        "benchmark_suite/data/gset/G35",
        None,
        Family::RandomPlus,
    ),
    (
        "G36",
        "benchmark_suite/data/gset/G36",
        None,
        Family::RandomPlus,
    ),
    (
        "G43",
        "benchmark_suite/data/gset/G43",
        Some('C'),
        Family::RandomPlus,
    ),
    (
        "G44",
        "benchmark_suite/data/gset/G44",
        Some('C'),
        Family::RandomPlus,
    ),
    (
        "G45",
        "benchmark_suite/data/gset/G45",
        Some('C'),
        Family::RandomPlus,
    ),
    (
        "G48",
        "benchmark_suite/data/gset/G48",
        Some('F'),
        Family::RandomPlus,
    ),
    (
        "G49",
        "benchmark_suite/data/gset/G49",
        Some('F'),
        Family::RandomPlus,
    ),
    (
        "G50",
        "benchmark_suite/data/gset/G50",
        Some('F'),
        Family::RandomPlus,
    ),
    (
        "G51",
        "benchmark_suite/data/gset/G51",
        None,
        Family::RandomPlus,
    ),
    (
        "G52",
        "benchmark_suite/data/gset/G52",
        None,
        Family::RandomPlus,
    ),
    (
        "G53",
        "benchmark_suite/data/gset/G53",
        None,
        Family::RandomPlus,
    ),
    (
        "G55",
        "benchmark_suite/data/gset/G55",
        None,
        Family::RandomPlus,
    ),
    (
        "G60",
        "benchmark_suite/data/gset/G60",
        None,
        Family::RandomPlus,
    ),
    (
        "G63",
        "benchmark_suite/data/gset/G63",
        None,
        Family::RandomPlus,
    ),
    (
        "G70",
        "benchmark_suite/data/gset/G70",
        None,
        Family::RandomPlus,
    ),
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
    let mut missing = Vec::new();
    for (name, path, _, _) in CORPUS {
        if !std::path::Path::new(path).exists() {
            missing.push(*name);
        }
    }
    let count_ok = CORPUS.len() == 30;
    println!(
        "    {} instances declared, {} missing on disk",
        CORPUS.len(),
        missing.len()
    );
    if !missing.is_empty() {
        println!("    MISSING: {missing:?}");
    }
    count_ok && missing.is_empty()
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

    let probe_ir = match load(CORPUS[0].1) {
        Ok(ir) => ir,
        Err(e) => {
            println!("\ncannot load {}: {e}", CORPUS[0].1);
            return false;
        }
    };

    println!("\n[alignment] both operators leave the generator aligned; shift detected");
    pass &= control_alignment(&probe_ir, reg);

    println!("\n[null] metropolis -> metropolis is bit-identical");
    pass &= control_null(&probe_ir, reg);

    println!("\n[inert] substitution at sweeps=0 gives exactly 0");
    pass &= control_inert_zero_budget(&probe_ir, reg);

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
    let (name, path, _, _) = CORPUS[idx];
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
    for (index, (name, path, group, family)) in CORPUS.iter().enumerate() {
        let ir = match load(path) {
            Ok(ir) => ir,
            Err(e) => {
                println!("  {name}: SKIP ({e})");
                continue;
            }
        };
        let Some(p) = contrast(&ir, reg, block.seeds(), SWEEPS) else {
            println!("  {name}: SKIP (run failed)");
            continue;
        };
        let s = sensitivity_ci(&p, index, block);
        let row = Row {
            index,
            name: (*name).to_string(),
            group: *group,
            family: *family,
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
    rows
}

/// STEP 4 — Amendment 1 §A2: BH-reject ∧ material ∧ A6 replication.
/// A TIE-BLOCKED instance can never satisfy this (Amendment 2 §B4).
fn qualifies(hi: &Row, ho: &Row, bh_reject: bool) -> bool {
    if hi.tie_blocked || hi.degenerate || ho.degenerate {
        return false;
    }
    let a6 = hi.i != 0.0 && hi.i.signum() == ho.i.signum() && ho.material && {
        let ratio = (ho.i / hi.i).abs();
        (0.5..=2.0).contains(&ratio)
    };
    bh_reject && hi.material && a6
}

fn verdict(held_in: &[Row], held_out: &[Row]) {
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

    let degen = held_in.iter().filter(|r| r.degenerate).count()
        + held_out.iter().filter(|r| r.degenerate).count();
    let degen_frac = degen as f64 / (held_in.len() + held_out.len()).max(1) as f64;

    let k_plus = qual.iter().filter(|r| r.i > 0.0).count();
    let k_minus = qual.iter().filter(|r| r.i < 0.0).count();
    let kq = qual.len();

    println!("\n  --- corpus census (descriptive; NO cross-instance p-value is computed or licensed) ---");
    println!(
        "  K = {kq} qualifying of {} enumerated   k+ = {k_plus}   k- = {k_minus}   K/30 = {:.3}",
        CORPUS.len(),
        kq as f64 / CORPUS.len() as f64
    );
    println!("  material-but-unqualified or tie-blocked: {material_unqualified}   degenerate cells: {degen} ({:.1}%)",
             degen_frac * 100.0);

    // Kill criterion 2 first: it overrides the sign question entirely.
    if degen_frac >= DEGENERATE_FRACTION_KILL {
        println!("\n  VERDICT: BENCHMARK-VALIDITY FINDING — >= 50% of cells are DEGENERATE_NULL.");
        println!("  The configuration is saturated at this budget and cannot discriminate.");
        println!("  The sign question is NOT answered.");
        return;
    }

    if k_plus >= 1 && k_minus >= 1 {
        println!("\n  VERDICT: SIGN VARIES");
        for g in ['A', 'B', 'C', 'D', 'E', 'F'] {
            let members: Vec<&&Row> = qual.iter().filter(|r| r.group == Some(g)).collect();
            let has_pos = members.iter().any(|r| r.i > 0.0);
            let has_neg = members.iter().any(|r| r.i < 0.0);
            if has_pos && has_neg {
                println!(
                    "    matched group {g}: opposite-signed qualifying members — S0 COUNTEREXAMPLE"
                );
                for m in &members {
                    println!("      {} I={:+.4}", m.name, m.i);
                }
            }
        }
        return;
    }

    if kq >= MIN_QUALIFYING && (k_plus == 0 || k_minus == 0) {
        let mut groups: Vec<char> = qual.iter().filter_map(|r| r.group).collect();
        groups.sort_unstable();
        groups.dedup();
        let fams: std::collections::BTreeSet<&str> = qual
            .iter()
            .map(|r| match r.family {
                Family::RandomPlus => "random+1",
                Family::Toroidal => "toroidal",
            })
            .collect();
        if groups.len() >= MIN_GROUPS && fams.len() >= 2 {
            println!(
                "\n  VERDICT: SIGN CONSTANT — {kq} qualifying, {} matched groups, {} families",
                groups.len(),
                fams.len()
            );
            return;
        }
        println!("\n  VERDICT: Q-INCONCLUSIVE — {kq} qualifying but coverage insufficient ({} groups, {} families; need >= {MIN_GROUPS} and 2)",
                 groups.len(), fams.len());
        return;
    }

    // NO MATERIAL EFFECT OBSERVED requires the POSITIVE condition, not K = 0.
    let all_clean = held_in
        .iter()
        .chain(held_out.iter())
        .all(|r| !r.degenerate && !r.material && !r.tie_blocked)
        && held_in.len() == CORPUS.len()
        && held_out.len() == CORPUS.len();

    if kq == 0 && all_clean {
        println!("\n  VERDICT: NO MATERIAL EFFECT OBSERVED");
        println!("  A DESCRIPTIVE statement about this design's sensitivity, and nothing more.");
        println!("  FORBIDDEN under this verdict: any claim of equivalence, interchangeability or");
        println!("  indistinguishability; any claim the effect is zero or negligible; any claim");
        println!("  bounding what a selector could gain.");
        let mut hw: Vec<f64> = held_in.iter().map(|r| r.ci_hw_pct).collect();
        hw.sort_by(f64::total_cmp);
        println!(
            "  Mandatory sensitivity: held-in CI half-width {:.4}%..{:.4}% of |mean Y(metropolis)|",
            hw.first().copied().unwrap_or(f64::NAN),
            hw.last().copied().unwrap_or(f64::NAN)
        );
        return;
    }

    println!("\n  VERDICT: Q-INCONCLUSIVE — K = {kq}; {material_unqualified} material-but-unqualified or tie-blocked; {degen} degenerate cells.");
    println!("  No post-hoc relaxation of any bar is permitted.");
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
        Err(e) => println!("\n  write failed: {e}"),
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

    // STEP 9 — controls gate everything.
    if flag("--controls") || !any_science {
        let ok = run_controls(&reg);
        if !ok {
            std::process::exit(1);
        }
        if !any_science {
            return;
        }
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
                    println!(
                        "  {:>5} R={:.4} Y_ref={:.2} d_seed={:.4} delta={:.4} power={:.4} {} {}",
                        c.name,
                        c.r,
                        c.y_ref,
                        c.d_seed_pilot,
                        c.delta,
                        c.power,
                        if c.powered {
                            "POWERED"
                        } else {
                            "UNDERPOWERED (cost arm NOT run)"
                        },
                        if c.cost_arm_needed {
                            "cost-arm-needed"
                        } else {
                            "R in band: equal sweeps IS equal cost"
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
                None => println!("  {:>5} calibration failed", entry.0),
            }
        }
        let _ = std::fs::create_dir_all(&dir);
        let path = format!("{dir}/rc016_calibration.tsv");
        match std::fs::write(&path, out) {
            Ok(()) => println!("\n  frozen: {path}"),
            Err(e) => println!("\n  write failed: {e}"),
        }
    }

    if flag("--science") {
        println!("\nRC-016 held-in block, seeds {HELD_IN:?}\n");
        let rows = measure_block(&reg, Block::HeldIn);
        write_rows(&dir, "heldin", &rows);
    }

    if flag("--holdout") {
        println!("\nRC-016 held-out block, seeds {HELD_OUT:?}\n");
        let rows = measure_block(&reg, Block::HeldOut);
        write_rows(&dir, "heldout", &rows);
    }

    if flag("--verdict") {
        match (read_rows(&dir, "heldin"), read_rows(&dir, "heldout")) {
            (Some(hi), Some(ho)) => verdict(&hi, &ho),
            _ => println!("\n  verdict needs both rc016_heldin.tsv and rc016_heldout.tsv in {dir}"),
        }
    }
}
