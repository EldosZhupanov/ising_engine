//! RC-021 journal and row codec — Amendment 2 §C1, §C8, §C9, §C10.1, §C11.4.
//!
//! The journal is the evidence of record. Everything here exists so that two
//! independent writers produce identical bytes and an independent reader can
//! recompute every quantity a closure asserts.
//!
//! Three rules shape the whole module and are not local choices:
//!
//! * **Shortest round-trip serialisation only** (§C8.1). A fixed number of
//!   decimals can round a `paired_spread` just above the `0.09` bound down onto
//!   it and turn a failure into a pass; `f64::to_string` preserves the bits.
//! * **Nothing is ever repaired.** A truncated final line becomes a *logical*
//!   `LOST` in memory; the bytes on disk are untouched (§C8.3).
//! * **No overwrite, no rename, no temporary file** (§C10.1). Paths are claimed
//!   with `create_new` and every row is durable before the next one begins.

#![allow(dead_code)] // The consuming layers arrive in later commits of the §12 plan.

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::Path;

// ===================================================================== SCHEMA

/// §C8.2 field 1 — the frozen schema literal.
pub const SCHEMA_VERSION: &str = "rc021/1";

/// §C1: exactly twenty-four columns, in this order. Never reordered, never
/// extended.
pub const COLUMNS: [&str; 24] = [
    "schema_version",
    "run_uuid",
    "repo_commit",
    "prereg_commit",
    "host_fingerprint",
    "phase",
    "session",
    "block",
    "measurement_index",
    "monotonic_offset_ms",
    "sentinel_first_ms",
    "sentinel_last_ms",
    "paired_spread",
    "timer_resolution_ms",
    "load_avg_start",
    "load_avg_end",
    "cpu_set",
    "thread_count",
    "diag_cpu_time",
    "diag_ctx_switches",
    "diag_freq",
    "diag_flags",
    "status",
    "reason",
];

/// The literal written for an absent value.
pub const NA: &str = "NA";

/// The frozen header line, tab separated and line-feed terminated.
pub fn header_line() -> String {
    let mut s = COLUMNS.join("\t");
    s.push('\n');
    s
}

// ====================================================================== ENUMS

/// §C8.2 field 6. `NA` is represented by `Option::None`, never by a variant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    A,
    B,
    C,
}

impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Phase::A => "A",
            Phase::B => "B",
            Phase::C => "C",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s {
            "A" => Some(Phase::A),
            "B" => Some(Phase::B),
            "C" => Some(Phase::C),
            _ => None,
        }
    }
}

/// §C8.2 field 23 — the closed status domain of §5.1 as extended by §C8.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Ok,
    Lost,
    SessionOpen,
    SessionCloseCompleted,
    SessionCloseAborted,
    ExternalCause,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "OK",
            Status::Lost => "LOST",
            Status::SessionOpen => "SESSION-OPEN",
            Status::SessionCloseCompleted => "SESSION-CLOSE-COMPLETED",
            Status::SessionCloseAborted => "SESSION-CLOSE-ABORTED",
            Status::ExternalCause => "EXTERNAL-CAUSE",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s {
            "OK" => Some(Status::Ok),
            "LOST" => Some(Status::Lost),
            "SESSION-OPEN" => Some(Status::SessionOpen),
            "SESSION-CLOSE-COMPLETED" => Some(Status::SessionCloseCompleted),
            "SESSION-CLOSE-ABORTED" => Some(Status::SessionCloseAborted),
            "EXTERNAL-CAUSE" => Some(Status::ExternalCause),
            _ => None,
        }
    }
    /// True for the four rows that are not measurements (§C8.4).
    pub fn is_lifecycle(self) -> bool {
        matches!(
            self,
            Status::SessionOpen
                | Status::SessionCloseCompleted
                | Status::SessionCloseAborted
                | Status::ExternalCause
        )
    }
}

/// §C9 metadata key `session` — closed domain `1..6 | "P2" | "N3" | "CONTROL"`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MetaSession {
    Qualification(u8),
    P2,
    N3,
    Control,
}

impl MetaSession {
    /// The JSON value: an integer for a qualification session, a string for a
    /// control journal.
    fn to_json(self) -> String {
        match self {
            MetaSession::Qualification(n) => n.to_string(),
            MetaSession::P2 => "\"P2\"".to_string(),
            MetaSession::N3 => "\"N3\"".to_string(),
            MetaSession::Control => "\"CONTROL\"".to_string(),
        }
    }
    fn parse(v: &str) -> Option<Self> {
        match v {
            "\"P2\"" => Some(MetaSession::P2),
            "\"N3\"" => Some(MetaSession::N3),
            "\"CONTROL\"" => Some(MetaSession::Control),
            other => other
                .parse::<u8>()
                .ok()
                .filter(|n| (1..=6).contains(n))
                .map(MetaSession::Qualification),
        }
    }
    /// §C8.2 field 7: `0` for every control journal, `1..6` otherwise.
    pub fn column_value(self) -> u8 {
        match self {
            MetaSession::Qualification(n) => n,
            _ => 0,
        }
    }
}

// ================================================================ CODEC ERROR

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CodecError {
    /// §C8.1: `NaN` and `±inf` are forbidden and yield `JOURNAL-INVALID`.
    NonFinite(&'static str),
    FieldCount {
        expected: usize,
        got: usize,
    },
    BadField {
        column: &'static str,
        value: String,
    },
    BadJsonString(String),
    /// A tab in a field that must not contain one.
    RawTab(&'static str),
    /// A §C8.2/§C8.3/§C8.4/§C11.4 invariant the row violates.
    Invariant(&'static str),
    /// §C7: a diag value is `NA` if and only if its missing bit is set.
    DiagMismatch(&'static str),
    /// §C6.1: a channel the journal declared unavailable carries a value.
    DiagUnavailable(&'static str),
    /// A repeated per-row field disagrees with the journal metadata.
    MetaMismatch(&'static str),
}

fn hex_field(s: &str, n: usize, column: &'static str) -> Result<(), CodecError> {
    if s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(CodecError::BadField {
            column,
            value: s.to_string(),
        })
    }
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::NonFinite(c) => write!(f, "non-finite f64 in column {c}"),
            CodecError::FieldCount { expected, got } => {
                write!(f, "expected {expected} fields, found {got}")
            }
            CodecError::BadField { column, value } => {
                write!(f, "column {column} rejects value {value:?}")
            }
            CodecError::BadJsonString(s) => write!(f, "malformed JSON string {s:?}"),
            CodecError::RawTab(c) => write!(f, "raw tab in column {c}"),
            CodecError::Invariant(m) => write!(f, "invariant violated: {m}"),
            CodecError::DiagMismatch(c) => {
                write!(f, "{c} disagrees with its diag_flags missing bit")
            }
            CodecError::DiagUnavailable(c) => {
                write!(
                    f,
                    "{c} carries a value although the journal declared it unavailable"
                )
            }
            CodecError::MetaMismatch(c) => write!(f, "{c} disagrees with the journal metadata"),
        }
    }
}

// =============================================================== JSON STRINGS

/// §C8.2 field 24: a JSON string per RFC 8259, quotes included.
///
/// Delegated to `serde_json`, which is already a dependency. An earlier
/// hand-written parser handled each `\uXXXX` in isolation and therefore
/// **rejected valid surrogate pairs** such as `"\uD834\uDD1E"` while claiming
/// to accept a full JSON string literal. Writing a partial JSON parser is the
/// defect; the fix is not to write one.
pub fn json_string_encode(s: &str) -> String {
    // Infallible for `str`: serialising a string cannot fail.
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Inverse of [`json_string_encode`]. Rejects anything that is not exactly one
/// complete, well-formed JSON string literal — trailing bytes included.
pub fn json_string_decode(field: &str) -> Result<String, CodecError> {
    serde_json::from_str::<String>(field).map_err(|_| CodecError::BadJsonString(field.to_string()))
}

// ============================================================ FIELD CODECS

/// §C8.1: the canonical shortest decimal form. `f64::to_string` is exactly the
/// shortest representation that round-trips, which is the property the bound
/// comparison depends on.
fn f64_encode(v: f64, column: &'static str) -> Result<String, CodecError> {
    if !v.is_finite() {
        return Err(CodecError::NonFinite(column));
    }
    Ok(v.to_string())
}

fn f64_decode(s: &str, column: &'static str) -> Result<f64, CodecError> {
    let v: f64 = s.parse().map_err(|_| CodecError::BadField {
        column,
        value: s.to_string(),
    })?;
    if !v.is_finite() {
        return Err(CodecError::NonFinite(column));
    }
    Ok(v)
}

fn opt_f64_encode(v: Option<f64>, column: &'static str) -> Result<String, CodecError> {
    match v {
        None => Ok(NA.to_string()),
        Some(x) => f64_encode(x, column),
    }
}

fn opt_f64_decode(s: &str, column: &'static str) -> Result<Option<f64>, CodecError> {
    if s == NA {
        Ok(None)
    } else {
        f64_decode(s, column).map(Some)
    }
}

fn opt_u64_encode(v: Option<u64>) -> String {
    match v {
        None => NA.to_string(),
        Some(x) => x.to_string(),
    }
}

fn opt_u64_decode(s: &str, column: &'static str) -> Result<Option<u64>, CodecError> {
    if s == NA {
        return Ok(None);
    }
    s.parse().map(Some).map_err(|_| CodecError::BadField {
        column,
        value: s.to_string(),
    })
}

// ========================================================================= ROW

/// One physical journal row: exactly the twenty-four fields of §C8.2.
#[derive(Clone, PartialEq, Debug)]
pub struct Row {
    pub schema_version: String,
    pub run_uuid: String,
    pub repo_commit: String,
    pub prereg_commit: String,
    pub host_fingerprint: String,
    pub phase: Option<Phase>,
    pub session: u8,
    pub block: Option<u8>,
    pub measurement_index: Option<u32>,
    pub monotonic_offset_ms: u64,
    pub sentinel_first_ms: Option<f64>,
    pub sentinel_last_ms: Option<f64>,
    pub paired_spread: Option<f64>,
    pub timer_resolution_ms: f64,
    pub load_avg_start: Option<f64>,
    pub load_avg_end: Option<f64>,
    pub cpu_set: String,
    pub thread_count: u32,
    pub diag_cpu_time: Option<u64>,
    pub diag_ctx_switches: Option<u64>,
    pub diag_freq: Option<u64>,
    pub diag_flags: u8,
    pub status: Status,
    pub reason: String,
}

impl Row {
    /// The single validation the writer and the reader both apply (§C8.2,
    /// §C8.3, §C8.4, §C7, §C11.4). An earlier version checked syntax only, so a
    /// row the reader would have to call `JOURNAL-INVALID` could still be
    /// written.
    ///
    /// `meta` is `Some` when the journal's own §C9 block validated, and the
    /// repeated per-row provenance is then cross-checked against it.
    pub fn validate(&self, meta: Option<&Metadata>) -> Result<(), CodecError> {
        let dom = |m: &'static str| CodecError::Invariant(m);

        // ---- field syntax and domains (§C8.2) ----
        if self.schema_version != SCHEMA_VERSION {
            return Err(dom("schema_version must be rc021/1"));
        }
        hex_field(&self.run_uuid, 32, "run_uuid")?;
        hex_field(&self.repo_commit, 40, "repo_commit")?;
        hex_field(&self.prereg_commit, 40, "prereg_commit")?;
        hex_field(&self.host_fingerprint, 64, "host_fingerprint")?;
        if self.session > 6 {
            return Err(dom("session must be 0..=6"));
        }
        if self.diag_flags > 7 {
            return Err(dom("diag_flags must be 0..=7"));
        }
        if let Some(b) = self.block {
            if !(1..=5).contains(&b) {
                return Err(dom("block must be 1..=5"));
            }
        }
        if let Some(m) = self.measurement_index {
            if !(1..=90).contains(&m) {
                return Err(dom("measurement_index must be 1..=90"));
            }
        }
        // Plain TSV fields may not carry a separator or break the line.
        for (v, name) in [
            (&self.schema_version, "schema_version"),
            (&self.run_uuid, "run_uuid"),
            (&self.repo_commit, "repo_commit"),
            (&self.prereg_commit, "prereg_commit"),
            (&self.host_fingerprint, "host_fingerprint"),
            (&self.cpu_set, "cpu_set"),
        ] {
            if v.contains('\t') || v.contains('\n') || v.contains('\r') {
                return Err(CodecError::RawTab(match name {
                    "cpu_set" => "cpu_set",
                    _ => "provenance field",
                }));
            }
        }
        for (v, n) in [
            (self.sentinel_first_ms, "sentinel_first_ms"),
            (self.sentinel_last_ms, "sentinel_last_ms"),
            (self.paired_spread, "paired_spread"),
            (self.load_avg_start, "load_avg_start"),
            (self.load_avg_end, "load_avg_end"),
        ] {
            if let Some(x) = v {
                if !x.is_finite() {
                    return Err(CodecError::NonFinite(n));
                }
            }
        }
        if !self.timer_resolution_ms.is_finite() {
            return Err(CodecError::NonFinite("timer_resolution_ms"));
        }

        // ---- §C7: a diag value is NA if and only if its missing bit is set ----
        for (val, bit, name) in [
            (self.diag_cpu_time, 1u8, "diag_cpu_time"),
            (self.diag_ctx_switches, 2, "diag_ctx_switches"),
            (self.diag_freq, 4, "diag_freq"),
        ] {
            let missing_bit = self.diag_flags & bit != 0;
            if val.is_none() != missing_bit {
                return Err(CodecError::DiagMismatch(name));
            }
        }

        // ---- §C11.4: the global index belongs to qualification sessions ----
        if self.session == 0 && self.measurement_index.is_some() {
            return Err(dom("a control row must carry measurement_index = NA"));
        }

        // ---- §C8.3 / §C8.4: shape by status ----
        let no_measure = self.sentinel_first_ms.is_none()
            && self.sentinel_last_ms.is_none()
            && self.paired_spread.is_none();
        let no_diag = self.diag_cpu_time.is_none()
            && self.diag_ctx_switches.is_none()
            && self.diag_freq.is_none();
        match self.status {
            Status::SessionOpen | Status::ExternalCause => {
                if self.phase.is_some() || self.block.is_some() || self.measurement_index.is_some()
                {
                    return Err(dom("a lifecycle row carries no coordinates"));
                }
                // §C8.4: EXTERNAL-CAUSE exists in order to name the cause.
                if self.status == Status::ExternalCause && self.reason.is_empty() {
                    return Err(dom("EXTERNAL-CAUSE carries its cause"));
                }
                if !no_measure || !no_diag || self.diag_flags != 7 {
                    return Err(dom("a lifecycle row carries no measurement or diagnostics"));
                }
                if self.load_avg_start.is_none() || self.load_avg_end.is_some() {
                    return Err(dom("SESSION-OPEN/EXTERNAL-CAUSE: start present, end NA"));
                }
            }
            Status::SessionCloseCompleted | Status::SessionCloseAborted => {
                if self.phase.is_some() || self.block.is_some() || self.measurement_index.is_some()
                {
                    return Err(dom("a lifecycle row carries no coordinates"));
                }
                if !no_measure || !no_diag || self.diag_flags != 7 {
                    return Err(dom("a lifecycle row carries no measurement or diagnostics"));
                }
                if self.load_avg_start.is_some() || self.load_avg_end.is_none() {
                    return Err(dom("SESSION-CLOSE-*: end present, start NA"));
                }
            }
            Status::Lost => {
                if self.phase.is_none() || self.block.is_none() {
                    return Err(dom("a physical LOST keeps phase and block"));
                }
                // §C8.3: fields 1-10 carry values, the single exception being a
                // control row's measurement_index. An earlier version enforced
                // that only for OK, so a qualification LOST could silently drop
                // the global index.
                if self.session != 0 && self.measurement_index.is_none() {
                    return Err(dom("a qualification LOST carries measurement_index"));
                }
                if self.reason.is_empty() {
                    return Err(dom("a physical LOST carries its reason"));
                }
                if !no_measure || !no_diag || self.diag_flags != 7 {
                    return Err(dom("a LOST row carries no measurement or diagnostics"));
                }
                // §C8.3: both load averages are NA.
                if self.load_avg_start.is_some() || self.load_avg_end.is_some() {
                    return Err(dom("a LOST row has both load averages NA"));
                }
            }
            Status::Ok => {
                if self.phase.is_none() || self.block.is_none() {
                    return Err(dom("an OK row carries phase and block"));
                }
                if self.sentinel_first_ms.is_none()
                    || self.sentinel_last_ms.is_none()
                    || self.paired_spread.is_none()
                {
                    return Err(dom("an OK row carries both sentinels and the spread"));
                }
                if self.load_avg_start.is_none() || self.load_avg_end.is_none() {
                    return Err(dom("an OK row carries both load averages"));
                }
                if self.session != 0 && self.measurement_index.is_none() {
                    return Err(dom("a qualification OK row carries measurement_index"));
                }
            }
        }

        // ---- cross-check against the journal's own metadata ----
        if let Some(m) = meta {
            let mismatch = |w: &'static str| Err(CodecError::MetaMismatch(w));
            if self.run_uuid != m.run_uuid {
                return mismatch("run_uuid");
            }
            if self.repo_commit != m.repo_commit {
                return mismatch("repo_commit");
            }
            if self.prereg_commit != m.prereg_commit {
                return mismatch("prereg_commit");
            }
            if self.host_fingerprint != m.host_fingerprint {
                return mismatch("host_fingerprint");
            }
            if self.cpu_set != m.cpu_set {
                return mismatch("cpu_set");
            }
            if self.thread_count != m.thread_count {
                return mismatch("thread_count");
            }
            if self.timer_resolution_ms.to_bits() != m.timer_resolution_ms.to_bits() {
                return mismatch("timer_resolution_ms");
            }
            if self.session != m.session.column_value() {
                return mismatch("session");
            }
            // §C6.1: a channel the journal declared unavailable must be NA in
            // every row, with its missing bit set.
            let a = m.diag_availability;
            for (avail, val, bit, name) in [
                (a.cpu_time, self.diag_cpu_time, 1u8, "diag_cpu_time"),
                (
                    a.ctx_switches,
                    self.diag_ctx_switches,
                    2,
                    "diag_ctx_switches",
                ),
                (a.freq, self.diag_freq, 4, "diag_freq"),
            ] {
                if !avail && (val.is_some() || self.diag_flags & bit == 0) {
                    return Err(CodecError::DiagUnavailable(name));
                }
            }
        }
        Ok(())
    }

    /// Serialise to one tab-separated, line-feed-terminated line, validating
    /// against the journal's metadata when one is available.
    pub fn encode_with(&self, meta: Option<&Metadata>) -> Result<String, CodecError> {
        self.validate(meta)?;
        if self.cpu_set.contains('\t') {
            return Err(CodecError::RawTab("cpu_set"));
        }
        if self.diag_flags > 7 {
            return Err(CodecError::BadField {
                column: "diag_flags",
                value: self.diag_flags.to_string(),
            });
        }
        let fields: Vec<String> = vec![
            self.schema_version.clone(),
            self.run_uuid.clone(),
            self.repo_commit.clone(),
            self.prereg_commit.clone(),
            self.host_fingerprint.clone(),
            self.phase
                .map(|p| p.as_str().to_string())
                .unwrap_or_else(|| NA.to_string()),
            self.session.to_string(),
            self.block
                .map(|b| b.to_string())
                .unwrap_or_else(|| NA.to_string()),
            self.measurement_index
                .map(|m| m.to_string())
                .unwrap_or_else(|| NA.to_string()),
            self.monotonic_offset_ms.to_string(),
            opt_f64_encode(self.sentinel_first_ms, "sentinel_first_ms")?,
            opt_f64_encode(self.sentinel_last_ms, "sentinel_last_ms")?,
            opt_f64_encode(self.paired_spread, "paired_spread")?,
            f64_encode(self.timer_resolution_ms, "timer_resolution_ms")?,
            opt_f64_encode(self.load_avg_start, "load_avg_start")?,
            opt_f64_encode(self.load_avg_end, "load_avg_end")?,
            self.cpu_set.clone(),
            self.thread_count.to_string(),
            opt_u64_encode(self.diag_cpu_time),
            opt_u64_encode(self.diag_ctx_switches),
            opt_u64_encode(self.diag_freq),
            self.diag_flags.to_string(),
            self.status.as_str().to_string(),
            json_string_encode(&self.reason),
        ];
        debug_assert_eq!(fields.len(), COLUMNS.len());
        let mut line = fields.join("\t");
        line.push('\n');
        Ok(line)
    }

    /// Serialise without a metadata cross-check. Only for contexts with no
    /// journal; [`Journal::append`] always supplies one.
    pub fn encode(&self) -> Result<String, CodecError> {
        self.encode_with(None)
    }

    /// Parse one line, with or without its trailing line feed.
    pub fn parse(line: &str) -> Result<Row, CodecError> {
        let line = line.strip_suffix('\n').unwrap_or(line);
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != COLUMNS.len() {
            return Err(CodecError::FieldCount {
                expected: COLUMNS.len(),
                got: f.len(),
            });
        }
        let int = |s: &str, column: &'static str| -> Result<u64, CodecError> {
            s.parse::<u64>().map_err(|_| CodecError::BadField {
                column,
                value: s.to_string(),
            })
        };
        let phase = if f[5] == NA {
            None
        } else {
            Some(Phase::parse(f[5]).ok_or_else(|| CodecError::BadField {
                column: "phase",
                value: f[5].to_string(),
            })?)
        };
        // Checked, not truncating: `as u8` turned 256 into 0 and accepted it.
        let session = u8::try_from(int(f[6], "session")?).map_err(|_| CodecError::BadField {
            column: "session",
            value: f[6].to_string(),
        })?;
        if session > 6 {
            return Err(CodecError::BadField {
                column: "session",
                value: f[6].to_string(),
            });
        }
        let block = opt_u64_decode(f[7], "block")?
            .map(|b| {
                if (1..=5).contains(&b) {
                    Ok(b as u8) // range-checked above, so the cast cannot wrap
                } else {
                    Err(CodecError::BadField {
                        column: "block",
                        value: f[7].to_string(),
                    })
                }
            })
            .transpose()?;
        let measurement_index = opt_u64_decode(f[8], "measurement_index")?
            .map(|m| {
                if (1..=90).contains(&m) {
                    Ok(m as u32) // range-checked above, so the cast cannot wrap
                } else {
                    Err(CodecError::BadField {
                        column: "measurement_index",
                        value: f[8].to_string(),
                    })
                }
            })
            .transpose()?;
        let diag_flags =
            u8::try_from(int(f[21], "diag_flags")?).map_err(|_| CodecError::BadField {
                column: "diag_flags",
                value: f[21].to_string(),
            })?;
        if diag_flags > 7 {
            return Err(CodecError::BadField {
                column: "diag_flags",
                value: f[21].to_string(),
            });
        }
        Ok(Row {
            schema_version: f[0].to_string(),
            run_uuid: f[1].to_string(),
            repo_commit: f[2].to_string(),
            prereg_commit: f[3].to_string(),
            host_fingerprint: f[4].to_string(),
            phase,
            session,
            block,
            measurement_index,
            monotonic_offset_ms: int(f[9], "monotonic_offset_ms")?,
            sentinel_first_ms: opt_f64_decode(f[10], "sentinel_first_ms")?,
            sentinel_last_ms: opt_f64_decode(f[11], "sentinel_last_ms")?,
            paired_spread: opt_f64_decode(f[12], "paired_spread")?,
            timer_resolution_ms: f64_decode(f[13], "timer_resolution_ms")?,
            load_avg_start: opt_f64_decode(f[14], "load_avg_start")?,
            load_avg_end: opt_f64_decode(f[15], "load_avg_end")?,
            cpu_set: f[16].to_string(),
            thread_count: u32::try_from(int(f[17], "thread_count")?).map_err(|_| {
                CodecError::BadField {
                    column: "thread_count",
                    value: f[17].to_string(),
                }
            })?,
            diag_cpu_time: opt_u64_decode(f[18], "diag_cpu_time")?,
            diag_ctx_switches: opt_u64_decode(f[19], "diag_ctx_switches")?,
            diag_freq: opt_u64_decode(f[20], "diag_freq")?,
            diag_flags,
            status: Status::parse(f[22]).ok_or_else(|| CodecError::BadField {
                column: "status",
                value: f[22].to_string(),
            })?,
            reason: json_string_decode(f[23])?,
        })
    }
}

// ============================================================ NA PROFILES

/// The provenance fields every row repeats (§C8.2 fields 1–5, 14, 17, 18).
#[derive(Clone, Debug)]
pub struct RowContext {
    pub run_uuid: String,
    pub repo_commit: String,
    pub prereg_commit: String,
    pub host_fingerprint: String,
    pub timer_resolution_ms: f64,
    pub cpu_set: String,
    pub thread_count: u32,
}

impl RowContext {
    fn base(&self, status: Status, session: u8, monotonic_offset_ms: u64, reason: &str) -> Row {
        Row {
            schema_version: SCHEMA_VERSION.to_string(),
            run_uuid: self.run_uuid.clone(),
            repo_commit: self.repo_commit.clone(),
            prereg_commit: self.prereg_commit.clone(),
            host_fingerprint: self.host_fingerprint.clone(),
            phase: None,
            session,
            block: None,
            measurement_index: None,
            monotonic_offset_ms,
            sentinel_first_ms: None,
            sentinel_last_ms: None,
            paired_spread: None,
            timer_resolution_ms: self.timer_resolution_ms,
            load_avg_start: None,
            load_avg_end: None,
            cpu_set: self.cpu_set.clone(),
            thread_count: self.thread_count,
            diag_cpu_time: None,
            diag_ctx_switches: None,
            diag_freq: None,
            // §C8.4: lifecycle rows sample no diagnostics, so every missing bit
            // is set.
            diag_flags: 7,
            status,
            reason: reason.to_string(),
        }
    }

    /// §C8.4 `SESSION-OPEN`: `load_avg_start` carries a value, `load_avg_end`
    /// does not.
    pub fn session_open(&self, session: u8, offset: u64, load_avg_start: f64) -> Row {
        let mut r = self.base(Status::SessionOpen, session, offset, "");
        r.load_avg_start = Some(load_avg_start);
        r
    }

    /// §C8.4 `SESSION-CLOSE-*`: the mirror image — `load_avg_end` carries a
    /// value, `load_avg_start` does not.
    pub fn session_close(
        &self,
        session: u8,
        offset: u64,
        completed: bool,
        load_avg_end: f64,
    ) -> Row {
        let status = if completed {
            Status::SessionCloseCompleted
        } else {
            Status::SessionCloseAborted
        };
        let mut r = self.base(status, session, offset, "");
        r.load_avg_end = Some(load_avg_end);
        r
    }

    /// §C8.4 `EXTERNAL-CAUSE`: shaped like `SESSION-OPEN`, but `reason` carries
    /// the cause and must be fsynced *before* the abort it justifies (§C12.3).
    pub fn external_cause(
        &self,
        session: u8,
        offset: u64,
        load_avg_start: f64,
        cause: &str,
    ) -> Row {
        let mut r = self.base(Status::ExternalCause, session, offset, cause);
        r.load_avg_start = Some(load_avg_start);
        r
    }

    /// §C8.3 physical `LOST`: coordinates are present, **both** load averages
    /// are `NA`, and every diagnostic is missing.
    pub fn lost(
        &self,
        session: u8,
        phase: Phase,
        block: u8,
        measurement_index: Option<u32>,
        offset: u64,
        reason: &str,
    ) -> Row {
        let mut r = self.base(Status::Lost, session, offset, reason);
        r.phase = Some(phase);
        r.block = Some(block);
        r.measurement_index = measurement_index;
        r
    }
}

// =================================================================== METADATA

/// §C5: the only permitted value of the `cpu_time_unit` key.
pub const CPU_TIME_UNIT: &str = "linux_clock_ticks";

/// Why a metadata block is not §C9-conforming. Every variant is
/// `JOURNAL-INVALID` for a reader and a refusal for a writer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MetaError {
    KeyCount {
        expected: usize,
        got: usize,
    },
    KeyOrder {
        position: usize,
        expected: &'static str,
        got: String,
    },
    Type {
        key: &'static str,
        want: &'static str,
    },
    Domain(&'static str),
    Hex {
        key: &'static str,
        len: usize,
    },
    Empty(&'static str),
    RawControl(&'static str),
    NotCanonical {
        key: &'static str,
        want: String,
        got: String,
    },
}

impl std::fmt::Display for MetaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetaError::KeyCount { expected, got } => {
                write!(f, "expected {expected} metadata keys, found {got}")
            }
            MetaError::KeyOrder {
                position,
                expected,
                got,
            } => {
                write!(f, "metadata key {position} must be {expected}, found {got}")
            }
            MetaError::Type { key, want } => write!(f, "metadata {key} must be a {want}"),
            MetaError::Domain(m) => write!(f, "metadata domain violation: {m}"),
            MetaError::Hex { key, len } => {
                write!(f, "metadata {key} must be {len} lower-case hex characters")
            }
            MetaError::Empty(k) => write!(f, "metadata {k} must not be empty"),
            MetaError::RawControl(k) => write!(f, "metadata {k} carries a raw control character"),
            MetaError::NotCanonical { key, want, got } => {
                write!(f, "metadata {key} is not canonical: want {want}, got {got}")
            }
        }
    }
}

/// §C9 — the seventeen mandatory keys, in this exact order.
pub const META_KEYS: [&str; 17] = [
    "schema_version",
    "run_uuid",
    "boot_id",
    "run_start_uptime_ms",
    "repo_commit",
    "prereg_commit",
    "amendment_commits",
    "instrument_birth_commit",
    "host_fingerprint",
    "cpu_set",
    "thread_count",
    "timer_resolution_ms",
    "cpu_time_unit",
    "command_line",
    "utc_start",
    "session",
    "diag_availability",
];

/// The three diagnostic channels, in the fixed order of §C9.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagAvailability {
    pub cpu_time: bool,
    pub ctx_switches: bool,
    pub freq: bool,
}

impl DiagAvailability {
    /// §C9: exactly three booleans, in this order. The canonical comparison in
    /// [`Metadata::parse`] enforces the order; this enforces the shape.
    fn parse(raw: &str) -> Result<Self, MetaError> {
        let want = MetaError::Type {
            key: "diag_availability",
            want: "object of exactly cpu_time, ctx_switches, freq booleans",
        };
        let v: serde_json::Value = serde_json::from_str(raw).map_err(|_| want.clone())?;
        let o = v.as_object().ok_or_else(|| want.clone())?;
        if o.len() != 3 {
            return Err(want);
        }
        let b = |k: &str| -> Result<bool, MetaError> {
            o.get(k).and_then(|x| x.as_bool()).ok_or(MetaError::Type {
                key: "diag_availability",
                want: "boolean member",
            })
        };
        Ok(DiagAvailability {
            cpu_time: b("cpu_time")?,
            ctx_switches: b("ctx_switches")?,
            freq: b("freq")?,
        })
    }

    fn to_json(self) -> String {
        format!(
            "{{\"cpu_time\":{},\"ctx_switches\":{},\"freq\":{}}}",
            self.cpu_time, self.ctx_switches, self.freq
        )
    }
}

/// The metadata block that precedes the header in every journal.
#[derive(Clone, Debug)]
pub struct Metadata {
    pub run_uuid: String,
    pub boot_id: String,
    pub run_start_uptime_ms: u64,
    pub repo_commit: String,
    pub prereg_commit: String,
    pub amendment_commits: Vec<String>,
    pub instrument_birth_commit: String,
    pub host_fingerprint: String,
    pub cpu_set: String,
    pub thread_count: u32,
    pub timer_resolution_ms: f64,
    pub cpu_time_unit: String,
    pub command_line: Vec<String>,
    pub utc_start: String,
    pub session: MetaSession,
    pub diag_availability: DiagAvailability,
}

impl Metadata {
    /// Reject a `Metadata` that violates §C9 **before** any byte is written.
    /// A writer that can emit a journal a reader must call `JOURNAL-INVALID` is
    /// itself the defect.
    pub fn validate(&self) -> Result<(), MetaError> {
        hexlen(&self.run_uuid, 32, "run_uuid")?;
        nonempty(&self.boot_id, "boot_id")?;
        hexlen(&self.repo_commit, 40, "repo_commit")?;
        hexlen(&self.prereg_commit, 40, "prereg_commit")?;
        hexlen(&self.instrument_birth_commit, 40, "instrument_birth_commit")?;
        // §C4: exactly two, Amendment 1 then Amendment 2, distinct. The
        // provenance layer will additionally check them against the real
        // binding commits; the shape is fixed here.
        if self.amendment_commits.len() != 2 {
            return Err(MetaError::Domain(
                "amendment_commits must hold exactly two SHAs",
            ));
        }
        for c in &self.amendment_commits {
            hexlen(c, 40, "amendment_commits")?;
        }
        if self.amendment_commits[0] == self.amendment_commits[1] {
            return Err(MetaError::Domain("amendment_commits must be distinct"));
        }
        hexlen(&self.host_fingerprint, 64, "host_fingerprint")?;
        no_tab_or_lf(&self.cpu_set, "cpu_set")?;
        if !self.timer_resolution_ms.is_finite() {
            return Err(MetaError::Domain("timer_resolution_ms is not finite"));
        }
        if self.cpu_time_unit != CPU_TIME_UNIT {
            return Err(MetaError::Domain("cpu_time_unit must be linux_clock_ticks"));
        }
        nonempty(&self.utc_start, "utc_start")?;
        // §C9 says RFC 3339; accepting any non-empty string made the field
        // decorative. `chrono` is already a dependency.
        if chrono::DateTime::parse_from_rfc3339(&self.utc_start).is_err() {
            return Err(MetaError::Domain("utc_start must be RFC 3339"));
        }
        if let MetaSession::Qualification(n) = self.session {
            if !(1..=6).contains(&n) {
                return Err(MetaError::Domain("session out of 1..=6"));
            }
        }
        Ok(())
    }

    /// The canonical JSON text of each key, in §C9 order. One function serves
    /// both the writer and the reader's byte-for-byte re-render check, so the
    /// two can never drift apart.
    fn canonical_values(&self) -> Result<Vec<String>, MetaError> {
        self.validate()?;
        let arr = |v: &[String]| -> String {
            let items: Vec<String> = v.iter().map(|s| json_string_encode(s)).collect();
            format!("[{}]", items.join(","))
        };
        Ok(vec![
            json_string_encode(SCHEMA_VERSION),
            json_string_encode(&self.run_uuid),
            json_string_encode(&self.boot_id),
            self.run_start_uptime_ms.to_string(),
            json_string_encode(&self.repo_commit),
            json_string_encode(&self.prereg_commit),
            arr(&self.amendment_commits),
            json_string_encode(&self.instrument_birth_commit),
            json_string_encode(&self.host_fingerprint),
            json_string_encode(&self.cpu_set),
            self.thread_count.to_string(),
            self.timer_resolution_ms.to_string(),
            json_string_encode(&self.cpu_time_unit),
            arr(&self.command_line),
            json_string_encode(&self.utc_start),
            self.session.to_json(),
            self.diag_availability.to_json(),
        ])
    }

    /// §C9: `#rc021_meta<TAB>key<TAB>JSON-value<LF>`, keys in fixed order,
    /// compact JSON. Invalid metadata is refused rather than written.
    pub fn render(&self) -> Result<String, MetaError> {
        let values = self.canonical_values()?;
        debug_assert_eq!(values.len(), META_KEYS.len());
        let mut out = String::new();
        for (k, v) in META_KEYS.iter().zip(values.iter()) {
            let _ = writeln!(out, "#rc021_meta\t{k}\t{v}");
        }
        Ok(out)
    }

    /// Strict typed parse of the metadata block (§C9).
    ///
    /// The positional key check catches a missing, extra, duplicated or
    /// reordered key in one comparison, and the canonical re-render at the end
    /// catches a value that parses but is not written in the frozen form.
    pub fn parse(lines: &[(String, String)]) -> Result<Metadata, MetaError> {
        if lines.len() != META_KEYS.len() {
            return Err(MetaError::KeyCount {
                expected: META_KEYS.len(),
                got: lines.len(),
            });
        }
        for (i, (k, _)) in lines.iter().enumerate() {
            if k != META_KEYS[i] {
                return Err(MetaError::KeyOrder {
                    position: i,
                    expected: META_KEYS[i],
                    got: k.clone(),
                });
            }
        }
        let raw: Vec<&str> = lines.iter().map(|(_, v)| v.as_str()).collect();

        let json_str = |i: usize| -> Result<String, MetaError> {
            serde_json::from_str::<String>(raw[i]).map_err(|_| MetaError::Type {
                key: META_KEYS[i],
                want: "string",
            })
        };
        let json_u64 = |i: usize| -> Result<u64, MetaError> {
            serde_json::from_str::<u64>(raw[i]).map_err(|_| MetaError::Type {
                key: META_KEYS[i],
                want: "unsigned integer",
            })
        };
        let json_f64 = |i: usize| -> Result<f64, MetaError> {
            let v: f64 = raw[i].parse().map_err(|_| MetaError::Type {
                key: META_KEYS[i],
                want: "finite number",
            })?;
            if v.is_finite() {
                Ok(v)
            } else {
                Err(MetaError::Type {
                    key: META_KEYS[i],
                    want: "finite number",
                })
            }
        };
        let json_arr = |i: usize| -> Result<Vec<String>, MetaError> {
            serde_json::from_str::<Vec<String>>(raw[i]).map_err(|_| MetaError::Type {
                key: META_KEYS[i],
                want: "array of strings",
            })
        };

        if json_str(0)? != SCHEMA_VERSION {
            return Err(MetaError::Domain("schema_version must be rc021/1"));
        }
        let thread_count = u32::try_from(json_u64(10)?).map_err(|_| MetaError::Type {
            key: "thread_count",
            want: "u32",
        })?;
        let session = MetaSession::parse(raw[15]).ok_or(MetaError::Domain(
            "session must be 1..=6 or \"P2\" or \"N3\" or \"CONTROL\"",
        ))?;
        let diag_availability = DiagAvailability::parse(raw[16])?;

        let m = Metadata {
            run_uuid: json_str(1)?,
            boot_id: json_str(2)?,
            run_start_uptime_ms: json_u64(3)?,
            repo_commit: json_str(4)?,
            prereg_commit: json_str(5)?,
            amendment_commits: json_arr(6)?,
            instrument_birth_commit: json_str(7)?,
            host_fingerprint: json_str(8)?,
            cpu_set: json_str(9)?,
            thread_count,
            timer_resolution_ms: json_f64(11)?,
            cpu_time_unit: json_str(12)?,
            command_line: json_arr(13)?,
            utc_start: json_str(14)?,
            session,
            diag_availability,
        };
        // Byte-for-byte: a value that parses but is not in the frozen compact
        // form — extra spaces, a reordered object, a non-round-trip number — is
        // rejected here.
        let canonical = m.canonical_values()?;
        for (i, (c, r)) in canonical.iter().zip(raw.iter()).enumerate() {
            if c != r {
                return Err(MetaError::NotCanonical {
                    key: META_KEYS[i],
                    want: c.clone(),
                    got: (*r).to_string(),
                });
            }
        }
        Ok(m)
    }
}

fn hexlen(s: &str, n: usize, key: &'static str) -> Result<(), MetaError> {
    if s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(MetaError::Hex { key, len: n })
    }
}

fn nonempty(s: &str, key: &'static str) -> Result<(), MetaError> {
    if s.is_empty() {
        Err(MetaError::Empty(key))
    } else {
        Ok(())
    }
}

fn no_tab_or_lf(s: &str, key: &'static str) -> Result<(), MetaError> {
    if s.contains('\t') || s.contains('\n') || s.contains('\r') {
        Err(MetaError::RawControl(key))
    } else {
        Ok(())
    }
}

/// One parsed metadata line.
fn parse_meta_line(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("#rc021_meta\t")?;
    let (k, v) = rest.split_once('\t')?;
    Some((k.to_string(), v.to_string()))
}

// ==================================================================== WRITER

/// An open, append-only journal. One process holds it for the life of the file.
#[derive(Debug)]
pub struct Journal {
    file: File,
    dir: std::path::PathBuf,
    /// The journal's own validated metadata. Held so that **every** appended
    /// row is checked against it. Without it a row carrying a foreign
    /// `run_uuid`, a wrong `session`, or a value for a channel the metadata
    /// declared unavailable was written and fsynced, and only a later reader
    /// discovered that the instrument had produced its own `JOURNAL-INVALID`.
    meta: Metadata,
}

impl Journal {
    /// §C10.1: claim the path with `create_new` — never truncating, never
    /// overwriting — write the metadata block and the header, make them durable,
    /// then fsync the parent directory so a crash leaves an empty file rather
    /// than no file (§5.2).
    pub fn create(path: &Path, meta: &Metadata) -> std::io::Result<Journal> {
        let dir = path
            .parent()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "journal path has no parent",
                )
            })?
            .to_path_buf();
        // Render — and therefore validate — BEFORE claiming the path, so
        // invalid metadata cannot leave an empty reserved file behind.
        let mut head = meta
            .render()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        head.push_str(&header_line());
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(head.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        fsync_dir(&dir)?;
        Ok(Journal {
            file,
            dir,
            meta: meta.clone(),
        })
    }

    /// Append one row. The line is formed in a single buffer and made durable
    /// **before this call returns**, so nothing accumulates in memory across
    /// measurements — the defect that invalidated RC-020.
    pub fn append(&mut self, row: &Row) -> std::io::Result<()> {
        // Metadata-aware: the writer applies exactly the check the reader will.
        let line = row
            .encode_with(Some(&self.meta))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        self.file.write_all(line.as_bytes())?;
        self.file.flush()?;
        self.file.sync_all()
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn metadata(&self) -> &Metadata {
        &self.meta
    }
}

/// fsync a directory by opening it read-only and syncing the handle. No `libc`,
/// no `unsafe`.
fn fsync_dir(dir: &Path) -> std::io::Result<()> {
    File::open(dir)?.sync_all()
}

// ==================================================================== READER

/// §C13.44 / Amendment 2 counters. Every field is an integer here because this
/// type describes a file that **exists**; an absent file is [`ReadOutcome::Missing`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Counters {
    pub byte_count: u64,
    pub physical_line_count: u64,
    pub metadata_line_count: u64,
    pub header_line_count: u64,
    pub data_row_count: u64,
    pub parsed_row_count: u64,
}

/// §C13.44: every domain key present, including zeros.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StatusCounts {
    pub ok: u64,
    pub lost: u64,
    pub session_open: u64,
    pub session_close_completed: u64,
    pub session_close_aborted: u64,
    pub external_cause: u64,
}

impl StatusCounts {
    fn add(&mut self, s: Status) {
        match s {
            Status::Ok => self.ok += 1,
            Status::Lost => self.lost += 1,
            Status::SessionOpen => self.session_open += 1,
            Status::SessionCloseCompleted => self.session_close_completed += 1,
            Status::SessionCloseAborted => self.session_close_aborted += 1,
            Status::ExternalCause => self.external_cause += 1,
        }
    }
}

/// The reader's verdict on one file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReadVerdict {
    Valid,
    /// §C13.7: a damaged header or a damaged interior row.
    JournalInvalid(String),
}

/// A physical line is counted even when the last one lacks a line feed, and
/// the count must be reportable without decoding the bytes.
fn physical_line_count(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let nl = bytes.iter().filter(|b| **b == b'\n').count() as u64;
    if bytes.last() == Some(&b'\n') {
        nl
    } else {
        nl + 1
    }
}

/// What a present journal yielded.
#[derive(Clone, Debug)]
pub struct JournalRead {
    pub verdict: ReadVerdict,
    pub counters: Counters,
    pub status_counts: StatusCounts,
    pub rows: Vec<Row>,
    /// §C8.3: a truncated final line becomes a logical `LOST` **in memory
    /// only**; no byte on disk is altered.
    pub logical_lost_from_truncation: bool,
    /// The raw key/value lines, kept so a reader can see what was there even
    /// when it did not validate.
    pub metadata: Vec<(String, String)>,
    /// `Some` only when the block satisfied §C9 in full.
    pub typed_metadata: Option<Metadata>,
}

/// §C13.5: an absent file carries `MISSING` and null counters; it is a distinct
/// outcome, not a `JournalRead` full of zeros.
#[derive(Clone, Debug)]
pub enum ReadOutcome {
    Missing,
    /// Boxed: the read is far larger than the absent case, and the enum is
    /// returned by value from every read.
    Present(Box<JournalRead>),
}

/// Read a journal without modifying a single byte.
pub fn read_journal(path: &Path) -> std::io::Result<ReadOutcome> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ReadOutcome::Missing),
        Err(e) => return Err(e),
    };
    Ok(ReadOutcome::Present(Box::new(parse_journal(&bytes))))
}

/// The pure part of the reader, so it is testable without a filesystem.
pub fn parse_journal(bytes: &[u8]) -> JournalRead {
    let mut counters = Counters {
        byte_count: bytes.len() as u64,
        ..Counters::default()
    };
    let mut status_counts = StatusCounts::default();
    let mut rows = Vec::new();

    if bytes.is_empty() {
        return JournalRead {
            verdict: ReadVerdict::JournalInvalid("empty file: no header".into()),
            counters,
            status_counts,
            rows,
            logical_lost_from_truncation: false,
            metadata: Vec::new(),
            typed_metadata: None,
        };
    }

    // §C9 requires UTF-8. `from_utf8_lossy` would substitute U+FFFD and let
    // corrupt bytes pass as a valid line, so the decode is strict and the
    // counters that do not depend on decoding are still reported.
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            counters.physical_line_count = physical_line_count(bytes);
            return JournalRead {
                verdict: ReadVerdict::JournalInvalid(format!(
                    "invalid UTF-8 at byte {}",
                    e.valid_up_to()
                )),
                counters,
                status_counts,
                rows,
                logical_lost_from_truncation: false,
                metadata: Vec::new(),
                typed_metadata: None,
            };
        }
    };
    // A physical line is counted even when the last one lacks a line feed.
    let terminated = bytes.last() == Some(&b'\n');
    let mut lines: Vec<&str> = text.split('\n').collect();
    if terminated {
        lines.pop(); // the empty tail after the final line feed
    }
    counters.physical_line_count = lines.len() as u64;

    let mut idx = 0usize;
    let mut metadata = Vec::new();
    while idx < lines.len() && lines[idx].starts_with("#rc021_meta") {
        // A truncated final metadata line is still a metadata physical line.
        if let Some(kv) = parse_meta_line(lines[idx]) {
            metadata.push(kv);
        }
        counters.metadata_line_count += 1;
        idx += 1;
    }

    // §C13.44: the header counts 1 only when it exists, is line-feed
    // terminated, and matches exactly.
    let header = header_line();
    let header_body = header.trim_end_matches('\n');
    let header_is_last_unterminated = !terminated && idx + 1 == lines.len();
    let header_ok = idx < lines.len() && lines[idx] == header_body && !header_is_last_unterminated;
    if !header_ok {
        return JournalRead {
            verdict: ReadVerdict::JournalInvalid("header missing, truncated or mismatched".into()),
            counters, // data and parsed counts stay 0: with no header there is no "after the header"
            status_counts,
            rows,
            logical_lost_from_truncation: false,
            metadata,
            typed_metadata: None,
        };
    }
    counters.header_line_count = 1;
    idx += 1;

    // §C9 is validated strictly: a missing, extra, duplicated, reordered,
    // mistyped or non-canonical key makes the journal invalid. An earlier
    // version only split key from value and accepted anything.
    let mut verdict = ReadVerdict::Valid;
    let typed_metadata = match Metadata::parse(&metadata) {
        Ok(m) => Some(m),
        Err(e) => {
            verdict = ReadVerdict::JournalInvalid(format!("metadata: {e}"));
            None
        }
    };

    let data = &lines[idx..];
    counters.data_row_count = data.len() as u64;
    let mut logical_lost = false;

    for (i, line) in data.iter().enumerate() {
        let is_final = i + 1 == data.len();
        if is_final && !terminated {
            // §C8.3: a truncated final line is a logical LOST in memory only.
            logical_lost = true;
            status_counts.add(Status::Lost);
            continue;
        }
        match Row::parse(line) {
            Ok(r) => {
                // The same invariants the writer enforces, applied on the way
                // back in and cross-checked against the journal's own metadata.
                if let Err(e) = r.validate(typed_metadata.as_ref()) {
                    verdict = ReadVerdict::JournalInvalid(format!("invalid row: {e}"));
                }
                status_counts.add(r.status);
                rows.push(r);
                counters.parsed_row_count += 1;
            }
            Err(e) => {
                verdict = ReadVerdict::JournalInvalid(format!("corrupt interior row: {e}"));
            }
        }
    }

    JournalRead {
        verdict,
        counters,
        status_counts,
        rows,
        logical_lost_from_truncation: logical_lost,
        metadata,
        typed_metadata,
    }
}

// ======================================================================= TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// A unique scratch directory per test, removed on drop. §C17 forbids any
    /// scheme that could let two runs share a path.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(test_name: &str) -> TempDir {
            let p = std::env::temp_dir().join(format!(
                "rc021_journal_{}_{}_{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::SeqCst),
                test_name
            ));
            std::fs::create_dir_all(&p).expect("scratch dir");
            TempDir(p)
        }
        fn path(&self, name: &str) -> std::path::PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn ctx() -> RowContext {
        RowContext {
            run_uuid: "0".repeat(32),
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            host_fingerprint: "c".repeat(64),
            timer_resolution_ms: 0.00002,
            cpu_set: "0-3".to_string(),
            thread_count: 1,
        }
    }

    fn meta() -> Metadata {
        Metadata {
            run_uuid: "0".repeat(32),
            boot_id: "boot".to_string(),
            run_start_uptime_ms: 1234,
            repo_commit: "a".repeat(40),
            prereg_commit: "b".repeat(40),
            amendment_commits: vec!["d".repeat(40), "e".repeat(40)],
            instrument_birth_commit: "f".repeat(40),
            host_fingerprint: "c".repeat(64),
            cpu_set: "0-3".to_string(),
            thread_count: 1,
            timer_resolution_ms: 0.00002,
            cpu_time_unit: "linux_clock_ticks".to_string(),
            command_line: vec!["exp_rc021_host_qualify".to_string(), "--dir".to_string()],
            utc_start: "2026-08-25T00:00:00Z".to_string(),
            session: MetaSession::Qualification(1),
            diag_availability: DiagAvailability {
                cpu_time: true,
                ctx_switches: true,
                freq: false,
            },
        }
    }

    fn measurement(spread: f64) -> Row {
        let c = ctx();
        Row {
            schema_version: SCHEMA_VERSION.to_string(),
            run_uuid: c.run_uuid.clone(),
            repo_commit: c.repo_commit.clone(),
            prereg_commit: c.prereg_commit.clone(),
            host_fingerprint: c.host_fingerprint.clone(),
            phase: Some(Phase::B),
            session: 1,
            block: Some(3),
            measurement_index: Some(18),
            monotonic_offset_ms: 900,
            sentinel_first_ms: Some(1.989),
            sentinel_last_ms: Some(2.0),
            paired_spread: Some(spread),
            timer_resolution_ms: c.timer_resolution_ms,
            load_avg_start: Some(0.13),
            load_avg_end: Some(0.14),
            cpu_set: c.cpu_set.clone(),
            thread_count: c.thread_count,
            diag_cpu_time: Some(7),
            diag_ctx_switches: Some(2),
            diag_freq: None,
            diag_flags: 4,
            status: Status::Ok,
            reason: String::new(),
        }
    }

    // 1 — header shape
    #[test]
    fn header_has_exactly_24_fields_in_order() {
        let h = header_line();
        assert!(h.ends_with('\n'));
        let cols: Vec<&str> = h.trim_end_matches('\n').split('\t').collect();
        assert_eq!(cols.len(), 24);
        assert_eq!(cols, COLUMNS.to_vec());
        assert_eq!(cols[0], "schema_version");
        assert_eq!(cols[7], "block");
        assert_eq!(cols[23], "reason");
    }

    // 2 — the bound-crossing round trip that fixed decimals would have broken
    #[test]
    fn round_trip_preserves_bits_near_the_bound() {
        for v in [
            0.09_f64,
            0.090000004,
            0.0900000049,
            0.09000000000000001,
            0.089999999,
            1.989,
            0.00002,
        ] {
            let line = measurement(v).encode().expect("encode");
            let back = Row::parse(&line).expect("parse");
            let got = back.paired_spread.expect("spread");
            assert_eq!(
                got.to_bits(),
                v.to_bits(),
                "bits changed for {v:?}: got {got:?}"
            );
            // The property the verdict depends on.
            assert_eq!(got > 0.09, v > 0.09, "bound comparison flipped for {v:?}");
        }
    }

    // 3 — non-finite values are rejected
    #[test]
    fn non_finite_values_are_rejected() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                measurement(bad).encode(),
                Err(CodecError::NonFinite("paired_spread"))
            );
        }
        let mut r = measurement(0.01);
        r.timer_resolution_ms = f64::NAN;
        assert_eq!(
            r.encode(),
            Err(CodecError::NonFinite("timer_resolution_ms"))
        );
        // and on the way back in
        assert!(matches!(
            f64_decode("NaN", "paired_spread"),
            Err(CodecError::NonFinite(_))
        ));
        assert!(matches!(
            f64_decode("inf", "paired_spread"),
            Err(CodecError::NonFinite(_))
        ));
    }

    // 4 — reason may contain tabs and line feeds; the file may not
    #[test]
    fn reason_escapes_tabs_and_newlines() {
        let nasty = "a\tb\nc\"d\\e\u{1}f";
        let mut r = measurement(0.01);
        r.reason = nasty.to_string();
        let line = r.encode().expect("encode");
        let body = line.trim_end_matches('\n');
        assert_eq!(body.matches('\t').count(), 23, "only field separators");
        assert!(!body.contains('\n'));
        let back = Row::parse(&line).expect("parse");
        assert_eq!(back.reason, nasty);
        assert_eq!(json_string_encode(""), "\"\"");
        assert_eq!(json_string_decode("\"\"").unwrap(), "");
        assert!(json_string_decode("no quotes").is_err());
    }

    // 5 — every lifecycle and LOST NA profile
    #[test]
    fn na_profiles_are_exact() {
        let c = ctx();
        let na = |line: &str, i: usize| line.split('\t').nth(i).unwrap() == NA;

        let open = c.session_open(1, 10, 0.13).encode().unwrap();
        for i in [5, 7, 8, 10, 11, 12, 15, 18, 19, 20] {
            assert!(na(&open, i), "SESSION-OPEN field {} must be NA", i + 1);
        }
        assert!(!na(&open, 14), "SESSION-OPEN keeps load_avg_start");
        assert_eq!(open.split('\t').nth(21).unwrap(), "7");

        for completed in [true, false] {
            let close = c.session_close(1, 20, completed, 0.15).encode().unwrap();
            for i in [5, 7, 8, 10, 11, 12, 14, 18, 19, 20] {
                assert!(na(&close, i), "SESSION-CLOSE field {} must be NA", i + 1);
            }
            assert!(!na(&close, 15), "SESSION-CLOSE keeps load_avg_end");
        }

        let ext = c.external_cause(1, 30, 0.13, "power cut").encode().unwrap();
        for i in [5, 7, 8, 10, 11, 12, 15, 18, 19, 20] {
            assert!(na(&ext, i), "EXTERNAL-CAUSE field {} must be NA", i + 1);
        }
        assert_eq!(Row::parse(&ext).unwrap().reason, "power cut");

        // §C8.3: a physical LOST keeps coordinates and nulls BOTH load averages.
        let lost = c
            .lost(1, Phase::C, 2, Some(42), 40, "guard")
            .encode()
            .unwrap();
        for i in [10, 11, 12, 14, 15, 18, 19, 20] {
            assert!(na(&lost, i), "LOST field {} must be NA", i + 1);
        }
        assert!(!na(&lost, 5) && !na(&lost, 7) && !na(&lost, 8));
        assert_eq!(lost.split('\t').nth(21).unwrap(), "7");
    }

    // 6 — qualification versus control coordinates
    #[test]
    fn qualification_and_control_coordinates() {
        let c = ctx();
        // Qualification: global index present, session 1..6.
        let q = measurement(0.01);
        assert_eq!(q.measurement_index, Some(18));
        assert!((1..=6).contains(&q.session));
        assert_eq!(MetaSession::Qualification(4).column_value(), 4);

        // §C11.4: control rows carry session 0 and measurement_index NA.
        for ms in [MetaSession::P2, MetaSession::N3, MetaSession::Control] {
            assert_eq!(ms.column_value(), 0);
        }
        let control = c.lost(0, Phase::A, 1, None, 5, "p2").encode().unwrap();
        assert_eq!(control.split('\t').nth(6).unwrap(), "0");
        assert_eq!(control.split('\t').nth(8).unwrap(), NA);

        // The global index may not stray outside 1..=90 — and the WRITER now
        // refuses it, so such a row can never reach a file at all.
        let mut bad = measurement(0.01);
        bad.measurement_index = Some(91);
        assert!(matches!(
            bad.encode(),
            Err(CodecError::Invariant("measurement_index must be 1..=90"))
        ));
        // A control row may not borrow the global index.
        let mut borrow = measurement(0.01);
        borrow.session = 0;
        assert!(matches!(
            borrow.encode(),
            Err(CodecError::Invariant(
                "a control row must carry measurement_index = NA"
            ))
        ));
    }

    // 7 — create_new refuses an existing path and changes nothing
    #[test]
    fn create_new_refuses_and_preserves_bytes() {
        let d = TempDir::new("create_new");
        let p = d.path("j.tsv");
        std::fs::write(&p, b"pre-existing").unwrap();
        let before = std::fs::read(&p).unwrap();
        let err = Journal::create(&p, &meta()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "bytes must be untouched"
        );
    }

    // 8 — every appended row is readable after the call returns
    #[test]
    fn appended_rows_are_durable_and_readable() {
        let d = TempDir::new("append");
        let p = d.path("j.tsv");
        let c = ctx();
        {
            let mut j = Journal::create(&p, &meta()).unwrap();
            j.append(&c.session_open(1, 0, 0.1)).unwrap();
            j.append(&measurement(0.05)).unwrap();
            j.append(&c.session_close(1, 99, true, 0.2)).unwrap();
        }
        let out = read_journal(&p).unwrap();
        let ReadOutcome::Present(r) = out else {
            panic!("journal must be present")
        };
        assert_eq!(r.verdict, ReadVerdict::Valid);
        assert_eq!(r.counters.metadata_line_count, 17);
        assert_eq!(r.counters.header_line_count, 1);
        assert_eq!(r.counters.data_row_count, 3);
        assert_eq!(r.counters.parsed_row_count, 3);
        assert_eq!(r.status_counts.session_open, 1);
        assert_eq!(r.status_counts.ok, 1);
        assert_eq!(r.status_counts.session_close_completed, 1);
        assert_eq!(r.metadata.len(), 17);
        assert_eq!(r.metadata[0].0, "schema_version");
        assert_eq!(r.metadata[16].0, "diag_availability");
    }

    // 9 — a truncated final line is a logical LOST, and the file is untouched
    #[test]
    fn truncated_final_line_is_logical_lost_only() {
        let d = TempDir::new("truncated");
        let p = d.path("j.tsv");
        {
            let mut j = Journal::create(&p, &meta()).unwrap();
            j.append(&measurement(0.01)).unwrap();
        }
        // Append a partial line the way a crash mid-write would leave one.
        let partial = "rc021/1\t0000";
        let mut f = OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(partial.as_bytes()).unwrap();
        f.sync_all().unwrap();
        let before = std::fs::read(&p).unwrap();

        let ReadOutcome::Present(r) = read_journal(&p).unwrap() else {
            panic!("present")
        };
        assert_eq!(
            r.verdict,
            ReadVerdict::Valid,
            "truncation is not corruption"
        );
        assert!(r.logical_lost_from_truncation);
        assert_eq!(
            r.counters.data_row_count, 2,
            "the partial line is a data row"
        );
        assert_eq!(r.counters.parsed_row_count, 1, "but it does not parse");
        assert_eq!(r.status_counts.lost, 1, "logical LOST is counted");
        assert_eq!(r.rows.len(), 1);
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "reading must not repair a single byte"
        );
    }

    // 10 — corrupt interior row and corrupt header
    #[test]
    fn corruption_yields_journal_invalid() {
        let good = {
            let mut s = meta().render().unwrap();
            s.push_str(&header_line());
            s.push_str(&measurement(0.01).encode().unwrap());
            s
        };

        // interior row damaged, with a well-formed row after it
        let mut interior = good.clone();
        interior.push_str("not\ta\tvalid\trow\n");
        interior.push_str(&measurement(0.02).encode().unwrap());
        let r = parse_journal(interior.as_bytes());
        assert!(matches!(r.verdict, ReadVerdict::JournalInvalid(_)));
        assert_eq!(r.counters.data_row_count, 3);
        assert_eq!(r.counters.parsed_row_count, 2);

        // header misspelled
        let bad_header = good.replace("schema_version\trun_uuid", "schema_version\trun_uuidX");
        let r = parse_journal(bad_header.as_bytes());
        assert!(matches!(r.verdict, ReadVerdict::JournalInvalid(_)));
        assert_eq!(r.counters.header_line_count, 0);
        assert_eq!(r.counters.data_row_count, 0, "no header means no data rows");
        assert_eq!(r.counters.parsed_row_count, 0);
    }

    // 11 — counters across every file state
    #[test]
    fn counters_cover_every_file_state() {
        // empty
        let r = parse_journal(b"");
        assert_eq!(r.counters.byte_count, 0);
        assert_eq!(r.counters.physical_line_count, 0);
        assert_eq!(r.counters.metadata_line_count, 0);
        assert_eq!(r.counters.header_line_count, 0);
        assert_eq!(r.counters.data_row_count, 0);
        assert_eq!(r.counters.parsed_row_count, 0);
        assert!(matches!(r.verdict, ReadVerdict::JournalInvalid(_)));

        // metadata only
        let m = meta().render().unwrap();
        let r = parse_journal(m.as_bytes());
        assert_eq!(r.counters.metadata_line_count, 17);
        assert_eq!(r.counters.header_line_count, 0);
        assert_eq!(r.counters.data_row_count, 0);
        assert!(matches!(r.verdict, ReadVerdict::JournalInvalid(_)));

        // metadata plus a truncated header
        let mut trunc_header = m.clone();
        trunc_header.push_str("schema_version\trun_uuid");
        let r = parse_journal(trunc_header.as_bytes());
        assert_eq!(r.counters.metadata_line_count, 17);
        assert_eq!(r.counters.header_line_count, 0);
        assert_eq!(r.counters.data_row_count, 0);

        // valid header and two rows
        let mut valid = m.clone();
        valid.push_str(&header_line());
        valid.push_str(&measurement(0.01).encode().unwrap());
        valid.push_str(&measurement(0.02).encode().unwrap());
        let r = parse_journal(valid.as_bytes());
        assert_eq!(r.verdict, ReadVerdict::Valid);
        assert_eq!(r.counters.header_line_count, 1);
        assert_eq!(r.counters.data_row_count, 2);
        assert_eq!(r.counters.parsed_row_count, 2);
        assert_eq!(r.counters.physical_line_count, 17 + 1 + 2);

        // truncated final data line
        let mut trunc = valid.clone();
        trunc.push_str("rc021/1\tpartial");
        let r = parse_journal(trunc.as_bytes());
        assert_eq!(r.counters.data_row_count, 3);
        assert_eq!(r.counters.parsed_row_count, 2);
        assert!(r.logical_lost_from_truncation);
    }

    // 12 — an absent file is Missing, not a JournalRead of zeros
    #[test]
    fn absent_file_is_missing_not_empty() {
        let d = TempDir::new("absent");
        let out = read_journal(&d.path("nope.tsv")).unwrap();
        assert!(matches!(out, ReadOutcome::Missing));
        // The distinction matters: an empty file exists and is JOURNAL-INVALID.
        let p = d.path("empty.tsv");
        std::fs::write(&p, b"").unwrap();
        let ReadOutcome::Present(r) = read_journal(&p).unwrap() else {
            panic!("an empty file is present, not missing")
        };
        assert!(matches!(r.verdict, ReadVerdict::JournalInvalid(_)));
    }

    #[test]
    fn metadata_grammar_is_exact() {
        let m = meta().render().unwrap();
        let lines: Vec<&str> = m.trim_end_matches('\n').split('\n').collect();
        assert_eq!(lines.len(), 17);
        for (i, l) in lines.iter().enumerate() {
            let parts: Vec<&str> = l.split('\t').collect();
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], "#rc021_meta");
            assert_eq!(parts[1], META_KEYS[i], "key order is fixed");
        }
        assert!(m.contains("#rc021_meta\tcpu_time_unit\t\"linux_clock_ticks\""));
        assert!(m.contains("#rc021_meta\tamendment_commits\t[\""));
        assert!(m.contains("\"cpu_time\":true,\"ctx_switches\":true,\"freq\":false"));
        assert!(m.contains("#rc021_meta\tsession\t1"));
    }

    // ---------------- regression: metadata is now strictly validated ---------

    fn meta_lines(m: &Metadata) -> Vec<(String, String)> {
        m.render()
            .unwrap()
            .lines()
            .map(|l| {
                let p: Vec<&str> = l.split('\t').collect();
                (p[1].to_string(), p[2].to_string())
            })
            .collect()
    }

    fn journal_bytes(meta_block: &str, rows: &[Row]) -> Vec<u8> {
        let mut s = meta_block.to_string();
        s.push_str(&header_line());
        for r in rows {
            s.push_str(&r.encode().expect("row encodes"));
        }
        s.into_bytes()
    }

    #[test]
    fn metadata_missing_extra_duplicate_or_reordered_keys_are_rejected() {
        let base = meta_lines(&meta());

        let mut missing = base.clone();
        missing.remove(3);
        assert!(matches!(
            Metadata::parse(&missing),
            Err(MetaError::KeyCount { .. })
        ));

        let mut extra = base.clone();
        extra.push(("surplus".into(), "1".into()));
        assert!(matches!(
            Metadata::parse(&extra),
            Err(MetaError::KeyCount { .. })
        ));

        let mut dup = base.clone();
        dup[4] = dup[3].clone(); // duplicate key, count unchanged
        assert!(matches!(
            Metadata::parse(&dup),
            Err(MetaError::KeyOrder { .. })
        ));

        let mut swapped = base.clone();
        swapped.swap(2, 3);
        assert!(matches!(
            Metadata::parse(&swapped),
            Err(MetaError::KeyOrder { .. })
        ));

        // and the unmodified block still parses
        assert!(Metadata::parse(&base).is_ok());
    }

    #[test]
    fn metadata_wrong_json_types_are_rejected() {
        let base = meta_lines(&meta());
        // key index -> a value of the wrong JSON type
        for (idx, wrong) in [
            (1usize, "123"),           // run_uuid: number, want string
            (3, "\"1234\""),           // run_start_uptime_ms: string, want int
            (6, "\"not-an-array\""),   // amendment_commits: string, want array
            (10, "\"1\""),             // thread_count: string, want int
            (13, "{}"),                // command_line: object, want array
            (16, "[true,true,false]"), // diag_availability: array, want object
        ] {
            let mut bad = base.clone();
            bad[idx].1 = wrong.to_string();
            assert!(
                Metadata::parse(&bad).is_err(),
                "key {} must reject {wrong}",
                META_KEYS[idx]
            );
        }
        // diag_availability with a missing member and with a surplus member
        for wrong in [
            "{\"cpu_time\":true,\"ctx_switches\":true}",
            "{\"cpu_time\":true,\"ctx_switches\":true,\"freq\":false,\"x\":1}",
            "{\"cpu_time\":1,\"ctx_switches\":true,\"freq\":false}",
        ] {
            let mut bad = base.clone();
            bad[16].1 = wrong.to_string();
            assert!(Metadata::parse(&bad).is_err(), "must reject {wrong}");
        }
    }

    #[test]
    fn metadata_bad_literals_and_hex_lengths_are_rejected() {
        let base = meta_lines(&meta());
        let cases = [
            (0usize, "\"rc021/2\""),                 // schema_version literal
            (1, "\"0123\""),                         // run_uuid too short
            (1, &format!("\"{}\"", "G".repeat(32))), // run_uuid not hex
            (4, &format!("\"{}\"", "a".repeat(39))), // repo_commit length
            (8, &format!("\"{}\"", "c".repeat(63))), // fingerprint length
            (12, "\"seconds\""),                     // cpu_time_unit literal
            (15, "0"),                               // session out of domain
            (15, "7"),
            (15, "\"NA\""),
        ];
        for (idx, wrong) in cases {
            let mut bad = base.clone();
            bad[idx].1 = wrong.to_string();
            assert!(
                Metadata::parse(&bad).is_err(),
                "key {} must reject {wrong}",
                META_KEYS[idx]
            );
        }
        // amendment_commits with a non-hex member
        let mut bad = base.clone();
        bad[6].1 = "[\"zz\"]".to_string();
        assert!(Metadata::parse(&bad).is_err());
    }

    #[test]
    fn metadata_non_canonical_form_is_rejected() {
        let base = meta_lines(&meta());
        // parses as the same value, but is not the frozen compact form
        let mut spaced = base.clone();
        spaced[16].1 = "{\"cpu_time\": true, \"ctx_switches\": true, \"freq\": false}".into();
        assert!(matches!(
            Metadata::parse(&spaced),
            Err(MetaError::NotCanonical { .. })
        ));
        // reordered object members
        let mut reordered = base.clone();
        reordered[16].1 = "{\"ctx_switches\":true,\"cpu_time\":true,\"freq\":false}".into();
        assert!(matches!(
            Metadata::parse(&reordered),
            Err(MetaError::NotCanonical { .. })
        ));
    }

    #[test]
    fn writer_refuses_metadata_the_reader_would_reject() {
        let mut m = meta();
        m.cpu_time_unit = "seconds".into();
        assert!(m.render().is_err(), "render must refuse, not write");
        let mut m = meta();
        m.run_uuid = "short".into();
        assert!(m.render().is_err());
        let mut m = meta();
        m.timer_resolution_ms = f64::NAN;
        assert!(m.render().is_err());
    }

    #[test]
    fn journal_with_bad_metadata_is_journal_invalid() {
        let mut lines = meta_lines(&meta());
        lines[12].1 = "\"seconds\"".to_string();
        let block: String = lines
            .iter()
            .map(|(k, v)| format!("#rc021_meta\t{k}\t{v}\n"))
            .collect();
        let bytes = journal_bytes(&block, &[]);
        let r = parse_journal(&bytes);
        assert!(matches!(r.verdict, ReadVerdict::JournalInvalid(_)));
        assert!(r.typed_metadata.is_none());
        // counters are still reported
        assert_eq!(r.counters.metadata_line_count, 17);
        assert_eq!(r.counters.header_line_count, 1);
    }

    // ---------------- regression: strict UTF-8 --------------------------------

    #[test]
    fn invalid_utf8_is_journal_invalid_and_counters_survive() {
        let d = TempDir::new("utf8");
        let p = d.path("j.tsv");
        let mut bytes = journal_bytes(&meta().render().unwrap(), &[measurement(0.01)]);
        bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
        std::fs::write(&p, &bytes).unwrap();
        let before = std::fs::read(&p).unwrap();

        let ReadOutcome::Present(r) = read_journal(&p).unwrap() else {
            panic!("present")
        };
        match &r.verdict {
            ReadVerdict::JournalInvalid(m) => assert!(m.contains("UTF-8")),
            v => panic!("want JOURNAL-INVALID, got {v:?}"),
        }
        assert_eq!(r.counters.byte_count, bytes.len() as u64);
        assert_eq!(r.counters.physical_line_count, 17 + 1 + 1 + 1);
        assert_eq!(std::fs::read(&p).unwrap(), before, "file must not change");
    }

    // ---------------- regression: checked integer conversion ------------------

    #[test]
    fn integer_fields_do_not_wrap() {
        let line = measurement(0.01).encode().unwrap();
        let f: Vec<&str> = line.trim_end_matches('\n').split('\t').collect();
        let rebuild = |i: usize, v: &str| {
            let mut g = f.clone();
            g[i] = v;
            let mut s = g.join("\t");
            s.push('\n');
            s
        };
        // 256 previously became 0 through `as u8` and was accepted.
        assert!(matches!(
            Row::parse(&rebuild(6, "256")),
            Err(CodecError::BadField {
                column: "session",
                ..
            })
        ));
        assert!(matches!(
            Row::parse(&rebuild(21, "256")),
            Err(CodecError::BadField {
                column: "diag_flags",
                ..
            })
        ));
        // u32::MAX + 1 previously truncated through `as u32`.
        assert!(matches!(
            Row::parse(&rebuild(17, "4294967296")),
            Err(CodecError::BadField {
                column: "thread_count",
                ..
            })
        ));
        assert!(Row::parse(&rebuild(17, "4294967295")).is_ok());
    }

    // ---------------- regression: row invariants ------------------------------

    #[test]
    fn every_bad_lifecycle_and_lost_combination_is_refused() {
        let c = ctx();
        let bad = |r: Row, what: &str| {
            assert!(r.encode().is_err(), "writer must refuse: {what}");
        };
        // SESSION-OPEN with an end average, or without a start one
        let mut r = c.session_open(1, 1, 0.1);
        r.load_avg_end = Some(0.2);
        bad(r, "SESSION-OPEN with load_avg_end");
        let mut r = c.session_open(1, 1, 0.1);
        r.load_avg_start = None;
        bad(r, "SESSION-OPEN without load_avg_start");
        // SESSION-CLOSE with a start average
        let mut r = c.session_close(1, 1, true, 0.2);
        r.load_avg_start = Some(0.1);
        bad(r, "SESSION-CLOSE with load_avg_start");
        // lifecycle row carrying coordinates or a measurement
        let mut r = c.session_open(1, 1, 0.1);
        r.phase = Some(Phase::A);
        bad(r, "lifecycle with a phase");
        let mut r = c.session_open(1, 1, 0.1);
        r.paired_spread = Some(0.01);
        bad(r, "lifecycle with a spread");
        // LOST with a load average, or without coordinates
        let mut r = c.lost(1, Phase::A, 1, Some(1), 1, "x");
        r.load_avg_start = Some(0.1);
        bad(r, "LOST with load_avg_start");
        let mut r = c.lost(1, Phase::A, 1, Some(1), 1, "x");
        r.load_avg_end = Some(0.1);
        bad(r, "LOST with load_avg_end");
        let mut r = c.lost(1, Phase::A, 1, Some(1), 1, "x");
        r.block = None;
        bad(r, "LOST without a block");
        // OK without a spread
        let mut r = measurement(0.01);
        r.paired_spread = None;
        bad(r, "OK without paired_spread");
        let mut r = measurement(0.01);
        r.measurement_index = None;
        bad(r, "qualification OK without measurement_index");
    }

    #[test]
    fn all_eight_diag_flag_combinations_must_agree() {
        for flags in 0u8..8 {
            let mut r = measurement(0.01);
            r.diag_flags = flags;
            r.diag_cpu_time = if flags & 1 == 0 { Some(7) } else { None };
            r.diag_ctx_switches = if flags & 2 == 0 { Some(2) } else { None };
            r.diag_freq = if flags & 4 == 0 {
                Some(3_000_000)
            } else {
                None
            };
            assert!(
                r.encode().is_ok(),
                "agreeing combination {flags} must encode"
            );

            // flip exactly one channel out of agreement
            for bit in [1u8, 2, 4] {
                let mut w = r.clone();
                match bit {
                    1 => {
                        w.diag_cpu_time = if w.diag_cpu_time.is_some() {
                            None
                        } else {
                            Some(1)
                        }
                    }
                    2 => {
                        w.diag_ctx_switches = if w.diag_ctx_switches.is_some() {
                            None
                        } else {
                            Some(1)
                        }
                    }
                    _ => w.diag_freq = if w.diag_freq.is_some() { None } else { Some(1) },
                }
                assert!(
                    matches!(w.encode(), Err(CodecError::DiagMismatch(_))),
                    "flags {flags}, bit {bit} must be refused"
                );
            }
        }
    }

    #[test]
    fn declared_unavailable_channel_may_not_carry_a_value() {
        // meta() declares freq = false.
        let m = meta();
        let mut r = measurement(0.01);
        r.diag_freq = Some(3_000_000);
        r.diag_flags = 0; // consistent with itself, but not with the metadata
        r.diag_cpu_time = Some(7);
        r.diag_ctx_switches = Some(2);
        assert!(r.validate(None).is_ok(), "self-consistent in isolation");
        assert!(
            matches!(
                r.validate(Some(&m)),
                Err(CodecError::DiagUnavailable("diag_freq"))
            ),
            "the journal declared freq unavailable"
        );
    }

    #[test]
    fn row_provenance_must_match_the_journal_metadata() {
        let m = meta();
        let ok = measurement(0.01);
        assert!(ok.validate(Some(&m)).is_ok());
        for (mutate, want) in [
            (
                (|r: &mut Row| r.run_uuid = "1".repeat(32)) as fn(&mut Row),
                "run_uuid",
            ),
            (|r: &mut Row| r.repo_commit = "9".repeat(40), "repo_commit"),
            (
                |r: &mut Row| r.host_fingerprint = "9".repeat(64),
                "host_fingerprint",
            ),
            (|r: &mut Row| r.cpu_set = "0-7".into(), "cpu_set"),
            (|r: &mut Row| r.thread_count = 9, "thread_count"),
            (
                |r: &mut Row| r.timer_resolution_ms = 0.5,
                "timer_resolution_ms",
            ),
            (|r: &mut Row| r.session = 2, "session"),
        ] {
            let mut r = measurement(0.01);
            mutate(&mut r);
            assert_eq!(r.validate(Some(&m)), Err(CodecError::MetaMismatch(want)));
        }
    }

    #[test]
    fn bad_provenance_syntax_is_refused() {
        for (mutate, _what) in [
            (
                (|r: &mut Row| r.schema_version = "rc021/2".into()) as fn(&mut Row),
                "schema",
            ),
            (|r: &mut Row| r.run_uuid = "z".repeat(32), "hex"),
            (|r: &mut Row| r.repo_commit = "a".repeat(39), "length"),
            (|r: &mut Row| r.cpu_set = "0\t3".into(), "tab"),
            (
                |r: &mut Row| r.host_fingerprint = "A".repeat(64),
                "upper case",
            ),
        ] {
            let mut r = measurement(0.01);
            mutate(&mut r);
            assert!(r.encode().is_err());
        }
    }

    // ---------------- regression: JSON string conformance ---------------------

    #[test]
    fn json_strings_follow_rfc_8259() {
        // A surrogate pair the hand-written parser rejected.
        assert_eq!(
            json_string_decode("\"\\uD834\\uDD1E\"").unwrap(),
            "\u{1D11E}"
        );
        // Round trip through the encoder.
        for s in [
            "\u{1D11E}",
            "\u{00e9}",
            "\u{10FFFF}",
            "tab\there",
            "nl\nhere",
            "",
        ] {
            assert_eq!(json_string_decode(&json_string_encode(s)).unwrap(), s);
        }
        // Raw control characters are escaped, never emitted.
        let enc = json_string_encode("\u{1}\u{1f}");
        assert!(enc.contains("\\u0001") && enc.contains("\\u001f"));
        // Trailing garbage is not a string.
        assert!(json_string_decode("\"a\" trailing").is_err());
        assert!(json_string_decode("\"a\"\"b\"").is_err());
        assert!(json_string_decode("123").is_err());
        assert!(json_string_decode("\"unterminated").is_err());
    }

    // ---------------- regression: reading never mutates ------------------------

    #[test]
    fn reading_leaves_the_file_byte_identical() {
        let d = TempDir::new("immutable");
        let c = ctx();
        for (name, make) in [
            (
                "valid",
                journal_bytes(&meta().render().unwrap(), &[measurement(0.02)]),
            ),
            ("empty", Vec::new()),
            ("meta_only", meta().render().unwrap().into_bytes()),
            ("truncated", {
                let mut b = journal_bytes(&meta().render().unwrap(), &[measurement(0.02)]);
                b.extend_from_slice(b"rc021/1\tpartial");
                b
            }),
            (
                "lifecycle",
                journal_bytes(&meta().render().unwrap(), &[c.session_open(1, 0, 0.1)]),
            ),
        ] {
            let p = d.path(name);
            std::fs::write(&p, &make).unwrap();
            let before = std::fs::read(&p).unwrap();
            let _ = read_journal(&p).unwrap();
            assert_eq!(std::fs::read(&p).unwrap(), before, "{name} must not change");
        }
    }

    // ------- regression: append is metadata-aware, through the real API ------

    #[test]
    fn append_refuses_rows_that_disagree_with_the_journal_metadata() {
        let d = TempDir::new("append_meta");
        let m = meta(); // declares session 1 and freq = false
        type Mutate = Box<dyn Fn(&mut Row)>;
        let cases: Vec<(&str, Mutate)> = vec![
            (
                "run_uuid",
                Box::new(|r: &mut Row| r.run_uuid = "1".repeat(32)),
            ),
            (
                "repo_commit",
                Box::new(|r: &mut Row| r.repo_commit = "9".repeat(40)),
            ),
            (
                "host_fingerprint",
                Box::new(|r: &mut Row| r.host_fingerprint = "9".repeat(64)),
            ),
            ("cpu_set", Box::new(|r: &mut Row| r.cpu_set = "0-7".into())),
            ("thread_count", Box::new(|r: &mut Row| r.thread_count = 9)),
            ("session", Box::new(|r: &mut Row| r.session = 2)),
            (
                "declared-unavailable freq",
                Box::new(|r: &mut Row| {
                    r.diag_freq = Some(3_000_000);
                    r.diag_flags &= !4;
                }),
            ),
        ];
        for (i, (what, mutate)) in cases.iter().enumerate() {
            let p = d.path(&format!("j{i}.tsv"));
            let mut j = Journal::create(&p, &m).unwrap();
            let before = std::fs::read(&p).unwrap();
            let mut r = measurement(0.01);
            mutate(&mut r);
            let err = j
                .append(&r)
                .expect_err(&format!("append must refuse {what}"));
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{what}");
            drop(j);
            assert_eq!(
                std::fs::read(&p).unwrap(),
                before,
                "{what}: a refused append must write no byte"
            );
        }
        // The agreeing row still appends, so the check is not vacuous.
        let p = d.path("good.tsv");
        let mut j = Journal::create(&p, &m).unwrap();
        let before_len = std::fs::read(&p).unwrap().len();
        j.append(&measurement(0.01)).unwrap();
        drop(j);
        assert!(std::fs::read(&p).unwrap().len() > before_len);
    }

    #[test]
    fn invalid_metadata_reserves_no_path() {
        let d = TempDir::new("no_reserve");
        let p = d.path("j.tsv");
        let mut bad = meta();
        bad.cpu_time_unit = "seconds".into();
        let err = Journal::create(&p, &bad).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(
            !p.exists(),
            "invalid metadata must not leave an empty reserved file"
        );
    }

    // ------- regression: LOST symmetry and mandatory reasons -----------------

    #[test]
    fn lost_measurement_index_is_symmetric_with_ok() {
        let c = ctx();
        // qualification LOST without the global index: refused by the writer
        let mut r = c.lost(1, Phase::A, 1, None, 5, "guard");
        assert!(matches!(
            r.encode(),
            Err(CodecError::Invariant(
                "a qualification LOST carries measurement_index"
            ))
        ));
        // with it: accepted
        r.measurement_index = Some(7);
        assert!(r.encode().is_ok());
        // control LOST with an index: refused
        let bad = c.lost(0, Phase::A, 1, Some(7), 5, "p2");
        assert!(matches!(
            bad.encode(),
            Err(CodecError::Invariant(
                "a control row must carry measurement_index = NA"
            ))
        ));
        // control LOST without one: accepted
        assert!(c.lost(0, Phase::A, 1, None, 5, "p2").encode().is_ok());

        // and the reader refuses the same shapes
        let line = c
            .lost(1, Phase::A, 1, Some(7), 5, "guard")
            .encode()
            .unwrap();
        let f: Vec<&str> = line.trim_end_matches('\n').split('\t').collect();
        let mut g = f.clone();
        g[8] = NA;
        let mut broken = g.join("\t");
        broken.push('\n');
        let parsed = Row::parse(&broken).unwrap();
        assert!(matches!(
            parsed.validate(None),
            Err(CodecError::Invariant(
                "a qualification LOST carries measurement_index"
            ))
        ));
    }

    #[test]
    fn lost_and_external_cause_require_a_reason() {
        let c = ctx();
        assert!(matches!(
            c.lost(1, Phase::A, 1, Some(1), 5, "").encode(),
            Err(CodecError::Invariant("a physical LOST carries its reason"))
        ));
        assert!(matches!(
            c.external_cause(1, 5, 0.1, "").encode(),
            Err(CodecError::Invariant("EXTERNAL-CAUSE carries its cause"))
        ));
        // a lifecycle row that is not EXTERNAL-CAUSE may have an empty reason
        assert!(c.session_open(1, 5, 0.1).encode().is_ok());
        assert!(c.session_close(1, 6, true, 0.2).encode().is_ok());
    }

    // ------- regression: amendment_commits and utc_start ---------------------

    #[test]
    fn amendment_commits_must_be_exactly_two_distinct_shas() {
        let sha = |c: char| c.to_string().repeat(40);
        for (list, what) in [
            (vec![], "empty"),
            (vec![sha('d')], "one"),
            (vec![sha('d'), sha('e'), sha('f')], "three"),
            (vec![sha('d'), sha('d')], "duplicate"),
            (vec![sha('d'), "z".repeat(40)], "non-hex"),
            (vec![sha('d'), "e".repeat(39)], "short"),
        ] {
            let mut m = meta();
            m.amendment_commits = list;
            assert!(m.validate().is_err(), "must reject {what}");
            assert!(m.render().is_err(), "writer must refuse {what}");
        }
        assert!(meta().validate().is_ok());
    }

    #[test]
    fn utc_start_must_be_rfc_3339() {
        for good in [
            "2026-08-25T00:00:00Z",
            "2026-08-25T12:34:56.789Z",
            "2026-08-25T12:34:56+05:00",
            "2026-08-25T12:34:56-08:00",
        ] {
            let mut m = meta();
            m.utc_start = good.into();
            assert!(m.validate().is_ok(), "{good} must be accepted");
        }
        for bad in [
            "not a date",
            "2026-13-01T00:00:00Z", // impossible month
            "2026-02-30T00:00:00Z", // impossible day
            "2026-08-25T00:00:00",  // no timezone
            "2026-08-25",           // date only
            "",
        ] {
            let mut m = meta();
            m.utc_start = bad.into();
            assert!(m.validate().is_err(), "{bad:?} must be rejected");
            assert!(m.render().is_err(), "writer must refuse {bad:?}");
        }
        // the reader applies the same rule
        let mut lines = meta_lines(&meta());
        lines[14].1 = "\"2026-08-25T00:00:00\"".to_string();
        assert!(Metadata::parse(&lines).is_err());
    }

    #[test]
    fn meta_session_domain_is_closed() {
        assert_eq!(MetaSession::parse("1"), Some(MetaSession::Qualification(1)));
        assert_eq!(MetaSession::parse("6"), Some(MetaSession::Qualification(6)));
        assert_eq!(
            MetaSession::parse("\"CONTROL\""),
            Some(MetaSession::Control)
        );
        assert_eq!(MetaSession::parse("\"P2\""), Some(MetaSession::P2));
        assert_eq!(MetaSession::parse("\"N3\""), Some(MetaSession::N3));
        // "NA" was in an earlier draft and is not in the domain.
        assert_eq!(MetaSession::parse("\"NA\""), None);
        assert_eq!(MetaSession::parse("0"), None);
        assert_eq!(MetaSession::parse("7"), None);
    }
}
