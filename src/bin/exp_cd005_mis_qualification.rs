//! CD005-Q: paired, equal-deadline MIS qualification on pinned QOBLIB graphs.
//! Research orchestration only; solver mathematics remains in UltimateSolver.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::UltimateSolver;
use serde::Serialize;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const FILES: [&str; 6] = [
    "C125-9.gph",
    "brock200-2.gph",
    "hamming6-4.gph",
    "sloane_1zc_128.gph",
    "johnson8-4-4.gph",
    "football.gph",
];
const BUDGET: Duration = Duration::from_secs(5);

struct Graph {
    n: usize,
    edges: Vec<(usize, usize)>,
}

fn parse_graph(contents: &str) -> Result<Graph, Box<dyn Error>> {
    let mut header: Option<(usize, usize)> = None;
    let mut edges = BTreeSet::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let parts: Vec<_> = raw_line.split_whitespace().collect();
        if parts.is_empty() || parts[0] == "c" {
            continue;
        }
        match parts.as_slice() {
            ["p", "edge", n, m] if header.is_none() => {
                header = Some((n.parse()?, m.parse()?));
            }
            ["e", u, v] => {
                let (n, _) = header.ok_or("edge before DIMACS header")?;
                let u: usize = u.parse()?;
                let v: usize = v.parse()?;
                if u == 0 || v == 0 || u > n || v > n || u == v {
                    return Err(format!("invalid edge at line {}", line_index + 1).into());
                }
                let edge = (u.min(v) - 1, u.max(v) - 1);
                if !edges.insert(edge) {
                    return Err(format!("duplicate edge at line {}", line_index + 1).into());
                }
            }
            _ => return Err(format!("invalid DIMACS line {}", line_index + 1).into()),
        }
    }
    let (n, expected_edges) = header.ok_or("missing DIMACS header")?;
    if edges.len() != expected_edges {
        return Err(format!("header says {expected_edges} edges, got {}", edges.len()).into());
    }
    Ok(Graph {
        n,
        edges: edges.into_iter().collect(),
    })
}

fn mis_qubo(graph: &Graph) -> QuboModel {
    let mut neighbors = vec![Vec::<usize>::new(); graph.n];
    for &(u, v) in &graph.edges {
        neighbors[u].push(v);
        neighbors[v].push(u);
    }
    let mut values = Vec::with_capacity(2 * graph.edges.len());
    let mut col_indices = Vec::with_capacity(2 * graph.edges.len());
    let mut row_offsets = Vec::with_capacity(graph.n + 1);
    row_offsets.push(0);
    for row in &mut neighbors {
        row.sort_unstable();
        for &v in row.iter() {
            col_indices.push(v);
            values.push(2.0);
        }
        row_offsets.push(values.len());
    }
    QuboModel {
        energy_offset: 0.0,
        num_vars: graph.n,
        linear: vec![-1.0; graph.n],
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

#[derive(Serialize)]
struct Solution {
    seed: u64,
    state: String,
    size: usize,
    collisions: usize,
    model_energy: f64,
    duration_ms: f64,
    elapsed_ms: f64,
    valid: bool,
}

fn validate_solution(
    graph: &Graph,
    model: &QuboModel,
    state: &[i8],
    seed: u64,
    duration_ms: f64,
    elapsed_ms: f64,
) -> Solution {
    let valid_spins = state.len() == graph.n && state.iter().all(|&x| x == 0 || x == 1);
    if !valid_spins {
        return Solution {
            seed,
            state: state
                .iter()
                .map(|&x| {
                    if x == 0 {
                        '0'
                    } else if x == 1 {
                        '1'
                    } else {
                        '?'
                    }
                })
                .collect(),
            size: 0,
            collisions: 0,
            model_energy: 0.0,
            duration_ms,
            elapsed_ms,
            valid: false,
        };
    }
    let size = state.iter().filter(|&&x| x == 1).count();
    let collisions = graph
        .edges
        .iter()
        .filter(|&&(u, v)| state[u] == 1 && state[v] == 1)
        .count();
    let model_energy = model.calculate_total_energy(state);
    let formula_energy = -(size as f64) + 2.0 * (collisions as f64);
    Solution {
        seed,
        state: state
            .iter()
            .map(|&x| if x == 1 { '1' } else { '0' })
            .collect(),
        size,
        collisions,
        model_energy,
        duration_ms,
        elapsed_ms,
        valid: collisions == 0 && (model_energy - formula_energy).abs() <= 1e-9,
    }
}

#[derive(Serialize)]
struct Arm {
    completed: usize,
    late_completions: usize,
    elapsed_ms: f64,
    best_size: Option<usize>,
    valid: bool,
    solutions: Vec<Solution>,
}

fn run_arm(
    graph: &Graph,
    model: &QuboModel,
    graph_index: usize,
    campaign: usize,
    use_2opt: bool,
) -> Arm {
    let mut first_solve_start = None;
    let mut solutions = Vec::new();
    let mut late_completions = 0;
    let mut valid = true;
    let mut attempt = 0u64;
    loop {
        let seed = 5_000_000 + 100_000 * graph_index as u64 + 10_000 * campaign as u64 + attempt;
        let solver = UltimateSolver::new(2.5, 0.05, 10, 5, Some(seed)).with_2opt(use_2opt);
        let solve_start = Instant::now();
        let start = *first_solve_start.get_or_insert(solve_start);
        let deadline = start + BUDGET;
        if solve_start >= deadline {
            break;
        }
        let state = solver.solve(model, &[]);
        let finished = Instant::now();
        attempt += 1;
        if finished <= deadline {
            let solution = validate_solution(
                graph,
                model,
                &state,
                seed,
                finished.duration_since(solve_start).as_secs_f64() * 1000.0,
                finished.duration_since(start).as_secs_f64() * 1000.0,
            );
            valid &= solution.valid;
            solutions.push(solution);
        } else {
            late_completions += 1;
        }
    }
    let best_size = solutions.iter().map(|s| s.size).max();
    let start = first_solve_start.expect("at least one solve was started");
    Arm {
        completed: solutions.len(),
        late_completions,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
        best_size,
        valid: valid && !solutions.is_empty(),
        solutions,
    }
}

#[derive(Serialize)]
struct Cell {
    graph: String,
    n: usize,
    edges: usize,
    campaign: usize,
    baseline: Arm,
    candidate: Arm,
    delta: Option<i64>,
    valid: bool,
}

fn run(data_dir: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let mut graphs = Vec::new();
    for name in FILES {
        let text = fs::read_to_string(data_dir.join(name))?;
        let graph = parse_graph(&text)?;
        graphs.push((name, mis_qubo(&graph), graph));
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut writer = BufWriter::new(file);
    let mut failures = 0;
    for (graph_index, (name, model, graph)) in graphs.iter().enumerate() {
        for campaign in 0..3 {
            let candidate_first = (graph_index + campaign) % 2 == 1;
            let (baseline, candidate) = if candidate_first {
                let candidate = run_arm(graph, model, graph_index, campaign, true);
                let baseline = run_arm(graph, model, graph_index, campaign, false);
                (baseline, candidate)
            } else {
                let baseline = run_arm(graph, model, graph_index, campaign, false);
                let candidate = run_arm(graph, model, graph_index, campaign, true);
                (baseline, candidate)
            };
            let delta = baseline
                .best_size
                .zip(candidate.best_size)
                .map(|(b, c)| c as i64 - b as i64);
            let valid = baseline.valid && candidate.valid && delta.is_some();
            failures += usize::from(!valid);
            let cell = Cell {
                graph: (*name).to_string(),
                n: graph.n,
                edges: graph.edges.len(),
                campaign,
                baseline,
                candidate,
                delta,
                valid,
            };
            serde_json::to_writer(&mut writer, &cell)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
            println!(
                "{} campaign {}: delta {:?}, valid {}",
                name, campaign, cell.delta, valid
            );
        }
    }
    if failures > 0 {
        return Err(format!("{failures} invalid cells; raw rows retained").into());
    }
    Ok(())
}

fn calibrate(path: &Path) -> Result<(), Box<dyn Error>> {
    let graph = parse_graph(&fs::read_to_string(path)?)?;
    let model = mis_qubo(&graph);
    let baseline = run_arm(&graph, &model, 0, 0, false);
    let candidate = run_arm(&graph, &model, 0, 0, true);
    if !baseline.valid || !candidate.valid {
        return Err("calibration produced an invalid or empty arm".into());
    }
    println!(
        "calibration {} n={} edges={} baseline_completed={} candidate_completed={} baseline_best={:?} candidate_best={:?}",
        path.display(),
        graph.n,
        graph.edges.len(),
        baseline.completed,
        candidate.completed,
        baseline.best_size,
        candidate.best_size
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if first.as_deref() == Some("--calibrate") {
        let path = args.next().ok_or("--calibrate requires GRAPH_PATH")?;
        if args.next().is_some() {
            return Err("unexpected calibration argument".into());
        }
        return calibrate(Path::new(&path));
    }
    let data_dir = match (first.as_deref(), args.next()) {
        (Some("--data-dir"), Some(path)) => PathBuf::from(path),
        _ => return Err("usage: exp_cd005_mis_qualification --data-dir DIR --output FILE".into()),
    };
    let output = match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--output"), Some(path), None) => PathBuf::from(path),
        _ => return Err("usage: exp_cd005_mis_qualification --data-dir DIR --output FILE".into()),
    };
    run(&data_dir, &output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_rejects_duplicate_edges() {
        assert!(parse_graph("p edge 3 2\ne 1 2\ne 2 1\n").is_err());
    }

    #[test]
    fn mis_energy_matches_independent_count() {
        let graph = parse_graph("p edge 3 2\ne 1 2\ne 2 3\n").unwrap();
        let model = mis_qubo(&graph);
        let valid = validate_solution(&graph, &model, &[1, 0, 1], 7, 0.0, 0.0);
        assert!(valid.valid);
        assert_eq!(valid.size, 2);
        assert_eq!(valid.model_energy, -2.0);
        let invalid = validate_solution(&graph, &model, &[1, 1, 0], 7, 0.0, 0.0);
        assert!(!invalid.valid);
        assert_eq!(invalid.collisions, 1);
        assert_eq!(invalid.model_energy, 0.0);
    }
}
