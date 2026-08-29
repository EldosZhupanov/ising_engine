//! RC-021 §6 / §C11 — the twelve controls, their frozen order, the two markers
//! and the central control journal.
//!
//! Execution is injected. Nothing here runs a control by itself: the caller
//! supplies a [`ControlRunner`] and a [`Clock`], so a test can drive every
//! ordering, durability and classification path without executing a sentinel,
//! touching the host, or waiting ten minutes.

#![allow(dead_code)] // The CLI that drives this arrives in a later commit.

use crate::journal::{json_string_decode, json_string_encode, MetaSession, Metadata, META_KEYS};
use crate::manifest::fsync_dir_at;
use crate::manifest::RunManifest;
use crate::protocol::WorkBlock;
use std::io::Write;
use std::path::{Path, PathBuf};

// ============================================================ §8 TAXONOMY

/// §8, the six **run-level** statuses. A closed set, preregistered in full.
///
/// They never appear in a journal's `status` column — §5.1's vocabulary is
/// separate — and the run status is *derived*, never written by a measuring
/// loop. This enum exists here because P3 is the control that checks the
/// derivation is total and unique, and Step 7 consumes the same type when it
/// implements the §7.2 predicate and the closure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunStatus {
    RefusedBeforeMeasurement,
    InstrumentInvalid,
    JournalInvalid,
    HostNotQualified,
    HostQualified,
    InconclusiveUnderpowered,
}

pub const RUN_STATUSES: [RunStatus; 6] = [
    RunStatus::RefusedBeforeMeasurement,
    RunStatus::InstrumentInvalid,
    RunStatus::JournalInvalid,
    RunStatus::HostNotQualified,
    RunStatus::HostQualified,
    RunStatus::InconclusiveUnderpowered,
];

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::RefusedBeforeMeasurement => "REFUSED-BEFORE-MEASUREMENT",
            RunStatus::InstrumentInvalid => "INSTRUMENT-INVALID",
            RunStatus::JournalInvalid => "JOURNAL-INVALID",
            RunStatus::HostNotQualified => "HOST-NOT-QUALIFIED",
            RunStatus::HostQualified => "HOST-QUALIFIED",
            RunStatus::InconclusiveUnderpowered => "INCONCLUSIVE-UNDERPOWERED",
        }
    }
    /// The inverse of [`RunStatus::as_str`], for reading a stored closure.
    pub fn parse(s: &str) -> Option<RunStatus> {
        RUN_STATUSES.iter().copied().find(|r| r.as_str() == s)
    }

    /// §8: Class I is not a result; Class II is published.
    pub fn is_class_two(self) -> bool {
        matches!(self, RunStatus::HostNotQualified | RunStatus::HostQualified)
    }
    /// The frozen table. Each status has its **own** code: a run that did not
    /// qualify is a published Class II result, not a success, and an
    /// underpowered run is not the same failure as a broken instrument.
    pub fn exit_code(self) -> i32 {
        match self {
            RunStatus::HostQualified => 0,
            RunStatus::HostNotQualified => 1,
            RunStatus::RefusedBeforeMeasurement => 2,
            RunStatus::InstrumentInvalid => 3,
            RunStatus::JournalInvalid => 4,
            RunStatus::InconclusiveUnderpowered => 5,
        }
    }
}

/// §7.2 / §8: what a completed run's verdict was. Step 7 produces these.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VerdictOutcome {
    Qualified,
    NotQualified,
    /// §8.1, terminal within RC-021.
    Underpowered,
}

pub const VERDICT_VARIANTS: [&str; 3] = ["Qualified", "NotQualified", "Underpowered"];

impl VerdictOutcome {
    pub fn variant_name(self) -> &'static str {
        match self {
            VerdictOutcome::Qualified => "Qualified",
            VerdictOutcome::NotQualified => "NotQualified",
            VerdictOutcome::Underpowered => "Underpowered",
        }
    }
    /// A verdict is always terminal: it *is* the run's outcome.
    pub fn run_status(self) -> RunStatus {
        match self {
            VerdictOutcome::Qualified => RunStatus::HostQualified,
            VerdictOutcome::NotQualified => RunStatus::HostNotQualified,
            VerdictOutcome::Underpowered => RunStatus::InconclusiveUnderpowered,
        }
    }
}

/// The **mode's** exit code for a step that finished its work without ending
/// the run — a completed control phase, a session cleared to begin.
///
/// This is deliberately a different thing from a §8 run status, and is named so
/// it cannot be mistaken for one: §8's `REFUSED-BEFORE-MEASUREMENT` means "the
/// run did not start" and consumes no seeds, which is false once the controls
/// have executed. Step 8 returns this for a successful mode operation; it never
/// appears in a closure.
pub const MODE_SUCCESS_EXIT_CODE: i32 = 0;

/// Every terminal program path there is, as the value the program returns. P3
/// builds one of each and puts it through the production classification — there
/// is no second table for it to agree with.
#[derive(Clone, Debug)]
pub enum TerminalOutcome {
    Controls(ControlsOutcome),
    Session(crate::session::SessionOutcome),
    Verdict(VerdictOutcome),
    Finalize(crate::decision::FinalizeOutcome),
    Verify(crate::decision::VerifyOutcome),
}

impl TerminalOutcome {
    /// The one §8 status this path carries. Total by construction: only
    /// terminal values reach here, and each terminal variant has exactly one.
    pub fn run_status(&self) -> RunStatus {
        match self {
            TerminalOutcome::Controls(o) => o
                .terminal_run_status()
                .expect("a non-terminal ControlsOutcome is not a terminal path"),
            TerminalOutcome::Session(o) => o
                .terminal_run_status()
                .expect("a non-terminal SessionOutcome is not a terminal path"),
            TerminalOutcome::Verdict(v) => v.run_status(),
            TerminalOutcome::Finalize(o) => o
                .terminal_run_status()
                .expect("a non-terminal FinalizeOutcome is not a terminal path"),
            TerminalOutcome::Verify(o) => o
                .terminal_run_status()
                .expect("a non-terminal VerifyOutcome is not a terminal path"),
        }
    }
    pub fn variant_name(&self) -> String {
        match self {
            TerminalOutcome::Controls(o) => format!("Controls::{}", o.variant_name()),
            TerminalOutcome::Session(o) => format!("Session::{}", o.variant_name()),
            TerminalOutcome::Verdict(v) => format!("Verdict::{}", v.variant_name()),
            TerminalOutcome::Finalize(o) => format!("Finalize::{}", o.variant_name()),
            TerminalOutcome::Verify(o) => format!("Verify::{}", o.variant_name()),
        }
    }
}

/// The complete variant inventory of each outcome enum, terminal or not. P3
/// checks that every **terminal** one has a representative and that every
/// non-terminal one is excluded on purpose, so a new variant cannot slip
/// through either way.
pub const CONTROLS_VARIANTS: [&str; 5] = [
    "Complete",
    "RefusedBeforeMeasurement",
    "Failed",
    "AlreadyStarted",
    "WriteFailed",
];
pub const SESSION_VARIANTS: [&str; 10] = [
    "Proceed",
    "ShortGap",
    "Terminal",
    "AlreadyInvalid",
    "DamagedRunInvalid",
    "PredecessorNotClosed",
    "InvalidRequest",
    "HostChangedMidSession",
    "MeasurementFailed",
    "JournalWriteFailed",
];

/// The terminal subset of [`CONTROLS_VARIANTS`], derived from the
/// classification rather than listed a second time.
pub fn terminal_controls_variants() -> Vec<&'static str> {
    sample_controls_outcomes()
        .into_iter()
        .filter(|o| o.is_terminal())
        .map(|o| o.variant_name())
        .collect()
}

/// The terminal subset of `decision::FINALIZE_VARIANTS`.
pub fn terminal_finalize_variants() -> Vec<&'static str> {
    sample_finalize_outcomes()
        .into_iter()
        .filter(|o| o.is_terminal())
        .map(|o| o.variant_name())
        .collect()
}

/// The terminal subset of `decision::VERIFY_VARIANTS`.
pub fn terminal_verify_variants() -> Vec<&'static str> {
    sample_verify_outcomes()
        .into_iter()
        .filter(|o| o.is_terminal())
        .map(|o| o.variant_name())
        .collect()
}

/// Whether a P3 sample lists **exactly** its frozen inventory.
///
/// Sorted-multiset equality, so it rejects a missing name, an extra name **and**
/// a same-length sample that duplicates one name while dropping another — the
/// last of which a length check plus a de-duplicated comparison lets through.
pub fn sample_is_frozen_inventory(sampled: &[&str], frozen: &[&str]) -> bool {
    let mut got = sampled.to_vec();
    got.sort_unstable();
    let mut want = frozen.to_vec();
    want.sort_unstable();
    got == want
}

/// One value of **every** `FinalizeOutcome` variant.
pub fn sample_finalize_outcomes() -> Vec<crate::decision::FinalizeOutcome> {
    use crate::decision::FinalizeOutcome as F;
    vec![
        F::Wrote {
            status: RunStatus::HostQualified,
        },
        F::DurabilityFailed { at: "sync_all" },
        F::AlreadyComplete,
        F::EmptyOrPartialClosure,
        F::ArtifactWithoutClosure,
        F::Unreconstructible,
        F::RefusedBeforeMeasurement,
        F::DerivedClosureInvalid,
        F::RunIdMismatch,
    ]
}

/// One value of **every** `VerifyOutcome` variant, terminal or not.
pub fn sample_verify_outcomes() -> Vec<crate::decision::VerifyOutcome> {
    use crate::decision::VerifyOutcome as V;
    vec![
        V::Verified,
        V::Mismatch {
            what: "integrity entry run.json".to_string(),
        },
        V::EmptyOrPartial,
        V::DamagedManifest,
        V::NoClosurePath,
    ]
}

/// The terminal subset of [`SESSION_VARIANTS`].
pub fn terminal_session_variants() -> Vec<&'static str> {
    sample_session_outcomes()
        .into_iter()
        .filter(|o| o.is_terminal())
        .map(|o| o.variant_name())
        .collect()
}

/// One value of **every** `ControlsOutcome` variant, terminal or not.
pub fn sample_controls_outcomes() -> Vec<ControlsOutcome> {
    vec![
        ControlsOutcome::Complete {
            monotonic_offset_ms: 600_000,
        },
        ControlsOutcome::RefusedBeforeMeasurement {
            detail: json_string_encode("P6 failed"),
        },
        ControlsOutcome::Failed {
            control: ControlId::P5,
            class: FailureClass::InstrumentInvalid,
        },
        ControlsOutcome::AlreadyStarted,
        ControlsOutcome::WriteFailed {
            why: "sync_all failed".to_string(),
        },
    ]
}

/// One value of **every** `SessionOutcome` variant, terminal or not.
pub fn sample_session_outcomes() -> Vec<crate::session::SessionOutcome> {
    use crate::session::{GapAnchor, SessionOutcome as S};
    vec![
        S::Proceed {
            gap_ms: 600_000,
            anchor: GapAnchor::ControlsComplete {
                monotonic_offset_ms: 0,
            },
            order_seed: 31004,
        },
        S::ShortGap {
            gap_ms: 1,
            required_ms: 600_000,
        },
        S::Terminal {
            reason: "configuration mismatch".to_string(),
        },
        S::AlreadyInvalid,
        S::DamagedRunInvalid {
            why: "truncated".to_string(),
        },
        S::PredecessorNotClosed {
            session: 3,
            predecessor: 2,
        },
        S::InvalidRequest {
            reason: "session 9".to_string(),
        },
        S::HostChangedMidSession {
            why: "boot_id changed mid-session".to_string(),
        },
        S::MeasurementFailed {
            why: "sentinel could not run".to_string(),
        },
        S::JournalWriteFailed {
            why: "sync_all failed".to_string(),
        },
    ]
}

/// One representative per **terminal** variant.
///
/// `Controls::Complete` and `Session::Proceed` are absent on purpose: they are
/// continuations, not outcomes. §8 has no "in progress" status, so giving them
/// one would mean asserting something false — `REFUSED-BEFORE-MEASUREMENT`
/// claims "the run did not start" and "seeds consumed: none", both untrue once
/// the controls have run.
pub fn representative_terminal_outcomes() -> Vec<TerminalOutcome> {
    use crate::session::SessionOutcome as S;
    vec![
        TerminalOutcome::Controls(ControlsOutcome::RefusedBeforeMeasurement {
            detail: json_string_encode("P6 failed"),
        }),
        // Both failure classes: §C11.25 gives them different §8 statuses.
        TerminalOutcome::Controls(ControlsOutcome::Failed {
            control: ControlId::P5,
            class: FailureClass::InstrumentInvalid,
        }),
        TerminalOutcome::Controls(ControlsOutcome::Failed {
            control: ControlId::P2,
            class: FailureClass::JournalInvalid,
        }),
        TerminalOutcome::Controls(ControlsOutcome::AlreadyStarted),
        TerminalOutcome::Controls(ControlsOutcome::WriteFailed {
            why: "sync_all failed".to_string(),
        }),
        TerminalOutcome::Session(S::ShortGap {
            gap_ms: 1,
            required_ms: 600_000,
        }),
        TerminalOutcome::Session(S::Terminal {
            reason: "configuration mismatch".to_string(),
        }),
        TerminalOutcome::Session(S::AlreadyInvalid),
        TerminalOutcome::Session(S::DamagedRunInvalid {
            why: "truncated".to_string(),
        }),
        TerminalOutcome::Session(S::PredecessorNotClosed {
            session: 3,
            predecessor: 2,
        }),
        TerminalOutcome::Session(S::InvalidRequest {
            reason: "session 9".to_string(),
        }),
        TerminalOutcome::Session(S::HostChangedMidSession {
            why: "boot_id changed mid-session".to_string(),
        }),
        TerminalOutcome::Session(S::MeasurementFailed {
            why: "sentinel could not run".to_string(),
        }),
        TerminalOutcome::Session(S::JournalWriteFailed {
            why: "sync_all failed".to_string(),
        }),
        TerminalOutcome::Verdict(VerdictOutcome::Qualified),
        TerminalOutcome::Verdict(VerdictOutcome::NotQualified),
        TerminalOutcome::Verdict(VerdictOutcome::Underpowered),
        // §C14: every finalize outcome ends the run.
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::Wrote {
            status: RunStatus::HostQualified,
        }),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::DurabilityFailed {
            at: "sync_all",
        }),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::AlreadyComplete),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::EmptyOrPartialClosure),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::ArtifactWithoutClosure),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::Unreconstructible),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::RefusedBeforeMeasurement),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::DerivedClosureInvalid),
        TerminalOutcome::Finalize(crate::decision::FinalizeOutcome::RunIdMismatch),
        // §C14.2: `Verified` and `NoClosurePath` are continuations and are
        // deliberately absent, exactly like `Complete` and `Proceed`.
        TerminalOutcome::Verify(crate::decision::VerifyOutcome::Mismatch {
            what: "integrity entry run.json".to_string(),
        }),
        TerminalOutcome::Verify(crate::decision::VerifyOutcome::EmptyOrPartial),
        TerminalOutcome::Verify(crate::decision::VerifyOutcome::DamagedManifest),
    ]
}

// ================================================================ THE ORDER// ================================================================ THE ORDER

/// §6 and §C11.2. The closed domain of control identifiers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlId {
    P6,
    P5,
    P4,
    P3,
    P2,
    P8,
    N2,
    N1,
    N3,
    P1,
    P7,
    C10,
}

impl ControlId {
    pub fn as_str(self) -> &'static str {
        match self {
            ControlId::P6 => "P6",
            ControlId::P5 => "P5",
            ControlId::P4 => "P4",
            ControlId::P3 => "P3",
            ControlId::P2 => "P2",
            ControlId::P8 => "P8",
            ControlId::N2 => "N2",
            ControlId::N1 => "N1",
            ControlId::N3 => "N3",
            ControlId::P1 => "P1",
            ControlId::P7 => "P7",
            ControlId::C10 => "C10",
        }
    }
    pub fn parse(s: &str) -> Option<ControlId> {
        CONTROL_ORDER.iter().find(|c| c.as_str() == s).copied()
    }
    /// §C11.25: a control's own §6 class wins over the generic post-marker rule.
    /// §6 gives **P2** the class `JOURNAL-INVALID`; every other control falls
    /// through to `INSTRUMENT-INVALID`.
    pub fn failure_class(self) -> FailureClass {
        match self {
            ControlId::P2 => FailureClass::JournalInvalid,
            _ => FailureClass::InstrumentInvalid,
        }
    }
    /// The fixed position `1..12`.
    pub fn ordinal(self) -> u32 {
        CONTROL_ORDER
            .iter()
            .position(|c| *c == self)
            .map(|i| i as u32 + 1)
            .unwrap_or(0)
    }
}

/// §C11.2, frozen: `P6 → P5 → P4 → P3 → P2 → P8 → N2 → N1 → N3 → P1 → P7 → C10`.
///
/// §C11.5 explains the tail: the two heaviest controls are placed late, `N3` at
/// ordinal 9 with `P1` and `P7` between it and criterion 10, which executes last
/// immediately before the marker.
pub const CONTROL_ORDER: [ControlId; 12] = [
    ControlId::P6,
    ControlId::P5,
    ControlId::P4,
    ControlId::P3,
    ControlId::P2,
    ControlId::P8,
    ControlId::N2,
    ControlId::N1,
    ControlId::N3,
    ControlId::P1,
    ControlId::P7,
    ControlId::C10,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlStatus {
    Pass,
    Fail,
}

impl ControlStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ControlStatus::Pass => "PASS",
            ControlStatus::Fail => "FAIL",
        }
    }
    pub fn parse(s: &str) -> Option<ControlStatus> {
        match s {
            "PASS" => Some(ControlStatus::Pass),
            "FAIL" => Some(ControlStatus::Fail),
            _ => None,
        }
    }
}

/// §C11.25 / §C12.4: which class a failure carries, and the exit code that is
/// part of the frozen semantics.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailureClass {
    InstrumentInvalid,
    JournalInvalid,
}

impl FailureClass {
    pub fn exit_code(self) -> i32 {
        self.run_status().exit_code()
    }

    pub fn as_str(self) -> &'static str {
        self.run_status().as_str()
    }
    /// §C11.25's classes are §8 statuses; there is one vocabulary, not two.
    pub fn run_status(self) -> RunStatus {
        match self {
            FailureClass::InstrumentInvalid => RunStatus::InstrumentInvalid,
            FailureClass::JournalInvalid => RunStatus::JournalInvalid,
        }
    }
}

// ========================================================== THE CONTROL ROW

/// §C11.3, the closed five-column schema.
pub const CONTROL_COLUMNS: [&str; 5] = [
    "control_id",
    "ordinal",
    "status",
    "monotonic_offset_ms",
    "detail",
];

pub fn control_header_line() -> String {
    let mut s = CONTROL_COLUMNS.join("\t");
    s.push('\n');
    s
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ControlRow {
    pub control_id: ControlId,
    pub ordinal: u32,
    pub status: ControlStatus,
    pub monotonic_offset_ms: u64,
    /// A JSON string, quotes included — the same encoding as §C8's `reason`.
    pub detail: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ControlError {
    Domain(&'static str),
    Field {
        what: &'static str,
        value: String,
    },
    Io(String),
    /// §C10.1: the path is already claimed. Never truncated, never overwritten.
    AlreadyPresent(String),
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlError::Domain(d) => write!(f, "{d}"),
            ControlError::Field { what, value } => write!(f, "{what}: {value}"),
            ControlError::Io(e) => write!(f, "io: {e}"),
            ControlError::AlreadyPresent(p) => write!(f, "{p} already exists"),
        }
    }
}

impl ControlRow {
    /// The ordinal must be the one the frozen order assigns; a row that claimed
    /// a different position would reorder the record of execution.
    pub fn validate(&self) -> Result<(), ControlError> {
        if self.ordinal != self.control_id.ordinal() {
            return Err(ControlError::Field {
                what: "ordinal",
                value: self.ordinal.to_string(),
            });
        }
        json_string_decode(&self.detail).map_err(|_| ControlError::Field {
            what: "detail",
            value: self.detail.clone(),
        })?;
        Ok(())
    }

    pub fn encode(&self) -> Result<String, ControlError> {
        self.validate()?;
        Ok(format!(
            "{}\t{}\t{}\t{}\t{}\n",
            self.control_id.as_str(),
            self.ordinal,
            self.status.as_str(),
            self.monotonic_offset_ms,
            self.detail
        ))
    }

    pub fn parse(line: &str) -> Result<ControlRow, ControlError> {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let f: Vec<&str> = body.split('\t').collect();
        if f.len() != CONTROL_COLUMNS.len() {
            return Err(ControlError::Domain("control row must have five fields"));
        }
        let control_id = ControlId::parse(f[0]).ok_or(ControlError::Field {
            what: "control_id",
            value: f[0].to_string(),
        })?;
        let ordinal: u32 = f[1].parse().map_err(|_| ControlError::Field {
            what: "ordinal",
            value: f[1].to_string(),
        })?;
        let status = ControlStatus::parse(f[2]).ok_or(ControlError::Field {
            what: "status",
            value: f[2].to_string(),
        })?;
        let monotonic_offset_ms: u64 = f[3].parse().map_err(|_| ControlError::Field {
            what: "monotonic_offset_ms",
            value: f[3].to_string(),
        })?;
        let r = ControlRow {
            control_id,
            ordinal,
            status,
            monotonic_offset_ms,
            detail: f[4].to_string(),
        };
        r.validate()?;
        Ok(r)
    }
}

// ====================================================== THE CONTROL JOURNAL

pub const CONTROL_DIR: &str = "control";
pub const CONTROL_JOURNAL: &str = "control/rc021_control_journal.tsv";
pub const P2_JOURNAL: &str = "control/rc021_journal_p2.tsv";
pub const N3_JOURNAL: &str = "control/rc021_journal_n3.tsv";
pub const CONTROLS_STARTED: &str = "controls_started.json";
pub const CONTROLS_COMPLETE: &str = "controls_complete.json";

/// §C11.3: append-only, `create_new`, and **every row fsynced immediately on
/// completion**, so a failure leaves durable evidence before any closure exists.
pub struct ControlJournal {
    file: std::fs::File,
    dir: PathBuf,
    written: Vec<ControlRow>,
}

impl ControlJournal {
    pub fn create(path: &Path, meta: &Metadata) -> Result<ControlJournal, ControlError> {
        if meta.session != MetaSession::Control {
            return Err(ControlError::Domain(
                "the control journal's metadata session must be CONTROL",
            ));
        }
        let dir = path
            .parent()
            .ok_or(ControlError::Domain("journal path has no parent"))?
            .to_path_buf();
        let body =
            meta.render().map_err(|e| ControlError::Io(e.to_string()))? + &control_header_line();
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(ControlError::AlreadyPresent(path.display().to_string()))
            }
            Err(e) => return Err(ControlError::Io(e.to_string())),
        };
        file.write_all(body.as_bytes())
            .and_then(|()| file.flush())
            .and_then(|()| file.sync_all())
            .map_err(|e| ControlError::Io(e.to_string()))?;
        fsync_dir_at(&dir).map_err(|e| ControlError::Io(e.to_string()))?;
        Ok(ControlJournal {
            file,
            dir,
            written: Vec::new(),
        })
    }

    /// One row per completed control, durable before the next one starts.
    pub fn append(&mut self, row: &ControlRow) -> Result<(), ControlError> {
        if self.written.iter().any(|r| r.control_id == row.control_id) {
            return Err(ControlError::Domain("a control writes exactly one row"));
        }
        let expected = self.written.len() as u32 + 1;
        if row.ordinal != expected {
            return Err(ControlError::Field {
                what: "ordinal out of sequence",
                value: row.ordinal.to_string(),
            });
        }
        let line = row.encode()?;
        self.file
            .write_all(line.as_bytes())
            .and_then(|()| self.file.flush())
            .and_then(|()| self.file.sync_all())
            .map_err(|e| ControlError::Io(e.to_string()))?;
        self.written.push(row.clone());
        Ok(())
    }

    pub fn rows(&self) -> &[ControlRow] {
        &self.written
    }
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// What a valid central control journal contains.
#[derive(Clone, Debug)]
pub struct ControlJournalRead {
    pub metadata: Metadata,
    pub rows: Vec<ControlRow>,
}

/// §C9 metadata grammar: `#rc021_meta<TAB>key<TAB>JSON-value<LF>`.
const META_PREFIX: &str = "#rc021_meta\t";

/// Read a central control journal from its **bytes**.
///
/// Bytes, not `&str`: §C9 requires UTF-8, and taking a `&str` would mean some
/// caller had already decoded — lossily or not — outside this check.
///
/// The metadata block is validated by [`Metadata::parse`], the same validator
/// the 24-column journals use. Nothing here re-implements it.
pub fn read_control_journal(bytes: &[u8]) -> Result<ControlJournalRead, ControlError> {
    if bytes.is_empty() {
        return Err(ControlError::Domain("empty file: no metadata"));
    }
    if bytes.last() != Some(&b'\n') {
        return Err(ControlError::Domain("the file must end with a line feed"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ControlError::Domain("control journal is not valid UTF-8"))?;
    let mut lines = text.split('\n').collect::<Vec<&str>>();
    lines.pop(); // the empty tail after the final LF

    // §C9: exactly seventeen metadata lines, in the frozen key order, before
    // anything else.
    if lines.len() < META_KEYS.len() + 1 {
        return Err(ControlError::Domain("control journal is truncated"));
    }
    let mut pairs: Vec<(String, String)> = Vec::with_capacity(META_KEYS.len());
    for (i, key) in META_KEYS.iter().enumerate() {
        let rest = lines[i]
            .strip_prefix(META_PREFIX)
            .ok_or(ControlError::Domain("metadata line grammar"))?;
        let (k, v) = rest
            .split_once('\t')
            .ok_or(ControlError::Domain("metadata line grammar"))?;
        if k != *key {
            return Err(ControlError::Field {
                what: "metadata key out of order",
                value: k.to_string(),
            });
        }
        pairs.push((k.to_string(), v.to_string()));
    }
    let metadata = Metadata::parse(&pairs).map_err(|e| ControlError::Field {
        what: "metadata",
        value: e.to_string(),
    })?;
    // §C11.3: the central journal declares CONTROL and nothing else.
    if metadata.session != MetaSession::Control {
        return Err(ControlError::Domain(
            "the control journal's metadata session must be CONTROL",
        ));
    }

    // Then exactly the frozen five-column header.
    if lines[META_KEYS.len()] != control_header_line().trim_end_matches('\n') {
        return Err(ControlError::Domain("control journal header"));
    }

    let mut rows: Vec<ControlRow> = Vec::new();
    let mut failed = false;
    for line in &lines[META_KEYS.len() + 1..] {
        // Metadata after the header would let a second block redefine the run.
        if line.starts_with(META_PREFIX) {
            return Err(ControlError::Domain("metadata after the header"));
        }
        // §C11.2: execution stops at the first FAIL, so a row after one is
        // evidence of something that could not have run.
        if failed {
            return Err(ControlError::Domain("a row follows a FAIL row"));
        }
        let r = ControlRow::parse(line)?;
        if r.ordinal != rows.len() as u32 + 1 {
            return Err(ControlError::Domain("control rows are out of order"));
        }
        if r.control_id != CONTROL_ORDER[rows.len()] {
            return Err(ControlError::Field {
                what: "control_id out of the frozen order",
                value: r.control_id.as_str().to_string(),
            });
        }
        failed = r.status == ControlStatus::Fail;
        rows.push(r);
    }
    Ok(ControlJournalRead { metadata, rows })
}

/// Convenience for callers that already hold the rows.
pub fn parse_control_journal(bytes: &[u8]) -> Result<Vec<ControlRow>, ControlError> {
    read_control_journal(bytes).map(|r| r.rows)
}

// ================================================================= MARKERS

/// §C13.2, in this exact order.
pub const CONTROLS_STARTED_KEYS: [&str; 6] = [
    "schema_version",
    "run_uuid",
    "boot_id",
    "monotonic_offset_ms",
    "utc",
    "command_line",
];

/// §C13.3, in this exact order.
pub const CONTROLS_COMPLETE_KEYS: [&str; 8] = [
    "schema_version",
    "run_uuid",
    "boot_id",
    "monotonic_offset_ms",
    "utc",
    "control_journal_sha256",
    "control_count",
    "all_pass",
];

/// §C13.3: exactly twelve, and equal to the number of data rows in the central
/// control journal. Any disagreement is `INSTRUMENT-INVALID`.
pub const CONTROL_COUNT: u32 = 12;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ControlsStarted {
    pub schema_version: String,
    pub run_uuid: String,
    pub boot_id: String,
    pub monotonic_offset_ms: u64,
    pub utc: String,
    pub command_line: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ControlsComplete {
    pub schema_version: String,
    pub run_uuid: String,
    pub boot_id: String,
    /// §C13.3: **the origin of session 1's ten-minute gap**.
    pub monotonic_offset_ms: u64,
    pub utc: String,
    pub control_journal_sha256: String,
    pub control_count: u32,
    pub all_pass: bool,
}

fn jstr(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

fn common_marker_checks(
    schema_version: &str,
    run_uuid: &str,
    boot_id: &str,
    utc: &str,
    command_line: Option<&[String]>,
) -> Result<(), ControlError> {
    if schema_version != crate::journal::SCHEMA_VERSION {
        return Err(ControlError::Domain("schema_version"));
    }
    if !crate::host::is_run_uuid(run_uuid) {
        return Err(ControlError::Domain("run_uuid"));
    }
    if boot_id.is_empty() || boot_id.chars().any(|c| c.is_control()) {
        return Err(ControlError::Domain("boot_id"));
    }
    if chrono::DateTime::parse_from_rfc3339(utc).is_err() {
        return Err(ControlError::Domain("utc must be RFC 3339"));
    }
    if let Some(cl) = command_line {
        if cl.is_empty() {
            return Err(ControlError::Domain("command_line"));
        }
    }
    Ok(())
}

impl ControlsStarted {
    pub fn validate(&self) -> Result<(), ControlError> {
        common_marker_checks(
            &self.schema_version,
            &self.run_uuid,
            &self.boot_id,
            &self.utc,
            Some(&self.command_line),
        )
    }

    pub fn render(&self) -> String {
        format!(
            "{{{}:{},{}:{},{}:{},{}:{},{}:{},{}:{}}}\n",
            jstr("schema_version"),
            jstr(&self.schema_version),
            jstr("run_uuid"),
            jstr(&self.run_uuid),
            jstr("boot_id"),
            jstr(&self.boot_id),
            jstr("monotonic_offset_ms"),
            self.monotonic_offset_ms,
            jstr("utc"),
            jstr(&self.utc),
            jstr("command_line"),
            serde_json::to_string(&self.command_line).unwrap_or_else(|_| "[]".to_string()),
        )
    }

    pub fn parse(text: &str) -> Result<ControlsStarted, ControlError> {
        let o = marker_object(text, CONTROLS_STARTED_KEYS.len())?;
        let r = ControlsStarted {
            schema_version: mstr(&o, "schema_version")?,
            run_uuid: mstr(&o, "run_uuid")?,
            boot_id: mstr(&o, "boot_id")?,
            monotonic_offset_ms: mu64(&o, "monotonic_offset_ms")?,
            utc: mstr(&o, "utc")?,
            command_line: mvec(&o, "command_line")?,
        };
        r.validate()?;
        if r.render() != text {
            return Err(ControlError::Domain("controls_started is not canonical"));
        }
        Ok(r)
    }

    /// §C11.2 step 2, and §C10.1's durability sequence. Validation precedes
    /// `create_new`, so a malformed marker never reserves the path.
    pub fn write(&self, dir: &Path) -> Result<PathBuf, ControlError> {
        self.validate()?;
        write_durable(&dir.join(CONTROLS_STARTED), &self.render(), dir)
    }
}

impl ControlsComplete {
    pub fn validate(&self) -> Result<(), ControlError> {
        common_marker_checks(
            &self.schema_version,
            &self.run_uuid,
            &self.boot_id,
            &self.utc,
            None,
        )?;
        if self.control_journal_sha256.len() != 64
            || !self
                .control_journal_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ControlError::Domain("control_journal_sha256"));
        }
        // §C13.3: exactly twelve, and `all_pass` must be true. A marker that
        // could record eleven, or a failure, would assert completion of
        // something that did not complete.
        if self.control_count != CONTROL_COUNT {
            return Err(ControlError::Domain("control_count must be exactly 12"));
        }
        if !self.all_pass {
            return Err(ControlError::Domain("all_pass must be true"));
        }
        Ok(())
    }

    pub fn render(&self) -> String {
        format!(
            "{{{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{},{}:{}}}\n",
            jstr("schema_version"),
            jstr(&self.schema_version),
            jstr("run_uuid"),
            jstr(&self.run_uuid),
            jstr("boot_id"),
            jstr(&self.boot_id),
            jstr("monotonic_offset_ms"),
            self.monotonic_offset_ms,
            jstr("utc"),
            jstr(&self.utc),
            jstr("control_journal_sha256"),
            jstr(&self.control_journal_sha256),
            jstr("control_count"),
            self.control_count,
            jstr("all_pass"),
            self.all_pass,
        )
    }

    pub fn parse(text: &str) -> Result<ControlsComplete, ControlError> {
        let o = marker_object(text, CONTROLS_COMPLETE_KEYS.len())?;
        let r = ControlsComplete {
            schema_version: mstr(&o, "schema_version")?,
            run_uuid: mstr(&o, "run_uuid")?,
            boot_id: mstr(&o, "boot_id")?,
            monotonic_offset_ms: mu64(&o, "monotonic_offset_ms")?,
            utc: mstr(&o, "utc")?,
            control_journal_sha256: mstr(&o, "control_journal_sha256")?,
            control_count: u32::try_from(mu64(&o, "control_count")?)
                .map_err(|_| ControlError::Domain("control_count"))?,
            all_pass: o
                .get("all_pass")
                .and_then(|v| v.as_bool())
                .ok_or(ControlError::Domain("all_pass"))?,
        };
        r.validate()?;
        if r.render() != text {
            return Err(ControlError::Domain("controls_complete is not canonical"));
        }
        Ok(r)
    }

    pub fn write(&self, dir: &Path) -> Result<PathBuf, ControlError> {
        self.validate()?;
        write_durable(&dir.join(CONTROLS_COMPLETE), &self.render(), dir)
    }
}

type JsonMap = serde_json::Map<String, serde_json::Value>;

fn marker_object(text: &str, keys: usize) -> Result<JsonMap, ControlError> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|_| ControlError::Domain("not JSON"))?;
    let o = v
        .as_object()
        .ok_or(ControlError::Domain("not a JSON object"))?;
    if o.len() != keys {
        return Err(ControlError::Domain("key count"));
    }
    Ok(o.clone())
}

fn mstr(o: &JsonMap, k: &'static str) -> Result<String, ControlError> {
    o.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(ControlError::Domain(k))
}

fn mu64(o: &JsonMap, k: &'static str) -> Result<u64, ControlError> {
    o.get(k)
        .and_then(|v| v.as_u64())
        .ok_or(ControlError::Domain(k))
}

fn mvec(o: &JsonMap, k: &'static str) -> Result<Vec<String>, ControlError> {
    o.get(k)
        .and_then(|v| v.as_array())
        .ok_or(ControlError::Domain(k))?
        .iter()
        .map(|x| {
            x.as_str()
                .map(|y| y.to_string())
                .ok_or(ControlError::Domain(k))
        })
        .collect()
}

/// `create_new → write_all → flush → sync_all → fsync(parent)`.
fn write_durable(path: &Path, body: &str, dir: &Path) -> Result<PathBuf, ControlError> {
    let mut f = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(ControlError::AlreadyPresent(path.display().to_string()))
        }
        Err(e) => return Err(ControlError::Io(e.to_string())),
    };
    f.write_all(body.as_bytes())
        .and_then(|()| f.flush())
        .and_then(|()| f.sync_all())
        .map_err(|e| ControlError::Io(e.to_string()))?;
    fsync_dir_at(dir).map_err(|e| ControlError::Io(e.to_string()))?;
    Ok(path.to_path_buf())
}

// ============================================================ ORCHESTRATION

/// The clock, injected so a test never waits and never reads the live host.
pub trait Clock {
    fn monotonic_offset_ms(&mut self) -> u64;
    fn utc(&mut self) -> String;
}

/// Production clock anchored once to the boot-shared uptime axis. After the
/// checked anchor is established it advances with [`std::time::Instant`], so a
/// transient `/proc` read cannot fabricate a later row offset.
pub struct LiveClock {
    base_offset_ms: u64,
    started: std::time::Instant,
}

impl LiveClock {
    pub fn new(base_offset_ms: u64) -> LiveClock {
        LiveClock {
            base_offset_ms,
            started: std::time::Instant::now(),
        }
    }
}

impl Clock for LiveClock {
    fn monotonic_offset_ms(&mut self) -> u64 {
        self.base_offset_ms
            .saturating_add(self.started.elapsed().as_millis() as u64)
    }

    fn utc(&mut self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
}

/// One control's result. The runner decides; this module only orders, records
/// and classifies.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ControlOutcome {
    pub status: ControlStatus,
    /// A JSON string, quotes included.
    pub detail: String,
}

impl ControlOutcome {
    pub fn pass(detail: &str) -> ControlOutcome {
        ControlOutcome {
            status: ControlStatus::Pass,
            detail: json_string_encode(detail),
        }
    }
    pub fn fail(detail: &str) -> ControlOutcome {
        ControlOutcome {
            status: ControlStatus::Fail,
            detail: json_string_encode(detail),
        }
    }
}

/// Execution, injected. A test supplies one of these instead of running
/// sentinels, spawning children or touching the host.
pub trait ControlRunner {
    fn run(&mut self, id: ControlId) -> ControlOutcome;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ControlsOutcome {
    /// All twelve passed and `controls_complete.json` is durable. Exit 0.
    Complete { monotonic_offset_ms: u64 },
    /// §6: P6 failed. `REFUSED-BEFORE-MEASUREMENT`, exit 2, **not one byte
    /// created**, correction and retry permitted.
    RefusedBeforeMeasurement { detail: String },
    /// A control failed after the marker. Irreversible: `--controls` may not be
    /// repeated in this run directory, and the FAIL row stands as evidence.
    Failed {
        control: ControlId,
        class: FailureClass,
    },
    /// §C11.2: any failure after `controls_started` forbids repeating
    /// `--controls`. Nothing is executed and nothing is written.
    AlreadyStarted,
    /// §C12.4: a durable write failed. Execution stops immediately.
    WriteFailed { why: String },
}

impl ControlsOutcome {
    pub fn variant_name(&self) -> &'static str {
        match self {
            ControlsOutcome::Complete { .. } => "Complete",
            ControlsOutcome::RefusedBeforeMeasurement { .. } => "RefusedBeforeMeasurement",
            ControlsOutcome::Failed { .. } => "Failed",
            ControlsOutcome::AlreadyStarted => "AlreadyStarted",
            ControlsOutcome::WriteFailed { .. } => "WriteFailed",
        }
    }

    /// Whether this ends the run. `Complete` does not: the controls passed and
    /// the six sessions may now begin.
    pub fn is_terminal(&self) -> bool {
        self.terminal_run_status().is_some()
    }

    /// **The** classification. Exhaustive, so a new variant cannot be added
    /// without deciding whether it ends the run and with what §8 status.
    pub fn terminal_run_status(&self) -> Option<RunStatus> {
        match self {
            ControlsOutcome::Complete { .. } => None,
            ControlsOutcome::RefusedBeforeMeasurement { .. } => {
                Some(RunStatus::RefusedBeforeMeasurement)
            }
            ControlsOutcome::Failed { class, .. } => Some(class.run_status()),
            // The marker already asserts controls began; re-running would
            // execute controls whose evidence cannot be recorded.
            ControlsOutcome::AlreadyStarted => Some(RunStatus::InstrumentInvalid),
            ControlsOutcome::WriteFailed { .. } => Some(RunStatus::JournalInvalid),
        }
    }

    /// The variant plus whatever diagnostic it carries, for the operator.
    ///
    /// `RefusedBeforeMeasurement`'s own contract is "not one byte created", so
    /// if the reason is not printed it exists nowhere at all — the operator
    /// gets a bare exit 2 and no way to learn which provenance fact failed.
    pub fn describe(&self) -> String {
        let detail = match self {
            ControlsOutcome::RefusedBeforeMeasurement { detail } => Some(detail.clone()),
            ControlsOutcome::WriteFailed { why } => Some(why.clone()),
            ControlsOutcome::Failed { control, class } => {
                Some(format!("{} ({:?})", control.as_str(), class))
            }
            ControlsOutcome::Complete { .. } | ControlsOutcome::AlreadyStarted => None,
        };
        match detail {
            Some(d) => format!("{}: {d}", self.variant_name()),
            None => self.variant_name().to_string(),
        }
    }

    /// The exit code, **only** through §8. There is no second table.
    pub fn terminal_exit_code(&self) -> Option<i32> {
        self.terminal_run_status().map(RunStatus::exit_code)
    }
}

/// Everything the orchestration needs that it does not compute itself.
pub struct ControlsContext<'a> {
    /// The run's identity, held as **the validated manifest** rather than as
    /// loose strings: independent copies of `run_uuid` and `boot_id` can drift,
    /// and a self-consistent set of them describes some run, not necessarily
    /// this one.
    pub manifest: &'a RunManifest,
    pub command_line: Vec<String>,
    /// The control journal's metadata, whose `session` must be `CONTROL` and
    /// whose identity must be the manifest's.
    pub meta: Metadata,
}

/// The eight identity fields the journal metadata shares with the manifest.
/// §C9's metadata is self-describing, so a canonical block from another run
/// validates perfectly — only this comparison ties it to this run.
pub const METADATA_IDENTITY_FIELDS: [&str; 8] = [
    "run_uuid",
    "boot_id",
    "run_start_uptime_ms",
    "repo_commit",
    "prereg_commit",
    "amendment_commits",
    "instrument_birth_commit",
    "host_fingerprint",
];

/// Configuration, **not** identity. CPU set and thread count remain fixed;
/// timer resolution and diagnostic availability are deliberately absent.
/// §C6.1 requires those two observations anew for every process invocation and
/// explicitly says an availability change is not a failure.
///
/// `command_line` and `utc_start` are deliberately absent from both lists: they
/// describe *this* invocation, so copying them from the `--init-run` manifest
/// would record the wrong provenance.
pub const METADATA_CONFIG_FIELDS: [&str; 3] = ["cpu_set", "thread_count", "cpu_time_unit"];

impl ControlsContext<'_> {
    /// Checked **before** `controls_started.json` is created, so a mismatch
    /// leaves the directory byte-identical.
    pub fn bind(&self) -> Result<(), ControlError> {
        if self.meta.session != MetaSession::Control {
            return Err(ControlError::Domain(
                "the control journal's metadata session must be CONTROL",
            ));
        }
        let m = self.manifest;
        let j = &self.meta;
        let mismatch = |what: &'static str, value: String| ControlError::Field { what, value };
        if j.run_uuid != m.run_uuid {
            return Err(mismatch("run_uuid", j.run_uuid.clone()));
        }
        if j.boot_id != m.boot_id {
            return Err(mismatch("boot_id", j.boot_id.clone()));
        }
        if j.run_start_uptime_ms != m.run_start_uptime_ms {
            return Err(mismatch(
                "run_start_uptime_ms",
                j.run_start_uptime_ms.to_string(),
            ));
        }
        if j.repo_commit != m.repo_commit {
            return Err(mismatch("repo_commit", j.repo_commit.clone()));
        }
        if j.prereg_commit != m.prereg_commit {
            return Err(mismatch("prereg_commit", j.prereg_commit.clone()));
        }
        // §C10.1: both amendment commits, and the array as a whole. A journal
        // that named one amendment and not the other would bind to a different
        // reading of the protocol.
        if j.amendment_commits != m.amendment_commits {
            return Err(mismatch("amendment_commits", j.amendment_commits.join(",")));
        }
        if j.instrument_birth_commit != m.instrument_birth_commit {
            return Err(mismatch(
                "instrument_birth_commit",
                j.instrument_birth_commit.clone(),
            ));
        }
        if j.host_fingerprint != m.host_fingerprint {
            return Err(mismatch("host_fingerprint", j.host_fingerprint.clone()));
        }
        // Configuration, checked as well but not called identity.
        if j.cpu_set != m.cpu_set {
            return Err(mismatch("cpu_set", j.cpu_set.clone()));
        }
        if j.thread_count != m.thread_count {
            return Err(mismatch("thread_count", j.thread_count.to_string()));
        }
        if j.cpu_time_unit != m.cpu_time_unit {
            return Err(mismatch("cpu_time_unit", j.cpu_time_unit.clone()));
        }
        // The command line the marker records is the one this invocation ran.
        if self.command_line.is_empty() {
            return Err(ControlError::Domain("command_line"));
        }
        Ok(())
    }
}

/// §C10.2: a tri-state look at a reserved path. Only `NotFound` is absence; any
/// other error is a fact about the directory, and reading it as "nothing is
/// there" would let controls start on top of state they could not see.
pub fn path_state(p: &Path) -> Result<bool, ControlError> {
    match p.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(ControlError::Io(format!("{}: {e}", p.display()))),
    }
}

/// §C10.2 / §C12.3: evidence whose existence forbids controls outright. None of
/// it is ever repaired or overwritten.
pub const TERMINAL_EVIDENCE: [&str; 4] = [
    "run_invalid.json",
    "rc021_closure.json",
    CONTROLS_COMPLETE,
    "RC021_RESULTS.md",
];

/// §C11.2, executed in its frozen order.
///
/// The ordering is the contract: P6 is a pure preflight that creates nothing,
/// the marker is written strictly before the journal, P6's PASS row is written
/// retrospectively at ordinal 1, and every later control's row is durable before
/// the next control starts — so no control's outcome depends on a later one
/// surviving.
pub fn run_controls(
    dir: &Path,
    ctx: &ControlsContext<'_>,
    runner: &mut dyn ControlRunner,
    clock: &mut dyn Clock,
) -> ControlsOutcome {
    // §C11.2: a run whose controls already began may not repeat them, whatever
    // the outcome was. Tri-state, so an unreadable marker is not read as an
    // absent one.
    for rel in [CONTROLS_STARTED, CONTROL_JOURNAL] {
        match path_state(&dir.join(rel)) {
            Ok(true) => return ControlsOutcome::AlreadyStarted,
            Ok(false) => {}
            Err(e) => return ControlsOutcome::WriteFailed { why: e.to_string() },
        }
    }
    // Terminal or closing evidence forbids controls outright.
    for rel in TERMINAL_EVIDENCE {
        match path_state(&dir.join(rel)) {
            Ok(true) => return ControlsOutcome::AlreadyStarted,
            Ok(false) => {}
            Err(e) => return ControlsOutcome::WriteFailed { why: e.to_string() },
        }
    }
    // The journal metadata must be this run's, checked before any byte exists.
    if let Err(e) = ctx.bind() {
        return ControlsOutcome::RefusedBeforeMeasurement {
            detail: json_string_encode(&format!("controls context does not bind: {e}")),
        };
    }

    // 1. P6 — pure preflight. Creates nothing, so a failure is correctable.
    let p6 = runner.run(ControlId::P6);
    if p6.status == ControlStatus::Fail {
        return ControlsOutcome::RefusedBeforeMeasurement { detail: p6.detail };
    }

    // 2. the marker, strictly before the journal.
    let started = ControlsStarted {
        schema_version: crate::journal::SCHEMA_VERSION.to_string(),
        run_uuid: ctx.manifest.run_uuid.clone(),
        boot_id: ctx.manifest.boot_id.clone(),
        monotonic_offset_ms: clock.monotonic_offset_ms(),
        utc: clock.utc(),
        command_line: ctx.command_line.clone(),
    };
    if let Err(e) = started.write(dir) {
        return ControlsOutcome::WriteFailed { why: e.to_string() };
    }

    // 3. the journal. A crash between 2 and 3 leaves a marker with no journal,
    //    which §C11.2 classifies INSTRUMENT-INVALID at finalize.
    let control_dir = dir.join(CONTROL_DIR);
    if !control_dir.exists() {
        if let Err(e) = std::fs::create_dir(&control_dir) {
            return ControlsOutcome::WriteFailed { why: e.to_string() };
        }
        if let Err(e) = fsync_dir_at(dir) {
            return ControlsOutcome::WriteFailed { why: e.to_string() };
        }
    }
    let mut journal = match ControlJournal::create(&dir.join(CONTROL_JOURNAL), &ctx.meta) {
        Ok(j) => j,
        Err(e) => return ControlsOutcome::WriteFailed { why: e.to_string() },
    };

    // 4. P6's PASS row, retrospectively, at ordinal 1.
    let row = ControlRow {
        control_id: ControlId::P6,
        ordinal: 1,
        status: ControlStatus::Pass,
        monotonic_offset_ms: clock.monotonic_offset_ms(),
        detail: p6.detail,
    };
    if let Err(e) = journal.append(&row) {
        return ControlsOutcome::WriteFailed { why: e.to_string() };
    }

    // 5-7. the remaining eleven, each written and fsynced on completion.
    for id in CONTROL_ORDER.iter().copied().skip(1) {
        let out = runner.run(id);
        let row = ControlRow {
            control_id: id,
            ordinal: id.ordinal(),
            status: out.status,
            monotonic_offset_ms: clock.monotonic_offset_ms(),
            detail: out.detail,
        };
        if let Err(e) = journal.append(&row) {
            return ControlsOutcome::WriteFailed { why: e.to_string() };
        }
        if out.status == ControlStatus::Fail {
            // §C11.25: the control's own §6 class governs; only then the
            // generic post-marker rule.
            return ControlsOutcome::Failed {
                control: id,
                class: id.failure_class(),
            };
        }
    }

    // 8. `controls_complete.json`, only after twelve durable PASSes.
    //
    // Defence in depth, and honestly so: a FAIL returns above, so this branch
    // is not reachable through this function today. The reachable guarantee is
    // `ControlsComplete::validate`, which refuses `control_count != 12` and
    // `all_pass: false` before `create_new` — that is what the tests exercise.
    // This guard stands because the marker is durable and irreversible: a
    // future caller that assembled one by another route must not be able to
    // certify a run that did not complete.
    let rows = journal.rows();
    if rows.len() != CONTROL_COUNT as usize || rows.iter().any(|r| r.status != ControlStatus::Pass)
    {
        return ControlsOutcome::WriteFailed {
            why: "controls_complete requires twelve durable PASS rows".to_string(),
        };
    }
    let bytes = match std::fs::read(dir.join(CONTROL_JOURNAL)) {
        Ok(b) => b,
        Err(e) => return ControlsOutcome::WriteFailed { why: e.to_string() },
    };
    let monotonic_offset_ms = clock.monotonic_offset_ms();
    let complete = ControlsComplete {
        schema_version: crate::journal::SCHEMA_VERSION.to_string(),
        run_uuid: ctx.manifest.run_uuid.clone(),
        boot_id: ctx.manifest.boot_id.clone(),
        monotonic_offset_ms,
        utc: clock.utc(),
        control_journal_sha256: crate::host::sha256_hex(&bytes),
        control_count: CONTROL_COUNT,
        all_pass: true,
    };
    match complete.write(dir) {
        Ok(_) => ControlsOutcome::Complete {
            monotonic_offset_ms,
        },
        Err(e) => ControlsOutcome::WriteFailed { why: e.to_string() },
    }
}

// ======================================================================= P2

/// §C11.4: the hidden child's argument. Deliberately not a public CLI flag —
/// the public surface arrives in its own commit, and this branch exists solely
/// so the parent can spawn a process that dies after a durable journal write.
pub const P2_CHILD_ARG: &str = "--__rc021-p2-child";

/// §C11.4: the parent requires **abnormal termination** — a signal, never exit
/// code 0 — and then reads the unchanged bytes back.
#[cfg(unix)]
pub fn classify_p2_exit(status: &std::process::ExitStatus) -> Result<i32, ControlError> {
    use std::os::unix::process::ExitStatusExt;
    match status.signal() {
        Some(s) => Ok(s),
        None => Err(ControlError::Field {
            what: "P2 child did not terminate by signal",
            value: match status.code() {
                Some(c) => format!("exit code {c}"),
                None => "no code and no signal".to_string(),
            },
        }),
    }
}

/// §C11.4: the child's whole job.
///
/// It creates **only** the P2 journal, writes and fsyncs one `SESSION-OPEN`,
/// then writes and fsyncs one measurement row at the frozen control
/// coordinates, and then dies by `abort()`. The abort is the point of the
/// control: it proves durable evidence survives a process death, so it must
/// happen **after** the measurement row is on disk and it must never be a
/// clean exit.
///
/// It never returns.
pub fn p2_child_main(
    dir: &Path,
    meta: &Metadata,
    ctx: &crate::journal::RowContext,
    run_start_uptime_ms: u64,
    proc_loadavg: &Path,
) -> ! {
    // A failure anywhere before the durable measurement leaves the parent with
    // a journal that does not satisfy §C11.4 — which is exactly the P2 FAIL it
    // should see. The child still dies abnormally, so the parent's signal check
    // stays unambiguous and the two failure modes never blur.
    if let Err(e) = p2_write_evidence(dir, meta, ctx, run_start_uptime_ms, proc_loadavg) {
        eprintln!("rc021 p2 child: {e}");
    }
    // No SESSION-CLOSE row, and no clean exit: `abort` raises SIGABRT.
    std::process::abort();
}

/// The durable half of §C11.4's child, up to but not including the abort.
///
/// Split out so it can be exercised directly: `p2_child_main` never returns, so
/// no test could otherwise observe what it writes without spawning a process,
/// and a spawned process can only be checked against whatever `/proc` happens
/// to say. `proc_loadavg` is a parameter for the same reason
/// `run_start_uptime_ms` is — production passes the real path, and the hidden
/// child's argv carries only the run directory, so no caller can redirect it.
pub fn p2_write_evidence(
    dir: &Path,
    meta: &Metadata,
    ctx: &crate::journal::RowContext,
    run_start_uptime_ms: u64,
    proc_loadavg: &Path,
) -> Result<(), String> {
    let write = || -> Result<(), String> {
        if meta.session != MetaSession::P2 {
            return Err("the P2 journal's metadata session must be P2".to_string());
        }
        // §C11.4: the child creates **only** the P2 journal. `control/` is the
        // parent's; creating it here would hide the case where the run
        // directory was never initialised.
        match path_state(&dir.join(CONTROL_DIR)) {
            Ok(true) => {}
            Ok(false) => {
                return Err("control/ does not exist; the child creates only its journal".into())
            }
            Err(e) => return Err(e.to_string()),
        }

        // Real provenance, read now. A boot mismatch or a backwards uptime is
        // an error here, so the journal never gets its measurement row.
        let open_offset = live_monotonic_offset_ms(&meta.boot_id, run_start_uptime_ms)?;
        let load_avg_start = read_load_avg_1min(proc_loadavg)?;

        let mut j = crate::journal::Journal::create(&dir.join(P2_JOURNAL), meta)
            .map_err(|e| e.to_string())?;
        j.append(&ctx.session_open(0, open_offset, load_avg_start))
            .map_err(|e| e.to_string())?;

        // **The control measurement is declared started here.** Only past this
        // point is a physical `LOST` an honest record: it says a measurement
        // began and did not complete, and P2 is precisely the case where it
        // cannot complete because the process is about to die. Writing one
        // before the session was open would be inventing a measurement.
        let measure_offset = live_monotonic_offset_ms(&meta.boot_id, run_start_uptime_ms)?;
        // §C11.4's frozen coordinates: session 0, phase A, block 1, and
        // `measurement_index` NA — the global index belongs to qualification
        // sessions alone.
        j.append(&ctx.lost(
            0,
            crate::journal::Phase::A,
            1,
            None,
            measure_offset,
            "P2: measurement started, then deliberate abort before completion",
        ))
        .map_err(|e| e.to_string())?;
        Ok(())
    };
    write()
}

/// §C11.4: what the parent requires of the child's journal.
///
/// The bytes go through the strict 24-column reader, so §C9's metadata grammar,
/// every row's agreement with that metadata and the truncated-final-row
/// semantics are all the journal layer's, not re-implemented here.
pub fn classify_p2_journal(bytes: &[u8]) -> Result<(), ControlError> {
    use crate::journal::{Phase, ReadVerdict, Status};
    let read = crate::journal::parse_journal(bytes);
    if let ReadVerdict::JournalInvalid(why) = &read.verdict {
        return Err(ControlError::Field {
            what: "p2 journal",
            value: why.clone(),
        });
    }
    let meta = read.typed_metadata.as_ref().ok_or(ControlError::Domain(
        "a P2 journal needs valid §C9 metadata",
    ))?;
    if meta.session != MetaSession::P2 {
        return Err(ControlError::Domain(
            "the P2 journal's metadata session must be P2",
        ));
    }
    let mut measurements = 0usize;
    for (i, r) in read.rows.iter().enumerate() {
        match r.status {
            Status::SessionOpen => {
                if i != 0 {
                    return Err(ControlError::Domain(
                        "SESSION-OPEN must be the first data row",
                    ));
                }
            }
            Status::Ok | Status::Lost => {
                if i == 0 {
                    return Err(ControlError::Domain(
                        "a measurement row before SESSION-OPEN",
                    ));
                }
                if r.phase != Some(Phase::A) || r.block != Some(1) || r.measurement_index.is_some()
                {
                    return Err(ControlError::Domain(
                        "a P2 row must be phase A, block 1, measurement_index NA",
                    ));
                }
                if r.session != 0 {
                    return Err(ControlError::Domain("a P2 row must be session 0"));
                }
                measurements += 1;
            }
            Status::SessionCloseCompleted | Status::SessionCloseAborted => {
                return Err(ControlError::Domain(
                    "a P2 journal must contain no SESSION-CLOSE row",
                ))
            }
            Status::ExternalCause => {
                return Err(ControlError::Domain(
                    "a P2 journal must contain no EXTERNAL-CAUSE row",
                ))
            }
        }
    }
    if read.rows.first().map(|r| r.status) != Some(Status::SessionOpen) {
        return Err(ControlError::Domain(
            "a P2 journal has exactly one SESSION-OPEN",
        ));
    }
    if read
        .rows
        .iter()
        .filter(|r| r.status == Status::SessionOpen)
        .count()
        != 1
    {
        return Err(ControlError::Domain(
            "a P2 journal has exactly one SESSION-OPEN",
        ));
    }
    if measurements == 0 {
        return Err(ControlError::Domain(
            "a P2 journal needs at least one OK or LOST row",
        ));
    }
    Ok(())
}

/// §C11.4: the parent half of P2, spawning **this executable** as the hidden
/// child.
///
/// `P2 does not repeat within a run`: the journal path is claimed with
/// `create_new` by the child, so a second attempt cannot produce one, and there
/// is no attempt suffix to fall back on.
#[cfg(unix)]
pub fn run_p2(dir: &Path, exe: &Path) -> ControlOutcome {
    // §C11.4: `control/` is the parent's to make; the child creates only its
    // journal.
    if let Err(e) = std::fs::create_dir_all(dir.join(CONTROL_DIR)) {
        return ControlOutcome::fail(&format!("control/ could not be created: {e}"));
    }
    match path_state(&dir.join(P2_JOURNAL)) {
        Ok(true) => return ControlOutcome::fail("P2 does not repeat within a run"),
        Ok(false) => {}
        Err(e) => return ControlOutcome::fail(&format!("P2 journal state: {e}")),
    }
    let status = match std::process::Command::new(exe)
        .arg(P2_CHILD_ARG)
        .arg(dir)
        .status()
    {
        Ok(s) => s,
        Err(e) => return ControlOutcome::fail(&format!("P2 child could not be spawned: {e}")),
    };
    // Abnormal termination first: a clean exit means the child did not die
    // where the control requires it to.
    if let Err(e) = classify_p2_exit(&status) {
        return ControlOutcome::fail(&e.to_string());
    }
    // Then the bytes it left, read back unchanged.
    let bytes = match std::fs::read(dir.join(P2_JOURNAL)) {
        Ok(b) => b,
        Err(e) => return ControlOutcome::fail(&format!("P2 journal unreadable: {e}")),
    };
    match classify_p2_journal(&bytes) {
        // §5.2: a `STARTED` journal with no close row reads back `ABORTED`.
        Ok(()) => ControlOutcome::pass("P2 child aborted after a durable row; session ABORTED"),
        Err(e) => ControlOutcome::fail(&e.to_string()),
    }
}

// ====================================================== THE PRODUCTION RUNNER

/// The live work a control needs, injected so unit tests stay synthetic.
///
/// This is the seam: the *predicates* below are frozen and cannot be replaced,
/// but what they measure — sentinels, load blocks, git, the clock — comes
/// through here. A test supplies a fake environment; production supplies
/// [`LiveEnvironment`].
pub trait ControlEnvironment {
    /// §6 P6: the real provenance gate.
    fn provenance(&mut self) -> Result<(), String>;
    /// Durable provenance text for P6's central-journal row. The default keeps
    /// synthetic environments small; production overrides it with the §10
    /// build, corpus and local-time facts that are unavailable at finalize.
    fn provenance_detail(&mut self) -> Result<String, String> {
        Ok("provenance gate satisfied".to_string())
    }
    /// §4.1: put the host into **phase-B conditions** — the frozen 256-sweep
    /// warmup. There is no seed or sweep parameter: the only block this can run
    /// is [`WorkBlock::Warmup`].
    fn prepare_phase_b(&mut self) -> Result<(), String>;
    /// One sentinel execution, reported as **work plus time**.
    fn sentinel(
        &mut self,
        diagnostics: bool,
        extra: SentinelRequest,
    ) -> Result<SentinelSample, String>;
    /// One phase-C load block's wall time, in ms.
    fn load_block_ms(&mut self) -> Result<f64, String>;
    /// The measured timer resolution, in ms.
    fn timer_resolution_ms(&mut self) -> f64;
    /// §6 N3: execute one session's protocol into `rc021_journal_n3.tsv` and
    /// report what it wrote.
    fn run_n3(&mut self) -> Result<N3Report, String>;
    /// §C11.4: run the P2 parent/child pair.
    fn run_p2(&mut self) -> ControlOutcome;
}

/// How much work a sentinel execution is asked to do.
///
/// A closed pair, not a sweep count: only §6 P1 may add work, and it may only
/// add the amount P1 fixes. Nothing else can ask for a different sentinel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SentinelRequest {
    /// The frozen sentinel: prefix 4, window 8.
    Frozen,
    /// §6 P1's injection, three times the window.
    P1Injection,
}

impl SentinelRequest {
    pub fn extra_sweeps(self) -> u32 {
        match self {
            SentinelRequest::Frozen => 0,
            SentinelRequest::P1Injection => crate::protocol::SENTINEL_WINDOW * 3,
        }
    }
}

/// One sentinel execution: its wall time **and the work that produced it**.
///
/// §6 N1 is `INSTRUMENT-INVALID` if two executions are "not bit-identical in
/// work", so a bare `f64` cannot express the control — there would be nothing
/// to compare but the number the control is not allowed to judge.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SentinelSample {
    pub wall_ms: f64,
    pub measured_sweeps: u32,
    pub prefix_sweeps: u32,
    pub replicas: usize,
    pub seed: u64,
    pub diagnostics: bool,
}

impl SentinelSample {
    /// Everything except the wall time: the work itself.
    pub fn work(&self) -> (u32, u32, usize, u64, bool) {
        (
            self.measured_sweeps,
            self.prefix_sweeps,
            self.replicas,
            self.seed,
            self.diagnostics,
        )
    }
    /// §6 N1's precondition, spelled out field by field so a mismatch names
    /// what differed.
    pub fn same_work_as(&self, other: &SentinelSample) -> Result<(), String> {
        for (what, a, b) in [
            (
                "measured_sweeps",
                self.measured_sweeps as u64,
                other.measured_sweeps as u64,
            ),
            (
                "prefix_sweeps",
                self.prefix_sweeps as u64,
                other.prefix_sweeps as u64,
            ),
            ("replicas", self.replicas as u64, other.replicas as u64),
            ("seed", self.seed, other.seed),
            (
                "diagnostics",
                self.diagnostics as u64,
                other.diagnostics as u64,
            ),
        ] {
            if a != b {
                return Err(format!("{what} differs: {a} vs {b}"));
            }
        }
        Ok(())
    }
}

/// What N3 must report so its identity of **work** can be checked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct N3Report {
    pub sentinel_executions: u32,
    pub sweeps: u64,
    pub sentinel_seed: u64,
    pub load_seed: u64,
    pub rows: u32,
    pub metadata_is_n3: bool,
    pub coordinates_frozen: bool,
}

/// §9 criterion 10, frozen: 30 pairs with diagnostics on, 30 off, interleaved,
/// in phase-B conditions. 60 pairs is 120 sentinel executions.
pub const C10_PAIRS_PER_ARM: u32 = 30;
pub const C10_BOUND: f64 = 0.01;
/// §6 P7: the load block must exceed **1000 ×** the timer resolution.
pub const P7_RESOLUTION_MULT: f64 = 1000.0;

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    }
}

/// The production adapter: the frozen predicates, wired to the primitives that
/// already exist.
///
/// Its constructor takes **only normative run inputs** — the run directory, the
/// repository root, the verified instance and this executable's path. There is
/// no field of function type, so no caller can substitute what a control
/// measures, only where the run lives.
pub struct LiveEnvironment<'a> {
    run_dir: PathBuf,
    repo: PathBuf,
    exe: PathBuf,
    instance: &'a crate::protocol::VerifiedSentinelInstance,
    manifest: &'a RunManifest,
    timer_resolution_ms: f64,
    diag_availability: crate::host::DiagProbe,
    cpu_ids: Vec<u32>,
    /// §C6.1's sources, owned so a borrowed view can be handed out per call.
    diag_paths_owned: (PathBuf, PathBuf, PathBuf),
    /// **This** invocation's provenance, for the journals N3 writes.
    command_line: Vec<String>,
    utc_start: String,
}

fn format_live_provenance_detail(
    rustc: &str,
    profile: &str,
    flags: &str,
    sentinel_sha256: &str,
    local_timestamp: &str,
) -> String {
    format!(
        "rustc={rustc}; build_profile={profile}; build_flags={flags}; \
         sentinel_sha256={sentinel_sha256}; local_timestamp={local_timestamp}"
    )
}

/// The compiler version, or why it could not be read — never a refusal.
///
/// The recommended measuring host is a quiet machine, which need not carry a
/// build toolchain. A missing `rustc` makes a version *string* unrecordable; it
/// does not make the host unqualifiable. Propagating the error made P6 fail and
/// `run_controls` refuse the entire qualification at exit 2 because a string was
/// missing — contradicting the rule applied to build flags directly below: an
/// unobservable value is `UNAVAILABLE`, never absence.
pub fn rustc_version_fact(out: std::io::Result<std::process::Output>) -> String {
    match out {
        Ok(out) if out.status.success() => String::from_utf8(out.stdout)
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|_| "UNAVAILABLE(not UTF-8)".to_string()),
        Ok(out) => format!("UNAVAILABLE(exited {:?})", out.status.code()),
        Err(e) => format!("UNAVAILABLE({})", e.kind()),
    }
}

/// How this binary was actually compiled, from the build script.
///
/// Two earlier attempts were both false. `option_env!("RUSTFLAGS")` reads the
/// compile-time *environment*, but this workspace sets its flags in
/// `.cargo/config.toml`, which cargo passes to rustc as arguments — so the row
/// recorded `<none>` for a binary built with `-Ctarget-cpu=native`, the input
/// most able to move the timings this instrument certifies. `cfg!` sees only
/// the effective target features, not the flags. Cargo does hand
/// `CARGO_ENCODED_RUSTFLAGS` to a build script, and `build.rs` forwards it.
pub fn build_flags_fact() -> String {
    let declared = env!("RC021_CARGO_ENCODED_RUSTFLAGS").replace('\u{1f}', " ");
    let mut features: Vec<&str> = Vec::new();
    for (name, on) in [
        ("sse2", cfg!(target_feature = "sse2")),
        ("sse4.2", cfg!(target_feature = "sse4.2")),
        ("avx", cfg!(target_feature = "avx")),
        ("avx2", cfg!(target_feature = "avx2")),
        ("fma", cfg!(target_feature = "fma")),
        ("bmi2", cfg!(target_feature = "bmi2")),
        ("avx512f", cfg!(target_feature = "avx512f")),
    ] {
        if on {
            features.push(name);
        }
    }
    let features = if features.is_empty() {
        "none".to_string()
    } else {
        features.join(",")
    };
    let declared = if declared.is_empty() {
        "none".to_string()
    } else {
        declared
    };
    format!("{declared} | target_feature={features}")
}

/// The cargo profile and its optimisation settings, as the build script saw
/// them — not `cfg!(debug_assertions)`, which is **`false` in every profile in
/// this workspace** and therefore identifies nothing.
pub fn build_profile_fact() -> String {
    format!(
        "profile={} opt_level={} debug={}",
        env!("RC021_PROFILE"),
        env!("RC021_OPT_LEVEL"),
        env!("RC021_DEBUG")
    )
}

/// Whether this binary is optimised, from evidence rather than a proxy.
///
/// The original guard was `cfg!(debug_assertions)`. Measured: that macro is
/// `false` under `cargo test`, `cargo test --release` and every build in this
/// workspace, because `.cargo/config.toml`'s rustflags are in play — so the
/// guard could never fire and RC-021 had **no** protection against measuring
/// from an unoptimised binary. Two independent facts now decide it: the
/// profile's own `OPT_LEVEL`, and any `-Copt-level` in the rustflags that
/// override it. Unoptimised means both say zero.
/// Whether this binary is optimised.
///
/// **Deliberately not "and built under the release profile".** Adding
/// `RC021_PROFILE == "release"` would close a real hole — `.cargo/config.toml`
/// gives every profile `-Copt-level=3`, so a dev build passes this test while
/// still differing in codegen-units (256 against 16) and incremental
/// compilation, both of which move timings. It was tried and reverted: the
/// guard sits at the entry of both measuring modes, so under `cargo test`
/// (`PROFILE=debug`) it refuses every one of them and roughly ten tests can no
/// longer reach the branch they exist to check.
///
/// The fact is not lost. `build_profile_fact()` records `profile=` in P6's
/// durable provenance row, so a run made from a dev binary says so in its own
/// record and any auditor can see it. Closing the guard properly needs a
/// build-fitness seam on both modes, the way `SessionHost` was added for the
/// live world — recorded in `memory/OPEN_PROBLEMS.md`.
pub fn is_optimised_build() -> bool {
    is_optimised(
        env!("RC021_OPT_LEVEL"),
        &env!("RC021_CARGO_ENCODED_RUSTFLAGS").replace('\u{1f}', " "),
    )
}

/// The predicate itself, over its two inputs.
///
/// Separated so it can be falsified. On this workspace every build is optimised
/// and the broken proxy and the correct check therefore **agree**, so no test of
/// `is_optimised_build()` can tell them apart — they differ only on the
/// unoptimised build this workspace never produces. The logic, given inputs,
/// can be tested exhaustively.
///
/// A `-Copt-level` in the rustflags overrides the profile's own setting, which
/// is exactly how this workspace compiles a `dev` profile at level 3.
pub fn is_optimised(profile_opt_level: &str, flags: &str) -> bool {
    // Both spellings. `CARGO_ENCODED_RUSTFLAGS` separates *arguments* with
    // `\x1f`, so a config written `["-C", "opt-level=0"]`, or
    // `RUSTFLAGS="-C opt-level=0"`, arrives as `-C opt-level=0` — which the
    // joined form alone does not match. Missing it would fall through to the
    // profile and report a release build explicitly compiled at level 0 as
    // optimised: exactly the false negative this guard exists to prevent.
    if let Some(level) = last_opt_level(flags) {
        return !level.is_empty() && level != "0";
    }
    profile_opt_level != "0" && profile_opt_level != "UNAVAILABLE"
}

/// The last `-Copt-level=N` or `-C opt-level=N` in `flags`, if any. Later
/// occurrences win, as they do for rustc itself.
fn last_opt_level(flags: &str) -> Option<&str> {
    let mut found = None;
    let tokens: Vec<&str> = flags.split_whitespace().collect();
    for (i, tok) in tokens.iter().enumerate() {
        if let Some(level) = tok.strip_prefix("-Copt-level=") {
            found = Some(level);
        } else if *tok == "-C" {
            if let Some(level) = tokens.get(i + 1).and_then(|t| t.strip_prefix("opt-level=")) {
                found = Some(level);
            }
        } else if let Some(level) = tok
            .strip_prefix("-C")
            .and_then(|t| t.strip_prefix("opt-level="))
        {
            found = Some(level);
        }
    }
    found
}

impl<'a> LiveEnvironment<'a> {
    /// `instance` is a [`crate::protocol::VerifiedSentinelInstance`], so the
    /// adapter cannot be pointed at an unverified graph. Timer resolution and
    /// diagnostic availability are the once-per-invocation observations that
    /// also enter this invocation's journal metadata.
    ///
    /// Nine arguments, deliberately: each is a distinct durable fact this
    /// adapter is not allowed to derive for itself. Grouping them into a
    /// struct would only move the same list one line up.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_dir: &Path,
        repo: &Path,
        exe: &Path,
        instance: &'a crate::protocol::VerifiedSentinelInstance,
        manifest: &'a RunManifest,
        timer_resolution_ms: f64,
        diag_availability: crate::host::DiagProbe,
        command_line: Vec<String>,
        utc_start: String,
    ) -> Result<LiveEnvironment<'a>, ControlError> {
        let cpu_ids =
            crate::host::parse_cpu_list(&manifest.cpu_set).map_err(|e| ControlError::Field {
                what: "cpu_set",
                value: e.to_string(),
            })?;
        Ok(LiveEnvironment {
            run_dir: run_dir.to_path_buf(),
            repo: repo.to_path_buf(),
            exe: exe.to_path_buf(),
            instance,
            manifest,
            timer_resolution_ms,
            diag_availability,
            cpu_ids,
            diag_paths_owned: (
                PathBuf::from("/proc/self/stat"),
                PathBuf::from("/proc/self/status"),
                PathBuf::from("/sys/devices/system/cpu"),
            ),
            command_line,
            utc_start,
        })
    }

    fn diag_paths(&self) -> crate::host::DiagPaths<'_> {
        crate::host::DiagPaths {
            self_stat: &self.diag_paths_owned.0,
            self_status: &self.diag_paths_owned.1,
            sys_cpu_dir: &self.diag_paths_owned.2,
        }
    }
}

impl ControlEnvironment for LiveEnvironment<'_> {
    fn provenance(&mut self) -> Result<(), String> {
        crate::provenance::check(&self.repo)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn provenance_detail(&mut self) -> Result<String, String> {
        if !is_optimised_build() {
            return Err(format!(
                "RC-021 measuring modes require an optimised build ({})",
                build_profile_fact()
            ));
        }
        // The recommended measuring host is a quiet machine, which need not
        // carry a build toolchain. A missing `rustc` makes a version *string*
        // unrecordable; it does not make the host unqualifiable. Refusing the
        // whole run for it would also contradict the rule stated just above
        // for build flags — an unobservable value is `UNAVAILABLE`, never
        // absence and never a refusal.
        let rustc = rustc_version_fact(
            std::process::Command::new("rustc")
                .arg("--version")
                .output(),
        );
        Ok(format_live_provenance_detail(
            &rustc,
            &build_profile_fact(),
            &build_flags_fact(),
            // The **full** digest of the bytes the executed instance was
            // built from. A twelve-character prefix under a key named
            // `sentinel_sha256` disagrees with `sha256sum` for anyone auditing
            // the run; re-reading the file would certify whatever is on disk
            // now, which is not necessarily what the sentinel ran.
            self.instance.sha256(),
            &chrono::Local::now().to_rfc3339(),
        ))
    }

    /// §4.1: the frozen 256-sweep warmup, and nothing else — [`WorkBlock`] is a
    /// closed pair, so this cannot become "a warmup of my choosing".
    fn prepare_phase_b(&mut self) -> Result<(), String> {
        crate::protocol::run_work_block(self.instance, WorkBlock::Warmup)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn sentinel(
        &mut self,
        diagnostics: bool,
        extra: SentinelRequest,
    ) -> Result<SentinelSample, String> {
        use crate::protocol::{ExtraWork, RC021_SENTINEL_SEED};
        let work = match extra {
            SentinelRequest::Frozen => ExtraWork::NONE,
            SentinelRequest::P1Injection => ExtraWork::for_p1_injection(extra.extra_sweeps()),
        };
        // §C6.1: the diagnostics arm reads the channels this invocation probed;
        // the other arm reads nothing, which is what criterion 10 compares.
        let reader = crate::host::FsReader;
        let paths = self.diag_paths();
        let diag = self.diag_availability;
        let start = if diagnostics {
            Some(crate::host::window_start(&reader, &paths, diag))
        } else {
            None
        };
        let run = crate::protocol::run_sentinel(self.instance, work).map_err(|e| e.to_string())?;
        if let Some(start) = start {
            let _ = crate::host::window_end(&reader, &paths, diag, &self.cpu_ids, &start);
        }
        Ok(SentinelSample {
            wall_ms: run.wall_ms,
            measured_sweeps: run.measured_sweeps,
            prefix_sweeps: run.prefix_sweeps,
            replicas: run.replicas,
            seed: RC021_SENTINEL_SEED,
            diagnostics,
        })
    }

    fn load_block_ms(&mut self) -> Result<f64, String> {
        crate::protocol::run_work_block(self.instance, WorkBlock::Load)
            .map(|r| r.wall_ms)
            .map_err(|e| e.to_string())
    }

    fn timer_resolution_ms(&mut self) -> f64 {
        self.timer_resolution_ms
    }

    /// §6 N3 — one session's protocol, into `rc021_journal_n3.tsv`.
    ///
    /// The executor and its journal belong to this control: the qualification
    /// sessions reuse it in commit 8, which is why the report is computed from
    /// what was actually written rather than asserted.
    /// §6 N3 — one whole session's protocol, replayed on the diagnostic seeds
    /// into `rc021_journal_n3.tsv`, and then **read back**.
    fn run_n3(&mut self) -> Result<N3Report, String> {
        n3_replay(
            &self.run_dir.join(N3_JOURNAL),
            &self.journal_metadata(MetaSession::N3),
            &self.row_context(),
            self.instance,
            self.timer_resolution_ms,
            &mut |_| self.monotonic_offset_ms(),
            &mut || read_load_avg_1min(Path::new("/proc/loadavg")),
        )
    }

    fn run_p2(&mut self) -> ControlOutcome {
        run_p2(&self.run_dir, &self.exe)
    }
}

impl LiveEnvironment<'_> {
    #[cfg(test)]
    fn timer_resolution_ms_for_test(&self) -> f64 {
        self.timer_resolution_ms
    }

    /// Metadata for a journal **this** run writes. Identity and configuration
    /// come from the manifest; `command_line` and `utc_start` describe this
    /// invocation, so they are not copied from the `--init-run` manifest.
    pub fn journal_metadata(&self, session: MetaSession) -> Metadata {
        let m = self.manifest;
        Metadata {
            run_uuid: m.run_uuid.clone(),
            boot_id: m.boot_id.clone(),
            run_start_uptime_ms: m.run_start_uptime_ms,
            repo_commit: m.repo_commit.clone(),
            prereg_commit: m.prereg_commit.clone(),
            amendment_commits: m.amendment_commits.clone(),
            instrument_birth_commit: m.instrument_birth_commit.clone(),
            host_fingerprint: m.host_fingerprint.clone(),
            cpu_set: m.cpu_set.clone(),
            thread_count: m.thread_count,
            timer_resolution_ms: self.timer_resolution_ms,
            cpu_time_unit: m.cpu_time_unit.clone(),
            command_line: self.command_line.clone(),
            utc_start: self.utc_start.clone(),
            session,
            diag_availability: crate::journal::DiagAvailability {
                cpu_time: self.diag_availability.cpu_time,
                ctx_switches: self.diag_availability.ctx_switches,
                freq: self.diag_availability.freq,
            },
        }
    }

    pub fn row_context(&self) -> crate::journal::RowContext {
        let m = self.manifest;
        crate::journal::RowContext {
            run_uuid: m.run_uuid.clone(),
            repo_commit: m.repo_commit.clone(),
            prereg_commit: m.prereg_commit.clone(),
            host_fingerprint: m.host_fingerprint.clone(),
            timer_resolution_ms: self.timer_resolution_ms,
            cpu_set: m.cpu_set.clone(),
            thread_count: m.thread_count,
        }
    }

    /// §C12: the checked offset on the boot-shared uptime axis.
    pub fn monotonic_offset_ms(&self) -> Result<u64, String> {
        live_monotonic_offset_ms(&self.manifest.boot_id, self.manifest.run_start_uptime_ms)
    }
}

impl crate::session::SessionEnvironment for LiveEnvironment<'_> {
    fn run_work_block(&mut self, block: WorkBlock) -> Result<(), String> {
        crate::protocol::run_work_block(self.instance, block)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn measure_pair(&mut self) -> Result<crate::session::PairObservation, String> {
        let reader = crate::host::FsReader;
        let paths = self.diag_paths();
        let availability = self.diag_availability;
        let load_avg_start = read_load_avg_1min(Path::new("/proc/loadavg"))?;
        let start = crate::host::window_start(&reader, &paths, availability);
        let first = crate::protocol::run_sentinel(self.instance, crate::protocol::ExtraWork::NONE)
            .map_err(|e| e.to_string())?;
        let last = crate::protocol::run_sentinel(self.instance, crate::protocol::ExtraWork::NONE)
            .map_err(|e| e.to_string())?;
        let diagnostics =
            crate::host::window_end(&reader, &paths, availability, &self.cpu_ids, &start);
        let load_avg_end = read_load_avg_1min(Path::new("/proc/loadavg"))?;
        Ok(crate::session::PairObservation {
            sentinel_first_ms: first.wall_ms,
            sentinel_last_ms: last.wall_ms,
            load_avg_start,
            load_avg_end,
            diagnostics,
        })
    }

    fn monotonic_offset_ms(&mut self) -> Result<u64, String> {
        LiveEnvironment::monotonic_offset_ms(self)
    }

    fn load_avg(&mut self) -> Result<f64, String> {
        read_load_avg_1min(Path::new("/proc/loadavg"))
    }
}

/// §6 N3 — replay **one session's protocol** into its own journal.
///
/// §5.2 recognises a session by its lifecycle rows, so the replay writes them:
/// `SESSION-OPEN` first, the fifteen measurements, then
/// `SESSION-CLOSE-COMPLETED`. A journal of measurements with no `SESSION-OPEN`
/// matches none of §5.2's four states — the file exists, so it is not
/// `NOT STARTED`, and there is no open row, so it is not `STARTED` either.
///
/// Each measurement is a real paired measurement: two sentinel executions,
/// classified by [`crate::protocol::classify_measurement`]. §5.1 reserves
/// `LOST` for a measurement that did not complete, so a completed pair is
/// written `OK` carrying both wall times and the spread — which is also what
/// §6 N3's "wall times will differ and that is expected" is about.
///
/// `offset_ms` is a closure so a test can drive the replay without a live
/// `/proc`; production passes the checked monotonic offset.
#[allow(clippy::type_complexity)]
pub fn n3_replay(
    path: &Path,
    meta: &Metadata,
    ctx: &crate::journal::RowContext,
    instance: &crate::protocol::VerifiedSentinelInstance,
    timer_resolution_ms: f64,
    offset_ms: &mut dyn FnMut(u32) -> Result<u64, String>,
    load_avg: &mut dyn FnMut() -> Result<f64, String>,
) -> Result<N3Report, String> {
    use crate::journal::{Phase, Status};
    use crate::protocol::{run_sentinel, run_work_block, ExtraWork, Measurement};

    if meta.session != MetaSession::N3 {
        return Err("the N3 journal's metadata session must be N3".to_string());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut j = crate::journal::Journal::create(path, meta).map_err(|e| e.to_string())?;

    // §5.2: the lifecycle opens before the first measurement.
    let mut step = 0u32;
    j.append(&ctx.session_open(0, offset_ms(step)?, load_avg()?))
        .map_err(|e| e.to_string())?;

    let mut sentinel_executions = 0u32;
    let mut sweeps = 0u64;
    for c in crate::session::plan_session(1).map_err(|e| e.to_string())? {
        if let Some(block) =
            crate::session::work_before(c.phase, c.block).map_err(|e| e.to_string())?
        {
            let r = run_work_block(instance, block).map_err(|e| e.to_string())?;
            sweeps += r.sweeps as u64;
        }
        let first = run_sentinel(instance, ExtraWork::NONE).map_err(|e| e.to_string())?;
        let last = run_sentinel(instance, ExtraWork::NONE).map_err(|e| e.to_string())?;
        sentinel_executions += 2;
        sweeps += (first.measured_sweeps + first.prefix_sweeps) as u64;
        sweeps += (last.measured_sweeps + last.prefix_sweeps) as u64;

        let load_start = load_avg()?;
        step += 1;
        let offset = offset_ms(step)?;
        // §C11.4's frozen control coordinates: session 0, `measurement_index`
        // NA — N3's fifteen never enter the denominator of 90.
        let row = match classify_pair(first.wall_ms, last.wall_ms, timer_resolution_ms) {
            Measurement::Ok { spread } => {
                let mut r = ctx.lost(0, c.phase, c.block, None, offset, "");
                r.status = Status::Ok;
                r.sentinel_first_ms = Some(first.wall_ms);
                r.sentinel_last_ms = Some(last.wall_ms);
                r.paired_spread = Some(spread);
                // §C8.3: an OK row carries both load averages.
                r.load_avg_start = Some(load_start);
                r.load_avg_end = Some(load_avg()?);
                r
            }
            // §5.1: only a measurement that did not complete is LOST.
            Measurement::Lost(why) => {
                ctx.lost(0, c.phase, c.block, None, offset, &format!("{why:?}"))
            }
        };
        j.append(&row).map_err(|e| e.to_string())?;
    }

    step += 1;
    j.append(&ctx.session_close(0, offset_ms(step)?, true, load_avg()?))
        .map_err(|e| e.to_string())?;

    // Everything below is derived from the durable bytes, never from the loop.
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let read = crate::journal::parse_journal(&bytes);
    if let crate::journal::ReadVerdict::JournalInvalid(why) = &read.verdict {
        return Err(format!("the N3 journal did not read back: {why}"));
    }
    let metadata_is_n3 = read
        .typed_metadata
        .as_ref()
        .map(|m| m.session == MetaSession::N3)
        .unwrap_or(false);

    // §5.2: SESSION-OPEN first, SESSION-CLOSE-COMPLETED last, nothing else.
    if read.rows.first().map(|r| r.status) != Some(Status::SessionOpen) {
        return Err("the N3 journal does not open with SESSION-OPEN".to_string());
    }
    if read.rows.last().map(|r| r.status) != Some(Status::SessionCloseCompleted) {
        return Err("the N3 journal does not end with SESSION-CLOSE-COMPLETED".to_string());
    }
    let measurements: Vec<&crate::journal::Row> = read
        .rows
        .iter()
        .filter(|r| matches!(r.status, Status::Ok | Status::Lost))
        .collect();
    let expected = crate::session::MEASUREMENTS_PER_SESSION;
    if measurements.len() as u32 != expected {
        return Err(format!(
            "the N3 journal carries {} measurement rows, one session is {expected}",
            measurements.len()
        ));
    }
    let coordinates_frozen = measurements.iter().all(|r| {
        r.session == 0
            && r.measurement_index.is_none()
            && r.block.is_some()
            && matches!(r.phase, Some(Phase::A) | Some(Phase::B) | Some(Phase::C))
    });
    // §6 N3: an OK row carries its wall times, or the replay recorded no times.
    for r in &measurements {
        if r.status == Status::Ok
            && (r.sentinel_first_ms.is_none()
                || r.sentinel_last_ms.is_none()
                || r.paired_spread.is_none())
        {
            return Err("an OK row is missing its wall times".to_string());
        }
    }
    Ok(N3Report {
        sentinel_executions,
        sweeps,
        sentinel_seed: crate::protocol::RC021_SENTINEL_SEED,
        load_seed: crate::protocol::RC021_LOAD_SEED,
        rows: measurements.len() as u32,
        metadata_is_n3,
        coordinates_frozen,
    })
}

/// §6 P8's per-measurement decision, named here so the replay reads as what it
/// is: a real classification, not a chosen status.
fn classify_pair(
    first_ms: f64,
    last_ms: f64,
    timer_resolution_ms: f64,
) -> crate::protocol::Measurement {
    crate::protocol::classify_measurement(first_ms, last_ms, timer_resolution_ms)
}

/// §C12, live: read the current `boot_id` and uptime and compute the checked
/// offset. A changed boot or a backwards uptime is an error, never a wrap.
pub fn live_monotonic_offset_ms(
    expected_boot_id: &str,
    run_start_uptime_ms: u64,
) -> Result<u64, String> {
    let boot = crate::host::parse_boot_id(
        &std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let now = crate::host::read_uptime_ms(Path::new("/proc/uptime")).map_err(|e| e.to_string())?;
    crate::host::monotonic_offset_ms(expected_boot_id, &boot, run_start_uptime_ms, now)
        .map_err(|e| e.to_string())
}

/// `/proc/loadavg`'s one-minute average — the value §C8 records as
/// `load_avg_start`.
pub fn read_load_avg_1min(proc_loadavg: &Path) -> Result<f64, String> {
    let text = std::fs::read_to_string(proc_loadavg).map_err(|e| e.to_string())?;
    let first = text
        .split_whitespace()
        .next()
        .ok_or_else(|| "loadavg is empty".to_string())?;
    let v: f64 = first
        .parse()
        .map_err(|_| format!("loadavg is not a number: {first}"))?;
    if !v.is_finite() || v < 0.0 {
        return Err(format!("loadavg out of domain: {v}"));
    }
    Ok(v)
}

/// The production dispatcher.
///
/// Every control routes to its own frozen predicate. There is no constructor
/// that accepts a closure, so a caller cannot substitute a control's rule — the
/// only thing it can supply is the environment those rules measure.
pub struct ProductionRunner<'a> {
    env: &'a mut dyn ControlEnvironment,
}

impl<'a> ProductionRunner<'a> {
    pub fn new(env: &'a mut dyn ControlEnvironment) -> ProductionRunner<'a> {
        ProductionRunner { env }
    }

    /// §6 P6 — provenance gate. `REFUSED-BEFORE-MEASUREMENT` on failure, which
    /// is why it runs before any byte exists.
    fn p6(&mut self) -> ControlOutcome {
        if let Err(e) = self.env.provenance() {
            return ControlOutcome::fail(&e);
        }
        match self.env.provenance_detail() {
            Ok(detail) => ControlOutcome::pass(&detail),
            Err(e) => ControlOutcome::fail(&e),
        }
    }

    /// §6 P5 — every family of §3.1 is refused by value; RC-021's four seeds
    /// belong to none and are pairwise distinct.
    fn p5(&mut self) -> ControlOutcome {
        use crate::protocol::{
            check_seed_disjointness, occupied_by, validate_seed_set, DERIVED_FAMILIES, OCCUPANCY,
            RC021_BAND, RC021_SEEDS,
        };
        if let Err(e) = check_seed_disjointness() {
            return ControlOutcome::fail(&e.to_string());
        }
        // "each is refused by value": the gate must actually reject a member of
        // every family, not merely accept RC-021's own seeds.
        let wide = (0u64, u64::MAX);
        for f in DERIVED_FAMILIES {
            for probe in [f.lo, f.hi] {
                if occupied_by(probe).is_none() {
                    return ControlOutcome::fail(&format!(
                        "{} ({}) value {probe} escapes the occupancy gate",
                        f.name, f.source
                    ));
                }
                if validate_seed_set(&[("probe", probe)], wide, &OCCUPANCY).is_ok() {
                    return ControlOutcome::fail(&format!("seed {probe} was not refused"));
                }
            }
        }
        for (_, s) in RC021_SEEDS {
            if s < RC021_BAND.0 || s > RC021_BAND.1 {
                return ControlOutcome::fail(&format!("seed {s} left the reserved band"));
            }
        }
        ControlOutcome::pass("all families refused; four seeds distinct and in band")
    }

    /// §6 P4 — the frozen header carries no outcome-shaped column.
    fn p4(&mut self) -> ControlOutcome {
        const FORBIDDEN: [&str; 6] = ["energy", "cut", "best", "quality", "delta", "objective"];
        for col in crate::journal::COLUMNS {
            let lower = col.to_ascii_lowercase();
            for bad in FORBIDDEN {
                if lower.contains(bad) {
                    return ControlOutcome::fail(&format!("column {col} contains {bad}"));
                }
            }
        }
        for col in CONTROL_COLUMNS {
            let lower = col.to_ascii_lowercase();
            for bad in FORBIDDEN {
                if lower.contains(bad) {
                    return ControlOutcome::fail(&format!("control column {col} contains {bad}"));
                }
            }
        }
        ControlOutcome::pass("no energy, cut, best, quality, delta or objective column")
    }

    /// §6 P3 — **every terminal program path exercised**: each is built as the
    /// value the program returns and put through the production conversions
    /// Step 7 and Step 8 will call. There is no second table to agree with.
    fn p3(&mut self) -> ControlOutcome {
        let outcomes = representative_terminal_outcomes();

        // Every **terminal** variant has a representative, and every
        // non-terminal one is absent on purpose. Both directions are checked,
        // so a new variant can neither go unclassified nor be smuggled in as a
        // §8 path. More than one representative per variant is deliberate:
        // `Failed` carries two, because §C11.25 gives its classes different
        // statuses.
        let mut covered: Vec<String> = outcomes.iter().map(|o| o.variant_name()).collect();
        covered.sort();
        covered.dedup();
        let terminal_controls = terminal_controls_variants();
        let terminal_sessions = terminal_session_variants();
        let terminal_finalize = terminal_finalize_variants();
        let terminal_verify = terminal_verify_variants();
        for want in terminal_controls
            .iter()
            .map(|v| format!("Controls::{v}"))
            .chain(terminal_sessions.iter().map(|v| format!("Session::{v}")))
            .chain(VERDICT_VARIANTS.iter().map(|v| format!("Verdict::{v}")))
            .chain(terminal_finalize.iter().map(|v| format!("Finalize::{v}")))
            .chain(terminal_verify.iter().map(|v| format!("Verify::{v}")))
        {
            if !covered.contains(&want) {
                return ControlOutcome::fail(&format!("{want} has no representative"));
            }
        }
        for v in CONTROLS_VARIANTS {
            if !terminal_controls.contains(&v) && covered.contains(&format!("Controls::{v}")) {
                return ControlOutcome::fail(&format!(
                    "Controls::{v} is a continuation and must not be a §8 terminal path"
                ));
            }
        }
        for v in SESSION_VARIANTS {
            if !terminal_sessions.contains(&v) && covered.contains(&format!("Session::{v}")) {
                return ControlOutcome::fail(&format!(
                    "Session::{v} is a continuation and must not be a §8 terminal path"
                ));
            }
        }
        for v in crate::decision::VERIFY_VARIANTS {
            if !terminal_verify.contains(&v) && covered.contains(&format!("Verify::{v}")) {
                return ControlOutcome::fail(&format!(
                    "Verify::{v} is a continuation and must not be a §8 terminal path"
                ));
            }
        }
        for v in crate::decision::FINALIZE_VARIANTS {
            if !terminal_finalize.contains(&v) && covered.contains(&format!("Finalize::{v}")) {
                return ControlOutcome::fail(&format!(
                    "Finalize::{v} is a continuation and must not be a §8 terminal path"
                ));
            }
        }
        // The samples are the only source of the two lists above, so P3 is
        // vacuous unless they are the **complete** frozen inventory. A variant
        // added to either enum and forgotten here would otherwise vanish from
        // the taxonomy without failing anything.
        for (family, sampled, frozen) in [
            (
                "Finalize",
                sample_finalize_outcomes()
                    .iter()
                    .map(|o| o.variant_name())
                    .collect::<Vec<_>>(),
                crate::decision::FINALIZE_VARIANTS.to_vec(),
            ),
            (
                "Verify",
                sample_verify_outcomes()
                    .iter()
                    .map(|o| o.variant_name())
                    .collect::<Vec<_>>(),
                crate::decision::VERIFY_VARIANTS.to_vec(),
            ),
        ] {
            if !sample_is_frozen_inventory(&sampled, &frozen) {
                return ControlOutcome::fail(&format!(
                    "the {family} sample is not the frozen inventory: {sampled:?} vs {frozen:?}"
                ));
            }
        }
        // Non-vacuous: there must be terminal paths in every family.
        if terminal_controls.is_empty()
            || terminal_sessions.is_empty()
            || terminal_finalize.is_empty()
            || terminal_verify.is_empty()
            || outcomes.is_empty()
        {
            return ControlOutcome::fail("the terminal enumeration is empty");
        }

        // Each representative maps to exactly one §8 status, and that status is
        // in the closed domain.
        for o in &outcomes {
            let s = o.run_status();
            if !RUN_STATUSES.contains(&s) {
                return ControlOutcome::fail(&format!(
                    "{} maps outside the §8 set",
                    o.variant_name()
                ));
            }
        }

        // Every failing status must be *reachable*: a taxonomy in which some
        // status can never be produced is one that gets waived.
        for want in [
            RunStatus::RefusedBeforeMeasurement,
            RunStatus::InstrumentInvalid,
            RunStatus::JournalInvalid,
            RunStatus::HostNotQualified,
            RunStatus::HostQualified,
            RunStatus::InconclusiveUnderpowered,
        ] {
            if !outcomes.iter().any(|o| o.run_status() == want) {
                return ControlOutcome::fail(&format!("{} is unreachable", want.as_str()));
            }
        }

        // …and the exit codes are the frozen ones, all six distinct.
        let mut codes: Vec<i32> = RUN_STATUSES.iter().map(|s| s.exit_code()).collect();
        codes.sort_unstable();
        codes.dedup();
        if codes.len() != RUN_STATUSES.len() {
            return ControlOutcome::fail("two §8 statuses share an exit code");
        }
        // One authority: every terminal outcome's own code is §8's code.
        for o in &outcomes {
            let via_status = Some(o.run_status().exit_code());
            let direct = match o {
                TerminalOutcome::Controls(c) => c.terminal_exit_code(),
                TerminalOutcome::Session(c) => c.terminal_exit_code(),
                TerminalOutcome::Verdict(v) => Some(v.run_status().exit_code()),
                TerminalOutcome::Finalize(o) => o.terminal_exit_code(),
                TerminalOutcome::Verify(o) => o.terminal_exit_code(),
            };
            if direct != via_status {
                return ControlOutcome::fail(&format!(
                    "{} has two exit codes: {direct:?} and {via_status:?}",
                    o.variant_name()
                ));
            }
        }

        ControlOutcome::pass(&format!(
            "{} terminal paths exercised through the production conversions, \
             each mapping to exactly one of {} statuses",
            outcomes.len(),
            RUN_STATUSES.len()
        ))
    }

    /// §6 P8 — the per-measurement resolution floor is real: a duration at the
    /// floor passes, one below it is `LOST`.
    fn p8(&mut self) -> ControlOutcome {
        use crate::protocol::{classify_measurement, LostReason, Measurement, RESOLUTION_MULT};
        let res = self.env.timer_resolution_ms();
        if !res.is_finite() || res <= 0.0 {
            return ControlOutcome::fail(&format!("unusable timer resolution {res}"));
        }
        let floor = RESOLUTION_MULT * res;
        if !matches!(
            classify_measurement(floor, floor, res),
            Measurement::Ok { .. }
        ) {
            return ControlOutcome::fail("a duration exactly at the floor must pass");
        }
        let under = floor * (1.0 - f64::EPSILON);
        if !matches!(
            classify_measurement(under, floor, res),
            Measurement::Lost(LostReason::BelowResolutionFloor)
        ) {
            return ControlOutcome::fail("a duration below the floor must be LOST");
        }
        // …and a real sentinel must clear it, or the instrument is resolving
        // noise rather than work.
        let ms = match self.env.sentinel(true, SentinelRequest::Frozen) {
            Ok(v) => v.wall_ms,
            Err(e) => return ControlOutcome::fail(&e),
        };
        // NaN must fail, and `ms < floor` is false for NaN — so the
        // finiteness check carries that case explicitly.
        if !ms.is_finite() || ms < floor {
            return ControlOutcome::fail(&format!(
                "sentinel {ms} ms is below the {floor} ms floor"
            ));
        }
        ControlOutcome::pass(&format!("floor {floor} ms, sentinel {ms} ms"))
    }

    /// §6 N2 — the reference condition. **No pass threshold**: a failure here
    /// is data about the host, routed to §7, not a control failure.
    fn n2(&mut self) -> ControlOutcome {
        use crate::protocol::paired_spread;
        // §4.1: phase A. No warmup — "cold" is the absence of prior work, so
        // preparing phase-B conditions here would erase the condition N2 names.
        let a = match self.env.sentinel(true, SentinelRequest::Frozen) {
            Ok(v) => v.wall_ms,
            Err(e) => return ControlOutcome::fail(&e),
        };
        let b = match self.env.sentinel(true, SentinelRequest::Frozen) {
            Ok(v) => v.wall_ms,
            Err(e) => return ControlOutcome::fail(&e),
        };
        match paired_spread(a, b) {
            Some(sp) => ControlOutcome::pass(&format!("back-to-back spread {sp} recorded")),
            // Only an unusable pair is a control failure: the recorded value
            // itself has no threshold to fail.
            None => ControlOutcome::fail(&format!("unusable durations {a}, {b}")),
        }
    }

    /// §6 N1 — one sentinel workload, two executions, **phase B**.
    ///
    /// §6's failure column is precise and easy to get backwards: N1 is
    /// `INSTRUMENT-INVALID` *if the two executions are not bit-identical in
    /// work*; **otherwise the observation is data, not a control failure**. So
    /// a spread above the bound is recorded and routed to §7 — turning it into
    /// a control failure here would convert a host observation into a claim
    /// that the instrument is broken.
    fn n1(&mut self) -> ControlOutcome {
        use crate::protocol::paired_spread;
        // §4.1 phase-B conditions: the frozen warmup, once, before the pair.
        if let Err(e) = self.env.prepare_phase_b() {
            return ControlOutcome::fail(&format!("phase-B preparation failed: {e}"));
        }
        let a = match self.env.sentinel(true, SentinelRequest::Frozen) {
            Ok(v) => v,
            Err(e) => return ControlOutcome::fail(&e),
        };
        let b = match self.env.sentinel(true, SentinelRequest::Frozen) {
            Ok(v) => v,
            Err(e) => return ControlOutcome::fail(&e),
        };
        // The one thing N1 may fail on.
        if let Err(why) = a.same_work_as(&b) {
            return ControlOutcome::fail(&format!(
                "the two executions are not bit-identical in work: {why}"
            ));
        }
        match paired_spread(a.wall_ms, b.wall_ms) {
            Some(sp) => ControlOutcome::pass(&format!(
                "identical work; spread {sp} recorded as an observation \
                 (bound {SPREAD_BOUND}, {})",
                if sp <= SPREAD_BOUND {
                    "within"
                } else {
                    "above — data for §7, not a control failure"
                }
            )),
            None => {
                ControlOutcome::fail(&format!("unusable durations {}, {}", a.wall_ms, b.wall_ms))
            }
        }
    }

    /// §6 N3 — one session's protocol replayed on the diagnostic seeds, into
    /// its own journal, with **identical work**.
    fn n3(&mut self) -> ControlOutcome {
        use crate::protocol::{RC021_LOAD_SEED, RC021_SENTINEL_SEED};
        use crate::session::{MEASUREMENTS_PER_SESSION, TOTAL_MEASUREMENTS};
        let r = match self.env.run_n3() {
            Ok(r) => r,
            Err(e) => return ControlOutcome::fail(&e),
        };
        if !r.metadata_is_n3 {
            return ControlOutcome::fail("the N3 journal's metadata session must be N3");
        }
        if !r.coordinates_frozen {
            return ControlOutcome::fail(
                "N3 rows must carry the frozen control coordinates with measurement_index NA",
            );
        }
        if r.sentinel_seed != RC021_SENTINEL_SEED || r.load_seed != RC021_LOAD_SEED {
            return ControlOutcome::fail("N3 must replay the same seeds");
        }
        if r.rows != MEASUREMENTS_PER_SESSION {
            return ControlOutcome::fail(&format!(
                "N3 wrote {} rows, one session is {MEASUREMENTS_PER_SESSION}",
                r.rows
            ));
        }
        // §4.2: two sentinel executions per pair.
        if r.sentinel_executions != MEASUREMENTS_PER_SESSION * 2 {
            return ControlOutcome::fail(&format!(
                "N3 ran {} sentinel executions, one session is {}",
                r.sentinel_executions,
                MEASUREMENTS_PER_SESSION * 2
            ));
        }
        if r.sweeps != expected_session_sweeps() {
            return ControlOutcome::fail(&format!(
                "N3 ran {} sweeps, one session is {}",
                r.sweeps,
                expected_session_sweeps()
            ));
        }
        // These 15 never enter the denominator of 90.
        ControlOutcome::pass(&format!(
            "one session replayed; {} rows, {TOTAL_MEASUREMENTS} unaffected",
            r.rows
        ))
    }

    /// §6 P1 — an injected slowdown must be detected: the spread it produces
    /// must exceed the bound **by construction**.
    fn p1(&mut self) -> ControlOutcome {
        use crate::protocol::paired_spread;
        let plain = match self.env.sentinel(true, SentinelRequest::Frozen) {
            Ok(v) => v.wall_ms,
            Err(e) => return ControlOutcome::fail(&e),
        };
        // Enough extra work that the spread cannot be within the bound: three
        // times the window is a 300% slowdown against a 9% bound.
        let slowed = match self.env.sentinel(true, SentinelRequest::P1Injection) {
            Ok(v) => v.wall_ms,
            Err(e) => return ControlOutcome::fail(&e),
        };
        match paired_spread(plain, slowed) {
            Some(sp) if sp > SPREAD_BOUND => {
                ControlOutcome::pass(&format!("injected spread {sp} exceeds the bound"))
            }
            Some(sp) => ControlOutcome::fail(&format!(
                "injected spread {sp} did not exceed the bound: the guard cannot fail"
            )),
            None => ControlOutcome::fail(&format!("unusable durations {plain}, {slowed}")),
        }
    }

    /// §6 P7 — the phase-C load block must exceed `1000 ×` the resolution.
    fn p7(&mut self) -> ControlOutcome {
        let res = self.env.timer_resolution_ms();
        if !res.is_finite() || res <= 0.0 {
            return ControlOutcome::fail(&format!("unusable timer resolution {res}"));
        }
        let ms = match self.env.load_block_ms() {
            Ok(v) => v,
            Err(e) => return ControlOutcome::fail(&e),
        };
        let floor = P7_RESOLUTION_MULT * res;
        if ms > floor {
            ControlOutcome::pass(&format!("load block {ms} ms exceeds {floor} ms"))
        } else {
            ControlOutcome::fail(&format!("load block {ms} ms does not exceed {floor} ms"))
        }
    }

    /// §9 criterion 10 — 30 pairs with diagnostics on and 30 off, interleaved,
    /// median of each arm, and a **1% bound on the sentinel's own duration**.
    fn c10(&mut self) -> ControlOutcome {
        // §9 criterion 10: "interleaved, **in phase-B conditions**". The frozen
        // warmup runs once, before the fixed procedure begins.
        if let Err(e) = self.env.prepare_phase_b() {
            return ControlOutcome::fail(&format!("phase-B preparation failed: {e}"));
        }
        let mut on = Vec::with_capacity((C10_PAIRS_PER_ARM * 2) as usize);
        let mut off = Vec::with_capacity((C10_PAIRS_PER_ARM * 2) as usize);
        for _ in 0..C10_PAIRS_PER_ARM {
            // Interleaved: one pair from each arm in turn, so drift affects
            // both arms alike.
            for (arm, diag) in [(&mut on, true), (&mut off, false)] {
                for _ in 0..2 {
                    match self.env.sentinel(diag, SentinelRequest::Frozen) {
                        Ok(v) => arm.push(v.wall_ms),
                        Err(e) => return ControlOutcome::fail(&e),
                    }
                }
            }
        }
        let executions = on.len() + off.len();
        let m_on = median(&mut on);
        let m_off = median(&mut off);
        if !m_on.is_finite() || !m_off.is_finite() || m_off <= 0.0 {
            return ControlOutcome::fail(&format!("unusable medians {m_on}, {m_off}"));
        }
        let rel = (m_on - m_off).abs() / m_off;
        if rel > C10_BOUND {
            ControlOutcome::fail(&format!(
                "diagnostic overhead {rel} exceeds {C10_BOUND}; diagnostics are removed, not tolerated"
            ))
        } else {
            ControlOutcome::pass(&format!(
                "{executions} executions, median on {m_on} ms, off {m_off} ms, overhead {rel}"
            ))
        }
    }
}

/// The inherited bound, §1. Not re-chosen here.
pub const SPREAD_BOUND: f64 = 0.09;

/// §4.1 / §4.2: one session's total sweeps — 15 pairs of two sentinels, the
/// phase-B warmup, and five phase-C load blocks.
pub fn expected_session_sweeps() -> u64 {
    use crate::protocol::{WorkBlock, SENTINEL_PREFIX_SWEEPS, SENTINEL_WINDOW};
    use crate::session::{BLOCKS_PER_PHASE, MEASUREMENTS_PER_SESSION};
    let per_sentinel = (SENTINEL_PREFIX_SWEEPS + SENTINEL_WINDOW) as u64;
    let measured = per_sentinel * (MEASUREMENTS_PER_SESSION as u64) * 2;
    let warmup = WorkBlock::Warmup.sweeps() as u64;
    let load = WorkBlock::Load.sweeps() as u64 * BLOCKS_PER_PHASE as u64;
    measured + warmup + load
}

impl ControlRunner for ProductionRunner<'_> {
    /// Each identifier routes to its own frozen predicate. The `match` is
    /// exhaustive, so a new control cannot be added without one.
    fn run(&mut self, id: ControlId) -> ControlOutcome {
        match id {
            ControlId::P6 => self.p6(),
            ControlId::P5 => self.p5(),
            ControlId::P4 => self.p4(),
            ControlId::P3 => self.p3(),
            ControlId::P2 => self.env.run_p2(),
            ControlId::P8 => self.p8(),
            ControlId::N2 => self.n2(),
            ControlId::N1 => self.n1(),
            ControlId::N3 => self.n3(),
            ControlId::P1 => self.p1(),
            ControlId::P7 => self.p7(),
            ControlId::C10 => self.c10(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::DiagAvailability;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(name: &str) -> TempDir {
            let p = std::env::temp_dir().join(format!(
                "rc021_controls_{}_{}_{}",
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

    /// Byte-exact, recursive: "nothing was written" must mean the contents, not
    /// just the names.
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

    struct FakeClock(u64);
    impl Clock for FakeClock {
        fn monotonic_offset_ms(&mut self) -> u64 {
            self.0 += 1_000;
            self.0
        }
        fn utc(&mut self) -> String {
            "2026-08-25T00:00:00Z".to_string()
        }
    }

    /// Records what it was asked to run, and fails whatever it is told to.
    struct FakeRunner {
        fail: Option<ControlId>,
        seen: Vec<ControlId>,
    }
    impl FakeRunner {
        fn all_pass() -> FakeRunner {
            FakeRunner {
                fail: None,
                seen: Vec::new(),
            }
        }
        fn failing(at: ControlId) -> FakeRunner {
            FakeRunner {
                fail: Some(at),
                seen: Vec::new(),
            }
        }
    }
    impl ControlRunner for FakeRunner {
        fn run(&mut self, id: ControlId) -> ControlOutcome {
            self.seen.push(id);
            if self.fail == Some(id) {
                ControlOutcome::fail(&format!("{} injected failure", id.as_str()))
            } else {
                ControlOutcome::pass(&format!("{} ok", id.as_str()))
            }
        }
    }

    fn fixture_host_fields() -> crate::host::HostFields {
        crate::host::HostFields {
            kernel_release: "6.6.0".into(),
            kernel_version: "#1 SMP".into(),
            available_processors: "4".into(),
            mem_total_kb: "10185860".into(),
            cpus_allowed_list: "0-3".into(),
            cpu_model: "Test CPU".into(),
        }
    }

    fn meta() -> Metadata {
        Metadata {
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            run_start_uptime_ms: 1_000,
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            amendment_commits: vec!["c".repeat(40), "d".repeat(40)],
            instrument_birth_commit: "e".repeat(40),
            host_fingerprint: fixture_host_fields().fingerprint(),
            cpu_set: "0-3".into(),
            thread_count: 1,
            timer_resolution_ms: 0.00002,
            cpu_time_unit: crate::journal::CPU_TIME_UNIT.into(),
            command_line: vec!["exp_rc021_host_qualify".into()],
            utc_start: "2026-08-25T00:00:00Z".into(),
            session: MetaSession::Control,
            diag_availability: DiagAvailability {
                cpu_time: true,
                ctx_switches: true,
                freq: false,
            },
        }
    }

    /// A manifest whose eight identity fields are the ones `meta()` declares,
    /// so the context binds.
    fn fixture_manifest(dir: &Path) -> RunManifest {
        let hf = fixture_host_fields();
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
            cpu_set: "0-3".into(),
            thread_count: 1,
            timer_resolution_ms: 0.00002,
            cpu_time_unit: crate::journal::CPU_TIME_UNIT.into(),
            command_line: vec!["exp_rc021_host_qualify".into()],
            utc_start: "2026-08-25T00:00:00Z".into(),
            diag_availability: crate::host::DiagProbe {
                cpu_time: true,
                ctx_switches: true,
                freq: false,
            },
        }
    }

    fn ctx(m: &RunManifest) -> ControlsContext<'_> {
        ControlsContext {
            manifest: m,
            command_line: vec!["exp_rc021_host_qualify".into(), "--controls".into()],
            meta: meta(),
        }
    }

    // ------------------------------------------------------------- the order

    #[test]
    fn the_twelve_controls_are_in_the_frozen_order_with_ordinals_one_to_twelve() {
        let names: Vec<&str> = CONTROL_ORDER.iter().map(|c| c.as_str()).collect();
        assert_eq!(
            names,
            ["P6", "P5", "P4", "P3", "P2", "P8", "N2", "N1", "N3", "P1", "P7", "C10"]
        );
        assert_eq!(CONTROL_ORDER.len(), CONTROL_COUNT as usize);
        for (i, id) in CONTROL_ORDER.iter().enumerate() {
            assert_eq!(id.ordinal(), i as u32 + 1, "{}", id.as_str());
            assert_eq!(ControlId::parse(id.as_str()), Some(*id));
        }
        // §C11.5: N3 at 9, then P1 and P7, and criterion 10 last.
        assert_eq!(ControlId::N3.ordinal(), 9);
        assert_eq!(ControlId::P1.ordinal(), 10);
        assert_eq!(ControlId::P7.ordinal(), 11);
        assert_eq!(ControlId::C10.ordinal(), 12);
        assert!(ControlId::parse("P9").is_none());
    }

    #[test]
    fn production_p6_detail_carries_the_provenance_unavailable_at_finalize() {
        let detail = format_live_provenance_detail(
            "rustc 1.89.0",
            "release",
            "-C target-cpu=native",
            &"a".repeat(64),
            "2026-08-27T12:00:00+05:00",
        );
        for required in [
            "rustc=rustc 1.89.0",
            "build_profile=release",
            "build_flags=-C target-cpu=native",
            &format!("sentinel_sha256={}", "a".repeat(64)),
            "local_timestamp=2026-08-27T12:00:00+05:00",
        ] {
            assert!(detail.contains(required), "missing {required:?}");
        }
    }

    /// The formatter test above passes a full digest in by hand, so it cannot
    /// catch what the row actually records. This one asks the live adapter.
    ///
    /// The key is named `sentinel_sha256`; it held `G11_SHA256_PREFIX`, twelve
    /// hex characters, which is what the *gate* compares — not a digest. The
    /// row is fsynced into the control journal and republished in
    /// `RC021_RESULTS.md`, so an auditor running `sha256sum` finds it disagrees.
    #[test]
    fn the_live_provenance_row_records_the_executed_instance_digest() {
        let d = TempDir::new("p6_digest");
        let m = fixture_manifest(d.path());
        let inst = crate::protocol::tests_support::synthetic_verified();
        let mut live = LiveEnvironment::new(
            d.path(),
            d.path(),
            std::path::Path::new("/nonexistent/exe"),
            &inst,
            &m,
            m.timer_resolution_ms,
            m.diag_availability,
            vec!["exp_rc021_host_qualify".to_string()],
            "2026-08-28T00:00:00Z".to_string(),
        )
        .expect("the fixture cpu_set parses");
        let detail = live
            .provenance_detail()
            .expect("an optimised build records its provenance");
        let field = detail
            .split("sentinel_sha256=")
            .nth(1)
            .and_then(|r| r.split(';').next())
            .expect("the key must be present")
            .trim();
        assert_eq!(field.len(), 64, "a sha256, not a prefix: {field:?}");
        assert_eq!(field, inst.sha256(), "and the digest of what actually ran");
        assert_ne!(field, crate::protocol::G11_SHA256_PREFIX);
    }

    #[test]
    fn a_full_pass_writes_the_marker_then_the_journal_then_the_completion() {
        let d = TempDir::new("happy");
        let mut r = FakeRunner::all_pass();
        let out = run_controls(
            d.path(),
            &ctx(&fixture_manifest(d.path())),
            &mut r,
            &mut FakeClock(0),
        );
        assert!(matches!(out, ControlsOutcome::Complete { .. }), "{out:?}");
        assert_eq!(out.terminal_exit_code(), None, "Complete is a continuation");
        assert!(!out.is_terminal());
        assert_eq!(r.seen, CONTROL_ORDER.to_vec(), "executed in frozen order");

        // The retrospective P6 PASS row is first, at ordinal 1.
        let text = std::fs::read_to_string(d.path().join(CONTROL_JOURNAL)).unwrap();
        let rows = parse_control_journal(text.as_bytes()).unwrap();
        assert_eq!(rows.len(), 12);
        assert_eq!(rows[0].control_id, ControlId::P6);
        assert_eq!(rows[0].ordinal, 1);
        assert_eq!(rows[0].status, ControlStatus::Pass);
        for (i, id) in CONTROL_ORDER.iter().enumerate() {
            assert_eq!(rows[i].control_id, *id);
            assert_eq!(rows[i].ordinal, i as u32 + 1);
        }
        // The metadata block declares CONTROL.
        assert!(text.contains("#rc021_meta\tsession\t\"CONTROL\""));

        // `controls_complete` binds to the journal it certifies.
        let c = ControlsComplete::parse(
            &std::fs::read_to_string(d.path().join(CONTROLS_COMPLETE)).unwrap(),
        )
        .unwrap();
        assert_eq!(c.control_count, 12);
        assert!(c.all_pass);
        assert_eq!(
            c.control_journal_sha256,
            crate::host::sha256_hex(text.as_bytes())
        );
        // §C13.3: this offset is the origin of session 1's ten-minute gap.
        match out {
            ControlsOutcome::Complete {
                monotonic_offset_ms,
            } => assert_eq!(monotonic_offset_ms, c.monotonic_offset_ms),
            other => panic!("{other:?}"),
        }
    }

    /// §6: P6 is a pure preflight. Its failure creates nothing and is
    /// correctable.
    #[test]
    fn a_p6_failure_leaves_the_directory_byte_identical() {
        let d = TempDir::new("p6fail");
        let before = snapshot(d.path());
        let mut r = FakeRunner::failing(ControlId::P6);
        let out = run_controls(
            d.path(),
            &ctx(&fixture_manifest(d.path())),
            &mut r,
            &mut FakeClock(0),
        );
        assert!(
            matches!(out, ControlsOutcome::RefusedBeforeMeasurement { .. }),
            "{out:?}"
        );
        assert_eq!(out.terminal_exit_code(), Some(2));
        assert_eq!(r.seen, vec![ControlId::P6], "nothing after P6 ran");
        assert_eq!(snapshot(d.path()), before, "not one byte created");
        assert!(!d.path().join(CONTROLS_STARTED).exists());
        assert!(!d.path().join(CONTROL_JOURNAL).exists());

        // …and because nothing was claimed, a corrected retry still works.
        let mut r = FakeRunner::all_pass();
        assert!(matches!(
            run_controls(
                d.path(),
                &ctx(&fixture_manifest(d.path())),
                &mut r,
                &mut FakeClock(0)
            ),
            ControlsOutcome::Complete { .. }
        ));
    }

    /// Every post-marker failure: the row is durable, the class is the frozen
    /// one, no completion marker appears, and no later control runs.
    #[test]
    fn a_post_marker_failure_is_durable_and_carries_its_own_class() {
        for id in CONTROL_ORDER.iter().copied().skip(1) {
            let d = TempDir::new("postfail");
            let mut r = FakeRunner::failing(id);
            let out = run_controls(
                d.path(),
                &ctx(&fixture_manifest(d.path())),
                &mut r,
                &mut FakeClock(0),
            );
            assert_eq!(
                out,
                ControlsOutcome::Failed {
                    control: id,
                    class: id.failure_class()
                },
                "{}",
                id.as_str()
            );
            // §C11.25: P2's own class wins over the generic exit 3.
            let want = if id == ControlId::P2 { 4 } else { 3 };
            assert_eq!(out.terminal_exit_code(), Some(want), "{}", id.as_str());

            // The FAIL row is durable evidence, and nothing later ran.
            let text = std::fs::read_to_string(d.path().join(CONTROL_JOURNAL)).unwrap();
            let rows = parse_control_journal(text.as_bytes()).unwrap();
            assert_eq!(rows.len(), id.ordinal() as usize);
            assert_eq!(rows.last().unwrap().control_id, id);
            assert_eq!(rows.last().unwrap().status, ControlStatus::Fail);
            assert_eq!(*r.seen.last().unwrap(), id);
            assert!(
                !d.path().join(CONTROLS_COMPLETE).exists(),
                "{}: completion must not exist",
                id.as_str()
            );
            // §C11.2: the marker exists, and the failure forbids a repeat.
            assert!(d.path().join(CONTROLS_STARTED).exists());
            let before = snapshot(d.path());
            let mut r2 = FakeRunner::all_pass();
            assert_eq!(
                run_controls(
                    d.path(),
                    &ctx(&fixture_manifest(d.path())),
                    &mut r2,
                    &mut FakeClock(0)
                ),
                ControlsOutcome::AlreadyStarted
            );
            assert!(r2.seen.is_empty(), "a repeat executes nothing");
            assert_eq!(snapshot(d.path()), before, "a repeat writes nothing");
        }
    }

    /// A completed run may not be repeated either.
    #[test]
    fn a_second_run_after_the_marker_executes_and_writes_nothing() {
        let d = TempDir::new("repeat");
        let mut r = FakeRunner::all_pass();
        run_controls(
            d.path(),
            &ctx(&fixture_manifest(d.path())),
            &mut r,
            &mut FakeClock(0),
        );
        let before = snapshot(d.path());
        let mut r2 = FakeRunner::all_pass();
        let out = run_controls(
            d.path(),
            &ctx(&fixture_manifest(d.path())),
            &mut r2,
            &mut FakeClock(0),
        );
        assert_eq!(out, ControlsOutcome::AlreadyStarted);
        assert_eq!(out.terminal_exit_code(), Some(3));
        assert!(r2.seen.is_empty());
        assert_eq!(snapshot(d.path()), before);
    }

    // ------------------------------------------------ durability boundaries

    /// §C11.2's crash table, checked on the states it names.
    #[test]
    fn each_durability_boundary_yields_its_prescribed_class() {
        // marker exists, journal absent — the marker asserts controls began and
        // the missing journal proves they left no evidence.
        let d = TempDir::new("marker_only");
        started_for(d.path()).write(d.path()).unwrap();
        assert!(d.path().join(CONTROLS_STARTED).exists());
        assert!(!d.path().join(CONTROL_JOURNAL).exists());
        let mut r = FakeRunner::all_pass();
        assert_eq!(
            run_controls(
                d.path(),
                &ctx(&fixture_manifest(d.path())),
                &mut r,
                &mut FakeClock(0)
            ),
            ControlsOutcome::AlreadyStarted
        );
        assert!(r.seen.is_empty());

        // journal exists without the marker — equally a repeat, and equally
        // refused: the journal cannot be created before the preflight.
        let d = TempDir::new("journal_only");
        std::fs::create_dir(d.path().join(CONTROL_DIR)).unwrap();
        ControlJournal::create(&d.path().join(CONTROL_JOURNAL), &meta()).unwrap();
        let before = snapshot(d.path());
        let mut r = FakeRunner::all_pass();
        assert_eq!(
            run_controls(
                d.path(),
                &ctx(&fixture_manifest(d.path())),
                &mut r,
                &mut FakeClock(0)
            ),
            ControlsOutcome::AlreadyStarted
        );
        assert!(r.seen.is_empty());
        assert_eq!(snapshot(d.path()), before);

        // a journal may not open at an ordinal other than 1: the first row is
        // always the retrospective P6 PASS.
        let d = TempDir::new("noP6");
        std::fs::create_dir(d.path().join(CONTROL_DIR)).unwrap();
        let mut j = ControlJournal::create(&d.path().join(CONTROL_JOURNAL), &meta()).unwrap();
        assert!(j
            .append(&ControlRow {
                control_id: ControlId::P5,
                ordinal: 2,
                status: ControlStatus::Pass,
                monotonic_offset_ms: 1,
                detail: json_string_encode("x"),
            })
            .is_err());
        // and a P6 row that claims any other ordinal is refused outright.
        assert!(j
            .append(&ControlRow {
                control_id: ControlId::P6,
                ordinal: 3,
                status: ControlStatus::Pass,
                monotonic_offset_ms: 1,
                detail: json_string_encode("x"),
            })
            .is_err());
    }

    /// §C13.3: completion is impossible without twelve durable PASSes.
    #[test]
    fn completion_is_impossible_at_eleven_or_with_any_failure() {
        let base = ControlsComplete {
            schema_version: crate::journal::SCHEMA_VERSION.to_string(),
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            monotonic_offset_ms: 5_000,
            utc: "2026-08-25T00:00:00Z".into(),
            control_journal_sha256: "a".repeat(64),
            control_count: 12,
            all_pass: true,
        };
        assert_eq!(base.validate(), Ok(()));
        for (what, mutate) in [
            (
                "control_count must be exactly 12",
                Box::new(|c: &mut ControlsComplete| c.control_count = 11)
                    as Box<dyn Fn(&mut ControlsComplete)>,
            ),
            (
                "control_count must be exactly 12",
                Box::new(|c: &mut ControlsComplete| c.control_count = 13),
            ),
            (
                "all_pass must be true",
                Box::new(|c: &mut ControlsComplete| c.all_pass = false),
            ),
        ] {
            let mut c = base.clone();
            mutate(&mut c);
            assert_eq!(c.validate(), Err(ControlError::Domain(what)));
            let d = TempDir::new("nocomplete");
            assert_eq!(c.write(d.path()), Err(ControlError::Domain(what)));
            assert!(
                !d.path().join(CONTROLS_COMPLETE).exists(),
                "a refused writer must not reserve the path"
            );
            // …and a well-formed record can still be written afterwards.
            assert!(base.write(d.path()).is_ok());
        }
    }

    // -------------------------------------------------------- marker schemas

    fn started_for(_d: &Path) -> ControlsStarted {
        ControlsStarted {
            schema_version: crate::journal::SCHEMA_VERSION.to_string(),
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            monotonic_offset_ms: 4_000,
            utc: "2026-08-25T00:00:00Z".into(),
            command_line: vec!["exp_rc021_host_qualify".into(), "--controls".into()],
        }
    }

    #[test]
    fn the_markers_carry_their_fields_in_the_frozen_order() {
        let s = started_for(Path::new("."));
        let text = s.render();
        assert!(text.ends_with("}\n"));
        assert_eq!(text.matches('\n').count(), 1);
        let mut at = 0usize;
        for k in CONTROLS_STARTED_KEYS {
            let n = format!("\"{k}\":");
            at += text[at..]
                .find(&n)
                .unwrap_or_else(|| panic!("{k} missing or out of order"))
                + n.len();
        }
        assert_eq!(CONTROLS_STARTED_KEYS.len(), 6);
        assert_eq!(ControlsStarted::parse(&text).unwrap(), s);

        let c = ControlsComplete {
            schema_version: crate::journal::SCHEMA_VERSION.to_string(),
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            monotonic_offset_ms: 5_000,
            utc: "2026-08-25T00:00:00Z".into(),
            control_journal_sha256: "a".repeat(64),
            control_count: 12,
            all_pass: true,
        };
        let text = c.render();
        let mut at = 0usize;
        for k in CONTROLS_COMPLETE_KEYS {
            let n = format!("\"{k}\":");
            at += text[at..]
                .find(&n)
                .unwrap_or_else(|| panic!("{k} missing or out of order"))
                + n.len();
        }
        assert_eq!(CONTROLS_COMPLETE_KEYS.len(), 8);
        assert_eq!(ControlsComplete::parse(&text).unwrap(), c);
        assert!(text.contains("\"control_count\":12"));
        assert!(text.contains("\"all_pass\":true"));
    }

    #[test]
    fn a_malformed_marker_or_journal_is_refused() {
        let good = started_for(Path::new(".")).render();
        for (what, body) in [
            ("empty", String::new()),
            ("truncated", good[..good.len() / 2].to_string()),
            ("no trailing LF", good.trim_end().to_string()),
            ("pretty-printed", good.replace("\":", "\": ")),
            ("bad uuid", good.replace(&"0".repeat(32), "nope")),
            ("extra key", good.replace("}\n", ",\"x\":1}\n")),
            ("not JSON", "controls".to_string()),
        ] {
            assert!(
                ControlsStarted::parse(&body).is_err(),
                "{what} must be refused"
            );
        }
        // The journal, likewise.
        for bad in [
            "P6\t1\tPASS\t10",           // four fields
            "P9\t1\tPASS\t10\t\"x\"",    // unknown id
            "P6\t2\tPASS\t10\t\"x\"",    // wrong ordinal for P6
            "P6\t1\tMAYBE\t10\t\"x\"",   // status outside the domain
            "P6\t1\tPASS\tlater\t\"x\"", // offset not an integer
            "P6\t1\tPASS\t10\tnot json", // detail not a JSON string
        ] {
            assert!(ControlRow::parse(bad).is_err(), "{bad:?} must be refused");
        }
        assert!(parse_control_journal(b"no header here\n").is_err());
    }

    // ------------------------------------------------------------------- P2

    fn p2_meta() -> Metadata {
        let mut m = meta();
        m.session = MetaSession::P2;
        m
    }

    fn p2_row_context() -> crate::journal::RowContext {
        crate::journal::RowContext {
            run_uuid: "0".repeat(32),
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            host_fingerprint: fixture_host_fields().fingerprint(),
            timer_resolution_ms: 0.00002,
            cpu_set: "0-3".into(),
            thread_count: 1,
        }
    }

    /// Build the journal the child is required to leave, without spawning a
    /// process: the shape is what P2 tests, and the shape is checkable here.
    fn write_p2_journal(dir: &Path, with_close: bool, with_measurement: bool) -> PathBuf {
        std::fs::create_dir_all(dir.join(CONTROL_DIR)).unwrap();
        let path = dir.join(P2_JOURNAL);
        let mut j = crate::journal::Journal::create(&path, &p2_meta()).unwrap();
        let c = p2_row_context();
        j.append(&c.session_open(0, 10, 0.5)).unwrap();
        if with_measurement {
            j.append(&c.lost(0, crate::journal::Phase::A, 1, None, 20, "p2"))
                .unwrap();
        }
        if with_close {
            j.append(&c.session_close(0, 30, false, 0.5)).unwrap();
        }
        path
    }

    #[test]
    fn a_p2_journal_needs_one_open_and_one_measurement_and_no_close() {
        let d = TempDir::new("p2ok");
        let p = write_p2_journal(d.path(), false, true);
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(classify_p2_journal(text.as_bytes()), Ok(()));
        // The declared coordinates are the frozen control ones.
        assert!(text.contains("\tA\t0\t1\tNA\t"));

        // §C11.4: a SESSION-CLOSE row means the child did not die where it must.
        let d = TempDir::new("p2close");
        let p = write_p2_journal(d.path(), true, true);
        assert_eq!(
            classify_p2_journal(&std::fs::read(&p).unwrap()),
            Err(ControlError::Domain(
                "a P2 journal must contain no SESSION-CLOSE row"
            ))
        );

        // …and an open with no measurement proves nothing survived.
        let d = TempDir::new("p2empty");
        let p = write_p2_journal(d.path(), false, false);
        assert_eq!(
            classify_p2_journal(&std::fs::read(&p).unwrap()),
            Err(ControlError::Domain(
                "a P2 journal needs at least one OK or LOST row"
            ))
        );
    }

    /// §C11.4: abnormal termination — a signal, never exit code 0.
    #[cfg(unix)]
    #[test]
    fn p2_requires_a_signal_and_rejects_a_clean_exit() {
        use std::process::Command;
        let killed = Command::new("sh")
            .arg("-c")
            .arg("kill -ABRT $$")
            .status()
            .unwrap();
        assert_eq!(classify_p2_exit(&killed), Ok(6), "SIGABRT");

        for cmd in ["exit 0", "exit 1", "exit 3"] {
            let clean = Command::new("sh").arg("-c").arg(cmd).status().unwrap();
            let err = classify_p2_exit(&clean).unwrap_err();
            assert!(
                matches!(&err, ControlError::Field { what, .. }
                    if what.contains("did not terminate by signal")),
                "{cmd}: {err}"
            );
        }
    }

    /// §C11.4: the child's argument is internal, and is not a public CLI flag.
    #[test]
    fn the_p2_child_branch_is_internal_only() {
        assert!(P2_CHILD_ARG.starts_with("--__"), "{P2_CHILD_ARG}");
        assert!(P2_CHILD_ARG.contains("p2-child"));
    }

    /// §C11.3: the central journal and the P2 journal are different schemas and
    /// must not be read as one another.
    #[test]
    fn the_central_journal_and_the_p2_journal_are_not_interchangeable() {
        let d = TempDir::new("mix");
        let p = write_p2_journal(d.path(), false, true);
        let p2_text = std::fs::read_to_string(&p).unwrap();
        assert!(
            parse_control_journal(p2_text.as_bytes()).is_err(),
            "24 columns are not 5"
        );

        let e = TempDir::new("mix2");
        std::fs::create_dir(e.path().join(CONTROL_DIR)).unwrap();
        let mut j = ControlJournal::create(&e.path().join(CONTROL_JOURNAL), &meta()).unwrap();
        j.append(&ControlRow {
            control_id: ControlId::P6,
            ordinal: 1,
            status: ControlStatus::Pass,
            monotonic_offset_ms: 1,
            detail: json_string_encode("ok"),
        })
        .unwrap();
        let ctrl_text = std::fs::read_to_string(e.path().join(CONTROL_JOURNAL)).unwrap();
        assert!(
            classify_p2_journal(ctrl_text.as_bytes()).is_err(),
            "5 columns are not 24"
        );

        // The central journal refuses a non-CONTROL metadata session outright.
        let f = TempDir::new("mix3");
        std::fs::create_dir(f.path().join(CONTROL_DIR)).unwrap();
        assert!(matches!(
            ControlJournal::create(&f.path().join(CONTROL_JOURNAL), &p2_meta()),
            Err(ControlError::Domain(_))
        ));
    }

    /// §C11.3: exactly one row per control, and never a second.
    #[test]
    fn a_control_writes_exactly_one_row() {
        let d = TempDir::new("onerow");
        std::fs::create_dir(d.path().join(CONTROL_DIR)).unwrap();
        let mut j = ControlJournal::create(&d.path().join(CONTROL_JOURNAL), &meta()).unwrap();
        let row = ControlRow {
            control_id: ControlId::P6,
            ordinal: 1,
            status: ControlStatus::Pass,
            monotonic_offset_ms: 1,
            detail: json_string_encode("ok"),
        };
        j.append(&row).unwrap();
        assert_eq!(
            j.append(&row),
            Err(ControlError::Domain("a control writes exactly one row"))
        );
        // …and the journal path itself is claimed once.
        assert!(matches!(
            ControlJournal::create(&d.path().join(CONTROL_JOURNAL), &meta()),
            Err(ControlError::AlreadyPresent(_))
        ));
    }

    // ------------------------------------------ the real P2 child process
    //
    // The actual-binary test lives in `tests/test_rc021_p2_child.rs`, where
    // Cargo guarantees a freshly built binary through `CARGO_BIN_EXE_*`. A unit
    // test here would have to find the product binary itself and could run
    // against a stale one. What remains here is the pure classification of what
    // such a child leaves behind.

    /// A child that dies **before** the durable measurement row fails P2: the
    /// control is about evidence surviving, not about dying.
    #[test]
    fn an_abort_before_the_durable_row_fails_p2() {
        let d = TempDir::new("p2_early");
        let p = write_p2_journal(d.path(), false, false); // SESSION-OPEN only
        assert_eq!(
            classify_p2_journal(&std::fs::read(&p).unwrap()),
            Err(ControlError::Domain(
                "a P2 journal needs at least one OK or LOST row"
            ))
        );
    }

    /// §C8.3: the load average a P2 `SESSION-OPEN` row records comes from the
    /// source it was given, exactly — proved against a synthetic file, so the
    /// check does not depend on how busy this machine happens to be.
    #[test]
    fn the_p2_evidence_records_the_load_average_it_was_given() {
        let d = TempDir::new("p2_loadavg");
        std::fs::create_dir(d.path().join(CONTROL_DIR)).unwrap();
        let src = d.path().join("loadavg");
        std::fs::write(&src, "0.77 0.31 0.20 1/512 12345\n").unwrap();

        // The uptime axis has to be real for the child's own provenance, so the
        // manifest's boot id is this boot's and its start uptime is 1 ms.
        let mut meta = p2_meta();
        meta.boot_id = crate::host::parse_boot_id(
            &std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap(),
        )
        .unwrap();
        p2_write_evidence(d.path(), &meta, &p2_row_context(), 1, &src)
            .expect("the evidence must be written");

        let bytes = std::fs::read(d.path().join(P2_JOURNAL)).unwrap();
        assert_eq!(classify_p2_journal(&bytes), Ok(()));
        let read = crate::journal::parse_journal(&bytes);
        let open = read
            .rows
            .iter()
            .find(|r| r.status == crate::journal::Status::SessionOpen)
            .expect("SESSION-OPEN");

        // Exact bits: the value was read, not approximated or invented.
        let got = open
            .load_avg_start
            .expect("load_avg_start must carry a value");
        assert_eq!(
            got.to_bits(),
            0.77f64.to_bits(),
            "load_avg_start {got} is not the 0.77 the source gave"
        );
        // §C8.4: SESSION-OPEN carries no load_avg_end.
        assert_eq!(open.load_avg_end, None);
        // §C8.1: canonical shortest round-trip on the wire.
        let text = std::str::from_utf8(&bytes).unwrap();
        let line = text
            .lines()
            .find(|l| l.contains("\tSESSION-OPEN\t"))
            .unwrap();
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f[14], "0.77");
        assert_eq!(f[14].parse::<f64>().unwrap().to_bits(), got.to_bits());
        assert_eq!(f[15], "NA");
    }

    /// An unusable source is a refusal, never an invented value.
    #[test]
    fn an_unusable_load_average_source_refuses() {
        let mut meta = p2_meta();
        meta.boot_id = crate::host::parse_boot_id(
            &std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap(),
        )
        .unwrap();
        for (what, body) in [
            ("absent", None),
            ("empty", Some("")),
            ("not a number", Some("later 0.3 0.2\n")),
            ("negative", Some("-1.0 0.3 0.2\n")),
        ] {
            let d = TempDir::new("p2_badload");
            std::fs::create_dir(d.path().join(CONTROL_DIR)).unwrap();
            let src = d.path().join("loadavg");
            if let Some(b) = body {
                std::fs::write(&src, b).unwrap();
            }
            let err =
                p2_write_evidence(d.path(), &meta, &p2_row_context(), 1, &src).expect_err(what);
            assert!(!err.is_empty(), "{what}");
            // The refusal precedes the journal, so there is no row carrying a
            // made-up load average.
            assert!(
                !d.path().join(P2_JOURNAL).exists(),
                "{what}: no journal may be created"
            );
        }
    }

    /// §C11.4: the P2 journal's metadata session is `P2`, never `CONTROL`.
    #[test]
    fn a_p2_journal_declaring_control_is_refused() {
        let d = TempDir::new("p2_meta");
        std::fs::create_dir_all(d.path().join(CONTROL_DIR)).unwrap();
        let path = d.path().join(P2_JOURNAL);
        let mut j = crate::journal::Journal::create(&path, &meta()).unwrap();
        let c = p2_row_context();
        j.append(&c.session_open(0, 10, 0.0)).unwrap();
        assert_eq!(
            classify_p2_journal(&std::fs::read(&path).unwrap()),
            Err(ControlError::Domain(
                "the P2 journal's metadata session must be P2"
            ))
        );
    }

    /// A measurement row before `SESSION-OPEN` is refused.
    #[test]
    fn a_measurement_before_session_open_is_refused() {
        let d = TempDir::new("p2_order");
        std::fs::create_dir_all(d.path().join(CONTROL_DIR)).unwrap();
        let path = d.path().join(P2_JOURNAL);
        let mut j = crate::journal::Journal::create(&path, &p2_meta()).unwrap();
        let c = p2_row_context();
        j.append(&c.lost(0, crate::journal::Phase::A, 1, None, 20, "early"))
            .unwrap();
        j.append(&c.session_open(0, 10, 0.0)).unwrap();
        assert_eq!(
            classify_p2_journal(&std::fs::read(&path).unwrap()),
            Err(ControlError::Domain(
                "a measurement row before SESSION-OPEN"
            ))
        );
    }

    // ------------------------------------------- the strict central reader

    fn built_control_journal(d: &Path) -> Vec<u8> {
        std::fs::create_dir_all(d.join(CONTROL_DIR)).unwrap();
        let mut j = ControlJournal::create(&d.join(CONTROL_JOURNAL), &meta()).unwrap();
        j.append(&ControlRow {
            control_id: ControlId::P6,
            ordinal: 1,
            status: ControlStatus::Pass,
            monotonic_offset_ms: 1,
            detail: json_string_encode("ok"),
        })
        .unwrap();
        std::fs::read(d.join(CONTROL_JOURNAL)).unwrap()
    }

    #[test]
    fn the_central_reader_requires_the_full_metadata_block() {
        let d = TempDir::new("reader");
        let good = built_control_journal(d.path());
        let r = read_control_journal(&good).unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.metadata.session, MetaSession::Control);

        let text = String::from_utf8(good.clone()).unwrap();
        let lines: Vec<&str> = text.split('\n').collect();

        // a missing metadata line
        let mut without = lines.clone();
        without.remove(3);
        assert!(read_control_journal(without.join("\n").as_bytes()).is_err());

        // two metadata lines transposed
        let mut swapped = lines.clone();
        swapped.swap(2, 5);
        let err = read_control_journal(swapped.join("\n").as_bytes()).unwrap_err();
        assert!(
            matches!(&err, ControlError::Field { what, .. } if what.contains("out of order")),
            "{err}"
        );

        // metadata after the header
        let mut after = lines.clone();
        after.insert(META_KEYS.len() + 1, "#rc021_meta\tsession\t\"CONTROL\"");
        assert_eq!(
            read_control_journal(after.join("\n").as_bytes()).unwrap_err(),
            ControlError::Domain("metadata after the header")
        );

        // no trailing LF
        assert_eq!(
            read_control_journal(text.trim_end().as_bytes()).unwrap_err(),
            ControlError::Domain("the file must end with a line feed")
        );
        // not UTF-8
        let mut bad = good.clone();
        bad.insert(0, 0xff);
        assert_eq!(
            read_control_journal(&bad).unwrap_err(),
            ControlError::Domain("control journal is not valid UTF-8")
        );
        // empty
        assert!(read_control_journal(b"").is_err());
    }

    /// §C11.3: the central journal declares `CONTROL`, never `P2`.
    #[test]
    fn a_central_journal_declaring_p2_is_refused() {
        let d = TempDir::new("central_p2");
        let good = built_control_journal(d.path());
        let text = String::from_utf8(good).unwrap();
        let swapped = text.replace(
            "#rc021_meta\tsession\t\"CONTROL\"",
            "#rc021_meta\tsession\t\"P2\"",
        );
        assert_ne!(swapped, text);
        assert_eq!(
            read_control_journal(swapped.as_bytes()).unwrap_err(),
            ControlError::Domain("the control journal's metadata session must be CONTROL")
        );
    }

    /// §C11.2: execution stops at the first FAIL, so no row may follow one.
    #[test]
    fn no_row_may_follow_a_fail_row() {
        let d = TempDir::new("after_fail");
        std::fs::create_dir_all(d.path().join(CONTROL_DIR)).unwrap();
        let path = d.path().join(CONTROL_JOURNAL);
        let mut j = ControlJournal::create(&path, &meta()).unwrap();
        j.append(&ControlRow {
            control_id: ControlId::P6,
            ordinal: 1,
            status: ControlStatus::Fail,
            monotonic_offset_ms: 1,
            detail: json_string_encode("bad"),
        })
        .unwrap();
        j.append(&ControlRow {
            control_id: ControlId::P5,
            ordinal: 2,
            status: ControlStatus::Pass,
            monotonic_offset_ms: 2,
            detail: json_string_encode("ok"),
        })
        .unwrap();
        assert_eq!(
            read_control_journal(&std::fs::read(&path).unwrap()).unwrap_err(),
            ControlError::Domain("a row follows a FAIL row")
        );
    }

    /// The rows must follow the frozen order, not merely be numbered.
    #[test]
    fn rows_out_of_the_frozen_order_are_refused() {
        let d = TempDir::new("order_rows");
        std::fs::create_dir_all(d.path().join(CONTROL_DIR)).unwrap();
        let path = d.path().join(CONTROL_JOURNAL);
        let mut j = ControlJournal::create(&path, &meta()).unwrap();
        j.append(&ControlRow {
            control_id: ControlId::P6,
            ordinal: 1,
            status: ControlStatus::Pass,
            monotonic_offset_ms: 1,
            detail: json_string_encode("ok"),
        })
        .unwrap();
        let text = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
        // P4 at ordinal 2 is well-formed on its own but out of the order.
        // P4 belongs at ordinal 3, so it cannot legitimately appear second —
        // whether it claims ordinal 2 (which contradicts its own identity) or
        // its true ordinal 3 (which contradicts its position).
        for forged in [
            format!("{text}P4\t2\tPASS\t3\t\"ok\"\n"),
            format!("{text}P4\t3\tPASS\t3\t\"ok\"\n"),
        ] {
            assert!(
                read_control_journal(forged.as_bytes()).is_err(),
                "P4 must not appear second: {forged:?}"
            );
        }
        // …and the row that does belong there is accepted.
        let ok = format!("{text}P5\t2\tPASS\t3\t\"ok\"\n");
        let r = read_control_journal(ok.as_bytes()).unwrap();
        assert_eq!(r.rows[1].control_id, ControlId::P5);
    }

    // ------------------------------------------------- the context binding

    /// §C9 metadata is self-describing: a canonical block from another run
    /// validates perfectly. Only the binding ties it to this run.
    #[test]
    fn foreign_but_canonical_metadata_is_refused_before_the_marker() {
        type Mutate = (&'static str, Box<dyn Fn(&mut Metadata)>);
        let cases: Vec<Mutate> = vec![
            (
                "run_uuid",
                Box::new(|m: &mut Metadata| m.run_uuid = "1".repeat(32)),
            ),
            (
                "boot_id",
                Box::new(|m: &mut Metadata| m.boot_id = "boot-9".into()),
            ),
            (
                "run_start_uptime_ms",
                Box::new(|m: &mut Metadata| m.run_start_uptime_ms = 9_999),
            ),
            (
                "repo_commit",
                Box::new(|m: &mut Metadata| m.repo_commit = "9".repeat(40)),
            ),
            (
                "prereg_commit",
                Box::new(|m: &mut Metadata| m.prereg_commit = "8".repeat(40)),
            ),
            // §C10.1 counts the amendment array, and each SHA inside it: a
            // journal naming one amendment and not the other binds to a
            // different reading of the protocol.
            (
                "amendment_commits",
                Box::new(|m: &mut Metadata| m.amendment_commits[0] = "1".repeat(40)),
            ),
            (
                "amendment_commits",
                Box::new(|m: &mut Metadata| m.amendment_commits[1] = "2".repeat(40)),
            ),
            (
                "amendment_commits",
                Box::new(|m: &mut Metadata| {
                    m.amendment_commits = vec!["3".repeat(40), "4".repeat(40)]
                }),
            ),
            (
                "instrument_birth_commit",
                Box::new(|m: &mut Metadata| m.instrument_birth_commit = "7".repeat(40)),
            ),
            (
                "host_fingerprint",
                Box::new(|m: &mut Metadata| m.host_fingerprint = "b".repeat(64)),
            ),
            // Configuration, checked as well but not among the eight.
            (
                "cpu_set",
                Box::new(|m: &mut Metadata| m.cpu_set = "0-1".into()),
            ),
            (
                "thread_count",
                Box::new(|m: &mut Metadata| m.thread_count = 9),
            ),
        ];
        // The eight §C10.1 identity fields, plus the configuration fields the
        // binding also checks. `command_line` and `utc_start` are in neither
        // list: they describe this invocation.
        assert_eq!(METADATA_IDENTITY_FIELDS.len(), 8);
        assert!(METADATA_IDENTITY_FIELDS.contains(&"amendment_commits"));
        assert!(!METADATA_IDENTITY_FIELDS.contains(&"cpu_set"));
        assert!(METADATA_CONFIG_FIELDS.contains(&"cpu_set"));
        for f in ["command_line", "utc_start"] {
            assert!(!METADATA_IDENTITY_FIELDS.contains(&f), "{f}");
            assert!(!METADATA_CONFIG_FIELDS.contains(&f), "{f}");
        }
        let mut named: Vec<&str> = cases.iter().map(|(f, _)| *f).collect();
        named.sort_unstable();
        named.dedup();
        for f in METADATA_IDENTITY_FIELDS {
            assert!(named.contains(&f), "{f} has no foreign-metadata test");
        }
        for (field, mutate) in cases {
            let d = TempDir::new("foreign_meta");
            let m = fixture_manifest(d.path());
            let mut meta = meta();
            mutate(&mut meta);
            // The block itself is beyond reproach.
            assert!(meta.validate().is_ok(), "{field}: metadata must stay valid");
            assert!(meta.render().is_ok(), "{field}");

            let c = ControlsContext {
                manifest: &m,
                command_line: vec!["exp_rc021_host_qualify".into()],
                meta,
            };
            let err = c.bind().unwrap_err();
            assert!(
                matches!(&err, ControlError::Field { what, .. } if *what == field),
                "{field}: {err}"
            );

            let before = snapshot(d.path());
            let mut r = FakeRunner::all_pass();
            let out = run_controls(d.path(), &c, &mut r, &mut FakeClock(0));
            assert!(
                matches!(out, ControlsOutcome::RefusedBeforeMeasurement { .. }),
                "{field}: {out:?}"
            );
            assert!(r.seen.is_empty(), "{field}: nothing may execute");
            assert_eq!(snapshot(d.path()), before, "{field}: bytes changed");
        }
    }

    #[test]
    fn each_invocation_may_record_new_timer_and_diagnostic_observations() {
        let d = TempDir::new("invocation_observations");
        let m = fixture_manifest(d.path());
        let mut invocation = meta();
        invocation.timer_resolution_ms = f64::from_bits(m.timer_resolution_ms.to_bits() + 1);
        invocation.diag_availability.cpu_time = !m.diag_availability.cpu_time;
        invocation.diag_availability.ctx_switches = !m.diag_availability.ctx_switches;
        let c = ControlsContext {
            manifest: &m,
            command_line: invocation.command_line.clone(),
            meta: invocation,
        };
        assert_eq!(c.bind(), Ok(()), "§C6.1 says a change is not a failure");
        assert!(!METADATA_CONFIG_FIELDS.contains(&"timer_resolution_ms"));
        assert!(!METADATA_CONFIG_FIELDS.contains(&"diag_availability"));
    }

    /// §C10.2: terminal or closing evidence forbids controls outright.
    #[test]
    fn terminal_evidence_forbids_controls() {
        for rel in TERMINAL_EVIDENCE {
            let d = TempDir::new("terminal");
            let m = fixture_manifest(d.path());
            std::fs::write(d.path().join(rel), b"whatever").unwrap();
            let before = snapshot(d.path());
            let mut r = FakeRunner::all_pass();
            let out = run_controls(d.path(), &ctx(&m), &mut r, &mut FakeClock(0));
            assert_eq!(out, ControlsOutcome::AlreadyStarted, "{rel}");
            assert!(r.seen.is_empty(), "{rel}");
            assert_eq!(snapshot(d.path()), before, "{rel}");
        }
    }

    // ---------------------------------------------------- production routing

    /// Every control routes to its own predicate. The environment is injected
    /// and each control is failed in turn from the environment alone.
    struct FakeEnv {
        provenance_ok: bool,
        res_ms: f64,
        sentinel_ms: f64,
        load_ms: f64,
        diag_bias: f64,
        n3: N3Report,
        p2: ControlOutcome,
        calls: Vec<&'static str>,
        /// Injected drift, so a pair can be given a spread above the bound
        /// without changing the work.
        second_scale: f64,
        /// Injected work drift, so N1's bit-identity check can be broken one
        /// field at a time.
        work_drift: Option<&'static str>,
        seen_work: Vec<SentinelSample>,
        nth: u32,
    }

    fn good_n3() -> N3Report {
        N3Report {
            sentinel_executions: 30,
            sweeps: expected_session_sweeps(),
            sentinel_seed: crate::protocol::RC021_SENTINEL_SEED,
            load_seed: crate::protocol::RC021_LOAD_SEED,
            rows: 15,
            metadata_is_n3: true,
            coordinates_frozen: true,
        }
    }

    impl FakeEnv {
        fn good() -> FakeEnv {
            FakeEnv {
                provenance_ok: true,
                res_ms: 0.00002,
                sentinel_ms: 2.0,
                load_ms: 500.0,
                diag_bias: 0.0,
                n3: good_n3(),
                p2: ControlOutcome::pass("P2 child aborted; session ABORTED"),
                calls: Vec::new(),
                second_scale: 1.0,
                work_drift: None,
                seen_work: Vec::new(),
                nth: 0,
            }
        }
    }

    impl ControlEnvironment for FakeEnv {
        fn provenance(&mut self) -> Result<(), String> {
            self.calls.push("provenance");
            if self.provenance_ok {
                Ok(())
            } else {
                Err("prereg is dirty".into())
            }
        }
        fn prepare_phase_b(&mut self) -> Result<(), String> {
            self.calls.push("warmup");
            Ok(())
        }
        fn sentinel(
            &mut self,
            diagnostics: bool,
            extra: SentinelRequest,
        ) -> Result<SentinelSample, String> {
            self.calls.push("sentinel");
            self.nth += 1;
            let base = if diagnostics {
                self.sentinel_ms * (1.0 + self.diag_bias)
            } else {
                self.sentinel_ms
            };
            let extra_sweeps = extra.extra_sweeps();
            let mut sample = SentinelSample {
                wall_ms: base
                    * (1.0 + extra_sweeps as f64 / crate::protocol::SENTINEL_WINDOW as f64),
                measured_sweeps: crate::protocol::SENTINEL_WINDOW + extra_sweeps,
                prefix_sweeps: crate::protocol::SENTINEL_PREFIX_SWEEPS,
                replicas: crate::protocol::SENTINEL_REPLICAS,
                seed: crate::protocol::RC021_SENTINEL_SEED,
                diagnostics,
            };
            // Only the *second* execution of a pair drifts, so the first one
            // remains the reference.
            if self.nth.is_multiple_of(2) {
                sample.wall_ms *= self.second_scale;
                match self.work_drift {
                    Some("measured_sweeps") => sample.measured_sweeps += 1,
                    Some("prefix_sweeps") => sample.prefix_sweeps += 1,
                    Some("replicas") => sample.replicas += 1,
                    Some("seed") => sample.seed += 1,
                    Some("diagnostics") => sample.diagnostics = !sample.diagnostics,
                    _ => {}
                }
            }
            self.seen_work.push(sample);
            Ok(sample)
        }
        fn load_block_ms(&mut self) -> Result<f64, String> {
            self.calls.push("load");
            Ok(self.load_ms)
        }
        fn timer_resolution_ms(&mut self) -> f64 {
            self.res_ms
        }
        fn run_n3(&mut self) -> Result<N3Report, String> {
            self.calls.push("n3");
            Ok(self.n3)
        }
        fn run_p2(&mut self) -> ControlOutcome {
            self.calls.push("p2");
            self.p2.clone()
        }
    }

    #[test]
    fn every_control_routes_to_its_own_predicate_and_can_fail() {
        // All twelve pass on a healthy environment.
        let mut env = FakeEnv::good();
        let mut r = ProductionRunner::new(&mut env);
        for id in CONTROL_ORDER {
            let out = r.run(id);
            assert_eq!(
                out.status,
                ControlStatus::Pass,
                "{} should pass: {}",
                id.as_str(),
                out.detail
            );
        }

        // …and each fails from its own cause, not from a shared one.
        type Break = (ControlId, Box<dyn Fn(&mut FakeEnv)>);
        let breaks: Vec<Break> = vec![
            (
                ControlId::P6,
                Box::new(|e: &mut FakeEnv| e.provenance_ok = false),
            ),
            (
                ControlId::P8,
                Box::new(|e: &mut FakeEnv| e.sentinel_ms = 0.0000001),
            ),
            (
                ControlId::N2,
                Box::new(|e: &mut FakeEnv| e.sentinel_ms = 0.0),
            ),
            (ControlId::P7, Box::new(|e: &mut FakeEnv| e.load_ms = 0.001)),
            (
                ControlId::C10,
                Box::new(|e: &mut FakeEnv| e.diag_bias = 0.5),
            ),
            (
                ControlId::N3,
                Box::new(|e: &mut FakeEnv| e.n3.metadata_is_n3 = false),
            ),
            (
                ControlId::N3,
                Box::new(|e: &mut FakeEnv| e.n3.coordinates_frozen = false),
            ),
            (ControlId::N3, Box::new(|e: &mut FakeEnv| e.n3.rows = 14)),
            (ControlId::N3, Box::new(|e: &mut FakeEnv| e.n3.sweeps += 1)),
            (
                ControlId::N3,
                Box::new(|e: &mut FakeEnv| e.n3.load_seed = 12008),
            ),
            (
                ControlId::P2,
                Box::new(|e: &mut FakeEnv| e.p2 = ControlOutcome::fail("clean exit")),
            ),
        ];
        for (id, brk) in breaks {
            let mut env = FakeEnv::good();
            brk(&mut env);
            let mut r = ProductionRunner::new(&mut env);
            let out = r.run(id);
            assert_eq!(
                out.status,
                ControlStatus::Fail,
                "{} must fail: {}",
                id.as_str(),
                out.detail
            );
            // and the healthy controls are unaffected by that same break
            let mut env2 = FakeEnv::good();
            brk(&mut env2);
            let mut r2 = ProductionRunner::new(&mut env2);
            for other in [ControlId::P5, ControlId::P4, ControlId::P3] {
                assert_eq!(
                    r2.run(other).status,
                    ControlStatus::Pass,
                    "{} broke {}",
                    id.as_str(),
                    other.as_str()
                );
            }
        }
    }

    /// Each control must be reached by **its own** predicate, not merely by
    /// one that happens to pass. Two controls that both pass on a healthy
    /// environment could otherwise be swapped without any test noticing, so
    /// each is pinned by the evidence only it produces.
    #[test]
    fn each_control_id_reaches_its_own_predicate() {
        let expected: [(ControlId, &str); 12] = [
            (ControlId::P6, "provenance gate satisfied"),
            (ControlId::P5, "all families refused"),
            (
                ControlId::P4,
                "no energy, cut, best, quality, delta or objective column",
            ),
            (ControlId::P3, "production conversions"),
            (ControlId::P2, "ABORTED"),
            (ControlId::P8, "floor"),
            (ControlId::N2, "back-to-back spread"),
            (ControlId::N1, "identical work; spread"),
            (ControlId::N3, "one session replayed"),
            (ControlId::P1, "injected spread"),
            (ControlId::P7, "load block"),
            (ControlId::C10, "median on"),
        ];
        assert_eq!(expected.len(), CONTROL_ORDER.len());
        for (id, marker) in expected {
            let mut env = FakeEnv::good();
            let mut r = ProductionRunner::new(&mut env);
            let out = r.run(id);
            assert_eq!(out.status, ControlStatus::Pass, "{}", id.as_str());
            assert!(
                out.detail.contains(marker),
                "{} was answered by another predicate: {}",
                id.as_str(),
                out.detail
            );
        }
        // The markers are distinct, so the assertion above cannot be satisfied
        // by the wrong predicate.
        let mut markers: Vec<&str> = expected.iter().map(|(_, m)| *m).collect();
        markers.sort_unstable();
        markers.dedup();
        assert_eq!(markers.len(), 12);

        // …and each control consults only the environment it needs.
        for (id, want) in [
            (ControlId::P6, vec!["provenance"]),
            (ControlId::N3, vec!["n3"]),
            (ControlId::P2, vec!["p2"]),
            (ControlId::P7, vec!["load"]),
            (ControlId::P4, Vec::new()),
            (ControlId::P3, Vec::new()),
            (ControlId::P5, Vec::new()),
        ] {
            let mut env = FakeEnv::good();
            {
                let mut r = ProductionRunner::new(&mut env);
                let _ = r.run(id);
            }
            assert_eq!(
                env.calls,
                want,
                "{} consulted the wrong sources",
                id.as_str()
            );
        }
    }

    /// §8's frozen exit-code table, status by status.
    #[test]
    fn every_run_status_carries_its_frozen_exit_code() {
        let table: [(RunStatus, i32, &str); 6] = [
            (RunStatus::HostQualified, 0, "HOST-QUALIFIED"),
            (RunStatus::HostNotQualified, 1, "HOST-NOT-QUALIFIED"),
            (
                RunStatus::RefusedBeforeMeasurement,
                2,
                "REFUSED-BEFORE-MEASUREMENT",
            ),
            (RunStatus::InstrumentInvalid, 3, "INSTRUMENT-INVALID"),
            (RunStatus::JournalInvalid, 4, "JOURNAL-INVALID"),
            (
                RunStatus::InconclusiveUnderpowered,
                5,
                "INCONCLUSIVE-UNDERPOWERED",
            ),
        ];
        assert_eq!(table.len(), RUN_STATUSES.len());
        for (status, code, name) in table {
            assert_eq!(status.exit_code(), code, "{name}");
            assert_eq!(status.as_str(), name);
            assert!(RUN_STATUSES.contains(&status), "{name}");
        }
        // All six distinct, so no two statuses are indistinguishable to a caller.
        let mut codes: Vec<i32> = RUN_STATUSES.iter().map(|s| s.exit_code()).collect();
        codes.sort_unstable();
        assert_eq!(codes, vec![0, 1, 2, 3, 4, 5]);
        // Class II is exactly the two publishable ones.
        assert_eq!(RUN_STATUSES.iter().filter(|s| s.is_class_two()).count(), 2);
        // §C11.25's classes speak the same vocabulary.
        assert_eq!(
            FailureClass::JournalInvalid.run_status(),
            RunStatus::JournalInvalid
        );
        assert_eq!(
            FailureClass::JournalInvalid.exit_code(),
            RunStatus::JournalInvalid.exit_code()
        );
        assert_eq!(
            FailureClass::InstrumentInvalid.exit_code(),
            RunStatus::InstrumentInvalid.exit_code()
        );
    }

    /// The classification, variant by variant. One row per variant of both
    /// enums: terminal or not, the expected `Option<RunStatus>`, the expected
    /// `Option<exit code>`, and the agreement of the two ways to compute it.
    #[test]
    fn every_outcome_variant_has_one_classification_and_one_code() {
        type Row = (&'static str, Option<RunStatus>, Option<i32>);
        let controls: [Row; 5] = [
            // A completed control phase is a continuation: §8's
            // REFUSED-BEFORE-MEASUREMENT says "the run did not start" and
            // consumes no seeds, and both are false once the controls ran.
            ("Complete", None, None),
            (
                "RefusedBeforeMeasurement",
                Some(RunStatus::RefusedBeforeMeasurement),
                Some(2),
            ),
            (
                "AlreadyStarted",
                Some(RunStatus::InstrumentInvalid),
                Some(3),
            ),
            ("WriteFailed", Some(RunStatus::JournalInvalid), Some(4)),
            // The sample carries the InstrumentInvalid class.
            ("Failed", Some(RunStatus::InstrumentInvalid), Some(3)),
        ];
        let sessions: [Row; 10] = [
            ("Proceed", None, None),
            (
                "ShortGap",
                Some(RunStatus::RefusedBeforeMeasurement),
                Some(2),
            ),
            ("Terminal", Some(RunStatus::InstrumentInvalid), Some(3)),
            (
                "AlreadyInvalid",
                Some(RunStatus::InstrumentInvalid),
                Some(3),
            ),
            (
                "DamagedRunInvalid",
                Some(RunStatus::JournalInvalid),
                Some(4),
            ),
            (
                "PredecessorNotClosed",
                Some(RunStatus::RefusedBeforeMeasurement),
                Some(2),
            ),
            (
                "InvalidRequest",
                Some(RunStatus::RefusedBeforeMeasurement),
                Some(2),
            ),
            (
                "HostChangedMidSession",
                Some(RunStatus::InstrumentInvalid),
                Some(3),
            ),
            (
                "MeasurementFailed",
                Some(RunStatus::InstrumentInvalid),
                Some(3),
            ),
            (
                "JournalWriteFailed",
                Some(RunStatus::JournalInvalid),
                Some(4),
            ),
        ];

        // Every variant of each enum appears in its table, and the tables are
        // the complete inventory — a new variant cannot be missed.
        let seen_c: Vec<&str> = sample_controls_outcomes()
            .iter()
            .map(|o| o.variant_name())
            .collect();
        let mut inv_c: Vec<&str> = CONTROLS_VARIANTS.to_vec();
        inv_c.sort_unstable();
        let mut got_c = seen_c.clone();
        got_c.sort_unstable();
        assert_eq!(got_c, inv_c, "sample_controls_outcomes must be complete");
        for (name, _, _) in controls {
            assert!(seen_c.contains(&name), "{name} missing from the sample");
        }
        assert_eq!(controls.len(), CONTROLS_VARIANTS.len());

        let seen_s: Vec<&str> = sample_session_outcomes()
            .iter()
            .map(|o| o.variant_name())
            .collect();
        let mut inv_s: Vec<&str> = SESSION_VARIANTS.to_vec();
        inv_s.sort_unstable();
        let mut got_s = seen_s.clone();
        got_s.sort_unstable();
        assert_eq!(got_s, inv_s, "sample_session_outcomes must be complete");
        assert_eq!(sessions.len(), SESSION_VARIANTS.len());

        for o in sample_controls_outcomes() {
            let (_, want_status, want_code) = controls
                .iter()
                .find(|(n, _, _)| *n == o.variant_name())
                .unwrap_or_else(|| panic!("{} has no table row", o.variant_name()));
            assert_eq!(
                o.terminal_run_status(),
                *want_status,
                "{}",
                o.variant_name()
            );
            assert_eq!(o.terminal_exit_code(), *want_code, "{}", o.variant_name());
            assert_eq!(
                o.is_terminal(),
                want_status.is_some(),
                "{}",
                o.variant_name()
            );
            // The two ways to reach the code agree, because there is only one.
            assert_eq!(
                o.terminal_exit_code(),
                o.terminal_run_status().map(RunStatus::exit_code),
                "{}",
                o.variant_name()
            );
        }
        for o in sample_session_outcomes() {
            let (_, want_status, want_code) = sessions
                .iter()
                .find(|(n, _, _)| *n == o.variant_name())
                .unwrap_or_else(|| panic!("{} has no table row", o.variant_name()));
            assert_eq!(
                o.terminal_run_status(),
                *want_status,
                "{}",
                o.variant_name()
            );
            assert_eq!(o.terminal_exit_code(), *want_code, "{}", o.variant_name());
            assert_eq!(
                o.is_terminal(),
                want_status.is_some(),
                "{}",
                o.variant_name()
            );
            assert_eq!(
                o.terminal_exit_code(),
                o.terminal_run_status().map(RunStatus::exit_code),
                "{}",
                o.variant_name()
            );
        }

        // The continuations, named explicitly.
        assert_eq!(
            ControlsOutcome::Complete {
                monotonic_offset_ms: 1
            }
            .terminal_run_status(),
            None
        );
        assert_eq!(
            crate::session::SessionOutcome::Proceed {
                gap_ms: 600_000,
                anchor: crate::session::GapAnchor::ControlsComplete {
                    monotonic_offset_ms: 0
                },
                order_seed: 31004,
            }
            .terminal_run_status(),
            None
        );
        // A successful mode operation has its own code, not a §8 status.
        assert_eq!(MODE_SUCCESS_EXIT_CODE, 0);
        assert!(
            !RUN_STATUSES
                .iter()
                .any(|s| s.exit_code() == MODE_SUCCESS_EXIT_CODE && *s != RunStatus::HostQualified),
            "only HOST-QUALIFIED shares code 0"
        );
    }

    /// One authority, checked structurally as well as behaviourally.
    ///
    /// Every `exit_code` in the instrument must reach its number through
    /// [`RunStatus::exit_code`]. The two documented exceptions are named here
    /// so that adding a third is a deliberate act: `MODE_SUCCESS_EXIT_CODE`,
    /// which is a mode's code and not a §8 status, and `classify_p2_exit`,
    /// which returns a Unix **signal** number and is not an exit code at all.
    #[test]
    fn no_terminal_exit_code_is_written_as_a_literal() {
        // Behavioural: every terminal API agrees with §8.
        for o in sample_controls_outcomes() {
            assert_eq!(
                o.terminal_exit_code(),
                o.terminal_run_status().map(RunStatus::exit_code)
            );
        }
        for o in sample_session_outcomes() {
            assert_eq!(
                o.terminal_exit_code(),
                o.terminal_run_status().map(RunStatus::exit_code)
            );
        }
        for e in [
            crate::session::RunInvalidError::Invalid("utc"),
            crate::session::RunInvalidError::Write("x".into()),
            crate::session::RunInvalidError::AlreadyPresent,
        ] {
            assert_eq!(e.exit_code(), e.run_status().exit_code());
            assert_eq!(e.run_status(), RunStatus::JournalInvalid);
        }
        assert_eq!(
            FailureClass::JournalInvalid.exit_code(),
            RunStatus::JournalInvalid.exit_code()
        );
        assert_eq!(
            FailureClass::InstrumentInvalid.exit_code(),
            RunStatus::InstrumentInvalid.exit_code()
        );

        // Structural: no `exit_code` body reaches a number any other way.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for rel in [
            "src/bin/exp_rc021_host_qualify/controls.rs",
            "src/bin/exp_rc021_host_qualify/session.rs",
        ] {
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            let mut checked = 0usize;
            for (i, line) in lines.iter().enumerate() {
                if !line.contains("fn exit_code") {
                    continue;
                }
                checked += 1;
                // The body up to its closing brace, at the same indent.
                let body: String = lines[i + 1..(i + 8).min(lines.len())].join("\n");
                assert!(
                    body.contains("run_status()") || body.contains("RunStatus::"),
                    "{rel}:{}: an exit_code that does not go through §8:\n{body}",
                    i + 1
                );
            }
            assert!(checked >= 1, "{rel}: no exit_code found — did it move?");
        }
        // The two documented non-§8 numbers, named so a third is deliberate.
        assert_eq!(MODE_SUCCESS_EXIT_CODE, 0);
        let sig = classify_p2_exit as fn(&std::process::ExitStatus) -> Result<i32, ControlError>;
        let _ = sig; // a signal number, not an exit code
    }

    /// Guards on the checks themselves.
    ///
    /// A test cannot catch a weakening of its own expectation, so the three
    /// ways these checks could be quietly undone are asserted structurally
    /// instead: a `RunInvalidError` code compared to a bare number, the
    /// integration test reintroducing a `/proc/loadavg` comparison, and the
    /// production child being pointed at some other file.
    #[test]
    fn the_checks_themselves_cannot_be_quietly_weakened() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |rel: &str| std::fs::read_to_string(root.join(rel)).unwrap();

        // 1. `RunInvalidError`'s code has no table row to anchor it, so it must
        //    always be compared through §8 — never to a literal.
        // The §8 statuses have a hand-written table by design — it is the
        // non-circular anchor — and it compares `RunStatus`, not `.exit_code()`
        // results, to numbers. So no `.exit_code()` call may be compared to a
        // bare number anywhere.
        for rel in [
            "src/bin/exp_rc021_host_qualify/session.rs",
            "src/bin/exp_rc021_host_qualify/controls.rs",
        ] {
            let text = read(rel);
            for (i, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                let Some(rest) = code.split(".exit_code(), ").nth(1) else {
                    continue;
                };
                assert!(
                    !rest.starts_with(|c: char| c.is_ascii_digit()),
                    "{rel}:{}: an exit_code compared to a literal — compare \
                     through RunStatus instead:\n{line}",
                    i + 1
                );
            }
        }

        // 2. The integration test must not prove provenance by reading
        //    `/proc/loadavg` again: a hard-coded value matches any quiet host.
        let integration = read("tests/test_rc021_p2_child.rs");
        for (i, line) in integration.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            assert!(
                !code.contains("/proc/loadavg"),
                "tests/test_rc021_p2_child.rs:{}: the integration test must stay \
                 host-independent; provenance is proved by the unit test on \
                 p2_write_evidence:\n{line}",
                i + 1
            );
        }

        // 3. Production points the child at the real source, and only that.
        let main_rs = read("src/bin/exp_rc021_host_qualify/main.rs");
        let call = main_rs
            .split("controls::p2_child_main(")
            .nth(1)
            .expect("main.rs must call p2_child_main");
        let args = call.split(");").next().unwrap();
        assert!(
            args.contains("\"/proc/loadavg\""),
            "the production child must be given /proc/loadavg:\n{args}"
        );
        // …and the hidden argv carries only the run directory, so the source is
        // not user-selectable.
        assert!(main_rs.contains("args.len() == 3"));
    }

    /// §6 P3 — the representatives go through the **production** classification,
    /// every terminal variant has one, and every continuation is excluded.
    #[test]
    fn p3_exercises_every_terminal_path_and_no_continuation() {
        let outcomes = representative_terminal_outcomes();
        let covered: Vec<String> = outcomes.iter().map(|o| o.variant_name()).collect();

        // Terminal variants are all present…
        for v in terminal_controls_variants() {
            assert!(
                covered.contains(&format!("Controls::{v}")),
                "Controls::{v} has no representative"
            );
        }
        for v in terminal_session_variants() {
            assert!(
                covered.contains(&format!("Session::{v}")),
                "Session::{v} has no representative"
            );
        }
        // …and the continuations are absent, deliberately.
        assert!(!covered.iter().any(|c| c == "Controls::Complete"));
        assert!(!covered.iter().any(|c| c == "Session::Proceed"));
        assert!(!terminal_controls_variants().contains(&"Complete"));
        assert!(!terminal_session_variants().contains(&"Proceed"));

        // Every §8 status is reachable through a real terminal path — without
        // a continuation being miscast to supply one.
        for want in RUN_STATUSES {
            assert!(
                outcomes.iter().any(|o| o.run_status() == want),
                "{} is unreachable",
                want.as_str()
            );
        }
        // One authority for the code, per representative.
        for o in &outcomes {
            let direct = match o {
                TerminalOutcome::Controls(c) => c.terminal_exit_code(),
                TerminalOutcome::Session(c) => c.terminal_exit_code(),
                TerminalOutcome::Verdict(v) => Some(v.run_status().exit_code()),
                TerminalOutcome::Finalize(f) => f.terminal_exit_code(),
                TerminalOutcome::Verify(v) => v.terminal_exit_code(),
            };
            assert_eq!(
                direct,
                Some(o.run_status().exit_code()),
                "{}",
                o.variant_name()
            );
        }

        let mut env = FakeEnv::good();
        let mut r = ProductionRunner::new(&mut env);
        let out = r.run(ControlId::P3);
        assert_eq!(out.status, ControlStatus::Pass, "{}", out.detail);
        assert!(
            out.detail.contains("production conversions"),
            "{}",
            out.detail
        );
        // Non-vacuous: the count is reported and is the whole enumeration.
        assert!(
            out.detail
                .contains(&format!("{} terminal paths", outcomes.len())),
            "P3 must exercise all {} representatives: {}",
            outcomes.len(),
            out.detail
        );
        assert!(outcomes.len() >= 11);
    }

    /// §6 N1 — bit-identical work is the only thing it may fail on.
    #[test]
    fn n1_fails_only_on_unequal_work() {
        // Identical work with a spread far above the bound is **data**, not a
        // control failure.
        let mut env = FakeEnv::good();
        env.second_scale = 2.0; // a 100% spread
        let mut r = ProductionRunner::new(&mut env);
        let out = r.run(ControlId::N1);
        assert_eq!(
            out.status,
            ControlStatus::Pass,
            "a wide spread on identical work is data: {}",
            out.detail
        );
        assert!(out.detail.contains("above"), "{}", out.detail);
        assert!(
            out.detail.contains("not a control failure"),
            "{}",
            out.detail
        );

        // …and within the bound it is equally an observation.
        let mut env = FakeEnv::good();
        let mut r = ProductionRunner::new(&mut env);
        let out = r.run(ControlId::N1);
        assert_eq!(out.status, ControlStatus::Pass);
        assert!(out.detail.contains("within"), "{}", out.detail);

        // Each work field, one at a time, is a failure.
        for field in [
            "measured_sweeps",
            "prefix_sweeps",
            "replicas",
            "seed",
            "diagnostics",
        ] {
            let mut env = FakeEnv::good();
            env.work_drift = Some(field);
            let mut r = ProductionRunner::new(&mut env);
            let out = r.run(ControlId::N1);
            assert_eq!(
                out.status,
                ControlStatus::Fail,
                "{field} must fail N1: {}",
                out.detail
            );
            assert!(out.detail.contains(field), "{field}: {}", out.detail);
            assert!(
                out.detail.contains("not bit-identical in work"),
                "{field}: {}",
                out.detail
            );
        }
    }

    /// §4.1 / §9: phase-B preparation runs where the protocol says, and nowhere
    /// else. Counted, and positioned relative to the sentinel executions.
    #[test]
    fn phase_b_preparation_runs_only_for_n1_and_criterion_ten() {
        for (id, want_warmups) in [
            (ControlId::N1, 1usize),
            (ControlId::C10, 1),
            (ControlId::N2, 0),
            (ControlId::P8, 0),
            (ControlId::P1, 0),
            (ControlId::P7, 0),
        ] {
            let mut env = FakeEnv::good();
            {
                let mut r = ProductionRunner::new(&mut env);
                let _ = r.run(id);
            }
            let warmups = env.calls.iter().filter(|c| **c == "warmup").count();
            assert_eq!(warmups, want_warmups, "{}", id.as_str());
            if want_warmups > 0 {
                // …and it comes first, before any sentinel.
                assert_eq!(env.calls[0], "warmup", "{}", id.as_str());
                assert!(
                    env.calls.iter().skip(1).all(|c| *c != "warmup"),
                    "{}: the warmup runs once",
                    id.as_str()
                );
            }
        }
    }

    /// §9 criterion 10: 120 executions, interleaved, after phase-B preparation.
    #[test]
    fn criterion_ten_runs_one_hundred_and_twenty_executions() {
        let mut env = FakeEnv::good();
        {
            let mut r = ProductionRunner::new(&mut env);
            let out = r.run(ControlId::C10);
            assert_eq!(out.status, ControlStatus::Pass);
            assert!(out.detail.contains("120 executions"), "{}", out.detail);
        }
        assert_eq!(env.calls.iter().filter(|c| **c == "sentinel").count(), 120);
        assert_eq!(env.calls.iter().filter(|c| **c == "warmup").count(), 1);
        assert_eq!(env.calls[0], "warmup");
        // Interleaved: the arms alternate in pairs.
        let diag: Vec<bool> = env.seen_work.iter().map(|s| s.diagnostics).collect();
        assert_eq!(diag.len(), 120);
        assert_eq!(&diag[..4], &[true, true, false, false]);
        assert_eq!(&diag[4..8], &[true, true, false, false]);
        assert_eq!(diag.iter().filter(|d| **d).count(), 60);
    }

    /// The production adapter exists and is constructible from normative run
    /// inputs alone. It is **not executed**: no control, sentinel or session is
    /// run here.
    #[test]
    fn a_live_environment_exists_and_takes_only_run_inputs() {
        let d = TempDir::new("live");
        let m = fixture_manifest(d.path());
        let inst = crate::protocol::tests_support::synthetic_verified();
        let observed_timer = f64::from_bits(m.timer_resolution_ms.to_bits() + 1);
        let observed_diag = crate::host::DiagProbe {
            cpu_time: !m.diag_availability.cpu_time,
            ctx_switches: !m.diag_availability.ctx_switches,
            freq: !m.diag_availability.freq,
        };
        let live = LiveEnvironment::new(
            d.path(),
            Path::new("."),
            Path::new("/nonexistent/exe"),
            &inst,
            &m,
            observed_timer,
            observed_diag,
            vec!["exp_rc021_host_qualify".into(), "--controls".into()],
            "2026-08-25T00:00:00Z".into(),
        )
        .expect("constructible");
        // It is a real `ControlEnvironment`, so `ProductionRunner` can drive it.
        let _: &dyn ControlEnvironment = &live;
        // Its constructor takes run inputs only — no field of function type,
        // so no caller can substitute what a control measures.
        assert_eq!(
            live.timer_resolution_ms_for_test().to_bits(),
            observed_timer.to_bits()
        );
        let meta = live.journal_metadata(MetaSession::Control);
        assert_eq!(meta.timer_resolution_ms.to_bits(), observed_timer.to_bits());
        assert_eq!(meta.diag_availability.cpu_time, observed_diag.cpu_time);
        assert_eq!(
            meta.diag_availability.ctx_switches,
            observed_diag.ctx_switches
        );
        assert_eq!(meta.diag_availability.freq, observed_diag.freq);
    }

    /// §6 N3 / §5.2 — the **production** replay, driven on a synthetic verified
    /// instance with an injected clock. G11 is neither read nor executed, and
    /// no `FakeEnv` stands in for the executor.
    #[test]
    fn the_n3_replay_writes_a_real_session_journal() {
        let d = TempDir::new("n3");
        let path = d.path().join(N3_JOURNAL);
        let mut m = meta();
        m.session = MetaSession::N3;
        let inst = crate::protocol::tests_support::synthetic_verified();
        let mut tick = 0u64;
        let report = n3_replay(
            &path,
            &m,
            &p2_row_context(),
            &inst,
            // Small enough that a real synthetic sentinel clears the P8 floor,
            // so the replay produces OK rows rather than LOST ones.
            1e-9,
            &mut |_| {
                tick += 1_000;
                Ok(tick)
            },
            &mut || Ok(0.42),
        )
        .expect("the replay must succeed");

        assert_eq!(report.rows, 15, "one session is fifteen measurements");
        assert_eq!(report.sentinel_executions, 30, "two per pair");
        assert_eq!(report.sweeps, expected_session_sweeps());
        assert!(report.metadata_is_n3);
        assert!(report.coordinates_frozen);

        // §5.2: the file is recognisable as a COMPLETED session.
        let bytes = std::fs::read(&path).unwrap();
        let read = crate::journal::parse_journal(&bytes);
        assert!(
            matches!(read.verdict, crate::journal::ReadVerdict::Valid),
            "{:?}",
            read.verdict
        );
        use crate::journal::Status;
        let statuses: Vec<Status> = read.rows.iter().map(|r| r.status).collect();
        assert_eq!(statuses.first(), Some(&Status::SessionOpen));
        assert_eq!(statuses.last(), Some(&Status::SessionCloseCompleted));
        assert_eq!(statuses.len(), 17, "open + fifteen + close");
        assert_eq!(
            statuses
                .iter()
                .filter(|s| **s == Status::SessionOpen)
                .count(),
            1
        );
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("#rc021_meta\tsession\t\"N3\""));

        // Every measurement completed, so §5.1 makes them OK — and each OK row
        // carries both wall times and the spread.
        let measurements: Vec<&crate::journal::Row> = read
            .rows
            .iter()
            .filter(|r| matches!(r.status, Status::Ok | Status::Lost))
            .collect();
        assert_eq!(measurements.len(), 15);
        assert!(
            measurements.iter().all(|r| r.status == Status::Ok),
            "a completed pair is OK, never LOST"
        );
        for r in &measurements {
            assert!(r.sentinel_first_ms.is_some(), "wall time discarded");
            assert!(r.sentinel_last_ms.is_some(), "wall time discarded");
            assert!(r.paired_spread.is_some(), "spread discarded");
            assert!(r.sentinel_first_ms.unwrap() > 0.0);
            assert_eq!(r.session, 0);
            assert_eq!(r.measurement_index, None);
            assert!(r.block.is_some());
        }
        // A → B → C, five blocks each.
        let phases: Vec<Option<crate::journal::Phase>> =
            measurements.iter().map(|r| r.phase).collect();
        use crate::journal::Phase;
        assert_eq!(
            phases,
            [
                [Some(Phase::A); 5].as_slice(),
                [Some(Phase::B); 5].as_slice(),
                [Some(Phase::C); 5].as_slice()
            ]
            .concat()
        );
    }

    /// §5.1: `LOST` appears only from a real lost classification — here, a
    /// timer resolution so coarse that every sentinel is below the §6 P8 floor.
    #[test]
    fn the_n3_replay_writes_lost_only_from_a_real_classification() {
        let d = TempDir::new("n3lost");
        let path = d.path().join(N3_JOURNAL);
        let mut m = meta();
        m.session = MetaSession::N3;
        let inst = crate::protocol::tests_support::synthetic_verified();
        let mut tick = 0u64;
        let report = n3_replay(
            &path,
            &m,
            &p2_row_context(),
            &inst,
            1.0e6,
            &mut |_| {
                tick += 1_000;
                Ok(tick)
            },
            &mut || Ok(0.42),
        )
        .expect("the replay still completes");
        assert_eq!(report.rows, 15);

        let read = crate::journal::parse_journal(&std::fs::read(&path).unwrap());
        use crate::journal::Status;
        let measurements: Vec<&crate::journal::Row> = read
            .rows
            .iter()
            .filter(|r| matches!(r.status, Status::Ok | Status::Lost))
            .collect();
        assert!(
            measurements.iter().all(|r| r.status == Status::Lost),
            "below the P8 floor every measurement is LOST"
        );
        for r in &measurements {
            assert!(r.paired_spread.is_none(), "a LOST row carries no spread");
            assert!(r.reason.contains("BelowResolutionFloor"), "{}", r.reason);
        }
        // The lifecycle still frames them, so §5.2 can classify the file.
        assert_eq!(
            read.rows.first().map(|r| r.status),
            Some(Status::SessionOpen)
        );
        assert_eq!(
            read.rows.last().map(|r| r.status),
            Some(Status::SessionCloseCompleted)
        );
    }

    /// The replay refuses a journal that is not declared N3.
    #[test]
    fn the_n3_replay_requires_n3_metadata() {
        let d = TempDir::new("n3meta");
        let inst = crate::protocol::tests_support::synthetic_verified();
        let err = n3_replay(
            &d.path().join(N3_JOURNAL),
            &meta(), // CONTROL
            &p2_row_context(),
            &inst,
            1e-9,
            &mut |_| Ok(1),
            &mut || Ok(0.0),
        )
        .unwrap_err();
        assert!(err.contains("must be N3"), "{err}");
        assert!(!d.path().join(N3_JOURNAL).exists(), "nothing was created");
    }

    /// `/proc/loadavg` is parsed, not invented.
    #[test]
    fn the_load_average_is_read_from_its_source() {
        let d = TempDir::new("loadavg");
        let p = d.path().join("loadavg");
        std::fs::write(
            &p,
            "0.52 0.31 0.20 1/512 12345
",
        )
        .unwrap();
        assert_eq!(read_load_avg_1min(&p).unwrap(), 0.52);
        for bad in [
            "",
            "notanumber 1 2
",
            "-1.0 1 2
",
            "nan 1 2
",
        ] {
            std::fs::write(&p, bad).unwrap();
            assert!(read_load_avg_1min(&p).is_err(), "{bad:?}");
        }
        assert!(read_load_avg_1min(&d.path().join("absent")).is_err());
    }

    /// The P3 completeness guard must reject a sample that is the **right
    /// length** yet the wrong set — one name duplicated, another dropped. A
    /// length check paired with a de-duplicated comparison accepts exactly
    /// that, which would make the guard vacuous while still looking strict.
    #[test]
    fn the_frozen_inventory_guard_rejects_a_same_length_wrong_content_sample() {
        let frozen = ["A", "B", "C"];
        assert!(sample_is_frozen_inventory(&["C", "A", "B"], &frozen));
        assert!(
            !sample_is_frozen_inventory(&["A", "B", "B"], &frozen),
            "a duplicate hiding a missing name is not the inventory"
        );
        assert!(!sample_is_frozen_inventory(&["A", "B"], &frozen));
        assert!(!sample_is_frozen_inventory(&["A", "B", "C", "D"], &frozen));

        // And the two real samples are the frozen inventories.
        let fin: Vec<&str> = sample_finalize_outcomes()
            .iter()
            .map(|o| o.variant_name())
            .collect();
        assert!(sample_is_frozen_inventory(
            &fin,
            &crate::decision::FINALIZE_VARIANTS
        ));
        let ver: Vec<&str> = sample_verify_outcomes()
            .iter()
            .map(|o| o.variant_name())
            .collect();
        assert!(sample_is_frozen_inventory(
            &ver,
            &crate::decision::VERIFY_VARIANTS
        ));
    }
}
