//! Frozen RC-025 corpus harness.
//!
//! Binding protocol: `research/PREREG_RC025_CORPUS_OR_MECHANISM.md`.
//!
//! Three cycles found their mechanism immaterial on G-Set, where `E = 2V − |E|`
//! makes energy-guided and constraint-guided search the same method. This runs
//! the same mechanisms, unchanged, up a ladder of weight diversity whose bottom
//! rung is G-Set's degenerate class — so the ladder carries its own control.

use ising_engine::engine_v2::ai_scientist::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::{maxcut_to_qubo, parse_rudy};
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::registry::OperatorRegistry;
use std::io::Write;
use std::path::{Path, PathBuf};

const INSTANCE_DIR: &str = "benchmark_suite/data/biqmac";
const SWEEPS: u32 = 50;
const REPLICAS: usize = 32;
const SEEDS: [u64; 3] = [201, 202, 203];
const FILES_PER_FAMILY: usize = 10;
const EXPECTED_ROWS: usize = 540;

/// §2's ladder, frozen. `weights` is the value this document registered; §5.4
/// requires it be **measured** from each file and the run refused on any
/// disagreement, so a mis-registered rung cannot become a silent finding.
struct Family {
    prefix: &'static str,
    ladder: &'static str,
    rung: u8,
    weights: usize,
}

const PRIMARY: [Family; 9] = [
    Family {
        prefix: "pm1s",
        ladder: "d10",
        rung: 1,
        weights: 2,
    },
    Family {
        prefix: "pw01",
        ladder: "d10",
        rung: 2,
        weights: 10,
    },
    Family {
        prefix: "w01",
        ladder: "d10",
        rung: 3,
        weights: 21,
    },
    Family {
        prefix: "g05_100",
        ladder: "d50",
        rung: 1,
        weights: 1,
    },
    Family {
        prefix: "pw05",
        ladder: "d50",
        rung: 2,
        weights: 10,
    },
    Family {
        prefix: "w05",
        ladder: "d50",
        rung: 3,
        weights: 21,
    },
    Family {
        prefix: "pm1d",
        ladder: "d90",
        rung: 1,
        weights: 2,
    },
    Family {
        prefix: "pw09",
        ladder: "d90",
        rung: 2,
        weights: 10,
    },
    Family {
        prefix: "w09",
        ladder: "d90",
        rung: 3,
        weights: 21,
    },
];

/// §3's two pairs, used exactly as their own cycles registered them.
const PAIRS: [(&str, &str, &str); 2] = [
    ("memory_form", "history_field", "tabu_sweep"),
    ("synthesis", "metropolis_sweep", "path_relink_sweep"),
];

fn frozen_args(args: &[String]) -> bool {
    const EXPECTED: [&str; 8] = [
        "--dir",
        INSTANCE_DIR,
        "--sweeps",
        "50",
        "--replicas",
        "32",
        "--seeds",
        "201,202,203",
    ];
    args.len() == EXPECTED.len() + 1 && args[1..].iter().map(String::as_str).eq(EXPECTED)
}

fn invalid(why: &str) -> ! {
    eprintln!("RC025_INSTRUMENT_INVALID\t{why}");
    std::process::exit(4);
}

/// A file belongs to a family when its name is the prefix followed by a
/// separator — not merely when the prefix is a substring. `pw01` must not
/// swallow `pw01x`, and `w01` must not swallow `pw01`.
fn in_family(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .is_some_and(|rest| rest.starts_with('.'))
}

#[cfg_attr(test, derive(Debug))]
struct Loaded {
    ir: ProblemIR,
    weights: usize,
}

/// §5.1 and §5.4, enforced at load rather than asserted in a test: no
/// self-loop, a measured weight diversity, and an energy the frontend agrees
/// with an independent evaluation of the same edge list.
fn load(path: &Path, registered_weights: usize) -> Result<Loaded, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    // The file is inspected directly rather than through the parser's output.
    // `parse_rudy` silently `continue`s past a self-loop and past an
    // out-of-range index, so a QUBO instance handed to it does not fail — it
    // quietly becomes a different model with its linear terms discarded. A
    // guard reading the parser's output could therefore never fire.
    let mut lines = text
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .filter(|t| !t.is_empty());
    let header = lines.next().ok_or("empty file")?;
    let declared: usize = header
        .get(1)
        .ok_or("header has no edge count")?
        .parse()
        .map_err(|_| "bad edge count")?;
    let mut seen = 0usize;
    for t in lines {
        if t.len() < 2 {
            return Err("a body line has fewer than two indices".to_string());
        }
        let (u, v) = (
            t[0].parse::<usize>().map_err(|_| "bad u")?,
            t[1].parse::<usize>().map_err(|_| "bad v")?,
        );
        if u == v {
            // A self-loop is a linear term, so the file is a QUBO instance and
            // not the MaxCut model this ladder compares. §2 excluded those
            // families; this refuses one that slipped through the name filter.
            return Err("self-loop: this is a QUBO instance, not MaxCut".to_string());
        }
        seen += 1;
    }
    if seen != declared {
        return Err(format!(
            "header declares {declared} edges, the body has {seen}"
        ));
    }
    let (n, edges) = parse_rudy(&text)?;
    if edges.len() != declared {
        return Err(format!(
            "the parser kept {} of {declared} edges; some line was dropped",
            edges.len()
        ));
    }
    let mut distinct: Vec<u64> = edges.iter().map(|&(_, _, w)| w.to_bits()).collect();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() != registered_weights {
        return Err(format!(
            "weight diversity {} does not match the {registered_weights} registered in PREREG §2",
            distinct.len()
        ));
    }
    let ir = maxcut_to_qubo(n, &edges);
    energy_is_the_cut(&ir, n, &edges)?;
    Ok(Loaded {
        ir,
        weights: distinct.len(),
    })
}

/// §5.1's tripwire. The frontend minimises −cut; evaluate the cut straight off
/// the parsed edges and require **exact** agreement. A mis-scaled weight would
/// fabricate the very axis this experiment measures, and would do so silently,
/// because a wrong-but-consistent model still produces plausible energies.
///
/// Two assignments are checked, not one: the alternating pattern cuts a
/// structured subset, and its complement-by-halves cuts a different one, so a
/// scaling error that happened to cancel on the first cannot survive the second.
fn energy_is_the_cut(ir: &ProblemIR, n: usize, edges: &[(u32, u32, f64)]) -> Result<(), String> {
    for (label, assignment) in [
        (
            "alternating",
            (0..n).map(|i| (i % 2) as u8).collect::<Vec<u8>>(),
        ),
        (
            "split",
            (0..n).map(|i| u8::from(i * 2 >= n)).collect::<Vec<u8>>(),
        ),
    ] {
        let cut: f64 = edges
            .iter()
            .filter(|&&(u, v, _)| assignment[u as usize] != assignment[v as usize])
            .map(|&(_, _, w)| w)
            .sum();
        let reported = ir.energy(&assignment);
        if reported != -cut {
            return Err(format!(
                "{label} assignment: frontend energy {reported} is not the cut {}",
                -cut
            ));
        }
    }
    Ok(())
}

fn collect(dir: &Path) -> Vec<(&'static Family, PathBuf)> {
    let mut all: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| invalid(&format!("cannot read {}: {e}", dir.display())))
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    all.sort();
    let mut out = Vec::with_capacity(PRIMARY.len() * FILES_PER_FAMILY);
    for family in &PRIMARY {
        let taken: Vec<&String> = all
            .iter()
            .filter(|name| in_family(name, family.prefix))
            .take(FILES_PER_FAMILY)
            .collect();
        if taken.len() != FILES_PER_FAMILY {
            invalid(&format!(
                "{} yielded {} files, not the {FILES_PER_FAMILY} PREREG §2 froze",
                family.prefix,
                taken.len()
            ));
        }
        for name in taken {
            out.push((family, dir.join(name)));
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if !frozen_args(&args) {
        eprintln!(
            "usage: exp_rc025_corpus_ladder --dir {INSTANCE_DIR} --sweeps 50 \
             --replicas 32 --seeds 201,202,203"
        );
        std::process::exit(2);
    }
    let registry = OperatorRegistry::standard();
    for (_, control, candidate) in PAIRS {
        for op in [control, candidate] {
            if !registry.contains(op) {
                invalid(&format!("{op} is absent from the standard registry"));
            }
        }
    }
    let executor = RuntimeExecutor::auto();
    let selected = collect(Path::new(INSTANCE_DIR));

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    writeln!(
        out,
        "pair\tladder\trung\tweights\tinstance\tseed\tcontrol_energy\tcandidate_energy\trelative_gain"
    )
    .unwrap_or_else(|e| invalid(&format!("stdout header: {e}")));

    let mut rows = 0usize;
    for (family, path) in &selected {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| invalid("an instance name is not UTF-8"));
        let loaded = load(path, family.weights)
            .unwrap_or_else(|e| invalid(&format!("{}: {e}", path.display())));

        for (pair, control_op, candidate_op) in PAIRS {
            for seed in SEEDS {
                let task = |op: &str| ExperimentTask {
                    schedule: Schedule {
                        ops: vec![op.to_string()],
                        sweeps: vec![SWEEPS],
                        temp_hi: 4.0,
                        temp_lo: 0.1,
                    },
                    num_replicas: REPLICAS,
                    seed,
                };
                let tasks = [task(control_op), task(candidate_op)];
                let observed = executor.run_batch(&loaded.ir, &registry, &tasks);
                if observed.len() != 2 {
                    invalid("executor did not return both frozen arms");
                }
                let (control, candidate) = (observed[0].score, observed[1].score);
                if !control.is_finite() || !candidate.is_finite() {
                    invalid(&format!("non-finite score for {name} seed {seed}"));
                }
                let gain = if control.abs() > 1e-12 {
                    (control - candidate) / control.abs()
                } else {
                    0.0
                };
                rows += 1;
                writeln!(
                    out,
                    "{pair}\t{}\t{}\t{}\t{name}\t{seed}\t{control:.17}\t{candidate:.17}\t{gain:.17}",
                    family.ladder, family.rung, loaded.weights
                )
                .unwrap_or_else(|e| invalid(&format!("stdout row: {e}")));
            }
        }
    }
    out.flush()
        .unwrap_or_else(|e| invalid(&format!("stdout flush: {e}")));
    if rows != EXPECTED_ROWS {
        invalid(&format!(
            "wrote {rows} rows, not the {EXPECTED_ROWS} PREREG §3 froze"
        ));
    }
    eprintln!("RC025_COMPLETE\trows={rows}\tinstances={}", selected.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("bin".to_string())
            .chain(list.iter().map(|s| s.to_string()))
            .collect()
    }

    #[test]
    fn only_the_preregistered_command_is_accepted() {
        let exact = args(&[
            "--dir",
            INSTANCE_DIR,
            "--sweeps",
            "50",
            "--replicas",
            "32",
            "--seeds",
            "201,202,203",
        ]);
        assert!(frozen_args(&exact));
        for position in 1..exact.len() {
            let mut changed = exact.clone();
            changed[position].push('x');
            assert!(!frozen_args(&changed), "position {position}");
        }
        assert!(!frozen_args(&args(&[])));
    }

    /// `w01` and `pw01` share a suffix and `pm1s` is a prefix of nothing else,
    /// but a substring match would put `pw01.*` into the `w01` rung and quietly
    /// move ten instances one step up the ladder.
    #[test]
    fn family_matching_does_not_leak_across_rungs() {
        assert!(in_family("w01.1.sparse", "w01"));
        assert!(!in_family("pw01.1.sparse", "w01"));
        assert!(in_family("pw01.1.sparse", "pw01"));
        assert!(!in_family("pw05.1.sparse", "pw01"));
        assert!(in_family("g05_100.0.sparse", "g05_100"));
        assert!(!in_family("g05_60.0.sparse", "g05_100"));
        assert!(!in_family("w01x.1.sparse", "w01"));
    }

    /// The ladder registered in PREREG §2: nine families, three ladders of three
    /// rungs, weight diversity strictly increasing within each ladder.
    #[test]
    fn the_ladder_matches_what_was_preregistered() {
        assert_eq!(PRIMARY.len(), 9);
        assert_eq!(PRIMARY.len() * FILES_PER_FAMILY, 90);
        assert_eq!(90 * SEEDS.len() * PAIRS.len(), EXPECTED_ROWS);
        for ladder in ["d10", "d50", "d90"] {
            let rungs: Vec<&Family> = PRIMARY.iter().filter(|f| f.ladder == ladder).collect();
            assert_eq!(rungs.len(), 3, "{ladder}");
            for (i, f) in rungs.iter().enumerate() {
                assert_eq!(f.rung as usize, i + 1);
            }
            assert!(
                rungs.windows(2).all(|w| w[0].weights < w[1].weights),
                "{ladder} is not strictly increasing in weight diversity"
            );
            // Rung 1 is G-Set's degenerate class: the affine bijection
            // E = 2V - |E| holds only at one or two distinct weights.
            assert!(rungs[0].weights <= 2, "{ladder} rung 1 is not degenerate");
            assert!(rungs[1].weights > 2, "{ladder} rung 2 is still degenerate");
        }
    }

    #[test]
    fn a_self_loop_is_refused_rather_than_treated_as_an_edge() {
        let dir = std::env::temp_dir().join("rc025_selfloop_test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("q.sparse");
        std::fs::write(&path, "3 3\n1 1 -5\n1 2 3\n2 3 4\n").expect("write");
        // The count and the diversity are both honest for this file, so the
        // only thing that can refuse it is the self-loop itself.
        let err = load(&path, 3).expect_err("a self-loop must be refused");
        assert!(err.contains("self-loop"), "{err}");
        std::fs::remove_file(&path).ok();
    }

    /// §5.4: the rung's weight diversity is measured, and a disagreement with
    /// the registered value stops the run instead of quietly relabelling a rung.
    #[test]
    fn a_rung_whose_measured_diversity_disagrees_is_refused() {
        let dir = std::env::temp_dir().join("rc025_diversity_test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("w.sparse");
        // Three edges, two distinct weights.
        std::fs::write(&path, "3 3\n1 2 3\n2 3 3\n1 3 7\n").expect("write");
        assert!(load(&path, 2).is_ok(), "the honest count must load");
        let err = load(&path, 21).expect_err("a wrong count must be refused");
        assert!(err.contains("weight diversity"), "{err}");
        std::fs::remove_file(&path).ok();
    }

    /// §5.1: the frontend's energy must equal the cut computed straight off the
    /// parsed edges, on weighted input with both signs.
    #[test]
    fn the_frontend_energy_is_the_cut_on_weighted_signed_input() {
        let dir = std::env::temp_dir().join("rc025_energy_test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("s.sparse");
        std::fs::write(&path, "4 4\n1 2 -7\n2 3 11\n3 4 -2\n1 4 5\n").expect("write");
        let loaded = load(&path, 4).expect("loads");
        assert_eq!(loaded.weights, 4);
        // Alternating assignment 0,1,0,1 cuts every edge of this 4-cycle.
        let x = [0u8, 1, 0, 1];
        assert_eq!(loaded.ir.energy(&x), -(-7.0 + 11.0 - 2.0 + 5.0));
        let y = [0u8, 0, 1, 1];
        assert_eq!(loaded.ir.energy(&y), -(11.0 + 5.0));
        std::fs::remove_file(&path).ok();
    }

    /// The guard above is a tripwire for a frontend that is *wrong*, and the
    /// frontend is currently right — so asserting correct energies can never
    /// exercise it. This drives the tripwire directly, with an IR built from
    /// deliberately mis-scaled weights, which is the shape a real regression in
    /// `maxcut_to_qubo` would take.
    #[test]
    fn a_model_that_does_not_match_its_edges_is_caught() {
        let edges = [(0u32, 1u32, -7.0), (1, 2, 11.0), (2, 3, -2.0), (0, 3, 5.0)];
        let honest = maxcut_to_qubo(4, &edges);
        assert!(energy_is_the_cut(&honest, 4, &edges).is_ok());

        let doubled: Vec<(u32, u32, f64)> =
            edges.iter().map(|&(u, v, w)| (u, v, 2.0 * w)).collect();
        let mis_scaled = maxcut_to_qubo(4, &doubled);
        let err = energy_is_the_cut(&mis_scaled, 4, &edges)
            .expect_err("a doubled model must not pass as the cut");
        assert!(err.contains("is not the cut"), "{err}");

        // The subtle regression is one that *conserves* the total weight, so the
        // alternating assignment — which cuts every edge of this 4-cycle — sums
        // to exactly the same number and sees nothing. Swapping two edges'
        // weights is that shape. Only the split assignment, which cuts a proper
        // subset, can catch it; this is what the second assignment is for.
        let swapped: Vec<(u32, u32, f64)> = edges
            .iter()
            .map(|&(u, v, w)| match (u, v) {
                (0, 1) => (u, v, 11.0),
                (1, 2) => (u, v, -7.0),
                _ => (u, v, w),
            })
            .collect();
        let alternating: Vec<u8> = (0..4).map(|i| (i % 2) as u8).collect();
        let honest_swapped = maxcut_to_qubo(4, &swapped);
        assert_eq!(
            honest_swapped.energy(&alternating),
            honest.energy(&alternating),
            "the fixture must be invisible to the alternating assignment"
        );
        let err = energy_is_the_cut(&honest_swapped, 4, &edges)
            .expect_err("a weight-conserving swap must still be caught");
        assert!(
            err.starts_with("split"),
            "only the split assignment can see it: {err}"
        );
    }
}
