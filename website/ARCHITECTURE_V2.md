# Ising Engine — Scientific Operating System
## Chief Architect's Architecture Decisions & Roadmap v2

> Builds on `ARCHITECTURE_REVIEW.md` (critique of the current build, verified LLM/GPU
> environment findings). **That critique still stands in full and is not repeated here.**
> This document adds the architecture for the *new* mandate: Autonomous Scientist,
> theory lifecycle, campaign CRUD, Application Engine, marketplaces — plus difficulty
> estimates and a re-prioritized roadmap.
>
> Every claim is verified against the repository. Where something is a design proposal
> rather than an existing fact, it is marked **[PROPOSAL]**.

---

## Part I — Four architecture decisions that shape everything

### AD-1. The Autonomous Scientist button runs **in-process**, not as a spawned CLI

**Verified facts:**
- `ResearchOrchestrator::tick(...)` is **`pub`** (orchestrator.rs:182).
- `run_service(&instances, &reg, &evolver, &executor, &ocfg, budget)` is `pub` (:513).
- `ServiceBudget { max_ticks, max_experiments, max_wall_secs }` (:616), and
  `research_platform` **requires at least one non-zero cap** (research_platform.rs:925).

**Decision.** `control_api` **owns the orchestrator in-process** and drives `tick()` in a
supervised loop — it does **not** shell out to `research_platform`.

**Why this is strictly better:**
| In-process `tick()` loop | Spawning the CLI |
|---|---|
| True **pause** between ticks (no signal handling, no orphaned children) | Pause = kill; resume = re-load from disk |
| **Step** one tick at a time (a debugger for science) | Impossible |
| Emit SSE **after every tick** with structured state | Scrape stdout |
| Cancellation is a flag check | Process kill, partial writes |
| Backpressure and budget enforced in one place | Two budget authorities |
| No zombie/`SIGKILL` fragility | Process-supervision bugs forever |

The CLI remains the *reference* path (and stays supported for headless/cron use); the
browser gets the better one. **Pause/resume maps naturally onto the existing design**
because state is append-only and `campaign_state.txt` holds the cursor — resuming is
re-opening the directory, which `CampaignManager::open` already does.

*One honest consequence:* budget caps are currently mandatory. "Run indefinitely" needs
either a very large cap or a new `Unbounded` variant — a ~5-line Rust addition, but it
**is** an addition, not something I can pretend exists.

### AD-2. A "campaign" is a **directory** — so fork/clone/export are filesystem operations

**Verified facts:** `CampaignManager::open(dir)` (campaign.rs:166) builds the entire
platform state from one directory: `ai_experiments.txt`, `knowledge.txt`,
`knowledge_graph.txt`, `reports/`, `model_registry.txt`, `campaign_state.txt`,
`analysis/`, `proposals/`. `next_campaign_id` is a counter *inside* that directory.

**Decision.** The OS exposes a **two-level model**, because the repo has two levels and
the current UI conflates them:
1. **Workspace** = a campaign *directory* (e.g. `platform_gset`) — the unit of
   fork / clone / import / export / delete.
2. **Campaign run** = a numbered `campaign_id` *inside* a workspace — the unit of
   compare / replay / annotate.

**Consequences (all cheap):**
- **Fork / clone / duplicate** = recursive directory copy. Because every artifact is
  **append-only**, a fork is a *consistent snapshot by construction* — no locking, no
  migration, no partial-state hazard. This is an unusually clean property.
- **Export** = tar/zip the directory (add a manifest with provenance + engine version).
- **Import** = unpack + `CampaignManager::open` validates it.
- **Delete** = remove directory (with a confirm + "this destroys N recorded
  experiments" warning; append-only history is the project's core value, so deletion
  must be maximally frictional).
- **Workspace switcher** becomes a first-class OS control (like a project picker).

### AD-3. Human theory actions must **never** overwrite evidence — the Epistemic Firewall

**Verified facts:** `TheoryStatus { Hypothesis | Supported | Refuted }` (theory.rs:72)
is **derived from evidence**: `trials`, `survived`, and `confidence = rate × weight`
(:114), set by ablation outcomes (:101–105). `Theory` carries `explanation` and
`evidence: Vec<String>`. Theories are **never persisted to their own file** — only
`publish()` writes them into the KnowledgeGraph (:408).

**This is the most important design decision in the whole brief.** You asked for
"Approve Theory / Reject Theory / Promote / Archive." Implemented naïvely, a human
button that flips a `Refuted` theory to `Supported` **destroys the platform's central
guarantee** — the constitution's "truth over optimism," and the entire reason the
confirmed/refuted ledger is credible. It would convert a falsification engine into an
opinion tracker.

**Decision [PROPOSAL].** Two separate, non-overlapping layers:

| Layer | Owner | Values | Mutable by human? |
|---|---|---|---|
| **Evidence status** | the engine | `Hypothesis` / `Supported` / `Refuted` + trials, survived, confidence | **Never** |
| **Curation state** | the human | `endorsed`, `needs-more-trials`, `disputed`, `archived-from-view`, `promoted-to-publication`, with author + timestamp + **required rationale** | Yes, append-only |

So the UI's verbs become precise and safe:
- **Approve** → `endorsed` (a human vouches for a *Supported* theory; cannot endorse a
  Refuted one without recording an explicit `disputed` rationale that is displayed
  beside the refutation).
- **Reject** → `disputed` + rationale (the evidence stays; the disagreement is recorded
  *as data*, which is scientifically richer than deletion).
- **Promote** → `promoted-to-publication` (queues it into the report/paper pipeline).
- **Archive** → `archived-from-view` (hides from default lists; **never deletes** —
  append-only is inviolable).
- **Request more trials** → enqueues a real `--investigate` ablation run (Track D).

Curation is stored append-only (`theory_curation.txt`, same discipline as the DB) and
always rendered *next to*, never *instead of*, the evidence. **Human judgment becomes
first-class without ever being able to launder a refutation into a claim.**

This also fixes a real gap: theories currently have **no persistent home**. They should
get one (`theories.txt`) so they can be listed, versioned, diffed, and linked.

### AD-4. The Application Engine is **reduction-based and evidence-gated** — never vibes

Your Application Engine is the highest-fabrication-risk item in the brief. "This may
improve drug discovery" with no evidence is precisely the hype the project's
constitution forbids, and it would poison the credibility that the benchmark page's
"underpowered (n=3)" caveat currently earns.

**Verified fact:** the engine's *tested* families are exactly three —
`families.rs` provides `tsp_qubo`/`decode_tsp` and `max2sat_qubo` (proven against brute
force), plus MaxCut via `rudy_maxcut_ir`. The real cross-domain result is the
**9/12 (ρ +0.747) portability** of a MaxCut-trained policy to non-MaxCut instances.
There is **no** application/domain data anywhere in the repository.

**Decision [PROPOSAL].** Every application claim carries a **provenance tier**, and the
UI never renders a domain without one:

| Tier | Meaning | Evidence required | UI treatment |
|---|---|---|---|
| **T1 · Demonstrated** | The engine has actually run this problem family | In-repo experiments (MaxCut/G-Set, Max2SAT, TSP) + measured transfer | Full claim, with numbers |
| **T2 · Reduction-known** | A published QUBO/Ising reduction exists; **untested here** | A *citation* (e.g. Lucas 2014, "Ising formulations of many NP problems") + the reduction sketch | "Reducible in principle — **not tested on this platform**." No performance claim. |
| **T3 · Open question** | Plausible but no reduction and no test | Nothing | Rendered as a **question**, in an "Open Questions" board — never as a capability |

So: *graph coloring, number partitioning, max clique, set cover, knapsack, job-shop
scheduling, vehicle routing, portfolio optimization, lattice protein folding* are honest
**T2** entries (real published reductions, untested here). *LLM optimization, robotics,
autonomous driving* are **T3 open questions** — visible as research directions, never as
"this improves X." A hard invariant: **no domain card without a citation or an
experiment.** Uncertainty and unknowns are mandatory fields, not optional footnotes.

This turns the Application Engine from a liability into the most defensible feature in
the product — it becomes a *reduction atlas* with an honesty gate.

---

## Part II — The 16 requested outputs

### 1 · Everything still missing
See `ARCHITECTURE_REVIEW.md` §2 for the full inventory (control, LLM, discovery,
observability, models, data, platform UX, motion). **New in this brief:**
Autonomous-Scientist supervisor · tick-level step/pause · theory persistence +
curation layer · campaign/workspace CRUD (fork/clone/import/export/delete) ·
Application Engine (reduction atlas) · Open-Questions board · model benchmarking ·
CPU/thread/queue inspection · marketplace substrate (identity, packaging, signing,
provenance-preserving import).

### 2 · Everything that feels wrong
The product currently *narrates* science instead of *hosting* it. Specifically: it is a
frozen snapshot (no liveness); the platform's own LLM scientist is invisible while
Ollama runs on the same machine; half the nav is stubs; there is no way to *start*
anything; the flagship graph is mouse-only; and there is no answer anywhere to "why
does this matter / where could it apply" — the very question that makes the research
valuable. Plus the concrete defects in review §1.7 (no mobile nav, unlabelled
109,758-vs-18,570 contradiction, 122 KB unrendered markdown, silent 20k histogram cap,
sub-AA contrast).

### 3 · Every UX weakness
Review §1.1–1.8. Additions from this brief: no workspace switcher; no "what is
happening now / what changed today / where do I start" orientation surface; no
notification of newly-arrived discoveries; no run-comparison workspace; no annotation;
no undo; no multi-pane; no saved views; no shareable citation; no empty-state teaching;
no `?` cheatsheet; command palette that cannot act.

### 4 · Every architectural weakness
Build-time-frozen data (staleness by construction) · ingest reads whole files into
memory (unearned "millions" claim) · no server-side query path for the full DB (3.6 %
sample) · no API seam actually implemented (only designed) · theories have no persistent
store · no provider abstraction for LLMs · no event-log serialization (runtime is a
black box) · no error boundaries/typecheck/CI/tests · single hardcoded theme with tokens
inlined at call sites · unused 3D dependency weight · no auth/trust boundary defined for
the moment mutations arrive.

### 5 · Every missing scientific workflow
Create/run/stop/pause/resume/step a campaign · author or AI-generate a hypothesis ·
design/modify/launch an experiment · **replay** a recorded run bit-identically ·
**fork** a run by changing one parameter · compare runs/campaigns/operators/models/
theories · request an ablation (`--investigate`) · curate a theory (endorse/dispute/
promote/archive) with rationale · build/export a dataset · train/evaluate/promote a
model · inspect the executive's decision chain · browse memory recall · annotate any
artifact · cite a view.

### 6 · Every missing AI capability
Provider registry (Ollama/Anthropic/OpenAI-compatible/OpenRouter) · model install/
delete/pull-with-progress/start/stop/restart/switch · **benchmark a model** (tok/s,
TTFT, load time — computable purely in-browser) · live reasoning stream · running-prompt
inspector · current-thinking view · queue + active tasks · confidence display ·
LLM-initiated experiment/theory/operator feeds · accepted-vs-rejected discoveries with
reasons · tier-routing policy (local vs cloud) · cost/latency accounting · context-window
utilization · an in-product AI assistant grounded in the *real* artifacts (RAG over
reports/graph/papers).

### 7 · Every missing visualization
Living Ising lattice (spins, temperature, magnetization, field lines, phase transition) ·
energy landscape · trajectory scrubber/replay · knowledge-graph propagation pulses ·
theory crystallization · operator lineage/evolution tree · concept admission timeline ·
model lineage DAG · campaign/generation timeline · transfer matrix (family × family) ·
GPU/VRAM/tok-s gauges · queue/thread activity · reduction atlas graph (problem →
QUBO) · discovery diff (what changed between reports) · depth/parallax/glass/volumetric
lighting/procedural particles as *meaningful* layers.

### 8 · Every missing interaction
Inspector drawers · context menus · multi-select · pin/compare · drag to reorder ·
keyboard traversal (`j/k`, `g`-prefix, `?`) · command palette actions · inline edit ·
form validation · optimistic updates with rollback · toasts · confirm dialogs for
destructive ops · copy-CLI / copy-citation / copy-reproduce-recipe · export CSV/JSON ·
saved views · deep-link every state · undo.

### 9 · Every missing control surface
Campaign lifecycle · experiment lifecycle · autonomous-scientist master switch (+ step) ·
budget caps · queue/priority · scheduler controls · model lifecycle · LLM provider &
routing · theory curation · dataset build/export · workspace CRUD · settings/theme/mode ·
system (GPU/CPU/RAM/threads).

### 10 · Every missing browser feature
Theme engine (8 handcrafted) · modes (7) · onboarding/missions/tutorial · contextual help
+ glossary · searchable docs · global entity search · notifications · persistence
(localStorage/IndexedDB) · offline-tolerant caching · print stylesheet · a11y (keyboard
graph, focus rings, contrast, landmarks, skip-link) · error boundaries · loading
skeletons · virtualization for large tables · Web Worker for client-side compute
(lattice sim, parsing) · streaming fetch consumption (SSE/NDJSON) · file import/export
via File System Access API.

### 11 · Every backend API required
`control_api` (new axum bin):
- **Lifecycle:** `POST /campaigns` · `POST /campaigns/:id/{start,pause,resume,stop,step}` ·
  `GET /campaigns/:id/status` (SSE) · `POST /experiments` · `POST /experiments/:id/{run,cancel}` ·
  `POST /experiments/:id/replay` · `POST /experiments/:id/fork`
- **Workspaces:** `GET /workspaces` · `POST /workspaces/{fork,import}` · `GET /workspaces/:id/export` · `DELETE /workspaces/:id`
- **Read/live:** `GET /artifacts?path=` + `GET /artifacts/watch` (SSE tail) ·
  `GET /experiments?filter&cursor` (server-side query over the *full* DB) ·
  `GET /runtime/events` (SSE) · `GET /logs` (SSE)
- **Knowledge:** `GET/POST /theories` + `/theories/:id/curate` · `GET /concepts` ·
  `GET /memory/recall?signature=` · `GET /planner/agenda` · `GET /curiosity` · `GET /executive/decisions`
- **Models:** `POST /models/{train,evaluate,promote,rollback}` · `GET /models/lineage`
- **Datasets:** `POST /datasets/build` · `GET /datasets/:id/export`
- **System:** `GET /system` (GPU/CPU/RAM/threads) · `GET /health`
- **Solve:** `POST /solve` (reuse existing `server_api::compute()` unchanged)

### 12 · Every Rust change required
1. **`control_api.rs`** — new bin; owns orchestrator in-process (AD-1). **L**
2. **Runtime event-log export** — serialize `StepEvent`/`RunRecord`/`QualityMetrics`. *Highest-value single change in the repo.* **M**
3. **`ServiceBudget::Unbounded`** (or a very large cap) for indefinite autonomous runs. **S**
4. **Theory persistence** (`theories.txt`) + **curation store** (`theory_curation.txt`, append-only). **M**
5. **Export seams** → upgrade PLANNED→REAL: `memory_os` recall, `planner` agenda, `curiosity`/`novelty`, `monitor` health, `concept` admissions, `meta_layer` consensus, executive decision log. **M each**
6. **Replay & fork** entry points (seed + recipe → run; lineage recorded). **M**
7. **LLM provider trait** (Ollama / Anthropic / OpenAI-compatible / OpenRouter) unifying `llm.rs` + `cloud.rs`. **M**
8. **Model ops** (train/evaluate/promote/rollback) over `model_registry.txt`. **M**
9. **Server-side DB query** (filter/sort/paginate 110k→millions without loading all). **M**
10. Optional: workspace fork/export helpers (or do it in the bridge). **S**
*All additive. The read-only core (Runtime/Scheduler/scorer/backends) is not touched — per CLAUDE.md §3.2 and ADR-0004.*

### 13 · Every browser capability possible **without** changing Rust
**Verified live this session** (Ollama CORS-enabled on :11434):
- **Complete LLM cockpit** — list/pull-with-progress/delete/show models; load/stop via
  `keep_alive`; **VRAM** from `/api/ps` (`size_vram`); context (`ctx` vs
  `context_length: 32768`); **tok/s + TTFT + load-time** from `eval_count` /
  `eval_duration` / `load_duration` (measured **9.8 tok/s**, 75.3 s cold load);
  **streaming reasoning console** (`"stream":true`); **model benchmarking**; queue state.
- **Anthropic / OpenAI-compatible / OpenRouter** directly from the browser with
  user-supplied keys (kept client-side only).
- **Everything in Track B**: 8-theme engine · 7 modes · onboarding/missions/tutorial ·
  acting command palette · global entity search · **Discovery Center** (real
  `operator_proposal_*.md`: idea/math/pseudocode/properties/complexity/provenance) ·
  **Application Engine** (reduction atlas, T1/T2/T3) · Open-Questions board ·
  inspectors/context-menus/compare/pin/saved-views/CSV-export · **Reproduce (copy
  command)** recipes · **interactive Ising lattice + energy landscape + full 3D motion**
  (client-side sim in a Web Worker) · all a11y + bug fixes · rendered papers + discovery
  diffs (data already ingested).
- **Solve** is already live via existing `server_api` (no *new* Rust).
**Not possible without a bridge:** GPU utilization %, CPU/RAM/threads, live artifact
tailing, full-DB server-side query, and every *mutation* of engine state.

### 14 · New implementation roadmap
**Phase 1 — Living Intelligence (no backend).** LLM cockpit (providers, model manager,
telemetry, streaming console, benchmark) → the first genuinely live, mutating surface.
**Phase 2 — Foundation repair (no backend).** Theme engine (8) + a11y/contrast/keyboard
graph + mobile nav + the honesty bugs + typecheck/CI/tests/error boundaries/skeletons.
**Phase 3 — Comprehension (no backend).** Onboarding/missions/tutorial · acting command
palette + global search · contextual help/glossary.
**Phase 4 — Meaning (no backend).** Discovery Center · **Application Engine (reduction
atlas)** · Open-Questions board · rendered papers + discovery diffs.
**Phase 5 — Signature experience (no backend).** Living Ising lattice · energy landscape ·
graph propagation · theory crystallization · operator lineage · depth/glass/camera —
each animation tied to a real quantity. Use the installed `three`/R3F **or delete it**.
**Phase 6 — Workbench (no backend).** Modes · inspectors · compare workspace ·
saved views · annotations · reproduce-recipes · export.
**Phase 7 — Bridge (small).** `/system` · `/artifacts`+SSE tail (**kills staleness**) ·
live `/solve` · workspace fork/export/import (FS ops per AD-2).
**Phase 8 — Command (Rust).** `control_api` with in-process orchestrator (AD-1):
create/start/pause/resume/**step**/stop · budget caps · SSE status · queue.
**Phase 9 — Signature capability (Rust).** Runtime event export → live Runtime Monitor +
trajectory scrubber; **Replay & Fork** (the moat).
**Phase 10 — Completion (Rust).** Theory persistence + curation (AD-3) · remaining export
seams · model ops · provider trait · server-side DB query.
**Phase 11 — Future substrate.** Multi-user/auth · remote & cloud execution · distributed
workers · cluster view · **marketplaces** (packaging = AD-2 export + manifest + signature
+ provenance-preserving import; this is why AD-2 matters — a fork *is* a shareable unit).

### 15 · Prioritized by impact
| # | Item | Impact | Effort | Why this order |
|---|---|---|---|---|
| 1 | LLM cockpit (Phase 1) | ★★★★★ | M | Only item that is maximal impact *and* zero backend — verified live today |
| 2 | Foundation repair (Phase 2) | ★★★★ | M | Debt compounds daily; a11y/honesty bugs are disqualifying in review |
| 3 | Onboarding + palette (Phase 3) | ★★★★★ | M | Without it nobody can use any of the above |
| 4 | Discovery + Application Engine (Phase 4) | ★★★★★ | M | Answers "why does this matter" — your stated most-important feature |
| 5 | Bridge (Phase 7) | ★★★★ | S | One endpoint kills staleness across every existing workspace |
| 6 | `control_api` lifecycle (Phase 8) | ★★★★★ | L | The actual "terminal optional" unlock |
| 7 | Runtime export + Replay/Fork (Phase 9) | ★★★★★ | L | The moat; only possible because of determinism |
| 8 | Ising lattice + motion (Phase 5) | ★★★★ | L | Signature feel; after substance exists |
| 9 | Workbench (Phase 6) | ★★★ | M | Depth for daily users |
| 10 | Completion (Phase 10) | ★★★ | L | Closes the PLANNED list |
| 11 | Future substrate (Phase 11) | ★★ | XL | Needs 1–10 first |

### 16 · Difficulty estimates
**S** ≈ hours–1 day · **M** ≈ 2–5 days · **L** ≈ 1–2 weeks · **XL** ≈ 1 month+ (solo, verified at each step)

| Work | Difficulty | Risk |
|---|---|---|
| Theme engine (8 handcrafted) | M | Low — token indirection already in place |
| A11y + honesty-bug repair | S–M | Low |
| LLM model manager + telemetry | M | Low — API verified |
| Streaming reasoning console | M | Low |
| Model benchmarking | S | Low |
| Onboarding/missions/tutorial | M–L | Medium — content-heavy |
| Command palette v2 (acting + entity search) | M | Low |
| Discovery Center | M | Low — real artifacts exist |
| **Application Engine (reduction atlas)** | M | **Medium — needs disciplined citation sourcing, not code** |
| Open-Questions board | S | Low |
| Interactive Ising lattice (Worker + WebGL) | L | Medium — perf/reduced-motion |
| Energy landscape / trajectory scrubber | L | High — **blocked on runtime export** |
| Inspectors/compare/saved views/export | M | Low |
| Local bridge (`/system`, `/artifacts` SSE, `/solve`) | S–M | Low |
| Workspace fork/export/import | S | Low — FS ops on append-only dirs (AD-2) |
| `control_api` + in-process orchestrator | L | **High — supervision, cancellation, backpressure** |
| Pause/resume/step semantics | M | Medium — `tick()` is public, so tractable |
| Runtime event-log export | M | Medium — touching a hot path; must stay bit-identical |
| Replay & Fork | M–L | Medium — determinism does the hard part |
| Theory persistence + curation (AD-3) | M | Medium — **governance design matters more than code** |
| Remaining export seams (6) | M each | Low–Medium |
| Model ops (train/eval/promote) | M–L | Medium |
| Provider trait in Rust | M | Low |
| Server-side DB query | M | Medium — needed before "millions" is claimable |
| Multi-user / auth / remote / cluster | XL | High — real security boundary |
| Marketplaces | XL | High — identity, signing, trust, provenance |

---

## Part III — Non-negotiable invariants for the OS

1. **Never fabricate.** No metric without a source; no domain claim without a citation or
   an experiment; no "live" without a live connection.
2. **The read-only core stays read-only.** Runtime, Scheduler, scorer, backends are not
   modified for UI convenience (CLAUDE.md §3.2, ADR-0004).
3. **Append-only is inviolable.** Nothing in the UI may delete or overwrite recorded
   evidence. "Archive" hides; it never destroys.
4. **The Epistemic Firewall (AD-3).** Human curation is additive and always displayed
   beside evidence — it can never rewrite an evidence-derived status.
5. **Determinism is the product.** Every run shows its seed; every view is a citation;
   replay must be bit-identical or it is a bug.
6. **Uncertainty is a required field**, not a footnote — confidence, support, unknowns,
   and open questions are part of every claim's schema.
7. **Motion must mean something.** Any animation that does not encode a real quantity or
   state transition is deleted.
8. **The terminal stays supported.** The browser becomes primary, not exclusive; the CLI
   remains the reference implementation and the headless path.

---

## Recommendation

Approve **Phases 1–4** as the first implementation block (LLM cockpit → foundation repair
→ onboarding/palette → Discovery + Application Engine). It is entirely browser-side,
needs **zero Rust**, is verified feasible on this machine, and converts the product from
a viewer into a living instrument that *explains why it matters* — while `control_api`
(Phases 8–9, the real command layer) is designed and scheduled behind it.

The end state to build toward: **a deterministic scientific OS where any of 109,758
recorded experiments can be replayed bit-identically or forked with one changed
parameter, where an autonomous scientist can be started and stepped from a button while
you watch its LLM reason and its VRAM fill, and where every discovery states what it is,
what it beats, how confident it is, where it might apply — and what it still does not
know.**
