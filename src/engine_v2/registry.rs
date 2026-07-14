//! `OperatorRegistry` — the catalog of every operator (Constitution §8, §10).
//!
//! An operator is registered once via a factory. The registry stores its
//! `OperatorDescriptor` (queryable without instantiating) and can construct
//! fresh instances for a run. The Decision Engine selects operators by
//! CAPABILITY, not by name:
//!
//! ```text
//! registry.find(&CapabilityQuery {
//!     required: caps![BarrierCrossing],
//!     backend: Some(Backend::SparseBitSlice),
//!     integral: Some(true), ..default
//! })  ->  ["houdayer_icm", "history_field", "extremal_opt"]
//! ```
//!
//! This is where operator choice becomes a search over properties. As the
//! operator library grows (Blueprint Step 5), nothing here changes: each new
//! operator publishes a descriptor and is immediately selectable.

use super::capability::{CapabilityQuery, OperatorDescriptor};
use super::operator::Operator;
use std::collections::HashMap;

type Factory = Box<dyn Fn() -> Box<dyn Operator> + Send + Sync>;

struct Entry {
    descriptor: OperatorDescriptor,
    factory: Factory,
}

#[derive(Default)]
pub struct OperatorRegistry {
    entries: HashMap<&'static str, Entry>,
}

impl OperatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// The standard registry — the full operator library. The Decision and
    /// Evolution engines select among these by CAPABILITY, never by name.
    pub fn standard() -> Self {
        use super::operators as ops;
        let mut reg = Self::new();
        reg.register(|| Box::new(ops::GibbsColorSweep::new()));
        reg.register(|| Box::new(ops::MetropolisSweep::new()));
        reg.register(|| Box::new(ops::GreedyDescent::new()));
        reg.register(|| Box::new(ops::SteepestDescent::new()));
        reg.register(|| Box::new(ops::RandomFlipSweep::new()));
        reg.register(|| Box::new(ops::ExtremalOptimization::new()));
        reg.register(|| Box::new(ops::ExtremalMetropolis::new()));
        reg.register(|| Box::new(ops::ReplicaExchange::new()));
        reg.register(|| Box::new(ops::HoudayerClusterMove::new()));
        reg.register(|| Box::new(ops::IsoenergeticClusterMove::new()));
        reg.register(|| Box::new(ops::PopulationResample::new()));
        reg.register(|| Box::new(ops::EliteBroadcast::new()));
        reg.register(|| Box::new(ops::RandomRestartWorst::new()));
        reg.register(|| Box::new(ops::HistoryFieldSweep::new()));
        reg
    }

    /// Register an operator by factory. The descriptor is read once, up front.
    pub fn register<F>(&mut self, factory: F)
    where
        F: Fn() -> Box<dyn Operator> + Send + Sync + 'static,
    {
        let descriptor = factory().descriptor();
        self.entries.insert(
            descriptor.name,
            Entry {
                descriptor,
                factory: Box::new(factory),
            },
        );
    }

    /// Construct a fresh instance of `name`, if registered.
    pub fn lookup(&self, name: &str) -> Option<Box<dyn Operator>> {
        self.entries.get(name).map(|e| (e.factory)())
    }

    /// Metadata for `name` without instantiating.
    pub fn metadata(&self, name: &str) -> Option<&OperatorDescriptor> {
        self.entries.get(name).map(|e| &e.descriptor)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &&'static str> {
        self.entries.keys()
    }

    /// All operators whose descriptor satisfies the query, sorted by name for
    /// deterministic selection (ADR-0004).
    pub fn find(&self, query: &CapabilityQuery) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self
            .entries
            .values()
            .filter(|e| query.matches(&e.descriptor))
            .map(|e| e.descriptor.name)
            .collect();
        out.sort_unstable();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::capability::{
        Capability, CapabilitySet, Complexity, Constraints, Guarantees, ObservableSet,
        OperatorDescriptor,
    };
    use super::super::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
    use super::super::runtime::RuntimeView;
    use super::super::state::SpinState;
    use super::*;
    use rand_chacha::ChaCha8Rng;

    struct Mock {
        name: &'static str,
        caps: CapabilitySet,
        cons: Constraints,
    }

    impl Operator for Mock {
        fn descriptor(&self) -> OperatorDescriptor {
            OperatorDescriptor {
                name: self.name,
                nature: Nature::TrajectoryPreserving,
                capabilities: self.caps,
                constraints: self.cons,
                observables: ObservableSet::empty(),
                guarantees: Guarantees::none(),
                complexity: Complexity::LinearEdges,
            }
        }
        fn cost_model(&self, _: InstanceShape) -> CostEstimate {
            CostEstimate {
                work_per_sweep: 1.0,
            }
        }
        fn apply(
            &mut self,
            _: &mut dyn SpinState,
            _: &RuntimeView,
            _: &mut ChaCha8Rng,
            _: Budget,
        ) -> Report {
            Report::default()
        }
    }

    #[test]
    fn find_by_capability_selects_by_property_not_name() {
        let mut reg = OperatorRegistry::new();
        reg.register(|| {
            Box::new(Mock {
                name: "explorer",
                caps: CapabilitySet::empty().with(Capability::Exploration),
                cons: Constraints::any(),
            })
        });
        reg.register(|| {
            Box::new(Mock {
                name: "barrier_int",
                caps: CapabilitySet::empty().with(Capability::BarrierCrossing),
                cons: Constraints {
                    needs_integer: true,
                    ..Constraints::any()
                },
            })
        });

        let q = CapabilityQuery {
            required: CapabilitySet::empty().with(Capability::BarrierCrossing),
            integral: Some(true),
            ..Default::default()
        };
        assert_eq!(reg.find(&q), vec!["barrier_int"]);

        // On a float instance the integer-only barrier operator drops out.
        let q_float = CapabilityQuery {
            integral: Some(false),
            ..q
        };
        assert!(reg.find(&q_float).is_empty());

        assert!(reg.lookup("explorer").is_some());
        assert_eq!(reg.metadata("barrier_int").unwrap().name, "barrier_int");
    }
}
