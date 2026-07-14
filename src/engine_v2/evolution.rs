//! Evolution Engine — the "search for an algorithm" (Constitution §10, §11;
//! Stage 4 "Foundation of Intelligence"). Instead of running a fixed pipeline
//! (PT → ICM → PA), the engine SEARCHES the space of operator SEQUENCES and
//! discovers which composition of physical processes solves a given instance
//! best:
//!
//! ```text
//! Operator A → C → A → F → D → C     ← found automatically, not hand-written
//! ```
//!
//! A candidate algorithm is a `Genome`: an ordered list of operators (drawn ONLY
//! from the capability-selected pool — never named directly, Task 4) plus their
//! sweep budgets and the ensemble temperature ladder. Fitness is measured, not
//! assumed: each genome is compiled to a `Plan`, executed on the real instance
//! through the `Runtime`, and scored by the best energy it reaches. A genetic
//! loop (elitist selection + crossover + mutation) evolves the population.
//!
//! Everything is deterministic (ADR-0004): one seeded RNG drives all variation,
//! and every genome is evaluated with the SAME fixed run-seed so fitness
//! comparisons are fair and reproducible.

use super::backends::{ReferenceState, SparseBitSlice};
use super::context::RunContext;
use super::decision::{geometric_ladder, DecisionEngine};
use super::ir::ProblemIR;
use super::knowledge::{Experience, InstanceFeatures, KnowledgeBase};
use super::operator::InstanceShape;
use super::plan::{Backend, Phase, Plan, PlanStep};
use super::registry::OperatorRegistry;
use super::runtime::Runtime;
use super::state::SpinState;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Search hyper-parameters. All bounded so a search is a few seconds.
#[derive(Debug, Clone)]
pub struct EvolveConfig {
    pub population: usize,
    pub generations: usize,
    /// Max operators in a discovered sequence.
    pub max_steps: usize,
    pub num_replicas: usize,
    /// Upper bound on a single step's sweep budget.
    pub max_sweeps_per_step: u32,
    /// Seed for the fitness runs — FIXED across genomes for a fair comparison.
    pub run_seed: u64,
    /// Seed for the evolutionary variation itself.
    pub evolve_seed: u64,
    /// How the four criteria trade off in the fitness (see `UtilityWeights`).
    pub utility: UtilityWeights,
}

/// Multi-criteria fitness weights (Constitution §10; the user's utility idea).
/// Fitness is MINIMIZED: `quality·energy + cost·work + memory·bytes +
/// latency·steps`. All inputs are deterministic (work is an operation COUNT,
/// not wall-clock; memory is an exact byte estimate), so fitness comparisons
/// stay reproducible (ADR-0004). Defaults keep solution quality dominant while
/// letting compute/memory/latency break ties toward the cheaper algorithm.
#[derive(Debug, Clone, Copy)]
pub struct UtilityWeights {
    /// Weight on solution quality (best energy). Usually 1.0.
    pub quality: f64,
    /// Weight per unit of computational work (spin-update operations).
    pub cost: f64,
    /// Weight per byte of ensemble memory.
    pub memory: f64,
    /// Weight per operator invocation (scheduling latency / plan length).
    pub latency: f64,
}

impl Default for UtilityWeights {
    fn default() -> Self {
        Self {
            quality: 1.0,
            cost: 1e-7,
            memory: 1e-9,
            latency: 1e-3,
        }
    }
}

impl UtilityWeights {
    /// Quality-only weighting (pure best-energy fitness).
    pub fn quality_only() -> Self {
        Self {
            quality: 1.0,
            cost: 0.0,
            memory: 0.0,
            latency: 0.0,
        }
    }
}

impl Default for EvolveConfig {
    fn default() -> Self {
        Self {
            population: 16,
            generations: 8,
            max_steps: 5,
            num_replicas: 64,
            max_sweeps_per_step: 40,
            run_seed: 0xE7_0100,
            evolve_seed: 0xE7_0EE7,
            utility: UtilityWeights::default(),
        }
    }
}

/// One candidate algorithm: a sequence of operators (indices into the pool),
/// their per-step sweep budgets, and the ensemble temperature ladder endpoints.
#[derive(Debug, Clone)]
struct Genome {
    ops: Vec<usize>,
    sweeps: Vec<u32>,
    t_hi: f64,
    t_lo: f64,
}

struct Scored {
    genome: Genome,
    fitness: f64, // best energy reached (lower is better)
}

/// The discovered algorithm and how it was found.
#[derive(Debug, Clone)]
pub struct EvolveResult {
    pub best_energy: f64,
    pub best_state: Vec<u8>,
    /// Multi-criteria fitness of the winner (lower is better).
    pub best_utility: f64,
    /// Winner's computational work (spin-update operation count).
    pub best_work: f64,
    /// Winner's ensemble memory estimate (bytes).
    pub best_memory_bytes: f64,
    /// The discovered operator sequence, in execution order.
    pub sequence: Vec<String>,
    pub sweeps: Vec<u32>,
    pub temp_hi: f64,
    pub temp_lo: f64,
    pub backend: Backend,
    /// Operators the Decision Engine made available (capability-selected).
    pub pool: Vec<&'static str>,
    /// Best fitness (utility) at the end of each generation (non-increasing).
    pub generation_best: Vec<f64>,
    pub evaluations: usize,
    /// The concrete, replayable plan realizing the best genome.
    pub best_plan: Plan,
}

impl EvolveResult {
    /// Package this outcome as a `KnowledgeBase` `Experience` so it can feed
    /// future meta-learning (Constitution §12). Features are recomputed from the
    /// instance the plan was evolved for.
    pub fn to_experience(&self, ir: &ProblemIR) -> Experience {
        let stats = DecisionEngine::analyze(ir);
        Experience {
            features: InstanceFeatures::from_stats(&stats),
            backend: format!("{:?}", self.backend),
            sequence: self.sequence.clone(),
            sweeps: self.sweeps.clone(),
            temp_hi: self.temp_hi,
            temp_lo: self.temp_lo,
            best_energy: self.best_energy,
            best_utility: self.best_utility,
        }
    }
}

/// A backend-agnostic algorithm recipe: an ordered operator sequence with
/// per-step sweep budgets and a temperature ladder. This is the unit the AI
/// Scientist (Stage 5) generates and the Evolution Engine mutates/recombines;
/// the executor lowers it to a `Plan` for the (read-only) Runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {
    pub ops: Vec<String>,
    pub sweeps: Vec<u32>,
    pub temp_hi: f64,
    pub temp_lo: f64,
}

pub struct Evolver {
    cfg: EvolveConfig,
}

impl Evolver {
    pub fn new(cfg: EvolveConfig) -> Self {
        Self { cfg }
    }

    /// Evolve an execution plan for `ir`, composing operators drawn by
    /// capability from `registry`. Deterministic given the config seeds.
    pub fn evolve(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
    ) -> Result<EvolveResult, String> {
        self.evolve_impl(ir, registry, &[])
    }

    /// Meta-learning entry point (Constitution §12): evolve, but SEED the initial
    /// population from operator sequences that worked on instances similar to
    /// this one in `kb` — knowledge transfer between tasks. With elitism, a
    /// transferred plan can only help: the search starts from proven algorithms.
    /// Record the returned result back into the base to keep learning.
    pub fn evolve_with_kb(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        kb: &KnowledgeBase,
    ) -> Result<EvolveResult, String> {
        let stats = DecisionEngine::analyze(ir);
        let backend = stats.select_backend();
        let pool = DecisionEngine::operator_pool(&stats, backend, registry);
        let feats = InstanceFeatures::from_stats(&stats);
        // Take a handful of the most similar past experiences as seeds.
        let seeds: Vec<Genome> = kb
            .similar(&feats, self.cfg.population / 2)
            .into_iter()
            .filter_map(|e| {
                self.genome_from_sequence(&pool, &e.sequence, &e.sweeps, e.temp_hi, e.temp_lo)
            })
            .collect();
        self.evolve_impl(ir, registry, &seeds)
    }

    /// Turn a recorded operator-name sequence into a genome for the CURRENT pool,
    /// dropping any operators no longer available. `None` if nothing maps.
    fn genome_from_sequence(
        &self,
        pool: &[&'static str],
        sequence: &[String],
        sweeps: &[u32],
        t_hi: f64,
        t_lo: f64,
    ) -> Option<Genome> {
        let mut ops = Vec::new();
        let mut sw = Vec::new();
        for (i, name) in sequence.iter().enumerate() {
            if let Some(idx) = pool.iter().position(|&p| p == name.as_str()) {
                ops.push(idx);
                sw.push(*sweeps.get(i).unwrap_or(&10).max(&1));
            }
            if ops.len() >= self.cfg.max_steps {
                break;
            }
        }
        if ops.is_empty() {
            return None;
        }
        Some(Genome {
            ops,
            sweeps: sw,
            t_hi,
            t_lo,
        })
    }

    fn genome_to_schedule(&self, pool: &[&'static str], g: &Genome) -> Schedule {
        Schedule {
            ops: g.ops.iter().map(|&i| pool[i].to_string()).collect(),
            sweeps: g.sweeps.clone(),
            temp_hi: g.t_hi,
            temp_lo: g.t_lo,
        }
    }

    /// Massively expand `seeds` into `count` candidate schedules by mutation and
    /// recombination — PURE GENERATION, no execution (the Runtime is untouched
    /// here). This is the Evolution Engine's mutate/recombine step exposed for
    /// the AI Scientist (Stage 5): the scientist proposes seeds, this fans them
    /// out into a large batch, and the executor runs them. Deterministic in
    /// `seed`. The seeds themselves are preserved at the front of the batch.
    pub fn expand(
        &self,
        pool: &[&'static str],
        seeds: &[Schedule],
        count: usize,
        seed: u64,
    ) -> Vec<Schedule> {
        if pool.is_empty() || count == 0 {
            return Vec::new();
        }
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut base: Vec<Genome> = seeds
            .iter()
            .filter_map(|s| {
                self.genome_from_sequence(pool, &s.ops, &s.sweeps, s.temp_hi, s.temp_lo)
            })
            .collect();
        if base.is_empty() {
            base.push(self.random_genome(pool, &mut rng));
        }
        let mut out: Vec<Schedule> = Vec::with_capacity(count);
        // Keep the seeds verbatim (elitism at the batch level).
        for g in &base {
            if out.len() >= count {
                break;
            }
            out.push(self.genome_to_schedule(pool, g));
        }
        while out.len() < count {
            let a = &base[rng.gen_range(0..base.len())];
            let b = &base[rng.gen_range(0..base.len())];
            let mut child = self.crossover(a, b, &mut rng);
            self.mutate(&mut child, pool, &mut rng);
            out.push(self.genome_to_schedule(pool, &child));
        }
        out
    }

    fn evolve_impl(
        &self,
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        seeds: &[Genome],
    ) -> Result<EvolveResult, String> {
        let stats = DecisionEngine::analyze(ir);
        let backend = stats.select_backend();
        let pool = DecisionEngine::operator_pool(&stats, backend, registry);
        if pool.is_empty() {
            return Err("no capability-compatible operators for this instance".into());
        }

        let mut rng = ChaCha8Rng::seed_from_u64(self.cfg.evolve_seed);
        let mut evaluations = 0usize;

        // Initial population: transferred seeds first (validated against the
        // current pool), then random genomes to fill.
        let mut pop: Vec<Scored> = Vec::with_capacity(self.cfg.population);
        for g in seeds.iter().take(self.cfg.population) {
            if g.ops.iter().all(|&i| i < pool.len()) && !g.ops.is_empty() {
                let fitness = self.evaluate(ir, backend, &pool, g, registry);
                evaluations += 1;
                pop.push(Scored {
                    genome: g.clone(),
                    fitness,
                });
            }
        }
        while pop.len() < self.cfg.population {
            let g = self.random_genome(&pool, &mut rng);
            let fitness = self.evaluate(ir, backend, &pool, &g, registry);
            evaluations += 1;
            pop.push(Scored { genome: g, fitness });
        }

        let mut generation_best = Vec::with_capacity(self.cfg.generations);
        let elite = (self.cfg.population / 2).max(1);

        for _ in 0..self.cfg.generations {
            pop.sort_by(|a, b| a.fitness.total_cmp(&b.fitness));
            generation_best.push(pop[0].fitness);

            // Elitism: carry the top genomes forward unchanged (fitness reused).
            let mut next: Vec<Scored> = Vec::with_capacity(self.cfg.population);
            for s in pop.iter().take(elite) {
                next.push(Scored {
                    genome: s.genome.clone(),
                    fitness: s.fitness,
                });
            }
            // Fill the rest with mutated crossovers of the elites.
            while next.len() < self.cfg.population {
                let a = &pop[rng.gen_range(0..elite)].genome;
                let b = &pop[rng.gen_range(0..elite)].genome;
                let mut child = self.crossover(a, b, &mut rng);
                self.mutate(&mut child, &pool, &mut rng);
                let fitness = self.evaluate(ir, backend, &pool, &child, registry);
                evaluations += 1;
                next.push(Scored {
                    genome: child,
                    fitness,
                });
            }
            pop = next;
        }

        pop.sort_by(|a, b| a.fitness.total_cmp(&b.fitness));
        generation_best.push(pop[0].fitness);
        let best = &pop[0].genome;
        let plan = self.build_plan(&pool, best, backend);

        // Re-run the winner once to recover its best configuration + canonical
        // score (the Runtime already re-scores canonically — ADR-0005).
        let init = vec![0u8; ir.n];
        let mut state = boxed_state(ir, backend, self.cfg.num_replicas, &init);
        let mut rt = Runtime::new(RunContext::new(self.cfg.run_seed), &plan);
        let rec = rt.run(&plan, state.as_mut(), registry, ir)?;
        let (work, mem, latency) = self.costs(ir, &pool, best, registry);

        Ok(EvolveResult {
            best_energy: rec.best_energy,
            best_state: rec.best_state,
            best_utility: self.utility(rec.best_energy, work, mem, latency),
            best_work: work,
            best_memory_bytes: mem,
            sequence: best.ops.iter().map(|&i| pool[i].to_string()).collect(),
            sweeps: best.sweeps.clone(),
            temp_hi: best.t_hi,
            temp_lo: best.t_lo,
            backend,
            pool,
            generation_best,
            evaluations,
            best_plan: plan,
        })
    }

    fn random_genome(&self, pool: &[&'static str], rng: &mut ChaCha8Rng) -> Genome {
        let len = rng.gen_range(1..=self.cfg.max_steps);
        let ops = (0..len).map(|_| rng.gen_range(0..pool.len())).collect();
        let sweeps = (0..len)
            .map(|_| rng.gen_range(1..=self.cfg.max_sweeps_per_step))
            .collect();
        let t_hi = rng.gen_range(1.0..8.0);
        let t_lo = rng.gen_range(0.02..1.0);
        Genome {
            ops,
            sweeps,
            t_hi,
            t_lo,
        }
    }

    fn crossover(&self, a: &Genome, b: &Genome, rng: &mut ChaCha8Rng) -> Genome {
        // Single-point splice of the operator sequences.
        let ca = rng.gen_range(0..=a.ops.len());
        let cb = rng.gen_range(0..=b.ops.len());
        let mut ops: Vec<usize> = a.ops[..ca].to_vec();
        ops.extend_from_slice(&b.ops[cb..]);
        let mut sweeps: Vec<u32> = a.sweeps[..ca].to_vec();
        sweeps.extend_from_slice(&b.sweeps[cb..]);
        if ops.is_empty() {
            ops.push(a.ops[0]);
            sweeps.push(a.sweeps[0]);
        }
        ops.truncate(self.cfg.max_steps);
        sweeps.truncate(self.cfg.max_steps);
        // Temperatures inherited from a random parent.
        let (t_hi, t_lo) = if rng.gen::<bool>() {
            (a.t_hi, a.t_lo)
        } else {
            (b.t_hi, b.t_lo)
        };
        Genome {
            ops,
            sweeps,
            t_hi,
            t_lo,
        }
    }

    fn mutate(&self, g: &mut Genome, pool: &[&'static str], rng: &mut ChaCha8Rng) {
        // Point mutation of one operator.
        if rng.gen_bool(0.5) && !g.ops.is_empty() {
            let i = rng.gen_range(0..g.ops.len());
            g.ops[i] = rng.gen_range(0..pool.len());
        }
        // Budget mutation.
        if rng.gen_bool(0.5) && !g.sweeps.is_empty() {
            let i = rng.gen_range(0..g.sweeps.len());
            g.sweeps[i] = rng.gen_range(1..=self.cfg.max_sweeps_per_step);
        }
        // Grow.
        if rng.gen_bool(0.3) && g.ops.len() < self.cfg.max_steps {
            g.ops.push(rng.gen_range(0..pool.len()));
            g.sweeps
                .push(rng.gen_range(1..=self.cfg.max_sweeps_per_step));
        }
        // Shrink.
        if rng.gen_bool(0.3) && g.ops.len() > 1 {
            let i = rng.gen_range(0..g.ops.len());
            g.ops.remove(i);
            g.sweeps.remove(i);
        }
        // Temperature perturbation (multiplicative, clamped).
        if rng.gen_bool(0.4) {
            g.t_hi = (g.t_hi * rng.gen_range(0.7..1.4)).clamp(0.1, 12.0);
            g.t_lo = (g.t_lo * rng.gen_range(0.7..1.4)).clamp(0.01, 4.0);
        }
    }

    fn build_plan(&self, pool: &[&'static str], g: &Genome, backend: Backend) -> Plan {
        let steps = g
            .ops
            .iter()
            .zip(&g.sweeps)
            .map(|(&oi, &sw)| PlanStep {
                operator: pool[oi].to_string(),
                // All steps share the Exploit phase so the scheduler executes
                // them in the exact declared sequence (the discovered order).
                phase: Phase::Exploit,
                sweeps: sw.max(1),
                repeat: 1,
            })
            .collect();
        Plan {
            name: "evolved".into(),
            backend,
            num_replicas: self.cfg.num_replicas,
            temperatures: geometric_ladder(self.cfg.num_replicas, g.t_hi, g.t_lo),
            steps,
            seed: self.cfg.run_seed,
            rationale: Default::default(),
        }
    }

    fn evaluate(
        &self,
        ir: &ProblemIR,
        backend: Backend,
        pool: &[&'static str],
        g: &Genome,
        registry: &OperatorRegistry,
    ) -> f64 {
        let plan = self.build_plan(pool, g, backend);
        let init = vec![0u8; ir.n];
        let mut state = boxed_state(ir, backend, self.cfg.num_replicas, &init);
        let mut rt = Runtime::new(RunContext::new(self.cfg.run_seed), &plan);
        let energy = match rt.run(&plan, state.as_mut(), registry, ir) {
            Ok(rec) => rec.best_energy,
            Err(_) => return f64::INFINITY,
        };
        let (work, mem, latency) = self.costs(ir, pool, g, registry);
        self.utility(energy, work, mem, latency)
    }

    /// Deterministic cost model of a genome, WITHOUT running it: total work
    /// (Σ sweeps·work_per_sweep from each operator's own cost model), ensemble
    /// memory (dominant field ledger = n·replicas·8 bytes), and latency (number
    /// of operator invocations).
    fn costs(
        &self,
        ir: &ProblemIR,
        pool: &[&'static str],
        g: &Genome,
        registry: &OperatorRegistry,
    ) -> (f64, f64, f64) {
        let shape = InstanceShape {
            n: ir.n,
            num_pairs: ir.num_pairs(),
            num_replicas: self.cfg.num_replicas,
        };
        let mut work = 0.0;
        for (&oi, &sw) in g.ops.iter().zip(&g.sweeps) {
            if let Some(op) = registry.lookup(pool[oi]) {
                work += sw.max(1) as f64 * op.cost_model(shape).work_per_sweep;
            }
        }
        let mem = (ir.n * self.cfg.num_replicas * 8) as f64;
        (work, mem, g.ops.len() as f64)
    }

    /// The multi-criteria fitness (lower is better): quality, compute cost,
    /// memory, and latency combined by the configured weights.
    pub fn utility(&self, energy: f64, work: f64, mem: f64, latency: f64) -> f64 {
        let w = &self.cfg.utility;
        w.quality * energy + w.cost * work + w.memory * mem + w.latency * latency
    }
}

/// Build a boxed backend state for `backend` (used by evolution and by the AI
/// Scientist's executor). `DenseByte` routes to the dense production-shaped
/// backend (Blueprint v0.2) unless the n² weight matrix would be prohibitive,
/// in which case the reference oracle keeps correctness.
pub fn boxed_state<'a>(
    ir: &'a ProblemIR,
    backend: Backend,
    replicas: usize,
    init: &[u8],
) -> Box<dyn SpinState + 'a> {
    match backend {
        Backend::SparseBitSlice => match SparseBitSlice::new(ir, replicas, init) {
            Ok(s) => Box::new(s),
            Err(_) => Box::new(ReferenceState::new(ir, replicas, init)),
        },
        Backend::DenseByte if ir.n <= super::backends::DENSE_N_LIMIT => {
            Box::new(super::backends::DenseByteState::new(ir, replicas, init))
        }
        Backend::DenseByte => Box::new(ReferenceState::new(ir, replicas, init)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sparse, frustrated, integral instance: a ±ring with a few chords, so a
    /// single short thermal pass is suboptimal and composition (thermal explore
    /// + greedy quench, tuned budgets/temperatures) can improve on it.
    fn ring(n: usize) -> ProblemIR {
        let mut pairs = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, b) = (i.min(j) as u32, i.max(j) as u32);
            let w = if i % 2 == 0 { 1.0 } else { -1.0 };
            pairs.push((a, b, w));
        }
        // A couple of long chords to add frustration.
        pairs.push((0, (n / 2) as u32, 1.0));
        pairs.push((1, (n / 2 + 1) as u32, -1.0));
        ProblemIR::from_pairs(n, 0.0, vec![0.0; n], &pairs)
    }

    fn small_cfg() -> EvolveConfig {
        EvolveConfig {
            population: 10,
            generations: 6,
            max_steps: 4,
            num_replicas: 32,
            max_sweeps_per_step: 20,
            run_seed: 123,
            evolve_seed: 456,
            // Quality-dominant so the "matches or beats energy" test is about
            // solution quality; a separate test covers the cost tradeoff.
            utility: UtilityWeights::quality_only(),
        }
    }

    #[test]
    fn utility_penalizes_cost_and_memory() {
        let cfg = EvolveConfig::default();
        let ev = Evolver::new(cfg);
        // Same energy: the cheaper / smaller / shorter plan must score better.
        let expensive = ev.utility(-100.0, 5.0e6, 2.0e6, 5.0);
        let cheap = ev.utility(-100.0, 1.0e5, 2.0e5, 1.0);
        assert!(
            cheap < expensive,
            "utility must reward efficiency at equal quality"
        );
        // But a real quality gain must still dominate a small cost increase.
        let better_energy = ev.utility(-110.0, 5.0e6, 2.0e6, 5.0);
        assert!(better_energy < cheap, "quality must outrank modest cost");
    }

    #[test]
    fn meta_learning_transfer_never_hurts_and_persists() {
        let ir = ring(24);
        let reg = OperatorRegistry::standard();
        let cfg = small_cfg();

        // Cold evolve, then record the outcome as an experience.
        let cold = Evolver::new(cfg.clone()).evolve(&ir, &reg).unwrap();
        let mut kb = KnowledgeBase::new();
        kb.record(cold.to_experience(&ir));

        // Persist + reload the base (exercises the file format).
        let path = std::env::temp_dir().join(format!("kb_meta_{}.txt", std::process::id()));
        kb.save(&path).unwrap();
        let kb = KnowledgeBase::load(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        // Evolve again WITH the base: the cold winner is a seed, so elitism
        // guarantees the warm result is no worse.
        let warm = Evolver::new(cfg).evolve_with_kb(&ir, &reg, &kb).unwrap();
        assert!(
            warm.best_utility <= cold.best_utility + 1e-9,
            "transfer must not worsen: warm {} vs cold {}",
            warm.best_utility,
            cold.best_utility
        );
        assert_eq!(ir.energy(&warm.best_state), warm.best_energy);
    }

    #[test]
    fn evolution_is_deterministic() {
        let ir = ring(24);
        let reg = OperatorRegistry::standard();
        let r1 = Evolver::new(small_cfg()).evolve(&ir, &reg).unwrap();
        let r2 = Evolver::new(small_cfg()).evolve(&ir, &reg).unwrap();
        assert_eq!(r1.best_energy, r2.best_energy);
        assert_eq!(r1.sequence, r2.sequence);
        assert_eq!(r1.sweeps, r2.sweeps);
        // Best fitness never worsens across generations (elitism).
        for w in r1.generation_best.windows(2) {
            assert!(w[1] <= w[0] + 1e-9);
        }
    }

    #[test]
    fn evolution_matches_or_beats_the_decision_default_and_rescore_is_exact() {
        let ir = ring(24);
        let reg = OperatorRegistry::standard();
        let cfg = small_cfg();

        // Strong hand baseline: the capability-assembled default plan.
        let baseline =
            DecisionEngine::default_plan(&ir, &reg, cfg.num_replicas, 20, cfg.run_seed).unwrap();
        let init = vec![0u8; ir.n];
        let mut st = boxed_state(&ir, baseline.backend, cfg.num_replicas, &init);
        let mut rt = Runtime::new(RunContext::new(cfg.run_seed), &baseline);
        let base_energy = rt
            .run(&baseline, st.as_mut(), &reg, &ir)
            .unwrap()
            .best_energy;

        let res = Evolver::new(cfg).evolve(&ir, &reg).unwrap();

        // The search must not do worse than the hand-written default.
        assert!(
            res.best_energy <= base_energy + 1e-9,
            "evolved {} worse than default {}",
            res.best_energy,
            base_energy
        );
        // Pool was capability-selected from the whole standard library.
        assert!(res.pool.len() >= 13);
        assert!(!res.sequence.is_empty());
        // Reported energy is the canonical re-score of the reported state.
        assert_eq!(ir.energy(&res.best_state), res.best_energy);
    }
}
