//! RC-021 §3 — the diagnostic seeds, the frozen sentinel workload, and the
//! per-measurement paired-spread decision.
//!
//! Everything here is frozen numerically by the preregistration. Nothing in
//! this module chooses a value, and nothing in it can observe a solution: the
//! sentinel returns wall time and a count of the work it did, and has no code
//! path that can compute an energy, a cut, a state digest or a quality.

#![allow(dead_code)] // Consumers arrive in later commits of the §12 plan.

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{Budget, Operator};
use ising_engine::engine_v2::operators::MetropolisSweep;
use ising_engine::engine_v2::runtime::RuntimeView;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

// ========================================================== §3.2 THE SEEDS

/// §3.2. RC-021 produces no science outcome, so these are **diagnostic and
/// control** seeds — never pilot, held-in or held-out.
pub const RC021_SENTINEL_SEED: u64 = 31001;
pub const RC021_LOAD_SEED: u64 = 31002;
pub const RC021_SYNTH_SEED: u64 = 31003;
pub const RC021_ORDER_SEED: u64 = 31004;

/// `[31001, 31099]` belongs to RC-021 and to nothing else.
pub const RC021_BAND: (u64, u64) = (31001, 31099);

pub const RC021_SEEDS: [(&str, u64); 4] = [
    ("RC021_SENTINEL_SEED", RC021_SENTINEL_SEED),
    ("RC021_LOAD_SEED", RC021_LOAD_SEED),
    ("RC021_SYNTH_SEED", RC021_SYNTH_SEED),
    ("RC021_ORDER_SEED", RC021_ORDER_SEED),
];

/// One occupied interval, inclusive at both ends.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SeedRange {
    pub name: &'static str,
    pub source: &'static str,
    pub lo: u64,
    pub hi: u64,
}

impl SeedRange {
    pub fn contains(&self, seed: u64) -> bool {
        self.lo <= seed && seed <= self.hi
    }
}

/// §3.1, frozen verbatim:
/// `{991, 993} ∪ [1001, 12008] ∪ {20001} ∪ [20260819, 20262109]`.
///
/// This is the gate. [`DERIVED_FAMILIES`] is the evidence that produced it.
pub const OCCUPANCY: [SeedRange; 5] = [
    SeedRange {
        name: "control 991",
        source: "exp_marginal_cost.rs:56",
        lo: 991,
        hi: 991,
    },
    SeedRange {
        name: "control 993",
        source: "exp_marginal_cost.rs:57",
        lo: 993,
        hi: 993,
    },
    SeedRange {
        name: "science families RC-014..RC-020",
        source: "PREREG_RC021_HOST_INSTRUMENT.md §3.1",
        lo: 1001,
        hi: 12008,
    },
    SeedRange {
        name: "control 20001",
        source: "exp_marginal_cost.rs:53",
        lo: 20001,
        hi: 20001,
    },
    SeedRange {
        name: "bootstrap and permutation, including derived offsets",
        source: "PREREG_RC021_HOST_INSTRUMENT.md §3.1",
        lo: 20260819,
        hi: 20262109,
    },
];

/// **Derived from call sites, not from constant declarations.**
///
/// §3.1 records why this distinction is load-bearing: `exp_cost_identification`
/// uses `PERM_BASE_SEED + 1000 + rep`, so with `PERM_BASE_SEED = 20261101` and
/// `REPETITIONS = 9` it occupies `20262101..=20262109` — a range no declared
/// constant names, and the one that sets the upper bound of [`OCCUPANCY`]. An
/// enumeration of constants alone would place RC-021's band next to an
/// occupancy that stops nearly a thousand values short.
pub const DERIVED_FAMILIES: [SeedRange; 16] = [
    SeedRange {
        name: "RC-014/015/016 held-in",
        source: "exp_counterfactual.rs:61",
        lo: 1001,
        hi: 1008,
    },
    SeedRange {
        name: "held-out",
        source: "exp_counterfactual.rs:62",
        lo: 2001,
        hi: 2008,
    },
    SeedRange {
        name: "RC-016 pilot",
        source: "exp_sensor_sufficiency.rs:65",
        lo: 3001,
        hi: 3008,
    },
    SeedRange {
        name: "RC-017 pilot",
        source: "PREREG_RC017.md §3",
        lo: 4001,
        hi: 4008,
    },
    SeedRange {
        name: "RC-017 held-in (burned)",
        source: "PREREG_RC017.md §3",
        lo: 5001,
        hi: 5008,
    },
    SeedRange {
        name: "RC-017 held-out",
        source: "PREREG_RC017.md §3",
        lo: 6001,
        hi: 6008,
    },
    SeedRange {
        name: "RC-018 pilot (burned)",
        source: "exp_cost_identification.rs:53",
        lo: 7001,
        hi: 7008,
    },
    SeedRange {
        name: "RC-018 held-in",
        source: "exp_cost_identification.rs:54",
        lo: 8001,
        hi: 8008,
    },
    SeedRange {
        name: "RC-018 held-out",
        source: "exp_cost_identification.rs:55",
        lo: 9001,
        hi: 9008,
    },
    SeedRange {
        name: "RC-020 pilot (burned)",
        source: "exp_marginal_cost.rs:40",
        lo: 10001,
        hi: 10008,
    },
    SeedRange {
        name: "RC-020 held-in",
        source: "exp_marginal_cost.rs:41",
        lo: 11001,
        hi: 11008,
    },
    SeedRange {
        name: "RC-020 held-out",
        source: "exp_marginal_cost.rs:42",
        lo: 12001,
        hi: 12008,
    },
    SeedRange {
        name: "control",
        source: "exp_marginal_cost.rs:53",
        lo: 20001,
        hi: 20001,
    },
    SeedRange {
        name: "bootstrap bases + rep",
        source: "exp_sensor_sufficiency.rs:79",
        lo: 20260819,
        hi: 20260828,
    },
    SeedRange {
        name: "bootstrap bases + rep",
        source: "exp_cost_identification.rs:72",
        lo: 20260901,
        hi: 20261309,
    },
    // The one that is easy to miss: `PERM_BASE_SEED + 1000 + rep`.
    SeedRange {
        name: "permutation +1000 offset",
        source: "exp_cost_identification.rs:978",
        lo: 20262101,
        hi: 20262109,
    },
];

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProtocolError {
    /// §3.3: an RC-021 seed is a member of an occupied family. Class I.
    SeedCollision {
        name: &'static str,
        seed: u64,
        family: &'static str,
    },
    SeedOutsideBand {
        name: &'static str,
        seed: u64,
    },
    SeedNotDistinct {
        a: &'static str,
        b: &'static str,
        seed: u64,
    },
    /// The instance bytes are not the frozen G11.
    InstanceHashMismatch {
        expected: &'static str,
        observed: String,
    },
    InstanceUnparsable(String),
    StateInit(String),
    /// §4.1: sessions are numbered `0..=5`; there is no seventh session.
    SessionIndexOutOfRange {
        index: u64,
        max: u64,
    },
    /// A work count that does not fit — caught before any state is built.
    WorkOverflow {
        window: u32,
        extra: u32,
    },
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::SeedCollision { name, seed, family } => {
                write!(f, "{name} = {seed} collides with {family}")
            }
            ProtocolError::SeedOutsideBand { name, seed } => write!(
                f,
                "{name} = {seed} is outside the reserved band [{}, {}]",
                RC021_BAND.0, RC021_BAND.1
            ),
            ProtocolError::SeedNotDistinct { a, b, seed } => {
                write!(f, "{a} and {b} are both {seed}")
            }
            ProtocolError::InstanceHashMismatch { expected, observed } => {
                write!(f, "instance sha256[:12] {observed}, expected {expected}")
            }
            ProtocolError::InstanceUnparsable(e) => write!(f, "instance unparsable: {e}"),
            ProtocolError::StateInit(e) => write!(f, "state init: {e}"),
            ProtocolError::SessionIndexOutOfRange { index, max } => {
                write!(f, "session index {index} is outside 0..={max}")
            }
            ProtocolError::WorkOverflow { window, extra } => {
                write!(f, "sweep count overflows: {window} + {extra}")
            }
        }
    }
}

/// §3.3, run **before any measurement**: no RC-021 seed is a member of any
/// occupied family, all four lie inside the reserved band, and all four are
/// pairwise distinct. Pure — it reads no file and touches no clock.
///
/// Failure is Class I `INSTRUMENT-INVALID`. **There is no fallback seed.** A
/// missing or colliding seed is a refusal; substituting an old, burned or
/// reserved value under any condition is forbidden.
pub fn check_seed_disjointness() -> Result<(), ProtocolError> {
    validate_seed_set(&RC021_SEEDS, RC021_BAND, &OCCUPANCY)
}

/// The gate itself, over an **injected** seed set, band and occupancy.
///
/// Separated so that the rule can be exercised against a genuine collision.
/// RC-021's frozen band is disjoint from the occupancy by construction, so a
/// test that could only pass the real constants could never reach the
/// collision branch, and a test that hand-builds the error value proves
/// nothing about the code that is supposed to produce it.
pub fn validate_seed_set(
    seeds: &[(&'static str, u64)],
    band: (u64, u64),
    occupancy: &[SeedRange],
) -> Result<(), ProtocolError> {
    for (name, seed) in seeds {
        if *seed < band.0 || *seed > band.1 {
            return Err(ProtocolError::SeedOutsideBand { name, seed: *seed });
        }
        if let Some(r) = occupied_in(occupancy, *seed) {
            return Err(ProtocolError::SeedCollision {
                name,
                seed: *seed,
                family: r.name,
            });
        }
    }
    for (i, (name_a, seed_a)) in seeds.iter().enumerate() {
        for (name_b, seed_b) in seeds.iter().skip(i + 1) {
            if seed_a == seed_b {
                return Err(ProtocolError::SeedNotDistinct {
                    a: name_a,
                    b: name_b,
                    seed: *seed_a,
                });
            }
        }
    }
    Ok(())
}

/// The occupied family a seed belongs to, if any, within [`OCCUPANCY`].
pub fn occupied_by(seed: u64) -> Option<SeedRange> {
    occupied_in(&OCCUPANCY, seed)
}

pub fn occupied_in(occupancy: &[SeedRange], seed: u64) -> Option<SeedRange> {
    occupancy.iter().find(|r| r.contains(seed)).copied()
}

/// §4.2: six sessions, indexed **zero-based** `0..=5`. There is no seventh.
pub const MAX_SESSION_INDEX: u64 = 5;

/// §4.1: deterministic ordering **within** a phase, where any freedom exists.
///
/// Fallible, and checked twice over: the index is bounded before it is used,
/// the addition is checked, and the result is verified to still lie inside the
/// reserved band. An unbounded `RC021_ORDER_SEED + session_index` can leave the
/// band, and at the top of `u64` it wraps back into it.
pub fn order_seed(session_index: u64) -> Result<u64, ProtocolError> {
    if session_index > MAX_SESSION_INDEX {
        return Err(ProtocolError::SessionIndexOutOfRange {
            index: session_index,
            max: MAX_SESSION_INDEX,
        });
    }
    let seed = RC021_ORDER_SEED.checked_add(session_index).ok_or(
        ProtocolError::SessionIndexOutOfRange {
            index: session_index,
            max: MAX_SESSION_INDEX,
        },
    )?;
    if seed < RC021_BAND.0 || seed > RC021_BAND.1 {
        return Err(ProtocolError::SeedOutsideBand {
            name: "RC021_ORDER_SEED + session_index",
            seed,
        });
    }
    Ok(seed)
}

// ====================================================== §3.4 THE SENTINEL

/// Every parameter is frozen numerically by §3.4 and **inherited** from
/// RC-020's sentinel rather than newly chosen.
pub const SENTINEL_OPERATOR: &str = "metropolis_sweep";
pub const SENTINEL_INSTANCE: &str = "G11";
pub const SENTINEL_TEMP: f64 = 0.5;
pub const SENTINEL_WINDOW: u32 = 8;
pub const SENTINEL_REPLICAS: usize = 32;
pub const SENTINEL_PREFIX_SWEEPS: u32 = 4;

/// `PREREG_RC016.md` §4: the first twelve hex digits of the SHA-256 of the
/// instance file's exact bytes.
pub const G11_SHA256_PREFIX: &str = "c2a760d2926d";

/// §3.4: the instance is hash-verified **before** any parse or execution, so a
/// substituted or corrupted file can never reach the solver.
pub fn verify_sentinel_instance(bytes: &[u8]) -> Result<(), ProtocolError> {
    let observed = crate::host::sha256_hex(bytes);
    if observed.get(..12) == Some(G11_SHA256_PREFIX) {
        Ok(())
    } else {
        Err(ProtocolError::InstanceHashMismatch {
            expected: G11_SHA256_PREFIX,
            observed,
        })
    }
}

/// A [`ProblemIR`] that **has** passed the §3.4 hash gate.
///
/// The field is private and there is no other constructor, so the type is the
/// proof: a production sentinel cannot be handed an arbitrary instance, because
/// the only way to obtain one of these is [`load_sentinel_instance`], and that
/// verifies the bytes before it parses them. A signature taking a bare
/// `ProblemIR` would let any graph — synthetic, substituted or corrupted —
/// reach the solver with the gate intact but bypassed.
pub struct VerifiedSentinelInstance(ProblemIR);

impl VerifiedSentinelInstance {
    pub fn num_vars(&self) -> usize {
        self.0.n
    }
}

/// Verify the bytes, then parse. Never the other way round.
pub fn load_sentinel_instance(bytes: &[u8]) -> Result<VerifiedSentinelInstance, ProtocolError> {
    verify_sentinel_instance(bytes)?;
    let text =
        std::str::from_utf8(bytes).map_err(|e| ProtocolError::InstanceUnparsable(e.to_string()))?;
    ising_engine::engine_v2::frontend::rudy_maxcut_ir(text)
        .map(VerifiedSentinelInstance)
        .map_err(ProtocolError::InstanceUnparsable)
}

/// §6 P1's injection hook, and nothing else.
///
/// P1 must deliberately make a sentinel slow enough to exceed the `0.09` bound,
/// because a guard that cannot fail is not a guard. It is named explicitly so
/// that extra work can never enter a measurement by accident: every ordinary
/// execution passes [`ExtraWork::NONE`], and the type has no other constructor
/// that a normal path would reach for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExtraWork(u32);

impl ExtraWork {
    pub const NONE: ExtraWork = ExtraWork(0);
    /// Reserved for the P1 positive control of a later commit.
    pub const fn for_p1_injection(extra_sweeps: u32) -> ExtraWork {
        ExtraWork(extra_sweeps)
    }
    pub const fn sweeps(self) -> u32 {
        self.0
    }
}

/// What one sentinel execution reports: wall time and an account of the work
/// that produced it. **No energy, state digest, cut, quality, objective or
/// operator delta** — the operator's return value is deliberately discarded,
/// and there is no field here that could hold one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SentinelRun {
    pub wall_ms: f64,
    pub measured_sweeps: u32,
    pub prefix_sweeps: u32,
    pub replicas: usize,
}

/// §3.4. One sentinel: all-zero init, an untimed 4-sweep prefix, then the timed
/// 8-sweep window. The prefix and the window are consecutive `apply` calls on
/// one state and one RNG stream, so nothing subtracts timings taken from
/// different executions.
///
/// **There is no seed parameter.** Every sentinel execution uses
/// [`RC021_SENTINEL_SEED`], which is what makes a paired spread a comparison of
/// identical work over identical numbers, and what keeps a burned or science
/// seed from ever reaching execution through this path. The phase-C load path
/// with [`RC021_LOAD_SEED`] is a later commit and will be its own function.
pub fn run_sentinel(
    instance: &VerifiedSentinelInstance,
    extra: ExtraWork,
) -> Result<SentinelRun, ProtocolError> {
    execute_sentinel(&instance.0, RC021_SENTINEL_SEED, extra)
}

/// The executor. Private: the only production caller is [`run_sentinel`], which
/// supplies a verified instance and the frozen seed. Tests reach it directly so
/// the timing machinery can be exercised on a tiny synthetic graph without ever
/// running the sentinel on G11 — a freedom deliberately denied to every
/// non-test caller.
fn execute_sentinel(
    ir: &ProblemIR,
    seed: u64,
    extra: ExtraWork,
) -> Result<SentinelRun, ProtocolError> {
    // Checked BEFORE any state is built and before the first sweep: in release
    // an overflowing `+` wraps, and injected extra work would silently become
    // *less* work than the plain window.
    let measured_sweeps =
        SENTINEL_WINDOW
            .checked_add(extra.sweeps())
            .ok_or(ProtocolError::WorkOverflow {
                window: SENTINEL_WINDOW,
                extra: extra.sweeps(),
            })?;
    let temps = vec![SENTINEL_TEMP; SENTINEL_REPLICAS];
    let init = vec![0u8; ir.n];
    let mut st = SparseBitSlice::new(ir, SENTINEL_REPLICAS, &init)
        .map_err(|e| ProtocolError::StateInit(format!("{e:?}")))?;
    let v = RuntimeView {
        iteration: 0,
        temperatures: &temps,
        num_replicas: SENTINEL_REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    };
    let mut op = MetropolisSweep::new();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Untimed prefix. The return value is dropped without inspection: reading
    // it is how an outcome-blind instrument stops being outcome-blind.
    let _ = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: SENTINEL_PREFIX_SWEEPS,
        },
    );

    let t0 = Instant::now();
    let _ = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: measured_sweeps,
        },
    );
    let wall_ms = t0.elapsed().as_secs_f64() * 1000.0;

    Ok(SentinelRun {
        wall_ms,
        measured_sweeps,
        prefix_sweeps: SENTINEL_PREFIX_SWEEPS,
        replicas: SENTINEL_REPLICAS,
    })
}

// ============================================== §4.1 THE UNMEASURED WORK

/// §4.1 phase-B warmup: **32 sentinel-equivalents, `32 × 8 = 256 sweeps`**,
/// with [`RC021_SENTINEL_SEED`], executed and discarded before phase B's pairs.
pub const WARMUP_SWEEPS: u32 = 256;

/// §4.1 phase-C load block: **256 sentinel-equivalents, `2048 sweeps`**, with
/// [`RC021_LOAD_SEED`], executed immediately before **each** phase-C pair.
pub const LOAD_BLOCK_SWEEPS: u32 = 2048;

/// §4.1's two blocks of unmeasured work — a **closed set of exactly two**.
///
/// There is no free-form constructor and no public field, so a caller cannot
/// substitute a sweep count or a seed: the only two blocks the protocol defines
/// are the ones it names. An open struct with public `sweeps` and `seed` would
/// let any caller drive the operator with an arbitrary seed and an arbitrary
/// amount of work — the same hole the sentinel closed with
/// [`VerifiedSentinelInstance`].
///
/// §4.1 freezes each block's **total sweep count** and its seed, and nothing
/// else: it gives `32 × 8 = 256` and `2048` as counts of sweeps, and calls the
/// warmup "32× one sentinel" while its own arithmetic excludes the untimed
/// 4-sweep prefix. It does **not** say whether those sweeps arrive as one
/// `apply` or as thirty-two, nor whether each unit would get a fresh state and
/// a fresh RNG stream. [`run_work_block`] runs the frozen count as a single
/// `apply` on one state and one stream — the reading that adds no structure the
/// documents do not state. Decomposing would require choosing that structure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkBlock {
    /// §4.1 phase-B: executed once, before the phase's five pairs.
    Warmup,
    /// §4.1 phase-C: executed immediately before **each** of the five pairs.
    Load,
}

impl WorkBlock {
    pub const fn sweeps(self) -> u32 {
        match self {
            WorkBlock::Warmup => WARMUP_SWEEPS,
            WorkBlock::Load => LOAD_BLOCK_SWEEPS,
        }
    }
    /// **[`RC021_LOAD_SEED`] appears here and nowhere else.** No sentinel, no
    /// measurement and no other block may reach for it.
    pub const fn seed(self) -> u64 {
        match self {
            WorkBlock::Warmup => RC021_SENTINEL_SEED,
            WorkBlock::Load => RC021_LOAD_SEED,
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            WorkBlock::Warmup => "warmup",
            WorkBlock::Load => "load",
        }
    }
}

/// What a block of unmeasured work reports: the count it actually ran and how
/// long it took.
///
/// The duration is here because §6 P7 checks that a phase-C load block exceeds
/// `1000 ×` the timer resolution. It is **not** a measurement: no block's
/// duration ever reaches a journal row or a `paired_spread`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorkBlockRun {
    pub sweeps: u32,
    pub wall_ms: f64,
}

/// Execute a block and discard everything it produced.
///
/// Like the sentinel, the operator's return value is dropped without
/// inspection: an outcome-blind instrument stays blind in its warmup too.
pub fn run_work_block(
    instance: &VerifiedSentinelInstance,
    block: WorkBlock,
) -> Result<WorkBlockRun, ProtocolError> {
    let temps = vec![SENTINEL_TEMP; SENTINEL_REPLICAS];
    let ir = &instance.0;
    let init = vec![0u8; ir.n];
    let mut st = SparseBitSlice::new(ir, SENTINEL_REPLICAS, &init)
        .map_err(|e| ProtocolError::StateInit(format!("{e:?}")))?;
    let v = RuntimeView {
        iteration: 0,
        temperatures: &temps,
        num_replicas: SENTINEL_REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    };
    let mut op = MetropolisSweep::new();
    let mut rng = ChaCha8Rng::seed_from_u64(block.seed());
    let t0 = Instant::now();
    let _ = op.apply(
        &mut st,
        &v,
        &mut rng,
        Budget {
            sweeps: block.sweeps(),
        },
    );
    Ok(WorkBlockRun {
        sweeps: block.sweeps(),
        wall_ms: t0.elapsed().as_secs_f64() * 1000.0,
    })
}

// ============================================= §1 / §6 PAIRED MEASUREMENT

/// §6 P8: `DEGENERATE_RESOLUTION_MULT`, inherited from
/// `exp_marginal_cost.rs:66` rather than newly chosen.
pub const RESOLUTION_MULT: f64 = 100.0;

/// Why a paired measurement is `LOST`. §10: a lost measurement is recorded as
/// `LOST`, never silently repaired, discarded, or re-executed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LostReason {
    /// A duration that is `NaN` or infinite.
    NonFiniteDuration,
    /// A duration of zero or less: no work can take no time.
    NonPositiveDuration,
    /// §6 P8: below `100 ×` the measured timer resolution.
    BelowResolutionFloor,
    /// The resolution itself is unusable. Fail closed — a non-finite or
    /// non-positive resolution makes the floor meaningless, and treating a
    /// meaningless floor as satisfied would turn every measurement into a
    /// vacuous pass.
    UnusableTimerResolution,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Measurement {
    Ok { spread: f64 },
    Lost(LostReason),
}

/// §1: `spread = max(t_first, t_last) / min(t_first, t_last) − 1`.
///
/// The `f64` is returned as computed. The journal codec serialises it in
/// shortest round-trip form, which is what keeps a `0.090000004` from becoming
/// a passing `0.09` (§C8.1).
pub fn paired_spread(first_ms: f64, last_ms: f64) -> Option<f64> {
    if !(first_ms.is_finite() && last_ms.is_finite()) || first_ms <= 0.0 || last_ms <= 0.0 {
        return None;
    }
    let hi = first_ms.max(last_ms);
    let lo = first_ms.min(last_ms);
    Some(hi / lo - 1.0)
}

/// The **per-measurement** decision, and only that.
///
/// §6 P8 also has a whole-phase clause — an entire phase below the floor is a
/// kill criterion — but that is a classification over a phase's rows and
/// belongs to the closure commit. Nothing here aggregates.
pub fn classify_measurement(first_ms: f64, last_ms: f64, timer_resolution_ms: f64) -> Measurement {
    if !timer_resolution_ms.is_finite() || timer_resolution_ms <= 0.0 {
        return Measurement::Lost(LostReason::UnusableTimerResolution);
    }
    for d in [first_ms, last_ms] {
        if !d.is_finite() {
            return Measurement::Lost(LostReason::NonFiniteDuration);
        }
        if d <= 0.0 {
            return Measurement::Lost(LostReason::NonPositiveDuration);
        }
    }
    // Equality with the floor passes: the contract is "at least 100 ×".
    let floor = RESOLUTION_MULT * timer_resolution_ms;
    if first_ms < floor || last_ms < floor {
        return Measurement::Lost(LostReason::BelowResolutionFloor);
    }
    match paired_spread(first_ms, last_ms) {
        Some(spread) if spread.is_finite() => Measurement::Ok { spread },
        _ => Measurement::Lost(LostReason::NonFiniteDuration),
    }
}

/// A verified-instance constructor for **tests in sibling modules only**.
///
/// `VerifiedSentinelInstance`'s field is private to this module, so a sibling
/// test cannot build one; without this it could not exercise the work blocks at
/// all except on G11, which no test may execute. It is `#[cfg(test)]`, so it
/// does not exist in any build the instrument actually runs.
#[cfg(test)]
pub mod tests_support {
    use super::*;

    pub fn synthetic_verified() -> VerifiedSentinelInstance {
        VerifiedSentinelInstance(
            ising_engine::engine_v2::frontend::rudy_maxcut_ir("4 4\n1 2 1\n2 3 1\n3 4 1\n4 1 1\n")
                .expect("synthetic rudy must parse"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------- seeds

    #[test]
    fn the_four_seeds_are_exactly_the_frozen_values() {
        assert_eq!(RC021_SENTINEL_SEED, 31001);
        assert_eq!(RC021_LOAD_SEED, 31002);
        assert_eq!(RC021_SYNTH_SEED, 31003);
        assert_eq!(RC021_ORDER_SEED, 31004);
        assert_eq!(RC021_BAND, (31001, 31099));
        assert_eq!(RC021_SEEDS.len(), 4);
    }

    #[test]
    fn disjointness_gate_passes_and_the_seeds_are_distinct_and_in_band() {
        assert_eq!(check_seed_disjointness(), Ok(()));
        for (_, s) in RC021_SEEDS {
            assert!(RC021_BAND.0 <= s && s <= RC021_BAND.1);
            assert_eq!(occupied_by(s), None, "{s} must be unoccupied");
        }
        let mut v: Vec<u64> = RC021_SEEDS.iter().map(|(_, s)| *s).collect();
        v.sort_unstable();
        v.dedup();
        assert_eq!(v.len(), 4, "pairwise distinct");
    }

    /// Every boundary of the frozen union, from both sides.
    #[test]
    fn occupancy_boundaries_are_exact() {
        let cases: [(u64, bool); 18] = [
            (990, false),
            (991, true),
            (992, false),
            (993, true),
            (994, false),
            (1000, false),
            (1001, true),
            (12008, true),
            (12009, false),
            (20000, false),
            (20001, true),
            (20002, false),
            (20260818, false),
            (20260819, true),
            (20262109, true),
            (20262110, false),
            (31001, false),
            (31099, false),
        ];
        for (seed, want) in cases {
            assert_eq!(occupied_by(seed).is_some(), want, "seed {seed}");
        }
    }

    /// The occupancy the gate uses must actually cover the call-site evidence,
    /// and its upper bound must come from the derived `+1000` range — not from
    /// any declared constant.
    #[test]
    fn derived_call_site_families_are_covered_and_set_the_upper_bound() {
        for f in DERIVED_FAMILIES {
            for probe in [f.lo, f.hi, (f.lo + f.hi) / 2] {
                assert!(
                    occupied_by(probe).is_some(),
                    "{} ({}) value {probe} escapes the gate",
                    f.name,
                    f.source
                );
            }
        }
        let derived_max = DERIVED_FAMILIES.iter().map(|f| f.hi).max().unwrap();
        assert_eq!(derived_max, 20262109);
        let offset = DERIVED_FAMILIES
            .iter()
            .find(|f| f.name == "permutation +1000 offset")
            .expect("the +1000 call-site family must be enumerated");
        assert_eq!((offset.lo, offset.hi), (20262101, 20262109));
        // Without it the declared bases would stop here, nearly 800 short.
        let declared_max = DERIVED_FAMILIES
            .iter()
            .filter(|f| f.name != "permutation +1000 offset")
            .map(|f| f.hi)
            .max()
            .unwrap();
        assert_eq!(declared_max, 20261309);
        assert!(occupied_by(20262105).is_some());
    }

    /// The gate is **called**, not re-implemented. RC-021's frozen band is
    /// disjoint from the occupancy by construction, so the collision branch is
    /// reachable only by injecting a band that overlaps it — which is exactly
    /// why the band is a parameter.
    #[test]
    fn the_gate_refuses_a_real_collision() {
        let wide = (0u64, u64::MAX);
        for occupied in [991u64, 993, 1001, 12008, 20001, 20260819, 20262109] {
            let family = occupied_by(occupied).expect("must be occupied").name;
            assert_eq!(
                validate_seed_set(&[("RC021_SENTINEL_SEED", occupied)], wide, &OCCUPANCY),
                Err(ProtocolError::SeedCollision {
                    name: "RC021_SENTINEL_SEED",
                    seed: occupied,
                    family,
                }),
                "seed {occupied} must be refused by the gate"
            );
        }
        // A seed just outside every family passes the same call, so the
        // refusals above are attributable to occupancy and nothing else.
        assert_eq!(
            validate_seed_set(&[("probe", 12009)], wide, &OCCUPANCY),
            Ok(())
        );
    }

    #[test]
    fn the_gate_refuses_an_out_of_band_seed_before_asking_about_occupancy() {
        for out in [31000u64, 31100, 0, u64::MAX] {
            assert_eq!(
                validate_seed_set(&[("RC021_LOAD_SEED", out)], RC021_BAND, &OCCUPANCY),
                Err(ProtocolError::SeedOutsideBand {
                    name: "RC021_LOAD_SEED",
                    seed: out,
                })
            );
        }
        // 20001 is occupied *and* out of the real band: band is checked first,
        // so the error names the band, not the family.
        assert_eq!(
            validate_seed_set(&[("probe", 20001)], RC021_BAND, &OCCUPANCY),
            Err(ProtocolError::SeedOutsideBand {
                name: "probe",
                seed: 20001
            })
        );
    }

    #[test]
    fn the_gate_refuses_duplicates() {
        assert_eq!(
            validate_seed_set(
                &[("RC021_SENTINEL_SEED", 31001), ("RC021_LOAD_SEED", 31001)],
                RC021_BAND,
                &OCCUPANCY,
            ),
            Err(ProtocolError::SeedNotDistinct {
                a: "RC021_SENTINEL_SEED",
                b: "RC021_LOAD_SEED",
                seed: 31001,
            })
        );
        // A duplicate anywhere in the set, not only adjacent entries.
        assert!(matches!(
            validate_seed_set(
                &[("a", 31001), ("b", 31002), ("c", 31003), ("d", 31002)],
                RC021_BAND,
                &OCCUPANCY,
            ),
            Err(ProtocolError::SeedNotDistinct { seed: 31002, .. })
        ));
        // and the production wrapper, over the frozen constants, passes
        assert_eq!(check_seed_disjointness(), Ok(()));
    }

    /// §4.2: six sessions, zero-based `0..=5`. Seven indices would be seven
    /// sessions.
    #[test]
    fn order_seed_is_bounded_to_the_six_zero_based_sessions() {
        assert_eq!(MAX_SESSION_INDEX, 5);
        assert_eq!(order_seed(0).unwrap(), 31004);
        assert_eq!(order_seed(5).unwrap(), 31009);

        let mut seen = Vec::new();
        for s in 0..=MAX_SESSION_INDEX {
            let v = order_seed(s).unwrap();
            assert!(RC021_BAND.0 <= v && v <= RC021_BAND.1);
            assert_eq!(occupied_by(v), None);
            seen.push(v);
        }
        assert_eq!(seen.len(), 6, "six sessions, not seven");
        seen.dedup();
        assert_eq!(seen.len(), 6, "one ordering seed per session");

        for bad in [6u64, 7, 100, u64::MAX] {
            assert_eq!(
                order_seed(bad),
                Err(ProtocolError::SessionIndexOutOfRange {
                    index: bad,
                    max: MAX_SESSION_INDEX,
                }),
                "index {bad} must be refused"
            );
        }
        // The unbounded form would wrap back into the band at the top of u64;
        // the bound is what stops it, so it is asserted rather than assumed.
        assert_eq!(
            RC021_ORDER_SEED.wrapping_add(u64::MAX),
            RC021_ORDER_SEED - 1
        );
        assert!(order_seed(u64::MAX).is_err());
    }

    // ---------------------------------------------------------- sentinel

    #[test]
    fn sentinel_spec_is_the_frozen_one() {
        assert_eq!(SENTINEL_OPERATOR, "metropolis_sweep");
        assert_eq!(SENTINEL_INSTANCE, "G11");
        assert_eq!(SENTINEL_TEMP, 0.5);
        assert_eq!(SENTINEL_WINDOW, 8);
        assert_eq!(SENTINEL_REPLICAS, 32);
        assert_eq!(SENTINEL_PREFIX_SWEEPS, 4);
        assert_eq!(G11_SHA256_PREFIX, "c2a760d2926d");
        assert_eq!(RESOLUTION_MULT, 100.0);
    }

    /// The frozen G11 bytes, read but **never executed**. Reading and parsing
    /// an instance is not a measurement; no sentinel is run on it here.
    fn g11_bytes() -> Vec<u8> {
        let p =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmark_suite/data/gset/G11");
        std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }

    /// A gate that only ever refuses proves nothing: it must be shown to
    /// **accept** the one instance the preregistration froze.
    #[test]
    fn the_frozen_g11_passes_verify_and_parse() {
        let bytes = g11_bytes();
        assert_eq!(&crate::host::sha256_hex(&bytes)[..12], G11_SHA256_PREFIX);
        assert_eq!(verify_sentinel_instance(&bytes), Ok(()));
        let inst = load_sentinel_instance(&bytes).expect("frozen G11 must load");
        assert_eq!(inst.num_vars(), 800, "G11 is the 800-node instance");
    }

    /// One byte changed anywhere must be refused — the property the gate rests
    /// on, checked at the front, the middle and the back of the real file.
    #[test]
    fn one_changed_g11_byte_is_refused() {
        let bytes = g11_bytes();
        let good = crate::host::sha256_hex(&bytes);
        for at in [0usize, bytes.len() / 2, bytes.len() - 1] {
            let mut tampered = bytes.clone();
            tampered[at] ^= 0x01;
            assert_ne!(tampered, bytes);
            let observed = crate::host::sha256_hex(&tampered);
            assert_ne!(observed, good, "byte {at} must change the digest");
            assert_eq!(
                verify_sentinel_instance(&tampered),
                Err(ProtocolError::InstanceHashMismatch {
                    expected: G11_SHA256_PREFIX,
                    observed,
                }),
                "a single flipped byte at {at} must be refused"
            );
        }
        // Truncation is equally refused.
        assert!(verify_sentinel_instance(&bytes[..bytes.len() - 1]).is_err());
        assert!(verify_sentinel_instance(&[]).is_err());
    }

    /// The gate runs BEFORE the parse: bytes that are not even valid rudy fail
    /// on the hash, never on a parser error.
    #[test]
    fn verification_precedes_parsing() {
        for malformed in [
            &b"not a rudy file at all"[..],
            &b""[..],
            &b"\xff\xfe not utf-8"[..],
            &b"3 2\n1 2 notanumber\n"[..],
        ] {
            assert!(
                matches!(
                    load_sentinel_instance(malformed),
                    Err(ProtocolError::InstanceHashMismatch { .. })
                ),
                "{malformed:?} must fail on the hash, not on the parser"
            );
        }
    }

    /// A tiny synthetic graph, so the sentinel is exercised without ever
    /// touching G11 or producing a timing datum that means anything.
    fn synthetic_ir() -> ProblemIR {
        ising_engine::engine_v2::frontend::rudy_maxcut_ir("4 4\n1 2 1\n2 3 1\n3 4 1\n4 1 1\n")
            .expect("synthetic rudy must parse")
    }

    #[test]
    fn sentinel_reports_only_wall_time_and_work() {
        let ir = synthetic_ir();
        let r = execute_sentinel(&ir, RC021_SENTINEL_SEED, ExtraWork::NONE).unwrap();
        assert_eq!(r.measured_sweeps, SENTINEL_WINDOW);
        assert_eq!(r.prefix_sweeps, SENTINEL_PREFIX_SWEEPS);
        assert_eq!(r.replicas, SENTINEL_REPLICAS);
        assert!(r.wall_ms.is_finite() && r.wall_ms >= 0.0);
        // The struct has exactly these four fields; there is no place to put an
        // energy, a cut, a digest or a quality. This is asserted by exhaustive
        // destructuring, which stops compiling the moment one is added.
        let SentinelRun {
            wall_ms: _,
            measured_sweeps: _,
            prefix_sweeps: _,
            replicas: _,
        } = r;
    }

    #[test]
    fn extra_work_is_an_explicitly_named_hook_only() {
        assert_eq!(ExtraWork::NONE.sweeps(), 0);
        assert_eq!(ExtraWork::for_p1_injection(24).sweeps(), 24);
        let ir = synthetic_ir();
        let plain = execute_sentinel(&ir, RC021_SENTINEL_SEED, ExtraWork::NONE).unwrap();
        let injected =
            execute_sentinel(&ir, RC021_SENTINEL_SEED, ExtraWork::for_p1_injection(24)).unwrap();
        // Only the accounted work differs; the hook cannot change anything else.
        assert_eq!(plain.measured_sweeps, 8);
        assert_eq!(injected.measured_sweeps, 32);
        assert_eq!(injected.prefix_sweeps, plain.prefix_sweeps);
        assert_eq!(injected.replicas, plain.replicas);
    }

    // ------------------------------------------------ paired measurement

    #[test]
    fn spread_is_max_over_min_minus_one() {
        assert_eq!(paired_spread(100.0, 100.0), Some(0.0));
        assert_eq!(paired_spread(100.0, 109.0).unwrap(), 0.09000000000000008);
        // symmetric in its arguments by construction
        assert_eq!(paired_spread(109.0, 100.0), paired_spread(100.0, 109.0));
        for bad in [
            (f64::NAN, 1.0),
            (1.0, f64::NAN),
            (f64::INFINITY, 1.0),
            (1.0, f64::NEG_INFINITY),
            (0.0, 1.0),
            (1.0, 0.0),
            (-1.0, 1.0),
        ] {
            assert_eq!(paired_spread(bad.0, bad.1), None, "{bad:?}");
        }
    }

    /// §C8.1: the value must survive the codec's shortest round-trip exactly,
    /// because a fixed-decimal form turns a failing `0.090000004` into a
    /// passing `0.09`.
    #[test]
    fn spread_survives_shortest_round_trip_unchanged() {
        for (a, b) in [(100.0, 109.0), (1.0, 1.0000001), (3.0, 7.0), (0.5, 0.5001)] {
            let v = paired_spread(a, b).unwrap();
            let back: f64 = v.to_string().parse().unwrap();
            assert_eq!(v.to_bits(), back.to_bits(), "{v}");
        }
        // The concrete flip the amendment records.
        let near = 0.090000004f64;
        assert_ne!(near.to_string(), "0.09");
        assert_eq!(format!("{near:.8}").parse::<f64>().unwrap(), 0.09);
    }

    /// §6 P8: at least `100 ×` the resolution. Equality passes.
    #[test]
    fn resolution_floor_is_inclusive_and_per_measurement() {
        let res = 0.001;
        let floor = RESOLUTION_MULT * res; // 0.1

        // exactly at the floor, both sides
        assert!(matches!(
            classify_measurement(floor, floor, res),
            Measurement::Ok { .. }
        ));
        // just below, on either side, is LOST — not a repair, not a re-run
        let under = floor - f64::EPSILON * floor;
        assert!(under < floor);
        for (a, b) in [(under, floor), (floor, under)] {
            assert_eq!(
                classify_measurement(a, b, res),
                Measurement::Lost(LostReason::BelowResolutionFloor)
            );
        }
        // comfortably above
        match classify_measurement(2.0, 2.18, res) {
            Measurement::Ok { spread } => assert!((spread - 0.09).abs() < 1e-12),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn degenerate_durations_are_lost_and_never_silently_repaired() {
        let res = 0.001;
        assert_eq!(
            classify_measurement(0.0, 1.0, res),
            Measurement::Lost(LostReason::NonPositiveDuration)
        );
        assert_eq!(
            classify_measurement(1.0, -1.0, res),
            Measurement::Lost(LostReason::NonPositiveDuration)
        );
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                classify_measurement(bad, 1.0, res),
                Measurement::Lost(LostReason::NonFiniteDuration),
                "{bad}"
            );
            assert_eq!(
                classify_measurement(1.0, bad, res),
                Measurement::Lost(LostReason::NonFiniteDuration),
                "{bad}"
            );
        }
    }

    /// Fail closed. A meaningless floor must not be read as a satisfied one.
    #[test]
    fn unusable_timer_resolution_fails_closed_rather_than_passing_vacuously() {
        for res in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                classify_measurement(1000.0, 1000.0, res),
                Measurement::Lost(LostReason::UnusableTimerResolution),
                "resolution {res} must not yield a pass"
            );
        }
        // With a usable resolution the very same durations pass, so the refusal
        // above is attributable to the resolution and nothing else.
        assert!(matches!(
            classify_measurement(1000.0, 1000.0, 0.001),
            Measurement::Ok { spread } if spread == 0.0
        ));
    }

    /// §3.4: extra work must never silently become *less* work. In release the
    /// unchecked `SENTINEL_WINDOW + extra` wraps.
    #[test]
    fn an_overflowing_sweep_count_is_refused_before_any_work() {
        let ir = synthetic_ir();
        for extra in [u32::MAX, u32::MAX - 7, u32::MAX - (SENTINEL_WINDOW - 1)] {
            assert_eq!(
                execute_sentinel(&ir, RC021_SENTINEL_SEED, ExtraWork::for_p1_injection(extra)),
                Err(ProtocolError::WorkOverflow {
                    window: SENTINEL_WINDOW,
                    extra,
                }),
                "extra {extra} must be refused"
            );
            // The wrap this prevents: it would have run *fewer* sweeps than the
            // plain window, so an injected slowdown would look like a speed-up.
            assert!(SENTINEL_WINDOW.wrapping_add(extra) < SENTINEL_WINDOW);
        }
        // The largest value that does fit is accepted as an accounted count,
        // so the refusal above is attributable to the overflow alone.
        let max_ok = u32::MAX - SENTINEL_WINDOW;
        assert_eq!(SENTINEL_WINDOW.checked_add(max_ok), Some(u32::MAX));
    }

    /// §3.4: the production sentinel takes a verified instance and **no seed**.
    ///
    /// Pinned by coercing `run_sentinel` to a function pointer of exactly that
    /// type: restoring a `seed` parameter, or widening the first argument back
    /// to a bare `ProblemIR`, stops this file compiling.
    ///
    /// A prose claim that some call "does not compile" would be worthless here
    /// — this is a child module, so it can see the private field and could
    /// build a `VerifiedSentinelInstance` itself. Only the signature binding
    /// below is load-bearing, and no sentinel is executed to obtain it.
    #[test]
    fn the_production_sentinel_takes_no_seed_and_no_unverified_instance() {
        let _: fn(&VerifiedSentinelInstance, ExtraWork) -> Result<SentinelRun, ProtocolError> =
            run_sentinel;

        // The only public constructor verifies the bytes before it parses them.
        let inst = load_sentinel_instance(&g11_bytes()).unwrap();
        assert_eq!(inst.num_vars(), 800);

        // The executor, which does take a seed and a bare IR, is private to
        // this module and is how the synthetic graph stays out of production.
        let _: fn(&ProblemIR, u64, ExtraWork) -> Result<SentinelRun, ProtocolError> =
            execute_sentinel;
        let a = execute_sentinel(&synthetic_ir(), RC021_SENTINEL_SEED, ExtraWork::NONE).unwrap();
        assert_eq!(a.measured_sweeps, SENTINEL_WINDOW);

        // No sentinel is executed on G11 here; the instance is only loaded.
    }
}
