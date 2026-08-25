//! RC-021 run manifest — Amendment 2 §C10.1, §C10.2, §C10.3, §C13.1.
//!
//! The manifest fixes the run's identity. It is written once, `create_new`,
//! never overwritten, never renamed, and never staged through a temporary file.
//!
//! Two rules make the codec byte-exact: **fixed field order** and a **canonical
//! re-render comparison** on read. A value that parses but is not in the frozen
//! compact form is rejected, so two writers cannot disagree and a verifier can
//! recompute the bytes.

#![allow(dead_code)] // Consumers arrive in later commits of the §12 plan.

use crate::host::{self, DiagProbe, HostFields, HOST_FIELD_KEYS};
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// §C13.1 field 1.
pub const SCHEMA_VERSION: &str = "rc021/1";
/// §C5 — the only permitted `cpu_time_unit`.
pub const CPU_TIME_UNIT: &str = "linux_clock_ticks";

/// §C13.1 — exactly eighteen fields, in this order.
pub const MANIFEST_KEYS: [&str; 18] = [
    "schema_version",
    "run_uuid",
    "boot_id",
    "run_start_uptime_ms",
    "run_dir",
    "repo_commit",
    "prereg_commit",
    "amendment_commits",
    "instrument_birth_commit",
    "host_fingerprint",
    "host_fields",
    "cpu_set",
    "thread_count",
    "timer_resolution_ms",
    "cpu_time_unit",
    "command_line",
    "utc_start",
    "diag_availability",
];

/// §C10.3 — every reserved path, relative to the run directory.
pub const RESERVED_PATHS: [&str; 15] = [
    "run.json",
    "controls_started.json",
    "controls_complete.json",
    "rc021_closure.json",
    "run_invalid.json",
    "rc021_journal_s1.tsv",
    "rc021_journal_s2.tsv",
    "rc021_journal_s3.tsv",
    "rc021_journal_s4.tsv",
    "rc021_journal_s5.tsv",
    "rc021_journal_s6.tsv",
    "rc021_observations.tsv",
    "RC021_RESULTS.md",
    "control",
    "control/rc021_control_journal.tsv",
];

// ======================================================================= ERROR

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ManifestError {
    /// §C10.2: a path whose bytes are not UTF-8 cannot be recorded, because a
    /// lossy conversion would make the later exact-match check compare a
    /// substituted string.
    PathNotUtf8(String),
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
    NotCanonical {
        key: &'static str,
        want: String,
        got: String,
    },
    /// §C10.2: a reserved path is already present, so nothing is created.
    ReservedPathPresent(String),
    RunDirMissing(String),
    RunDirNotADirectory(String),
    PathMismatch {
        recorded: String,
        observed: String,
    },
    Io(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::KeyCount { expected, got } => {
                write!(f, "expected {expected} manifest fields, found {got}")
            }
            ManifestError::KeyOrder {
                position,
                expected,
                got,
            } => {
                write!(
                    f,
                    "manifest field {position} must be {expected}, found {got}"
                )
            }
            ManifestError::Type { key, want } => write!(f, "manifest {key} must be a {want}"),
            ManifestError::Domain(m) => write!(f, "manifest domain violation: {m}"),
            ManifestError::Hex { key, len } => {
                write!(f, "manifest {key} must be {len} lower-case hex characters")
            }
            ManifestError::NotCanonical { key, want, got } => {
                write!(f, "manifest {key} is not canonical: want {want}, got {got}")
            }
            ManifestError::ReservedPathPresent(p) => {
                write!(f, "reserved RC-021 path already exists: {p}")
            }
            ManifestError::RunDirMissing(p) => write!(f, "run directory does not exist: {p}"),
            ManifestError::RunDirNotADirectory(p) => write!(f, "not a directory: {p}"),
            ManifestError::PathMismatch { recorded, observed } => {
                write!(
                    f,
                    "run_dir mismatch: manifest {recorded}, observed {observed}"
                )
            }
            ManifestError::PathNotUtf8(p) => {
                write!(f, "path is not valid UTF-8 and cannot be recorded: {p}")
            }
            ManifestError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

// ==================================================================== MANIFEST

/// §C13.1, typed.
///
/// `thread_count` and `timer_resolution_ms` are **validated inputs**, not values
/// this module discovers: no binding document defines a source for either, and
/// inventing one here would freeze a rule the pre-registration never made.
#[derive(Clone, PartialEq, Debug)]
pub struct RunManifest {
    pub run_uuid: String,
    pub boot_id: String,
    pub run_start_uptime_ms: u64,
    pub run_dir: String,
    pub repo_commit: String,
    pub prereg_commit: String,
    pub amendment_commits: Vec<String>,
    pub instrument_birth_commit: String,
    pub host_fingerprint: String,
    pub host_fields: HostFields,
    pub cpu_set: String,
    pub thread_count: u32,
    pub timer_resolution_ms: f64,
    pub cpu_time_unit: String,
    pub command_line: Vec<String>,
    pub utc_start: String,
    pub diag_availability: DiagProbe,
}

/// Strict: `display()` would replace non-UTF-8 bytes with U+FFFD, and the
/// recorded `run_dir` must be the path, not a rendering of it.
fn path_to_string(p: &Path) -> Result<String, ManifestError> {
    p.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| ManifestError::PathNotUtf8(p.to_string_lossy().into_owned()))
}

/// §C10.2: only `NotFound` means a reserved path is absent. Any other error —
/// a permission failure, a broken mount — is a fact about the directory that
/// must stop the run, not be read as "nothing is there".
fn reserved_path_state(p: &Path) -> Result<bool, ManifestError> {
    match p.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(ManifestError::Io(format!("{}: {e}", p.display()))),
    }
}

fn hexlen(s: &str, n: usize, key: &'static str) -> Result<(), ManifestError> {
    if s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(ManifestError::Hex { key, len: n })
    }
}

fn jstr(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn jarr(v: &[String]) -> String {
    let items: Vec<String> = v.iter().map(|s| jstr(s)).collect();
    format!("[{}]", items.join(","))
}

impl RunManifest {
    /// Refuse an invalid manifest **before** any byte is written. The writer
    /// must never produce a file the reader has to reject.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if !host::is_run_uuid(&self.run_uuid) {
            return Err(ManifestError::Hex {
                key: "run_uuid",
                len: 32,
            });
        }
        if self.boot_id.is_empty() {
            return Err(ManifestError::Domain("boot_id must not be empty"));
        }
        if self.run_dir.is_empty() || !Path::new(&self.run_dir).is_absolute() {
            return Err(ManifestError::Domain("run_dir must be an absolute path"));
        }
        hexlen(&self.repo_commit, 40, "repo_commit")?;
        hexlen(&self.prereg_commit, 40, "prereg_commit")?;
        hexlen(&self.instrument_birth_commit, 40, "instrument_birth_commit")?;
        // §C4: exactly two distinct SHAs, Amendment 1 then Amendment 2.
        if self.amendment_commits.len() != 2 {
            return Err(ManifestError::Domain(
                "amendment_commits must hold exactly two SHAs",
            ));
        }
        for c in &self.amendment_commits {
            hexlen(c, 40, "amendment_commits")?;
        }
        if self.amendment_commits[0] == self.amendment_commits[1] {
            return Err(ManifestError::Domain("amendment_commits must be distinct"));
        }
        // §C3: the fields must themselves be in normalised form. Without this
        // a self-consistent but non-normative HostFields would pass simply
        // because its fingerprint was recomputed over its own bad values.

        self.host_fields
            .validate()
            .map_err(|_| ManifestError::Domain("host_fields are not in normalised form"))?;
        hexlen(&self.host_fingerprint, 64, "host_fingerprint")?;
        // §C3: the fingerprint must be the one these fields produce.
        if self.host_fingerprint != self.host_fields.fingerprint() {
            return Err(ManifestError::Domain(
                "host_fingerprint does not match host_fields",
            ));
        }
        // §C11.45: the recorded cpu_set is the one the host reported.
        if self.cpu_set != self.host_fields.cpus_allowed_list {
            return Err(ManifestError::Domain(
                "cpu_set must equal host_fields.cpus_allowed_list",
            ));
        }
        if self.cpu_set.contains('\t') || self.cpu_set.contains('\n') {
            return Err(ManifestError::Domain("cpu_set carries a control character"));
        }
        if !self.timer_resolution_ms.is_finite() {
            return Err(ManifestError::Domain("timer_resolution_ms is not finite"));
        }
        if self.cpu_time_unit != CPU_TIME_UNIT {
            return Err(ManifestError::Domain(
                "cpu_time_unit must be linux_clock_ticks",
            ));
        }
        if self.command_line.is_empty() {
            return Err(ManifestError::Domain("command_line must not be empty"));
        }
        if chrono::DateTime::parse_from_rfc3339(&self.utc_start).is_err() {
            return Err(ManifestError::Domain("utc_start must be RFC 3339"));
        }
        Ok(())
    }

    /// The canonical JSON text of each field, in §C13.1 order. One function
    /// serves the writer and the reader's byte-for-byte check, so the two
    /// cannot drift apart.
    fn canonical_values(&self) -> Result<Vec<String>, ManifestError> {
        self.validate()?;
        let hf = &self.host_fields;
        let host_fields_json = {
            let pairs: Vec<String> = HOST_FIELD_KEYS
                .iter()
                .zip(hf.values())
                .map(|(k, v)| format!("{}:{}", jstr(k), jstr(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        };
        let diag = format!(
            "{{\"cpu_time\":{},\"ctx_switches\":{},\"freq\":{}}}",
            self.diag_availability.cpu_time,
            self.diag_availability.ctx_switches,
            self.diag_availability.freq
        );
        Ok(vec![
            jstr(SCHEMA_VERSION),
            jstr(&self.run_uuid),
            jstr(&self.boot_id),
            self.run_start_uptime_ms.to_string(),
            jstr(&self.run_dir),
            jstr(&self.repo_commit),
            jstr(&self.prereg_commit),
            jarr(&self.amendment_commits),
            jstr(&self.instrument_birth_commit),
            jstr(&self.host_fingerprint),
            host_fields_json,
            jstr(&self.cpu_set),
            self.thread_count.to_string(),
            // §C8.1's rule applies to every f64 the cycle writes.
            self.timer_resolution_ms.to_string(),
            jstr(&self.cpu_time_unit),
            jarr(&self.command_line),
            jstr(&self.utc_start),
            diag,
        ])
    }

    /// Compact JSON, fixed field order, UTF-8, one trailing line feed.
    pub fn render(&self) -> Result<String, ManifestError> {
        let values = self.canonical_values()?;
        debug_assert_eq!(values.len(), MANIFEST_KEYS.len());
        let body: Vec<String> = MANIFEST_KEYS
            .iter()
            .zip(values.iter())
            .map(|(k, v)| format!("{}:{}", jstr(k), v))
            .collect();
        Ok(format!("{{{}}}\n", body.join(",")))
    }

    /// Strict typed read. The positional key check catches a missing, extra,
    /// duplicated or reordered field in one comparison; the canonical
    /// re-render catches a value that parses but is not in the frozen form.
    pub fn parse(text: &str) -> Result<RunManifest, ManifestError> {
        let body = text.strip_suffix('\n').ok_or(ManifestError::Domain(
            "manifest must end with one line feed",
        ))?;
        if body.contains('\n') {
            return Err(ManifestError::Domain("manifest must be a single line"));
        }
        let v: serde_json::Value =
            serde_json::from_str(body).map_err(|e| ManifestError::Io(e.to_string()))?;
        let obj = v
            .as_object()
            .ok_or(ManifestError::Domain("manifest must be a JSON object"))?;
        if obj.len() != MANIFEST_KEYS.len() {
            return Err(ManifestError::KeyCount {
                expected: MANIFEST_KEYS.len(),
                got: obj.len(),
            });
        }
        // Order is not preserved by the parsed map, so it is checked on the raw
        // text: the keys must appear in §C13.1 order.
        let mut cursor = 0usize;
        for (i, k) in MANIFEST_KEYS.iter().enumerate() {
            let needle = format!("{}:", jstr(k));
            match body[cursor..].find(&needle) {
                Some(off) => cursor += off + needle.len(),
                None => {
                    return Err(ManifestError::KeyOrder {
                        position: i,
                        expected: k,
                        got: "absent or out of order".to_string(),
                    })
                }
            }
        }

        let s = |k: &'static str| -> Result<String, ManifestError> {
            obj.get(k)
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
                .ok_or(ManifestError::Type {
                    key: k,
                    want: "string",
                })
        };
        let u = |k: &'static str| -> Result<u64, ManifestError> {
            obj.get(k)
                .and_then(|x| x.as_u64())
                .ok_or(ManifestError::Type {
                    key: k,
                    want: "unsigned integer",
                })
        };
        let arr = |k: &'static str| -> Result<Vec<String>, ManifestError> {
            let a = obj
                .get(k)
                .and_then(|x| x.as_array())
                .ok_or(ManifestError::Type {
                    key: k,
                    want: "array of strings",
                })?;
            a.iter()
                .map(|x| {
                    x.as_str()
                        .map(|y| y.to_string())
                        .ok_or(ManifestError::Type {
                            key: k,
                            want: "array of strings",
                        })
                })
                .collect()
        };
        let hf_obj =
            obj.get("host_fields")
                .and_then(|x| x.as_object())
                .ok_or(ManifestError::Type {
                    key: "host_fields",
                    want: "object of six strings",
                })?;
        if hf_obj.len() != HOST_FIELD_KEYS.len() {
            return Err(ManifestError::Type {
                key: "host_fields",
                want: "object of exactly six strings",
            });
        }
        let hfv = |k: &str| -> Result<String, ManifestError> {
            hf_obj
                .get(k)
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
                .ok_or(ManifestError::Type {
                    key: "host_fields",
                    want: "object of six strings",
                })
        };
        let d_obj = obj
            .get("diag_availability")
            .and_then(|x| x.as_object())
            .ok_or(ManifestError::Type {
                key: "diag_availability",
                want: "object of three booleans",
            })?;
        if d_obj.len() != 3 {
            return Err(ManifestError::Type {
                key: "diag_availability",
                want: "object of exactly three booleans",
            });
        }
        let db = |k: &str| -> Result<bool, ManifestError> {
            d_obj
                .get(k)
                .and_then(|x| x.as_bool())
                .ok_or(ManifestError::Type {
                    key: "diag_availability",
                    want: "boolean member",
                })
        };
        let tr = obj
            .get("timer_resolution_ms")
            .and_then(|x| x.as_f64())
            .filter(|x| x.is_finite())
            .ok_or(ManifestError::Type {
                key: "timer_resolution_ms",
                want: "finite number",
            })?;

        if s("schema_version")? != SCHEMA_VERSION {
            return Err(ManifestError::Domain("schema_version must be rc021/1"));
        }
        let thread_count = u32::try_from(u("thread_count")?).map_err(|_| ManifestError::Type {
            key: "thread_count",
            want: "u32",
        })?;

        let m = RunManifest {
            run_uuid: s("run_uuid")?,
            boot_id: s("boot_id")?,
            run_start_uptime_ms: u("run_start_uptime_ms")?,
            run_dir: s("run_dir")?,
            repo_commit: s("repo_commit")?,
            prereg_commit: s("prereg_commit")?,
            amendment_commits: arr("amendment_commits")?,
            instrument_birth_commit: s("instrument_birth_commit")?,
            host_fingerprint: s("host_fingerprint")?,
            host_fields: HostFields {
                kernel_release: hfv("kernel_release")?,
                kernel_version: hfv("kernel_version")?,
                available_processors: hfv("available_processors")?,
                mem_total_kb: hfv("mem_total_kb")?,
                cpus_allowed_list: hfv("cpus_allowed_list")?,
                cpu_model: hfv("cpu_model")?,
            },
            cpu_set: s("cpu_set")?,
            thread_count,
            timer_resolution_ms: tr,
            cpu_time_unit: s("cpu_time_unit")?,
            command_line: arr("command_line")?,
            utc_start: s("utc_start")?,
            diag_availability: DiagProbe {
                cpu_time: db("cpu_time")?,
                ctx_switches: db("ctx_switches")?,
                freq: db("freq")?,
            },
        };
        // Byte-for-byte: anything that parses but is not the frozen form — a
        // reordered object, added spaces, a non-round-trip number — is rejected.
        if m.render()? != text {
            return Err(ManifestError::NotCanonical {
                key: "manifest",
                want: m.render()?,
                got: text.to_string(),
            });
        }
        Ok(m)
    }

    /// The recorded `run_dir` must equal the canonical form of the directory a
    /// later mode was pointed at (§C10.2).
    pub fn check_run_dir(&self, observed: &Path) -> Result<(), ManifestError> {
        let canon = path_to_string(
            &std::fs::canonicalize(observed).map_err(|e| ManifestError::Io(e.to_string()))?,
        )?;
        if canon == self.run_dir {
            Ok(())
        } else {
            Err(ManifestError::PathMismatch {
                recorded: self.run_dir.clone(),
                observed: canon,
            })
        }
    }
}

// =============================================================== RUN DIRECTORY

/// §C10.2 preflight, read-only. `D` must already exist; this never creates it,
/// and it creates nothing at all when any reserved path is present.
pub fn preflight_run_dir(dir: &Path) -> Result<PathBuf, ManifestError> {
    if !dir.exists() {
        return Err(ManifestError::RunDirMissing(dir.display().to_string()));
    }
    if !dir.is_dir() {
        return Err(ManifestError::RunDirNotADirectory(
            dir.display().to_string(),
        ));
    }
    for rel in RESERVED_PATHS {
        // `symlink_metadata` so a dangling symlink still counts as present.
        if reserved_path_state(&dir.join(rel))? {
            return Err(ManifestError::ReservedPathPresent(rel.to_string()));
        }
    }
    // §C10.2 reserves `rc021_journal_s*.tsv` as a pattern, not only s1..s6: a
    // stray `s7` or `sx` is still a reserved name and must block the run.
    let rd = std::fs::read_dir(dir).map_err(|e| ManifestError::Io(e.to_string()))?;
    for entry in rd {
        let e = entry.map_err(|err| ManifestError::Io(err.to_string()))?;
        let raw = e.file_name();
        // Byte-wise: `rc021_journal_s\xff.tsv` matches the reserved shape even
        // though it is not UTF-8, and must not slip through a `to_str()` gate.
        let bytes = raw.as_encoded_bytes();
        if bytes.starts_with(b"rc021_journal_s") && bytes.ends_with(b".tsv") {
            return Err(ManifestError::ReservedPathPresent(
                raw.to_string_lossy().into_owned(),
            ));
        }
    }
    let canon = std::fs::canonicalize(dir).map_err(|e| ManifestError::Io(e.to_string()))?;
    // Reject a non-UTF-8 run directory here, before anything is created.
    path_to_string(&canon)?;
    Ok(canon)
}

fn fsync_dir(dir: &Path) -> Result<(), ManifestError> {
    File::open(dir)
        .and_then(|f| f.sync_all())
        .map_err(|e| ManifestError::Io(e.to_string()))
}

/// §C10.2: after a complete preflight, create `<D>/control/`, fsync `D`, then
/// write `<D>/run.json`. Nothing is created before the preflight passes, and
/// the manifest is rendered — and therefore validated — before the path is
/// claimed.
pub fn init_run(dir: &Path, manifest: &RunManifest) -> Result<PathBuf, ManifestError> {
    let canon = preflight_run_dir(dir)?;
    let canon_str = path_to_string(&canon)?;
    if manifest.run_dir != canon_str {
        return Err(ManifestError::PathMismatch {
            recorded: manifest.run_dir.clone(),
            observed: canon_str,
        });
    }
    // Render first: invalid input must not leave a directory or a file behind.
    let body = manifest.render()?;

    std::fs::create_dir(canon.join("control")).map_err(|e| ManifestError::Io(e.to_string()))?;
    fsync_dir(&canon)?;

    let path = canon.join("run.json");
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| ManifestError::Io(e.to_string()))?;
    f.write_all(body.as_bytes())
        .and_then(|()| f.flush())
        .and_then(|()| f.sync_all())
        .map_err(|e| ManifestError::Io(e.to_string()))?;
    fsync_dir(&canon)?;
    Ok(path)
}

/// Read and strictly validate a manifest from disk.
pub fn read_manifest(path: &Path) -> Result<RunManifest, ManifestError> {
    let text = std::fs::read_to_string(path).map_err(|e| ManifestError::Io(e.to_string()))?;
    RunManifest::parse(&text)
}

// ======================================================================= TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(name: &str) -> TempDir {
            let p = std::env::temp_dir().join(format!(
                "rc021_manifest_{}_{}_{}",
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

    fn manifest_for(dir: &Path) -> RunManifest {
        let hf = fields();
        RunManifest {
            run_uuid: "0".repeat(32),
            boot_id: "boot-1".into(),
            run_start_uptime_ms: 1_234_567,
            run_dir: std::fs::canonicalize(dir).unwrap().display().to_string(),
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
            command_line: vec!["exp_rc021_host_qualify".into(), "--init-run".into()],
            utc_start: "2026-08-25T00:00:00Z".into(),
            diag_availability: DiagProbe {
                cpu_time: true,
                ctx_switches: true,
                freq: false,
            },
        }
    }

    /// §C10.2 reserves the *pattern*, so a journal outside s1..s6 is still a
    /// reserved name and still blocks the run.
    #[test]
    fn any_journal_shaped_name_blocks_the_run() {
        for stray in [
            "rc021_journal_s7.tsv",
            "rc021_journal_sx.tsv",
            "rc021_journal_s.tsv",
        ] {
            let d = TempDir::new("wildcard");
            std::fs::write(d.path().join(stray), b"leftover").unwrap();
            let m = manifest_for(d.path());
            let err = init_run(d.path(), &m).unwrap_err();
            assert!(
                matches!(&err, ManifestError::ReservedPathPresent(p) if p == stray),
                "{stray}: {err}"
            );
            // A refusal must leave the directory exactly as it was.
            assert!(
                !d.path().join("control").exists(),
                "{stray}: control/ created"
            );
            assert!(
                !d.path().join("run.json").exists(),
                "{stray}: run.json created"
            );
        }
    }

    /// A name that merely looks similar is not reserved, or the instrument
    /// would refuse to run beside unrelated files.
    #[test]
    fn similar_but_unreserved_names_do_not_block() {
        let d = TempDir::new("wildcard_ok");
        for ok in [
            "rc021_journal_s1.tsv.bak",
            "rc020_journal_s1.tsv",
            "notes.tsv",
        ] {
            std::fs::write(d.path().join(ok), b"x").unwrap();
        }
        assert!(preflight_run_dir(d.path()).is_ok());
    }

    /// Only `NotFound` means absent. Any other error is a fact about the
    /// directory, and reading it as "nothing is there" would let the run start
    /// on top of state it could not see.
    #[test]
    fn reserved_path_errors_are_not_read_as_absence() {
        let d = TempDir::new("classify");
        let file = d.path().join("occupied");
        std::fs::write(&file, b"x").unwrap();

        assert!(!reserved_path_state(&d.path().join("nothing")).unwrap());
        assert!(reserved_path_state(&file).unwrap());

        // A path *through* a regular file yields ENOTDIR, not ENOENT. This is
        // deterministic and, unlike a permission fixture, not bypassed by root.
        let through = file.join("child");
        assert!(
            std::fs::symlink_metadata(&through).is_err(),
            "fixture must produce an error"
        );
        assert!(
            matches!(reserved_path_state(&through), Err(ManifestError::Io(_))),
            "a non-NotFound error must surface, not be read as absence"
        );
    }

    /// `display()` substitutes U+FFFD, so the recorded `run_dir` would no
    /// longer be the path and the later exact-match check would compare a
    /// rendering against a rendering.
    /// A non-UTF-8 name can still have the reserved shape.
    #[cfg(unix)]
    #[test]
    fn non_utf8_journal_name_also_blocks_the_run() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let d = TempDir::new("wildcard_bytes");
        let mut raw = b"rc021_journal_s".to_vec();
        raw.extend_from_slice(b"\xff\xfe.tsv");
        std::fs::write(d.path().join(OsString::from_vec(raw)), b"x").unwrap();
        assert!(matches!(
            preflight_run_dir(d.path()).unwrap_err(),
            ManifestError::ReservedPathPresent(_)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_run_dir_is_refused_before_anything_is_created() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let parent = TempDir::new("nonutf8");
        let raw = OsString::from_vec(b"run_\xff\xfe".to_vec());
        let dir = parent.path().join(&raw);
        std::fs::create_dir(&dir).unwrap();

        let err = preflight_run_dir(&dir).unwrap_err();
        assert!(matches!(err, ManifestError::PathNotUtf8(_)), "{err}");

        let mut m = manifest_for(parent.path());
        m.run_dir = dir.to_string_lossy().into_owned();
        assert!(matches!(
            init_run(&dir, &m).unwrap_err(),
            ManifestError::PathNotUtf8(_)
        ));
        assert!(!dir.join("control").exists());
        assert!(!dir.join("run.json").exists());
    }

    /// The fingerprint is computed over whatever the fields hold, so a
    /// self-consistent manifest can still carry non-normalised host fields.
    /// The manifest must reject them on its own.
    #[test]
    fn malformed_host_fields_are_rejected_even_when_self_consistent() {
        let d = TempDir::new("hostfields");
        let mut m = manifest_for(d.path());
        m.host_fields.cpu_model = "Test   CPU".into(); // not collapsed
        m.host_fingerprint = m.host_fields.fingerprint(); // consistent, still wrong
        let err = m.validate().unwrap_err();
        assert!(matches!(err, ManifestError::Domain(_)), "{err}");

        // and the writer refuses before creating anything
        assert!(init_run(d.path(), &m).is_err());
        assert!(!d.path().join("control").exists());
        assert!(!d.path().join("run.json").exists());
    }

    /// The reader half, as its own test: inside the combined test the writer
    /// assertion fails first, so the reader path would never be reached and
    /// would carry no load of its own.
    ///
    /// Doctoring only the field would leave the fingerprint stale, and the
    /// test would then pass on the SHA mismatch without proving anything about
    /// normalisation — so the fingerprint is recomputed for the malformed
    /// fields as well.
    #[test]
    fn reader_rejects_a_self_consistent_but_malformed_run_json() {
        let d = TempDir::new("hostfields_reader");
        let good = manifest_for(d.path());
        init_run(d.path(), &good).unwrap();
        let text = std::fs::read_to_string(d.path().join("run.json")).unwrap();

        let mut doctored_fields = good.host_fields.clone();
        doctored_fields.cpu_model = "Test   CPU".into();
        let doctored_fp = doctored_fields.fingerprint();
        assert_ne!(doctored_fp, good.host_fingerprint);

        let broken = text
            .replace(&good.host_fingerprint, &doctored_fp)
            .replace("\"Test CPU\"", "\"Test   CPU\"");
        assert_ne!(broken, text);

        // This JSON is internally consistent: the recorded fingerprint IS the
        // one its own host_fields produce. Every check except the normalised
        // form would accept it.
        assert!(broken.contains(&doctored_fp));
        assert!(broken.contains("Test   CPU"));

        let err = RunManifest::parse(&broken).unwrap_err();
        assert!(
            matches!(
                err,
                ManifestError::Domain("host_fields are not in normalised form")
            ),
            "must be rejected by normalisation, not by the fingerprint: {err}"
        );
    }

    #[test]
    fn eighteen_fields_in_order_with_one_trailing_lf() {
        let d = TempDir::new("order");
        let text = manifest_for(d.path()).render().unwrap();
        assert!(text.ends_with("}\n"));
        assert_eq!(text.matches('\n').count(), 1, "exactly one trailing LF");
        // Compact: no space after a separator. Values may contain spaces
        // ("#1 SMP"), so a blanket space check would be wrong.
        assert!(!text.contains("\": "), "compact: no space after a colon");
        assert!(!text.contains(", \""), "compact: no space after a comma");
        // keys appear in §C13.1 order and nowhere else
        let mut cursor = 0usize;
        for k in MANIFEST_KEYS {
            let needle = format!("\"{k}\":");
            let at = text[cursor..]
                .find(&needle)
                .unwrap_or_else(|| panic!("{k} missing or out of order"));
            cursor += at + needle.len();
        }
        assert_eq!(
            text.matches("\":").count(),
            MANIFEST_KEYS.len() + HOST_FIELD_KEYS.len() + 3,
            "18 top level + 6 host_fields + 3 diag"
        );
    }

    #[test]
    fn canonical_round_trip() {
        let d = TempDir::new("round");
        let m = manifest_for(d.path());
        let text = m.render().unwrap();
        let back = RunManifest::parse(&text).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.render().unwrap(), text);
        assert_eq!(
            back.timer_resolution_ms.to_bits(),
            m.timer_resolution_ms.to_bits(),
            "shortest round-trip preserves bits"
        );
    }

    #[test]
    fn missing_extra_reordered_and_mistyped_fields_are_rejected() {
        let d = TempDir::new("shape");
        let text = manifest_for(d.path()).render().unwrap();

        // missing
        let missing = text.replacen("\"boot_id\":\"boot-1\",", "", 1);
        assert!(matches!(
            RunManifest::parse(&missing),
            Err(ManifestError::KeyCount { .. })
        ));
        // extra
        let extra = text.replacen("{", "{\"surplus\":1,", 1);
        assert!(matches!(
            RunManifest::parse(&extra),
            Err(ManifestError::KeyCount { .. })
        ));
        // reordered: swap the first two keys, count unchanged
        let reordered = text.replacen(
            "\"schema_version\":\"rc021/1\",\"run_uuid\":\"00000000000000000000000000000000\"",
            "\"run_uuid\":\"00000000000000000000000000000000\",\"schema_version\":\"rc021/1\"",
            1,
        );
        assert!(matches!(
            RunManifest::parse(&reordered),
            Err(ManifestError::KeyOrder { .. })
        ));
        // wrong type
        let mistyped = text.replacen("\"thread_count\":1", "\"thread_count\":\"1\"", 1);
        assert!(RunManifest::parse(&mistyped).is_err());
        // missing trailing LF, and a second line
        assert!(RunManifest::parse(text.trim_end_matches('\n')).is_err());
        assert!(RunManifest::parse(&format!("{text}extra\n")).is_err());
    }

    #[test]
    fn invalid_domains_are_rejected_by_writer_and_reader() {
        let d = TempDir::new("domains");
        let base = manifest_for(d.path());
        type Mutate = Box<dyn Fn(&mut RunManifest)>;
        let cases: Vec<(&str, Mutate)> = vec![
            (
                "run_uuid",
                Box::new(|m: &mut RunManifest| m.run_uuid = "short".into()),
            ),
            (
                "repo_commit",
                Box::new(|m: &mut RunManifest| m.repo_commit = "a".repeat(39)),
            ),
            (
                "upper hex",
                Box::new(|m: &mut RunManifest| m.prereg_commit = "A".repeat(40)),
            ),
            (
                "amendment count",
                Box::new(|m: &mut RunManifest| m.amendment_commits = vec!["c".repeat(40)]),
            ),
            (
                "amendment duplicate",
                Box::new(|m: &mut RunManifest| {
                    m.amendment_commits = vec!["c".repeat(40), "c".repeat(40)]
                }),
            ),
            (
                "fingerprint mismatch",
                Box::new(|m: &mut RunManifest| m.host_fingerprint = "f".repeat(64)),
            ),
            (
                "cpu_set mismatch",
                Box::new(|m: &mut RunManifest| m.cpu_set = "0-7".into()),
            ),
            (
                "nonfinite",
                Box::new(|m: &mut RunManifest| m.timer_resolution_ms = f64::NAN),
            ),
            (
                "cpu_time_unit",
                Box::new(|m: &mut RunManifest| m.cpu_time_unit = "seconds".into()),
            ),
            (
                "rfc3339",
                Box::new(|m: &mut RunManifest| m.utc_start = "yesterday".into()),
            ),
            (
                "rfc3339 no tz",
                Box::new(|m: &mut RunManifest| m.utc_start = "2026-08-25T00:00:00".into()),
            ),
            (
                "relative run_dir",
                Box::new(|m: &mut RunManifest| m.run_dir = "rel".into()),
            ),
            (
                "empty command_line",
                Box::new(|m: &mut RunManifest| m.command_line = vec![]),
            ),
        ];
        for (what, mutate) in cases {
            let mut m = base.clone();
            mutate(&mut m);
            assert!(m.validate().is_err(), "validate must reject {what}");
            assert!(m.render().is_err(), "writer must refuse {what}");
        }
        assert!(base.validate().is_ok());
    }

    #[test]
    fn non_canonical_text_is_rejected() {
        let d = TempDir::new("canon");
        let text = manifest_for(d.path()).render().unwrap();
        // parses to the same value, but is not the frozen form
        let spaced = text.replacen("\"boot_id\":", "\"boot_id\": ", 1);
        assert!(RunManifest::parse(&spaced).is_err());
        // reordered host_fields members
        let hf = text.replacen(
            "\"kernel_release\":\"6.6.0\",\"kernel_version\":\"#1 SMP\"",
            "\"kernel_version\":\"#1 SMP\",\"kernel_release\":\"6.6.0\"",
            1,
        );
        assert!(matches!(
            RunManifest::parse(&hf),
            Err(ManifestError::NotCanonical { .. })
        ));
    }

    #[test]
    fn preflight_refuses_every_reserved_path_and_creates_nothing() {
        for rel in RESERVED_PATHS {
            let d = TempDir::new("reserved");
            let p = d.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            if rel == "control" {
                std::fs::create_dir(&p).unwrap();
            } else {
                std::fs::write(&p, b"x").unwrap();
            }
            let before: Vec<_> = std::fs::read_dir(d.path())
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            let m = manifest_for(d.path());
            assert!(
                matches!(
                    init_run(d.path(), &m),
                    Err(ManifestError::ReservedPathPresent(_))
                ),
                "must refuse when {rel} exists"
            );
            let after: Vec<_> = std::fs::read_dir(d.path())
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(before.len(), after.len(), "{rel}: nothing may be created");
            assert!(!d.path().join("run.json").exists() || rel == "run.json");
        }
    }

    #[test]
    fn init_run_writes_control_dir_then_manifest() {
        let d = TempDir::new("init");
        let m = manifest_for(d.path());
        let p = init_run(d.path(), &m).unwrap();
        assert!(d.path().join("control").is_dir());
        assert_eq!(p, std::fs::canonicalize(d.path()).unwrap().join("run.json"));
        let back = read_manifest(&p).unwrap();
        assert_eq!(back, m);
        // create_new: a second init cannot overwrite
        let before = std::fs::read(&p).unwrap();
        assert!(init_run(d.path(), &m).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), before, "bytes untouched");
    }

    #[test]
    fn init_run_creates_nothing_when_the_manifest_is_invalid() {
        let d = TempDir::new("bad_manifest");
        let mut m = manifest_for(d.path());
        m.cpu_time_unit = "seconds".into();
        assert!(init_run(d.path(), &m).is_err());
        assert!(!d.path().join("run.json").exists());
        // the control directory is created only after the render succeeds
        assert!(!d.path().join("control").exists());
    }

    #[test]
    fn missing_or_non_directory_run_dir_is_refused() {
        let d = TempDir::new("missing");
        let absent = d.path().join("nope");
        let m = manifest_for(d.path());
        assert!(matches!(
            preflight_run_dir(&absent),
            Err(ManifestError::RunDirMissing(_))
        ));
        let file = d.path().join("afile");
        std::fs::write(&file, b"x").unwrap();
        assert!(matches!(
            preflight_run_dir(&file),
            Err(ManifestError::RunDirNotADirectory(_))
        ));
        assert!(init_run(&absent, &m).is_err());
    }

    #[test]
    fn canonical_path_mismatch_is_detected() {
        let d = TempDir::new("path");
        let e = TempDir::new("other");
        let m = manifest_for(d.path());
        // the manifest names d, but init is pointed at e
        assert!(matches!(
            init_run(e.path(), &m),
            Err(ManifestError::PathMismatch { .. })
        ));
        assert!(!e.path().join("run.json").exists());
        // and a later mode checks the same way
        let p = init_run(d.path(), &m).unwrap();
        let back = read_manifest(&p).unwrap();
        assert!(back.check_run_dir(d.path()).is_ok());
        assert!(matches!(
            back.check_run_dir(e.path()),
            Err(ManifestError::PathMismatch { .. })
        ));
    }

    #[test]
    fn a_corrupt_manifest_on_disk_is_refused() {
        let d = TempDir::new("corrupt");
        let m = manifest_for(d.path());
        let p = init_run(d.path(), &m).unwrap();
        let good = std::fs::read_to_string(&p).unwrap();
        for bad in [
            good.replacen("rc021/1", "rc021/2", 1),
            good.replacen('{', "", 1),
            good.replacen("\"thread_count\":1", "\"thread_count\":-1", 1),
            String::new(),
        ] {
            std::fs::write(&p, &bad).unwrap();
            assert!(read_manifest(&p).is_err(), "must refuse {bad:?}");
        }
    }
}
