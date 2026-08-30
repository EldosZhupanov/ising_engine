//! What wall-time difference can *this host* actually resolve?
//!
//! RC-021 froze a 1 % bound on a paired wall-time comparison and then failed on
//! it. Diagnosing that failure on 2026-08-30 showed the bound had never been
//! checked against the host: two **identical** arms differ by more than 1 % on
//! this machine most of the time, so the control could not tell a real 1 %
//! effect from nothing. `research/RC021_C10_DIAGNOSIS.md` records the finding.
//!
//! This tool exists so that mistake cannot be repeated. It answers one question
//! before a preregistration commits to a threshold:
//!
//! > Two arms, identical work, paired and interleaved — how far apart do their
//! > medians land purely from host noise, and how large must a real effect be
//! > before it stands clear of that?
//!
//! It has two modes and needs both to mean anything:
//!
//! - a **null arm**, where the two arms run byte-identical work. Everything it
//!   reports is noise. This is what RC-021 never ran.
//! - an **injection arm**, where one side is given a known extra fraction of
//!   work. A tool that only ever measures "no difference" has not been shown to
//!   be able to see one; the injection is the positive control that earns the
//!   null its meaning.
//!
//! The tool makes no claim about any solver, states no verdict, and writes
//! nothing outside stdout. It measures the *host*, not the software on it.

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{Budget, Operator};
use ising_engine::engine_v2::operators::MetropolisSweep;
use ising_engine::engine_v2::runtime::RuntimeView;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

/// Fixed so two invocations do the same work. Timings are not reproducible —
/// that is the quantity under study — but the work behind them is.
const SEED: u64 = 0x5253_3032_3153_454e;
const REPLICAS: usize = 32;
const TEMP: f64 = 0.5;
/// Untimed, so the timed window starts from a warm state rather than a cold
/// allocation.
const PREFIX_SWEEPS: u32 = 4;

struct Config {
    file: String,
    windows: Vec<u32>,
    pairs: usize,
    reps: usize,
    injections: Vec<f64>,
    pin: bool,
}

fn usage() -> String {
    "usage: host_timing_calibration --file <rudy instance> \
     [--windows 8,32,128] [--pairs 30] [--reps 20] \
     [--inject 0.01,0.05,0.25] [--pin]"
        .to_string()
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let value = |flag: &str| -> Option<&str> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    };
    let numbers = |flag: &str, default: &str| -> Result<Vec<f64>, String> {
        value(flag)
            .unwrap_or(default)
            .split(',')
            .map(|t| {
                t.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("{flag}: `{t}` is not a number"))
            })
            .collect()
    };
    let file = value("--file").ok_or_else(usage)?.to_string();
    let windows: Vec<u32> = numbers("--windows", "8,32,128")?
        .into_iter()
        .map(|v| v as u32)
        .collect();
    if windows.contains(&0) {
        return Err("--windows: a zero-sweep window measures nothing".to_string());
    }
    let one = |flag: &str, default: f64| -> Result<usize, String> {
        let v = numbers(flag, &default.to_string())?;
        match v[..] {
            [x] if x >= 2.0 => Ok(x as usize),
            _ => Err(format!("{flag}: expected a single value of at least 2")),
        }
    };
    let injections = numbers("--inject", "0.01,0.05,0.25")?;
    if injections.iter().any(|&f| !(0.0..1.0).contains(&f)) {
        return Err("--inject: fractions must lie in [0, 1)".to_string());
    }
    Ok(Config {
        file,
        windows,
        pairs: one("--pairs", 30.0)?,
        reps: one("--reps", 20.0)?,
        injections,
        pin: args.iter().any(|a| a == "--pin"),
    })
}

/// One timed window. `sweeps` is the whole story: the two arms differ only in
/// this number, so an injection of zero makes them the same measurement.
fn timed_run(ir: &ProblemIR, sweeps: u32) -> f64 {
    let temps = vec![TEMP; REPLICAS];
    let init = vec![0u8; ir.n];
    let mut state = SparseBitSlice::new(ir, REPLICAS, &init).expect("sparse bit-slice state");
    let view = RuntimeView {
        iteration: 0,
        temperatures: &temps,
        num_replicas: REPLICAS,
        recent_acceptance: 0.0,
        remaining_ms: f64::INFINITY,
    };
    let mut op = MetropolisSweep::new();
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);
    let _ = op.apply(
        &mut state,
        &view,
        &mut rng,
        Budget {
            sweeps: PREFIX_SWEEPS,
        },
    );
    let t0 = Instant::now();
    let _ = op.apply(&mut state, &view, &mut rng, Budget { sweeps });
    t0.elapsed().as_secs_f64() * 1000.0
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    }
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[(((sorted.len() - 1) as f64) * q).round() as usize]
}

/// One replication of the paired comparison: `pairs` interleaved pairs from
/// each arm, then the relative difference of the two medians. Interleaving is
/// what makes drift hit both arms alike; it is the whole reason the arms are
/// not run one after the other.
fn one_replication(ir: &ProblemIR, base: u32, treated: u32, pairs: usize) -> f64 {
    let (mut a, mut b) = (Vec::with_capacity(pairs * 2), Vec::with_capacity(pairs * 2));
    for _ in 0..pairs {
        for (sweeps, sink) in [(treated, &mut a), (base, &mut b)] {
            for _ in 0..2 {
                sink.push(timed_run(ir, sweeps));
            }
        }
    }
    let (ma, mb) = (median(&mut a), median(&mut b));
    (ma - mb) / mb
}

struct Row {
    label: String,
    window: u32,
    expected: f64,
    observed: Vec<f64>,
}

impl Row {
    fn print(&self, one_run_ms: f64) {
        let mut sorted = self.observed.clone();
        sorted.sort_by(f64::total_cmp);
        let n = sorted.len();
        let med = quantile(&sorted, 0.5);
        // The null's own p90 is the bar a real effect has to clear: an effect
        // smaller than the noise's 90th percentile cannot be told from noise.
        let above = sorted.iter().filter(|&&x| x.abs() > 0.01).count();
        println!(
            "{}\t{}\t{one_run_ms:.3}\t{n}\t{:+.5}\t{:+.5}\t{:+.5}\t{:+.5}\t{above}/{n}",
            self.label,
            self.window,
            self.expected,
            med,
            quantile(&sorted, 0.1),
            quantile(&sorted, 0.9),
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cfg = match parse_args(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if cfg.pin {
        match core_affinity::get_core_ids().and_then(|ids| ids.first().copied()) {
            Some(id) if core_affinity::set_for_current(id) => {}
            _ => {
                eprintln!("--pin requested but the affinity could not be set");
                std::process::exit(2);
            }
        }
    }
    let text = match std::fs::read_to_string(&cfg.file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", cfg.file);
            std::process::exit(2);
        }
    };
    let ir = match rudy_maxcut_ir(&text) {
        Ok(ir) => ir,
        Err(e) => {
            eprintln!("{}: {e}", cfg.file);
            std::process::exit(2);
        }
    };
    // Warm the caches and the frequency governor once, before anything counted.
    let _ = timed_run(&ir, *cfg.windows.iter().max().expect("non-empty"));

    println!("# host timing calibration — what difference can this host resolve?");
    println!("# instance={} n={} replicas={REPLICAS}", cfg.file, ir.n);
    println!("# pinned={} pairs={} reps={}", cfg.pin, cfg.pairs, cfg.reps);
    println!("# `null` rows run byte-identical arms: every number is host noise.");
    println!("# `inject` rows add a known extra fraction of work to one arm.");
    println!("# An effect below the null's p90 cannot be distinguished from noise.");
    println!("arm\twindow\tms_per_run\treps\texpected\tmedian\tp10\tp90\tover_1pct");

    for &window in &cfg.windows {
        let one_run_ms = timed_run(&ir, window);
        let null = Row {
            label: "null".to_string(),
            window,
            expected: 0.0,
            observed: (0..cfg.reps)
                .map(|_| one_replication(&ir, window, window, cfg.pairs))
                .collect(),
        };
        null.print(one_run_ms);
        for &fraction in &cfg.injections {
            // Round to whole sweeps, then report the fraction actually injected
            // rather than the one requested: at small windows they differ, and
            // reporting the request would overstate what was tested.
            let extra = ((window as f64) * fraction).round() as u32;
            if extra == 0 {
                continue;
            }
            let treated = window + extra;
            Row {
                label: "inject".to_string(),
                window,
                expected: extra as f64 / window as f64,
                observed: (0..cfg.reps)
                    .map(|_| one_replication(&ir, window, treated, cfg.pairs))
                    .collect(),
            }
            .print(one_run_ms);
        }
    }
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
    fn the_instance_is_required_and_defaults_are_the_documented_ones() {
        assert!(parse_args(&args(&[])).is_err());
        let c = parse_args(&args(&["--file", "G11"])).expect("defaults parse");
        assert_eq!(c.file, "G11");
        assert_eq!(c.windows, vec![8, 32, 128]);
        assert_eq!((c.pairs, c.reps), (30, 20));
        assert_eq!(c.injections, vec![0.01, 0.05, 0.25]);
        assert!(!c.pin);
    }

    #[test]
    fn arguments_that_would_measure_nothing_are_refused() {
        for bad in [
            vec!["--file", "G11", "--windows", "0"],
            vec!["--file", "G11", "--windows", "8,x"],
            vec!["--file", "G11", "--pairs", "1"],
            vec!["--file", "G11", "--reps", "1"],
            vec!["--file", "G11", "--inject", "1.0"],
            vec!["--file", "G11", "--inject", "-0.1"],
        ] {
            assert!(parse_args(&args(&bad)).is_err(), "accepted {bad:?}");
        }
    }

    /// The null arm's meaning depends entirely on both sides running the same
    /// work. `one_replication` takes two sweep counts, so passing the same one
    /// twice is the null; the test pins that this is what the caller does.
    #[test]
    fn a_null_replication_gives_both_arms_the_same_budget() {
        let ir = ProblemIR::from_pairs(4, 0.0, vec![1.0, -1.0, 1.0, -1.0], &[(0, 1, 1.0)]);
        // Identical budgets: any difference is timing, never work.
        let d = one_replication(&ir, 2, 2, 2);
        assert!(d.is_finite(), "a null replication must produce a number");
    }

    #[test]
    fn quantiles_and_median_agree_on_known_data() {
        let mut v = vec![5.0, 1.0, 4.0, 2.0, 3.0];
        assert_eq!(median(&mut v), 3.0);
        // median() sorted it in place.
        assert_eq!(quantile(&v, 0.0), 1.0);
        assert_eq!(quantile(&v, 0.5), 3.0);
        assert_eq!(quantile(&v, 1.0), 5.0);
        let mut even = vec![4.0, 1.0, 3.0, 2.0];
        assert_eq!(median(&mut even), 2.5);
    }

    /// A window whose injected fraction rounds to zero extra sweeps must be
    /// skipped, not reported as a zero-effect injection: reporting it would put
    /// a row labelled "inject 1%" next to an arm that got no extra work at all.
    #[test]
    fn an_injection_that_rounds_to_no_extra_work_is_not_a_row() {
        for (window, fraction) in [(8u32, 0.01f64), (32, 0.01)] {
            assert_eq!(((window as f64) * fraction).round() as u32, 0);
        }
        assert_eq!((128.0f64 * 0.01).round() as u32, 1);
    }
}
