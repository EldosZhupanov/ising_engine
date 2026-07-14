//! `Plan` — behavior as a schedule of operators (Constitution §3, §8).
//! Named methods ("PT-ICM", "population annealing") are plans, not code paths.
//! A plan is data: the scheduler executes it; the decision engine emits it.

/// Which state backend a plan targets (ADR-0002). Selected by the decision
/// engine, never hard-coded in production paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Byte-per-replica SIMD, float couplings — the dense workhorse
    /// (the production `UltimateSolver` at v0.2).
    DenseByte,
    /// Bit-per-replica planes, integer fields — the sparse machine.
    SparseBitSlice,
}

/// Coarse execution phase (Blueprint pipeline: presolve → explore → exploit
/// → finish). The scheduler advances phases; controllers may trigger early
/// advancement on sensor signals (Constitution §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Presolve,
    Explore,
    Exploit,
    Finish,
}

/// One scheduled operator invocation: by name (resolved against the operator
/// registry), in a phase, with a sweep budget and an optional repeat count.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub operator: String,
    pub phase: Phase,
    pub sweeps: u32,
    pub repeat: u32,
}

/// A complete execution plan. Reproducibility contract: (IR, plan, seeds,
/// threads) determines the trajectory (ADR-0004), so the plan is versioned
/// and recorded with every run.
#[derive(Debug, Clone)]
pub struct Plan {
    pub name: String,
    pub backend: Backend,
    pub num_replicas: usize,
    pub temperatures: Vec<f64>,
    pub steps: Vec<PlanStep>,
    pub seed: u64,
    /// Per-operator selection rationale (Constitution §10 rule 2: every
    /// decision is recorded with its inputs). Filled by the utility-based
    /// Decision Engine; empty for hand-written plans. The Runtime copies the
    /// entry into each step's explainable decision log.
    pub rationale: std::collections::HashMap<String, String>,
}

impl Plan {
    pub fn steps_in(&self, phase: Phase) -> impl Iterator<Item = &PlanStep> {
        self.steps.iter().filter(move |s| s.phase == phase)
    }
}
