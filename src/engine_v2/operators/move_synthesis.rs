//! `MoveSynthesizer` — an operator whose MOVE SET is not designed.
//!
//! Every solver audited in `research/RC003_ARCHITECTURE_AUDIT.md` — Neal,
//! OpenJij, CP-SAT, SCIP, and every operator in this library — shares assumption
//! A10: *the move set is written by a human and fixed at compile time.* It is the
//! only assumption in the audit that nobody breaks, and it is why the registry's
//! empty capabilities stay empty forever.
//!
//! This operator breaks it. It carries no neighborhood of its own. It watches the
//! run, mines structure from what it sees, and **synthesizes collective moves at
//! runtime** — a different move set per instance, per run, discovered rather than
//! specified. `runtime.rs:287` instantiates each operator once per run and reuses
//! it across every step, so the synthesized set can persist and evolve.
//!
//! Two sources of structure, and the difference between them is the experiment:
//!
//!  - [`Source::Population`] — covariance across replicas at one instant. This is
//!    the KNOWN method (linkage learning: LTGA, GOMEA, the EDA/BOA family).
//!  - [`Source::Dynamics`] — temporal co-flip along the trajectory: which
//!    variables move *together in time*.
//!
//! By the ergodic theorem these coincide at equilibrium —
//! `⟨δsᵢδsⱼ⟩_time = ⟨δsᵢδsⱼ⟩_ensemble` — so `Dynamics` can only differ from
//! `Population` **out of equilibrium**, which is exactly where annealing lives.
//! That is hypothesis H-C′, and it predicts the gap must vanish as mixing
//! improves. If it does not vanish, the ergodic argument holds in practice and
//! H-C′ is refuted.
//!
//! [`Source::Disabled`] performs no synthesis at all and is bit-identical to
//! `MetropolisSweep` — the null control, asserted by unit test.

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Where the synthesized move set comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// No synthesis — single-flip only. Bit-identical to `MetropolisSweep`.
    Disabled,
    /// Instantaneous cross-replica covariance (the known linkage-learning route).
    Population,
    /// Temporal co-flip along the trajectory (H-C′).
    Dynamics,
}

/// Replicas tracked bit-parallel in a `u64`; beyond this the tail is ignored for
/// STATISTICS only (the dynamics itself always covers every replica).
const TRACKED: usize = 64;

pub struct MoveSynthesizer {
    name: &'static str,
    source: Source,
    /// Resynthesize and apply collective moves every `period` sweeps. Keeps the
    /// added work a few percent of a full sweep.
    period: u32,
    max_moves: usize,
    max_move_size: usize,

    // --- base single-flip sweep (mirrors MetropolisSweep exactly) ---
    order: Vec<usize>,
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,

    // --- learned structure ---
    edges: Vec<(u32, u32)>,
    score: Vec<f64>,
    /// Per-site bitmask of replicas that flipped during the current sweep.
    flipped: Vec<u64>,
    moves: Vec<Vec<u32>>,
    sweeps_seen: u32,

    // --- scratch ---
    parent: Vec<u32>,
    acc: Vec<f64>,
    revert: ReplicaMask,
}

impl MoveSynthesizer {
    pub const NULL: &'static str = "synth_null";
    pub const POPULATION: &'static str = "synth_population";
    pub const DYNAMICS: &'static str = "synth_dynamics";

    pub fn new(name: &'static str, source: Source) -> Self {
        Self {
            name,
            source,
            period: 5,
            max_moves: 8,
            max_move_size: 64,
            order: Vec::new(),
            de: Vec::new(),
            mask: ReplicaMask::default(),
            ready: false,
            edges: Vec::new(),
            score: Vec::new(),
            flipped: Vec::new(),
            moves: Vec::new(),
            sweeps_seen: 0,
            parent: Vec::new(),
            acc: Vec::new(),
            revert: ReplicaMask::default(),
        }
    }

    pub fn null() -> Self {
        Self::new(Self::NULL, Source::Disabled)
    }
    pub fn population() -> Self {
        Self::new(Self::POPULATION, Source::Population)
    }
    pub fn dynamics() -> Self {
        Self::new(Self::DYNAMICS, Source::Dynamics)
    }

    fn ensure(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let (n, r) = (state.num_vars(), state.num_replicas());
        let colors = state.coloring();
        let ncolors = colors.iter().copied().max().map_or(0, |c| c as usize + 1);
        let mut order = Vec::with_capacity(n);
        for c in 0..ncolors as u32 {
            for (site, &col) in colors.iter().enumerate() {
                if col == c {
                    order.push(site);
                }
            }
        }
        self.order = order;
        self.de = vec![0.0; r];
        self.acc = vec![0.0; r];
        self.mask = ReplicaMask::new(r);
        self.revert = ReplicaMask::new(r);

        if self.source != Source::Disabled {
            // Candidate pairs are graph edges (i < j). Which of them become moves
            // is decided by the learned score, never by the coupling value —
            // operators cannot see weights.
            let mut edges = Vec::new();
            for i in 0..n {
                for &j in state.neighbors(i) {
                    if (i as u32) < j {
                        edges.push((i as u32, j));
                    }
                }
            }
            self.score = vec![0.0; edges.len()];
            self.edges = edges;
            self.flipped = vec![0u64; n];
            self.parent = vec![0u32; n];
        }
        self.ready = true;
    }

    fn find(parent: &mut [u32], mut x: u32) -> u32 {
        while parent[x as usize] != x {
            parent[x as usize] = parent[parent[x as usize] as usize];
            x = parent[x as usize];
        }
        x
    }

    /// Rebuild the move set: take the strongest-scoring edges and let connected
    /// components of that subgraph BE the moves. Nothing here is designed — the
    /// shape and number of moves follow from the learned scores.
    fn synthesize(&mut self, n: usize) {
        self.moves.clear();
        if self.edges.is_empty() {
            return;
        }
        let mut idx: Vec<usize> = (0..self.edges.len()).collect();
        idx.sort_unstable_by(|&a, &b| {
            self.score[b]
                .partial_cmp(&self.score[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // Keep only edges with real signal, capped so components stay small.
        let keep = (self.max_moves * self.max_move_size).min(idx.len());
        for (i, p) in self.parent.iter_mut().enumerate().take(n) {
            *p = i as u32;
        }
        // Component sizes, so a merge can be REFUSED before a move grows into a
        // near-global flip. Unbounded growth is not a hypothetical: on a small
        // graph the top-scoring edges connect everything, and flipping every spin
        // is an exact symmetry of zero-field Ising (ΔE ≡ 0) — a synthesized
        // no-op that would silently make every source look alike.
        let mut size = vec![1u32; n];
        // Also bounded by a fraction of the system: a move spanning most of the
        // graph approaches the global spin flip, which is a symmetry rather than
        // a move. A synthesized move must stay a genuine sub-system.
        let cap = (self.max_move_size.min(n / 4).max(2)) as u32;
        let mut used = 0usize;
        for &e in idx.iter().take(keep) {
            if self.score[e] <= 0.0 {
                break;
            }
            let (a, b) = self.edges[e];
            let (ra, rb) = (
                Self::find(&mut self.parent, a),
                Self::find(&mut self.parent, b),
            );
            if ra != rb && size[ra as usize] + size[rb as usize] <= cap {
                size[rb as usize] += size[ra as usize];
                self.parent[ra as usize] = rb;
                used += 1;
            }
        }
        if used == 0 {
            return;
        }
        // Collect components of size ≥ 2, largest-first, capped.
        let mut buckets: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();
        for site in 0..n as u32 {
            let root = Self::find(&mut self.parent, site);
            buckets.entry(root).or_default().push(site);
        }
        let mut comps: Vec<Vec<u32>> = buckets
            .into_values()
            .filter(|c| c.len() >= 2 && c.len() <= self.max_move_size)
            .collect();
        comps.sort_unstable_by(|a, b| b.len().cmp(&a.len()).then_with(|| a[0].cmp(&b[0])));
        comps.truncate(self.max_moves);
        self.moves = comps;
    }

    /// Propose each synthesized subset as ONE move, per replica, Metropolis-
    /// accepted on the exact multi-site ΔE. Exact because ΔE accumulates through
    /// the same `delta_e_into` ledger the single-flip path uses.
    fn apply_collective(
        &mut self,
        state: &mut dyn SpinState,
        temps: &[f64],
        rng: &mut ChaCha8Rng,
        report: &mut Report,
    ) {
        let r = state.num_replicas();
        for m in 0..self.moves.len() {
            let cluster = std::mem::take(&mut self.moves[m]);
            self.acc.iter_mut().for_each(|a| *a = 0.0);
            // Flip the whole subset in every replica, accumulating exact ΔE.
            self.mask.clear();
            for rep in 0..r {
                self.mask.set(rep);
            }
            for &site in &cluster {
                state.delta_e_into(site as usize, &mut self.de);
                for rep in 0..r {
                    self.acc[rep] += self.de[rep];
                }
                state.apply_flips(site as usize, &self.mask);
            }
            // Accept or reject per replica; revert the rejected ones wholesale.
            self.revert.clear();
            let mut any_revert = false;
            for rep in 0..r {
                let d = self.acc[rep];
                let t = temps[rep % temps.len()];
                let accept = if d <= 0.0 {
                    true
                } else {
                    let p = if t > 0.0 { (-d / t).exp() } else { 0.0 };
                    rng.gen::<f64>() < p
                };
                report.proposed += 1;
                if accept {
                    report.accepted += 1;
                } else {
                    self.revert.set(rep);
                    any_revert = true;
                }
            }
            if any_revert {
                for &site in &cluster {
                    state.apply_flips(site as usize, &self.revert);
                }
            }
            self.moves[m] = cluster;
        }
    }
}

impl Operator for MoveSynthesizer {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: self.name,
            nature: if self.source == Source::Disabled {
                Nature::TrajectoryPreserving
            } else {
                // The move set is state- and history-dependent, so the transition
                // kernel changes during the run. Honest typing (Constitution §4.11).
                Nature::BehaviorChanging
            },
            capabilities: CapabilitySet::empty()
                .with(Capability::Exploration)
                .with(Capability::Exploitation)
                .with(Capability::BarrierCrossing),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: self.source == Source::Population,
                needs_temperature: true,
            },
            observables: ObservableSet::empty()
                .with(Observable::Acceptance)
                .with(Observable::EnergyDelta)
                .with(Observable::Diversity),
            guarantees: Guarantees {
                equilibrates: self.source == Source::Disabled,
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
        self.ensure(state);
        let (n, r) = (state.num_vars(), state.num_replicas());
        let temps = view.temperatures;
        let tracked = r.min(TRACKED);
        let mut report = Report::default();

        for _ in 0..budget.sweeps {
            if self.source == Source::Dynamics {
                self.flipped.iter_mut().for_each(|f| *f = 0);
            }

            // ---- base single-flip sweep, RNG-identical to MetropolisSweep ----
            for &site in &self.order {
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                for rep in 0..r {
                    let d = self.de[rep];
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
                        report.accepted += 1;
                        if self.source == Source::Dynamics && rep < tracked {
                            self.flipped[site] |= 1u64 << rep;
                        }
                    }
                }
                if !self.mask.is_empty() {
                    state.apply_flips(site, &self.mask);
                }
            }

            if self.source == Source::Disabled {
                continue;
            }
            self.sweeps_seen += 1;

            // ---- accumulate structure ----
            match self.source {
                Source::Dynamics => {
                    // How often did i and j flip in the SAME sweep, same replica?
                    for (e, &(i, j)) in self.edges.iter().enumerate() {
                        let both = self.flipped[i as usize] & self.flipped[j as usize];
                        if both != 0 {
                            self.score[e] += both.count_ones() as f64;
                        }
                    }
                }
                Source::Population if self.sweeps_seen.is_multiple_of(self.period) => {
                    // Instantaneous cross-replica agreement |⟨sᵢsⱼ⟩ − ⟨sᵢ⟩⟨sⱼ⟩|.
                    // Sampled once per synthesis window rather than every sweep:
                    // this is a SNAPSHOT statistic, so denser sampling adds cost
                    // (O(E·R) vs Dynamics' O(E) bit-ops) without adding signal.
                    // Keeps the two sources cost-comparable, so the comparison
                    // measures structure and not overhead.
                    for (e, &(i, j)) in self.edges.iter().enumerate() {
                        let (mut agree, mut si, mut sj) = (0i64, 0i64, 0i64);
                        for rep in 0..tracked {
                            let a = state.spin(i as usize, rep);
                            let b = state.spin(j as usize, rep);
                            agree += if a == b { 1 } else { -1 };
                            si += if a { 1 } else { -1 };
                            sj += if b { 1 } else { -1 };
                        }
                        let t = tracked as f64;
                        let cov = agree as f64 / t - (si as f64 / t) * (sj as f64 / t);
                        self.score[e] += cov.abs();
                    }
                }
                Source::Population => {}
                Source::Disabled => unreachable!(),
            }

            // ---- resynthesize and fire the learned moves ----
            if self.sweeps_seen.is_multiple_of(self.period) {
                self.synthesize(n);
                if !self.moves.is_empty() {
                    self.apply_collective(state, temps, rng, &mut report);
                }
            }
        }

        report.aux.push(("synth_moves", self.moves.len() as f64));
        let mean_size = if self.moves.is_empty() {
            0.0
        } else {
            self.moves.iter().map(|m| m.len()).sum::<usize>() as f64 / self.moves.len() as f64
        };
        report.aux.push(("synth_mean_size", mean_size));
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
    use crate::engine_v2::operators::MetropolisSweep;
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

    fn temps(r: usize) -> Vec<f64> {
        (0..r).map(|k| 0.2 + 0.05 * k as f64).collect()
    }

    /// Null control: synthesis off must reproduce `MetropolisSweep` exactly, or
    /// every arm comparison is measuring construction error.
    #[test]
    fn disabled_is_bit_identical_to_metropolis() {
        let ir = frustrated();
        let (r, init) = (64, [1u8, 0, 1, 0, 0, 1, 0]);
        let t = temps(r);
        let view = RuntimeView {
            iteration: 0,
            temperatures: &t,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let budget = Budget { sweeps: 40 };

        let mut a = ReferenceState::new(&ir, r, &init);
        let mut b = ReferenceState::new(&ir, r, &init);
        let ra =
            MetropolisSweep::new().apply(&mut a, &view, &mut ChaCha8Rng::seed_from_u64(5), budget);
        let rb =
            MoveSynthesizer::null().apply(&mut b, &view, &mut ChaCha8Rng::seed_from_u64(5), budget);

        assert_eq!(ra.accepted, rb.accepted);
        assert_eq!(ra.proposed, rb.proposed);
        assert_eq!(a.digest(), b.digest(), "null arm diverged from Metropolis");
    }

    /// The architecture's core claim: a move set actually gets built at runtime.
    #[test]
    fn synthesizes_a_nonempty_move_set() {
        let ir = frustrated();
        let (r, init) = (32, [1u8, 0, 1, 0, 0, 1, 0]);
        let t = temps(r);
        let view = RuntimeView {
            iteration: 0,
            temperatures: &t,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        for mut op in [MoveSynthesizer::dynamics(), MoveSynthesizer::population()] {
            let mut st = ReferenceState::new(&ir, r, &init);
            let rep = op.apply(
                &mut st,
                &view,
                &mut ChaCha8Rng::seed_from_u64(9),
                Budget { sweeps: 30 },
            );
            let k = rep
                .aux
                .iter()
                .find(|(k, _)| *k == "synth_moves")
                .map(|(_, v)| *v)
                .expect("synth sensor");
            assert!(k > 0.0, "{} synthesized no moves", op.name());
            assert_eq!(st.audit(), 0.0, "collective moves corrupted the ledger");
        }
    }

    /// Collective moves must keep both backends bit-identical (ADR-0004) and
    /// leave zero ledger drift after accept/revert cycles.
    #[test]
    fn identical_on_both_backends() {
        let ir = frustrated();
        let (r, init) = (32, [1u8, 0, 1, 0, 0, 1, 0]);
        let t = temps(r);
        let view = RuntimeView {
            iteration: 0,
            temperatures: &t,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let budget = Budget { sweeps: 25 };
        for src in [Source::Dynamics, Source::Population] {
            let mut refs = ReferenceState::new(&ir, r, &init);
            let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
            let a = MoveSynthesizer::new("x", src).apply(
                &mut refs,
                &view,
                &mut ChaCha8Rng::seed_from_u64(3),
                budget,
            );
            let b = MoveSynthesizer::new("x", src).apply(
                &mut bs,
                &view,
                &mut ChaCha8Rng::seed_from_u64(3),
                budget,
            );
            assert_eq!(a.accepted, b.accepted, "{src:?} diverged");
            let (mut e1, mut e2) = (vec![0.0; r], vec![0.0; r]);
            refs.energies_into(&mut e1);
            bs.energies_into(&mut e2);
            assert_eq!(e1, e2, "{src:?} trajectories differ across backends");
            assert_eq!(bs.audit(), 0.0);
            assert_eq!(refs.audit(), 0.0);
        }
    }

    /// Dynamics and Population must actually learn DIFFERENT move sets, or H-C′
    /// has no content to test.
    #[test]
    fn sources_learn_different_structure() {
        let ir = frustrated();
        let (r, init) = (32, [1u8, 0, 1, 0, 0, 1, 0]);
        let t = temps(r);
        let view = RuntimeView {
            iteration: 0,
            temperatures: &t,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let mut d = ReferenceState::new(&ir, r, &init);
        let mut p = ReferenceState::new(&ir, r, &init);
        MoveSynthesizer::dynamics().apply(
            &mut d,
            &view,
            &mut ChaCha8Rng::seed_from_u64(4),
            Budget { sweeps: 30 },
        );
        MoveSynthesizer::population().apply(
            &mut p,
            &view,
            &mut ChaCha8Rng::seed_from_u64(4),
            Budget { sweeps: 30 },
        );
        assert_ne!(d.digest(), p.digest());
    }
}
