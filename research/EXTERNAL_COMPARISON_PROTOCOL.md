# External comparison protocol

**Status:** binding procedure. Derived from `EXTERNAL_PROJECTS_BACKLOG.md`
(§1 priority queue, §5 platform-source policy, §6 activation order, §7
non-goals), which is not restated here. Gate and sequence context:
`../PROJECT_PLAN.md` §3 and §4.

**Precondition — non-negotiable.** No wave starts until a **replicated
equal-cost result** exists (`PROJECT_PLAN.md` §2 Stage S3). Comparing before
that produces a number nobody is allowed to interpret.

**Purpose.** To find out whether we are competitive, and to stop us presenting
an established field as a new one. A comparison is a falsification attempt, not
a marketing exercise.

---

## 1. Minimal baseline set — start here, do not install everything

The first comparison runs **four arms only**:

| Arm | What | Why it is mandatory |
|---|---|---|
| `random` | uniform random operator/schedule choice at the same budget | floor; a selector that does not beat it is not a selector |
| `greedy` | fixed best-on-average choice, no per-instance adaptation | separates *per-instance* value from *good default* value |
| one **established specialist** | one, chosen per wave (§3, §4) | the only arm that can refute a novelty claim |
| `ours` | the selector under test, in front of `UltimateSolver` | the claim |

`UltimateSolver` at identical seeds is present in every wave as the production
arbiter. Adding a fifth arm requires a written reason. **Do not install every
backlog candidate** — each added dependency is an unpaid maintenance and
provenance cost.

## 2. What is frozen before any external datum is collected

Written into a run manifest and committed **before** execution. Nothing below
may change after data is seen; a change requires an amendment naming what was
already observed.

| Field | Requirement |
|---|---|
| **Project commit / version** | exact SHA of this repo, plus version and commit or release tag of every external tool |
| **Adapters / conversions** | the exact conversion each way, with a round-trip identity test; conversion cost measured and reported separately, never hidden inside a solver's time |
| **Corpus** | named instances, their source and checksums; split into held-in and held-out **before** any run |
| **Seeds** | enumerated per arm; identical across arms wherever the design allows; no seed reused from a burned or reserved block |
| **Wall-time budget** | one number per instance, identical for every arm, including each arm's own setup and overhead |
| **Evaluation budget** | objective-call count where the comparison is call-based; an arm may not spend more calls to look better |
| **Anytime curves** | best-so-far versus wall time, recorded for every run, not just the endpoint |
| **TTS** | time-to-solution at a fixed target (`src/solver/tts.rs` convention), target frozen in advance |
| **Quality gap** | relative gap to the best known value for the instance, defined and validated against published optima |
| **Overhead** | our selector's own cost as a share of total wall time, reported always |
| **Confidence intervals** | bootstrap or Wilcoxon per `src/benchmark/stats.rs`; a point estimate without an interval is not a result |
| **Held-out routing** | held-out is opened **once**, after held-in analysis is frozen in git; no peeking, no re-splitting |
| **Kill criteria** | §5 below, fixed before the run |

**Budget parity is the whole comparison.** No claim survives a budget
mismatch — that is the single most common way this kind of work becomes
dishonest.

## 3. Wave 1 — algorithm selection and configuration conventions

**Question.** Is our per-instance choice better than what the established
selection/configuration field already does, at equal budget?

- **Conventions:** encode one frozen selector dataset in **ASlib** form and one
  small control environment in **DACBench** form. Stop if budgets, censoring, or
  state/action timing cannot be represented faithfully — a distorted encoding
  makes the comparison meaningless, and that stop is itself a reportable finding.
- **Specialist arm:** exactly one of **SMAC3**, **Nevergrad**, **Optuna**, chosen
  and named in the manifest before the run. Equal evaluation budget and seeds.
- **Bar:** our method must improve **held-out** quality or cost. Finding a
  *different* schedule is not an improvement.

## 4. Wave 2 — solver-quality comparison

**Question.** On weighted instances G-Set cannot identify, does the advantage
survive against a real solver baseline?

- **Corpus:** **BiqMac** (`benchmark_suite/biqmac/`). A G-Set-only result is
  insufficient by construction for any weight-aware or frustration claim.
- **Specialist arm:** one of **OpenJij** or **MQLib** — not both in the first
  run. Compare solution quality versus wall time on the frozen common corpus.
  Record conversion and initialization cost. **Reject any comparison based on
  iteration count alone.**

## 5. Kill criteria

- **X1** Adapter round-trip is not identity, or conversion changes the estimand
  → the arm is invalid; report and stop, do not "adjust" the conversion.
- **X2** Budgets cannot be equalised across arms → no claim is made from that
  run.
- **X3** Our arm does not beat `random` **and** `greedy` on held-in → stop
  before touching held-out or any external tool.
- **X4** Held-out shows no gain, or the interval crosses zero → publish the
  negative; do not re-split, re-target TTS, or widen the corpus.
- **X5** Overhead consumes the gain at the stated budget → the gain is not real
  at that budget; say so.
- **X6** The specialist matches or beats us → publish it, and revise the novelty
  position accordingly.

Kills are published with the same care as wins. Changing the metric, the corpus,
the budget, or the wording to avoid a kill is a protocol violation.

## 6. Wave 3 — applied pilot, blocked

**Soup / PEFT (LoRA subset selection as QUBO) runs only after a core
optimization win** — that is, after waves 1 and 2 have produced a defended
result. Its own conditions when it becomes eligible: identical real-evaluation
budget against Soup's CMA-ES, greedy, random, Optuna and exhaustive search where
feasible; task metrics frozen before optimization so the optimizer cannot select
its own judge; base model, revision, adapter hashes and licenses recorded.

Our engine is the outer-loop discrete optimizer there, never a replacement for
gradient training.

**Bounded uses, unchanged from the backlog.** MiroFish — market and scenario
research only; it cannot validate solver quality or forecast demand. Obsidian —
a generated, one-way Markdown view; git-tracked files stay the source of truth.
Graphiti — rejected; it duplicates the existing deterministic knowledge graph.

## 7. Execution discipline

Each wave is a **separate atomic plan**, preregistered in this directory before
it runs, with an **independent read-only review** before any datum is collected
and again before any result is written up. One implementer, one reviewer.
`memory/CURRENT_HANDOFF.md` is updated after each wave.

A comparison that is not recorded with its manifest did not happen.
