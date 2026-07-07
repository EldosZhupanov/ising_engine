//! Publication-grade benchmarking framework.
//!
//! Ties together instance acquisition ([`instances`]), solver adapters
//! ([`adapters`]), statistics ([`stats`]), and reporting ([`report`]) into a
//! single reproducible run. Every solver sees identical instances, seeds, and
//! computational budget; results carry bootstrap confidence intervals and
//! paired significance tests, and are emitted as CSV, JSON, LaTeX tables, and
//! self-contained SVG plots.

pub mod adapters;
pub mod instances;
pub mod report;
pub mod stats;

use adapters::{Budget, Solver};
use instances::Instance;

/// family → solver → instance → (sum native, count), for pairwise tests.
type FamilySolverInstance = std::collections::BTreeMap<
    String,
    std::collections::BTreeMap<String, std::collections::BTreeMap<String, (f64, usize)>>,
>;

/// Suite configuration. Defaults are bounded to finish quickly; scale up the
/// sizes/instances/budget for a full publication run.
pub struct SuiteConfig {
    pub seeds_per: usize,
    pub budget: Budget,
    /// Bootstrap resamples for confidence intervals.
    pub boot: usize,
    /// Two-sided confidence level for all CIs.
    pub confidence: f64,
    /// Relative tolerance for counting a run as "reached the reference".
    pub success_tol: f64,
    /// Output directory.
    pub out_dir: String,
}

impl Default for SuiteConfig {
    fn default() -> Self {
        Self {
            seeds_per: 20,
            budget: Budget::default(),
            boot: 2000,
            confidence: 0.95,
            success_tol: 1e-6,
            out_dir: "benchmarks".to_string(),
        }
    }
}

/// One solver run on one instance under one seed.
pub struct RunRecord {
    pub instance: String,
    pub family: String,
    pub n: usize,
    pub solver: String,
    pub seed: u64,
    pub energy: f64,
    pub native: f64,
    pub wall_ms: f64,
    pub reference: f64,
    pub gap: f64,
    pub success: bool,
}

/// Aggregate metrics for a (family, solver) cell.
pub struct Summary {
    pub family: String,
    pub solver: String,
    pub n_runs: usize,
    pub n_instances: usize,
    pub p_success: f64,
    pub tts: f64,
    pub tts_lo: f64,
    pub tts_hi: f64,
    pub mean_native: f64,
    pub best_native: f64,
    pub mean_gap: f64,
    pub gap_lo: f64,
    pub gap_hi: f64,
    pub mean_ms: f64,
    pub maximize: bool,
}

/// A pairwise significance test between two solvers within a family.
pub struct Significance {
    pub family: String,
    pub solver_a: String,
    pub solver_b: String,
    pub metric: String,
    pub wilcoxon_p: f64,
    pub t_p: f64,
    pub n_pairs: usize,
    /// True if `solver_a`'s mean native objective is better than `solver_b`'s.
    pub a_better: bool,
}

/// Whether `native` reaches `reference` within tolerance (direction-aware).
fn reached(maximize: bool, native: f64, reference: f64, tol: f64) -> bool {
    let slack = tol * reference.abs().max(1.0);
    if maximize {
        native >= reference - slack
    } else {
        native <= reference + slack
    }
}

/// Run the full suite over the given instances and available solvers, writing
/// every artifact under `config.out_dir`. Returns a short human summary.
pub fn run_suite(instances: &[Instance], solvers: &[Solver], config: &SuiteConfig) -> String {
    let available: Vec<Solver> = solvers.iter().copied().filter(|s| s.available()).collect();
    let skipped: Vec<&Solver> = solvers.iter().filter(|s| !s.available()).collect();

    let mut records: Vec<RunRecord> = Vec::new();

    for inst in instances {
        // Collect raw runs first so a best-found reference is available before
        // scoring (used only when no verified best_known is supplied).
        let mut raw: Vec<(usize, u64, f64, f64, f64)> = Vec::new(); // (solver_idx, seed, energy, native, ms)
        for (si, solver) in available.iter().enumerate() {
            for s in 0..config.seeds_per {
                let seed = mix_seed(&inst.name, s as u64);
                if let Some(res) = solver.run(&inst.model, &config.budget, seed) {
                    let energy = inst.model.calculate_total_energy(&res.state);
                    let native = inst.native_objective(energy);
                    raw.push((si, seed, energy, native, res.wall_ms));
                }
            }
        }
        // Reference: verified best-known if present, else best native found.
        let reference = inst.best_known.unwrap_or_else(|| {
            raw.iter().map(|r| r.3).fold(f64::NEG_INFINITY, |acc, v| {
                if inst.maximize {
                    acc.max(v)
                } else if acc == f64::NEG_INFINITY {
                    v
                } else {
                    acc.min(v)
                }
            })
        });
        for (si, seed, energy, native, ms) in raw {
            let gap = ((reference - native) / reference.abs().max(1.0))
                * if inst.maximize { 1.0 } else { -1.0 };
            records.push(RunRecord {
                instance: inst.name.clone(),
                family: inst.family.clone(),
                n: inst.n(),
                solver: available[si].label().to_string(),
                seed,
                energy,
                native,
                wall_ms: ms,
                reference,
                gap: gap.max(0.0),
                success: reached(inst.maximize, native, reference, config.success_tol),
            });
        }
    }

    let maximize_of: std::collections::HashMap<String, bool> = instances
        .iter()
        .map(|i| (i.family.clone(), i.maximize))
        .collect();

    let summaries = summarize(&records, config, &maximize_of);
    let significance = significance_tests(&records, "Ultimate", &maximize_of);

    report::write_all(config, &records, &summaries, &significance, &skipped);

    report::stdout_summary(&summaries, &significance, &skipped, instances.len())
}

/// Deterministic per-run seed from instance name + seed index (FNV-1a).
fn mix_seed(name: &str, s: u64) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h ^= s.wrapping_add(0x9e3779b97f4a7c15);
    h = h.wrapping_mul(0x100000001b3);
    h
}

fn summarize(
    records: &[RunRecord],
    config: &SuiteConfig,
    maximize_of: &std::collections::HashMap<String, bool>,
) -> Vec<Summary> {
    use std::collections::BTreeSet;
    let cells: BTreeSet<(String, String)> = records
        .iter()
        .map(|r| (r.family.clone(), r.solver.clone()))
        .collect();
    let mut out = Vec::new();
    for (family, solver) in cells {
        let cell: Vec<&RunRecord> = records
            .iter()
            .filter(|r| r.family == family && r.solver == solver)
            .collect();
        if cell.is_empty() {
            continue;
        }
        let successes: Vec<bool> = cell.iter().map(|r| r.success).collect();
        let gaps: Vec<f64> = cell.iter().map(|r| r.gap).collect();
        let p = stats::mean_bool(&successes);
        let mean_ms = mean(cell.iter().map(|r| r.wall_ms));
        let tts_val = crate::solver::tts::tts(p, mean_ms, 0.99);
        let (tts_lo, tts_hi) = crate::solver::tts::tts_ci(
            &successes,
            mean_ms,
            0.99,
            config.boot,
            config.confidence,
            7,
        );
        let (gap_lo, gap_hi) = stats::bootstrap_mean_ci(&gaps, config.boot, config.confidence, 9);
        let maximize = *maximize_of.get(&family).unwrap_or(&false);
        let natives = cell.iter().map(|r| r.native);
        let best_native = if maximize {
            natives.clone().fold(f64::NEG_INFINITY, f64::max)
        } else {
            natives.clone().fold(f64::INFINITY, f64::min)
        };
        let n_instances = cell
            .iter()
            .map(|r| r.instance.clone())
            .collect::<BTreeSet<_>>()
            .len();
        out.push(Summary {
            family,
            solver,
            n_runs: cell.len(),
            n_instances,
            p_success: p,
            tts: tts_val,
            tts_lo,
            tts_hi,
            mean_native: mean(cell.iter().map(|r| r.native)),
            best_native,
            mean_gap: mean(gaps.iter().copied()),
            gap_lo,
            gap_hi,
            mean_ms,
            maximize,
        });
    }
    out
}

/// Paired Wilcoxon + t-tests of `reference_solver` against every other solver,
/// per family, pairing by per-instance mean native objective.
fn significance_tests(
    records: &[RunRecord],
    reference_solver: &str,
    maximize_of: &std::collections::HashMap<String, bool>,
) -> Vec<Significance> {
    use std::collections::BTreeSet;
    // family -> solver -> instance -> (sum native, count)
    let mut agg: FamilySolverInstance = FamilySolverInstance::new();
    for r in records {
        let e = agg
            .entry(r.family.clone())
            .or_default()
            .entry(r.solver.clone())
            .or_default()
            .entry(r.instance.clone())
            .or_insert((0.0, 0));
        e.0 += r.native;
        e.1 += 1;
    }
    let mut out = Vec::new();
    for (family, solvers) in &agg {
        let Some(a_map) = solvers.get(reference_solver) else {
            continue;
        };
        let maximize = *maximize_of.get(family).unwrap_or(&false);
        for (solver, b_map) in solvers {
            if solver == reference_solver {
                continue;
            }
            let insts: BTreeSet<&String> =
                a_map.keys().filter(|k| b_map.contains_key(*k)).collect();
            if insts.len() < 2 {
                continue;
            }
            let a: Vec<f64> = insts.iter().map(|i| mean_pair(a_map[*i])).collect();
            let b: Vec<f64> = insts.iter().map(|i| mean_pair(b_map[*i])).collect();
            let w = stats::wilcoxon_signed_rank(&a, &b);
            let t = stats::paired_t_test(&a, &b);
            let mean_a = a.iter().sum::<f64>() / a.len() as f64;
            let mean_b = b.iter().sum::<f64>() / b.len() as f64;
            let a_better = if maximize {
                mean_a > mean_b
            } else {
                mean_a < mean_b
            };
            out.push(Significance {
                family: family.clone(),
                solver_a: reference_solver.to_string(),
                solver_b: solver.clone(),
                metric: "native_objective".to_string(),
                wilcoxon_p: w.p_value,
                t_p: t.p_value,
                n_pairs: insts.len(),
                a_better,
            });
        }
    }
    out
}

fn mean_pair(p: (f64, usize)) -> f64 {
    if p.1 == 0 {
        0.0
    } else {
        p.0 / p.1 as f64
    }
}

fn mean<I: Iterator<Item = f64>>(it: I) -> f64 {
    let mut sum = 0.0;
    let mut n = 0usize;
    for v in it {
        sum += v;
        n += 1;
    }
    if n == 0 {
        f64::NAN
    } else {
        sum / n as f64
    }
}
