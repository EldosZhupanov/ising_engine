//! RC-021 §C11.4 — the P2 child, exercised against the **real binary**.
//!
//! This lives in `tests/` on purpose. Cargo builds and passes
//! `CARGO_BIN_EXE_exp_rc021_host_qualify` for an integration target, so the
//! binary under test is always current: a unit test inside the binary crate
//! would have to locate the product binary itself and could run against a stale
//! one, reporting coverage it does not have.
//!
//! Nothing here runs a sentinel, a control phase or a qualification session,
//! and nothing touches `experiments/rc021`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let p = std::env::temp_dir().join(format!(
            "rc021_p2_int_{}_{}_{}",
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

/// §C10.3's frozen host fields, and the SHA-256 of their canonical input.
///
/// The digest is pinned here because a `tests/` target cannot reach the
/// binary's private modules. `host::tests` asserts the same constant against
/// `HostFields::fingerprint()`, so the two cannot drift apart silently.
const FIXTURE_FINGERPRINT: &str =
    "89b369a2f89e24862187ecc55056b61baccda7d83813804994e8c38fbc6fba33";

/// A canonical `run.json`: §C13.1's eighteen keys, in order, compact, one
/// trailing LF.
fn write_run_json(dir: &Path) {
    let canon = std::fs::canonicalize(dir).unwrap();
    let run_dir = canon.to_str().unwrap();
    let body = format!(
        concat!(
            "{{",
            "\"schema_version\":\"rc021/1\",",
            "\"run_uuid\":\"{uuid}\",",
            "\"boot_id\":\"{boot}\",",
            "\"run_start_uptime_ms\":{uptime},",
            "\"run_dir\":\"{dir}\",",
            "\"repo_commit\":\"{a40}\",",
            "\"prereg_commit\":\"{b40}\",",
            "\"amendment_commits\":[\"{c40}\",\"{d40}\"],",
            "\"instrument_birth_commit\":\"{e40}\",",
            "\"host_fingerprint\":\"{fp}\",",
            "\"host_fields\":{{",
            "\"kernel_release\":\"6.6.0\",",
            "\"kernel_version\":\"#1 SMP\",",
            "\"available_processors\":\"4\",",
            "\"mem_total_kb\":\"10185860\",",
            "\"cpus_allowed_list\":\"0-3\",",
            "\"cpu_model\":\"Test CPU\"",
            "}},",
            "\"cpu_set\":\"0-3\",",
            "\"thread_count\":1,",
            "\"timer_resolution_ms\":0.00002,",
            "\"cpu_time_unit\":\"linux_clock_ticks\",",
            "\"command_line\":[\"exp_rc021_host_qualify\"],",
            "\"utc_start\":\"2026-08-25T00:00:00Z\",",
            "\"diag_availability\":{{\"cpu_time\":true,\"ctx_switches\":true,\"freq\":false}}",
            "}}\n"
        ),
        uuid = "0".repeat(32),
        boot = current_boot_id(),
        uptime = 1u64,
        dir = run_dir,
        a40 = "a".repeat(40),
        b40 = "b".repeat(40),
        c40 = "c".repeat(40),
        d40 = "d".repeat(40),
        e40 = "e".repeat(40),
        fp = FIXTURE_FINGERPRINT,
    );
    std::fs::write(canon.join("run.json"), body).unwrap();
}

/// §C12: the child computes a real offset against this, so the fixture must
/// name the boot the test is actually running on.
fn current_boot_id() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .expect("/proc/sys/kernel/random/boot_id")
        .trim()
        .to_string()
}

const P2_CHILD_ARG: &str = "--__rc021-p2-child";
const P2_JOURNAL: &str = "control/rc021_journal_p2.tsv";

fn spawn_child(dir: &Path) -> std::process::ExitStatus {
    std::process::Command::new(env!("CARGO_BIN_EXE_exp_rc021_host_qualify"))
        .arg(P2_CHILD_ARG)
        .arg(dir)
        .output()
        .expect("spawn the RC-021 binary")
        .status
}

#[cfg(unix)]
fn signal_of(s: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    s.signal()
}

/// The whole control: a real process, a durable journal, and SIGABRT.
#[cfg(unix)]
#[test]
fn the_p2_child_leaves_a_durable_journal_and_dies_by_signal() {
    let d = TempDir::new("happy");
    write_run_json(d.path());
    std::fs::create_dir(d.path().join("control")).unwrap();

    let status = spawn_child(d.path());
    assert_eq!(signal_of(&status), Some(6), "SIGABRT, not a clean exit");
    assert!(status.code().is_none(), "no exit code: it did not return");

    let text = std::fs::read_to_string(d.path().join(P2_JOURNAL))
        .expect("the child's journal must survive its death");

    // §C9 metadata, declaring P2 and this invocation's own provenance.
    assert!(text.contains("#rc021_meta\tsession\t\"P2\""));
    assert!(
        text.contains(P2_CHILD_ARG),
        "command_line must be the child's argv, not the manifest's"
    );
    assert!(
        !text.contains("\"exp_rc021_host_qualify\"]"),
        "the manifest's one-element command_line must not be copied verbatim"
    );

    // Exactly one SESSION-OPEN, at least one measurement row, no close row.
    let data: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("#rc021_meta\t") && !l.starts_with("schema_version\t"))
        .collect();
    assert_eq!(
        data.iter()
            .filter(|l| l.contains("\tSESSION-OPEN\t"))
            .count(),
        1
    );
    assert!(data
        .iter()
        .any(|l| l.contains("\tLOST\t") || l.contains("\tOK\t")));
    assert!(!text.contains("SESSION-CLOSE"), "the child must not close");

    // §C11.4's frozen coordinates: session 0, phase A, block 1, index NA.
    let measurement = data
        .iter()
        .find(|l| l.contains("\tLOST\t") || l.contains("\tOK\t"))
        .unwrap();
    let f: Vec<&str> = measurement.split('\t').collect();
    assert_eq!(f.len(), 24, "the 24-column schema");
    assert_eq!(f[5], "A");
    assert_eq!(f[6], "0");
    assert_eq!(f[7], "1");
    assert_eq!(f[8], "NA");

    // Real provenance: no hard-coded 10, 20 or 0.0.
    let open = data
        .iter()
        .find(|l| l.contains("\tSESSION-OPEN\t"))
        .unwrap();
    let of: Vec<&str> = open.split('\t').collect();
    let open_offset: u64 = of[9].parse().expect("monotonic_offset_ms");
    let meas_offset: u64 = f[9].parse().expect("monotonic_offset_ms");
    assert!(
        open_offset > 100,
        "a real uptime offset, not the constant 10"
    );
    assert!(meas_offset >= open_offset, "the axis must not go backwards");
    // §C8.1/§C8.3, host-independent: `load_avg_start` is present and is a
    // canonical, finite, non-negative shortest-round-trip f64.
    //
    // No comparison against a second `/proc/loadavg` read: that would prove
    // nothing — a hard-coded `0.0` matches any host whose one-minute load is
    // below the tolerance. That the value actually comes from the source is
    // proved deterministically by the unit test on `p2_write_evidence`, which
    // feeds it a synthetic file and requires the exact bits back.
    assert_ne!(of[14], "NA", "load_avg_start must carry a value");
    let load: f64 = of[14]
        .parse()
        .unwrap_or_else(|_| panic!("load_avg_start is not an f64: {:?}", of[14]));
    assert!(load.is_finite() && load >= 0.0, "load_avg_start {load}");
    assert_eq!(
        load.to_string(),
        of[14],
        "load_avg_start must be shortest round-trip"
    );
    assert_eq!(of[15], "NA", "SESSION-OPEN carries no load_avg_end");
}

/// §C11.4: the child creates **only** its journal. Without `control/` it must
/// still die abnormally and leave no journal, so the parent sees a P2 FAIL.
#[cfg(unix)]
#[test]
fn the_child_does_not_create_the_control_directory() {
    let d = TempDir::new("nocontrol");
    write_run_json(d.path());
    assert!(!d.path().join("control").exists());

    let status = spawn_child(d.path());
    assert_eq!(signal_of(&status), Some(6), "still an abnormal death");
    assert!(
        !d.path().join("control").exists(),
        "the child must not create control/"
    );
    assert!(!d.path().join(P2_JOURNAL).exists());
}

/// §C12: a boot mismatch leaves a journal that cannot satisfy P2.
#[cfg(unix)]
#[test]
fn a_boot_mismatch_leaves_no_measurement_row() {
    let d = TempDir::new("bootmismatch");
    write_run_json(d.path());
    let text = std::fs::read_to_string(d.path().join("run.json")).unwrap();
    let swapped = text.replace(&current_boot_id(), "00000000-0000-0000-0000-000000000000");
    assert_ne!(swapped, text);
    std::fs::write(d.path().join("run.json"), swapped).unwrap();
    std::fs::create_dir(d.path().join("control")).unwrap();

    let status = spawn_child(d.path());
    assert_eq!(signal_of(&status), Some(6));
    // Nothing durable that satisfies §C11.4: either no journal at all, or one
    // without a measurement row.
    let journal = d.path().join(P2_JOURNAL);
    if journal.exists() {
        let t = std::fs::read_to_string(&journal).unwrap();
        assert!(
            !t.contains("\tLOST\t") && !t.contains("\tOK\t"),
            "a mismatched boot must not produce a measurement row"
        );
    }
}

/// §C11.4: P2 does not repeat. The journal path is claimed with `create_new`,
/// so a second child cannot produce one and no attempt suffix appears.
#[cfg(unix)]
#[test]
fn a_second_child_cannot_replace_the_journal() {
    let d = TempDir::new("norepeat");
    write_run_json(d.path());
    std::fs::create_dir(d.path().join("control")).unwrap();

    assert_eq!(signal_of(&spawn_child(d.path())), Some(6));
    let first = std::fs::read(d.path().join(P2_JOURNAL)).unwrap();

    assert_eq!(signal_of(&spawn_child(d.path())), Some(6));
    assert_eq!(
        std::fs::read(d.path().join(P2_JOURNAL)).unwrap(),
        first,
        "the first journal is immutable"
    );
    let entries: Vec<String> = std::fs::read_dir(d.path().join("control"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec!["rc021_journal_p2.tsv"], "no attempt suffix");
}
