# Architecture Addendum v3 — Discovery Center, Git-for-Science, Publication
### Chief Architect · additions to `ARCHITECTURE_V2.md` (AD-1…AD-4 stand unchanged)

This addendum covers **only** what the latest brief adds. It does not restate the
critique (`ARCHITECTURE_REVIEW.md`) or the four prior decisions (`ARCHITECTURE_V2.md`).

---

## AD-5. The Discovery Center is a **UI for faculties that already exist** — not new AI

This is the most consequential finding of this round. Every "automatically generated"
item you asked for maps onto a **real, already-implemented** engine structure. We are not
inventing a research director; we are finally *exposing* the one that has been running
headless all along.

| Your ask | Existing engine source | Verified | On disk today? |
|---|---|---|---|
| **Operator Gaps** | `MetaLearner::suggest_operator_gap(&db) → OperatorGap{predecessor, successor, suggested_name, rationale, support}` (meta_learner.rs:100,513) | ✅ auto-mined | **Yes** — `operator_proposal_*.md` (real: `metropolis_gibbs`, provenance *"adjacent 1692× in top solutions"*) |
| **Weak Areas** (where we're wrong) | Curiosity **DISAGREEMENT** — predictor residual: "where the platform's own models are wrong" | ✅ computed | No — in-memory |
| **Unexplored / thin data** | Curiosity **COVERAGE** — operators tried rarely or never | ✅ computed | No — in-memory |
| **Unstable / needs characterizing** | Curiosity **ANOMALY** — cross-seed variance on the same instance | ✅ computed | No — in-memory |
| **Suggested Research / "what next?"** | `ResearchPlanner::rank_targets` → `TargetScore{reason}`; `PlannedTask{expected_value, info_gain, reason}` (planner.rs) | ✅ ranked **with reasons** | No — in-memory |
| **Unexplored design space** | `NoveltyArchive` + `schedule_distance` (novelty.rs) | ✅ computed | No — in-memory |
| **Theory Gaps** | Theories stuck at `TheoryStatus::Hypothesis` with low `trials`; low-reproducibility signatures | ✅ | **Partly** — `evaluation_report.md` |
| **Dataset Weaknesses** | `foundation_manifest.md` scale-gap (18,570 vs 500k → **27×**) + instance coverage | ✅ | **Yes** |
| **Benchmark Weaknesses** | `results/index.json` — underpowered runs (n=3, p=0.25) | ✅ | **Yes** |

**Decisions:**
1. **Discovery Center ships in two stages.** Stage 1 (**no Rust**): Operator Gaps, Dataset
   Weaknesses, Benchmark Weaknesses, Theory Gaps — all from artifacts already on disk.
   Stage 2 (**needs the export seam**): Curiosity's three signals + the Planner agenda.
2. **"What should I do next?" is answered by the engine, never by the UI.** The interface
   renders `PlannedTask.reason` / `TargetScore.reason` verbatim. It must **never**
   synthesize a suggestion of its own — that would be fabricated science.
3. **Priority upgrade:** exporting Curiosity + Planner is now a *top-tier* Rust item
   (it was "D5, later"). It converts the platform's own research direction into the
   product's most compelling screen — "here is where I am blind, and here is what I will
   study next, and why."

## AD-6. Campaigns as **Git for science** — the append-only log is already the substrate

Verified: a campaign is a directory of append-only artifacts (AD-2); the DB has monotonic
`id`, `timestamp`, `campaign_id`, `generation_id`; `model_registry.txt` already carries
**parent lineage**; `reports/report_{08}.md` are numbered and never rewritten.

**Insight:** the repository already has Git's *substrate* — an immutable, ordered,
content-addressed-by-seed history. What is missing is only **refs** and **diff views**.

| Git concept | Ising Engine equivalent | Status |
|---|---|---|
| commit | an appended `ExperimentRecord` (monotonic id) | exists |
| history | the append-only DB | exists |
| tag / release | a numbered `report_*.md` (a knowledge snapshot) | exists |
| lineage | `model_registry.txt` parent chain | exists |
| **clone / fork** | recursive copy of the workspace dir (consistent by construction) | **[PROPOSAL] S** |
| **ref / branch point** | `fork_manifest.json`: parent workspace, fork-point row id, engine version, timestamp | **[PROPOSAL] S** |
| **diff** | set-difference over graph facts + rule deltas + best-score/coverage deltas between two workspaces or two report numbers | **[PROPOSAL] M** |
| **checkout / replay** | re-run from a recorded seed (bit-identical, ADR-0004) | **needs D-phase** |
| **cherry-pick** | import a single operator proposal / theory with provenance intact | **[PROPOSAL] M** |
| **push / share** | export dir + manifest (+ signature later) → the marketplace unit | **[PROPOSAL] M** |

**Decision.** Implement `fork_manifest.json` and the **diff view** as the two new
primitives. Everything else is already there. Fork is *cheap and safe precisely because
nothing is ever mutated* — that is the payoff of the append-only constitution, and it
should be advertised as such. **Never** implement rebase/amend/force-push analogues:
rewriting history is the one operation this platform must never support.

## AD-7. Publication pipeline — "how do I publish this?"

You asked that a discovery immediately explain how to publish/export/share it. The repo
already produces the raw material: `reports/report_*.md`, `deep_analysis_*.md`,
`operator_proposal_*.md` (idea/math/pseudocode/properties/complexity/provenance),
`evaluation_report.md`, `portability_report.md`, `results/index.json` (with p-values).

**Decision [PROPOSAL].** A **Publication Composer** that assembles a discovery dossier
from existing artifacts — claim, mechanism, math/pseudocode, evidence table, benchmark
H2H with the honest power caveat, reproduce-recipe (seed + exact invocation), provenance
chain, refutations, open questions — exporting to Markdown / PDF (print theme) / BibTeX.
**No new science, no generated prose beyond templated assembly**; the LLM may *draft*
narrative but every number is transcluded from an artifact and labelled with its source.
Difficulty **M**, zero Rust (all inputs already on disk).

## AD-8. Provider matrix — one adapter covers most of the list

| Provider | Protocol | Browser-direct? | Work |
|---|---|---|---|
| **Ollama** | native `/api/*` | ✅ **verified live** (CORS OK, VRAM, tok/s) | none |
| **LM Studio** | OpenAI-compatible | ✅ (local, CORS-permissive) | *same adapter* |
| **vLLM** | OpenAI-compatible | ✅ (self-hosted) | *same adapter* |
| **OpenAI** | OpenAI | ✅ w/ user key | *same adapter* |
| **DeepSeek** | OpenAI-compatible | ✅ w/ user key | *same adapter* |
| **OpenRouter** | OpenAI-compatible | ✅ w/ user key | *same adapter* |
| **Anthropic** | Messages API | ✅ w/ user key (+ `anthropic-dangerous-direct-browser-access`) | small adapter |
| **Gemini** | Google GenAI | ✅ w/ user key | small adapter |

**Decision.** Two adapters (**OpenAI-compatible** + **Anthropic**) plus **Ollama native**
cover all eight. Keys live client-side only (never persisted server-side, never logged).
Engine-side parity (`llm.rs`/`cloud.rs` behind one trait) remains a later Rust item so the
engine and UI can share one provider config.

---

## Consolidated first block (revised by AD-5)

Unchanged in spirit from V2, with the Discovery Center promoted because it is cheaper and
more valuable than previously assessed:

| Order | Block | Rust? | Difficulty |
|---|---|---|---|
| 1 | **LLM Control Center** (providers ×8, model install/pull/load/stop, VRAM/ctx/tok-s, streaming reasoning, benchmark & compare) | none | M |
| 2 | **Foundation** (5-theme appearance engine, a11y/keyboard graph/contrast, mobile nav, honesty bugs, typecheck/CI/tests, error boundaries, skeletons) | none | S–M |
| 3 | **Comprehension** (Beginner Mode: tutorial/missions/guided experiment · Professional Mode · acting command palette + global entity search · contextual help/glossary) | none | M–L |
| 4 | **Discovery Center Stage 1 + Application Engine (T1/T2/T3 reduction atlas) + Publication Composer** | none | M |
| 5 | **Bridge** (`/system` CPU/RAM/GPU · `/artifacts`+SSE tail → kills staleness · live `/solve` · workspace fork/export/import + `fork_manifest.json`) | small | S–M |
| 6 | **Command** (`control_api`, in-process orchestrator: START AUTONOMOUS SCIENTIST, pause/resume/**step**, budgets, SSE mission-control stream) | Rust | L |
| 7 | **Signature** (runtime event export → live telemetry, energy landscape, trajectory scrubber; **Replay & Fork**; Curiosity+Planner export → Discovery Center Stage 2) | Rust | L |
| 8 | **Experience** (3D: living Ising lattice, energy landscape, knowledge-graph universe, operator evolution — R3F, GPU, reduced-motion-safe) | none | L |
| 9 | **Workbench + Git-for-science diff/lineage views** | none | M |
| 10 | **Completion** (theory persistence + curation per AD-3, remaining exports, model ops, provider trait, server-side query) | Rust | L |
| 11 | **Future** (multi-user, remote/cluster, marketplaces) | Rust | XL |

---

## Chief Architect's note on process

Three architecture documents now exist (`PRODUCT_SPEC.md` 810 lines,
`ARCHITECTURE_REVIEW.md`, `ARCHITECTURE_V2.md`) plus this addendum. The design is
**complete and internally consistent**: the environment is verified live, every
capability is classified REAL/OFFLINE/PLANNED, every subsystem is mapped to a real Rust
module or artifact, and the honesty invariants are specified.

Further planning now has **negative** expected value — the remaining uncertainty is the
kind only implementation resolves (real interaction feel, real perf, real ergonomics).
My recommendation as architect: **approve Blocks 1–4** (zero Rust, verified feasible,
highest impact) and let me build, verifying each block with lint → build → typecheck →
browser → screenshots → self-critique before moving on. Blocks 6–7 (`control_api`,
Replay/Fork) are the true "terminal optional" unlock and should follow immediately after,
as a separately-scoped Rust engagement.
