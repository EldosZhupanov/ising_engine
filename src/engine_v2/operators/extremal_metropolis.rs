//! `ExtremalMetropolis` — the FIRST operator to come out of the platform's own
//! research loop: drafted by the meta-learner as
//! `proposals/operator_proposal_extremal_metropolis.md` (the τ-EO →
//! metropolis_sweep transition dominated the best discovered schedules),
//! implemented here manually under the Verification Track, exactly as the
//! proposal specifies:
//!
//!   Phase 1 — τ-EO extremal selection proposes the move set M: per replica,
//!             rank all sites by ascending ΔE and flip one rank-selected site
//!             (always-move ⇒ barrier crossing).
//!   Phase 2 — a Metropolis pass RESTRICTED to the neighborhood N(M) of the
//!             sites touched in Phase 1 (the union over replicas, ascending
//!             site order), rather than a full lattice pass — the fused kernel
//!             relaxes exactly the region the extremal move disturbed.
//!
//! Purity and determinism match every other operator: reads only the
//! `SpinState` interface + `RuntimeView` temperatures, one seeded RNG with a
//! fixed draw order, ±0.0 canonicalized before ranking (the EO gotcha) — so it
//! is bit-identical across all three backends (ADR-0004).

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
pub struct ExtremalMetropolis {
    /// ΔE for every (site, replica), row-major by site (Phase 1 ranking).
    de_mat: Vec<f64>,
    order: Vec<usize>,
    pick: Vec<usize>,
    /// Phase 2 region: sorted, deduplicated M ∪ N(M).
    region: Vec<usize>,
    in_region: Vec<bool>,
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl ExtremalMetropolis {
    pub const NAME: &'static str = "extremal_metropolis";
    /// Same rank bias as the parent τ-EO, so Phase 1 is the proven dynamics.
    const BIAS: f64 = 2.5;

    pub fn new() -> Self {
        Self::default()
    }

    fn ensure(&mut self, state: &dyn SpinState) {
        if self.ready {
            return;
        }
        let (n, r) = (state.num_vars(), state.num_replicas());
        self.de_mat = vec![0.0; n * r];
        self.order = (0..n).collect();
        self.pick = vec![0; r];
        self.region = Vec::with_capacity(n);
        self.in_region = vec![false; n];
        self.de = vec![0.0; r];
        self.mask = ReplicaMask::new(r);
        self.ready = true;
    }
}

impl Operator for ExtremalMetropolis {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            // The union of the parents' passports: EO's barrier crossing and
            // exploration, Metropolis' exploitation of the disturbed region.
            capabilities: CapabilitySet::empty()
                .with(Capability::BarrierCrossing)
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
            guarantees: Guarantees::deterministic(),
            complexity: Complexity::Superlinear,
        }
    }

    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        // Phase 1 dominates (ΔE fill + per-replica rank): n·r·log n. Phase 2
        // touches only ~r·(1+deg) sites, bounded by one partial sweep.
        let n = shape.n.max(1) as f64;
        let r = shape.num_replicas.max(1) as f64;
        CostEstimate {
            work_per_sweep: n * r * n.log2().max(1.0) + (shape.num_pairs.max(1) as f64),
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
        if n == 0 {
            return Report::default();
        }
        let temps = view.temperatures;
        debug_assert!(!temps.is_empty(), "ExtremalMetropolis needs a temperature");

        let mut report = Report::default();
        let mut de_col = vec![0.0f64; r];
        for _ in 0..budget.sweeps {
            // ---- Phase 1: τ-EO extremal move set M ---------------------------
            for site in 0..n {
                state.delta_e_into(site, &mut de_col);
                let base = site * r;
                self.de_mat[base..base + r].copy_from_slice(&de_col);
            }
            for rep in 0..r {
                let de = &self.de_mat;
                // ±0.0 canonicalization keeps the ranking backend-agnostic.
                let key = |site: usize| {
                    let v = de[site * r + rep];
                    if v == 0.0 {
                        0.0
                    } else {
                        v
                    }
                };
                self.order
                    .sort_by(|&a, &b| key(a).total_cmp(&key(b)).then(a.cmp(&b)));
                let u: f64 = rng.gen();
                let k = ((n as f64) * u.powf(Self::BIAS)) as usize;
                self.pick[rep] = self.order[k.min(n - 1)];
            }
            for site in 0..n {
                self.mask.clear();
                let mut hit = false;
                for rep in 0..r {
                    if self.pick[rep] == site {
                        self.mask.set(rep);
                        report.accepted += 1;
                        hit = true;
                    }
                }
                if hit {
                    state.apply_flips(site, &self.mask);
                }
            }
            report.proposed += r as u64;

            // ---- Phase 2: Metropolis restricted to N(M) ----------------------
            // Region = M ∪ neighbors(M), union over replicas, ascending order
            // (deterministic and identical on every backend: `neighbors` is
            // pure topology).
            self.region.clear();
            for rep in 0..r {
                let p = self.pick[rep];
                if !self.in_region[p] {
                    self.in_region[p] = true;
                    self.region.push(p);
                }
                for &j in state.neighbors(p) {
                    let j = j as usize;
                    if !self.in_region[j] {
                        self.in_region[j] = true;
                        self.region.push(j);
                    }
                }
            }
            self.region.sort_unstable();
            for &site in &self.region {
                self.in_region[site] = false; // reset for the next round
                state.delta_e_into(site, &mut self.de);
                self.mask.clear();
                for rep in 0..r {
                    let d = self.de[rep];
                    let t = temps[rep % temps.len()];
                    // Same acceptance law and RNG discipline as the parent
                    // MetropolisSweep (always draw ⇒ aligned streams).
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
                    }
                }
                if !self.mask.is_empty() {
                    state.apply_flips(site, &self.mask);
                }
            }
        }
        report.work = self
            .cost_model(InstanceShape {
                n,
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
    use crate::engine_v2::backends::{DenseByteState, ReferenceState, SparseBitSlice};
    use crate::engine_v2::ir::ProblemIR;
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

    fn view<'a>(r: usize, temps: &'a [f64]) -> RuntimeView<'a> {
        RuntimeView {
            iteration: 0,
            temperatures: temps,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        }
    }

    #[test]
    fn identical_across_all_three_backends() {
        let ir = ir7();
        let r = 40;
        let init = [1u8, 0, 1, 0, 0, 1, 0];
        let temps = [2.0, 1.0, 0.5, 0.1];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        let mut dense = DenseByteState::new(&ir, r, &init);
        for state in [
            &mut refs as &mut dyn SpinState,
            &mut bs as &mut dyn SpinState,
            &mut dense as &mut dyn SpinState,
        ] {
            ExtremalMetropolis::new().apply(
                state,
                &view(r, &temps),
                &mut ChaCha8Rng::seed_from_u64(5),
                Budget { sweeps: 15 },
            );
        }
        let mut e_ref = vec![0.0; r];
        let mut e_bs = vec![0.0; r];
        let mut e_dn = vec![0.0; r];
        refs.energies_into(&mut e_ref);
        bs.energies_into(&mut e_bs);
        dense.energies_into(&mut e_dn);
        assert_eq!(e_ref, e_bs);
        assert_eq!(e_ref, e_dn);
        // Configurations must agree exactly. (Digests hash different byte
        // layouts on the bit-sliced backend, so compare extracted spins.)
        let mut xa = vec![0u8; 7];
        let mut xb = vec![0u8; 7];
        for rep in 0..r {
            refs.extract_into(rep, &mut xa);
            bs.extract_into(rep, &mut xb);
            assert_eq!(xa, xb, "replica {rep} differs on SparseBitSlice");
            dense.extract_into(rep, &mut xb);
            assert_eq!(xa, xb, "replica {rep} differs on DenseByte");
        }
        assert_eq!(refs.audit(), 0.0);
        assert_eq!(bs.audit(), 0.0);
        assert_eq!(dense.audit(), 0.0);
    }

    #[test]
    fn same_seed_replays_and_relaxation_tracks_the_disturbed_region() {
        let ir = ir7();
        let r = 8;
        let init = [0u8; 7];
        let temps = [0.5];
        let run = || {
            let mut st = ReferenceState::new(&ir, r, &init);
            let rep = ExtremalMetropolis::new().apply(
                &mut st,
                &view(r, &temps),
                &mut ChaCha8Rng::seed_from_u64(11),
                Budget { sweeps: 10 },
            );
            let mut e = vec![0.0; r];
            st.energies_into(&mut e);
            (e, st.digest(), rep.proposed, rep.accepted)
        };
        let (e1, d1, p1, a1) = run();
        let (e2, d2, p2, a2) = run();
        assert_eq!(e1, e2);
        assert_eq!(d1, d2);
        assert_eq!((p1, a1), (p2, a2));
        // Phase 2 really proposed local moves beyond the r extremal flips.
        assert!(p1 > 10 * r as u64, "phase 2 must propose in N(M): {p1}");
    }
}
