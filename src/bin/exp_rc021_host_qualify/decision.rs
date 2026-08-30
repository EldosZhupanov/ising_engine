//! RC-021 §7.2, §8, §C13 and §C14 — the decision rule, finalization and
//! read-only verification.
//!
//! §C16 names exactly one module for this work, so every part of it lives here:
//! the session-state classifier, the measurement ledger, the frozen §7.2
//! predicate, the descriptive leave-one-session-out table, the ordered §8.2
//! terminal classifier, identity establishment, the fifteen-element integrity
//! inventory, the twenty-one-field closure codec, both artifact writers, the
//! reservation-first finalizer and the verifier.
//!
//! **Nothing here certifies a rate.** Amendment 1 §A2 withdrew the statistical
//! reading: the gate is a frozen engineering acceptance rule, and no code,
//! comment or output may say the protocol certifies any `p`, establishes a
//! confidence bound, or gives 95% confidence of anything.

// Step 8 wired this module to the CLI, so the blanket `allow(dead_code)` that
// stood here while it was unreachable is gone: over 5 700 lines it would hide
// genuinely dead code for the rest of the instrument's life.

use crate::controls::{RunStatus, SPREAD_BOUND};
use crate::journal::{JournalRead, ReadOutcome, Row, Status};
use std::io;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

// ============================================================== CONSTANTS

/// §C10.3 reserved names this module owns.
pub const CLOSURE_FILE: &str = "rc021_closure.json";
pub const OBSERVATIONS_FILE: &str = "rc021_observations.tsv";
pub const RESULTS_FILE: &str = "RC021_RESULTS.md";

/// §7.2 rule 2: failures ≤ 2 of 90.
pub const MAX_FAILURES: u32 = 2;
/// §7.2 rule 3: no single session contributes more than 1.
pub const MAX_PER_SESSION: u32 = 1;

/// §C13.4, the twenty-one top-level closure keys in their frozen order.
pub const CLOSURE_KEYS: [&str; 21] = [
    "schema_version",
    "run_uuid",
    "boot_id",
    "finalized_monotonic_offset_ms",
    "finalized_utc",
    "repo_commit",
    "prereg_commit",
    "amendment_commits",
    "instrument_birth_commit",
    "host_fingerprint",
    "identity_source",
    "terminal_status",
    "exit_code",
    "session_states",
    "verdict_rules",
    "failures_total",
    "failures_by_session",
    "control_outcomes",
    "integrity",
    "leave_one_out",
    "artifacts",
];

/// §C13.4: the kind of a file in the integrity inventory. It decides which
/// counters are integers and which are null — but only for a file that exists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntegrityKind {
    Json,
    JournalTsv,
    ControlTsv,
    ObservationsTsv,
    Markdown,
}

impl IntegrityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            IntegrityKind::Json => "JSON",
            IntegrityKind::JournalTsv => "JOURNAL_TSV",
            IntegrityKind::ControlTsv => "CONTROL_TSV",
            IntegrityKind::ObservationsTsv => "OBSERVATIONS_TSV",
            IntegrityKind::Markdown => "MARKDOWN",
        }
    }
    fn is_tabular(self) -> bool {
        !matches!(self, IntegrityKind::Json | IntegrityKind::Markdown)
    }
}

/// §C13.5, the fixed ordered set of fifteen elements.
///
/// **Deliberately not `manifest::RESERVED_PATHS`.** That constant is a
/// different set in a different order: it carries `rc021_closure.json` and a
/// bare `control` directory, and omits the P2 and N3 control journals. Reusing
/// it here would silently produce a wrong inventory.
pub const INTEGRITY_INVENTORY: [(&str, IntegrityKind); 15] = [
    ("run.json", IntegrityKind::Json),
    ("controls_started.json", IntegrityKind::Json),
    ("controls_complete.json", IntegrityKind::Json),
    ("run_invalid.json", IntegrityKind::Json),
    (
        "control/rc021_control_journal.tsv",
        IntegrityKind::ControlTsv,
    ),
    ("control/rc021_journal_p2.tsv", IntegrityKind::JournalTsv),
    ("control/rc021_journal_n3.tsv", IntegrityKind::JournalTsv),
    ("rc021_journal_s1.tsv", IntegrityKind::JournalTsv),
    ("rc021_journal_s2.tsv", IntegrityKind::JournalTsv),
    ("rc021_journal_s3.tsv", IntegrityKind::JournalTsv),
    ("rc021_journal_s4.tsv", IntegrityKind::JournalTsv),
    ("rc021_journal_s5.tsv", IntegrityKind::JournalTsv),
    ("rc021_journal_s6.tsv", IntegrityKind::JournalTsv),
    ("rc021_observations.tsv", IntegrityKind::ObservationsTsv),
    ("RC021_RESULTS.md", IntegrityKind::Markdown),
];

/// The path of session `s`'s journal, `s ∈ 1..6`.
pub fn session_journal_name(session: u8) -> String {
    format!("rc021_journal_s{session}.tsv")
}

// ================================================= §5.2 SESSION LIFECYCLE

/// §5.2's four states, and nothing else.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionState {
    NotStarted,
    Started,
    Completed,
    Aborted,
}

impl SessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionState::NotStarted => "NOT STARTED",
            SessionState::Started => "STARTED",
            SessionState::Completed => "COMPLETED",
            SessionState::Aborted => "ABORTED",
        }
    }
    pub fn parse(s: &str) -> Option<SessionState> {
        match s {
            "NOT STARTED" => Some(SessionState::NotStarted),
            "STARTED" => Some(SessionState::Started),
            "COMPLETED" => Some(SessionState::Completed),
            "ABORTED" => Some(SessionState::Aborted),
            _ => None,
        }
    }
}

/// One session's durable evidence, read and classified. Nothing is repaired and
/// no row is ever appended: §C8.3 keeps an unwritten measurement synthetic.
#[derive(Clone, Debug)]
pub struct SessionClassification {
    pub session: u8,
    /// `None` when the journal is absent or could not be classified.
    pub state: Option<SessionState>,
    /// `true` when the reader refused the file (§C13.7 damaged header or row).
    pub journal_invalid: bool,
    /// Rows with status `OK`, in file order.
    pub ok_rows: Vec<Row>,
    /// `OK` rows whose spread exceeds the bound.
    pub over_bound: u32,
    /// Physical `LOST` measurement rows.
    pub physical_lost: u32,
    /// §C8.3: one logical `LOST` from a truncated final line, in memory only.
    pub logical_lost: u32,
    /// Measurements the session never reached, counted synthetically.
    pub unwritten: u32,
    /// §8.1: an `EXTERNAL-CAUSE` row was written and fsynced before the abort.
    pub external_cause_before_abort: bool,
}

impl SessionClassification {
    /// §7.2: every measurement that is not a within-bound `OK` is a failure,
    /// and the denominator never shrinks.
    pub fn failures(&self) -> u32 {
        self.over_bound + self.physical_lost + self.logical_lost + self.unwritten
    }
    /// The spreads a quantile may use: `OK` rows only.
    pub fn valid_spreads(&self) -> Vec<f64> {
        self.ok_rows
            .iter()
            .filter_map(|r| r.paired_spread)
            .collect()
    }
}

/// The cross-row shape a session journal must have. Every field is derived
/// under [`session_grammar`], so each one is already known to be consistent
/// with the frozen plan.
#[derive(Clone, Copy, Default, Debug)]
struct SessionShape {
    open: u32,
    close_completed: u32,
    close_aborted: u32,
    /// Physical `OK`/`LOST` rows. Always an in-order prefix of the plan.
    physical_measurements: u32,
    external_cause: bool,
}

/// §5.2 and §C13.7 — the cross-row grammar, checked against the frozen plan of
/// `session.rs`.
///
/// [`crate::journal::parse_journal`] validates each row **in isolation**: it
/// never relates one row to the next, and `Row::validate` only range-checks
/// `measurement_index` against `1..=90`. Fifteen copies of one coordinate
/// therefore parse cleanly. The plan is the authority the rest of the
/// instrument already assumes:
///
/// - base §:680 rests "double-counting a retried repetition is **not
///   reachable**" on "every row carries a unique `measurement_index`";
/// - §C13.85 orders the observations by that index "since the index is global
///   and unique across the run, the order is total and no tie rule is needed";
/// - §C1 makes `(session, phase, block)` the address and `measurement_index`
///   the global order, so the two must agree row by row.
///
/// Any violation is a §C13.7 **damaged interior row**. What is *not* a
/// violation is an incomplete run: the observed measurement rows need only be
/// an in-order **prefix** of the plan, and the missing tail stays accounted for
/// by §C8.3's logical `LOST` and §7.2's synthetic unwritten failures.
fn session_grammar(
    session: u8,
    read: &JournalRead,
    identity: &RunIdentity,
) -> Result<SessionShape, String> {
    let plan = crate::session::plan_session(session).map_err(|e| format!("no frozen plan: {e}"))?;
    let planned = crate::session::MEASUREMENTS_PER_SESSION;
    let Some(metadata) = read.typed_metadata.as_ref() else {
        return Err("the journal has no typed metadata".to_string());
    };
    // §C10.1: the eight identity values. A journal that is not this run's is
    // not this run's evidence, whatever it says about itself.
    if RunIdentity::from_metadata(metadata) != *identity {
        return Err("journal metadata does not bind to this run".to_string());
    }
    if metadata.session != crate::journal::MetaSession::Qualification(session) {
        return Err(format!(
            "the journal path is for session {session}, but its metadata names {:?}",
            metadata.session
        ));
    }
    let mut s = SessionShape::default();
    let mut closed = false;
    for (i, r) in read.rows.iter().enumerate() {
        // §5.2: the close row ends the session, so nothing may follow it. This
        // also rejects a second close row.
        if closed {
            return Err(format!("row {} follows the close row", i + 1));
        }
        match r.status {
            // Exactly one, and first: a second `SESSION-OPEN` is not at index 0.
            Status::SessionOpen => {
                if i != 0 {
                    return Err(format!("SESSION-OPEN at row {}, not first", i + 1));
                }
                s.open += 1;
            }
            Status::SessionCloseCompleted | Status::SessionCloseAborted => {
                if s.open == 0 {
                    return Err("a close row with no SESSION-OPEN".to_string());
                }
                if r.status == Status::SessionCloseCompleted {
                    if s.external_cause {
                        return Err("SESSION-CLOSE-COMPLETED follows an EXTERNAL-CAUSE".to_string());
                    }
                    if s.physical_measurements != planned {
                        return Err(format!(
                            "SESSION-CLOSE-COMPLETED follows {} measurements, expected {planned}",
                            s.physical_measurements
                        ));
                    }
                    s.close_completed += 1;
                } else {
                    s.close_aborted += 1;
                }
                closed = true;
            }
            // §8.1 clause 3: the row justifies an abort, so it lives inside the
            // session it aborts.
            Status::ExternalCause => {
                if s.open == 0 {
                    return Err("EXTERNAL-CAUSE before SESSION-OPEN".to_string());
                }
                s.external_cause = true;
            }
            Status::Ok | Status::Lost => {
                if s.open == 0 {
                    return Err(format!("measurement row {} precedes SESSION-OPEN", i + 1));
                }
                let Some(want) = plan.get(s.physical_measurements as usize) else {
                    return Err(format!("more than {planned} measurement rows"));
                };
                if r.session != want.session
                    || r.phase != Some(want.phase)
                    || r.block != Some(want.block)
                    || r.measurement_index != Some(want.measurement_index)
                {
                    return Err(format!(
                        "row {} is not plan position {}: expected session {}, phase {}, \
                         block {}, index {}",
                        i + 1,
                        s.physical_measurements + 1,
                        want.session,
                        phase_str(want.phase),
                        want.block,
                        want.measurement_index
                    ));
                }
                s.physical_measurements += 1;
            }
        }
    }
    // §C8.3: the truncated final line is a logical `LOST` that occupies the
    // next plan position. It can therefore neither exceed the plan nor follow a
    // close row. A truncated line with no `SESSION-OPEN` is the ordinary crash
    // during the very first write and stays valid.
    if read.logical_lost_from_truncation {
        if closed {
            return Err("a truncated line follows the close row".to_string());
        }
        if s.physical_measurements >= planned {
            return Err(format!("a truncated line after measurement {planned}"));
        }
    }
    Ok(s)
}

/// §5.2 recognition from durable bytes. Never writes, never repairs.
/// `identity` binds the journal to **this** run.
///
/// Every other durable input is bound: the control journal by
/// `RunIdentity::from_metadata`, both markers in `read_controls_started` and
/// `read_controls_complete`, and a predecessor journal by `metadata_binds` in
/// the CLI. The session journals — the only ones the verdict is derived from —
/// were not. `Row::validate` cross-checks each row against *its own file's*
/// header, so a journal copied from another run, or another host, parses
/// cleanly, classifies `COMPLETED` with fifteen valid measurements, and counts
/// toward the ninety. `--verify` re-derives through the same path and confirms
/// it rather than catching it.
pub fn classify_session(
    session: u8,
    outcome: &ReadOutcome,
    identity: &RunIdentity,
) -> SessionClassification {
    let mut c = SessionClassification {
        session,
        state: Some(SessionState::NotStarted),
        journal_invalid: false,
        ok_rows: Vec::new(),
        over_bound: 0,
        physical_lost: 0,
        logical_lost: 0,
        unwritten: crate::session::MEASUREMENTS_PER_SESSION,
        external_cause_before_abort: false,
    };
    let read: &JournalRead = match outcome {
        // §C13.7: an absent journal is NOT STARTED, not a fault.
        ReadOutcome::Missing => return c,
        ReadOutcome::Present(r) => r,
    };
    if let crate::journal::ReadVerdict::JournalInvalid(_) = read.verdict {
        // §C13.7: a damaged header or interior row. No state can be asserted.
        c.journal_invalid = true;
        c.state = None;
        return c;
    }

    // §C13.7: the cross-row grammar, against the frozen plan. A coordinate or
    // lifecycle violation is a damaged interior row, and the ordered §8.2 test
    // reaches JOURNAL-INVALID through `MeasurementLedger::any_journal_invalid`.
    let shape = match session_grammar(session, read, identity) {
        Ok(s) => s,
        Err(_) => {
            c.journal_invalid = true;
            c.state = None;
            return c;
        }
    };

    // The grammar already proved every measurement row is at its planned
    // coordinate, so this pass only weighs the spreads.
    for r in &read.rows {
        match r.status {
            Status::Ok => {
                if r.paired_spread.map(|s| s > SPREAD_BOUND).unwrap_or(true) {
                    c.over_bound += 1;
                }
                c.ok_rows.push(r.clone());
            }
            Status::Lost => c.physical_lost += 1,
            _ => {}
        }
    }
    let mut measurements = shape.physical_measurements;
    // §C8.3: the truncated final line is a logical LOST held in memory only.
    // `parse_journal` pushes no `Row` for it, so it is never double-counted.
    if read.logical_lost_from_truncation {
        measurements += 1;
        c.logical_lost += 1;
    }

    let planned = crate::session::MEASUREMENTS_PER_SESSION;
    c.unwritten = planned.saturating_sub(measurements);
    c.external_cause_before_abort = shape.external_cause && shape.close_completed == 0;

    let close_completed = shape.close_completed;
    c.state = Some(if close_completed > 0 && measurements == planned {
        SessionState::Completed
    } else {
        // `ReadOutcome::Missing` returned above is the only NOT STARTED state.
        // A present durable header proves allocation began even when a crash
        // happened before SESSION-OPEN. Post-hoc, every valid non-completed
        // present journal is ABORTED; STARTED remains in the frozen closure
        // domain for observations made while a process is still alive.
        SessionState::Aborted
    });
    c
}

// ==================================================== §7.2 THE DECISION RULE

/// §C13.44 `verdict_rules` — six keys in this order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VerdictRules {
    pub rule1_all_completed: bool,
    pub rule2_failures: u32,
    pub rule2_pass: bool,
    pub rule3_max_per_session: u32,
    pub rule3_pass: bool,
    pub rule4_controls_pass: bool,
}

impl VerdictRules {
    pub fn qualified(&self) -> bool {
        self.rule1_all_completed && self.rule2_pass && self.rule3_pass && self.rule4_controls_pass
    }
}

/// The six sessions' evidence, held together so no caller can compute a
/// failure count from a different set than the verdict used.
#[derive(Clone, Debug)]
pub struct MeasurementLedger {
    pub sessions: Vec<SessionClassification>,
}

impl MeasurementLedger {
    pub fn new(sessions: Vec<SessionClassification>) -> MeasurementLedger {
        MeasurementLedger { sessions }
    }
    pub fn failures_by_session(&self) -> Vec<u32> {
        self.sessions.iter().map(|s| s.failures()).collect()
    }
    /// §7.2: the denominator is fixed at 90 and never shrinks.
    pub fn failures_total(&self) -> u32 {
        self.failures_by_session().iter().sum()
    }
    pub fn max_per_session(&self) -> u32 {
        self.failures_by_session().into_iter().max().unwrap_or(0)
    }
    pub fn all_completed(&self) -> bool {
        self.sessions
            .iter()
            .all(|s| s.state == Some(SessionState::Completed))
    }
    pub fn any_journal_invalid(&self) -> bool {
        self.sessions.iter().any(|s| s.journal_invalid)
    }
    /// §8.1 clause 3: an `EXTERNAL-CAUSE` row fsynced before the abort.
    pub fn external_cause_before_abort(&self) -> bool {
        self.sessions.iter().any(|s| s.external_cause_before_abort)
    }
}

/// §7.2, the frozen predicate. Four rules, evaluated over all six sessions and
/// the fixed denominator of 90.
///
/// Amendment 1 §A3: this is an **engineering acceptance rule**. It estimates
/// nothing and certifies no rate.
pub fn derive_verdict(
    ledger: &MeasurementLedger,
    controls_all_pass: bool,
) -> (VerdictRules, crate::controls::VerdictOutcome) {
    let rules = VerdictRules {
        rule1_all_completed: ledger.all_completed(),
        rule2_failures: ledger.failures_total(),
        rule2_pass: ledger.failures_total() <= MAX_FAILURES,
        rule3_max_per_session: ledger.max_per_session(),
        rule3_pass: ledger.max_per_session() <= MAX_PER_SESSION,
        rule4_controls_pass: controls_all_pass,
    };
    let verdict = if rules.qualified() {
        crate::controls::VerdictOutcome::Qualified
    } else {
        crate::controls::VerdictOutcome::NotQualified
    };
    (rules, verdict)
}

// ==================================== AMENDMENT 1 §A5 — DESCRIPTIVE ONLY

/// §C13.44's summary object — eight keys in this order.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LooSummary {
    pub omitted_session: Option<u8>,
    pub failure_count: u32,
    pub max_per_session: u32,
    pub median_spread: Option<f64>,
    pub p95_spread: Option<f64>,
    pub excluded_lost: u32,
    pub excluded_unwritten: u32,
    pub valid_spread_count: u32,
}

impl LooSummary {
    /// §C13.44: the three counters must sum to the summary's own set size.
    pub fn counters_sum(&self) -> u32 {
        self.valid_spread_count + self.excluded_lost + self.excluded_unwritten
    }
}

/// §C13.44 `leave_one_out` — `full_90` plus six omissions.
#[derive(Clone, PartialEq, Debug)]
pub struct LeaveOneOut {
    pub full_90: LooSummary,
    pub omissions: Vec<LooSummary>,
}

/// Amendment 1 §A5's nearest-rank quantile: the smallest value at or above
/// which at least `⌈q·n⌉` observations lie. **No interpolation** — it and
/// linear interpolation differ materially at p95 on n = 75.
pub fn nearest_rank(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let n = sorted.len() as f64;
    let rank = (q * n).ceil().max(1.0) as usize;
    sorted.get(rank.min(sorted.len()) - 1).copied()
}

fn summarise(sessions: &[&SessionClassification], omitted: Option<u8>) -> LooSummary {
    let mut spreads: Vec<f64> = Vec::new();
    let mut excluded_lost = 0u32;
    let mut excluded_unwritten = 0u32;
    let mut failure_count = 0u32;
    let mut max_per_session = 0u32;
    for s in sessions {
        spreads.extend(s.valid_spreads());
        // §A5: LOST rows carry no valid spread and are excluded from the two
        // quantiles, while still counting as failures.
        excluded_lost += s.physical_lost + s.logical_lost;
        excluded_unwritten += s.unwritten;
        failure_count += s.failures();
        max_per_session = max_per_session.max(s.failures());
    }
    spreads.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    LooSummary {
        omitted_session: omitted,
        failure_count,
        max_per_session,
        median_spread: nearest_rank(&spreads, 0.50),
        p95_spread: nearest_rank(&spreads, 0.95),
        excluded_lost,
        excluded_unwritten,
        valid_spread_count: spreads.len() as u32,
    }
}

/// Amendment 1 §A5 — **descriptive**. The verdict is the §7.2 predicate on all
/// six sessions and 90 measurements, full stop. No cell of this table may
/// change it, and nothing here is consulted by [`derive_verdict`].
pub fn leave_one_out(ledger: &MeasurementLedger) -> LeaveOneOut {
    let all: Vec<&SessionClassification> = ledger.sessions.iter().collect();
    let full_90 = summarise(&all, None);
    let mut omissions = Vec::with_capacity(6);
    for s in &ledger.sessions {
        let kept: Vec<&SessionClassification> = ledger
            .sessions
            .iter()
            .filter(|o| o.session != s.session)
            .collect();
        omissions.push(summarise(&kept, Some(s.session)));
    }
    omissions.sort_by_key(|o| o.omitted_session);
    LeaveOneOut { full_90, omissions }
}

// ================================================ §8.2 ORDERED CLASSIFIER

/// §C12.3 and §C12.4 — `run_invalid.json` as a strict reader sees it.
///
/// The old `matches!(…, Valid(_))` boolean collapsed three distinct states into
/// two and dropped the binding step entirely: a *damaged* record read as
/// "absent" let a run that had recorded a terminal host mismatch publish a
/// Class II verdict, and a *canonical record about another run* ended this run
/// at exit 3 on someone else's finding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunInvalidEvidence {
    /// No record: the run was never invalidated.
    Absent,
    /// §C12.3: canonical **and** bound to this run. Class I, exit 3.
    BoundToThisRun,
    /// §C12.4: present but unreadable, or canonical and about a different run
    /// (`session.rs`'s `bind_run_invalid` doctrine). Either way it is not this
    /// run's terminal evidence: `JOURNAL-INVALID`, exit 4.
    Unusable,
}

/// §C10.3: the run manifest, the one file that names whose run this is.
pub const MANIFEST_FILE: &str = "run.json";

/// Classify `run_invalid.json` **once**, for both finalize and verify.
///
/// Binding goes through `session::bind_run_invalid`, the same function
/// `session_preflight` uses, so the rule lives in exactly one place. When the
/// manifest cannot be read there is nothing to bind against and the run is
/// already `JOURNAL-INVALID` under §C10.1, so the record is `Unusable`: that is
/// the same exit 4 by a shorter path and can never be weaker.
pub fn read_run_invalid_evidence(dir: &Path) -> RunInvalidEvidence {
    match crate::session::classify_run_invalid(dir) {
        crate::session::RunInvalidState::Absent => RunInvalidEvidence::Absent,
        crate::session::RunInvalidState::Damaged(_) => RunInvalidEvidence::Unusable,
        crate::session::RunInvalidState::Valid(r) => {
            match crate::manifest::read_manifest(&dir.join(MANIFEST_FILE)) {
                Ok(m) => match crate::session::bind_run_invalid(&r, &m) {
                    Ok(()) => RunInvalidEvidence::BoundToThisRun,
                    Err(_) => RunInvalidEvidence::Unusable,
                },
                Err(_) => RunInvalidEvidence::Unusable,
            }
        }
    }
}

/// §C13.6 and the §C13 damage table — a control marker as a strict reader sees
/// it. Presence alone is never enough: the norm makes a *damaged* marker
/// `INSTRUMENT-INVALID`, which a `Path::exists` probe cannot distinguish from a
/// sound one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkerState {
    Absent,
    /// Canonical §C13.2/§C13.3 bytes carrying this run's identity.
    Bound,
    /// Present, but not readable as this run's canonical marker.
    Unusable,
}

/// Whether the path exists, treating an I/O error as presence rather than
/// absence — an unreadable marker is not an absent one.
/// Exposed so the presence doctrine itself can be tested against `.exists()`.
#[cfg(test)]
pub fn path_present_for_test(path: &Path) -> bool {
    path_present(path)
}

fn path_present(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(_) => true,
        Err(e) => e.kind() != std::io::ErrorKind::NotFound,
    }
}

/// §C13.2: `controls_started.json`, parsed canonically and bound to `identity`.
pub fn read_controls_started(dir: &Path, identity: &RunIdentity) -> MarkerState {
    let path = dir.join(crate::controls::CONTROLS_STARTED);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return if path_present(&path) {
            MarkerState::Unusable
        } else {
            MarkerState::Absent
        };
    };
    match crate::controls::ControlsStarted::parse(&text) {
        Ok(m) if m.run_uuid == identity.run_uuid && m.boot_id == identity.boot_id => {
            MarkerState::Bound
        }
        _ => MarkerState::Unusable,
    }
}

/// §C13.3: `controls_complete.json`, bound both to this run and to the central
/// control journal whose completion it asserts.
fn read_controls_complete(
    dir: &Path,
    identity: &RunIdentity,
    controls: &TerminalEvidenceControls,
) -> MarkerState {
    let path = dir.join(crate::controls::CONTROLS_COMPLETE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return if path_present(&path) {
            MarkerState::Unusable
        } else {
            MarkerState::Absent
        };
    };
    match crate::controls::ControlsComplete::parse(&text) {
        Ok(m)
            if m.run_uuid == identity.run_uuid
                && m.boot_id == identity.boot_id
                && controls.sha256.as_deref() == Some(m.control_journal_sha256.as_str())
                && controls.row_count == m.control_count
                && controls.all_pass == m.all_pass =>
        {
            MarkerState::Bound
        }
        _ => MarkerState::Unusable,
    }
}

/// The durable evidence the ordered test reads. Every field comes from bytes.
#[derive(Clone, Debug)]
pub struct TerminalEvidence {
    /// §C11.25: the class of the first `FAIL` row by ordinal, if any.
    pub first_control_failure: Option<crate::controls::FailureClass>,
    /// `true` when the control evidence itself could not be read.
    pub control_evidence_unreadable: bool,
    /// §C12.3/§C12.4: `run_invalid.json` as a strict reader sees it.
    pub run_invalid: RunInvalidEvidence,
    /// §C13's damage table: a present `controls_started.json` or
    /// `controls_complete.json` that is not this run's canonical marker.
    pub marker_unusable: bool,
    pub ledger: MeasurementLedger,
    pub controls_all_pass: bool,
}

/// §8.2's ordered test. Exactly one branch is taken, and the order is the
/// document's:
///
/// ```text
/// 1. a mandatory control failed          -> its §C11.25 class
/// 2. a journal integrity rule failed     -> JOURNAL-INVALID
/// 3. a preceding EXTERNAL-CAUSE row      -> INCONCLUSIVE-UNDERPOWERED
/// 4. otherwise                           -> §7.2 decides
/// ```
///
/// **Sole owner.** Finalize and verify both call this; neither keeps a copy.
pub fn classify_terminal(
    ev: &TerminalEvidence,
) -> (RunStatus, Option<(VerdictRules, LeaveOneOut)>) {
    match ev.run_invalid {
        // §C12.4: a record that cannot be read as this run's is JOURNAL-INVALID
        // and stops execution immediately. It is tested first because exit 4 is
        // the strongest code any later branch could reach, so placing it here
        // can never under-classify.
        RunInvalidEvidence::Unusable => return (RunStatus::JournalInvalid, None),
        // §C12.3: a standing, bound run_invalid.json is a recorded host or
        // provenance mismatch — Class I, exit 3, and no verdict is derived.
        RunInvalidEvidence::BoundToThisRun => return (RunStatus::InstrumentInvalid, None),
        RunInvalidEvidence::Absent => {}
    }
    // 1. §C11.25: a control's own class wins over the generic rule.
    if let Some(class) = ev.first_control_failure {
        return (class.run_status(), None);
    }
    if ev.control_evidence_unreadable {
        return (RunStatus::InstrumentInvalid, None);
    }
    // §C13's damage table: `controls_started.json`, `controls_complete.json`
    // and the control journal are all INSTRUMENT-INVALID when damaged.
    if ev.marker_unusable {
        return (RunStatus::InstrumentInvalid, None);
    }
    // 2. journal integrity.
    if ev.ledger.any_journal_invalid() {
        return (RunStatus::JournalInvalid, None);
    }
    // 3. §8.1: reachable only from a durable pre-abort EXTERNAL-CAUSE row.
    if ev.ledger.external_cause_before_abort() {
        return (RunStatus::InconclusiveUnderpowered, None);
    }
    // 4. §7.2 decides, and only here is a Class II verdict derived.
    let (rules, verdict) = derive_verdict(&ev.ledger, ev.controls_all_pass);
    let loo = leave_one_out(&ev.ledger);
    (verdict.run_status(), Some((rules, loo)))
}

// ============================================ §C10.1 IDENTITY ESTABLISHMENT

/// §C10.1's eight identity values, and nothing else.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RunIdentity {
    pub run_uuid: String,
    pub boot_id: String,
    pub run_start_uptime_ms: u64,
    pub repo_commit: String,
    pub prereg_commit: String,
    pub amendment_commits: Vec<String>,
    pub instrument_birth_commit: String,
    pub host_fingerprint: String,
}

/// §C13.4 field 11 — closed domain.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdentitySource {
    Manifest,
    ReconstructedHeaders,
}

impl IdentitySource {
    pub fn as_str(self) -> &'static str {
        match self {
            IdentitySource::Manifest => "MANIFEST",
            IdentitySource::ReconstructedHeaders => "RECONSTRUCTED_HEADERS",
        }
    }
    pub fn parse(s: &str) -> Option<IdentitySource> {
        match s {
            "MANIFEST" => Some(IdentitySource::Manifest),
            "RECONSTRUCTED_HEADERS" => Some(IdentitySource::ReconstructedHeaders),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum IdentityOutcome {
    Established {
        identity: RunIdentity,
        source: IdentitySource,
        /// §C10.1: a damaged-but-reconstructible manifest is `JOURNAL-INVALID`,
        /// and the closure records that while still naming the run.
        manifest_damaged: bool,
    },
    /// §C10.1 Branch B: the identity cannot be asserted, so **no closure is
    /// written**, not even as an empty reservation (§C13.9 step 1).
    Unreconstructible(String),
}

impl RunIdentity {
    pub fn from_manifest(m: &crate::manifest::RunManifest) -> RunIdentity {
        RunIdentity {
            run_uuid: m.run_uuid.clone(),
            boot_id: m.boot_id.clone(),
            run_start_uptime_ms: m.run_start_uptime_ms,
            repo_commit: m.repo_commit.clone(),
            prereg_commit: m.prereg_commit.clone(),
            amendment_commits: m.amendment_commits.clone(),
            instrument_birth_commit: m.instrument_birth_commit.clone(),
            host_fingerprint: m.host_fingerprint.clone(),
        }
    }
    fn from_metadata(m: &crate::journal::Metadata) -> RunIdentity {
        RunIdentity {
            run_uuid: m.run_uuid.clone(),
            boot_id: m.boot_id.clone(),
            run_start_uptime_ms: m.run_start_uptime_ms,
            repo_commit: m.repo_commit.clone(),
            prereg_commit: m.prereg_commit.clone(),
            amendment_commits: m.amendment_commits.clone(),
            instrument_birth_commit: m.instrument_birth_commit.clone(),
            host_fingerprint: m.host_fingerprint.clone(),
        }
    }
}

/// Every journal that may carry a §C9 metadata header, in a fixed order.
fn header_bearing_paths() -> Vec<String> {
    let mut v = vec![
        "control/rc021_control_journal.tsv".to_string(),
        "control/rc021_journal_p2.tsv".to_string(),
        "control/rc021_journal_n3.tsv".to_string(),
    ];
    for s in 1..=crate::session::SESSIONS {
        v.push(session_journal_name(s));
    }
    v
}

/// §C13.9 step 1 — identity, established **before anything is reserved**.
///
/// Branch A recovers the eight values from the durable headers and requires
/// every surviving header to agree on every one. Branch B returns without
/// reserving, so a closure that cannot name whose run it is is never created.
///
/// §C11.45: finalize applies **no** live qualification or provenance gate, so
/// nothing here calls `provenance::check`.
pub fn establish_identity(dir: &Path) -> IdentityOutcome {
    let manifest_path = dir.join("run.json");
    match crate::manifest::read_manifest(&manifest_path) {
        Ok(m) => {
            return IdentityOutcome::Established {
                identity: RunIdentity::from_manifest(&m),
                source: IdentitySource::Manifest,
                manifest_damaged: false,
            }
        }
        Err(_) => { /* fall through to Branch A */ }
    }

    let mut agreed: Option<RunIdentity> = None;
    for rel in header_bearing_paths() {
        let bytes = match std::fs::read(dir.join(&rel)) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let read = crate::journal::parse_journal(&bytes);
        let Some(meta) = read.typed_metadata.as_ref() else {
            continue;
        };
        let candidate = RunIdentity::from_metadata(meta);
        match &agreed {
            None => agreed = Some(candidate),
            Some(existing) if *existing == candidate => {}
            Some(_) => {
                return IdentityOutcome::Unreconstructible(format!(
                    "surviving headers disagree; {rel} differs"
                ))
            }
        }
    }
    match agreed {
        Some(identity) => IdentityOutcome::Established {
            identity,
            source: IdentitySource::ReconstructedHeaders,
            manifest_damaged: true,
        },
        None => IdentityOutcome::Unreconstructible(
            "no surviving header carries a valid §C9 metadata block".to_string(),
        ),
    }
}

// ============================================ §C13.5 INTEGRITY INVENTORY

/// §C13.4's per-file record. Existence governs first, `kind` second.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IntegrityEntry {
    pub path: String,
    pub kind: IntegrityKind,
    /// 64 hex, or the literal `MISSING`.
    pub sha256: String,
    pub byte_count: Option<u64>,
    pub physical_line_count: Option<u64>,
    pub metadata_line_count: Option<u64>,
    pub header_line_count: Option<u64>,
    pub parsed_row_count: Option<u64>,
    pub data_row_count: Option<u64>,
    /// `JOURNAL_TSV` carries the six journal statuses; `CONTROL_TSV` carries
    /// `PASS`/`FAIL`; every other kind, and every absent file, carries null.
    pub status_counts: Option<StatusCountsRecord>,
}

/// §C13.44: every key of the domain, including zeros, in fixed order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StatusCountsRecord {
    Journal(crate::journal::StatusCounts),
    Control { pass: u64, fail: u64 },
}

pub const MISSING: &str = "MISSING";

fn tabular_counters(bytes: &[u8], header: &str) -> (u64, u64, u64, u64, u64) {
    // (physical, metadata, header, data, parsed) for a non-24-column file.
    let physical = if bytes.is_empty() {
        0
    } else {
        let mut n = bytes.iter().filter(|b| **b == b'\n').count() as u64;
        if bytes.last() != Some(&b'\n') {
            n += 1;
        }
        n
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return (physical, 0, 0, 0, 0);
    };
    let mut lines: Vec<&str> = text.split('\n').collect();
    if bytes.last() == Some(&b'\n') {
        lines.pop();
    }
    let metadata = lines
        .iter()
        .take_while(|l| l.starts_with(crate::controls::META_PREFIX))
        .count() as u64;
    let rest = &lines[metadata as usize..];
    // §C13.4: header_line_count is 1 only for a complete, line-feed-terminated
    // line that matches the frozen header exactly.
    let header_ok = rest.first().map(|l| *l == header).unwrap_or(false)
        && (rest.len() > 1 || bytes.last() == Some(&b'\n'));
    if !header_ok {
        return (physical, metadata, 0, 0, 0);
    }
    let data: Vec<&str> = rest[1..].to_vec();
    // §C13.4: `parsed_row_count` is the reader's count, `data_row_count` the
    // file's. Returning the same number for both made them equal by
    // construction, so a torn final line was recorded as "parsed" beside a
    // `status_counts` of all zeros — a self-contradiction in the one artifact
    // that exists to be audited. A line the writer never terminated was never
    // parsed by anybody.
    let terminated = bytes.last() == Some(&b'\n');
    let parsed = if terminated {
        data.len() as u64
    } else {
        data.len().saturating_sub(1) as u64
    };
    (physical, metadata, 1, data.len() as u64, parsed)
}

fn control_header() -> String {
    crate::controls::control_header_line()
        .trim_end_matches('\n')
        .to_string()
}

fn observations_header() -> String {
    OBSERVATIONS_COLUMNS.join("\t")
}

/// §C13.85's eight frozen columns.
pub const OBSERVATIONS_COLUMNS: [&str; 8] = [
    "session",
    "phase",
    "block",
    "measurement_index",
    "monotonic_offset_ms",
    "sentinel_first_ms",
    "sentinel_last_ms",
    "paired_spread",
];

/// §C13.5 — the fixed ordered fifteen, always all fifteen.
pub fn build_integrity(dir: &Path) -> Vec<IntegrityEntry> {
    INTEGRITY_INVENTORY
        .iter()
        .map(|(rel, kind)| integrity_entry(dir, rel, *kind))
        .collect()
}

fn integrity_entry(dir: &Path, rel: &str, kind: IntegrityKind) -> IntegrityEntry {
    let absent = IntegrityEntry {
        path: rel.to_string(),
        kind,
        sha256: MISSING.to_string(),
        byte_count: None,
        physical_line_count: None,
        metadata_line_count: None,
        header_line_count: None,
        parsed_row_count: None,
        data_row_count: None,
        status_counts: None,
    };
    let Ok(bytes) = std::fs::read(dir.join(rel)) else {
        return absent;
    };
    let sha256 = crate::host::sha256_hex(&bytes);
    let byte_count = Some(bytes.len() as u64);
    if !kind.is_tabular() {
        // §C13.4: the five line counters are not applicable to JSON or MARKDOWN.
        return IntegrityEntry {
            sha256,
            byte_count,
            ..absent
        };
    }
    match kind {
        IntegrityKind::JournalTsv => {
            let read = crate::journal::parse_journal(&bytes);
            let c = read.counters;
            IntegrityEntry {
                sha256,
                byte_count,
                physical_line_count: Some(c.physical_line_count),
                metadata_line_count: Some(c.metadata_line_count),
                header_line_count: Some(c.header_line_count),
                parsed_row_count: Some(c.parsed_row_count),
                data_row_count: Some(c.data_row_count),
                // §C13.4: the reader's logical interpretation, so a truncated
                // final line contributes 1 to LOST even though no row on disk
                // carries that status.
                status_counts: Some(StatusCountsRecord::Journal(read.status_counts)),
                ..absent
            }
        }
        IntegrityKind::ControlTsv => {
            let (p, m, h, d, parsed) = tabular_counters(&bytes, &control_header());
            let (mut pass, mut fail) = (0u64, 0u64);
            if let Ok(rows) = crate::controls::parse_control_journal(&bytes) {
                for r in rows {
                    match r.status {
                        crate::controls::ControlStatus::Pass => pass += 1,
                        crate::controls::ControlStatus::Fail => fail += 1,
                    }
                }
            }
            IntegrityEntry {
                sha256,
                byte_count,
                physical_line_count: Some(p),
                metadata_line_count: Some(m),
                header_line_count: Some(h),
                parsed_row_count: Some(parsed),
                data_row_count: Some(d),
                status_counts: Some(StatusCountsRecord::Control { pass, fail }),
                ..absent
            }
        }
        _ => {
            let (p, m, h, d, parsed) = tabular_counters(&bytes, &observations_header());
            IntegrityEntry {
                sha256,
                byte_count,
                physical_line_count: Some(p),
                metadata_line_count: Some(m),
                header_line_count: Some(h),
                parsed_row_count: Some(parsed),
                data_row_count: Some(d),
                // §C13.4: observations carry no status column.
                status_counts: None,
                ..absent
            }
        }
    }
}

// ================================================== §C13.4 THE CLOSURE CODEC

/// §C13.44 `control_outcomes` — four keys, `detail` deliberately not copied.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ControlOutcomeRecord {
    pub control_id: String,
    pub ordinal: u32,
    pub status: String,
    pub monotonic_offset_ms: u64,
}

/// §C13.4 field 21.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Artifacts {
    pub observations_sha256: Option<String>,
    pub results_md_sha256: Option<String>,
}

/// §C13.4 — the twenty-one fields, in order.
#[derive(Clone, PartialEq, Debug)]
pub struct Closure {
    pub schema_version: String,
    pub run_uuid: String,
    pub boot_id: String,
    pub finalized_monotonic_offset_ms: Option<u64>,
    pub finalized_utc: String,
    pub repo_commit: String,
    pub prereg_commit: String,
    pub amendment_commits: Vec<String>,
    pub instrument_birth_commit: String,
    pub host_fingerprint: String,
    pub identity_source: IdentitySource,
    pub terminal_status: RunStatus,
    pub exit_code: i32,
    pub session_states: Vec<Option<SessionState>>,
    pub verdict_rules: Option<VerdictRules>,
    pub failures_total: Option<u32>,
    pub failures_by_session: Option<Vec<u32>>,
    pub control_outcomes: Vec<ControlOutcomeRecord>,
    pub integrity: Vec<IntegrityEntry>,
    pub leave_one_out: Option<LeaveOneOut>,
    pub artifacts: Artifacts,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ClosureError {
    Domain(&'static str),
    Field { what: &'static str, value: String },
}

impl std::fmt::Display for ClosureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClosureError::Domain(d) => write!(f, "{d}"),
            ClosureError::Field { what, value } => write!(f, "{what}: {value}"),
        }
    }
}

fn jstr(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn jnum(v: f64) -> String {
    // §C8.1: shortest round-trip, never fixed precision. This workspace's
    // serde_json is not built with its optional `float_roundtrip` feature, so
    // the standard shortest spelling can occasionally be decoded one ULP
    // away by `Value::as_f64`. Pick the shortest decimal spelling that the
    // actual closure reader maps back to the exact bits.
    let mut candidates = Vec::new();
    candidates.push(v.to_string());
    for precision in 0..=32 {
        let fixed = format!("{v:.precision$}");
        let fixed = if fixed.contains('.') {
            fixed
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            fixed
        };
        candidates.push(fixed);
        candidates.push(format!("{v:.precision$e}"));
    }
    candidates
        .into_iter()
        .filter(|candidate| {
            serde_json::from_str::<serde_json::Value>(candidate)
                .ok()
                .and_then(|n| n.as_f64())
                .is_some_and(|parsed| parsed.to_bits() == v.to_bits())
        })
        .min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
        .unwrap_or_else(|| v.to_string())
}

fn jopt_u64(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    }
}

fn jopt_f64(v: Option<f64>) -> String {
    match v {
        Some(n) => jnum(n),
        None => "null".to_string(),
    }
}

fn jarr(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

fn hexlen(s: &str, n: usize, what: &'static str) -> Result<(), ClosureError> {
    let ok = s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if ok {
        Ok(())
    } else {
        Err(ClosureError::Field {
            what,
            value: s.to_string(),
        })
    }
}

impl StatusCountsRecord {
    fn render(&self) -> String {
        match self {
            // §C13.44: every key of the domain, in fixed order, zeros included.
            StatusCountsRecord::Journal(c) => format!(
                "{{{}:{},{}:{},{}:{},{}:{},{}:{},{}:{}}}",
                jstr("OK"),
                c.ok,
                jstr("LOST"),
                c.lost,
                jstr("SESSION-OPEN"),
                c.session_open,
                jstr("SESSION-CLOSE-COMPLETED"),
                c.session_close_completed,
                jstr("SESSION-CLOSE-ABORTED"),
                c.session_close_aborted,
                jstr("EXTERNAL-CAUSE"),
                c.external_cause
            ),
            StatusCountsRecord::Control { pass, fail } => {
                format!("{{{}:{},{}:{}}}", jstr("PASS"), pass, jstr("FAIL"), fail)
            }
        }
    }
}

impl IntegrityEntry {
    fn render(&self) -> String {
        format!(
            "{{{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{}}}",
            jstr("path"),
            jstr(&self.path),
            jstr("kind"),
            jstr(self.kind.as_str()),
            jstr("sha256"),
            jstr(&self.sha256),
            jstr("byte_count"),
            jopt_u64(self.byte_count),
            jstr("physical_line_count"),
            jopt_u64(self.physical_line_count),
            jstr("metadata_line_count"),
            jopt_u64(self.metadata_line_count),
            jstr("header_line_count"),
            jopt_u64(self.header_line_count),
            jstr("parsed_row_count"),
            jopt_u64(self.parsed_row_count),
            jstr("data_row_count"),
            jopt_u64(self.data_row_count),
            jstr("status_counts"),
            self.status_counts
                .as_ref()
                .map(|c| c.render())
                .unwrap_or_else(|| "null".to_string())
        )
    }
}

impl VerdictRules {
    fn render(&self) -> String {
        format!(
            "{{{}:{},{}:{},{}:{},{}:{},{}:{},{}:{}}}",
            jstr("rule1_all_completed"),
            self.rule1_all_completed,
            jstr("rule2_failures"),
            self.rule2_failures,
            jstr("rule2_pass"),
            self.rule2_pass,
            jstr("rule3_max_per_session"),
            self.rule3_max_per_session,
            jstr("rule3_pass"),
            self.rule3_pass,
            jstr("rule4_controls_pass"),
            self.rule4_controls_pass
        )
    }
}

impl LooSummary {
    fn render(&self) -> String {
        format!(
            "{{{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{}}}",
            jstr("omitted_session"),
            self.omitted_session
                .map(|s| s.to_string())
                .unwrap_or_else(|| "null".to_string()),
            jstr("failure_count"),
            self.failure_count,
            jstr("max_per_session"),
            self.max_per_session,
            jstr("median_spread"),
            jopt_f64(self.median_spread),
            jstr("p95_spread"),
            jopt_f64(self.p95_spread),
            jstr("excluded_lost"),
            self.excluded_lost,
            jstr("excluded_unwritten"),
            self.excluded_unwritten,
            jstr("valid_spread_count"),
            self.valid_spread_count
        )
    }
}

impl LeaveOneOut {
    fn render(&self) -> String {
        let oms: Vec<String> = self.omissions.iter().map(|o| o.render()).collect();
        format!(
            "{{{}:{},{}:{}}}",
            jstr("full_90"),
            self.full_90.render(),
            jstr("omissions"),
            jarr(&oms)
        )
    }
}

impl ControlOutcomeRecord {
    fn render(&self) -> String {
        format!(
            "{{{}:{},{}:{},{}:{},{}:{}}}",
            jstr("control_id"),
            jstr(&self.control_id),
            jstr("ordinal"),
            self.ordinal,
            jstr("status"),
            jstr(&self.status),
            jstr("monotonic_offset_ms"),
            self.monotonic_offset_ms
        )
    }
}

impl Closure {
    /// §C13.45's mandatory-by-class matrix, plus every domain the schema fixes.
    pub fn validate(&self) -> Result<(), ClosureError> {
        if self.schema_version != crate::journal::SCHEMA_VERSION {
            return Err(ClosureError::Domain("schema_version"));
        }
        if !crate::host::is_run_uuid(&self.run_uuid) {
            return Err(ClosureError::Domain("run_uuid"));
        }
        if self.boot_id.is_empty() || self.boot_id.chars().any(|c| c.is_control()) {
            return Err(ClosureError::Domain("boot_id"));
        }
        // §C13.45: field 5 is mandatory in every closure.
        if chrono::DateTime::parse_from_rfc3339(&self.finalized_utc).is_err() {
            return Err(ClosureError::Domain("finalized_utc must be RFC 3339"));
        }
        hexlen(&self.repo_commit, 40, "repo_commit")?;
        hexlen(&self.prereg_commit, 40, "prereg_commit")?;
        if self.amendment_commits.len() != 2
            || self.amendment_commits[0] == self.amendment_commits[1]
        {
            return Err(ClosureError::Domain("amendment_commits"));
        }
        for a in &self.amendment_commits {
            hexlen(a, 40, "amendment_commits")?;
        }
        hexlen(&self.instrument_birth_commit, 40, "instrument_birth_commit")?;
        hexlen(&self.host_fingerprint, 64, "host_fingerprint")?;
        // The one authority for the code.
        if self.exit_code != self.terminal_status.exit_code() {
            return Err(ClosureError::Domain(
                "exit_code must be terminal_status.exit_code()",
            ));
        }
        if self.session_states.len() != crate::session::SESSIONS as usize {
            return Err(ClosureError::Domain(
                "session_states must have six elements",
            ));
        }
        if self.integrity.len() != INTEGRITY_INVENTORY.len() {
            return Err(ClosureError::Domain("integrity must have fifteen elements"));
        }
        for (e, (rel, kind)) in self.integrity.iter().zip(INTEGRITY_INVENTORY.iter()) {
            if e.path != *rel || e.kind != *kind {
                return Err(ClosureError::Field {
                    what: "integrity out of the frozen order",
                    value: e.path.clone(),
                });
            }
            if e.sha256 != MISSING {
                hexlen(&e.sha256, 64, "integrity sha256")?;
            }
        }
        if self.control_outcomes.len() > crate::controls::CONTROL_COUNT as usize {
            return Err(ClosureError::Domain("control_outcomes"));
        }
        for (i, c) in self.control_outcomes.iter().enumerate() {
            if c.ordinal != i as u32 + 1 {
                return Err(ClosureError::Field {
                    what: "control_outcomes must ascend by ordinal",
                    value: c.ordinal.to_string(),
                });
            }
        }

        let class_two = self.terminal_status.is_class_two();
        let scientific_present = self.verdict_rules.is_some()
            || self.failures_total.is_some()
            || self.failures_by_session.is_some()
            || self.leave_one_out.is_some();
        if class_two {
            if !scientific_present
                || self.verdict_rules.is_none()
                || self.failures_total.is_none()
                || self.failures_by_session.is_none()
                || self.leave_one_out.is_none()
            {
                return Err(ClosureError::Domain(
                    "a Class II closure must carry every scientific field",
                ));
            }
            if self.session_states.iter().any(|s| s.is_none()) {
                return Err(ClosureError::Domain(
                    "a Class II closure must classify all six sessions",
                ));
            }
            if self.control_outcomes.len() != crate::controls::CONTROL_COUNT as usize {
                return Err(ClosureError::Domain(
                    "a Class II closure must carry twelve control rows",
                ));
            }
            let by = self.failures_by_session.as_ref().unwrap();
            if by.len() != crate::session::SESSIONS as usize
                || by.iter().sum::<u32>() != self.failures_total.unwrap()
            {
                return Err(ClosureError::Domain("failures_by_session"));
            }
            let loo = self.leave_one_out.as_ref().unwrap();
            if loo.omissions.len() != crate::session::SESSIONS as usize {
                return Err(ClosureError::Domain("leave_one_out omissions"));
            }
            if loo.full_90.counters_sum() != crate::session::TOTAL_MEASUREMENTS {
                return Err(ClosureError::Domain("full_90 counters must sum to 90"));
            }
            let omitted_size =
                crate::session::TOTAL_MEASUREMENTS - crate::session::MEASUREMENTS_PER_SESSION;
            for o in &loo.omissions {
                if o.counters_sum() != omitted_size {
                    return Err(ClosureError::Domain("omission counters must sum to 75"));
                }
                // §C13.44: quantiles are null exactly when the denominator is 0.
                if (o.valid_spread_count == 0) != o.median_spread.is_none() {
                    return Err(ClosureError::Domain("median_spread nullity"));
                }
                if (o.valid_spread_count == 0) != o.p95_spread.is_none() {
                    return Err(ClosureError::Domain("p95_spread nullity"));
                }
            }
        } else {
            // §C13.45: a Class I closure carrying a derived scientific quantity
            // is itself invalid — it would publish a result the run never
            // earned.
            if scientific_present {
                return Err(ClosureError::Domain(
                    "a Class I closure must null every scientific field",
                ));
            }
            if self.artifacts.observations_sha256.is_some()
                || self.artifacts.results_md_sha256.is_some()
            {
                return Err(ClosureError::Domain(
                    "a Class I closure publishes no artifact hash",
                ));
            }
        }
        Ok(())
    }

    /// §C10.1: compact, field order fixed, UTF-8, one trailing line feed.
    pub fn render(&self) -> String {
        let states: Vec<String> = self
            .session_states
            .iter()
            .map(|s| {
                s.map(|v| jstr(v.as_str()))
                    .unwrap_or_else(|| "null".to_string())
            })
            .collect();
        let controls: Vec<String> = self.control_outcomes.iter().map(|c| c.render()).collect();
        let integrity: Vec<String> = self.integrity.iter().map(|e| e.render()).collect();
        let amendments: Vec<String> = self.amendment_commits.iter().map(|a| jstr(a)).collect();
        let by_session = self
            .failures_by_session
            .as_ref()
            .map(|v| jarr(&v.iter().map(|n| n.to_string()).collect::<Vec<_>>()))
            .unwrap_or_else(|| "null".to_string());
        let values: [String; 21] = [
            jstr(&self.schema_version),
            jstr(&self.run_uuid),
            jstr(&self.boot_id),
            jopt_u64(self.finalized_monotonic_offset_ms),
            jstr(&self.finalized_utc),
            jstr(&self.repo_commit),
            jstr(&self.prereg_commit),
            jarr(&amendments),
            jstr(&self.instrument_birth_commit),
            jstr(&self.host_fingerprint),
            jstr(self.identity_source.as_str()),
            jstr(self.terminal_status.as_str()),
            self.exit_code.to_string(),
            jarr(&states),
            self.verdict_rules
                .map(|r| r.render())
                .unwrap_or_else(|| "null".to_string()),
            self.failures_total
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".to_string()),
            by_session,
            jarr(&controls),
            jarr(&integrity),
            self.leave_one_out
                .as_ref()
                .map(|l| l.render())
                .unwrap_or_else(|| "null".to_string()),
            format!(
                "{{{}:{},{}:{}}}",
                jstr("observations_sha256"),
                self.artifacts
                    .observations_sha256
                    .as_ref()
                    .map(|s| jstr(s))
                    .unwrap_or_else(|| "null".to_string()),
                jstr("results_md_sha256"),
                self.artifacts
                    .results_md_sha256
                    .as_ref()
                    .map(|s| jstr(s))
                    .unwrap_or_else(|| "null".to_string())
            ),
        ];
        let body: Vec<String> = CLOSURE_KEYS
            .iter()
            .zip(values.iter())
            .map(|(k, v)| format!("{}:{}", jstr(k), v))
            .collect();
        format!("{{{}}}\n", body.join(","))
    }
}

type JsonMap = serde_json::Map<String, serde_json::Value>;

fn obj<'a>(v: &'a serde_json::Value, what: &'static str) -> Result<&'a JsonMap, ClosureError> {
    v.as_object().ok_or(ClosureError::Domain(what))
}

fn get<'a>(o: &'a JsonMap, k: &'static str) -> Result<&'a serde_json::Value, ClosureError> {
    o.get(k).ok_or(ClosureError::Domain(k))
}

fn s_of(o: &JsonMap, k: &'static str) -> Result<String, ClosureError> {
    get(o, k)?
        .as_str()
        .map(|s| s.to_string())
        .ok_or(ClosureError::Domain(k))
}

fn u64_of(o: &JsonMap, k: &'static str) -> Result<u64, ClosureError> {
    get(o, k)?.as_u64().ok_or(ClosureError::Domain(k))
}

fn u32_of(o: &JsonMap, k: &'static str) -> Result<u32, ClosureError> {
    u32::try_from(u64_of(o, k)?).map_err(|_| ClosureError::Domain(k))
}

fn bool_of(o: &JsonMap, k: &'static str) -> Result<bool, ClosureError> {
    get(o, k)?.as_bool().ok_or(ClosureError::Domain(k))
}

fn opt_u64_of(o: &JsonMap, k: &'static str) -> Result<Option<u64>, ClosureError> {
    let v = get(o, k)?;
    if v.is_null() {
        return Ok(None);
    }
    v.as_u64().map(Some).ok_or(ClosureError::Domain(k))
}

fn opt_f64_of(o: &JsonMap, k: &'static str) -> Result<Option<f64>, ClosureError> {
    let v = get(o, k)?;
    if v.is_null() {
        return Ok(None);
    }
    v.as_f64().map(Some).ok_or(ClosureError::Domain(k))
}

fn arr_of<'a>(o: &'a JsonMap, k: &'static str) -> Result<&'a Vec<serde_json::Value>, ClosureError> {
    get(o, k)?.as_array().ok_or(ClosureError::Domain(k))
}

fn parse_status_counts(
    v: &serde_json::Value,
    kind: IntegrityKind,
) -> Result<Option<StatusCountsRecord>, ClosureError> {
    if v.is_null() {
        return Ok(None);
    }
    let o = obj(v, "status_counts")?;
    match kind {
        IntegrityKind::JournalTsv => Ok(Some(StatusCountsRecord::Journal(
            crate::journal::StatusCounts {
                ok: u64_of(o, "OK")?,
                lost: u64_of(o, "LOST")?,
                session_open: u64_of(o, "SESSION-OPEN")?,
                session_close_completed: u64_of(o, "SESSION-CLOSE-COMPLETED")?,
                session_close_aborted: u64_of(o, "SESSION-CLOSE-ABORTED")?,
                external_cause: u64_of(o, "EXTERNAL-CAUSE")?,
            },
        ))),
        IntegrityKind::ControlTsv => Ok(Some(StatusCountsRecord::Control {
            pass: u64_of(o, "PASS")?,
            fail: u64_of(o, "FAIL")?,
        })),
        _ => Err(ClosureError::Domain(
            "status_counts must be null for this kind",
        )),
    }
}

fn parse_summary(v: &serde_json::Value) -> Result<LooSummary, ClosureError> {
    let o = obj(v, "leave_one_out summary")?;
    let omitted = {
        let raw = get(o, "omitted_session")?;
        if raw.is_null() {
            None
        } else {
            Some(
                u8::try_from(
                    raw.as_u64()
                        .ok_or(ClosureError::Domain("omitted_session"))?,
                )
                .map_err(|_| ClosureError::Domain("omitted_session"))?,
            )
        }
    };
    Ok(LooSummary {
        omitted_session: omitted,
        failure_count: u32_of(o, "failure_count")?,
        max_per_session: u32_of(o, "max_per_session")?,
        median_spread: opt_f64_of(o, "median_spread")?,
        p95_spread: opt_f64_of(o, "p95_spread")?,
        excluded_lost: u32_of(o, "excluded_lost")?,
        excluded_unwritten: u32_of(o, "excluded_unwritten")?,
        valid_spread_count: u32_of(o, "valid_spread_count")?,
    })
}

impl Closure {
    /// Strict: every field present with its declared type, every domain
    /// satisfied, and the text **exactly** what [`Closure::render`] produces —
    /// which is what makes key order, compactness, escaping and the single
    /// trailing line feed part of the check.
    pub fn parse(text: &str) -> Result<Closure, ClosureError> {
        let v: serde_json::Value =
            serde_json::from_str(text).map_err(|_| ClosureError::Domain("not JSON"))?;
        let o = obj(&v, "not a JSON object")?;
        if o.len() != CLOSURE_KEYS.len() {
            return Err(ClosureError::Domain("closure key count"));
        }

        let identity_source = IdentitySource::parse(&s_of(o, "identity_source")?)
            .ok_or(ClosureError::Domain("identity_source"))?;
        let terminal_status = RunStatus::parse(&s_of(o, "terminal_status")?)
            .ok_or(ClosureError::Domain("terminal_status"))?;

        let mut session_states = Vec::new();
        for e in arr_of(o, "session_states")? {
            if e.is_null() {
                session_states.push(None);
            } else {
                let s = e.as_str().ok_or(ClosureError::Domain("session_states"))?;
                session_states.push(Some(
                    SessionState::parse(s).ok_or(ClosureError::Domain("session_states domain"))?,
                ));
            }
        }

        let verdict_rules = {
            let raw = get(o, "verdict_rules")?;
            if raw.is_null() {
                None
            } else {
                let r = obj(raw, "verdict_rules")?;
                if r.len() != 6 {
                    return Err(ClosureError::Domain("verdict_rules key count"));
                }
                Some(VerdictRules {
                    rule1_all_completed: bool_of(r, "rule1_all_completed")?,
                    rule2_failures: u32_of(r, "rule2_failures")?,
                    rule2_pass: bool_of(r, "rule2_pass")?,
                    rule3_max_per_session: u32_of(r, "rule3_max_per_session")?,
                    rule3_pass: bool_of(r, "rule3_pass")?,
                    rule4_controls_pass: bool_of(r, "rule4_controls_pass")?,
                })
            }
        };

        let failures_total = opt_u64_of(o, "failures_total")?
            .map(|n| u32::try_from(n).map_err(|_| ClosureError::Domain("failures_total")))
            .transpose()?;
        let failures_by_session = {
            let raw = get(o, "failures_by_session")?;
            if raw.is_null() {
                None
            } else {
                let mut v = Vec::new();
                for e in raw
                    .as_array()
                    .ok_or(ClosureError::Domain("failures_by_session"))?
                {
                    v.push(
                        u32::try_from(
                            e.as_u64()
                                .ok_or(ClosureError::Domain("failures_by_session"))?,
                        )
                        .map_err(|_| ClosureError::Domain("failures_by_session"))?,
                    );
                }
                Some(v)
            }
        };

        let mut control_outcomes = Vec::new();
        for e in arr_of(o, "control_outcomes")? {
            let c = obj(e, "control_outcomes")?;
            if c.len() != 4 {
                return Err(ClosureError::Domain("control_outcomes key count"));
            }
            control_outcomes.push(ControlOutcomeRecord {
                control_id: s_of(c, "control_id")?,
                ordinal: u32_of(c, "ordinal")?,
                status: s_of(c, "status")?,
                monotonic_offset_ms: u64_of(c, "monotonic_offset_ms")?,
            });
        }

        let mut integrity = Vec::new();
        for (e, (_, kind)) in arr_of(o, "integrity")?
            .iter()
            .zip(INTEGRITY_INVENTORY.iter())
        {
            let g = obj(e, "integrity")?;
            if g.len() != 10 {
                return Err(ClosureError::Domain("integrity key count"));
            }
            let declared = s_of(g, "kind")?;
            if declared != kind.as_str() {
                return Err(ClosureError::Field {
                    what: "integrity kind",
                    value: declared,
                });
            }
            integrity.push(IntegrityEntry {
                path: s_of(g, "path")?,
                kind: *kind,
                sha256: s_of(g, "sha256")?,
                byte_count: opt_u64_of(g, "byte_count")?,
                physical_line_count: opt_u64_of(g, "physical_line_count")?,
                metadata_line_count: opt_u64_of(g, "metadata_line_count")?,
                header_line_count: opt_u64_of(g, "header_line_count")?,
                parsed_row_count: opt_u64_of(g, "parsed_row_count")?,
                data_row_count: opt_u64_of(g, "data_row_count")?,
                status_counts: parse_status_counts(get(g, "status_counts")?, *kind)?,
            });
        }

        let leave_one_out = {
            let raw = get(o, "leave_one_out")?;
            if raw.is_null() {
                None
            } else {
                let l = obj(raw, "leave_one_out")?;
                if l.len() != 2 {
                    return Err(ClosureError::Domain("leave_one_out key count"));
                }
                let mut omissions = Vec::new();
                for e in arr_of(l, "omissions")? {
                    omissions.push(parse_summary(e)?);
                }
                Some(LeaveOneOut {
                    full_90: parse_summary(get(l, "full_90")?)?,
                    omissions,
                })
            }
        };

        let artifacts = {
            let a = obj(get(o, "artifacts")?, "artifacts")?;
            if a.len() != 2 {
                return Err(ClosureError::Domain("artifacts key count"));
            }
            let pick = |k: &'static str| -> Result<Option<String>, ClosureError> {
                let raw = get(a, k)?;
                if raw.is_null() {
                    Ok(None)
                } else {
                    Ok(Some(
                        raw.as_str().ok_or(ClosureError::Domain(k))?.to_string(),
                    ))
                }
            };
            Artifacts {
                observations_sha256: pick("observations_sha256")?,
                results_md_sha256: pick("results_md_sha256")?,
            }
        };

        let c = Closure {
            schema_version: s_of(o, "schema_version")?,
            run_uuid: s_of(o, "run_uuid")?,
            boot_id: s_of(o, "boot_id")?,
            finalized_monotonic_offset_ms: opt_u64_of(o, "finalized_monotonic_offset_ms")?,
            finalized_utc: s_of(o, "finalized_utc")?,
            repo_commit: s_of(o, "repo_commit")?,
            prereg_commit: s_of(o, "prereg_commit")?,
            amendment_commits: arr_of(o, "amendment_commits")?
                .iter()
                .map(|e| {
                    e.as_str()
                        .map(|s| s.to_string())
                        .ok_or(ClosureError::Domain("amendment_commits"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            instrument_birth_commit: s_of(o, "instrument_birth_commit")?,
            host_fingerprint: s_of(o, "host_fingerprint")?,
            identity_source,
            terminal_status,
            exit_code: i32::try_from(u64_of(o, "exit_code")?)
                .map_err(|_| ClosureError::Domain("exit_code"))?,
            session_states,
            verdict_rules,
            failures_total,
            failures_by_session,
            control_outcomes,
            integrity,
            leave_one_out,
            artifacts,
        };
        c.validate()?;
        // Canonical form: key order, compactness and the single trailing LF.
        if c.render() != text {
            return Err(ClosureError::Domain("closure is not in canonical form"));
        }
        Ok(c)
    }
}

// ================================================= §C13.85 THE ARTIFACTS

/// §C13.85 — header plus data, one tab between fields, a line feed after every
/// row including the last, no metadata and no comment lines.
///
/// **Row inclusion:** exactly the rows whose journal status is `OK`. Lifecycle
/// rows, `LOST`, logical `LOST` and synthetic unwritten failures are all
/// excluded — which is why the file has no `status` column. An `OK` row whose
/// spread exceeds the bound **is present**: it is a failure for the verdict but
/// a real measurement with a real spread.
/// `journal::Phase::as_str` is private and `journal.rs` is out of this step's
/// touch-set, so the three-value mapping is restated here rather than widening
/// another module's API for one call site.
fn phase_str(p: crate::journal::Phase) -> &'static str {
    match p {
        crate::journal::Phase::A => "A",
        crate::journal::Phase::B => "B",
        crate::journal::Phase::C => "C",
    }
}

pub fn render_observations(ledger: &MeasurementLedger) -> String {
    let mut rows: Vec<&Row> = ledger
        .sessions
        .iter()
        .flat_map(|s| s.ok_rows.iter())
        .collect();
    // §C13.85: strictly ascending measurement_index; the index is global and
    // unique, so the order is total and needs no tie rule.
    rows.sort_by_key(|r| r.measurement_index);
    let mut out = OBSERVATIONS_COLUMNS.join("\t");
    out.push('\n');
    for r in rows {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            r.session,
            r.phase.map(phase_str).unwrap_or(crate::journal::NA),
            r.block.map(|b| b.to_string()).unwrap_or_default(),
            r.measurement_index
                .map(|m| m.to_string())
                .unwrap_or_default(),
            r.monotonic_offset_ms,
            // §C8.1: shortest round-trip, never fixed precision.
            r.sentinel_first_ms.map(jnum).unwrap_or_default(),
            r.sentinel_last_ms.map(jnum).unwrap_or_default(),
            r.paired_spread.map(jnum).unwrap_or_default(),
        ));
    }
    out
}

/// The Class II results document.
///
/// Amendment 1 §A2 forbids stating or implying that the protocol certifies any
/// `p`, that a PASS establishes a confidence bound, or that `N = 90` gives 95%
/// confidence of anything. This renderer states the verdict as passage of a
/// frozen engineering protocol and nothing more.
pub fn render_results_md(
    dir: &Path,
    c: &Closure,
    loo: &LeaveOneOut,
    rules: &VerdictRules,
) -> String {
    let mut s = String::new();
    s.push_str("# RC-021 — host and instrument qualification\n\n");
    s.push_str(&format!(
        "**Terminal status:** `{}`\n\n",
        c.terminal_status.as_str()
    ));
    s.push_str(match c.terminal_status {
        RunStatus::HostQualified => {
            "The host and instrument **passed the frozen engineering qualification protocol** of \
             the pre-registration, on this host configuration, on this occasion. This is not a \
             statistical certification and states nothing about a true per-measurement rate.\n\n"
        }
        _ => {
            "The host and instrument **did not pass this protocol on this host configuration**. \
             This states nothing about a true per-measurement rate, and does not mean a different \
             protocol would also reject it.\n\n"
        }
    });

    s.push_str("## Verdict rules (§7.2)\n\n");
    s.push_str("| rule | value |\n|---|---|\n");
    s.push_str(&format!(
        "| 1 — all six sessions COMPLETED | {} |\n",
        rules.rule1_all_completed
    ));
    s.push_str(&format!(
        "| 2 — failures of 90 | {} (pass: {}) |\n",
        rules.rule2_failures, rules.rule2_pass
    ));
    s.push_str(&format!(
        "| 3 — max per session | {} (pass: {}) |\n",
        rules.rule3_max_per_session, rules.rule3_pass
    ));
    s.push_str(&format!(
        "| 4 — every control passed | {} |\n\n",
        rules.rule4_controls_pass
    ));

    s.push_str("## Leave-one-session-out — descriptive only (Amendment 1 §A5)\n\n");
    s.push_str(
        "This table is descriptive. The verdict is the §7.2 predicate on all six sessions and \
         90 measurements. No cell here may change it.\n\n",
    );
    s.push_str(
        "| omitted | failures | max/session | median | p95 | excluded LOST | excluded unwritten | valid |\n\
         |---|---|---|---|---|---|---|---|\n",
    );
    let row = |x: &LooSummary| {
        format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            x.omitted_session
                .map(|v| v.to_string())
                .unwrap_or_else(|| "full 90".to_string()),
            x.failure_count,
            x.max_per_session,
            x.median_spread.map(jnum).unwrap_or_else(|| "—".into()),
            x.p95_spread.map(jnum).unwrap_or_else(|| "—".into()),
            x.excluded_lost,
            x.excluded_unwritten,
            x.valid_spread_count
        )
    };
    s.push_str(&row(&loo.full_90));
    for o in &loo.omissions {
        s.push_str(&row(o));
    }
    s.push('\n');

    s.push_str("## Provenance (§10)\n\n");
    s.push_str(&format!("- `run_uuid`: `{}`\n", c.run_uuid));
    s.push_str(&format!("- `boot_id`: `{}`\n", c.boot_id));
    s.push_str(&format!("- `repo_commit`: `{}`\n", c.repo_commit));
    s.push_str(&format!("- `prereg_commit`: `{}`\n", c.prereg_commit));
    s.push_str(&format!(
        "- `amendment_commits`: `{}`\n",
        c.amendment_commits.join("`, `")
    ));
    s.push_str(&format!(
        "- `instrument_birth_commit`: `{}`\n",
        c.instrument_birth_commit
    ));
    s.push_str(&format!("- `host_fingerprint`: `{}`\n", c.host_fingerprint));
    s.push_str(&format!(
        "- `identity_source`: `{}`\n",
        c.identity_source.as_str()
    ));
    s.push_str(&format!("- `finalized_utc`: `{}`\n", c.finalized_utc));

    if let Ok(manifest) = crate::manifest::read_manifest(&dir.join("run.json")) {
        let h = &manifest.host_fields;
        s.push_str(&format!(
            "- `init_utc`: `{}`\n- `init_command_line`: `{}`\n",
            manifest.utc_start,
            serde_json::to_string(&manifest.command_line).unwrap_or_else(|_| "[]".into())
        ));
        s.push_str(&format!(
            "- `kernel_release`: `{}`\n- `kernel_version`: `{}`\n\
             - `available_processors`: `{}`\n- `mem_total_kb`: `{}`\n\
             - `cpu_model`: `{}`\n- `cpu_set`: `{}`\n- `thread_count`: `{}`\n",
            h.kernel_release,
            h.kernel_version,
            h.available_processors,
            h.mem_total_kb,
            h.cpu_model,
            manifest.cpu_set,
            manifest.thread_count
        ));
    }

    s.push_str("\n### Invocation headers\n\n");
    s.push_str("| journal | UTC start | command line | timer resolution ms | diagnostics |\n");
    s.push_str("|---|---|---|---:|---|\n");
    let mut metadata = Vec::new();
    if let Ok(bytes) = std::fs::read(dir.join(crate::controls::CONTROL_JOURNAL)) {
        if let Ok(read) = crate::controls::read_control_journal(&bytes) {
            metadata.push(("CONTROL".to_string(), read.metadata));
        }
    }
    for (label, rel) in [
        ("P2".to_string(), crate::controls::P2_JOURNAL.to_string()),
        ("N3".to_string(), crate::controls::N3_JOURNAL.to_string()),
    ]
    .into_iter()
    .chain(
        (1..=crate::session::SESSIONS)
            .map(|session| (format!("session {session}"), session_journal_name(session))),
    ) {
        if let Ok(crate::journal::ReadOutcome::Present(read)) =
            crate::journal::read_journal(&dir.join(rel))
        {
            if let Some(meta) = read.typed_metadata {
                metadata.push((label, meta));
            }
        }
    }
    for (label, meta) in &metadata {
        let diag = format!(
            "cpu_time={} via /proc/self/stat; ctx_switches={} via /proc/self/status; \
             freq={} via sysfs cpufreq",
            meta.diag_availability.cpu_time,
            meta.diag_availability.ctx_switches,
            meta.diag_availability.freq
        );
        s.push_str(&format!(
            "| {label} | `{}` | `{}` | {} | {} |\n",
            meta.utc_start,
            serde_json::to_string(&meta.command_line).unwrap_or_else(|_| "[]".into()),
            jnum(meta.timer_resolution_ms),
            diag
        ));
    }

    s.push_str("\n### Frozen diagnostic seeds\n\n");
    for (name, seed) in crate::protocol::RC021_SEEDS {
        s.push_str(&format!("- `{name}`: `{seed}`\n"));
    }
    s.push_str(&format!(
        "- reserved band: `{}..={}`\n- per-session order seeds: `{}`\n",
        crate::protocol::RC021_BAND.0,
        crate::protocol::RC021_BAND.1,
        (1..=crate::session::SESSIONS)
            .filter_map(|session| crate::session::session_order_seed(session).ok())
            .map(|seed| seed.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    s.push_str("\n### Six measured idle intervals\n\n");
    s.push_str("| before session | observed ms | floor ms |\n|---:|---:|---:|\n");
    let complete = std::fs::read_to_string(dir.join(crate::controls::CONTROLS_COMPLETE))
        .ok()
        .and_then(|body| crate::controls::ControlsComplete::parse(&body).ok());
    let mut anchor = complete.map(|marker| marker.monotonic_offset_ms);
    for session in 1..=crate::session::SESSIONS {
        let read = crate::journal::read_journal(&dir.join(session_journal_name(session))).ok();
        let present = match read {
            Some(crate::journal::ReadOutcome::Present(read)) => Some(read),
            _ => None,
        };
        let open = present.as_ref().and_then(|read| {
            read.rows
                .iter()
                .find(|row| row.status == Status::SessionOpen)
                .map(|row| row.monotonic_offset_ms)
        });
        let gap = anchor.zip(open).and_then(|(a, o)| o.checked_sub(a));
        s.push_str(&format!(
            "| {session} | {} | {} |\n",
            gap.map(|v| v.to_string()).unwrap_or_else(|| "NA".into()),
            crate::session::MIN_GAP_MS
        ));
        anchor = present.as_ref().and_then(|read| {
            read.rows
                .iter()
                .find(|row| {
                    matches!(
                        row.status,
                        Status::SessionCloseCompleted | Status::SessionCloseAborted
                    )
                })
                .map(|row| row.monotonic_offset_ms)
        });
    }

    if let Ok(bytes) = std::fs::read(dir.join(crate::controls::CONTROL_JOURNAL)) {
        if let Ok(read) = crate::controls::read_control_journal(&bytes) {
            s.push_str("\n### Durable control provenance\n\n");
            for id in [
                crate::controls::ControlId::P6,
                crate::controls::ControlId::C10,
            ] {
                if let Some(row) = read.rows.iter().find(|row| row.control_id == id) {
                    let detail = crate::journal::json_string_decode(&row.detail)
                        .unwrap_or_else(|_| row.detail.clone());
                    s.push_str(&format!("- `{}`: {}\n", id.as_str(), detail));
                }
            }
        }
    }

    s.push_str("\n### Integrity inventory before the results file\n\n");
    s.push_str("| path | SHA-256 | bytes | rows |\n|---|---|---:|---:|\n");
    for entry in &c.integrity {
        s.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            entry.path,
            entry.sha256,
            entry
                .byte_count
                .map(|v| v.to_string())
                .unwrap_or_else(|| "NA".into()),
            entry
                .parsed_row_count
                .map(|v| v.to_string())
                .unwrap_or_else(|| "NA".into())
        ));
    }
    s.push_str(&format!("\n- `exit_status`: `{}`\n", c.exit_code));
    s.push_str(
        "\nThe canonical closure records the final SHA-256, byte count, line counters and \
         per-status counts for the frozen fifteen-file inventory, including the completed \
         results document itself.\n",
    );
    s
}

// ================================================== §C13.9 THE FINALIZER

/// The closure path's observable state. §C14.2 makes the distinction between a
/// **path** and a **valid complete closure** load-bearing.
#[derive(Clone, PartialEq, Debug)]
pub enum ClosurePathState {
    Absent,
    /// The reservation succeeded and the content did not.
    Empty,
    /// Bytes exist but are not a valid canonical closure.
    Partial(String),
    Complete(Box<Closure>),
}

/// Classify the closure path from its current bytes, and only those.
///
/// §C14.155: "No historical SHA comparison is claimed anywhere. The check is
/// the presence or absence of one path, **evaluated now**." A past syscall
/// error is not recoverable from bytes and is not reconstructed here.
pub fn closure_path_state(dir: &Path) -> ClosurePathState {
    let path = dir.join(CLOSURE_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return ClosurePathState::Absent,
        Err(e) => return ClosurePathState::Partial(e.to_string()),
    };
    if bytes.is_empty() {
        return ClosurePathState::Empty;
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return ClosurePathState::Partial("closure is not valid UTF-8".to_string());
    };
    match Closure::parse(text) {
        Ok(c) => ClosurePathState::Complete(Box::new(c)),
        Err(e) => ClosurePathState::Partial(e.to_string()),
    }
}

/// The closure file, held open from reservation to content.
trait ClosureSink {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
    fn sync_all(&mut self) -> io::Result<()>;
}

/// The durability and clock points of §C13.9, and **only** those.
///
/// This seam is private to the module by convention and carries no scientific
/// input: it cannot supply a threshold, a seed, a count, a rule set, an
/// inventory or a classifier. A test can make any one syscall fail; it can
/// never change what the instrument decides.
trait FinalizeIo {
    fn create_new_closure(&mut self, path: &Path) -> io::Result<Box<dyn ClosureSink>>;
    fn fsync_dir(&mut self, dir: &Path) -> io::Result<()>;
    fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn now_utc(&mut self) -> String;
    fn monotonic_offset_ms(
        &mut self,
        expected_boot_id: &str,
        run_start_uptime_ms: u64,
    ) -> Option<u64>;
    /// Test-only corruption proves that the production canonical-reader
    /// postcondition stands between derivation and the irreversible write.
    #[cfg(test)]
    fn intercept_closure_bytes(&mut self, body: String) -> String {
        body
    }
}

struct RealSink(std::fs::File);

impl ClosureSink for RealSink {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        std::io::Write::write_all(&mut self.0, bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        std::io::Write::flush(&mut self.0)
    }
    fn sync_all(&mut self) -> io::Result<()> {
        self.0.sync_all()
    }
}

/// Production I/O. The only implementation the instrument ever constructs.
struct RealIo;

impl FinalizeIo for RealIo {
    fn create_new_closure(&mut self, path: &Path) -> io::Result<Box<dyn ClosureSink>> {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        Ok(Box::new(RealSink(f)))
    }
    fn fsync_dir(&mut self, dir: &Path) -> io::Result<()> {
        crate::manifest::fsync_dir_at(dir).map_err(|e| io::Error::other(e.to_string()))
    }
    fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        std::io::Write::write_all(&mut f, bytes)?;
        std::io::Write::flush(&mut f)?;
        f.sync_all()?;
        if let Some(parent) = path.parent() {
            crate::manifest::fsync_dir_at(parent).map_err(|e| io::Error::other(e.to_string()))?;
        }
        Ok(())
    }
    fn now_utc(&mut self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
    fn monotonic_offset_ms(
        &mut self,
        expected_boot_id: &str,
        run_start_uptime_ms: u64,
    ) -> Option<u64> {
        // §C11.45: finalize may read the current UTC, boot_id and uptime, and
        // only to stamp the closure. Nothing here gates anything.
        crate::controls::live_monotonic_offset_ms(expected_boot_id, run_start_uptime_ms).ok()
    }
}

/// What a `--finalize` invocation ended as.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FinalizeOutcome {
    /// A closure was written; the run's terminal status is this one.
    Wrote { status: RunStatus },
    /// §C14.15: a durability point failed. The invocation reports it; the path
    /// may now be empty, partial **or complete**, and a later reader classifies
    /// from whichever bytes survived.
    DurabilityFailed { at: &'static str },
    /// §C14.2: a valid complete closure already stands.
    AlreadyComplete,
    /// §C14.2: the reservation succeeded and the content did not.
    EmptyOrPartialClosure,
    /// §C14.16: an instrument-created artifact with no closure path is external
    /// or corrupt legacy state.
    ArtifactWithoutClosure,
    /// §C10.1 Branch B: the identity cannot be asserted, so nothing is
    /// reserved and nothing is written.
    Unreconstructible { why: String },
    /// §C13.6 and §C14.1: the run never began — no `controls_started.json`, no
    /// `run_invalid.json` and no session journal — so **no closure path is
    /// reserved**. `REFUSED-BEFORE-MEASUREMENT`, exit 2, correctable.
    RefusedBeforeMeasurement,
    /// §C14.15: the derived closure failed its own canonical reader, so its
    /// bytes were never written. The reservation stands, empty, and locks the
    /// run: `JOURNAL-INVALID`, exit 4.
    DerivedClosureInvalid,
    /// §C13.6: `--run-id R` names a different run than the durable identity
    /// does. Nothing is written and the invocation is correctable, so it is
    /// `REFUSED-BEFORE-MEASUREMENT`, exit 2.
    ///
    /// It is tested **after** the terminal lock and the §C14.1 obligation, so
    /// a wrong `--run-id` can never downgrade a locked, broken run's exit 4.
    RunIdMismatch,
}

impl FinalizeOutcome {
    pub fn variant_name(&self) -> &'static str {
        match self {
            FinalizeOutcome::Wrote { .. } => "Wrote",
            FinalizeOutcome::DurabilityFailed { .. } => "DurabilityFailed",
            FinalizeOutcome::AlreadyComplete => "AlreadyComplete",
            FinalizeOutcome::EmptyOrPartialClosure => "EmptyOrPartialClosure",
            FinalizeOutcome::ArtifactWithoutClosure => "ArtifactWithoutClosure",
            FinalizeOutcome::Unreconstructible { .. } => "Unreconstructible",
            FinalizeOutcome::RefusedBeforeMeasurement => "RefusedBeforeMeasurement",
            FinalizeOutcome::DerivedClosureInvalid => "DerivedClosureInvalid",
            FinalizeOutcome::RunIdMismatch => "RunIdMismatch",
        }
    }
    /// The variant plus what it actually decided, for the operator.
    ///
    /// `variant_name()` is `"Wrote"` for every terminal status and
    /// `"DurabilityFailed"` for every durability point, so printing it alone
    /// discards the run's verdict and the failing step.
    pub fn describe(&self) -> String {
        match self {
            FinalizeOutcome::Wrote { status } => format!("Wrote: {}", status.as_str()),
            FinalizeOutcome::DurabilityFailed { at } => format!("DurabilityFailed at {at}"),
            // §C10.1 Branch B names *why* the identity could not be rebuilt.
            // Discarding it left exit 4 with no account of what was damaged.
            FinalizeOutcome::Unreconstructible { why } => format!("Unreconstructible: {why}"),
            other => other.variant_name().to_string(),
        }
    }

    /// Every finalize outcome ends the run, so every one is terminal.
    pub fn is_terminal(&self) -> bool {
        self.terminal_run_status().is_some()
    }
    pub fn terminal_run_status(&self) -> Option<RunStatus> {
        Some(match self {
            FinalizeOutcome::Wrote { status } => *status,
            FinalizeOutcome::DurabilityFailed { .. } => RunStatus::JournalInvalid,
            FinalizeOutcome::AlreadyComplete => RunStatus::RefusedBeforeMeasurement,
            FinalizeOutcome::EmptyOrPartialClosure => RunStatus::JournalInvalid,
            FinalizeOutcome::ArtifactWithoutClosure => RunStatus::JournalInvalid,
            FinalizeOutcome::Unreconstructible { .. } => RunStatus::JournalInvalid,
            // §8's own code, never an invented literal.
            FinalizeOutcome::RefusedBeforeMeasurement => RunStatus::RefusedBeforeMeasurement,
            FinalizeOutcome::DerivedClosureInvalid => RunStatus::JournalInvalid,
            FinalizeOutcome::RunIdMismatch => RunStatus::RefusedBeforeMeasurement,
        })
    }
    /// The code, **only** through §8.
    pub fn terminal_exit_code(&self) -> Option<i32> {
        self.terminal_run_status().map(RunStatus::exit_code)
    }
}

/// The frozen inventory P3 checks `sample_finalize_outcomes` against.
pub const FINALIZE_VARIANTS: [&str; 9] = [
    "Wrote",
    "DurabilityFailed",
    "AlreadyComplete",
    "EmptyOrPartialClosure",
    "ArtifactWithoutClosure",
    "Unreconstructible",
    "RefusedBeforeMeasurement",
    "DerivedClosureInvalid",
    "RunIdMismatch",
];

/// Whether an instrument-written artifact exists.
fn any_artifact_present(dir: &Path) -> bool {
    // `path_present`, not `.exists()`: the module's doctrine is that an
    // unreadable path is not an absent one. `.exists()` maps EACCES/EIO to
    // `false`, which would skip §C14.16's guard, reserve the closure path —
    // locking the run irreversibly — and only then fail the artifact write.
    [OBSERVATIONS_FILE, RESULTS_FILE]
        .iter()
        .any(|f| path_present(&dir.join(f)))
}

/// Read the durable control evidence: the first `FAIL` row by ordinal, and the
/// twelve rows that exist.
fn read_control_evidence(
    dir: &Path,
    identity: &RunIdentity,
) -> (Vec<ControlOutcomeRecord>, TerminalEvidenceControls) {
    let path = dir.join("control/rc021_control_journal.tsv");
    let Ok(bytes) = std::fs::read(&path) else {
        return (
            Vec::new(),
            TerminalEvidenceControls {
                first_failure: None,
                unreadable: true,
                all_pass: false,
                sha256: None,
                row_count: 0,
            },
        );
    };
    let read = match crate::controls::read_control_journal(&bytes) {
        Ok(r) => r,
        Err(_) => {
            return (
                Vec::new(),
                TerminalEvidenceControls {
                    first_failure: None,
                    unreadable: true,
                    all_pass: false,
                    sha256: None,
                    row_count: 0,
                },
            )
        }
    };
    if RunIdentity::from_metadata(&read.metadata) != *identity {
        return (
            Vec::new(),
            TerminalEvidenceControls {
                first_failure: None,
                unreadable: true,
                all_pass: false,
                sha256: Some(crate::host::sha256_hex(&bytes)),
                row_count: read.rows.len() as u32,
            },
        );
    }
    let rows = read.rows;
    let records: Vec<ControlOutcomeRecord> = rows
        .iter()
        .map(|r| ControlOutcomeRecord {
            control_id: r.control_id.as_str().to_string(),
            ordinal: r.ordinal,
            status: r.status.as_str().to_string(),
            monotonic_offset_ms: r.monotonic_offset_ms,
        })
        .collect();
    // §C11.25 / §C13.6: the class of the **first `FAIL` row by ordinal**.
    let first_failure = rows
        .iter()
        .find(|r| r.status == crate::controls::ControlStatus::Fail)
        .map(|r| r.control_id.failure_class());
    let all_pass = rows.len() == crate::controls::CONTROL_COUNT as usize
        && rows
            .iter()
            .all(|r| r.status == crate::controls::ControlStatus::Pass);
    // §C13.6: an incomplete PASS prefix contains no distinguishing failure
    // evidence. It is the generic INSTRUMENT-INVALID case, not a Class-II
    // HOST-NOT-QUALIFIED verdict.
    let unreadable = first_failure.is_none() && !all_pass;
    (
        records,
        TerminalEvidenceControls {
            first_failure,
            unreadable,
            all_pass,
            sha256: Some(crate::host::sha256_hex(&bytes)),
            row_count: rows.len() as u32,
        },
    )
}

struct TerminalEvidenceControls {
    first_failure: Option<crate::controls::FailureClass>,
    unreadable: bool,
    all_pass: bool,
    sha256: Option<String>,
    row_count: u32,
}

/// §8.2's whole input, read once from bytes.
///
/// **Sole gatherer.** Finalize and verify both call this and then the single
/// [`classify_terminal`]; neither keeps a second copy of a classification rule,
/// so the two can no longer drift apart on a state only one of them models.
fn gather_terminal_evidence(
    dir: &Path,
    identity: &RunIdentity,
) -> (
    Vec<ControlOutcomeRecord>,
    MeasurementLedger,
    TerminalEvidence,
) {
    let ledger = read_ledger(dir, identity);
    let (control_records, controls) = read_control_evidence(dir, identity);
    let started = read_controls_started(dir, identity);
    let complete = read_controls_complete(dir, identity, &controls);
    // Once the run has begun, Class II is possible only after both canonical,
    // bound markers exist. A missing complete marker with an explicit FAIL is
    // still classified by that row because control precedence comes first.
    let marker_unusable = started != MarkerState::Bound || complete != MarkerState::Bound;
    let evidence = TerminalEvidence {
        first_control_failure: controls.first_failure,
        control_evidence_unreadable: controls.unreadable,
        run_invalid: read_run_invalid_evidence(dir),
        marker_unusable,
        ledger: ledger.clone(),
        controls_all_pass: controls.all_pass,
    };
    (control_records, ledger, evidence)
}

/// §C13.6 and §C14.1 — may this directory be finalized at all?
///
/// A closure is obligatory for a run that "reached `controls_started` **or**
/// created at least one session journal" (§C14.1), and §C12.3 makes
/// `run_invalid.json` a third form of began-and-stopped evidence. A directory
/// with none of the three holds a run that never began: §C14.1 says such a
/// preflight failure "does not create a closure", so the check runs **before**
/// the reservation, not after it.
///
/// Presence, not validity, is what opens the gate — a *damaged* marker means
/// the run began and is `INSTRUMENT-INVALID` under the §C13 damage table, which
/// is a closure-writing outcome. Validity is classified later, by the strict
/// readers, through [`classify_terminal`].
fn run_began(dir: &Path) -> bool {
    if path_present(&dir.join(crate::controls::CONTROLS_STARTED))
        || path_present(&dir.join(crate::session::RUN_INVALID_FILE))
    {
        return true;
    }
    (1..=crate::session::SESSIONS).any(|s| path_present(&dir.join(session_journal_name(s))))
}

/// Read and classify all six session journals.
pub fn read_ledger(dir: &Path, identity: &RunIdentity) -> MeasurementLedger {
    let mut sessions = Vec::with_capacity(crate::session::SESSIONS as usize);
    for s in 1..=crate::session::SESSIONS {
        match crate::journal::read_journal(&dir.join(session_journal_name(s))) {
            Ok(outcome) => sessions.push(classify_session(s, &outcome, identity)),
            Err(_) => sessions.push(SessionClassification {
                session: s,
                state: None,
                journal_invalid: true,
                ok_rows: Vec::new(),
                over_bound: 0,
                physical_lost: 0,
                logical_lost: 0,
                unwritten: crate::session::MEASUREMENTS_PER_SESSION,
                external_cause_before_abort: false,
            }),
        }
    }
    MeasurementLedger::new(sessions)
}

/// §C13.9: whether the artifacts on disk are the ones a Class II publication
/// requires, judged from the bytes alone and with no historical record.
///
/// §C13.8: observations exist exactly when the result is Class II and all six
/// sessions are `COMPLETED`; the results Markdown exists for both Class II
/// verdicts.
///
/// **The two artifacts are checked to different depths, deliberately.**
/// Observations are a pure function of the ledger, so they are compared
/// byte-for-byte against a fresh render. The Markdown is not reproducible at
/// verify time — it embeds `finalized_utc` and the integrity snapshot as it
/// stood before the Markdown itself existed — so only its **presence** is
/// checked here. A Markdown that exists with the wrong bytes is therefore not
/// caught by this predicate; it is caught by `artifacts.results_md_sha256`,
/// which verify recomputes from the file.
pub fn artifacts_consistent(dir: &Path, ledger: &MeasurementLedger, status: RunStatus) -> bool {
    if !status.is_class_two() {
        return true;
    }
    let obs = dir.join(OBSERVATIONS_FILE);
    if ledger.all_completed() {
        let Ok(bytes) = std::fs::read(&obs) else {
            return false;
        };
        if bytes != render_observations(ledger).as_bytes() {
            return false;
        }
    } else if path_present(&obs) {
        return false;
    }
    // `path_present`, not `.exists()`: the module's doctrine is that an
    // unreadable path is not an absent one, and `.exists()` maps EACCES to
    // `false` — blessing a closure whose artifact state was never observed.
    path_present(&dir.join(RESULTS_FILE))
}

/// §C13.9 — the frozen artifact write order.
///
/// ```text
/// 1. read-only preflight and identity   — write nothing
/// 2. create_new rc021_closure.json      — EMPTY; fsync parent; keep the handle
/// 3. derive the remaining classification
/// 4. observations TSV                   (only when §C13.8 permits)
/// 5. results Markdown                   (Class II only)
/// 6. SHA-256 and byte_count of both
/// 7. write_all the complete closure through the already-open handle
/// 8. flush → sync_all → fsync the parent directory
/// ```
///
/// **Durability semantics, §C14.15.** A failure at any point from step 2 onward
/// makes *this invocation* report `JOURNAL-INVALID`. The path then remains and
/// permanently blocks a rewrite. Its observable state may be empty, partial
/// **or complete** — §C14.15 names all three — and a later `--verify`
/// classifies from the bytes that survived, never from a past syscall error,
/// which is not recorded and not reconstructed.
/// `run_id` is `--run-id R` from §C13.6's precondition row.
///
/// **The whole precondition order lives here and nowhere else.** An earlier
/// CLI wrapper checked the run identity before calling this function, which
/// put the run-id test ahead of the §C14.2 terminal lock and the §C14.1
/// closure obligation; on a directory holding no run at all that returned
/// `JOURNAL-INVALID` where §C13.6 requires `REFUSED-BEFORE-MEASUREMENT`. One
/// owner, one order.
pub fn finalize(dir: &Path, run_id: &str) -> FinalizeOutcome {
    finalize_with_io(dir, run_id, &mut RealIo)
}

fn finalize_with_io(dir: &Path, run_id: &str, io: &mut dyn FinalizeIo) -> FinalizeOutcome {
    // ---- step 1: read-only preflight ------------------------------------
    // §C14.2 / §C14.16: the terminal lock comes first, before any derivation.
    match closure_path_state(dir) {
        ClosurePathState::Complete(_) => return FinalizeOutcome::AlreadyComplete,
        ClosurePathState::Empty | ClosurePathState::Partial(_) => {
            return FinalizeOutcome::EmptyOrPartialClosure
        }
        ClosurePathState::Absent => {
            if any_artifact_present(dir) {
                // §C14.16: under reservation-first this cannot be a stage of a
                // run this instrument conducted.
                return FinalizeOutcome::ArtifactWithoutClosure;
            }
        }
    }

    // §C13.6 / §C14.1: a run that never began earns no closure, and the check
    // must precede the reservation — a refusal that had already claimed the
    // closure path would lock a correctable run forever.
    if !run_began(dir) {
        return FinalizeOutcome::RefusedBeforeMeasurement;
    }

    let (identity, source, manifest_damaged) = match establish_identity(dir) {
        IdentityOutcome::Established {
            identity,
            source,
            manifest_damaged,
        } => (identity, source, manifest_damaged),
        // §C13.9: Branch B exits **without reserving the closure path**.
        IdentityOutcome::Unreconstructible(why) => {
            return FinalizeOutcome::Unreconstructible { why }
        }
    };
    // §C13.6: `R` must name this run. Last of the read-only preconditions and
    // still before the reservation, so a mistyped argument writes nothing.
    if identity.run_uuid != run_id {
        return FinalizeOutcome::RunIdMismatch;
    }

    // ---- step 2: reserve the path, empty --------------------------------
    let closure_path = dir.join(CLOSURE_FILE);
    let mut sink = match io.create_new_closure(&closure_path) {
        Ok(s) => s,
        Err(_) => return FinalizeOutcome::DurabilityFailed { at: "create_new" },
    };
    // ---- step 3 (first half): fsync the parent, keep the handle ---------
    if io.fsync_dir(dir).is_err() {
        return FinalizeOutcome::DurabilityFailed {
            at: "fsync_parent_after_reservation",
        };
    }

    // ---- step 3: derive terminal and scientific classification ----------
    // One clock read for this whole finalization.
    let stamp = FinalizeStamp::take(io, &identity);
    let (control_records, ledger, evidence) = gather_terminal_evidence(dir, &identity);
    let (mut status, mut scientific) = classify_terminal(&evidence);
    // §C10.1: a damaged but reconstructible manifest is JOURNAL-INVALID, and no
    // verdict may be published from it.
    if manifest_damaged {
        status = RunStatus::JournalInvalid;
        scientific = None;
    }

    // ---- steps 4 and 5: the artifacts -----------------------------------
    let mut artifacts = Artifacts::default();
    let mut artifact_failed = false;
    // §C13.8: observations only for a valid Class II result with all six
    // sessions COMPLETED and valid controls and journals.
    let observations_permitted = status.is_class_two() && ledger.all_completed();
    if observations_permitted {
        let body = render_observations(&ledger);
        if io
            .write_artifact(&dir.join(OBSERVATIONS_FILE), body.as_bytes())
            .is_err()
        {
            artifact_failed = true;
        }
    }
    if status.is_class_two() && !artifact_failed {
        if let Some((rules, loo)) = scientific.as_ref() {
            // One stamp for both. `build_closure` reads the clock, so calling
            // it twice lets the published Markdown and the canonical closure
            // disagree about when the run was finalized whenever the artifact
            // writes straddle a second — and `verify` cannot catch it, because
            // it hashes the Markdown without re-reading its content.
            let preview = build_closure(
                &identity,
                source,
                status,
                &ledger,
                &control_records,
                Some((*rules, loo.clone())),
                Artifacts::default(),
                &stamp,
                dir,
            );
            let body = render_results_md(dir, &preview, loo, rules);
            if io
                .write_artifact(&dir.join(RESULTS_FILE), body.as_bytes())
                .is_err()
            {
                artifact_failed = true;
            }
        }
    }
    // §C13.9: an artifact failure makes the run JOURNAL-INVALID and publishes no
    // Class II verdict, while `integrity` still records the actual bytes.
    if artifact_failed {
        status = RunStatus::JournalInvalid;
        scientific = None;
    }

    // ---- step 6: hash and count the artifacts that exist ----------------
    if status.is_class_two() {
        artifacts.observations_sha256 = file_sha256(&dir.join(OBSERVATIONS_FILE));
        artifacts.results_md_sha256 = file_sha256(&dir.join(RESULTS_FILE));
    }

    let closure = build_closure(
        &identity,
        source,
        status,
        &ledger,
        &control_records,
        scientific,
        artifacts,
        &stamp,
        dir,
    );

    // ---- step 7: write the complete closure through the open handle -----
    let body = closure.render();
    #[cfg(test)]
    let body = io.intercept_closure_bytes(body);
    // §C14.15: "only a **valid, complete** closure means finalization
    // succeeded", and §C14.16 makes the reservation un-retryable — so the bytes
    // are put through the canonical reader *before* they become the terminal
    // record. `parse` and not `validate`: parse re-renders and compares, so key
    // order and serialisation are covered as well as the domains. On rejection
    // nothing is written and the reservation stands, empty, as §C14.15's
    // terminal evidence.
    if Closure::parse(&body).is_err() {
        return FinalizeOutcome::DerivedClosureInvalid;
    }
    if sink.write_all(body.as_bytes()).is_err() {
        return FinalizeOutcome::DurabilityFailed { at: "write_all" };
    }
    // ---- step 8: flush → sync_all → fsync the parent --------------------
    if sink.flush().is_err() {
        return FinalizeOutcome::DurabilityFailed { at: "flush" };
    }
    if sink.sync_all().is_err() {
        return FinalizeOutcome::DurabilityFailed { at: "sync_all" };
    }
    if io.fsync_dir(dir).is_err() {
        return FinalizeOutcome::DurabilityFailed {
            at: "fsync_parent_after_write",
        };
    }
    FinalizeOutcome::Wrote { status }
}

fn file_sha256(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|b| crate::host::sha256_hex(&b))
}

/// §C14.155: `finalized_utc` and the final monotonic offset are **provenance**,
/// not results — they differ between invocations by construction. Within one
/// invocation they must not: the closure and the results Markdown that
/// describes it are one act of finalization, and reading the clock once per
/// call let them disagree whenever the artifact writes straddled a second.
/// `verify` could not catch that, since it hashes the Markdown without
/// re-reading its content.
#[derive(Clone)]
struct FinalizeStamp {
    utc: String,
    monotonic_offset_ms: Option<u64>,
}

impl FinalizeStamp {
    fn take(io: &mut dyn FinalizeIo, identity: &RunIdentity) -> FinalizeStamp {
        FinalizeStamp {
            monotonic_offset_ms: io
                .monotonic_offset_ms(&identity.boot_id, identity.run_start_uptime_ms),
            utc: io.now_utc(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_closure(
    identity: &RunIdentity,
    source: IdentitySource,
    status: RunStatus,
    ledger: &MeasurementLedger,
    control_records: &[ControlOutcomeRecord],
    scientific: Option<(VerdictRules, LeaveOneOut)>,
    artifacts: Artifacts,
    stamp: &FinalizeStamp,
    dir: &Path,
) -> Closure {
    let (verdict_rules, leave_one_out, failures_total, failures_by_session) = match scientific {
        Some((r, l)) => (
            Some(r),
            Some(l),
            Some(ledger.failures_total()),
            Some(ledger.failures_by_session()),
        ),
        // §C13.45: a Class I closure nulls every derived scientific quantity.
        None => (None, None, None, None),
    };
    Closure {
        schema_version: crate::journal::SCHEMA_VERSION.to_string(),
        run_uuid: identity.run_uuid.clone(),
        boot_id: identity.boot_id.clone(),
        finalized_monotonic_offset_ms: stamp.monotonic_offset_ms,
        finalized_utc: stamp.utc.clone(),
        repo_commit: identity.repo_commit.clone(),
        prereg_commit: identity.prereg_commit.clone(),
        amendment_commits: identity.amendment_commits.clone(),
        instrument_birth_commit: identity.instrument_birth_commit.clone(),
        host_fingerprint: identity.host_fingerprint.clone(),
        identity_source: source,
        terminal_status: status,
        exit_code: status.exit_code(),
        session_states: ledger.sessions.iter().map(|s| s.state).collect(),
        verdict_rules,
        failures_total,
        failures_by_session,
        control_outcomes: control_records.to_vec(),
        integrity: build_integrity(dir),
        leave_one_out,
        artifacts,
    }
}

// ==================================================== §C14.3 THE VERIFIER

/// What a `--verify` invocation ended as.
///
/// §C14.2 gives verify exactly two documented outcomes: a valid complete
/// closure "verifies normally", and "any mismatch exits 4". It never assigns
/// verify an exit of 1, 2, 3 or 5 — so a successful verification is a **mode
/// success**, not a new run-level terminal status. Verify confirms a verdict;
/// it does not publish one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VerifyOutcome {
    /// Continuation: mode success, whatever class the stored closure carries.
    Verified,
    Mismatch {
        what: String,
    },
    EmptyOrPartial,
    DamagedManifest,
    /// The precondition "a closure path exists" is not met. Step 8 decides how
    /// a CLI reports this; §C14 assigns it no §8 status, so none is invented.
    NoClosurePath,
}

impl VerifyOutcome {
    pub fn variant_name(&self) -> &'static str {
        match self {
            VerifyOutcome::Verified => "Verified",
            VerifyOutcome::Mismatch { .. } => "Mismatch",
            VerifyOutcome::EmptyOrPartial => "EmptyOrPartial",
            VerifyOutcome::DamagedManifest => "DamagedManifest",
            VerifyOutcome::NoClosurePath => "NoClosurePath",
        }
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal_run_status().is_some()
    }
    /// `Verified` and `NoClosurePath` carry no §8 status: the first is a mode
    /// success, the second an unmet precondition Step 8 will surface.
    pub fn terminal_run_status(&self) -> Option<RunStatus> {
        match self {
            VerifyOutcome::Verified => None,
            VerifyOutcome::NoClosurePath => None,
            VerifyOutcome::Mismatch { .. }
            | VerifyOutcome::EmptyOrPartial
            | VerifyOutcome::DamagedManifest => Some(RunStatus::JournalInvalid),
        }
    }
    pub fn terminal_exit_code(&self) -> Option<i32> {
        self.terminal_run_status().map(RunStatus::exit_code)
    }
    /// The mode's own code for a successful verification.
    pub fn mode_exit_code(&self) -> Option<i32> {
        match self {
            VerifyOutcome::Verified => Some(crate::controls::MODE_SUCCESS_EXIT_CODE),
            _ => None,
        }
    }
}

pub const VERIFY_VARIANTS: [&str; 5] = [
    "Verified",
    "Mismatch",
    "EmptyOrPartial",
    "DamagedManifest",
    "NoClosurePath",
];

fn mismatch(what: &str) -> VerifyOutcome {
    VerifyOutcome::Mismatch {
        what: what.to_string(),
    }
}

/// §C14.3 — strictly read-only.
///
/// Opens nothing for writing, repairs nothing, regenerates nothing and reads no
/// live host state. `finalized_utc` and `finalized_monotonic_offset_ms` are
/// **stored provenance** (§C14.155) and are never recomputed.
impl VerifyOutcome {
    /// The variant plus what it found. `Mismatch { what }` names the inventory
    /// entry that failed — the single most useful fact `--verify` produces, and
    /// the one an operator has no other way to learn.
    pub fn describe(&self) -> String {
        match self {
            VerifyOutcome::Mismatch { what } => format!("Mismatch: {what}"),
            other => other.variant_name().to_string(),
        }
    }
}

pub fn verify(dir: &Path) -> VerifyOutcome {
    let closure = match closure_path_state(dir) {
        ClosurePathState::Absent => return VerifyOutcome::NoClosurePath,
        // §C14.2: only a valid complete closure asserts that a finalization
        // actually completed.
        ClosurePathState::Empty | ClosurePathState::Partial(_) => {
            return VerifyOutcome::EmptyOrPartial
        }
        ClosurePathState::Complete(c) => *c,
    };

    // §C14.3: a damaged or unreadable manifest is exit 4 — verification cannot
    // proceed against a run whose identity cannot be established.
    let identity = match establish_identity(dir) {
        IdentityOutcome::Established { identity, .. } => identity,
        IdentityOutcome::Unreconstructible(why) => {
            return VerifyOutcome::Mismatch {
                what: format!("identity is unreconstructible: {why}"),
            }
        }
    };
    if closure.identity_source == IdentitySource::Manifest
        && crate::manifest::read_manifest(&dir.join("run.json")).is_err()
    {
        return VerifyOutcome::DamagedManifest;
    }
    for (what, a, b) in [
        ("run_uuid", &closure.run_uuid, &identity.run_uuid),
        ("boot_id", &closure.boot_id, &identity.boot_id),
        ("repo_commit", &closure.repo_commit, &identity.repo_commit),
        (
            "prereg_commit",
            &closure.prereg_commit,
            &identity.prereg_commit,
        ),
        (
            "instrument_birth_commit",
            &closure.instrument_birth_commit,
            &identity.instrument_birth_commit,
        ),
        (
            "host_fingerprint",
            &closure.host_fingerprint,
            &identity.host_fingerprint,
        ),
    ] {
        if a != b {
            return mismatch(what);
        }
    }
    if closure.amendment_commits != identity.amendment_commits {
        return mismatch("amendment_commits");
    }

    // Always recomputed, for every class: the full inventory.
    let integrity = build_integrity(dir);
    if integrity.len() != closure.integrity.len() {
        return mismatch("integrity length");
    }
    for (a, b) in closure.integrity.iter().zip(integrity.iter()) {
        if a != b {
            return mismatch(&format!("integrity entry {}", b.path));
        }
    }

    // Always recomputed: session states, control outcomes, terminal status —
    // through the same gatherer and the same classifier finalize used.
    let (control_records, ledger, evidence) = gather_terminal_evidence(dir, &identity);
    let states: Vec<Option<SessionState>> = ledger.sessions.iter().map(|s| s.state).collect();
    if states != closure.session_states {
        return mismatch("session_states");
    }
    if control_records != closure.control_outcomes {
        return mismatch("control_outcomes");
    }
    let (mut status, mut scientific) = classify_terminal(&evidence);
    // §C13.9: an artifact that is absent or not the canonical bytes means the
    // artifact write failed, which makes the run JOURNAL-INVALID and publishes
    // no Class II verdict. Finalize saw the failure; a reader sees the bytes.
    if !artifacts_consistent(dir, &ledger, status) {
        status = RunStatus::JournalInvalid;
        scientific = None;
    }
    if closure.identity_source == IdentitySource::ReconstructedHeaders {
        // §C10.1: the run was finalized from reconstructed identity, which is
        // JOURNAL-INVALID by construction.
        status = RunStatus::JournalInvalid;
    }
    if status != closure.terminal_status {
        return mismatch("terminal_status");
    }
    if closure.exit_code != closure.terminal_status.exit_code() {
        return mismatch("exit_code");
    }

    if closure.terminal_status.is_class_two() {
        // Recomputed only for Class II.
        let Some((rules, loo)) = scientific else {
            return mismatch("verdict_rules");
        };
        if Some(rules) != closure.verdict_rules {
            return mismatch("verdict_rules");
        }
        if Some(ledger.failures_total()) != closure.failures_total {
            return mismatch("failures_total");
        }
        if Some(ledger.failures_by_session()) != closure.failures_by_session {
            return mismatch("failures_by_session");
        }
        if Some(&loo) != closure.leave_one_out.as_ref() {
            return mismatch("leave_one_out");
        }
        let observations = file_sha256(&dir.join(OBSERVATIONS_FILE));
        if observations != closure.artifacts.observations_sha256 {
            return mismatch("artifacts.observations_sha256");
        }
        let results = file_sha256(&dir.join(RESULTS_FILE));
        if results != closure.artifacts.results_md_sha256 {
            return mismatch("artifacts.results_md_sha256");
        }
    } else {
        // §C14.3: for Class I the four scientific fields are asserted null.
        if closure.verdict_rules.is_some()
            || closure.failures_total.is_some()
            || closure.failures_by_session.is_some()
            || closure.leave_one_out.is_some()
        {
            return mismatch("a Class I closure carries a scientific field");
        }
    }
    VerifyOutcome::Verified
}

// ======================================================================= TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{Metadata, Phase, RowContext};
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// Every fixture manifest and every fixture journal header carries this
    /// run id, so it is the one `--run-id R` a fixture can legally present.
    const TEST_RUN_ID: &str = "00000000000000000000000000000000";

    /// The identity every fixture in this module writes into its manifest and
    /// its journal headers.
    fn fixture_identity(dir: &Path) -> RunIdentity {
        RunIdentity::from_manifest(&manifest(dir))
    }

    /// §C17: a unique directory from the process id, an atomic counter and the
    /// test name, with a cleanup guard. Any other scheme reintroduces
    /// retry-until-pass through a back door.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(name: &str) -> TempDir {
            let p = std::env::temp_dir().join(format!(
                "rc021_decision_{}_{}_{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::SeqCst),
                name
            ));
            std::fs::create_dir_all(p.join("control")).unwrap();
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

    /// A byte-exact recursive snapshot, so "wrote nothing" means the contents.
    fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
        fn walk(p: &Path, base: &Path, out: &mut Vec<(String, Vec<u8>)>) {
            let Ok(rd) = std::fs::read_dir(p) else { return };
            for e in rd.flatten() {
                let path = e.path();
                let rel = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                if path.is_dir() {
                    out.push((rel, Vec::new()));
                    walk(&path, base, out);
                } else {
                    out.push((rel, std::fs::read(&path).unwrap_or_default()));
                }
            }
        }
        let mut v = Vec::new();
        walk(root, root, &mut v);
        v.sort();
        v
    }

    fn host_fields() -> crate::host::HostFields {
        crate::host::HostFields {
            kernel_release: "6.6.0".into(),
            kernel_version: "#1 SMP".into(),
            available_processors: "4".into(),
            mem_total_kb: "10185860".into(),
            cpus_allowed_list: "0-3".into(),
            cpu_model: "Test CPU".into(),
        }
    }

    fn manifest(dir: &Path) -> crate::manifest::RunManifest {
        let hf = host_fields();
        crate::manifest::RunManifest {
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
            cpu_set: "0-3".into(),
            thread_count: 1,
            timer_resolution_ms: 0.00002,
            cpu_time_unit: crate::journal::CPU_TIME_UNIT.into(),
            command_line: vec!["exp_rc021_host_qualify".into()],
            utc_start: "2026-08-27T00:00:00Z".into(),
            diag_availability: crate::host::DiagProbe {
                cpu_time: true,
                ctx_switches: true,
                freq: false,
            },
        }
    }

    fn meta(session: crate::journal::MetaSession) -> Metadata {
        let hf = host_fields();
        Metadata {
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            run_start_uptime_ms: 1_000,
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            amendment_commits: vec!["c".repeat(40), "d".repeat(40)],
            instrument_birth_commit: "e".repeat(40),
            host_fingerprint: hf.fingerprint(),
            cpu_set: "0-3".into(),
            thread_count: 1,
            timer_resolution_ms: 0.00002,
            cpu_time_unit: crate::journal::CPU_TIME_UNIT.into(),
            command_line: vec!["exp_rc021_host_qualify".into()],
            utc_start: "2026-08-27T00:00:00Z".into(),
            session,
            diag_availability: crate::journal::DiagAvailability {
                cpu_time: true,
                ctx_switches: true,
                freq: false,
            },
        }
    }

    fn ctx() -> RowContext {
        let hf = host_fields();
        RowContext {
            run_uuid: "0".repeat(32),
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            host_fingerprint: hf.fingerprint(),
            timer_resolution_ms: 0.00002,
            cpu_set: "0-3".into(),
            thread_count: 1,
        }
    }

    /// An `OK` measurement row at the given coordinate and spread.
    fn ok_row(session: u8, phase: Phase, block: u8, spread: f64) -> crate::journal::Row {
        let c = ctx();
        let mut r = c.lost(
            session,
            phase,
            block,
            Some(crate::session::measurement_index(session, phase, block).unwrap()),
            1_000,
            "",
        );
        r.status = Status::Ok;
        r.sentinel_first_ms = Some(2.0);
        r.sentinel_last_ms = Some(2.0 * (1.0 + spread));
        r.paired_spread = Some(spread);
        r.load_avg_start = Some(0.1);
        r.load_avg_end = Some(0.1);
        r
    }

    /// Write one session journal: SESSION-OPEN, `spreads.len()` OK rows at the
    /// planned coordinates, then a close row when asked.
    fn write_session(dir: &Path, session: u8, spreads: &[f64], completed: bool) {
        let path = dir.join(session_journal_name(session));
        let m = meta(crate::journal::MetaSession::Qualification(session));
        let c = ctx();
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(session, 10, 0.1)).unwrap();
        let plan = crate::session::plan_session(session).unwrap();
        for (coord, spread) in plan.iter().zip(spreads.iter()) {
            j.append(&ok_row(session, coord.phase, coord.block, *spread))
                .unwrap();
        }
        if completed {
            j.append(&c.session_close(session, 20, true, 0.1)).unwrap();
        }
    }

    /// Six sessions, every measurement within the bound.
    fn write_clean_run(dir: &Path) {
        let within = vec![0.01f64; crate::session::MEASUREMENTS_PER_SESSION as usize];
        for s in 1..=crate::session::SESSIONS {
            write_session(dir, s, &within, true);
        }
    }

    /// §C13.2's canonical marker, carrying this fixture's identity. Earlier
    /// fixtures wrote `{}` here, which the strict reader now — correctly —
    /// classifies as a damaged marker.
    fn controls_started_record() -> crate::controls::ControlsStarted {
        crate::controls::ControlsStarted {
            schema_version: crate::journal::SCHEMA_VERSION.into(),
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            monotonic_offset_ms: 50,
            utc: "2026-08-27T00:00:00Z".into(),
            command_line: vec!["exp_rc021_host_qualify".into()],
        }
    }

    fn write_controls_started(dir: &Path) {
        std::fs::write(
            dir.join(crate::controls::CONTROLS_STARTED),
            controls_started_record().render(),
        )
        .unwrap();
    }

    /// §C13.3's canonical marker. Written after the control journal, because it
    /// carries that journal's SHA-256.
    fn write_controls_complete(dir: &Path) {
        let bytes = std::fs::read(dir.join("control/rc021_control_journal.tsv")).unwrap();
        let r = crate::controls::ControlsComplete {
            schema_version: crate::journal::SCHEMA_VERSION.into(),
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            monotonic_offset_ms: 200,
            utc: "2026-08-27T00:00:00Z".into(),
            control_journal_sha256: crate::host::sha256_hex(&bytes),
            control_count: crate::controls::CONTROL_COUNT,
            all_pass: true,
        };
        std::fs::write(dir.join(crate::controls::CONTROLS_COMPLETE), r.render()).unwrap();
    }

    fn write_passing_controls(dir: &Path) {
        write_control_prefix(
            dir,
            meta(crate::journal::MetaSession::Control),
            crate::controls::CONTROL_COUNT as usize,
        );
    }

    fn write_control_prefix(dir: &Path, metadata: Metadata, count: usize) {
        let path = dir.join("control/rc021_control_journal.tsv");
        let mut j = crate::controls::ControlJournal::create(&path, &metadata).unwrap();
        for id in crate::controls::CONTROL_ORDER.into_iter().take(count) {
            j.append(&crate::controls::ControlRow {
                control_id: id,
                ordinal: id.ordinal(),
                status: crate::controls::ControlStatus::Pass,
                monotonic_offset_ms: 100,
                detail: crate::journal::json_string_encode("ok"),
            })
            .unwrap();
        }
    }

    // ------------------------------------------------- §7.2 exact boundaries

    fn ledger_from(spreads_per_session: &[Vec<f64>], completed: bool) -> MeasurementLedger {
        let d = TempDir::new("ledger");
        for (i, sp) in spreads_per_session.iter().enumerate() {
            write_session(d.path(), i as u8 + 1, sp, completed);
        }
        read_ledger(d.path(), &fixture_identity(d.path()))
    }

    /// §7.2 rule 2 at each boundary, and the fixed denominator.
    #[test]
    fn the_verdict_turns_at_two_failures_of_ninety() {
        let within = 0.01f64;
        let over = 0.5f64;
        for (n_failures, want) in [(0u32, true), (1, true), (2, true), (3, false)] {
            // Spread the failures across distinct sessions so rule 3 holds.
            let mut sessions: Vec<Vec<f64>> = (0..crate::session::SESSIONS)
                .map(|_| vec![within; crate::session::MEASUREMENTS_PER_SESSION as usize])
                .collect();
            for session in sessions.iter_mut().take(n_failures as usize) {
                session[0] = over;
            }
            let ledger = ledger_from(&sessions, true);
            assert_eq!(ledger.failures_total(), n_failures);
            assert_eq!(
                ledger.sessions.len() as u32 * crate::session::MEASUREMENTS_PER_SESSION,
                crate::session::TOTAL_MEASUREMENTS,
                "the denominator is fixed at 90"
            );
            let (rules, verdict) = derive_verdict(&ledger, true);
            assert_eq!(
                verdict == crate::controls::VerdictOutcome::Qualified,
                want,
                "{n_failures} failures"
            );
            assert_eq!(rules.rule2_failures, n_failures);
            assert_eq!(rules.rule2_pass, n_failures <= MAX_FAILURES);
        }
    }

    /// §7.2 rule 3: a pass cannot rest on one session carrying two failures.
    #[test]
    fn one_failure_per_session_passes_and_two_in_one_session_does_not() {
        let within = 0.01f64;
        let over = 0.5f64;
        let mut one_each: Vec<Vec<f64>> = (0..crate::session::SESSIONS)
            .map(|_| vec![within; crate::session::MEASUREMENTS_PER_SESSION as usize])
            .collect();
        one_each[0][0] = over;
        one_each[1][0] = over;
        let ledger = ledger_from(&one_each, true);
        let (rules, verdict) = derive_verdict(&ledger, true);
        assert_eq!(rules.rule3_max_per_session, 1);
        assert!(rules.rule3_pass);
        assert_eq!(verdict, crate::controls::VerdictOutcome::Qualified);

        let mut two_in_one: Vec<Vec<f64>> = (0..crate::session::SESSIONS)
            .map(|_| vec![within; crate::session::MEASUREMENTS_PER_SESSION as usize])
            .collect();
        two_in_one[0][0] = over;
        two_in_one[0][1] = over;
        let ledger = ledger_from(&two_in_one, true);
        let (rules, verdict) = derive_verdict(&ledger, true);
        assert_eq!(rules.rule2_failures, 2, "still within rule 2");
        assert!(rules.rule2_pass);
        assert_eq!(rules.rule3_max_per_session, 2);
        assert!(!rules.rule3_pass);
        assert_eq!(verdict, crate::controls::VerdictOutcome::NotQualified);
    }

    /// §7.2: a failure is `paired_spread > 0.09`. Equality passes.
    #[test]
    fn the_bound_is_strict_and_equality_passes() {
        let at = SPREAD_BOUND;
        let above = f64::from_bits(SPREAD_BOUND.to_bits() + 1);
        assert!(above > SPREAD_BOUND);
        assert_ne!(at.to_bits(), above.to_bits());

        for (spread, want_failure) in [(at, false), (above, true)] {
            let mut sessions: Vec<Vec<f64>> = (0..crate::session::SESSIONS)
                .map(|_| vec![0.01; crate::session::MEASUREMENTS_PER_SESSION as usize])
                .collect();
            sessions[0][0] = spread;
            let ledger = ledger_from(&sessions, true);
            assert_eq!(
                ledger.failures_total(),
                want_failure as u32,
                "spread {spread} ({:#x})",
                spread.to_bits()
            );
        }
    }

    /// §7.2 rule 4 and rule 1 are conjuncts, not decoration.
    #[test]
    fn rules_one_and_four_can_each_deny_the_verdict() {
        let clean: Vec<Vec<f64>> = (0..crate::session::SESSIONS)
            .map(|_| vec![0.01; crate::session::MEASUREMENTS_PER_SESSION as usize])
            .collect();
        let ledger = ledger_from(&clean, true);
        assert_eq!(
            derive_verdict(&ledger, true).1,
            crate::controls::VerdictOutcome::Qualified
        );
        // rule 4
        assert_eq!(
            derive_verdict(&ledger, false).1,
            crate::controls::VerdictOutcome::NotQualified
        );
        // rule 1
        let incomplete = ledger_from(&clean, false);
        let (rules, verdict) = derive_verdict(&incomplete, true);
        assert!(!rules.rule1_all_completed);
        assert_eq!(verdict, crate::controls::VerdictOutcome::NotQualified);
    }

    // ---------------------------------------------- §5.2 session lifecycle

    #[test]
    fn every_session_state_is_recognised() {
        let within = vec![0.01f64; crate::session::MEASUREMENTS_PER_SESSION as usize];

        // NOT STARTED — no journal file at all.
        let d = TempDir::new("notstarted");
        let c = classify_session(1, &ReadOutcome::Missing, &fixture_identity(d.path()));
        assert_eq!(c.state, Some(SessionState::NotStarted));
        assert_eq!(c.unwritten, crate::session::MEASUREMENTS_PER_SESSION);
        assert_eq!(c.failures(), crate::session::MEASUREMENTS_PER_SESSION);
        drop(d);

        // COMPLETED — fifteen measurements and a close row.
        let d = TempDir::new("completed");
        write_session(d.path(), 1, &within, true);
        let c = &read_ledger(d.path(), &fixture_identity(d.path())).sessions[0];
        assert_eq!(c.state, Some(SessionState::Completed));
        assert_eq!(c.unwritten, 0);
        assert_eq!(c.failures(), 0);

        // ABORTED — an open row and no close row. §8.2: the reader classifies
        // it ABORTED, never NOT STARTED.
        let d = TempDir::new("aborted");
        write_session(d.path(), 1, &within[..3], false);
        let c = &read_ledger(d.path(), &fixture_identity(d.path())).sessions[0];
        assert_eq!(c.state, Some(SessionState::Aborted));
        assert_eq!(c.unwritten, 12);
        assert_eq!(c.failures(), 12);

        // A journal the reader refuses classifies to no state at all.
        let d = TempDir::new("damaged");
        write_session(d.path(), 1, &within, true);
        let p = d.path().join(session_journal_name(1));
        let mut bytes = std::fs::read(&p).unwrap();
        // Corrupt an interior row rather than the final line.
        let at = bytes.len() / 2;
        bytes[at] = b'\x00';
        std::fs::write(&p, &bytes).unwrap();
        let c = &read_ledger(d.path(), &fixture_identity(d.path())).sessions[0];
        assert!(c.journal_invalid);
        assert_eq!(c.state, None);

        // The domain round-trips.
        for st in [
            SessionState::NotStarted,
            SessionState::Started,
            SessionState::Completed,
            SessionState::Aborted,
        ] {
            assert_eq!(SessionState::parse(st.as_str()), Some(st));
        }
    }

    /// §7.2 and §C8.3: three different forms, all failures, denominator fixed.
    #[test]
    fn physical_logical_and_unwritten_all_count_as_failures() {
        let within = vec![0.01f64; crate::session::MEASUREMENTS_PER_SESSION as usize];

        // A physical LOST row.
        let d = TempDir::new("physlost");
        let path = d.path().join(session_journal_name(1));
        let m = meta(crate::journal::MetaSession::Qualification(1));
        let c = ctx();
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        let plan = crate::session::plan_session(1).unwrap();
        j.append(&c.lost(
            1,
            plan[0].phase,
            plan[0].block,
            Some(plan[0].measurement_index),
            11,
            "did not complete",
        ))
        .unwrap();
        for coord in plan.iter().skip(1) {
            j.append(&ok_row(1, coord.phase, coord.block, 0.01))
                .unwrap();
        }
        j.append(&c.session_close(1, 20, true, 0.1)).unwrap();
        let s = &read_ledger(d.path(), &fixture_identity(d.path())).sessions[0];
        assert_eq!(s.physical_lost, 1);
        assert_eq!(s.failures(), 1);
        assert_eq!(s.state, Some(SessionState::Completed));

        // A truncated final line — logical LOST, bytes untouched.
        let d = TempDir::new("logicallost");
        write_session(d.path(), 1, &within[..14], false);
        let path = d.path().join(session_journal_name(1));
        let before = std::fs::read(&path).unwrap();
        let mut bytes = before.clone();
        bytes.extend_from_slice(b"rc021/1\ttruncated");
        std::fs::write(&path, &bytes).unwrap();
        let s = &read_ledger(d.path(), &fixture_identity(d.path())).sessions[0];
        assert_eq!(s.logical_lost, 1);
        assert_eq!(s.failures(), 1, "14 written + 1 logical LOST = 15");
        assert_eq!(s.unwritten, 0);

        // Unwritten measurements, counted synthetically and never appended.
        let d = TempDir::new("unwritten");
        write_session(d.path(), 1, &within[..5], false);
        let path = d.path().join(session_journal_name(1));
        let before = std::fs::read(&path).unwrap();
        let s = &read_ledger(d.path(), &fixture_identity(d.path())).sessions[0];
        assert_eq!(s.unwritten, 10);
        assert_eq!(s.failures(), 10);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "§C8.3: an unwritten measurement never appends a row"
        );
    }

    // --------------------------------------------------- §8.2 ordered test

    fn evidence(ledger: MeasurementLedger) -> TerminalEvidence {
        TerminalEvidence {
            first_control_failure: None,
            control_evidence_unreadable: false,
            run_invalid: RunInvalidEvidence::Absent,
            marker_unusable: false,
            ledger,
            controls_all_pass: true,
        }
    }

    /// §8.2's four branches, in order, each decidable from bytes.
    #[test]
    fn the_ordered_test_takes_exactly_one_branch_in_the_frozen_order() {
        let within = vec![0.01f64; crate::session::MEASUREMENTS_PER_SESSION as usize];

        // 4. nothing wrong -> §7.2 decides, and only here is a verdict derived.
        let d = TempDir::new("branch4");
        for s in 1..=crate::session::SESSIONS {
            write_session(d.path(), s, &within, true);
        }
        let clean = read_ledger(d.path(), &fixture_identity(d.path()));
        let (status, sci) = classify_terminal(&evidence(clean.clone()));
        assert_eq!(status, RunStatus::HostQualified);
        assert!(sci.is_some());

        // 3. a preceding EXTERNAL-CAUSE row.
        let d = TempDir::new("branch3");
        let path = d.path().join(session_journal_name(1));
        let m = meta(crate::journal::MetaSession::Qualification(1));
        let c = ctx();
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        j.append(&c.external_cause(1, 11, 0.1, "power loss"))
            .unwrap();
        drop(j);
        for s in 2..=crate::session::SESSIONS {
            write_session(d.path(), s, &within, true);
        }
        let ext = read_ledger(d.path(), &fixture_identity(d.path()));
        assert!(ext.external_cause_before_abort());
        let (status, sci) = classify_terminal(&evidence(ext.clone()));
        assert_eq!(status, RunStatus::InconclusiveUnderpowered);
        assert!(sci.is_none(), "Class I derives no verdict");

        // 2. a journal integrity failure outranks the EXTERNAL-CAUSE row.
        let mut ev = evidence(ext.clone());
        ev.ledger.sessions[1].journal_invalid = true;
        assert_eq!(classify_terminal(&ev).0, RunStatus::JournalInvalid);

        // 1. a control failure outranks both, and §C11.25 gives P2 its class.
        let mut ev = evidence(ext);
        ev.ledger.sessions[1].journal_invalid = true;
        ev.first_control_failure = Some(crate::controls::FailureClass::InstrumentInvalid);
        assert_eq!(classify_terminal(&ev).0, RunStatus::InstrumentInvalid);
        ev.first_control_failure = Some(crate::controls::FailureClass::JournalInvalid);
        assert_eq!(classify_terminal(&ev).0, RunStatus::JournalInvalid);
    }

    /// §8.1 clause 3: the row must precede the abort it justifies.
    #[test]
    fn an_external_cause_after_a_completed_close_does_not_qualify() {
        let d = TempDir::new("extafter");
        let path = d.path().join(session_journal_name(1));
        let m = meta(crate::journal::MetaSession::Qualification(1));
        let c = ctx();
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        for coord in crate::session::plan_session(1).unwrap() {
            j.append(&ok_row(1, coord.phase, coord.block, 0.01))
                .unwrap();
        }
        j.append(&c.session_close(1, 20, true, 0.1)).unwrap();
        j.append(&c.external_cause(1, 21, 0.1, "after the fact"))
            .unwrap();
        drop(j);
        let s = &read_ledger(d.path(), &fixture_identity(d.path())).sessions[0];
        assert_journal_invalid("a row after the close", s);
    }

    /// §C11.25: P2's own class wins over the generic post-marker rule.
    #[test]
    fn p2_failure_is_journal_invalid_every_other_is_instrument_invalid() {
        for id in crate::controls::CONTROL_ORDER {
            let want = if id == crate::controls::ControlId::P2 {
                RunStatus::JournalInvalid
            } else {
                RunStatus::InstrumentInvalid
            };
            assert_eq!(id.failure_class().run_status(), want, "{}", id.as_str());
            let mut ev = evidence(ledger_from(
                &(0..crate::session::SESSIONS)
                    .map(|_| vec![0.01; crate::session::MEASUREMENTS_PER_SESSION as usize])
                    .collect::<Vec<_>>(),
                true,
            ));
            ev.first_control_failure = Some(id.failure_class());
            let (status, sci) = classify_terminal(&ev);
            assert_eq!(status, want, "{}", id.as_str());
            assert!(sci.is_none(), "no verdict is derived for a control failure");
        }
    }

    // ------------------------------------------------- §C10.1 identity

    #[test]
    fn identity_comes_from_the_manifest_when_it_is_valid() {
        let d = TempDir::new("idmanifest");
        let m = manifest(d.path());
        std::fs::write(d.path().join("run.json"), m.render().unwrap()).unwrap();
        match establish_identity(d.path()) {
            IdentityOutcome::Established {
                identity,
                source,
                manifest_damaged,
            } => {
                assert_eq!(source, IdentitySource::Manifest);
                assert!(!manifest_damaged);
                assert_eq!(identity.run_uuid, m.run_uuid);
                assert_eq!(identity.amendment_commits, m.amendment_commits);
            }
            other => panic!("{other:?}"),
        }
    }

    /// §C10.1 Branch A: all eight recovered from the durable headers.
    #[test]
    fn identity_is_reconstructed_from_agreeing_headers() {
        let d = TempDir::new("idheaders");
        write_clean_run(d.path());
        std::fs::write(d.path().join("run.json"), b"{ truncated").unwrap();
        match establish_identity(d.path()) {
            IdentityOutcome::Established {
                identity,
                source,
                manifest_damaged,
            } => {
                assert_eq!(source, IdentitySource::ReconstructedHeaders);
                assert!(manifest_damaged);
                assert_eq!(identity.run_uuid, "0".repeat(32));
                assert_eq!(identity.amendment_commits.len(), 2);
            }
            other => panic!("{other:?}"),
        }
    }

    /// §C10.1 Branch B: disagreeing or insufficient headers.
    #[test]
    fn identity_is_unreconstructible_when_headers_conflict_or_are_absent() {
        // Conflict.
        let d = TempDir::new("idconflict");
        write_session(d.path(), 1, &[0.01; 15], true);
        let mut m2 = meta(crate::journal::MetaSession::Qualification(2));
        m2.run_uuid = "1".repeat(32);
        // The writer is metadata-aware, so the row context must agree with the
        // header it is written under — which is exactly what makes the two
        // headers disagree with each other.
        let mut c = ctx();
        c.run_uuid = m2.run_uuid.clone();
        let mut j =
            crate::journal::Journal::create(&d.path().join(session_journal_name(2)), &m2).unwrap();
        j.append(&c.session_open(2, 10, 0.1)).unwrap();
        drop(j);
        std::fs::write(d.path().join("run.json"), b"{ truncated").unwrap();
        assert!(matches!(
            establish_identity(d.path()),
            IdentityOutcome::Unreconstructible(_)
        ));

        // Nothing to recover from.
        let d = TempDir::new("idnone");
        std::fs::write(d.path().join("run.json"), b"{ truncated").unwrap();
        assert!(matches!(
            establish_identity(d.path()),
            IdentityOutcome::Unreconstructible(_)
        ));
    }

    // -------------------------------------------------- closure codec

    fn integrity_of(dir: &Path) -> Vec<IntegrityEntry> {
        build_integrity(dir)
    }

    fn class_one_closure(dir: &Path) -> Closure {
        Closure {
            schema_version: crate::journal::SCHEMA_VERSION.to_string(),
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            finalized_monotonic_offset_ms: Some(12_345),
            finalized_utc: "2026-08-27T00:00:00Z".into(),
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            amendment_commits: vec!["c".repeat(40), "d".repeat(40)],
            instrument_birth_commit: "e".repeat(40),
            host_fingerprint: host_fields().fingerprint(),
            identity_source: IdentitySource::Manifest,
            terminal_status: RunStatus::InstrumentInvalid,
            exit_code: RunStatus::InstrumentInvalid.exit_code(),
            session_states: vec![None; crate::session::SESSIONS as usize],
            verdict_rules: None,
            failures_total: None,
            failures_by_session: None,
            control_outcomes: Vec::new(),
            integrity: integrity_of(dir),
            leave_one_out: None,
            artifacts: Artifacts::default(),
        }
    }

    /// §C13.4: twenty-one keys, in the frozen order, compact, one trailing LF.
    #[test]
    fn the_closure_carries_twenty_one_keys_in_the_frozen_order() {
        let d = TempDir::new("keys");
        let c = class_one_closure(d.path());
        let text = c.render();
        assert!(text.ends_with("}\n"));
        assert_eq!(text.matches('\n').count(), 1, "exactly one trailing LF");
        assert!(!text.contains("\": "), "compact: no space after a colon");
        let mut at = 0usize;
        for k in CLOSURE_KEYS {
            let n = format!("\"{k}\":");
            at += text[at..]
                .find(&n)
                .unwrap_or_else(|| panic!("{k} missing or out of order"))
                + n.len();
        }
        assert_eq!(CLOSURE_KEYS.len(), 21);
        assert_eq!(Closure::parse(&text).unwrap(), c, "canonical round trip");
    }

    #[test]
    fn closure_float_spelling_round_trips_through_the_actual_json_reader() {
        // Without the codec's reader-aware spelling search this value is
        // decoded one ULP away by the workspace's serde_json configuration.
        let value = 0.010000000000000009_f64;
        let encoded = jnum(value);
        let parsed = serde_json::from_str::<serde_json::Value>(&encoded)
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(parsed.to_bits(), value.to_bits());
        assert_eq!(jnum(parsed), encoded, "the canonical spelling is stable");
    }

    /// §C13.5: the fixed ordered fifteen, always all fifteen.
    #[test]
    fn the_integrity_inventory_is_the_fixed_fifteen_in_order() {
        assert_eq!(INTEGRITY_INVENTORY.len(), 15);
        let d = TempDir::new("inventory");
        let entries = build_integrity(d.path());
        assert_eq!(entries.len(), 15);
        for (e, (rel, kind)) in entries.iter().zip(INTEGRITY_INVENTORY.iter()) {
            assert_eq!(&e.path, rel);
            assert_eq!(e.kind, *kind);
            // An empty directory: every element is MISSING with null counters.
            assert_eq!(e.sha256, MISSING);
            assert_eq!(e.byte_count, None);
            assert_eq!(e.status_counts, None);
        }
        // It is deliberately not the manifest's reserved-path list.
        let reserved: Vec<&str> = crate::manifest::RESERVED_PATHS.to_vec();
        let inventory: Vec<&str> = INTEGRITY_INVENTORY.iter().map(|(p, _)| *p).collect();
        assert_ne!(reserved, inventory, "the two lists are not interchangeable");
    }

    /// §C13.4: existence governs first, kind second.
    #[test]
    fn counters_follow_existence_then_kind() {
        let d = TempDir::new("counters");
        write_session(d.path(), 1, &[0.01; 15], true);
        write_passing_controls(d.path());
        std::fs::write(d.path().join("run.json"), b"{}\n").unwrap();
        let entries = build_integrity(d.path());
        let find = |p: &str| entries.iter().find(|e| e.path == p).unwrap().clone();

        let j = find("rc021_journal_s1.tsv");
        assert_ne!(j.sha256, MISSING);
        assert_eq!(j.header_line_count, Some(1));
        assert!(matches!(
            j.status_counts,
            Some(StatusCountsRecord::Journal(_))
        ));

        let c = find("control/rc021_control_journal.tsv");
        assert_eq!(c.header_line_count, Some(1));
        match c.status_counts {
            Some(StatusCountsRecord::Control { pass, fail }) => {
                assert_eq!((pass, fail), (12, 0));
            }
            other => panic!("{other:?}"),
        }

        // JSON: only sha256 and byte_count; the five line counters are null.
        let r = find("run.json");
        assert_ne!(r.sha256, MISSING);
        assert_eq!(r.byte_count, Some(3));
        assert_eq!(r.physical_line_count, None);
        assert_eq!(r.status_counts, None);

        // Absent: MISSING and nulls throughout, whatever the kind.
        let m = find("RC021_RESULTS.md");
        assert_eq!(m.sha256, MISSING);
        assert_eq!(m.byte_count, None);
    }

    /// §C13.45: the mandatory-by-class matrix, in both directions.
    #[test]
    fn each_mandatory_by_class_violation_is_refused() {
        let d = TempDir::new("byclass");
        let base = class_one_closure(d.path());
        assert_eq!(base.validate(), Ok(()));

        // A Class I closure carrying any derived scientific quantity.
        for mutate in [
            (|c: &mut Closure| {
                c.verdict_rules = Some(VerdictRules {
                    rule1_all_completed: true,
                    rule2_failures: 0,
                    rule2_pass: true,
                    rule3_max_per_session: 0,
                    rule3_pass: true,
                    rule4_controls_pass: true,
                })
            }) as fn(&mut Closure),
            |c: &mut Closure| c.failures_total = Some(0),
            |c: &mut Closure| c.failures_by_session = Some(vec![0; 6]),
            |c: &mut Closure| {
                c.leave_one_out = Some(LeaveOneOut {
                    full_90: LooSummary {
                        omitted_session: None,
                        failure_count: 0,
                        max_per_session: 0,
                        median_spread: None,
                        p95_spread: None,
                        excluded_lost: 0,
                        excluded_unwritten: 90,
                        valid_spread_count: 0,
                    },
                    omissions: Vec::new(),
                })
            },
            |c: &mut Closure| c.artifacts.observations_sha256 = Some("f".repeat(64)),
            |c: &mut Closure| c.artifacts.results_md_sha256 = Some("f".repeat(64)),
        ] {
            let mut c = base.clone();
            mutate(&mut c);
            assert!(c.validate().is_err(), "a Class I closure must null this");
        }

        // exit_code must be the status's own code.
        let mut c = base.clone();
        c.exit_code = 0;
        assert_eq!(
            c.validate(),
            Err(ClosureError::Domain(
                "exit_code must be terminal_status.exit_code()"
            ))
        );

        // A Class II closure missing a scientific field.
        let mut c = base.clone();
        c.terminal_status = RunStatus::HostQualified;
        c.exit_code = RunStatus::HostQualified.exit_code();
        assert!(c.validate().is_err(), "Class II must carry every field");
    }

    /// §C13.4/§C13.44: nested-object domains.
    #[test]
    fn each_nested_domain_violation_is_refused() {
        let d = TempDir::new("nested");
        let base = class_one_closure(d.path());

        // The inventory must keep its frozen order.
        let mut c = base.clone();
        c.integrity.swap(0, 1);
        assert!(c.validate().is_err());

        // …and its length.
        let mut c = base.clone();
        c.integrity.pop();
        assert_eq!(
            c.validate(),
            Err(ClosureError::Domain("integrity must have fifteen elements"))
        );

        // session_states is exactly six.
        let mut c = base.clone();
        c.session_states.pop();
        assert!(c.validate().is_err());

        // control_outcomes ascend by ordinal.
        let mut c = base.clone();
        c.control_outcomes = vec![ControlOutcomeRecord {
            control_id: "P5".into(),
            ordinal: 2,
            status: "PASS".into(),
            monotonic_offset_ms: 1,
        }];
        assert!(c.validate().is_err());

        // An unknown key is refused on lookup…
        let text = base.render();
        let renamed = text.replacen("\"schema_version\":", "\"zzz_schema_version\":", 1);
        assert!(Closure::parse(&renamed).is_err());

        // …and a genuinely *reordered* one — same keys, same values, different
        // order — is refused only by the canonical re-render, since JSON object
        // order carries no meaning to a parser.
        let seg = format!("\"exit_code\":{},", base.exit_code);
        assert!(text.contains(&seg));
        let moved = format!(
            "{{{}{}",
            seg,
            text.replacen(&seg, "", 1).trim_start_matches('{')
        );
        assert_ne!(moved, text);
        let a: serde_json::Value = serde_json::from_str(&moved).unwrap();
        let b: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(a, b, "the reordering changes no value");
        assert_eq!(
            Closure::parse(&moved),
            Err(ClosureError::Domain("closure is not in canonical form"))
        );
    }

    // ------------------------------------------- Amendment 1 §A5 table

    /// §A5 and §C13.44: descriptive, nearest-rank, counters that sum.
    #[test]
    fn leave_one_out_is_descriptive_and_its_counters_sum() {
        let d = TempDir::new("loo");
        write_clean_run(d.path());
        let ledger = read_ledger(d.path(), &fixture_identity(d.path()));
        let loo = leave_one_out(&ledger);
        assert_eq!(loo.omissions.len(), 6);
        assert_eq!(
            loo.full_90.counters_sum(),
            crate::session::TOTAL_MEASUREMENTS
        );
        assert_eq!(loo.full_90.valid_spread_count, 90);
        for (i, o) in loo.omissions.iter().enumerate() {
            assert_eq!(o.omitted_session, Some(i as u8 + 1));
            assert_eq!(o.counters_sum(), 75);
            assert_eq!(o.valid_spread_count, 75);
        }
        // The verdict does not consult it: same ledger, same verdict.
        let (_, verdict) = derive_verdict(&ledger, true);
        assert_eq!(verdict, crate::controls::VerdictOutcome::Qualified);
    }

    /// §C13.44: quantiles are null exactly when the denominator is zero.
    #[test]
    fn quantiles_are_null_exactly_when_no_valid_spread_remains() {
        let d = TempDir::new("loonull");
        // Every session NOT STARTED: 90 synthetic failures, no spread at all.
        let ledger = read_ledger(d.path(), &fixture_identity(d.path()));
        let loo = leave_one_out(&ledger);
        assert_eq!(loo.full_90.valid_spread_count, 0);
        assert_eq!(loo.full_90.median_spread, None);
        assert_eq!(loo.full_90.p95_spread, None);
        assert_eq!(loo.full_90.excluded_unwritten, 90);
        assert_eq!(loo.full_90.counters_sum(), 90);
        for o in &loo.omissions {
            assert_eq!(o.counters_sum(), 75);
            assert_eq!(o.median_spread, None);
        }
    }

    /// §A5: nearest rank, no interpolation.
    #[test]
    fn nearest_rank_has_no_interpolation() {
        let v = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(nearest_rank(&v, 0.50), Some(2.0));
        assert_eq!(nearest_rank(&v, 0.95), Some(4.0));
        assert_eq!(nearest_rank(&v, 0.0), Some(1.0));
        assert_eq!(nearest_rank(&[], 0.5), None);
        // The interpolating median of this set is 2.5; nearest rank is 2.0.
        assert_ne!(nearest_rank(&v, 0.50), Some(2.5));
        let n75: Vec<f64> = (1..=75).map(|i| i as f64).collect();
        assert_eq!(nearest_rank(&n75, 0.95), Some(72.0));
    }

    // ------------------------------------------- the injected durability seam

    /// Which durability point fails, and what the caller leaves behind.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum FailAt {
        None,
        CreateNew,
        FsyncAfterReservation,
        Artifact,
        WriteAll,
        Flush,
        SyncAll,
        FsyncAfterWrite,
    }

    /// A fake that fails one syscall and counts what production did.
    ///
    /// It carries **no** scientific input: no threshold, seed, count, rule set,
    /// inventory or classifier. It can make the disk refuse; it can never
    /// change what the instrument decides.
    struct FakeIo {
        fail_at: FailAt,
        /// Replaces the rendered closure with bytes the canonical reader must
        /// reject, so the §C14.15 self-check can be observed end to end.
        corrupt_closure: bool,
        /// §C13.9: what `write_all` leaves on disk when it "fails" — the norm
        /// names empty, partial **and complete** as reachable.
        partial_write: bool,
        creates: std::cell::RefCell<u32>,
        sink_bytes: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
        path: std::cell::RefCell<Option<PathBuf>>,
    }

    impl FakeIo {
        fn new(fail_at: FailAt) -> FakeIo {
            FakeIo {
                fail_at,
                corrupt_closure: false,
                partial_write: false,
                creates: std::cell::RefCell::new(0),
                sink_bytes: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
                path: std::cell::RefCell::new(None),
            }
        }
    }

    struct FakeSink {
        fail_at: FailAt,
        partial_write: bool,
        path: PathBuf,
        bytes: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
    }

    impl FakeSink {
        fn persist(&self) {
            let _ = std::fs::write(&self.path, self.bytes.borrow().as_slice());
        }
    }

    impl ClosureSink for FakeSink {
        fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            if self.fail_at == FailAt::WriteAll {
                if self.partial_write {
                    // A genuinely partial file.
                    *self.bytes.borrow_mut() = bytes[..bytes.len() / 2].to_vec();
                    self.persist();
                }
                return Err(io::Error::other("injected write_all failure"));
            }
            *self.bytes.borrow_mut() = bytes.to_vec();
            self.persist();
            Ok(())
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.fail_at == FailAt::Flush {
                return Err(io::Error::other("injected flush failure"));
            }
            Ok(())
        }
        fn sync_all(&mut self) -> io::Result<()> {
            if self.fail_at == FailAt::SyncAll {
                return Err(io::Error::other("injected sync_all failure"));
            }
            Ok(())
        }
    }

    impl FinalizeIo for FakeIo {
        fn create_new_closure(&mut self, path: &Path) -> io::Result<Box<dyn ClosureSink>> {
            *self.creates.borrow_mut() += 1;
            if self.fail_at == FailAt::CreateNew {
                return Err(io::Error::other("injected create_new failure"));
            }
            // The reservation really happens: an empty file appears.
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            *self.path.borrow_mut() = Some(path.to_path_buf());
            Ok(Box::new(FakeSink {
                fail_at: self.fail_at,
                partial_write: self.partial_write,
                path: path.to_path_buf(),
                bytes: std::rc::Rc::clone(&self.sink_bytes),
            }))
        }
        fn fsync_dir(&mut self, _dir: &Path) -> io::Result<()> {
            // The first call is after reservation; a later one is after writing.
            let after_write = !self.sink_bytes.borrow().is_empty();
            let want = if after_write {
                FailAt::FsyncAfterWrite
            } else {
                FailAt::FsyncAfterReservation
            };
            if self.fail_at == want {
                return Err(io::Error::other("injected fsync failure"));
            }
            Ok(())
        }
        fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
            if self.fail_at == FailAt::Artifact {
                // §C13.9: a partial file exists and its bytes are evidence.
                let _ = std::fs::write(path, &bytes[..bytes.len() / 2]);
                return Err(io::Error::other("injected artifact failure"));
            }
            std::fs::write(path, bytes)
        }
        fn now_utc(&mut self) -> String {
            "2026-08-27T00:00:00Z".to_string()
        }
        fn intercept_closure_bytes(&mut self, body: String) -> String {
            if self.corrupt_closure {
                // Canonical JSON with one key renamed: every value is intact,
                // so only the reader can tell it is not a closure.
                return body.replacen("\"exit_code\"", "\"exit_status\"", 1);
            }
            body
        }
        fn monotonic_offset_ms(&mut self, _boot: &str, _start: u64) -> Option<u64> {
            Some(600_000)
        }
    }

    /// A complete run directory: manifest, passing controls, six clean sessions.
    fn prepared_run(name: &str) -> TempDir {
        let d = TempDir::new(name);
        let m = manifest(d.path());
        std::fs::write(d.path().join("run.json"), m.render().unwrap()).unwrap();
        write_controls_started(d.path());
        write_passing_controls(d.path());
        write_controls_complete(d.path());
        write_clean_run(d.path());
        d
    }

    // -------------------------------------------- §C13.9 the happy path

    #[test]
    fn a_clean_run_finalizes_to_a_class_two_closure_with_both_artifacts() {
        let d = prepared_run("happy");
        let mut io = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::HostQualified
            }
        );
        assert_eq!(out.terminal_exit_code(), Some(0));
        // §C13.9 step 2 happens exactly once and the handle is retained.
        assert_eq!(*io.creates.borrow(), 1, "one create_new, no reopen");

        let c = match closure_path_state(d.path()) {
            ClosurePathState::Complete(c) => *c,
            other => panic!("{other:?}"),
        };
        assert_eq!(c.terminal_status, RunStatus::HostQualified);
        assert_eq!(c.exit_code, 0);
        assert_eq!(c.identity_source, IdentitySource::Manifest);
        assert_eq!(c.failures_total, Some(0));
        assert!(c.verdict_rules.is_some());
        assert!(c.leave_one_out.is_some());
        assert_eq!(c.session_states.iter().filter(|s| s.is_some()).count(), 6);
        assert_eq!(c.control_outcomes.len(), 12);
        assert_eq!(c.integrity.len(), 15);
        assert!(c.artifacts.observations_sha256.is_some());
        assert!(c.artifacts.results_md_sha256.is_some());

        // §C13.85: header plus the 90 OK rows, ascending index.
        let obs = std::fs::read_to_string(d.path().join(OBSERVATIONS_FILE)).unwrap();
        let lines: Vec<&str> = obs.lines().collect();
        assert_eq!(lines[0], OBSERVATIONS_COLUMNS.join("\t"));
        assert_eq!(lines.len(), 91);
        let idx: Vec<u32> = lines[1..]
            .iter()
            .map(|l| l.split('\t').nth(3).unwrap().parse().unwrap())
            .collect();
        assert_eq!(idx, (1..=90).collect::<Vec<u32>>());

        // Amendment 1 §A2's prohibited phrasings, precisely. A *denial* that
        // the protocol certifies anything is required, not forbidden, so the
        // check bans the claims rather than the word stems.
        let md = std::fs::read_to_string(d.path().join(RESULTS_FILE)).unwrap();
        let lower = md.to_lowercase();
        for banned in [
            "certifies",
            "certified rate",
            "confidence",
            "0.9317",
            "0.9023",
            "clopper",
            "binomial",
        ] {
            assert!(!lower.contains(banned), "prohibited phrasing: {banned}");
        }
        assert!(md.contains("not a statistical certification"));
        assert!(md.contains("descriptive"));

        // …and verify accepts it as a mode success.
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
        assert_eq!(verify(d.path()).mode_exit_code(), Some(0));
    }

    // ------------------------------ §C14.15 the seven durability points

    /// **The corrected contract.** Every post-reservation failure leaves the
    /// path — empty, partial **or complete**, all three named by §C14.15 — the
    /// invocation reports `JOURNAL-INVALID`, a repeat writes nothing, and a
    /// later verify classifies from the surviving bytes alone.
    #[test]
    fn every_durability_point_reports_journal_invalid_and_locks_the_path() {
        let points = [
            (FailAt::CreateNew, "create_new"),
            (
                FailAt::FsyncAfterReservation,
                "fsync_parent_after_reservation",
            ),
            (FailAt::WriteAll, "write_all"),
            (FailAt::Flush, "flush"),
            (FailAt::SyncAll, "sync_all"),
            (FailAt::FsyncAfterWrite, "fsync_parent_after_write"),
        ];
        for (fail_at, at) in points {
            let d = prepared_run("durability");
            let mut io = FakeIo::new(fail_at);
            let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
            assert_eq!(out, FinalizeOutcome::DurabilityFailed { at }, "{at}");
            assert_eq!(
                out.terminal_exit_code(),
                Some(RunStatus::JournalInvalid.exit_code()),
                "{at}"
            );
            assert_eq!(*io.creates.borrow(), 1, "{at}: one create_new");

            if fail_at == FailAt::CreateNew {
                // §C14.155: no closure path, so no finalize-owned byte exists
                // and the invocation may be repeated safely.
                assert_eq!(closure_path_state(d.path()), ClosurePathState::Absent);
                continue;
            }

            // The path is reserved and now locks the run permanently.
            let state = closure_path_state(d.path());
            assert_ne!(
                state,
                ClosurePathState::Absent,
                "{at}: the path must remain"
            );

            // A repeat writes nothing at all.
            let before = snapshot(d.path());
            let mut io2 = FakeIo::new(FailAt::None);
            let repeat = finalize_with_io(d.path(), TEST_RUN_ID, &mut io2);
            assert_eq!(*io2.creates.borrow(), 0, "{at}: no second create_new");
            assert_eq!(snapshot(d.path()), before, "{at}: a repeat wrote bytes");
            match state {
                ClosurePathState::Complete(_) => {
                    assert_eq!(repeat, FinalizeOutcome::AlreadyComplete, "{at}");
                    // §C14.2: a complete closure verifies normally.
                    assert_eq!(verify(d.path()), VerifyOutcome::Verified, "{at}");
                }
                _ => {
                    assert_eq!(repeat, FinalizeOutcome::EmptyOrPartialClosure, "{at}");
                    assert_eq!(verify(d.path()), VerifyOutcome::EmptyOrPartial, "{at}");
                    assert_eq!(
                        verify(d.path()).terminal_exit_code(),
                        Some(RunStatus::JournalInvalid.exit_code()),
                        "{at}"
                    );
                }
            }
        }
    }

    /// The scenario the durability review named: the complete canonical Class
    /// II bytes survive, the syscall still failed.
    ///
    /// §C14.15 allows exactly this — "empty, partial **or complete**" — and its
    /// own tie-breaker settles the later reading: "only a valid, complete
    /// closure means finalization succeeded". The invocation reports the error;
    /// the reader classifies the bytes. The past error is not recorded and not
    /// reconstructed, and §C14.155 disclaims any historical check.
    #[test]
    fn complete_bytes_plus_a_syscall_error_report_four_now_and_verify_zero_later() {
        for fail_at in [FailAt::Flush, FailAt::SyncAll, FailAt::FsyncAfterWrite] {
            let d = prepared_run("completeplus");
            let mut io = FakeIo::new(fail_at);
            let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
            // This invocation reports the durability failure.
            assert_eq!(
                out.terminal_exit_code(),
                Some(RunStatus::JournalInvalid.exit_code()),
                "{fail_at:?}"
            );

            // The bytes on disk are a complete canonical Class II closure.
            let c = match closure_path_state(d.path()) {
                ClosurePathState::Complete(c) => *c,
                other => panic!("{fail_at:?}: {other:?}"),
            };
            assert_eq!(c.terminal_status, RunStatus::HostQualified);
            assert!(c.verdict_rules.is_some());

            // A later verify classifies from those bytes and succeeds.
            assert_eq!(verify(d.path()), VerifyOutcome::Verified, "{fail_at:?}");
            assert_eq!(verify(d.path()).mode_exit_code(), Some(0));
            assert_eq!(verify(d.path()).terminal_run_status(), None);

            // The status is the one derived before the write; nothing replaced
            // it with the invocation's own outcome.
            assert_ne!(c.terminal_status, RunStatus::JournalInvalid);
        }
    }

    /// A genuinely partial write is a different observable state.
    #[test]
    fn a_partial_closure_is_journal_invalid_for_both_modes() {
        let d = prepared_run("partial");
        let mut io = FakeIo::new(FailAt::WriteAll);
        io.partial_write = true;
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(out, FinalizeOutcome::DurabilityFailed { at: "write_all" });
        assert!(matches!(
            closure_path_state(d.path()),
            ClosurePathState::Partial(_)
        ));
        assert_eq!(verify(d.path()), VerifyOutcome::EmptyOrPartial);
        assert_eq!(
            verify(d.path()).terminal_exit_code(),
            Some(RunStatus::JournalInvalid.exit_code())
        );
    }

    // ------------------------------------ §C13.9 artifact failure before closure

    /// §C13.9: the closure is still written, the run becomes `JOURNAL-INVALID`,
    /// every scientific field and both artifact hashes are null, and
    /// `integrity` nonetheless records the partial artifact's **actual** bytes.
    #[test]
    fn an_artifact_failure_yields_a_class_one_closure_that_still_hashes_the_partial_bytes() {
        let d = prepared_run("artifactfail");
        let mut io = FakeIo::new(FailAt::Artifact);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::JournalInvalid
            }
        );
        assert_eq!(
            out.terminal_exit_code(),
            Some(RunStatus::JournalInvalid.exit_code())
        );

        let c = match closure_path_state(d.path()) {
            ClosurePathState::Complete(c) => *c,
            other => panic!("{other:?}"),
        };
        assert_eq!(c.terminal_status, RunStatus::JournalInvalid);
        // §C13.45: no Class II verdict is published.
        assert_eq!(c.verdict_rules, None);
        assert_eq!(c.failures_total, None);
        assert_eq!(c.failures_by_session, None);
        assert_eq!(c.leave_one_out, None);
        // Field 21: both null for a Class I closure.
        assert_eq!(c.artifacts.observations_sha256, None);
        assert_eq!(c.artifacts.results_md_sha256, None);

        // Field 19: the partial observations file exists and its bytes are
        // evidence — the actual hash, never MISSING.
        let obs_path = d.path().join(OBSERVATIONS_FILE);
        assert!(obs_path.exists(), "a partial artifact was left on disk");
        let actual = crate::host::sha256_hex(&std::fs::read(&obs_path).unwrap());
        let entry = c
            .integrity
            .iter()
            .find(|e| e.path == OBSERVATIONS_FILE)
            .unwrap();
        assert_eq!(entry.sha256, actual, "integrity records the actual bytes");
        assert_ne!(entry.sha256, MISSING);
        assert!(entry.byte_count.unwrap() > 0);

        // The intended full bytes are NOT what was hashed.
        let ledger = read_ledger(d.path(), &fixture_identity(d.path()));
        let intended = crate::host::sha256_hex(render_observations(&ledger).as_bytes());
        assert_ne!(
            entry.sha256, intended,
            "the intended bytes are not the record"
        );

        // The Markdown was never attempted, so it stays MISSING.
        let md = c.integrity.iter().find(|e| e.path == RESULTS_FILE).unwrap();
        assert_eq!(md.sha256, MISSING);
        assert_eq!(md.byte_count, None);

        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// §C13.8: no Class I run receives observations or a results document.
    #[test]
    fn a_class_one_run_publishes_no_scientific_artifact() {
        let d = prepared_run("classone");
        // A control failure makes the run Class I.
        std::fs::remove_file(d.path().join("control/rc021_control_journal.tsv")).unwrap();
        let path = d.path().join("control/rc021_control_journal.tsv");
        let m = meta(crate::journal::MetaSession::Control);
        let mut j = crate::controls::ControlJournal::create(&path, &m).unwrap();
        j.append(&crate::controls::ControlRow {
            control_id: crate::controls::ControlId::P6,
            ordinal: 1,
            status: crate::controls::ControlStatus::Fail,
            monotonic_offset_ms: 1,
            detail: crate::journal::json_string_encode("provenance"),
        })
        .unwrap();
        drop(j);

        let mut io = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );
        assert!(!d.path().join(OBSERVATIONS_FILE).exists());
        assert!(!d.path().join(RESULTS_FILE).exists());
        let c = match closure_path_state(d.path()) {
            ClosurePathState::Complete(c) => *c,
            other => panic!("{other:?}"),
        };
        assert_eq!(c.control_outcomes.len(), 1);
        assert_eq!(c.verdict_rules, None);
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// §C13.8: a `HOST-NOT-QUALIFIED` from a NOT STARTED session receives no
    /// observations, because the six-COMPLETED condition is false.
    #[test]
    fn a_not_started_session_denies_observations_but_still_gets_results_md() {
        let d = prepared_run("notstartedrun");
        std::fs::remove_file(d.path().join(session_journal_name(6))).unwrap();
        let mut io = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::HostNotQualified
            }
        );
        assert_eq!(out.terminal_exit_code(), Some(1));
        assert!(!d.path().join(OBSERVATIONS_FILE).exists(), "§C13.8");
        assert!(
            d.path().join(RESULTS_FILE).exists(),
            "both Class II verdicts"
        );
        let c = match closure_path_state(d.path()) {
            ClosurePathState::Complete(c) => *c,
            other => panic!("{other:?}"),
        };
        assert_eq!(c.session_states[5], Some(SessionState::NotStarted));
        assert_eq!(c.failures_total, Some(15));
        assert_eq!(c.artifacts.observations_sha256, None);
        assert!(c.artifacts.results_md_sha256.is_some());
        let missing = c
            .integrity
            .iter()
            .find(|e| e.path == session_journal_name(6))
            .unwrap();
        assert_eq!(missing.sha256, MISSING, "§C13.7");
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    // ------------------------------------------- §C14.16 the terminal lock

    /// §C10.1 Branch B exits before reserving: no closure path is created.
    #[test]
    fn an_unreconstructible_identity_reserves_nothing() {
        let d = TempDir::new("branchb");
        std::fs::write(d.path().join("run.json"), b"{ truncated").unwrap();
        // §C13.6: the run began, so the §C14.1 obligation is live and Branch B
        // is what decides — not the pre-measurement refusal.
        write_controls_started(d.path());
        let before = snapshot(d.path());
        let mut io = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert!(
            matches!(out, FinalizeOutcome::Unreconstructible { .. }),
            "{out:?}"
        );
        // §C10.1 Branch B must say what was damaged, not merely that it was.
        assert!(
            out.describe().len() > "Unreconstructible".len(),
            "the reason must reach the operator: {}",
            out.describe()
        );
        assert_eq!(
            out.terminal_exit_code(),
            Some(RunStatus::JournalInvalid.exit_code())
        );
        assert_eq!(*io.creates.borrow(), 0, "nothing was reserved");
        assert_eq!(closure_path_state(d.path()), ClosurePathState::Absent);
        assert_eq!(snapshot(d.path()), before);
    }

    /// §C14.16: an instrument-created artifact with no closure path is external
    /// or corrupt legacy state, not a stage of a run this instrument conducted.
    #[test]
    fn an_artifact_with_no_closure_path_is_external_state() {
        for artifact in [OBSERVATIONS_FILE, RESULTS_FILE] {
            let d = prepared_run("external");
            std::fs::write(d.path().join(artifact), b"left behind\n").unwrap();
            let before = snapshot(d.path());
            let mut io = FakeIo::new(FailAt::None);
            let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
            assert_eq!(out, FinalizeOutcome::ArtifactWithoutClosure, "{artifact}");
            assert_eq!(
                out.terminal_exit_code(),
                Some(RunStatus::JournalInvalid.exit_code())
            );
            assert_eq!(*io.creates.borrow(), 0, "{artifact}: nothing reserved");
            assert_eq!(snapshot(d.path()), before, "{artifact}: bytes changed");
        }
    }

    /// §C14.2: a valid complete closure refuses finalize with exit 2 and
    /// verifies normally.
    #[test]
    fn a_valid_complete_closure_refuses_finalize_and_verifies() {
        let d = prepared_run("locked");
        let mut io = FakeIo::new(FailAt::None);
        assert!(matches!(
            finalize_with_io(d.path(), TEST_RUN_ID, &mut io),
            FinalizeOutcome::Wrote { .. }
        ));
        let before = snapshot(d.path());
        let mut io2 = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io2);
        assert_eq!(out, FinalizeOutcome::AlreadyComplete);
        assert_eq!(
            out.terminal_exit_code(),
            Some(RunStatus::RefusedBeforeMeasurement.exit_code())
        );
        assert_eq!(*io2.creates.borrow(), 0);
        assert_eq!(snapshot(d.path()), before);
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// §C14.2: an empty reservation is a different, broken state.
    #[test]
    fn an_empty_closure_path_is_journal_invalid_for_both_modes() {
        let d = prepared_run("emptyclosure");
        std::fs::write(d.path().join(CLOSURE_FILE), b"").unwrap();
        assert_eq!(closure_path_state(d.path()), ClosurePathState::Empty);
        let before = snapshot(d.path());
        let mut io = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(out, FinalizeOutcome::EmptyOrPartialClosure);
        assert_eq!(
            out.terminal_exit_code(),
            Some(RunStatus::JournalInvalid.exit_code())
        );
        assert_eq!(snapshot(d.path()), before);
        assert_eq!(verify(d.path()), VerifyOutcome::EmptyOrPartial);
    }

    // --------------------------------------------------- §C14.3 verify

    /// Verify is a mode: every valid closure, of any class, is a mode success.
    #[test]
    fn verify_is_a_mode_success_for_every_class_of_valid_closure() {
        // Class II HOST-QUALIFIED.
        let d = prepared_run("vq");
        finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
        assert_eq!(verify(d.path()).mode_exit_code(), Some(0));
        assert_eq!(verify(d.path()).terminal_run_status(), None);
        assert!(!verify(d.path()).is_terminal());

        // Class II HOST-NOT-QUALIFIED — still a mode success, not exit 1.
        let d = prepared_run("vnq");
        std::fs::remove_file(d.path().join(session_journal_name(6))).unwrap();
        finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        let c = match closure_path_state(d.path()) {
            ClosurePathState::Complete(c) => *c,
            other => panic!("{other:?}"),
        };
        assert_eq!(c.terminal_status, RunStatus::HostNotQualified);
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
        assert_eq!(verify(d.path()).mode_exit_code(), Some(0));

        // Class I — the four scientific fields are asserted null.
        let d = prepared_run("vci");
        std::fs::write(d.path().join(session_journal_name(1)), b"broken\n").unwrap();
        finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        let c = match closure_path_state(d.path()) {
            ClosurePathState::Complete(c) => *c,
            other => panic!("{other:?}"),
        };
        assert!(!c.terminal_status.is_class_two());
        assert_eq!(c.verdict_rules, None);
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
        assert_eq!(verify(d.path()).mode_exit_code(), Some(0));
    }

    /// §C14.3: verify writes no byte and repairs nothing.
    #[test]
    fn verify_leaves_the_directory_byte_identical() {
        let d = prepared_run("readonly");
        finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        let before = snapshot(d.path());
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
        assert_eq!(snapshot(d.path()), before, "verify wrote or repaired bytes");
        // …and on a broken run too.
        let d = prepared_run("readonly2");
        std::fs::write(d.path().join(CLOSURE_FILE), b"{partial").unwrap();
        let before = snapshot(d.path());
        assert_eq!(verify(d.path()), VerifyOutcome::EmptyOrPartial);
        assert_eq!(snapshot(d.path()), before);
    }

    /// §C14.3: every one of the fifteen inventory entries is recomputed.
    #[test]
    fn verify_detects_a_mutation_in_every_inventory_entry() {
        for (rel, _) in INTEGRITY_INVENTORY {
            let d = prepared_run("invmut");
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
            assert_eq!(verify(d.path()), VerifyOutcome::Verified, "{rel}");
            let p = d.path().join(rel);
            if rel == CLOSURE_FILE {
                continue; // the closure is not in the inventory
            }
            // Mutate the file the entry describes: present ones by appending,
            // absent ones by creating.
            if p.exists() {
                let mut b = std::fs::read(&p).unwrap();
                b.extend_from_slice(b"x");
                std::fs::write(&p, b).unwrap();
            } else {
                if let Some(parent) = p.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(&p, b"appeared\n").unwrap();
            }
            let out = verify(d.path());
            assert!(
                matches!(out, VerifyOutcome::Mismatch { .. })
                    || matches!(out, VerifyOutcome::DamagedManifest),
                "{rel}: a mutation went undetected ({out:?})"
            );
            assert_eq!(
                out.terminal_exit_code(),
                Some(RunStatus::JournalInvalid.exit_code()),
                "{rel}"
            );
        }
    }

    /// §C14.3: every recomputed closure field is compared, and the two stored
    /// timestamps are **not** among them.
    #[test]
    fn verify_detects_a_mutation_in_every_recomputed_field() {
        type Mutate = (&'static str, Box<dyn Fn(&mut Closure)>);
        let cases: Vec<Mutate> = vec![
            (
                "run_uuid",
                Box::new(|c: &mut Closure| c.run_uuid = "1".repeat(32)),
            ),
            (
                "boot_id",
                Box::new(|c: &mut Closure| c.boot_id = "boot-9".into()),
            ),
            (
                "repo_commit",
                Box::new(|c: &mut Closure| c.repo_commit = "9".repeat(40)),
            ),
            (
                "amendment_commits",
                Box::new(|c: &mut Closure| c.amendment_commits[1] = "9".repeat(40)),
            ),
            (
                "terminal_status",
                Box::new(|c: &mut Closure| {
                    c.terminal_status = RunStatus::HostNotQualified;
                    c.exit_code = RunStatus::HostNotQualified.exit_code();
                }),
            ),
            (
                "session_states",
                Box::new(|c: &mut Closure| c.session_states[0] = Some(SessionState::Aborted)),
            ),
            (
                "failures_total",
                Box::new(|c: &mut Closure| {
                    c.failures_total = Some(1);
                    c.failures_by_session = Some(vec![1, 0, 0, 0, 0, 0]);
                }),
            ),
            (
                "verdict_rules",
                Box::new(|c: &mut Closure| {
                    if let Some(r) = c.verdict_rules.as_mut() {
                        r.rule2_failures = 1;
                    }
                }),
            ),
            (
                "leave_one_out",
                Box::new(|c: &mut Closure| {
                    if let Some(l) = c.leave_one_out.as_mut() {
                        l.full_90.median_spread = Some(0.5);
                    }
                }),
            ),
            (
                "control_outcomes",
                Box::new(|c: &mut Closure| {
                    c.control_outcomes[0].monotonic_offset_ms += 1;
                }),
            ),
            (
                "integrity",
                Box::new(|c: &mut Closure| c.integrity[0].sha256 = "f".repeat(64)),
            ),
            (
                "artifacts",
                Box::new(|c: &mut Closure| c.artifacts.observations_sha256 = Some("f".repeat(64))),
            ),
        ];
        for (what, mutate) in cases {
            let d = prepared_run("fieldmut");
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
            let mut c = match closure_path_state(d.path()) {
                ClosurePathState::Complete(c) => *c,
                other => panic!("{what}: {other:?}"),
            };
            mutate(&mut c);
            std::fs::write(d.path().join(CLOSURE_FILE), c.render()).unwrap();
            let out = verify(d.path());
            assert!(
                !matches!(out, VerifyOutcome::Verified),
                "{what}: the mutation went undetected"
            );
            assert_eq!(
                out.terminal_exit_code(),
                Some(RunStatus::JournalInvalid.exit_code()),
                "{what}"
            );
        }

        // The two timestamps are stored provenance (§C14.155): changing them
        // does not make a verification fail, because verify never recomputes
        // them.
        for stamp in ["finalized_utc", "finalized_monotonic_offset_ms"] {
            let d = prepared_run("stamp");
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
            let mut c = match closure_path_state(d.path()) {
                ClosurePathState::Complete(c) => *c,
                other => panic!("{other:?}"),
            };
            if stamp == "finalized_utc" {
                c.finalized_utc = "2099-01-01T00:00:00Z".into();
            } else {
                c.finalized_monotonic_offset_ms = Some(999_999);
            }
            std::fs::write(d.path().join(CLOSURE_FILE), c.render()).unwrap();
            assert_eq!(
                verify(d.path()),
                VerifyOutcome::Verified,
                "{stamp} is provenance, not a recomputed result"
            );
        }
    }

    /// §C14.3: a damaged manifest stops verification.
    #[test]
    fn verify_refuses_a_damaged_manifest() {
        let d = prepared_run("vmanifest");
        finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        std::fs::write(d.path().join("run.json"), b"{ truncated").unwrap();
        let out = verify(d.path());
        assert!(
            matches!(
                out,
                VerifyOutcome::DamagedManifest | VerifyOutcome::Mismatch { .. }
            ),
            "{out:?}"
        );
        assert_eq!(
            out.terminal_exit_code(),
            Some(RunStatus::JournalInvalid.exit_code())
        );
    }

    /// Verify with no closure path is an unmet precondition, and §C14 gives it
    /// no §8 status — so none is invented here.
    #[test]
    fn verify_without_a_closure_path_is_an_unmet_precondition() {
        let d = prepared_run("noclosure");
        let out = verify(d.path());
        assert_eq!(out, VerifyOutcome::NoClosurePath);
        assert_eq!(out.terminal_run_status(), None);
        assert!(!out.is_terminal());
        assert_eq!(out.mode_exit_code(), None);
    }

    // ------------------------------------------------ taxonomy and isolation

    /// §C17: unique directories, no attempt suffix, and no execution.
    #[test]
    fn test_directories_are_unique_and_nothing_is_executed() {
        let a = TempDir::new("iso");
        let b = TempDir::new("iso");
        assert_ne!(a.path(), b.path());
        for d in [&a, &b] {
            let name = d.path().file_name().unwrap().to_string_lossy().into_owned();
            assert!(name.contains(&std::process::id().to_string()));
            assert!(!name.contains(".1"), "no attempt suffix");
        }
        // Nothing in this module reaches a sentinel, a control or a session.
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/bin/exp_rc021_host_qualify/decision.rs"),
        )
        .unwrap();
        // Only the production half: this test's own banned list would otherwise
        // match itself.
        let production = src
            .split(
                "// ======================================================================= TESTS",
            )
            .next()
            .expect("the production half");
        for banned in [
            "run_sentinel",
            "run_work_block",
            "run_controls",
            "run_p2",
            "n3_replay",
        ] {
            assert!(
                !production.contains(banned),
                "decision.rs must not execute {banned}"
            );
        }
        assert!(production.len() > 1000, "the production half must be real");
    }

    // ============================================ ADVERSARIAL: §C13.7 COORDINATES

    /// A measurement row whose coordinates are **given**, not derived.
    ///
    /// `ok_row` computes its index with `session::measurement_index`, so it
    /// cannot express a wrong coordinate — which is exactly why none of the
    /// fixtures below use it for the rows under test. Each row here is
    /// individually valid: `Row::validate` range-checks `measurement_index`
    /// against `1..=90` and nothing more, so only the cross-row grammar can
    /// reject them.
    fn raw_ok_row(
        session: u8,
        phase: Phase,
        block: u8,
        index: u32,
        spread: f64,
    ) -> crate::journal::Row {
        let c = ctx();
        let mut r = c.lost(session, phase, block, Some(index), 1_000, "");
        r.status = Status::Ok;
        r.sentinel_first_ms = Some(2.0);
        r.sentinel_last_ms = Some(2.0 * (1.0 + spread));
        r.paired_spread = Some(spread);
        r.load_avg_start = Some(0.1);
        r.load_avg_end = Some(0.1);
        r
    }

    /// The frozen plan's rows, as the writer would emit them.
    fn planned_rows(session: u8) -> Vec<crate::journal::Row> {
        crate::session::plan_session(session)
            .unwrap()
            .iter()
            .map(|c| raw_ok_row(session, c.phase, c.block, c.measurement_index, 0.01))
            .collect()
    }

    /// Overwrite session `s`'s journal with `rows`, inside a real open/close.
    fn write_raw_session(
        dir: &Path,
        session: u8,
        rows: &[crate::journal::Row],
        close: Option<bool>,
    ) {
        let path = dir.join(session_journal_name(session));
        let _ = std::fs::remove_file(&path);
        let m = meta(crate::journal::MetaSession::Qualification(session));
        let c = ctx();
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(session, 10, 0.1)).unwrap();
        for r in rows {
            j.append(r).unwrap();
        }
        if let Some(completed) = close {
            j.append(&c.session_close(session, 20, completed, 0.1))
                .unwrap();
        }
    }

    fn classify_raw(
        name: &str,
        session: u8,
        rows: &[crate::journal::Row],
        close: Option<bool>,
    ) -> SessionClassification {
        let d = TempDir::new(name);
        write_raw_session(d.path(), session, rows, close);
        let outcome =
            crate::journal::read_journal(&d.path().join(session_journal_name(session))).unwrap();
        classify_session(session, &outcome, &fixture_identity(d.path()))
    }

    fn assert_journal_invalid(what: &str, c: &SessionClassification) {
        assert!(c.journal_invalid, "{what}: not rejected");
        assert_eq!(c.state, None, "{what}: a state was asserted anyway");
    }

    /// Every way fifteen individually-valid rows can fail to be the frozen
    /// plan. Each was `COMPLETED` with zero failures before the grammar.
    #[test]
    fn a_measurement_sequence_that_is_not_the_frozen_plan_is_journal_invalid() {
        let plan = crate::session::plan_session(1).unwrap();

        // 1. fifteen copies of one coordinate.
        let dup: Vec<_> = (0..15)
            .map(|_| raw_ok_row(1, Phase::A, 1, 1, 0.01))
            .collect();
        assert_journal_invalid("duplicates", &classify_raw("cdup", 1, &dup, Some(true)));

        // 2. another session's coordinates, in session 1's journal.
        let foreign: Vec<_> = crate::session::plan_session(6)
            .unwrap()
            .iter()
            .map(|c| raw_ok_row(1, c.phase, c.block, c.measurement_index, 0.01))
            .collect();
        assert_journal_invalid("foreign", &classify_raw("cfor", 1, &foreign, Some(true)));

        // 3. the right fifteen, reordered.
        let mut reordered = planned_rows(1);
        reordered.swap(3, 9);
        assert_journal_invalid(
            "reordered",
            &classify_raw("cord", 1, &reordered, Some(true)),
        );

        // 4. a skipped middle coordinate, padded back to fifteen rows so the
        //    cardinality test alone still passes.
        let mut skipped: Vec<_> = planned_rows(1);
        skipped.remove(7);
        skipped.push(raw_ok_row(1, Phase::C, 5, 16, 0.01));
        assert_eq!(skipped.len(), 15);
        assert_journal_invalid("skipped", &classify_raw("cskp", 1, &skipped, Some(true)));

        // 5. the right index, the wrong phase.
        let mut wrong_phase = planned_rows(1);
        wrong_phase[2] = raw_ok_row(1, Phase::C, 3, plan[2].measurement_index, 0.01);
        assert_journal_invalid("phase", &classify_raw("cph", 1, &wrong_phase, Some(true)));

        // 6. the right index and phase, the wrong block.
        let mut wrong_block = planned_rows(1);
        wrong_block[2] = raw_ok_row(1, plan[2].phase, 5, plan[2].measurement_index, 0.01);
        assert_journal_invalid("block", &classify_raw("cbl", 1, &wrong_block, Some(true)));

        // 7. the right address, a duplicated index.
        let mut dup_index = planned_rows(1);
        dup_index[4] = raw_ok_row(
            1,
            plan[4].phase,
            plan[4].block,
            plan[3].measurement_index,
            0.01,
        );
        assert_journal_invalid("index", &classify_raw("cix", 1, &dup_index, Some(true)));

        // 8. the whole correct plan, plus a sixteenth row.
        let mut sixteen = planned_rows(1);
        sixteen.push(raw_ok_row(1, Phase::A, 1, 1, 0.01));
        assert_journal_invalid("sixteenth", &classify_raw("c16", 1, &sixteen, Some(true)));
    }

    /// §5.2's lifecycle grammar: the rows around the measurements.
    #[test]
    fn lifecycle_violations_are_journal_invalid() {
        let d = TempDir::new("lifecycle");
        let c = ctx();
        let plan = planned_rows(1);

        // A measurement before SESSION-OPEN.
        let path = d.path().join("a.tsv");
        let m = meta(crate::journal::MetaSession::Qualification(1));
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&plan[0]).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        drop(j);
        assert_journal_invalid(
            "measurement before open",
            &classify_session(
                1,
                &crate::journal::read_journal(&path).unwrap(),
                &fixture_identity(d.path()),
            ),
        );

        // Two SESSION-OPEN rows.
        let path = d.path().join("b.tsv");
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        j.append(&c.session_open(1, 11, 0.1)).unwrap();
        drop(j);
        assert_journal_invalid(
            "two opens",
            &classify_session(
                1,
                &crate::journal::read_journal(&path).unwrap(),
                &fixture_identity(d.path()),
            ),
        );

        // Two close rows.
        let path = d.path().join("c.tsv");
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        for r in &plan {
            j.append(r).unwrap();
        }
        j.append(&c.session_close(1, 20, true, 0.1)).unwrap();
        j.append(&c.session_close(1, 21, true, 0.1)).unwrap();
        drop(j);
        assert_journal_invalid(
            "two closes",
            &classify_session(
                1,
                &crate::journal::read_journal(&path).unwrap(),
                &fixture_identity(d.path()),
            ),
        );

        // A measurement after the close row.
        let path = d.path().join("d.tsv");
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        for r in &plan {
            j.append(r).unwrap();
        }
        j.append(&c.session_close(1, 20, true, 0.1)).unwrap();
        j.append(&raw_ok_row(1, Phase::A, 1, 1, 0.01)).unwrap();
        drop(j);
        assert_journal_invalid(
            "row after close",
            &classify_session(
                1,
                &crate::journal::read_journal(&path).unwrap(),
                &fixture_identity(d.path()),
            ),
        );

        let early = planned_rows(1).into_iter().take(14).collect::<Vec<_>>();
        assert_journal_invalid(
            "completed close before the plan ended",
            &classify_raw("early_complete", 1, &early, Some(true)),
        );

        // Metadata binds the journal to its reserved session path even when no
        // measurement row is available to expose the mismatch.
        let path = d.path().join("foreign_metadata.tsv");
        let foreign = meta(crate::journal::MetaSession::Qualification(2));
        let mut j = crate::journal::Journal::create(&path, &foreign).unwrap();
        j.append(&c.session_open(2, 10, 0.1)).unwrap();
        drop(j);
        assert_journal_invalid(
            "metadata session does not match the path",
            &classify_session(
                1,
                &crate::journal::read_journal(&path).unwrap(),
                &fixture_identity(d.path()),
            ),
        );

        // EXTERNAL-CAUSE can justify only an abort. A completed close after it
        // is contradictory lifecycle evidence.
        let path = d.path().join("external_then_complete.tsv");
        let mut j = crate::journal::Journal::create(&path, &m).unwrap();
        j.append(&c.session_open(1, 10, 0.1)).unwrap();
        j.append(&c.external_cause(1, 11, 0.1, "power loss"))
            .unwrap();
        for r in &plan {
            j.append(r).unwrap();
        }
        j.append(&c.session_close(1, 20, true, 0.1)).unwrap();
        drop(j);
        assert_journal_invalid(
            "EXTERNAL-CAUSE followed by completed close",
            &classify_session(
                1,
                &crate::journal::read_journal(&path).unwrap(),
                &fixture_identity(d.path()),
            ),
        );
    }

    #[test]
    fn only_an_absent_session_is_not_started() {
        let d = TempDir::new("header_only");
        let path = d.path().join(session_journal_name(1));
        let m = meta(crate::journal::MetaSession::Qualification(1));
        drop(crate::journal::Journal::create(&path, &m).unwrap());
        let c = classify_session(
            1,
            &crate::journal::read_journal(&path).unwrap(),
            &fixture_identity(d.path()),
        );
        assert!(!c.journal_invalid);
        assert_eq!(c.state, Some(SessionState::Aborted));
        assert_eq!(c.unwritten, crate::session::MEASUREMENTS_PER_SESSION);

        let missing = classify_session(1, &ReadOutcome::Missing, &fixture_identity(d.path()));
        assert_eq!(missing.state, Some(SessionState::NotStarted));
    }

    /// The states the grammar must **not** reject: an honest incomplete run.
    #[test]
    fn an_in_order_prefix_of_the_plan_stays_aborted_and_is_never_journal_invalid() {
        // A crash after seven measurements, with an explicit abort row.
        let seven: Vec<_> = planned_rows(1).into_iter().take(7).collect();
        let c = classify_raw("prefix_abort", 1, &seven, Some(false));
        assert!(!c.journal_invalid);
        assert_eq!(c.state, Some(SessionState::Aborted));
        assert_eq!((c.physical_lost, c.logical_lost, c.unwritten), (0, 0, 8));
        assert_eq!(c.failures(), 8);

        // The same, with no close row at all: §5.2 says a crash is ABORTED.
        let c = classify_raw("prefix_crash", 1, &seven, None);
        assert!(!c.journal_invalid);
        assert_eq!(c.state, Some(SessionState::Aborted));
        assert_eq!(c.unwritten, 8);

        // A completed session is still COMPLETED.
        let c = classify_raw("prefix_full", 1, &planned_rows(1), Some(true));
        assert!(!c.journal_invalid);
        assert_eq!(c.state, Some(SessionState::Completed));
        assert_eq!(c.failures(), 0);
    }

    /// §C8.3: a truncated final line is one logical `LOST`, counted once.
    #[test]
    fn a_truncated_final_line_is_one_logical_lost_and_never_double_counted() {
        let d = TempDir::new("trunc_prefix");
        write_raw_session(
            d.path(),
            1,
            &planned_rows(1).into_iter().take(7).collect::<Vec<_>>(),
            None,
        );
        let path = d.path().join(session_journal_name(1));
        let mut bytes = std::fs::read(&path).unwrap();
        // Chop the trailing line feed and half of the last row.
        bytes.pop();
        bytes.truncate(bytes.len() - 20);
        std::fs::write(&path, &bytes).unwrap();

        let c = classify_session(
            1,
            &crate::journal::read_journal(&path).unwrap(),
            &fixture_identity(d.path()),
        );
        assert!(!c.journal_invalid, "a truncated tail is not a damaged row");
        assert_eq!(c.state, Some(SessionState::Aborted));
        assert_eq!(c.logical_lost, 1);
        assert_eq!(c.physical_lost, 0);
        // Six intact rows + one logical LOST + eight unwritten = fifteen.
        assert_eq!(c.ok_rows.len(), 6);
        assert_eq!(c.unwritten, 8);
        assert_eq!(c.ok_rows.len() as u32 + c.logical_lost + c.unwritten, 15);
    }

    /// End to end: a coordinate violation is exit 4, publishes no verdict, and
    /// verify agrees from the same bytes.
    #[test]
    fn a_coordinate_violation_finalizes_as_journal_invalid_exit_four() {
        let d = prepared_run("coord_e2e");
        let dup: Vec<_> = (0..15)
            .map(|_| raw_ok_row(1, Phase::A, 1, 1, 0.01))
            .collect();
        write_raw_session(d.path(), 1, &dup, Some(true));

        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::JournalInvalid
            }
        );
        assert_eq!(out.terminal_exit_code(), Some(4));
        let closure =
            Closure::parse(&std::fs::read_to_string(d.path().join(CLOSURE_FILE)).unwrap())
                .expect("the closure is canonical");
        assert_eq!(closure.terminal_status, RunStatus::JournalInvalid);
        assert!(closure.verdict_rules.is_none() && closure.leave_one_out.is_none());
        assert_eq!(closure.session_states[0], None);
        // §C13.8: a Class I outcome receives no artifacts.
        assert!(!d.path().join(OBSERVATIONS_FILE).exists());
        assert!(!d.path().join(RESULTS_FILE).exists());
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// §C13.85: the artifact of six sound sessions is ninety strictly ascending
    /// unique indices — the property the grammar now guarantees.
    #[test]
    fn observations_of_six_valid_sessions_are_ninety_unique_ascending_indices() {
        let d = prepared_run("obs_ascending");
        assert_eq!(
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None)),
            FinalizeOutcome::Wrote {
                status: RunStatus::HostQualified
            }
        );
        let obs = std::fs::read_to_string(d.path().join(OBSERVATIONS_FILE)).unwrap();
        let idx: Vec<u32> = obs
            .lines()
            .skip(1)
            .map(|l| l.split('\t').nth(3).unwrap().parse().unwrap())
            .collect();
        assert_eq!(idx.len(), crate::session::TOTAL_MEASUREMENTS as usize);
        assert!(idx.windows(2).all(|w| w[0] < w[1]), "strictly ascending");
        let mut uniq = idx.clone();
        uniq.sort_unstable();
        uniq.dedup();
        assert_eq!(uniq.len(), 90);
        assert_eq!((*uniq.first().unwrap(), *uniq.last().unwrap()), (1, 90));
    }

    #[test]
    fn a_present_but_unreadable_session_path_is_journal_invalid() {
        let d = prepared_run("session_io_error");
        let path = d.path().join(session_journal_name(1));
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();

        let ledger = read_ledger(d.path(), &fixture_identity(d.path()));
        assert_journal_invalid("session path is not a readable file", &ledger.sessions[0]);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::JournalInvalid
            }
        );
        assert_eq!(
            out.terminal_exit_code(),
            Some(RunStatus::JournalInvalid.exit_code())
        );
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    // ================================ §C12.3 / §C12.4 RUN-INVALID EVIDENCE

    fn run_invalid_record(
        run_uuid: &str,
        boot_expected: &str,
        fp_expected: &str,
    ) -> crate::session::RunInvalid {
        crate::session::RunInvalid {
            schema_version: crate::journal::SCHEMA_VERSION.into(),
            run_uuid: run_uuid.to_string(),
            boot_id_observed: "boot-9".into(),
            boot_id_expected: boot_expected.to_string(),
            host_fingerprint_observed: "f".repeat(64),
            host_fingerprint_expected: fp_expected.to_string(),
            reason: "boot_id mismatch".into(),
            command_line: vec!["exp_rc021_host_qualify".into()],
            monotonic_offset_ms: Some(5),
            utc: "2026-08-27T00:00:00Z".into(),
        }
    }

    fn finalized_status(dir: &Path) -> RunStatus {
        Closure::parse(&std::fs::read_to_string(dir.join(CLOSURE_FILE)).unwrap())
            .expect("the closure is canonical")
            .terminal_status
    }

    /// §C12.4: a record that cannot be read is not an absent one.
    #[test]
    fn a_damaged_run_invalid_is_journal_invalid_in_both_finalize_and_verify() {
        let d = prepared_run("ri_damaged");
        std::fs::write(
            d.path().join(crate::session::RUN_INVALID_FILE),
            b"{\"schema_ver",
        )
        .unwrap();
        assert_eq!(
            read_run_invalid_evidence(d.path()),
            RunInvalidEvidence::Unusable
        );
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::JournalInvalid
            }
        );
        assert_eq!(out.terminal_exit_code(), Some(4));
        assert_eq!(finalized_status(d.path()), RunStatus::JournalInvalid);
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// §C10.1: a canonical record about **another** run is not this run's
    /// terminal evidence, so it is exit 4 and never exit 3 on someone else's
    /// finding — the same doctrine `session::bind_run_invalid` states.
    #[test]
    fn a_canonical_run_invalid_of_another_run_is_journal_invalid_not_instrument_invalid() {
        let d = prepared_run("ri_foreign");
        let foreign = run_invalid_record(&"9".repeat(32), "boot-XX", &"e".repeat(64));
        foreign.validate().expect("the foreign record is canonical");
        std::fs::write(
            d.path().join(crate::session::RUN_INVALID_FILE),
            foreign.render(),
        )
        .unwrap();
        // It parses as `Valid`; only the binding step tells the two apart.
        assert!(matches!(
            crate::session::classify_run_invalid(d.path()),
            crate::session::RunInvalidState::Valid(_)
        ));
        assert_eq!(
            read_run_invalid_evidence(d.path()),
            RunInvalidEvidence::Unusable
        );
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(out.terminal_exit_code(), Some(4));
        assert_eq!(finalized_status(d.path()), RunStatus::JournalInvalid);
        assert_ne!(finalized_status(d.path()), RunStatus::InstrumentInvalid);
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// §C12.3: a record that **is** this run's keeps its Class I exit 3.
    #[test]
    fn a_bound_run_invalid_stays_instrument_invalid_exit_three() {
        let d = prepared_run("ri_bound");
        let hf = host_fields();
        let mine = run_invalid_record(&"0".repeat(32), "boot-1", &hf.fingerprint());
        std::fs::write(
            d.path().join(crate::session::RUN_INVALID_FILE),
            mine.render(),
        )
        .unwrap();
        assert_eq!(
            read_run_invalid_evidence(d.path()),
            RunInvalidEvidence::BoundToThisRun
        );
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );
        assert_eq!(out.terminal_exit_code(), Some(3));
        let c =
            Closure::parse(&std::fs::read_to_string(d.path().join(CLOSURE_FILE)).unwrap()).unwrap();
        assert!(c.verdict_rules.is_none(), "§C13.45");
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// The control: with no record at all the Class II path is untouched.
    #[test]
    fn an_absent_run_invalid_leaves_the_class_two_path_alone() {
        let d = prepared_run("ri_absent");
        assert_eq!(
            read_run_invalid_evidence(d.path()),
            RunInvalidEvidence::Absent
        );
        assert_eq!(
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None)),
            FinalizeOutcome::Wrote {
                status: RunStatus::HostQualified
            }
        );
    }

    // ==================================== §C13.6 THE PRE-MEASUREMENT REFUSAL

    /// §C14.1: a run that never began earns no closure — and, because the check
    /// precedes the reservation, no lock either.
    #[test]
    fn a_run_that_never_began_is_refused_before_measurement_and_reserves_nothing() {
        let d = TempDir::new("never_began");
        let m = manifest(d.path());
        std::fs::write(d.path().join(MANIFEST_FILE), m.render().unwrap()).unwrap();
        let before = snapshot(d.path());
        let mut io = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(out, FinalizeOutcome::RefusedBeforeMeasurement);
        assert_eq!(
            out.terminal_exit_code(),
            Some(RunStatus::RefusedBeforeMeasurement.exit_code()),
            "exit 2, through §8 and never an invented literal"
        );
        assert_eq!(*io.creates.borrow(), 0, "no reservation");
        assert!(!d.path().join(CLOSURE_FILE).exists());
        assert_eq!(snapshot(d.path()), before, "not one byte");
        // Correctable, exactly as §C14.155 permits when no path was claimed.
        assert_eq!(verify(d.path()), VerifyOutcome::NoClosurePath);
    }

    /// §C12.3: `run_invalid.json` alone is began-and-stopped evidence, so the
    /// Class I closure it earns is written.
    #[test]
    fn a_run_invalid_alone_permits_a_class_one_finalize() {
        let d = TempDir::new("ri_only");
        let m = manifest(d.path());
        std::fs::write(d.path().join(MANIFEST_FILE), m.render().unwrap()).unwrap();
        let hf = host_fields();
        let mine = run_invalid_record(&"0".repeat(32), "boot-1", &hf.fingerprint());
        std::fs::write(
            d.path().join(crate::session::RUN_INVALID_FILE),
            mine.render(),
        )
        .unwrap();
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );
        assert!(d.path().join(CLOSURE_FILE).exists());
    }

    /// §C13.6: `controls_started` without `controls_complete` is finalizable,
    /// but a PASS-only journal has no distinguishing failure evidence and is
    /// therefore the generic INSTRUMENT-INVALID case.
    #[test]
    fn controls_started_without_controls_complete_is_not_a_pure_preflight() {
        let d = TempDir::new("started_only");
        let m = manifest(d.path());
        std::fs::write(d.path().join(MANIFEST_FILE), m.render().unwrap()).unwrap();
        std::fs::create_dir_all(d.path().join("control")).unwrap();
        write_controls_started(d.path());
        write_passing_controls(d.path());
        let mut io = FakeIo::new(FailAt::None);
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_ne!(out, FinalizeOutcome::RefusedBeforeMeasurement);
        assert_eq!(*io.creates.borrow(), 1, "the closure was reserved");
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    #[test]
    fn class_two_requires_both_bound_markers() {
        let d = prepared_run("missing_started");
        std::fs::remove_file(d.path().join(crate::controls::CONTROLS_STARTED)).unwrap();
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);

        let d = TempDir::new("incomplete_controls");
        let m = manifest(d.path());
        std::fs::write(d.path().join(MANIFEST_FILE), m.render().unwrap()).unwrap();
        write_controls_started(d.path());
        write_control_prefix(
            d.path(),
            meta(crate::journal::MetaSession::Control),
            crate::controls::CONTROL_COUNT as usize - 1,
        );
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );
    }

    #[test]
    fn control_journal_and_completion_marker_are_bound_to_the_run_bytes() {
        // Twelve canonical PASS rows with another run's metadata cannot
        // certify this manifest.
        let d = prepared_run("foreign_control_journal");
        std::fs::remove_file(d.path().join("control/rc021_control_journal.tsv")).unwrap();
        std::fs::remove_file(d.path().join(crate::controls::CONTROLS_COMPLETE)).unwrap();
        let mut foreign = meta(crate::journal::MetaSession::Control);
        foreign.run_uuid = "9".repeat(32);
        write_control_prefix(d.path(), foreign, crate::controls::CONTROL_COUNT as usize);
        write_controls_complete(d.path());
        assert_eq!(
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None)),
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );

        // A canonical marker with a syntactically valid but false SHA is not
        // completion evidence for the journal on disk.
        let d = prepared_run("wrong_control_sha");
        let marker_path = d.path().join(crate::controls::CONTROLS_COMPLETE);
        let mut complete = crate::controls::ControlsComplete::parse(
            &std::fs::read_to_string(&marker_path).unwrap(),
        )
        .unwrap();
        complete.control_journal_sha256 = "a".repeat(64);
        std::fs::write(&marker_path, complete.render()).unwrap();
        assert_eq!(
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None)),
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );

        // The marker's frozen count must also equal the rows actually present.
        let d = TempDir::new("wrong_control_count");
        let m = manifest(d.path());
        std::fs::write(d.path().join(MANIFEST_FILE), m.render().unwrap()).unwrap();
        write_controls_started(d.path());
        write_control_prefix(
            d.path(),
            meta(crate::journal::MetaSession::Control),
            crate::controls::CONTROL_COUNT as usize - 1,
        );
        write_controls_complete(d.path());
        assert_eq!(
            finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None)),
            FinalizeOutcome::Wrote {
                status: RunStatus::InstrumentInvalid
            }
        );
    }

    /// The §C13 damage table: a marker that is present but not this run's
    /// canonical one is `INSTRUMENT-INVALID`, not a refusal and not a verdict.
    #[test]
    fn a_damaged_or_foreign_marker_is_instrument_invalid_not_a_refusal() {
        for (name, bytes) in [
            ("marker_damaged", b"{}\n".to_vec()),
            (
                "marker_foreign",
                {
                    let mut r = controls_started_record();
                    r.run_uuid = "9".repeat(32);
                    r.render()
                }
                .into_bytes(),
            ),
        ] {
            let d = prepared_run(name);
            std::fs::write(d.path().join(crate::controls::CONTROLS_STARTED), &bytes).unwrap();
            let mut io = FakeIo::new(FailAt::None);
            let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
            assert_ne!(out, FinalizeOutcome::RefusedBeforeMeasurement, "{name}");
            assert_eq!(
                out,
                FinalizeOutcome::Wrote {
                    status: RunStatus::InstrumentInvalid
                },
                "{name}"
            );
            assert_eq!(out.terminal_exit_code(), Some(3), "{name}");
            assert_eq!(verify(d.path()), VerifyOutcome::Verified, "{name}");
        }
    }

    // ============================== §C14.15 THE CLOSURE SELF-CHECK BEFORE WRITE

    /// The postcondition, asserted directly across every class this module can
    /// reach: **whatever** finalize reports as `Wrote`, the bytes it left are
    /// bytes its own canonical reader accepts.
    #[test]
    fn every_wrote_leaves_a_closure_its_own_reader_accepts() {
        type Arrange = Box<dyn Fn(&Path)>;
        let mut cases: Vec<(&str, Arrange)> = Vec::new();
        cases.push(("clean", Box::new(|_: &Path| {})));
        cases.push((
            "coordinates",
            Box::new(move |d: &Path| {
                let dup: Vec<_> = (0..15)
                    .map(|_| raw_ok_row(1, Phase::A, 1, 1, 0.01))
                    .collect();
                write_raw_session(d, 1, &dup, Some(true));
            }),
        ));
        cases.push((
            "aborted",
            Box::new(|d: &Path| {
                let prefix: Vec<_> = planned_rows(3).into_iter().take(4).collect();
                write_raw_session(d, 3, &prefix, None);
            }),
        ));
        cases.push((
            "not_started",
            Box::new(|d: &Path| {
                std::fs::remove_file(d.join(session_journal_name(5))).unwrap();
            }),
        ));
        cases.push((
            "run_invalid",
            Box::new(|d: &Path| {
                let hf = host_fields();
                let mine = run_invalid_record(&"0".repeat(32), "boot-1", &hf.fingerprint());
                std::fs::write(d.join(crate::session::RUN_INVALID_FILE), mine.render()).unwrap();
            }),
        ));
        cases.push((
            "marker",
            Box::new(|d: &Path| {
                std::fs::write(d.join(crate::controls::CONTROLS_STARTED), b"{}\n").unwrap();
            }),
        ));

        for (name, arrange) in cases {
            let d = prepared_run(&format!("post_{name}"));
            arrange(d.path());
            let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
            let FinalizeOutcome::Wrote { status } = out else {
                panic!("{name}: expected a write, got {out:?}");
            };
            let raw = std::fs::read_to_string(d.path().join(CLOSURE_FILE)).unwrap();
            let parsed = Closure::parse(&raw)
                .unwrap_or_else(|e| panic!("{name}: finalize wrote a closure it cannot read: {e}"));
            assert_eq!(parsed.terminal_status, status, "{name}");
            assert_eq!(parsed.exit_code, status.exit_code(), "{name}");
            assert_eq!(verify(d.path()), VerifyOutcome::Verified, "{name}");
        }
    }

    /// §C14.15: a derived closure that fails its own reader is never written.
    /// The reservation stands — **empty** — and locks the run at exit 4.
    #[test]
    fn a_derived_closure_that_fails_its_own_reader_is_never_written() {
        let d = prepared_run("selfcheck");
        let mut io = FakeIo::new(FailAt::None);
        io.corrupt_closure = true;
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut io);
        assert_eq!(out, FinalizeOutcome::DerivedClosureInvalid);
        assert_eq!(out.terminal_exit_code(), Some(4));

        let path = d.path().join(CLOSURE_FILE);
        assert!(path.exists(), "the reservation stands");
        assert_eq!(std::fs::read(&path).unwrap(), Vec::<u8>::new(), "empty");
        assert_eq!(
            closure_path_state(d.path()),
            ClosurePathState::Empty,
            "§C14.2"
        );
        // §C14.16: the lock is permanent — a repeat writes nothing.
        let mut io2 = FakeIo::new(FailAt::None);
        assert_eq!(
            finalize_with_io(d.path(), TEST_RUN_ID, &mut io2),
            FinalizeOutcome::EmptyOrPartialClosure
        );
        assert_eq!(*io2.creates.borrow(), 0);
        // §C14.3: only a valid complete closure can be verified against.
        assert_eq!(verify(d.path()), VerifyOutcome::EmptyOrPartial);
        assert_eq!(
            VerifyOutcome::EmptyOrPartial.terminal_exit_code(),
            Some(RunStatus::JournalInvalid.exit_code())
        );
    }

    /// §C8.3 and the frozen plan: the logical `LOST` from a truncated line
    /// takes the **next** plan position, so a truncated line after the
    /// fifteenth measurement is a sixteenth measurement.
    ///
    /// Without the guard the session carries sixteen measurements against a
    /// per-session count Amendment 1 §A6 freezes at fifteen, `full_90` sums to
    /// 91, and the closure fails its own reader — trading a written
    /// `JOURNAL-INVALID` record for an empty lock with no record at all.
    #[test]
    fn a_truncated_line_after_the_full_plan_is_a_sixteenth_measurement() {
        let d = prepared_run("trunc_overflow");
        write_raw_session(d.path(), 1, &planned_rows(1), None);
        let path = d.path().join(session_journal_name(1));
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            // A real partial row: content, and no line feed to terminate it.
            f.write_all(b"rc021/1\tpartial-sixteenth").unwrap();
        }
        let read = crate::journal::read_journal(&path).unwrap();
        let crate::journal::ReadOutcome::Present(r) = &read else {
            panic!("the journal is present");
        };
        assert!(r.logical_lost_from_truncation, "the tail is truncated");
        assert_journal_invalid(
            "truncated sixteenth",
            &classify_session(1, &read, &fixture_identity(d.path())),
        );

        // The run is finalized as a written JOURNAL-INVALID record, not as a
        // closure the reader would reject.
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::JournalInvalid
            }
        );
        assert_eq!(out.terminal_exit_code(), Some(4));
        Closure::parse(&std::fs::read_to_string(d.path().join(CLOSURE_FILE)).unwrap())
            .expect("the closure is canonical");
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// A session journal from another run must not count toward the ninety.
    ///
    /// `Row::validate` cross-checks each row against **its own file's** header,
    /// so a journal lifted from a different run — or a different host — parses
    /// cleanly. Every other durable input was bound to the run identity: the
    /// control journal, both markers, and a predecessor journal in the CLI. The
    /// six the verdict is actually derived from were not, so fifteen foreign
    /// measurements counted, and `--verify` re-derived through the same path
    /// and confirmed the result instead of catching it.
    #[test]
    fn a_session_journal_from_another_run_is_not_this_runs_evidence() {
        let d = prepared_run("foreign_session");
        // A complete, internally consistent session 4 belonging to another run.
        let mut foreign = manifest(d.path());
        foreign.run_uuid = "7".repeat(32);
        let path = d.path().join(session_journal_name(4));
        std::fs::remove_file(&path).unwrap();
        let meta = crate::journal::Metadata {
            run_uuid: foreign.run_uuid.clone(),
            ..meta(crate::journal::MetaSession::Qualification(4))
        };
        let ctx = crate::journal::RowContext {
            run_uuid: foreign.run_uuid.clone(),
            ..ctx()
        };
        let mut j = crate::journal::Journal::create(&path, &meta).unwrap();
        j.append(&ctx.session_open(4, 10, 0.1)).unwrap();
        for coord in crate::session::plan_session(4).unwrap() {
            let mut r = ctx.lost(
                4,
                coord.phase,
                coord.block,
                Some(coord.measurement_index),
                1_000,
                "",
            );
            r.status = Status::Ok;
            r.sentinel_first_ms = Some(2.0);
            r.sentinel_last_ms = Some(2.02);
            r.paired_spread = Some(0.01);
            r.load_avg_start = Some(0.1);
            r.load_avg_end = Some(0.1);
            j.append(&r).unwrap();
        }
        j.append(&ctx.session_close(4, 20, true, 0.1)).unwrap();
        drop(j);

        // On its own terms the file is flawless.
        let read = crate::journal::read_journal(&path).unwrap();
        let crate::journal::ReadOutcome::Present(r) = &read else {
            panic!("present");
        };
        assert!(matches!(r.verdict, crate::journal::ReadVerdict::Valid));

        // Against this run's identity it is not evidence at all.
        let c = classify_session(4, &read, &fixture_identity(d.path()));
        assert!(c.journal_invalid, "a foreign journal must not classify");
        assert_eq!(c.state, None);

        // And the run therefore cannot be qualified from it.
        let out = finalize_with_io(d.path(), TEST_RUN_ID, &mut FakeIo::new(FailAt::None));
        assert_eq!(
            out,
            FinalizeOutcome::Wrote {
                status: RunStatus::JournalInvalid
            },
            "fifteen foreign measurements must never reach the ninety"
        );
        assert_eq!(verify(d.path()), VerifyOutcome::Verified);
    }

    /// §C13.4: `parsed_row_count` is the reader's count, `data_row_count` the
    /// file's. Returning one number for both made a torn final line count as
    /// parsed, beside a `status_counts` of all zeros.
    #[test]
    fn a_torn_final_line_is_not_counted_as_parsed() {
        let d = prepared_run("torn_control");
        let path = d.path().join("control/rc021_control_journal.tsv");
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.pop(); // remove the trailing line feed: the last write was torn
        std::fs::write(&path, &bytes).unwrap();
        let entry = build_integrity(d.path())
            .into_iter()
            .find(|e| e.path == "control/rc021_control_journal.tsv")
            .unwrap();
        let data = entry.data_row_count.unwrap();
        let parsed = entry.parsed_row_count.unwrap();
        assert_eq!(data, parsed + 1, "the torn line is data but was not parsed");
        // And the record no longer contradicts itself: an all-or-nothing reader
        // that produced no statuses must not claim it parsed every row.
        if let Some(StatusCountsRecord::Control { pass, fail }) = entry.status_counts {
            assert!(
                pass + fail <= parsed,
                "status counts {pass}+{fail} exceed parsed {parsed}"
            );
        }
    }
}
