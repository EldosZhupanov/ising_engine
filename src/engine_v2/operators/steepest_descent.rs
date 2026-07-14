//! `SteepestDescent` — greedy that flips only the single BEST-improving spin per
//! replica each step (vs `GreedyDescent`, which flips every improving spin at
//! once). One "sweep" is one steepest step (one flip per replica that still has
//! a downhill move). Strictly monotone per replica, deterministic (no RNG).

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand_chacha::ChaCha8Rng;

#[derive(Default)]
pub struct SteepestDescent {
    de: Vec<f64>,
    best_de: Vec<f64>,
    best_site: Vec<usize>,
    mask: ReplicaMask,
    ready: bool,
}

impl SteepestDescent {
    pub const NAME: &'static str = "steepest_descent";
    pub fn new() -> Self {
        Self::default()
    }
    fn ensure(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let r = state.num_replicas();
        self.de = vec![0.0; r];
        self.best_de = vec![0.0; r];
        self.best_site = vec![0; r];
        self.mask = ReplicaMask::new(r);
        self.ready = true;
    }
}

impl Operator for SteepestDescent {
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
            work_per_sweep: (shape.n * shape.num_replicas.max(1)) as f64,
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
        let (n, r) = (state.num_vars(), state.num_replicas());
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            // Find each replica's most-improving site.
            for b in self.best_de.iter_mut() {
                *b = 0.0; // only strictly-negative moves qualify
            }
            for s in self.best_site.iter_mut() {
                *s = usize::MAX;
            }
            for site in 0..n {
                state.delta_e_into(site, &mut self.de);
                for rep in 0..r {
                    if self.de[rep] < self.best_de[rep] {
                        self.best_de[rep] = self.de[rep];
                        self.best_site[rep] = site;
                    }
                }
            }
            // Apply one flip per replica, grouped by chosen site.
            let mut any = false;
            for site in 0..n {
                self.mask.clear();
                let mut hit = false;
                for rep in 0..r {
                    report.proposed += 1;
                    if self.best_site[rep] == site {
                        self.mask.set(rep);
                        report.accepted += 1;
                        hit = true;
                    }
                }
                if hit {
                    state.apply_flips(site, &self.mask);
                    any = true;
                }
            }
            if !any {
                break; // all replicas at a local minimum
            }
        }
        report.work = (budget.sweeps as f64) * (n as f64) * (r as f64);
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_v2::backends::{ReferenceState, SparseBitSlice};
    use crate::engine_v2::ir::ProblemIR;
    use rand::SeedableRng;

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

    #[test]
    fn monotone_and_identical() {
        let ir = ir7();
        let r = 64;
        let init = [1u8, 1, 0, 1, 0, 1, 1];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        let mut before = vec![0.0; r];
        refs.energies_into(&mut before);
        let view = RuntimeView {
            iteration: 0,
            temperatures: &[],
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        SteepestDescent::new().apply(
            &mut refs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(0),
            Budget { sweeps: 20 },
        );
        SteepestDescent::new().apply(
            &mut bs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(0),
            Budget { sweeps: 20 },
        );
        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs);
        for rep in 0..r {
            assert!(e_ref[rep] <= before[rep] + 1e-9);
        }
        assert_eq!(bs.audit(), 0.0);
    }
}
