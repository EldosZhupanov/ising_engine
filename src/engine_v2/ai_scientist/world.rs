//! World Model (roadmap: the fourth model — "what will the system DO if I apply
//! this operator?"). This is the model-based-RL / Dreamer idea, scoped
//! honestly: it does NOT predict the literal spin configuration (n×r bits —
//! infeasible and pointless). It predicts the REDUCED OBSERVABLE STATE the
//! Runtime already summarizes — best energy, mean energy, energy entropy,
//! replica diversity — as a function of (current observable, operator,
//! temperature, sweeps). Composed step by step, it "imagines" a whole
//! schedule's trajectory WITHOUT running the Runtime, so the Evolution Engine
//! can rank thousands of candidate schedules in its head and only execute the
//! promising few.
//!
//! Its value is not assumed — it is MEASURED. [`rank_correlation`] runs a set
//! of schedules for real and imagined, and reports the Spearman correlation of
//! the two rankings. A world model is only worth using as a filter to the
//! extent that number is high; if it is near zero on your instances, this
//! module tells you so rather than pretending.
//!
//! Dependency-free and deterministic: four ridge regressions (one per
//! observable) over engineered transition features.

use super::super::context::RunContext;
use super::super::evolution::{boxed_state, Schedule};
use super::super::ir::ProblemIR;
use super::super::registry::OperatorRegistry;
use super::super::runtime::Runtime;
use super::executor::{lower_public, ExperimentTask};
use super::predictor::ridge_fit;
use std::collections::BTreeSet;

/// The reduced observable state — the world model's "latent". Energies are
/// stored already normalized by the instance scale so one model spans sizes.
#[derive(Debug, Clone, Copy)]
pub struct ObsState {
    pub best: f64,
    pub mean: f64,
    pub entropy: f64,
    pub diversity: f64,
}

/// Structural energy scale of an instance (always > 0, ~ magnitude of a typical
/// energy), so normalized observables are O(1) and comparable across instances.
fn escale(ir: &ProblemIR) -> f64 {
    let lin: f64 = ir.linear.iter().map(|v| v.abs()).sum();
    let quad: f64 = ir.weights.iter().map(|v| v.abs()).sum::<f64>() * 0.5;
    1.0 + lin + quad
}

/// The observable state before any operator runs: the all-zero start. Cheap and
/// exact — no Runtime needed, which is the whole point of "imagining".
fn initial_obs(ir: &ProblemIR) -> ObsState {
    let e = ir.offset / escale(ir);
    ObsState {
        best: e,
        mean: e,
        entropy: 0.0,
        diversity: 0.0,
    }
}

/// One observed transition (state, action) → next state, for training.
struct Transition {
    from: ObsState,
    op: String,
    temp_hi: f64,
    sweeps: u32,
    to: ObsState,
}

/// Capture a schedule's real transitions on the Runtime (read-only core;
/// deterministic in `seed`). `lower_public` runs everything in one phase in
/// declared order, so `events[i]` corresponds to `schedule.ops[i]`.
fn capture_transitions(
    ir: &ProblemIR,
    registry: &OperatorRegistry,
    schedule: &Schedule,
    num_replicas: usize,
    seed: u64,
) -> Vec<Transition> {
    let task = ExperimentTask {
        schedule: schedule.clone(),
        num_replicas,
        seed,
    };
    let plan = lower_public(ir, &task);
    let init = vec![0u8; ir.n];
    let mut state = boxed_state(ir, plan.backend, num_replicas, &init);
    let mut rt = Runtime::new(RunContext::new(seed), &plan);
    let scale = escale(ir);
    let mut out = Vec::new();
    if let Ok(rec) = rt.run(&plan, state.as_mut(), registry, ir) {
        let mut from = initial_obs(ir);
        let temp_hi = *plan.temperatures.first().unwrap_or(&1.0);
        for (i, e) in rec.events.iter().enumerate() {
            let to = ObsState {
                best: e.best_energy / scale,
                mean: e.metrics.mean_energy / scale,
                entropy: e.metrics.energy_entropy,
                diversity: e.metrics.diversity,
            };
            let sweeps = schedule.sweeps.get(i).copied().unwrap_or(1);
            out.push(Transition {
                from,
                op: e.operator.clone(),
                temp_hi,
                sweeps,
                to,
            });
            from = to;
        }
    }
    out
}

/// Learned transition model over the reduced observable state.
#[derive(Debug, Clone)]
pub struct WorldModel {
    vocab: Vec<String>,
    /// One weight vector per observable (best, mean, entropy, diversity),
    /// predicting the DELTA from the current observable.
    w_best: Vec<f64>,
    w_mean: Vec<f64>,
    w_entropy: Vec<f64>,
    w_diversity: Vec<f64>,
    pub trained_on: usize,
}

fn transition_features(
    vocab: &[String],
    s: &ObsState,
    op: &str,
    temp_hi: f64,
    sweeps: u32,
) -> Vec<f64> {
    let mut x = Vec::with_capacity(6 + vocab.len());
    x.push(s.best);
    x.push(s.mean);
    x.push(s.entropy);
    x.push(s.diversity);
    x.push(temp_hi / 10.0);
    x.push(((sweeps as f64) + 1.0).ln() / 5.0);
    for name in vocab {
        x.push(if name == op { 1.0 } else { 0.0 });
    }
    x.push(1.0); // bias
    x
}

impl WorldModel {
    /// Fit on the transitions of `schedules` captured across `(ir, seed)` pairs.
    /// `None` if too few transitions to trust.
    pub fn fit(
        instances: &[&ProblemIR],
        registry: &OperatorRegistry,
        schedules: &[Schedule],
        num_replicas: usize,
        seeds: &[u64],
        lambda: f64,
    ) -> Option<Self> {
        let mut trans = Vec::new();
        for ir in instances {
            for sched in schedules {
                for &seed in seeds {
                    trans.extend(capture_transitions(ir, registry, sched, num_replicas, seed));
                }
            }
        }
        if trans.len() < 20 {
            return None;
        }
        let vocab: Vec<String> = trans
            .iter()
            .map(|t| t.op.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let rows: Vec<Vec<f64>> = trans
            .iter()
            .map(|t| transition_features(&vocab, &t.from, &t.op, t.temp_hi, t.sweeps))
            .collect();
        let d_best: Vec<f64> = trans.iter().map(|t| t.to.best - t.from.best).collect();
        let d_mean: Vec<f64> = trans.iter().map(|t| t.to.mean - t.from.mean).collect();
        let d_ent: Vec<f64> = trans
            .iter()
            .map(|t| t.to.entropy - t.from.entropy)
            .collect();
        let d_div: Vec<f64> = trans
            .iter()
            .map(|t| t.to.diversity - t.from.diversity)
            .collect();
        Some(Self {
            w_best: ridge_fit(&rows, &d_best, lambda)?,
            w_mean: ridge_fit(&rows, &d_mean, lambda)?,
            w_entropy: ridge_fit(&rows, &d_ent, lambda)?,
            w_diversity: ridge_fit(&rows, &d_div, lambda)?,
            vocab,
            trained_on: trans.len(),
        })
    }

    fn dot(w: &[f64], x: &[f64]) -> f64 {
        w.iter().zip(x).map(|(a, b)| a * b).sum()
    }

    /// Imagine one step: predict the next observable state from the current one
    /// and an action, WITHOUT the Runtime. `best` is clamped monotone (a real
    /// best-so-far never increases), which keeps composed rollouts physical.
    pub fn step(&self, s: &ObsState, op: &str, temp_hi: f64, sweeps: u32) -> ObsState {
        let x = transition_features(&self.vocab, s, op, temp_hi, sweeps);
        let next_best = (s.best + Self::dot(&self.w_best, &x)).min(s.best);
        ObsState {
            best: next_best,
            mean: s.mean + Self::dot(&self.w_mean, &x),
            entropy: (s.entropy + Self::dot(&self.w_entropy, &x)).max(0.0),
            diversity: (s.diversity + Self::dot(&self.w_diversity, &x)).clamp(0.0, 1.0),
        }
    }

    /// Imagine a whole schedule from the analytic initial state and return the
    /// predicted FINAL best energy (denormalized to raw units). No Runtime.
    pub fn imagine_final_best(&self, ir: &ProblemIR, schedule: &Schedule) -> f64 {
        let mut s = initial_obs(ir);
        for (op, &sw) in schedule.ops.iter().zip(&schedule.sweeps) {
            s = self.step(&s, op, schedule.temp_hi, sw);
        }
        s.best * escale(ir)
    }

    /// Predicted single-step yield (raw energy improvement, ≥ 0) of applying
    /// `op` once from the analytic initial state. The World model's
    /// contribution to the shared Meta-Learning Layer: which operators it
    /// expects to pay off on this instance — imagined, no Runtime.
    pub fn predicted_yield(&self, ir: &ProblemIR, op: &str, temp_hi: f64, sweeps: u32) -> f64 {
        let s0 = initial_obs(ir);
        let s1 = self.step(&s0, op, temp_hi, sweeps);
        ((s0.best - s1.best) * escale(ir)).max(0.0)
    }

    /// Operators known to the world model (training vocabulary).
    pub fn vocab(&self) -> &[String] {
        &self.vocab
    }
}

/// The HONEST verification: rank a set of candidate schedules by imagined final
/// best and by REAL Runtime final best (mean over seeds), and return the
/// Spearman correlation of the two rankings. 1.0 = the world model orders
/// candidates exactly as reality does (a perfect cheap filter); ~0 = it does
/// not, and should not be trusted. Also returns (imagined, real) pairs so
/// callers can report the raw numbers.
pub fn rank_correlation(
    ir: &ProblemIR,
    registry: &OperatorRegistry,
    model: &WorldModel,
    candidates: &[Schedule],
    num_replicas: usize,
    seeds: &[u64],
) -> (f64, Vec<(f64, f64)>) {
    use super::evaluation::spearman;
    use super::executor::{BatchExecutor, RuntimeExecutor};
    let exec = RuntimeExecutor::new(1);
    let mut imagined = Vec::with_capacity(candidates.len());
    let mut real = Vec::with_capacity(candidates.len());
    for sched in candidates {
        imagined.push(model.imagine_final_best(ir, sched));
        let tasks: Vec<ExperimentTask> = seeds
            .iter()
            .map(|&seed| ExperimentTask {
                schedule: sched.clone(),
                num_replicas,
                seed,
            })
            .collect();
        let out = exec.run_batch(ir, registry, &tasks);
        real.push(out.iter().map(|o| o.score).sum::<f64>() / out.len().max(1) as f64);
    }
    let rho = spearman(&imagined, &real);
    (rho, imagined.into_iter().zip(real).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(n: usize, shift: f64) -> ProblemIR {
        let mut pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
            .map(|i| (i, i + 1, if i % 2 == 0 { -1.0 } else { 1.0 }))
            .collect();
        pairs.push((0, n as u32 - 1, -1.0));
        let linear: Vec<f64> = (0..n).map(|i| ((i as f64) * shift).cos() * 0.5).collect();
        ProblemIR::from_pairs(n, 0.0, linear, &pairs)
    }

    fn sched(ops: &[&str], sweeps: u32) -> Schedule {
        Schedule {
            ops: ops.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![sweeps; ops.len()],
            temp_hi: 3.0,
            temp_lo: 0.1,
        }
    }

    #[test]
    fn world_model_imagined_ranking_correlates_with_reality() {
        let reg = OperatorRegistry::standard();
        // A spread of schedules with clearly different quality: pure noise,
        // pure quench, thermal+quench, extremal+quench.
        let train_scheds = vec![
            sched(&["random_flip_sweep"], 4),
            sched(&["greedy_descent"], 4),
            sched(&["metropolis_sweep", "greedy_descent"], 4),
            sched(&["gibbs_color_sweep", "greedy_descent"], 4),
            sched(&["extremal_optimization", "greedy_descent"], 4),
        ];
        let train: Vec<ProblemIR> = (0..5).map(|i| ring(30, 0.2 + i as f64 * 0.13)).collect();
        let refs: Vec<&ProblemIR> = train.iter().collect();
        let model = WorldModel::fit(&refs, &reg, &train_scheds, 8, &[1, 2, 3], 1e-4)
            .expect("enough transitions");
        assert!(model.trained_on > 100);

        // Held-out instance + held-out-ish candidate set: does the imagined
        // ordering match the real ordering?
        let held = ring(30, 4.7);
        let candidates = vec![
            sched(&["random_flip_sweep"], 4),
            sched(&["greedy_descent"], 4),
            sched(&["metropolis_sweep", "greedy_descent"], 4),
            sched(&["gibbs_color_sweep", "greedy_descent"], 4),
            sched(&["extremal_optimization", "metropolis_sweep"], 4),
        ];
        let (rho, pairs) = rank_correlation(&held, &reg, &model, &candidates, 8, &[10, 11, 12]);
        eprintln!("world-model rank Spearman = {rho:.3}; (imagined, real) = {pairs:?}");
        assert!(
            rho >= 0.5,
            "imagined ranking must track reality to be a useful filter: rho={rho}"
        );
    }

    #[test]
    fn imagination_is_deterministic_and_needs_no_runtime() {
        let reg = OperatorRegistry::standard();
        // Multi-operator schedules ⇒ several transitions each, comfortably over
        // the training-data floor.
        let train_scheds = vec![
            sched(
                &["metropolis_sweep", "greedy_descent", "metropolis_sweep"],
                4,
            ),
            sched(
                &["gibbs_color_sweep", "greedy_descent", "gibbs_color_sweep"],
                4,
            ),
        ];
        let train: Vec<ProblemIR> = (0..5).map(|i| ring(20, 0.3 + i as f64 * 0.2)).collect();
        let refs: Vec<&ProblemIR> = train.iter().collect();
        let model = WorldModel::fit(&refs, &reg, &train_scheds, 8, &[1, 2, 3], 1e-4).unwrap();
        let held = ring(20, 3.0);
        let s = sched(&["metropolis_sweep", "greedy_descent"], 4);
        let a = model.imagine_final_best(&held, &s);
        let b = model.imagine_final_best(&held, &s);
        assert_eq!(a.to_bits(), b.to_bits());
    }
}
