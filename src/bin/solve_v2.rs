//! `solve_v2` — the engine_v2 reference solver for the v0.1 gate (EXP-0001).
//!
//! Stack under test (Constitution §3, §7–§9): the `SparseBitSlice` integer
//! backend running the single `GibbsColorSweep` operator. This binary is the
//! application that composes the framework; the audited core (backend +
//! operator, cross-validated to zero drift against `ReferenceState`) is used
//! verbatim.
//!
//! Method: R independent heat-bath chains packed into the bit-sliced ensemble
//! (64 replicas per machine word), sharing one geometric cooling schedule —
//! parallel simulated annealing. The best replica's configuration is reported.
//! The cooling schedule lives here for v0.1; at v0.5 it migrates into a Runtime
//! temperature controller (the `maybe_adapt` hook), leaving this binary to just
//! request a Plan.
//!
//! Determinism (ADR-0004): a fixed schedule + fixed seed ⇒ identical output on
//! re-run. Work is bounded by SWEEPS, never wall-clock; `--wall-ms` is only an
//! optional guard (off by default) and never influences the trajectory.
//!
//! Contract with the experiment runner: read `--file/--format/--seed/--sweeps`,
//! print one JSON line `{"state":"010…"}` of length n. MaxCut is lowered to a
//! QUBO minimization identical to the runner's `_qubo_from_edges`, so the
//! single canonical scorer applies to every engine.

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{Budget, Operator};
use ising_engine::engine_v2::operators::GibbsColorSweep;
use ising_engine::engine_v2::runtime::RuntimeView;
use ising_engine::engine_v2::state::SpinState;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::process::exit;
use std::time::Instant;

fn arg(name: &str) -> Option<String> {
    let argv: Vec<String> = std::env::args().collect();
    argv.iter()
        .position(|a| a == name)
        .and_then(|i| argv.get(i + 1).cloned())
}

fn arg_usize(name: &str, default: usize) -> usize {
    arg(name).and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// An upper-triangle weighted edge list: (i, j, weight) with i < j.
type Edges = Vec<(u32, u32, f64)>;

/// Parse a rudy weighted-MaxCut file: header `n m`, then `m` lines `i j [w]`
/// (1-indexed; missing weight = 1). Returns (n, upper-triangle pairs).
fn parse_rudy(text: &str) -> Result<(usize, Edges), String> {
    let mut lines = text
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .filter(|t| !t.is_empty());
    let header = lines.next().ok_or("empty file")?;
    let n: usize = header
        .first()
        .ok_or("missing n")?
        .parse()
        .map_err(|_| "bad n")?;
    let mut pairs = Vec::new();
    for t in lines {
        if t.len() < 2 {
            continue;
        }
        let (u, v): (usize, usize) = (
            t[0].parse().map_err(|_| "bad u")?,
            t[1].parse().map_err(|_| "bad v")?,
        );
        let w: f64 = if t.len() > 2 {
            t[2].parse().map_err(|_| "bad w")?
        } else {
            1.0
        };
        if u == v || u < 1 || v < 1 || u > n || v > n {
            continue;
        }
        let (a, b) = ((u - 1).min(v - 1) as u32, (u - 1).max(v - 1) as u32);
        pairs.push((a, b, w));
    }
    Ok((n, pairs))
}

/// Lower MaxCut to a QUBO minimization (minimize −cut). For each edge (u,v,w):
///   −cut term = −w·(x_u + x_v − 2·x_u·x_v)  ⇒  `linear[u]−=w`, `linear[v]−=w`,
///   pair(u,v) += 2w. Identical to the runner's `_qubo_from_edges`, so lowest
///   QUBO energy ⇔ highest cut and the canonical scorer agrees across engines.
fn maxcut_to_qubo(n: usize, pairs: &[(u32, u32, f64)]) -> ProblemIR {
    let mut linear = vec![0.0f64; n];
    let mut qpairs = Vec::with_capacity(pairs.len());
    for &(u, v, w) in pairs {
        linear[u as usize] -= w;
        linear[v as usize] -= w;
        qpairs.push((u, v, 2.0 * w));
    }
    ProblemIR::from_pairs(n, 0.0, linear, &qpairs)
}

fn main() {
    let file = arg("--file").unwrap_or_else(|| {
        eprintln!(
            "usage: solve_v2 --file <path> --format rudy --seed N [--sweeps N] [--replicas N]"
        );
        exit(2);
    });
    let format = arg("--format").unwrap_or_else(|| "rudy".into());
    let seed = arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(1u64);
    let sweeps = arg_usize("--sweeps", 200);
    let replicas = arg_usize("--replicas", 256);
    let wall_ms: Option<f64> = arg("--wall-ms").and_then(|s| s.parse().ok());

    if format != "rudy" {
        eprintln!("unsupported format: {format}");
        exit(2);
    }
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("cannot read {file}: {e}");
        exit(1);
    });
    let (n, pairs) = parse_rudy(&text).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        exit(1);
    });

    let ir = maxcut_to_qubo(n, &pairs);
    if !ir.is_integral() {
        eprintln!("instance is non-integral; SparseBitSlice not eligible");
        exit(1);
    }

    // All replicas start identical (all-zeros); independent per-replica RNG
    // draws diverge them into distinct chains after the first hot sweeps.
    let init = vec![0u8; n];
    let mut state = SparseBitSlice::new(&ir, replicas, &init).unwrap_or_else(|e| {
        eprintln!("backend build failed: {e}");
        exit(1);
    });

    // Geometric cooling schedule (QUBO energy units). One sweep per level; the
    // hot head randomizes the chains, the cold tail exploits.
    let (t_hi, t_lo) = (4.0f64, 0.08f64);
    let levels = sweeps.max(1);
    let ratio = if levels > 1 {
        (t_lo / t_hi).powf(1.0 / (levels as f64 - 1.0))
    } else {
        1.0
    };

    let mut op = GibbsColorSweep::new();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut energies = vec![0.0f64; replicas];
    let mut best_energy = f64::INFINITY;
    let mut best_state = vec![0u8; n];
    let clock = Instant::now();

    let mut t = t_hi;
    for _ in 0..levels {
        let view = RuntimeView {
            iteration: 0,
            temperatures: std::slice::from_ref(&t),
            num_replicas: replicas,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        op.apply(&mut state, &view, &mut rng, Budget { sweeps: 1 });

        // Track the best replica seen so far (SA may step uphill).
        state.energies_into(&mut energies);
        if let Some((r, &e)) = energies
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
        {
            if e < best_energy {
                best_energy = e;
                state.extract_into(r, &mut best_state);
            }
        }
        t *= ratio;

        // Optional wall guard only (never affects the trajectory by default).
        if let Some(limit) = wall_ms {
            if clock.elapsed().as_secs_f64() * 1000.0 >= limit {
                break;
            }
        }
    }

    // Emit the best configuration as a compact 0/1 string.
    let mut s = String::with_capacity(n);
    for &b in &best_state {
        s.push(if b == 1 { '1' } else { '0' });
    }
    println!("{{\"state\":\"{s}\",\"energy\":{best_energy}}}");
}
