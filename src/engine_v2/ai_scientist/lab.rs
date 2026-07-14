//! Scientific Lab (Stage 5+) — the AI Scientist as a TEAM of specialized agents,
//! not one monolith. Each owns one responsibility and nothing else:
//!
//!   - `HypothesisGenerator` — reasons from operator CAPABILITIES and the
//!     knowledge graph to propose ideas WITH a causal argument (Theory Engine).
//!   - `ExperimentDesigner`  — turns ideas into experiment plans, expands them
//!     with the Evolution Engine, and applies NOVELTY SEARCH so the batch stays
//!     architecturally diverse.
//!   - `Statistician`        — confidence intervals + significance; decides
//!     confirmed / refuted / inconclusive.
//!   - `KnowledgeManager`    — writes only reproducible patterns into the
//!     Knowledge Graph (and the flat knowledge base).
//!
//! `ScientificLab` coordinates them into the discovery loop. It NEVER computes:
//! all experiments go through a `BatchExecutor` (which owns the read-only
//! Runtime). Fitness is COST-AWARE — energy minus compute, memory, complexity,
//! plus a novelty reward — so a bloated "million-operator" plan cannot win by
//! quality alone. Fully deterministic (ADR-0004).

use super::super::capability::Capability;
use super::super::decision::DecisionEngine;
use super::super::evolution::{Evolver, Schedule};
use super::super::ir::ProblemIR;
use super::super::knowledge::{Experience, InstanceFeatures, KnowledgeBase};
use super::super::registry::OperatorRegistry;
use super::db::{ExperimentDb, ExperimentRecord};
use super::executor::{BatchExecutor, ExperimentTask};
use super::graph::KnowledgeGraph;
use super::novelty::NoveltyArchive;
use super::scientist::{AIScientist, Analysis, Hypothesis, HypothesisStatus};
use super::stats::compare;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// Cost-aware fitness weights (the user's `Energy − Time − Memory − Complexity`).
/// Fitness is MINIMIZED: `score + cost·work + memory·bytes + complexity·n_ops −
/// novelty·novelty_score`. Novelty lowers fitness, rewarding new architectures.
#[derive(Debug, Clone, Copy)]
pub struct FitnessWeights {
    pub cost: f64,
    pub memory: f64,
    pub complexity: f64,
    pub novelty: f64,
}

impl Default for FitnessWeights {
    fn default() -> Self {
        Self {
            cost: 1e-7,
            memory: 1e-9,
            complexity: 0.5,
            novelty: 2.0,
        }
    }
}

impl FitnessWeights {
    pub fn fitness(&self, score: f64, work: f64, mem: f64, n_ops: usize, novelty: f64) -> f64 {
        score + self.cost * work + self.memory * mem + self.complexity * n_ops as f64
            - self.novelty * novelty
    }
}

/// Lab campaign configuration.
#[derive(Debug, Clone)]
pub struct LabConfig {
    pub rounds: usize,
    pub hypotheses_per_round: usize,
    pub batch_size: usize,
    pub seeds_per_hypothesis: usize,
    pub num_replicas: usize,
    pub max_ops: usize,
    pub base_seed: u64,
    pub alpha: f64,
    pub fitness: FitnessWeights,
    /// Candidates whose novelty is below this are dropped (novelty search).
    pub novelty_floor: f64,
}

impl Default for LabConfig {
    fn default() -> Self {
        Self {
            rounds: 4,
            hypotheses_per_round: 12,
            batch_size: 48,
            seeds_per_hypothesis: 4,
            num_replicas: 32,
            max_ops: 4,
            base_seed: 0x5C1AB,
            alpha: 0.05,
            fitness: FitnessWeights::default(),
            novelty_floor: 0.15,
        }
    }
}

// ============================================================ agent 1: ideas
/// Everything an idea-generator needs to reason about the current state of the
/// research. Passed to any `Ideator` (heuristic OR an LLM), so the two are
/// interchangeable in the lab.
pub struct ResearchBrief<'a> {
    pub stats: &'a super::super::decision::InstanceStats,
    pub analysis: &'a Analysis,
    pub graph: &'a KnowledgeGraph,
    pub pool: &'a [&'static str],
    pub registry: &'a OperatorRegistry,
    /// A short instance-context tag ("sparse"/"dense").
    pub context: &'a str,
}

/// The scientist ROLE: propose hypotheses from a research brief. Implemented by
/// the built-in `HypothesisGenerator` (capability heuristics) and by an LLM
/// (`super::llm::LlmHypothesisGenerator`). The LLM proposes 20–100 reasoned
/// ideas; the deterministic core (Evolution + Runtime + Statistician) does all
/// the heavy computation — the LLM never enumerates millions of options.
pub trait Ideator {
    fn propose(&mut self, brief: &ResearchBrief, n: usize, rng: &mut ChaCha8Rng)
        -> Vec<Hypothesis>;
}

/// Proposes hypotheses with explicit capability-grounded reasoning.
#[derive(Debug, Default)]
pub struct HypothesisGenerator {
    next_id: u64,
}

impl Ideator for HypothesisGenerator {
    fn propose(
        &mut self,
        brief: &ResearchBrief,
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        self.generate(
            brief.analysis,
            brief.graph,
            brief.pool,
            brief.registry,
            brief.context,
            n,
            rng,
        )
    }
}

impl HypothesisGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    fn caps_of(
        registry: &OperatorRegistry,
        name: &str,
    ) -> Option<super::super::capability::CapabilitySet> {
        registry.metadata(name).map(|d| d.capabilities)
    }

    /// First pool operator that provides `cap` (capability-based selection).
    fn provider<'a>(
        pool: &[&'a str],
        registry: &OperatorRegistry,
        cap: Capability,
    ) -> Option<&'a str> {
        pool.iter()
            .copied()
            .find(|n| Self::caps_of(registry, n).is_some_and(|c| c.contains(cap)))
    }

    fn make(
        &mut self,
        ops: Vec<String>,
        rationale: String,
        reasoning: String,
        pred: f64,
    ) -> Hypothesis {
        let id = self.next_id;
        self.next_id += 1;
        Hypothesis {
            id,
            operators: ops,
            rationale,
            reasoning,
            predicted_improvement: pred,
            status: HypothesisStatus::Proposed,
            observed_improvement: 0.0,
            confidence: 0.0,
        }
    }

    /// Generate `n` hypotheses using four reasoning strategies: capability-gap
    /// analysis of the current best, graph-guided sequencing, evidence-driven
    /// pairing, and novelty-driven exploration.
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        &mut self,
        analysis: &Analysis,
        graph: &KnowledgeGraph,
        pool: &[&'static str],
        registry: &OperatorRegistry,
        context: &str,
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        let mut out: Vec<Hypothesis> = Vec::with_capacity(n);
        let best_ops: Vec<String> = analysis
            .best_schedule
            .as_ref()
            .map(|s| s.ops.clone())
            .unwrap_or_default();

        // --- Strategy 1: capability-gap reasoning (Theory Engine) ------------
        // If the current best lacks a useful capability, hypothesize adding an
        // operator that provides it, and SAY WHY.
        let mut best_caps = super::super::capability::CapabilitySet::empty();
        for op in &best_ops {
            if let Some(c) = Self::caps_of(registry, op) {
                for cap in [
                    Capability::Exploration,
                    Capability::Exploitation,
                    Capability::BarrierCrossing,
                ] {
                    if c.contains(cap) {
                        best_caps = best_caps.with(cap);
                    }
                }
            }
        }
        let gaps = [
            (
                Capability::BarrierCrossing,
                "escape local minima it is trapped in",
            ),
            (
                Capability::Exploration,
                "diversify instead of over-exploiting",
            ),
            (
                Capability::Exploitation,
                "refine solutions it leaves unpolished",
            ),
        ];
        for (cap, why) in gaps {
            if out.len() >= n {
                break;
            }
            if !best_caps.contains(cap) {
                if let Some(op) = Self::provider(pool, registry, cap) {
                    let mut ops = best_ops.clone();
                    ops.push(op.to_string());
                    if ops.is_empty() {
                        ops.push(op.to_string());
                    }
                    out.push(self.make(
                        ops,
                        format!("add {op} for {cap:?}"),
                        format!("the current best lacks {cap:?}, so it may {why}; {op} provides {cap:?}"),
                        analysis.best_score.abs() * 0.01,
                    ));
                }
            }
        }

        // --- Strategy 2: graph-guided sequencing -----------------------------
        for (op, score, _) in analysis.operator_scores.iter().take(3) {
            if out.len() >= n {
                break;
            }
            if let Some(next) = graph.best_object(op, "precedes-well", 2) {
                out.push(self.make(
                    vec![op.clone(), next.to_string()],
                    format!("follow strong edge {op}->{next}"),
                    format!("the knowledge graph shows {op} precedes-well {next} on {context} instances (reproduced)"),
                    *score,
                ));
            }
        }

        // --- Strategy 3: evidence-driven pairing -----------------------------
        let winners: Vec<String> = analysis
            .operator_scores
            .iter()
            .filter(|(_, s, _)| *s >= 0.0)
            .take(3)
            .map(|(k, _, _)| k.clone())
            .collect();
        for i in 0..winners.len() {
            for j in 0..winners.len() {
                if i != j && out.len() < n {
                    out.push(self.make(
                        vec![winners[i].clone(), winners[j].clone()],
                        format!("pair {} + {}", winners[i], winners[j]),
                        format!(
                            "{} and {} each improved on {context}; their combination may compound",
                            winners[i], winners[j]
                        ),
                        0.0,
                    ));
                }
            }
        }

        // --- Strategy 4: novelty-driven exploration --------------------------
        while out.len() < n && !pool.is_empty() {
            let len = rng.gen_range(2..=pool.len().clamp(2, 4));
            let ops: Vec<String> = (0..len)
                .map(|_| pool[rng.gen_range(0..pool.len())].to_string())
                .filter(|o| !analysis.failing_operators.contains(o))
                .collect();
            let ops = if ops.is_empty() {
                vec![pool[rng.gen_range(0..pool.len())].to_string()]
            } else {
                ops
            };
            out.push(self.make(
                ops,
                "novel architecture".into(),
                "explore a region of algorithm-space not yet covered".into(),
                0.0,
            ));
        }
        out.truncate(n);
        out
    }
}

// ======================================================= agent 2: experiment
/// Turns hypotheses into concrete experiment plans and applies novelty search.
#[derive(Debug, Default)]
pub struct ExperimentDesigner;

impl ExperimentDesigner {
    pub fn new() -> Self {
        Self
    }

    fn schedule_from(
        hyp: &Hypothesis,
        pool: &[&'static str],
        max_ops: usize,
        rng: &mut ChaCha8Rng,
    ) -> Schedule {
        let mut ops: Vec<String> = hyp
            .operators
            .iter()
            .filter(|o| pool.contains(&o.as_str()))
            .cloned()
            .collect();
        ops.truncate(max_ops.max(1));
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

    /// One schedule per hypothesis (for statistical testing).
    pub fn hypothesis_schedules(
        &self,
        hyps: &[Hypothesis],
        pool: &[&'static str],
        max_ops: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Schedule> {
        hyps.iter()
            .map(|h| Self::schedule_from(h, pool, max_ops, rng))
            .collect()
    }

    /// Expand seeds into a candidate batch, then KEEP ONLY architecturally novel
    /// ones (novelty ≥ floor vs the archive). This is where near-duplicate
    /// algorithms get penalized out of the search.
    #[allow(clippy::too_many_arguments)]
    pub fn explore(
        &self,
        evolver: &Evolver,
        pool: &[&'static str],
        seeds: &[Schedule],
        archive: &NoveltyArchive,
        count: usize,
        floor: f64,
        seed: u64,
    ) -> Vec<Schedule> {
        let raw = evolver.expand(pool, seeds, count, seed);
        let mut local = archive.clone();
        let mut kept = Vec::with_capacity(raw.len());
        for sched in raw {
            if local.novelty(&sched) >= floor {
                local.add(&sched);
                kept.push(sched);
            }
        }
        kept
    }
}

// ====================================================== agent 3: statistician
/// The verdict on a hypothesis: significance, effect, and a confidence interval.
#[derive(Debug, Clone, Copy)]
pub struct Verdict {
    pub status: HypothesisStatus,
    pub observed_improvement: f64,
    pub confidence: f64,
    /// 95% confidence interval on the mean improvement over baseline.
    pub ci_low: f64,
    pub ci_high: f64,
}

#[derive(Debug, Default)]
pub struct Statistician;

impl Statistician {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate candidate scores against the baseline sample. Improvement =
    /// baseline − candidate (positive is better, i.e. lower energy).
    pub fn evaluate(&self, candidate: &[f64], baseline: &[f64], alpha: f64) -> Verdict {
        let c = compare(candidate, baseline);
        let improvement = c.mean_b - c.mean_a;
        // 95% CI on the mean improvement via the standard error of the difference.
        let se = (c.std_a * c.std_a / c.n_a.max(1) as f64
            + c.std_b * c.std_b / c.n_b.max(1) as f64)
            .sqrt();
        let (ci_low, ci_high) = (improvement - 1.96 * se, improvement + 1.96 * se);
        let status = if c.is_significant_improvement(alpha) {
            HypothesisStatus::Confirmed
        } else if c.mean_a > c.mean_b && c.p_value_approx < alpha {
            HypothesisStatus::Refuted
        } else {
            HypothesisStatus::Inconclusive
        };
        Verdict {
            status,
            observed_improvement: improvement,
            confidence: c.confidence(),
            ci_low,
            ci_high,
        }
    }
}

// ==================================================== agent 4: knowledge mgr
/// Writes durable, reproducible knowledge into the graph + knowledge base.
#[derive(Debug, Default)]
pub struct KnowledgeManager;

impl KnowledgeManager {
    pub fn new() -> Self {
        Self
    }

    /// Integrate this round's verdicts. Only CONFIRMED hypotheses add positive
    /// evidence (precedes-well edges + effective-on context); refuted ones are
    /// recorded as negative evidence so the same dead end is not re-explored.
    pub fn integrate(&self, graph: &mut KnowledgeGraph, hyps: &[Hypothesis], context: &str) {
        for h in hyps {
            let proof = format!("hypothesis #{} (conf {:.2})", h.id, h.confidence);
            match h.status {
                HypothesisStatus::Confirmed => {
                    for w in h.operators.windows(2) {
                        graph.observe_if(
                            &w[0],
                            "precedes-well",
                            &w[1],
                            "",
                            h.observed_improvement,
                            &proof,
                        );
                    }
                    for op in &h.operators {
                        graph.observe_if(
                            op,
                            "effective-on",
                            context,
                            "",
                            h.observed_improvement,
                            &proof,
                        );
                    }
                }
                HypothesisStatus::Refuted => {
                    for op in &h.operators {
                        graph.observe_if(
                            op,
                            "fails-on",
                            context,
                            "",
                            h.observed_improvement,
                            &proof,
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

// ============================================================== orchestrator
/// Outcome of a lab campaign.
#[derive(Debug, Clone)]
pub struct LabReport {
    pub best_schedule: Schedule,
    pub best_score: f64,
    pub best_state: Vec<u8>,
    pub best_fitness: f64,
    pub baseline: f64,
    pub rounds: usize,
    pub experiments_run: usize,
    pub confirmed: Vec<Hypothesis>,
    pub round_best: Vec<f64>,
    pub graph_facts: usize,
    pub archive_size: usize,
}

/// The lab: owns the four agents, the novelty archive, and the knowledge graph.
/// The idea-generating agent is a trait object, so it can be the built-in
/// heuristic OR an LLM — nothing else in the lab changes.
pub struct ScientificLab {
    ideator: Box<dyn Ideator>,
    designer: ExperimentDesigner,
    statistician: Statistician,
    knowledge: KnowledgeManager,
    archive: NoveltyArchive,
    graph: KnowledgeGraph,
    seed: u64,
    decisions: Vec<String>,
}

impl ScientificLab {
    /// Lab with the built-in heuristic idea generator (deterministic).
    pub fn new(seed: u64) -> Self {
        Self::with_graph(seed, KnowledgeGraph::new())
    }

    /// Start from an existing knowledge graph (long-term learning across runs).
    pub fn with_graph(seed: u64, graph: KnowledgeGraph) -> Self {
        Self::with_ideator(seed, graph, Box::new(HypothesisGenerator::new()))
    }

    /// Lab driven by a custom idea generator — e.g. an LLM scientist. The LLM
    /// proposes; the deterministic core still does all computation.
    pub fn with_ideator(seed: u64, graph: KnowledgeGraph, ideator: Box<dyn Ideator>) -> Self {
        Self {
            ideator,
            designer: ExperimentDesigner::new(),
            statistician: Statistician::new(),
            knowledge: KnowledgeManager::new(),
            archive: NoveltyArchive::new(),
            graph,
            seed,
            decisions: Vec::new(),
        }
    }

    pub fn graph(&self) -> &KnowledgeGraph {
        &self.graph
    }
    pub fn decisions(&self) -> &[String] {
        &self.decisions
    }

    /// Run a discovery campaign. The lab proposes and analyzes; `executor`
    /// (Runtime) computes; `db` records every result; `kb`/graph accumulate
    /// knowledge. Deterministic.
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        evolver: &Evolver,
        executor: &dyn BatchExecutor,
        db: &mut ExperimentDb,
        kb: &mut KnowledgeBase,
        cfg: &LabConfig,
    ) -> LabReport {
        let stats = DecisionEngine::analyze(ir);
        let backend = stats.select_backend();
        let backend_name = format!("{backend:?}");
        let context = if stats.density < 0.05 {
            "sparse"
        } else {
            "dense"
        };
        let pool = DecisionEngine::operator_pool(&stats, backend, registry);
        let mem = (ir.n * cfg.num_replicas * 8) as f64;
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);

        // Baseline sample.
        let base_sched = baseline_schedule(&pool);
        let base_tasks: Vec<ExperimentTask> = (0..cfg.seeds_per_hypothesis.max(2))
            .map(|s| ExperimentTask {
                schedule: base_sched.clone(),
                num_replicas: cfg.num_replicas,
                seed: cfg.base_seed.wrapping_add(s as u64),
            })
            .collect();
        let base_out = executor.run_batch(ir, registry, &base_tasks);
        let baseline_scores: Vec<f64> = base_out.iter().map(|o| o.score).collect();
        let baseline = baseline_scores.iter().sum::<f64>() / baseline_scores.len().max(1) as f64;

        let mut best_score = f64::INFINITY;
        let mut best_fitness = f64::INFINITY;
        let mut best_schedule = base_sched.clone();
        let mut best_state = vec![0u8; ir.n];
        let mut round_best = Vec::with_capacity(cfg.rounds);
        let mut confirmed: Vec<Hypothesis> = Vec::new();
        let mut seed_counter = cfg.base_seed.wrapping_add(1000);

        for round in 0..cfg.rounds {
            let analysis = AIScientist::analyze(db);
            // Agent 1: the idea generator (heuristic OR LLM) proposes hypotheses
            // with reasoning, from a full research brief.
            let brief = ResearchBrief {
                stats: &stats,
                analysis: &analysis,
                graph: &self.graph,
                pool: &pool,
                registry,
                context,
            };
            let hyps = self
                .ideator
                .propose(&brief, cfg.hypotheses_per_round, &mut rng);
            // Agent 2: design experiments + novelty-filtered exploration.
            let hyp_scheds =
                self.designer
                    .hypothesis_schedules(&hyps, &pool, cfg.max_ops, &mut rng);
            let mut expand_seeds = hyp_scheds.clone();
            expand_seeds.push(best_schedule.clone());
            let exploration = self.designer.explore(
                evolver,
                &pool,
                &expand_seeds,
                &self.archive,
                cfg.batch_size,
                cfg.novelty_floor,
                cfg.base_seed.wrapping_add(round as u64 * 7919),
            );

            // Build the task batch (hypothesis tests repeated for statistics).
            let mut tagged: Vec<(Option<u64>, ExperimentTask)> = Vec::new();
            for (hyp, sched) in hyps.iter().zip(&hyp_scheds) {
                for _ in 0..cfg.seeds_per_hypothesis {
                    seed_counter = seed_counter.wrapping_add(1);
                    tagged.push((
                        Some(hyp.id),
                        ExperimentTask {
                            schedule: sched.clone(),
                            num_replicas: cfg.num_replicas,
                            seed: seed_counter,
                        },
                    ));
                }
            }
            for sched in exploration {
                seed_counter = seed_counter.wrapping_add(1);
                tagged.push((
                    None,
                    ExperimentTask {
                        schedule: sched,
                        num_replicas: cfg.num_replicas,
                        seed: seed_counter,
                    },
                ));
            }

            // Runtime computes the whole batch.
            let plain: Vec<ExperimentTask> = tagged.iter().map(|(_, t)| t.clone()).collect();
            let batch_start = std::time::Instant::now();
            let outcomes = executor.run_batch(ir, registry, &plain);
            // Approximate per-task wall time (batch runs in parallel).
            let task_ms = batch_start.elapsed().as_secs_f64() * 1e3 / plain.len().max(1) as f64;

            // Record every result; update archive; track cost-aware best.
            let mut hyp_scores: HashMap<u64, Vec<f64>> = HashMap::new();
            for ((hyp_id, task), out) in tagged.iter().zip(&outcomes) {
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
                let novelty = self.archive.novelty(&task.schedule);
                self.archive.add(&task.schedule);
                let fit =
                    cfg.fitness
                        .fitness(out.score, out.work, mem, task.schedule.ops.len(), novelty);
                if fit < best_fitness {
                    best_fitness = fit;
                    best_schedule = task.schedule.clone();
                }
                if out.score < best_score {
                    best_score = out.score;
                    best_state = out.best_state.clone();
                }
            }

            // Agent 3: statistician confirms/refutes each hypothesis.
            let mut evaluated = hyps;
            for hyp in &mut evaluated {
                if let Some(scores) = hyp_scores.get(&hyp.id) {
                    let v = self
                        .statistician
                        .evaluate(scores, &baseline_scores, cfg.alpha);
                    hyp.status = v.status;
                    hyp.observed_improvement = v.observed_improvement;
                    hyp.confidence = v.confidence;
                }
            }
            // Agent 4: knowledge manager integrates reproducible patterns.
            self.knowledge
                .integrate(&mut self.graph, &evaluated, context);
            for h in evaluated
                .iter()
                .filter(|h| h.status == HypothesisStatus::Confirmed)
            {
                confirmed.push(h.clone());
            }

            let n_conf = evaluated
                .iter()
                .filter(|h| h.status == HypothesisStatus::Confirmed)
                .count();
            self.decisions.push(format!(
                "round {round}: {} hypotheses, {n_conf} confirmed, best cut {:.0}, {} graph facts, {} explored",
                evaluated.len(),
                -best_score,
                self.graph.len(),
                self.archive.len(),
            ));
            kb.record(Experience {
                features: InstanceFeatures::from_stats(&stats),
                backend: backend_name.clone(),
                sequence: best_schedule.ops.clone(),
                sweeps: best_schedule.sweeps.clone(),
                temp_hi: best_schedule.temp_hi,
                temp_lo: best_schedule.temp_lo,
                best_energy: best_score,
                best_utility: best_fitness,
            });
            round_best.push(best_score);
        }

        LabReport {
            best_schedule,
            best_score,
            best_state,
            best_fitness,
            baseline,
            rounds: cfg.rounds,
            experiments_run: db.len(),
            confirmed,
            round_best,
            graph_facts: self.graph.len(),
            archive_size: self.archive.len(),
        }
    }
}

fn baseline_schedule(pool: &[&'static str]) -> Schedule {
    let pick = |name: &str| {
        if pool.contains(&name) {
            name.to_string()
        } else {
            pool[0].to_string()
        }
    };
    Schedule {
        ops: vec![pick("gibbs_color_sweep"), pick("greedy_descent")],
        sweeps: vec![20, 20],
        temp_hi: 4.0,
        temp_lo: 0.1,
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

    fn cfg() -> LabConfig {
        LabConfig {
            rounds: 3,
            hypotheses_per_round: 6,
            batch_size: 20,
            seeds_per_hypothesis: 3,
            num_replicas: 16,
            max_ops: 4,
            base_seed: 5,
            alpha: 0.05,
            fitness: FitnessWeights::default(),
            novelty_floor: 0.1,
        }
    }

    #[test]
    fn lab_is_deterministic_records_everything_and_builds_knowledge() {
        let ir = ring(24);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(4);

        let run = || {
            let mut db = ExperimentDb::new();
            let mut kb = KnowledgeBase::new();
            let mut lab = ScientificLab::new(5);
            let rep = lab.run(&ir, &reg, &evolver, &exec, &mut db, &mut kb, &cfg());
            (rep, db.len(), lab.graph().len(), lab.decisions().len())
        };
        let (r1, db1, g1, d1) = run();
        let (r2, db2, g2, d2) = run();

        assert_eq!(r1.best_score, r2.best_score);
        assert_eq!(r1.best_schedule, r2.best_schedule);
        assert_eq!(db1, db2);
        assert_eq!(g1, g2);
        assert_eq!(d1, d2);
        assert!(db1 > 0);
        assert_eq!(r1.round_best.len(), 3);
        for w in r1.round_best.windows(2) {
            assert!(w[1] <= w[0] + 1e-9); // best never worsens
        }
        assert_eq!(ir.energy(&r1.best_state), r1.best_score);
        assert!(r1.best_score <= r1.baseline + 1e-9);
        assert!(r1.archive_size > 0, "novelty archive should have grown");
    }

    #[test]
    fn confirmed_hypotheses_carry_reasoning() {
        let ir = ring(24);
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);
        let mut db = ExperimentDb::new();
        let mut kb = KnowledgeBase::new();
        let mut lab = ScientificLab::new(11);
        let rep = lab.run(&ir, &reg, &evolver, &exec, &mut db, &mut kb, &cfg());
        // Every confirmed hypothesis has a causal argument (Theory Engine).
        for h in &rep.confirmed {
            assert!(!h.reasoning.is_empty());
            assert!(h.confidence >= 0.0 && h.confidence <= 1.0);
        }
    }
}
