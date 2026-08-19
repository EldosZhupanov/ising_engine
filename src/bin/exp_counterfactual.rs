//! RC-014 Phase 1 — the restricted exact-counterfactual instrument.
//!
//! Pre-registered in `research/PREREG_RC014.md` as amended by
//! `research/PREREG_RC014_AMENDMENT_1.md` (the amendment controls where they
//! differ). Phase 0 audit: `research/RC014_PHASE0_AUDIT.md`.
//!
//! # What this measures
//!
//! ```text
//!   I_replace_work(k) = Y(P[k <- B, equal sweeps and equal draws]) - Y(P)
//! ```
//!
//! an **equal-work / equal-random-opportunity** counterfactual (amendment A1) —
//! NOT equal wall-time. `Y` is the best canonical energy of the run, so a
//! positive `I` means the substitution was worse.
//!
//! # Why it is exact under a stream RNG
//!
//! `engine_v2`'s Runtime threads ONE `ChaCha8Rng` through every operator
//! (`runtime.rs:196`, `:296`), so in general an intervention desynchronises every
//! later draw. Two conditions make this one substitution exact anyway:
//!
//! 1. `metropolis_sweep` and `gibbs_color_sweep` are draw-identical — both walk
//!    the same colour order over all `n` sites and take exactly one `f64` per
//!    `(sweep, site, replica)` unconditionally (metropolis keeps its count
//!    data-independent with an explicit discarded draw).
//! 2. No later operator's draw count depends on state — the schedule's only
//!    downstream operator is `greedy_descent`, which draws nothing.
//!
//! Condition 1 is *measured* here by `probe_draw_alignment`, not assumed.
//! Anything wider (a second pair, a stochastic downstream operator, deletion
//! instead of substitution) is blocked by ADR-0009.
//!
//! ```text
//! cargo run --release --bin exp_counterfactual -- --controls
//! cargo run --release --bin exp_counterfactual -- --science
//! cargo run --release --bin exp_counterfactual -- --transitions --out FILE
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

// ---------------------------------------------------------------- parameters

/// Pre-registered design (`PREREG_RC014.md` §4). These are FROZEN — a change
/// here is a protocol change, not a tuning knob.
const REPLICAS: usize = 32;
const TEMP_HI: f64 = 4.0;
const TEMP_LO: f64 = 0.1;
const SWEEPS: u32 = 16;
const HELD_IN: [u64; 8] = [1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008];
const HELD_OUT: [u64; 8] = [2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008];
const OP_A: &str = "metropolis_sweep";
const OP_B: &str = "gibbs_color_sweep";

/// The six pre-registered instances, in declaration order. The first is the
/// primary named contrast (§5): it is tested at alpha=0.05 unadjusted, and the
/// remaining eleven (5 instances x 2 arms, plus the primary's other arm) form
/// the Benjamini-Hochberg family at FDR 0.10 (amendment A5).
const INSTANCES: &[(&str, &str)] = &[
    ("G22", "benchmark_suite/data/gset/G22"),
    ("G1", "benchmark_suite/data/gset/G1"),
    ("G11", "benchmark_suite/data/gset/G11"),
    ("G32", "benchmark_suite/data/gset/G32"),
    ("G43", "benchmark_suite/data/gset/G43"),
    ("be100.1", "benchmark_suite/data/biqmac/be100.1.sparse"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    /// All-zeros, every replica identical — what all recorded runs used (RC-002).
    Legacy,
    /// One `random_flip_sweep` prepended: the reachability endpoint.
    Diverse,
}

impl Arm {
    fn tag(self) -> &'static str {
        match self {
            Arm::Legacy => "legacy",
            Arm::Diverse => "diverse",
        }
    }
}

// ------------------------------------------------------------------ plumbing

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

/// Build the pre-registered plan. `x` is the substituted thermal operator; the
/// diverse arm prepends an identical randomising step to BOTH arms, so the two
/// arms still reach the substitution point at the same stream position.
fn build_plan(ir: &ProblemIR, x: &str, sweeps: u32, arm: Arm, seed: u64) -> Plan {
    let mut steps = Vec::new();
    if arm == Arm::Diverse {
        steps.push(PlanStep {
            operator: "random_flip_sweep".into(),
            phase: Phase::Exploit,
            sweeps: 1,
            repeat: 1,
        });
    }
    steps.push(PlanStep {
        operator: x.into(),
        phase: Phase::Exploit,
        sweeps,
        repeat: 1,
    });
    steps.push(PlanStep {
        operator: "greedy_descent".into(),
        phase: Phase::Exploit,
        sweeps: SWEEPS,
        repeat: 1,
    });
    Plan {
        name: "rc014".into(),
        backend: DecisionEngine::analyze(ir).select_backend(),
        num_replicas: REPLICAS,
        temperatures: geometric_ladder(REPLICAS, TEMP_HI, TEMP_LO),
        steps,
        seed,
        rationale: Default::default(),
    }
}

/// Execute a plan through the read-only Runtime and return its record. The
/// initial state is all-zeros in both arms; the diverse arm's randomisation is a
/// plan STEP, not a different initialisation, so the executor's contract is
/// untouched.
fn execute(ir: &ProblemIR, reg: &OperatorRegistry, plan: &Plan) -> Result<RunOut, String> {
    let init = vec![0u8; ir.n];
    let mut state = boxed_state(ir, plan.backend, plan.num_replicas, &init);
    let mut rt = Runtime::new(RunContext::new(plan.seed), plan);
    let rec = rt.run(plan, state.as_mut(), reg, ir)?;
    Ok(RunOut {
        y: rec.best_energy,
        canonical: rec.canonical_energy,
        best_state: rec.best_state.clone(),
        events: rec
            .events
            .iter()
            .map(|e| {
                (
                    e.operator.clone(),
                    e.acceptance,
                    e.best_energy,
                    e.metrics.mean_energy,
                    e.metrics.energy_entropy,
                    e.metrics.diversity,
                )
            })
            .collect(),
    })
}

struct RunOut {
    y: f64,
    canonical: f64,
    best_state: Vec<u8>,
    /// (operator, acceptance, best_energy, mean_energy, entropy, diversity) —
    /// exactly the `StepEvent` fields, which is all Phase 1 claims to observe
    /// (amendment A2).
    events: Vec<(String, f64, f64, f64, f64, f64)>,
}

fn load(path: &str) -> Result<ProblemIR, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    rudy_maxcut_ir(&text)
}

// --------------------------------------------------------- paired statistics
//
// This layer is validated INDEPENDENTLY of the Runtime by the synthetic
// arithmetic positive (amendment A4), because an attribution bug and an absent
// effect look identical in the output.

struct Paired {
    full: Vec<f64>,
    replace: Vec<f64>,
}

impl Paired {
    fn effects(&self) -> Vec<f64> {
        self.replace
            .iter()
            .zip(&self.full)
            .map(|(r, f)| r - f)
            .collect()
    }

    fn mean_effect(&self) -> f64 {
        let e = self.effects();
        e.iter().sum::<f64>() / e.len() as f64
    }

    /// The null scale: spread the SAME schedule produces from seed alone
    /// (RC-007's `d_seed`). Sample standard deviation of the `full` arm.
    fn d_seed(&self) -> f64 {
        let n = self.full.len() as f64;
        if n < 2.0 {
            return 0.0;
        }
        let m = self.full.iter().sum::<f64>() / n;
        (self.full.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    }

    /// RC-007's `inf` failure made explicit: a degenerate null is FLAGGED, never
    /// divided by and never silently dropped.
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

    /// Relative magnitude against the run's own energy scale (§6 threshold 2).
    fn rel(&self) -> f64 {
        let scale = (self.full.iter().sum::<f64>() / self.full.len() as f64).abs();
        if scale < 1e-12 {
            0.0
        } else {
            self.mean_effect().abs() / scale
        }
    }

    /// Exact two-sided sign-flip test over all 2^k assignments (amendment A5).
    /// No Monte-Carlo approximation; ties retained.
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

/// Benjamini-Hochberg step-up. Returns the rejection flags in input order.
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

/// Deterministic percentile bootstrap (seeded), for the real-positive CI.
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

// ------------------------------------------------------------------ controls

/// A3 — the load-bearing measurement, at the operator boundary.
///
/// Two identical states, two cloned generators; apply Metropolis to one and
/// Gibbs to the other at equal sweeps, then draw the next `u64` from each. Equal
/// ⇒ both operators left the stream at the same position, which is the exact
/// property the whole cycle rests on. The shift arm consumes one extra `u32`
/// first and MUST be detected, so a passing probe cannot be vacuous.
fn probe_draw_alignment(ir: &ProblemIR, reg: &OperatorRegistry, seed: u64) -> bool {
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

    let mut sa = boxed_state(ir, backend, REPLICAS, &init);
    let mut sb = boxed_state(ir, backend, REPLICAS, &init);
    let mut sc = boxed_state(ir, backend, REPLICAS, &init);
    let mut ra = ChaCha8Rng::seed_from_u64(seed);
    let mut rb = ra.clone();
    let mut rc = ra.clone();

    let (mut a, mut b, mut c) = (
        reg.lookup(OP_A).expect("registry has metropolis_sweep"),
        reg.lookup(OP_B).expect("registry has gibbs_color_sweep"),
        reg.lookup(OP_B).expect("registry has gibbs_color_sweep"),
    );
    a.apply(sa.as_mut(), &view, &mut ra, Budget { sweeps: SWEEPS });
    b.apply(sb.as_mut(), &view, &mut rb, Budget { sweeps: SWEEPS });
    // Deliberate one-word shift.
    let _: u32 = rc.gen();
    c.apply(sc.as_mut(), &view, &mut rc, Budget { sweeps: SWEEPS });

    let (pa, pb, pc): (u64, u64, u64) = (ra.gen(), rb.gen(), rc.gen());
    let aligned = pa == pb;
    let shift_detected = pa != pc;
    println!(
        "  aligned: metropolis={pa:#018x} gibbs={pb:#018x} -> {}",
        if aligned {
            "EQUAL (pass)"
        } else {
            "DIFFER (FAIL)"
        }
    );
    println!(
        "  shifted: gibbs+1u32={pc:#018x} -> {}",
        if shift_detected {
            "DETECTED (pass)"
        } else {
            "MISSED (FAIL)"
        }
    );
    aligned && shift_detected
}

/// A4 — synthetic arithmetic positive. Fixed paired table with analytically
/// known effects, so the attribution/materiality/degenerate-null logic is
/// validated with no solver in the loop.
fn synthetic_arithmetic_positive() -> bool {
    let p = Paired {
        full: vec![-10.0, -20.0, -30.0, -40.0],
        replace: vec![-9.0, -18.0, -27.0, -36.0],
    };
    let eff = p.effects();
    let want_eff = [1.0, 2.0, 3.0, 4.0];
    let eff_ok = eff.iter().zip(want_eff).all(|(a, b)| (a - b).abs() < 1e-12);
    let mean_ok = (p.mean_effect() - 2.5).abs() < 1e-12;
    // sd of [-10,-20,-30,-40] = sqrt(500/3)
    let sd_ok = (p.d_seed() - (500.0f64 / 3.0).sqrt()).abs() < 1e-12;
    let signs_ok = eff.iter().all(|v| *v > 0.0);
    let not_degen = !p.degenerate();

    let flat = Paired {
        full: vec![-10.0; 4],
        replace: vec![-9.0, -18.0, -27.0, -36.0],
    };
    let degen_ok = flat.degenerate() && flat.rho().is_none();

    println!(
        "  effects={eff:?} mean={:.12} d_seed={:.12} rho={:?}",
        p.mean_effect(),
        p.d_seed(),
        p.rho().map(|r| (r * 1e6).round() / 1e6)
    );
    println!("  constant-full table -> DEGENERATE_NULL: {degen_ok}");
    let ok = eff_ok && mean_ok && sd_ok && signs_ok && not_degen && degen_ok;
    println!(
        "  {}",
        if ok {
            "attribution arithmetic PASS"
        } else {
            "attribution arithmetic FAIL"
        }
    );
    ok
}

/// Null: substituting `metropolis_sweep` for itself must be bit-identical at the
/// run outcome and event level.
fn null_control(ir: &ProblemIR, reg: &OperatorRegistry, seed: u64) -> bool {
    let pa = build_plan(ir, OP_A, SWEEPS, Arm::Legacy, seed);
    let pb = build_plan(ir, OP_A, SWEEPS, Arm::Legacy, seed);
    let (a, b) = match (execute(ir, reg, &pa), execute(ir, reg, &pb)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    let ok = a.y.to_bits() == b.y.to_bits()
        && a.canonical.to_bits() == b.canonical.to_bits()
        && a.best_state == b.best_state
        && a.events.len() == b.events.len()
        && a.events.iter().zip(&b.events).all(|(x, y)| {
            x.0 == y.0 && x.1.to_bits() == y.1.to_bits() && x.2.to_bits() == y.2.to_bits()
        });
    println!(
        "  Y={:.6} bit-identical across replay: {}",
        a.y,
        if ok { "yes (pass)" } else { "NO (FAIL)" }
    );
    ok
}

/// Inert A — zero budget. Both substituted operators consume no draws and change
/// no state, so `I` must be exactly 0. Plumbing only; not the independent inert
/// control (amendment A4).
fn inert_a(ir: &ProblemIR, reg: &OperatorRegistry, seed: u64) -> bool {
    let pa = build_plan(ir, OP_A, 0, Arm::Legacy, seed);
    let pb = build_plan(ir, OP_B, 0, Arm::Legacy, seed);
    let (a, b) = match (execute(ir, reg, &pa), execute(ir, reg, &pb)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    let i = b.y - a.y;
    println!(
        "  I(sweeps=0) = {i:.17e} -> {}",
        if i == 0.0 {
            "exactly 0 (pass)"
        } else {
            "NONZERO (FAIL)"
        }
    );
    i == 0.0
}

/// Inert B — structurally inert `replica_exchange` (amendment A4).
///
/// From all-zeros every replica is identical, so an exchange can only permute
/// identical states: RC-007's vacuity case, used here deliberately. Deleting the
/// step must leave best state and canonical energy exactly unchanged — and it can
/// only do so because the sole downstream operator (`greedy_descent`) draws
/// nothing, so the stream shift the deletion causes reaches nobody.
fn inert_b(ir: &ProblemIR, reg: &OperatorRegistry, seed: u64) -> bool {
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
            name: "rc014-inertB".into(),
            backend: DecisionEngine::analyze(ir).select_backend(),
            num_replicas: REPLICAS,
            temperatures: geometric_ladder(REPLICAS, TEMP_HI, TEMP_LO),
            steps,
            seed,
            rationale: Default::default(),
        }
    };
    let (a, b) = match (execute(ir, reg, &mk(true)), execute(ir, reg, &mk(false))) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return false,
    };
    let ok = a.canonical.to_bits() == b.canonical.to_bits() && a.best_state == b.best_state;
    println!(
        "  with={:.6} without={:.6} state-identical={} -> {}",
        a.canonical,
        b.canonical,
        a.best_state == b.best_state,
        if ok { "pass" } else { "FAIL" }
    );
    ok
}

// ------------------------------------- synthetic operator positive (reference)

/// A deterministic small instance whose full state space is enumerable, so the
/// fixture's own optimum is knowable and the reference is checkable.
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

fn brute_force_min(ir: &ProblemIR) -> f64 {
    let n = ir.n;
    assert!(n <= 22, "enumeration is 2^n");
    let mut best = f64::INFINITY;
    let mut x = vec![0u8; n];
    for m in 0u32..(1u32 << n) {
        for (i, xi) in x.iter_mut().enumerate() {
            *xi = (m >> i & 1) as u8;
        }
        let e = ir.energy(&x);
        if e < best {
            best = e;
        }
    }
    best
}

/// Independent re-derivation of the two operators' decisions from the seed's
/// draws and the exact local fields. Deliberately written against `ProblemIR`
/// only — it shares no code with the production operators, so agreement is
/// evidence rather than tautology.
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
            // Local field h = l_i + sum_j q_ij x_j, recomputed from scratch.
            let (cols, ws) = ir.row(site);
            let mut flip = vec![false; r];
            for rep in 0..r {
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
                    flip[rep] = (u < p1) != xi;
                } else if de <= 0.0 {
                    let _u: f64 = rng.gen(); // alignment draw, discarded
                    flip[rep] = true;
                } else {
                    let p = if t > 0.0 { (-de / t).exp() } else { 0.0 };
                    let u: f64 = rng.gen();
                    flip[rep] = u < p;
                }
            }
            for (rep, &f) in flip.iter().enumerate() {
                if f {
                    x[site * r + rep] ^= 1;
                }
            }
        }
    }
    // Y for a single-step plan is the min over replicas of the canonical energy.
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

/// The colour order both operators walk (`ensure_order`): colours ascending,
/// sites ascending within a colour.
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

/// A4 — synthetic operator positive, the blocking control for the REPLACEMENT
/// path. Two separable claims, reported separately:
///
/// 1. **Agreement** — on EVERY candidate fixture the production operators must
///    reproduce the independent reference exactly. This is the actual check, and
///    it cannot be made to pass by choosing an easier or harder fixture.
/// 2. **Non-vacuity** — at least one candidate must have a non-zero reference
///    difference, otherwise agreement is trivially satisfied by both operators
///    landing on the same optimum (the RC-009 vacuity lesson).
///
/// The fixture grid is deterministic and committed in source, so the accepted
/// `(n, seed, sweeps, replicas)` is fixed before any science seed runs. The first
/// grid used 32 replicas x 16 sweeps and was VACUOUS — both operators saturate to
/// the enumerated optimum on every small instance, so every reference difference
/// was 0. Recorded here rather than silently re-tuned.
fn synthetic_operator_positive(reg: &OperatorRegistry) -> bool {
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
                        name: "rc014-synth".into(),
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
                            "n={n} seed={seed} sweeps={sweeps} r={replicas}: \
                             reference (m={ref_a:.6}, g={ref_b:.6}) vs \
                             production (m={:.6}, g={:.6})",
                            pa.y, pb.y
                        ));
                    }

                    let ref_i = ref_b - ref_a;
                    if accepted.is_none() && ref_i != 0.0 && ok && pb.y - pa.y == ref_i {
                        accepted = Some((n, seed, sweeps, replicas, ref_a, ref_b, ref_i));
                        // Keep scanning a little to build the agreement statistic,
                        // but stop the (large) grid once we have both facts.
                        if checked >= 40 {
                            break 'grid;
                        }
                    }
                }
            }
        }
    }

    println!("  agreement: {agreed}/{checked} fixtures — production operators reproduce the independent reference");
    if let Some(d) = &first_disagreement {
        println!("  FIRST DISAGREEMENT: {d}");
    }
    match accepted {
        Some((n, seed, sweeps, replicas, a, b, i)) => {
            let ir = small_instance(n, seed);
            println!(
                "  accepted fixture: n={n} seed={seed} sweeps={sweeps} replicas={replicas} \
                 pairs={} brute-force optimum={:.1}",
                ir.num_pairs(),
                brute_force_min(&ir)
            );
            println!("    metropolis={a:.6}  gibbs={b:.6}  I_replace_work={i:+.6} (non-zero, non-vacuous)");
            let pass = agreed == checked;
            println!(
                "    {}",
                if pass {
                    "exact agreement on every fixture (pass)"
                } else {
                    "DISAGREEMENT FOUND (FAIL)"
                }
            );
            pass
        }
        None => {
            println!("  no fixture with a non-zero reference difference — control would be VACUOUS (FAIL)");
            false
        }
    }
}

/// Real positive — reproduce the existing Theory Engine DELETION protocol with
/// current code and establish its CI. Named honestly (amendment A4): it validates
/// the Runtime pipeline, not replacement attribution.
fn real_positive(ir: &ProblemIR, reg: &OperatorRegistry, name: &str) -> bool {
    let mk = |with_thermal: bool| {
        let mut steps = Vec::new();
        if with_thermal {
            steps.push(PlanStep {
                operator: OP_A.into(),
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
        move |seed: u64| Plan {
            name: "rc014-realpos".into(),
            backend: DecisionEngine::analyze(ir).select_backend(),
            num_replicas: REPLICAS,
            temperatures: geometric_ladder(REPLICAS, TEMP_HI, TEMP_LO),
            steps: steps.clone(),
            seed,
            rationale: Default::default(),
        }
    };
    let (full_p, abl_p) = (mk(true), mk(false));
    let mut degs = Vec::new();
    for &s in HELD_IN.iter() {
        let (f, a) = match (execute(ir, reg, &full_p(s)), execute(ir, reg, &abl_p(s))) {
            (Ok(f), Ok(a)) => (f, a),
            _ => return false,
        };
        let scale = f.y.abs().max(1.0);
        degs.push((a.y - f.y) / scale);
    }
    let mean = degs.iter().sum::<f64>() / degs.len() as f64;
    let (lo, hi) = bootstrap_ci(&degs, 2000, 0xC0FFEE);
    println!(
        "  {name}: deletion degradation mean {:+.4}% CI95 [{:+.4}%, {:+.4}%]  (n={} seeds)",
        mean * 100.0,
        lo * 100.0,
        hi * 100.0,
        degs.len()
    );
    // The control passes if the pipeline reproduces a POSITIVE degradation
    // (deleting the thermal operator hurts) with a CI excluding zero. This is
    // the freshly established reference, per the amendment's honest naming.
    lo > 0.0
}

// ------------------------------------------------------------------- science

struct Row {
    instance: String,
    arm: &'static str,
    i: f64,
    rho: Option<f64>,
    rel: f64,
    p: f64,
    degenerate: bool,
}

fn science(reg: &OperatorRegistry, seeds: &[u64], label: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for (name, path) in INSTANCES {
        let ir = match load(path) {
            Ok(ir) => ir,
            Err(e) => {
                println!("  {name}: SKIP ({e})");
                continue;
            }
        };
        let st = DecisionEngine::analyze(&ir);
        for arm in [Arm::Legacy, Arm::Diverse] {
            let mut full = Vec::new();
            let mut repl = Vec::new();
            for &s in seeds {
                let a = execute(&ir, reg, &build_plan(&ir, OP_A, SWEEPS, arm, s));
                let b = execute(&ir, reg, &build_plan(&ir, OP_B, SWEEPS, arm, s));
                match (a, b) {
                    (Ok(a), Ok(b)) => {
                        full.push(a.y);
                        repl.push(b.y);
                    }
                    _ => break,
                }
            }
            if full.len() != seeds.len() {
                println!("  {name}/{}: SKIP (run failed)", arm.tag());
                continue;
            }
            let p = Paired {
                full,
                replace: repl,
            };
            let row = Row {
                instance: (*name).to_string(),
                arm: arm.tag(),
                i: p.mean_effect(),
                rho: p.rho(),
                rel: p.rel(),
                p: p.signflip_p(),
                degenerate: p.degenerate(),
            };
            println!(
                "  [{label}] {:>8} n={:<5} d={:.4} {:<7} I={:+.4} rho={} rel={:.4}% p={:.4}{}",
                name,
                ir.n,
                st.density,
                arm.tag(),
                row.i,
                row.rho.map_or("DEGEN".to_string(), |r| format!("{r:.3}")),
                row.rel * 100.0,
                row.p,
                if row.degenerate {
                    "  DEGENERATE_NULL"
                } else {
                    ""
                }
            );
            rows.push(row);
        }
    }
    rows
}

fn report(rows: &[Row]) {
    // Primary named contrast: G22 / legacy, alpha = 0.05 UNADJUSTED (A5).
    let primary = rows
        .iter()
        .find(|r| r.instance == "G22" && r.arm == "legacy");
    println!("\n  --- primary named contrast (alpha=0.05, no multiplicity adjustment) ---");
    match primary {
        Some(r) => {
            let material = !r.degenerate && r.rho.is_some_and(|x| x >= 0.5) && r.rel >= 0.001;
            println!(
                "  G22/legacy  I={:+.4}  rho={}  rel={:.4}%  p={:.4}  material={material}  significant={}",
                r.i,
                r.rho.map_or("DEGEN".into(), |x| format!("{x:.3}")),
                r.rel * 100.0,
                r.p,
                r.p < 0.05
            );
        }
        None => println!("  G22/legacy MISSING"),
    }

    // Secondary family: everything else, BH at FDR 0.10 (A5).
    let sec: Vec<&Row> = rows
        .iter()
        .filter(|r| !(r.instance == "G22" && r.arm == "legacy"))
        .collect();
    let ps: Vec<f64> = sec.iter().map(|r| r.p).collect();
    let rej = benjamini_hochberg(&ps, 0.10);
    println!(
        "\n  --- secondary family, Benjamini-Hochberg FDR=0.10 (n={}) ---",
        sec.len()
    );
    for (r, &ok) in sec.iter().zip(&rej) {
        let material = !r.degenerate && r.rho.is_some_and(|x| x >= 0.5) && r.rel >= 0.001;
        println!(
            "  {:>8}/{:<7} I={:+.4} rho={} rel={:.4}% p={:.4} BH={} material={material}",
            r.instance,
            r.arm,
            r.i,
            r.rho.map_or("DEGEN".into(), |x| format!("{x:.3}")),
            r.rel * 100.0,
            r.p,
            if ok { "reject" } else { "keep-null" }
        );
    }
}

/// Fraction of `(sweep, site, replica)` proposals whose local field is exactly
/// zero, measured along a Metropolis trajectory.
///
/// Required co-observable for the T -> 0 mechanism test (Amendment 2 A2.2): at
/// `dE = 0` the two kernels differ by a CONSTANT — `metropolis_sweep.rs:117`
/// takes the `d <= 0.0` branch and accepts, heat-bath flips with probability
/// 1/2 — and cooling never closes that gap. So a flat or growing advantage at
/// low `T` is uninterpretable without knowing this mass.
fn tie_fraction(
    ir: &ProblemIR,
    order: &[usize],
    temps: &[f64],
    sweeps: u32,
    replicas: usize,
    rng: &mut ChaCha8Rng,
) -> f64 {
    let (n, r) = (ir.n, replicas);
    let mut x = vec![0u8; n * r];
    let (mut ties, mut total) = (0u64, 0u64);
    for _ in 0..sweeps {
        for &site in order {
            let (cols, ws) = ir.row(site);
            let mut flip = vec![false; r];
            for (rep, f) in flip.iter_mut().enumerate() {
                let mut h = ir.linear[site];
                for (&j, &w) in cols.iter().zip(ws) {
                    if x[j as usize * r + rep] != 0 {
                        h += w;
                    }
                }
                let xi = x[site * r + rep] != 0;
                let de = if xi { -h } else { h };
                total += 1;
                if de == 0.0 {
                    ties += 1;
                }
                let t = temps[rep % temps.len()];
                if de <= 0.0 {
                    let _u: f64 = rng.gen();
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
    ties as f64 / total.max(1) as f64
}

/// A deterministic UNWEIGHTED (J in {+-1}) graph at a target density, emitted as
/// rudy text and parsed through the SAME frontend as every benchmark instance,
/// so the QUBO conversion is identical and the comparison is not a parser
/// artifact. D-14a's instance.
fn synthetic_unweighted(n: usize, density: f64, seed: u64) -> Result<ProblemIR, String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut edges = Vec::new();
    for i in 1..=n {
        for j in (i + 1)..=n {
            if rng.gen::<f64>() < density {
                let w = if rng.gen::<bool>() { 1 } else { -1 };
                edges.push((i, j, w));
            }
        }
    }
    let mut text = format!("{n} {}\n", edges.len());
    for (i, j, w) in &edges {
        text.push_str(&format!("{i} {j} {w}\n"));
    }
    rudy_maxcut_ir(&text)
}

/// One (instance, arm) contrast at a given ladder, returned as the paired table.
fn contrast(
    ir: &ProblemIR,
    reg: &OperatorRegistry,
    arm: Arm,
    seeds: &[u64],
    temps: Option<&[f64]>,
) -> Option<Paired> {
    let (mut full, mut repl) = (Vec::new(), Vec::new());
    for &s in seeds {
        let mut pa = build_plan(ir, OP_A, SWEEPS, arm, s);
        let mut pb = build_plan(ir, OP_B, SWEEPS, arm, s);
        if let Some(t) = temps {
            pa.temperatures = t.to_vec();
            pb.temperatures = t.to_vec();
        }
        match (execute(ir, reg, &pa), execute(ir, reg, &pb)) {
            (Ok(a), Ok(b)) => {
                full.push(a.y);
                repl.push(b.y);
            }
            _ => return None,
        }
    }
    Some(Paired {
        full,
        replace: repl,
    })
}

fn line(tag: &str, p: &Paired) {
    println!(
        "    {tag:<28} I={:+.4}  rho={}  rel={:.4}%  p={:.4}{}",
        p.mean_effect(),
        p.rho().map_or("DEGEN".to_string(), |r| format!("{r:.3}")),
        p.rel() * 100.0,
        p.signflip_p(),
        if p.degenerate() {
            "  DEGENERATE_NULL"
        } else {
            ""
        }
    );
}

/// D-14 — is the sign of `I_replace_work` governed by DENSITY or by WEIGHTING?
///
/// Pre-registered in `PREREG_RC014.md` §12 before the held-out arm; instance
/// resolution amended in `PREREG_RC014_AMENDMENT_2.md` A2.3, because NO weighted
/// BiqMac instance has density <= 0.06 (measured over all 125; the sparsest is
/// `gka8a` at 0.0814). Run exactly as written on its two points.
fn d14(reg: &OperatorRegistry) {
    println!("D-14 — density vs weighting (PREREG §12, instances per AMENDMENT_2 A2.3)\n");

    println!("  D-14a: UNWEIGHTED, dense (synthetic, J in {{+-1}}, n=100, target density 0.99)");
    println!("         prediction: I < 0 (Gibbs wins) if the sign follows DENSITY");
    match synthetic_unweighted(100, 0.99, 20260819) {
        Ok(ir) => {
            let st = DecisionEngine::analyze(&ir);
            println!(
                "    realised n={} density={:.4} pairs={}",
                ir.n,
                st.density,
                ir.num_pairs()
            );
            for arm in [Arm::Legacy, Arm::Diverse] {
                match contrast(&ir, reg, arm, &HELD_IN, None) {
                    Some(p) => line(arm.tag(), &p),
                    None => println!("    {} run failed", arm.tag()),
                }
            }
        }
        Err(e) => println!("    generation failed: {e}"),
    }

    println!("\n  D-14b: WEIGHTED, sparsest available (gka8a, density 0.0814)");
    println!("         prediction: I > 0 (Metropolis wins) if the sign follows DENSITY");
    println!("         NOTE: the pre-registered clause asked for density <= 0.06; no weighted");
    println!("         BiqMac instance satisfies it (AMENDMENT_2 A2.3). Deviation recorded.");
    match load("benchmark_suite/data/biqmac/gka8a.sparse") {
        Ok(ir) => {
            let st = DecisionEngine::analyze(&ir);
            println!(
                "    realised n={} density={:.4} pairs={}",
                ir.n,
                st.density,
                ir.num_pairs()
            );
            for arm in [Arm::Legacy, Arm::Diverse] {
                match contrast(&ir, reg, arm, &HELD_IN, None) {
                    Some(p) => line(arm.tag(), &p),
                    None => println!("    {} run failed", arm.tag()),
                }
            }
        }
        Err(e) => println!("    load failed: {e}"),
    }
}

/// Non-gating mechanism probe: flat ladders, with the `dE = 0` tie mass reported
/// alongside (Amendment 2 A2.2 makes the tie fraction a required co-observable).
fn temperature_curve(reg: &OperatorRegistry) {
    println!(
        "Temperature curve — flat ladders, with dE=0 tie mass (non-gating, AMENDMENT_2 A2.2)\n"
    );
    println!("  prediction: the Metropolis advantage shrinks as T -> 0 PROVIDED tie mass is small");
    println!("  at dE = 0 the kernels differ by a constant (accept vs flip w.p. 1/2), so a flat");
    println!("  or growing advantage at low T is uninterpretable without the tie column.\n");
    for (name, path) in INSTANCES {
        let Ok(ir) = load(path) else { continue };
        let backend = DecisionEngine::analyze(&ir).select_backend();
        let init = vec![0u8; ir.n];
        let probe = boxed_state(&ir, backend, REPLICAS, &init);
        let order = colour_order(probe.as_ref());
        drop(probe);
        println!(
            "  {name} (n={}, density={:.4})",
            ir.n,
            DecisionEngine::analyze(&ir).density
        );
        for t in [2.0f64, 0.5, 0.1] {
            let temps = vec![t];
            let mut rng = ChaCha8Rng::seed_from_u64(HELD_IN[0]);
            let ties = tie_fraction(&ir, &order, &temps, SWEEPS, REPLICAS, &mut rng);
            match contrast(&ir, reg, Arm::Legacy, &HELD_IN, Some(&temps)) {
                Some(p) => println!(
                    "    T={t:<4} I={:+10.4}  rho={:<7}  rel={:7.4}%  p={:.4}  dE=0 mass={:6.3}%",
                    p.mean_effect(),
                    p.rho().map_or("DEGEN".to_string(), |r| format!("{r:.3}")),
                    p.rel() * 100.0,
                    p.signflip_p(),
                    ties * 100.0
                ),
                None => println!("    T={t}: run failed"),
            }
        }
    }
}

// ---------------------------------------------------------------------- main

// --------------------------------------------------- held-out provenance gate
//
// PREREG_RC014.md Gate A condition 6: the falsifiable descendant must be written
// into section 12 of that file BEFORE any held-out seed is run. Until RC-019 this
// was a printed reminder with no executable effect; it is now a refusal.

const PREREG_PATH: &str = "research/PREREG_RC014.md";
const DESCENDANT_HEADING: &str = "## 12. Falsifiable descendant";

/// The facts the decision needs, gathered separately so the decision itself is
/// pure and unit-testable without a git tree.
#[derive(Clone, Copy, Debug)]
struct PreregFacts {
    exists: bool,
    tracked: bool,
    clean: bool,
    committed: bool,
    has_descendant: bool,
}

/// Pure: same facts -> same verdict. Names the failed condition.
fn descendant_gate(f: &PreregFacts) -> Result<(), &'static str> {
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
    if !f.has_descendant {
        return Err("the falsifiable descendant section is absent");
    }
    Ok(())
}

fn git_ok(args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn prereg_facts() -> PreregFacts {
    let text = std::fs::read_to_string(PREREG_PATH).unwrap_or_default();
    PreregFacts {
        exists: std::path::Path::new(PREREG_PATH).exists(),
        tracked: git_ok(&["ls-files", "--error-unmatch", PREREG_PATH]),
        clean: git_ok(&["diff", "--quiet", "HEAD", "--", PREREG_PATH]),
        committed: git_ok(&["log", "-1", "--format=%ct", "--", PREREG_PATH]),
        has_descendant: text.contains(DESCENDANT_HEADING),
    }
}

/// Refuse rather than silently inspecting the held-out seeds.
fn require_descendant(mode: &str) {
    if let Err(e) = descendant_gate(&prereg_facts()) {
        eprintln!("\n  {mode} REFUSED — {e} ({PREREG_PATH})");
        eprintln!("  Gate A condition 6: the descendant must be written into");
        eprintln!("  section 12 of {PREREG_PATH} before any held-out seed is run.");
        std::process::exit(3);
    }
}

fn main() {
    let reg = OperatorRegistry::standard();

    if flag("--d14") {
        d14(&reg);
        return;
    }
    if flag("--tempcurve") {
        temperature_curve(&reg);
        return;
    }

    // PREREG_RC014.md section 7: "Controls - all four are Gate-A blocking."
    // The earlier guard ran this block only when --controls was passed or when
    // no science mode was, so `--science` alone SKIPPED every control - the
    // exact inverse of the requirement (RC-019 audit). Controls now gate
    // unconditionally and exit non-zero on failure.
    {
        println!("RC-014 Phase 1 — Gate A controls (PREREG_RC014.md + AMENDMENT_1)\n");
        let mut pass = true;

        println!("[A3] draw-alignment probe at the operator boundary");
        let probe_ir = match load(INSTANCES[0].1) {
            Ok(ir) => ir,
            Err(e) => {
                println!("  cannot load {}: {e}", INSTANCES[0].1);
                std::process::exit(2);
            }
        };
        pass &= probe_draw_alignment(&probe_ir, &reg, 1001);

        println!("\n[A4] synthetic arithmetic positive");
        pass &= synthetic_arithmetic_positive();

        println!("\n[A4] synthetic operator positive (blocking control for replacement)");
        pass &= synthetic_operator_positive(&reg);

        println!("\n[null] metropolis -> metropolis, bit-identical");
        pass &= null_control(&probe_ir, &reg, 1001);

        println!("\n[inert A] substitution at sweeps=0");
        pass &= inert_a(&probe_ir, &reg, 1001);

        println!("\n[inert B] structurally inert replica_exchange deletion");
        pass &= inert_b(&probe_ir, &reg, 1001);

        println!("\n[real positive] fresh deletion CI (Theory Engine protocol)");
        pass &= real_positive(&probe_ir, &reg, INSTANCES[0].0);

        println!(
            "\nGATE A CONTROLS: {}",
            if pass {
                "ALL PASS — science run authorised"
            } else {
                "FAILED — instrument invalid; this is NOT evidence about H-14"
            }
        );
        if !pass {
            std::process::exit(1);
        }
    }

    if flag("--science") {
        println!("\nRC-014 held-in science run (seeds {HELD_IN:?})\n");
        let rows = science(&reg, &HELD_IN, "in");
        report(&rows);
    }

    if flag("--holdout") {
        println!("\nRC-014 HELD-OUT run (seeds {HELD_OUT:?})");
        require_descendant("HELD-OUT");
        println!("Gate A condition 6 verified: descendant written and committed\n");
        let rows = science(&reg, &HELD_OUT, "out");
        report(&rows);
    }

    if flag("--transitions") {
        let out: String = arg("--out", "transitions.tsv".to_string());
        let mut buf = String::from(
            "instance\tarm\toperator_x\tseed\tstep\toperator\tacceptance\tbest_energy\tmean_energy\tentropy\tdiversity\ty_end\n",
        );
        for (name, path) in INSTANCES {
            let Ok(ir) = load(path) else { continue };
            for arm in [Arm::Legacy, Arm::Diverse] {
                for x in [OP_A, OP_B] {
                    for &s in HELD_IN.iter().take(2) {
                        let Ok(r) = execute(&ir, &reg, &build_plan(&ir, x, SWEEPS, arm, s)) else {
                            continue;
                        };
                        for (k, e) in r.events.iter().enumerate() {
                            buf.push_str(&format!(
                                "{name}\t{}\t{x}\t{s}\t{k}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\n",
                                arm.tag(), e.0, e.1, e.2, e.3, e.4, e.5, r.y
                            ));
                        }
                    }
                }
            }
        }
        match std::fs::write(&out, buf) {
            Ok(()) => println!("transitions written to {out}"),
            Err(e) => println!("write failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RC-019 recorded this gate as VACUOUS: a printed reminder with no
    /// executable effect. Each condition must now actually refuse.
    #[test]
    fn descendant_gate_refuses_each_missing_condition() {
        let good = PreregFacts {
            exists: true,
            tracked: true,
            clean: true,
            committed: true,
            has_descendant: true,
        };
        assert!(descendant_gate(&good).is_ok());
        for (f, want) in [
            (
                PreregFacts {
                    exists: false,
                    ..good
                },
                "missing",
            ),
            (
                PreregFacts {
                    tracked: false,
                    ..good
                },
                "tracked",
            ),
            (
                PreregFacts {
                    clean: false,
                    ..good
                },
                "uncommitted",
            ),
            (
                PreregFacts {
                    committed: false,
                    ..good
                },
                "never been committed",
            ),
            (
                PreregFacts {
                    has_descendant: false,
                    ..good
                },
                "descendant section is absent",
            ),
        ] {
            let e = descendant_gate(&f).expect_err("must refuse");
            assert!(e.contains(want), "message {e:?} should name {want:?}");
        }
    }

    /// The heading is the literal the gate greps for; if the pre-registration
    /// is renamed the gate must fail loudly here, not silently pass forever.
    #[test]
    fn descendant_heading_exists_in_the_preregistration() {
        let text = std::fs::read_to_string(PREREG_PATH).expect("pre-registration readable");
        assert!(
            text.contains(DESCENDANT_HEADING),
            "{DESCENDANT_HEADING} not found in {PREREG_PATH}"
        );
    }
}
