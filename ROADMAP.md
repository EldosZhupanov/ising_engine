# Ising Engine — Roadmap

> The single source of truth for **direction** is
> `research/ISING_ENGINE_CONSTITUTION.md`. This roadmap is the **status** view:
> what exists, what is being built, what is planned, and where we are going.
> Philosophy lives in `SOUL.md`; working rules in `CLAUDE.md`; architecture
> decisions in `research/architecture/ADR/`.

The project is two things at once:

1. **A production optimization engine** — `src/solver` (`UltimateSolver`: MSC
   bit-sliced + Parallel Tempering + Population Annealing) and the `src/core`
   QUBO/HUBO model. Stable, fast, verified. *Do not rewrite it.*
2. **An autonomous research platform** — `src/engine_v2`, a self-learning
   optimization OS that discovers, explains, and transfers solver strategies,
   and is growing toward a **Research Foundation Model**.

Scale as of this writing: ~20.6k lines in `engine_v2`, 28 `ai_scientist`
modules, **280 tests**, 8 committed feature waves this session. Accumulated
research data: **18,570 recorded experiments** on G-Set.

Legend: 🟢 Completed · 🟡 In Progress · 🔵 Planned · 🔬 Research · 🔭 Future Vision

---

## Stage overview (completion %)

| Stage | What | Status | % |
|---|---|---|---|
| 0 | Production solver (`UltimateSolver`, legacy solvers, core) | 🟢 | 100% |
| 1 | Benchmark framework (parsers, stats, TTS, A/B, portability) | 🟢 | 90% |
| 2 | engine_v2 substrate (IR, 3 backends, 14 operators, Runtime) | 🟢 | 95% |
| 3 | Generation (Decision Engine, Evolution, capability selection) | 🟢 | 90% |
| 4 | Knowledge system (append-only DB, Knowledge Graph, Meta-Learner) | 🟢 | 90% |
| 5 | AI Scientist (multi-agent Lab, LLM ideator, novelty, reports) | 🟢 | 90% |
| 6 | **Autonomous Research Platform** (Campaign Manager + all above) | 🟢 | 95% |
| 6b | Learned models (Predictor, Policy, Dynamics, World) | 🟡 | 80% |
| 6c | Meta-Learning Layer (cross-model consensus) | 🟢 | 85% |
| 6d | Curiosity Engine (active learning) | 🟢 | 80% |
| 7 | **Research OS / Operations** (Orchestrator, Planner, Executive) | 🟡 | 60% |
| 8 | **Knowledge OS** (Theory Engine, Scientific Memory, Dataset) | 🟡 | 70% |
| 9 | Problem families (MaxCut/BQP/QPLIB + TSP/SAT) | 🟡 | 50% |
| 10 | Dashboard (single pane, executive-driven) | 🟢 | 90% |
| 11 | Foundation Dataset → **Research Foundation Model** | 🔬 | 8% |

---

## Stage 0 — Production Solver 🟢 100%

`UltimateSolver` (`src/solver/ultimate.rs`): MSC bit-sliced (64 replicas/u64) +
Parallel Tempering + Population Annealing + QPBO/roof-duality presolve
(`src/presolve`). Legacy solvers (`parallel_tempering`, `adaptive`, `cluster`,
`tabu`, `autopilot`) are independent and **frozen** — never merged, never
rewritten. Golden regression (`tests/test_regression_golden.rs`) protects it.
This is the arbiter of any production performance claim (A/B vs it, identical
seeds — it currently wins 5/5 vs evolved plans; **no superiority is claimed for
the research engine over production**).

## Stage 1 — Benchmark Framework 🟢 90%

`src/benchmark`: energy-exact parsers for **G-Set / Biq Mac / OR-Library BQP /
QPLIB** (`instances.rs`, convention-validated against published optima);
Wilcoxon + paired-t + bootstrap CIs (`stats.rs`); TTS(0.99) harness
(`solver/tts.rs`); OpenJij/dwave-neal adapters (honest skip when absent); CSV/
JSON/LaTeX/SVG reports. `frontend::qubo_model_to_ir` bridges any parsed model
into the engine_v2 IR (energy-exact, tested). A/B harness
(`bin/ab_evolved_vs_ultimate.rs`). *Remaining:* commercial-solver ingestion is
drop-in only; live external baselines depend on the machine.

## Stage 2 — engine_v2 Substrate 🟢 95% (read-only core)

`ProblemIR` + `SpinState` trait; three verified backends —
`ReferenceState` (f64 oracle), `SparseBitSlice` (exact integer, >100k
cross-checks), `DenseByte` (production-shaped, **4.8× vs oracle**, bit-identical).
**14 operators** selected by capability passport, all cross-validated
bit-identical on every backend. Deterministic `Runtime` with adaptive
controllers (`maybe_adapt`: ladder heat/cool, UCB1 operator bandit, ensemble-
collapse phase-skip) and a learned `RunController` early-stop hook — all opt-in
and **bit-identical on replay** (ADR-0004). *Remaining:* AVX/SIMD field-ledger
throughput in `SparseBitSlice`; wall-time in the utility model.

## Stage 3 — Generation 🟢 90%

`DecisionEngine`: structural analysis → backend selection → utility plan
synthesis (`synthesize_plan`, α·q̂ + β·ĉ + γ·m̂ + δ·l̂ with recorded rationale,
learned from the KnowledgeBase). `Evolution Engine`: genetic search over
operator **sequences** with multi-criteria utility fitness. Operators are chosen
by **capability, never by name**. *Remaining:* World-model-filtered candidate
pre-ranking is designed, not wired.

## Stage 4 — Knowledge System 🟢 90%

Append-only `ExperimentDb` (22-field provenance rows, legacy-14 compatible,
`flush_append`); `KnowledgeGraph` of conditional, evidence-weighted, source-
attributed facts (`observe_if`, Welford `confidence()`, `condition_holds`);
`MetaLearner` mining 8 rule kinds (dominance/conditional/ordering/antipattern/
useless/temperature/budget/backend) + `publish()` of consistent findings.
Cross-instance reproducibility audit (`evaluation.rs`): 30 rules recur on ≥12
G-Set instances, 5 on all 23.

## Stage 5 — AI Scientist 🟢 90%

Multi-agent `ScientificLab` (HypothesisGenerator, ExperimentDesigner,
Statistician, KnowledgeManager); capability-grounded reasoning; novelty search;
cost-aware fitness; local LLM ideator (`LlmHypothesisGenerator` over Ollama,
graceful heuristic fallback); cloud analyst (`CloudScientist`, Claude Messages
API, honest skip without key); operator-proposal drafter (`proposal.rs`, no
auto-code).

## Stage 6 — Autonomous Research Platform 🟢 95%

`CampaignManager`: generational closed loop — provenance-stamped experiments →
Meta-Learner → graph → reports every N → cloud review every M → operator
proposals → Predictor refit → dashboard, all persisted and resumable. Verified:
18,570 experiments across 23 G-Set instances.

### 6b — Learned Models 🟡 80%
- **Predictor** (ridge) — schedule → expected improvement; used as a filter.
- **Policy** (`policy.rs`) — neural next-operator model; supervised on history +
  REINFORCE against the Runtime. Held-out on G-Set: **wins 5/5 vs default & random**.
- **Dynamics** (`dynamics.rs`) — remaining-improvement from a partial trajectory;
  powers the early-stop controller.
- **World** (`world.rs`) — imagined observable-state rollouts; imagined-vs-real
  schedule ranking **Spearman 0.975**. Warm-start probe fixes the cold-start bias.
- *Confirmed:* cross-family transfer is real (portability **9/12**, predictor
  leave-one-instance-out **Spearman +0.747**).
- *Remaining:* all four are small (ridge/MLP); foundation scale is Stage 11.

### 6c — Meta-Learning Layer 🟢 85%
`meta_layer.rs`: consolidates per-operator signals from Meta-Learner + Policy +
World + Dynamics into one source-attributed consensus; publishes to the graph;
`MetaBiasedIdeator` biases evolution. Opt-in (`--shared-knowledge`).

### 6d — Curiosity Engine 🟢 80%
`curiosity.rs`: surprise = coverage deficit + cross-seed anomaly + model
disagreement; the explore/exploit λ dial; `CuriousIdeator`. Steers compute at
the unknown, not only the best.

## Stage 7 — Research OS / Operations 🟡 60%

- 🟢 `ResearchOrchestrator` — the lifecycle loop (observe→analyze→learn→
  plan→run→evaluate→update→write-knowledge), auto-exports dataset + dashboard.
- 🟢 `ResearchPlanner` — autonomous task-setting: picks the next instance by
  expected new knowledge (coverage deficit + structural novelty).
- 🟢 `ResearchExecutive` (**Chief Scientist**) — decides what to research, which
  model to retrain, local(Qwen)-vs-cloud(Claude) routing, publish-vs-gather-
  evidence — each with a measured reason. **The loop OBEYS it** (`--executive`):
  it retrains stale models, investigates flagged operators, and **routes idea
  generation to a live Ollama/Qwen** (verified: 12 Qwen-originated experiments).
- 🔵 **Model Registry** (versioned weights + lineage) — designed (Stage 7 doc), unbuilt.
- 🔵 **Monitor** (continuous golden/replay/ledger + model-health gates) — designed, unbuilt.
- 🔵 **Persistent Scheduler service** (job queue, budgets, triggers) — designed, unbuilt.

## Stage 8 — Knowledge OS 🟡 70%

- 🟢 **Theory Engine** (`theory.rs`) — the true-researcher core:
  rule → mechanism (from `StepEvent` entropy/acceptance/diversity signatures) →
  falsifiable prediction → **ablation on the Runtime** → refutation record →
  theory. A theory is a mechanism that *survived* an attempt to break it. Honest:
  metropolis_sweep is confirmed causal (ablation costs 5%), redundant operators
  are refuted, and the mechanism narrative says "not clearly evident" when it is.
- 🟢 **Scientific Memory + Memory Manager** (`memory_os.rs`) — structure-keyed
  recall ("seen a similar structure; X worked") + regime index; append-only,
  never deletes.
- 🟢 **Foundation Dataset** (`dataset.rs`) — ML-ready corpus export + decision-log
  (why each agent chose) + search-trajectory digest; blunt manifest on the scale gap.
- 🔵 Explanation assembler → ablation loop at scale; structural-signature memory
  index across families; Popperian confidence via many-instance investigation.

## Stage 9 — Problem Families 🟡 50%

`engine_v2::families`: **TSP** (Lucas permutation encoding, + decoder) and
**MAX-2-SAT** — both **proven correct vs brute force** (QUBO ground state = true
optimum). Everything downstream runs on `ProblemIR` unchanged. Existing:
MaxCut / BQP / QPLIB / BiqMac. 🔵 *Planned:* wire TSP/SAT as runnable platform
research targets; add Scheduling (job-shop) and full VRP; SAT gadgets for k>2.

## Stage 10 — Dashboard 🟢 90%

`dashboard.rs`: a single self-contained page (dark-first, light via media query,
no external assets) — a **Chief Scientist hero panel** with urgency-ranked
directives, KPI cards, gradient-area quality chart, operator bars, and cards for
Theories / Consensus / Curiosity / Memory / Best-algorithms / Knowledge-graph
with status pills. `--render-dashboard` regenerates from stores. Rendered over
18,570 experiments. 🔵 *Planned:* a hosted live site (deliberately deferred —
attack surface).

## Stage 11 — Research Foundation Model 🔬 8%

The honest frontier (`research/STAGE_8_KNOWLEDGE_OS.md §7`). A model that
predicts *which algorithm behaves how* across all families, trained on the
platform's own studies. Needs three things, none of which is architecture alone:
**(1) data volume** — 18,570 today; milestones **500k → 1M → 5M → 20M** (the
Foundation Dataset manifest tracks the multiplier still needed); **(2) a shared
representation** across families (where a GNN finally earns its complexity —
gated on evidence the 5 scalar features are the bottleneck, not assumed);
**(3) scale**. Evidenced possible by the portability result; a seed, not a
harvest. *Building a Transformer/GNN now would over-fit — the path is to GROW the
dataset (Curiosity + Orchestrator generate informative experiments) first.*

---

## Confirmed vs refuted (research honesty)

**Confirmed:** cross-family transfer (9/12, +0.747); World-model imagined ranking
tracks reality (0.975); Curiosity and consensus disagree *meaningfully*
(explorer targets the exploiter's uncertainty); Theory Engine separates causal
from spurious via ablation; DenseByte 4.8× and bit-identical.

**Refuted / honest negatives (kept as knowledge):** the platform's own proposed
operator `extremal_metropolis` is **weak** on G-Set (worst-solutions 8×) — the
loop's proposal was tested and failed, honestly; **evolved plans lose 5/5 to
`UltimateSolver`** at equal budget (no superiority claimed); the cold-start probe
bias over-AVOIDed ensemble operators (**now fixed** via warm-start).

## Technical debt / unfinished

- Per-situation **model selection** (the meta-layer consults all models).
- **World-filtered ideation** (imagine→rank→run-top-few) designed, not wired.
- **Dynamics early-stop** not applied on the executor's every-run path.
- **Adaptive cloud routing** recommended but cloud unavailable (no API key).
- Stage 7 **Model Registry / Monitor / Scheduler service** designed, unbuilt.
- SAT/routing **not yet runnable platform research targets** (encoders only).
- Consensus/theory facts publish at **support=1** → low graph-confidence until
  accumulated across campaigns.
- `experiments/` generated state (~5MB) is gitignored (regenerable).

---

## Next 10 (highest priority)

1. Wire **World-model filtering** into ideation (rank Qwen/Evolution candidates, run top few).
2. Wire **Dynamics early-stop** into the executor's every-run path (bounded, opt-in).
3. **Per-situation model selection** in the Executive (choose the model that fits).
4. Add **SAT/TSP as runnable platform research targets** (a families frontend + campaign path).
5. **Model Registry** (Stage 7): versioned Policy/World/Dynamics/Predictor + lineage.
6. **Monitor** (Stage 7): continuous golden/replay/ledger + model-health gates that pause the loop.
7. **Cross-family transfer study** at scale: train on MaxCut, evaluate cold on BQP/QPLIB/SAT.
8. **Persistent Orchestrator service** + job scheduler (24/7 autonomous cadence, budget-accounted).
9. **Grow the Foundation Dataset** toward 500k via Curiosity-driven campaigns; re-measure transfer.
10. **Warm-start audit + confidence** for theories: multi-instance `investigate` to earn real Popperian confidence.
