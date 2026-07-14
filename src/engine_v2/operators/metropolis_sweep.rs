//! `MetropolisSweep` — single-spin Metropolis dynamics in chromatic order.
//!
//! A second thermal law, distinct from `GibbsColorSweep`: a flip of ΔE is
//! accepted with probability min(1, exp(−ΔE/T)) rather than resampled from the
//! conditional. Same detailed balance, different acceptance profile (Metropolis
//! rejects less at high T, more decisively at low T), so an evolved schedule may
//! prefer one over the other in different regimes — which is exactly the kind of
//! choice the Evolution Engine is meant to discover.
//!
//! Like every operator it is PURE (reads only the per-replica temperature from
//! the `RuntimeView`), chromatic (independent sites per color), and deterministic
//! given the seeded RNG (one draw per (site, replica), fixed order) — so it is
//! bit-identical on both backends (ADR-0004).

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
pub struct MetropolisSweep {
    order: Vec<usize>,
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl MetropolisSweep {
    pub const NAME: &'static str = "metropolis_sweep";

    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_order(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let colors = state.coloring();
        let ncolors = colors.iter().copied().max().map_or(0, |c| c as usize + 1);
        let mut order = Vec::with_capacity(state.num_vars());
        for c in 0..ncolors as u32 {
            for (site, &col) in colors.iter().enumerate() {
                if col == c {
                    order.push(site);
                }
            }
        }
        self.order = order;
        self.de = vec![0.0; state.num_replicas()];
        self.mask = ReplicaMask::new(state.num_replicas());
        self.ready = true;
    }
}

impl Operator for MetropolisSweep {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty()
                .with(Capability::Exploration)
                .with(Capability::Exploitation),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: false,
                needs_temperature: true,
            },
            observables: ObservableSet::empty()
                .with(Observable::Acceptance)
                .with(Observable::EnergyDelta),
            guarantees: Guarantees {
                equilibrates: true,
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
        view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        self.ensure_order(state);
        let r = state.num_replicas();
        let temps = view.temperatures;
        debug_assert!(!temps.is_empty(), "MetropolisSweep needs a temperature");

        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            for &site in &self.order {
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                for rep in 0..r {
                    let d = self.de[rep];
                    // Accept downhill unconditionally; uphill with exp(−ΔE/T).
                    // T = 0 ⇒ pure descent (uphill probability → 0).
                    let t = temps[rep % temps.len()];
                    let accept = if d <= 0.0 {
                        // still draw to keep the RNG stream aligned across configs
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
                n: self.order.len(),
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

    #[test]
    fn identical_on_both_backends() {
        let ir = frustrated();
        let r = 100;
        let init = [1u8, 0, 1, 0, 0, 1, 0];
        let temps: Vec<f64> = (0..r).map(|k| 0.2 + 0.05 * k as f64).collect();

        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let budget = Budget { sweeps: 30 };

        let a = MetropolisSweep::new().apply(
            &mut refs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(11),
            budget,
        );
        let b = MetropolisSweep::new().apply(
            &mut bs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(11),
            budget,
        );
        assert_eq!(a.accepted, b.accepted);
        assert_eq!(a.proposed, b.proposed);

        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs, "trajectories diverged between backends");
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
    }
}
