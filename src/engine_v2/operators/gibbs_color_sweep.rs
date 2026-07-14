//! `GibbsColorSweep` — the first real operator (Constitution §8, ADR-0001).
//!
//! ONE physical law: the heat-bath (Gibbs) local update, applied over the
//! interaction graph one color class at a time. For a QUBO variable x_i with
//! local field h_i = linear_i + Σ_j q_ij·x_j, the heat-bath resamples
//!
//! ```text
//! P(x_i = 1) = 1 / (1 + exp(β·h_i)),     β = 1/T,
//! ```
//!
//! independent of the current value of x_i. Sites of one color form an
//! independent set (`SpinState::coloring`), so every site in a class can be
//! resampled against the SAME frozen neighborhood — a valid synchronous
//! (chromatic) Gibbs sweep. Across all colors this is one full lattice sweep.
//!
//! The operator is PURE: it never reads wall-clock, never knows what runs next,
//! never touches the coupling weights. It reads only the per-replica
//! temperature the Runtime hands it in `RuntimeView`, applies the law, and
//! returns statistics. Deterministic given the seeded RNG: draws proceed in a
//! fixed (color-order site, then replica) order, so runs replay bit-identically
//! and the two backends produce identical trajectories (ADR-0004).

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Heat-bath sweeps in chromatic order. Holds only the cached coloring-derived
/// visitation order — no problem data, no schedule, no time.
#[derive(Default)]
pub struct GibbsColorSweep {
    /// Site indices grouped by color (built once from `state.coloring()`).
    order: Vec<usize>,
    /// Reusable scratch: ΔE and per-replica flip mask.
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl GibbsColorSweep {
    pub const NAME: &'static str = "gibbs_color_sweep";

    pub fn new() -> Self {
        Self::default()
    }

    /// Build the chromatic visitation order once. Sites are visited grouped by
    /// color; within a color they are in ascending index order. Both are
    /// deterministic, so the RNG draw order is fixed across runs and backends.
    fn ensure_order(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let n = state.num_vars();
        let colors = state.coloring();
        let ncolors = colors.iter().copied().max().map_or(0, |c| c as usize + 1);
        // Stable bucket-by-color: ascending color, ascending site within color.
        let mut order = Vec::with_capacity(n);
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

impl Operator for GibbsColorSweep {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            // Thermal single-spin dynamics: explores at high T, exploits at low
            // T. It does NOT cross barriers a single flip cannot (that is ICM /
            // Houdayer / Extremal, added later).
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
            // Detailed balance ⇒ equilibrates; seeded ⇒ deterministic. It is not
            // monotone (accepts uphill moves) and does not preserve energy.
            guarantees: Guarantees {
                equilibrates: true,
                deterministic: true,
                ..Guarantees::none()
            },
            complexity: Complexity::LinearEdges,
        }
    }

    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        // One sweep touches every site once and rescans each edge for the field
        // update, across all replicas.
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
        debug_assert!(!temps.is_empty(), "GibbsColorSweep needs a temperature");

        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            for &site in &self.order {
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                for rep in 0..r {
                    // Recover the local field h from ΔE = (1 − 2·x)·h.
                    let x = state.spin(site, rep);
                    let h = if x { -self.de[rep] } else { self.de[rep] };
                    // β·h with T = 0 handled as the greedy (β → ∞) limit.
                    let t = temps[rep % temps.len()];
                    let bh = if t > 0.0 {
                        h / t
                    } else if h > 0.0 {
                        f64::INFINITY
                    } else if h < 0.0 {
                        f64::NEG_INFINITY
                    } else {
                        0.0
                    };
                    let p1 = 1.0 / (1.0 + bh.exp()); // P(new spin = 1)
                    let u: f64 = rng.gen(); // one draw per (site, replica) — fixed order
                    let new_one = u < p1;
                    report.proposed += 1;
                    if new_one != x {
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
        // Integral so BOTH backends are eligible; frustrated so temperature
        // actually matters.
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

    /// Same seed + same law on both backends ⇒ bit-identical trajectory
    /// (ADR-0004). This ties the operator to the cross-validated substrate.
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

        let mut op_a = GibbsColorSweep::new();
        let mut op_b = GibbsColorSweep::new();
        let rep_a = op_a.apply(&mut refs, &view, &mut ChaCha8Rng::seed_from_u64(7), budget);
        let rep_b = op_b.apply(&mut bs, &view, &mut ChaCha8Rng::seed_from_u64(7), budget);

        assert_eq!(
            rep_a.accepted, rep_b.accepted,
            "acceptance must match exactly"
        );
        assert_eq!(rep_a.proposed, rep_b.proposed);

        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs, "energy trajectories diverged between backends");
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);

        // Extracted configs agree replica-by-replica, and the ledger matches the
        // canonical scorer.
        for (rep, &e) in e_bs.iter().enumerate() {
            let mut xr = vec![0u8; ir.n];
            let mut xb = vec![0u8; ir.n];
            refs.extract_into(rep, &mut xr);
            bs.extract_into(rep, &mut xb);
            assert_eq!(xr, xb);
            assert_eq!(ir.energy(&xb), e);
        }
    }

    /// At T → 0 the heat-bath becomes greedy descent: energy is non-increasing
    /// over a sweep for every replica.
    #[test]
    fn cold_temperature_is_monotone_descent() {
        let ir = frustrated();
        let r = 8;
        let init = [0u8; 7];
        let temps = vec![1e-9; r]; // effectively zero → greedy
        let mut st = ReferenceState::new(&ir, r, &init);
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };

        let mut before = vec![0.0; r];
        st.energies_into(&mut before);
        let mut op = GibbsColorSweep::new();
        op.apply(
            &mut st,
            &view,
            &mut ChaCha8Rng::seed_from_u64(1),
            Budget { sweeps: 5 },
        );
        let mut after = vec![0.0; r];
        st.energies_into(&mut after);
        for rep in 0..r {
            assert!(
                after[rep] <= before[rep] + 1e-9,
                "cold Gibbs raised energy: {} -> {}",
                before[rep],
                after[rep]
            );
        }
        assert_eq!(st.audit(), 0.0);
    }
}
