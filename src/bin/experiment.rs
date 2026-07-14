//! Experiment entrypoint (Constitution §7, §12; Blueprint Step 2).
//!
//!   cargo run --release --bin experiment -- --exp EXP-0000
//!
//! Ensures the solver binaries this experiment needs are built in release,
//! then hands off to the Python experiment runner (which owns the stats and
//! visualization stack). One command → json/tables/graphs/html/report.md
//! under experiments/results/<EXP>/.
//!
//! This shim contains no experiment logic; it exists so the canonical
//! invocation is a single `cargo run`.

use std::path::PathBuf;
use std::process::{exit, Command};

fn arg(name: &str) -> Option<String> {
    let argv: Vec<String> = std::env::args().collect();
    argv.iter()
        .position(|a| a == name)
        .and_then(|i| argv.get(i + 1).cloned())
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is the crate root at build time; fall back to CWD.
    option_env!("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn main() {
    let exp = arg("--exp").unwrap_or_else(|| {
        eprintln!("usage: experiment --exp EXP-XXXX");
        exit(2);
    });

    let root = repo_root();
    let py = root.join("benchmark-env/bin/python3");
    let runner = root.join("experiments/run_experiment.py");
    let python = if py.exists() {
        py
    } else {
        PathBuf::from("python3")
    };

    if !runner.exists() {
        eprintln!("runner not found: {}", runner.display());
        exit(1);
    }

    println!("[experiment] {exp}: invoking {}", runner.display());
    let status = Command::new(python)
        .arg(&runner)
        .arg("--exp")
        .arg(&exp)
        .current_dir(&root)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("failed to launch python runner: {e}");
            exit(1);
        }
    }
}
