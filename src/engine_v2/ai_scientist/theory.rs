//! Theory Engine (Stage 8, Pillar I — the true-researcher core).
//!
//! Every other analyzer stops at a RULE — a correlation ("metropolis_sweep is in
//! 95% of the best solutions"). A rule says *what*. This engine builds toward
//! *why*, and it does it the only honest way: not by asking an LLM to narrate a
//! mechanism, but by turning the mechanism into a FALSIFIABLE PREDICTION and
//! letting the Runtime try to refute it.
//!
//!   RULE → HYPOTHESIS → MECHANISM → PREDICTION → EXPERIMENT → REFUTATION → THEORY
//!
//! Concretely, to explain why an operator earns its place in a working schedule:
//!   1. MECHANISM  — assemble from the operator's capability passport + the
//!      observable trajectory signature (does energy-entropy fall while it runs?
//!      what is the acceptance profile?) already recorded in every `StepEvent`.
//!   2. PREDICTION — a falsifiable claim: "ABLATE this operator and the outcome
//!      degrades by at least δ (and the signature changes)."
//!   3. EXPERIMENT — run the full schedule and the ablated schedule on the real
//!      Runtime, at identical seeds.
//!   4. REFUTATION — if ablation does NOT degrade as predicted, the operator was
//!      not causal here: the theory is REFUTED (and kept — a dead theory is
//!      knowledge). If it degrades as predicted across trials, the theory is
//!      SUPPORTED and earns confidence.
//!
//! A theory is thus not prose and not a fancier rule: it is a mechanism claim
//! that survived the platform's own attempt to break it. This is Popper wired
//! into the experiment loop, and it is what turns the platform from a system
//! that *learns rules* into one that *understands*.

use super::super::capability::Capability;
use super::super::evolution::Schedule;
use super::super::ir::ProblemIR;
use super::super::registry::OperatorRegistry;
use super::dynamics::capture_trajectory;
use super::executor::{BatchExecutor, ExperimentTask, RuntimeExecutor};
use super::graph::KnowledgeGraph;

/// The observable mechanism signature of an operator inside a schedule, read
/// from the trajectory the Runtime already logs.
#[derive(Debug, Clone)]
pub struct MechanismSignature {
    /// Change in energy-entropy across the operator's steps (negative ⇒ the
    /// ensemble relaxed / collapsed a barrier while it ran).
    pub entropy_delta: f64,
    /// Mean acceptance during the operator's steps (0 ⇒ frozen, >0 ⇒ moving).
    pub mean_acceptance: f64,
    /// Change in replica diversity across the operator's steps.
    pub diversity_delta: f64,
}

/// A mechanism hypothesis: what the operator is claimed to do, and why.
#[derive(Debug, Clone)]
pub struct MechanismHypothesis {
    pub operator: String,
    pub condition: String,
    /// The capability the passport says it provides (the claimed role).
    pub capability: String,
    pub signature: MechanismSignature,
    /// Human-readable mechanism narrative (assembled, not invented).
    pub claim: String,
}

/// A falsifiable prediction derived from a mechanism hypothesis.
#[derive(Debug, Clone)]
pub struct Prediction {
    pub statement: String,
    /// Minimum relative degradation (fraction of scale) that ablation must
    /// cause for the prediction to be upheld.
    pub min_degradation: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TheoryStatus {
    /// Proposed but not yet tested.
    Hypothesis,
    /// Ablation degraded the outcome as predicted (the operator is causal).
    Supported,
    /// Ablation did NOT degrade as predicted (the correlation was spurious).
    Refuted,
}

/// A theory: a mechanism hypothesis that has faced the Runtime's attempt to
/// refute it, with the evidence of every trial.
#[derive(Debug, Clone)]
pub struct Theory {
    pub hypothesis: MechanismHypothesis,
    pub prediction: Prediction,
    pub status: TheoryStatus,
    pub trials: u32,
    pub survived: u32,
    /// survived/trials, shrunk toward 0 while trials are few (evidence weight).
    pub confidence: f64,
    /// A clean, self-contained answer to "WHY is this algorithm good here?" —
    /// causality (ablation cost) tied to the measured mechanism.
    pub explanation: String,
    pub evidence: Vec<String>,
}

impl Theory {
    fn recompute(&mut self) {
        self.status = if self.trials == 0 {
            TheoryStatus::Hypothesis
        } else if self.survived * 2 > self.trials {
            TheoryStatus::Supported
        } else {
            TheoryStatus::Refuted
        };
        let rate = if self.trials == 0 {
            0.0
        } else {
            self.survived as f64 / self.trials as f64
        };
        // Shrink for low trial counts: 1 − e^(−trials/3).
        let weight = 1.0 - (-(self.trials as f64) / 3.0).exp();
        self.confidence = rate * weight;
    }
}

/// Configuration for the ablation test.
#[derive(Debug, Clone, Copy)]
pub struct TheoryConfig {
    pub num_replicas: usize,
    /// Relative degradation threshold δ for "ablation hurt as predicted".
    pub min_degradation: f64,
}

impl Default for TheoryConfig {
    fn default() -> Self {
        Self {
            num_replicas: 16,
            min_degradation: 0.01,
        }
    }
}

/// Builds and tests mechanistic theories about operators.
pub struct TheoryEngine {
    cfg: TheoryConfig,
    exec: RuntimeExecutor,
}

impl TheoryEngine {
    pub fn new(cfg: TheoryConfig) -> Self {
        Self {
            exec: RuntimeExecutor::new(1),
            cfg,
        }
    }

    fn capability_name(registry: &OperatorRegistry, op: &str) -> String {
        let Some(d) = registry.metadata(op) else {
            return "unknown".into();
        };
        for (c, tag) in [
            (Capability::BarrierCrossing, "barrier-crossing"),
            (Capability::Exploration, "exploration"),
            (Capability::Exploitation, "exploitation"),
            (Capability::ExactInference, "exact-inference"),
            (Capability::Warmstart, "warm-start"),
            (Capability::Approximate, "approximate"),
        ] {
            if d.capabilities.contains(c) {
                return tag.into();
            }
        }
        "generic".into()
    }

    /// Read the operator's mechanism signature from a captured trajectory of the
    /// full schedule (step i ↔ schedule.ops[i], single-phase execution order).
    fn signature(
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        schedule: &Schedule,
        operator: &str,
        num_replicas: usize,
        seeds: &[u64],
    ) -> MechanismSignature {
        // Average the signature over ALL seeds so the mechanism narrative rests
        // on a stable measurement, not one noisy trajectory.
        let (mut ed, mut ma, mut dd, mut n) = (0.0, 0.0, 0.0, 0usize);
        for &seed in seeds {
            let tr = capture_trajectory(ir, registry, schedule, num_replicas, seed);
            let idx: Vec<usize> = schedule
                .ops
                .iter()
                .enumerate()
                .filter(|(_, o)| o.as_str() == operator)
                .map(|(i, _)| i)
                .filter(|&i| i < tr.steps.len())
                .collect();
            if idx.is_empty() {
                continue;
            }
            let first = idx[0];
            let last = *idx.last().unwrap();
            let before = if first > 0 { first - 1 } else { first };
            ed += tr.steps[last].entropy - tr.steps[before].entropy;
            dd += tr.steps[last].diversity - tr.steps[before].diversity;
            ma += idx.iter().map(|&i| tr.steps[i].acceptance).sum::<f64>() / idx.len() as f64;
            n += 1;
        }
        if n == 0 {
            return MechanismSignature {
                entropy_delta: 0.0,
                mean_acceptance: 0.0,
                diversity_delta: 0.0,
            };
        }
        MechanismSignature {
            entropy_delta: ed / n as f64,
            mean_acceptance: ma / n as f64,
            diversity_delta: dd / n as f64,
        }
    }

    /// The dominant mechanism narrative from the measured signature — honest:
    /// it names the STRONGEST observed effect and only asserts a mechanism when
    /// the signal is clear, otherwise it says the mechanism is not evident.
    fn mechanism_story(s: &MechanismSignature) -> &'static str {
        const STRONG: f64 = 0.1;
        let signals = [
            (-s.entropy_delta, "it relaxes the ensemble — energy-entropy falls, collapsing a barrier a greedy quench stalls at"),
            (-s.diversity_delta, "it converges the replicas — diversity collapses onto a shared basin"),
            (s.mean_acceptance - 0.4, "it explores broadly — acceptance stays high, keeping the ensemble moving"),
            (s.entropy_delta, "it injects exploration — energy-entropy rises while it runs"),
        ];
        let (mag, story) = signals.iter().cloned().fold(
            (
                f64::NEG_INFINITY,
                "its mechanism is not clearly evident in the trajectory",
            ),
            |acc, (m, st)| {
                if m > acc.0 {
                    (m, st)
                } else {
                    acc
                }
            },
        );
        if mag >= STRONG {
            story
        } else {
            "its mechanism is not clearly evident in the trajectory (weak, mixed signals)"
        }
    }

    fn mean_best(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        schedule: &Schedule,
        seeds: &[u64],
    ) -> f64 {
        if schedule.ops.is_empty() {
            // An empty schedule does nothing: the all-zero start energy.
            return ir.energy(&vec![0u8; ir.n]);
        }
        let tasks: Vec<ExperimentTask> = seeds
            .iter()
            .map(|&seed| ExperimentTask {
                schedule: schedule.clone(),
                num_replicas: self.cfg.num_replicas,
                seed,
            })
            .collect();
        let out = self.exec.run_batch(ir, registry, &tasks);
        out.iter().map(|o| o.score).sum::<f64>() / out.len().max(1) as f64
    }

    /// Explain why `operator` earns its place in `schedule` on `ir`: assemble
    /// the mechanism, form the falsifiable prediction, run the ABLATION, and
    /// return the resulting theory (Supported/Refuted). `condition` is the
    /// structural regime the theory holds under.
    pub fn explain(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        schedule: &Schedule,
        operator: &str,
        condition: &str,
        seeds: &[u64],
    ) -> Theory {
        let sig = Self::signature(
            ir,
            registry,
            schedule,
            operator,
            self.cfg.num_replicas,
            seeds,
        );
        let capability = Self::capability_name(registry, operator);
        let mechanism = Self::mechanism_story(&sig);
        let claim = format!(
            "On {condition}, {operator} acts as a {capability} operator: {mechanism} \
(mean acceptance {:.2}, entropy Δ {:+.2}, diversity Δ {:+.2}).",
            sig.mean_acceptance, sig.entropy_delta, sig.diversity_delta
        );
        let hypothesis = MechanismHypothesis {
            operator: operator.to_string(),
            condition: condition.to_string(),
            capability: capability.clone(),
            signature: sig,
            claim,
        };

        // Falsifiable prediction: ablate the operator ⇒ outcome degrades ≥ δ.
        let prediction = Prediction {
            statement: format!(
                "Ablating {operator} worsens the best energy by ≥ {:.0}% of scale.",
                self.cfg.min_degradation * 100.0
            ),
            min_degradation: self.cfg.min_degradation,
        };

        // EXPERIMENT: full vs ablated (operator removed) at identical seeds.
        let full = self.mean_best(ir, registry, schedule, seeds);
        let ablated_ops: Vec<String> = schedule
            .ops
            .iter()
            .filter(|o| o.as_str() != operator)
            .cloned()
            .collect();
        let ablated = Schedule {
            sweeps: vec![schedule.sweeps.first().copied().unwrap_or(16); ablated_ops.len()],
            ops: ablated_ops,
            temp_hi: schedule.temp_hi,
            temp_lo: schedule.temp_lo,
        };
        let ablated_best = self.mean_best(ir, registry, &ablated, seeds);

        // Degradation: how much WORSE (higher energy) removing the operator is,
        // relative to the instance's energy scale.
        let scale = full.abs().max(1.0);
        let degradation = (ablated_best - full) / scale; // ≥ 0 ⇒ operator helped
        let upheld = degradation >= prediction.min_degradation;

        // The clean "why is this algorithm good here?" — causality (measured by
        // the ablation) tied to the mechanism, stated plainly either way.
        let explanation = if upheld {
            format!(
                "`{operator}` is genuinely good on {condition}: removing it worsens the best \
energy by {:.1}% — so it is the causal {} in this algorithm. It works because {}.",
                degradation * 100.0,
                hypothesis.capability,
                mechanism
            )
        } else {
            format!(
                "`{operator}` is NOT what makes this algorithm good on {condition}: removing it \
changes the best energy by only {:.1}%, so its presence in the schedule is a spurious \
correlation — the useful work is done by the other operators. (Observed while it ran: {}.)",
                degradation * 100.0,
                mechanism
            )
        };

        let mut theory = Theory {
            hypothesis,
            prediction,
            status: TheoryStatus::Hypothesis,
            trials: 1,
            survived: u32::from(upheld),
            confidence: 0.0,
            explanation,
            evidence: vec![format!(
                "ablation trial: full best {full:.2}, ablated best {ablated_best:.2}, \
degradation {:+.1}% of scale ⇒ prediction {}",
                degradation * 100.0,
                if upheld { "UPHELD" } else { "REFUTED" }
            )],
        };
        theory.recompute();
        theory
    }

    /// Aggregate `explain` across several instances of the same regime, so a
    /// theory earns confidence from surviving refutation on MANY instances, not
    /// one. Returns the consolidated theory (mechanism from the first instance,
    /// confidence from the trial record across all).
    pub fn investigate(
        &self,
        instances: &[&ProblemIR],
        registry: &OperatorRegistry,
        schedule: &Schedule,
        operator: &str,
        condition: &str,
        seeds: &[u64],
    ) -> Option<Theory> {
        let mut theory: Option<Theory> = None;
        for ir in instances {
            let t = self.explain(ir, registry, schedule, operator, condition, seeds);
            match &mut theory {
                None => theory = Some(t),
                Some(acc) => {
                    acc.trials += t.trials;
                    acc.survived += t.survived;
                    acc.evidence.extend(t.evidence);
                    acc.recompute();
                }
            }
        }
        theory
    }

    /// Publish a SUPPORTED theory into the knowledge graph as a conditional,
    /// mechanism-bearing fact the LLM and reports can read. Refuted theories are
    /// published too (as negative knowledge) so a dead end is not re-explored.
    pub fn publish(&self, theory: &Theory, graph: &mut KnowledgeGraph) {
        let (predicate, weight) = match theory.status {
            TheoryStatus::Supported => ("theory-explains", theory.confidence),
            TheoryStatus::Refuted => ("theory-refuted", -theory.confidence.max(0.1)),
            TheoryStatus::Hypothesis => return,
        };
        let proof = format!(
            "{} | prediction: {} | {} | confidence {:.2} ({}/{} trials)",
            theory.explanation,
            theory.prediction.statement,
            theory.evidence.last().cloned().unwrap_or_default(),
            theory.confidence,
            theory.survived,
            theory.trials
        );
        graph.observe_if(
            &theory.hypothesis.operator,
            predicate,
            &theory.hypothesis.capability,
            &theory.hypothesis.condition,
            weight,
            &proof,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(n: usize, shift: f64) -> ProblemIR {
        let mut pairs: Vec<(u32, u32, f64)> = (0..n as u32 - 1)
            .map(|i| (i, i + 1, if i % 2 == 0 { -1.0 } else { 1.0 }))
            .collect();
        pairs.push((0, n as u32 - 1, -1.0));
        let linear: Vec<f64> = (0..n).map(|i| ((i as f64) * shift).sin()).collect();
        ProblemIR::from_pairs(n, 0.0, linear, &pairs)
    }

    fn sched(ops: &[&str], sweeps: u32) -> Schedule {
        Schedule {
            ops: ops.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![sweeps; ops.len()],
            temp_hi: 3.0,
            temp_lo: 0.05,
        }
    }

    #[test]
    fn a_causal_operator_survives_ablation_and_becomes_a_theory() {
        let reg = OperatorRegistry::standard();
        let eng = TheoryEngine::new(TheoryConfig {
            num_replicas: 8,
            min_degradation: 0.005,
        });
        let ir = ring(24, 0.7);
        // greedy_descent is the ONLY optimizer here: ablating it leaves an empty
        // schedule (the all-zero start, no optimization at all) → its causal
        // contribution is unmistakable → SUPPORTED.
        let s = sched(&["greedy_descent"], 8);
        let t = eng.explain(&ir, &reg, &s, "greedy_descent", "density<0.05", &[1, 2, 3]);
        assert_eq!(
            t.status,
            TheoryStatus::Supported,
            "greedy is the whole schedule; it must be causal: {:?}",
            t.evidence
        );
        assert!(t.confidence > 0.0);
        assert!(t.hypothesis.claim.contains("greedy_descent"));
        assert_eq!(t.hypothesis.capability, "exploitation");
        // The explanation states clearly WHY it is good: causality + mechanism.
        assert!(
            t.explanation.contains("genuinely good") && t.explanation.contains("causal"),
            "explanation must state why it's good: {}",
            t.explanation
        );
    }

    #[test]
    fn a_redundant_operator_is_refuted_by_ablation() {
        let reg = OperatorRegistry::standard();
        let eng = TheoryEngine::new(TheoryConfig {
            num_replicas: 8,
            min_degradation: 0.02,
        });
        let ir = ring(24, 1.3);
        // After a strong greedy quench, a trailing random_flip_sweep does not
        // improve the result — removing it should NOT degrade → REFUTED.
        let s = sched(
            &["greedy_descent", "greedy_descent", "random_flip_sweep"],
            8,
        );
        let t = eng.explain(
            &ir,
            &reg,
            &s,
            "random_flip_sweep",
            "density<0.05",
            &[1, 2, 3],
        );
        assert_eq!(
            t.status,
            TheoryStatus::Refuted,
            "trailing random_flip should not be causal: {:?}",
            t.evidence
        );
    }

    #[test]
    fn investigate_aggregates_trials_and_publishes() {
        let reg = OperatorRegistry::standard();
        let eng = TheoryEngine::new(TheoryConfig {
            num_replicas: 8,
            min_degradation: 0.005,
        });
        let irs: Vec<ProblemIR> = (0..3).map(|i| ring(24, 0.5 + i as f64 * 0.3)).collect();
        let refs: Vec<&ProblemIR> = irs.iter().collect();
        // greedy alone is causal on every instance (ablation ⇒ no optimization).
        let s = sched(&["greedy_descent"], 8);
        let t = eng
            .investigate(&refs, &reg, &s, "greedy_descent", "density<0.05", &[1, 2])
            .unwrap();
        assert_eq!(t.trials, 3, "one trial per instance");
        assert!(t.survived >= 2, "should survive on most instances");

        let mut g = KnowledgeGraph::new();
        eng.publish(&t, &mut g);
        let facts = g.query_applicable("greedy_descent", "theory-explains", 0.01, 0.3);
        assert_eq!(facts.len(), 1);
        assert!(facts[0].proof.contains("prediction:"));
        assert!(facts[0].proof.contains("ablation trial"));
    }

    #[test]
    fn deterministic() {
        let reg = OperatorRegistry::standard();
        let eng = TheoryEngine::new(TheoryConfig {
            num_replicas: 8,
            min_degradation: 0.005,
        });
        let ir = ring(20, 0.9);
        let s = sched(&["metropolis_sweep", "greedy_descent"], 5);
        let a = eng.explain(&ir, &reg, &s, "greedy_descent", "c", &[7, 8]);
        let b = eng.explain(&ir, &reg, &s, "greedy_descent", "c", &[7, 8]);
        assert_eq!(a.survived, b.survived);
        assert_eq!(a.confidence.to_bits(), b.confidence.to_bits());
    }
}
