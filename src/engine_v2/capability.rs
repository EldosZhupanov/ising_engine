//! Capability system (Constitution §8, §10). Operators are described by WHAT
//! THEY DO (capabilities) and WHAT THEY NEED/SUPPORT (constraints) — never by
//! name. The Decision Engine asks the `OperatorRegistry` for "an operator that
//! can cross barriers on a sparse integer instance" and gets candidates back;
//! it never hard-codes "ICM" or "Houdayer". This is where operator selection
//! becomes a search over properties instead of a switch over names.

use super::operator::Nature;
use super::plan::Backend;

/// What an operator is good FOR — its role in a schedule.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Capability {
    /// Moves broadly across the landscape (Gibbs/Metropolis at high T, PA).
    Exploration = 0,
    /// Refines within a basin (low-T sweeps, rejection-free, greedy).
    Exploitation = 1,
    /// Escapes local minima that single-spin moves cannot (clusters, history
    /// bias, extremal selection).
    BarrierCrossing = 2,
    /// Provably optimal on a subproblem (tree-DP, enumeration, QPBO).
    ExactInference = 3,
    /// Heuristic improvement with no optimality guarantee.
    Approximate = 4,
    /// Produces an informed starting configuration (BP, continuous relax).
    Warmstart = 5,
}

/// A set of capabilities as a small bitset (no external crate).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct CapabilitySet(u32);

impl CapabilitySet {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn with(self, c: Capability) -> Self {
        Self(self.0 | (1 << c as u32))
    }

    pub fn contains(self, c: Capability) -> bool {
        self.0 & (1 << c as u32) != 0
    }

    /// True iff `self` contains every capability in `other`.
    pub fn contains_all(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// True iff `self` shares any capability with `other`.
    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

/// What an operator NEEDS from the substrate/runtime and what it SUPPORTS.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Constraints {
    /// Requires integer couplings (e.g. bucketed extremal, bit-exact energy).
    pub needs_integer: bool,
    /// Requires float couplings / continuous state (e.g. SB, mean-field).
    pub needs_float: bool,
    pub supports_sparse: bool,
    pub supports_dense: bool,
    /// Requires more than one replica (e.g. Houdayer XOR-plane clusters).
    pub needs_replicas: bool,
    /// Reads a temperature from the runtime (thermal operators).
    pub needs_temperature: bool,
}

impl Constraints {
    /// Sensible default: supports both backends, no special requirement.
    pub const fn any() -> Self {
        Self {
            needs_integer: false,
            needs_float: false,
            supports_sparse: true,
            supports_dense: true,
            needs_replicas: false,
            needs_temperature: false,
        }
    }

    pub fn supports_backend(&self, backend: Backend) -> bool {
        match backend {
            Backend::SparseBitSlice => self.supports_sparse,
            Backend::DenseByte => self.supports_dense,
        }
    }
}

/// What an operator MEASURES and reports each `apply` — the sensor channels the
/// Runtime records and a utility-based Decision Engine consumes (Constitution
/// §10, §11). An operator's passport lists which of these it actually produces,
/// so the Runtime knows what signals are trustworthy for that operator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Observable {
    /// Fraction of proposed moves accepted.
    Acceptance = 0,
    /// Change in best/ensemble energy over the call.
    EnergyDelta = 1,
    /// Spread of the energy ensemble (exploration proxy).
    Entropy = 2,
    /// Pairwise replica dissimilarity (population health).
    Diversity = 3,
    /// How far the effective temperature moved (thermal operators).
    TemperatureDrift = 4,
}

/// A set of observables as a small bitset (mirrors `CapabilitySet`).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct ObservableSet(u32);

impl ObservableSet {
    pub const fn empty() -> Self {
        Self(0)
    }
    pub const fn with(self, o: Observable) -> Self {
        Self(self.0 | (1 << o as u32))
    }
    pub fn contains(self, o: Observable) -> bool {
        self.0 & (1 << o as u32) != 0
    }
}

/// Asymptotic cost class of one `apply` sweep — the coarse tag the scheduler and
/// bandit use before falling back to the numeric `CostEstimate`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Complexity {
    /// O(1) amortized per touched site.
    Constant,
    /// O(n) in the number of variables.
    LinearVars,
    /// O(E) — one pass over all edges (a full sweep).
    LinearEdges,
    /// Strictly worse than O(E) (clustering, enumeration).
    Superlinear,
}

/// What an operator PROMISES about its transition. These are checkable claims
/// the Verification Track can assert against (Constitution §4, ADR-0004/0005).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Guarantees {
    /// Never raises the best-so-far energy (monotone descent).
    pub monotone: bool,
    /// Leaves the ensemble in thermal equilibrium for the given temperature.
    pub equilibrates: bool,
    /// Bit-identical replay under a fixed seed (true for all
    /// `TrajectoryPreserving` operators).
    pub deterministic: bool,
    /// Preserves the canonical energy exactly (a pure re-encoding move).
    pub energy_preserving: bool,
}

impl Guarantees {
    /// No promises — the conservative default a new operator starts from.
    pub const fn none() -> Self {
        Self {
            monotone: false,
            equilibrates: false,
            deterministic: false,
            energy_preserving: false,
        }
    }
    pub const fn deterministic() -> Self {
        Self {
            deterministic: true,
            ..Self::none()
        }
    }
}

/// The full self-description — the operator's PASSPORT (Constitution §8).
/// It describes the operator purely through properties, never behavior:
///
/// - State it needs: `constraints`
/// - Transition it makes: `nature` + `capabilities` + `complexity`
/// - Observables it emits: `observables`
/// - Guarantees it keeps: `guarantees`
///
/// The Runtime selects and drives operators through this passport alone; it
/// never knows an operator's identity or internal algorithm.
#[derive(Clone, Copy, Debug)]
pub struct OperatorDescriptor {
    pub name: &'static str,
    pub nature: Nature,
    pub capabilities: CapabilitySet,
    pub constraints: Constraints,
    pub observables: ObservableSet,
    pub guarantees: Guarantees,
    pub complexity: Complexity,
}

/// A property query the Decision Engine sends to the registry. `None` fields
/// are "don't care". Matching is conjunctive.
#[derive(Clone, Copy, Default)]
pub struct CapabilityQuery {
    /// The operator must have ALL of these capabilities.
    pub required: CapabilitySet,
    /// The operator must have AT LEAST ONE of these (if non-empty).
    pub any_of: CapabilitySet,
    /// The operator must support this backend.
    pub backend: Option<Backend>,
    /// The instance is integral (Some(true)) or has float coeffs (Some(false)).
    /// A float instance excludes `needs_integer` operators.
    pub integral: Option<bool>,
    /// The run has >1 replica (Some(true)) or a single replica (Some(false)).
    pub multi_replica: Option<bool>,
    pub nature: Option<Nature>,
}

impl CapabilityQuery {
    pub fn matches(&self, d: &OperatorDescriptor) -> bool {
        if !d.capabilities.contains_all(self.required) {
            return false;
        }
        if self.any_of != CapabilitySet::empty() && !d.capabilities.intersects(self.any_of) {
            return false;
        }
        if let Some(b) = self.backend {
            if !d.constraints.supports_backend(b) {
                return false;
            }
        }
        if self.integral == Some(false) && d.constraints.needs_integer {
            return false;
        }
        if self.integral == Some(true) && d.constraints.needs_float {
            return false;
        }
        if self.multi_replica == Some(false) && d.constraints.needs_replicas {
            return false;
        }
        if let Some(nat) = self.nature {
            if d.nature != nat {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_ops() {
        let s = CapabilitySet::empty()
            .with(Capability::Exploration)
            .with(Capability::BarrierCrossing);
        assert!(s.contains(Capability::Exploration));
        assert!(!s.contains(Capability::ExactInference));
        assert!(s.contains_all(
            CapabilitySet::empty()
                .with(Capability::Exploration)
                .with(Capability::BarrierCrossing)
        ));
        assert!(!s.contains_all(CapabilitySet::empty().with(Capability::ExactInference)));
        assert!(s.intersects(CapabilitySet::empty().with(Capability::BarrierCrossing)));
    }

    #[test]
    fn query_filters_by_backend_and_integrality() {
        let d = OperatorDescriptor {
            name: "x",
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty().with(Capability::BarrierCrossing),
            constraints: Constraints {
                needs_integer: true,
                ..Constraints::any()
            },
            observables: ObservableSet::empty(),
            guarantees: Guarantees::none(),
            complexity: Complexity::LinearEdges,
        };
        // float instance excludes an integer-only operator
        let q = CapabilityQuery {
            integral: Some(false),
            ..Default::default()
        };
        assert!(!q.matches(&d));
        // integer instance + barrier-crossing required → match
        let q2 = CapabilityQuery {
            integral: Some(true),
            required: CapabilitySet::empty().with(Capability::BarrierCrossing),
            ..Default::default()
        };
        assert!(q2.matches(&d));
    }
}
