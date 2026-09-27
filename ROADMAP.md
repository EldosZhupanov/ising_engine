# Ising Engine — Roadmap

## Current ordered backlog (repository recovery, 2026-09-26)

This queue records the current recovery work. [memory/NOW.md](memory/NOW.md)
selects the active step; [PROJECT_PLAN.md](PROJECT_PLAN.md) remains the binding
order for product research gates. Statuses below are evidence states, not
percent-complete estimates. The stage percentages later in this file are
historical inventory until independently re-audited.

| Priority | Task | Exit evidence |
|---|---|---|
| P0 — COMPLETE | Make the full checkout discoverable and stop invalid Market Split candidates from appearing as solutions. | [PROJECTS.md](PROJECTS.md), `memory/FILE_MAP.tsv`, catalogue coverage; all five rejected candidates preserved outside `solutions/`, seven remaining files accepted by checker. |
| P0 — PRESERVED; REVIEWED | Preserve formerly untracked source and results before any clean clone or submission. | Separate commits `cffaee3`, `004385e`, `31de138`, `4435ae9`, `683c5a7`, `4924772`, `d179e39`; raw replays and checker gates passed. A later independent review found empty-kernel/timeout bugs (fixed at `e04301f`) and architectural debt (still open). No production integration or solver-superiority conclusion follows. The broken upstream clone remains visibly untracked and unused. |
| P1 — COMPLETE: SCOPED TIMING EVIDENCE | Qualify native model preparation after the nondiscriminating MQLib screen. | [MQ-NATIVE-001](research/experiments/mqlib_native_calibration/RESULT.md):960 valid cells, overall FAIL;50/100ms pass all four diagnostic paths. Independent audit PASS; no solver advantage or full initialization claim. |
| P1 — OPEN | Qualify proposed stronger MQLib baseline and freeze difficult-corpus comparison. | Exact PALUBECKIS2004bMST2 sign/offset/witness and callback checks first; separate qualification/untouched holdout, predeclared targets and actual-wrapper cost curves. See [design](research/experiments/mqlib_screen/NEXT_DESIGN.md); existing303 files are not established unseen. |
| P1 — COMPLETE | Establish exact direct-HUBO/quadratization equivalence before timing. | [HUBO-RG001 result](research/experiments/hubo_representation_gate/RESULT.md): 6/6 tests, 832 energies, 2,880 deltas and 4,506 expanded states; no performance claim. |
| P1 — COMPLETE: NOT QUALIFIED FOR ADVANTAGE | Compare native HUBO and quadratization with an external native control. | [HUBO-C001](research/experiments/hubo_comparison/RESULT.md): 400 valid cells; MSC native wins 100/100 against each quadratic arm, ties native OpenJij 100/100. Supports only the registered representation contrast, not competitive solver superiority. |
| COMPLETE — MIXED | HUBO-Q002 local penalties and endpoint-variation qualification. | [Phase A](research/experiments/hubo_corpus_qualification/RESULT_PHASE_A.md): 24 certified reductions. [Phase B](research/experiments/hubo_corpus_qualification/RESULT_PHASE_B.md): 240 valid cells, 6/12 instances and 2/4 strata qualify; >=3/4 success criterion unmet. Independent audit PASS; no superiority claim. |
| RESEARCH — ER-001 COMPLETE; NEXT SCORER/BLOCKING DESIGN | Retain verified entity reconciliation; test the remaining semantic/coverage bottleneck on fresh data. | [Result](research/experiments/entity_resolution/RESULT.md): H1PASS, H2FAIL, H3PASS;120 valid cells, native optima120/120, but greedy equals exact on all12 blocks. No added-search benefit established. New data/protocol and identical decoder required before model claims; Laya not run. |
| P1 — DEFERRED, DESIGN FIRST | Fresh holdout for penalty/schedule controls with native baselines. | Preregister local/global penalties crossed with derived/common temperature endpoints; declare population before new inputs. Q002 is opened diagnostic data. No new campaign without its own binding protocol. |
| P1 — COMPLETE | Repair provenance errors prospectively, including the RC027 instrument-commit typo; avoid editing frozen results. | Actual commit plus source SHA match recorded in [memory/RESEARCH.md](memory/RESEARCH.md); frozen result retained. |
| P1 — COMPLETE | Fix concurrent LABS checkpoint publication and isolate output paths before qualification. | `66e9934`: synced atomic checkpoint replacement before best publication; strict `--output-dir`; synchronous checker status; 4/4 targeted tests and independent review PASS. Scope is one process per output directory. |
| P1 — COMPLETE: NOT QUALIFIED | Qualify the later LABS memetic hunter against lMAts at equal time before another record campaign. | [LABS-Q002 result](research/experiments/labs_q002/RESULT.md): 60/60 valid cells, hunter hits 4/0/0 versus lMAts 10/8/0, 0 wins/4 ties/26 losses; independent raw audit PASS. Long record campaign is NO-GO on this evidence. |
| P1 — DEFERRED | Diagnose LABS search-budget allocation before adding or tuning operators. | New protocol and diagnostic seeds, measured initialization/PT/tabu/2-opt costs, then a named matched-cost ablation; Q002 outcomes remain frozen. |
| P1 — OPEN | Resolve lattice prototype architecture and arithmetic safety before production use. | Move preprocessing and QUBO lowering out of `core/`, move search math out of `bin/`, prove complete integer kernel or narrow its claim, and test overflow limits. Preserve experimental snapshot and separate this from the LABS campaign. |
| P2 | Run a blind Market Split lattice/UltimateSolver ablation only as an algorithmic benchmark. | Label-free `(A,b)`, preprocessing inside wall budget, strong specialist baseline, censored outcomes; no first-known-solution framing. |
| P2 | Audit `fundamental_ai` raw/protocol chronology and external baselines before promotion. | Standalone tests plus independent reproduction and claim ledger. |
| P3 | Validate website and old root scripts only when they enter the active product path. | Own build/typecheck and call-site map. |
| RESEARCH — SCREENED | Direct fourth-order HUBO versus quadratization. | C001 representation signal is scoped to its conservative reduction; native solver advantage remains unqualified. See the next measurement-design task above. |
| CLEANUP | Classify ignored `experiments/` and `archive/`; retain significant results, remove nothing by age alone. | Inventory rows with provenance and explicit disposition. |

---

> The single source of truth for **direction** is
> `research/ISING_ENGINE_CONSTITUTION.md`. This roadmap is the **status** view:
> what exists, what is being built, what is planned, and where we are going.
> Philosophy lives in `SOUL.md`; working rules in `CLAUDE.md`; architecture
> decisions in `research/architecture/ADR/`.
>
> **The active sequence lives in `PROJECT_PLAN.md`** — the current gate, its
> branches, and the ordered path to a verifiable product. This roadmap stays the
> stage inventory; it does not track what is in flight.

The project is two things at once:

1. **A production optimization engine** — `src/solver` (`UltimateSolver`: MSC
   bit-sliced + Parallel Tempering + Population Annealing) and the `src/core`
   QUBO/HUBO model. Stable, fast, verified. *Do not rewrite it.*
2. **An autonomous research platform** — `src/engine_v2`, a self-learning
   optimization OS that discovers, explains, and transfers solver strategies,
   and is growing toward a **Research Foundation Model**.

Scale-sensitive counts (lines, tests, modules, registry size) are intentionally
not frozen in this roadmap: query the code and test runner when they matter.
The committed research corpus contains **18,570 recorded experiments** on G-Set.

Legend: 🟢 Completed · 🟡 In Progress · 🔵 Planned · 🔬 Research · 🔭 Future Vision

---

## Stage overview (completion %)

| Stage | What | Status | % |
|---|---|---|---|
| 0 | Production solver (`UltimateSolver`, legacy solvers, core) | 🟢 | 100% |
| 1 | Benchmark framework (parsers, stats, TTS, A/B, portability) | 🟢 | 90% |
| 2 | engine_v2 substrate (IR, verified backends, operator registry, Runtime) | 🟢 | 95% |
| 3 | Generation (Decision Engine, Evolution, capability selection) | 🟢 | 90% |
| 4 | Knowledge system (append-only DB, Knowledge Graph, Meta-Learner) | 🟢 | 90% |
| 5 | AI Scientist (multi-agent Lab, LLM ideator, novelty, reports) | 🟢 | 90% |
| 6 | **Autonomous Research Platform** (Campaign Manager + all above) | 🟢 | 95% |
| 6b | Learned models (Predictor, Policy, Dynamics, World) + wiring | 🟢 | 90% |
| 6c | Meta-Learning Layer (cross-model consensus) | 🟢 | 85% |
| 6d | Curiosity Engine (active learning) | 🟢 | 80% |
| 7 | **Research OS / Operations** (Orchestrator, Planner, Executive, Registry, Monitor, Service) | 🟢 | 90% |
| 8 | **Knowledge OS** (Theory Engine + multi-instance investigate, Memory, Dataset) | 🟢 | 80% |
| 9 | Problem families (MaxCut/BQP/QPLIB + runnable TSP/SAT) | 🟡 | 75% |
| 10 | Dashboard (single pane, executive-driven) | 🟢 | 90% |
| 11 | Foundation Dataset → **Research Foundation Model** | 🔬 | 10% |

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
Registered operators are selected by capability passport and cross-validated
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

### 6b — Learned Models 🟢 90%
- **Predictor** (ridge) — schedule → expected improvement; used as a filter.
- **Policy** (`policy.rs`) — neural next-operator model; supervised on history +
  REINFORCE against the Runtime. Held-out on G-Set: **wins 5/5 vs default & random**.
- **Dynamics** (`dynamics.rs`) — remaining-improvement from a partial trajectory.
  Its deployed early-stop path is disabled and refuses loudly after RC-012;
  repairing model and controller together remains future work.
- **World** (`world.rs`) — imagined observable-state rollouts; imagined-vs-real
  schedule ranking **Spearman ≈0.85–0.90 on genuinely held-out candidates**
  (RC-010). The previously reported **0.975** came from a 5-candidate set of
  which **4 were verbatim training members**; `world.rs`'s test gates only at
  `rho >= 0.5`. Audited: rho does NOT collapse when the easy noise-vs-greedy gap
  is removed (0.8485 with the ranking task 7.5× narrower), so the skill is
  genuine — the headline number was inflated, not fabricated. Warm-start probe
  fixes the cold-start bias.
- *Confirmed:* cross-family transfer is real (portability **9/12**). ⚠️ The
  predictor's leave-one-instance-out **Spearman 0.747** is **NOT** evidence for
  transfer and has been removed from that claim (RC-011): the predictor is purely
  additive in (instance, schedule) with no interaction terms, and every row of a
  held-out instance shares one signature, so `w·x_instance` is a constant offset
  across the fold — and Spearman, being rank-based, is *provably invariant* to it
  (verified: max deviation 1.1e-16, 0 signature violations over 18,570 rows).
  0.747 measures **schedule-quality ranking**, which is what `filter` needs; a
  predictor ignoring the instance entirely would score the same.
- *Remaining:* all four are small (ridge/MLP); foundation scale is Stage 11.

### 6c — Meta-Learning Layer 🟢 85%
`meta_layer.rs`: consolidates per-operator signals from Meta-Learner + Policy +
World + Dynamics into one source-attributed consensus; publishes to the graph;
`MetaBiasedIdeator` biases evolution. Opt-in (`--shared-knowledge`).

### 6d — Curiosity Engine 🟢 80%
`curiosity.rs`: surprise = coverage deficit + cross-seed anomaly + model
disagreement; the explore/exploit λ dial; `CuriousIdeator`. Steers compute at
the unknown, not only the best.

## Stage 7 — Research OS / Operations 🟢 90%

- 🟢 `ResearchOrchestrator` — the lifecycle loop (observe→analyze→learn→
  plan→run→evaluate→update→write-knowledge), auto-exports dataset + dashboard.
- 🟢 `ResearchPlanner` — autonomous task-setting: picks the next instance by
  expected new knowledge (coverage deficit + structural novelty).
- 🟢 `ResearchExecutive` (**Chief Scientist**) — decides what to research, which
  model to retrain, local(Qwen)-vs-cloud(Claude) routing, publish-vs-gather-
  evidence — each with a measured reason. **The loop OBEYS it** (`--executive`):
  it retrains stale models, investigates flagged operators, and **routes idea
  generation to a live Ollama/Qwen** (verified: 12 Qwen-originated experiments).
- 🟢 **Model Registry** (`model_registry.rs`) — versioned weights, lineage, and
  campaign/generation snapshots. Loading snapshots for warm-start/A-B remains.
- 🟢 **Monitor** (`monitor.rs`) — health gates that halt on broken append-only
  invariants. Live alerting and tuned auto-pause remain.
- 🟢 **Persistent budget-capped service** — `run_service` with explicit tick,
  experiment, and wall-time budgets. Multi-machine execution remains.

## Stage 8 — Knowledge OS 🟢 80%

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
- 🟢 Multi-instance `investigate_operator` aggregates ablations into Popperian
  confidence. Remaining: broader family coverage and a stronger
  structural-signature memory index.

## Stage 9 — Problem Families 🟡 75%

`engine_v2::families`: **TSP** (Lucas permutation encoding, + decoder) and
**MAX-2-SAT** — both **proven correct vs brute force** (QUBO ground state = true
optimum) and wired as runnable platform targets. Everything downstream runs on
`ProblemIR` unchanged. Existing: MaxCut / BQP / QPLIB / BiqMac. Remaining:
family-specific campaigns, Scheduling (job-shop), full VRP, and SAT gadgets for
k>2.

## Stage 10 — Dashboard 🟢 90%

`dashboard.rs`: a single self-contained page (dark-first, light via media query,
no external assets) — a **Chief Scientist hero panel** with urgency-ranked
directives, KPI cards, gradient-area quality chart, operator bars, and cards for
Theories / Consensus / Curiosity / Memory / Best-algorithms / Knowledge-graph
with status pills. `--render-dashboard` regenerates from stores. Rendered over
18,570 experiments. 🔵 *Planned:* a hosted live site (deliberately deferred —
attack surface).

## Stage 11 — Research Foundation Model 🔬 10%

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

- ~~**`cargo doc` is broken by pre-existing rustdoc-link errors**~~ **RESOLVED
  (2026-07-27).** 18 error sites across 9 files (`core/hubo`, `concept`,
  `curiosity`, `graph`, `frontend`, `ir`, `solver/engine`,
  `solver/population_annealing`, `bin/solve_v2`, `bin/experiment`) — bare `[x]`
  in prose parsed as intra-doc links, `Vec<Vec<Edge>>` and `<EXP>` parsed as
  unclosed HTML tags, and one public→private link (`[`ridge_fit`]`). All fixed by
  backticking the prose; **doc-comments only, 15 lines changed, zero code lines**
  (verified by diff). `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace`
  now returns **0 errors**. Note the errors cascade — rustdoc aborts per crate, so
  each fix reveals the next; three passes were needed. 🔵 *Remaining:* add that
  command as a CI gate to prevent regression.
- **Adaptive cloud routing** recommended but cloud unavailable (no API key).
- **World-filtered ideation / Dynamics early-stop are opt-in** (change
  candidate-selection / budget by design); default-on needs per-family
  rank-correlation / early-stop validation first.
- ~~Monitor predictor-health cost~~ **RESOLVED at the root**: `evaluate_predictor`
  rewritten from naïve leave-one-out (O(K·N·d²) + O(K·N) row-clones) to additive
  sufficient statistics (O(N·d²), zero clones). **Measured 217.6 → 12.1 ms/call
  (~18×)** on the 18,570-exp DB; `Monitor::check` 238 → 12.4 ms (~19×); mean
  leave-one-out Spearman preserved at 0.7467 (≈ historical 0.747). No Monitor
  throttling needed — the symptom is gone because the root cost is gone.
- **Registry** snapshots the Predictor every generation (append-only growth is
  fine but consider bounding for very long 24/7 services).
- **`--early-stop`** bootstraps one Dynamics model from `instance[0]`; a
  per-instance controller would transfer its stop decisions better.
- **Multi-instance `investigate`** confidence stays low until run across many
  instances (single-instance = weak, by the shrink-for-few-trials design).
- Consensus/theory facts publish at **support=1** → low graph-confidence until
  accumulated across campaigns.
- `experiments/` generated state (gitignored, regenerable).

---

## Done — the prior "Next 10" (all shipped)

All ten prior-priority tasks are implemented, tested, and committed on
`feat/solver-research-upgrades` (additive / opt-in throughout — the golden
regression and cross-backend bit-identity firewall pass unchanged):

1. ✅ **World-model filtering** in ideation (`WorldFilteredIdeator`: imagine → rank → run top few).
2. ⚠️ ~~**Dynamics early-stop**~~ **WITHDRAWN 2026-08-19.** `--early-stop` now **refuses loudly and exits 2** instead of silently doing nothing. Trajectory-neutral: the feature never activated, so disabling it changes no run. **RC-012: non-functional as shipped.** The deployed bootstrap (2-step schedule × 3 seeds = 6 rows) is below `fit`'s 20-row floor, so the flag *always* skips; and even a fitted model (module-test corpus) predicts ≡0 remaining at every step, making the ε-gate vacuous — behaviourally 'stop at min_frac'. Layers must be fixed together; see `research/RC012_DYNAMICS_EARLY_STOP_AUDIT.md`.
3. ✅ **Per-situation model selection** in the Executive (`Action::UseModel`, regime + staleness conditioned).
4. ✅ **SAT/TSP runnable** (`families::{tsp_instance,max2sat_instance}` + `--family tsp|max2sat`; verified end-to-end).
5. ✅ **Model Registry** (`model_registry.rs`: versioned weights + lineage; snapshots each campaign/generation).
6. ✅ **Monitor** (`monitor.rs`: health gates; halts the loop on a broken append-only invariant).
7. ✅ **Cross-family transfer at scale** (portability now covers TSP/SAT; MaxCut-policy 12/15 non-MaxCut on the 18,570-exp DB).
8. ✅ **Persistent budget-capped service** (`run_service` + `ServiceBudget`; `--service --max-ticks/-experiments/-wall-secs`).
9. ✅ **Foundation Dataset growth observable** (`--dataset`: export + honest scale-gap manifest; growth via the curiosity service).
10. ✅ **Multi-instance theory `investigate`** (`investigate_operator` + `--investigate <op>`; aggregates ablation trials → Popperian confidence).

## Next 10 (new priorities)

1. **Run the growth campaign**: a long curiosity-driven `--service` toward 500k experiments; re-measure transfer as it grows.
2. **Distributed `BatchExecutor`** behind the same trait (multi-machine), for the growth campaign's throughput.
3. **Model Registry consumption**: load the latest snapshot to warm-start training / A-B model versions on held-out instances.
4. **Monitor → live gate** in the service loop with alerting + auto-pause thresholds tuned on real runs.
5. **Reconstruct-and-serve models** from the registry (a `--serve-model` path) so a trained Policy/World is reusable without retraining.
6. **SAT/TSP knowledge**: run campaigns on the new families and mine family-specific rules; add k>2 SAT gadgets, job-shop scheduling.
7. ~~AVX/SIMD field ledger~~ **PARTIALLY DONE**: `SparseBitSlice::apply_flips` now
   adaptively dispatches the field update — scattered `O(neighbors·flips)` for
   sparse flips, contiguous/vectorizable `O(neighbors·r)` for dense flips
   (crossover ~`r/3`, measured). Bit-identical (integer ledger; firewall
   unchanged). Isolated-kernel A/B: no regression at low flip count, **1.6–4.6×
   faster at high flip count** (G11→G1); random_flip_sweep −29% end-to-end.
   Remaining: explicit `#[target_feature]` AVX2/512 for the dense inner SAXPY.
8. **World-filtered ideation on by default** once its rank-correlation is validated per-family (currently opt-in with shared-knowledge).
9. **Theory confidence at scale**: `--investigate` across the full G-Set to move key operators from support≈1 to durable confidence.
10. **Foundation-model readiness gate**: when the dataset crosses 500k, re-evaluate whether a shared GNN/Transformer representation is finally earned.
