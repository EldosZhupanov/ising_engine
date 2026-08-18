# ROADMAP_AUTONOMOUS_SCIENTIST.md

> The path from "a platform a human drives" to "a scientist that drives itself."
> Each phase states **why before how**. Sequenced so that every phase is gated on
> *evidence*, not ambition (SOUL §6). This roadmap governs *research operations*;
> it does not supersede the Constitution (direction) or `ROADMAP.md` (engine status).

**North star:** a standing research operation that poses its own questions, invents
its own operators, refutes itself, and grows an append-only body of optimization
science — measured by *knowledge recorded and survived*, never by a benchmark score.

**Global success metric (not a benchmark):** monotone growth of *survived,
source-attributed, ablation-backed* facts in the KnowledgeGraph, with a falling
rate of later refutations of previously "confirmed" facts (calibration improving).

---

## Phase 0 — Recover the research memory
**Why.** By the project's own rule, an unrecorded run didn't happen — yet R1–R4 and
C1–C2 (the last months of findings) live in commit messages and assistant memory,
not in the ExperimentDb/KnowledgeGraph. Before building forward we must stop the
leak and repatriate what we learned, or we build on sand.
**How.**
- Land `research/RESEARCH_HISTORY.md` (done) as the canonical human-readable record.
- Write a one-shot importer that records the recovered findings into the append-only
  stores as *conditional, source-attributed* facts (source = "recovered-hand-study",
  low support), so the graph reflects reality.
- **Fix the leak:** the CLI research modes must write their results through
  `observe_if`/`flush_append`, not only `println!`. No new finding is produced
  outside the knowledge system again.
**Done when.** Every claim in RESEARCH_HISTORY.md has a graph fact with provenance;
new CLI experiments append automatically.

## Phase 1 — Clean the dead branches
**Why.** Dead ends left in the tree get re-explored and mislead the agenda. The
single-scalar predictor hunt (R1), the "universal law" framings (R3), and the
multidimensional-descriptor predictor (R4) are refuted; keeping them *as active
directions* wastes future compute. (Keep them as *recorded refutations* — that is
different.)
**How.**
- Mark R1/R3/R4 threads REFUTED in the agenda seed; forbid the loop from re-opening
  them without new evidence (Constitution amendment discipline).
- Seed the literature table (RESEARCH_GAPS PART 5) so the Reviewer tags re-derivations
  KNOWN and the agenda de-prioritises them.
- No code deletion of the CLI probes (they are useful measurement tools); only the
  *research directions* are closed.
**Done when.** The agenda contains explicit "do-not-reopen" refutation records and a
KNOWN-science table the Reviewer consults.

## Phase 2 — Make the Knowledge Graph the substrate of decisions
**Why.** Everything downstream (agenda, planner, executive) is only as good as the
knowledge it reads. Today the graph is under-fed and low-confidence (support≈1). The
scientist cannot prioritise questions it cannot see.
**How.**
- Route *all* agents' outputs (findings, refutations, ablations) into the graph with
  Welford confidence and `condition_holds` gating.
- Add **refutation debts** as first-class graph objects: any fact at support < k is a
  standing obligation to re-test.
- Backfill confidence via multi-instance `investigate` on the key operators (moves
  C1/C5 from support≈1 to durable).
**Done when.** The graph answers "what do we know, how sure, and what needs re-testing?"
and the answer drives the next two phases.

## Phase 3 — The Hypothesis / Question Engine (+ Red Team + Reviewer)
**Why.** This is the first phase that makes the system a *scientist* rather than a
runner: it must generate *falsifiable questions*, not just schedules, and must attack
its own answers the way a human red-team did this session (finding the density=ruggedness
and between-family confounds). Without an adversarial critic, the platform will confirm
its own artifacts.
**How.**
- Extend `lab`/`llm`/`curiosity` to emit **falsifiable questions with pre-registered
  success/failure criteria** (Experiment Designer records the criteria *before* the run).
- Build `redteam.rs`: for every candidate finding, enumerate confounds, run the
  between-group vs within-group decomposition, demand an ablation, and *veto* findings
  that don't survive. (This session's manual red-team is the spec.)
- Build `reviewer.rs`: check novelty against the graph **and the literature table**;
  tag KNOWN/NEW; block KNOWN re-derivations from consuming a campaign.
**Done when.** A finding can only reach "confirmed" after the Red Team failed to break
it and the Reviewer confirmed it is not KNOWN — with a recorded pre-registration.

## Phase 4 — The self-rewriting Research Agenda + Instance Foundry
**Why.** "Autonomous" means *choosing what to study*. The Planner picks instances; the
missing piece is a component that picks *questions* by expected knowledge gain and
rewrites its own priorities as facts accrue. And the highest-value *classical* open
question (O2: is ruggedness causal?) needs **controlled instance generation** — vary
one landscape axis all-else-equal — which `families.rs` cannot do yet.
**How.**
- Build `agenda.rs`: rank open questions/refutation-debts by expected knowledge gain
  (coverage deficit + model disagreement + confound risk); retire closed questions;
  hand the top question to the Executive each cycle.
- Extend `families.rs` into an **Instance Foundry**: generate families targeting a
  chosen ruggedness/frustration/degeneracy while holding size/density fixed, so the
  Theory Engine can run *causal* ablations (O2), not correlational ones.
- First autonomous science target: **O2 causal ruggedness study**, end-to-end through
  the loop (agenda → design → foundry → run → analyze → red-team → publish → graph).
**Done when.** The system runs O2 to a *published, self-refuted* conclusion with no
human specifying the hypothesis, design, or instances.

## Phase 5 — Autonomous campaigns (the standing operation)
**Why.** The mission is a research operation that runs around the clock (SOUL §"5–10
year vision"). Phases 0–4 make one honest cycle possible; Phase 5 makes it *continuous*
and self-sustaining — the "cumulative mechanism catalogue" (gaps PART 6, direction #3).
**How.**
- Run the budget-capped `--service` loop under the Agenda + Executive for weeks; grow
  the Foundation Dataset toward the 500k milestone; re-measure transfer as it grows.
- Monitor/Model-Registry go live as health gates so the unattended loop self-pauses on a
  broken invariant instead of drifting.
- Output: an append-only, self-refuted catalogue of *conditional* operator mechanisms.
**Done when.** The platform produces new *survived* graph facts per week unattended, and
its calibration (later-refutation rate) is improving.

## Phase 6 — Operator/Algorithm evolution → **synthesis** (the true frontier)
**Why.** This is the only phase that converts *selection* into *discovery* and is the
project's genuinely novel contribution (gaps PART 6, direction #1): FunSearch/AutoML-Zero
for physics-inspired operators, on a substrate where determinism makes fitness exact and
every discovery reproducible. It is deliberately last-but-one: it is only meaningful once
the agenda, red-team, controlled instances, and ablation loop (Phases 3–4) exist to judge
what it invents.
**How.**
- Extend evolution from *sequences of 14 operators* to *programs* over a typed sub-API of
  `SpinState`; compile in a sandbox behind the **verification firewall** (bit-identical
  cross-backend + golden regression + capability passport).
- Gate every survivor on **behavioral non-redundancy** vs existing operators and on a
  **causal ablation** (Theory Engine). Register only causal, non-redundant operators.
- Publish both outcomes honestly: a discovered operator *with its ablation*, **or** the
  negative result "the operator space is effectively closed under this API" — itself a
  real scientific statement.
**Done when.** The system has either registered a novel, ablation-backed operator no human
wrote, or published the rigorous negative result — both are wins.

## Phase 7 — Scientific publication
**Why.** Knowledge that isn't communicated in falsification-first form is not yet science
for the next reader (SOUL §7). The final step is a paper-grade, self-contained writeup —
including the kept refutations — generated from the stores.
**How.**
- Extend `reports.rs` into a Publication agent: assemble Question→…→Confidence→Refutations
  from the graph/DB for any finding, with reproducibility seeds and the KNOWN/NEW verdict.
- Cloud tier (Claude) drafts prose from the structured evidence; **honest skip** without a key.
**Done when.** The platform emits a reproducible, literature-aware, refutation-carrying
writeup of a Phase-6 (or Phase-4) result without human authoring.

---

## Sequencing rationale (why this order)
- **0→1→2** first because a scientist that can't trust or read its own memory cannot
  choose well. Fix the record and the substrate before adding cognition.
- **3 before 4** because a question engine without a Red Team will manufacture artifacts;
  the critic must exist before the system is trusted to self-direct.
- **4 before 5** because continuous operation amplifies whatever the agenda points at —
  point it well first.
- **6 near the end** because operator *synthesis* is only science if the machinery to
  *judge and refute* its inventions (3–4) already runs. Building the generator first would
  produce unfalsifiable novelty — the exact trap this whole document exists to avoid.

## What we will NOT do (guardrails, from SOUL §"important rules")
- Not optimise for benchmark scores, prettier code, or publication count.
- Not build a Transformer/GNN before the dataset earns it (Stage 11 gate stays).
- Not let the Operator Synthesizer bypass the verification firewall — ever.
- Not protect a prior conclusion: if a phase shows the whole direction is a dead end,
  we record it and turn.
