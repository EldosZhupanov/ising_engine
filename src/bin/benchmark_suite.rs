//! Publication-grade benchmark runner — reproducible from a single command.
//!
//! ```text
//! cargo run --release --bin benchmark_suite               # synthetic suite
//! cargo run --release --bin benchmark_suite -- --download # + G-Set / OR-Lib
//! ```
//!
//! Assembles instances (synthetic families always; standard benchmark sets
//! G-Set, Biq Mac, OR-Library BQP, QPLIB when `--download`/local files are
//! present), then runs every available solver (Ultimate, PT, SA natively;
//! OpenJij and dwave-neal via the Python bridge when installed) under an
//! identical budget and seeds, and writes CSV/JSON/LaTeX/SVG/Markdown with
//! bootstrap CIs and paired significance tests (Wilcoxon + t).
//!
//! Flags: `--download`, `--sizes 64,128`, `--families sk,random,…`,
//! `--instances N`, `--seeds N`, `--sweeps N`, `--exchanges N`,
//! `--gset-limit N`, `--orlib-limit N`, `--out DIR`.

use ising_engine::benchmark::adapters::{Budget, Solver};
use ising_engine::benchmark::instances::{
    self, download_cached, parse_orlib_bqp, Instance, GSET_SUBSET,
};
use ising_engine::benchmark::{run_suite, SuiteConfig};
use ising_engine::core::{CsrMatrix, QuboModel};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::Path;

const BQP_URL: &str = "http://people.brunel.ac.uk/~mastjjb/jeb/orlib/files/bqp50.txt";

// --------------------------------------------------------------------------
// Synthetic instance families (QUBO minimization; native objective = energy)
// --------------------------------------------------------------------------

fn symmetric_model(n: usize, edges: Vec<(usize, usize, f64)>, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for (i, j, w) in edges {
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
        linear: (0..n).map(|_| rng.gen_range(-1.0..1.0)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
        energy_offset: 0.0,
    }
}

fn gen_sk(n: usize, seed: u64) -> QuboModel {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut e = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            e.push((i, j, rng.gen_range(-1.0..1.0)));
        }
    }
    symmetric_model(n, e, &mut rng)
}

fn gen_random(n: usize, density: f64, seed: u64) -> QuboModel {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut e = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < density {
                e.push((i, j, rng.gen_range(-2.0..2.0)));
            }
        }
    }
    symmetric_model(n, e, &mut rng)
}

/// Chimera(m,m,4): m×m grid of K_{4,4} cells (D-Wave 2000Q topology). n = 8m².
fn gen_chimera(target: usize, seed: u64) -> QuboModel {
    let t = 4usize;
    let m = (((target as f64) / (2.0 * t as f64 * t as f64))
        .sqrt()
        .round() as usize)
        .max(1);
    let cell = 2 * t;
    let idx = |r: usize, c: usize, k: usize| -> usize { (r * m + c) * cell + k };
    let n = m * m * cell;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut e = Vec::new();
    for r in 0..m {
        for c in 0..m {
            for a in 0..t {
                for b in 0..t {
                    e.push((idx(r, c, a), idx(r, c, t + b), rng.gen_range(-1.0..1.0)));
                }
            }
            if c + 1 < m {
                for b in 0..t {
                    e.push((
                        idx(r, c, t + b),
                        idx(r, c + 1, t + b),
                        rng.gen_range(-1.0..1.0),
                    ));
                }
            }
            if r + 1 < m {
                for a in 0..t {
                    e.push((idx(r, c, a), idx(r + 1, c, a), rng.gen_range(-1.0..1.0)));
                }
            }
        }
    }
    symmetric_model(n, e, &mut rng)
}

fn gen_model(family: &str, n: usize, seed: u64) -> QuboModel {
    match family {
        "sk" => gen_sk(n, seed),
        "random" => gen_random(n, 0.25, seed),
        "dense" => gen_random(n, 0.9, seed),
        "sparse" => gen_random(n, 4.0 / n as f64, seed),
        "chimera" => gen_chimera(n, seed),
        other => {
            eprintln!("unknown family '{other}', using sk");
            gen_sk(n, seed)
        }
    }
}

// --------------------------------------------------------------------------
// CLI
// --------------------------------------------------------------------------

struct Args {
    families: Vec<String>,
    sizes: Vec<usize>,
    instances: usize,
    seeds: usize,
    sweeps: usize,
    exchanges: usize,
    download: bool,
    gset_limit: usize,
    orlib_limit: usize,
    timeout: u64,
    out: String,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            families: ["sk", "random", "dense", "sparse", "chimera"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            sizes: vec![64, 128],
            instances: 3,
            seeds: 20,
            sweeps: 50,
            exchanges: 40,
            download: false,
            gset_limit: 2,
            orlib_limit: 3,
            timeout: 30,
            out: "benchmarks".to_string(),
        }
    }
}

fn parse_args() -> Args {
    let mut a = Args::default();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let list = |s: &str| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .collect::<Vec<_>>()
    };
    let nums = |s: &str| {
        s.split(',')
            .filter_map(|x| x.trim().parse::<usize>().ok())
            .collect::<Vec<_>>()
    };
    while i < argv.len() {
        let val = |i: usize| argv.get(i + 1).cloned().unwrap_or_default();
        match argv[i].as_str() {
            "--download" => {
                a.download = true;
                i += 1;
                continue;
            }
            "--families" => a.families = list(&val(i)),
            "--sizes" => a.sizes = nums(&val(i)),
            "--instances" => a.instances = val(i).parse().unwrap_or(a.instances),
            "--seeds" => a.seeds = val(i).parse().unwrap_or(a.seeds),
            "--sweeps" => a.sweeps = val(i).parse().unwrap_or(a.sweeps),
            "--exchanges" => a.exchanges = val(i).parse().unwrap_or(a.exchanges),
            "--gset-limit" => a.gset_limit = val(i).parse().unwrap_or(a.gset_limit),
            "--orlib-limit" => a.orlib_limit = val(i).parse().unwrap_or(a.orlib_limit),
            "--timeout" => a.timeout = val(i).parse().unwrap_or(a.timeout),
            "--out" => a.out = val(i),
            other => eprintln!("ignoring unknown flag '{other}'"),
        }
        i += 2;
    }
    a
}

fn main() {
    let args = parse_args();
    let mut instances: Vec<Instance> = Vec::new();

    // Synthetic families: distinct instance per (family, n, replicate).
    for family in &args.families {
        for &n in &args.sizes {
            for rep in 0..args.instances {
                let seed = 0xBEEF ^ ((n as u64) << 8) ^ ((rep as u64) << 1);
                let model = gen_model(family, n, seed);
                // Name keys on the REQUESTED size so topology families whose
                // node count rounds (chimera) don't collide across sizes.
                instances.push(Instance {
                    name: format!("{family}_s{n}_i{rep}"),
                    family: family.clone(),
                    model,
                    maximize: false,
                    best_known: None,
                });
            }
        }
    }

    // Standard benchmark sets (network / local files).
    if args.download {
        let (gset, errs) = instances::load_gset(GSET_SUBSET, args.gset_limit, args.timeout);
        for e in &errs {
            eprintln!("gset skip: {e}");
        }
        eprintln!("loaded {} G-Set instances", gset.len());
        instances.extend(gset);

        match download_cached(BQP_URL, "bqp50.txt", args.timeout)
            .and_then(|t| parse_orlib_bqp(&t, "bqp50"))
        {
            Ok(mut v) => {
                v.truncate(args.orlib_limit);
                eprintln!("loaded {} OR-Library BQP instances", v.len());
                instances.extend(v);
            }
            Err(e) => eprintln!("orlib skip: {e}"),
        }
    }

    instances::attach_best_known(&mut instances, Path::new("benchmarks/best_known.csv"));

    let solvers = [
        Solver::Ultimate,
        Solver::ParallelTempering,
        Solver::SimulatedAnnealing,
        Solver::OpenJij,
        Solver::DwaveNeal,
    ];
    let config = SuiteConfig {
        seeds_per: args.seeds,
        budget: Budget {
            sweeps: args.sweeps,
            exchanges: args.exchanges,
            ..Budget::default()
        },
        out_dir: args.out.clone(),
        ..SuiteConfig::default()
    };

    eprintln!(
        "running {} instances × up to {} solvers × {} seeds …",
        instances.len(),
        solvers.len(),
        args.seeds
    );
    let report = run_suite(&instances, &solvers, &config);
    print!("{report}");
    eprintln!("artifacts written to {}/", args.out);
}
