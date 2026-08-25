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
        }
    }
}

// =============================================================== JSON STRINGS

/// §C8.2 field 24: a JSON string per RFC 8259, quotes included. Because the
/// value is escaped, a raw tab or line feed can never reach the file.
pub fn json_string_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Inverse of [`json_string_encode`]. Rejects anything that is not a complete,
/// well-formed JSON string literal.
pub fn json_string_decode(field: &str) -> Result<String, CodecError> {
    let bad = || CodecError::BadJsonString(field.to_string());
    let inner = field
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(bad)?;
    // A trailing backslash would have escaped the closing quote.
    if inner.len() != inner.trim_end_matches('\\').len() % 2 + inner.trim_end_matches('\\').len()
        && inner.chars().rev().take_while(|c| *c == '\\').count() % 2 == 1
    {
        return Err(bad());
    }
    let mut out = String::with_capacity(inner.len());
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            if (c as u32) < 0x20 {
                return Err(bad());
            }
            out.push(c);
            continue;
        }
        match it.next().ok_or_else(bad)? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'b' => out.push('\u{08}'),
            'f' => out.push('\u{0c}'),
            'u' => {
                let hex: String = it.by_ref().take(4).collect();
                if hex.len() != 4 {
                    return Err(bad());
                }
                let cp = u32::from_str_radix(&hex, 16).map_err(|_| bad())?;
                out.push(char::from_u32(cp).ok_or_else(bad)?);
            }
            _ => return Err(bad()),
        }
    }
    Ok(out)
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
    /// Serialise to one tab-separated, line-feed-terminated line.
    pub fn encode(&self) -> Result<String, CodecError> {
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
        let session = int(f[6], "session")? as u8;
        if session > 6 {
            return Err(CodecError::BadField {
                column: "session",
                value: f[6].to_string(),
            });
        }
        let block = opt_u64_decode(f[7], "block")?
            .map(|b| {
                if (1..=5).contains(&b) {
                    Ok(b as u8)
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
                    Ok(m as u32)
                } else {
                    Err(CodecError::BadField {
                        column: "measurement_index",
                        value: f[8].to_string(),
                    })
                }
            })
            .transpose()?;
        let diag_flags = int(f[21], "diag_flags")? as u8;
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
            thread_count: int(f[17], "thread_count")? as u32,
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
    /// §C9: `#rc021_meta<TAB>key<TAB>JSON-value<LF>`, keys in fixed order,
    /// compact JSON.
    pub fn render(&self) -> Result<String, CodecError> {
        let arr = |v: &[String]| -> String {
            let items: Vec<String> = v.iter().map(|s| json_string_encode(s)).collect();
            format!("[{}]", items.join(","))
        };
        let values: Vec<String> = vec![
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
            f64_encode(self.timer_resolution_ms, "timer_resolution_ms")?,
            json_string_encode(&self.cpu_time_unit),
            arr(&self.command_line),
            json_string_encode(&self.utc_start),
            self.session.to_json(),
            self.diag_availability.to_json(),
        ];
        debug_assert_eq!(values.len(), META_KEYS.len());
        let mut out = String::new();
        for (k, v) in META_KEYS.iter().zip(values.iter()) {
            let _ = writeln!(out, "#rc021_meta\t{k}\t{v}");
        }
        Ok(out)
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
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut head = meta
            .render()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        head.push_str(&header_line());
        file.write_all(head.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        fsync_dir(&dir)?;
        Ok(Journal { file, dir })
    }

    /// Append one row. The line is formed in a single buffer and made durable
    /// **before this call returns**, so nothing accumulates in memory across
    /// measurements — the defect that invalidated RC-020.
    pub fn append(&mut self, row: &Row) -> std::io::Result<()> {
        let line = row
            .encode()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        self.file.write_all(line.as_bytes())?;
        self.file.flush()?;
        self.file.sync_all()
    }

    pub fn dir(&self) -> &Path {
        &self.dir
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
    pub metadata: Vec<(String, String)>,
}

/// §C13.5: an absent file carries `MISSING` and null counters; it is a distinct
/// outcome, not a `JournalRead` full of zeros.
#[derive(Clone, Debug)]
pub enum ReadOutcome {
    Missing,
    Present(JournalRead),
}

/// Read a journal without modifying a single byte.
pub fn read_journal(path: &Path) -> std::io::Result<ReadOutcome> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ReadOutcome::Missing),
        Err(e) => return Err(e),
    };
    Ok(ReadOutcome::Present(parse_journal(&bytes)))
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
        };
    }

    let text = String::from_utf8_lossy(bytes);
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
        };
    }
    counters.header_line_count = 1;
    idx += 1;

    let data = &lines[idx..];
    counters.data_row_count = data.len() as u64;
    let mut logical_lost = false;
    let mut verdict = ReadVerdict::Valid;

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

        // The global index may not stray outside 1..=90.
        let mut bad = measurement(0.01);
        bad.measurement_index = Some(91);
        let line = bad.encode().unwrap();
        assert!(matches!(
            Row::parse(&line),
            Err(CodecError::BadField {
                column: "measurement_index",
                ..
            })
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
