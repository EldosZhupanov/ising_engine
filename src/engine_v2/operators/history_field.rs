//! `HistoryFieldSweep` — a metadynamics-style history bias. Alongside the real
//! ΔE it maintains a per-(site, replica) history field that GROWS each time a
//! site is flipped and slowly decays. The acceptance uses ΔE plus this bias, so
//! recently-flipped sites become temporarily costly to touch again — the walk is
//! pushed to fill in basins and explore fresh moves instead of oscillating.
//!
//! This is a `BehaviorChanging` operator: the effective landscape is
//! history-dependent, so it is NOT a pure physical law and may only serve in
//! declared exploration roles. Crucially the REPORTED energy is always the
//! canonical ledger energy (the bias never enters the score), and the operator
//! stays deterministic given the seed — bias evolves identically on both
//! backends, so it remains bit-identical.

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
pub struct HistoryFieldSweep {
    /// History bias per (site, replica), row-major by site.
    bias: Vec<f64>,
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl HistoryFieldSweep {
    pub const NAME: &'static str = "history_field";
    /// Strength of the history penalty (in energy units).
    const GAMMA: f64 = 0.5;
    /// Per-sweep decay of the accumulated history.
    const DECAY: f64 = 0.98;

    pub fn new() -> Self {
        Self::default()
    }

    fn ensure(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let (n, r) = (state.num_vars(), state.num_replicas());
        self.bias = vec![0.0; n * r];
        self.de = vec![0.0; r];
        self.mask = ReplicaMask::new(r);
        self.ready = true;
    }
}

impl Operator for HistoryFieldSweep {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            // History bias makes the transition landscape non-stationary.
            nature: Nature::BehaviorChanging,
            capabilities: CapabilitySet::empty()
                .with(Capability::Exploration)
                .with(Capability::BarrierCrossing),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: false,
                needs_temperature: true,
            },
            observables: ObservableSet::empty().with(Observable::Acceptance),
            guarantees: Guarantees::deterministic(),
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
        view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        self.ensure(state);
        let (n, r) = (state.num_vars(), state.num_replicas());
        let temps = view.temperatures;
        debug_assert!(!temps.is_empty(), "HistoryFieldSweep needs a temperature");

        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            for site in 0..n {
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                let base = site * r;
                for rep in 0..r {
                    // Effective barrier = real ΔE + accumulated history penalty.
                    let d = self.de[rep] + Self::GAMMA * self.bias[base + rep];
                    let t = temps[rep % temps.len()];
                    let accept = if d <= 0.0 {
                        let _u: f64 = rng.gen();
                        true
                    } else {
                        let p = if t > 0.0 { (-d / t).exp() } else { 0.0 };
                        let u: f64 = rng.gen();
                        u < p
                    };
                    report.proposed += 1;
                    if accept {
                        self.mask.set(rep);
                        self.bias[base + rep] += 1.0; // ratchet: discourage re-flip
                        report.accepted += 1;
                    }
                }
                if !self.mask.is_empty() {
                    state.apply_flips(site, &self.mask);
                }
            }
            // Decay history so old moves stop mattering.
            for b in self.bias.iter_mut() {
                *b *= Self::DECAY;
            }
        }
        report.work = (budget.sweeps as f64) * ((n + 2 * self.de.len()) as f64) * (r as f64);
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
    fn identical_and_ledger_exact() {
        let ir = ProblemIR::from_pairs(
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
        );
        let r = 32;
        let init = [1u8, 0, 1, 0, 0, 1, 0];
        let temps: Vec<f64> = (0..r).map(|k| 0.3 + 0.1 * k as f64).collect();
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        HistoryFieldSweep::new().apply(
            &mut refs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(3),
            Budget { sweeps: 20 },
        );
        HistoryFieldSweep::new().apply(
            &mut bs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(3),
            Budget { sweeps: 20 },
        );
        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs);
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
        let mut x = vec![0u8; ir.n];
        bs.extract_into(5, &mut x);
        assert_eq!(ir.energy(&x), e_bs[5]);
    }
}
