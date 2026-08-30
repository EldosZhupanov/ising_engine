# Ising Engine — Development Operating System

**READ FIRST — every new agent, before anything else.** Read `START_HERE.md`,
then `memory/NOW.md`, then only the task bundle named there. **Do not scan the
repository and do not read every `.md`.** `memory/AUTHORITY.md` resolves which
source governs a question; summaries never override their sources.

This file is how work gets done here. It is the operational layer.

**Governance order (higher outranks lower):**

1. `research/ISING_ENGINE_CONSTITUTION.md` — the Single Source of Truth for
   *direction*. It outranks this file and every blueprint. Align with it first.
2. `SOUL.md` — *why* the project exists. When a rule here and the mission conflict,
   the mission decides what the rule was trying to protect.
3. `research/architecture/ADR/` — architecture decisions and their rationale
   (queryable: `research/architecture/query_graph.py`). Consult before
   architectural work; record new decisions as ADRs.
4. `ROADMAP.md` — status: what exists, what's in flight, what's next.
5. **This file** — the day-to-day rules.

Direction is presumed fixed by the Constitution. Research clarifies details;
direction changes only via its Amendment Procedure (§15) with reproducible
evidence.

---

## 1. Project Architecture

Two systems in one repository, with a hard boundary between them.

```
┌──────────────────────────────────────────────────────────────┐
│  PRODUCTION ENGINE (stable, fast, verified — DO NOT REWRITE)  │
│    src/core       QUBO/HUBO model, Ising types                │
│    src/solver     UltimateSolver (MSC bit-slice + PT + PA)    │
│    src/presolve   QPBO / roof-duality                         │
│    src/benchmark  parsers, stats, TTS, A/B, portability       │
│    Legacy solvers: parallel_tempering, adaptive, cluster,     │
│                    tabu, autopilot  (frozen, independent)     │
└──────────────────────────────────────────────────────────────┘
                         ▲ read-only, energy-exact bridge
                         │  (frontend::qubo_model_to_ir)
┌──────────────────────────────────────────────────────────────┐
│  RESEARCH PLATFORM  (src/engine_v2 — where new work happens)  │
│    Substrate: ProblemIR, SpinState, backends, operator registry│
│               deterministic Runtime + adaptive controllers    │
│    Generation: DecisionEngine, Evolution, capability select   │
│    Knowledge:  ExperimentDb, KnowledgeGraph, MetaLearner      │
│    ai_scientist/: autonomous research and knowledge systems   │
└──────────────────────────────────────────────────────────────┘
```

**The read-only core** — `Runtime`, `Scheduler`, `SpinState`, the three backends,
the Operator API, the Experiment Runner, and the canonical scorer — is the set of
"electric light sources." The research platform only **generates** (hypotheses,
schedules, genomes) and **analyzes**. All computation is delegated to a
`BatchExecutor` that owns the Runtime. Never reach around it.

## 2. All Main Modules

**Production:**
- `src/core/` — problem model, Ising/QUBO types.
- `src/solver/ultimate.rs` — `UltimateSolver`, the production solver and the
  arbiter of every production performance claim.
- `src/solver/engine.rs`, `types.rs` — engine + quantum state.
- `src/presolve/` — QPBO/roof-duality reductions.
- `src/benchmark/` — `instances.rs` (G-Set/BiqMac/BQP/QPLIB parsers),
  `stats.rs` (Wilcoxon/paired-t/bootstrap), `solver/tts.rs` (TTS 0.99).
- `src/server_api.rs` — must always call `UltimateSolver`; never bypass it.

**Research substrate (`src/engine_v2/`):**
- `frontend.rs` — `qubo_model_to_ir`, `rudy_maxcut_ir` (energy-exact bridges).
- backends: `ReferenceState` (f64 oracle), `SparseBitSlice` (exact integer,
  >100k cross-checks), `DenseByte` (production-shaped, 4.8× vs oracle).
- `registry.rs` — registered operators + capability passports. Query the
  registry when an exact count matters; do not freeze dynamic counts here.
- `Runtime` — deterministic execution + `maybe_adapt` controllers.
- `evolution.rs` — genetic search over operator sequences.
- `families.rs` — `tsp_qubo`/`decode_tsp`, `max2sat_qubo` (proven vs brute force).

**AI Scientist (`src/engine_v2/ai_scientist/`):**
- Data & knowledge: `db` (append-only ExperimentDb), `graph` (KnowledgeGraph),
  `meta_learner`, `meta_layer`, `evaluation`, `reports`, `memory_os`, `dataset`.
- Models: `predictor`, `policy`, `dynamics`, `world`.
- Agents & control: `lab`, `scientist`, `campaign`, `orchestrator`, `planner`,
  `executive`, `curiosity`, `novelty`, `theory`, `proposal`.
- LLM tiers: `llm` (local Ollama), `cloud` (Claude, honest skip).
- Output: `dashboard`, `stats`, `executor`.

**Binaries (`src/bin/`):** `research_platform.rs` is the one entry point for the
platform (`--orchestrate`, `--executive`, `--planner`, `--shared-knowledge`,
`--curiosity`, `--render-dashboard`). Keep every binary target compilable.

## 3. Core Principles

1. **Correctness is never traded for speed.** The decision priority is fixed:
   Correctness → Compilation → Tests → Performance → Readability → Style.
2. **The core is read-only.** Extend by adding operators/agents/analysis above it,
   never by editing the Runtime/Scheduler/scorer to fit a caller.
3. **Determinism and replay.** Same seed → same trajectory → same energy. Every
   adaptive feature is opt-in and bit-identical on replay (ADR-0004).
4. **Select by capability, never by name.** Operators are chosen through their
   capability passport, so new operators are usable the moment they're registered.
5. **Append-only knowledge.** The ExperimentDb and Scientific Memory never
   overwrite or delete. History is evidence.
6. **Honesty over optimism.** Report refutations as first-class results. Skip
   unavailable tiers explicitly; never fabricate a number.
7. **Minimize tokens and file reads.** Search before reading; open the minimum.

## 4. Rules to Avoid (hard "never")

- Never rewrite `UltimateSolver` or any legacy solver; never merge them.
- Never edit `ultimate_patch.rs` (dead code) instead of `ultimate.rs`.
- Never bypass `UltimateSolver` in `server_api.rs`.
- Never make a performance change that alters trajectories unless explicitly
  approved as behavior-changing. FP contraction (`mul_add`), reduced-precision RNG
  compares, and reordered float sums CHANGE trajectories — rejected by default.
- Never claim a performance win without measuring it (see §9).
- Never introduce `unsafe` for micro-optimization; justify every `unsafe`.
- Never add a crate without justification; reuse existing dependencies.
- Never scan the whole repository, reformat unrelated code, or do cosmetic
  rewrites. Never break an unrelated binary target.
- Never fabricate a research result or hide a negative one.

## 5. Coding Rules

- Prefer surgical, focused edits. Preserve compilation on every change.
- Hot paths: prefer iterators, slices, references, packed layouts, cache
  locality. Avoid allocations, temporary `Vec`, `HashMap`, string cloning.
- Prefer `Result`/`Option`/`Iterator`/`match`. Avoid `unwrap()`/`expect()` unless
  failure is truly impossible (then it's fine).
- Borrow, don't clone. Reuse buffers.
- Comments explain **why**, not **what**. Update docs only when behavior changes.
- Match the surrounding code's idiom, naming, and comment density.

## 6. Research Rules

- A result exists only if it is **recorded** (append-only DB with full provenance)
  and **reproducible** (deterministic seed). An unrecorded run didn't happen.
- Every claim carries its evidence and its source. The KnowledgeGraph stores
  *conditional*, evidence-weighted, source-attributed facts — never bare
  assertions.
- A mechanism is not a theory until an **ablation on the Runtime** tried to break
  it and failed (Theory Engine, Popperian). Refutations are kept, not discarded.
- Negative results are results: `extremal_metropolis` is weak, evolved plans lose
  5/5 to `UltimateSolver` — both are recorded knowledge, not swept away.
- No superiority is claimed for the research engine over production. A/B against
  `UltimateSolver` at identical seeds is the only production arbiter.
- Grow the dataset before growing the model. A GNN/Transformer is gated on
  evidence the current features are the bottleneck, not on ambition.

## 7. Documentation

- `research/ISING_ENGINE_CONSTITUTION.md` — direction (SSoT).
- `SOUL.md` — philosophy and mission.
- `ROADMAP.md` — status with per-stage completion %.
- `CLAUDE.md` (this file) — how work is done.
- `research/architecture/ADR/` — architecture decisions (see §12).
- `research/*_BLUEPRINT.md`, `STAGE_7/8_*.md` — design documents per stage.
- Auto-generated context: `INDEX.md`, `.claude/context/{tree,modules,symbols,git}`.
  Prefer these over scanning.
- Update docs only when behavior or direction changes. No redundant docs.

## 8. Development Style / Working Strategy

Solve tasks in this order (stop as soon as you have what you need):

1. `git diff` / `git status`.
2. `INDEX.md`, then `.claude/context/`.
3. `rg` (ripgrep) → `fd` → `ast-grep`.
4. `cargo metadata`.
5. Open only the minimum source files.

Investigation when a bug appears: `git diff` → failing test → related module →
dependencies → implementation. Never randomly inspect files. Search first; never
guess.

## 9. How to Run Experiments, Benchmarks & Reviews

**Build/verify (required before finishing):**
```
cargo check
cargo test                      # affected tests; full suite if public API changed
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

**A platform campaign (the closed scientific loop):**
```
cargo run --release --bin research_platform -- \
    --file benchmark_suite/data/gset/G11 --campaigns 2 --generations 4 \
    [--shared-knowledge] [--curiosity 0.5] [--llm qwen2.5-coder:7b]
```

**The full research lifecycle (Orchestrator):**
```
cargo run --release --bin research_platform -- \
    --file "$(ls benchmark_suite/data/gset/G* | paste -sd,)" \
    --orchestrate 20 --executive --planner --shared-knowledge
```
`--executive` makes the loop obey the Chief Scientist (retrains stale models,
investigates flagged theories, routes ideation to a live Ollama/Qwen).
`--planner` lets it choose its own next instance by expected new knowledge.

**Other run modes of `research_platform` (all opt-in, deterministic):**
- `--family tsp|max2sat [--cities N | --vars N --clauses N] [--seed N]` —
  synthesize a runnable SAT/TSP instance and run the same loop (no rudy file).
- `--service --max-ticks N | --max-experiments N | --max-wall-secs N [--no-planner]`
  — persistent, budget-capped orchestrator loop (24/7 pattern; Monitor-gated,
  resumable). At least one cap is required.
- `--investigate <operator> [--inv-seeds N]` — multi-instance Theory-Engine
  ablation of one operator across all `--file`/`--family` instances; aggregated
  Popperian confidence, published (supported or refuted).
- `--dataset --dir D` — export the Foundation Dataset + print the honest
  scale-gap manifest (progress toward 500k/1M/5M/20M).
- `--early-stop` — attach the Dynamics early-stop controller to every run
  (opt-in; the plain executor stays bit-identical).

**Performance claims (bit-identical protocol):**
- Measure — never assert. Use `criterion` / `hyperfine` / `cargo bench`.
- A/B with identical seeds via `benchmark_suite/scripts/ab_engine_compare.py`
  (asserts bit-identical energies) or `bin/ab_evolved_vs_ultimate.rs`.
- `tests/test_regression_golden.rs` must pass unchanged.
- Keep only measured **>1%** wins. Revert regressions immediately. Keep the
  previous release binary for A/B.
- **Know what your host can resolve before you claim a win.** Measured on
  2026-08-30 (`research/RC021_C10_DIAGNOSIS.md`), this WSL2 host resolves a
  paired wall-time difference of ~25 % cleanly, ~5 % marginally, and **1 % not at
  all** — two *identical* arms differ by up to 3 % at p90. The >1 % rule above is
  therefore **not verifiable on this machine**: a "1.4 % win" measured here is
  noise wearing a decimal point. Before any timing claim, run
  `cargo run --release --bin host_timing_calibration -- --file <instance>` and
  report the effect against that host's measured null. Do **not** pass `--pin` on
  this host: pinning leaves the null near zero while nearly doubling a known
  injected effect, so it looks safe and is not.

**Reviews:** `/code-review` for the branch; `/code-review ultra` for the
multi-agent cloud review (user-triggered, billed — cannot be launched
programmatically).

## 10. How to Update the Knowledge Graph & Theory Engine

- **Knowledge Graph** (`graph.rs`): facts enter via `observe_if(subject, relation,
  object, condition, evidence, source)`. Confidence accrues by Welford stats over
  repeated observations; `condition_holds` gates applicability. Never write a bare
  fact — always conditional and source-attributed. The MetaLearner's `publish()`
  emits only findings consistent across the history.
- **Theory Engine** (`theory.rs`): the pipeline is
  rule → `MechanismHypothesis` (from `StepEvent` entropy/acceptance/diversity
  signatures) → falsifiable `Prediction` → **ablation on the Runtime** →
  refutation record → `Theory`. Add a mechanism signature only if a real Runtime
  ablation can test it. If the signal is weak, the narrative must say
  "mechanism not clearly evident" — do not overclaim.

## 11. How to Work with Curiosity, Dashboard & Dataset

- **Curiosity** (`curiosity.rs`): surprise = coverage deficit + cross-seed anomaly
  + model disagreement. The explore/exploit λ dial steers compute at the unknown.
  Turn on with `--curiosity <λ>`; `CuriousIdeator` biases proposal generation.
- **Dashboard** (`dashboard.rs`): one self-contained page, dark-first, no external
  assets. Regenerate any time from the persisted stores:
  `research_platform --dir <D> --render-dashboard` — it prints a `file://` path.
  Generated instances live under `experiments/` and are not startup memory.
- **Foundation Dataset** (`dataset.rs`): `FoundationDataset` exports the ML-ready
  corpus + decision log + trajectory digest. It ships a blunt manifest of the
  scale gap to the foundation-model milestones. Export happens automatically in
  the Orchestrator loop; keep the manifest honest.

## 12. How to Write ADRs

Architecture decisions live in `research/architecture/ADR/` (`ADR-0000`…). Before
architectural work, query the existing graph:
`python research/architecture/query_graph.py`. Record a new decision when you
change structure, a boundary, or a cross-cutting guarantee. An ADR states:
context, the decision, the alternatives considered, consequences, and links to
superseded/related ADRs. Keep them small and factual; they are the queryable
memory of *why the architecture is shaped this way*.

## 13. How to Conduct Research (the loop)

The platform embodies the method; follow it when you do research by hand too:

**observe → analyze → learn → plan → run → evaluate → update → write-knowledge.**

1. **Observe** structure and recall similar past structures (Memory Manager).
2. **Analyze** what's known and where confidence is thin (MetaLearner, graph).
3. **Learn / plan** the next question by expected new knowledge (Planner), under
   the Chief Scientist's priorities (Executive).
4. **Run** provenance-stamped experiments through the Runtime (never around it).
5. **Evaluate** with real statistics (Welch/Wilcoxon/bootstrap), not eyeballing.
6. **Update** models (Predictor/Policy/World/Dynamics) and the graph.
7. **Write knowledge** — theories (post-ablation), consensus, dataset rows,
   memory. A finding not written down did not happen.

Every step is honest about uncertainty and keeps its refutations.

---

## Before Finishing — checklist

- ✓ compiles (`cargo check`)
- ✓ affected tests pass (`cargo test`); full suite if public API changed
- ✓ `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean
- ✓ golden regression unchanged; any perf change A/B'd bit-identical
- ✓ no unrelated edits, no broken binaries
- ✓ minimal token usage, minimal file reads
- ✓ new knowledge recorded; new architecture decisions written as ADRs
