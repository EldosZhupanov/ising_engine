# Ising Engine Laboratory — Architecture Review & OS Roadmap
### Brutal review of the current build + the plan to turn a viewer into an operating system

> Written after first-hand investigation of the repository and **live probing of the
> local runtime environment**. Every capability claim below is verified, not assumed.
> Where I could not verify something, it says so.

---

## 0. Verified environment findings (these change the plan)

These were probed live on this machine during the review:

| Finding | Evidence | Consequence |
|---|---|---|
| **Ollama is installed and running** | `/usr/local/bin/ollama`, `ollama serve` pid 145, `/api/version` → `0.31.2` | The LLM tier is not hypothetical |
| **`qwen2.5-coder:7b` present** | `/api/tags`: 4.68 GB, `7.6B` params, `Q4_K_M`, `context_length: 32768`, `capabilities:[completion,tools,insert]` | Rich model metadata is available |
| **Ollama allows browser origins** | `curl -H "Origin: http://localhost:3000" /api/tags` → `Access-Control-Allow-Origin: http://localhost:3000` | **The browser can drive the LLM directly — no proxy, no Rust work** |
| **Live VRAM + keep-alive is exposed** | `/api/ps` after load → `size_vram: 4,185,758,105`, `expires_at`, `ctx: 4096` | Real VRAM/stop/TTL telemetry, browser-reachable |
| **Token rate is derivable and real** | `/api/generate` → `eval_count:2`, `eval_duration:203,803,000` ns ⇒ **9.8 tok/s**; `load_duration: 75.3 s` (cold 4.7 GB load) | tok/s, TTFT, load time are all measurable |
| **GPU is real** | `nvidia-smi`: RTX 4050 Laptop, 6141 MiB total; **4091 MiB used** while the model was loaded | GPU panels have a real source… |
| **…but `nvidia-smi` is shell-only** | not an HTTP surface | GPU% needs a small local endpoint; **VRAM can come from Ollama instead (browser-reachable)** |
| **`llm.rs` already speaks this API** | `OllamaClient{command:"curl", host:"http://localhost:11434", model, timeout:180s}` → `POST /api/generate {"stream":false}` | The UI and the engine can share one model registry/config |
| **`cloud.rs` = Anthropic Messages** | `https://api.anthropic.com/v1/messages`, `ANTHROPIC_API_KEY`, `max_tokens:2000`, cadence "every N experiments", honest `available()` skip | Cloud tier exists; multi-provider does not |
| **No OpenAI-compatible / OpenRouter support** | repo-wide grep for `openai\|openrouter\|/v1/chat/completions` → **0 hits** | Multi-provider = new Rust work (or browser-side) |
| **No GPU/VRAM/tok-s monitoring in Rust** | grep `nvidia-smi\|vram\|tokens_per_sec` → **0 hits** | Entirely new capability |
| **`proposal.rs` writes real discovery artifacts** | `OperatorProposal{name, idea, math, pseudocode, expected_properties, required_capabilities, expected_complexity, provenance}` → `operator_proposal_*.md`; real file `metropolis_gibbs` with provenance *"meta-learner gap: metropolis_sweep → gibbs_color_sweep adjacent 1692x in top solutions"* | **The Discovery Center has real content today** |
| **`executive.rs` is an auditable reasoning chain** | "*the LLM proposes; the executive decides*"; measured state: `predictor_trained_at`, `world_trained_at`, `cloud_available`, `regime_saturation`, `theory_publish_conf` | Reasoning-chain inspector is grounded, not invented |

**Single most important conclusion:** the LLM cockpit — model choice, pull/download, load/stop, VRAM, context, tok/s, live streaming reasoning — is **implementable immediately, entirely in the browser, with zero Rust changes**. It is the highest value-to-effort item in the entire brief.

---

## 1. Architecture review — what is actually wrong

I built the current version. I am reviewing it as if a stranger did.

### 1.1 The fundamental failure: it is a museum, not a cockpit

Quantified: of the ~40 verbs in your brief, the current build supports **4** (browse experiments, inspect graph, read theories, read papers). It supports **zero** mutations — there is not one `POST`, not one write, not one action that changes platform state. `grep` for form submission, mutation, or fetch in `src/` returns nothing but navigation.

The interface is a **read-only projection of a frozen build-time snapshot**. Everything you actually want — create, run, stop, pause, resume, fork, replay, train, export — is absent. Calling this an "operating system" is currently false advertising. Your critique is correct and it is the defining problem.

### 1.2 Half the OS is an empty promise

18 routes; **9 are honest stubs** (Memory, Operators, Runtime, Models, Features, Campaigns, Solve, Docs, Settings). The status dots are honest, but a nav that is 50% "PLANNED" teaches the user that the product is mostly aspiration. Worse, two are *mis*classified:
- **Solver Playground = OFFLINE** — wrong, pessimistic. `server_api /api/v1/solve` exists and works; this could be genuinely live.
- **Campaign Manager = OFFLINE stub** — the spec promised the *recorded* timeline as REAL. It shipped as an empty stub, so a real capability is hidden behind a placeholder.

### 1.3 The LLM — the platform's own scientist — is entirely invisible

The engine has two LLM tiers (`llm.rs` local Qwen ideator, `cloud.rs` Claude deep-analysis). The interface shows **neither**. There is no model picker, no reasoning stream, no proposal feed, no cost/latency view, no cloud-skip indicator. Meanwhile Ollama is *running on this machine and reachable from the browser*. This is the biggest single omission, and it is an unforced error.

### 1.4 No theme engine

Dark only, with `--os-*` hardcoded in `:root`. No light, no OLED, no high-contrast, no **print** — and print matters here, because this platform generates *research papers*; a scientist will want to print/PDF a report and currently cannot without a dark-ink disaster. There is no `data-theme` switching layer, no persistence, no `prefers-color-scheme` respect. Adding themes later will require touching every component that hardcodes a token — the debt compounds daily.

### 1.5 Motion is thin, and the 3D stack is dead weight

Implemented motion: a loop ring, a histogram grow, count-ups, hover lifts. That is it. No depth, no camera, no lattice, no field lines, no knowledge propagation, no theory evolution, no memory crystallization.

Worse: **`three`, `@react-three/fiber`, and `@react-three/drei` are installed and completely unused.** Dead dependencies inflating the tree. Either use them deliberately or remove them; shipping unused 3D libraries while claiming a flat UI is the worst of both.

### 1.6 Zero onboarding — a stranger is lost in 5 seconds

No tour, no first-run experience, no example campaign, no tutorial, no cheatsheet, no contextual help, no glossary. The domain is *hard* (Ising models, operators, ablation, Occam gating, Welford confidence) and the UI explains none of it in situ. The command palette only navigates 12 items — it cannot act, cannot search entities, and has no aliases. There is no `?` shortcut, no discoverability of the shortcuts that do exist. A newcomer cannot form a mental model, so they will never become a user.

### 1.7 Concrete correctness / honesty bugs found while reviewing

These are real defects, not aesthetics:

1. **No mobile navigation at all.** `Rail` is `hidden lg:flex` and the OS build has no replacement (the old lab had a mobile row; I removed it). Below 1024 px the only nav is a ⌘K button that needs a physical keyboard. On a phone or tablet the app is **a dead end**.
2. **Inconsistent totals with no labelling.** Mission Control says **109,758 experiments**; Experiments says "full DB **18,570** records". Both are true (all-campaigns vs. gset-primary) but nothing says so on either screen. A reviewer will read this as a contradiction — and in an honesty-first product that is the most expensive kind of bug.
3. **122 KB of shipped, never-rendered markdown.** `papers.json` carries each report's full `markdown`; the Papers page renders only the parsed discovery lines. Pure payload waste and a broken promise (the spec said rendered report + diff-of-discoveries).
4. **Silent histogram cap.** `ingest.mjs` stops collecting improvements at 20,000 (`if (agg.improvements.length < 20000)`). For gset (18,570) it is currently accurate — but it is an undisclosed cap that will silently start lying the moment the primary campaign exceeds 20k rows. Silent truncation in a truth-over-optimism product is unacceptable.
5. **Spec features silently dropped.** Promised and not built: the record **Inspector** drawer, the **"Reproduce (copy command)"** recipe (the single most valuable honest action available today), **"Show supporting runs"** graph→evidence pivot, compare/pin, CSV export, context menus, `j/k` traversal.
6. **Ingest loads whole files into memory** (`readFileSync().split("\n")`). Fine at 110k; it will not survive the "millions" the spec claims to design for. The scalability claim is currently unearned.
7. **Experiments re-sorts the full sample per request** with no virtualization, capped at 200 displayed rows. Every filter click re-renders server-side. It works at 3,913 rows and will not scale.

### 1.8 Accessibility is unvalidated and partly broken

No skip-link. No landmark audit. Focus-visible styling never customized on a dark ground (default rings are near-invisible against `#07090C`). The knowledge graph is **mouse-only** — SVG nodes/edges have no `tabindex`, no keyboard activation, no accessible names, so the flagship visualization is unusable without a mouse. Tables lack `<caption>`/`scope`. Contrast of `--os-faint` (`#59657A`) on `--os-ground` (`#07090C`) is around 4.0:1 — **below WCAG AA for small text**, and it is used for exactly that. Claiming "accessible" in the spec while shipping this is not defensible.

### 1.9 Data architecture is stale-by-construction

The build-time snapshot means the UI is a photograph. There is no refresh, no tail, no invalidation, no "data as of" freshness warning beyond an mtime chip. If the engine runs a campaign right now, the interface will not notice until someone re-runs `npm run build`. For a platform whose whole point is a *continuously running* autonomous loop, that is the wrong architecture for anything beyond the Observatory phase.

### 1.10 Engineering hygiene gaps

No website tests (zero). No standalone `typecheck` script. No CI. No error boundaries. No `not-found`/`error` route handlers. `Skeleton` was specified but never built, so loading states don't exist. No bundle budget. No Lighthouse measurement (the ≥95 target was asserted, never measured).

### 1.11 What is genuinely good (keep it)

Being fair to the parts that work: the **real-data ingestion with exact validation** (109,758 / 18,570 / 14 operators / 64 facts — all verified against source files); the **REAL/OFFLINE/PLANNED honesty system**; **provenance chips**; the **kill-criterion + "underpowered (n=3)" caveat** on benchmarks (genuinely excellent — it is the single most credibility-building element in the product); the **token-based design system**; RSC/server-side data so heavy payloads stay off the client; the committed **Playwright driver** for verification.

---

## 2. Everything still missing (by subsystem)

**Control (the OS itself):** create/edit/duplicate campaign · start/stop/pause/resume · create/edit/run experiment · cancel · fork · replay · batch/queue submit · budget caps · priority · process supervision · cancellation semantics.

**LLM cockpit:** provider registry (Ollama/Anthropic/OpenAI-compatible/OpenRouter) · model list/pull-with-progress/delete · load/unload/keep-alive · VRAM/RAM/GPU% · context window vs. used · tok/s · TTFT · queue depth · live reasoning stream · prompt/response inspector · cost accounting · rate limits · per-tier routing policy (which questions go local vs. cloud) · LLM-initiated experiments feed · proposals feed · accepted/rejected discoveries w/ reasons · confidence.

**Discovery & meaning:** Discovery Center (what changed / why interesting / evidence / confidence) · Application Explorer (domain mapping w/ confidence + transfer evidence + unknowns) · operator lineage/evolution tree · theory versioning & diffs · concept admission history · "resembles simulated annealing/tabu" similarity analysis · open-questions board.

**Observability:** live logs (SSE) · runtime metrics (StepEvent/RunRecord/QualityMetrics) · energy-landscape visualization · trajectory replay scrubber · health gates · scheduler state · queue state · planner agenda · curiosity scores · executive decision log w/ measured reasons · memory recall browser.

**Models:** training trigger · evaluation · lineage graph (registry has it) · weight inspection · per-family transfer matrix · staleness alerts · promote/rollback.

**Data:** dataset builder · export (CSV/TSV/Parquet/JSON) · saved views · shareable citations · annotations/notes · comparison workspace (runs, benchmarks, theories side-by-side).

**Platform UX:** modes (Explorer/Scientist/Research/Engineering/Debug/Executive/Presentation) · theme engine (8 handcrafted) · onboarding & missions · command palette that *acts* · global entity search · inspector drawers · context menus · keyboard system · notifications/toasts · settings persistence · multi-pane layouts · undo.

**Motion:** 3D camera/depth · glass refraction · volumetric light · parallax panels · procedural particles · energy waves · knowledge pulses · graph propagation · theory evolution · operator activation · memory crystallization · **interactive Ising lattice** (spin flips, temperature slider, magnetization/field lines, phase transition) · quantum-state transitions.

---

## 3. Roadmap — four independent tracks, ordered by value ÷ effort

The critical insight: **"liveness" is not one project.** It splits into four tracks with radically different costs. Track A and B need **no Rust at all**; Track D is the expensive one.

### TRACK A — LLM Cockpit · *live today, zero Rust* · **DO FIRST**
Verified reachable from the browser. Real, live, mutating — the first true "OS" surface.
- **A1** Provider registry + settings (Ollama local first; Anthropic/OpenAI-compatible/OpenRouter via user-supplied keys, browser-side, never persisted server-side).
- **A2** Model manager: list (`/api/tags`), **pull with streaming progress** (`/api/pull`), delete, show (`/api/show`).
- **A3** Load/stop + keep-alive TTL (`/api/generate` w/ `keep_alive`, `/api/ps`).
- **A4** Live telemetry: VRAM (`size_vram`), context (`ctx` vs `context_length`), **tok/s + TTFT + load time** (derived from `eval_count`/`eval_duration`/`load_duration`), queue depth.
- **A5** **Reasoning console** — streaming (`"stream":true`) chat/generate against the *engine's own prompts* (graph + report digest), so you watch the platform's scientist think, in the browser.
- **A6** Proposal/discovery feed w/ accept-reject + reasons (reads `operator_proposal_*.md` now; writes need Track D).
- *GPU% caveat:* `nvidia-smi` is shell-only → GPU utilization needs a tiny endpoint (Track C). **VRAM does not** — Ollama reports it.

### TRACK B — Browser-native platform UX · *no backend* · **DO SECOND**
Everything that makes it feel like an OS and teaches the user, computable client-side.
- **B1 Theme engine** — 8 handcrafted themes (Dark, Light, Scientific White, OLED, Night, Presentation, Print, High Contrast) via `data-theme` + full token indirection; fix the AA contrast failures; persist; respect `prefers-color-scheme`.
- **B2 Onboarding** — first-run tour, mission system, guided tutorial on *real* recorded data, contextual help, glossary, `?` cheatsheet, empty-state teaching.
- **B3 Command palette v2** — acts, not just navigates: search real entities (operators, instances, facts, papers, records), run actions, copy CLI, switch theme/mode, recent/aliases.
- **B4 Modes** — Explorer/Scientist/Research/Engineering/Debug/Executive/Presentation, each changing layout, density, and available tools.
- **B5 Discovery Center + Application Explorer** — built on the *real* `operator_proposal_*.md` (idea/math/pseudocode/properties/complexity/provenance) + graph facts + transfer evidence. Domain mapping shown **only** with confidence, reason, transfer evidence, and explicit unknowns — hedged, never hyped.
- **B6 Inspectors, context menus, compare/pin, saved views, CSV export, keyboard system, `Reproduce (copy command)`** — the dropped spec features, including the deterministic reproduce recipe.
- **B7 Interactive Ising lattice + 3D motion system** — the flagship: real spin dynamics, temperature slider, magnetization readout, field lines, phase transition; plus depth/parallax/glass/pulses. Use the already-installed `three`/R3F **or remove them**. GPU-friendly, reduced-motion-safe, every animation carrying meaning.
- **B8 Fix the 1.7 bugs** — mobile nav, totals labelling, drop unused markdown from payload (or render it), disclose/remove the histogram cap, virtualize the table.
- **B9 Hygiene** — typecheck script, error boundaries, `not-found`/`error`, skeletons, a11y pass (keyboard-navigable graph, focus rings, contrast), Playwright assertions, measured Lighthouse.

### TRACK C — Thin local bridge · *small Rust or Node* · **DO THIRD**
One tiny read-only local service unlocks the things the browser physically cannot reach.
- **C1** `GET /api/system` → GPU% / RAM / CPU (wraps `nvidia-smi`, `/proc`).
- **C2** `GET /api/artifacts?path=…` + `watch` → read the *current* on-disk artifacts and **tail** them (SSE), replacing the frozen build-time snapshot with live-refreshing real data. This single endpoint kills the staleness problem for every existing REAL workspace.
- **C3** `POST /api/solve` proxy → make **Solver Playground genuinely live** (reuses the existing `server_api compute()` unchanged).
- Local-bind + token by default; the trust boundary starts here.

### TRACK D — Engine control plane · *substantial Rust* · **THE REAL OS**
This is what actually makes the terminal optional. It cannot be faked and must not be.
- **D1 `control_api.rs`** (axum, extends `server_api` pattern): supervised child-process launch of `research_platform` (`--orchestrate`, `--service`, `--campaigns`, `--investigate`, `--family`, `--dataset`), with budget caps (`--max-ticks/-experiments/-wall-secs`), cancellation, and status.
- **D2 Lifecycle semantics** — start/stop/pause/resume mapped onto what the engine actually supports: `ServiceBudget` caps + `campaign_state.txt` cursor + append-only DB ⇒ resume is natural, pause is a budget stop. Queue + scheduler on top.
- **D3 Runtime event-log export** — serialize `StepEvent`/`RunRecord`/`QualityMetrics` (in-memory today). Unlocks live Runtime Monitor, energy-landscape viz, trajectory scrubber. *Highest-value single Rust change in the repo.*
- **D4 Replay & Fork** — the killer feature, and **only possible because of ADR-0004 determinism**: replay any of 109,758 records bit-identically from its seed; fork it by mutating one parameter and enqueue the child with lineage. No other platform of this type can offer exact replay. This should be the product's signature verb.
- **D5 Export seams** → upgrade PLANNED→REAL: `memory_os` recall, `planner` agenda, `curiosity`/`novelty` scores, `monitor` health, `concept` admissions, `meta_layer` consensus, executive decision log.
- **D6 Model ops** — train/evaluate/promote/rollback over `model_registry.txt` lineage.
- **D7 LLM provider abstraction in Rust** — generalize `llm.rs`/`cloud.rs` behind one trait (Ollama / Anthropic / OpenAI-compatible / OpenRouter) so engine and UI share one provider config.

### Honest caveat on "never touch the terminal again"
There is an irreducible bootstrap: *something* must start `ollama serve`, the control plane, and the web server. Full terminal independence needs a **launcher** (a single `ising up` script, a desktop/Tauri shell, or a systemd/user service) plus in-browser process supervision. I will not pretend a static page can bootstrap its own backend.

---

## 4. Priority order (recommended execution sequence)

1. **A1–A5** LLM cockpit live (real mutations, zero Rust) — *the fastest path to "this is an OS"*
2. **B1 + B8 + B9** Theme engine + the correctness/a11y bugs — *stop the debt compounding*
3. **B2 + B3** Onboarding + acting command palette — *make it usable by a stranger*
4. **B5** Discovery Center + Application Explorer — *answer "why does this exist"*
5. **C1–C3** Local bridge: live artifacts, system metrics, live solve — *kill staleness*
6. **B4 + B6** Modes + inspectors/compare/reproduce — *make it a workbench*
7. **B7** Ising lattice + 3D motion — *the signature experience*
8. **D1–D2** Control plane: campaigns start/stop/pause/resume — *terminal becomes optional*
9. **D3–D4** Runtime telemetry + **Replay/Fork** — *the signature capability*
10. **D5–D7** Remaining exports, model ops, provider abstraction — *close the PLANNED list*

---

## 5. Answers to your 15 questions, condensed

1. **Architecture review** → §1. 2. **Missing** → §2. 3. **UX mistakes** → §1.1–1.8. 4. **Missing scientific workflows** → create/run/replay/fork/train/dataset-build/compare/annotate; §2. 5. **Missing interactions** → §2 "Platform UX" + §1.7.5. 6. **Missing subsystems** → §2. 7. **Missing animations** → §2 "Motion". 8. **Missing onboarding** → §1.6, B2. 9. **Missing AI capabilities** → §1.3, Track A. 10. **Roadmap** → §3. 11. **Priority** → §4.
12. **Implementable immediately (no backend):** all of Track A (LLM cockpit — *verified live*), all of Track B (themes, onboarding, palette, modes, Discovery Center, Application Explorer, inspectors, reproduce-recipe, Ising lattice + 3D motion, bug/a11y fixes).
13. **Requires Rust:** Track D — control plane, lifecycle, **runtime event-log export**, replay/fork, PLANNED-subsystem exports, model ops, provider trait. (Track C is a *thin* bridge — small Rust or Node.)
14. **Requires new APIs:** `control_api` (campaign/experiment lifecycle, investigate, dataset), `/api/artifacts` + SSE tail, `/api/system`, `/api/logs` SSE, runtime-telemetry stream, model-ops endpoints.
15. **Requires local LLM integration:** provider registry, model manager (pull/load/stop), telemetry (VRAM/ctx/tok-s), streaming reasoning console, proposal accept/reject, tier-routing policy — *all of the read/monitor/stream side is available today via Ollama's CORS-enabled HTTP API; only writing decisions back into the engine needs Track D.*

---

## 6. Recommendation

Do **Track A first**. It is the only part of your brief that is simultaneously the most impressive, the most "operating system", and free of backend work — and I verified the API, the CORS header, the VRAM field, and the token rate on this machine during this review. Pair it with **B1/B8/B9** so the foundation stops accruing debt, then **B2/B3** so a stranger can use it, then the bridge and the control plane.

The end state worth aiming at is not "a nicer dashboard". It is: **a deterministic scientific OS where every one of 109,758 recorded experiments can be replayed bit-identically, forked with one changed parameter, watched live as it runs, explained by an LLM whose reasoning you can see and whose VRAM you can watch — with every claim carrying its evidence, its confidence, and its refutations.** No other platform in this space can offer exact replay. That is the moat, and the interface should be built around it.
