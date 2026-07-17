//! Batch executor (Stage 5) — the boundary where the AI Scientist's generated
//! schedules become actual computation. This is the ONLY place the read-only
//! Runtime is driven; the scientist never touches it. Tasks are independent and
//! each fully seeded, so a batch is embarrassingly parallel AND reproducible:
//! `RuntimeExecutor` fans it out across CPU cores with `std::thread::scope` and
//! reassembles results in task order, so the output is identical to a serial run
//! (ADR-0004). The same `BatchExecutor` trait is the seam for a future
//! multi-machine executor — the scientist code above it never changes.

use super::super::context::RunContext;
use super::super::decision::{geometric_ladder, DecisionEngine};
use super::super::evolution::{boxed_state, Schedule};
use super::super::ir::ProblemIR;
use super::super::plan::{Backend, Phase, Plan, PlanStep};
use super::super::registry::OperatorRegistry;
use super::super::runtime::Runtime;
use super::dynamics::{DynamicsModel, EarlyStopController, PlateauAction};
use std::thread;

/// One reproducible experiment to run: a schedule plus its execution params.
#[derive(Debug, Clone)]
pub struct ExperimentTask {
    pub schedule: Schedule,
    pub num_replicas: usize,
    pub seed: u64,
}

/// The measured result of one task.
#[derive(Debug, Clone)]
pub struct ExperimentOutcome {
    pub score: f64,
    pub best_state: Vec<u8>,
    pub work: f64,
    pub backend: Backend,
}

/// Executes batches of experiments. Implementors own all computation; the AI
/// Scientist only hands them tasks (Stage 5: "Runtime performs all computation").
pub trait BatchExecutor {
    fn run_batch(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        tasks: &[ExperimentTask],
    ) -> Vec<ExperimentOutcome>;
}

/// Public lowering (same `Schedule` → `Plan` mapping the executor uses), so
/// other modules — the Dynamics/World models capturing trajectories — run the
/// IDENTICAL plan the batch executor would, not a divergent copy.
pub fn lower_public(ir: &ProblemIR, task: &ExperimentTask) -> Plan {
    lower(ir, task)
}

/// Lower a backend-agnostic `Schedule` to a concrete `Plan` for the Runtime.
fn lower(ir: &ProblemIR, task: &ExperimentTask) -> Plan {
    let backend = DecisionEngine::analyze(ir).select_backend();
    let steps = task
        .schedule
        .ops
        .iter()
        .zip(&task.schedule.sweeps)
        .map(|(op, &sw)| PlanStep {
            operator: op.clone(),
            phase: Phase::Exploit, // single phase ⇒ executed in declared order
            sweeps: sw.max(1),
            repeat: 1,
        })
        .collect();
    Plan {
        name: "ai_scientist".into(),
        backend,
        num_replicas: task.num_replicas,
        temperatures: geometric_ladder(
            task.num_replicas,
            task.schedule.temp_hi,
            task.schedule.temp_lo,
        ),
        steps,
        seed: task.seed,
        rationale: Default::default(),
    }
}

/// Opt-in early-stopping for the executor: attach a Dynamics-model
/// [`EarlyStopController`] to EVERY run, so a task that has plateaued stops (or
/// switches operator) instead of burning its full budget. Deterministic and
/// replay-safe (the controller is a pure function of the run's own sensors); a
/// `RuntimeExecutor` without this behaves EXACTLY as before (bit-identical).
#[derive(Debug, Clone)]
pub struct EarlyStopConfig {
    pub model: DynamicsModel,
    /// Stop/switch once the predicted remaining improvement falls below this.
    pub epsilon: f64,
    /// …but only after this fraction of the operator's budget has elapsed.
    pub min_frac: f64,
    pub action: PlateauAction,
}

impl EarlyStopConfig {
    /// Sensible defaults: stop the run once < 0.5% further improvement is
    /// predicted, after at least half the budget.
    pub fn new(model: DynamicsModel) -> Self {
        Self {
            model,
            epsilon: 0.005,
            min_frac: 0.5,
            action: PlateauAction::Stop,
        }
    }
}

/// Run a single task through the Runtime and measure it. Deterministic in
/// `task.seed`; touches only read-only core components. When `early` is set, an
/// opt-in Dynamics early-stop controller is attached (a fresh one per run, since
/// it accumulates the run's own best-history).
fn run_one(
    ir: &ProblemIR,
    registry: &OperatorRegistry,
    task: &ExperimentTask,
    early: Option<&EarlyStopConfig>,
) -> ExperimentOutcome {
    let plan = lower(ir, task);
    let init = vec![0u8; ir.n];
    let mut state = boxed_state(ir, plan.backend, task.num_replicas, &init);
    let mut rt = Runtime::new(RunContext::new(task.seed), &plan);
    if let Some(cfg) = early {
        rt = rt.with_controller(Box::new(EarlyStopController::new(
            cfg.model.clone(),
            ir,
            cfg.epsilon,
            cfg.min_frac,
            cfg.action,
        )));
    }
    match rt.run(&plan, state.as_mut(), registry, ir) {
        Ok(rec) => {
            let work: f64 = rt.context().profiler.entries().map(|(_, e)| e.work).sum();
            ExperimentOutcome {
                score: rec.best_energy,
                best_state: rec.best_state,
                work,
                backend: plan.backend,
            }
        }
        Err(_) => ExperimentOutcome {
            score: f64::INFINITY,
            best_state: vec![0u8; ir.n],
            work: 0.0,
            backend: plan.backend,
        },
    }
}

/// Local executor that distributes a batch across `threads` OS threads.
pub struct RuntimeExecutor {
    pub threads: usize,
    /// Opt-in Dynamics early-stop applied to every run. `None` ⇒ full budget,
    /// bit-identical to the historical executor.
    early_stop: Option<EarlyStopConfig>,
}

impl RuntimeExecutor {
    pub fn new(threads: usize) -> Self {
        Self {
            threads: threads.max(1),
            early_stop: None,
        }
    }

    /// Threads = detected logical cores.
    pub fn auto() -> Self {
        Self::new(
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        )
    }

    /// Enable opt-in Dynamics early-stopping on every run with default
    /// thresholds. Changes trajectories BY DESIGN (that is the point) — so it is
    /// opt-in and never the default; the plain executor stays bit-identical.
    pub fn with_early_stop_default(mut self, model: DynamicsModel) -> Self {
        self.early_stop = Some(EarlyStopConfig::new(model));
        self
    }

    /// Enable opt-in Dynamics early-stopping with explicit thresholds.
    pub fn with_early_stop(
        mut self,
        model: DynamicsModel,
        epsilon: f64,
        min_frac: f64,
        action: PlateauAction,
    ) -> Self {
        self.early_stop = Some(EarlyStopConfig {
            model,
            epsilon,
            min_frac,
            action,
        });
        self
    }

    /// Whether opt-in early-stopping is active.
    pub fn early_stopping(&self) -> bool {
        self.early_stop.is_some()
    }
}

impl BatchExecutor for RuntimeExecutor {
    fn run_batch(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        tasks: &[ExperimentTask],
    ) -> Vec<ExperimentOutcome> {
        if tasks.is_empty() {
            return Vec::new();
        }
        let nthreads = self.threads.min(tasks.len()).max(1);
        let chunk = tasks.len().div_ceil(nthreads);
        let early = self.early_stop.as_ref();
        // Each thread handles a contiguous chunk; concatenating in chunk order
        // reproduces the serial ordering exactly.
        let chunk_results: Vec<Vec<ExperimentOutcome>> = thread::scope(|s| {
            let handles: Vec<_> = tasks
                .chunks(chunk)
                .map(|c| {
                    s.spawn(move || {
                        c.iter()
                            .map(|t| run_one(ir, registry, t, early))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("experiment thread panicked"))
                .collect()
        });
        chunk_results.into_iter().flatten().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ir7() -> ProblemIR {
        ProblemIR::from_pairs(
            7,
            -2.0,
            vec![1.0, -2.0, 3.0, 0.0, -1.0, 2.0, 1.0],
            &[
                (0, 1, -1.0),
                (1, 2, 2.0),
                (2, 3, -3.0),
                (0, 3, 1.0),
                (3, 4, -1.0),
                (4, 5, 2.0),
                (5, 6, -2.0),
                (2, 6, 1.0),
            ],
        )
    }

    fn task(seed: u64) -> ExperimentTask {
        ExperimentTask {
            schedule: Schedule {
                ops: vec!["gibbs_color_sweep".into(), "greedy_descent".into()],
                sweeps: vec![15, 10],
                temp_hi: 4.0,
                temp_lo: 0.1,
            },
            num_replicas: 16,
            seed,
        }
    }

    #[test]
    fn parallel_matches_serial_and_is_deterministic() {
        let ir = ir7();
        let reg = OperatorRegistry::standard();
        let tasks: Vec<ExperimentTask> = (0..12).map(task).collect();

        let serial = RuntimeExecutor::new(1).run_batch(&ir, &reg, &tasks);
        let parallel = RuntimeExecutor::new(4).run_batch(&ir, &reg, &tasks);
        assert_eq!(serial.len(), tasks.len());
        for (a, b) in serial.iter().zip(&parallel) {
            assert_eq!(a.score, b.score, "parallel execution changed a result");
            assert_eq!(a.best_state, b.best_state);
            assert_eq!(a.work, b.work);
        }
        // Canonical re-score matches the reported score.
        for (t, o) in tasks.iter().zip(&serial) {
            assert_eq!(ir.energy(&o.best_state), o.score, "seed {}", t.seed);
            assert!(o.work > 0.0);
        }
    }

    #[test]
    #[ignore = "profiling: ISING_PROFILE_RUDY=<gset file> cargo test profile_operator_kernels -- --ignored --nocapture"]
    fn profile_operator_kernels_on_a_real_instance() {
        use crate::engine_v2::frontend::rudy_maxcut_ir;
        let Ok(path) = std::env::var("ISING_PROFILE_RUDY") else {
            return;
        };
        let ir = rudy_maxcut_ir(&std::fs::read_to_string(&path).expect("read")).expect("parse");
        let reg = OperatorRegistry::standard();
        let exec = RuntimeExecutor::new(1); // single-thread for clean per-op timing
        let (replicas, sweeps) = (64usize, 50u32);
        let seeds = [1u64, 2, 3];
        eprintln!(
            "profiling n={} edges≈? on the selected backend; {replicas} replicas, {sweeps} sweeps, {} seeds",
            ir.n,
            seeds.len()
        );
        let ops: Vec<String> = reg.names().map(|s| s.to_string()).collect();
        let mut timings: Vec<(String, f64)> = Vec::new();
        for op in &ops {
            let tasks: Vec<ExperimentTask> = seeds
                .iter()
                .map(|&s| ExperimentTask {
                    schedule: Schedule {
                        ops: vec![op.clone()],
                        sweeps: vec![sweeps],
                        temp_hi: 4.0,
                        temp_lo: 0.1,
                    },
                    num_replicas: replicas,
                    seed: s,
                })
                .collect();
            let _ = exec.run_batch(&ir, &reg, &tasks); // warm
            let iters = 5;
            let t = std::time::Instant::now();
            for _ in 0..iters {
                std::hint::black_box(exec.run_batch(&ir, &reg, &tasks));
            }
            timings.push((
                op.clone(),
                t.elapsed().as_secs_f64() / iters as f64 * 1000.0,
            ));
        }
        timings.sort_by(|a, b| b.1.total_cmp(&a.1));
        eprintln!(
            "--- per-operator wall time (batch of {} runs) ---",
            seeds.len()
        );
        for (op, ms) in &timings {
            eprintln!("  {op:>26}: {ms:8.2} ms");
        }
    }

    #[test]
    fn early_stop_executor_stays_deterministic_and_valid() {
        use super::super::dynamics::train_on_instances;
        let ir = ir7();
        let reg = OperatorRegistry::standard();
        // Train a small Dynamics model on a couple of long schedules.
        let train_sched = Schedule {
            ops: vec!["metropolis_sweep".into(), "greedy_descent".into()],
            sweeps: vec![20, 20],
            temp_hi: 4.0,
            temp_lo: 0.1,
        };
        let Some(model) = train_on_instances(&[&ir], &reg, &train_sched, 16, &[1, 2, 3], 1e-4)
        else {
            return; // honest skip if there is too little data to fit
        };

        let tasks: Vec<ExperimentTask> = (0..8).map(task).collect();
        // Opt-in early-stop, serial vs parallel: attaching a controller must not
        // break the executor's determinism guarantee (ADR-0004).
        let serial = RuntimeExecutor::new(1)
            .with_early_stop(model.clone(), 0.01, 0.5, PlateauAction::Stop)
            .run_batch(&ir, &reg, &tasks);
        let parallel = RuntimeExecutor::new(4)
            .with_early_stop(model, 0.01, 0.5, PlateauAction::Stop)
            .run_batch(&ir, &reg, &tasks);
        assert_eq!(serial.len(), tasks.len());
        for (a, b) in serial.iter().zip(&parallel) {
            assert_eq!(a.score, b.score, "early-stop broke thread determinism");
            assert_eq!(a.best_state, b.best_state);
        }
        // Results remain VALID: the canonical re-score matches the reported score.
        for (t, o) in tasks.iter().zip(&serial) {
            assert_eq!(ir.energy(&o.best_state), o.score, "seed {}", t.seed);
        }
        // The plain executor (no early-stop) is unchanged — the opt-in never
        // touches the default path.
        assert!(!RuntimeExecutor::new(1).early_stopping());
    }
}
