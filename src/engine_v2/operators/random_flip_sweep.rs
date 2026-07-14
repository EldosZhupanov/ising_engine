//! `RandomFlipSweep` — the infinite-temperature limit: each site in each replica
//! flips with probability 1/2, independent of energy. Pure exploration / a
//! diversifying kick; carries no temperature and no monotonicity. Deterministic
//! given the seeded RNG (one draw per (site, replica), fixed order), so it is
//! bit-identical across backends.

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

#[derive(Default)]
pub struct RandomFlipSweep {
    mask: ReplicaMask,
    ready: bool,
}

impl RandomFlipSweep {
    pub const NAME: &'static str = "random_flip_sweep";
    pub fn new() -> Self {
        Self::default()
    }
}

impl Operator for RandomFlipSweep {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty().with(Capability::Exploration),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: false,
                needs_temperature: false,
            },
            observables: ObservableSet::empty().with(Observable::Acceptance),
            guarantees: Guarantees::deterministic(),
            complexity: Complexity::LinearVars,
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
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        let (n, r) = (state.num_vars(), state.num_replicas());
        if !self.ready {
            self.mask = ReplicaMask::new(r);
            self.ready = true;
        }
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            for site in 0..n {
                self.mask.clear();
                for rep in 0..r {
                    report.proposed += 1;
                    if rng.gen::<bool>() {
                        self.mask.set(rep);
                        report.accepted += 1;
                    }
                }
                if !self.mask.is_empty() {
                    state.apply_flips(site, &self.mask);
                }
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

    #[test]
    fn identical_across_backends() {
        let ir = ProblemIR::from_pairs(
            6,
            0.0,
            vec![1.0, -1.0, 2.0, 0.0, -2.0, 1.0],
            &[
                (0, 1, -1.0),
                (1, 2, 1.0),
                (2, 3, -1.0),
                (3, 4, 1.0),
                (4, 5, -1.0),
            ],
        );
        let r = 70;
        let init = vec![0u8; 6];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        let view = RuntimeView {
            iteration: 0,
            temperatures: &[],
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        RandomFlipSweep::new().apply(
            &mut refs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(5),
            Budget { sweeps: 8 },
        );
        RandomFlipSweep::new().apply(
            &mut bs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(5),
            Budget { sweeps: 8 },
        );
        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs);
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
    }
}
