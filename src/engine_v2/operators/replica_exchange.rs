//! `ReplicaExchange` — the parallel-tempering swap. Configurations move BETWEEN
//! temperatures: adjacent replicas on the ladder attempt to exchange configs
//! with the standard PT acceptance min(1, exp((β_i − β_j)(E_i − E_j))). A config
//! that gets stuck in a low-T basin can be lifted to a hotter replica, escape,
//! and come back cold — barrier crossing at the ENSEMBLE level, which no single
//! local operator provides.
//!
//! Reads the per-replica temperature ladder from the `RuntimeView`; needs ≥2
//! replicas. Deterministic (one RNG draw per attempted pair, fixed order) and
//! bit-identical across backends (swap via the verified `swap_replicas`).

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::SpinState;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

#[derive(Default)]
pub struct ReplicaExchange {
    energies: Vec<f64>,
    ready: bool,
}

impl ReplicaExchange {
    pub const NAME: &'static str = "replica_exchange";
    pub fn new() -> Self {
        Self::default()
    }
}

impl Operator for ReplicaExchange {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty().with(Capability::BarrierCrossing),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: true,
                needs_temperature: true,
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
        view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        let r = state.num_replicas();
        if r < 2 || view.temperatures.is_empty() {
            return Report::default();
        }
        if !self.ready {
            self.energies = vec![0.0; r];
            self.ready = true;
        }
        let beta = |rep: usize| {
            let t = view.temperatures[rep % view.temperatures.len()];
            if t > 0.0 {
                1.0 / t
            } else {
                f64::INFINITY
            }
        };
        let mut report = Report::default();
        for round in 0..budget.sweeps {
            state.energies_into(&mut self.energies);
            // Alternate even/odd adjacent pairings so every replica can drift.
            let start = (round as usize) & 1;
            let mut k = start;
            while k + 1 < r {
                let (i, j) = (k, k + 1);
                let (bi, bj) = (beta(i), beta(j));
                let delta = (bi - bj) * (self.energies[i] - self.energies[j]);
                report.proposed += 1;
                let u: f64 = rng.gen();
                // Accept if delta >= 0 (always) or u < exp(delta).
                let accept = delta >= 0.0 || u < delta.exp();
                if accept {
                    state.swap_replicas(i, j);
                    // Keep our energy snapshot consistent for later pairs.
                    self.energies.swap(i, j);
                    report.accepted += 1;
                }
                k += 2;
            }
        }
        report.work = (budget.sweeps as f64) * (state.num_vars() as f64) * (r as f64);
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_v2::backends::{ReferenceState, SparseBitSlice};
    use crate::engine_v2::ir::ProblemIR;
    use crate::engine_v2::operators::GibbsColorSweep;
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
    fn swaps_preserve_energy_multiset_and_match_reference() {
        let ir = ir7();
        let r = 16;
        let init = [1u8, 0, 1, 0, 0, 1, 0];
        let temps: Vec<f64> = (0..r).map(|k| 0.1 + 0.3 * k as f64).collect();
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();

        // Diverge replicas with a shared Gibbs pass so energies differ.
        let gview = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        GibbsColorSweep::new().apply(
            &mut refs,
            &gview,
            &mut ChaCha8Rng::seed_from_u64(1),
            Budget { sweeps: 5 },
        );
        GibbsColorSweep::new().apply(
            &mut bs,
            &gview,
            &mut ChaCha8Rng::seed_from_u64(1),
            Budget { sweeps: 5 },
        );
        let mut before: Vec<f64> = vec![0.0; r];
        refs.energies_into(&mut before);
        let mut sorted_before = before.clone();
        sorted_before.sort_by(|a, b| a.total_cmp(b));

        ReplicaExchange::new().apply(
            &mut refs,
            &gview,
            &mut ChaCha8Rng::seed_from_u64(9),
            Budget { sweeps: 6 },
        );
        ReplicaExchange::new().apply(
            &mut bs,
            &gview,
            &mut ChaCha8Rng::seed_from_u64(9),
            Budget { sweeps: 6 },
        );

        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs, "exchange diverged between backends");
        // Exchanges only permute configs → the multiset of energies is invariant.
        let mut sorted_after = e_ref.clone();
        sorted_after.sort_by(|a, b| a.total_cmp(b));
        assert_eq!(sorted_before, sorted_after);
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
    }
}
