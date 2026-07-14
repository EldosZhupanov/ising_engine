//! Population operators — they act on the ENSEMBLE, not a single chain.
//!
//!  - `PopulationResample`: the core of Population Annealing. Replicas are
//!    resampled with Boltzmann weight exp(−β·ΔE) at the reference temperature
//!    (systematic resampling): low-energy configs are duplicated, high-energy
//!    ones die. Concentrates the population on promising basins.
//!  - `EliteBroadcast`: copy the best replica over the worst quarter (elitism).
//!  - `RandomRestartWorst`: re-randomize the worst quarter (diversity injection).
//!
//! All use the verified `copy_replica`/`apply_flips`, so ledgers stay exact and
//! results are bit-identical across backends. Deterministic given the seed.

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

fn energies_sorted_desc(state: &dyn SpinState, buf: &mut Vec<f64>) -> Vec<usize> {
    let r = state.num_replicas();
    buf.resize(r, 0.0);
    state.energies_into(buf);
    let mut idx: Vec<usize> = (0..r).collect();
    let e = &buf;
    idx.sort_by(|&a, &b| e[b].total_cmp(&e[a]).then(a.cmp(&b))); // worst (highest E) first
    idx
}

// -------------------------------------------------------- PopulationResample --

#[derive(Default)]
pub struct PopulationResample {
    energies: Vec<f64>,
    ready: bool,
}

impl PopulationResample {
    pub const NAME: &'static str = "population_resample";
    pub fn new() -> Self {
        Self::default()
    }
}

impl Operator for PopulationResample {
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
                needs_replicas: true,
                needs_temperature: true,
            },
            observables: ObservableSet::empty().with(Observable::Diversity),
            guarantees: Guarantees::deterministic(),
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
        let beta = {
            let t = view.temperatures[0];
            if t > 0.0 {
                1.0 / t
            } else {
                1e6
            }
        };
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            state.energies_into(&mut self.energies);
            let emin = self.energies.iter().copied().fold(f64::INFINITY, f64::min);
            // Relative Boltzmann weights (overflow-safe).
            let weights: Vec<f64> = self
                .energies
                .iter()
                .map(|&e| (-beta * (e - emin)).exp())
                .collect();
            let total: f64 = weights.iter().sum();
            if total <= 0.0 {
                continue;
            }
            // Systematic resampling → per-source copy counts.
            let u0: f64 = rng.gen();
            let mut counts = vec![0u32; r];
            let mut cum = 0.0;
            let mut src = 0usize;
            for k in 0..r {
                let pos = (k as f64 + u0) / r as f64 * total;
                while src + 1 < r && cum + weights[src] < pos {
                    cum += weights[src];
                    src += 1;
                }
                counts[src] += 1;
            }
            // Survivors keep their slot; extra copies fill the dead slots.
            let mut dead: Vec<usize> = (0..r).filter(|&i| counts[i] == 0).collect();
            for (source, &count) in counts.iter().enumerate() {
                for _ in 1..count {
                    if let Some(slot) = dead.pop() {
                        state.copy_replica(source, slot);
                        report.accepted += 1;
                    }
                }
                report.proposed += 1;
            }
        }
        report.work = (budget.sweeps as f64) * (state.num_vars() as f64) * (r as f64);
        report
    }
}

// ------------------------------------------------------------- EliteBroadcast --

#[derive(Default)]
pub struct EliteBroadcast {
    buf: Vec<f64>,
}

impl EliteBroadcast {
    pub const NAME: &'static str = "elite_broadcast";
    pub fn new() -> Self {
        Self::default()
    }
}

impl Operator for EliteBroadcast {
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
                needs_replicas: true,
                needs_temperature: false,
            },
            observables: ObservableSet::empty(),
            guarantees: Guarantees::deterministic(),
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
        let r = state.num_replicas();
        if r < 2 {
            return Report::default();
        }
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            let ranked = energies_sorted_desc(state, &mut self.buf); // worst first
            let best = ranked[r - 1];
            let k = (r / 4).max(1);
            for &worst in ranked.iter().take(k) {
                if worst != best {
                    state.copy_replica(best, worst);
                    report.accepted += 1;
                }
                report.proposed += 1;
            }
        }
        report.work = (budget.sweeps as f64) * (state.num_vars() as f64) * (r as f64);
        report
    }
}

// --------------------------------------------------------- RandomRestartWorst --

#[derive(Default)]
pub struct RandomRestartWorst {
    buf: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl RandomRestartWorst {
    pub const NAME: &'static str = "random_restart_worst";
    pub fn new() -> Self {
        Self::default()
    }
}

impl Operator for RandomRestartWorst {
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
                needs_replicas: true,
                needs_temperature: false,
            },
            observables: ObservableSet::empty().with(Observable::Diversity),
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
        if r < 2 {
            return Report::default();
        }
        if !self.ready {
            self.mask = ReplicaMask::new(r);
            self.ready = true;
        }
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            let ranked = energies_sorted_desc(state, &mut self.buf);
            let k = (r / 4).max(1);
            // Re-randomize the worst k replicas: each site flips w.p. 1/2.
            for site in 0..n {
                self.mask.clear();
                let mut any = false;
                for &worst in ranked.iter().take(k) {
                    report.proposed += 1;
                    if rng.gen::<bool>() {
                        self.mask.set(worst);
                        report.accepted += 1;
                        any = true;
                    }
                }
                if any {
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
    use crate::engine_v2::operators::GibbsColorSweep;
    use rand::SeedableRng;

    fn ir8() -> ProblemIR {
        ProblemIR::from_pairs(
            8,
            0.0,
            vec![1.0, -1.0, 2.0, 0.0, -2.0, 1.0, -1.0, 2.0],
            &[
                (0, 1, -1.0),
                (1, 2, 1.0),
                (2, 3, -1.0),
                (3, 4, 1.0),
                (4, 5, -1.0),
                (5, 6, 1.0),
                (6, 7, -1.0),
                (0, 4, 1.0),
            ],
        )
    }

    fn diverge(refs: &mut ReferenceState, bs: &mut SparseBitSlice, r: usize) {
        let temps: Vec<f64> = (0..r).map(|k| 0.3 + 0.2 * k as f64).collect();
        let v = RuntimeView {
            iteration: 0,
            temperatures: &temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        GibbsColorSweep::new().apply(
            refs,
            &v,
            &mut ChaCha8Rng::seed_from_u64(1),
            Budget { sweeps: 6 },
        );
        GibbsColorSweep::new().apply(
            bs,
            &v,
            &mut ChaCha8Rng::seed_from_u64(1),
            Budget { sweeps: 6 },
        );
    }

    fn tview(r: usize, temps: &[f64]) -> RuntimeView<'_> {
        RuntimeView {
            iteration: 0,
            temperatures: temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        }
    }

    #[test]
    fn resample_identical_and_exact() {
        static T: [f64; 1] = [1.0];
        let ir = ir8();
        let r = 16;
        let init = vec![0u8; 8];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        diverge(&mut refs, &mut bs, r);
        PopulationResample::new().apply(
            &mut refs,
            &tview(r, &T),
            &mut ChaCha8Rng::seed_from_u64(5),
            Budget { sweeps: 3 },
        );
        PopulationResample::new().apply(
            &mut bs,
            &tview(r, &T),
            &mut ChaCha8Rng::seed_from_u64(5),
            Budget { sweeps: 3 },
        );
        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs);
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
    }

    #[test]
    fn elite_and_restart_identical_and_exact() {
        let ir = ir8();
        let r = 16;
        let init = vec![0u8; 8];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        diverge(&mut refs, &mut bs, r);

        let empty: [f64; 0] = [];
        EliteBroadcast::new().apply(
            &mut refs,
            &tview(r, &empty),
            &mut ChaCha8Rng::seed_from_u64(0),
            Budget { sweeps: 2 },
        );
        EliteBroadcast::new().apply(
            &mut bs,
            &tview(r, &empty),
            &mut ChaCha8Rng::seed_from_u64(0),
            Budget { sweeps: 2 },
        );
        RandomRestartWorst::new().apply(
            &mut refs,
            &tview(r, &empty),
            &mut ChaCha8Rng::seed_from_u64(6),
            Budget { sweeps: 2 },
        );
        RandomRestartWorst::new().apply(
            &mut bs,
            &tview(r, &empty),
            &mut ChaCha8Rng::seed_from_u64(6),
            Budget { sweeps: 2 },
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
