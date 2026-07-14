//! Cluster moves over replica pairs — Houdayer and the Isoenergetic Cluster Move
//! (ICM, Zhu–Ochoa–Katzgraber). Both look at two replicas, find the connected
//! components of the DISAGREEMENT graph (sites where the two replicas differ,
//! connected through interaction edges), and flip whole components in BOTH
//! replicas at once. A single move can reorganize a large correlated region —
//! crossing barriers that no local flip can.
//!
//!  - `HoudayerClusterMove`: flip ONE randomly chosen component per pair.
//!  - `IsoenergeticClusterMove`: flip EACH component independently with prob 1/2.
//!
//! Both need ≥2 replicas and read only graph TOPOLOGY (`neighbors`), never the
//! couplings. Deterministic (seeded RNG, fixed pairing/BFS order) ⇒ bit-identical
//! across backends (flips go through the verified `apply_flips`).

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Connected components of the subgraph induced by sites where replicas `a` and
/// `b` disagree. Deterministic: BFS from the lowest-index unvisited site, using
/// `neighbors` in CSR order. `visited` and `queue` are caller-owned scratch.
fn disagreement_components(
    state: &dyn SpinState,
    a: usize,
    b: usize,
    visited: &mut [bool],
    queue: &mut Vec<usize>,
) -> Vec<Vec<usize>> {
    let n = state.num_vars();
    for v in visited.iter_mut() {
        *v = false;
    }
    let differs = |site: usize| state.spin(site, a) != state.spin(site, b);
    let mut comps = Vec::new();
    for start in 0..n {
        if visited[start] || !differs(start) {
            continue;
        }
        let mut comp = Vec::new();
        queue.clear();
        queue.push(start);
        visited[start] = true;
        while let Some(site) = queue.pop() {
            comp.push(site);
            for &nb in state.neighbors(site) {
                let j = nb as usize;
                if !visited[j] && differs(j) {
                    visited[j] = true;
                    queue.push(j);
                }
            }
        }
        comps.push(comp);
    }
    comps
}

/// Flip every site of `comp` in replicas `a` and `b`.
fn flip_component(
    state: &mut dyn SpinState,
    comp: &[usize],
    a: usize,
    b: usize,
    mask: &mut ReplicaMask,
) {
    mask.clear();
    mask.set(a);
    mask.set(b);
    for &site in comp {
        state.apply_flips(site, mask);
    }
}

fn cluster_constraints() -> Constraints {
    Constraints {
        needs_integer: false,
        needs_float: false,
        supports_sparse: true,
        supports_dense: true,
        needs_replicas: true,
        needs_temperature: false,
    }
}

// ---------------------------------------------------------------- Houdayer ----

#[derive(Default)]
pub struct HoudayerClusterMove {
    visited: Vec<bool>,
    queue: Vec<usize>,
    mask: ReplicaMask,
    ready: bool,
}

impl HoudayerClusterMove {
    pub const NAME: &'static str = "houdayer_cluster";
    pub fn new() -> Self {
        Self::default()
    }
    fn ensure(&mut self, state: &dyn SpinState) {
        if !self.ready {
            self.visited = vec![false; state.num_vars()];
            self.mask = ReplicaMask::new(state.num_replicas());
            self.ready = true;
        }
    }
}

impl Operator for HoudayerClusterMove {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty().with(Capability::BarrierCrossing),
            constraints: cluster_constraints(),
            observables: ObservableSet::empty().with(Observable::Acceptance),
            guarantees: Guarantees::deterministic(),
            complexity: Complexity::LinearEdges,
        }
    }
    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        CostEstimate {
            work_per_sweep: (shape.n + 2 * shape.num_pairs) as f64
                * (shape.num_replicas.max(2) / 2) as f64,
        }
    }
    fn apply(
        &mut self,
        state: &mut dyn SpinState,
        _view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        self.ensure(state);
        let r = state.num_replicas();
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            let mut k = 0;
            while k + 1 < r {
                let (a, b) = (k, k + 1);
                let comps =
                    disagreement_components(state, a, b, &mut self.visited, &mut self.queue);
                report.proposed += 1;
                if !comps.is_empty() {
                    let pick = rng.gen_range(0..comps.len());
                    flip_component(state, &comps[pick], a, b, &mut self.mask);
                    report.accepted += 1;
                }
                k += 2;
            }
        }
        report.work = (budget.sweeps as f64) * (state.num_vars() as f64) * (r as f64);
        report
    }
}

// --------------------------------------------------------------------- ICM ----

#[derive(Default)]
pub struct IsoenergeticClusterMove {
    visited: Vec<bool>,
    queue: Vec<usize>,
    mask: ReplicaMask,
    ready: bool,
}

impl IsoenergeticClusterMove {
    pub const NAME: &'static str = "isoenergetic_cluster";
    pub fn new() -> Self {
        Self::default()
    }
    fn ensure(&mut self, state: &dyn SpinState) {
        if !self.ready {
            self.visited = vec![false; state.num_vars()];
            self.mask = ReplicaMask::new(state.num_replicas());
            self.ready = true;
        }
    }
}

impl Operator for IsoenergeticClusterMove {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty()
                .with(Capability::BarrierCrossing)
                .with(Capability::Exploration),
            constraints: cluster_constraints(),
            observables: ObservableSet::empty().with(Observable::Acceptance),
            guarantees: Guarantees::deterministic(),
            complexity: Complexity::LinearEdges,
        }
    }
    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        CostEstimate {
            work_per_sweep: (shape.n + 2 * shape.num_pairs) as f64
                * (shape.num_replicas.max(2) / 2) as f64,
        }
    }
    fn apply(
        &mut self,
        state: &mut dyn SpinState,
        _view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        self.ensure(state);
        let r = state.num_replicas();
        let mut report = Report::default();
        for _ in 0..budget.sweeps {
            let mut k = 0;
            while k + 1 < r {
                let (a, b) = (k, k + 1);
                let comps =
                    disagreement_components(state, a, b, &mut self.visited, &mut self.queue);
                for comp in &comps {
                    report.proposed += 1;
                    if rng.gen::<bool>() {
                        flip_component(state, comp, a, b, &mut self.mask);
                        report.accepted += 1;
                    }
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

    fn ir10() -> ProblemIR {
        let mut pairs = Vec::new();
        for i in 0..10u32 {
            pairs.push((i, (i + 1) % 10, if i % 2 == 0 { 1.0 } else { -1.0 }));
        }
        // fix wrap edge to be upper-triangle
        pairs.retain(|&(a, b, _)| a < b);
        pairs.push((0, 9, 1.0));
        pairs.push((2, 7, -1.0));
        ProblemIR::from_pairs(10, 0.0, vec![0.0; 10], &pairs)
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
            &mut ChaCha8Rng::seed_from_u64(4),
            Budget { sweeps: 6 },
        );
        GibbsColorSweep::new().apply(
            bs,
            &v,
            &mut ChaCha8Rng::seed_from_u64(4),
            Budget { sweeps: 6 },
        );
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
    fn houdayer_identical_and_ledger_exact() {
        let ir = ir10();
        let r = 16;
        let init = vec![0u8; 10];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        diverge(&mut refs, &mut bs, r);
        HoudayerClusterMove::new().apply(
            &mut refs,
            &view(r),
            &mut ChaCha8Rng::seed_from_u64(7),
            Budget { sweeps: 4 },
        );
        HoudayerClusterMove::new().apply(
            &mut bs,
            &view(r),
            &mut ChaCha8Rng::seed_from_u64(7),
            Budget { sweeps: 4 },
        );
        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        assert_eq!(e_ref, e_bs);
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(refs.audit(), 0.0);
        // Canonical re-score of a sampled replica agrees with the ledger.
        let mut x = vec![0u8; ir.n];
        bs.extract_into(3, &mut x);
        assert_eq!(ir.energy(&x), e_bs[3]);
    }

    #[test]
    fn icm_identical_and_ledger_exact() {
        let ir = ir10();
        let r = 16;
        let init = vec![0u8; 10];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        diverge(&mut refs, &mut bs, r);
        IsoenergeticClusterMove::new().apply(
            &mut refs,
            &view(r),
            &mut ChaCha8Rng::seed_from_u64(8),
            Budget { sweeps: 4 },
        );
        IsoenergeticClusterMove::new().apply(
            &mut bs,
            &view(r),
            &mut ChaCha8Rng::seed_from_u64(8),
            Budget { sweeps: 4 },
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
