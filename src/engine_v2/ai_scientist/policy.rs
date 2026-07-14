//! Neural operator policy (roadmap Levels 3–7, in miniature and honest).
//!
//! A small autoregressive network over OPERATOR TOKENS: given the instance's
//! structural features and the operators chosen so far, it predicts the next
//! operator (or STOP). Trained two ways:
//!
//!  1. SUPERVISED — on the append-only experiment database: sequences that
//!     beat their instance's median improvement are the positive corpus
//!     (`train_supervised`), so the policy distills millions of recorded
//!     experiments into weights.
//!  2. REINFORCE — against the real Runtime as the environment
//!     (`train_reinforce`): sample schedules, execute them, reward = relative
//!     improvement minus a length cost, baseline-subtracted policy gradient.
//!
//! Architecture (all hand-rolled, no crates, fully deterministic given seeds):
//! token embeddings + a feature projection + exponentially recency-weighted
//! context (a cheap stand-in for attention), one tanh hidden state, softmax
//! head. This is deliberately NOT called a transformer or GNN: it is a small
//! neural sequence policy; those are the natural upgrades once this rung
//! proves value. It never solves the problem itself — it only proposes which
//! algorithm to run; the Runtime remains the sole judge.

use super::super::evolution::Schedule;
use super::super::ir::ProblemIR;
use super::super::registry::OperatorRegistry;
use super::db::ExperimentDb;
use super::executor::{BatchExecutor, ExperimentTask};
use super::lab::{Ideator, ResearchBrief};
use super::predictor::InstanceSignature;
use super::scientist::{Hypothesis, HypothesisStatus};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, HashSet};

const DIM: usize = 16;
const N_FEATS: usize = 5;
/// Context recency decay: the previous operator matters most, order is felt.
const GAMMA: f64 = 0.7;
const MAX_LEN: usize = 4;

/// The policy network. `vocab[t]` is operator token t; token `vocab.len()`
/// is STOP (output only), token `vocab.len()+1` is START (input only).
#[derive(Debug, Clone)]
pub struct OperatorPolicy {
    pub vocab: Vec<String>,
    /// Token embeddings: (vocab + START) × DIM.
    emb: Vec<f64>,
    /// Feature projection: N_FEATS × DIM.
    w_feat: Vec<f64>,
    bias: Vec<f64>,
    /// Output head: (vocab + STOP) × DIM.
    head: Vec<f64>,
    head_bias: Vec<f64>,
}

fn features(sig: &InstanceSignature) -> [f64; N_FEATS] {
    [
        ((sig.n as f64) + 1.0).ln() / 10.0,
        sig.density,
        sig.clustering,
        sig.mean_degree / 10.0,
        sig.degree_cv,
    ]
}

fn softmax(logits: &mut [f64]) {
    let m = logits.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let mut z = 0.0;
    for l in logits.iter_mut() {
        *l = (*l - m).exp();
        z += *l;
    }
    for l in logits.iter_mut() {
        *l /= z;
    }
}

/// Forward-pass scratch reused across steps.
struct Forward {
    h: Vec<f64>,
    probs: Vec<f64>,
    ctx_weights: Vec<f64>, // γ^(t-1-i) per context token
}

impl OperatorPolicy {
    pub fn stop_token(&self) -> usize {
        self.vocab.len()
    }
    fn start_token(&self) -> usize {
        self.vocab.len() + 1 - 1 // embedding row index for START = vocab.len()
    }
    fn n_out(&self) -> usize {
        self.vocab.len() + 1 // operators + STOP
    }

    /// Fresh policy over `vocab` (sorted for determinism), small seeded init.
    pub fn new(mut vocab: Vec<String>, seed: u64) -> Self {
        vocab.sort();
        vocab.dedup();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut init =
            |len: usize| -> Vec<f64> { (0..len).map(|_| rng.gen_range(-0.1..0.1)).collect() };
        let n_emb = vocab.len() + 1; // + START
        let n_out = vocab.len() + 1; // + STOP
        Self {
            emb: init(n_emb * DIM),
            w_feat: init(N_FEATS * DIM),
            bias: init(DIM),
            head: init(n_out * DIM),
            head_bias: init(n_out),
            vocab,
        }
    }

    /// Token id of an operator name, if in vocabulary.
    pub fn token(&self, op: &str) -> Option<usize> {
        self.vocab.binary_search_by(|v| v.as_str().cmp(op)).ok()
    }

    /// One forward step: probability distribution over the next token given
    /// the features and the tokens chosen so far.
    fn forward(&self, feats: &[f64; N_FEATS], context: &[usize]) -> Forward {
        let mut h_raw = self.bias.clone();
        for (f, row) in feats.iter().zip(self.w_feat.chunks(DIM)) {
            for (h, w) in h_raw.iter_mut().zip(row) {
                *h += f * w;
            }
        }
        // Recency-weighted context sum; START stands in for the empty prefix.
        let mut ctx_weights = Vec::with_capacity(context.len().max(1));
        if context.is_empty() {
            let row = &self.emb[self.start_token() * DIM..(self.start_token() + 1) * DIM];
            for (h, e) in h_raw.iter_mut().zip(row) {
                *h += e;
            }
        } else {
            for (i, &tok) in context.iter().enumerate() {
                let w = GAMMA.powi((context.len() - 1 - i) as i32);
                ctx_weights.push(w);
                let row = &self.emb[tok * DIM..(tok + 1) * DIM];
                for (h, e) in h_raw.iter_mut().zip(row) {
                    *h += w * e;
                }
            }
        }
        let h: Vec<f64> = h_raw.iter().map(|v| v.tanh()).collect();
        let mut probs = vec![0.0; self.n_out()];
        for (o, row) in probs.iter_mut().zip(self.head.chunks(DIM)) {
            *o = row.iter().zip(&h).map(|(w, x)| w * x).sum::<f64>();
        }
        for (o, b) in probs.iter_mut().zip(&self.head_bias) {
            *o += b;
        }
        softmax(&mut probs);
        Forward {
            h,
            probs,
            ctx_weights,
        }
    }

    /// Backprop one step given d(logits) = probs − onehot(target), scaled by
    /// `scale` (supervised: example weight; REINFORCE: −advantage). SGD update.
    fn backward(
        &mut self,
        feats: &[f64; N_FEATS],
        context: &[usize],
        fwd: &Forward,
        target: usize,
        scale: f64,
        lr: f64,
    ) {
        let mut dlogit = fwd.probs.clone();
        dlogit[target] -= 1.0;
        for d in dlogit.iter_mut() {
            *d *= scale;
        }
        // dh = headᵀ · dlogit ; update head simultaneously.
        let mut dh = [0.0; DIM];
        for (o, (row, &dl)) in self.head.chunks_mut(DIM).zip(&dlogit).enumerate() {
            for ((d, w), &hk) in dh.iter_mut().zip(row.iter_mut()).zip(&fwd.h) {
                *d += *w * dl;
                *w -= lr * dl * hk;
            }
            self.head_bias[o] -= lr * dl;
        }
        // Through tanh.
        let dhraw: Vec<f64> = dh
            .iter()
            .zip(&fwd.h)
            .map(|(d, h)| d * (1.0 - h * h))
            .collect();
        for (b, &d) in self.bias.iter_mut().zip(&dhraw) {
            *b -= lr * d;
        }
        for (fi, f) in feats.iter().enumerate() {
            let row = &mut self.w_feat[fi * DIM..(fi + 1) * DIM];
            for k in 0..DIM {
                row[k] -= lr * f * dhraw[k];
            }
        }
        if context.is_empty() {
            let s = self.start_token();
            let row = &mut self.emb[s * DIM..(s + 1) * DIM];
            for k in 0..DIM {
                row[k] -= lr * dhraw[k];
            }
        } else {
            for (i, &tok) in context.iter().enumerate() {
                let w = fwd.ctx_weights[i];
                let row = &mut self.emb[tok * DIM..(tok + 1) * DIM];
                for k in 0..DIM {
                    row[k] -= lr * w * dhraw[k];
                }
            }
        }
    }

    /// Mean cross-entropy of the corpus (diagnostic).
    pub fn loss(&self, corpus: &[(InstanceSignature, Vec<usize>)]) -> f64 {
        let (mut total, mut count) = (0.0, 0usize);
        for (sig, seq) in corpus {
            let feats = features(sig);
            let mut targets: Vec<usize> = seq.clone();
            targets.push(self.stop_token());
            for t in 0..targets.len() {
                let fwd = self.forward(&feats, &seq[..t]);
                total -= fwd.probs[targets[t]].max(1e-12).ln();
                count += 1;
            }
        }
        if count == 0 {
            0.0
        } else {
            total / count as f64
        }
    }

    /// SUPERVISED: distill the experiment database. Records whose relative
    /// improvement beats their instance's median form the corpus; each
    /// sequence teaches "next operator given features + prefix" plus STOP.
    /// Returns (initial loss, final loss) so callers can verify learning.
    pub fn train_supervised(&mut self, db: &ExperimentDb, epochs: usize, lr: f64) -> (f64, f64) {
        let corpus = Self::corpus_from_db(self, db);
        if corpus.is_empty() {
            return (0.0, 0.0);
        }
        let before = self.loss(&corpus);
        for _ in 0..epochs {
            for (sig, seq) in &corpus {
                let feats = features(sig);
                let mut targets: Vec<usize> = seq.clone();
                targets.push(self.stop_token());
                for t in 0..targets.len() {
                    let fwd = self.forward(&feats, &seq[..t]);
                    self.backward(&feats, &seq[..t], &fwd, targets[t], 1.0, lr);
                }
            }
        }
        (before, self.loss(&corpus))
    }

    /// Build the positive training corpus: per instance, keep records at or
    /// above the median relative improvement (ties keep the better half).
    pub fn corpus_from_db(
        policy: &OperatorPolicy,
        db: &ExperimentDb,
    ) -> Vec<(InstanceSignature, Vec<usize>)> {
        let mut per_inst: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (i, r) in db.all().iter().enumerate() {
            if !r.sequence.is_empty() {
                per_inst.entry(r.instance_id.as_str()).or_default().push(i);
            }
        }
        let mut corpus = Vec::new();
        for idx in per_inst.values() {
            let mut imps: Vec<f64> = idx.iter().map(|&i| db.all()[i].rel_improvement()).collect();
            imps.sort_by(|a, b| a.total_cmp(b));
            let median = imps[imps.len() / 2];
            for &i in idx {
                let r = &db.all()[i];
                if r.rel_improvement() < median {
                    continue;
                }
                let toks: Option<Vec<usize>> = r
                    .sequence
                    .iter()
                    .take(MAX_LEN)
                    .map(|op| policy.token(op))
                    .collect();
                if let Some(toks) = toks {
                    corpus.push((
                        InstanceSignature {
                            n: r.n,
                            density: r.density,
                            clustering: r.clustering,
                            mean_degree: r.mean_degree,
                            degree_cv: r.degree_cv,
                        },
                        toks,
                    ));
                }
            }
        }
        corpus
    }

    /// The policy's first-operator preference distribution for an instance,
    /// as (operator, probability) pairs renormalized over operators (STOP
    /// excluded). This is the Policy's contribution to the shared Meta-Learning
    /// Layer: which physics it has learned to reach for on this regime.
    pub fn first_op_probs(&self, sig: &InstanceSignature) -> Vec<(String, f64)> {
        let feats = features(sig);
        let fwd = self.forward(&feats, &[]);
        let mut out: Vec<(String, f64)> = self
            .vocab
            .iter()
            .enumerate()
            .map(|(t, name)| (name.clone(), fwd.probs[t]))
            .collect();
        let z: f64 = out.iter().map(|(_, p)| *p).sum();
        if z > 0.0 {
            for (_, p) in &mut out {
                *p /= z;
            }
        }
        out
    }

    /// Greedy decode: the policy's single best sequence for this instance.
    pub fn generate_greedy(&self, sig: &InstanceSignature) -> Vec<String> {
        let feats = features(sig);
        let mut seq = Vec::new();
        while seq.len() < MAX_LEN {
            let fwd = self.forward(&feats, &seq);
            let (best, _) =
                fwd.probs
                    .iter()
                    .enumerate()
                    .fold((0usize, f64::NEG_INFINITY), |acc, (i, &p)| {
                        if p > acc.1 {
                            (i, p)
                        } else {
                            acc
                        }
                    });
            if best == self.stop_token() && !seq.is_empty() {
                break;
            }
            if best == self.stop_token() {
                // Empty schedule is useless; take the best operator instead.
                let (op, _) = fwd.probs[..self.vocab.len()].iter().enumerate().fold(
                    (0usize, f64::NEG_INFINITY),
                    |acc, (i, &p)| if p > acc.1 { (i, p) } else { acc },
                );
                seq.push(op);
                continue;
            }
            seq.push(best);
        }
        seq.into_iter().map(|t| self.vocab[t].clone()).collect()
    }

    /// Sample a sequence (temperature 1, seeded RNG); returns token ids.
    pub fn sample(&self, sig: &InstanceSignature, rng: &mut ChaCha8Rng) -> Vec<usize> {
        let feats = features(sig);
        let mut seq = Vec::new();
        while seq.len() < MAX_LEN {
            let fwd = self.forward(&feats, &seq);
            let u: f64 = rng.gen();
            let mut acc = 0.0;
            let mut choice = self.stop_token();
            for (i, &p) in fwd.probs.iter().enumerate() {
                acc += p;
                if u < acc {
                    choice = i;
                    break;
                }
            }
            if choice == self.stop_token() {
                if seq.is_empty() {
                    continue; // an empty schedule teaches nothing — resample
                }
                break;
            }
            seq.push(choice);
        }
        seq
    }

    /// REINFORCE: `rollouts` sampled schedules are EXECUTED by the caller
    /// (the Runtime is the environment); this applies the baseline-subtracted
    /// policy gradient for one batch. `episodes[k] = (token seq, reward)`.
    pub fn reinforce_update(
        &mut self,
        sig: &InstanceSignature,
        episodes: &[(Vec<usize>, f64)],
        lr: f64,
    ) {
        if episodes.len() < 2 {
            return;
        }
        let feats = features(sig);
        let baseline = episodes.iter().map(|e| e.1).sum::<f64>() / episodes.len() as f64;
        for (seq, reward) in episodes {
            let advantage = reward - baseline;
            if advantage == 0.0 {
                continue;
            }
            let mut targets: Vec<usize> = seq.clone();
            targets.push(self.stop_token());
            for t in 0..targets.len() {
                let fwd = self.forward(&feats, &seq[..t]);
                // backward performs θ ← θ − lr·scale·∇CE = θ + lr·scale·∇log π,
                // so scale = +advantage IS gradient ascent on advantage·log π.
                self.backward(&feats, &seq[..t], &fwd, targets[t], advantage, lr);
            }
        }
    }
}

/// Lower a token sequence to a runnable `Schedule` with uniform sweeps and a
/// standard ladder — the policy chooses WHICH physics; budgets stay explicit.
pub fn tokens_to_schedule(policy: &OperatorPolicy, toks: &[usize], sweeps: u32) -> Schedule {
    Schedule {
        ops: toks.iter().map(|&t| policy.vocab[t].clone()).collect(),
        sweeps: vec![sweeps; toks.len()],
        temp_hi: 4.0,
        temp_lo: 0.1,
    }
}

/// Mean best-energy of `schedule` over `seeds`, executed on the real Runtime
/// (via `executor`) — used both as the REINFORCE baseline anchor and to
/// measure a rolled-out episode's reward.
fn mean_energy(
    executor: &dyn BatchExecutor,
    ir: &ProblemIR,
    registry: &OperatorRegistry,
    schedule: &Schedule,
    num_replicas: usize,
    seeds: &[u64],
) -> f64 {
    let tasks: Vec<ExperimentTask> = seeds
        .iter()
        .map(|&seed| ExperimentTask {
            schedule: schedule.clone(),
            num_replicas,
            seed,
        })
        .collect();
    let out = executor.run_batch(ir, registry, &tasks);
    out.iter().map(|o| o.score).sum::<f64>() / out.len().max(1) as f64
}

/// History of one REINFORCE batch: mean reward and mean episode length.
#[derive(Debug, Clone, Copy)]
pub struct ReinforceStep {
    pub mean_reward: f64,
    pub mean_len: f64,
}

/// REINFORCE fine-tuning against the REAL Runtime as the environment (roadmap
/// Level 7): for `iterations` batches, sample `rollouts` schedules from the
/// policy, execute each with `seeds_per` seeds on `executor` (the only thing
/// that ever runs physics — the policy never computes an energy itself),
/// score reward = relative improvement over a fixed baseline schedule's mean
/// energy, minus `length_cost` per operator (so free-riding on long, aimless
/// sequences is not rewarded), and apply the baseline-subtracted policy
/// gradient. Returns the per-batch trace so callers can verify the reward
/// actually climbs — no claim of learning without measuring it.
#[allow(clippy::too_many_arguments)]
pub fn train_reinforce(
    policy: &mut OperatorPolicy,
    ir: &ProblemIR,
    registry: &OperatorRegistry,
    executor: &dyn BatchExecutor,
    sig: &InstanceSignature,
    num_replicas: usize,
    sweeps: u32,
    iterations: usize,
    rollouts: usize,
    seeds_per: usize,
    length_cost: f64,
    lr: f64,
    seed: u64,
) -> Vec<ReinforceStep> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // Fixed reference point: a plain single-operator pass (whatever token 0
    // is) run once, so every episode's reward is comparable across the run.
    let baseline_schedule = Schedule {
        ops: vec![policy.vocab[0].clone()],
        sweeps: vec![sweeps],
        temp_hi: 4.0,
        temp_lo: 0.1,
    };
    let baseline_seeds: Vec<u64> = (0..seeds_per as u64).map(|s| seed ^ (s + 1)).collect();
    let baseline_energy = mean_energy(
        executor,
        ir,
        registry,
        &baseline_schedule,
        num_replicas,
        &baseline_seeds,
    );

    let mut trace = Vec::with_capacity(iterations);
    for it in 0..iterations {
        let mut episodes: Vec<(Vec<usize>, f64)> = Vec::with_capacity(rollouts);
        for r in 0..rollouts {
            let toks = policy.sample(sig, &mut rng);
            let sched = tokens_to_schedule(policy, &toks, sweeps);
            let ep_seeds: Vec<u64> = (0..seeds_per as u64)
                .map(|s| seed ^ ((it * rollouts + r) as u64 * 7919 + s + 1000))
                .collect();
            let energy = mean_energy(executor, ir, registry, &sched, num_replicas, &ep_seeds);
            let rel = (baseline_energy - energy) / baseline_energy.abs().max(1.0);
            let reward = rel - length_cost * toks.len() as f64;
            episodes.push((toks, reward));
        }
        let mean_reward = episodes.iter().map(|(_, r)| r).sum::<f64>() / rollouts as f64;
        let mean_len =
            episodes.iter().map(|(t, _)| t.len()).sum::<usize>() as f64 / rollouts as f64;
        trace.push(ReinforceStep {
            mean_reward,
            mean_len,
        });
        policy.reinforce_update(sig, &episodes, lr);
    }
    trace
}

/// Roadmap Level 4: the trained network as an `Ideator` — it PROPOSES operator
/// sequences (genomes) for the Evolution Engine and lab to expand and test,
/// exactly like the heuristic generator or the LLM. It never computes an
/// energy itself; the lab's Runtime/Statistician still judge every proposal.
pub struct PolicyIdeator {
    policy: OperatorPolicy,
    next_id: u64,
}

impl PolicyIdeator {
    /// `next_id` starts at 300_000 — clearly distinct from heuristic (0+) and
    /// LLM (100_000+) hypothesis ids, so provenance is visible in the DB.
    pub fn new(policy: OperatorPolicy) -> Self {
        Self {
            policy,
            next_id: 300_000,
        }
    }

    pub fn policy(&self) -> &OperatorPolicy {
        &self.policy
    }
}

impl Ideator for PolicyIdeator {
    fn propose(
        &mut self,
        brief: &ResearchBrief,
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        let pool: HashSet<&str> = brief.pool.iter().copied().collect();
        let sig = InstanceSignature {
            n: brief.stats.n,
            density: brief.stats.density,
            clustering: brief.stats.clustering,
            mean_degree: brief.stats.mean_degree,
            degree_cv: brief.stats.degree_cv,
        };
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let toks = self.policy.sample(&sig, rng);
            let mut ops: Vec<String> = toks
                .iter()
                .map(|&t| self.policy.vocab[t].clone())
                .filter(|op| pool.contains(op.as_str()))
                .collect();
            // A proposal naming only operators outside this instance's
            // capability pool teaches nothing here — fall back to the pool's
            // first (deterministic) operator so every call still proposes.
            if ops.is_empty() {
                if let Some(first) = brief.pool.first() {
                    ops.push(first.to_string());
                } else {
                    continue;
                }
            }
            let id = self.next_id;
            self.next_id += 1;
            out.push(Hypothesis {
                id,
                operators: ops,
                rationale: "learned neural policy proposal".into(),
                reasoning: format!(
                    "sampled from the operator policy trained on the experiment history \
(features: n={}, density={:.4}, clustering={:.3})",
                    sig.n, sig.density, sig.clustering
                ),
                predicted_improvement: 0.0,
                status: HypothesisStatus::Proposed,
                observed_improvement: 0.0,
                confidence: 0.0,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn sig(density: f64) -> InstanceSignature {
        InstanceSignature {
            n: 800,
            density,
            clustering: 0.3,
            mean_degree: 4.0,
            degree_cv: 0.1,
        }
    }

    fn rec(inst: &str, seq: &[&str], score: f64, density: f64) -> ExperimentRecord {
        ExperimentRecord {
            instance_id: inst.into(),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
            score,
            baseline: 0.0,
            density,
            n: 800,
            clustering: 0.3,
            mean_degree: 4.0,
            ..Default::default()
        }
    }

    fn vocab() -> Vec<String> {
        vec![
            "gibbs_color_sweep".into(),
            "greedy_descent".into(),
            "metropolis_sweep".into(),
            "random_flip_sweep".into(),
        ]
    }

    #[test]
    fn supervised_training_reduces_loss_and_learns_the_winning_sequence() {
        let mut db = ExperimentDb::new();
        // On sparse instances metropolis→greedy wins; random_flip loses.
        for i in 0..30 {
            db.record(rec(
                "G_sparse",
                &["metropolis_sweep", "greedy_descent"],
                -0.5 - 0.001 * i as f64,
                0.005,
            ));
            db.record(rec("G_sparse", &["random_flip_sweep"], 0.05, 0.005));
        }
        let mut p = OperatorPolicy::new(vocab(), 42);
        let (before, after) = p.train_supervised(&db, 30, 0.05);
        assert!(
            after < before * 0.6,
            "loss must drop substantially: {before} -> {after}"
        );
        let gen = p.generate_greedy(&sig(0.005));
        assert_eq!(
            gen,
            vec!["metropolis_sweep".to_string(), "greedy_descent".into()],
            "the policy must reproduce the winning program"
        );
    }

    #[test]
    fn policy_conditions_on_instance_features() {
        let mut db = ExperimentDb::new();
        // Sparse ⇒ metropolis wins; dense ⇒ gibbs wins. Median split keeps
        // only the winner per instance.
        for i in 0..40 {
            db.record(rec(
                "G_sparse",
                &["metropolis_sweep"],
                -0.5,
                0.005 + 1e-6 * i as f64,
            ));
            db.record(rec("G_sparse", &["gibbs_color_sweep"], -0.05, 0.005));
            db.record(rec("K_dense", &["gibbs_color_sweep"], -0.5, 0.4));
            db.record(rec("K_dense", &["metropolis_sweep"], -0.05, 0.4));
        }
        let mut p = OperatorPolicy::new(vocab(), 7);
        p.train_supervised(&db, 40, 0.05);
        assert_eq!(p.generate_greedy(&sig(0.005))[0], "metropolis_sweep");
        assert_eq!(p.generate_greedy(&sig(0.4))[0], "gibbs_color_sweep");
    }

    #[test]
    fn training_and_generation_are_deterministic() {
        let mut db = ExperimentDb::new();
        for _ in 0..10 {
            db.record(rec("g", &["greedy_descent"], -0.3, 0.01));
        }
        let run = || {
            let mut p = OperatorPolicy::new(vocab(), 9);
            p.train_supervised(&db, 10, 0.05);
            (
                p.generate_greedy(&sig(0.01)),
                p.sample(&sig(0.01), &mut ChaCha8Rng::seed_from_u64(3)),
            )
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn reinforce_against_real_runtime_climbs_reward_and_prefers_the_improving_operator() {
        use super::super::executor::RuntimeExecutor;
        use crate::engine_v2::ir::ProblemIR;
        use crate::engine_v2::registry::OperatorRegistry;
        // A ring with alternating couplings: pure noise (random_flip_sweep)
        // cannot beat a monotone quench (greedy_descent) here.
        let n = 16u32;
        let mut pairs: Vec<(u32, u32, f64)> = (0..n - 1)
            .map(|i| (i, i + 1, if i % 2 == 0 { -1.0 } else { 1.0 }))
            .collect();
        pairs.push((0, n - 1, -1.0));
        let ir = ProblemIR::from_pairs(n as usize, 0.0, vec![0.5; n as usize], &pairs);
        let reg = OperatorRegistry::standard();
        let exec = RuntimeExecutor::new(2);
        let sig = InstanceSignature {
            n: n as usize,
            density: 0.15,
            clustering: 0.0,
            mean_degree: 2.0,
            degree_cv: 0.0,
        };
        let pool = vec![
            "greedy_descent".to_string(),
            "random_flip_sweep".to_string(),
        ];
        let mut policy = OperatorPolicy::new(pool, 4);

        let trace = train_reinforce(
            &mut policy,
            &ir,
            &reg,
            &exec,
            &sig,
            /* num_replicas */ 8,
            /* sweeps */ 6,
            /* iterations */ 12,
            /* rollouts */ 8,
            /* seeds_per */ 3,
            /* length_cost */ 0.01,
            /* lr */ 0.1,
            /* seed */ 21,
        );
        assert_eq!(trace.len(), 12);
        let early: f64 = trace[..3].iter().map(|s| s.mean_reward).sum::<f64>() / 3.0;
        let late: f64 = trace[trace.len() - 3..]
            .iter()
            .map(|s| s.mean_reward)
            .sum::<f64>()
            / 3.0;
        assert!(
            late > early,
            "REINFORCE against the real Runtime must raise mean reward: {early:.4} -> {late:.4}"
        );
        let gen = policy.generate_greedy(&sig);
        assert!(
            gen.contains(&"greedy_descent".to_string()),
            "policy should have learned to prefer the improving operator: {gen:?}"
        );
    }

    #[test]
    fn policy_ideator_runs_end_to_end_in_the_lab() {
        use super::super::executor::RuntimeExecutor;
        use super::super::graph::KnowledgeGraph;
        use super::super::lab::{LabConfig, ScientificLab};
        use crate::engine_v2::evolution::Evolver;
        use crate::engine_v2::ir::ProblemIR;
        use crate::engine_v2::knowledge::KnowledgeBase;
        use crate::engine_v2::registry::OperatorRegistry;

        let ir = ProblemIR::from_pairs(
            6,
            0.0,
            vec![0.0; 6],
            &[
                (0, 1, 1.0),
                (1, 2, -1.0),
                (2, 3, 1.0),
                (3, 4, -1.0),
                (4, 5, 1.0),
            ],
        );
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);

        // A minimally-trained policy is enough to exercise the wiring.
        let mut db = ExperimentDb::new();
        for _ in 0..30 {
            db.record(rec("t", &["greedy_descent"], -0.4, 0.3));
        }
        let vocab = vec!["greedy_descent".to_string(), "metropolis_sweep".to_string()];
        let mut policy = OperatorPolicy::new(vocab, 2);
        policy.train_supervised(&db, 20, 0.05);
        let ideator = Box::new(PolicyIdeator::new(policy));

        let cfg = LabConfig {
            rounds: 2,
            hypotheses_per_round: 6,
            batch_size: 16,
            seeds_per_hypothesis: 2,
            num_replicas: 8,
            base_seed: 5,
            ..Default::default()
        };
        let mut edb = ExperimentDb::new();
        let mut kb = KnowledgeBase::new();
        let mut lab = ScientificLab::with_ideator(5, KnowledgeGraph::new(), ideator);
        let report = lab.run(&ir, &reg, &evolver, &exec, &mut edb, &mut kb, &cfg);
        assert!(report.experiments_run > 0);
        assert!(edb
            .all()
            .iter()
            .any(|r| r.hypothesis_id.is_some_and(|h| h >= 300_000)));
    }

    #[test]
    fn reinforce_shifts_probability_toward_rewarded_sequences() {
        let p0 = OperatorPolicy::new(vocab(), 11);
        let mut p = p0.clone();
        let s = sig(0.01);
        let good = vec![p.token("greedy_descent").unwrap()];
        let bad = vec![p.token("random_flip_sweep").unwrap()];
        // Simulated environment: greedy gets reward 1, random_flip 0.
        for _ in 0..60 {
            let episodes = vec![(good.clone(), 1.0), (bad.clone(), 0.0)];
            p.reinforce_update(&s, &episodes, 0.05);
        }
        let feats = super::features(&s);
        let before = p0.forward(&feats, &[]).probs[good[0]];
        let after = p.forward(&feats, &[]).probs[good[0]];
        assert!(
            after > before + 0.2,
            "P(greedy first) must grow: {before:.3} -> {after:.3}"
        );
        let gen = p.generate_greedy(&s);
        assert_eq!(gen[0], "greedy_descent");
    }
}
