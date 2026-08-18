//! Dynamics Model (roadmap: the third model — "how will this run end?").
//!
//! It consumes the EXECUTION LOG the Runtime already produces — per-step best
//! energy, mean energy, entropy, replica diversity, acceptance — and predicts,
//! from a PARTIAL trajectory, how much improvement remains. That prediction
//! powers a learned [`EarlyStopController`]: when the model says the current
//! operator has plateaued and little is left, the Runtime either switches to
//! the next operator early or stops — turning saved sweeps into either speed or
//! budget for more promising work.
//!
//! Honest scope: the target is a SCALE-FREE remaining-improvement ratio
//! (relative to the current best), so one model transfers across instance
//! sizes. It's a ridge regression over engineered trajectory features — not a
//! deep sequence model — which is the right rung to prove the idea pays before
//! reaching for an RNN/transformer. Fully deterministic (fixed weights + the
//! deterministic sensors the Runtime exposes), so an early-stopped run still
//! replays bit-identically (ADR-0004).

use super::super::context::RunContext;
use super::super::decision::DecisionEngine;
use super::super::evolution::{boxed_state, Schedule};
use super::super::ir::ProblemIR;
use super::super::registry::OperatorRegistry;
use super::super::runtime::{QualityMetrics, RunControl, RunController, Runtime, StepSensors};
use super::executor::{lower_public, ExperimentTask};
use super::predictor::ridge_fit;

const N_INSTANCE_FEATS: usize = 5;
/// Window (in steps) over which "recent progress" is measured.
const WINDOW: usize = 2;

/// One observed step of a run's execution log.
#[derive(Debug, Clone, Copy)]
pub struct StepObs {
    pub best_energy: f64,
    pub mean_energy: f64,
    pub entropy: f64,
    pub diversity: f64,
    pub acceptance: f64,
    pub frac_elapsed: f64,
}

/// A captured trajectory: the instance it ran on plus its per-step log.
#[derive(Debug, Clone)]
pub struct Trajectory {
    pub features: [f64; N_INSTANCE_FEATS],
    pub steps: Vec<StepObs>,
}

fn instance_features(ir: &ProblemIR) -> [f64; N_INSTANCE_FEATS] {
    let s = DecisionEngine::analyze(ir);
    // Shared vocabulary (feature_registry v0), not a private copy.
    let sig = super::predictor::InstanceSignature {
        n: s.n,
        density: s.density,
        clustering: s.clustering,
        mean_degree: s.mean_degree,
        degree_cv: s.degree_cv,
    };
    let v = super::feature_registry::v0().encode(&sig);
    debug_assert_eq!(v.len(), N_INSTANCE_FEATS);
    let mut a = [0.0; N_INSTANCE_FEATS];
    a.copy_from_slice(&v[..N_INSTANCE_FEATS]);
    a
}

/// Run `schedule` on the real Runtime and capture its execution log. Uses only
/// read-only core components; deterministic in `seed`.
pub fn capture_trajectory(
    ir: &ProblemIR,
    registry: &OperatorRegistry,
    schedule: &Schedule,
    num_replicas: usize,
    seed: u64,
) -> Trajectory {
    let task = ExperimentTask {
        schedule: schedule.clone(),
        num_replicas,
        seed,
    };
    let plan = lower_public(ir, &task);
    let init = vec![0u8; ir.n];
    let mut state = boxed_state(ir, plan.backend, num_replicas, &init);
    let mut rt = Runtime::new(RunContext::new(seed), &plan);
    let steps = match rt.run(&plan, state.as_mut(), registry, ir) {
        Ok(rec) => {
            let total = rec.events.len().max(1) as f64;
            rec.events
                .iter()
                .enumerate()
                .map(|(i, e)| StepObs {
                    best_energy: e.best_energy,
                    mean_energy: e.metrics.mean_energy,
                    entropy: e.metrics.energy_entropy,
                    diversity: e.metrics.diversity,
                    acceptance: e.acceptance,
                    frac_elapsed: (i + 1) as f64 / total,
                })
                .collect()
        }
        Err(_) => Vec::new(),
    };
    Trajectory {
        features: instance_features(ir),
        steps,
    }
}

/// Feature vector for the step at index `k` of a best-energy history. Shared by
/// training and inference so the two never diverge.
#[allow(clippy::too_many_arguments)]
fn step_features(
    feats: &[f64; N_INSTANCE_FEATS],
    best_hist: &[f64],
    k: usize,
    mean_energy: f64,
    entropy: f64,
    diversity: f64,
    acceptance: f64,
    frac_elapsed: f64,
) -> Vec<f64> {
    let best_k = best_hist[k];
    let scale = best_k.abs().max(1.0);
    let prev = best_hist[k.saturating_sub(WINDOW)];
    let recent_progress = (prev - best_k) / scale; // ≥ 0, scale-free
    let spread = (mean_energy - best_k) / scale;
    let best_norm = best_k / (best_hist[0].abs() + 1.0);
    vec![
        feats[0],
        feats[1],
        feats[2],
        feats[3],
        feats[4],
        entropy,
        diversity,
        acceptance,
        frac_elapsed,
        recent_progress,
        spread,
        best_norm,
        1.0, // bias
    ]
}

/// Remaining-improvement predictor.
#[derive(Debug, Clone)]
pub struct DynamicsModel {
    weights: Vec<f64>,
    pub trained_on: usize,
}

impl DynamicsModel {
    /// Serialize weights for the Model Registry (round-trip f64 `Display`).
    /// Layout: `weights;trained_on`.
    pub fn to_weights_text(&self) -> String {
        let w = self
            .weights
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("{};{}", w, self.trained_on)
    }

    /// Reconstruct from [`Self::to_weights_text`]; `None` on a malformed payload.
    pub fn from_weights_text(s: &str) -> Option<Self> {
        let mut g = s.split(';');
        let w_s = g.next()?;
        let trained_on: usize = g.next()?.parse().ok()?;
        let weights: Vec<f64> = if w_s.is_empty() {
            return None;
        } else {
            w_s.split(',')
                .map(|x| x.parse().ok())
                .collect::<Option<_>>()?
        };
        Some(Self {
            weights,
            trained_on,
        })
    }

    /// Fit on captured trajectories. Each step contributes one example:
    /// features(step k) → (best_k − final_best)/|best_k|, the scale-free
    /// improvement STILL TO COME. `None` if there is too little data.
    pub fn fit(trajectories: &[Trajectory], lambda: f64) -> Option<Self> {
        let mut rows = Vec::new();
        let mut targets = Vec::new();
        for tr in trajectories {
            if tr.steps.len() < 2 {
                continue;
            }
            let best_hist: Vec<f64> = tr.steps.iter().map(|s| s.best_energy).collect();
            let final_best = *best_hist.last().unwrap();
            for (k, s) in tr.steps.iter().enumerate() {
                let x = step_features(
                    &tr.features,
                    &best_hist,
                    k,
                    s.mean_energy,
                    s.entropy,
                    s.diversity,
                    s.acceptance,
                    s.frac_elapsed,
                );
                let scale = best_hist[k].abs().max(1.0);
                targets.push((best_hist[k] - final_best) / scale);
                rows.push(x);
            }
        }
        if rows.len() < 20 {
            return None;
        }
        let n = rows.len();
        ridge_fit(&rows, &targets, lambda).map(|weights| Self {
            weights,
            trained_on: n,
        })
    }

    fn predict_raw(&self, x: &[f64]) -> f64 {
        x.iter().zip(&self.weights).map(|(a, b)| a * b).sum()
    }

    /// Fraction of the budget at which this model first predicts the remaining
    /// improvement has fallen below `epsilon` on the given captured trajectory —
    /// where the run PLATEAUS. Returns 1.0 if it never plateaus within the
    /// observed run. This is the Dynamics model's contribution to the shared
    /// Meta-Learning Layer ("runs of this operator front-load and plateau by
    /// X% of the budget → a switch-early candidate").
    pub fn plateau_frac(&self, tr: &Trajectory, epsilon: f64) -> f64 {
        if tr.steps.len() < 2 {
            return 1.0;
        }
        let best: Vec<f64> = tr.steps.iter().map(|s| s.best_energy).collect();
        for (k, s) in tr.steps.iter().enumerate() {
            let rem = self.predict_remaining(
                &tr.features,
                &best,
                k,
                s.mean_energy,
                s.entropy,
                s.diversity,
                s.acceptance,
                s.frac_elapsed,
            );
            if rem < epsilon {
                return s.frac_elapsed;
            }
        }
        1.0
    }

    /// Predicted remaining-improvement ratio given a best-energy history up to
    /// step `k` and the step's observables. Clamped to ≥ 0 (improvement can't
    /// be negative in a minimization).
    #[allow(clippy::too_many_arguments)]
    pub fn predict_remaining(
        &self,
        feats: &[f64; N_INSTANCE_FEATS],
        best_hist: &[f64],
        k: usize,
        mean_energy: f64,
        entropy: f64,
        diversity: f64,
        acceptance: f64,
        frac_elapsed: f64,
    ) -> f64 {
        let x = step_features(
            feats,
            best_hist,
            k,
            mean_energy,
            entropy,
            diversity,
            acceptance,
            frac_elapsed,
        );
        self.predict_raw(&x).max(0.0)
    }
}

/// Fit a `DynamicsModel` by capturing trajectories of `schedule` across the
/// given `(ir, seed)` pairs — a convenience for the platform's training loop.
pub fn train_on_instances(
    instances: &[&ProblemIR],
    registry: &OperatorRegistry,
    schedule: &Schedule,
    num_replicas: usize,
    seeds: &[u64],
    lambda: f64,
) -> Option<DynamicsModel> {
    let mut trajectories = Vec::new();
    for ir in instances {
        for &seed in seeds {
            trajectories.push(capture_trajectory(
                ir,
                registry,
                schedule,
                num_replicas,
                seed,
            ));
        }
    }
    DynamicsModel::fit(&trajectories, lambda)
}

/// What the controller does when the model predicts a plateau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlateauAction {
    /// Advance to the next distinct operator early.
    Switch,
    /// End the whole run early.
    Stop,
}

/// Learned early-stop / early-switch controller. Streams the best-energy
/// history, predicts remaining improvement each step, and once past
/// `min_frac` of the schedule acts when the prediction drops below `epsilon`.
/// Deterministic (fixed model + deterministic sensors) ⇒ replay-safe.
pub struct EarlyStopController {
    model: DynamicsModel,
    feats: [f64; N_INSTANCE_FEATS],
    best_hist: Vec<f64>,
    epsilon: f64,
    min_frac: f64,
    action: PlateauAction,
}

impl EarlyStopController {
    pub fn new(
        model: DynamicsModel,
        ir: &ProblemIR,
        epsilon: f64,
        min_frac: f64,
        action: PlateauAction,
    ) -> Self {
        Self {
            model,
            feats: instance_features(ir),
            best_hist: Vec::new(),
            epsilon,
            min_frac,
            action,
        }
    }
}

impl RunController for EarlyStopController {
    fn decide(&mut self, s: &StepSensors) -> RunControl {
        self.best_hist.push(s.best_energy);
        let k = self.best_hist.len() - 1;
        let QualityMetrics {
            mean_energy,
            energy_entropy,
            diversity,
            ..
        } = *s.metrics;
        let remaining = self.model.predict_remaining(
            &self.feats,
            &self.best_hist,
            k,
            mean_energy,
            energy_entropy,
            diversity,
            s.acceptance,
            s.frac_elapsed,
        );
        if s.frac_elapsed >= self.min_frac && remaining < self.epsilon {
            match self.action {
                PlateauAction::Switch => RunControl::SwitchOperator,
                PlateauAction::Stop => RunControl::Stop,
            }
        } else {
            RunControl::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_v2::ai_scientist::executor::{BatchExecutor, RuntimeExecutor};

    fn ring(n: usize, seed_shift: f64) -> ProblemIR {
        let mut pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
            .map(|i| (i, i + 1, if i % 2 == 0 { -1.0 } else { 1.0 }))
            .collect();
        pairs.push((0, n as u32 - 1, -1.0));
        let linear: Vec<f64> = (0..n).map(|i| ((i as f64) * seed_shift).sin()).collect();
        ProblemIR::from_pairs(n, 0.0, linear, &pairs)
    }

    fn quench_schedule(sweeps_each: u32, repeats: usize) -> Schedule {
        // A long single-operator quench: greedy descent plateaus quickly, so a
        // good dynamics model should recognize the tail is wasted.
        Schedule {
            ops: vec!["greedy_descent".to_string(); repeats],
            sweeps: vec![sweeps_each; repeats],
            temp_hi: 2.0,
            temp_lo: 0.05,
        }
    }

    #[test]
    fn model_learns_that_a_plateaued_greedy_run_has_little_left() {
        let reg = OperatorRegistry::standard();
        let sched = quench_schedule(2, 12);
        let train: Vec<ProblemIR> = (0..4).map(|i| ring(24, 0.3 + i as f64 * 0.2)).collect();
        let refs: Vec<&ProblemIR> = train.iter().collect();
        let model = train_on_instances(&refs, &reg, &sched, 8, &[1, 2, 3], 1e-4)
            .expect("enough trajectory data");
        assert!(model.trained_on > 50);

        // On a held-out instance, capture a trajectory and check the model's
        // late-step prediction is much smaller than its early-step prediction
        // (it has learned the run plateaus).
        let held = ring(24, 5.0);
        let tr = capture_trajectory(&held, &reg, &sched, 8, 99);
        let best: Vec<f64> = tr.steps.iter().map(|s| s.best_energy).collect();
        let early = tr.steps.len() / 6;
        let late = tr.steps.len() - 1;
        let pe = model.predict_remaining(
            &tr.features,
            &best,
            early,
            tr.steps[early].mean_energy,
            tr.steps[early].entropy,
            tr.steps[early].diversity,
            tr.steps[early].acceptance,
            tr.steps[early].frac_elapsed,
        );
        let pl = model.predict_remaining(
            &tr.features,
            &best,
            late,
            tr.steps[late].mean_energy,
            tr.steps[late].entropy,
            tr.steps[late].diversity,
            tr.steps[late].acceptance,
            tr.steps[late].frac_elapsed,
        );
        assert!(
            pe >= pl,
            "early prediction {pe} should be >= late prediction {pl}"
        );
        assert!(pl < 0.02, "late remaining-improvement should be tiny: {pl}");
    }

    #[test]
    fn weights_text_round_trips_exactly() {
        let reg = OperatorRegistry::standard();
        let sched = quench_schedule(2, 12);
        let train: Vec<ProblemIR> = (0..4).map(|i| ring(24, 0.3 + i as f64 * 0.2)).collect();
        let refs: Vec<&ProblemIR> = train.iter().collect();
        let model = train_on_instances(&refs, &reg, &sched, 8, &[1, 2, 3], 1e-4).unwrap();
        let round = DynamicsModel::from_weights_text(&model.to_weights_text()).unwrap();
        assert_eq!(round.trained_on, model.trained_on);
        // Bit-identical remaining-improvement predictions → a reloaded registry
        // snapshot is exactly the same model (registry correctness).
        let held = ring(24, 5.0);
        let tr = capture_trajectory(&held, &reg, &sched, 8, 99);
        let best: Vec<f64> = tr.steps.iter().map(|s| s.best_energy).collect();
        for k in [0, tr.steps.len() / 2, tr.steps.len() - 1] {
            let s = &tr.steps[k];
            let a = model.predict_remaining(
                &tr.features,
                &best,
                k,
                s.mean_energy,
                s.entropy,
                s.diversity,
                s.acceptance,
                s.frac_elapsed,
            );
            let b = round.predict_remaining(
                &tr.features,
                &best,
                k,
                s.mean_energy,
                s.entropy,
                s.diversity,
                s.acceptance,
                s.frac_elapsed,
            );
            assert_eq!(a.to_bits(), b.to_bits(), "step {k} prediction diverged");
        }
        assert!(DynamicsModel::from_weights_text("").is_none());
    }

    #[test]
    fn early_stop_saves_steps_without_losing_much_quality_and_replays() {
        let reg = OperatorRegistry::standard();
        let sched = quench_schedule(2, 16);
        let train: Vec<ProblemIR> = (0..5).map(|i| ring(28, 0.25 + i as f64 * 0.17)).collect();
        let refs: Vec<&ProblemIR> = train.iter().collect();
        let model = train_on_instances(&refs, &reg, &sched, 8, &[1, 2, 3], 1e-4).unwrap();

        let held = ring(28, 4.2);
        let seed = 7u64;

        // Baseline: full run via the executor.
        let exec = RuntimeExecutor::new(1);
        let full = &exec.run_batch(
            &held,
            &reg,
            &[ExperimentTask {
                schedule: sched.clone(),
                num_replicas: 8,
                seed,
            }],
        )[0];

        // Early-stopped run with the learned controller.
        let run_early = |m: DynamicsModel| {
            let task = ExperimentTask {
                schedule: sched.clone(),
                num_replicas: 8,
                seed,
            };
            let plan = lower_public(&held, &task);
            let init = vec![0u8; held.n];
            let mut state = boxed_state(&held, plan.backend, 8, &init);
            let ctrl = EarlyStopController::new(m, &held, 0.01, 0.3, PlateauAction::Stop);
            let mut rt = Runtime::new(RunContext::new(seed), &plan).with_controller(Box::new(ctrl));
            rt.run(&plan, state.as_mut(), &reg, &held).unwrap()
        };
        let early = run_early(model.clone());

        // Fewer steps than the full 16, and quality within a small tolerance.
        assert!(
            (early.iterations as usize) < 16,
            "early stop must cut steps: ran {}",
            early.iterations
        );
        assert!(
            !early.adaptations.is_empty(),
            "a stop decision was recorded"
        );
        let quality_loss = early.best_energy - full.score; // ≥ 0 (worse = higher)
        let tol = 0.02 * full.score.abs().max(1.0);
        assert!(
            quality_loss <= tol,
            "early stop lost too much quality: {} vs full {} (tol {tol})",
            early.best_energy,
            full.score
        );

        // Deterministic replay: same seed + same model ⇒ identical run.
        let early2 = run_early(model);
        assert_eq!(early.best_energy.to_bits(), early2.best_energy.to_bits());
        assert_eq!(early.iterations, early2.iterations);
        assert_eq!(early.adaptations, early2.adaptations);
    }
}
