# Stage 8 — The Knowledge OS (the lab that explains, remembers, and wonders)

Status: DESIGN. Companion to `research/STAGE_7_RESEARCH_PLATFORM.md`.
Subordinate to the Constitution and ADRs. Nothing here weakens §4, §14,
ADR-0004 (reproducibility), or ADR-0005 (certified lowering). Where Stage 7 is
the *operating system* (run the lab), Stage 8 is the *knowledge system* (make
the lab understand what it is doing).

---

## 0. Thesis — three faculties and one honest frontier

Stage 6 gave the lab **capabilities**; Stage 7 gave it **operations**. Both stop
at the same ceiling: the platform *learns rules* but does not *understand*. It
can write "metropolis_sweep appears in 95% of the best solutions." It cannot yet
write "*because* the graph is sparse and highly clustered, a thermal pass lowers
the entropy barrier the greedy quench cannot cross — and here is the entropy
trace that shows it, and here is the ablation that would refute it if I am wrong."

Stage 8 adds three faculties, and names one honest frontier:

| Faculty | What it does | Existing seed | Frontier? |
|---|---|---|---|
| **Explain** (Theory Generator) | Turn rules into *falsifiable mechanism theories* | Meta-Learner rules + `StepEvent` entropy/acceptance/diversity traces | buildable |
| **Remember** (Scientific Memory) | Never lose a study; transfer across problem families and years | append-only DB + graph + portability bridge | buildable |
| **Wonder** (Curiosity Engine) | Direct compute at the *unknown*, not only the best | Predictor/World/Dynamics residuals + coverage | buildable |
| **Unify** (Research Foundation Model) | One model that predicts algorithm behavior across all families | Policy/World/Dynamics + portability evidence | **frontier** — gated on data + representation + scale |

The first three are real next rungs. The fourth is a direction, not a
deliverable — and this document says so plainly, because the difference between
a research OS and a slide deck is refusing to call an aspiration a feature.

---

## 1. The gap — correlation is not explanation

The Meta-Learner produces **rules**: statistically-supported correlations
conditioned on structure. A rule is a *what*.

```
 RULE       "metropolis_sweep is in 95% of best solutions   (support 89, conf 0.93)"
 THEORY     "On sparse (density<0.05), high-clustering instances, a thermal pass
             crosses the entropy barrier a greedy quench stalls at.
             MECHANISM SIGNATURE: energy-entropy falls monotonically during the
                                  metropolis phase while best-energy drops;
                                  acceptance stays > 0 (the ensemble is still moving).
             PREDICTION (falsifiable): remove the thermal pass on the same instance
                                  and the run stalls at HIGHER entropy and worse energy.
             STATUS: survived 6/6 ablation attempts → confidence 0.9."
```

The theory has four parts a rule lacks: a **mechanism**, a **signature** in the
observable trajectory, a **falsifiable prediction**, and a **refutation record**.
That is the whole difference, and it is the whole design.

**The load-bearing principle of Stage 8:** *a theory is not prose and not a
fancier rule — it is a mechanism hypothesis that makes predictions the Runtime
can try to refute.* An LLM may write the narrative; it may not confer truth. A
theory earns confidence only by surviving the platform's own attempts to break
it. This is Popper, wired into the experiment loop.

---

## 2. Pillar I — the Theory Generator (Explain)

### Role
Assemble candidate mechanistic theories from evidence, and — crucially — design
the experiments that would falsify them, so a theory is testable, not decorative.

### Inputs (all already recorded)
- **Rules** from the Meta-Learner (the correlation to be explained).
- **Structural features** that condition the rule (`InstanceStats`: density, clustering, degree-CV, …) — the *causal candidates*.
- **Mechanism signatures**: the per-step `StepEvent` log the Runtime already writes — `energy_entropy`, `acceptance`, `diversity`, `mean_energy` trajectories. This is the physical trace of what actually happened.

### The pipeline
```
 Rule (Meta-Learner)
   │
   ├─▶ correlate with structural features        → which structure predicts the rule
   ├─▶ correlate with trajectory signatures      → what observable mechanism co-occurs
   │      (entropy monotone? acceptance profile? diversity collapse?)
   ▼
 Mechanism hypothesis   ── LLM writes the narrative (proposer, not judge) ──┐
   │                                                                        │
   ▼                                                                        │
 Falsifiable prediction  ── "ablate operator X ⇒ the signature disappears" ─┤
   │                                                                        │
   ▼                                                                        │
 ABLATION experiment  ── designed automatically, RUN on the Runtime ────────┘
   │        (remove the pass / swap the operator / change the regime and check
   │         whether the predicted signature and outcome change)
   ▼
 Refutation record  ── survived k/k attempts → confidence ↑; refuted → archived as
                       a DEAD theory (kept forever — a refuted theory is knowledge too)
```

### The honest boundary
The Theory Generator does **not** *discover* causation from observation alone —
that is impossible and the design does not pretend otherwise. It *proposes*
mechanisms (cheap, from correlation + signature) and *earns* them by falsification
(expensive, on the Runtime). Correlation between sparsity and metropolis-success
never proves the entropy-barrier story; the ablation that shows the run stalls at
higher entropy *without* the thermal pass is what promotes hypothesis → theory.

### Output
Theories are first-class objects in the Knowledge Graph, with: mechanism,
signature, prediction, ablation results, confidence, and provenance. They become
context for the LLM ("here is what we *understand*, not just what correlates"),
inputs to the Decision Engine (choose operators by *why*, not just by rank), and
the seeds of operator proposals ("if the mechanism is barrier-crossing, a fused
thermal→cluster operator should cross it in one cheaper step").

---

## 3. Pillar II — Scientific Memory (Remember)

### Role
Never lose a study. Make the *entire* research history — across problem families
and across years — durable, queryable, and transferable, so 2026's MaxCut work
informs 2030's routing work.

```
 2026  MaxCut     ─┐
 2027  QUBO/BQP   ─┤
 2028  SAT        ─┼─▶  SCIENTIFIC MEMORY  ─▶  keyed by STRUCTURAL SIGNATURE,
 2030  routing    ─┘    (append-only, forever)   not by problem name
```

### The key design decision: index by structure, not by name
The current DB keys experiments by `instance_id` (a name). That is enough for one
family. Cross-family transfer needs the memory keyed by **structural signature**
— density, clustering, degree distribution, frustration, spectral features — so
"a sparse, high-clustering, frustrated instance" retrieves relevant history
whether it came from MaxCut, SAT, or routing. The portability bridge
(`qubo_model_to_ir`, energy-exact across BiqMac/BQP/QPLIB) and the +0.747 predictor
transfer already show cross-family knowledge is *real*; Scientific Memory is the
store that makes it *addressable*.

### What it holds (append-only, never deleted — §12)
- Every experiment (already: the DB).
- Every rule and theory, alive and refuted (a dead theory prevents re-exploring a dead end).
- Every model version and the snapshot it trained on (Stage 7 Model Registry).
- Every report and analysis.

### Contract
- **Nothing is ever lost** — deletion is not an operation. Compaction (summarize old detail into rules/theories) is allowed; erasure is not.
- **Everything is recomputable** — memory is derived from the append-only stream, so a corruption is repaired by replay.
- **Queryable by structure and by concept** — "what do we know about frustrated sparse instances?" returns experiments + rules + theories across all families and years.

Scientific Memory is what lets the lab *accumulate* rather than merely *run*. It
is the difference between a system that solves the current instance and one that
gets wiser every year.

---

## 4. Pillar III — the Research Planner (decide what to study)

### Role
Replace "a task exists because a human queued it" with "the lab wakes up and
decides what is worth studying today." The Planner ranks candidate hypotheses by
**expected value** and hands the top K to the Stage 7 Scheduler.

```
 morning:  candidate hypotheses = { (instance-regime, schedule) to try }
           score(h) = ExpectedValue(h)                         ← exploitation
           enqueue top-K by score, under the day's budget
           → "Today: 42 hypotheses. Here is why each earns its slot."
```

`ExpectedValue` comes from the models the lab already has: the Predictor's
expected improvement, the coverage deficit (regimes thin on data), and the World
model's imagined cost. Every ranked hypothesis carries *its reason* — so the
plan is auditable, exactly as the Decision Engine's plans are (§10 rule 2).

The Planner is the **exploitation** half. On its own it would grind the same
productive furrow forever. It needs a counterweight.

---

## 5. Pillar IV — the Curiosity Engine (seek the unknown)

### Role — the biggest departure from a reward-maximizer
A conventional optimizer chases the best solution. The Curiosity Engine chases
the **most informative** one. It directs compute where the lab's *understanding*
is weakest, not where its *reward* is highest. This is curiosity-driven /
active learning, made concrete.

### The three surprise signals (expected information gain)
```
 1. MODEL DISAGREEMENT   where Predictor / World / Dynamics predictions diverge
                         from each other OR from reality (high residual).
                         → the models are confused here; measure it.
 2. EPISTEMIC DEFICIT    regimes with few experiments (thin coverage).
                         → we simply do not know; go look.
 3. ANOMALY              the 5% that behave strangely: an operator whose
                         cross-seed variance is high, or whose behavior
                         CONTRADICTS a mined rule / theory prediction.
                         → the exception is where new physics hides.
```

`Curiosity(h) = w1·disagreement + w2·(1/coverage) + w3·anomaly`.

### The explore/exploit dial
The Scheduler blends the two planners:
```
 priority(h) = (1−λ)·ExpectedValue(h)  +  λ·Curiosity(h)
```
`λ` is the lab's temperament. `λ→0` is a pure optimizer (find the best schedule).
`λ→1` is a pure scientist (map the unknown). Stage 8 keeps `λ` non-zero and
adaptive: raise it when the best-energy trend plateaus (exploitation is spent),
lower it when a rich anomaly is being characterized. This is the classic
tradeoff, and making it a first-class dial is what turns a solver into a
researcher.

### Why anomalies matter most
A refuted theory prediction and a high-variance operator are the two richest
signals in the whole platform: they are where the current understanding is
*wrong*. The Curiosity Engine steering compute toward its own errors is the
engine of genuine discovery — and it is directly buildable as a scheduler term.

---

## 6. The knowledge ladder (and where we actually stand)

```
 RUNG              WHAT IT IS                          STATUS TODAY
 ──────────────────────────────────────────────────────────────────────────
 Experiment        a single provenance-stamped run     ✓ (18,570+ recorded)
 Rule              supported correlation               ✓ (Meta-Learner, 8 kinds)
 Hypothesis        a reasoned, testable proposal       ✓ (AI Scientist + LLM)
 Theory            mechanism + falsifiable prediction  ＋ Stage 8 (Pillar I)
                   + refutation record
 Knowledge         theories that survived + the        ＋ Stage 8 (Scientific Memory)
                   memory that keeps them, forever
 Foundation Model  one model that predicts algorithm   ✱ FRONTIER — see §7
                   behavior across all families
```

The platform today lives solidly at **Rule** and reaches into **Hypothesis**.
Stage 8's honest job is the **Theory** and **Knowledge** rungs. The Foundation
Model is over the horizon and is treated as such.

---

## 7. The Research Foundation Model — honestly scoped

> "Not GPT, but a Research Foundation Model, trained not on the internet but on
> millions of its own studies."

The instinct is right and the framing is right. The honest specifics:

**What it would be.** One model that, given an instance's structure (and,
eventually, the graph itself), predicts *which algorithm behaves how* — the
unification of today's four narrow models (Predictor, Policy, World, Dynamics)
into a single transferable one.

**What it needs — none of which is architecture alone:**
1. **Data volume.** Foundation-scale behavior needs orders of magnitude more than 18,570 experiments. Scientific Memory (Pillar II) is the substrate that accumulates it; the Curiosity Engine (Pillar IV) is what makes that data *informative* rather than redundant.
2. **A shared representation across families.** Today's models use 5 scalar features. A model that transfers MaxCut→SAT→routing needs a representation that captures structure those scalars miss — the point at which a Graph Neural Network finally earns its complexity (gated, per the roadmap, on evidence the scalars are the bottleneck — not assumed).
3. **Scale.** More parameters and compute than the current ridge/MLP regressors.

**The evidence it is even possible:** the portability result — a MaxCut-trained
policy winning 9/12 cross-family instances, +0.747 predictor transfer — is the
first sign that behavior learned on one family carries to another. That is the
seed of "foundation-ness." It is a seed, not a harvest.

**The honest verdict:** the Research Foundation Model is a *direction*, reached by
climbing the ladder (more data via Curiosity + Memory, then representation, then
scale), **not** by declaring it built. Saying so is the difference between this
document and a pitch.

---

## 8. How Stage 8 plugs into Stage 7

Stage 8 adds knowledge components; Stage 7's operational buses carry them.

```
 Runtime ─experiments─▶ Scientific Memory (append-only DB + graph, structure-keyed)
                              │
     ┌────────────────────────┼───────────────────────────┐
     ▼                        ▼                             ▼
 Meta-Learner            Theory Generator              Curiosity Engine
 (rules)                 (rules + StepEvent            (model residuals +
     │                    signatures → mechanism         coverage + anomaly
     ▼                    hypothesis → ABLATION on        → surprise score)
 Knowledge Graph ◀────────  the Runtime → theory)             │
     │  (rules + theories, alive & refuted)                   │
     ▼                                                        ▼
 Research Memory ─▶ Local + Cloud LLM (read theories, not just rules)
     │                                                        │
     ▼                                                        ▼
 Research Planner (ExpectedValue) ──blend λ──▶  Stage-7 SCHEDULER  ◀── Curiosity
                                                     │
                                                     ▼
                                                 next campaigns → Runtime → (loop)
```

The Theory Generator writes theories into the same graph the LLM and reports
already read, so understanding propagates for free. The Planner and Curiosity
Engine are two scoring functions the Stage-7 Scheduler blends — no new
substrate, just a richer priority. Scientific Memory is the Stage-7 append-only
store, upgraded with a structural-signature index.

---

## 9. What exists today vs. what Stage 8 adds

| Capability | Today | Stage 8 adds |
|---|---|---|
| Correlational rules | ✓ Meta-Learner (8 kinds, conditional, published) | — |
| Mechanism signatures | ✓ recorded in every `StepEvent` (entropy/acceptance/diversity) but unused for explanation | Theory Generator consumes them |
| Explanation | prose only, via LLM, unverified | falsifiable theory + auto-ablation + refutation record |
| Memory | ✓ append-only DB + graph, keyed by name | structural-signature index; cross-family/-year retrieval |
| Dead knowledge | rules can churn | refuted theories kept forever (prevents re-exploring dead ends) |
| Planning | Claude/human queues tasks | Research Planner ranks by expected value, auto-enqueues |
| Exploration | novelty floor in the designer | Curiosity Engine: disagreement + coverage + anomaly, as a scheduler term |
| Explore/exploit control | implicit | explicit adaptive λ dial |
| Cross-family transfer | evidenced (portability) | made addressable by Scientific Memory |
| Foundation model | 4 narrow models | direction, gated on data + representation + scale |

---

## 10. Build order (seeds first — each small, verifiable, behind a flag)

1. **Curiosity scheduler term** — the most self-contained and the biggest conceptual departure. Compute `disagreement` (model-prediction residual vs realized outcome, already measurable from the DB), `coverage` (experiments per structural bucket), `anomaly` (cross-seed variance + rule-contradiction), blend into the Stage-7 priority with a `λ` flag. Ships as: the Scheduler starts choosing the strange 5% on its own. [ADR-0012]
2. **Explanation assembler** — for each mined rule, attach the structural features that condition it and the mean trajectory signature (entropy/acceptance/diversity) of the runs it appears in. A "mechanism hypothesis" object in the graph. No causal claim yet — just the assembled evidence. Cheap, immediately useful to the LLM.
3. **Ablation loop** — turn a mechanism hypothesis into a falsifiable prediction and run the ablation on the Runtime; record survival/refutation; promote to theory on survival. This is the Popperian core; it closes the rule→theory gap honestly. [ADR-0013]
4. **Structural-signature memory index** — re-key retrieval by structure, enabling cross-family queries; validate on the existing MaxCut/BQP/QPLIB data.
5. **Research Planner** — the expected-value ranker over candidate hypotheses; blend with Curiosity via λ; auto-enqueue. The lab now sets its own agenda.

Order rationale: Curiosity first (novel, self-contained, and it *generates the
anomalies* the Theory Generator will want to explain); then explanation → theory
(the intellectual core); then memory index and planner (the scale-out). The
Foundation Model is not on this list — it is the *consequence* of running this
loop at volume, not a step to schedule.

---

## 11. Risks & failure modes

| Risk | Containment |
|---|---|
| **Theory over-claiming** — an LLM narrative treated as truth | A theory has no confidence until it survives ablations on the Runtime; prose alone is a hypothesis, labeled as such. |
| **Curiosity wireheading** — the engine chases noise as if it were signal | Anomalies must be *reproducible* across seeds to count; one-off variance is filtered; λ is capped so exploration never starves exploitation of the Runtime. |
| **Memory rot** — the store grows unqueryable | Compaction into rules/theories (never erasure); structural index; everything recomputable from the stream. |
| **Confirmation bias** — the platform only tests theories it expects to confirm | The ablation *tries to refute*; a theory that is never at risk earns no confidence; refuted theories are kept as evidence. |
| **Foundation-model mirage** — declaring the frontier reached | This document, and the reporting invariant: no capability is claimed beyond what a measured number shows. |

---

## 12. Summary

Stage 8 is where the lab stops merely *learning rules* and starts *understanding*.
Four faculties, honestly graded: it **explains** — turning correlations into
falsifiable mechanism theories the Runtime tries to refute, so a theory is earned,
not asserted; it **remembers** — an append-only Scientific Memory keyed by
structure, not name, so 2026's MaxCut work is still working in 2030's routing
study, and no idea is ever lost; it **plans** — a Research Planner that wakes up
and ranks the day's hypotheses by expected value, with a reason for each; and it
**wonders** — a Curiosity Engine that spends compute on the strange 5% where the
models are wrong, because that is where new physics lives. Above them sits one
honest frontier: a Research Foundation Model that unifies the narrow models across
families — a *direction* reached by climbing the ladder with more data,
representation, and scale, evidenced by real cross-family transfer, and never
called finished before a number says so. The Runtime still judges; the LLM still
only proposes; the memory only grows; and every theory must survive the lab's own
attempt to break it. That is a knowledge engine — a system that does not just
solve problems, but comes to understand them.
