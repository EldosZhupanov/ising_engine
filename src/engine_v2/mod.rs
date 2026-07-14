//! Engine v2 — the Universal Physics-Inspired Optimization Architecture.
//!
//! Constitutional module (research/ISING_ENGINE_CONSTITUTION.md §3, §7–§11;
//! ADR-0001..0008). This is the framework tier: typed IR, `SpinState`
//! backends, `Operator` contract, capability system, `OperatorRegistry`,
//! `RunContext`, `Runtime`, `Scheduler`, and the `DecisionEngine`.
//! The production `UltimateSolver` is untouched; it becomes the `DenseByte`
//! backend behind this interface at Blueprint v0.2.
//!
//! Execution layering (each layer calls only layers below it):
//!   Decision → Runtime → Scheduler → Operator
//!   with state/backends and the IR beneath, and the RunContext (hardware,
//!   budget, profiler, logger) owned by the Runtime and never seen by operators.
//!
//! No optimization algorithms live here. Dynamics are operators; behavior is a
//! `Plan`; operators are selected by CAPABILITY, not by name.

pub mod ai_scientist;
pub mod backends;
pub mod capability;
pub mod context;
pub mod decision;
pub mod evolution;
pub mod frontend;
pub mod ir;
pub mod knowledge;
pub mod operator;
pub mod operators;
pub mod plan;
pub mod registry;
pub mod runtime;
pub mod scheduler;
pub mod state;

pub use ai_scientist::{
    AIScientist, BatchExecutor, DiscoverConfig, DiscoveryReport, ExperimentDb, Hypothesis,
    RuntimeExecutor,
};
pub use capability::{
    Capability, CapabilityQuery, CapabilitySet, Complexity, Constraints, Guarantees, Observable,
    ObservableSet, OperatorDescriptor,
};
pub use context::{HardwareProfile, Profiler, RunContext};
pub use decision::{geometric_ladder, DecisionEngine, InstanceStats};
pub use evolution::{boxed_state, EvolveConfig, EvolveResult, Evolver, Schedule, UtilityWeights};
pub use ir::ProblemIR;
pub use knowledge::{Experience, InstanceFeatures, KnowledgeBase};
pub use operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
pub use operators::{
    EliteBroadcast, ExtremalOptimization, GibbsColorSweep, GreedyDescent, HistoryFieldSweep,
    HoudayerClusterMove, IsoenergeticClusterMove, MetropolisSweep, PopulationResample,
    RandomFlipSweep, RandomRestartWorst, ReplicaExchange, SteepestDescent,
};
pub use plan::{Backend, Phase, Plan, PlanStep};
pub use registry::OperatorRegistry;
pub use runtime::{QualityMetrics, RunRecord, Runtime, RuntimeView, StepEvent};
pub use scheduler::{PhaseScheduler, ScheduledStep, Scheduler};
pub use state::{ReplicaMask, SpinState, StateDigest};
