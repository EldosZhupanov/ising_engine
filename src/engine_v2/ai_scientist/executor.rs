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

/// Run a single task through the Runtime and measure it. Deterministic in
/// `task.seed`; touches only read-only core components.
fn run_one(
    ir: &ProblemIR,
    registry: &OperatorRegistry,
    task: &ExperimentTask,
) -> ExperimentOutcome {
    let plan = lower(ir, task);
    let init = vec![0u8; ir.n];
    let mut state = boxed_state(ir, plan.backend, task.num_replicas, &init);
    let mut rt = Runtime::new(RunContext::new(task.seed), &plan);
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
}

impl RuntimeExecutor {
    pub fn new(threads: usize) -> Self {
        Self {
            threads: threads.max(1),
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
        // Each thread handles a contiguous chunk; concatenating in chunk order
        // reproduces the serial ordering exactly.
        let chunk_results: Vec<Vec<ExperimentOutcome>> = thread::scope(|s| {
            let handles: Vec<_> = tasks
                .chunks(chunk)
                .map(|c| {
                    s.spawn(move || {
                        c.iter()
                            .map(|t| run_one(ir, registry, t))
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
}
