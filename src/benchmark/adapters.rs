//! Solver adapters under one interface, so every solver sees identical
//! instances, seeds, and computational budget.
//!
//! - Native (always available): the full `UltimateSolver`, a scalar
//!   `ParallelTemperingSolver`, and a single-chain simulated-annealing
//!   baseline.
//! - External via a Python bridge: **OpenJij** and **dwave-neal**. The bridge
//!   writes the QUBO to a temp file, runs the library at the matched sweep
//!   budget, and returns the state as JSON. If the library (or Python) is
//!   absent, the adapter reports unavailable and is cleanly skipped — never
//!   fabricated.
//! - Commercial result files (Toshiba SBM, Fujitsu DA, Kaiwu): ingested from
//!   `benchmarks/external/<solver>.csv` (`instance,best_energy,wall_ms`) when
//!   the vendor's own runs are dropped in. The objective is recomputed under
//!   our energy convention so comparisons stay fair.

use crate::core::QuboModel;
use crate::solver::{ParallelTemperingSolver, UltimateSolver};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use std::time::Instant;

/// Matched computational budget shared by all solvers.
#[derive(Clone, Copy)]
pub struct Budget {
    pub sweeps: usize,
    pub exchanges: usize,
    pub replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            sweeps: 50,
            exchanges: 40,
            replicas: 24,
            temp_max: 5.0,
            temp_min: 0.02,
        }
    }
}

impl Budget {
    /// Total single-spin update sweeps (the common work unit across solvers).
    pub fn total_sweeps(&self) -> usize {
        self.sweeps * self.exchanges
    }
}

/// One solver run: the final state and the measured wall-clock in ms.
pub struct RunResult {
    pub state: Vec<i8>,
    pub wall_ms: f64,
}

/// Solvers the harness can drive.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Solver {
    Ultimate,
    ParallelTempering,
    SimulatedAnnealing,
    OpenJij,
    DwaveNeal,
}

impl Solver {
    pub fn label(&self) -> &'static str {
        match self {
            Solver::Ultimate => "Ultimate",
            Solver::ParallelTempering => "PT",
            Solver::SimulatedAnnealing => "SA",
            Solver::OpenJij => "OpenJij",
            Solver::DwaveNeal => "dwave-neal",
        }
    }
    pub fn is_native(&self) -> bool {
        matches!(
            self,
            Solver::Ultimate | Solver::ParallelTempering | Solver::SimulatedAnnealing
        )
    }
    /// Python module + adapter script for external solvers.
    fn bridge(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Solver::OpenJij => Some(("openjij", "benchmarks/adapters/run_openjij.py")),
            Solver::DwaveNeal => Some(("neal", "benchmarks/adapters/run_neal.py")),
            _ => None,
        }
    }

    /// Whether this solver can actually run in the current environment.
    pub fn available(&self) -> bool {
        match self.bridge() {
            None => true,
            Some((module, _)) => python_can_import(module),
        }
    }

    /// Run under the matched budget. `None` if an external solver is
    /// unavailable or the bridge failed.
    pub fn run(&self, model: &QuboModel, budget: &Budget, seed: u64) -> Option<RunResult> {
        match self {
            Solver::Ultimate => Some(timed(|| {
                UltimateSolver::new(
                    budget.temp_max,
                    budget.temp_min,
                    budget.sweeps,
                    budget.exchanges,
                    Some(seed),
                )
                .solve(model, &[])
            })),
            Solver::ParallelTempering => Some(timed(|| {
                ParallelTemperingSolver {
                    num_replicas: budget.replicas,
                    temp_max: budget.temp_max,
                    temp_min: budget.temp_min,
                    sweeps_per_exchange: budget.sweeps,
                    total_exchanges: budget.exchanges,
                    seed: Some(seed),
                }
                .solve(model, &[])
            })),
            Solver::SimulatedAnnealing => Some(timed(|| {
                simulated_annealing(
                    model,
                    budget.total_sweeps(),
                    budget.temp_max,
                    budget.temp_min,
                    seed,
                )
            })),
            Solver::OpenJij | Solver::DwaveNeal => self.run_bridge(model, budget, seed),
        }
    }

    fn run_bridge(&self, model: &QuboModel, budget: &Budget, seed: u64) -> Option<RunResult> {
        let (_, script) = self.bridge()?;
        run_python_solver(script, model, budget, seed)
    }
}

/// Time a closure returning the state.
fn timed<F: FnOnce() -> Vec<i8>>(f: F) -> RunResult {
    let start = Instant::now();
    let state = f();
    RunResult {
        state,
        wall_ms: start.elapsed().as_secs_f64() * 1000.0,
    }
}

/// Single-chain simulated annealing (geometric cooling) baseline.
pub fn simulated_annealing(
    model: &QuboModel,
    sweeps: usize,
    t_hi: f64,
    t_lo: f64,
    seed: u64,
) -> Vec<i8> {
    let n = model.num_vars;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut state: Vec<i8> = (0..n).map(|_| rng.gen_range(0..=1)).collect();
    for s in 0..sweeps {
        let frac = s as f64 / sweeps.max(1) as f64;
        let temp = t_hi * (t_lo / t_hi).powf(frac);
        for _ in 0..n {
            let v = rng.gen_range(0..n);
            let mut field = model.linear[v];
            for (j, w) in model.quadratic.get_row(v) {
                field += w * state[j] as f64;
            }
            let delta = (1.0 - 2.0 * state[v] as f64) * field;
            if delta <= 0.0 || rng.gen_range(0.0..1.0) < (-delta / temp).exp() {
                state[v] = 1 - state[v];
            }
        }
    }
    state
}

// --------------------------------------------------------------------------
// Python bridge
// --------------------------------------------------------------------------

/// True if `python3 -c "import <module>"` succeeds.
pub fn python_can_import(module: &str) -> bool {
    Command::new("python3")
        .args(["-c", &format!("import {module}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Serialize the model into the bridge's line format:
/// `P n sweeps seed`, then `L i coeff` (linear) and `Q i j coeff` (i<j pair).
fn serialize_model(model: &QuboModel, sweeps: usize, seed: u64) -> String {
    let mut s = format!("P {} {} {}\n", model.num_vars, sweeps, seed);
    for i in 0..model.num_vars {
        if model.linear[i] != 0.0 {
            s.push_str(&format!("L {i} {}\n", model.linear[i]));
        }
        for (j, w) in model.quadratic.get_row(i) {
            if i < j && w != 0.0 {
                s.push_str(&format!("Q {i} {j} {w}\n"));
            }
        }
    }
    s
}

/// Invoke an external Python solver. Returns `None` on any failure so the
/// harness records the solver as unavailable rather than crashing.
fn run_python_solver(
    script: &str,
    model: &QuboModel,
    budget: &Budget,
    seed: u64,
) -> Option<RunResult> {
    let payload = serialize_model(model, budget.total_sweeps(), seed);
    let tmp = std::env::temp_dir().join(format!("ising_bridge_{seed}.txt"));
    std::fs::write(&tmp, payload).ok()?;
    let out = Command::new("python3")
        .arg(script)
        .arg(&tmp)
        .output()
        .ok()?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_bridge_json(&stdout, model.num_vars)
}

/// Minimal parse of the bridge's JSON reply:
/// `{"available": true, "state": [0,1,...], "wall_ms": 12.3}`.
fn parse_bridge_json(text: &str, n: usize) -> Option<RunResult> {
    if !text.contains("\"available\"") || text.contains("\"available\": false") {
        return None;
    }
    let state = extract_int_array(text, "state")?;
    if state.len() != n {
        return None;
    }
    let wall = extract_number(text, "wall_ms").unwrap_or(f64::NAN);
    Some(RunResult {
        state: state.into_iter().map(|v| v.clamp(0, 1) as i8).collect(),
        wall_ms: wall,
    })
}

fn extract_number(text: &str, key: &str) -> Option<f64> {
    let pat = format!("\"{key}\"");
    let start = text.find(&pat)? + pat.len();
    let rest = &text[start..];
    let colon = rest.find(':')? + 1;
    let tail = rest[colon..].trim_start();
    let end = tail
        .find(|c: char| {
            !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E')
        })
        .unwrap_or(tail.len());
    tail[..end].parse().ok()
}

fn extract_int_array(text: &str, key: &str) -> Option<Vec<i64>> {
    let pat = format!("\"{key}\"");
    let start = text.find(&pat)? + pat.len();
    let lb = text[start..].find('[')? + start + 1;
    let rb = text[lb..].find(']')? + lb;
    let mut out = Vec::new();
    for tok in text[lb..rb].split(',') {
        let t = tok.trim();
        if t.is_empty() {
            continue;
        }
        out.push(t.parse::<i64>().ok()?);
    }
    Some(out)
}

// --------------------------------------------------------------------------
// Commercial result-file ingestion
// --------------------------------------------------------------------------

/// A dropped-in vendor result: best objective (native units) and wall-clock.
#[derive(Clone, Copy)]
pub struct ExternalResult {
    pub best_native: f64,
    pub wall_ms: f64,
}

/// Load `benchmarks/external/<solver>.csv` (`instance,best_native,wall_ms`)
/// when a vendor's own results (Toshiba/Fujitsu/Kaiwu/…) are provided. Empty
/// map if the file is absent.
pub fn load_external_results(solver: &str) -> HashMap<String, ExternalResult> {
    let path = format!("benchmarks/external/{solver}.csv");
    let mut map = HashMap::new();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return map;
    };
    for (i, line) in text.lines().enumerate() {
        let t: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if t.len() < 3 || (i == 0 && t[1].parse::<f64>().is_err()) {
            continue; // skip header / malformed
        }
        if let (Ok(best), Ok(wall)) = (t[1].parse::<f64>(), t[2].parse::<f64>()) {
            map.insert(
                t[0].to_string(),
                ExternalResult {
                    best_native: best,
                    wall_ms: wall,
                },
            );
        }
    }
    map
}

/// Write the Python bridge scripts to `benchmarks/adapters/` if missing, so a
/// single command is self-contained on a machine that later installs the
/// libraries.
pub fn ensure_bridge_scripts() -> std::io::Result<()> {
    let dir = std::path::Path::new("benchmarks/adapters");
    std::fs::create_dir_all(dir)?;
    write_if_absent(&dir.join("run_openjij.py"), OPENJIJ_PY)?;
    write_if_absent(&dir.join("run_neal.py"), NEAL_PY)?;
    Ok(())
}

fn write_if_absent(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(content.as_bytes())
}

const OPENJIJ_PY: &str = include_str!("adapters/run_openjij.py");
const NEAL_PY: &str = include_str!("adapters/run_neal.py");
