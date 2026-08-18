//! `research_platform` — Stage 6: the fully autonomous research platform, from
//! one command. Runs continuous campaigns of the closed scientific cycle
//! (hypotheses → evolution → Runtime → append-only DB → meta-learner →
//! knowledge graph → reports → optional local/cloud LLM tiers → operator
//! proposals → predictor filter → dashboard), persisting everything under
//! `--dir` so it can be interrupted and resumed indefinitely.
//!
//!   cargo run --release --bin research_platform -- \
//!       --file benchmark_suite/data/gset/G11 --campaigns 2 --generations 4 \
//!       [--llm qwen2.5-coder:7b] [--cloud-model claude-sonnet-5]
//!
//! The core (Runtime, Scheduler, SpinState, backends, Experiment Runner,
//! canonical scorer, Operator API) is read-only; this binary only orchestrates.

use ising_engine::engine_v2::ai_scientist::{
    train_on_instances, write_evaluation, CampaignConfig, CampaignManager, LabConfig, MetaLearner,
    RuntimeExecutor,
};
use ising_engine::engine_v2::evolution::{Evolver, Schedule};
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::registry::OperatorRegistry;
use std::process::exit;

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1).cloned())
}
fn argn(name: &str, d: usize) -> usize {
    arg(name).and_then(|s| s.parse().ok()).unwrap_or(d)
}

fn main() {
    // --dataset (standalone): export the Foundation Dataset (ML-ready corpus)
    // and print the HONEST scale-gap manifest from a persisted platform dir, so
    // progress toward the 500k/1M/5M/20M foundation-scale milestones is
    // observable. Grow the corpus with a curiosity-driven service, then re-check:
    //   research_platform --file <...> --dir D --service --max-experiments 500000 \
    //       --curiosity 0.5 --planner --shared-knowledge
    //   research_platform --dir D --dataset      # progress + re-export
    if std::env::args().any(|a| a == "--dataset") {
        use ising_engine::engine_v2::ai_scientist::FoundationDataset;
        let dir = arg("--dir").unwrap_or_else(|| "experiments/platform".into());
        let report_every = argn("--report-every", 5000);
        let mgr = CampaignManager::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        let ds = std::path::Path::new(&dir).join("dataset");
        match FoundationDataset::export(&mgr.db, &ds) {
            Ok(rows) => {
                println!(
                    "Foundation Dataset exported: {rows} rows → {}",
                    ds.join("foundation_dataset.tsv").display()
                );
                println!("\n{}", FoundationDataset::manifest(&mgr.db));
            }
            Err(e) => {
                eprintln!("dataset export failed: {e}");
                exit(1);
            }
        }
        return;
    }

    let mut instances = Vec::new();
    // --benchmark {biqmac|orlib} loads REAL benchmark instances (energy-exact via
    // qubo_model_to_ir), a natural range of densities for the structural-law test.
    if let Some(bench) = arg("--benchmark") {
        use ising_engine::benchmark::instances::{parse_biqmac_sparse, parse_orlib_bqp};
        use ising_engine::engine_v2::frontend::qubo_model_to_ir;
        let n_take = argn("--count", 20);
        match bench.as_str() {
            "biqmac" => {
                let mut files: Vec<_> = std::fs::read_dir("benchmark_suite/data/biqmac")
                    .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
                    .unwrap_or_default();
                files.sort();
                let files: Vec<_> = files.into_iter().filter(|p| p.is_file()).collect();
                // Stride-sample across the (subfamily-clustered) file list to span
                // the full density range (planar pw → random w → dense be).
                let stride = (files.len() / n_take.max(1)).max(1);
                for path in files.iter().step_by(stride) {
                    let name = path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .unwrap_or("")
                        .to_string();
                    if let Ok(t) = std::fs::read_to_string(path) {
                        if let Ok(inst) = parse_biqmac_sparse(&t, &name) {
                            instances.push((inst.name.clone(), qubo_model_to_ir(&inst.model)));
                        }
                    }
                }
            }
            "orlib" => {
                for f in ["bqp50.txt", "bqp100.txt", "bqp250.txt", "bqp500.txt"] {
                    if let Ok(t) =
                        std::fs::read_to_string(format!("benchmark_suite/data/orlib/{f}"))
                    {
                        if let Ok(insts) = parse_orlib_bqp(&t, f) {
                            for inst in insts.into_iter().take(n_take) {
                                instances.push((inst.name.clone(), qubo_model_to_ir(&inst.model)));
                            }
                        }
                    }
                }
            }
            other => {
                eprintln!("unknown --benchmark '{other}' (use biqmac | orlib)");
                exit(2);
            }
        }
        println!("loaded {} real {bench} instances", instances.len());
    } else if let Some(fam) = arg("--family") {
        use ising_engine::engine_v2::families::{
            coloring_instance, max2sat_instance, mis_instance, partition_instance, tsp_instance,
        };
        let seed = arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(1);
        // --count N synthesizes N distinct instances (seeds seed..seed+N), so a
        // whole synthetic family can be studied/investigated in one command.
        let count = argn("--count", 1).max(1) as u64;
        match fam.as_str() {
            "tsp" => {
                let cities = argn("--cities", 8);
                println!(
                    "synthesizing {count} TSP instance(s): {cities} cities, seeds {seed}..{}",
                    seed + count - 1
                );
                for s in seed..seed + count {
                    instances.push(tsp_instance(cities, s));
                }
            }
            "max2sat" | "sat" => {
                let vars = argn("--vars", 40);
                let clauses = argn("--clauses", vars * 4);
                println!(
                    "synthesizing {count} MAX-2-SAT instance(s): {vars} vars, {clauses} clauses, seeds {seed}..{}",
                    seed + count - 1
                );
                for s in seed..seed + count {
                    instances.push(max2sat_instance(vars, clauses, s));
                }
            }
            "coloring" | "color" => {
                let verts = argn("--vertices", 20);
                let deg = argn("--avg-deg", 4);
                let k = argn("--colors", 4);
                println!(
                    "synthesizing {count} coloring instance(s): {verts} vertices, k={k}, avg-deg {deg}, seeds {seed}..{}",
                    seed + count - 1
                );
                for s in seed..seed + count {
                    instances.push(coloring_instance(verts, deg, k, s));
                }
            }
            "npart" | "partition" => {
                let nums = argn("--nums", 40);
                println!(
                    "synthesizing {count} number-partition instance(s): {nums} numbers, seeds {seed}..{}",
                    seed + count - 1
                );
                for s in seed..seed + count {
                    instances.push(partition_instance(nums, s));
                }
            }
            "mis" => {
                let verts = argn("--vertices", 40);
                let deg = argn("--avg-deg", 6);
                println!(
                    "synthesizing {count} max-independent-set instance(s): {verts} vertices, avg-deg {deg}, seeds {seed}..{}",
                    seed + count - 1
                );
                for s in seed..seed + count {
                    instances.push(mis_instance(verts, deg, s));
                }
            }
            other => {
                eprintln!("unknown --family '{other}' (use tsp | max2sat)");
                exit(2);
            }
        }
    } else {
        // One or more instances: --file accepts a comma-separated list, so a whole
        // benchmark family runs from one command (e.g. --file "$(ls d/G* | paste -sd,)").
        let files_arg = arg("--file").unwrap_or_else(|| {
            eprintln!(
                "usage: research_platform (--file <rudy>[,...] | --family tsp|max2sat) [--dir D]\n\
       [--campaigns N] [--generations N] [--rounds N] [--hypotheses N] [--batch N] [--seeds N]\n\
       [--replicas N] [--report-every N] [--cloud-every N] [--llm MODEL] [--cloud-model MODEL]\n\
       [--seed N] [--cities N] [--vars N] [--clauses N] [--early-stop]"
            );
            exit(2);
        });
        for file in files_arg.split(',').filter(|f| !f.trim().is_empty()) {
            let file = file.trim();
            let text = std::fs::read_to_string(file).unwrap_or_else(|e| {
                eprintln!("cannot read {file}: {e}");
                exit(1);
            });
            let ir = rudy_maxcut_ir(&text).unwrap_or_else(|e| {
                eprintln!("parse error in {file}: {e}");
                exit(1);
            });
            let id = std::path::Path::new(file)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("unnamed")
                .to_string();
            instances.push((id, ir));
        }
    }

    // --structural prints structural landscape signals for the loaded instances,
    // used to test whether RUGGEDNESS predicts which quench/barrier operator is
    // causal across problem classes. ruggedness = fraction of DISTINCT local
    // minima reached by `samples` random-start greedy descents (smooth landscape
    // → few distinct traps → low; rugged → many → high).
    if std::env::args().any(|a| a == "--structural") {
        use ising_engine::engine_v2::ir::ProblemIR;
        use rand::{Rng, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        // One greedy-descent pass from a random start (shared by several probes).
        fn descend(ir: &ProblemIR, x: &mut [u8]) {
            let n = ir.n;
            loop {
                let mut improved = false;
                for i in 0..n {
                    let (a, b) = (ir.row_ptr[i] as usize, ir.row_ptr[i + 1] as usize);
                    let mut h = ir.linear[i];
                    for k in a..b {
                        h += ir.weights[k] * x[ir.col_idx[k] as usize] as f64;
                    }
                    // ΔE of flipping spin i = (1 - 2·x_i)·field_i.
                    if (1.0 - 2.0 * x[i] as f64) * h < -1e-9 {
                        x[i] ^= 1;
                        improved = true;
                    }
                }
                if !improved {
                    break;
                }
            }
        }
        // Sample `samples` random-start descents; return the multiset of local-min
        // energies (µ-scaled ints). ruggedness / minima-entropy / funnel all derive
        // from this single pass so they describe the SAME sampled landscape.
        fn descend_minima(ir: &ProblemIR, samples: usize) -> Vec<i64> {
            let mut rng = ChaCha8Rng::seed_from_u64(12345);
            let n = ir.n;
            let mut mins = Vec::with_capacity(samples);
            for _ in 0..samples {
                let mut x: Vec<u8> = (0..n).map(|_| rng.gen::<bool>() as u8).collect();
                descend(ir, &mut x);
                mins.push((ir.energy(&x) * 1e6).round() as i64);
            }
            mins
        }
        // Random-walk (accept-all single flips) energy autocorrelation length τ, the
        // lag at which the normalized autocorrelation first drops below 1/e; returned
        // normalized by n. Energy is tracked incrementally (O(deg) per step).
        fn autocorr_len(ir: &ProblemIR, steps: usize) -> f64 {
            let n = ir.n;
            if n == 0 {
                return 0.0;
            }
            let mut rng = ChaCha8Rng::seed_from_u64(999);
            let mut x: Vec<u8> = (0..n).map(|_| rng.gen::<bool>() as u8).collect();
            let mut e = ir.energy(&x);
            let mut series = Vec::with_capacity(steps);
            for _ in 0..steps {
                let i = (rng.gen::<u32>() as usize) % n;
                let (a, b) = (ir.row_ptr[i] as usize, ir.row_ptr[i + 1] as usize);
                let mut h = ir.linear[i];
                for k in a..b {
                    h += ir.weights[k] * x[ir.col_idx[k] as usize] as f64;
                }
                e += (1.0 - 2.0 * x[i] as f64) * h; // ΔE
                x[i] ^= 1;
                series.push(e);
            }
            let m = series.iter().sum::<f64>() / series.len() as f64;
            let var = series.iter().map(|z| (z - m) * (z - m)).sum::<f64>() / series.len() as f64;
            if var < 1e-12 {
                return 0.0;
            }
            let max_lag = (steps / 2).min(4 * n);
            let mut tau = max_lag as f64;
            for lag in 1..max_lag {
                let mut c = 0.0;
                for t in 0..series.len() - lag {
                    c += (series[t] - m) * (series[t + lag] - m);
                }
                c /= (series.len() - lag) as f64 * var;
                if c < std::f64::consts::E.recip() {
                    tau = lag as f64;
                    break;
                }
            }
            tau / n as f64
        }
        // Frustration index: fraction of coupled edges left UNSATISFIED at a greedy
        // local minimum. In Ising form (s = 2x−1, J = q/4) an edge raises energy when
        // J·s_i·s_j > 0 — a convention-free frustration signal.
        fn frustration(ir: &ProblemIR) -> f64 {
            let n = ir.n;
            let mut rng = ChaCha8Rng::seed_from_u64(7);
            let mut x: Vec<u8> = (0..n).map(|_| rng.gen::<bool>() as u8).collect();
            descend(ir, &mut x);
            let (mut tot, mut fr) = (0usize, 0usize);
            for i in 0..n {
                let (a, b) = (ir.row_ptr[i] as usize, ir.row_ptr[i + 1] as usize);
                for k in a..b {
                    let j = ir.col_idx[k] as usize;
                    if j <= i {
                        continue; // each edge once
                    }
                    let si = 2.0 * x[i] as f64 - 1.0;
                    let sj = 2.0 * x[j] as f64 - 1.0;
                    tot += 1;
                    if (ir.weights[k] / 4.0) * si * sj > 1e-12 {
                        fr += 1;
                    }
                }
            }
            if tot > 0 {
                fr as f64 / tot as f64
            } else {
                0.0
            }
        }
        // Normalized spectral gap (λ1−λ2)/λ1 of the |coupling| matrix, via power
        // iteration + deflation (200 iters). Cheap CSR mat-vec; a graph-mixing proxy.
        fn spectral_gap(ir: &ProblemIR) -> f64 {
            let n = ir.n;
            if n < 2 {
                return 0.0;
            }
            let matvec = |v: &[f64], out: &mut [f64]| {
                for (i, oi) in out.iter_mut().enumerate() {
                    let (a, b) = (ir.row_ptr[i] as usize, ir.row_ptr[i + 1] as usize);
                    let mut s = 0.0;
                    for k in a..b {
                        s += ir.weights[k].abs() * v[ir.col_idx[k] as usize];
                    }
                    *oi = s;
                }
            };
            let norm = |v: &[f64]| v.iter().map(|z| z * z).sum::<f64>().sqrt();
            let mut v = vec![1.0 / (n as f64).sqrt(); n];
            let mut w = vec![0.0; n];
            let mut l1 = 0.0;
            for _ in 0..200 {
                matvec(&v, &mut w);
                let nn = norm(&w);
                if nn < 1e-12 {
                    break;
                }
                l1 = nn;
                for z in w.iter_mut() {
                    *z /= nn;
                }
                v.copy_from_slice(&w);
            }
            let v1 = v.clone();
            let mut u: Vec<f64> = (0..n)
                .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
                .collect();
            let nu = norm(&u);
            for z in u.iter_mut() {
                *z /= nu;
            }
            let mut l2 = 0.0;
            for _ in 0..200 {
                matvec(&u, &mut w);
                let dot: f64 = w.iter().zip(&v1).map(|(a, b)| a * b).sum();
                for i in 0..n {
                    w[i] -= dot * v1[i]; // deflate the Perron component
                }
                let nn = norm(&w);
                if nn < 1e-12 {
                    break;
                }
                l2 = nn;
                for z in w.iter_mut() {
                    *z /= nn;
                }
                u.copy_from_slice(&w);
            }
            if l1 > 1e-12 {
                (l1 - l2) / l1
            } else {
                0.0
            }
        }
        // Multidimensional landscape descriptor: structural (density/degree/weight) +
        // landscape (ruggedness/entropy/funnel/autocorr/frustration/spectral gap).
        // The single-scalar hunt (density/weight_cv/lin_coup) is dead; this is the
        // vector-descriptor test — does the WHOLE vector predict operator behavior?
        println!("instance\tn\tdensity\tmean_deg\tdeg_cv\tweight_cv\tlin_coup\truggedness\tlmin_entropy\tfunnel\tautocorr\tfrustration\tspectral_gap");
        for (id, ir) in &instances {
            let n = ir.n;
            let degs: Vec<f64> = (0..n)
                .map(|i| (ir.row_ptr[i + 1] - ir.row_ptr[i]) as f64)
                .collect();
            let mean_deg = degs.iter().sum::<f64>() / n.max(1) as f64;
            let deg_cv = if mean_deg > 1e-9 {
                (degs.iter().map(|d| (d - mean_deg).powi(2)).sum::<f64>() / n.max(1) as f64).sqrt()
                    / mean_deg
            } else {
                0.0
            };
            let density = if n > 1 {
                mean_deg / (n - 1) as f64
            } else {
                0.0
            };
            let w: Vec<f64> = ir.weights.iter().map(|x| x.abs()).collect();
            let wm = w.iter().sum::<f64>() / w.len().max(1) as f64;
            let weight_cv = if wm > 1e-9 {
                (w.iter().map(|x| (x - wm).powi(2)).sum::<f64>() / w.len().max(1) as f64).sqrt()
                    / wm
            } else {
                0.0
            };
            let lm = ir.linear.iter().map(|x| x.abs()).sum::<f64>() / n.max(1) as f64;
            let lin_coup = if wm > 1e-9 { lm / wm } else { 0.0 };
            // Landscape probes from the sampled-minima pass.
            let samples = 64usize;
            let mins = descend_minima(ir, samples);
            let mut sorted = mins.clone();
            sorted.sort_unstable();
            let best = *sorted.first().unwrap_or(&0);
            let funnel = mins.iter().filter(|&&m| m == best).count() as f64 / samples as f64;
            sorted.dedup();
            let ruggedness = sorted.len() as f64 / samples as f64;
            // Shannon entropy over the empirical local-min energy distribution, in
            // bits, normalized by log2(samples) → [0,1].
            let mut counts: std::collections::HashMap<i64, usize> =
                std::collections::HashMap::new();
            for &m in &mins {
                *counts.entry(m).or_insert(0) += 1;
            }
            let entropy: f64 = counts
                .values()
                .map(|&c| {
                    let p = c as f64 / samples as f64;
                    -p * p.log2()
                })
                .sum();
            let lmin_entropy = entropy / (samples as f64).log2().max(1e-9);
            println!(
                "{id}\t{n}\t{density:.4}\t{mean_deg:.2}\t{deg_cv:.3}\t{weight_cv:.3}\t{lin_coup:.3}\t{ruggedness:.3}\t{lmin_entropy:.3}\t{funnel:.3}\t{:.3}\t{:.3}\t{:.3}",
                autocorr_len(ir, 8 * n.max(1)),
                frustration(ir),
                spectral_gap(ir)
            );
        }
        return;
    }

    // --op-benchmark: ABSOLUTE operator-quality metric (fixes the marginal-quench
    // confound). For each instance, run every operator SOLO (fixed budget) and
    // normalize the mean best energy per instance to [0,1] (0 = best operator on
    // that instance, 1 = worst). No quench baseline ⇒ no shifting reference.
    // Prints a tab-separated instance × operator matrix for cross-family analysis.
    if std::env::args().any(|a| a == "--op-benchmark") {
        use ising_engine::engine_v2::ai_scientist::{
            BatchExecutor, ExperimentTask, RuntimeExecutor,
        };
        use ising_engine::engine_v2::evolution::Schedule;
        use ising_engine::engine_v2::registry::OperatorRegistry;
        // A HUMAN DIAGNOSTIC (print-only): the absolute operator-quality matrix.
        // It is deliberately NOT a knowledge source — the autonomous loop
        // (--orchestrate / --service) is the sole writer of canonical knowledge,
        // so this hand tool never forks a parallel store.
        let reg = OperatorRegistry::standard();
        let exec = RuntimeExecutor::auto();
        let ops: Vec<String> = reg.names().map(|s| s.to_string()).collect();
        let sweeps = argn("--sweeps", 50) as u32;
        let replicas = argn("--replicas", 32);
        let seeds = [1u64, 2, 3];
        // --warm evaluates each operator in a WARM context [metropolis_sweep, op].
        let warm = std::env::args().any(|a| a == "--warm");
        print!("instance");
        for op in &ops {
            print!("\t{op}");
        }
        println!();
        for (id, ir) in &instances {
            let mut means = Vec::with_capacity(ops.len());
            for op in &ops {
                let (sched_ops, sched_sw) = if warm {
                    (
                        vec!["metropolis_sweep".to_string(), op.clone()],
                        vec![sweeps, sweeps],
                    )
                } else {
                    (vec![op.clone()], vec![sweeps])
                };
                let tasks: Vec<ExperimentTask> = seeds
                    .iter()
                    .map(|&s| ExperimentTask {
                        schedule: Schedule {
                            ops: sched_ops.clone(),
                            sweeps: sched_sw.clone(),
                            temp_hi: 4.0,
                            temp_lo: 0.1,
                        },
                        num_replicas: replicas,
                        seed: s,
                    })
                    .collect();
                let outs = exec.run_batch(ir, &reg, &tasks);
                means.push(outs.iter().map(|o| o.score).sum::<f64>() / outs.len().max(1) as f64);
            }
            let mn = means.iter().cloned().fold(f64::INFINITY, f64::min);
            let mx = means.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            print!("{id}");
            for &e in &means {
                // 0 = best (lowest energy) operator on this instance, 1 = worst.
                let norm = if mx > mn { (e - mn) / (mx - mn) } else { 0.0 };
                print!("\t{norm:.3}");
            }
            println!();
        }
        return;
    }

    // --pt-ab: does a real PARALLEL-TEMPERING-style schedule (many
    // metropolis+replica_exchange rounds over the temperature ladder) beat PURE
    // metropolis of EQUAL metropolis budget? The decisive falsification of the
    // universal "thermal is best" law — the exchange mechanism is PT's whole point.
    if std::env::args().any(|a| a == "--pt-ab") {
        use ising_engine::engine_v2::ai_scientist::{
            BatchExecutor, ExperimentTask, RuntimeExecutor,
        };
        use ising_engine::engine_v2::evolution::Schedule;
        use ising_engine::engine_v2::registry::OperatorRegistry;
        let reg = OperatorRegistry::standard();
        let exec = RuntimeExecutor::auto();
        let replicas = argn("--replicas", 32);
        let rounds = argn("--rounds", 20);
        let per = argn("--per", 2) as u32; // metropolis sweeps per round
        let seeds = [1u64, 2, 3, 4];
        // pure: metropolis. --fair gives it PT's TOTAL sweep count (metro+exchange)
        // as metropolis sweeps, so pure gets MORE thermal work than PT — killing the
        // "PT just does extra sweeps" confound.
        let total = rounds as u32 * per;
        let pure_sweeps = if std::env::args().any(|a| a == "--fair") {
            total + rounds as u32
        } else {
            total
        };
        let pure = Schedule {
            ops: vec!["metropolis_sweep".into()],
            sweeps: vec![pure_sweeps],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        // PT: [metropolis(per), replica_exchange(1)] × rounds.
        let mut pt_ops = Vec::new();
        let mut pt_sw = Vec::new();
        for _ in 0..rounds {
            pt_ops.push("metropolis_sweep".to_string());
            pt_sw.push(per);
            pt_ops.push("replica_exchange".to_string());
            pt_sw.push(1);
        }
        let pt = Schedule {
            ops: pt_ops,
            sweeps: pt_sw,
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        println!("instance\tpure_metro\tPT_metro+exchange\tPT_better?\trel_gain");
        let (mut pt_wins, mut n) = (0usize, 0usize);
        for (id, ir) in &instances {
            let run = |sc: &Schedule| -> f64 {
                let tasks: Vec<ExperimentTask> = seeds
                    .iter()
                    .map(|&s| ExperimentTask {
                        schedule: sc.clone(),
                        num_replicas: replicas,
                        seed: s,
                    })
                    .collect();
                let outs = exec.run_batch(ir, &reg, &tasks);
                outs.iter().map(|o| o.score).sum::<f64>() / outs.len().max(1) as f64
            };
            let (ep, ept) = (run(&pure), run(&pt));
            let better = ept < ep - 1e-9;
            if better {
                pt_wins += 1;
            }
            n += 1;
            let rel = if ep.abs() > 1e-9 {
                (ep - ept) / ep.abs()
            } else {
                0.0
            };
            println!(
                "{id}\t{ep:.1}\t{ept:.1}\t{}\t{rel:+.4}",
                if better { "YES" } else { "no" }
            );
        }
        eprintln!("\nPT (metro+exchange) beat pure metropolis on {pt_wins}/{n} instances (equal metropolis budget)");
        return;
    }

    // --adaptive-oracle: is the best operator STATE-dependent? Tests the thesis
    // "Search State → Best Next Operator" against "Problem → Best Operator". Uses
    // deterministic replay (ADR-0004): to score prefix+[op] we re-run the whole
    // schedule; identical prefix ⇒ identical intermediate state across candidates,
    // so a greedy oracle schedule is buildable purely through the executor. The
    // oracle PEEKS at true energies — it measures whether the INFORMATION exists
    // (the adaptivity ceiling), not a deployable policy. Greedy ⇒ a LOWER bound on
    // the value of adaptivity (myopic). Equal budget: both use phases*block sweeps.
    if std::env::args().any(|a| a == "--adaptive-oracle") {
        use ising_engine::engine_v2::ai_scientist::{
            BatchExecutor, ExperimentTask, RuntimeExecutor,
        };
        use ising_engine::engine_v2::evolution::Schedule;
        use ising_engine::engine_v2::registry::OperatorRegistry;
        let reg = OperatorRegistry::standard();
        let exec = RuntimeExecutor::auto();
        let replicas = argn("--replicas", 24);
        let phases = argn("--phases", 5);
        let block = argn("--block", 20) as u32;
        // --temp-hi/--temp-lo override the ladder; set equal for a FIXED-temperature
        // ensemble (no in-operator annealing) — the confound check for whether the
        // always-on ladder is masking schedule-level operator state-dependence.
        let temp_hi = arg("--temp-hi").and_then(|s| s.parse().ok()).unwrap_or(4.0);
        let temp_lo = arg("--temp-lo").and_then(|s| s.parse().ok()).unwrap_or(0.1);
        let seeds = [1u64, 2, 3];
        let ops: Vec<String> = reg.names().map(|s| s.to_string()).collect();
        let run =
            |ir: &ising_engine::engine_v2::ir::ProblemIR, ops_: &[String], sw: &[u32]| -> f64 {
                let tasks: Vec<ExperimentTask> = seeds
                    .iter()
                    .map(|&s| ExperimentTask {
                        schedule: Schedule {
                            ops: ops_.to_vec(),
                            sweeps: sw.to_vec(),
                            temp_hi,
                            temp_lo,
                        },
                        num_replicas: replicas,
                        seed: s,
                    })
                    .collect();
                let outs = exec.run_batch(ir, &reg, &tasks);
                outs.iter().map(|o| o.score).sum::<f64>() / outs.len().max(1) as f64
            };
        println!("instance\tstatic_op\tstatic_e\tadaptive_e\trel_gain\tsequence");
        let mut phase_pick: Vec<std::collections::HashMap<String, usize>> =
            vec![Default::default(); phases];
        let (mut wins, mut n, mut sum_gain) = (0usize, 0usize, 0.0f64);
        for (id, ir) in &instances {
            // Best PROBLEM→OPERATOR: single op repeated over all phases (equal budget).
            let mut best_static = f64::INFINITY;
            let mut best_static_op = ops[0].clone();
            for op in &ops {
                let e = run(ir, &vec![op.clone(); phases], &vec![block; phases]);
                if e < best_static {
                    best_static = e;
                    best_static_op = op.clone();
                }
            }
            // Greedy STATE→NEXT-OPERATOR oracle: at each phase pick the op that most
            // lowers energy given the committed prefix.
            let mut pre_ops: Vec<String> = Vec::new();
            let mut pre_sw: Vec<u32> = Vec::new();
            let mut adaptive_e = f64::INFINITY;
            for pick in phase_pick.iter_mut().take(phases) {
                let (mut best_e, mut best_op) = (f64::INFINITY, ops[0].clone());
                for op in &ops {
                    let mut t_ops = pre_ops.clone();
                    t_ops.push(op.clone());
                    let mut t_sw = pre_sw.clone();
                    t_sw.push(block);
                    let e = run(ir, &t_ops, &t_sw);
                    if e < best_e {
                        best_e = e;
                        best_op = op.clone();
                    }
                }
                pre_ops.push(best_op.clone());
                pre_sw.push(block);
                adaptive_e = best_e;
                *pick.entry(best_op).or_insert(0) += 1;
            }
            let rel = if best_static.abs() > 1e-9 {
                (best_static - adaptive_e) / best_static.abs()
            } else {
                0.0
            };
            if adaptive_e < best_static - 1e-9 {
                wins += 1;
            }
            n += 1;
            sum_gain += rel;
            println!(
                "{id}\t{best_static_op}\t{best_static:.1}\t{adaptive_e:.1}\t{rel:+.4}\t{}",
                pre_ops.join(">")
            );
        }
        eprintln!(
            "\nadaptive-oracle beat best-static on {wins}/{n}; mean rel_gain = {:+.4}",
            sum_gain / n.max(1) as f64
        );
        eprintln!("phase-wise best-next-operator selection frequency (state-dependence signal):");
        for (ph, m) in phase_pick.iter().enumerate() {
            let mut items: Vec<_> = m.iter().collect();
            items.sort_by(|a, b| b.1.cmp(a.1));
            let s: Vec<String> = items.iter().map(|(k, v)| format!("{k}:{v}")).collect();
            eprintln!("  phase {ph}: {}", s.join("  "));
        }
        return;
    }

    let dir = arg("--dir").unwrap_or_else(|| "experiments/platform".into());
    let campaigns = argn("--campaigns", 1);
    let report_every = argn("--report-every", 5000);

    let base_cfg = CampaignConfig {
        instance_id: String::new(), // set per instance below
        generations: argn("--generations", 4),
        lab: LabConfig {
            rounds: argn("--rounds", 3),
            hypotheses_per_round: argn("--hypotheses", 12),
            batch_size: argn("--batch", 60),
            seeds_per_hypothesis: argn("--seeds", 4),
            num_replicas: argn("--replicas", 32),
            max_ops: argn("--max-ops", 4),
            ..Default::default()
        },
        report_every,
        cloud_every: argn("--cloud-every", 10_000),
        llm_model: arg("--llm"),
        cloud_model: arg("--cloud-model"),
        predictor_oversample: argn("--oversample", 3),
        // --shared-knowledge turns on the Meta-Learning Layer (Policy/World/
        // Dynamics consensus biasing ideation + published to the graph).
        shared_knowledge: std::env::args().any(|a| a == "--shared-knowledge"),
        // --curiosity <λ> turns on the Curiosity Engine (explore/exploit dial).
        curiosity_lambda: arg("--curiosity")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0),
        base_seed: arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(1),
    };

    let reg = OperatorRegistry::standard();
    let evolver = Evolver::new(Default::default());
    let mut executor = RuntimeExecutor::auto();
    // --early-stop (opt-in): attach a Dynamics early-stop controller to EVERY
    // run, so plateaued tasks stop instead of burning their full budget. We
    // bootstrap a Dynamics model from the first instance; if there is too little
    // trajectory data to fit one, we skip honestly rather than pretend.
    if std::env::args().any(|a| a == "--early-stop") {
        if let Some((_, ir0)) = instances.first() {
            let boot = Schedule {
                ops: vec!["metropolis_sweep".into(), "greedy_descent".into()],
                sweeps: vec![24, 24],
                temp_hi: 4.0,
                temp_lo: 0.1,
            };
            match train_on_instances(
                &[ir0],
                &reg,
                &boot,
                base_cfg.lab.num_replicas.max(2),
                &[1, 2, 3],
                1e-4,
            ) {
                Some(model) => {
                    executor = executor.with_early_stop_default(model);
                    println!("early-stop: Dynamics controller active on every run (opt-in)");
                }
                None => println!(
                    "early-stop requested but too little trajectory data to fit a Dynamics model — skipping honestly"
                ),
            }
        }
    }

    // --render-dashboard just (re)generates the dashboard from the persisted
    // stores — no experiments run — and prints its path. Use it to view the
    // accumulated platform state at any time.
    if std::env::args().any(|a| a == "--render-dashboard") {
        use ising_engine::engine_v2::ai_scientist::{
            write_dashboard_with, ResearchExecutive, ResourceState,
        };
        let mgr = CampaignManager::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        // The Chief Scientist surveys the accumulated state for the hero panel.
        let resources = ResourceState {
            cloud_available: std::env::var("ANTHROPIC_API_KEY").is_ok(),
            local_available: true,
            ..Default::default()
        };
        let brief = ResearchExecutive::new(Default::default())
            .assess(&mgr.db, &mgr.graph, &resources, None);
        match write_dashboard_with(&dir, &mgr.db, &mgr.graph, &mgr.archive, &[], Some(&brief)) {
            Ok(p) => {
                let abs = std::fs::canonicalize(&p).unwrap_or(p);
                println!(
                    "dashboard rendered from {} experiments, {} knowledge facts",
                    mgr.db.len(),
                    mgr.graph.len()
                );
                println!("open: {}", abs.display());
                println!("      file://{}", abs.display());
            }
            Err(e) => {
                eprintln!("failed to render dashboard: {e}");
                exit(1);
            }
        }
        return;
    }

    // --orchestrate <ticks> runs the Stage-8 Research Orchestrator: the full
    // life cycle (observe→analyze→learn→plan→run→evaluate→update→write-knowledge)
    // with the Theory Engine, Scientific Memory, and Foundation Dataset export.
    if let Some(ticks) = arg("--orchestrate").and_then(|s| s.parse::<usize>().ok()) {
        use ising_engine::engine_v2::ai_scientist::{OrchestratorConfig, ResearchOrchestrator};
        let mut orch = ResearchOrchestrator::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        let ocfg = OrchestratorConfig {
            campaign: base_cfg.clone(),
            // --planner lets the loop choose its own next instance by expected
            // new knowledge (autonomous task-setting) instead of round-robin.
            planner_driven: std::env::args().any(|a| a == "--planner"),
            // --executive makes the loop OBEY the Chief Scientist: it trains the
            // models the executive says are stale, investigates the theories it
            // flags, and ROUTES idea generation to the local/cloud LLM.
            executive_driven: std::env::args().any(|a| a == "--executive"),
            // Local tier the executive may route to: --llm, else auto-detect a
            // running Ollama server. Cloud tier: --cloud-model (honest skip
            // without an API key).
            local_llm: base_cfg
                .llm_model
                .clone()
                .or_else(ising_engine::engine_v2::ai_scientist::detect_local_llm),
            cloud_llm: base_cfg.cloud_model.clone(),
            ..Default::default()
        };
        if let Some(m) = &ocfg.local_llm {
            println!("local LLM available for the executive to route to: {m}");
        }
        println!(
            "Research Orchestrator: {ticks} lifecycle ticks over {} instance(s)\n",
            instances.len()
        );
        let reports = orch
            .run(&instances, &reg, &evolver, &executor, &ocfg, ticks)
            .unwrap_or_else(|e| {
                eprintln!("orchestrator failed: {e}");
                exit(1);
            });
        for r in &reports {
            println!("── tick {} [{}] ──", r.tick, r.instance);
            println!("  observe : {}", r.recall);
            for d in r.directives.iter().take(5) {
                println!("  chief   : {d}");
            }
            println!(
                "  run     : experiments {} → {}, best {:.1}",
                r.campaign.experiments_before, r.campaign.experiments_after, r.campaign.best_score
            );
            for t in &r.theories {
                println!("  theory  : {t}");
            }
            println!(
                "  write   : {} theories published, {} dataset rows, {}",
                r.theories_published, r.dataset_rows, r.memory
            );
        }
        println!(
            "\nplatform state: {} experiments, {} knowledge facts",
            orch.manager().db.len(),
            orch.manager().graph.len()
        );
        return;
    }

    // --service runs the orchestrator as a persistent, budget-capped loop (the
    // planner picks its own tasks). Caps: --max-ticks / --max-experiments /
    // --max-wall-secs (0 = uncapped on that axis). Interruptible & resumable —
    // everything persists after every tick.
    if std::env::args().any(|a| a == "--service") {
        use ising_engine::engine_v2::ai_scientist::{
            OrchestratorConfig, ResearchOrchestrator, ServiceBudget,
        };
        let mut orch = ResearchOrchestrator::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        let ocfg = OrchestratorConfig {
            campaign: base_cfg.clone(),
            // A service is autonomous by default: the planner sets its own tasks
            // unless --no-planner forces round-robin.
            planner_driven: !std::env::args().any(|a| a == "--no-planner"),
            executive_driven: std::env::args().any(|a| a == "--executive"),
            local_llm: base_cfg
                .llm_model
                .clone()
                .or_else(ising_engine::engine_v2::ai_scientist::detect_local_llm),
            cloud_llm: base_cfg.cloud_model.clone(),
            ..Default::default()
        };
        let budget = ServiceBudget {
            max_ticks: argn("--max-ticks", 0),
            max_experiments: argn("--max-experiments", 0),
            max_wall_secs: arg("--max-wall-secs")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        };
        if budget.max_ticks == 0 && budget.max_experiments == 0 && budget.max_wall_secs == 0 {
            eprintln!(
                "--service needs at least one cap: --max-ticks N / --max-experiments N / --max-wall-secs N"
            );
            exit(2);
        }
        println!(
            "Research service: budget ticks={} experiments={} wall_secs={} over {} instance(s)\n",
            budget.max_ticks,
            budget.max_experiments,
            budget.max_wall_secs,
            instances.len()
        );
        let reports = orch
            .run_service(&instances, &reg, &evolver, &executor, &ocfg, budget)
            .unwrap_or_else(|e| {
                eprintln!("service failed: {e}");
                exit(1);
            });
        println!(
            "service ran {} ticks; platform state: {} experiments, {} knowledge facts",
            reports.len(),
            orch.manager().db.len(),
            orch.manager().graph.len()
        );
        return;
    }

    // --investigate <operator> runs a MULTI-INSTANCE Theory-Engine ablation of
    // one operator across every loaded instance, aggregating the trials into a
    // single theory (Popperian confidence grows with cross-instance survival),
    // and publishes it (supported OR refuted — refutations are kept).
    if let Some(op) = arg("--investigate") {
        use ising_engine::engine_v2::ai_scientist::{
            ResearchOrchestrator, TheoryConfig, TheoryStatus,
        };
        let mut orch = ResearchOrchestrator::open(&dir, report_every).unwrap_or_else(|e| {
            eprintln!("cannot open platform dir {dir}: {e}");
            exit(1);
        });
        let nseeds = argn("--inv-seeds", 3).max(1) as u64;
        let seeds: Vec<u64> = (0..nseeds)
            .map(|i| base_cfg.base_seed.wrapping_add(i + 1))
            .collect();
        println!(
            "Investigating '{op}' across {} instance(s), {} seeds each…",
            instances.len(),
            seeds.len()
        );
        match orch.investigate_operator(&instances, &reg, &op, TheoryConfig::default(), &seeds) {
            Some(theory) => {
                println!("  status     : {:?}", theory.status);
                println!(
                    "  trials     : {} (survived {})",
                    theory.trials, theory.survived
                );
                println!("  confidence : {:.2}", theory.confidence);
                println!("  explanation: {}", theory.explanation);
                match theory.status {
                    TheoryStatus::Supported => {
                        println!("  → published: the mechanism survived ablation across instances")
                    }
                    _ => println!("  → recorded as a refutation (kept, not discarded)"),
                }
            }
            None => println!("  no theory produced (no instances?)"),
        }
        println!(
            "platform state: {} knowledge facts",
            orch.manager().graph.len()
        );
        return;
    }

    let mut mgr = CampaignManager::open(&dir, report_every).unwrap_or_else(|e| {
        eprintln!("cannot open platform dir {dir}: {e}");
        exit(1);
    });
    println!(
        "Research platform in {dir}: resuming with {} experiments, {} knowledge facts, {} reports",
        mgr.db.len(),
        mgr.graph.len(),
        mgr.archive.reports().len()
    );
    println!(
        "local scientist = {}, cloud scientist = {}, instances = {}\n",
        base_cfg.llm_model.as_deref().unwrap_or("heuristic"),
        base_cfg.cloud_model.as_deref().unwrap_or("disabled"),
        instances.len()
    );

    for (id, ir) in &instances {
        for c in 0..campaigns {
            let cfg = CampaignConfig {
                instance_id: id.clone(),
                ..base_cfg.clone()
            };
            let summary = mgr
                .run(ir, &reg, &evolver, &executor, &cfg)
                .unwrap_or_else(|e| {
                    eprintln!("campaign failed on {id}: {e}");
                    exit(1);
                });
            println!(
                "[{id}] campaign #{} ({} generations): experiments {} → {}",
                summary.campaign_id,
                summary.generations_run,
                summary.experiments_before,
                summary.experiments_after
            );
            println!(
                "  baseline {:.1} | best score {:.1} via {:?}",
                summary.baseline, summary.best_score, summary.best_schedule
            );
            for p in &summary.reports {
                println!("  report   : {}", p.display());
            }
            for p in &summary.analyses {
                println!("  analysis : {}", p.display());
            }
            for p in &summary.proposals {
                println!("  proposal : {}", p.display());
            }
            for line in &summary.consensus {
                if !line.trim().is_empty() {
                    println!("  meta     : {}", line.trim());
                }
            }
            for n in &summary.notes {
                println!("  note     : {n}");
            }
            if c + 1 < campaigns {
                println!();
            }
        }
    }
    // Self-audit over everything recorded so far: which rules reproduce across
    // instances, and whether the predictor transfers (honest "not yet" if the
    // history is too thin).
    match write_evaluation(std::path::Path::new(&dir), &mgr.db, &MetaLearner::new()) {
        Ok(p) => println!("\nevaluation: {}", p.display()),
        Err(e) => eprintln!("evaluation report failed: {e}"),
    }
    println!(
        "platform state persisted: {} experiments (append-only), {} knowledge facts",
        mgr.db.len(),
        mgr.graph.len()
    );
}
