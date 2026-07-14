//! `Scheduler` — the sequencing layer between Runtime and Operator
//! (Constitution §8). It decides WHICH operator runs NEXT and with what
//! budget; it does not touch time, temperature, statistics, or state — that is
//! the Runtime's job. Today `PhaseScheduler` walks the plan in phase order;
//! tomorrow an adaptive scheduler (operator bandit, OGP regime switch) drops in
//! behind the same trait with zero changes to Runtime or operators.

use super::operator::Budget;
use super::plan::{Phase, Plan};

/// A single resolved invocation the Runtime should execute next.
#[derive(Debug, Clone)]
pub struct ScheduledStep {
    pub operator: String,
    pub phase: Phase,
    pub budget: Budget,
}

/// Sequencing policy. `next` returns `None` when the schedule is exhausted.
pub trait Scheduler {
    fn next(&mut self) -> Option<ScheduledStep>;
}

/// Static policy: expand the plan's steps in canonical phase order, honoring
/// each step's repeat count. Deterministic (ADR-0004).
pub struct PhaseScheduler {
    queue: std::collections::VecDeque<ScheduledStep>,
}

impl PhaseScheduler {
    pub fn from_plan(plan: &Plan) -> Self {
        let mut queue = std::collections::VecDeque::new();
        for phase in [
            Phase::Presolve,
            Phase::Explore,
            Phase::Exploit,
            Phase::Finish,
        ] {
            for step in plan.steps_in(phase) {
                for _ in 0..step.repeat.max(1) {
                    queue.push_back(ScheduledStep {
                        operator: step.operator.clone(),
                        phase,
                        budget: Budget {
                            sweeps: step.sweeps,
                        },
                    });
                }
            }
        }
        Self { queue }
    }

    pub fn remaining(&self) -> usize {
        self.queue.len()
    }

    /// Adaptive early phase transition (Runtime v0.5): drop every remaining
    /// step of `phase` from the FRONT of the queue, advancing the schedule to
    /// the next phase. Deterministic — a pure queue operation.
    pub fn skip_phase(&mut self, phase: Phase) -> usize {
        let mut dropped = 0;
        while self.queue.front().is_some_and(|s| s.phase == phase) {
            self.queue.pop_front();
            dropped += 1;
        }
        dropped
    }

    /// EARLY OPERATOR SWITCHING (Dynamics-model controller): drop every
    /// remaining queued step whose operator is `operator`, up to (but not
    /// including) the next DIFFERENT operator, so the schedule advances to the
    /// next distinct physics. Deterministic queue operation.
    pub fn skip_current_operator(&mut self, operator: &str) -> usize {
        let mut dropped = 0;
        while self.queue.front().is_some_and(|s| s.operator == operator) {
            self.queue.pop_front();
            dropped += 1;
        }
        dropped
    }
}

impl Scheduler for PhaseScheduler {
    fn next(&mut self) -> Option<ScheduledStep> {
        self.queue.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::super::plan::{Backend, Plan, PlanStep};
    use super::*;

    #[test]
    fn expands_phase_order_and_repeats() {
        let plan = Plan {
            name: "x".into(),
            backend: Backend::SparseBitSlice,
            num_replicas: 8,
            temperatures: vec![1.0],
            steps: vec![
                PlanStep {
                    operator: "finish".into(),
                    phase: Phase::Finish,
                    sweeps: 1,
                    repeat: 1,
                },
                PlanStep {
                    operator: "explore".into(),
                    phase: Phase::Explore,
                    sweeps: 5,
                    repeat: 3,
                },
            ],
            seed: 1,
            rationale: Default::default(),
        };
        let mut s = PhaseScheduler::from_plan(&plan);
        // Explore (×3) comes before Finish regardless of declaration order.
        let names: Vec<String> = std::iter::from_fn(|| s.next().map(|st| st.operator)).collect();
        assert_eq!(names, vec!["explore", "explore", "explore", "finish"]);
    }
}
