//! EXP-007W: frozen weighted-MIS structural probe and equal-time CD005 comparison.
//! Research orchestration only; no solver implementation changes.

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::{local_search, UltimateSolver};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(2);
const SPECS: [(usize, usize); 6] = [(64, 8), (64, 16), (96, 8), (96, 16), (128, 8), (128, 16)];
const PENALTY: f64 = 11.0;

#[derive(Deserialize)]
struct Dataset {
    experiment_id: String,
    generator: String,
    instances: Vec<Graph>,
}

#[derive(Deserialize)]
struct Graph {
    name: String,
    n: usize,
    edge_percent: usize,
    weights: Vec<i64>,
    edges: Vec<(usize, usize)>,
}

fn validate_graphs(data: Dataset) -> Result<Vec<Graph>, Box<dyn Error>> {
    if data.experiment_id != "EXP-007W"
        || data.generator != "SHA-256-v1"
        || data.instances.len() != SPECS.len()
    {
        return Err("unexpected dataset identity or number of graphs".into());
    }
    for (g, graph) in data.instances.iter().enumerate() {
        let (n, p) = SPECS[g];
        if graph.name != format!("er_n{n}_p{p:02}_g{g}")
            || graph.n != n
            || graph.edge_percent != p
            || graph.weights.len() != n
            || graph.weights.iter().any(|&w| !(1..=10).contains(&w))
        {
            return Err(format!("invalid metadata or weights for graph {g}").into());
        }
        let mut seen = BTreeSet::new();
        for &(u, v) in &graph.edges {
            if u >= v || v >= n || !seen.insert((u, v)) {
                return Err(format!("invalid or duplicate edge in graph {g}").into());
            }
        }
        if graph.edges.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(format!("unsorted edges in graph {g}").into());
        }
    }
    Ok(data.instances)
}

fn model_for(graph: &Graph) -> QuboModel {
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
            values.push(PENALTY);
        }
        row_offsets.push(values.len());
    }
    QuboModel {
        energy_offset: 0.0,
        num_vars: graph.n,
        linear: graph.weights.iter().map(|&w| -(w as f64)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
    }
}

fn direct_energy(graph: &Graph, state: &[i8]) -> f64 {
    let reward: i64 = graph
        .weights
        .iter()
        .zip(state)
        .map(|(&weight, &bit)| weight * i64::from(bit))
        .sum();
    let collisions = graph
        .edges
        .iter()
        .filter(|&&(u, v)| state[u] == 1 && state[v] == 1)
        .count();
    -(reward as f64) + PENALTY * collisions as f64
}

#[derive(Serialize)]
struct Pair {
    u: usize,
    v: usize,
    delta: f64,
}

#[derive(Serialize)]
struct Probe {
    evaluation_id: &'static str,
    graph: String,
    start_index: usize,
    seed: u64,
    fixed: Vec<(usize, i8)>,
    state: String,
    model_energy: f64,
    direct_energy: f64,
    min_single_delta: Option<f64>,
    min_pair_delta: Option<f64>,
    improving_pair: Option<Pair>,
    valid: bool,
}

fn state_string(state: &[i8]) -> String {
    state
        .iter()
        .map(|&bit| match bit {
            0 => '0',
            1 => '1',
            _ => '?',
        })
        .collect()
}

fn flip_energy(graph: &Graph, state: &mut [i8], indices: &[usize]) -> f64 {
    for &index in indices {
        state[index] = 1 - state[index];
    }
    let energy = direct_energy(graph, state);
    for &index in indices {
        state[index] = 1 - state[index];
    }
    energy
}

fn structural_probe(
    graph: &Graph,
    model: &QuboModel,
    g: usize,
    s: usize,
    fixed: &[(usize, i8)],
) -> Probe {
    let seed = 9_000_000 + 1_000 * g as u64 + s as u64;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut state = vec![0i8; graph.n];
    let mut is_fixed = vec![false; graph.n];
    for &(index, value) in fixed {
        state[index] = value;
        is_fixed[index] = true;
    }
    for index in 0..graph.n {
        if !is_fixed[index] {
            state[index] = i8::from(rng.gen_bool(0.5));
        }
    }
    local_search::steepest_descent_1opt(model, &mut state, &is_fixed);
    let model_energy = model.calculate_total_energy(&state);
    let energy = direct_energy(graph, &state);
    let mut min_single_delta: Option<f64> = None;
    let mut min_pair_delta: Option<f64> = None;
    let mut improving_pair = None;
    for (u, &fixed_bit) in is_fixed.iter().enumerate() {
        if !fixed_bit {
            let delta = flip_energy(graph, &mut state, &[u]) - energy;
            min_single_delta = Some(min_single_delta.map_or(delta, |old| old.min(delta)));
        }
    }
    for &(u, v) in &graph.edges {
        if !is_fixed[u] && !is_fixed[v] {
            let delta = flip_energy(graph, &mut state, &[u, v]) - energy;
            min_pair_delta = Some(min_pair_delta.map_or(delta, |old| old.min(delta)));
            if delta < -1e-9
                && improving_pair
                    .as_ref()
                    .is_none_or(|p: &Pair| delta < p.delta)
            {
                improving_pair = Some(Pair { u, v, delta });
            }
        }
    }
    let valid = state.iter().all(|&x| x == 0 || x == 1)
        && (model_energy - energy).abs() <= 1e-9
        && min_single_delta.is_none_or(|delta| delta >= -1e-9)
        && fixed.iter().all(|&(i, v)| state[i] == v);
    Probe {
        evaluation_id: "EXP-007W",
        graph: graph.name.clone(),
        start_index: s,
        seed,
        fixed: fixed.to_vec(),
        state: state_string(&state),
        model_energy,
        direct_energy: energy,
        min_single_delta,
        min_pair_delta,
        improving_pair,
        valid,
    }
}

#[derive(Serialize)]
struct Solution {
    seed: u64,
    state: String,
    weight: i64,
    collisions: usize,
    model_energy: f64,
    duration_ms: f64,
    elapsed_ms: f64,
    valid: bool,
}

fn solution(
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
            state: state_string(state),
            weight: 0,
            collisions: 0,
            model_energy: 0.0,
            duration_ms,
            elapsed_ms,
            valid: false,
        };
    }
    let weight: i64 = graph
        .weights
        .iter()
        .zip(state)
        .map(|(&w, &x)| w * i64::from(x))
        .sum();
    let collisions = graph
        .edges
        .iter()
        .filter(|&&(u, v)| state[u] == 1 && state[v] == 1)
        .count();
    let model_energy = model.calculate_total_energy(state);
    let direct = direct_energy(graph, state);
    Solution {
        seed,
        state: state_string(state),
        weight,
        collisions,
        model_energy,
        duration_ms,
        elapsed_ms,
        valid: collisions == 0 && (model_energy - direct).abs() <= 1e-9,
    }
}

#[derive(Serialize)]
struct Arm {
    completed: usize,
    late_completions: usize,
    elapsed_ms: f64,
    best_weight: Option<i64>,
    valid: bool,
    solutions: Vec<Solution>,
}

fn run_arm(
    graph: &Graph,
    model: &QuboModel,
    g: usize,
    c: usize,
    use_2opt: bool,
    budget: Duration,
) -> Arm {
    let mut first_start = None;
    let mut solutions = Vec::new();
    let mut late_completions = 0;
    let mut attempt = 0u64;
    let mut valid = true;
    loop {
        let seed = 7_000_000 + 100_000 * g as u64 + 10_000 * c as u64 + attempt;
        let solver = UltimateSolver::new(2.5, 0.05, 10, 5, Some(seed)).with_2opt(use_2opt);
        let solve_start = Instant::now();
        let start = *first_start.get_or_insert(solve_start);
        let deadline = start + budget;
        if solve_start >= deadline {
            break;
        }
        let state = solver.solve(model, &[]);
        let finished = Instant::now();
        attempt += 1;
        if finished <= deadline {
            let found = solution(
                graph,
                model,
                &state,
                seed,
                finished.duration_since(solve_start).as_secs_f64() * 1000.0,
                finished.duration_since(start).as_secs_f64() * 1000.0,
            );
            valid &= found.valid;
            solutions.push(found);
        } else {
            late_completions += 1;
        }
    }
    let best_weight = solutions.iter().map(|s| s.weight).max();
    Arm {
        completed: solutions.len(),
        late_completions,
        elapsed_ms: first_start
            .expect("one solve starts")
            .elapsed()
            .as_secs_f64()
            * 1000.0,
        best_weight,
        valid: valid && !solutions.is_empty(),
        solutions,
    }
}

#[derive(Serialize)]
struct Cell {
    evaluation_id: &'static str,
    graph: String,
    n: usize,
    edges: usize,
    campaign: usize,
    baseline: Arm,
    candidate: Arm,
    delta: Option<i64>,
    valid: bool,
}

fn new_writer(path: &Path) -> Result<BufWriter<std::fs::File>, Box<dyn Error>> {
    Ok(BufWriter::new(
        OpenOptions::new().write(true).create_new(true).open(path)?,
    ))
}

fn run(dataset_path: &Path, structural_path: &Path, raw_path: &Path) -> Result<(), Box<dyn Error>> {
    let dataset: Dataset = serde_json::from_slice(&fs::read(dataset_path)?)?;
    let graphs = validate_graphs(dataset)?;
    let models: Vec<_> = graphs.iter().map(model_for).collect();
    let mut structural = new_writer(structural_path)?;
    let mut raw = new_writer(raw_path)?;
    let mut failures = 0usize;
    for (g, (graph, model)) in graphs.iter().zip(&models).enumerate() {
        let fixed = ising_engine::presolve::full_presolve(model, &[]);
        for s in 0..10 {
            let probe = structural_probe(graph, model, g, s, &fixed);
            failures += usize::from(!probe.valid);
            serde_json::to_writer(&mut structural, &probe)?;
            structural.write_all(b"\n")?;
            structural.flush()?;
        }
        for c in 0..10 {
            let candidate_first = (g + c) % 2 == 1;
            let (baseline, candidate) = if candidate_first {
                let candidate = run_arm(graph, model, g, c, true, BUDGET);
                let baseline = run_arm(graph, model, g, c, false, BUDGET);
                (baseline, candidate)
            } else {
                let baseline = run_arm(graph, model, g, c, false, BUDGET);
                let candidate = run_arm(graph, model, g, c, true, BUDGET);
                (baseline, candidate)
            };
            let delta = baseline
                .best_weight
                .zip(candidate.best_weight)
                .map(|(b, a)| a - b);
            let valid = baseline.valid && candidate.valid && delta.is_some();
            failures += usize::from(!valid);
            let cell = Cell {
                evaluation_id: "EXP-007W",
                graph: graph.name.clone(),
                n: graph.n,
                edges: graph.edges.len(),
                campaign: c,
                baseline,
                candidate,
                delta,
                valid,
            };
            serde_json::to_writer(&mut raw, &cell)?;
            raw.write_all(b"\n")?;
            raw.flush()?;
            println!(
                "{} campaign {}: delta {:?}, valid {}",
                graph.name, c, cell.delta, valid
            );
        }
    }
    if failures > 0 {
        return Err(format!("{failures} invalid probe/cell rows; raw outputs retained").into());
    }
    Ok(())
}

fn calibrate() -> Result<(), Box<dyn Error>> {
    let graph = Graph {
        name: "calibration-edge".into(),
        n: 2,
        edge_percent: 100,
        weights: vec![1, 2],
        edges: vec![(0, 1)],
    };
    let model = model_for(&graph);
    let state = [1, 0];
    let mut one_opt = state;
    local_search::steepest_descent_1opt(&model, &mut one_opt, &[false, false]);
    let mut two_opt = one_opt;
    local_search::steepest_descent_2opt_escapes(&model, &mut two_opt, &[false, false]);
    if one_opt != [1, 0]
        || two_opt != [0, 1]
        || direct_energy(&graph, &two_opt) >= direct_energy(&graph, &one_opt)
    {
        return Err("weighted two-flip calibration failed".into());
    }
    let baseline = run_arm(&graph, &model, 0, 0, false, Duration::from_millis(200));
    let candidate = run_arm(&graph, &model, 0, 0, true, Duration::from_millis(200));
    if !baseline.valid || !candidate.valid {
        return Err("timed calibration produced invalid arm".into());
    }
    println!("calibration passed: one_opt=-1, two_opt=-2; arms valid");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--calibrate"] {
        return calibrate();
    }
    if args.len() != 6 || args[0] != "--data" || args[2] != "--structural" || args[4] != "--raw" {
        return Err("usage: exp007_weighted_mis --data FILE --structural FILE --raw FILE".into());
    }
    run(
        Path::new(&args[1]),
        Path::new(&args[3]),
        Path::new(&args[5]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_pair_witness_and_energy_agree() {
        let graph = Graph {
            name: "witness".into(),
            n: 2,
            edge_percent: 100,
            weights: vec![1, 2],
            edges: vec![(0, 1)],
        };
        let model = model_for(&graph);
        for state in [[0, 0], [1, 0], [0, 1], [1, 1]] {
            assert_eq!(
                model.calculate_total_energy(&state),
                direct_energy(&graph, &state)
            );
        }
        let mut state = [1, 0];
        local_search::steepest_descent_1opt(&model, &mut state, &[false, false]);
        assert_eq!(state, [1, 0]);
        local_search::steepest_descent_2opt_escapes(&model, &mut state, &[false, false]);
        assert_eq!(state, [0, 1]);
    }

    #[test]
    fn rejects_duplicate_edges() {
        let data = Dataset {
            experiment_id: "EXP-007W".into(),
            generator: "SHA-256-v1".into(),
            instances: (0..6)
                .map(|g| {
                    let (n, p) = SPECS[g];
                    Graph {
                        name: format!("er_n{n}_p{p:02}_g{g}"),
                        n,
                        edge_percent: p,
                        weights: vec![1; n],
                        edges: if g == 0 { vec![(0, 1), (0, 1)] } else { vec![] },
                    }
                })
                .collect(),
        };
        assert!(validate_graphs(data).is_err());
    }
}
