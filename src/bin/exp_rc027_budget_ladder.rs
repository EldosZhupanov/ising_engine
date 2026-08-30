//! Frozen RC-027 budget-axis harness.
//!
//! Binding protocol: `research/PREREG_RC027_BUDGET_AXIS.md`.
//!
//! The two registered operator pairs are run unchanged at four common sweep
//! budgets. This binary is private research instrumentation: it accepts one
//! exact command, emits paired quality observations, and makes no timing claim.

use ising_engine::engine_v2::ai_scientist::{
    BatchExecutor, ExperimentOutcome, ExperimentTask, RuntimeExecutor,
};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

const INSTANCE_DIR: &str = "benchmark_suite/data/gset";
const BUDGETS: [u32; 4] = [50, 200, 800, 3200];
const REPLICAS: usize = 32;
const SEEDS: [u64; 3] = [401, 402, 403];
const EXPECTED_INSTANCES: usize = 30;
const EXPECTED_ROWS: usize = 720;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pair {
    name: &'static str,
    control: &'static str,
    candidate: &'static str,
    never_worse: bool,
}

const PAIRS: [Pair; 2] = [
    Pair {
        name: "synthesis",
        control: "metropolis_sweep",
        candidate: "path_relink_sweep",
        never_worse: true,
    },
    Pair {
        name: "memory_form",
        control: "history_field",
        candidate: "tabu_sweep",
        never_worse: false,
    },
];

fn frozen_args(args: &[String]) -> bool {
    const EXPECTED: [&str; 8] = [
        "--dir",
        INSTANCE_DIR,
        "--budgets",
        "50,200,800,3200",
        "--replicas",
        "32",
        "--seeds",
        "401,402,403",
    ];
    args.len() == EXPECTED.len() + 1 && args[1..].iter().map(String::as_str).eq(EXPECTED)
}

fn invalid(why: &str) -> ! {
    eprintln!("RC027_INSTRUMENT_INVALID\t{why}");
    std::process::exit(4);
}

fn is_gset_instance(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix('G'))
        .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
}

fn validate_corpus_paths(paths: &[PathBuf]) -> Result<(), String> {
    if paths.len() != EXPECTED_INSTANCES {
        return Err(format!(
            "found {} instances, not the {EXPECTED_INSTANCES} PREREG §2 froze",
            paths.len()
        ));
    }
    if let Some(path) = paths.iter().find(|path| !is_gset_instance(path)) {
        return Err(format!("{} is not a frozen G-Set instance", path.display()));
    }
    let mut names = HashSet::with_capacity(paths.len());
    for path in paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{} has a non-UTF-8 instance name", path.display()))?;
        if !names.insert(name) {
            return Err(format!("duplicate instance {name}"));
        }
    }
    Ok(())
}

fn collect_corpus(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("read directory entry: {e}"))?;
        let path = entry.path();
        if is_gset_instance(&path) {
            if !path.is_file() {
                return Err(format!("{} is not a regular file", path.display()));
            }
            paths.push(path);
        }
    }
    paths.sort();
    validate_corpus_paths(&paths)?;
    Ok(paths)
}

#[derive(Clone, Debug)]
struct RawEdge {
    u: usize,
    v: usize,
    weight: f64,
}

#[derive(Clone, Debug)]
struct RawRudy {
    n: usize,
    edges: Vec<RawEdge>,
}

/// Inspect the bytes before the frontend. `parse_rudy` intentionally drops
/// self-loops and out-of-range endpoints, so its output cannot police those
/// conditions without making the guard vacuous.
fn validate_raw_rudy(text: &str) -> Result<RawRudy, String> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header: Vec<&str> = lines
        .next()
        .ok_or("empty file")?
        .split_whitespace()
        .collect();
    if header.len() != 2 {
        return Err("header must contain exactly n and edge_count".to_string());
    }
    let n: usize = header[0].parse().map_err(|_| "bad vertex count")?;
    let declared: usize = header[1].parse().map_err(|_| "bad edge count")?;
    if n == 0 {
        return Err("vertex count is zero".to_string());
    }
    let mut edges = Vec::with_capacity(declared);
    for (offset, line) in lines.enumerate() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 3 {
            return Err(format!(
                "body line {} must contain exactly u, v and weight",
                offset + 2
            ));
        }
        let u: usize = fields[0].parse().map_err(|_| "bad u")?;
        let v: usize = fields[1].parse().map_err(|_| "bad v")?;
        let weight: f64 = fields[2].parse().map_err(|_| "bad weight")?;
        if !(1..=n).contains(&u) || !(1..=n).contains(&v) {
            return Err(format!(
                "body line {} has an out-of-range endpoint",
                offset + 2
            ));
        }
        if u == v {
            return Err(format!("body line {} contains a self-loop", offset + 2));
        }
        if !weight.is_finite() {
            return Err(format!("body line {} has a non-finite weight", offset + 2));
        }
        edges.push(RawEdge {
            u: u - 1,
            v: v - 1,
            weight,
        });
    }
    if edges.len() != declared {
        return Err(format!(
            "header declares {declared} edges, the body has {}",
            edges.len()
        ));
    }
    Ok(RawRudy { n, edges })
}

fn energy_matches_raw(ir: &ProblemIR, raw: &RawRudy) -> Result<(), String> {
    if ir.n != raw.n {
        return Err(format!("frontend has n={}, raw file has n={}", ir.n, raw.n));
    }
    for (label, assignment) in [
        (
            "alternating",
            (0..raw.n).map(|i| (i % 2) as u8).collect::<Vec<_>>(),
        ),
        (
            "split",
            (0..raw.n)
                .map(|i| u8::from(i * 2 >= raw.n))
                .collect::<Vec<_>>(),
        ),
    ] {
        let cut: f64 = raw
            .edges
            .iter()
            .filter(|edge| assignment[edge.u] != assignment[edge.v])
            .map(|edge| edge.weight)
            .sum();
        let expected = -cut;
        let reported = ir.energy(&assignment);
        if reported.to_bits() != expected.to_bits() {
            return Err(format!(
                "{label} assignment: frontend energy {reported} is not raw cut {expected}"
            ));
        }
    }
    Ok(())
}

fn load_instance(path: &Path) -> Result<ProblemIR, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    let raw = validate_raw_rudy(&text)?;
    let ir = rudy_maxcut_ir(&text).map_err(|e| format!("frontend: {e}"))?;
    energy_matches_raw(&ir, &raw)?;
    Ok(ir)
}

fn task(op: &str, sweeps: u32, seed: u64) -> ExperimentTask {
    ExperimentTask {
        schedule: Schedule {
            ops: vec![op.to_string()],
            sweeps: vec![sweeps],
            temp_hi: 4.0,
            temp_lo: 0.1,
        },
        num_replicas: REPLICAS,
        seed,
    }
}

fn validate_task(task: &ExperimentTask, op: &str, sweeps: u32, seed: u64) -> Result<(), String> {
    if task.schedule.ops != [op] {
        return Err(format!("{op} arm has the wrong operator schedule"));
    }
    if task.schedule.sweeps != [sweeps] {
        return Err(format!("{op} arm does not spend {sweeps} sweeps"));
    }
    if task.num_replicas != REPLICAS
        || task.seed != seed
        || task.schedule.temp_hi.to_bits() != 4.0f64.to_bits()
        || task.schedule.temp_lo.to_bits() != 0.1f64.to_bits()
    {
        return Err(format!(
            "{op} arm differs from the frozen execution parameters"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
struct ValidatedPair {
    control: f64,
    candidate: f64,
    gain: f64,
}

fn validate_outcomes(
    pair: Pair,
    ir: &ProblemIR,
    observed: &[ExperimentOutcome],
) -> Result<ValidatedPair, String> {
    if observed.len() != 2 {
        return Err(format!("{} did not return both frozen arms", pair.name));
    }
    for (label, outcome) in [("control", &observed[0]), ("candidate", &observed[1])] {
        if !outcome.score.is_finite() {
            return Err(format!("{} {label} score is non-finite", pair.name));
        }
        if outcome.best_state.len() != ir.n {
            return Err(format!("{} {label} state has the wrong length", pair.name));
        }
        let rescored = ir.energy(&outcome.best_state);
        if rescored.to_bits() != outcome.score.to_bits() {
            return Err(format!(
                "{} {label} score {} differs from canonical rescore {rescored}",
                pair.name, outcome.score
            ));
        }
    }
    let control = observed[0].score;
    let candidate = observed[1].score;
    if pair.never_worse && candidate > control {
        return Err(format!(
            "{} candidate worsened: {candidate} > {control}",
            pair.name
        ));
    }
    let gain = if control.abs() > 1e-12 {
        (control - candidate) / control.abs()
    } else {
        0.0
    };
    if !gain.is_finite() {
        return Err(format!("{} gain is non-finite", pair.name));
    }
    Ok(ValidatedPair {
        control,
        candidate,
        gain,
    })
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct RowKey {
    pair: &'static str,
    instance: String,
    sweeps: u32,
    seed: u64,
}

#[derive(Clone, Debug)]
struct Row {
    key: RowKey,
    control: f64,
    candidate: f64,
    gain: f64,
}

fn record_key(seen: &mut HashSet<RowKey>, key: &RowKey) -> Result<(), String> {
    if !seen.insert(key.clone()) {
        return Err(format!(
            "duplicate row ({}, {}, {}, {})",
            key.pair, key.instance, key.sweeps, key.seed
        ));
    }
    Ok(())
}

fn validate_complete(seen: &HashSet<RowKey>, instances: &[String]) -> Result<(), String> {
    let mut expected = HashSet::with_capacity(EXPECTED_ROWS);
    for pair in PAIRS {
        for instance in instances {
            for sweeps in BUDGETS {
                for seed in SEEDS {
                    expected.insert(RowKey {
                        pair: pair.name,
                        instance: instance.clone(),
                        sweeps,
                        seed,
                    });
                }
            }
        }
    }
    if expected.len() != EXPECTED_ROWS || seen != &expected {
        return Err(format!(
            "observed {} unique rows, not the complete {EXPECTED_ROWS}-row key set",
            seen.len()
        ));
    }
    Ok(())
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn normal_cdf(z: f64) -> f64 {
    // Abramowitz-Stegun 7.1.26; ample precision for the frozen alpha=0.05 gate.
    let x = z.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * x);
    let density = 0.398_942_280_401_432_7 * (-0.5 * x * x).exp();
    let tail = density
        * t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    if z >= 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

fn wilcoxon_signed_rank_p(values: &[f64]) -> f64 {
    let mut ranked: Vec<(f64, bool)> = values
        .iter()
        .copied()
        .filter(|value| *value != 0.0)
        .map(|value| (value.abs(), value > 0.0))
        .collect();
    if ranked.is_empty() {
        return 1.0;
    }
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut assigned = Vec::with_capacity(ranked.len());
    let mut start = 0usize;
    while start < ranked.len() {
        let mut end = start + 1;
        while end < ranked.len() && ranked[end].0.to_bits() == ranked[start].0.to_bits() {
            end += 1;
        }
        let average_rank = ((start + 1 + end) as f64) / 2.0;
        assigned.extend(
            ranked[start..end]
                .iter()
                .map(|(_, positive)| (average_rank, *positive)),
        );
        start = end;
    }
    let total_rank: f64 = assigned.iter().map(|(rank, _)| rank).sum();
    let positive_rank: f64 = assigned
        .iter()
        .filter(|(_, positive)| *positive)
        .map(|(rank, _)| rank)
        .sum();
    let variance: f64 = assigned.iter().map(|(rank, _)| rank * rank).sum::<f64>() / 4.0;
    if variance == 0.0 {
        return 1.0;
    }
    let z = (positive_rank - total_rank / 2.0) / variance.sqrt();
    (2.0 * (1.0 - normal_cdf(z.abs()))).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug)]
struct BaselineControls {
    synthesis_mean: f64,
    memory_mean: f64,
    memory_wilcoxon_p: f64,
}

fn validate_baseline_controls(rows: &[Row]) -> Result<BaselineControls, String> {
    let mut groups: BTreeMap<(&str, &str), Vec<f64>> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.key.sweeps == 50) {
        groups
            .entry((row.key.pair, row.key.instance.as_str()))
            .or_default()
            .push(row.gain);
    }
    let per_instance = |pair: &str| -> Result<Vec<f64>, String> {
        let mut values = Vec::with_capacity(EXPECTED_INSTANCES);
        for ((observed_pair, instance), gains) in &groups {
            if *observed_pair != pair {
                continue;
            }
            if gains.len() != SEEDS.len() {
                return Err(format!(
                    "{pair}/{instance} has {} 50-sweep seeds, not {}",
                    gains.len(),
                    SEEDS.len()
                ));
            }
            values.push(mean(gains));
        }
        if values.len() != EXPECTED_INSTANCES {
            return Err(format!(
                "{pair} has {} instance means at 50 sweeps, not {EXPECTED_INSTANCES}",
                values.len()
            ));
        }
        Ok(values)
    };
    let synthesis = per_instance("synthesis")?;
    let memory = per_instance("memory_form")?;
    let synthesis_mean = mean(&synthesis);
    let memory_mean = mean(&memory);
    let memory_wilcoxon_p = wilcoxon_signed_rank_p(&memory);
    if synthesis_mean <= 0.0 || synthesis_mean >= 0.01 {
        return Err(format!(
            "50-sweep synthesis control mean {synthesis_mean} does not reproduce RC-024"
        ));
    }
    if memory_mean.abs() >= 0.01 || memory_wilcoxon_p < 0.05 {
        return Err(format!(
            "50-sweep memory control mean={memory_mean} p={memory_wilcoxon_p} does not reproduce RC-023"
        ));
    }
    Ok(BaselineControls {
        synthesis_mean,
        memory_mean,
        memory_wilcoxon_p,
    })
}

fn render_rows(rows: &[Row]) -> String {
    let mut rendered = String::from(
        "pair\tinstance\tsweeps\tseed\tcontrol_energy\tcandidate_energy\trelative_gain\n",
    );
    for row in rows {
        rendered.push_str(&format!(
            "{}\t{}\t{}\t{}\t{:.17}\t{:.17}\t{:.17}\n",
            row.key.pair,
            row.key.instance,
            row.key.sweeps,
            row.key.seed,
            row.control,
            row.candidate,
            row.gain
        ));
    }
    rendered
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if !frozen_args(&args) {
        eprintln!(
            "usage: exp_rc027_budget_ladder --dir {INSTANCE_DIR} \
             --budgets 50,200,800,3200 --replicas 32 --seeds 401,402,403"
        );
        std::process::exit(2);
    }

    let registry = OperatorRegistry::standard();
    for pair in PAIRS {
        for op in [pair.control, pair.candidate] {
            if !registry.contains(op) {
                invalid(&format!("{op} is absent from the standard registry"));
            }
        }
    }
    let paths = collect_corpus(Path::new(INSTANCE_DIR)).unwrap_or_else(|e| invalid(&e));
    let instances: Vec<String> = paths
        .iter()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_else(|| invalid("an instance name is not UTF-8"))
                .to_string()
        })
        .collect();
    let executor = RuntimeExecutor::auto();
    let mut rows = Vec::with_capacity(EXPECTED_ROWS);
    let mut seen = HashSet::with_capacity(EXPECTED_ROWS);

    for (path, instance) in paths.iter().zip(&instances) {
        let ir =
            load_instance(path).unwrap_or_else(|e| invalid(&format!("{}: {e}", path.display())));
        for pair in PAIRS {
            for sweeps in BUDGETS {
                for seed in SEEDS {
                    let tasks = [
                        task(pair.control, sweeps, seed),
                        task(pair.candidate, sweeps, seed),
                    ];
                    validate_task(&tasks[0], pair.control, sweeps, seed)
                        .and_then(|()| validate_task(&tasks[1], pair.candidate, sweeps, seed))
                        .unwrap_or_else(|e| invalid(&e));
                    let observed = executor.run_batch(&ir, &registry, &tasks);
                    let validated = validate_outcomes(pair, &ir, &observed)
                        .unwrap_or_else(|e| invalid(&format!("{instance}/{sweeps}/{seed}: {e}")));
                    let key = RowKey {
                        pair: pair.name,
                        instance: instance.clone(),
                        sweeps,
                        seed,
                    };
                    record_key(&mut seen, &key).unwrap_or_else(|e| invalid(&e));
                    rows.push(Row {
                        key,
                        control: validated.control,
                        candidate: validated.candidate,
                        gain: validated.gain,
                    });
                }
            }
        }
    }

    validate_complete(&seen, &instances).unwrap_or_else(|e| invalid(&e));
    let rendered = render_rows(&rows);
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    out.write_all(rendered.as_bytes())
        .unwrap_or_else(|e| invalid(&format!("stdout write: {e}")));
    out.flush()
        .unwrap_or_else(|e| invalid(&format!("stdout flush: {e}")));

    // The raw evidence has been emitted before the mandatory controls are
    // evaluated, so even an instrument-invalid outcome remains publishable.
    let controls = validate_baseline_controls(&rows).unwrap_or_else(|e| invalid(&e));
    eprintln!(
        "RC027_COMPLETE\trows={}\tinstances={}\tsynthesis_50_mean={:.17}\t\
         memory_50_mean={:.17}\tmemory_50_wilcoxon_p={:.17}",
        rows.len(),
        instances.len(),
        controls.synthesis_mean,
        controls.memory_mean,
        controls.memory_wilcoxon_p
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ising_engine::engine_v2::plan::Backend;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("bin".to_string())
            .chain(list.iter().map(|value| value.to_string()))
            .collect()
    }

    #[test]
    fn only_the_preregistered_command_is_accepted() {
        let exact = args(&[
            "--dir",
            INSTANCE_DIR,
            "--budgets",
            "50,200,800,3200",
            "--replicas",
            "32",
            "--seeds",
            "401,402,403",
        ]);
        assert!(frozen_args(&exact));
        for position in 1..exact.len() {
            let mut changed = exact.clone();
            changed[position].push('x');
            assert!(!frozen_args(&changed), "position {position}");
        }
        assert!(!frozen_args(&args(&[])));
    }

    #[test]
    fn only_the_exact_thirty_gset_instances_are_accepted() {
        let paths: Vec<PathBuf> = (1..=EXPECTED_INSTANCES)
            .map(|index| PathBuf::from(format!("x/G{index}")))
            .collect();
        assert!(validate_corpus_paths(&paths).is_ok());
        assert!(validate_corpus_paths(&paths[..paths.len() - 1]).is_err());
        let mut wrong = paths.clone();
        wrong[0] = PathBuf::from("x/README.md");
        assert!(validate_corpus_paths(&wrong).is_err());
        let mut duplicate = paths.clone();
        duplicate[1] = duplicate[0].clone();
        assert!(validate_corpus_paths(&duplicate).is_err());
    }

    #[test]
    fn registered_corpus_is_raw_valid_and_frontend_equivalent() {
        let dir = Path::new(INSTANCE_DIR);
        if !dir.is_dir() {
            // A package consumer may run tests without the benchmark corpus.
            return;
        }
        let paths = collect_corpus(dir).expect("the registered corpus resolves exactly");
        assert_eq!(paths.len(), EXPECTED_INSTANCES);
        for path in paths {
            load_instance(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        }
    }

    #[test]
    fn raw_rudy_structure_is_checked_before_the_frontend() {
        let honest = "4 4\n1 2 -7\n2 3 11\n3 4 -2\n1 4 5\n";
        assert!(validate_raw_rudy(honest).is_ok());
        for bad in [
            "4 5\n1 2 -7\n2 3 11\n3 4 -2\n1 4 5\n",
            "4 4\n1 1 -7\n2 3 11\n3 4 -2\n1 4 5\n",
            "4 4\n1 5 -7\n2 3 11\n3 4 -2\n1 4 5\n",
            "4 4\n1 2 -7 extra\n2 3 11\n3 4 -2\n1 4 5\n",
            "4 4\n1 2 NaN\n2 3 11\n3 4 -2\n1 4 5\n",
        ] {
            assert!(validate_raw_rudy(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn frontend_energy_must_match_the_raw_edges() {
        let text = "4 4\n1 2 -7\n2 3 11\n3 4 -2\n1 4 5\n";
        let raw = validate_raw_rudy(text).unwrap();
        let honest = rudy_maxcut_ir(text).unwrap();
        assert!(energy_matches_raw(&honest, &raw).is_ok());
        let doubled = rudy_maxcut_ir("4 4\n1 2 -14\n2 3 22\n3 4 -4\n1 4 10\n").unwrap();
        assert!(energy_matches_raw(&doubled, &raw).is_err());
    }

    #[test]
    fn every_arm_has_exactly_the_frozen_budget_and_parameters() {
        for pair in PAIRS {
            for sweeps in BUDGETS {
                for seed in SEEDS {
                    assert!(validate_task(
                        &task(pair.control, sweeps, seed),
                        pair.control,
                        sweeps,
                        seed
                    )
                    .is_ok());
                    assert!(validate_task(
                        &task(pair.candidate, sweeps, seed),
                        pair.candidate,
                        sweeps,
                        seed
                    )
                    .is_ok());
                    let mut wrong = task(pair.control, sweeps, seed);
                    wrong.schedule.sweeps[0] += 1;
                    assert!(validate_task(&wrong, pair.control, sweeps, seed).is_err());
                }
            }
        }
    }

    fn outcome(score: f64, best_state: Vec<u8>) -> ExperimentOutcome {
        ExperimentOutcome {
            score,
            best_state,
            work: 0.0,
            backend: Backend::DenseByte,
        }
    }

    #[test]
    fn bad_outcomes_and_path_regressions_are_refused() {
        let ir = rudy_maxcut_ir("2 1\n1 2 1\n").unwrap();
        let control_state = vec![0, 1];
        let tied_state = vec![1, 0];
        let control = ir.energy(&control_state);
        assert_eq!(control, -1.0);
        let honest = [
            outcome(control, control_state.clone()),
            outcome(control, tied_state),
        ];
        assert!(validate_outcomes(PAIRS[0], &ir, &honest).is_ok());
        assert!(validate_outcomes(PAIRS[0], &ir, &honest[..1]).is_err());
        let non_finite = [
            outcome(control, control_state.clone()),
            outcome(f64::NAN, vec![0, 0]),
        ];
        assert!(validate_outcomes(PAIRS[0], &ir, &non_finite).is_err());
        let wrong_rescore = [
            outcome(control, control_state.clone()),
            outcome(-2.0, control_state.clone()),
        ];
        assert!(validate_outcomes(PAIRS[0], &ir, &wrong_rescore).is_err());
        let worse = [outcome(control, control_state), outcome(0.0, vec![0, 0])];
        assert!(validate_outcomes(PAIRS[0], &ir, &worse).is_err());
        assert!(validate_outcomes(PAIRS[1], &ir, &worse).is_ok());
    }

    #[test]
    fn duplicate_or_missing_rows_are_refused() {
        let instances: Vec<String> = (1..=EXPECTED_INSTANCES).map(|i| format!("G{i}")).collect();
        let mut seen = HashSet::new();
        for pair in PAIRS {
            for instance in &instances {
                for sweeps in BUDGETS {
                    for seed in SEEDS {
                        let key = RowKey {
                            pair: pair.name,
                            instance: instance.clone(),
                            sweeps,
                            seed,
                        };
                        assert!(record_key(&mut seen, &key).is_ok());
                    }
                }
            }
        }
        assert!(validate_complete(&seen, &instances).is_ok());
        let duplicate = RowKey {
            pair: PAIRS[0].name,
            instance: instances[0].clone(),
            sweeps: BUDGETS[0],
            seed: SEEDS[0],
        };
        assert!(record_key(&mut seen, &duplicate).is_err());
        seen.remove(&duplicate);
        assert!(validate_complete(&seen, &instances).is_err());
    }

    fn baseline_rows(synthesis_gain: f64, memory: impl Fn(usize) -> f64) -> Vec<Row> {
        let mut rows = Vec::new();
        for instance_index in 0..EXPECTED_INSTANCES {
            let instance = format!("G{}", instance_index + 1);
            for seed in SEEDS {
                for (pair, gain) in [
                    (PAIRS[0], synthesis_gain),
                    (PAIRS[1], memory(instance_index)),
                ] {
                    rows.push(Row {
                        key: RowKey {
                            pair: pair.name,
                            instance: instance.clone(),
                            sweeps: 50,
                            seed,
                        },
                        control: -100.0,
                        candidate: -100.0 - 100.0 * gain,
                        gain,
                    });
                }
            }
        }
        rows
    }

    #[test]
    fn both_fifty_sweep_controls_are_mandatory() {
        let balanced = |index: usize| {
            if index.is_multiple_of(2) {
                0.001
            } else {
                -0.001
            }
        };
        let controls = validate_baseline_controls(&baseline_rows(0.001, balanced)).unwrap();
        assert!((controls.memory_wilcoxon_p - 1.0).abs() < 1e-8);
        assert!(validate_baseline_controls(&baseline_rows(0.0, balanced)).is_err());
        assert!(validate_baseline_controls(&baseline_rows(0.01, balanced)).is_err());
        assert!(validate_baseline_controls(&baseline_rows(0.001, |_| 0.02)).is_err());
        assert!(
            validate_baseline_controls(&baseline_rows(0.001, |_| 0.001)).is_err(),
            "a statistically one-sided memory result does not reproduce RC-023"
        );
    }

    #[test]
    fn every_registered_operator_exists() {
        let registry = OperatorRegistry::standard();
        for pair in PAIRS {
            assert!(registry.contains(pair.control), "{}", pair.control);
            assert!(registry.contains(pair.candidate), "{}", pair.candidate);
        }
    }
}
