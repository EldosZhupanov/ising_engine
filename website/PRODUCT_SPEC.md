# Ising Engine — Scientific Operating System
## Complete Product Specification (v1, pre-implementation)

> **Status:** specification for review. No implementation has begun. Every
> feature is classified **REAL** (backed by data/endpoints that exist today),
> **OFFLINE** (interface designed against a typed seam; wiring requires a
> control-plane API that does not exist yet), or **PLANNED** (subsystem has no
> serialized output yet; needs a Rust export seam first). Nothing here invents
> data, metrics, or APIs. All numbers cited are drawn from the real repository
> artifacts (`experiments/…`).

---

## 0. What this is

The Ising Engine is an **autonomous scientist**: a deterministic Rust runtime
(3 verified backends, 14 physics operators, bit-identical replay) beneath a
34-module cognitive stack that forms hypotheses, runs provenance-stamped
experiments, mines rules, builds a knowledge graph, promotes rules to
falsifiable theories, refutes them by ablation on its own runtime, discovers new
structural concepts, and never forgets a result. Today it is operated entirely
from a CLI (`research_platform`, ~55 flags) and observed only by opening
generated text/HTML files. **Its intelligence is real; its interface is a
terminal and a folder of `.txt` files.**

This product turns the browser into the **primary interface** of that platform —
a dark, instrument-grade *operating system* of workspaces. The terminal becomes
optional. You do not read *about* the engine; you *operate* it and *interrogate*
its evidence. The bar is Palantir Foundry × DeepMind internal research × Apple
developer tools × Linear/Raycast interaction quality.

**The prime directive — honesty.** The engine's own constitution is "truth over
optimism; a result exists only if recorded and reproducible." The interface
inherits this literally: recorded data is shown with provenance; designed-but-
unwired controls are labelled OFFLINE; unbuilt surfaces are labelled PLANNED. A
fake dashboard would betray the product it represents.

---

## 1. Product vision

**Vision.** *The control room of an autonomous scientist — where a researcher can
watch it think, interrogate everything it has learned, reproduce any result to
the bit, and (increasingly) direct what it studies next, all from the browser.*

**Three-year arc.**
1. **Observatory (this build).** Everything the engine has already recorded
   becomes browsable, queryable, and reproducible-on-paper: 110k experiments,
   ~300 graph facts, the evaluation, the dataset, the written reports. REAL,
   static-first, no backend required.
2. **Cockpit.** A control-plane API (`control_api`) wraps `research_platform` and
   streams the runtime event log. Campaigns start/pause/resume from the browser;
   live telemetry replaces recorded snapshots. OFFLINE surfaces light up.
3. **Laboratory-as-a-service.** Remote/cloud workers, multi-user, versioned
   theories/concepts, replay-as-action, collaborative annotation.

**What it is not.** Not a landing page, not a marketing site, not a generic
BI dashboard, not a SaaS product tour. It has no "sign up," no pricing, no
testimonials. It is an instrument.

---

## 2. Design principles

1. **Expose, don't decorate.** Every pixel carries information or affords an
   action. If a panel cannot be backed by a real artifact or a real action, it is
   labelled PLANNED — never faked.
2. **Provenance is first-class.** Determinism is the platform's whole point, so
   every datum shows where it came from (file, row count, campaign, seed, mtime).
3. **Recorded ≠ live ≠ planned.** A persistent connection-state model makes the
   epistemic status of every surface unmistakable.
4. **Density with calm.** Bloomberg-terminal information density rendered with
   Apple restraint: hairlines, mono numerics, one accent, generous negative space.
5. **Keyboard-native.** `⌘K` command palette, `/` search, `g`-prefixed workspace
   jumps, `j/k` list traversal. The terminal user must feel faster here, not slower.
6. **Reproducible views.** Every workspace, filter, and selection is URL-encoded
   and shareable; back/forward works; a link is a scientific citation.
7. **Motion is telemetry.** Animation communicates state (a pulse = a live tick, a
   travelling dot = accruing support), never ornamentation. Fully reduced-motion
   safe.
8. **Scale by construction.** The client never receives raw 110k rows; it receives
   aggregates + bounded samples, and (later) paginated/streamed API responses.

---

## 3. User personas

- **P1 · The Principal Investigator ("does it actually work?").** Skims Mission
  Control, checks the confirmed/refuted ledger, drills a benchmark head-to-head,
  reads the latest report. Wants: credibility, honesty, the one-glance verdict.
  Success = "I trust these numbers and I can see the refutations."
- **P2 · The Operator ("run me a campaign").** Lives in Campaign Manager and
  Runtime Monitor. Wants to launch a service loop on a set of instances, watch it
  tick, pause when a health gate trips. (Mostly OFFLINE today → the seam is built
  for them.)
- **P3 · The Analyst ("what did it learn, and why?").** Interrogates the Knowledge
  Graph, Theory Engine, Experiments Explorer, Evaluation. Wants filterable
  evidence, ablation results, rule reproducibility across instances, and the
  ability to trace a claim to its supporting runs.
- **P4 · The ML/Foundation researcher ("is the dataset ready?").** Dataset
  Explorer + Model Registry. Wants the honest scale-gap, the feature schema, model
  lineage. Success = "I can see exactly how far from foundation scale we are."
- **P5 · The Reviewer / Investor (YC, OpenAI, DeepMind).** Drops in cold, needs to
  believe within 60 seconds this is a serious research platform. Every surface must
  survive scrutiny — hence no fake data anywhere.

---

## 4. Core workflows (as the interface supports them)

Each workflow lists the **real artifacts/modules** it touches and its classification.

**W1 · Research loop (observe→learn→discover→theorize→plan→run→refute→record).**
The engine's heartbeat (`orchestrator.rs::tick`). *Observatory:* Mission Control
visualizes the loop as a live-shaped ring with each stage's real last-tick output
(recorded). *Cockpit:* the ring animates on real ticks via SSE. REAL (recorded) →
OFFLINE (live).

**W2 · Experiment interrogation.** Filter the ~110k `ai_experiments.txt` records by
operator/backend/instance/campaign; inspect energy-vs-baseline; open one record to
see its full provenance (seed, sweeps, temps, replicas, timestamp) and a
"reproduce" recipe (the exact `research_platform` invocation). REAL.

**W3 · Knowledge interrogation.** Traverse the real `knowledge_graph.txt` triples
(`fails-on`/`effective-on`/`precedes-well`) with weight+support+confidence; filter,
search, and pivot to the runs that support a fact. REAL.

**W4 · Theory discovery.** See rules → mechanisms → falsifiable predictions →
ablation verdicts (`theory.rs`), with the real refutations (e.g. `extremal_metropolis`
weak on G-Set) shown as kept knowledge. REAL for recorded verdicts (via graph +
reports); OFFLINE for launching a new `--investigate`.

**W5 · Concept evolution.** The `propose→test→gate→admit` pipeline (`concept.rs`),
with the real Occam story (the 11-feature descriptor was refuted). REAL as
narrative + admitted-concept list; PLANNED for live concept proposals (no
serialized concept stream yet).

**W6 · Campaign operation.** Timeline of recorded campaigns/generations (from the DB
`campaign_id`/`generation_id` + `campaign_state.txt`); create/start/pause/resume
designed against the API seam. REAL (recorded timeline) → OFFLINE (control).

**W7 · Runtime & operators.** 3 backends; the real 14-operator vocabulary with the
discovered behavior of each (ordering effects, budget-scaling, antipatterns from
the reports/evaluation); capability-passport framing. REAL.

**W8 · Benchmark comparison.** Real head-to-heads vs OpenJij/neal (`results/index.json`):
win-fraction, Wilcoxon/sign-test p, per-engine means. REAL.

**W9 · Dataset / foundation-model readiness.** The `dataset/*.tsv` + manifest with the
honest scale-gap (18,570 vs 500k/1M/5M/20M). REAL.

**W10 · Memory recall.** "Have we seen a structure like this?" (`memory_os.rs`).
PLANNED (recall/index is in-memory only; needs an export seam) — designed with a
precise "what's needed to go live" note.

**W11 · Solve (playground).** POST a QUBO to the existing `server_api /api/v1/solve`
and show the real solution. REAL-live (requires the user to run `server_api`).

---

## 5. Information architecture

### 5.1 Sitemap / workspace hierarchy (grouped rail)

```
Ising Engine OS
├─ OVERVIEW
│   └─ Mission Control            /                      REAL
├─ OPERATE
│   ├─ Campaign Manager           /campaigns             REAL(recorded)+OFFLINE
│   ├─ Experiment Builder         /experiments/new       OFFLINE
│   ├─ Experiment Queue           /queue                 PLANNED
│   ├─ Scheduler                  /scheduler             PLANNED
│   ├─ Runtime Monitor            /runtime/monitor       PLANNED(needs event-log export)
│   └─ Solver Playground          /solve                 REAL-live(server_api)
├─ EVIDENCE
│   ├─ Experiments Explorer       /experiments           REAL
│   ├─ Benchmark Center           /benchmarks            REAL
│   ├─ Dataset Explorer           /dataset               REAL
│   └─ Evaluation & Portability   /evaluation            REAL
├─ KNOWLEDGE
│   ├─ Knowledge Graph            /graph                 REAL
│   ├─ Theory Engine              /theories              REAL(recorded)+OFFLINE
│   ├─ Concept Evolution          /concepts              REAL+PLANNED
│   ├─ Scientific Memory          /memory                PLANNED
│   └─ Research Papers            /papers                REAL
├─ MACHINERY
│   ├─ Operator Registry          /operators             REAL
│   ├─ Runtime & Backends         /runtime               REAL
│   ├─ Feature Registry           /features              REAL+PLANNED
│   ├─ Model Registry             /models                REAL
│   └─ Predictor · World · Policy  /models/:kind         REAL(weights)+PLANNED(internals)
└─ SYSTEM
    ├─ Operator Proposals         /proposals             REAL
    ├─ API Explorer               /api                   REAL(/solve)+OFFLINE
    ├─ Logs                       /logs                  OFFLINE
    ├─ Health Monitor             /health                PLANNED
    ├─ Documentation              /docs                  REAL
    └─ Settings                   /settings              REAL(local)
```

### 5.2 Navigation hierarchy
- **Primary:** fixed left rail, 6 groups (Overview · Operate · Evidence ·
  Knowledge · Machinery · System), collapsible to icons. Active item + group
  header. Each item shows a tiny status dot (REAL green · OFFLINE amber · PLANNED
  hollow).
- **Secondary:** in-workspace tabs / segmented controls (e.g. Experiments →
  Table · Distributions · Provenance).
- **Tertiary:** contextual inspector (right drawer) for a selected entity.
- **Command:** `⌘K` palette overlays everything (navigate, filter, run action,
  copy CLI, jump to entity).

### 5.3 URL hierarchy (deep-linkable, shareable)
```
/                         Mission Control
/experiments?op=metropolis_sweep&backend=DenseByte&instance=G22&sort=-improvement
/experiments/:id          single record (inspector deep-link)
/graph?rel=fails-on&node=extremal_metropolis
/benchmarks?exp=EXP-0000
/evaluation?signature=Ordering
/dataset?tab=schema
/operators/:name          e.g. /operators/gibbs_color_sweep
/models/:kind             predictor|world|policy|dynamics
/campaigns/:id/generations
/solve                    playground
```
Every filter/selection is a query param; the URL is the citation.

### 5.4 State hierarchy
- **Route state** (URL): active workspace, tab, filters, selected entity. Source
  of truth; enables share/replay/back-forward.
- **Session state** (memory): rail collapsed, theme, last-visited, command history.
- **Data state** (build-time, immutable): the generated real-data bundle +
  provenance manifest.
- **Connection state** (`offline` default; `live` when a control-plane responds):
  gates every OFFLINE/live surface. Global banner + per-panel badges.
- **Ephemeral UI** (component): hover, focus, open menus.

### 5.5 Permission hierarchy (designed for the Cockpit era)
- **Viewer** (default, and the only role in the static Observatory): read
  everything, reproduce-on-paper, copy CLI, no side effects.
- **Operator:** may start/pause/resume campaigns and solves (guarded by the
  control-plane; local-bind + token before any of this is enabled).
- **Admin:** configuration, model promotion, data-dir selection.
- The static build ships as Viewer-only; the trust boundary is the future API.

### 5.6 Command hierarchy (⌘K)
- **Navigate:** "Go to <workspace>", "Open operator gibbs_color_sweep",
  "Open record #40542".
- **Filter/Query:** "Experiments where op = metropolis_sweep", "Facts: fails-on".
- **Act (REAL):** "Copy reproduce command", "Solve current QUBO", "Export view CSV".
- **Act (OFFLINE, shown disabled with reason):** "Start campaign", "Pause loop".
- **Meta:** "Toggle theme", "Show provenance", "Copy shareable link".

---

## 6. Design system — "Instrument" (dark)

**Palette (tokens, CSS vars `--os-*`).** ground `#07090C` · panel `#0E1319` ·
raised `#141B23` · glass `rgba(16,22,30,.6)`+blur(20px) · hairline `#1B2530` ·
hairline-2 `#243040` · ink `#E6EBF1` · muted `#8A97A6` · faint `#59657A` · accent
orange LED `#F0842E` (single action/active color) · science cyan `#49B6E8` ·
science violet `#8B7DF6` (systems/graph only) · confirmed `#39B87A` · refuted
`#E5624A` · open `#49B6E8` · warn amber `#E0A73B`. WCAG-AA verified on ground.

**Material.** Instrument panels: 1px hairline frame, 12–16px radius, faint inner
top-highlight, corner tick marks, optional 2% scanline/grid texture; glass for
overlays (command palette, inspector, toasts). Depth from light + hairlines, never
saturated gradient.

**Typography.** Geist Sans (UI/headings) + Geist Mono (all data, IDs, energies,
seeds, operator names, code). Tabular-nums everywhere numbers align. Mono eyebrows
`NN · WORKSPACE`.

**Grid & density.** 8-pt system; rail 248px (64px collapsed); content max 1440;
inspector 360px; panel padding 20–24; comfortable/compact density toggle.

**Motion.** Eases `[0.16,1,0.3,1]`; reveals 400–700ms; hovers 120–160ms; workspace
cross-fade 260ms; live pulses 1–2s. All gated by `useReducedMotion`.

**Iconography.** lucide, 1.75 stroke, science-neutral; status dots not emojis.

**Component library (atoms→organisms).**
`Panel`, `PanelHeader`(title+status badge+actions), `Readout`(mono value+label),
`StatTile`, `Pill`(confirmed/refuted/open/neutral), `StatusDot`, `ProvenanceTag`
(file · rows · mtime), `ConnBadge`(REAL/OFFLINE/PLANNED), `KV` table, `DataTable`
(virtualized, sortable, sticky header, keyboard j/k), `FilterBar`, `Chip`,
`Distribution`(histogram), `Sparkline`, `GraphCanvas`(force/'arc), `Timeline`,
`CodeBlock`(copyable CLI), `Inspector`(right drawer), `CommandPalette`, `EmptyState`,
`ErrorState`, `SkeletonPanel`, `Toast`.

**Global states (every data surface implements all four).**
- *Loading:* skeleton panels with shimmering hairlines (no spinners).
- *Empty:* explicit — "No records match op=X" + a reset action, never a blank box.
- *Error:* "Couldn't read `<file>`" + what to check; static build uses the fixture
  fallback so this is rare.
- *Offline/Planned:* a labelled scrim with the precise "what's needed to make this
  live" note and, where possible, the recorded data behind it.

---

## 7. Screen-by-screen specification

Each screen follows one template:
**Purpose · Classification · Repo mapping** (Rust modules / structs / files /
artifacts / binaries / CLI replaced / future endpoint) **· Layout · Panels &
widgets · Interactions · Keyboard · Context menu · States.**
The seven core workspaces are specified at full fidelity; the long tail reuses the
same template with its own repo mapping (given in condensed form). "Repo mapping"
is the contract that keeps every screen honest.

---

### 7.1 Mission Control — `/` — REAL

**Purpose.** The 60-second verdict + the live-shaped heartbeat. Answers "is it
real, is it running, what has it learned lately, does it tell the truth?"

**Repo mapping.**
- Reads (ingested): `ai_experiments.txt` (aggregates), `knowledge_graph.txt`
  (fact counts by relation), `campaign_state.txt` (tick cursor), latest
  `reports/report_*.md` (best-result line + newest discoveries), ROADMAP
  confirmed/refuted ledger.
- Modules represented: `orchestrator`, `campaign`, `db`, `graph`, `executive`.
- CLI replaced: eyeballing `dashboard.html` + `tail report_*.md`.
- Future endpoint: `GET /api/v1/overview`, `GET /api/v1/loop/status` (SSE) for live.

**Layout.** Header (title, connection badge, data provenance) → hero row: left
= headline + loop ring; right = the four telemetry readouts. Then: "Latest
discoveries" (from newest report), "Confirmed vs Refuted" ledger, "Campaigns"
mini-timeline, "Jump in" grid.

**Panels & widgets.**
- *Loop ring* — 8 stages (observe…record) as an arc; each node shows the real
  module + its last-tick role; the active node pulses (recorded position from
  `campaign_state`; live via SSE later). `ProvenanceTag: campaign_state.txt`.
- *Telemetry readouts* — Experiments (Σ rows across DBs), Distinct instances (23),
  Operators (14), Backends (2), Best score (−26664 on G63 from report). Count-up.
- *Latest discoveries* — top N rule lines with support+confidence, parsed from the
  newest `report_*.md`; each links into Knowledge Graph / Experiments filtered.
- *Confirmed/Refuted ledger* — real entries (cross-family transfer 9/12 +0.747;
  world-model 0.975; DenseByte 4.8×; **refuted:** extremal_metropolis weak,
  evolved plans lose 5/5 to UltimateSolver). Pills; each expandable to its evidence.
- *Campaign strip* — recorded campaigns (platform_gset/grow/grow_div) with run
  counts; click → Campaign Manager.

**Interactions.** Click any readout → the workspace that owns it. Hover a loop
node → tooltip with the module + file it writes. Everything deep-links.

**Keyboard.** `g m` Mission Control (global); `1–5` focus the readouts; `⌘K`.

**Context menu (right-click a readout/fact).** Copy value · Copy provenance ·
Open source workspace · Copy shareable link.

**States.** Loading → skeleton readouts; Empty (no experiments dir) → fixture
fallback + a banner "showing bundled sample data"; Error → which file failed.

---

### 7.2 Experiments Explorer — `/experiments` — REAL

**Purpose.** Interrogate the ~110k append-only records; find, compare, and
reproduce any run.

**Repo mapping.**
- Reads: `ai_experiments.txt` (decoded 21-field schema). Ingest emits per-
  operator/backend/instance/campaign rollups + a bounded ≤5k representative sample
  (stratified by instance × operator) for the table; full aggregates for charts.
- Structs: `ExperimentRecord` (`db.rs`).
- CLI replaced: `awk`/`grep` over the DB; `--structural`.
- Future endpoint: `GET /api/v1/experiments?filter…&page…` (server-side paging over
  the full DB); `GET /api/v1/experiments/:id`.

**Layout.** Left FilterBar (sticky) · center Table/Distributions tabs · right
Inspector (on row select).

**Panels & widgets.**
- *FilterBar* — Operator (multiselect of the real 14), Backend (DenseByte,
  SparseBitSlice), Instance (G1…G63, 23 real), Campaign/Generation, Score better-
  than-baseline toggle, sweeps range, temp range. Chips summarize; all → URL.
- *DataTable* (virtualized) — columns: id · instance · operator-sequence · backend
  · score · baseline · Δimprovement (color: green<0) · sweeps · seed · campaign/gen
  · ts. Sortable, sticky header, `j/k` traversal, row → inspector.
- *Distributions tab* — histogram of Δimprovement (overall + per-operator small
  multiples); scatter score-vs-n; "improvement by operator" bar (from rollups).
- *Inspector (selected record)* — full provenance KV; the operator sequence as a
  chain; **Reproduce recipe**: the exact deterministic `research_platform`
  invocation (seed + instance + sequence + sweeps/temps) in a copyable CodeBlock —
  this is the "reproduce-on-paper" action (REAL; determinism guarantees it).

**Interactions.** Multi-filter compose; "compare" pins up to 3 records side-by-side;
column menu to add/remove fields; export current view → CSV (client-side).

**Keyboard.** `/` focus filter; `j/k` rows; `Enter` inspect; `x` pin; `c` copy
reproduce cmd; `e` export.

**Context menu (row).** Inspect · Pin to compare · Filter to this operator/instance
· Copy reproduce command · Copy record JSON · Open supporting facts in Graph.

**States.** Loading skeleton rows; Empty "0 of N records match — reset filters";
Error per-file; a persistent note: "table shows a stratified 5k sample of 110,743
records; charts use full aggregates" (honest sampling disclosure).

---

### 7.3 Knowledge Graph — `/graph` — REAL

**Purpose.** Traverse what the engine believes, as typed evidence, and pivot to the
runs that justify it.

**Repo mapping.**
- Reads: `knowledge_graph.txt` (~300 real facts: `subj|rel|obj||weight|support|
  variance|"hypothesis #N (conf X)"`).
- Structs/modules: `KnowledgeGraph` (`graph.rs`); facts authored by
  `meta_learner`/`theory`/`orchestrator`.
- CLI replaced: reading `knowledge_graph.txt` by hand.
- Future endpoint: `GET /api/v1/graph?rel=…&node=…`.

**Layout.** Center GraphCanvas · left relation/entity filters · right Inspector
(selected node or edge).

**Panels & widgets.**
- *GraphCanvas* — nodes = operators/structure-classes (dense/sparse/…); edges =
  relations (`fails-on` red, `effective-on` green, `precedes-well` blue). Edge
  thickness = support; opacity = confidence. Hover lifts incident edges; travelling
  dots suggest accruing support (motion = telemetry). Force layout with pinning;
  arc-diagram alt view for dense hubs.
- *Filters* — relation type, min confidence, min support, entity search.
- *Inspector (edge)* — the full fact: subject → relation → object, weight, support,
  variance, confidence, source hypothesis #; **"Show supporting runs"** → deep-links
  to Experiments filtered to that operator/structure (bridges belief → evidence).
- *Inspector (node)* — all incident facts grouped by relation; the operator's
  discovered profile.

**Interactions.** Click node to focus its neighborhood; double-click to isolate;
relation legend toggles; confidence slider re-weights opacity live.

**Keyboard.** `f` focus search; `1/2/3` toggle relation types; `Esc` clear focus.

**Context menu (node/edge).** Focus neighborhood · Show supporting runs · Copy fact
· Filter to relation · Open operator page.

**States.** Loading → node skeleton; Empty (filtered out) → "no facts at conf ≥ X";
Error → file note.

---

### 7.4 Theory Engine — `/theories` — REAL(recorded) + OFFLINE(investigate)

**Purpose.** Show the Popperian pipeline and its verdicts; (later) launch ablations.

**Repo mapping.**
- Reads: `knowledge_graph.txt` (theory verdicts land here), `report_*.md` /
  `evaluation_report.md` (reproducibility + refutations), ROADMAP honesty ledger.
- Modules/structs: `theory.rs` (rule→MechanismHypothesis→Prediction→ablation→Theory);
  signatures from `StepEvent` (entropy/acceptance/diversity).
- CLI replaced/needed: `research_platform --investigate <op>` (OFFLINE launch).
- Future endpoint: `POST /api/v1/investigate {operator}` (spawns the ablation).

**Layout.** Pipeline rail (5 stages) → Theory cards (confirmed/refuted) → an
"Investigate" launcher (OFFLINE).

**Panels & widgets.**
- *Pipeline* — Rule→Mechanism→Prediction→Ablation→Theory StageChain, each with the
  real definition.
- *Theory cards* — real examples: *survived* → "thermal exploration dominates
  (n=75, 6 families)"; *refuted·kept* → "single structural scalar → density/ruggedness
  confound"; *refuted·kept* → "extremal_metropolis weak on G-Set (worst-solutions
  8×)". Each shows verdict, support, and the ablation cost where known ("metropolis_sweep
  confirmed causal — ablation costs 5%").
- *Investigate launcher* — operator picker + "Run ablation across instances"
  button, **disabled with OFFLINE badge** and the exact CLI it will call, plus the
  recorded result if one exists.

**Interactions/Keyboard/Context/States** per template; the launcher's disabled
state explains precisely what the control-plane must provide.

---

### 7.5 Concept Evolution — `/concepts` — REAL(narrative) + PLANNED(live)

**Purpose.** Explain and (later) watch the vocabulary grow.

**Repo mapping.** `concept.rs` (`propose→test→gate→admit`; InstanceSignature→f64;
leave-one-instance-out ridge RMSE; Occam gate) + `feature_registry.rs` (shared
versioned vocabulary). No serialized concept stream today → live view PLANNED.
Future: an export of admitted concepts + their OOD gain per generation.

**Panels.** The 4-stage pipeline with the real Occam story (the 11-feature
descriptor refuted — in-sample fit never enough); the current feature vocabulary
(from `feature_registry`, versioned); a PLANNED "concept feed" scrim with the exact
export needed.

---

### 7.6 Campaign Manager — `/campaigns` — REAL(recorded) + OFFLINE(control)

**Purpose.** See recorded campaigns and (later) drive new ones.

**Repo mapping.**
- Reads: DB `campaign_id`/`generation_id` rollups, `campaign_state.txt`,
  `model_registry.txt` (per-generation snapshots).
- Modules: `campaign.rs`, `orchestrator.rs`, `executive.rs`, `planner.rs`.
- CLI replaced: `research_platform --orchestrate N --executive --planner
  --shared-knowledge` / `--service --max-*`.
- Future endpoints: `POST /campaigns`, `POST /campaigns/:id/pause|resume`,
  `GET /campaigns/:id` (SSE).

**Panels & widgets.**
- *Campaign list* — the three real campaigns with run counts, instances, generations.
- *Generation timeline* — per-generation run counts, best-score trace, model
  snapshot markers (from `model_registry.txt`). Recorded.
- *Launcher (OFFLINE)* — a real form: pick instances (G-Set list), generations,
  flags (executive/planner/curiosity/shared-knowledge), budget caps. Renders the
  exact CLI it would run; **Start** disabled with OFFLINE badge + reason. Pause/
  Resume likewise.

**States.** Live tick, log stream, and controls are the canonical OFFLINE surfaces
— each shows the recorded equivalent + the endpoint that will light it up.

---

### 7.7 Runtime & Registries — `/runtime`, `/operators`, `/models`, `/features` — REAL

**Purpose.** Understand the machinery: backends, the 14 operators with their
*discovered* behavior, versioned models, the feature vocabulary.

**Repo mapping.**
- `state.rs` (ReferenceState/SparseBitSlice/DenseByte), `registry.rs` +
  `capability.rs` (operators + passports), `model_registry.txt` (versioned weights
  + lineage), `feature_registry.rs`.
- Reads: DB (per-operator behavior), `report_*.md` + `evaluation_report.md`
  (ordering/budget/antipattern rules per operator), `model_registry.txt`.

**Operator Registry (`/operators`).** Grid of the real 14
(`metropolis_sweep, greedy_descent, gibbs_color_sweep, history_field,
extremal_metropolis, extremal_optimization, replica_exchange, houdayer_cluster,
isoenergetic_cluster, elite_broadcast, population_resample, steepest_descent,
random_flip_sweep, random_restart_worst`). Each card: capability passport +
**discovered behavior** from the evidence — e.g. *gibbs_color_sweep* "works better
AFTER metropolis_sweep (+8.6) than first (−64.8), conf 1.00"; *random_flip_sweep*
"antipattern — in worst solutions 4× more than best, conf 0.84"; budget-scaling
badges ("keeps paying with budget" where supported). Operator page `/operators/:name`
= full profile: incident graph facts, ordering effects, budget curve, the runs
that use it (→ Experiments).

**Model Registry (`/models`).** Real versioned snapshots from `model_registry.txt`:
kind (Predictor/World/Policy/Dynamics), version, parent (lineage tree), when
snapshotted. Predictor page shows its real quality (leave-one-instance-out Spearman
0.747). Internals beyond weights = PLANNED (needs export).

**Runtime & Backends (`/runtime`).** 3 backends with roles (f64 oracle · exact
integer >100k cross-check · DenseByte 4.8×, bit-identical). Live run telemetry
(StepEvent/RunRecord) = PLANNED with the precise export note.

---

### 7.8 Benchmark Center — `/benchmarks` — REAL

**Purpose.** Honest head-to-heads vs external solvers.
**Repo mapping.** `results/index.json` (per-engine mean_best, mean_wall_ms,
head_to_head vs openjij_sa/neal: instances, win_fraction, wilcoxon_p, sign_test_p);
`specs/*.json` (hypothesis + kill-criterion). Future: `GET /api/v1/benchmarks`.
**Panels.** Experiment selector (EXP-0000/0001) → spec card (title, hypothesis,
kill-criterion, status) → per-engine comparison table → H2H panel (win-fraction
gauge, Wilcoxon/sign-test p with an honest "n=3, p=0.25 — underpowered" caveat where
true). No cherry-picking; the kill-criterion is shown next to the result.

---

### 7.9 Dataset Explorer — `/dataset` — REAL

**Purpose.** Foundation-model readiness, honestly.
**Repo mapping.** `dataset/foundation_dataset.tsv`, `foundation_manifest.md`,
`decision_log.tsv`, `trajectories.tsv` (`dataset.rs`).
**Panels.** Manifest readouts (18,570 examples · 23 instances · 14 operators);
**scale-gap table** (500k 27× · 1M 54× · 5M 269× · 20M 1077×) with the verdict
"a training corpus, not a foundation model"; feature schema (structural features →
algorithm → outcome); a sampled preview of rows. The honesty here is the feature.

---

### 7.10 Evaluation & Portability — `/evaluation` — REAL

**Purpose.** Does the platform reproduce its own rules, and do they transfer?
**Repo mapping.** `evaluation_report.md` (rule reproducibility across ≥2 instances;
predictor leave-one-instance-out) + `portability_report.md` (`src/bin/portability.rs`).
**Panels.** Reproducibility table (signature · #instances · mean confidence ·
example) grouped by signature family (Ordering / Budget / Antipattern); a
"reproduced across all 23 instances" highlight; predictor accuracy readout; the
portability matrix (MaxCut-policy → non-MaxCut, 12/15).

---

### 7.11 Research Papers — `/papers` — REAL

**Purpose.** The platform's cumulative written memory as a readable corpus.
**Repo mapping.** `reports/report_{08}.md` + `INDEX.md` (numbered, never rewritten)
+ `deep_analysis_{08}.md` (Claude tier). Rendered markdown.
**Panels.** Left = chronological index (report N at M experiments); right = rendered
report with its best-result line and discovery list; a diff-of-discoveries between
consecutive reports (what the engine newly learned). Deep-analysis entries badged as
the cloud tier.

---

### 7.12 Solver Playground — `/solve` — REAL-live (server_api)

**Purpose.** Actually solve a QUBO from the browser.
**Repo mapping.** `POST /api/v1/solve` on `server_api` (`QuboRequest` → `QuboResponse{
status, energy, state, compute_time_ms}`), semaphore-guarded, core-pinned rayon.
**Panels.** A small QUBO editor (paste triples / pick a bundled sample instance) →
Solve button → real result (energy, compute time, a spin-state visualization).
Connection badge shows whether `server_api` is reachable; if not, an explicit
"start server_api on :PORT" instruction (never a fake result). This is the one
genuinely-live control in the Observatory build.

---

### 7.13 Long-tail workspaces (template applied, condensed)

- **Experiment Builder `/experiments/new` — OFFLINE.** Compose a spec (instance ·
  operator sequence · sweeps/temps · seed) → renders the exact `research_platform`
  CLI; **Build & Run** disabled until control-plane. (Repo: `families`, `frontend`,
  executor; future `POST /experiments`.)
- **Experiment Queue `/queue` — PLANNED** (no queue serialized; needs export).
- **Scheduler `/scheduler` — PLANNED** (`scheduler.rs` in-memory).
- **Runtime Monitor `/runtime/monitor` — PLANNED** (needs `StepEvent`/`RunRecord`
  export — the highest-value new seam).
- **Scientific Memory `/memory` — PLANNED** (`memory_os` recall in-memory).
- **Operator Proposals `/proposals` — REAL** (`operator_proposal_*.md` drafts).
- **API Explorer `/api` — REAL(/health,/solve)+OFFLINE** (documents the endpoint
  surface; live-tests `/solve`).
- **Logs `/logs` — OFFLINE** (needs SSE log stream).
- **Health Monitor `/health` — PLANNED** (`monitor.rs` report in-memory).
- **Documentation `/docs` — REAL** (Constitution → SOUL → ADRs → ROADMAP; governance
  order; links to research/*).
- **Settings `/settings` — REAL(local)** (theme, density, data-dir label,
  reduced-motion, control-plane URL for the future).

---

## 8. Feature classification matrix

| Workspace / feature | Class | Backing (real artifact / endpoint) | To upgrade |
|---|---|---|---|
| Mission Control readouts & ledger | REAL | DB aggregates, reports, ROADMAP ledger | — |
| Loop ring (position) | REAL(recorded) | `campaign_state.txt` | SSE `loop/status` → live |
| Experiments table & filters | REAL | `ai_experiments.txt` (sample+rollups) | server paging endpoint for full 110k |
| Reproduce recipe | REAL | determinism (seed+sequence) | — (copy-only; execution is OFFLINE) |
| Knowledge Graph | REAL | `knowledge_graph.txt` | endpoint for incremental facts |
| Theory verdicts | REAL | graph + reports + evaluation | — |
| Investigate launcher | OFFLINE | — | `POST /investigate` |
| Concept narrative + vocabulary | REAL | `concept.rs`, `feature_registry` | — |
| Concept live feed | PLANNED | — | admitted-concept export |
| Campaign timeline | REAL(recorded) | DB campaign/gen + `model_registry.txt` | — |
| Campaign start/pause/resume | OFFLINE | — | `POST /campaigns…` |
| Operators (14) + discovered behavior | REAL | DB, reports, evaluation | — |
| Model Registry (versions+lineage) | REAL | `model_registry.txt` | — |
| Model internals (weights viz) | REAL(partial) | `model_registry.txt` weights | richer export |
| Benchmark Center | REAL | `results/index.json`, `specs/*.json` | — |
| Dataset Explorer + scale-gap | REAL | `dataset/*.tsv`, manifest | — |
| Evaluation & Portability | REAL | `evaluation_report.md`, `portability_report.md` | — |
| Research Papers | REAL | `reports/*.md`, `deep_analysis_*.md` | — |
| Operator Proposals | REAL | `operator_proposal_*.md` | — |
| Solver Playground | REAL-live | `server_api /api/v1/solve` | — |
| Runtime Monitor (live telemetry) | PLANNED | — | `StepEvent`/`RunRecord` export |
| Scientific Memory recall | PLANNED | — | memory_os export |
| Queue / Scheduler / Health | PLANNED | — | respective exports |
| Logs stream | OFFLINE | — | SSE `logs` |
| Documentation / Settings | REAL | `research/*`, localStorage | — |

**Honesty invariant:** every OFFLINE/PLANNED surface renders (a) a status badge,
(b) the recorded data behind it where any exists, and (c) the precise export or
endpoint that upgrades it. No surface is blank, and none pretends to be live.

---

## 9. Brutal self-critique → redesign (iterated)

**Round 1 — attack.**
1. *"A 25-workspace nav is overwhelming; it looks like a settings screen, not an
   instrument."* → **Redesign:** 6 semantic groups + collapse-to-icons; Mission
   Control is the only default; the command palette is the real navigation for
   power users. Progressive disclosure: OFFLINE/PLANNED items are visible but
   dimmed with dots, so the map is honest without demanding attention.
2. *"Static real data will feel dead — the mandate is 'watch it think.'"* →
   **Redesign:** motion-as-telemetry on recorded structure (loop ring position from
   `campaign_state`, accruing-support dots on graph edges sized by real support),
   plus the **one genuinely live** surface (Solver Playground) as proof the seam is
   real. The Cockpit phase makes the rest live; we never fake it in between.
3. *"110k rows: either the bundle explodes or the table lies about completeness."*
   → **Redesign:** ingest emits aggregates + a **stratified 5k sample** with a
   permanent, visible disclosure ("5k of 110,743"); full-fidelity querying is an
   explicit endpoint in the Cockpit phase, not a fake now.
4. *"OFFLINE everywhere reads as vaporware to a skeptical reviewer."* → **Redesign:**
   lead with REAL depth (Evidence + Knowledge groups are almost entirely REAL and
   genuinely deep); OFFLINE is confined to control actions, each showing the exact
   CLI it wraps — which reads as *engineering plan*, not vapor, because the CLI
   provably exists.
5. *"Provenance tags everywhere = clutter."* → **Redesign:** provenance is a single
   quiet chip per panel, expandable; not repeated per row.

**Round 2 — attack the redesign.**
6. *"The graph with ~300 facts and force layout will hairball on dense hubs."* →
   **Redesign:** default to a **relation-filtered** view (one relation at a time)
   + arc-diagram alt; force layout only within a focused neighborhood.
7. *"Reproduce-recipe implies you can run it — a tease."* → **Redesign:** label it
   "Reproduce (copy command)" with a note "execution arrives with the control-plane";
   the value (a citable, deterministic recipe) is real and complete on its own.
8. *"Papers as rendered markdown is just a file viewer."* → **Redesign:** add the
   **diff-of-discoveries** between consecutive reports (what was newly learned) and
   cross-links from each discovery line into Graph/Experiments — turning a file
   viewer into an evidence-navigator.
9. *"Benchmarks with n=3, p=0.25 could look like a strong claim."* → **Redesign:**
   surface the kill-criterion and an explicit "underpowered (n=3)" caveat beside
   every H2H; honesty becomes a credibility feature, not a weakness.
10. *"Two themes (earlier light lab) will rot."* → **Redesign:** one dark instrument
    theme, token-driven; the earlier light components are replaced, not maintained
    in parallel.

**Round 3 — architectural stress.**
11. *"URL-as-state + Next App Router + heavy client viz can hurt TTFB/Lighthouse."*
    → **Redesign:** RSC for data-heavy shells, client islands only for
    canvas/interactive viz, code-split per workspace, no heavy WebGL.
12. *"Ingest coupling to `experiments/` paths breaks builds elsewhere."* →
    **Redesign:** `$ISING_DATA_DIR` + committed tiny fixture fallback; ingest
    asserts row-counts and fails loudly in CI but never silently ships wrong data.
13. *"Scaling to millions of nodes (Phase 7) won't fit the ingest model."* →
    **Redesign:** ingest is Observatory-only; the Cockpit's API is
    paginated/streamed by contract, so the client is already written against
    cursors, not arrays.

**Exit criterion:** further attacks now target Cockpit-phase concerns (auth,
realtime backpressure, multi-user merge) that are explicitly out of this build's
scope and designed-for in §11. The Observatory design is stable.

---

## 10. Implementation plan & verification protocol

**Phasing (incremental; each phase gates on the checklist below).**
- **P0 — Foundation.** Dark `--os-*` tokens + materials in `globals.css`; the
  `OSShell` (rail groups, top bar, status bar, workspace outlet); routing + URL
  state; component atoms; connection-state + REAL/OFFLINE/PLANNED badges.
- **P1 — Data layer.** `scripts/ingest.mjs` (reads `$ISING_DATA_DIR`, decodes the
  21-field schema, emits aggregates + stratified 5k sample + full graph facts +
  benchmark/specs/manifest verbatim + `provenance.json`); `src/data` typed
  accessors; fixture fallback; a validation step asserting row-counts vs real files.
- **P2 — Evidence & Knowledge (REAL depth first).** Mission Control · Experiments
  Explorer · Knowledge Graph · Benchmark Center · Evaluation · Dataset · Papers.
- **P3 — Machinery.** Operators · Runtime & Backends · Model Registry · Feature
  Registry · Theory Engine · Concept Evolution · Operator Proposals.
- **P4 — Operate + global surfaces.** Campaign Manager (recorded + OFFLINE launcher)
  · Experiment Builder · Solver Playground (REAL-live) · API Explorer · Docs ·
  Settings · `⌘K` command palette · remaining PLANNED scaffolds with go-live notes.

**Verification checklist (run after every phase — non-negotiable).**
1. `npm run lint` → zero errors.
2. `npm run build` → zero errors (typecheck included).
3. Data validation → ingest row-counts match `experiments/` (or fixture) exactly.
4. Browser verification via the `run-ising-website` driver (headless Playwright).
5. Screenshot verification → capture each new/changed workspace; **inspect** them.
6. Self-critique → note regressions vs this spec; fix before proceeding.
7. Never continue while a check is red.

**Definition of done (Observatory / this build).** All REAL workspaces render real
ingested data with provenance; all OFFLINE/PLANNED surfaces show badge + recorded
data + go-live note; Solver Playground solves live against `server_api`; lint/build
green; Lighthouse ≥95; reduced-motion clean; every view deep-linkable.

---

## 11. Future architecture (Cockpit & beyond — designed now, built later)

- **Control-plane** `src/bin/control_api.rs` (extends the axum pattern already in
  `server_api`): read endpoints that parse the on-disk artifacts (or gain serde-JSON
  accessors); `POST /campaigns` spawns `research_platform --service/--orchestrate` as
  a supervised child; `pause/resume`; `POST /investigate`; `GET /logs` (SSE);
  `POST /solve` reuses the existing `compute()` path unchanged.
- **New Rust export seam (highest value):** serialize the runtime **`StepEvent`/
  `RunRecord`/`QualityMetrics`** (in-memory today) → the live Runtime Monitor and the
  animated loop. Additional exports upgrade Memory/Queue/Scheduler/Health/Concept
  from PLANNED to REAL.
- **Scale:** the API is cursor/stream-based by contract; the client is written
  against pagination + SSE/WebSocket from day one, so 110k→millions never lands in
  the browser as an array.
- **Provenance & replay:** determinism makes **replay-as-action** a first-class verb
  once the control-plane exists (re-run any record by its seed+recipe).
- **Versioning & collaboration:** Model/Feature registries already version; theories/
  concepts are append-only → the UI surfaces lineage now; multi-user annotation,
  auth, and remote/cloud workers are a later realtime layer (trust boundary = the
  API, local-bind + token before any remote control).

---

## 12. Open decisions for the reviewer

1. **Ingest data source:** point `$ISING_DATA_DIR` at `experiments/platform_gset`
   (18,570 · clean · matches the manifest) as the primary, or union all three
   campaigns (~110k, richer but mixed provenance)? *Recommendation: gset primary +
   an all-campaigns aggregate on Mission Control, clearly labelled per source.*
2. **Build the control-plane (`control_api`) in a later increment**, or keep this
   build strictly Observatory (no Rust changes)? *Recommendation: Observatory now;
   `control_api` as a separate, explicitly-scoped follow-up.*
3. **Scope of first implementation pass:** all of P0–P4, or P0–P2 (Foundation +
   Evidence/Knowledge) to a verified finish first, then continue? *Recommendation:
   P0–P2 first — the highest-REAL-density surfaces — then P3–P4.*

*End of specification. Awaiting approval before any implementation.*
