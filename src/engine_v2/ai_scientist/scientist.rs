//! `AIScientist` (Stage 5) — the layer ABOVE the optimization core. It reads the
//! experiment database, analyzes what worked, forms hypotheses about operator
//! combinations, turns them into candidate schedules, and drives the discovery
//! loop. It NEVER computes: every experiment is handed to a `BatchExecutor`
//! (which owns the read-only Runtime). The core is untouched.
//!
//! Loop (Constitution §12, Stage 5 workflow):
//!   analyze DB → hypotheses → candidate schedules → Evolution Engine expands
//!   → executor (Runtime) runs → DB records every result → analyze → repeat.
//!
//! Everything is deterministic: one seeded RNG drives hypothesis/candidate
//! generation, every experiment carries its own seed, and nothing is ever
//! overwritten (ADR-0004).

use super::super::decision::DecisionEngine;
use super::super::evolution::{Evolver, Schedule};
use super::super::ir::ProblemIR;
use super::super::knowledge::{Experience, InstanceFeatures, KnowledgeBase};
use super::super::registry::OperatorRegistry;
use super::db::{ExperimentDb, ExperimentRecord};
use super::executor::{BatchExecutor, ExperimentTask};
use super::stats::compare;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// Verdict on a hypothesis after its experiments run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    Proposed,
    Confirmed,
    Refuted,
    Inconclusive,
}

/// A structured, testable claim: "this operator combination will beat baseline".
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub id: u64,
    pub operators: Vec<String>,
    pub rationale: String,
    /// Theory Engine: the CAUSAL argument for the hypothesis — WHY these
    /// operators, in terms of their capabilities (e.g. "the current best lacks a
    /// barrier-crossing move, so it stalls in local minima; add ICM"). Recorded
    /// whether the hypothesis is later confirmed OR refuted.
    pub reasoning: String,
    pub predicted_improvement: f64,
    pub status: HypothesisStatus,
    /// Observed mean improvement over baseline once tested.
    pub observed_improvement: f64,
    /// Statistical confidence (1 − approx p) that it beats baseline.
    pub confidence: f64,
}

/// Aggregated read of the experiment database.
#[derive(Debug, Clone)]
pub struct Analysis {
    pub baseline: f64,
    pub experiments: usize,
    /// (operator, mean improvement over baseline, times seen), best first.
    pub operator_scores: Vec<(String, f64, usize)>,
    /// (op_a, op_b, mean improvement), best adjacent pairs first.
    pub best_pairs: Vec<(String, String, f64)>,
    /// Operators whose mean improvement is negative with enough evidence.
    pub failing_operators: Vec<String>,
    pub best_schedule: Option<Schedule>,
    pub best_score: f64,
}

/// Search-scale configuration for a discovery campaign.
#[derive(Debug, Clone)]
pub struct DiscoverConfig {
    pub rounds: usize,
    pub hypotheses_per_round: usize,
    /// Candidate schedules produced per round by the Evolution Engine.
    pub batch_size: usize,
    /// Repeat runs per hypothesis (for statistical significance).
    pub seeds_per_hypothesis: usize,
    pub num_replicas: usize,
    pub max_ops: usize,
    pub base_seed: u64,
    /// Significance threshold for confirming a hypothesis.
    pub alpha: f64,
}

impl Default for DiscoverConfig {
    fn default() -> Self {
        Self {
            rounds: 4,
            hypotheses_per_round: 12,
            batch_size: 40,
            seeds_per_hypothesis: 4,
            num_replicas: 32,
            max_ops: 4,
            base_seed: 0xA1_5C1,
            alpha: 0.05,
        }
    }
}

/// Outcome of a discovery campaign.
#[derive(Debug, Clone)]
pub struct DiscoveryReport {
    pub best_schedule: Schedule,
    pub best_score: f64,
    pub best_state: Vec<u8>,
    pub baseline: f64,
    pub rounds: usize,
    pub experiments_run: usize,
    pub confirmed: Vec<Hypothesis>,
    /// Best score after each round (monotone non-increasing).
    pub round_best: Vec<f64>,
}

/// The AI Scientist controller. Holds only generative/analytic state; no compute.
pub struct AIScientist {
    seed: u64,
    next_hyp_id: u64,
    decisions: Vec<String>,
}

impl AIScientist {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            next_hyp_id: 0,
            decisions: Vec::new(),
        }
    }

    /// The decision log — every choice the scientist made, in order.
    pub fn decisions(&self) -> &[String] {
        &self.decisions
    }

    // -------------------------------------------------------------- analysis
    /// Analyze the database: per-operator and per-pair mean improvement over
    /// baseline, failing operators, and the best schedule found so far. Shared
    /// by the single `AIScientist` and the multi-agent `ScientificLab`.
    pub fn analyze(db: &ExperimentDb) -> Analysis {
        let baseline = db
            .all()
            .first()
            .map(|r| r.baseline)
            .unwrap_or(f64::INFINITY);
        let mut op_imp: HashMap<String, (f64, usize)> = HashMap::new();
        let mut pair_imp: HashMap<(String, String), (f64, usize)> = HashMap::new();
        for r in db.all() {
            let imp = r.improvement();
            for op in &r.sequence {
                let e = op_imp.entry(op.clone()).or_insert((0.0, 0));
                e.0 += imp;
                e.1 += 1;
            }
            for w in r.sequence.windows(2) {
                let key = (w[0].clone(), w[1].clone());
                let e = pair_imp.entry(key).or_insert((0.0, 0));
                e.0 += imp;
                e.1 += 1;
            }
        }
        let mut operator_scores: Vec<(String, f64, usize)> = op_imp
            .into_iter()
            .map(|(k, (s, c))| (k, if c > 0 { s / c as f64 } else { 0.0 }, c))
            .collect();
        operator_scores.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));

        let mut best_pairs: Vec<(String, String, f64)> = pair_imp
            .into_iter()
            .map(|((a, b), (s, c))| (a, b, if c > 0 { s / c as f64 } else { 0.0 }))
            .collect();
        best_pairs.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
        best_pairs.truncate(8);

        let failing_operators: Vec<String> = operator_scores
            .iter()
            .filter(|(_, s, c)| *s < 0.0 && *c >= 2)
            .map(|(k, _, _)| k.clone())
            .collect();

        let best_schedule = db.best().map(|r| Schedule {
            ops: r.sequence.clone(),
            sweeps: r.sweeps.clone(),
            temp_hi: r.temp_hi,
            temp_lo: r.temp_lo,
        });
        let best_score = db.best().map(|r| r.score).unwrap_or(f64::INFINITY);

        Analysis {
            baseline,
            experiments: db.len(),
            operator_scores,
            best_pairs,
            failing_operators,
            best_schedule,
            best_score,
        }
    }

    // ---------------------------------------------------------- hypotheses
    /// Generate `n` hypotheses. With evidence, it combines the highest-improving
    /// operators and pairs and avoids the failing ones; cold, it proposes diverse
    /// combinations across the whole pool. This is the generative step that a
    /// larger deployment would delegate to an LLM producing 100–1000 hypotheses.
    pub fn generate_hypotheses(
        &mut self,
        analysis: &Analysis,
        pool: &[&'static str],
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        let mut out = Vec::with_capacity(n);
        let winners: Vec<String> = analysis
            .operator_scores
            .iter()
            .filter(|(_, s, _)| *s >= 0.0)
            .take(4)
            .map(|(k, _, _)| k.clone())
            .collect();

        // Evidence-driven: pairs of top operators, and top pairs extended by an
        // exploiter.
        if winners.len() >= 2 {
            for i in 0..winners.len() {
                for j in 0..winners.len() {
                    if i != j && out.len() < n {
                        let ops = vec![winners[i].clone(), winners[j].clone()];
                        let pred = self.predicted(&ops, analysis);
                        out.push(self.make_hypothesis(
                            ops,
                            format!("combine top operators {} + {}", winners[i], winners[j]),
                            pred,
                        ));
                    }
                }
            }
        }
        for (a, b, imp) in analysis.best_pairs.iter().take(4) {
            if out.len() >= n {
                break;
            }
            let mut ops = vec![a.clone(), b.clone()];
            if let Some(w) = winners.first() {
                ops.push(w.clone());
            }
            out.push(self.make_hypothesis(
                ops,
                format!("extend strong pair {a}->{b} (imp {imp:.2})"),
                *imp,
            ));
        }

        // Novelty / exploration: random combinations over the pool (never using
        // known-failing operators), to keep the search from collapsing.
        while out.len() < n && !pool.is_empty() {
            let len = rng.gen_range(2..=pool.len().clamp(2, 4));
            let mut ops = Vec::new();
            for _ in 0..len {
                let cand = pool[rng.gen_range(0..pool.len())].to_string();
                if !analysis.failing_operators.contains(&cand) {
                    ops.push(cand);
                }
            }
            if ops.is_empty() {
                ops.push(pool[rng.gen_range(0..pool.len())].to_string());
            }
            out.push(self.make_hypothesis(ops, "novel exploratory combination".into(), 0.0));
        }
        out.truncate(n);
        out
    }

    fn predicted(&self, ops: &[String], analysis: &Analysis) -> f64 {
        let mut sum = 0.0;
        let mut cnt = 0;
        for op in ops {
            if let Some((_, s, _)) = analysis.operator_scores.iter().find(|(k, _, _)| k == op) {
                sum += s;
                cnt += 1;
            }
        }
        if cnt > 0 {
            sum / cnt as f64
        } else {
            0.0
        }
    }

    fn make_hypothesis(&mut self, ops: Vec<String>, rationale: String, pred: f64) -> Hypothesis {
        let id = self.next_hyp_id;
        self.next_hyp_id += 1;
        Hypothesis {
            id,
            reasoning: rationale.clone(),
            operators: ops,
            rationale,
            predicted_improvement: pred,
            status: HypothesisStatus::Proposed,
            observed_improvement: 0.0,
            confidence: 0.0,
        }
    }

    /// Turn a hypothesis into a concrete candidate schedule (parameters sampled
    /// deterministically). Operators not in the current pool are dropped.
    fn schedule_from(
        &self,
        hyp: &Hypothesis,
        pool: &[&'static str],
        rng: &mut ChaCha8Rng,
    ) -> Schedule {
        let mut ops: Vec<String> = hyp
            .operators
            .iter()
            .filter(|o| pool.contains(&o.as_str()))
            .cloned()
            .collect();
        if ops.is_empty() {
            ops.push(pool[rng.gen_range(0..pool.len())].to_string());
        }
        let sweeps = (0..ops.len()).map(|_| rng.gen_range(5..=25)).collect();
        Schedule {
            ops,
            sweeps,
            temp_hi: rng.gen_range(2.0..6.0),
            temp_lo: rng.gen_range(0.05..0.5),
        }
    }

    fn baseline_schedule(pool: &[&'static str]) -> Schedule {
        let pick = |name: &str, fallback: usize| {
            if pool.contains(&name) {
                name.to_string()
            } else {
                pool[fallback.min(pool.len() - 1)].to_string()
            }
        };
        Schedule {
            ops: vec![pick("gibbs_color_sweep", 0), pick("greedy_descent", 0)],
            sweeps: vec![20, 20],
            temp_hi: 4.0,
            temp_lo: 0.1,
        }
    }

    // ------------------------------------------------------------- discover
    /// Run a full discovery campaign. The scientist proposes; the `executor`
    /// (Runtime) computes; the `db` records every result; the `kb` accumulates
    /// long-term knowledge. Deterministic.
    #[allow(clippy::too_many_arguments)]
    pub fn discover(
        &mut self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        evolver: &Evolver,
        executor: &dyn BatchExecutor,
        db: &mut ExperimentDb,
        kb: &mut KnowledgeBase,
        cfg: &DiscoverConfig,
    ) -> DiscoveryReport {
        let stats = DecisionEngine::analyze(ir);
        let backend = stats.select_backend();
        let backend_name = format!("{backend:?}");
        let pool = DecisionEngine::operator_pool(&stats, backend, registry);
        let max_ops = cfg.max_ops.max(1);
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);

        // Establish a baseline sample (single reference schedule, several seeds).
        let base_sched = Self::baseline_schedule(&pool);
        let base_tasks: Vec<ExperimentTask> = (0..cfg.seeds_per_hypothesis.max(2))
            .map(|s| ExperimentTask {
                schedule: base_sched.clone(),
                num_replicas: cfg.num_replicas,
                seed: cfg.base_seed.wrapping_add(s as u64),
            })
            .collect();
        let base_out = executor.run_batch(ir, registry, &base_tasks);
        let baseline_scores: Vec<f64> = base_out.iter().map(|o| o.score).collect();
        let baseline = baseline_scores.iter().copied().fold(0.0, |a, b| a + b)
            / baseline_scores.len().max(1) as f64;

        let mut best_score = f64::INFINITY;
        let mut best_schedule = base_sched.clone();
        let mut best_state = vec![0u8; ir.n];
        let mut round_best = Vec::with_capacity(cfg.rounds);
        let mut confirmed: Vec<Hypothesis> = Vec::new();
        let mut seed_counter = cfg.base_seed.wrapping_add(1000);

        for round in 0..cfg.rounds {
            let analysis = Self::analyze(db);
            let mut hyps =
                self.generate_hypotheses(&analysis, &pool, cfg.hypotheses_per_round, &mut rng);
            for h in &mut hyps {
                h.operators.truncate(max_ops); // honor the configured plan-length cap
            }

            // Hypothesis-test tasks (repeated seeds ⇒ statistics), tagged by id.
            let mut tasks: Vec<(Option<u64>, ExperimentTask)> = Vec::new();
            let mut hyp_schedules: Vec<Schedule> = Vec::new();
            for hyp in &hyps {
                let sched = self.schedule_from(hyp, &pool, &mut rng);
                hyp_schedules.push(sched.clone());
                for _ in 0..cfg.seeds_per_hypothesis {
                    seed_counter = seed_counter.wrapping_add(1);
                    tasks.push((
                        Some(hyp.id),
                        ExperimentTask {
                            schedule: sched.clone(),
                            num_replicas: cfg.num_replicas,
                            seed: seed_counter,
                        },
                    ));
                }
            }

            // Exploration batch: Evolution Engine expands hypothesis schedules
            // (+ best so far) into many candidates; 1 seed each.
            let mut seeds_for_expand = hyp_schedules.clone();
            seeds_for_expand.push(best_schedule.clone());
            let expand_seed = cfg.base_seed.wrapping_add(round as u64 * 7919);
            let batch = evolver.expand(&pool, &seeds_for_expand, cfg.batch_size, expand_seed);
            for sched in batch {
                seed_counter = seed_counter.wrapping_add(1);
                tasks.push((
                    None,
                    ExperimentTask {
                        schedule: sched,
                        num_replicas: cfg.num_replicas,
                        seed: seed_counter,
                    },
                ));
            }

            // Runtime executes the whole batch (the scientist does not compute).
            let plain: Vec<ExperimentTask> = tasks.iter().map(|(_, t)| t.clone()).collect();
            let batch_start = std::time::Instant::now();
            let outcomes = executor.run_batch(ir, registry, &plain);
            // Approximate per-task wall time (batch runs in parallel).
            let task_ms = batch_start.elapsed().as_secs_f64() * 1e3 / plain.len().max(1) as f64;

            // Record EVERY result (append-only) and collect per-hypothesis scores.
            let mut hyp_scores: HashMap<u64, Vec<f64>> = HashMap::new();
            for ((hyp_id, task), out) in tasks.iter().zip(&outcomes) {
                db.record(ExperimentRecord {
                    id: 0,
                    hypothesis_id: *hyp_id,
                    sequence: task.schedule.ops.clone(),
                    sweeps: task.schedule.sweeps.clone(),
                    temp_hi: task.schedule.temp_hi,
                    temp_lo: task.schedule.temp_lo,
                    num_replicas: task.num_replicas,
                    seed: task.seed,
                    backend: backend_name.clone(),
                    work: out.work,
                    score: out.score,
                    baseline,
                    density: stats.density,
                    clustering: stats.clustering,
                    wall_ms: task_ms,
                    ..Default::default()
                });
                if let Some(id) = hyp_id {
                    hyp_scores.entry(*id).or_default().push(out.score);
                }
                if out.score < best_score {
                    best_score = out.score;
                    best_schedule = task.schedule.clone();
                    best_state = out.best_state.clone();
                }
            }

            // Confirm / refute hypotheses by statistical comparison vs baseline.
            for hyp in &mut hyps {
                if let Some(scores) = hyp_scores.get(&hyp.id) {
                    let cmp = compare(scores, &baseline_scores);
                    hyp.observed_improvement = cmp.mean_b - cmp.mean_a;
                    hyp.confidence = cmp.confidence();
                    hyp.status = if cmp.is_significant_improvement(cfg.alpha) {
                        HypothesisStatus::Confirmed
                    } else if cmp.mean_a > cmp.mean_b && cmp.p_value_approx < cfg.alpha {
                        HypothesisStatus::Refuted
                    } else {
                        HypothesisStatus::Inconclusive
                    };
                }
            }
            for h in hyps
                .iter()
                .filter(|h| h.status == HypothesisStatus::Confirmed)
            {
                confirmed.push(h.clone());
            }

            // Record the decision + persist knowledge for long-term learning.
            let n_conf = hyps
                .iter()
                .filter(|h| h.status == HypothesisStatus::Confirmed)
                .count();
            self.decisions.push(format!(
                "round {round}: {} hypotheses, {n_conf} confirmed, best score {:.3} (baseline {:.3})",
                hyps.len(),
                best_score,
                baseline
            ));
            kb.record(Experience {
                features: InstanceFeatures::from_stats(&stats),
                backend: backend_name.clone(),
                sequence: best_schedule.ops.clone(),
                sweeps: best_schedule.sweeps.clone(),
                temp_hi: best_schedule.temp_hi,
                temp_lo: best_schedule.temp_lo,
                best_energy: best_score,
                best_utility: best_score,
            });
            round_best.push(best_score);
        }

        DiscoveryReport {
            best_schedule,
            best_score,
            best_state,
            baseline,
            rounds: cfg.rounds,
            experiments_run: db.len(),
            confirmed,
            round_best,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::executor::RuntimeExecutor;
    use super::*;

    fn ring(n: usize) -> ProblemIR {
        let mut pairs = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, b) = (i.min(j) as u32, i.max(j) as u32);
            pairs.push((a, b, if i % 2 == 0 { 1.0 } else { -1.0 }));
        }
        pairs.push((0, (n / 2) as u32, 1.0));
        ProblemIR::from_pairs(n, 0.0, vec![0.0; n], &pairs)
    }

    fn small_cfg() -> DiscoverConfig {
        DiscoverConfig {
            rounds: 3,
            hypotheses_per_round: 6,
            batch_size: 16,
            seeds_per_hypothesis: 3,
            num_replicas: 16,
            max_ops: 4,
            base_seed: 7,
            alpha: 0.05,
        }
    }

    #[test]
    fn discovery_is_deterministic_and_records_everything() {
        let ir = ring(24);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(4);

        let run = |seed: u64| {
            let mut db = ExperimentDb::new();
            let mut kb = KnowledgeBase::new();
            let mut sci = AIScientist::new(seed);
            let rep = sci.discover(&ir, &reg, &evolver, &exec, &mut db, &mut kb, &small_cfg());
            (rep, db.len(), kb.len(), sci.decisions().len())
        };
        let (r1, db1, kb1, d1) = run(7);
        let (r2, db2, kb2, d2) = run(7);

        // Determinism: identical seed ⇒ identical campaign.
        assert_eq!(r1.best_score, r2.best_score);
        assert_eq!(r1.best_schedule, r2.best_schedule);
        assert_eq!(db1, db2);
        assert_eq!(kb1, kb2);
        assert_eq!(d1, d2);

        // Every experiment was recorded; nothing overwritten.
        assert!(db1 > 0);
        assert_eq!(r1.round_best.len(), 3);
        // Best score never worsens across rounds.
        for w in r1.round_best.windows(2) {
            assert!(w[1] <= w[0] + 1e-9);
        }
        // Reported best is the canonical score of the reported state.
        assert_eq!(ir.energy(&r1.best_state), r1.best_score);
        // The discovered algorithm is at least as good as the baseline.
        assert!(r1.best_score <= r1.baseline + 1e-9);
    }
}
