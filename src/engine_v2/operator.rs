//! `Operator` — the instruction set of optimization dynamics
//! (Constitution §8, ADR-0001). Each operator is one physical law: a typed
//! transformation over `SpinState`. It knows nothing about other operators,
//! nothing about wall-clock time, temperature schedules, or adaptation — the
//! Runtime owns all of that and hands the operator a read-only `RuntimeView`.

use super::capability::OperatorDescriptor;
use super::runtime::RuntimeView;
use super::state::SpinState;
use rand_chacha::ChaCha8Rng;

/// Whether an operator preserves trajectories bit-identically (may enter A/B
/// comparisons) or changes behavior (metadynamics bias, ε-quantization,
/// continuous relaxation). Behavior-changing operators must be typed as such,
/// may serve only in declared roles, and their output is always re-scored
/// canonically (Constitution §4.11, ADR-0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nature {
    TrajectoryPreserving,
    BehaviorChanging,
}

/// Sweep budget for a single `apply` call. Deterministic runs express work in
/// sweeps, not wall-clock (ADR-0004); `RuntimeView::remaining_ms` is a guard an
/// operator must not branch on unless it declares itself BehaviorChanging.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    pub sweeps: u32,
}

/// Sensor data an operator emits for the adaptive controllers (Constitution
/// §11). Cheap by construction — counters accumulated during `apply`.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub accepted: u64,
    pub proposed: u64,
    pub best_energy: Option<f64>,
    /// Estimated work units done (fed to the profiler).
    pub work: f64,
    /// Operator-specific scalars (e.g. mean cluster size), keyed by short tag.
    pub aux: Vec<(&'static str, f64)>,
}

impl Report {
    pub fn acceptance_rate(&self) -> f64 {
        if self.proposed == 0 {
            0.0
        } else {
            self.accepted as f64 / self.proposed as f64
        }
    }
}

/// Cost estimate the scheduler/bandit use to allocate budget. An operator
/// without a cost model cannot be scheduled (Constitution §8).
#[derive(Debug, Clone, Copy)]
pub struct CostEstimate {
    /// Estimated work units (∝ edges·replicas) per sweep.
    pub work_per_sweep: f64,
}

/// Instance statistics an operator may consult for its cost model.
#[derive(Debug, Clone, Copy)]
pub struct InstanceShape {
    pub n: usize,
    pub num_pairs: usize,
    pub num_replicas: usize,
}

/// The uniform contract. Implementors are the LEGO bricks of the engine and
/// are registered in the `OperatorRegistry` by their `descriptor()`.
pub trait Operator {
    /// Self-description for the registry and capability-based selection.
    fn descriptor(&self) -> OperatorDescriptor;

    fn cost_model(&self, shape: InstanceShape) -> CostEstimate;

    /// Apply this physical law to the state for the given budget. Must not
    /// allocate on the hot path; must be deterministic given `view` + `rng`.
    fn apply(
        &mut self,
        state: &mut dyn SpinState,
        view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report;

    fn name(&self) -> &'static str {
        self.descriptor().name
    }

    fn nature(&self) -> Nature {
        self.descriptor().nature
    }
}
