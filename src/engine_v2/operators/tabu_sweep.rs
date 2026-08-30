//! `TabuSweep` — hard prohibition memory with an aspiration criterion.
//!
//! RC-022 found that memory beyond the configuration is an occupied cell in
//! both corpora, and that we had entered it once, from physics:
//! [`super::history_field::HistoryFieldSweep`] adds a decaying penalty to ΔE, so
//! a recently-flipped site becomes temporarily *costly*. Seven of MQLib's twenty
//! families entered the same cell from operations research instead, where a
//! recently-flipped site becomes temporarily *forbidden*, and the prohibition is
//! bought back only by an **aspiration criterion**: a tabu move is allowed after
//! all if it would beat the best solution that replica has seen.
//!
//! This operator is deliberately `history_field`'s twin. The sweep frame, the
//! Metropolis acceptance on the **real** ΔE, the unconditional RNG draw on both
//! branches, the `Report` accounting and the untouched canonical ledger are all
//! identical. The only difference is the form of the memory — which is what
//! PREREG RC-023 exists to test.
//!
//! `BehaviorChanging`, for the same reason `history_field` is: the reachable
//! move set depends on history, so this is not a pure physical law. The REPORTED
//! energy is always the canonical ledger energy — the tenure never enters the
//! score — and the operator is deterministic given the seed, so it stays
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
pub struct TabuSweep {
    /// Sweep index until which (site, replica) is forbidden, row-major by site.
    /// Zero means never visited, so nothing is tabu at the start.
    until: Vec<u32>,
    /// Best canonical energy each replica has reached, for the aspiration test.
    best: Vec<f64>,
    /// Current canonical energy per replica, tracked incrementally.
    cur: Vec<f64>,
    de: Vec<f64>,
    mask: ReplicaMask,
    sweep: u32,
    ready: bool,
}

impl TabuSweep {
    pub const NAME: &'static str = "tabu_sweep";
    /// Sweeps for which a flipped site stays forbidden. **Fixed by PREREG
    /// RC-023 §3 and deliberately untuned**: a tenure chosen after seeing
    /// results would make this a search for a winning configuration rather
    /// than a test of a mechanism.
    const TENURE: u32 = 10;

    pub fn new() -> Self {
        Self::default()
    }

    fn ensure(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let (n, r) = (state.num_vars(), state.num_replicas());
        self.until = vec![0; n * r];
        self.de = vec![0.0; r];
        self.cur = vec![0.0; r];
        state.energies_into(&mut self.cur);
        self.best = self.cur.clone();
        self.mask = ReplicaMask::new(r);
        self.sweep = 0;
        self.ready = true;
    }
}

impl Operator for TabuSweep {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            // A history-dependent move set is not a stationary physical law.
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
        debug_assert!(!temps.is_empty(), "TabuSweep needs a temperature");

        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            self.sweep += 1;
            for site in 0..n {
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                let base = site * r;
                for rep in 0..r {
                    let d = self.de[rep];
                    let t = temps[rep % temps.len()];
                    // The acceptance is history_field's, on the real ΔE. The RNG
                    // is drawn on both branches so the stream stays aligned.
                    let accept = if d <= 0.0 {
                        let _u: f64 = rng.gen();
                        true
                    } else {
                        let p = if t > 0.0 { (-d / t).exp() } else { 0.0 };
                        let u: f64 = rng.gen();
                        u < p
                    };
                    report.proposed += 1;
                    if !accept {
                        continue;
                    }
                    // The one difference: prohibition, and the aspiration that
                    // buys it back. A forbidden move is allowed only if taking
                    // it would beat the best this replica has reached.
                    let forbidden = self.until[base + rep] > self.sweep;
                    let aspires = self.cur[rep] + d < self.best[rep];
                    if forbidden && !aspires {
                        continue;
                    }
                    self.mask.set(rep);
                    self.until[base + rep] = self.sweep + Self::TENURE;
                    self.cur[rep] += d;
                    if self.cur[rep] < self.best[rep] {
                        self.best[rep] = self.cur[rep];
                    }
                    report.accepted += 1;
                }
                if !self.mask.is_empty() {
                    state.apply_flips(site, &self.mask);
                }
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

    fn fixture() -> ProblemIR {
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
    fn identical_and_ledger_exact() {
        let ir = fixture();
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
        TabuSweep::new().apply(
            &mut refs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(3),
            Budget { sweeps: 20 },
        );
        TabuSweep::new().apply(
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

    /// The aspiration criterion reads `cur`, which is tracked incrementally
    /// rather than recomputed. If that tracking drifted from the ledger the
    /// prohibition would be bought back on a fiction, so the drift is checked
    /// against the canonical energy the backend reports.
    #[test]
    fn tracked_energy_never_drifts_from_the_ledger() {
        let ir = fixture();
        let r = 16;
        let init = [0u8, 1, 1, 0, 1, 0, 0];
        let temps: Vec<f64> = (0..r).map(|k| 0.2 + 0.2 * k as f64).collect();
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let mut st = ReferenceState::new(&ir, r, &init);
        let mut op = TabuSweep::new();
        let mut canonical = vec![0.0; r];
        for _ in 0..25 {
            op.apply(
                &mut st,
                &view,
                &mut ChaCha8Rng::seed_from_u64(11),
                Budget { sweeps: 1 },
            );
            st.energies_into(&mut canonical);
            for (rep, &ledger) in canonical.iter().enumerate() {
                assert!(
                    (op.cur[rep] - ledger).abs() < 1e-9,
                    "replica {rep}: tracked {} vs ledger {ledger}",
                    op.cur[rep]
                );
                assert!(op.best[rep] <= ledger + 1e-9);
            }
        }
    }

    /// RC-023 compares two operators that must see the *same* random numbers,
    /// so the only difference between them is the form of the memory. Both draw
    /// exactly once per proposal, on either acceptance branch; if this operator
    /// ever skipped a draw the streams would diverge and the comparison would
    /// silently become "different memory **and** different randomness".
    #[test]
    fn the_rng_stream_stays_aligned_with_the_twin() {
        let ir = fixture();
        let r = 8;
        let init = [1u8, 1, 0, 0, 1, 0, 1];
        let temps: Vec<f64> = (0..r).map(|k| 0.1 + 0.3 * k as f64).collect();
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let budget = Budget { sweeps: 9 };

        let mut st = ReferenceState::new(&ir, r, &init);
        let mut rng_tabu = ChaCha8Rng::seed_from_u64(29);
        TabuSweep::new().apply(&mut st, &view, &mut rng_tabu, budget);

        let mut st2 = ReferenceState::new(&ir, r, &init);
        let mut rng_twin = ChaCha8Rng::seed_from_u64(29);
        super::super::HistoryFieldSweep::new().apply(&mut st2, &view, &mut rng_twin, budget);

        assert_eq!(
            rng_tabu.gen::<u64>(),
            rng_twin.gen::<u64>(),
            "the twins consumed a different number of random draws"
        );
    }

    /// The mechanism under test must actually fire: with a tenure of 10 sweeps
    /// some accepted proposals have to be refused as tabu, otherwise the
    /// operator would silently be plain Metropolis and RC-023 would compare a
    /// twin against itself.
    #[test]
    fn prohibition_actually_refuses_accepted_moves() {
        let ir = fixture();
        let r = 8;
        let init = [0u8; 7];
        // Hot enough that Metropolis accepts almost everything, so any refusal
        // observed below is the tenure and not the temperature.
        let temps = vec![50.0; r];
        let view = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let mut st = ReferenceState::new(&ir, r, &init);
        let tabu = TabuSweep::new().apply(
            &mut st,
            &view,
            &mut ChaCha8Rng::seed_from_u64(7),
            Budget { sweeps: 12 },
        );
        let mut st2 = ReferenceState::new(&ir, r, &init);
        let free = super::super::HistoryFieldSweep::new().apply(
            &mut st2,
            &view,
            &mut ChaCha8Rng::seed_from_u64(7),
            Budget { sweeps: 12 },
        );
        assert_eq!(tabu.proposed, free.proposed, "the twins must propose alike");
        assert!(
            tabu.accepted < free.accepted,
            "prohibition never fired: {} accepted vs {}",
            tabu.accepted,
            free.accepted
        );
    }
}
