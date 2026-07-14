//! `ExtremalOptimization` — τ-EO (Boettcher & Percus). Rather than accept/reject
//! a random flip, EO ranks the variables by how "unhappy" they are and flips a
//! rank-selected one — ALWAYS moving, even out of a local minimum, which is what
//! lets it cross barriers a Metropolis step cannot. One "sweep" is one extremal
//! update round (one flip per replica). Deterministic given the seeded RNG.
//!
//! Fitness uses only ΔE (no coupling access): the most-negative ΔE is the site
//! most "out of place". A power-law over the ascending-ΔE rank biases selection
//! toward those sites while still occasionally moving well-placed ones.

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
pub struct ExtremalOptimization {
    /// ΔE for every (site, replica), row-major by site.
    de_mat: Vec<f64>,
    order: Vec<usize>,
    pick: Vec<usize>, // chosen site per replica this round
    mask: ReplicaMask,
    ready: bool,
}

impl ExtremalOptimization {
    pub const NAME: &'static str = "extremal_optimization";
    /// Rank bias exponent: higher ⇒ stronger preference for the worst site.
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
        self.mask = ReplicaMask::new(r);
        self.ready = true;
    }
}

impl Operator for ExtremalOptimization {
    fn descriptor(&self) -> OperatorDescriptor {
        OperatorDescriptor {
            name: Self::NAME,
            nature: Nature::TrajectoryPreserving,
            capabilities: CapabilitySet::empty()
                .with(Capability::BarrierCrossing)
                .with(Capability::Exploration),
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                needs_replicas: false,
                needs_temperature: false,
            },
            observables: ObservableSet::empty()
                .with(Observable::Acceptance)
                .with(Observable::EnergyDelta),
            guarantees: Guarantees::deterministic(),
            complexity: Complexity::Superlinear,
        }
    }

    fn cost_model(&self, shape: InstanceShape) -> CostEstimate {
        // per round: fill ΔE (n·r) + per-replica sort (~r·n·log n)
        let n = shape.n.max(1) as f64;
        CostEstimate {
            work_per_sweep: n * shape.num_replicas.max(1) as f64 * n.log2().max(1.0),
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
        let (n, r) = (state.num_vars(), state.num_replicas());
        if n == 0 {
            return Report::default();
        }
        let mut report = Report::default();
        let mut de_col = vec![0.0f64; r]; // scratch reused each site
        for _ in 0..budget.sweeps {
            // Fill ΔE for all sites.
            for site in 0..n {
                state.delta_e_into(site, &mut de_col);
                let base = site * r;
                self.de_mat[base..base + r].copy_from_slice(&de_col);
            }
            // Per replica: rank sites by ascending ΔE, pick via power law.
            for rep in 0..r {
                let de = &self.de_mat;
                // Canonicalize the key: the two backends can produce −0.0 vs
                // +0.0 for a zero ΔE, which `total_cmp` would order differently.
                // Mapping every zero to +0.0 keeps the ranking backend-agnostic.
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
            // Apply one flip per replica, grouped by chosen site.
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
        }
        report.proposed = (budget.sweeps as u64) * (r as u64); // one move per replica per round
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
    use crate::engine_v2::backends::{ReferenceState, SparseBitSlice};
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

    #[test]
    fn identical_across_backends() {
        let ir = ir7();
        let r = 40;
        let init = [1u8, 0, 1, 0, 0, 1, 0];
        let mut refs = ReferenceState::new(&ir, r, &init);
        let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
        let view = RuntimeView {
            iteration: 0,
            temperatures: &[],
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        ExtremalOptimization::new().apply(
            &mut refs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(2),
            Budget { sweeps: 15 },
        );
        ExtremalOptimization::new().apply(
            &mut bs,
            &view,
            &mut ChaCha8Rng::seed_from_u64(2),
            Budget { sweeps: 15 },
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
