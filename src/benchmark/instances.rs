//! Benchmark instance acquisition and parsing.
//!
//! Loads the standard combinatorial-optimization test sets used in the
//! Ising/QUBO literature and maps each to this engine's minimization form:
//!
//! - **G-Set** (Stanford) and **Biq Mac** — weighted MaxCut in `rudy` edge
//!   format. MaxCut(S) = Σ w_ij·[s_i≠s_j]; with s∈{0,1},
//!   [s_i≠s_j] = s_i+s_j−2 s_i s_j, so minimizing E(s) = −MaxCut(s) gives
//!   the model below (quadratic pair coeff 2 w_ij, linear −Σ_j w_ij).
//! - **OR-Library BQP** (Beasley) — maximize xᵀQ x over binary x, Q listed as
//!   upper-triangular entries. E = −objective.
//! - **QPLIB** — the *unconstrained binary quadratic* subset only
//!   (objective ½ xᵀQx + bᵀx + c). Constrained instances are refused with a
//!   reason rather than silently mis-modelled.
//!
//! Native objective vs. energy: `maximize` means the natural score is a
//! maximization equal to `−energy` (MaxCut value, BQP objective); otherwise
//! the natural score is the minimized `energy` itself. Downloads go through
//! `curl` into a content cache; nothing is re-fetched if already present.

use crate::core::{CsrMatrix, QuboModel};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A benchmark instance mapped to minimization form.
pub struct Instance {
    pub name: String,
    pub family: String,
    pub model: QuboModel,
    /// True when the natural objective is a maximization equal to `−energy`.
    pub maximize: bool,
    /// Best-known objective in native units, if supplied.
    pub best_known: Option<f64>,
}

impl Instance {
    /// Natural-objective score for a given model energy.
    pub fn native_objective(&self, energy: f64) -> f64 {
        if self.maximize {
            -energy
        } else {
            energy
        }
    }
    pub fn n(&self) -> usize {
        self.model.num_vars
    }
    /// "Better" comparison in native units (higher for maximize, lower else).
    pub fn is_better(&self, a: f64, b: f64) -> bool {
        if self.maximize {
            a > b
        } else {
            a < b
        }
    }
}

/// Assemble a symmetric QUBO from undirected weighted edges `(i, j, pair)`
/// where `pair` is the both-ones coefficient, plus per-variable linear terms.
fn assemble(n: usize, edges: &[(usize, usize, f64)], linear: Vec<f64>) -> QuboModel {
    let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for &(i, j, w) in edges {
        rows[i].push((j, w));
        rows[j].push((i, w));
    }
    let (mut values, mut col_indices, mut row_offsets) = (Vec::new(), Vec::new(), vec![0usize]);
    for row in &mut rows {
        row.sort_by_key(|&(j, _)| j);
        for &(j, w) in row.iter() {
            col_indices.push(j);
            values.push(w);
        }
        row_offsets.push(col_indices.len());
    }
    QuboModel {
        num_vars: n,
        linear,
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
        energy_offset: 0.0,
    }
}

/// Strip a trailing `#`/`%` comment and yield whitespace tokens of a line.
fn tokens(line: &str) -> Vec<&str> {
    let line = line.split(['#', '%']).next().unwrap_or("").trim();
    if line.is_empty() {
        Vec::new()
    } else {
        line.split_whitespace().collect()
    }
}

// --------------------------------------------------------------------------
// Parsers
// --------------------------------------------------------------------------

/// Parse a `rudy` weighted-MaxCut file (G-Set / Biq Mac):
/// header `n m`, then `m` lines `i j w` (1-indexed vertices).
pub fn parse_rudy_maxcut(text: &str, name: &str) -> Result<Instance, String> {
    let mut lines = text.lines().filter(|l| !tokens(l).is_empty());
    let header = lines.next().ok_or("empty MaxCut file")?;
    let ht = tokens(header);
    if ht.len() < 2 {
        return Err(format!("{name}: bad MaxCut header"));
    }
    let n: usize = ht[0].parse().map_err(|_| "bad n")?;
    let m: usize = ht[1].parse().map_err(|_| "bad m")?;
    let mut edges = Vec::with_capacity(m);
    let mut linear = vec![0.0f64; n];
    for l in lines {
        let t = tokens(l);
        if t.len() < 3 {
            continue;
        }
        let i: usize = t[0].parse().map_err(|_| "bad edge i")?;
        let j: usize = t[1].parse().map_err(|_| "bad edge j")?;
        let w: f64 = t[2].parse().map_err(|_| "bad edge w")?;
        if i == 0 || j == 0 || i > n || j > n {
            return Err(format!("{name}: vertex out of range"));
        }
        let (i, j) = (i - 1, j - 1);
        if i == j {
            continue;
        }
        edges.push((i, j, 2.0 * w)); // pair coeff for −MaxCut
        linear[i] -= w;
        linear[j] -= w;
    }
    Ok(Instance {
        name: name.to_string(),
        family: "maxcut".to_string(),
        model: assemble(n, &edges, linear),
        maximize: true, // native = MaxCut value = −energy
        best_known: None,
    })
}

/// Parse OR-Library BQP (`bqpgka`, `bqp50`…): first line is the number of
/// problems; each problem is `n nnz` then `nnz` lines `i j q_ij` (1-indexed,
/// one triangle incl. diagonal). Q is SYMMETRIC with only one triangle
/// listed, so the maximized objective x'Qx counts each off-diagonal TWICE:
/// Σ q_ii x_i + 2·Σ_{i<j} q_ij x_i x_j (verified against the published
/// optimum of bqp50 #1 = 2098).
pub fn parse_orlib_bqp(text: &str, name: &str) -> Result<Vec<Instance>, String> {
    let mut it = text
        .split_whitespace()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .into_iter()
        .peekable();
    let mut next = || it.next().ok_or_else(|| "unexpected EOF in BQP".to_string());
    let p: usize = next()?.parse().map_err(|_| "bad problem count")?;
    let mut out = Vec::with_capacity(p);
    for k in 0..p {
        let n: usize = next()?.parse().map_err(|_| "bad n")?;
        let nnz: usize = next()?.parse().map_err(|_| "bad nnz")?;
        let mut edges = Vec::new();
        let mut linear = vec![0.0f64; n];
        for _ in 0..nnz {
            let i: usize = next()?.parse().map_err(|_| "bad i")?;
            let j: usize = next()?.parse().map_err(|_| "bad j")?;
            let v: f64 = next()?.parse().map_err(|_| "bad q")?;
            if i == 0 || j == 0 || i > n || j > n {
                return Err(format!("{name}: index out of range"));
            }
            let (i, j) = (i - 1, j - 1);
            if i == j {
                linear[i] -= v; // E = −objective
            } else {
                edges.push((i, j, -2.0 * v)); // symmetric Q: off-diag ×2
            }
        }
        out.push(Instance {
            name: format!("{name}.{}", k + 1),
            family: "orlib_bqp".to_string(),
            model: assemble(n, &edges, linear),
            maximize: true,
            best_known: None,
        });
    }
    Ok(out)
}

/// Parse a Biq Mac single-instance sparse BQP (`be*.sparse`, `gka*.sparse`):
/// `n nnz` then `nnz` lines `i j q` with Q symmetric, one triangle listed
/// (off-diagonals count TWICE). Unlike the OR-Library originals, Biq Mac's
/// converted files are for MINIMIZATION of x'Qx (their docs note the
/// OR-Library versions are "given for maximization!"). Verified: gka1a
/// minimum = -3414 exactly matches the published optimum.
pub fn parse_biqmac_sparse(text: &str, name: &str) -> Result<Instance, String> {
    let mut it = text.split_whitespace();
    let mut next = || {
        it.next()
            .ok_or_else(|| "unexpected EOF in Biq Mac sparse".to_string())
    };
    let n: usize = next()?.parse().map_err(|_| "bad n")?;
    let nnz: usize = next()?.parse().map_err(|_| "bad nnz")?;
    let mut edges = Vec::new();
    let mut linear = vec![0.0f64; n];
    for _ in 0..nnz {
        let i: usize = next()?.parse().map_err(|_| "bad i")?;
        let j: usize = next()?.parse().map_err(|_| "bad j")?;
        let v: f64 = next()?.parse().map_err(|_| "bad q")?;
        if i == 0 || j == 0 || i > n || j > n {
            return Err(format!("{name}: index out of range"));
        }
        let (i, j) = (i - 1, j - 1);
        if i == j {
            linear[i] += v;
        } else {
            edges.push((i, j, 2.0 * v));
        }
    }
    Ok(Instance {
        name: name.to_string(),
        family: "biqmac_sparse".to_string(),
        model: assemble(n, &edges, linear),
        maximize: false,
        best_known: None,
    })
}

/// Parse the unconstrained binary-quadratic subset of a QPLIB `.qplib` file.
/// Objective ½ xᵀQx + bᵀx + c with Q lower-triangular entries. Refuses any
/// instance that is not binary-variable and constraint-free, because folding
/// constraints away would change the optimum (which the caller forbids).
pub fn parse_qplib(text: &str, name: &str) -> Result<Instance, String> {
    // QPLIB is line-structured: each logical line holds a fixed number of
    // numeric fields followed by free descriptive text (a comment with no
    // marker). So we read line by line and take only the fields we expect.
    let mut lines = text.lines().filter_map(|l| {
        let t: Vec<&str> = l.split_whitespace().collect();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    });
    let mut next_line = |what: &str| -> Result<Vec<String>, String> {
        lines
            .next()
            .map(|t| t.into_iter().map(|s| s.to_string()).collect())
            .ok_or_else(|| format!("qplib: missing {what}"))
    };
    let _name = next_line("name")?;
    // 3-char problem type: objective / VARIABLES / CONSTRAINTS (QPLIB spec
    // §4.1) — e.g. "QBN" = quadratic objective, binary vars, no constraints.
    // The layout is adaptive: a constraint-count line exists ONLY when the
    // constraints character is not 'N', so unconstrained files have no 'm'
    // line at all.
    let ptype = next_line("type")?[0].clone();
    if !ptype
        .chars()
        .nth(1)
        .unwrap_or('?')
        .eq_ignore_ascii_case(&'B')
    {
        return Err(format!(
            "{name}: QPLIB type {ptype} not binary-variable (unsupported)"
        ));
    }
    if !ptype
        .chars()
        .nth(2)
        .unwrap_or('?')
        .eq_ignore_ascii_case(&'N')
    {
        return Err(format!(
            "{name}: QPLIB type {ptype} is constrained; only unconstrained instances supported"
        ));
    }
    let sense = next_line("sense")?[0].clone();
    let maximize = sense.eq_ignore_ascii_case("maximize");
    let parse1 = |t: &[String], w: &str| -> Result<f64, String> {
        t.first()
            .ok_or_else(|| format!("qplib: empty {w}"))?
            .parse::<f64>()
            .map_err(|_| format!("qplib: bad {w}"))
    };
    let n = parse1(&next_line("n")?, "n")? as usize;
    // Sign so that internally we always MINIMIZE E: a native maximize stores
    // E = −objective; a minimize stores E = +objective. `maximize` drives
    // reporting.
    let obj_sign = if maximize { -1.0 } else { 1.0 };
    let nq = parse1(&next_line("nq")?, "nq")? as usize;
    let mut edges = Vec::new();
    let mut linear = vec![0.0f64; n];
    for _ in 0..nq {
        let t = next_line("quad term")?;
        if t.len() < 3 {
            return Err(format!("{name}: short quad line"));
        }
        let i = parse1(&t[0..1], "qi")? as usize;
        let j = parse1(&t[1..2], "qj")? as usize;
        let q = parse1(&t[2..3], "qv")?;
        if i == 0 || j == 0 || i > n || j > n {
            return Err(format!("{name}: quad index out of range"));
        }
        let (i, j) = (i - 1, j - 1);
        // Objective = ½·Σ_listed q_ij x_i x_j + b'x + c: every LISTED entry
        // (diagonal and off-diagonal alike) carries the ½ factor and is NOT
        // symmetric-doubled. Verified exactly against the official
        // QPLIB_3565 solution file (objective 282.0000).
        if i == j {
            linear[i] += obj_sign * 0.5 * q;
        } else {
            edges.push((i, j, obj_sign * 0.5 * q));
        }
    }
    let b_default = parse1(&next_line("b_default")?, "b_default")?;
    for slot in linear.iter_mut() {
        *slot += obj_sign * b_default;
    }
    let nb = parse1(&next_line("nb")?, "nb")? as usize;
    for _ in 0..nb {
        let t = next_line("linear term")?;
        if t.len() < 2 {
            return Err(format!("{name}: short linear line"));
        }
        let i = parse1(&t[0..1], "bi")? as usize;
        let v = parse1(&t[1..2], "bv")?;
        if i == 0 || i > n {
            return Err(format!("{name}: linear index out of range"));
        }
        // Non-default entries REPLACE the default for that variable.
        linear[i - 1] += obj_sign * (v - b_default);
    }
    let c = next_line("c")
        .ok()
        .and_then(|t| parse1(&t, "c").ok())
        .unwrap_or(0.0);
    let mut model = assemble(n, &edges, linear);
    model.energy_offset = obj_sign * c;
    Ok(Instance {
        name: name.to_string(),
        family: "qplib".to_string(),
        model,
        maximize,
        best_known: None,
    })
}

// --------------------------------------------------------------------------
// Download + registry
// --------------------------------------------------------------------------

/// Cache directory for downloaded raw instance files.
pub fn cache_dir() -> PathBuf {
    PathBuf::from("benchmarks/cache")
}

/// Fetch `url` into the cache (by `filename`) via `curl`, unless already
/// present and non-empty. Returns the file contents.
pub fn download_cached(url: &str, filename: &str, timeout_s: u64) -> Result<String, String> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(filename);
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 0 {
            return std::fs::read_to_string(&path).map_err(|e| e.to_string());
        }
    }
    let status = Command::new("curl")
        .args([
            "-sSL",
            "--fail",
            "--max-time",
            &timeout_s.to_string(),
            "-o",
            path.to_str().ok_or("bad path")?,
            url,
        ])
        .status()
        .map_err(|e| format!("curl failed to launch: {e} (is curl installed?)"))?;
    if !status.success() {
        return Err(format!("download failed ({url}) — offline?"));
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// The canonical Stanford URL for G-Set instance `k`.
pub fn gset_url(k: u32) -> String {
    format!("https://web.stanford.edu/~yyye/yyye/Gset/G{k}")
}

/// A small, widely-cited G-Set subset spanning sizes/densities (n=800…2000).
pub const GSET_SUBSET: &[u32] = &[1, 2, 11, 12, 14, 15, 22, 43, 51];

/// Load a G-Set subset (downloading as needed). `limit` caps how many are
/// fetched (keeps a real run bounded); errors on individual instances are
/// collected, not fatal.
pub fn load_gset(ids: &[u32], limit: usize, timeout_s: u64) -> (Vec<Instance>, Vec<String>) {
    let mut ok = Vec::new();
    let mut errs = Vec::new();
    for &k in ids.iter().take(limit) {
        let name = format!("G{k}");
        match download_cached(&gset_url(k), &name, timeout_s)
            .and_then(|t| parse_rudy_maxcut(&t, &name))
        {
            Ok(inst) => ok.push(inst),
            Err(e) => errs.push(format!("{name}: {e}")),
        }
    }
    (ok, errs)
}

/// Load best-known objectives from an optional `benchmarks/best_known.csv`
/// (`name,value` rows) and attach them to instances by name. Avoids baking
/// literature numbers into the binary (transcription risk); users supply
/// verified optima when they have them.
pub fn attach_best_known(instances: &mut [Instance], path: &Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let t: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if t.len() < 2 {
            continue;
        }
        if let Ok(v) = t[1].parse::<f64>() {
            for inst in instances.iter_mut() {
                if inst.name == t[0] {
                    inst.best_known = Some(v);
                }
            }
        }
    }
}
