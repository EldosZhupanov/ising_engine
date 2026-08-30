//! Process-level contract tests for the public RC-021 CLI grammar and safe
//! refusal paths. These tests deliberately never satisfy a measuring-mode
//! precondition, so they cannot execute a control, a qualification pair, or
//! the G11 sentinel.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc021_cli_process_{}_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst),
            name
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
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

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_exp_rc021_host_qualify")
}

fn run(args: &[&str]) -> i32 {
    Command::new(binary())
        .args(args)
        .status()
        .unwrap()
        .code()
        .expect("the CLI must return an ordinary exit code")
}

/// The exit code **and** what the operator was told.
fn run_with_stderr(args: &[&str]) -> (i32, String) {
    let out = Command::new(binary()).args(args).output().unwrap();
    (
        out.status.code().expect("an ordinary exit code"),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, here: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(here).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(dir, dir, &mut result);
    result
}

#[test]
fn malformed_public_invocations_exit_two_at_the_process_boundary() {
    let id = "0".repeat(32);
    for args in [
        vec![],
        vec!["--unknown"],
        vec!["--verify", "/tmp/not-a-mode"],
        vec!["--session", "0", "--dir", "/tmp/x", "--run-id", &id],
        vec!["--session", "7", "--dir", "/tmp/x", "--run-id", &id],
        vec!["--controls", "--run-id", &id, "--dir", "/tmp/x"],
        vec!["--finalize", "--dir", "/tmp/x", "--run-id", "ABC"],
    ] {
        assert_eq!(run(&args), 2, "arguments: {args:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_non_utf8_argument_is_a_refusal_not_a_panic() {
    use std::os::unix::ffi::OsStringExt;

    let status = Command::new(binary())
        .arg(std::ffi::OsString::from_vec(vec![0xff]))
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn safe_unmet_preconditions_have_frozen_exit_codes_and_write_nothing() {
    let dir = TempDir::new("preconditions");
    let path = dir.path().to_str().unwrap();
    let id = "0".repeat(32);
    let before = snapshot(dir.path());

    assert_eq!(run(&["--verify", "--dir", path]), 2);
    assert_eq!(snapshot(dir.path()), before, "verify is read-only");

    assert_eq!(run(&["--controls", "--dir", path, "--run-id", &id]), 2);
    assert_eq!(snapshot(dir.path()), before, "controls refuse before work");

    assert_eq!(run(&["--session", "1", "--dir", path, "--run-id", &id,]), 2);
    assert_eq!(snapshot(dir.path()), before, "session refuses before work");

    // §C13.6 / §C14.1: a directory holding no run has not begun. That is a
    // refusal, not a diagnosis — exit 2, correctable, and no closure path is
    // claimed. The run-id is irrelevant here and must not change the code.
    for run_id in [id.as_str(), &"9".repeat(32)] {
        assert_eq!(run(&["--finalize", "--dir", path, "--run-id", run_id]), 2);
    }
    assert_eq!(
        snapshot(dir.path()),
        before,
        "finalize claims nothing for a run that never began"
    );
}

/// §C10.2 at the real process boundary.  The diagnostic assertion makes this
/// non-vacuous: a later provenance refusal also exits 2, but cannot satisfy the
/// reserved-path message below.
#[test]
fn init_run_refuses_a_reserved_path_before_provenance_and_writes_nothing() {
    let dir = TempDir::new("init_reserved");
    let path = dir.path().to_str().unwrap();
    let reserved = dir.path().join("rc021_closure.json");
    std::fs::write(&reserved, b"existing terminal evidence").unwrap();
    let before = snapshot(dir.path());

    let (code, stderr) = run_with_stderr(&["--init-run", "--dir", path]);

    assert_eq!(code, 2);
    assert!(
        stderr.contains("reserved RC-021 path already exists: rc021_closure.json"),
        "the refusal must come from the reserved-path branch: {stderr:?}"
    );
    assert_eq!(
        snapshot(dir.path()),
        before,
        "a refused initialization must not write or repair anything"
    );
}

/// §C14.2 at the process boundary: once the closure path exists the run is
/// locked, and a wrong `--run-id` must not downgrade exit 4 to a correctable
/// exit 2.
#[test]
fn the_terminal_lock_outranks_the_run_id_at_the_process_boundary() {
    let dir = TempDir::new("lock_vs_runid");
    let path = dir.path().to_str().unwrap();
    std::fs::write(dir.path().join("rc021_closure.json"), b"").unwrap();
    let before = snapshot(dir.path());
    for run_id in ["0".repeat(32), "9".repeat(32)] {
        assert_eq!(run(&["--finalize", "--dir", path, "--run-id", &run_id]), 4);
    }
    assert_eq!(snapshot(dir.path()), before, "a locked run writes nothing");
}

/// These tests must not touch the repository's real run directory.
///
/// Asserting it never *exists* would be wrong: the instrument's whole purpose is
/// to create it, so that assertion fails permanently the first time anyone runs
/// RC-021 for real — a failure with nothing to do with these tests. What matters
/// is that the process tests leave it **unchanged**, which is what is checked.
#[test]
fn process_tests_leave_the_real_rc021_directory_untouched() {
    let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("experiments/rc021");
    let before = if real.exists() {
        Some(snapshot(&real))
    } else {
        None
    };

    let dir = TempDir::new("untouched");
    let path = dir.path().to_str().unwrap();
    let id = "0".repeat(32);
    for args in [
        vec!["--verify", "--dir", path],
        vec!["--controls", "--dir", path, "--run-id", &id],
        vec!["--session", "1", "--dir", path, "--run-id", &id],
        vec!["--finalize", "--dir", path, "--run-id", &id],
    ] {
        run(&args);
    }

    let after = if real.exists() {
        Some(snapshot(&real))
    } else {
        None
    };
    assert_eq!(
        before.is_some(),
        after.is_some(),
        "the process tests changed whether the real run directory exists"
    );
    assert_eq!(
        before, after,
        "the process tests wrote into the real run directory"
    );
}

/// Every refusal must say why.
///
/// `RefusedBeforeMeasurement`'s own contract is "not one byte created", so a
/// reason that is not printed exists nowhere at all — the operator gets a bare
/// exit code and no way to learn which precondition failed. The modes had
/// diverged: sessions and finalize described their outcome, `--controls` did
/// not.
#[test]
fn every_refusal_tells_the_operator_why() {
    let dir = TempDir::new("diagnostics");
    let path = dir.path().to_str().unwrap();
    let id = "0".repeat(32);
    for args in [
        vec!["--controls", "--dir", path, "--run-id", &id],
        vec!["--session", "1", "--dir", path, "--run-id", &id],
        vec!["--finalize", "--dir", path, "--run-id", &id],
        vec!["--verify", "--dir", path],
    ] {
        let (code, stderr) = run_with_stderr(&args);
        assert_ne!(code, 0, "{args:?} must not succeed");
        assert!(
            stderr.contains("rc021"),
            "{args:?} exited {code} with no diagnostic: {stderr:?}"
        );
        assert!(
            stderr.trim().len() > "rc021".len() + 4,
            "{args:?} printed a bare tag with no reason: {stderr:?}"
        );
    }
}
