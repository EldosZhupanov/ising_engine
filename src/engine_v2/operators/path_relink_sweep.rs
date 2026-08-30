//! RC-024 endpoint-guided path relinking.
//!
//! The control is the existing [`MetropolisSweep`].  The enabled operator runs
//! that exact prefix, consuming the exact same RNG stream, then synthesizes one
//! deterministic greedy path from each selected source replica toward the best
//! population member.  Only the best prefix of each path is retained.
//!
//! Binding design: `research/PREREG_RC024_PATH_RELINKING.md`.

use super::MetropolisSweep;
use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand_chacha::ChaCha8Rng;

pub struct PathRelinkSweep {
    enabled: bool,
    base: MetropolisSweep,
    energies: Vec<f64>,
    ranked: Vec<usize>,
    sources: Vec<usize>,
    differing: Vec<usize>,
    path: Vec<usize>,
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl PathRelinkSweep {
    pub const NAME: &'static str = "path_relink_sweep";

    pub fn new() -> Self {
        Self::with_enabled(true)
    }

    #[cfg(test)]
    fn disabled() -> Self {
        Self::with_enabled(false)
    }

    fn with_enabled(enabled: bool) -> Self {
        Self {
            enabled,
            base: MetropolisSweep::new(),
            energies: Vec::new(),
            ranked: Vec::new(),
            sources: Vec::new(),
            differing: Vec::new(),
            path: Vec::new(),
            de: Vec::new(),
            mask: ReplicaMask::default(),
            ready: false,
        }
    }

    fn ensure(&mut self, state: &dyn SpinState) {
        if self.ready {
            debug_assert_eq!(self.energies.len(), state.num_replicas());
            debug_assert_eq!(self.de.len(), state.num_replicas());
            return;
        }
        let (n, r) = (state.num_vars(), state.num_replicas());
        self.energies = vec![0.0; r];
        self.ranked = Vec::with_capacity(r);
        self.sources = Vec::with_capacity((r / 4).max(1));
        self.differing = Vec::with_capacity(n);
        self.path = Vec::with_capacity(n);
        self.de = vec![0.0; r];
        self.mask = ReplicaMask::new(r);
        self.ready = true;
    }

    /// Relink one source to one fixed target. Returns
    /// `(path_steps, delta_e_evaluations, retained_prefix_len)`.
    fn relink_source(
        &mut self,
        state: &mut dyn SpinState,
        source: usize,
        target: usize,
        start_energy: f64,
    ) -> (usize, usize, usize) {
        self.differing.clear();
        self.path.clear();
        for site in 0..state.num_vars() {
            if state.spin(site, source) != state.spin(site, target) {
                self.differing.push(site);
            }
        }

        let mut current = start_energy;
        let mut best = start_energy;
        let mut best_step = 0usize;
        let mut evaluations = 0usize;

        while !self.differing.is_empty() {
            let mut pick_pos = 0usize;
            let mut pick_site = usize::MAX;
            let mut pick_delta = f64::INFINITY;
            for (pos, &site) in self.differing.iter().enumerate() {
                state.delta_e_into(site, &mut self.de);
                evaluations += 1;
                let delta = self.de[source];
                let order = delta
                    .total_cmp(&pick_delta)
                    .then_with(|| site.cmp(&pick_site));
                if order.is_lt() {
                    pick_pos = pos;
                    pick_site = site;
                    pick_delta = delta;
                }
            }

            self.differing.swap_remove(pick_pos);
            self.path.push(pick_site);
            self.mask.clear();
            self.mask.set(source);
            state.apply_flips(pick_site, &self.mask);
            current += pick_delta;
            if current < best {
                best = current;
                best_step = self.path.len();
            }
        }

        // The path reached the target. Walk only the suffix back so the source
        // ends at its best prefix. This uses the same exact ledger operation as
        // the forward path and cannot leave the source worse than its endpoint.
        for &site in self.path[best_step..].iter().rev() {
            self.mask.clear();
            self.mask.set(source);
            state.apply_flips(site, &self.mask);
        }

        (self.path.len(), evaluations, best_step)
    }

    fn relink_round(&mut self, state: &mut dyn SpinState) -> (usize, usize, usize) {
        self.ensure(state);
        let r = state.num_replicas();
        if r < 2 {
            return (0, 0, 0);
        }
        state.energies_into(&mut self.energies);
        let target = (0..r)
            .min_by(|&a, &b| {
                self.energies[a]
                    .total_cmp(&self.energies[b])
                    .then_with(|| a.cmp(&b))
            })
            .expect("r >= 2");

        self.ranked.clear();
        self.ranked.extend(0..r);
        self.ranked.sort_unstable_by(|&a, &b| {
            self.energies[b]
                .total_cmp(&self.energies[a])
                .then_with(|| a.cmp(&b))
        });
        self.sources.clear();
        self.sources.extend(
            self.ranked
                .iter()
                .copied()
                .filter(|&rep| rep != target)
                .take((r / 4).max(1)),
        );

        let (mut paths, mut evaluations, mut retained) = (0usize, 0usize, 0usize);
        // `relink_source` mutates other scratch fields. Indexing avoids holding
        // an immutable borrow of `self.sources` across that call.
        for position in 0..self.sources.len() {
            let source = self.sources[position];
            let (steps, evals, kept) =
                self.relink_source(state, source, target, self.energies[source]);
            paths += usize::from(steps != 0);
            evaluations += evals;
            retained += kept;
        }
        (paths, evaluations, retained)
    }
}

impl Default for PathRelinkSweep {
    fn default() -> Self {
        Self::new()
    }
}

impl Operator for PathRelinkSweep {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: if self.enabled {
                Nature::BehaviorChanging
            } else {
                Nature::TrajectoryPreserving
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
                needs_replicas: self.enabled,
                needs_temperature: true,
            },
            observables: ObservableSet::empty()
                .with(Observable::Acceptance)
                .with(Observable::EnergyDelta)
                .with(Observable::Diversity),
            guarantees: Guarantees::deterministic(),
            complexity: if self.enabled {
                Complexity::Superlinear
            } else {
                Complexity::LinearEdges
            },
        }
    }

    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        let base = ((shape.n + 2 * shape.num_pairs) * shape.num_replicas.max(1)) as f64;
        let relink = if self.enabled {
            (shape.n * shape.n * shape.num_replicas.max(1) / 8) as f64
        } else {
            0.0
        };
        CostEstimate {
            work_per_sweep: base + relink,
        }
    }

    fn apply(
        &mut self,
        state: &mut dyn SpinState,
        view: &RuntimeView,
        rng: &mut ChaCha8Rng,
        budget: Budget,
    ) -> Report {
        let mut report = self.base.apply(state, view, rng, budget);
        if !self.enabled {
            return report;
        }

        let (paths, evaluations, retained) = self.relink_round(state);
        report.proposed += paths as u64;
        report.accepted += usize::from(retained != 0) as u64;
        report.work += (evaluations * state.num_replicas()) as f64;
        report.aux.push(("relink_paths", paths as f64));
        report.aux.push(("relink_retained_steps", retained as f64));
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_v2::backends::{ReferenceState, SparseBitSlice};
    use crate::engine_v2::ir::ProblemIR;
    use rand::{Rng, SeedableRng};

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

    /// A frustrated ring with chords. Large enough that relinking paths are
    /// long and non-monotone, which the seven-variable fixture is not: there the
    /// greedy walk to the best replica happens to descend all the way, so the
    /// rewind never fires and the never-worse invariant cannot be observed.
    fn frustrated_large() -> ProblemIR {
        const N: usize = 40;
        let linear: Vec<f64> = (0..N)
            .map(|i| if i % 3 == 0 { 1.0 } else { -0.5 })
            .collect();
        let mut edges: Vec<(u32, u32, f64)> = Vec::with_capacity(2 * N);
        for i in 0..N as u32 {
            for (other, weight) in [((i + 1) % N as u32, 1.0), ((i + 13) % N as u32, -1.0)] {
                if i != other {
                    edges.push((i.min(other), i.max(other), weight));
                }
            }
        }
        ProblemIR::from_pairs(N, 0.0, linear, &edges)
    }

    fn view(r: usize, temps: &[f64]) -> RuntimeView<'_> {
        RuntimeView {
            iteration: 0,
            temperatures: temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        }
    }

    #[test]
    fn disabled_is_bit_identical_to_metropolis_including_rng_continuation() {
        let ir = frustrated();
        let r = 16;
        let temps: Vec<f64> = (0..r).map(|k| 0.2 + 0.05 * k as f64).collect();
        let mut a = ReferenceState::new(&ir, r, &[1, 0, 1, 0, 0, 1, 0]);
        let mut b = ReferenceState::new(&ir, r, &[1, 0, 1, 0, 0, 1, 0]);
        let mut ra = ChaCha8Rng::seed_from_u64(101);
        let mut rb = ChaCha8Rng::seed_from_u64(101);
        let budget = Budget { sweeps: 20 };
        let left = MetropolisSweep::new().apply(&mut a, &view(r, &temps), &mut ra, budget);
        let right = PathRelinkSweep::disabled().apply(&mut b, &view(r, &temps), &mut rb, budget);
        assert_eq!(left.accepted, right.accepted);
        assert_eq!(left.proposed, right.proposed);
        assert_eq!(left.work, right.work);
        assert_eq!(a.digest(), b.digest());
        assert_eq!(ra.gen::<u64>(), rb.gen::<u64>());
    }

    #[test]
    fn relinking_is_backend_identical_exact_and_deterministic() {
        let ir = frustrated();
        let r = 32;
        let temps: Vec<f64> = (0..r).map(|k| 0.1 + 0.1 * k as f64).collect();
        let mut refs = ReferenceState::new(&ir, r, &[0; 7]);
        let mut replay = ReferenceState::new(&ir, r, &[0; 7]);
        let mut bits = SparseBitSlice::new(&ir, r, &[0; 7]).unwrap();
        let budget = Budget { sweeps: 25 };
        let a = PathRelinkSweep::new().apply(
            &mut refs,
            &view(r, &temps),
            &mut ChaCha8Rng::seed_from_u64(102),
            budget,
        );
        let b = PathRelinkSweep::new().apply(
            &mut bits,
            &view(r, &temps),
            &mut ChaCha8Rng::seed_from_u64(102),
            budget,
        );
        let c = PathRelinkSweep::new().apply(
            &mut replay,
            &view(r, &temps),
            &mut ChaCha8Rng::seed_from_u64(102),
            budget,
        );
        assert_eq!(a.accepted, b.accepted);
        assert_eq!(a.proposed, b.proposed);
        assert_eq!(a.accepted, c.accepted);
        assert_eq!(a.proposed, c.proposed);
        assert_eq!(refs.digest(), replay.digest(), "fixed-seed replay drifted");
        assert_eq!(refs.audit(), 0.0);
        assert_eq!(bits.audit(), 0.0);

        let mut energies = vec![0.0; r];
        let mut bit_energies = vec![0.0; r];
        let mut state = vec![0u8; ir.n];
        let mut bit_state = vec![0u8; ir.n];
        refs.energies_into(&mut energies);
        bits.energies_into(&mut bit_energies);
        assert_eq!(energies, bit_energies);
        for (replica, &ledger) in energies.iter().enumerate() {
            refs.extract_into(replica, &mut state);
            bits.extract_into(replica, &mut bit_state);
            assert_eq!(state, bit_state, "replica {replica} differs by backend");
            assert_eq!(ir.energy(&state), ledger);
        }
    }

    #[test]
    fn lowest_index_tie_and_best_prefix_are_retained() {
        // E(00)=0, E(10)=E(01)=-2, E(11)=-1. Relinking 00 -> 11 has a
        // lowest-index tie at its first step and must retain 10, not endpoint 11.
        let ir = ProblemIR::from_pairs(2, 0.0, vec![-2.0, -2.0], &[(0, 1, 3.0)]);
        let mut state = ReferenceState::new(&ir, 2, &[0, 0]);
        let mut both = ReplicaMask::new(2);
        both.set(0);
        state.apply_flips(0, &both);
        state.apply_flips(1, &both);

        let mut op = PathRelinkSweep::new();
        op.ensure(&state);
        let (_, _, best_step) = op.relink_source(&mut state, 1, 0, 0.0);
        assert_eq!(op.path, vec![0, 1], "ties must choose the lowest site");
        assert_eq!(best_step, 1);
        assert!(state.spin(0, 1));
        assert!(!state.spin(1, 1));
        let mut energies = [0.0; 2];
        state.energies_into(&mut energies);
        assert_eq!(energies[1], -2.0, "the best intermediate must survive");
        assert!(energies[1] <= 0.0, "a source may never finish worse");
        assert_eq!(state.audit(), 0.0);
    }

    fn hamming(state: &dyn SpinState, a: usize, b: usize) -> usize {
        (0..state.num_vars())
            .filter(|&site| state.spin(site, a) != state.spin(site, b))
            .count()
    }

    /// PREREG §5.4 — strict Hamming progress. Every step of a path must remove
    /// exactly one still-differing variable, never revisit a site and never
    /// touch a site the endpoints already agree on. The path is therefore a
    /// permutation of the initial difference set, and the distance left at the
    /// end is exactly the suffix the rewind undid. Asserting those three facts
    /// together pins the step arithmetic without needing a hook inside the loop.
    #[test]
    fn every_path_step_removes_exactly_one_differing_variable() {
        let ir = frustrated_large();
        let r = 4;
        // Diversify with the real prefix so the endpoints are not hand-picked.
        let mut state = ReferenceState::new(&ir, r, &vec![0; ir.n]);
        let temps: Vec<f64> = (0..r).map(|k| 0.5 + 1.5 * k as f64).collect();
        MetropolisSweep::new().apply(
            &mut state,
            &view(r, &temps),
            &mut ChaCha8Rng::seed_from_u64(101),
            Budget { sweeps: 12 },
        );

        let (source, target) = (3usize, 0usize);
        let distance_before = hamming(&state, source, target);
        assert!(distance_before >= 3, "the endpoints must actually differ");
        let mut expected: Vec<usize> = (0..ir.n)
            .filter(|&site| state.spin(site, source) != state.spin(site, target))
            .collect();
        let mut energies = vec![0.0; r];
        state.energies_into(&mut energies);
        let target_before = energies[target];

        let mut op = PathRelinkSweep::new();
        op.ensure(&state);
        let (steps, evaluations, best_step) =
            op.relink_source(&mut state, source, target, energies[source]);

        assert_eq!(steps, distance_before, "a step must close exactly one gap");
        assert_eq!(op.path.len(), distance_before);
        let mut walked = op.path.clone();
        walked.sort_unstable();
        expected.sort_unstable();
        assert_eq!(
            walked, expected,
            "the path must be a permutation of the difference set: no repeats, \
             no sites the endpoints already agreed on"
        );
        assert!(best_step <= steps);
        assert_eq!(
            hamming(&state, source, target),
            steps - best_step,
            "the retained prefix and the rewound suffix must account for the \
             whole distance"
        );
        // Every still-differing variable is re-evaluated at every step.
        assert_eq!(evaluations, distance_before * (distance_before + 1) / 2);

        state.energies_into(&mut energies);
        assert_eq!(energies[target], target_before, "the target was modified");
        assert_eq!(state.audit(), 0.0);
    }

    /// PREREG §2 step 6 — "the lowest-energy prefix, **endpoints included**".
    /// The zero-length prefix is the source itself, and it is the only thing
    /// that protects a source whose entire path merely ties its start. Because
    /// the target is by construction the lowest-energy replica, the path's far
    /// endpoint can never be worse than the source, so a seed that drops the
    /// zero-length prefix stays invisible everywhere except on an exact tie.
    /// These synthetic endpoints are that tie.
    #[test]
    fn a_source_that_never_beats_its_start_is_left_exactly_where_it_began() {
        // E(000) = 0, E(100) = E(110) = 5, E(111) = 0: the walk ties its start
        // at the far end and is strictly worse everywhere in between.
        let ir = ProblemIR::from_pairs(
            3,
            0.0,
            vec![5.0, 5.0, 5.0],
            &[(0, 1, -5.0), (0, 2, -5.0), (1, 2, -5.0)],
        );
        assert_eq!(ir.energy(&[0, 0, 0]), 0.0);
        assert_eq!(ir.energy(&[1, 0, 0]), 5.0);
        assert_eq!(ir.energy(&[1, 1, 0]), 5.0);
        assert_eq!(ir.energy(&[1, 1, 1]), 0.0, "the endpoints must tie exactly");

        // Replica 0 is the target at 111; replica 1 is the source at 000.
        let mut state = ReferenceState::new(&ir, 2, &[0, 0, 0]);
        let mut target_only = ReplicaMask::new(2);
        target_only.set(0);
        for site in 0..3 {
            state.apply_flips(site, &target_only);
        }
        let mut energies = [0.0; 2];
        state.energies_into(&mut energies);
        assert_eq!(energies, [0.0, 0.0]);

        let mut op = PathRelinkSweep::new();
        op.ensure(&state);
        let (steps, _, best_step) = op.relink_source(&mut state, 1, 0, energies[1]);

        assert_eq!(steps, 3, "the walk must cross the whole difference");
        assert_eq!(
            best_step, 0,
            "no prefix beat the start, so the retained prefix is the source"
        );
        for site in 0..3 {
            assert!(!state.spin(site, 1), "the source was moved for nothing");
        }
        state.energies_into(&mut energies);
        assert_eq!(energies[1], 0.0);
        assert_eq!(state.audit(), 0.0);
    }

    /// PREREG §5.4 — the never-worse invariant, on a population where the
    /// greedy path genuinely has to climb before it reaches the target, so the
    /// rewind is what enforces the guarantee rather than the arithmetic being
    /// trivially satisfied. The test is only meaningful if a rewind actually
    /// fires and some source actually improves, so both are asserted.
    #[test]
    fn no_source_ever_finishes_worse_than_it_began() {
        let ir = frustrated_large();
        let r = 32;
        let temps: Vec<f64> = (0..r).map(|k| 0.3 + 0.3 * k as f64).collect();
        let mut state = ReferenceState::new(&ir, r, &vec![0; ir.n]);
        MetropolisSweep::new().apply(
            &mut state,
            &view(r, &temps),
            &mut ChaCha8Rng::seed_from_u64(102),
            Budget { sweeps: 30 },
        );

        let mut before = vec![0.0; r];
        state.energies_into(&mut before);
        let target = (0..r)
            .min_by(|&a, &b| before[a].total_cmp(&before[b]).then_with(|| a.cmp(&b)))
            .expect("r >= 2");

        let mut op = PathRelinkSweep::new();
        let (paths, _, _) = op.relink_round(&mut state);
        assert!(paths > 0, "no path ran; the invariant would be vacuous");
        let sources = op.sources.clone();
        assert!(!sources.is_empty());

        let mut after = vec![0.0; r];
        state.energies_into(&mut after);
        assert_eq!(after[target], before[target], "the target was modified");

        let mut improved = 0usize;
        let mut rewound = 0usize;
        for &source in &sources {
            assert!(
                after[source] <= before[source],
                "source {source} finished worse: {} -> {}",
                before[source],
                after[source]
            );
            if after[source] < before[source] {
                improved += 1;
            }
            // A path walked to its end leaves the source *at* the target. Any
            // source that differs from the target was rewound to a prefix.
            if hamming(&state, source, target) != 0 {
                rewound += 1;
            }
        }
        assert!(improved > 0, "no source improved; the test proves nothing");
        assert!(
            rewound > 0,
            "no rewind fired; the invariant is untested here"
        );

        // The retained states are real states, not bookkeeping.
        assert_eq!(state.audit(), 0.0);
        let mut config = vec![0u8; ir.n];
        for (replica, &energy) in after.iter().enumerate() {
            state.extract_into(replica, &mut config);
            assert_eq!(ir.energy(&config), energy, "replica {replica} drifted");
        }
    }

    #[test]
    fn enabled_apply_reaches_the_relinker_after_its_identical_prefix() {
        let ir = ProblemIR::from_pairs(2, 0.0, vec![-2.0, -2.0], &[(0, 1, 3.0)]);
        let mut state = ReferenceState::new(&ir, 2, &[0, 0]);
        let mut target = ReplicaMask::new(2);
        target.set(0);
        state.apply_flips(0, &target);
        state.apply_flips(1, &target);
        let temps = [1.0, 1.0];

        let report = PathRelinkSweep::new().apply(
            &mut state,
            &view(2, &temps),
            &mut ChaCha8Rng::seed_from_u64(103),
            Budget { sweeps: 0 },
        );

        assert!(state.spin(0, 1));
        assert!(!state.spin(1, 1));
        assert_eq!(
            report
                .aux
                .iter()
                .find(|(name, _)| *name == "relink_paths")
                .map(|(_, value)| *value),
            Some(1.0),
            "removing the enabled relink call must fail this test"
        );
    }
}
