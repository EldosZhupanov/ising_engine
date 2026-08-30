//! Frozen RC-024 quality harness.
//!
//! Binding protocol: `research/PREREG_RC024_PATH_RELINKING.md`.

use ising_engine::benchmark::stats::wilcoxon_signed_rank;
use ising_engine::engine_v2::ai_scientist::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use ising_engine::engine_v2::evolution::Schedule;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::registry::OperatorRegistry;
use std::io::Write;
use std::path::{Path, PathBuf};

const INSTANCE_DIR: &str = "benchmark_suite/data/gset";
const SWEEPS: u32 = 50;
const REPLICAS: usize = 32;
const SEEDS: [u64; 3] = [101, 102, 103];
const EXPECTED_PAIRS: usize = 90;

fn frozen_args(args: &[String]) -> bool {
    const EXPECTED: [&str; 8] = [
        "--dir",
        INSTANCE_DIR,
        "--sweeps",
        "50",
        "--replicas",
        "32",
        "--seeds",
        "101,102,103",
    ];
    args.len() == EXPECTED.len() + 1 && args[1..].iter().map(String::as_str).eq(EXPECTED)
}

fn is_gset_instance(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix('G'))
        .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    MaterialQualitySignal,
    WorksButImmaterial,
    NoEvidenceOfValue,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::MaterialQualitySignal => "MATERIAL_QUALITY_SIGNAL",
            Self::WorksButImmaterial => "WORKS_BUT_IMMATERIAL",
            Self::NoEvidenceOfValue => "NO_EVIDENCE_OF_VALUE",
        }
    }
}

fn decide(mean: f64, median: f64, p: f64) -> Outcome {
    if p < 0.05 && mean >= 0.01 && median > 0.0 {
        Outcome::MaterialQualitySignal
    } else if p < 0.05 {
        Outcome::WorksButImmaterial
    } else {
        Outcome::NoEvidenceOfValue
    }
}

fn invalid(why: &str) -> ! {
    eprintln!("RC024_INSTRUMENT_INVALID\t{why}");
    std::process::exit(4);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if !frozen_args(&args) {
        eprintln!(
            "usage: exp_rc024_path_relink --dir {INSTANCE_DIR} --sweeps 50 \
             --replicas 32 --seeds 101,102,103"
        );
        std::process::exit(2);
    }

    let mut paths: Vec<PathBuf> = std::fs::read_dir(INSTANCE_DIR)
        .unwrap_or_else(|e| invalid(&format!("cannot read {INSTANCE_DIR}: {e}")))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| is_gset_instance(path))
        .collect();
    paths.sort();
    if paths.len() != 30 {
        invalid(&format!(
            "expected 30 G-Set instances, found {}",
            paths.len()
        ));
    }

    let registry = OperatorRegistry::standard();
    if !registry.contains("path_relink_sweep") {
        invalid("path_relink_sweep is absent from the standard registry");
    }
    let executor = RuntimeExecutor::auto();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    writeln!(
        out,
        "instance\tseed\tcontrol_energy\tcandidate_energy\trelative_gain"
    )
    .unwrap_or_else(|e| invalid(&format!("stdout header: {e}")));

    let mut gains = Vec::with_capacity(EXPECTED_PAIRS);
    let (mut wins, mut losses, mut ties) = (0usize, 0usize, 0usize);
    for path in paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| invalid("an instance name is not UTF-8"));
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| invalid(&format!("cannot read {}: {e}", path.display())));
        let ir = rudy_maxcut_ir(&text)
            .unwrap_or_else(|e| invalid(&format!("cannot parse {}: {e}", path.display())));

        for seed in SEEDS {
            let tasks = [
                ExperimentTask {
                    schedule: Schedule {
                        ops: vec!["metropolis_sweep".to_string()],
                        sweeps: vec![SWEEPS],
                        temp_hi: 4.0,
                        temp_lo: 0.1,
                    },
                    num_replicas: REPLICAS,
                    seed,
                },
                ExperimentTask {
                    schedule: Schedule {
                        ops: vec!["path_relink_sweep".to_string()],
                        sweeps: vec![SWEEPS],
                        temp_hi: 4.0,
                        temp_lo: 0.1,
                    },
                    num_replicas: REPLICAS,
                    seed,
                },
            ];
            let observed = executor.run_batch(&ir, &registry, &tasks);
            if observed.len() != 2 {
                invalid("executor did not return both frozen arms");
            }
            let control = observed[0].score;
            let candidate = observed[1].score;
            if !control.is_finite() || !candidate.is_finite() {
                invalid(&format!("non-finite score for {name} seed {seed}"));
            }
            if candidate > control + 1e-9 {
                invalid(&format!(
                    "candidate worsened {name} seed {seed}: {candidate} > {control}"
                ));
            }
            let gain = if control.abs() > 1e-12 {
                (control - candidate) / control.abs()
            } else {
                0.0
            };
            match candidate.total_cmp(&(control - 1e-9)) {
                std::cmp::Ordering::Less => wins += 1,
                _ if candidate > control + 1e-9 => losses += 1,
                _ => ties += 1,
            }
            gains.push(gain);
            writeln!(
                out,
                "{name}\t{seed}\t{control:.17}\t{candidate:.17}\t{gain:.17}"
            )
            .unwrap_or_else(|e| invalid(&format!("stdout row: {e}")));
        }
    }
    out.flush()
        .unwrap_or_else(|e| invalid(&format!("stdout flush: {e}")));

    if gains.len() != EXPECTED_PAIRS || losses != 0 {
        invalid(&format!(
            "pairs={} wins={wins} losses={losses} ties={ties}",
            gains.len()
        ));
    }
    let mean = gains.iter().sum::<f64>() / gains.len() as f64;
    let mut sorted = gains.clone();
    sorted.sort_by(f64::total_cmp);
    let median = (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0;
    let zeros = vec![0.0; gains.len()];
    let test = wilcoxon_signed_rank(&gains, &zeros);
    let outcome = decide(mean, median, test.p_value);
    eprintln!(
        "RC024_SUMMARY\tpairs={}\tmean_gain={mean:.17}\tmedian_gain={median:.17}\t\
         wins={wins}\tlosses={losses}\tties={ties}\twilcoxon_n={}\twilcoxon_z={:.17}\t\
         p={:.17}\toutcome={}",
        gains.len(),
        test.n,
        test.statistic,
        test.p_value,
        outcome.as_str()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_preregistered_command_is_accepted() {
        let exact = vec![
            "bin".to_string(),
            "--dir".to_string(),
            INSTANCE_DIR.to_string(),
            "--sweeps".to_string(),
            "50".to_string(),
            "--replicas".to_string(),
            "32".to_string(),
            "--seeds".to_string(),
            "101,102,103".to_string(),
        ];
        assert!(frozen_args(&exact));
        for position in 1..exact.len() {
            let mut changed = exact.clone();
            changed[position].push('x');
            assert!(!frozen_args(&changed), "position {position}");
        }
    }

    #[test]
    fn decision_boundaries_are_frozen() {
        assert_eq!(
            decide(0.01, f64::MIN_POSITIVE, 0.049),
            Outcome::MaterialQualitySignal
        );
        assert_eq!(
            decide(0.01 - f64::EPSILON, 0.1, 0.049),
            Outcome::WorksButImmaterial
        );
        assert_eq!(decide(0.02, 0.0, 0.049), Outcome::WorksButImmaterial);
        assert_eq!(decide(0.02, 0.1, 0.05), Outcome::NoEvidenceOfValue);
    }
}
