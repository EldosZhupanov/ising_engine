# The Concept Evolution Constitution

*The scientific methodology governing how the Ising Engine's autonomous scientist
creates, validates, compares, merges, splits, retires, archives, and rediscovers
scientific concepts over years of unattended operation. This document is design
law, not a sketch. It is deliberately conservative: in a system that runs for
years, a false concept admitted into the shared representation is far more costly
than a true concept missed, because it silently corrupts every downstream model
until it is caught. When in doubt, do not admit.*

Subordinate to the Constitution, SOUL, and the ADRs. Nothing here weakens
determinism (ADR-0004), append-only evidence (§12), or honesty-over-optimism.

---

## 0. First principles (the non-negotiables)

1. **A concept is a named, interpretable, deterministic measurement of structure
   or dynamics.** If it cannot be written as a closed-form quantity a human could
   read and understand, it is not a concept — it is a latent dimension, and it is
   rejected. The project's entire claim is to do *science*; a representation you
   cannot explain is not science.
2. **Fitness is measured against the real downstream use, never a proxy.** A
   concept earns its place only if it improves the actual models and knowledge the
   scientist uses — not an internal stand-in metric.
3. **Evidence is append-only; the active representation is not.** These are two
   different stores with two different contracts. Confusing them is the central
   architectural error to avoid (§8).
4. **Admission is a statistical decision under multiple testing, not a threshold
   crossing.** Over years, thousands of candidates will be tested. Without false-
   discovery control, the representation *will* fill with noise. This is the single
   most important requirement in the document.
5. **Every act — proposal, admission, merge, split, retirement — is recorded as
   evidence with provenance, and is reversible in light of new data.** Science
   changes its mind; the record of why is permanent.

---

## 1. Concept Proposal — where candidates come from

The gravest limitation of the current prototype: it proposes only *algebraic
interactions of the five existing scalars*. Such a concept carries **no new
information** — it only re-expresses, nonlinearly, what the base features already
contain. The famous ruggedness result could never be discovered this way, because
ruggedness is not a function of density; it is a *new measurement of the landscape*.
Therefore the concept domain must be broadened, and organised into cost/power tiers:

- **Tier 0 — Algebraic** (interactions, ratios, powers of existing concepts).
  Cheap. Value strictly bounded by the information already present; useful only
  because linear models cannot represent products. A convenience, not discovery.
- **Tier 1 — Structural (from the problem itself).** Graph/coupling statistics the
  base scalars miss: degree-distribution moments, weight-distribution shape,
  frustration index, modularity/community structure, motif counts, coupling
  sign-balance. These are genuinely new measurements.
- **Tier 2 — Spectral.** Eigenstructure of the coupling matrix / graph Laplacian:
  spectral gap, spectral radius, participation ratios. A principled, well-founded
  family with strong ties to hardness.
- **Tier 3 — Landscape / dynamical (from sampling via the Runtime).** Ruggedness,
  autocorrelation length, basin/minima entropy, funnel depth, fitness-distance
  correlation, escape difficulty. The **most powerful** (they measure actual
  optimisation difficulty) and the **most expensive** (they cost Runtime budget).
  Determinism makes them exactly reproducible — a genuine, rare advantage.
- **Tier 4 — Symbolic composition.** Interpretable closed-form expressions built by
  symbolic regression / evolutionary search over the Tier 0–3 primitives. This is
  the engine of genuine, nameable discovery.

**Rejected as first-class concepts:** learned latent embeddings (autoencoder/GNN
codes), sparse-coding dictionaries, any black-box vector. They violate first
principle #1. A black-box embedding may be used *only diagnostically* — to detect
that structure the current vocabulary misses exists, thereby *motivating* a search
for an interpretable concept that captures it. The embedding is a smoke detector,
never a resident of the house.

**Proposal is memory-guided (§9):** before synthesising a candidate, the proposer
consults the archive so it does not re-derive a concept already refuted, and can
resurrect one retired for distributional reasons.

---

## 2. Validation — the statistical contract

The enemy is small N (few instances/families) and multiple testing (many candidates
per cycle). A point estimate of "RMSE improved" is worthless. Admission requires
**all** of the following to hold; failing any one is rejection:

- **Cross-family generalisation, not just cross-instance.** The hold-out unit is the
  *problem family* (leave-one-family-out), because the mission is transfer. A concept
  that helps within a family but not across families is a within-family curiosity,
  and the prototype's own history proves the trap: instance-level gains that
  evaporated at family level.
- **Repeated / nested cross-validation.** Nested, so the admission threshold is never
  tuned on the same data used to estimate the gain (selection bias). Repeated across
  seeds and folds to produce a *distribution* of the gain, not a number.
- **Bootstrap confidence interval on the gain; require the lower bound > 0.** Not the
  point estimate — the lower CI bound. A concept whose 95% interval includes zero
  does not enter.
- **Effect size gate, separate from significance.** A statistically significant 0.1%
  gain is scientifically nothing. Require a standardised effect size above a
  meaningful floor. Significance answers "is it real?"; effect size answers "does it
  matter?"; both are required.
- **Multiple-testing / false-discovery control.** Every cycle tests many candidates;
  p-values must be corrected (Benjamini–Hochberg FDR) across the batch. Without this,
  years of autonomous testing guarantee a representation of false positives. This is
  the load-bearing requirement of the whole system.
- **Stability.** The gain's variance across seeds/folds must be low; an unstable
  gain is a fluke. Stability is a first-class objective (§3), not an afterthought.
- **Bayesian complement (optional but recommended).** Posterior odds / BIC as a
  second, independent line of evidence and the principled way to compare non-nested
  concepts (§6, §11). A concept that passes frequentist FDR *and* has favourable
  posterior odds is trustworthy; disagreement between the two is itself informative
  and should trigger more evidence-gathering, not admission.

---

## 3. Fitness — multi-objective, never scalarised

A concept is never accepted because one number improved. Fitness is a **vector**,
and the objectives genuinely conflict (a complex concept may predict better yet fail
simplicity and interpretability). Scalarising into a weighted sum is forbidden: the
weights are arbitrary, hide the trade-off, and are gameable. The objectives:

1. **Predictive improvement** — marginal out-of-sample skill on the *real* Predictor.
2. **Generalisation / cross-family transfer** — the same, measured across held-out
   families. Weighted as primary, because it *is* the mission.
3. **Simplicity** — the description length of the concept's own definition (MDL).
4. **Stability** — inverse variance of the gain across seeds/folds/resamples.
5. **Interpretability** — a graded gate: closed-form and nameable (pass) → opaque
   (reject). Not traded off; a hard constraint.
6. **Compression power** — does admitting the concept let the Meta-Learner state
   *fewer, higher-confidence* rules, and lower the entropy of the knowledge graph?
   A concept that compresses knowledge is doing real explanatory work.

Decision rule: maintain a **Pareto frontier** over these objectives; a candidate is
eligible only if non-dominated. Among the frontier, break ties toward **parsimony**
(shortest description length) — an explicit Occam prior. This is how the system knows
one concept is "better": Pareto dominance, with parsimony as the tiebreak, not a
hidden weighted score.

---

## 4. Competition — how concepts are compared

- **Dominance & Pareto frontier.** Concept A dominates B if A is ≥ B on every
  objective and > on at least one. The active vocabulary is (a projection of) the
  Pareto-non-dominated set. Dominated concepts are retirement candidates.
- **Redundancy — correlation and mutual information as cheap pre-filters.**
  Pairwise correlation and MI of concept values across the corpus flag *suspects*.
  But correlation is necessary, not sufficient: two concepts can be uncorrelated yet
  jointly redundant, or correlated yet complementary in the presence of a third.
- **Redundancy — ablation as the causal verdict.** The decisive test reuses the
  Theory Engine's existing ablation machinery, lifted from operators to features:
  remove the concept from the vocabulary and refit; if the real Predictor's
  cross-family skill is unchanged (within noise), the concept is redundant regardless
  of its correlations. Ablation is the gold standard; correlation/MI are the screen.
- **Feature importance** — permutation importance on the real Predictor, to rank
  contribution and prioritise which concepts to ablation-test under a budget.

---

## 5. Merge — two concepts become one scientific concept

Two concepts merge when they are (a) strongly correlated **and** (b) ablation shows
they carry the *same* marginal information (removing either leaves skill intact;
removing both drops it). Merging is a scientific assertion — "these two measurements
are the same underlying property" — and is treated as such:

- The survivor is the **simpler / more interpretable** of the two (parsimony); or, if
  symbolic regression finds a single closed form that dominates both, that canonical
  form.
- The other concept becomes an **alias**: its identity in the evidence store points
  to the survivor; its history is preserved; its slot in the active representation is
  freed. Nothing is deleted.
- The merge is recorded as a reversible claim. If later data *separates* the two
  (they diverge in some regime), the merge is undone via a split.

---

## 6. Split — one concept becomes regime-specific concepts

A concept splits when its marginal value is **regime-dependent**: it contributes in
one structural regime and not (or negatively) in another. Detected when the concept's
ablation-importance varies significantly across the knowledge graph's existing
conditional partitions (density bands, families, frustration levels). The concept is
replaced by conditioned variants — "concept given condition" — mirroring exactly the
graph's existing conditional-fact structure (`observe_if(condition)`). Split is the
inverse of merge; both are driven by whether the evidence supports one underlying
property or several. Splitting adds dimensions, so it is gated by the same FDR,
stability, and effect-size requirements as fresh admission — over-splitting is a
failure mode to be policed.

---

## 7. Retirement — how a concept leaves the active representation

A concept is retired from the **active vocabulary** (not from evidence) when, over a
sliding recency window, any holds:

- its ablation marginal contribution has fallen to ≤ 0 (within noise), or
- it is dominated / made redundant by another (superseded), or
- its confidence has decayed below a floor.

**Confidence decay / forgetting.** Each active concept carries a *recency-weighted*
estimate of its marginal value, updated every cycle. This is deliberately different
from the knowledge graph's all-history Welford confidence: the *evidence* about a
concept accumulates forever, but a concept's *current usefulness* can decay as the
data distribution shifts (new families arrive; a once-vital measurement becomes
inert). The active representation must track recency; the evidence store must not.
Using the wrong statistic for either is an error.

**Tombstones.** Retirement leaves a tombstone in the representation: the index is
never reused or shifted (append-only *indices*, even as the active *set* shrinks), so
models trained at an older vocabulary version remain interpretable. The concept's
full definition and value history remain in the evidence store forever — a retired
concept, like a refuted theory, prevents re-exploring a known dead end.

---

## 8. Archive — how append-only evidence and a prunable representation coexist

This is the architectural keystone, and it resolves the apparent contradiction
between "knowledge is append-only" and "the representation must be prunable."

- **The Evidence Store** (KnowledgeGraph + ExperimentDb) is append-only and eternal.
  Every proposal, verdict, admission, merge, split, and retirement is an event with
  full provenance. Nothing is ever deleted. This is the scientific record.
- **The Active Representation** (the working vocabulary the models encode against) is
  small, versioned, and prunable. Crucially, it is **not an independent store** — it
  is a **materialised view derived from the evidence**: precisely "the set of
  concepts currently non-dominated, contributing, and non-retired." (This is a
  correction to the current prototype, where the registry is an independent mutable
  object.)
- **Coexistence:** because the active representation is *recomputable from the
  evidence stream* (STAGE_8 §3), retiring a concept never destroys anything — it only
  changes the projection. Reprocessing the evidence reconstructs the vocabulary
  deterministically. Append-only and prunable stop being in tension the moment the
  prunable thing is a *view* of the append-only thing.

---

## 9. Rediscovery — recovering a concept years later

Because a retired concept's definition and history live in the evidence store, it can
be **re-proposed** cheaply (no re-synthesis; only re-validation on new data). The
methodology distinguishes *why* a concept was retired:

- **Retired as intrinsically redundant / dominated** → stays dead, unless the concept
  that superseded it is itself later retired (then its dependents become rediscovery
  candidates).
- **Retired due to distribution shift** (it stopped contributing when the data moved,
  not because it was ever wrong) → a **rediscovery candidate** the moment new evidence
  enters a region where it was historically valuable.

Rediscovery is triggered by the proposal stage consulting Scientific Memory: when the
active vocabulary shows a coverage gap in a structural regime, the archive is queried
for concepts that historically filled that gap, and the best archived candidate is
re-validated before any new synthesis is attempted. This is why archival is not
sentimentality — it is what lets a years-old measurement come back into service the
instant it becomes relevant again, and what stops the scientist from forever
rediscovering and re-refuting the same dead ends.

---

## 10. The unified cognitive architecture

One evidence spine, one derived representation, all faculties reading it:

- **Evidence spine:** ExperimentDb (runs) + KnowledgeGraph (facts, concept verdicts,
  theories, refutations, merges, splits, retirements) — append-only, source-attributed.
- **Concept Evolution** is the *representation-learning faculty*. It consumes evidence,
  runs the proposal→validation→competition→lifecycle loop, and emits the **derived
  active vocabulary**.
- **The active vocabulary is the single shared language.** Predictor, Policy, World,
  Dynamics, Scientific Memory, Meta-Learner, Curiosity, and Planner all encode against
  it. No faculty holds a private representation.
- **Theory Engine** supplies the ablation machinery, reused for concept redundancy and
  retirement — the causal test is the same whether the object is an operator or a
  feature.
- **Meta-Learner** provides the *compression* fitness signal: fewer, sharper rules
  after admitting a concept is direct evidence the concept explains something.
- **Curiosity** does double duty: it biases concept *proposal* toward regions of high
  model residual (where the current vocabulary fails), and biases *experiment*
  planning toward regions where a concept's value is most uncertain.
- **Planner** schedules two kinds of work under one budget: experiments (to gather
  evidence) and concept evaluations (to grow the representation), trading them by
  expected information gain.

The resulting loop is the mission made literal: *experiments → evidence → concept
evolution → a better representation → better models and theories → better experiment
proposals → …*. The scientist improves its **language of thought**, not merely its
**beliefs within a fixed language**. That distinction is the whole point.

---

## 11. Mathematical foundations — adopt / reject, with reasons

| Method | Verdict | Role and reasoning |
|---|---|---|
| **MDL (Minimum Description Length)** | **Adopt — the backbone** | The principled unification of prediction and simplicity: a concept earns its place iff it *compresses* the data (model + residuals shorter than without it). It is the theoretical foundation of admission and of the simplicity objective. |
| **Bayesian model selection / BIC** | **Adopt — BIC as the tractable form** | Answers "is A better than B?" for non-nested concepts via posterior odds, with built-in complexity penalty. Full marginal likelihood is costly; BIC is the practical approximation and complements frequentist FDR. |
| **Pareto multi-objective optimisation** | **Adopt — the decision structure** | Fitness is genuinely multi-objective (§3); Pareto dominance with a parsimony tiebreak replaces gameable weighted sums. Backbone of fitness and competition. |
| **Symbolic regression** | **Adopt — the generator (Tier 4)** | Natively produces interpretable closed forms *and* an accuracy-vs-complexity Pareto frontier — exactly our fitness frame. The primary engine of nameable discovery. |
| **Evolutionary search** | **Adopt — the search strategy** | The natural way to explore the open-ended symbolic concept space (mutation/crossover of expressions), reusing the engine's existing evolutionary machinery conceptually. |
| **Elastic Net** | **Adopt — the cheap screen** | L1+L2 handles correlated concept groups gracefully; a fast pre-ranking of candidates and a cue for which to ablation-test. Not the final arbiter. |
| **LASSO (pure L1)** | **Adopt narrowly, reject as sole selector** | Useful as a fast sparsity screen, but under collinearity it arbitrarily picks one of correlated features and is unstable across resamples — so it must never make the final admission decision. Ablation + Pareto do. |
| **Information Bottleneck** | **Adopt as principle, reject as algorithm** | The right *ideal* — a concept as a minimal sufficient statistic of structure for predicting behaviour — but its variational/distributional estimation does not fit a small-N, deterministic regime. Use as a guiding principle, not a computation. |
| **Sparse coding / learned dictionaries** | **Reject** | Produces distributed, uninterpretable codes — violates first principle #1. Permissible only as a diagnostic that motivates an interpretable concept, never as a resident concept. |

**The stack, stated plainly:** MDL/BIC as the principled admission and comparison
backbone; Pareto multi-objective as the decision structure; symbolic regression +
evolutionary search as the interpretable generator; Elastic Net as the cheap screen;
ablation (reused from the Theory Engine) as the causal redundancy and retirement test;
FDR control over every batch. Rejected: any black-box latent representation as a
concept, and IB as a computational method.

---

## 12. The five standing dangers (read before extending this system)

1. **A concept space that is only algebraic combinations of five scalars is not
   discovery.** Real concepts measure the problem and its landscape (Tiers 1–3). If
   the system only ever combines the existing five, it is doing cosmetics, and it will
   plateau. Broadening the concept domain is prerequisite to everything else.
2. **Without false-discovery control, autonomous operation over years guarantees a
   corrupted representation.** FDR is not optional polish; it is survival.
3. **Validating a concept against a proxy model instead of the real downstream models
   admits concepts that help nothing.** Fitness must always be measured on the actual
   Predictor/Memory/Meta-Learner that will consume the concept.
4. **Conflating append-only evidence with the active representation blocks both
   pruning and honesty.** The active vocabulary must be a *derived, prunable view* of
   the *append-only evidence*. Keep them distinct and keep the view recomputable.
5. **The interpretability line is the science line.** The day the system admits a
   concept no human can name or explain, it stops being a scientist and becomes an
   opaque optimiser wearing a lab coat. Hold the line without exception.

---

## 13. What this means for implementation order (guidance, not schedule)

Do not extend the prototype toward open-ended generation or propagate concepts into
the learned models until: (a) validation is hardened to the §2 contract (folded,
FDR-controlled, family-level, CI-gated), (b) fitness is the §3 vector on the real
models, (c) redundancy/retirement (§4–7) exist, and (d) the active representation is
the §8 derived view. Building generation before judgement produces unfalsifiable
novelty — the precise failure this document exists to prevent. Judgement first,
generation second, propagation last.
