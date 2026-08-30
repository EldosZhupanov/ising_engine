//! Frozen RC-026 embedding harness.
//!
//! Binding protocol: `research/PREREG_RC026_EMBEDDING_LADDER.md`.
//!
//! Four cycles scored a mechanism as a solo operator over the whole budget. In
//! the field path relinking is applied repeatedly, inside the local search that
//! feeds it endpoints. This varies only how finely it is interleaved, at a
//! Metropolis budget that is identical on every rung.

use ising_engine::engine_v2::ai_scientist::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::registry::OperatorRegistry;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

const INSTANCE_DIR: &str = "benchmark_suite/data/gset";
const SWEEPS: u32 = 50;
const REPLICAS: usize = 32;
const SEEDS: [u64; 3] = [301, 302, 303];
/// §2's ladder. Every value divides `SWEEPS` exactly, so the Metropolis budget
/// is identical at every rung and only the interleaving changes.
const ROUNDS: [u32; 5] = [1, 2, 5, 10, 25];
const CONTROL_OP: &str = "metropolis_sweep";
const CANDIDATE_OP: &str = "path_relink_sweep";
const EXPECTED_ROWS: usize = 450;

fn frozen_args(args: &[String]) -> bool {
    const EXPECTED: [&str; 8] = [
        "--dir",
        INSTANCE_DIR,
        "--sweeps",
        "50",
        "--replicas",
        "32",
        "--seeds",
        "301,302,303",
    ];
    args.len() == EXPECTED.len() + 1 && args[1..].iter().map(String::as_str).eq(EXPECTED)
}

fn invalid(why: &str) -> ! {
    eprintln!("RC026_INSTRUMENT_INVALID\t{why}");
    std::process::exit(4);
}

fn is_gset_instance(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix('G'))
        .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
}

/// The candidate at rung `k`: the operator named `k` times, each block carrying
/// `SWEEPS / k` sweeps. The Runtime instantiates an operator once per plan, so
/// all `k` blocks share one relinker and the mechanism is embedded in a single
/// search rather than restarted `k` times.
fn candidate_schedule(k: u32) -> Schedule {
    Schedule {
        ops: vec![CANDIDATE_OP.to_string(); k as usize],
        sweeps: vec![SWEEPS / k; k as usize],
        temp_hi: 4.0,
        temp_lo: 0.1,
    }
}

/// The control, byte-identical at every rung: that is what makes §3's
/// second mandatory control — a control energy that must not move across k —
/// meaningful.
fn control_schedule() -> Schedule {
    Schedule {
        ops: vec![CONTROL_OP.to_string()],
        sweeps: vec![SWEEPS],
        temp_hi: 4.0,
        temp_lo: 0.1,
    }
}

/// §3's second mandatory control. The control schedule and seed do not depend
/// on `k`, so its energy must not either. The first rung records it and every
/// later rung must match it **bit for bit** — a control that drifts across rungs
/// means state is leaking between arms, and every gain computed against it would
/// be measuring that leak rather than the embedding.
fn record_control(seen: &mut HashMap<u64, f64>, seed: u64, control: f64) -> Result<(), String> {
    match seen.get(&seed) {
        None => {
            seen.insert(seed, control);
            Ok(())
        }
        // Bit-for-bit: `==` on f64 is the intended comparison here, since the
        // two runs are the same schedule at the same seed and must agree
        // exactly, not approximately.
        Some(&first) if first.to_bits() == control.to_bits() => Ok(()),
        Some(&first) => Err(format!(
            "the control moved from {first} to {control}; state is leaking between arms"
        )),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if !frozen_args(&args) {
        eprintln!(
            "usage: exp_rc026_embedding_ladder --dir {INSTANCE_DIR} --sweeps 50 \
             --replicas 32 --seeds 301,302,303"
        );
        std::process::exit(2);
    }
    let registry = OperatorRegistry::standard();
    for op in [CONTROL_OP, CANDIDATE_OP] {
        if !registry.contains(op) {
            invalid(&format!("{op} is absent from the standard registry"));
        }
    }
    for k in ROUNDS {
        if k == 0 || !SWEEPS.is_multiple_of(k) {
            invalid(&format!("k={k} does not divide the {SWEEPS}-sweep budget"));
        }
    }
    let executor = RuntimeExecutor::auto();

    let mut paths: Vec<PathBuf> = std::fs::read_dir(INSTANCE_DIR)
        .unwrap_or_else(|e| invalid(&format!("cannot read {INSTANCE_DIR}: {e}")))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| is_gset_instance(p))
        .collect();
    paths.sort();
    if paths.len() != 30 {
        invalid(&format!(
            "found {} instances, not the 30 PREREG §2 froze",
            paths.len()
        ));
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    writeln!(
        out,
        "instance\trounds\tblock_sweeps\tseed\tcontrol_energy\tcandidate_energy\trelative_gain"
    )
    .unwrap_or_else(|e| invalid(&format!("stdout header: {e}")));

    let mut rows = 0usize;
    for path in &paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_else(|| invalid("an instance name is not UTF-8"));
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| invalid(&format!("cannot read {}: {e}", path.display())));
        let ir = rudy_maxcut_ir(&text)
            .unwrap_or_else(|e| invalid(&format!("cannot parse {}: {e}", path.display())));

        // §3's second mandatory control. The control schedule and seed do not
        // depend on k, so its energy must not either. The first rung records it;
        // every later rung must match it bit for bit.
        let mut control_at_seed: HashMap<u64, f64> = HashMap::new();

        for k in ROUNDS {
            for seed in SEEDS {
                let tasks = [
                    ExperimentTask {
                        schedule: control_schedule(),
                        num_replicas: REPLICAS,
                        seed,
                    },
                    ExperimentTask {
                        schedule: candidate_schedule(k),
                        num_replicas: REPLICAS,
                        seed,
                    },
                ];
                let observed = executor.run_batch(&ir, &registry, &tasks);
                if observed.len() != 2 {
                    invalid("executor did not return both frozen arms");
                }
                let (control, candidate) = (observed[0].score, observed[1].score);
                if !control.is_finite() || !candidate.is_finite() {
                    invalid(&format!("non-finite score for {name} k={k} seed {seed}"));
                }
                if let Err(e) = record_control(&mut control_at_seed, seed, control) {
                    invalid(&format!("{name} seed {seed} at k={k}: {e}"));
                }
                let gain = if control.abs() > 1e-12 {
                    (control - candidate) / control.abs()
                } else {
                    0.0
                };
                rows += 1;
                writeln!(
                    out,
                    "{name}\t{k}\t{}\t{seed}\t{control:.17}\t{candidate:.17}\t{gain:.17}",
                    SWEEPS / k
                )
                .unwrap_or_else(|e| invalid(&format!("stdout row: {e}")));
            }
        }
    }
    out.flush()
        .unwrap_or_else(|e| invalid(&format!("stdout flush: {e}")));
    if rows != EXPECTED_ROWS {
        invalid(&format!(
            "wrote {rows} rows, not the {EXPECTED_ROWS} PREREG §2 froze"
        ));
    }
    eprintln!("RC026_COMPLETE\trows={rows}\tinstances={}", paths.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use ising_engine::engine_v2::ai_scientist::executor::lower_public;
    use ising_engine::engine_v2::ir::ProblemIR;

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
            "301,302,303",
        ]);
        assert!(frozen_args(&exact));
        for position in 1..exact.len() {
            let mut changed = exact.clone();
            changed[position].push('x');
            assert!(!frozen_args(&changed), "position {position}");
        }
        assert!(!frozen_args(&args(&[])));
    }

    /// §4.2: every rung must deliver exactly the same Metropolis budget, and
    /// exactly `k` relinking rounds. If a rung's blocks did not sum to 50 the
    /// ladder would be varying work, not embedding, and any trend it produced
    /// would be uninterpretable.
    #[test]
    fn every_rung_spends_the_same_metropolis_budget() {
        for k in ROUNDS {
            let s = candidate_schedule(k);
            assert_eq!(s.ops.len(), k as usize, "k={k} rounds");
            assert!(s.ops.iter().all(|op| op == CANDIDATE_OP));
            assert_eq!(
                s.sweeps.iter().sum::<u32>(),
                SWEEPS,
                "k={k} does not spend the frozen budget"
            );
            assert_eq!(s.sweeps.len(), s.ops.len());
        }
        assert_eq!(ROUNDS.len() * SEEDS.len() * 30, EXPECTED_ROWS);
    }

    /// §4.1: the Runtime instantiates an operator once per plan, so `k` steps
    /// naming the same operator share one instance and the relinker's state
    /// survives across blocks. The lowering must therefore emit `k` steps of the
    /// same name — not one merged step, which would silently collapse the ladder
    /// to a single round at every rung.
    #[test]
    fn a_rung_lowers_to_k_steps_of_one_operator() {
        let ir = ProblemIR::from_pairs(4, 0.0, vec![1.0, -1.0, 1.0, -1.0], &[(0, 1, 1.0)]);
        for k in ROUNDS {
            let plan = lower_public(
                &ir,
                &ExperimentTask {
                    schedule: candidate_schedule(k),
                    num_replicas: REPLICAS,
                    seed: SEEDS[0],
                },
            );
            assert_eq!(plan.steps.len(), k as usize, "k={k} collapsed");
            assert!(plan.steps.iter().all(|s| s.operator == CANDIDATE_OP));
            assert_eq!(
                plan.steps.iter().map(|s| s.sweeps * s.repeat).sum::<u32>(),
                SWEEPS,
                "k={k} lowered to the wrong budget"
            );
        }
    }

    /// §3's second mandatory control is only meaningful if the control arm truly
    /// does not depend on `k`. It is built without reference to `k`; this pins
    /// that, so a future edge that threaded `k` into it would fail here rather
    /// than silently make the invariance check vacuous.
    #[test]
    fn the_control_arm_does_not_depend_on_the_rung() {
        let first = control_schedule();
        for _ in ROUNDS {
            let s = control_schedule();
            assert_eq!(s.ops, first.ops);
            assert_eq!(s.sweeps, first.sweeps);
            assert_eq!(s.ops, vec![CONTROL_OP.to_string()]);
            assert_eq!(s.sweeps, vec![SWEEPS]);
        }
    }

    /// The invariance guard lives in the run loop, where no test reaches it, so
    /// it is driven directly. Without this, deleting the guard passes every test
    /// and the mandatory control becomes a comment.
    #[test]
    fn a_control_that_moves_across_rungs_is_refused() {
        let mut seen = HashMap::new();
        assert!(
            record_control(&mut seen, 301, -1234.5).is_ok(),
            "first sighting"
        );
        assert!(
            record_control(&mut seen, 301, -1234.5).is_ok(),
            "an exact repeat"
        );
        assert!(
            record_control(&mut seen, 302, -99.0).is_ok(),
            "a different seed"
        );
        let err = record_control(&mut seen, 301, -1234.5000000001)
            .expect_err("a drifting control must be refused");
        assert!(err.contains("leaking between arms"), "{err}");
        // Bit-for-bit, so even a signed zero counts as a change.
        let mut zeros = HashMap::new();
        assert!(record_control(&mut zeros, 1, 0.0).is_ok());
        assert!(
            record_control(&mut zeros, 1, -0.0).is_err(),
            "-0.0 is not 0.0 here"
        );
    }

    #[test]
    fn only_gset_instance_files_are_taken() {
        assert!(is_gset_instance(Path::new("x/G11")));
        assert!(is_gset_instance(Path::new("x/G1")));
        assert!(!is_gset_instance(Path::new("x/README.md")));
        assert!(!is_gset_instance(Path::new("x/metadata.json")));
        assert!(!is_gset_instance(Path::new("x/G")));
        assert!(!is_gset_instance(Path::new("x/G11.bak")));
    }
}
