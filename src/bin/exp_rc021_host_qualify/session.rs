//! RC-021 §4 — the host protocol: the frozen coordinates, the phase order, the
//! work that precedes each phase, the configuration check and the idle-gap
//! floor.
//!
//! Everything here is **pure** except the one durable record a terminal refusal
//! must leave behind. Nothing in this module measures, and nothing in it creates
//! a journal: the whole point of §C11.45 and §C11.5 is that these decisions are
//! reached *before* a session owns any file.

#![allow(dead_code)] // Consumers arrive in later commits of the §12 plan.

use crate::journal::Phase;
use crate::manifest::RunManifest;
use crate::protocol::{order_seed, ProtocolError, WorkBlock, MAX_SESSION_INDEX};
use std::io::Write;
use std::path::Path;

// ============================================================ §4.2 COUNTS

/// §4.2, frozen: six independent host sessions.
pub const SESSIONS: u8 = 6;
/// §4.1, frozen and **not randomised**: the phases are defined by cumulative
/// thermal and cache state, so permuting them would destroy what they name.
pub const PHASE_ORDER: [Phase; 3] = [Phase::A, Phase::B, Phase::C];
/// §C1: `block` is the phase-local pair index, `1..5`.
pub const BLOCKS_PER_PHASE: u8 = 5;
/// §4.2: 3 phases × 5 pairs.
pub const MEASUREMENTS_PER_SESSION: u32 = 15;
/// §4.2: 6 × 15. `measurement_index` is global, `1..90`.
pub const TOTAL_MEASUREMENTS: u32 = 90;

/// §4.1: the minimum idle interval before **every** session, including the
/// first — ten minutes.
pub const MIN_GAP_MS: u64 = 600_000;

/// One measurement's address. §C1: the triple `(session, phase, block)`
/// addresses it; `measurement_index` orders it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Coordinate {
    /// `1..6`. Session `0` is the control journal and never appears here.
    pub session: u8,
    pub phase: Phase,
    /// `1..5`, phase-local.
    pub block: u8,
    /// `1..90`, global and gapless.
    pub measurement_index: u32,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PlanError {
    /// Qualification sessions are numbered `1..=6`; `0` is the control journal.
    SessionOutOfRange {
        session: u8,
    },
    BlockOutOfRange {
        block: u8,
    },
    /// A session `2..6` whose predecessor left no `SESSION-CLOSE` row has no
    /// anchor to measure its ten minutes from.
    MissingPredecessorClose {
        session: u8,
    },
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::SessionOutOfRange { session } => {
                write!(f, "session {session} is outside 1..={SESSIONS}")
            }
            PlanError::BlockOutOfRange { block } => {
                write!(f, "block {block} is outside 1..={BLOCKS_PER_PHASE}")
            }
            PlanError::MissingPredecessorClose { session } => write!(
                f,
                "session {session} has no SESSION-CLOSE row for session {}",
                session.saturating_sub(1)
            ),
        }
    }
}

/// §4.1: phases in their frozen positions. `A` is first because "cold" is
/// defined by the absence of prior work.
pub fn phase_ordinal(phase: Phase) -> u32 {
    match phase {
        Phase::A => 0,
        Phase::B => 1,
        Phase::C => 2,
    }
}

/// The global index of one measurement. Derived from the frozen counts alone,
/// so it cannot drift from them.
pub fn measurement_index(session: u8, phase: Phase, block: u8) -> Result<u32, PlanError> {
    if session == 0 || session > SESSIONS {
        return Err(PlanError::SessionOutOfRange { session });
    }
    if block == 0 || block > BLOCKS_PER_PHASE {
        return Err(PlanError::BlockOutOfRange { block });
    }
    let session_base = (session as u32 - 1) * MEASUREMENTS_PER_SESSION;
    let phase_base = phase_ordinal(phase) * BLOCKS_PER_PHASE as u32;
    Ok(session_base + phase_base + block as u32)
}

/// One session's fifteen measurements, in the order they are taken.
pub fn plan_session(session: u8) -> Result<Vec<Coordinate>, PlanError> {
    if session == 0 || session > SESSIONS {
        return Err(PlanError::SessionOutOfRange { session });
    }
    let mut out = Vec::with_capacity(MEASUREMENTS_PER_SESSION as usize);
    for phase in PHASE_ORDER {
        for block in 1..=BLOCKS_PER_PHASE {
            out.push(Coordinate {
                session,
                phase,
                block,
                measurement_index: measurement_index(session, phase, block)?,
            });
        }
    }
    Ok(out)
}

/// The whole run: all ninety, in order.
pub fn plan_run() -> Vec<Coordinate> {
    let mut out = Vec::with_capacity(TOTAL_MEASUREMENTS as usize);
    for session in 1..=SESSIONS {
        // `plan_session` cannot fail for a session in range.
        if let Ok(mut s) = plan_session(session) {
            out.append(&mut s);
        }
    }
    out
}

/// §4.1: the unmeasured work that precedes a measurement, if any.
///
/// - **A** — nothing. "Cold" is the absence of prior work; a warmup here would
///   erase the phase.
/// - **B** — the warmup runs **once**, before the phase's five pairs.
/// - **C** — the load block runs immediately before **each** of the five pairs.
pub fn work_before(phase: Phase, block: u8) -> Result<Option<WorkBlock>, PlanError> {
    if block == 0 || block > BLOCKS_PER_PHASE {
        return Err(PlanError::BlockOutOfRange { block });
    }
    Ok(match phase {
        Phase::A => None,
        Phase::B if block == 1 => Some(WorkBlock::Warmup),
        Phase::B => None,
        Phase::C => Some(WorkBlock::Load),
    })
}

/// §4.1: ordering **within** a phase uses `RC021_ORDER_SEED + session_index`.
///
/// The journal numbers sessions `1..6` (§C8.2 field 7, where `0` is the control
/// journal) while the seed is zero-based `0..5`, so the two are converted here
/// rather than at each call site.
pub fn session_order_seed(session: u8) -> Result<u64, ProtocolError> {
    if session == 0 || session > SESSIONS {
        return Err(ProtocolError::SessionIndexOutOfRange {
            index: session as u64,
            max: MAX_SESSION_INDEX,
        });
    }
    order_seed(session as u64 - 1)
}

// ============================================ §C11.45 CONFIGURATION CHECK

/// What the two measuring modes compare against the manifest, and the **only**
/// thing they do about affinity.
///
/// §C11.45: *the instrument never sets affinity and never chooses a CPU set.*
/// There is no setter in this module, and this type carries no method that
/// could become one — it is read from the host and compared, never applied.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ObservedConfig {
    pub cpus_allowed_list: String,
    pub thread_count: u32,
    pub host_fingerprint: String,
    pub repo_commit: String,
}

/// Which of the four fields disagreed. Named individually so a refusal says
/// what changed under a protocol whose premise is that nothing did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConfigMismatch {
    pub field: &'static str,
    pub expected: String,
    pub observed: String,
}

impl std::fmt::Display for ConfigMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: manifest {}, observed {}",
            self.field, self.expected, self.observed
        )
    }
}

/// §C11.45: an **exact match** of `Cpus_allowed_list`, `thread_count`,
/// `host_fingerprint` and `repo_commit`. Pure comparison — no process control.
pub fn check_configuration(
    observed: &ObservedConfig,
    manifest: &RunManifest,
) -> Result<(), ConfigMismatch> {
    let pairs: [(&'static str, &str, &str); 3] = [
        (
            "cpus_allowed_list",
            &manifest.host_fields.cpus_allowed_list,
            &observed.cpus_allowed_list,
        ),
        (
            "host_fingerprint",
            &manifest.host_fingerprint,
            &observed.host_fingerprint,
        ),
        ("repo_commit", &manifest.repo_commit, &observed.repo_commit),
    ];
    for (field, expected, obs) in pairs {
        if expected != obs {
            return Err(ConfigMismatch {
                field,
                expected: expected.to_string(),
                observed: obs.to_string(),
            });
        }
    }
    if manifest.thread_count != observed.thread_count {
        return Err(ConfigMismatch {
            field: "thread_count",
            expected: manifest.thread_count.to_string(),
            observed: observed.thread_count.to_string(),
        });
    }
    Ok(())
}

// ================================================== §C11.5 THE IDLE GAP

/// Where a session's ten minutes are measured from.
///
/// §C11.5, frozen: session 1 measures from the `monotonic_offset_ms` recorded
/// in `controls_complete.json`; sessions 2 through 6 measure from the previous
/// session's `SESSION-CLOSE` row. The floor therefore covers six intervals, not
/// five — without the first, session 1 would be the only session that began on
/// a host which had just done seconds of sustained control work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GapAnchor {
    ControlsComplete {
        monotonic_offset_ms: u64,
    },
    PreviousSessionClose {
        session: u8,
        monotonic_offset_ms: u64,
    },
}

impl GapAnchor {
    pub fn offset_ms(self) -> u64 {
        match self {
            GapAnchor::ControlsComplete {
                monotonic_offset_ms,
            } => monotonic_offset_ms,
            GapAnchor::PreviousSessionClose {
                monotonic_offset_ms,
                ..
            } => monotonic_offset_ms,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GapVerdict {
    /// At or above the floor. Equality passes: the contract is "at least".
    Sufficient { gap_ms: u64 },
    /// Below the floor. **The only correctable operational refusal after
    /// `controls_complete`** — nothing about the host is wrong, the operator
    /// simply has to wait.
    Short { gap_ms: u64, required_ms: u64 },
    /// The anchor lies in the future on the shared uptime axis. This is not a
    /// short gap and is not corrected by waiting; the axis itself is broken.
    ClockWentBackwards { anchor_ms: u64, now_ms: u64 },
}

/// §C11.5. `now_ms` is the current `monotonic_offset_ms`; both values are on
/// the boot-shared uptime axis, so they are comparable across processes.
pub fn check_gap(anchor: GapAnchor, now_ms: u64) -> GapVerdict {
    let anchor_ms = anchor.offset_ms();
    match now_ms.checked_sub(anchor_ms) {
        None => GapVerdict::ClockWentBackwards { anchor_ms, now_ms },
        Some(gap_ms) if gap_ms >= MIN_GAP_MS => GapVerdict::Sufficient { gap_ms },
        Some(gap_ms) => GapVerdict::Short {
            gap_ms,
            required_ms: MIN_GAP_MS,
        },
    }
}

/// The anchor a given session must use, from the two recorded offsets.
pub fn anchor_for(
    session: u8,
    controls_complete_offset_ms: u64,
    previous_close_offset_ms: Option<u64>,
) -> Result<GapAnchor, PlanError> {
    if session == 0 || session > SESSIONS {
        return Err(PlanError::SessionOutOfRange { session });
    }
    if session == 1 {
        return Ok(GapAnchor::ControlsComplete {
            monotonic_offset_ms: controls_complete_offset_ms,
        });
    }
    match previous_close_offset_ms {
        Some(monotonic_offset_ms) => Ok(GapAnchor::PreviousSessionClose {
            session: session - 1,
            monotonic_offset_ms,
        }),
        // That is a missing predecessor, not a gap question, and it must not
        // silently fall back to the controls anchor: measuring session 6 from
        // `controls_complete` would compare its ten minutes against a point
        // five sessions in the past and pass every time.
        None => Err(PlanError::MissingPredecessorClose { session }),
    }
}

// ======================================== §C11.45 / §C12.3 THE PRE-JOURNAL GATE

/// What the host looks like at the moment a session is about to start, and what
/// the run has already recorded. Every field is injected, so the gate is pure.
pub struct SessionPreflight<'a> {
    /// `1..6`. The anchor is **derived** from this, never supplied beside it.
    pub session: u8,
    pub manifest: &'a RunManifest,
    pub observed: ObservedConfig,
    /// Current `boot_id`, for the §C12 axis check.
    pub observed_boot_id: String,
    /// Current `monotonic_offset_ms`.
    pub now_offset_ms: u64,
    /// §C11.5: the `monotonic_offset_ms` recorded in `controls_complete.json`.
    pub controls_complete_offset_ms: u64,
    /// §C11.5: the `monotonic_offset_ms` of the previous session's
    /// `SESSION-CLOSE` row, when there is a previous session.
    pub previous_close_offset_ms: Option<u64>,
    /// §C12.3 / §C10.1: what `run_invalid.json` on disk actually is. A bare
    /// "present" cannot distinguish the record that forbids further sessions
    /// from a corrupt file that is itself `JOURNAL-INVALID`.
    pub run_invalid: RunInvalidState,
}

/// The outcome of the gate. **No file has been created when any of these is
/// returned** — §C11.45: a mode that has already created a file cannot
/// un-create it, so the check precedes creation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SessionOutcome {
    Proceed {
        gap_ms: u64,
        anchor: GapAnchor,
        order_seed: u64,
    },
    /// Exit 2, nothing written, **and the operator may simply wait and retry**.
    /// §C12.3: the only non-terminal exit 2 that writes no `run_invalid.json`.
    ShortGap { gap_ms: u64, required_ms: u64 },
    /// A host or configuration mismatch. `run_invalid.json` **must now be
    /// written**; `INSTRUMENT-INVALID`, exit 3.
    ///
    /// A session runs only after `controls_complete`, so its mismatch is always
    /// post-marker and always terminal. §C12.3's correctable
    /// `REFUSED-BEFORE-MEASUREMENT` belongs to the **pre-marker controls gate**,
    /// which is a later commit and a separate function — a `controls_started`
    /// flag here would only admit a state a session cannot be in.
    Terminal { reason: String },
    /// §C12.3: a **valid** `run_invalid.json` is already present. Exit 3, and
    /// — unlike [`SessionOutcome::Terminal`] — **nothing is written**.
    ///
    /// The record is immutable, so a second `create_new` would fail and turn an
    /// already-recorded `INSTRUMENT-INVALID` into a spurious `JOURNAL-INVALID`
    /// at exit 4. The two cases are therefore distinct outcomes, not one.
    AlreadyInvalid,
    /// §C10.1: `run_invalid.json` exists but is empty, partial or corrupt.
    /// `JOURNAL-INVALID`, exit 4, **nothing written**.
    ///
    /// It is not `AlreadyInvalid`: that outcome asserts a readable record of
    /// why the run stopped, and this file is not one. It is not `Terminal`
    /// either — writing a second record over a damaged one is exactly what
    /// §C12.4 forbids.
    DamagedRunInvalid { why: String },
    /// The previous session left no `SESSION-CLOSE` row, so this session has no
    /// anchor to measure its ten minutes from. Exit 2, **nothing written**.
    ///
    /// This is a durable *session state* — §5.2 classifies such a predecessor
    /// `ABORTED`, or `NOT STARTED` when its journal is absent — and that
    /// evidence already stands in the journals. Writing `run_invalid.json` here
    /// would convert a session-level fact into a Class I `INSTRUMENT-INVALID`
    /// about the host, which is a different and stronger claim. The run's class
    /// is derived at finalize from the journals; nothing here decides it.
    PredecessorNotClosed { session: u8, predecessor: u8 },
    /// A precondition of the invocation is wrong — a session number outside
    /// `1..=6`. Exit 2, **nothing written**.
    ///
    /// Nothing about the host has changed, so this is not a host mismatch and
    /// must not be recorded as one. Correcting the request and retrying is
    /// permitted.
    InvalidRequest { reason: String },
}

impl SessionOutcome {
    /// §C12.3's exit codes, which are part of the frozen semantics.
    pub fn exit_code(&self) -> i32 {
        match self {
            SessionOutcome::Proceed { .. } => 0,
            SessionOutcome::ShortGap { .. }
            | SessionOutcome::PredecessorNotClosed { .. }
            | SessionOutcome::InvalidRequest { .. } => 2,
            SessionOutcome::Terminal { .. } | SessionOutcome::AlreadyInvalid => 3,
            SessionOutcome::DamagedRunInvalid { .. } => 4,
        }
    }
    /// Whether this outcome requires a durable `run_invalid.json` (§C12.3).
    /// `AlreadyInvalid` is deliberately `false`: the record exists already.
    pub fn writes_run_invalid(&self) -> bool {
        matches!(self, SessionOutcome::Terminal { .. })
    }
    pub fn may_retry_after_waiting(&self) -> bool {
        matches!(self, SessionOutcome::ShortGap { .. })
    }
}

/// The gate, in its frozen order.
///
/// Terminal conditions are tested **before** the gap, and that ordering is
/// forced rather than chosen: §C11.45 states that the short gap is the *only*
/// correctable operational refusal and that every host or configuration
/// mismatch is terminal. Testing the gap first could return a correctable
/// `ShortGap` on a host that had already changed underneath the run, hiding a
/// terminal fact behind an invitation to wait and retry.
pub fn session_preflight(p: &SessionPreflight<'_>) -> SessionOutcome {
    // §C12.3: once `run_invalid.json` exists it forbids all further sessions —
    // and it already *is* the record, so nothing more is written.
    match &p.run_invalid {
        RunInvalidState::Valid(r) => {
            // A canonical record still has to be *this run's* record. One left
            // by another run parses perfectly and would otherwise be read as
            // terminal evidence about a run it says nothing about.
            return match bind_run_invalid(r, p.manifest) {
                Ok(()) => SessionOutcome::AlreadyInvalid,
                Err(why) => SessionOutcome::DamagedRunInvalid { why },
            };
        }
        // §C10.1: a corrupt record is JOURNAL-INVALID, not a record.
        RunInvalidState::Damaged(why) => {
            return SessionOutcome::DamagedRunInvalid { why: why.clone() }
        }
        RunInvalidState::Absent => {}
    }
    let terminal = |reason: String| SessionOutcome::Terminal { reason };
    // §C11.45: the exact-match configuration and provenance check, before any
    // work and before this mode creates any file of its own.
    if let Err(m) = check_configuration(&p.observed, p.manifest) {
        return terminal(format!("configuration mismatch: {m}"));
    }
    // §C12: a changed boot_id, or an uptime below the run's start, breaks the
    // monotonic axis and is detected before any new journal is created.
    if p.observed_boot_id != p.manifest.boot_id {
        return terminal(format!(
            "boot_id mismatch: manifest {}, observed {}",
            p.manifest.boot_id, p.observed_boot_id
        ));
    }
    // §C11.5: the anchor is **derived from the session number**, never accepted
    // beside it. Taking a caller's anchor would let session 6 measure its ten
    // minutes from `controls_complete` — a point five sessions in the past,
    // which passes unconditionally.
    //
    // Neither failure of `anchor_for` is a host mismatch, so neither writes
    // `run_invalid.json`: one is a durable session state, the other a bad
    // request.
    let anchor = match anchor_for(
        p.session,
        p.controls_complete_offset_ms,
        p.previous_close_offset_ms,
    ) {
        Ok(a) => a,
        Err(PlanError::MissingPredecessorClose { session }) => {
            return SessionOutcome::PredecessorNotClosed {
                session,
                predecessor: session - 1,
            }
        }
        Err(e) => {
            return SessionOutcome::InvalidRequest {
                reason: e.to_string(),
            }
        }
    };
    let order_seed = match session_order_seed(p.session) {
        Ok(s) => s,
        Err(e) => {
            return SessionOutcome::InvalidRequest {
                reason: e.to_string(),
            }
        }
    };
    match check_gap(anchor, p.now_offset_ms) {
        GapVerdict::ClockWentBackwards { anchor_ms, now_ms } => terminal(format!(
            "uptime went backwards: anchor {anchor_ms} ms, now {now_ms} ms"
        )),
        GapVerdict::Short {
            gap_ms,
            required_ms,
        } => SessionOutcome::ShortGap {
            gap_ms,
            required_ms,
        },
        GapVerdict::Sufficient { gap_ms } => SessionOutcome::Proceed {
            gap_ms,
            anchor,
            order_seed,
        },
    }
}

// ===================================================== §C12.3 run_invalid.json

pub const RUN_INVALID_FILE: &str = "run_invalid.json";

/// §C12.3 / §C10.1: what the reserved path actually holds.
///
/// A boolean cannot carry this distinction, and the distinction changes the
/// exit code: a readable record forbids further sessions at exit 3, while a
/// damaged one is `JOURNAL-INVALID` at exit 4.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RunInvalidState {
    Absent,
    /// Parsed and canonical. The record is carried, not merely acknowledged:
    /// a well-formed record of a *different run* is not this run's evidence,
    /// and only the record itself can say which run it belongs to.
    Valid(Box<RunInvalid>),
    /// Present but empty, partial or corrupt, with the reason it failed.
    Damaged(String),
}

/// Does this record belong to **this** run?
///
/// Only the three *expected* fields are bound. The `observed` boot id and
/// fingerprint describe the mismatch the record exists to document, so they
/// legitimately differ from the manifest — comparing them would reject exactly
/// the records the field was created to hold.
///
/// A mismatch is `DamagedRunInvalid`, exit 4, never `AlreadyInvalid`: a record
/// about another run is not this run's terminal evidence, and treating it as
/// one would end this run at exit 3 on someone else's finding.
pub fn bind_run_invalid(r: &RunInvalid, manifest: &RunManifest) -> Result<(), String> {
    for (field, expected, found) in [
        ("run_uuid", &manifest.run_uuid, &r.run_uuid),
        ("boot_id_expected", &manifest.boot_id, &r.boot_id_expected),
        (
            "host_fingerprint_expected",
            &manifest.host_fingerprint,
            &r.host_fingerprint_expected,
        ),
    ] {
        if expected != found {
            return Err(format!(
                "run_invalid.json belongs to another run: {field} is {found}, manifest has {expected}"
            ));
        }
    }
    Ok(())
}

/// Classify the on-disk `run_invalid.json`, reading nothing else.
///
/// Only `NotFound` means absent — a path that exists but cannot be read is a
/// fact about the run, never "nothing is there".
pub fn classify_run_invalid(dir: &Path) -> RunInvalidState {
    let path = dir.join(RUN_INVALID_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return RunInvalidState::Absent,
        Err(e) => return RunInvalidState::Damaged(e.to_string()),
    };
    match RunInvalid::parse(&text) {
        Ok(r) => RunInvalidState::Valid(Box::new(r)),
        Err(e) => RunInvalidState::Damaged(e.to_string()),
    }
}

/// §C12.3, in this exact order. `monotonic_offset_ms` is `null` when the clock
/// itself is untrustworthy.
pub const RUN_INVALID_KEYS: [&str; 10] = [
    "schema_version",
    "run_uuid",
    "boot_id_observed",
    "boot_id_expected",
    "host_fingerprint_observed",
    "host_fingerprint_expected",
    "reason",
    "command_line",
    "monotonic_offset_ms",
    "utc",
];

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RunInvalid {
    pub schema_version: String,
    pub run_uuid: String,
    pub boot_id_observed: String,
    pub boot_id_expected: String,
    pub host_fingerprint_observed: String,
    pub host_fingerprint_expected: String,
    pub reason: String,
    pub command_line: Vec<String>,
    pub monotonic_offset_ms: Option<u64>,
    pub utc: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RunInvalidError {
    /// §C12.4: a failure of `create_new`, `write_all`, `flush` or `sync_all` is
    /// `JOURNAL-INVALID`, exit 4. Execution stops; no partial file is repaired.
    Write(String),
    AlreadyPresent,
    /// The record is malformed. Refused **before** `create_new`, so it never
    /// reserves the path: §C10.1 would classify a corrupt `run_invalid.json` as
    /// `JOURNAL-INVALID`, and the instrument must not be the thing that creates
    /// one while recording why it is stopping.
    Invalid(&'static str),
}

impl std::fmt::Display for RunInvalidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunInvalidError::Write(e) => write!(f, "run_invalid.json could not be written: {e}"),
            RunInvalidError::AlreadyPresent => write!(f, "run_invalid.json already exists"),
            RunInvalidError::Invalid(what) => write!(f, "run_invalid.json is malformed: {what}"),
        }
    }
}

impl RunInvalidError {
    /// §C12.4: a durable-write failure is `JOURNAL-INVALID`, exit 4.
    ///
    /// A refused malformed record is exit 4 too: the instrument could not
    /// durably record why it is stopping, which §C12.4 says must stop it.
    pub fn exit_code(&self) -> i32 {
        4
    }
}

fn lower_hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_plain_text(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(|c| c.is_control())
}

impl RunInvalid {
    /// Every domain the §C12.3 record has, checked **before** anything is
    /// created. The instrument records why it is stopping; it must not do so in
    /// a file that is itself invalid.
    pub fn validate(&self) -> Result<(), RunInvalidError> {
        if self.schema_version != crate::journal::SCHEMA_VERSION {
            return Err(RunInvalidError::Invalid("schema_version"));
        }
        if !crate::host::is_run_uuid(&self.run_uuid) {
            return Err(RunInvalidError::Invalid("run_uuid"));
        }
        if !lower_hex(&self.host_fingerprint_observed, 64) {
            return Err(RunInvalidError::Invalid("host_fingerprint_observed"));
        }
        if !lower_hex(&self.host_fingerprint_expected, 64) {
            return Err(RunInvalidError::Invalid("host_fingerprint_expected"));
        }
        if !is_plain_text(&self.boot_id_observed) {
            return Err(RunInvalidError::Invalid("boot_id_observed"));
        }
        if !is_plain_text(&self.boot_id_expected) {
            return Err(RunInvalidError::Invalid("boot_id_expected"));
        }
        // A record whose `reason` is empty documents nothing.
        if !is_plain_text(&self.reason) {
            return Err(RunInvalidError::Invalid("reason"));
        }
        if self.command_line.is_empty() {
            return Err(RunInvalidError::Invalid("command_line"));
        }
        if !self.command_line.iter().all(|a| !a.contains('\u{0}')) {
            return Err(RunInvalidError::Invalid("command_line"));
        }
        if chrono::DateTime::parse_from_rfc3339(&self.utc).is_err() {
            return Err(RunInvalidError::Invalid("utc"));
        }
        Ok(())
    }

    /// Read a record back. Strict: every field must be present with its
    /// declared type, every domain must hold, and the text must be **exactly**
    /// what [`RunInvalid::render`] produces — which is what makes key order,
    /// spacing, escaping and the single trailing LF part of the check.
    pub fn parse(text: &str) -> Result<RunInvalid, RunInvalidError> {
        let v: serde_json::Value =
            serde_json::from_str(text).map_err(|_| RunInvalidError::Invalid("not JSON"))?;
        let o = v
            .as_object()
            .ok_or(RunInvalidError::Invalid("not a JSON object"))?;
        if o.len() != RUN_INVALID_KEYS.len() {
            return Err(RunInvalidError::Invalid("key count"));
        }
        let s = |k: &'static str| -> Result<String, RunInvalidError> {
            o.get(k)
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
                .ok_or(RunInvalidError::Invalid(k))
        };
        let command_line = o
            .get("command_line")
            .and_then(|x| x.as_array())
            .ok_or(RunInvalidError::Invalid("command_line"))?
            .iter()
            .map(|x| {
                x.as_str()
                    .map(|y| y.to_string())
                    .ok_or(RunInvalidError::Invalid("command_line"))
            })
            .collect::<Result<Vec<String>, RunInvalidError>>()?;
        let raw_offset = o
            .get("monotonic_offset_ms")
            .ok_or(RunInvalidError::Invalid("monotonic_offset_ms"))?;
        let monotonic_offset_ms = if raw_offset.is_null() {
            None
        } else {
            Some(
                raw_offset
                    .as_u64()
                    .ok_or(RunInvalidError::Invalid("monotonic_offset_ms"))?,
            )
        };
        let r = RunInvalid {
            schema_version: s("schema_version")?,
            run_uuid: s("run_uuid")?,
            boot_id_observed: s("boot_id_observed")?,
            boot_id_expected: s("boot_id_expected")?,
            host_fingerprint_observed: s("host_fingerprint_observed")?,
            host_fingerprint_expected: s("host_fingerprint_expected")?,
            reason: s("reason")?,
            command_line,
            monotonic_offset_ms,
            utc: s("utc")?,
        };
        r.validate()?;
        if r.render() != text {
            return Err(RunInvalidError::Invalid("not in canonical form"));
        }
        Ok(r)
    }

    /// Compact JSON, keys in the §C12.3 order, one trailing LF.
    pub fn render(&self) -> String {
        let mut o = String::from("{");
        let mut push = |k: &str, v: String, first: bool| {
            if !first {
                o.push(',');
            }
            o.push_str(&serde_json::to_string(k).unwrap_or_default());
            o.push(':');
            o.push_str(&v);
        };
        let s = |v: &str| serde_json::to_string(v).unwrap_or_default();
        push("schema_version", s(&self.schema_version), true);
        push("run_uuid", s(&self.run_uuid), false);
        push("boot_id_observed", s(&self.boot_id_observed), false);
        push("boot_id_expected", s(&self.boot_id_expected), false);
        push(
            "host_fingerprint_observed",
            s(&self.host_fingerprint_observed),
            false,
        );
        push(
            "host_fingerprint_expected",
            s(&self.host_fingerprint_expected),
            false,
        );
        push("reason", s(&self.reason), false);
        push(
            "command_line",
            serde_json::to_string(&self.command_line).unwrap_or_else(|_| "[]".to_string()),
            false,
        );
        push(
            "monotonic_offset_ms",
            match self.monotonic_offset_ms {
                Some(v) => v.to_string(),
                None => "null".to_string(),
            },
            false,
        );
        push("utc", s(&self.utc), false);
        o.push_str("}\n");
        o
    }

    /// §C12.3: written `create_new` and fsynced, and **immutable** thereafter.
    /// `create_new` is what makes a second one impossible.
    pub fn write(&self, dir: &Path) -> Result<std::path::PathBuf, RunInvalidError> {
        // Before `create_new`: a malformed record must not reserve the path,
        // because the reservation is what makes the file immutable and a second
        // attempt impossible.
        self.validate()?;
        let path = dir.join(RUN_INVALID_FILE);
        let body = self.render();
        let mut f = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(RunInvalidError::AlreadyPresent)
            }
            Err(e) => return Err(RunInvalidError::Write(e.to_string())),
        };
        f.write_all(body.as_bytes())
            .and_then(|()| f.flush())
            .and_then(|()| f.sync_all())
            .map_err(|e| RunInvalidError::Write(e.to_string()))?;
        crate::manifest::fsync_dir_at(dir).map_err(|e| RunInvalidError::Write(e.to_string()))?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{DiagProbe, HostFields};
    use crate::journal::CPU_TIME_UNIT;
    use crate::protocol::{
        run_work_block, LOAD_BLOCK_SWEEPS, RC021_LOAD_SEED, RC021_SENTINEL_SEED, WARMUP_SWEEPS,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempDir(std::path::PathBuf);
    impl TempDir {
        fn new(name: &str) -> TempDir {
            let p = std::env::temp_dir().join(format!(
                "rc021_session_{}_{}_{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::SeqCst),
                name
            ));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // ------------------------------------------------- §4.2 the 90 coordinates

    #[test]
    fn the_frozen_counts_are_the_preregistered_ones() {
        assert_eq!(SESSIONS, 6);
        assert_eq!(PHASE_ORDER, [Phase::A, Phase::B, Phase::C]);
        assert_eq!(BLOCKS_PER_PHASE, 5);
        assert_eq!(MEASUREMENTS_PER_SESSION, 15);
        assert_eq!(TOTAL_MEASUREMENTS, 90);
        assert_eq!(MIN_GAP_MS, 600_000);
        assert_eq!(
            SESSIONS as u32 * PHASE_ORDER.len() as u32 * BLOCKS_PER_PHASE as u32,
            TOTAL_MEASUREMENTS
        );
    }

    /// Exactly ninety coordinates, each appearing once, indices `1..=90`
    /// ascending and gapless, phases always A → B → C, blocks always `1..5`.
    #[test]
    fn the_plan_is_exactly_ninety_coordinates_in_the_frozen_order() {
        let plan = plan_run();
        assert_eq!(plan.len(), 90);

        let indices: Vec<u32> = plan.iter().map(|c| c.measurement_index).collect();
        assert_eq!(
            indices,
            (1..=90).collect::<Vec<u32>>(),
            "global and gapless"
        );

        let mut triples: Vec<(u8, u32, u8)> = plan
            .iter()
            .map(|c| (c.session, phase_ordinal(c.phase), c.block))
            .collect();
        let ordered = triples.clone();
        triples.sort_unstable();
        triples.dedup();
        assert_eq!(triples.len(), 90, "every coordinate distinct");
        assert_eq!(ordered, triples, "already in (session, phase, block) order");

        for c in &plan {
            assert!((1..=SESSIONS).contains(&c.session));
            assert!((1..=BLOCKS_PER_PHASE).contains(&c.block));
        }
        // Each session contributes fifteen, each phase five.
        for s in 1..=SESSIONS {
            let of_session: Vec<&Coordinate> = plan.iter().filter(|c| c.session == s).collect();
            assert_eq!(of_session.len(), 15);
            let phases: Vec<Phase> = of_session.iter().map(|c| c.phase).collect();
            assert_eq!(
                phases,
                [
                    [Phase::A; 5].as_slice(),
                    [Phase::B; 5].as_slice(),
                    [Phase::C; 5].as_slice()
                ]
                .concat(),
                "A then B then C, never permuted"
            );
            let blocks: Vec<u8> = of_session.iter().map(|c| c.block).collect();
            assert_eq!(blocks, [1, 2, 3, 4, 5].repeat(3));
        }
    }

    #[test]
    fn measurement_index_is_derived_and_bounded() {
        assert_eq!(measurement_index(1, Phase::A, 1).unwrap(), 1);
        assert_eq!(measurement_index(1, Phase::C, 5).unwrap(), 15);
        assert_eq!(measurement_index(2, Phase::A, 1).unwrap(), 16);
        assert_eq!(measurement_index(6, Phase::C, 5).unwrap(), 90);
        for bad in [0u8, 7, 255] {
            assert_eq!(
                measurement_index(bad, Phase::A, 1),
                Err(PlanError::SessionOutOfRange { session: bad })
            );
            assert_eq!(
                plan_session(bad),
                Err(PlanError::SessionOutOfRange { session: bad })
            );
        }
        for bad in [0u8, 6, 255] {
            assert_eq!(
                measurement_index(1, Phase::A, bad),
                Err(PlanError::BlockOutOfRange { block: bad })
            );
        }
    }

    // ------------------------------------------- §4.1 the unmeasured work

    #[test]
    fn phase_a_is_cold_b_warms_once_and_c_loads_before_every_pair() {
        for b in 1..=BLOCKS_PER_PHASE {
            assert_eq!(work_before(Phase::A, b).unwrap(), None, "A is cold at {b}");
        }
        assert_eq!(work_before(Phase::B, 1).unwrap(), Some(WorkBlock::Warmup));
        for b in 2..=BLOCKS_PER_PHASE {
            assert_eq!(work_before(Phase::B, b).unwrap(), None, "warmup runs once");
        }
        for b in 1..=BLOCKS_PER_PHASE {
            assert_eq!(
                work_before(Phase::C, b).unwrap(),
                Some(WorkBlock::Load),
                "the load block precedes every phase-C pair"
            );
        }
        assert_eq!(
            work_before(Phase::A, 6),
            Err(PlanError::BlockOutOfRange { block: 6 })
        );
    }

    /// §4.1's own arithmetic: `32 × 8 = 256` and `256 × 8 = 2048`, and the load
    /// block is exactly eight times the warmup.
    #[test]
    fn the_work_blocks_carry_the_frozen_counts_and_seeds() {
        assert_eq!(WARMUP_SWEEPS, 32 * 8);
        assert_eq!(LOAD_BLOCK_SWEEPS, 256 * 8);
        assert_eq!(LOAD_BLOCK_SWEEPS, 8 * WARMUP_SWEEPS);
        assert_eq!(WorkBlock::Warmup.seed(), RC021_SENTINEL_SEED);
        assert_eq!(WorkBlock::Warmup.sweeps(), WARMUP_SWEEPS);
        assert_eq!(WorkBlock::Load.seed(), RC021_LOAD_SEED);
        assert_eq!(WorkBlock::Load.sweeps(), LOAD_BLOCK_SWEEPS);
        // A closed set of exactly two: no third block, and no way to build one
        // with a chosen count or a chosen seed.
        let _: fn(
            &crate::protocol::VerifiedSentinelInstance,
            WorkBlock,
        ) -> Result<crate::protocol::WorkBlockRun, ProtocolError> = run_work_block;
        for b in [WorkBlock::Warmup, WorkBlock::Load] {
            match b {
                WorkBlock::Warmup | WorkBlock::Load => {}
            }
        }
    }

    /// §4.1: `RC021_LOAD_SEED` drives the load block and nothing else. The
    /// warmup, which is otherwise the same shape, uses the sentinel seed.
    #[test]
    fn the_load_seed_appears_only_in_the_load_block() {
        let mut with_load = 0;
        for phase in PHASE_ORDER {
            for b in 1..=BLOCKS_PER_PHASE {
                if let Some(w) = work_before(phase, b).unwrap() {
                    if w.seed() == RC021_LOAD_SEED {
                        assert_eq!(phase, Phase::C, "only phase C loads");
                        assert_eq!(w.sweeps(), LOAD_BLOCK_SWEEPS);
                        with_load += 1;
                    } else {
                        assert_eq!(w.seed(), RC021_SENTINEL_SEED);
                    }
                }
            }
        }
        assert_eq!(with_load, 5, "five load blocks per session");
        // §4.1: one whole session is five load blocks of 2048 sweeps each.
        assert_eq!(with_load * LOAD_BLOCK_SWEEPS, 5 * 2048);
    }

    /// The load path is exercised on a tiny synthetic graph. No RC-021 session
    /// runs, and no sentinel is executed on G11.
    #[test]
    fn a_work_block_runs_its_frozen_count_on_a_synthetic_graph() {
        let ir = crate::protocol::tests_support::synthetic_verified();
        for block in [WorkBlock::Warmup, WorkBlock::Load] {
            let r = run_work_block(&ir, block).unwrap();
            assert_eq!(r.sweeps, block.sweeps(), "{}", block.name());
            assert!(r.wall_ms.is_finite() && r.wall_ms >= 0.0);
        }
    }

    #[test]
    fn session_order_seed_maps_one_based_sessions_onto_the_bounded_index() {
        assert_eq!(session_order_seed(1).unwrap(), 31004);
        assert_eq!(session_order_seed(6).unwrap(), 31009);
        let mut seen = Vec::new();
        for s in 1..=SESSIONS {
            seen.push(session_order_seed(s).unwrap());
        }
        assert_eq!(seen.len(), 6);
        seen.dedup();
        assert_eq!(seen.len(), 6, "one ordering seed per session");
        for bad in [0u8, 7, 255] {
            assert!(session_order_seed(bad).is_err(), "session {bad}");
        }
    }

    // ------------------------------------------------------ fixtures

    /// A byte-exact snapshot of a directory, so "nothing was written" means
    /// the contents, not merely the file names.
    fn dir_bytes(p: &Path) -> Vec<(String, Vec<u8>)> {
        let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(p)
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                let name = e.file_name().to_string_lossy().into_owned();
                let body = std::fs::read(e.path()).unwrap_or_default();
                (name, body)
            })
            .collect();
        v.sort();
        v
    }

    fn fields() -> HostFields {
        HostFields {
            kernel_release: "6.6.0".into(),
            kernel_version: "#1 SMP".into(),
            available_processors: "4".into(),
            mem_total_kb: "10185860".into(),
            cpus_allowed_list: "0-3".into(),
            cpu_model: "Test CPU".into(),
        }
    }

    fn manifest(dir: &Path) -> RunManifest {
        let hf = fields();
        RunManifest {
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            run_start_uptime_ms: 1_000,
            run_dir: std::fs::canonicalize(dir)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            amendment_commits: vec!["c".repeat(40), "d".repeat(40)],
            instrument_birth_commit: "e".repeat(40),
            host_fingerprint: hf.fingerprint(),
            host_fields: hf.clone(),
            cpu_set: hf.cpus_allowed_list.clone(),
            thread_count: 1,
            timer_resolution_ms: 0.00002,
            cpu_time_unit: CPU_TIME_UNIT.into(),
            command_line: vec!["exp_rc021_host_qualify".into()],
            utc_start: "2026-08-25T00:00:00Z".into(),
            diag_availability: DiagProbe {
                cpu_time: true,
                ctx_switches: true,
                freq: false,
            },
        }
    }

    fn observed(m: &RunManifest) -> ObservedConfig {
        ObservedConfig {
            cpus_allowed_list: m.host_fields.cpus_allowed_list.clone(),
            thread_count: m.thread_count,
            host_fingerprint: m.host_fingerprint.clone(),
            repo_commit: m.repo_commit.clone(),
        }
    }

    // ------------------------------------------ §C11.45 the configuration check

    #[test]
    fn configuration_must_match_the_manifest_exactly() {
        let d = TempDir::new("config");
        let m = manifest(d.path());
        assert_eq!(check_configuration(&observed(&m), &m), Ok(()));

        type Mutate = (&'static str, Box<dyn Fn(&mut ObservedConfig)>);
        let cases: Vec<Mutate> = vec![
            (
                "cpus_allowed_list",
                Box::new(|o: &mut ObservedConfig| o.cpus_allowed_list = "0-1".into()),
            ),
            (
                "thread_count",
                Box::new(|o: &mut ObservedConfig| o.thread_count = 2),
            ),
            (
                "host_fingerprint",
                Box::new(|o: &mut ObservedConfig| o.host_fingerprint = "f".repeat(64)),
            ),
            (
                "repo_commit",
                Box::new(|o: &mut ObservedConfig| o.repo_commit = "9".repeat(40)),
            ),
        ];
        for (field, mutate) in cases {
            let mut o = observed(&m);
            mutate(&mut o);
            let err = check_configuration(&o, &m).unwrap_err();
            assert_eq!(err.field, field, "{field} must be reported by name");
        }
        // Not even a whitespace variant of the same CPU set passes: §C11.45
        // says exact match.
        let mut o = observed(&m);
        o.cpus_allowed_list = " 0-3".into();
        assert!(check_configuration(&o, &m).is_err());
    }

    // --------------------------------------------------- §C11.5 the idle gap

    #[test]
    fn session_one_anchors_on_controls_complete_and_the_rest_on_the_previous_close() {
        assert_eq!(
            anchor_for(1, 5_000, None).unwrap(),
            GapAnchor::ControlsComplete {
                monotonic_offset_ms: 5_000
            }
        );
        // Session 1 ignores any previous close, because there is none.
        assert_eq!(
            anchor_for(1, 5_000, Some(9_000)).unwrap(),
            GapAnchor::ControlsComplete {
                monotonic_offset_ms: 5_000
            }
        );
        for s in 2..=SESSIONS {
            assert_eq!(
                anchor_for(s, 5_000, Some(9_000)).unwrap(),
                GapAnchor::PreviousSessionClose {
                    session: s - 1,
                    monotonic_offset_ms: 9_000
                }
            );
            // A predecessor with no SESSION-CLOSE row leaves no anchor.
            assert!(anchor_for(s, 5_000, None).is_err());
        }
        assert!(anchor_for(0, 1, None).is_err());
        assert!(anchor_for(7, 1, Some(1)).is_err());
    }

    /// Ten minutes, inclusive: "at least" means equality passes.
    #[test]
    fn the_gap_floor_is_inclusive_and_a_backwards_clock_is_not_a_short_gap() {
        let a = GapAnchor::ControlsComplete {
            monotonic_offset_ms: 1_000,
        };
        assert_eq!(
            check_gap(a, 1_000 + MIN_GAP_MS),
            GapVerdict::Sufficient { gap_ms: MIN_GAP_MS }
        );
        assert_eq!(
            check_gap(a, 1_000 + MIN_GAP_MS - 1),
            GapVerdict::Short {
                gap_ms: MIN_GAP_MS - 1,
                required_ms: MIN_GAP_MS
            }
        );
        assert_eq!(
            check_gap(a, 1_000),
            GapVerdict::Short {
                gap_ms: 0,
                required_ms: MIN_GAP_MS
            }
        );
        // Below the anchor is a broken axis, not a gap to be waited out.
        assert_eq!(
            check_gap(a, 999),
            GapVerdict::ClockWentBackwards {
                anchor_ms: 1_000,
                now_ms: 999
            }
        );
    }

    // ------------------------------------------------- the pre-journal gate

    fn preflight_for(m: &RunManifest, now_offset_ms: u64) -> SessionPreflight<'_> {
        SessionPreflight {
            session: 1,
            manifest: m,
            observed: observed(m),
            observed_boot_id: m.boot_id.clone(),
            now_offset_ms,
            controls_complete_offset_ms: 1_000,
            previous_close_offset_ms: None,
            run_invalid: RunInvalidState::Absent,
        }
    }

    /// §C11.45: the gate runs before any file of this mode exists — proved by
    /// checking the directory is untouched after every outcome, refusal or not.
    #[test]
    fn every_outcome_leaves_the_directory_untouched() {
        let d = TempDir::new("gate");
        let m = manifest(d.path());
        let entries = |p: &Path| -> Vec<String> {
            let mut v: Vec<String> = std::fs::read_dir(p)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            v.sort();
            v
        };
        let empty: Vec<String> = Vec::new();
        assert_eq!(entries(d.path()), empty);

        // short gap — correctable, nothing written
        let p = preflight_for(&m, 1_000 + MIN_GAP_MS - 1);
        let out = session_preflight(&p);
        assert_eq!(
            out,
            SessionOutcome::ShortGap {
                gap_ms: MIN_GAP_MS - 1,
                required_ms: MIN_GAP_MS
            }
        );
        assert_eq!(out.exit_code(), 2);
        assert!(out.may_retry_after_waiting());
        assert!(!out.writes_run_invalid());
        assert_eq!(entries(d.path()), empty, "short gap created nothing");

        // affinity mismatch — a session is always post-marker, so terminal
        let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
        p.observed.cpus_allowed_list = "0-1".into();
        let out = session_preflight(&p);
        assert!(matches!(out, SessionOutcome::Terminal { .. }), "{out:?}");
        assert_eq!(out.exit_code(), 3);
        assert!(out.writes_run_invalid());
        assert!(!out.may_retry_after_waiting());
        assert_eq!(entries(d.path()), empty, "the gate created nothing");

        // clean case proceeds, and creates nothing either
        let p = preflight_for(&m, 1_000 + MIN_GAP_MS);
        assert_eq!(
            session_preflight(&p),
            SessionOutcome::Proceed {
                gap_ms: MIN_GAP_MS,
                anchor: GapAnchor::ControlsComplete {
                    monotonic_offset_ms: 1_000
                },
                order_seed: 31004
            }
        );
        assert_eq!(entries(d.path()), empty);
    }

    /// A terminal fact must not hide behind a correctable one.
    #[test]
    fn a_terminal_mismatch_outranks_a_short_gap() {
        let d = TempDir::new("order");
        let m = manifest(d.path());
        let mut p = preflight_for(&m, 1_000); // gap = 0, far too short
        p.observed.host_fingerprint = "f".repeat(64);
        let out = session_preflight(&p);
        assert!(matches!(out, SessionOutcome::Terminal { .. }), "{out:?}");
        assert_eq!(out.exit_code(), 3);

        let mut p = preflight_for(&m, 1_000);
        p.observed_boot_id = "boot-2".into();
        assert!(matches!(
            session_preflight(&p),
            SessionOutcome::Terminal { .. }
        ));

        // a backwards clock is terminal, never a ShortGap
        let mut p = preflight_for(&m, 0);
        p.controls_complete_offset_ms = 5_000;
        assert!(matches!(
            session_preflight(&p),
            SessionOutcome::Terminal { .. }
        ));
    }

    /// §C11.5: the anchor is derived from the session number. A caller cannot
    /// hand session 6 the controls anchor, which would compare its ten minutes
    /// against a point five sessions in the past.
    #[test]
    fn the_anchor_is_derived_from_the_session_number() {
        let d = TempDir::new("anchor");
        let m = manifest(d.path());

        // Session 1 uses controls_complete even when a close offset is present.
        let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
        p.previous_close_offset_ms = Some(999_999_999);
        assert!(matches!(
            session_preflight(&p),
            SessionOutcome::Proceed {
                anchor: GapAnchor::ControlsComplete { .. },
                ..
            }
        ));

        // Sessions 2..6 use the previous close, and the controls offset — old
        // enough to pass on its own — cannot rescue a short gap.
        for s in 2..=SESSIONS {
            let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
            p.session = s;
            p.controls_complete_offset_ms = 0;
            p.previous_close_offset_ms = Some(1_000 + MIN_GAP_MS - 1);
            let out = session_preflight(&p);
            assert_eq!(
                out,
                SessionOutcome::ShortGap {
                    gap_ms: 1,
                    required_ms: MIN_GAP_MS
                },
                "session {s} must anchor on its predecessor's close"
            );

            let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
            p.session = s;
            p.previous_close_offset_ms = Some(1_000);
            match session_preflight(&p) {
                SessionOutcome::Proceed {
                    anchor, order_seed, ..
                } => {
                    assert_eq!(
                        anchor,
                        GapAnchor::PreviousSessionClose {
                            session: s - 1,
                            monotonic_offset_ms: 1_000
                        }
                    );
                    assert_eq!(order_seed, 31003 + s as u64);
                }
                other => panic!("session {s}: {other:?}"),
            }

            // A predecessor with no close row refuses, and never silently
            // falls back to the controls anchor.
            let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
            p.session = s;
            p.previous_close_offset_ms = None;
            assert_eq!(
                session_preflight(&p),
                SessionOutcome::PredecessorNotClosed {
                    session: s,
                    predecessor: s - 1
                },
                "session {s} without a predecessor close"
            );
        }
    }

    /// A record of **this** run: the three expected fields mirror the manifest
    /// fixture, while the observed ones carry the mismatch it documents.
    fn run_invalid_for(m: &RunManifest) -> RunInvalid {
        RunInvalid {
            schema_version: crate::journal::SCHEMA_VERSION.to_string(),
            run_uuid: m.run_uuid.clone(),
            boot_id_observed: "boot-2".into(),
            boot_id_expected: m.boot_id.clone(),
            host_fingerprint_observed: "f".repeat(64),
            host_fingerprint_expected: m.host_fingerprint.clone(),
            reason: "boot_id mismatch".into(),
            command_line: vec!["exp_rc021_host_qualify".into(), "--session".into()],
            monotonic_offset_ms: Some(12_345),
            utc: "2026-08-25T00:00:00Z".into(),
        }
    }

    /// §C12.3: a **valid** existing `run_invalid.json` is its own outcome.
    /// Writing a second one would fail `create_new` and downgrade a recorded
    /// `INSTRUMENT-INVALID` into a spurious `JOURNAL-INVALID`.
    #[test]
    fn a_valid_existing_run_invalid_is_exit_three_and_writes_nothing() {
        let d = TempDir::new("forbid");
        let m = manifest(d.path());
        let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
        p.run_invalid = RunInvalidState::Valid(Box::new(run_invalid_for(&m)));
        let out = session_preflight(&p);
        assert_eq!(out, SessionOutcome::AlreadyInvalid);
        assert_eq!(out.exit_code(), 3);
        assert!(!out.writes_run_invalid(), "the record already exists");
        assert!(!out.may_retry_after_waiting());

        // It outranks every other condition, including a short gap.
        let mut p = preflight_for(&m, 1_000);
        p.run_invalid = RunInvalidState::Valid(Box::new(run_invalid_for(&m)));
        p.observed.cpus_allowed_list = "0-1".into();
        assert_eq!(session_preflight(&p), SessionOutcome::AlreadyInvalid);

        // Following the API is what protects the recorded record: a second
        // write is refused by `create_new`, which is exactly the exit 4 the
        // separate outcome exists to avoid reaching.
        run_invalid_for(&m).write(d.path()).unwrap();
        assert_eq!(
            run_invalid_for(&m).write(d.path()),
            Err(RunInvalidError::AlreadyPresent)
        );
    }

    /// §C10.1: a damaged `run_invalid.json` is `JOURNAL-INVALID`, exit 4 — not
    /// a record that forbids further sessions, and not something to write over.
    #[test]
    fn a_damaged_run_invalid_is_journal_invalid_and_writes_nothing() {
        let d = TempDir::new("damaged");
        let m = manifest(d.path());
        let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
        p.run_invalid = RunInvalidState::Damaged("truncated".into());
        let out = session_preflight(&p);
        assert_eq!(
            out,
            SessionOutcome::DamagedRunInvalid {
                why: "truncated".into()
            }
        );
        assert_eq!(out.exit_code(), 4);
        assert!(
            !out.writes_run_invalid(),
            "never write over a damaged record"
        );
        assert!(!out.may_retry_after_waiting());
        // It outranks a configuration mismatch too: the damaged file is the
        // more serious fact, and a Terminal here would write a second record.
        let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
        p.run_invalid = RunInvalidState::Damaged("empty".into());
        p.observed_boot_id = "boot-2".into();
        assert!(matches!(
            session_preflight(&p),
            SessionOutcome::DamagedRunInvalid { .. }
        ));
    }

    /// A canonical `run_invalid.json` left by **another run** is not this run's
    /// terminal evidence. It parses, it is canonical, and it must still be
    /// refused — as `JOURNAL-INVALID`, exit 4, with nothing written.
    #[test]
    fn a_canonical_record_of_another_run_is_not_this_runs_evidence() {
        type Mutate = (&'static str, Box<dyn Fn(&mut RunInvalid)>);
        let cases: Vec<Mutate> = vec![
            (
                "run_uuid",
                Box::new(|r: &mut RunInvalid| r.run_uuid = "1".repeat(32)),
            ),
            (
                "boot_id_expected",
                Box::new(|r: &mut RunInvalid| r.boot_id_expected = "boot-99".into()),
            ),
            (
                "host_fingerprint_expected",
                Box::new(|r: &mut RunInvalid| r.host_fingerprint_expected = "b".repeat(64)),
            ),
        ];
        for (field, mutate) in cases {
            let d = TempDir::new("foreign");
            let m = manifest(d.path());
            let mut foreign = run_invalid_for(&m);
            mutate(&mut foreign);

            // The file itself is beyond reproach: well-formed and canonical.
            let text = foreign.render();
            assert_eq!(foreign.validate(), Ok(()), "{field}");
            assert_eq!(
                RunInvalid::parse(&text).unwrap(),
                foreign,
                "{field}: the JSON must still parse and be canonical"
            );

            // Only the binding to this run's manifest rejects it.
            assert!(bind_run_invalid(&run_invalid_for(&m), &m).is_ok());
            let why = bind_run_invalid(&foreign, &m).unwrap_err();
            assert!(why.contains(field), "{field}: {why}");

            // …and the gate turns that into exit 4 with no write.
            let before = dir_bytes(d.path());
            let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
            p.run_invalid = RunInvalidState::Valid(Box::new(foreign.clone()));
            let out = session_preflight(&p);
            assert_eq!(out, SessionOutcome::DamagedRunInvalid { why }, "{field}");
            assert_eq!(out.exit_code(), 4);
            assert!(!out.writes_run_invalid(), "{field}");
            assert!(!out.may_retry_after_waiting());
            assert_eq!(dir_bytes(d.path()), before, "{field}: directory changed");

            // The identity branch outranks a configuration mismatch, which
            // would otherwise write a second record over a foreign one.
            let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
            p.run_invalid = RunInvalidState::Valid(Box::new(foreign));
            p.observed.thread_count = 99;
            p.observed_boot_id = "boot-2".into();
            let out = session_preflight(&p);
            assert!(
                matches!(out, SessionOutcome::DamagedRunInvalid { .. }),
                "{field}: {out:?}"
            );
            assert_eq!(dir_bytes(d.path()), before, "{field}: directory changed");
        }
    }

    /// §C12.3: the `observed` fields document the mismatch, so they are
    /// expected to differ from the manifest and must never be bound.
    #[test]
    fn the_observed_fields_are_never_bound_to_the_manifest() {
        let d = TempDir::new("observed");
        let m = manifest(d.path());
        let mut r = run_invalid_for(&m);
        r.boot_id_observed = "some other boot".into();
        r.host_fingerprint_observed = "c".repeat(64);
        assert_ne!(r.boot_id_observed, m.boot_id);
        assert_ne!(r.host_fingerprint_observed, m.host_fingerprint);
        assert_eq!(
            bind_run_invalid(&r, &m),
            Ok(()),
            "a record documenting a mismatch is still this run's record"
        );
        let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
        p.run_invalid = RunInvalidState::Valid(Box::new(r));
        assert_eq!(session_preflight(&p), SessionOutcome::AlreadyInvalid);
    }

    /// The three states are read from disk, and only a canonical, valid record
    /// is `Valid`.
    #[test]
    fn the_three_run_invalid_states_are_distinguished_on_disk() {
        let d = TempDir::new("classify");
        let m = manifest(d.path());
        assert_eq!(classify_run_invalid(d.path()), RunInvalidState::Absent);

        run_invalid_for(&m).write(d.path()).unwrap();
        assert_eq!(
            classify_run_invalid(d.path()),
            RunInvalidState::Valid(Box::new(run_invalid_for(&m)))
        );
        assert_eq!(
            RunInvalid::parse(&run_invalid_for(&m).render()).unwrap(),
            run_invalid_for(&m),
            "a record round-trips"
        );

        let good = run_invalid_for(&m).render();
        let damaged: [(&str, String); 6] = [
            ("empty", String::new()),
            ("truncated", good[..good.len() / 2].to_string()),
            ("no trailing LF", good.trim_end().to_string()),
            ("pretty-printed", good.replace("\":\"", "\": \"")),
            (
                "domain violation",
                good.replace(&"0".repeat(32), "not-a-uuid"),
            ),
            ("not JSON", "run_invalid".to_string()),
        ];
        for (what, body) in damaged {
            let d = TempDir::new("damaged_disk");
            std::fs::write(d.path().join(RUN_INVALID_FILE), &body).unwrap();
            assert!(
                matches!(classify_run_invalid(d.path()), RunInvalidState::Damaged(_)),
                "{what} must be Damaged, body {body:?}"
            );
        }
        // A key removed, and a key added, are both damage.
        let d = TempDir::new("keys");
        std::fs::write(
            d.path().join(RUN_INVALID_FILE),
            good.replace("\"utc\":", "\"utc2\":"),
        )
        .unwrap();
        assert!(matches!(
            classify_run_invalid(d.path()),
            RunInvalidState::Damaged(_)
        ));
    }

    /// A bad session number is a precondition refusal, not a host mismatch.
    #[test]
    fn an_out_of_range_session_is_a_request_refusal_not_a_host_mismatch() {
        let d = TempDir::new("badsession");
        let m = manifest(d.path());
        for bad in [0u8, 7, 255] {
            let mut p = preflight_for(&m, 1_000 + MIN_GAP_MS);
            p.session = bad;
            p.previous_close_offset_ms = Some(1_000);
            let out = session_preflight(&p);
            assert!(
                matches!(out, SessionOutcome::InvalidRequest { .. }),
                "session {bad}: {out:?}"
            );
            assert_eq!(out.exit_code(), 2);
            assert!(!out.writes_run_invalid(), "nothing about the host changed");
        }
    }

    /// Neither classification refusal may demand a `run_invalid.json`: doing so
    /// would turn a session state or a bad request into a Class I claim about
    /// the host.
    #[test]
    fn only_a_host_mismatch_writes_run_invalid() {
        let d = TempDir::new("class");
        let m = manifest(d.path());
        let entries = |p: &Path| std::fs::read_dir(p).unwrap().count();

        let mut writes = Vec::new();
        let mut cases: Vec<(&str, SessionPreflight<'_>)> = Vec::new();

        let mut a = preflight_for(&m, 1_000 + MIN_GAP_MS);
        a.session = 3;
        a.previous_close_offset_ms = None;
        cases.push(("predecessor not closed", a));

        let mut b = preflight_for(&m, 1_000 + MIN_GAP_MS);
        b.session = 9;
        cases.push(("bad session number", b));

        let mut c = preflight_for(&m, 1_000 + MIN_GAP_MS);
        c.run_invalid = RunInvalidState::Damaged("x".into());
        cases.push(("damaged record", c));

        let mut e = preflight_for(&m, 1_000 + MIN_GAP_MS);
        e.observed.thread_count = 99;
        cases.push(("configuration mismatch", e));

        for (what, p) in cases {
            let out = session_preflight(&p);
            writes.push((what, out.writes_run_invalid(), out.exit_code()));
        }
        assert_eq!(
            writes,
            vec![
                ("predecessor not closed", false, 2),
                ("bad session number", false, 2),
                ("damaged record", false, 4),
                ("configuration mismatch", true, 3),
            ]
        );
        assert_eq!(entries(d.path()), 0, "the gate created nothing");
    }

    /// §C10.1 would classify a corrupt `run_invalid.json` as `JOURNAL-INVALID`.
    /// The instrument must not be the thing that creates one — so a malformed
    /// record is refused **before** `create_new`, and therefore never reserves
    /// the path that makes the record immutable.
    #[test]
    fn a_malformed_run_invalid_is_refused_before_it_reserves_the_path() {
        let base = TempDir::new("malformed_base");
        let m = manifest(base.path());
        assert_eq!(run_invalid_for(&m).validate(), Ok(()));

        type Mutate = (&'static str, Box<dyn Fn(&mut RunInvalid)>);
        let cases: Vec<Mutate> = vec![
            (
                "schema_version",
                Box::new(|r: &mut RunInvalid| r.schema_version = "rc021/2".into()),
            ),
            (
                "run_uuid",
                Box::new(|r: &mut RunInvalid| r.run_uuid = "not-a-uuid".into()),
            ),
            (
                "run_uuid",
                Box::new(|r: &mut RunInvalid| r.run_uuid = "A".repeat(32)),
            ),
            (
                "host_fingerprint_observed",
                Box::new(|r: &mut RunInvalid| r.host_fingerprint_observed = "f".repeat(63)),
            ),
            (
                "host_fingerprint_expected",
                Box::new(|r: &mut RunInvalid| r.host_fingerprint_expected = "Z".repeat(64)),
            ),
            (
                "boot_id_observed",
                Box::new(|r: &mut RunInvalid| r.boot_id_observed = String::new()),
            ),
            (
                "boot_id_expected",
                Box::new(|r: &mut RunInvalid| r.boot_id_expected = "a\nb".into()),
            ),
            (
                "reason",
                Box::new(|r: &mut RunInvalid| r.reason = String::new()),
            ),
            (
                "reason",
                Box::new(|r: &mut RunInvalid| r.reason = "why\u{7}".into()),
            ),
            (
                "command_line",
                Box::new(|r: &mut RunInvalid| r.command_line = Vec::new()),
            ),
            (
                "utc",
                Box::new(|r: &mut RunInvalid| r.utc = "yesterday".into()),
            ),
            (
                "utc",
                Box::new(|r: &mut RunInvalid| r.utc = "2026-08-25".into()),
            ),
        ];
        for (field, mutate) in cases {
            let d = TempDir::new("bad_runinvalid");
            let mut r = run_invalid_for(&m);
            mutate(&mut r);
            assert_eq!(
                r.validate(),
                Err(RunInvalidError::Invalid(field)),
                "{field} must be refused"
            );
            assert_eq!(r.write(d.path()), Err(RunInvalidError::Invalid(field)));
            assert!(
                !d.path().join(RUN_INVALID_FILE).exists(),
                "{field}: the path must not be reserved"
            );
            // …and because it was not reserved, a well-formed record can still
            // be written afterwards. A reservation would have made the run
            // permanently unrecordable.
            assert!(run_invalid_for(&m).write(d.path()).is_ok());
        }
        assert_eq!(RunInvalidError::Invalid("utc").exit_code(), 4);
    }

    // ------------------------------------------------------------ no retry

    /// §2 and §C: a `LOST` measurement is recorded and never re-executed, and
    /// the plan offers no second attempt at any coordinate.
    #[test]
    fn the_plan_offers_no_second_attempt_at_any_coordinate() {
        let plan = plan_run();
        let mut keys: Vec<(u8, u32, u8, u32)> = plan
            .iter()
            .map(|c| {
                (
                    c.session,
                    phase_ordinal(c.phase),
                    c.block,
                    c.measurement_index,
                )
            })
            .collect();
        let n = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), n, "no coordinate is planned twice");
        assert_eq!(n, 90, "and there is no ninety-first attempt");

        // Two independent plans are identical: nothing is re-drawn or retried.
        assert_eq!(plan_run(), plan_run());
        for s in 1..=SESSIONS {
            assert_eq!(plan_session(s).unwrap(), plan_session(s).unwrap());
        }
    }
}
