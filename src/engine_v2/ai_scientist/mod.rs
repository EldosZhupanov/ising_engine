//! AI Scientist (Stage 5) — a controller layer ABOVE the optimization core.
//!
//! The core (`Runtime`, `Scheduler`, `SpinState`, `SparseBitSlice`,
//! `ReferenceState`, the Experiment Runner, the canonical scorer) is READ-ONLY
//! and untouched. This layer only GENERATES (hypotheses, schedules, genomes) and
//! ANALYZES; all computation is delegated to a `BatchExecutor` that owns the
//! Runtime. The workflow:
//!
//!   AIScientist → hypotheses → candidate schedules → Evolution Engine expands
//!   → BatchExecutor (Runtime) runs → ExperimentDb records every result
//!   → AIScientist analyzes → repeat.
//!
//! Sub-modules:
//!   - `db`       — append-only, reproducible experiment database (no overwrite)
//!   - `stats`    — statistical significance (Welch t + normal approx)
//!   - `executor` — the only Runtime driver; parallel across cores, deterministic
//!   - `scientist`— hypotheses, analysis, and the discovery loop

pub mod campaign;
pub mod cloud;
pub mod concept;
pub mod curiosity;
pub mod dashboard;
pub mod dataset;
pub mod db;
pub mod dynamics;
pub mod evaluation;
pub mod executive;
pub mod executor;
pub mod feature_registry;
pub mod graph;
pub mod lab;
pub mod llm;
pub mod memory_os;
pub mod meta_layer;
pub mod meta_learner;
pub mod model_registry;
pub mod monitor;
pub mod novelty;
pub mod orchestrator;
pub mod planner;
pub mod policy;
pub mod predictor;
pub mod proposal;
pub mod reports;
pub mod scientist;
pub mod stats;
pub mod theory;
pub mod world;

pub use campaign::{CampaignConfig, CampaignManager, CampaignSummary};
pub use cloud::{deep_analysis_prompt, CloudScientist};
pub use concept::{discover as discover_concepts, ConceptConfig, ConceptVerdict};
pub use curiosity::{explore_exploit_priority, CuriosityConfig, CuriosityEngine, CuriousIdeator};
pub use dashboard::{write_dashboard, write_dashboard_with};
pub use dataset::FoundationDataset;
pub use db::{ExperimentDb, ExperimentRecord, RunContext};
pub use dynamics::{
    capture_trajectory, train_on_instances, DynamicsModel, EarlyStopController, PlateauAction,
    Trajectory,
};
pub use evaluation::{evaluate_predictor, rule_reproducibility, write_evaluation};
pub use executive::{
    Action, Decision, ExecutiveBrief, ExecutiveConfig, ResearchExecutive, ResourceState,
};
pub use executor::{BatchExecutor, ExperimentOutcome, ExperimentTask, RuntimeExecutor};
pub use feature_registry::{Feature, FeatureRegistry};
pub use graph::{KnowledgeGraph, Triple};
pub use lab::{
    ExperimentDesigner, FitnessWeights, HypothesisGenerator, Ideator, KnowledgeManager, LabConfig,
    LabReport, ResearchBrief, ScientificLab, Statistician, Verdict,
};
pub use llm::{build_prompt, parse_proposals, LlmHypothesisGenerator, OllamaClient};
pub use memory_os::{recall, Bucket, MemoryManager, MemoryReport, Recollection};
pub use meta_layer::{
    MetaBiasedIdeator, MetaKnowledge, MetaWeights, OperatorSignals, OperatorVerdict, Source,
};
pub use meta_learner::{MetaLearner, OperatorGap, Rule, RuleKind};
pub use model_registry::{ModelKind, ModelRegistry, ModelSnapshot};
pub use monitor::{HealthCheck, HealthReport, HealthStatus, Monitor, MonitorConfig};
pub use novelty::{schedule_distance, NoveltyArchive};
pub use orchestrator::{
    detect_local_llm, OrchestratorConfig, ResearchOrchestrator, ServiceBudget, TickReport,
};
pub use planner::{PlannedTask, ResearchPlanner, TargetScore};
pub use policy::{
    tokens_to_schedule, train_reinforce, OperatorPolicy, PolicyIdeator, ReinforceStep,
};
pub use predictor::{InstanceSignature, Predictor};
pub use proposal::OperatorProposal;
pub use reports::{ReportArchive, ResearchMemory};
pub use scientist::{
    AIScientist, Analysis, DiscoverConfig, DiscoveryReport, Hypothesis, HypothesisStatus,
};
pub use stats::{compare, Comparison};
pub use theory::{
    MechanismHypothesis, MechanismSignature, Prediction, Theory, TheoryConfig, TheoryEngine,
    TheoryStatus,
};
pub use world::{rank_correlation, ObsState, WorldFilteredIdeator, WorldModel};
