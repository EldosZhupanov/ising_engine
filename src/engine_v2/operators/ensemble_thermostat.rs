//! `EnsembleThermostat` — a per-SITE temperature driven by ensemble consensus.
//!
//! Every other thermal operator in the library gives each replica one
//! temperature and every site the same one. But the ensemble already knows more
//! than that. At site i the replicas vote:
//!
//! ```text
//!   m_i = <s_i> over replicas ∈ [−1, 1],   c_i = |m_i| ∈ [0, 1]
//! ```
//!
//! `c_i = 1` means every replica agrees on this variable; `c_i = 0` means the
//! ensemble is split. That statistic costs R bit reads per site — the same order
//! as the ΔE loop we already run — and until now it was discarded.
//!
//! This operator spends it, modulating the thermostat per site:
//!
//! ```text
//!   T_eff(i, r) = T_r · exp(λ · c_i)
//! ```
//!
//!  - **λ < 0** — consensus sites run COLDER. *Backbone freezing*: where the
//!    replicas agree, treat it as discovered structure and protect it, spending
//!    the noise budget on the undecided sites.
//!  - **λ > 0** — consensus sites run HOTTER. *Curiosity*: where every replica
//!    agrees, the ensemble holds no information at all about the other branch,
//!    and no amount of further independent sampling will ever test it, so the
//!    most informative move is to attack the consensus.
//!
//! Both readings are plausible and they are opposite, which is what makes λ an
//! experiment rather than a tuning knob. At **λ = 0** the modulation is exactly
//! `exp(0) = 1`, so the operator degenerates to `MetropolisSweep` bit-for-bit —
//! the null hypothesis lives inside the algorithm and is asserted by test.
//!
//! A site-dependent temperature is not the Gibbs measure of any Hamiltonian, so
//! this is `BehaviorChanging` (Constitution §4.11): it optimizes, it does not
//! sample, and its output is always canonically re-scored. The RNG is drawn once
//! per (site, replica) in fixed chromatic order, exactly as `MetropolisSweep`
//! does, so replay stays bit-identical on both backends (ADR-0004).

use crate::engine_v2::capability::{
    Capability, CapabilitySet, Complexity, Constraints, Guarantees, Observable, ObservableSet,
    OperatorDescriptor,
};
use crate::engine_v2::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
use crate::engine_v2::runtime::RuntimeView;
use crate::engine_v2::state::{ReplicaMask, SpinState};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

pub struct EnsembleThermostat {
    name: &'static str,
    lambda: f64,
    order: Vec<usize>,
    de: Vec<f64>,
    mask: ReplicaMask,
    ready: bool,
}

impl EnsembleThermostat {
    /// Consensus sites run colder — protect what the ensemble agrees on.
    pub const FREEZE: &'static str = "consensus_freeze";
    /// Consensus sites run hotter — attack what the ensemble agrees on.
    pub const SEEK: &'static str = "consensus_seek";

    /// A registered variant. `lambda` is fixed at construction so the registry
    /// exposes each thermostat as its own capability passport.
    pub fn new(name: &'static str, lambda: f64) -> Self {
        Self {
            name,
            lambda,
            order: Vec::new(),
            de: Vec::new(),
            mask: ReplicaMask::default(),
            ready: false,
        }
    }

    pub fn freeze() -> Self {
        Self::new(Self::FREEZE, -1.0)
    }

    pub fn seek() -> Self {
        Self::new(Self::SEEK, 1.0)
    }

    /// Chromatic site order, built once — identical to `MetropolisSweep`'s, so
    /// the λ = 0 degeneracy is exact rather than merely statistical.
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

impl Operator for EnsembleThermostat {
    fn descriptor(&self) -> OperatorDescriptor {
        // The passport reflects what the sign actually does: heating the
        // consensus is a barrier-crossing move, cooling it is refinement.
        let capabilities = if self.lambda > 0.0 {
            CapabilitySet::empty()
                .with(Capability::Exploration)
                .with(Capability::BarrierCrossing)
        } else {
            CapabilitySet::empty()
                .with(Capability::Exploitation)
                .with(Capability::Exploration)
        };
        OperatorDescriptor {
            name: self.name,
            // Site-dependent temperature is not any Hamiltonian's Gibbs measure.
            nature: Nature::BehaviorChanging,
            capabilities,
            constraints: Constraints {
                needs_integer: false,
                needs_float: false,
                supports_sparse: true,
                supports_dense: true,
                // With one replica the consensus is trivially 1 everywhere and
                // the operator degenerates to a constant rescale of T.
                needs_replicas: true,
                needs_temperature: true,
            },
            observables: ObservableSet::empty()
                .with(Observable::Acceptance)
                .with(Observable::EnergyDelta)
                .with(Observable::TemperatureDrift),
            guarantees: Guarantees {
                // Detailed balance is deliberately broken; it does not equilibrate.
                equilibrates: false,
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
        debug_assert!(!temps.is_empty(), "EnsembleThermostat needs a temperature");

        let mut report = Report::default();
        // Sensors: how much consensus the ensemble actually carried, and how far
        // that moved the thermostat. Both are diagnostic only.
        let (mut consensus_sum, mut scale_sum, mut samples) = (0.0f64, 0.0f64, 0u64);

        for _ in 0..budget.sweeps {
            for &site in &self.order {
                state.delta_e_into(site, &mut self.de);

                // The ensemble votes on this site. Read live, so the estimate
                // reflects the flips already made this sweep.
                let mut up = 0usize;
                for rep in 0..r {
                    if state.spin(site, rep) {
                        up += 1;
                    }
                }
                let m = (2.0 * up as f64 - r as f64) / r as f64;
                let consensus = m.abs();
                // λ = 0 ⇒ exp(0.0) == 1.0 exactly ⇒ t * 1.0 == t bit-for-bit.
                let scale = (self.lambda * consensus).exp();
                consensus_sum += consensus;
                scale_sum += scale;
                samples += 1;

                self.mask.clear();
                for rep in 0..r {
                    let d = self.de[rep];
                    let t = temps[rep % temps.len()] * scale;
                    let accept = if d <= 0.0 {
                        // Draw anyway to keep the RNG stream aligned with the
                        // λ = 0 reference trajectory.
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

        let denom = samples.max(1) as f64;
        report.aux.push(("mean_consensus", consensus_sum / denom));
        report.aux.push(("mean_temp_scale", scale_sum / denom));
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

    /// The null hypothesis is inside the algorithm: at λ = 0 the thermostat must
    /// reproduce `MetropolisSweep` EXACTLY, not merely on average. If this ever
    /// fails, every λ ≠ 0 comparison is measuring construction error instead of
    /// the consensus term.
    #[test]
    fn lambda_zero_is_bit_identical_to_metropolis() {
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
            MetropolisSweep::new().apply(&mut a, &view, &mut ChaCha8Rng::seed_from_u64(99), budget);
        let rb = EnsembleThermostat::new("ect_null", 0.0).apply(
            &mut b,
            &view,
            &mut ChaCha8Rng::seed_from_u64(99),
            budget,
        );

        assert_eq!(ra.accepted, rb.accepted);
        assert_eq!(ra.proposed, rb.proposed);
        let (mut ea, mut eb) = (vec![0.0; r], vec![0.0; r]);
        a.energies_into(&mut ea);
        b.energies_into(&mut eb);
        assert_eq!(ea, eb, "λ=0 diverged from MetropolisSweep");
        assert_eq!(a.digest(), b.digest(), "λ=0 reached a different state");
    }

    /// λ ≠ 0 must actually change the trajectory — otherwise the operator is an
    /// expensive alias and every experiment on it is vacuous.
    #[test]
    fn nonzero_lambda_changes_the_trajectory() {
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

        let mut null = ReferenceState::new(&ir, r, &init);
        let mut seek = ReferenceState::new(&ir, r, &init);
        EnsembleThermostat::new("ect_null", 0.0).apply(
            &mut null,
            &view,
            &mut ChaCha8Rng::seed_from_u64(7),
            budget,
        );
        EnsembleThermostat::seek().apply(
            &mut seek,
            &view,
            &mut ChaCha8Rng::seed_from_u64(7),
            budget,
        );
        assert_ne!(null.digest(), seek.digest());
    }

    /// The project-wide invariant: identical trajectories on both backends and
    /// zero ledger drift (ADR-0004).
    #[test]
    fn identical_on_both_backends() {
        let ir = frustrated();
        let (r, init) = (100, [1u8, 0, 1, 0, 0, 1, 0]);
        let t = temps(r);
        let view = RuntimeView {
            iteration: 0,
            temperatures: &t,
            num_replicas: r,
            recent_acceptance: 0.0,
            remaining_ms: f64::INFINITY,
        };
        let budget = Budget { sweeps: 30 };

        for mut op in [EnsembleThermostat::freeze(), EnsembleThermostat::seek()] {
            let mut refs = ReferenceState::new(&ir, r, &init);
            let mut bs = SparseBitSlice::new(&ir, r, &init).unwrap();
            let a = op.apply(&mut refs, &view, &mut ChaCha8Rng::seed_from_u64(11), budget);
            let mut op2 = EnsembleThermostat::new(
                op.name(),
                if op.name() == "consensus_seek" {
                    1.0
                } else {
                    -1.0
                },
            );
            let b = op2.apply(&mut bs, &view, &mut ChaCha8Rng::seed_from_u64(11), budget);

            assert_eq!(a.accepted, b.accepted);
            assert_eq!(a.proposed, b.proposed);
            let (mut e1, mut e2) = (vec![0.0; r], vec![0.0; r]);
            refs.energies_into(&mut e1);
            bs.energies_into(&mut e2);
            assert_eq!(e1, e2, "trajectories diverged between backends");
            assert_eq!(bs.audit(), 0.0);
            assert_eq!(refs.audit(), 0.0);
        }
    }

    /// Consensus must be a real, varying signal — a thermostat driven by a
    /// constant would be a relabelled Metropolis.
    #[test]
    fn consensus_sensor_is_informative() {
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
        let mut st = ReferenceState::new(&ir, r, &init);
        let rep = EnsembleThermostat::freeze().apply(
            &mut st,
            &view,
            &mut ChaCha8Rng::seed_from_u64(3),
            Budget { sweeps: 20 },
        );
        let c = rep
            .aux
            .iter()
            .find(|(k, _)| *k == "mean_consensus")
            .map(|(_, v)| *v)
            .expect("consensus sensor");
        // Started fully consensual (all replicas identical) and thermalised, so
        // the mean must have left both extremes.
        assert!(c > 0.0 && c < 1.0, "consensus was degenerate: {c}");
    }
}
