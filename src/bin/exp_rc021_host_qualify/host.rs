//! RC-021 host identity, clock and diagnostics — Amendment 2 §C2, §C3, §C6.1, §C12.
//!
//! Everything here reads; nothing here decides. The instrument **never sets CPU
//! affinity** (§C11.45) — it records `Cpus_allowed_list` and later modes require
//! an exact match.
//!
//! Every parser is a pure function over text or over an injected root, so the
//! unit tests below do not depend on the `/proc` of the machine running them.

#![allow(dead_code)] // Consumers arrive in later commits of the §12 plan.

use rand::RngCore;
use std::path::Path;

// ===================================================================== SHA-256
//
// Adapted unchanged from the reviewed implementation in
// `src/bin/exp_marginal_cost.rs:209`. No dependency, no `unsafe`, no external
// process. Pinned by a known-answer test.

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256_hex(data: &[u8]) -> String {
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
    let mut w = [0u32; 64];
    for chunk in msg.chunks_exact(64) {
        for (i, wi) in w.iter_mut().enumerate().take(16) {
            let j = i * 4;
            *wi = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
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
                .wrapping_add(K256[i])
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
        for (hv, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *hv = hv.wrapping_add(v);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

// ======================================================================= ERROR

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HostError {
    Read {
        path: String,
        why: String,
    },
    Parse {
        what: &'static str,
        value: String,
    },
    Overflow(&'static str),
    /// §C12: the monotonic axis is broken — a different boot, or a uptime that
    /// went backwards.
    BootMismatch {
        expected: String,
        observed: String,
    },
    ClockWentBackwards {
        start_ms: u64,
        current_ms: u64,
    },
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::Read { path, why } => write!(f, "cannot read {path}: {why}"),
            HostError::Parse { what, value } => write!(f, "cannot parse {what} from {value:?}"),
            HostError::Overflow(w) => write!(f, "arithmetic overflow in {w}"),
            HostError::BootMismatch { expected, observed } => {
                write!(
                    f,
                    "boot_id changed: expected {expected}, observed {observed}"
                )
            }
            HostError::ClockWentBackwards {
                start_ms,
                current_ms,
            } => write!(
                f,
                "uptime went backwards: start {start_ms} ms, current {current_ms} ms"
            ),
        }
    }
}

fn read_to_string(path: &Path) -> Result<String, HostError> {
    std::fs::read_to_string(path).map_err(|e| HostError::Read {
        path: path.display().to_string(),
        why: e.to_string(),
    })
}

// ==================================================================== RUN UUID

/// §C2: sixteen bytes from `rand::rngs::OsRng`, lower-case hex, exactly 32
/// characters. This is an identifier and **never** a scientific seed; it enters
/// no measurement.
pub fn new_run_uuid() -> String {
    let mut b = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The shape a valid `run_uuid` must have.
pub fn is_run_uuid(s: &str) -> bool {
    s.len() == 32
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// ======================================================================= CLOCK

/// §C12: the frozen decimal parser. `/proc/uptime` is **not** parsed as an
/// `f64`; the fraction is truncated after the third digit, never rounded, and
/// a short fraction is zero-padded on the right.
pub fn parse_uptime_ms(text: &str) -> Result<u64, HostError> {
    let bad = |v: &str| HostError::Parse {
        what: "/proc/uptime",
        value: v.to_string(),
    };
    let token = text.split_whitespace().next().ok_or_else(|| bad(text))?;
    let (int_part, frac_part) = match token.split_once('.') {
        Some((i, f)) => (i, f),
        None => (token, ""),
    };
    if int_part.is_empty() || !int_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad(token));
    }
    if !frac_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad(token));
    }
    let seconds: u64 = int_part.parse().map_err(|_| bad(token))?;
    // Truncate after three digits, then pad on the right.
    let mut millis_text: String = frac_part.chars().take(3).collect();
    while millis_text.len() < 3 {
        millis_text.push('0');
    }
    let millis: u64 = millis_text.parse().map_err(|_| bad(token))?;
    seconds
        .checked_mul(1000)
        .and_then(|s| s.checked_add(millis))
        .ok_or(HostError::Overflow("uptime milliseconds"))
}

pub fn read_uptime_ms(proc_uptime: &Path) -> Result<u64, HostError> {
    parse_uptime_ms(&read_to_string(proc_uptime)?)
}

/// `/proc/sys/kernel/random/boot_id`, trailing newline dropped.
pub fn parse_boot_id(text: &str) -> Result<String, HostError> {
    let t = text.trim_end_matches('\n').trim();
    if t.is_empty() {
        return Err(HostError::Parse {
            what: "boot_id",
            value: text.to_string(),
        });
    }
    Ok(t.to_string())
}

/// §C12: the cross-process coordinate. Both failure modes are errors, and
/// neither writes anything — the caller decides whether a refusal or a
/// `run_invalid.json` follows (§C12.3), which is a later commit's concern.
pub fn monotonic_offset_ms(
    manifest_boot_id: &str,
    current_boot_id: &str,
    run_start_uptime_ms: u64,
    current_uptime_ms: u64,
) -> Result<u64, HostError> {
    if manifest_boot_id != current_boot_id {
        return Err(HostError::BootMismatch {
            expected: manifest_boot_id.to_string(),
            observed: current_boot_id.to_string(),
        });
    }
    current_uptime_ms
        .checked_sub(run_start_uptime_ms)
        .ok_or(HostError::ClockWentBackwards {
            start_ms: run_start_uptime_ms,
            current_ms: current_uptime_ms,
        })
}

// ================================================================ HOST FIELDS

/// §C3 — the six keys, in this order. The order is part of the fingerprint.
pub const HOST_FIELD_KEYS: [&str; 6] = [
    "kernel_release",
    "kernel_version",
    "available_processors",
    "mem_total_kb",
    "cpus_allowed_list",
    "cpu_model",
];

/// The six normalised values behind `host_fingerprint`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HostFields {
    pub kernel_release: String,
    pub kernel_version: String,
    pub available_processors: String,
    pub mem_total_kb: String,
    pub cpus_allowed_list: String,
    pub cpu_model: String,
}

/// A canonical decimal integer: digits only, non-empty, and no leading zero
/// unless the value is exactly `0`.
pub fn is_canonical_decimal(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'))
}

fn has_control_chars(s: &str) -> bool {
    s.chars().any(|c| c.is_control())
}

impl HostFields {
    /// §C3: the fields must already be in their normalised form.
    ///
    /// Recomputing the fingerprint over malformed values would make it
    /// self-consistent and still wrong — the fingerprint would faithfully
    /// identify a host description that no source could have produced. The
    /// normalised form is therefore checked, not assumed.
    pub fn validate(&self) -> Result<(), HostError> {
        let bad = |what: &'static str, v: &str| HostError::Parse {
            what,
            value: v.to_string(),
        };
        for (what, v) in [
            ("kernel_release", &self.kernel_release),
            ("kernel_version", &self.kernel_version),
        ] {
            if v.is_empty() || v.contains('\n') || v.contains('\r') {
                return Err(bad(what, v));
            }
        }
        if !is_canonical_decimal(&self.available_processors) {
            return Err(bad("available_processors", &self.available_processors));
        }
        if !is_canonical_decimal(&self.mem_total_kb) {
            return Err(bad("mem_total_kb", &self.mem_total_kb));
        }
        if self.cpus_allowed_list.is_empty() || has_control_chars(&self.cpus_allowed_list) {
            return Err(bad("cpus_allowed_list", &self.cpus_allowed_list));
        }
        // Syntactically a CPU list, not merely a non-empty string.
        parse_cpu_list(&self.cpus_allowed_list)?;
        if self.cpu_model.is_empty() || has_control_chars(&self.cpu_model) {
            return Err(bad("cpu_model", &self.cpu_model));
        }
        // Already collapsed: normalising again must be a no-op.
        let collapsed = self
            .cpu_model
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if collapsed != self.cpu_model {
            return Err(bad("cpu_model", &self.cpu_model));
        }
        Ok(())
    }

    pub fn values(&self) -> [&str; 6] {
        [
            &self.kernel_release,
            &self.kernel_version,
            &self.available_processors,
            &self.mem_total_kb,
            &self.cpus_allowed_list,
            &self.cpu_model,
        ]
    }

    /// §C3: `key=value` and a line feed for each of the six, in order.
    pub fn canonical_input(&self) -> String {
        let mut s = String::new();
        for (k, v) in HOST_FIELD_KEYS.iter().zip(self.values()) {
            s.push_str(k);
            s.push('=');
            s.push_str(v);
            s.push('\n');
        }
        s
    }

    /// §C3: SHA-256 of the exact UTF-8 bytes of [`Self::canonical_input`].
    pub fn fingerprint(&self) -> String {
        sha256_hex(self.canonical_input().as_bytes())
    }
}

// -- the six normalisations, each a pure function over its source text --------

/// `/proc/sys/kernel/osrelease` and `/proc/sys/kernel/version`: §C3 asks for
/// **one** line with **one** trailing LF removed.
///
/// Fallible on purpose. `trim_end_matches('\n')` would silently turn the
/// unusable `"release\n\n"` into the perfectly valid `"release"`, so a broken
/// source would reach [`HostFields::validate`] already repaired and be
/// recorded as if the host had reported it. Malformed evidence must be
/// rejected, never normalised into shape.
pub fn norm_single_line(text: &str, what: &'static str) -> Result<String, HostError> {
    // Exactly one optional trailing LF — never a run of them.
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() || body.contains('\n') || body.contains('\r') {
        return Err(HostError::Parse {
            what,
            value: text.to_string(),
        });
    }
    Ok(body.to_string())
}

/// `/proc/meminfo`, the `MemTotal:` line, integer kB only with the unit
/// stripped.
pub fn norm_mem_total_kb(meminfo: &str) -> Result<String, HostError> {
    let line = meminfo
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .ok_or_else(|| HostError::Parse {
            what: "MemTotal",
            value: "absent".to_string(),
        })?;
    // Exactly three tokens: the key, the number, and the unit. A missing unit,
    // a different unit, or a surplus token is malformed evidence, not something
    // to normalise away.
    let toks: Vec<&str> = line.split_whitespace().collect();
    let bad = || HostError::Parse {
        what: "MemTotal",
        value: line.to_string(),
    };
    if toks.len() != 3 || toks[0] != "MemTotal:" || toks[2] != "kB" {
        return Err(bad());
    }
    if !is_canonical_decimal(toks[1]) {
        return Err(bad());
    }
    Ok(toks[1].to_string())
}

/// `/proc/self/status`, the `Cpus_allowed_list:` line, the value after the
/// separator with surrounding spaces and tabs stripped.
pub fn norm_cpus_allowed_list(status: &str) -> Result<String, HostError> {
    status
        .lines()
        .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
        .map(|v| v.trim_matches(|c| c == ' ' || c == '\t').to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| HostError::Parse {
            what: "Cpus_allowed_list",
            value: "absent".to_string(),
        })
}

/// `/proc/cpuinfo`, the **first** `model name` line: the value after the colon,
/// internal whitespace collapsed to one space, ends trimmed.
pub fn norm_cpu_model(cpuinfo: &str) -> Result<String, HostError> {
    let raw = cpuinfo
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v)
        .ok_or_else(|| HostError::Parse {
            what: "model name",
            value: "absent".to_string(),
        })?;
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return Err(HostError::Parse {
            what: "model name",
            value: raw.to_string(),
        });
    }
    Ok(collapsed)
}

/// The count of directories matching `/sys/devices/system/cpu/cpu[0-9]*` under
/// an injected sysfs root, as a decimal integer.
pub fn count_cpu_dirs(sys_devices_system_cpu: &Path) -> Result<String, HostError> {
    let rd = std::fs::read_dir(sys_devices_system_cpu).map_err(|e| HostError::Read {
        path: sys_devices_system_cpu.display().to_string(),
        why: e.to_string(),
    })?;
    let mut n = 0usize;
    for entry in rd {
        // No `flatten()`: an unreadable entry is evidence of a broken source,
        // not an entry that does not exist.
        let e = entry.map_err(|err| HostError::Read {
            path: sys_devices_system_cpu.display().to_string(),
            why: err.to_string(),
        })?;
        let raw = e.file_name();
        // Strict UTF-8: a non-UTF-8 entry name cannot be `cpuN`.
        let Some(name) = raw.to_str() else { continue };
        let Some(rest) = name.strip_prefix("cpu") else {
            continue;
        };
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        // `file_type()` reports an error instead of silently answering "no",
        // which is what `Path::is_dir()` does.
        let ft = e.file_type().map_err(|err| HostError::Read {
            path: e.path().display().to_string(),
            why: err.to_string(),
        })?;
        if ft.is_dir() {
            n += 1;
        }
    }
    Ok(n.to_string())
}

/// Read the six fields from a live system. Each source is a separate path so a
/// test can point them anywhere.
pub struct HostSources<'a> {
    pub osrelease: &'a Path,
    pub kernel_version: &'a Path,
    pub sys_cpu_dir: &'a Path,
    pub meminfo: &'a Path,
    pub self_status: &'a Path,
    pub cpuinfo: &'a Path,
}

impl HostSources<'_> {
    pub fn read(&self) -> Result<HostFields, HostError> {
        Ok(HostFields {
            kernel_release: norm_single_line(&read_to_string(self.osrelease)?, "kernel_release")?,
            kernel_version: norm_single_line(
                &read_to_string(self.kernel_version)?,
                "kernel_version",
            )?,
            available_processors: count_cpu_dirs(self.sys_cpu_dir)?,
            mem_total_kb: norm_mem_total_kb(&read_to_string(self.meminfo)?)?,
            cpus_allowed_list: norm_cpus_allowed_list(&read_to_string(self.self_status)?)?,
            cpu_model: norm_cpu_model(&read_to_string(self.cpuinfo)?)?,
        })
    }
}

// ============================================================== DIAGNOSTICS

/// §C6.1: availability is probed **once per process invocation** and never
/// re-probed mid-session, so a channel cannot appear or disappear inside a
/// journal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiagProbe {
    pub cpu_time: bool,
    pub ctx_switches: bool,
    pub freq: bool,
}

/// `true` iff `/proc/self/stat` yields parsable `utime` and `stime`.
pub fn probe_cpu_time(stat_text: &str) -> bool {
    parse_cpu_time_ticks(stat_text).is_some()
}

/// §C5: the integer delta of `utime + stime`, in Linux clock ticks. Fields 14
/// and 15 of `/proc/self/stat`, located after the parenthesised comm so that a
/// process name containing spaces or brackets cannot shift them.
pub fn parse_cpu_time_ticks(stat_text: &str) -> Option<u64> {
    let close = stat_text.rfind(')')?;
    let rest = stat_text.get(close + 1..)?;
    let f: Vec<&str> = rest.split_whitespace().collect();
    // After `)` the next field is `state`, so utime is index 11 and stime 12.
    let utime: u64 = f.get(11)?.parse().ok()?;
    let stime: u64 = f.get(12)?.parse().ok()?;
    utime.checked_add(stime)
}

/// `true` iff `/proc/self/status` carries a parsable
/// `nonvoluntary_ctxt_switches`.
pub fn probe_ctx_switches(status_text: &str) -> bool {
    parse_nonvoluntary_ctxt_switches(status_text).is_some()
}

pub fn parse_nonvoluntary_ctxt_switches(status_text: &str) -> Option<u64> {
    status_text
        .lines()
        .find_map(|l| l.strip_prefix("nonvoluntary_ctxt_switches:"))
        .and_then(|v| v.trim().parse().ok())
}

/// §C6: `true` only when `scaling_cur_freq` is readable **and** parsable for
/// **every** CPU in `cpu_set`. Partial availability is `false`; partial
/// aggregation is forbidden.
pub fn probe_freq(sys_cpu_dir: &Path, cpu_ids: &[u32]) -> bool {
    read_freq_khz(sys_cpu_dir, cpu_ids).is_some()
}

/// The truncated integer arithmetic mean in kHz, or `None` if any required file
/// is absent or unparsable.
pub fn read_freq_khz(sys_cpu_dir: &Path, cpu_ids: &[u32]) -> Option<u64> {
    if cpu_ids.is_empty() {
        return None;
    }
    let mut sum: u64 = 0;
    for id in cpu_ids {
        let p = sys_cpu_dir
            .join(format!("cpu{id}"))
            .join("cpufreq")
            .join("scaling_cur_freq");
        let v: u64 = std::fs::read_to_string(&p).ok()?.trim().parse().ok()?;
        sum = sum.checked_add(v)?;
    }
    Some(sum / cpu_ids.len() as u64)
}

/// The largest CPU list this instrument will expand. A malformed
/// `Cpus_allowed_list` such as `0-4294967295` is evidence of a broken source
/// and must produce an error, never an allocation attempt.
pub const MAX_CPU_LIST: usize = 4096;

/// Expand a `Cpus_allowed_list` such as `0-3,6` into the CPU ids it names,
/// bounded by [`MAX_CPU_LIST`].
pub fn parse_cpu_list(list: &str) -> Result<Vec<u32>, HostError> {
    let bad = || HostError::Parse {
        what: "cpu list",
        value: list.to_string(),
    };
    let mut out = Vec::new();
    for part in list.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(bad());
        }
        if out.len() >= MAX_CPU_LIST {
            return Err(HostError::Overflow("cpu list exceeds MAX_CPU_LIST"));
        }
        match part.split_once('-') {
            None => out.push(part.parse::<u32>().map_err(|_| bad())?),
            Some((a, b)) => {
                let (a, b) = (
                    a.parse::<u32>().map_err(|_| bad())?,
                    b.parse::<u32>().map_err(|_| bad())?,
                );
                if b < a {
                    return Err(bad());
                }
                // Checked and capped BEFORE any allocation.
                let span = (b as u64)
                    .checked_sub(a as u64)
                    .and_then(|d| d.checked_add(1))
                    .ok_or(HostError::Overflow("cpu list range"))?;
                if span as usize > MAX_CPU_LIST || out.len() as u64 + span > MAX_CPU_LIST as u64 {
                    return Err(HostError::Overflow("cpu list exceeds MAX_CPU_LIST"));
                }
                out.extend(a..=b);
            }
        }
    }
    if out.is_empty() {
        return Err(bad());
    }
    Ok(out)
}

// ======================================================================= TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempDir(std::path::PathBuf);
    impl TempDir {
        fn new(name: &str) -> TempDir {
            let p = std::env::temp_dir().join(format!(
                "rc021_host_{}_{}_{}",
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

    #[test]
    fn sha256_known_answers() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn run_uuid_shape_and_variability() {
        let a = new_run_uuid();
        assert_eq!(a.len(), 32);
        assert!(is_run_uuid(&a), "{a}");
        assert!(a
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        // Two draws from OsRng colliding on 128 bits would be a broken source.
        assert_ne!(a, new_run_uuid());
        assert!(!is_run_uuid(&a[..31]));
        assert!(!is_run_uuid(&a.to_uppercase()));
        assert!(!is_run_uuid(""));
    }

    #[test]
    fn uptime_truncates_and_pads_and_never_rounds() {
        // exact three digits
        assert_eq!(parse_uptime_ms("12345.678 99999.9").unwrap(), 12_345_678);
        // truncation, not rounding: .9999 -> .999
        assert_eq!(parse_uptime_ms("1.9999").unwrap(), 1_999);
        assert_eq!(parse_uptime_ms("1.0009").unwrap(), 1_000);
        // short fraction is padded on the right
        assert_eq!(parse_uptime_ms("7.5").unwrap(), 7_500);
        assert_eq!(parse_uptime_ms("7.05").unwrap(), 7_050);
        // no fraction at all
        assert_eq!(parse_uptime_ms("42").unwrap(), 42_000);
        assert_eq!(parse_uptime_ms("0.000").unwrap(), 0);
    }

    #[test]
    fn uptime_rejects_malformed_and_overflow() {
        for bad in [
            "", "   ", "abc", "-1.0", "1.2x", "1.-2", ".5", "+3.0", "1,5",
        ] {
            assert!(parse_uptime_ms(bad).is_err(), "{bad:?} must be rejected");
        }
        // seconds * 1000 must not wrap
        let huge = format!("{}.000", u64::MAX);
        assert_eq!(
            parse_uptime_ms(&huge),
            Err(HostError::Overflow("uptime milliseconds"))
        );
    }

    #[test]
    fn boot_id_and_monotonic_offset() {
        assert_eq!(parse_boot_id("abc-def\n").unwrap(), "abc-def");
        assert!(parse_boot_id("\n").is_err());

        assert_eq!(monotonic_offset_ms("b", "b", 1_000, 4_500).unwrap(), 3_500);
        assert_eq!(monotonic_offset_ms("b", "b", 1_000, 1_000).unwrap(), 0);
        // a different boot invalidates the axis
        assert!(matches!(
            monotonic_offset_ms("b", "c", 0, 1),
            Err(HostError::BootMismatch { .. })
        ));
        // and a backwards clock is an error, not a wrapped subtraction
        assert!(matches!(
            monotonic_offset_ms("b", "b", 5_000, 4_999),
            Err(HostError::ClockWentBackwards { .. })
        ));
    }

    fn fixture_fields() -> HostFields {
        HostFields {
            kernel_release: norm_single_line(
                "6.6.114.1-microsoft-standard-WSL2\n",
                "kernel_release",
            )
            .unwrap(),
            kernel_version: norm_single_line("#1 SMP PREEMPT_DYNAMIC\n", "kernel_version").unwrap(),
            available_processors: "4".to_string(),
            mem_total_kb: norm_mem_total_kb("MemFree: 1 kB\nMemTotal:       10185860 kB\n")
                .unwrap(),
            cpus_allowed_list: norm_cpus_allowed_list("Threads:\t1\nCpus_allowed_list:\t0-3\n")
                .unwrap(),
            cpu_model: norm_cpu_model(
                "vendor_id\t: X\nmodel name\t:   AMD   Ryzen  7\nmodel name\t: OTHER\n",
            )
            .unwrap(),
        }
    }

    #[test]
    fn host_field_normalisation_is_exact() {
        let f = fixture_fields();
        assert_eq!(f.kernel_release, "6.6.114.1-microsoft-standard-WSL2");
        assert_eq!(f.mem_total_kb, "10185860", "unit stripped, digits only");
        assert_eq!(f.cpus_allowed_list, "0-3", "tab stripped");
        assert_eq!(
            f.cpu_model, "AMD Ryzen 7",
            "whitespace collapsed, first line only"
        );

        // failures are errors, never silent defaults
        assert!(norm_mem_total_kb("MemFree: 1 kB\n").is_err());
        assert!(norm_mem_total_kb("MemTotal:       x kB\n").is_err());
        assert!(norm_cpus_allowed_list("Threads:\t1\n").is_err());
        assert!(norm_cpu_model("vendor_id\t: X\n").is_err());
    }

    #[test]
    fn fingerprint_of_a_known_fixture_is_stable() {
        let f = fixture_fields();
        let input = f.canonical_input();
        // key=value<LF> for all six, in §C3 order
        let lines: Vec<&str> = input.trim_end_matches('\n').split('\n').collect();
        assert_eq!(lines.len(), 6);
        for (i, l) in lines.iter().enumerate() {
            assert!(l.starts_with(&format!("{}=", HOST_FIELD_KEYS[i])), "{l}");
        }
        assert_eq!(
            input,
            "kernel_release=6.6.114.1-microsoft-standard-WSL2\n\
             kernel_version=#1 SMP PREEMPT_DYNAMIC\n\
             available_processors=4\n\
             mem_total_kb=10185860\n\
             cpus_allowed_list=0-3\n\
             cpu_model=AMD Ryzen 7\n"
        );
        let fp = f.fingerprint();
        assert_eq!(fp.len(), 64);
        assert_eq!(fp, sha256_hex(input.as_bytes()));
        // the order is part of the identity
        let mut g = f.clone();
        std::mem::swap(&mut g.kernel_release, &mut g.kernel_version);
        assert_ne!(g.fingerprint(), fp);
    }

    #[test]
    fn cpu_directory_count_uses_an_injected_root() {
        let d = TempDir::new("cpudirs");
        for name in ["cpu0", "cpu1", "cpu10", "cpuidle", "cpufreq", "cpu"] {
            std::fs::create_dir_all(d.path().join(name)).unwrap();
        }
        std::fs::write(d.path().join("cpu7"), b"not a dir").unwrap();
        assert_eq!(count_cpu_dirs(d.path()).unwrap(), "3");
        assert!(count_cpu_dirs(&d.path().join("absent")).is_err());
    }

    #[test]
    fn single_line_accepts_one_optional_trailing_lf_and_nothing_else() {
        for (input, want) in [
            ("release", "release"),
            ("release\n", "release"),
            ("a b", "a b"),
        ] {
            assert_eq!(norm_single_line(input, "k").unwrap(), want, "{input:?}");
        }
        for bad in [
            "",
            "\n",
            "\n\n",
            "release\n\n",
            "a\nb",
            "a\r\n",
            "a\rb",
            "\rrelease",
        ] {
            assert!(
                matches!(
                    norm_single_line(bad, "k"),
                    Err(HostError::Parse { what: "k", .. })
                ),
                "{bad:?} must be rejected"
            );
        }
        // The defect this replaces: the doubled LF was silently repaired into
        // a value that HostFields::validate() then happily accepted.
        let repaired = "release\n\n".trim_end_matches('\n').to_string();
        assert_eq!(repaired, "release");
        assert!(norm_single_line("release\n\n", "k").is_err());
    }

    #[test]
    fn host_sources_reject_a_malformed_single_line_source() {
        let d = TempDir::new("sources_bad");
        let w = |n: &str, c: &str| {
            let p = d.path().join(n);
            std::fs::write(&p, c).unwrap();
            p
        };
        let meminfo = w("meminfo", "MemTotal:       123 kB\n");
        let status = w("status", "Cpus_allowed_list:\t0-1\n");
        let cpuinfo = w("cpuinfo", "model name\t: Test  CPU\n");
        let sysdir = d.path().join("sys");
        std::fs::create_dir_all(sysdir.join("cpu0")).unwrap();

        // Each malformed source in turn, with the other one well-formed, so
        // the failure is attributed to the right field.
        for (bad_release, bad_version, what) in [
            ("6.6.0\n\n", "#1 SMP\n", "kernel_release"),
            ("6.6.0\n", "#1\nSMP\n", "kernel_version"),
            ("6.6.0\r\n", "#1 SMP\n", "kernel_release"),
            ("", "#1 SMP\n", "kernel_release"),
        ] {
            let osrelease = w("osrelease", bad_release);
            let version = w("version", bad_version);
            let err = HostSources {
                osrelease: &osrelease,
                kernel_version: &version,
                sys_cpu_dir: &sysdir,
                meminfo: &meminfo,
                self_status: &status,
                cpuinfo: &cpuinfo,
            }
            .read()
            .unwrap_err();
            assert!(
                matches!(&err, HostError::Parse { what: w2, .. } if *w2 == what),
                "{bad_release:?}/{bad_version:?}: expected {what}, got {err}"
            );
        }
    }

    #[test]
    fn host_sources_read_from_injected_paths() {
        let d = TempDir::new("sources");
        let w = |n: &str, c: &str| {
            let p = d.path().join(n);
            std::fs::write(&p, c).unwrap();
            p
        };
        let osrelease = w("osrelease", "6.6.0\n");
        let version = w("version", "#1 SMP\n");
        let meminfo = w("meminfo", "MemTotal:       123 kB\n");
        let status = w("status", "Cpus_allowed_list:\t0-1\n");
        let cpuinfo = w("cpuinfo", "model name\t: Test  CPU\n");
        let sysdir = d.path().join("sys");
        std::fs::create_dir_all(sysdir.join("cpu0")).unwrap();
        std::fs::create_dir_all(sysdir.join("cpu1")).unwrap();

        let f = HostSources {
            osrelease: &osrelease,
            kernel_version: &version,
            sys_cpu_dir: &sysdir,
            meminfo: &meminfo,
            self_status: &status,
            cpuinfo: &cpuinfo,
        }
        .read()
        .unwrap();
        assert_eq!(f.available_processors, "2");
        assert_eq!(f.cpu_model, "Test CPU");
        assert_eq!(f.mem_total_kb, "123");
    }

    #[test]
    fn cpu_time_ticks_survive_a_hostile_comm() {
        // A process name containing spaces and a bracket must not shift fields.
        let stat = "42 (weird ) name) S 1 2 3 4 5 6 7 8 9 10 111 222 13 14";
        assert_eq!(parse_cpu_time_ticks(stat), Some(333));
        assert!(probe_cpu_time(stat));
        assert!(parse_cpu_time_ticks("42 (x) S 1").is_none());
        assert!(!probe_cpu_time("no parens at all"));
    }

    #[test]
    fn ctx_switch_probe() {
        let s = "Threads:\t1\nnonvoluntary_ctxt_switches:\t17\n";
        assert_eq!(parse_nonvoluntary_ctxt_switches(s), Some(17));
        assert!(probe_ctx_switches(s));
        assert!(!probe_ctx_switches("Threads:\t1\n"));
        assert!(!probe_ctx_switches("nonvoluntary_ctxt_switches:\tx\n"));
    }

    #[test]
    fn freq_requires_every_cpu_in_the_set() {
        let d = TempDir::new("freq");
        let mk = |id: u32, khz: &str| {
            let p = d.path().join(format!("cpu{id}")).join("cpufreq");
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("scaling_cur_freq"), khz).unwrap();
        };
        mk(0, "1000000\n");
        mk(1, "2000000\n");
        assert_eq!(read_freq_khz(d.path(), &[0, 1]), Some(1_500_000));
        assert!(probe_freq(d.path(), &[0, 1]));
        // §C6: partial availability is false, never a partial mean
        assert_eq!(read_freq_khz(d.path(), &[0, 1, 2]), None);
        assert!(!probe_freq(d.path(), &[0, 1, 2]));
        assert!(!probe_freq(d.path(), &[]));
        // On the target host no cpufreq directory exists at all, so this is the
        // expected steady state there (§C6) and not a fault.
        let empty = TempDir::new("nofreq");
        assert!(!probe_freq(empty.path(), &[0]));
    }

    #[test]
    fn mem_total_requires_number_and_kb_unit_exactly() {
        assert_eq!(norm_mem_total_kb("MemTotal:  123 kB\n").unwrap(), "123");
        for bad in [
            "MemTotal:  123\n",          // missing unit
            "MemTotal:  123 MB\n",       // wrong unit
            "MemTotal:  123 kB extra\n", // surplus token
            "MemTotal:  kB\n",           // missing number
            "MemTotal:  0123 kB\n",      // not canonical decimal
            "MemTotal:  -1 kB\n",
        ] {
            assert!(norm_mem_total_kb(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn host_fields_must_be_in_normalised_form() {
        let ok = fixture_fields();
        assert!(ok.validate().is_ok());
        type Mutate = Box<dyn Fn(&mut HostFields)>;
        let cases: Vec<(&str, Mutate)> = vec![
            (
                "empty release",
                Box::new(|f: &mut HostFields| f.kernel_release = String::new()),
            ),
            (
                "release with LF",
                Box::new(|f: &mut HostFields| f.kernel_release = "a\nb".into()),
            ),
            (
                "version with CR",
                Box::new(|f: &mut HostFields| f.kernel_version = "a\rb".into()),
            ),
            (
                "processors not decimal",
                Box::new(|f: &mut HostFields| f.available_processors = "four".into()),
            ),
            (
                "processors leading zero",
                Box::new(|f: &mut HostFields| f.available_processors = "04".into()),
            ),
            (
                "mem not decimal",
                Box::new(|f: &mut HostFields| f.mem_total_kb = "1 kB".into()),
            ),
            (
                "cpu list empty",
                Box::new(|f: &mut HostFields| f.cpus_allowed_list = String::new()),
            ),
            (
                "cpu list malformed",
                Box::new(|f: &mut HostFields| f.cpus_allowed_list = "3-1".into()),
            ),
            (
                "cpu list control char",
                Box::new(|f: &mut HostFields| f.cpus_allowed_list = "0-3\t".into()),
            ),
            (
                "model empty",
                Box::new(|f: &mut HostFields| f.cpu_model = String::new()),
            ),
            (
                "model not collapsed",
                Box::new(|f: &mut HostFields| f.cpu_model = "AMD  Ryzen".into()),
            ),
            (
                "model padded",
                Box::new(|f: &mut HostFields| f.cpu_model = " AMD Ryzen".into()),
            ),
        ];
        for (what, mutate) in cases {
            let mut f = ok.clone();
            mutate(&mut f);
            assert!(f.validate().is_err(), "must reject {what}");
            // The fingerprint would happily hash the bad value, which is why
            // validation cannot rely on it.
            assert_eq!(f.fingerprint().len(), 64);
        }
    }

    #[test]
    fn cpu_dir_count_surfaces_errors_instead_of_hiding_them() {
        let d = TempDir::new("cpudirs_err");
        std::fs::create_dir_all(d.path().join("cpu0")).unwrap();
        // a `cpuN` that is a regular file is not a CPU directory
        std::fs::write(d.path().join("cpu1"), b"x").unwrap();
        assert_eq!(count_cpu_dirs(d.path()).unwrap(), "1");
        // a missing root is an error, never "zero CPUs"
        assert!(matches!(
            count_cpu_dirs(&d.path().join("absent")),
            Err(HostError::Read { .. })
        ));
    }

    #[test]
    fn cpu_list_expansion_is_bounded() {
        // The malformed evidence that would previously try to allocate 4 Gi.
        assert_eq!(
            parse_cpu_list("0-4294967295"),
            Err(HostError::Overflow("cpu list exceeds MAX_CPU_LIST"))
        );
        // exactly at the cap is fine, one beyond is not
        let at_cap = format!("0-{}", MAX_CPU_LIST - 1);
        assert_eq!(parse_cpu_list(&at_cap).unwrap().len(), MAX_CPU_LIST);
        let over = format!("0-{MAX_CPU_LIST}");
        assert!(matches!(
            parse_cpu_list(&over),
            Err(HostError::Overflow("cpu list exceeds MAX_CPU_LIST"))
        ));
        // and the cap counts across comma-separated parts
        let split = format!("0-{},{}", MAX_CPU_LIST - 1, MAX_CPU_LIST);
        assert!(parse_cpu_list(&split).is_err());
    }

    #[test]
    fn cpu_list_expansion() {
        assert_eq!(parse_cpu_list("0-3").unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(parse_cpu_list("0,2,4").unwrap(), vec![0, 2, 4]);
        assert_eq!(parse_cpu_list("0-1,5").unwrap(), vec![0, 1, 5]);
        assert_eq!(parse_cpu_list("7").unwrap(), vec![7]);
        for bad in ["", "3-1", "a", "0,", "0-", "-2"] {
            assert!(parse_cpu_list(bad).is_err(), "{bad:?}");
        }
    }
}
