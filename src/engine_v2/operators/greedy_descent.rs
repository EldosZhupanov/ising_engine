//! `GreedyDescent` — deterministic monotone local search (the "quench").
//!
//! For each site it flips exactly the replicas whose ΔE is strictly negative.
//! No temperature, no randomness: every accepted flip lowers that replica's
//! energy, so a sweep is monotone non-increasing per replica and the operator
//! carries a `monotone` guarantee. It is the exploitation counterpart to the
//! thermal operators — an evolved plan typically ends with it to quench the
//! ensemble into the nearest minimum after exploration.
//!
//! Distinct passport (Exploitation only, no `needs_temperature`, `monotone`)
//! lets the Decision Engine and Evolution Engine reason about it by CAPABILITY:
//! "give me a monotone exploitation operator" resolves here without naming it.

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand_chacha::ChaCha8Rng;

#[derive(Default)]
pub struct GreedyDescent {
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl GreedyDescent {
    pub const NAME: &'static str = "greedy_descent";

    pub fn new() -> Self {
        Self::default()
    }

    fn ensure(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        self.de = vec![0.0; state.num_replicas()];
        self.mask = ReplicaMask::new(state.num_replicas());
        self.ready = true;
    }
}

impl Operator for GreedyDescent {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty().with(Capability::Exploitation),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: false,
                needs_temperature: false,
            },
            observables: ObservableSet::empty()
                .with(Observable::Acceptance)
                .with(Observable::EnergyDelta),
            guarantees: Guarantees {
                monotone: true,
                deterministic: true,
                ..Guarantees::none()
            },
            complexity: Complexity::LinearEdges,
        }
    }

    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        CostEstimate {
            work_per_sweep: ((shape.n + 2 * shape.num_pairs) * shape.num_replicas.max(1)) as f64,
        }
    }

    fn apply(
        &mut self,
        state: &mut dyn SpinState,
        _view: &RuntimeView,
        _rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        self.ensure(state);
        let n = state.num_vars();
        let r = state.num_replicas();
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            for site in 0..n {
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                for rep in 0..r {
                    report.proposed += 1;
                    if self.de[rep] < 0.0 {
                        self.mask.set(rep);
                        report.accepted += 1;
                    }
                }
                if !self.mask.is_empty() {
                    state.apply_flips(site, &self.mask);
                }
            }
        }
        report.work = self
            .cost_model(InstanceShape {
                n,
                num_pairs: 0,
                num_replicas: r,
            })
            .work_per_sweep
            * budget.sweeps as f64;
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_v2::backends::{ReferenceState, SparseBitSlice};
    use crate::engine_v2::ir::ProblemIR;
    use rand::SeedableRng;

    fn frustrated() -> ProblemIR {
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

    fn view(r: usize) -> RuntimeView<'static> {
        RuntimeView {
            iteration: 0,
            temperatures: &[],
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        }
    }

    #[test]
    fn monotone_and_identical_on_both_backends() {
        let ir = frustrated();
        let r = 64;
        let init = [1u8, 1, 0, 1, 0, 1, 1];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();

        let mut before = vec![0.0; r];
        refs.energies_into(&mut before);

        let mut rng = ChaCha8Rng::seed_from_u64(0);
        GreedyDescent::new().apply(&mut refs, &view(r), &mut rng, Budget { sweeps: 10 });
        GreedyDescent::new().apply(&mut bs, &view(r), &mut rng, Budget { sweeps: 10 });

        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs, "greedy diverged between backends");
        for rep in 0..r {
            assert!(e_ref[rep] <= before[rep] + 1e-9, "greedy raised energy");
        }
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
    }
}
