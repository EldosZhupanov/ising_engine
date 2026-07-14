//! `Runtime` — execution control, entirely separate from the operators
//! (Constitution §7, §11; user architecture directive). The Runtime owns:
//! temperature ladder, replica count, iteration counter, wall-clock, per-run
//! statistics, the event log, adaptation hooks, and the RNG. It drives the
//! `Scheduler` (which decides sequencing) and hands each operator a read-only
//! `RuntimeView`.
//!
//! Layering:  Decision → Runtime → Scheduler → Operator.
//!
//! An operator therefore never asks "how much time has passed" or "what
//! temperature am I at" — the Runtime tells it. The operator stays pure.

use super::context::RunContext;
use super::ir::ProblemIR;
use super::plan::Plan;
use super::registry::OperatorRegistry;
use super::scheduler::{PhaseScheduler, Scheduler};
use super::state::SpinState;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::time::Instant;

/// The read-only snapshot an operator sees. It exposes runtime state but grants
/// no control. `remaining_ms` is a guard: a trajectory-preserving operator must
/// not branch on it (doing so makes the operator BehaviorChanging — ADR-0004).
pub struct RuntimeView<'a> {
    pub iteration: u64,
    pub temperatures: &'a [f64],
    pub num_replicas: usize,
    pub recent_acceptance: f64,
    pub remaining_ms: f64,
}

/// Ensemble quality snapshot the Runtime measures AFTER each operator step —
/// the observables a utility-based Decision Engine (§10, §11) will consume.
/// Computed by reading the state only; diagnostic, never trajectory-affecting
/// (ADR-0004).
#[derive(Debug, Clone, Copy, Default)]
pub struct QualityMetrics {
    pub best_energy: f64,
    pub mean_energy: f64,
    /// Shannon entropy (bits) of the per-replica energy distribution. 0 ⇒ the
    /// ensemble has collapsed to a single energy; grows as it spreads out.
    pub energy_entropy: f64,
    /// Mean pairwise Hamming diversity over a sampled disjoint replica set,
    /// in [0, 1]. 0 ⇒ replicas identical, 1 ⇒ complementary. Sampling keeps it
    /// O(pairs·n), independent of replica count.
    pub diversity: f64,
}

/// One execution event — the run stays a reproducible experiment (§11): every
/// operator invocation is logged with what it did (acceptance, quality) and
/// WHY it ran (`decision`), so the whole run is explainable after the fact.
#[derive(Debug, Clone)]
pub struct StepEvent {
    pub iteration: u64,
    pub operator: String,
    pub acceptance: f64,
    pub best_energy: f64,
    /// Post-step ensemble quality (energy spread, replica diversity).
    pub metrics: QualityMetrics,
    /// Human-readable rationale for selecting this operator this step. Today the
    /// schedule is plan-driven; the utility Decision Engine (§10) fills this
    /// with its α/β/γ/δ breakdown when it lands.
    pub decision: String,
}

/// Structured output of a run, destined for the result database and the
/// knowledge system (Constitution §12).
#[derive(Debug, Clone)]
pub struct RunRecord {
    pub plan_name: String,
    pub seed: u64,
    pub iterations: u64,
    pub best_energy: f64,
    pub best_state: Vec<u8>,
    pub events: Vec<StepEvent>,
    /// Every adaptation decision the Runtime took, in order (empty when
    /// adaptation is off). Part of the replay contract: same seed + same
    /// config ⇒ same decisions (ADR-0004).
    pub adaptations: Vec<String>,
    /// Canonical re-score of `best_state` against the bare IR (ADR-0005). MUST
    /// equal `best_energy`; any mismatch is a ledger/pass bug.
    pub canonical_energy: f64,
}

/// Sensors handed to an external [`RunController`] after every step — the
/// deterministic, read-only signals a learned early-stop / early-switch model
/// is allowed to see (Constitution §11; ADR-0004 replay contract). A
/// controller that reads only these and holds fixed weights keeps the run
/// bit-identical under the same seed.
pub struct StepSensors<'a> {
    pub iteration: u64,
    /// Fraction of the plan's scheduled steps already executed, in [0, 1].
    pub frac_elapsed: f64,
    pub operator: &'a str,
    /// Best energy so far and the current ensemble quality.
    pub best_energy: f64,
    pub metrics: &'a QualityMetrics,
    pub acceptance: f64,
}

/// A controller's verdict after a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunControl {
    /// Keep executing the plan as scheduled.
    Continue,
    /// Abandon the rest of the CURRENT operator's queued steps; advance to the
    /// next distinct operator (early operator switching).
    SwitchOperator,
    /// End the run now (early stopping).
    Stop,
}

/// The plug-in point for a LEARNED run controller (the Dynamics-model early
/// stopper lives above the core and implements this — the core never depends
/// on the learning layer). `decide` must be a deterministic function of the
/// sensors for the run to stay replayable.
pub trait RunController {
    fn decide(&mut self, sensors: &StepSensors) -> RunControl;
}

/// Adaptive-controller configuration (Runtime v0.5, Constitution §11).
/// OPT-IN: `enabled: false` reproduces the plan-driven Runtime exactly. Every
/// controller reads only deterministic sensors (seeded-trajectory acceptance,
/// ledger energies, quality metrics), so adaptive runs replay bit-identically
/// under the same seed and config (ADR-0004).
#[derive(Debug, Clone)]
pub struct AdaptationConfig {
    pub enabled: bool,
    /// Ladder controller: EMA acceptance below `accept_lo` heats the ladder by
    /// `heat_factor`; above `accept_hi` cools it by `cool_factor`.
    pub accept_lo: f64,
    pub accept_hi: f64,
    pub heat_factor: f64,
    pub cool_factor: f64,
    /// Never heat beyond `max_heat`× the plan's initial hottest rung.
    pub max_heat: f64,
    /// UCB1 exploration constant for the operator bandit.
    pub bandit_c: f64,
    /// Sensors: ensemble counts as collapsed when energy entropy AND replica
    /// diversity both fall below these floors — Explore is then cut short.
    pub collapse_entropy: f64,
    pub collapse_diversity: f64,
}

impl Default for AdaptationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            accept_lo: 0.05,
            accept_hi: 0.70,
            heat_factor: 1.2,
            cool_factor: 0.85,
            max_heat: 8.0,
            bandit_c: 1.4,
            collapse_entropy: 0.02,
            collapse_diversity: 0.02,
        }
    }
}

impl AdaptationConfig {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }
}

pub struct Runtime {
    ctx: RunContext,
    temperatures: Vec<f64>,
    num_replicas: usize,
    iteration: u64,
    recent_acceptance: f64,
    rng: ChaCha8Rng,
    adapt: AdaptationConfig,
    /// Bandit statistics per operator: (total reward, pulls). Read through the
    /// SORTED pool only — never iterated as a map — so decisions are
    /// deterministic.
    op_reward: HashMap<String, (f64, u64)>,
    /// The bandit's current recommendation for the next Exploit step.
    bandit_choice: Option<String>,
    adaptations: Vec<String>,
    initial_temp_hi: f64,
    /// Optional learned early-stop / early-switch controller (opt-in). Reads
    /// only deterministic sensors, so replay stays bit-identical.
    controller: Option<Box<dyn RunController>>,
}

impl Runtime {
    pub fn new(ctx: RunContext, plan: &Plan) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(plan.seed);
        let initial_temp_hi = plan.temperatures.first().copied().unwrap_or(1.0);
        Self {
            ctx,
            temperatures: plan.temperatures.clone(),
            num_replicas: plan.num_replicas,
            iteration: 0,
            recent_acceptance: 0.0,
            rng,
            adapt: AdaptationConfig::default(),
            op_reward: HashMap::new(),
            bandit_choice: None,
            adaptations: Vec::new(),
            initial_temp_hi,
            controller: None,
        }
    }

    /// Turn the v0.5 adaptive controllers on (or reconfigure them). Replay
    /// contract: the SAME config + seed + plan reproduces the identical run.
    pub fn with_adaptation(mut self, cfg: AdaptationConfig) -> Self {
        self.adapt = cfg;
        self
    }

    /// Attach a learned early-stop / early-switch controller (opt-in). It sees
    /// only deterministic per-step sensors, so a deterministic controller keeps
    /// the run replayable (ADR-0004). Its decisions are recorded in
    /// `RunRecord.adaptations`.
    pub fn with_controller(mut self, controller: Box<dyn RunController>) -> Self {
        self.controller = Some(controller);
        self
    }

    pub fn context(&self) -> &RunContext {
        &self.ctx
    }

    pub fn context_mut(&mut self) -> &mut RunContext {
        &mut self.ctx
    }

    /// Execute `plan` over `state`, drawing operators from `registry`. `ir` is
    /// used only for the final canonical re-score (ADR-0005) — the trajectory
    /// is driven entirely through the `SpinState` ledger.
    pub fn run(
        &mut self,
        plan: &Plan,
        state: &mut dyn SpinState,
        registry: &OperatorRegistry,
        ir: &ProblemIR,
    ) -> Result<RunRecord, String> {
        // Instantiate the operators this plan needs, once (register-once,
        // use-many). Preserves per-operator state across steps.
        let mut ops = HashMap::new();
        for step in &plan.steps {
            if !ops.contains_key(&step.operator) {
                let op = registry
                    .lookup(&step.operator)
                    .ok_or_else(|| format!("operator '{}' not in registry", step.operator))?;
                ops.insert(step.operator.clone(), op);
            }
        }

        let mut scheduler = PhaseScheduler::from_plan(plan);
        // Total scheduled steps up front, for the controller's frac_elapsed.
        let total_steps = scheduler.remaining().max(1) as f64;
        let mut events = Vec::new();
        let mut best_energy = f64::INFINITY;
        let mut best_state = vec![0u8; state.num_vars()];
        let mut energies = vec![0.0f64; state.num_replicas()];
        let mut scratch = vec![0u8; state.num_vars()];

        self.ctx.start_clock();
        while let Some(mut step) = scheduler.next() {
            // Operator bandit (v0.5): during Exploit, the bandit may substitute
            // the scheduled operator with its UCB choice. Budget is unchanged —
            // the bandit reallocates WHICH physics runs, not how much.
            if self.adapt.enabled && step.phase == crate::engine_v2::plan::Phase::Exploit {
                if let Some(choice) = &self.bandit_choice {
                    if *choice != step.operator && ops.contains_key(choice) {
                        let msg = format!(
                            "iter {}: bandit substituted {} for scheduled {}",
                            self.iteration, choice, step.operator
                        );
                        self.ctx.log(format!("adapt: {msg}"));
                        self.adaptations.push(msg);
                        step.operator = choice.clone();
                    }
                }
            }
            let op = ops.get_mut(&step.operator).expect("instantiated above");
            let view = RuntimeView {
                iteration: self.iteration,
                temperatures: &self.temperatures,
                num_replicas: self.num_replicas,
                recent_acceptance: self.recent_acceptance,
                remaining_ms: self.ctx.remaining_ms(),
            };
            let t0 = Instant::now();
            let report = op.apply(state, &view, &mut self.rng, step.budget);
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            self.ctx.profiler.record(op.name(), ms, report.work);

            self.iteration += 1;
            // EMA of acceptance for the adaptive controllers (deterministic).
            self.recent_acceptance = 0.9 * self.recent_acceptance + 0.1 * report.acceptance_rate();

            let prev_best = best_energy;
            state.energies_into(&mut energies);
            if let Some((r, &e)) = energies
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
            {
                if e < best_energy {
                    best_energy = e;
                    state.extract_into(r, &mut scratch);
                    best_state.copy_from_slice(&scratch);
                }
            }

            let metrics = Self::quality(&energies, state, best_energy);
            let acceptance = report.acceptance_rate();
            // The step's WHY: the Decision Engine's recorded α/β/γ/δ selection
            // rationale when the plan was utility-synthesized (§10 rule 2);
            // otherwise the plan itself is the (hand-written) reason.
            let decision = match plan.rationale.get(&step.operator) {
                Some(why) => format!("phase={:?}; {why}; work={:.0}", step.phase, report.work),
                None => format!(
                    "phase={:?} scheduled by plan; work={:.0}",
                    step.phase, report.work
                ),
            };
            // Explainable log line: what ran, what it achieved, why it ran.
            self.ctx.log(format!(
                "iter {:>5} op={:<20} accept={:.3} best={:.6} mean={:.6} entropy={:.3} diversity={:.3} :: {}",
                self.iteration,
                step.operator,
                acceptance,
                metrics.best_energy,
                metrics.mean_energy,
                metrics.energy_entropy,
                metrics.diversity,
                decision,
            ));
            events.push(StepEvent {
                iteration: self.iteration,
                operator: step.operator.clone(),
                acceptance,
                best_energy,
                metrics,
                decision,
            });

            self.maybe_adapt(
                &mut scheduler,
                &step.operator,
                step.phase,
                prev_best,
                best_energy,
                report.work,
                &metrics,
            );
            // Learned early-stop / early-switch controller (opt-in). Consulted
            // AFTER the adaptation hook so both see the same post-step state.
            if let Some(controller) = &mut self.controller {
                let sensors = StepSensors {
                    iteration: self.iteration,
                    frac_elapsed: self.iteration as f64 / total_steps,
                    operator: &step.operator,
                    best_energy,
                    metrics: &metrics,
                    acceptance,
                };
                match controller.decide(&sensors) {
                    RunControl::Continue => {}
                    RunControl::SwitchOperator => {
                        let dropped = scheduler.skip_current_operator(&step.operator);
                        if dropped > 0 {
                            let msg = format!(
                                "iter {}: controller switched away from {} early ({dropped} steps skipped)",
                                self.iteration, step.operator
                            );
                            self.ctx.log(format!("control: {msg}"));
                            self.adaptations.push(msg);
                        }
                    }
                    RunControl::Stop => {
                        let msg = format!(
                            "iter {}: controller stopped the run early ({} steps remained)",
                            self.iteration,
                            scheduler.remaining()
                        );
                        self.ctx.log(format!("control: {msg}"));
                        self.adaptations.push(msg);
                        break;
                    }
                }
            }
            if self.ctx.deadline_hit() {
                self.ctx.log("wall budget reached; stopping");
                break;
            }
        }

        let canonical_energy = ir.energy(&best_state);
        Ok(RunRecord {
            plan_name: plan.name.clone(),
            seed: plan.seed,
            iterations: self.iteration,
            best_energy,
            best_state,
            events,
            adaptations: std::mem::take(&mut self.adaptations),
            canonical_energy,
        })
    }

    /// Adaptation hook (Constitution §11, Runtime v0.5). Three controllers:
    ///
    /// 1. LADDER — EMA acceptance outside [accept_lo, accept_hi] rescales the
    ///    whole temperature ladder (frozen ⇒ heat, boiling ⇒ cool), bounded by
    ///    `max_heat`× the plan's initial hottest rung.
    /// 2. OPERATOR BANDIT — UCB1 over the plan's operator pool with reward =
    ///    best-energy improvement per unit work; the choice substitutes the
    ///    next Exploit step's operator (budget untouched).
    /// 3. PHASE SENSOR — when the ensemble has collapsed (energy entropy AND
    ///    replica diversity below floors), the remaining Explore steps are cut
    ///    and the schedule advances early.
    ///
    /// Every input is a deterministic function of the seeded trajectory, every
    /// decision is logged into `adaptations` — same seed + config replays
    /// bit-identically (ADR-0004).
    #[allow(clippy::too_many_arguments)]
    fn maybe_adapt(
        &mut self,
        scheduler: &mut PhaseScheduler,
        ran_op: &str,
        phase: crate::engine_v2::plan::Phase,
        prev_best: f64,
        best: f64,
        work: f64,
        metrics: &QualityMetrics,
    ) {
        if !self.adapt.enabled {
            return;
        }
        // 1. Ladder controller.
        let a = self.recent_acceptance;
        if a < self.adapt.accept_lo {
            let cap = self.initial_temp_hi * self.adapt.max_heat;
            if self.temperatures.first().copied().unwrap_or(0.0) * self.adapt.heat_factor <= cap {
                for t in &mut self.temperatures {
                    *t *= self.adapt.heat_factor;
                }
                let msg = format!(
                    "iter {}: acceptance {a:.3} < {} — ladder heated ×{} (hi now {:.3})",
                    self.iteration,
                    self.adapt.accept_lo,
                    self.adapt.heat_factor,
                    self.temperatures.first().copied().unwrap_or(0.0)
                );
                self.ctx.log(format!("adapt: {msg}"));
                self.adaptations.push(msg);
            }
        } else if a > self.adapt.accept_hi {
            for t in &mut self.temperatures {
                *t *= self.adapt.cool_factor;
            }
            let msg = format!(
                "iter {}: acceptance {a:.3} > {} — ladder cooled ×{}",
                self.iteration, self.adapt.accept_hi, self.adapt.cool_factor
            );
            self.ctx.log(format!("adapt: {msg}"));
            self.adaptations.push(msg);
        }

        // 2. Operator bandit: reward = improvement per unit work (≥ 0).
        let reward = (prev_best - best).max(0.0) / work.max(1.0);
        let e = self.op_reward.entry(ran_op.to_string()).or_insert((0.0, 0));
        e.0 += reward;
        e.1 += 1;
        let total: u64 = self.op_reward.values().map(|&(_, n)| n).sum::<u64>().max(1);
        let mut pool: Vec<&String> = self.op_reward.keys().collect();
        pool.sort(); // deterministic order regardless of map internals
        self.bandit_choice = pool
            .into_iter()
            .map(|op| {
                let (r, n) = self.op_reward[op];
                let ucb = if n == 0 {
                    f64::INFINITY
                } else {
                    r / n as f64 + self.adapt.bandit_c * ((total as f64).ln() / n as f64).sqrt()
                };
                (op.clone(), ucb)
            })
            // max by UCB; ties resolve to the lexicographically FIRST operator.
            .fold(None::<(String, f64)>, |acc, (op, u)| match acc {
                Some((bop, bu)) if bu >= u => Some((bop, bu)),
                _ => Some((op, u)),
            })
            .map(|(op, _)| op);

        // 3. Early phase transition on ensemble collapse.
        if phase == crate::engine_v2::plan::Phase::Explore
            && metrics.energy_entropy < self.adapt.collapse_entropy
            && metrics.diversity < self.adapt.collapse_diversity
        {
            let dropped = scheduler.skip_phase(crate::engine_v2::plan::Phase::Explore);
            if dropped > 0 {
                let msg = format!(
                    "iter {}: ensemble collapsed (entropy {:.3}, diversity {:.3}) — skipped {dropped} Explore steps",
                    self.iteration, metrics.energy_entropy, metrics.diversity
                );
                self.ctx.log(format!("adapt: {msg}"));
                self.adaptations.push(msg);
            }
        }
    }

    /// Measure ensemble quality from the current state — diagnostic only, no
    /// trajectory influence (ADR-0004). Entropy is the Shannon entropy of the
    /// per-replica energy histogram (exact-value bins; integer backends make
    /// these discrete). Diversity averages Hamming dissimilarity over disjoint
    /// replica pairs, capped at `DIVERSITY_PAIRS` so cost is O(pairs·n).
    fn quality(energies: &[f64], state: &dyn SpinState, best_energy: f64) -> QualityMetrics {
        const DIVERSITY_PAIRS: usize = 16;
        let r = energies.len();
        if r == 0 {
            return QualityMetrics::default();
        }
        let mean_energy = energies.iter().sum::<f64>() / r as f64;

        // Shannon entropy of the energy distribution (bits).
        let mut hist: HashMap<u64, u32> = HashMap::new();
        for &e in energies {
            *hist.entry(e.to_bits()).or_insert(0) += 1;
        }
        let n = r as f64;
        let energy_entropy = hist
            .values()
            .map(|&c| {
                let p = c as f64 / n;
                -p * p.log2()
            })
            .sum::<f64>();

        // Mean pairwise diversity over disjoint pairs (0,1),(2,3),…
        let pairs = (r / 2).min(DIVERSITY_PAIRS);
        let diversity = if pairs == 0 {
            0.0
        } else {
            let mut acc = 0.0;
            for p in 0..pairs {
                // overlap ∈ [−1,1]; diversity contribution (1−overlap)/2 ∈ [0,1].
                acc += (1.0 - state.overlap(2 * p, 2 * p + 1)) * 0.5;
            }
            acc / pairs as f64
        };

        QualityMetrics {
            best_energy,
            mean_energy,
            energy_entropy,
            diversity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::backends::ReferenceState;
    use super::super::capability::{
        CapabilitySet, Complexity, Constraints, Guarantees, ObservableSet, OperatorDescriptor,
    };
    use super::super::operator::{Budget, CostEstimate, InstanceShape, Nature, Operator, Report};
    use super::super::plan::{Backend, Phase, Plan, PlanStep};
    use super::super::registry::OperatorRegistry;
    use super::super::state::{ReplicaMask, SpinState};
    use super::*;
    use rand::Rng;

    /// Test-only operator: single-spin greedy descent. Not a production
    /// algorithm — it exists purely to exercise the Runtime wiring.
    struct GreedyDescent;

    impl Operator for GreedyDescent {
        fn descriptor(&self) -> OperatorDescriptor {
            OperatorDescriptor {
                name: "test_greedy",
                nature: Nature::TrajectoryPreserving,
                capabilities: CapabilitySet::empty(),
                constraints: Constraints::any(),
                observables: ObservableSet::empty(),
                guarantees: Guarantees::none(),
                complexity: Complexity::LinearEdges,
            }
        }
        fn cost_model(&self, s: InstanceShape) -> CostEstimate {
            CostEstimate {
                work_per_sweep: s.n as f64,
            }
        }
        fn apply(
            &mut self,
            state: &mut dyn SpinState,
            _view: &RuntimeView,
            rng: &mut ChaCha8Rng,
            budget: Budget,
        ) -> Report {
            let n = state.num_vars();
            let r = state.num_replicas();
            let mut de = vec![0.0; r];
            let mut rep = Report::default();
            for _ in 0..budget.sweeps {
                for _ in 0..n {
                    let site = rng.gen_range(0..n);
                    state.delta_e_into(site, &mut de);
                    let mut mask = ReplicaMask::new(r);
                    for (k, &d) in de.iter().enumerate() {
                        rep.proposed += 1;
                        if d < 0.0 {
                            mask.set(k);
                            rep.accepted += 1;
                        }
                    }
                    if !mask.is_empty() {
                        state.apply_flips(site, &mask);
                    }
                }
            }
            rep.work = (budget.sweeps as f64) * (n as f64);
            rep
        }
    }

    #[test]
    fn runtime_drives_operator_and_rescore_matches() {
        // E = 1 + 2x0 - 3x1 + x2 - 4x0x1 + 2x1x2 ; min = -4 at x=(1,1,0)
        let ir = ProblemIR::from_pairs(3, 1.0, vec![2.0, -3.0, 1.0], &[(0, 1, -4.0), (1, 2, 2.0)]);
        let mut reg = OperatorRegistry::new();
        reg.register(|| Box::new(GreedyDescent));

        let plan = Plan {
            name: "greedy".into(),
            backend: Backend::DenseByte,
            num_replicas: 4,
            temperatures: vec![1.0],
            steps: vec![PlanStep {
                operator: "test_greedy".into(),
                phase: Phase::Exploit,
                sweeps: 20,
                repeat: 1,
            }],
            seed: 42,
            rationale: Default::default(),
        };
        let mut state = ReferenceState::new(&ir, 4, &[0, 0, 0]);
        let mut rt = Runtime::new(RunContext::new(plan.seed), &plan);
        let rec = rt.run(&plan, &mut state, &reg, &ir).unwrap();

        assert_eq!(
            rec.best_energy, rec.canonical_energy,
            "ledger must match canonical scorer"
        );
        assert!(
            rec.best_energy <= 1.0,
            "greedy must not worsen the start energy"
        );
        assert_eq!(state.audit(), 0.0, "field ledger drift must be zero");
        assert!(rec.iterations >= 1);

        // Every step is explainable: an event with a rationale + quality, and a
        // matching human-readable log line the Runtime wrote (§11).
        assert_eq!(rec.events.len() as u64, rec.iterations);
        for ev in &rec.events {
            assert!(
                !ev.decision.is_empty(),
                "every step must record a rationale"
            );
            assert!(ev.metrics.diversity >= 0.0 && ev.metrics.diversity <= 1.0);
            assert!(ev.metrics.energy_entropy >= 0.0);
            assert!(ev.metrics.mean_energy >= ev.metrics.best_energy - 1e-9);
        }
        assert_eq!(
            rt.context().log.len() as u64,
            rec.iterations,
            "one explainable log line per executed step"
        );
    }

    /// Test-only operator that proposes nothing and never improves — the
    /// bandit must learn to route around it, and its 0 acceptance must trigger
    /// the ladder heater.
    struct Noop;
    impl Operator for Noop {
        fn descriptor(&self) -> OperatorDescriptor {
            OperatorDescriptor {
                name: "test_noop",
                nature: Nature::TrajectoryPreserving,
                capabilities: CapabilitySet::empty(),
                constraints: Constraints::any(),
                observables: ObservableSet::empty(),
                guarantees: Guarantees::none(),
                complexity: Complexity::Constant,
            }
        }
        fn cost_model(&self, _s: InstanceShape) -> CostEstimate {
            CostEstimate {
                work_per_sweep: 1.0,
            }
        }
        fn apply(
            &mut self,
            _state: &mut dyn SpinState,
            _view: &RuntimeView,
            _rng: &mut ChaCha8Rng,
            budget: Budget,
        ) -> Report {
            Report {
                proposed: budget.sweeps as u64,
                accepted: 0,
                work: budget.sweeps as f64,
                ..Default::default()
            }
        }
    }

    fn chain_ir(n: usize) -> ProblemIR {
        let pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
            .map(|i| (i, i + 1, if i % 3 == 0 { -2.0 } else { 1.0 }))
            .collect();
        let linear: Vec<f64> = (0..n)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        ProblemIR::from_pairs(n, 0.0, linear, &pairs)
    }

    fn adaptive_plan(seed: u64) -> Plan {
        Plan {
            name: "adaptive".into(),
            backend: Backend::DenseByte,
            num_replicas: 4,
            temperatures: vec![2.0, 1.0, 0.5, 0.25],
            steps: vec![
                PlanStep {
                    operator: "test_greedy".into(),
                    phase: Phase::Explore,
                    sweeps: 2,
                    repeat: 2,
                },
                PlanStep {
                    operator: "test_noop".into(),
                    phase: Phase::Exploit,
                    sweeps: 2,
                    repeat: 8,
                },
                PlanStep {
                    operator: "test_greedy".into(),
                    phase: Phase::Exploit,
                    sweeps: 2,
                    repeat: 2,
                },
            ],
            seed,
            rationale: Default::default(),
        }
    }

    fn run_adaptive(seed: u64) -> (RunRecord, Vec<String>) {
        let ir = chain_ir(12);
        let mut reg = OperatorRegistry::new();
        reg.register(|| Box::new(GreedyDescent));
        reg.register(|| Box::new(Noop));
        let plan = adaptive_plan(seed);
        let mut state = ReferenceState::new(&ir, 4, &[0u8; 12]);
        let mut rt = Runtime::new(RunContext::new(plan.seed), &plan)
            .with_adaptation(AdaptationConfig::enabled());
        let rec = rt.run(&plan, &mut state, &reg, &ir).unwrap();
        let ops: Vec<String> = rec.events.iter().map(|e| e.operator.clone()).collect();
        (rec, ops)
    }

    #[test]
    fn adaptive_replay_is_bit_identical() {
        let (a, ops_a) = run_adaptive(7);
        let (b, ops_b) = run_adaptive(7);
        assert_eq!(a.best_energy.to_bits(), b.best_energy.to_bits());
        assert_eq!(a.best_state, b.best_state);
        assert_eq!(ops_a, ops_b, "operator sequence must replay identically");
        assert_eq!(
            a.adaptations, b.adaptations,
            "decisions must replay identically"
        );
        assert_eq!(a.best_energy, a.canonical_energy);
    }

    #[test]
    fn bandit_routes_exploit_budget_toward_the_improving_operator() {
        let (rec, ops) = run_adaptive(11);
        assert!(
            rec.adaptations
                .iter()
                .any(|m| m.contains("bandit substituted test_greedy")),
            "bandit should substitute the useless operator: {:?}",
            rec.adaptations
        );
        // The executed schedule contains more greedy steps than planned (4).
        let greedy_runs = ops.iter().filter(|o| *o == "test_greedy").count();
        assert!(greedy_runs > 4, "executed greedy {greedy_runs} ≤ planned 4");
    }

    #[test]
    fn frozen_acceptance_heats_the_ladder_within_cap() {
        let (rec, _) = run_adaptive(3);
        // Noop steps report 0 acceptance ⇒ EMA collapses below accept_lo.
        assert!(
            rec.adaptations.iter().any(|m| m.contains("ladder heated")),
            "expected ladder heating: {:?}",
            rec.adaptations
        );
        // Cap respected: hottest rung never exceeds max_heat × initial 2.0.
        for m in &rec.adaptations {
            if let Some(pos) = m.find("hi now ") {
                let v: f64 = m[pos + 7..].trim_end_matches(')').parse().unwrap();
                assert!(v <= 2.0 * 8.0 + 1e-9, "ladder exceeded cap: {m}");
            }
        }
    }

    #[test]
    fn collapsed_ensemble_cuts_explore_short() {
        // Single replica ⇒ zero entropy and zero diversity after any step.
        let ir = chain_ir(8);
        let mut reg = OperatorRegistry::new();
        reg.register(|| Box::new(GreedyDescent));
        let plan = Plan {
            name: "collapse".into(),
            backend: Backend::DenseByte,
            num_replicas: 1,
            temperatures: vec![1.0],
            steps: vec![
                PlanStep {
                    operator: "test_greedy".into(),
                    phase: Phase::Explore,
                    sweeps: 1,
                    repeat: 10,
                },
                PlanStep {
                    operator: "test_greedy".into(),
                    phase: Phase::Finish,
                    sweeps: 1,
                    repeat: 1,
                },
            ],
            seed: 9,
            rationale: Default::default(),
        };
        let mut state = ReferenceState::new(&ir, 1, &[0u8; 8]);
        let mut rt = Runtime::new(RunContext::new(plan.seed), &plan)
            .with_adaptation(AdaptationConfig::enabled());
        let rec = rt.run(&plan, &mut state, &reg, &ir).unwrap();
        assert!(
            rec.adaptations.iter().any(|m| m.contains("collapsed")),
            "expected collapse decision: {:?}",
            rec.adaptations
        );
        assert!(
            rec.iterations < 11,
            "Explore should be cut short, ran {} steps",
            rec.iterations
        );
        // Adaptation OFF runs the full schedule — the controllers are opt-in.
        let mut state2 = ReferenceState::new(&ir, 1, &[0u8; 8]);
        let mut rt2 = Runtime::new(RunContext::new(plan.seed), &plan);
        let rec2 = rt2.run(&plan, &mut state2, &reg, &ir).unwrap();
        assert_eq!(rec2.iterations, 11);
        assert!(rec2.adaptations.is_empty());
    }

    #[test]
    fn unregistered_operator_is_an_error() {
        let ir = ProblemIR::from_pairs(2, 0.0, vec![0.0, 0.0], &[(0, 1, -1.0)]);
        let reg = OperatorRegistry::new();
        let plan = Plan {
            name: "x".into(),
            backend: Backend::DenseByte,
            num_replicas: 1,
            temperatures: vec![1.0],
            steps: vec![PlanStep {
                operator: "missing".into(),
                phase: Phase::Explore,
                sweeps: 1,
                repeat: 1,
            }],
            seed: 1,
            rationale: Default::default(),
        };
        let mut state = ReferenceState::new(&ir, 1, &[0, 0]);
        let mut rt = Runtime::new(RunContext::new(1), &plan);
        assert!(rt.run(&plan, &mut state, &reg, &ir).is_err());
    }
}
