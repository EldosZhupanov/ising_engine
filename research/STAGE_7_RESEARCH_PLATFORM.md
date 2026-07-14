# Stage 7 — The Research Platform (the Operating System of the Lab)

Status: DESIGN. Governs orchestration only; subordinate to the Constitution
(`research/ISING_ENGINE_CONSTITUTION.md`) and the ADRs
(`research/architecture/ADR/`). Nothing here may weaken §4 (Immovable
Principles), §14 (What Cannot Change), ADR-0004 (reproducibility), or
ADR-0005 (certified lowering).

---

## 0. Thesis — why Stage 7 is a distinct stage

Stage 6 built the research **capabilities**: an append-only experiment memory,
a knowledge graph, a meta-learner, four learned models (Predictor, Policy,
Dynamics, World), a multi-agent AI Scientist, local + cloud LLM tiers, and a
generational Campaign Manager. Those are the *organs*.

They are still driven as a **batch tool**: `research_platform --campaigns N`
runs a fixed number of campaigns and exits. The models are trained per campaign
and thrown away. There is no persistent scheduler, no resource governor, no
health monitor, no model lineage. A human (or Claude Code) is the loop.

**Stage 7 builds the research _operations_** — the always-on kernel that runs
the organs continuously, safely, and autonomously:

> Stage 6 = *what the lab can do.*  Stage 7 = *the lab running itself.*

Concretely, Stage 7 adds seven operational components on top of the existing
scientific ones, and formalizes the buses that connect them. It turns a
command you invoke into a service that operates.

---

## 1. Architectural analysis of the current project

The codebase separates into five strata. The bottom two are **read-only**
(Constitution "electric light sources"); everything above only *generates* and
*analyzes*, and delegates all computation downward.

```
 STRATUM                 MODULES (src/…)                                   MUTABILITY
 ─────────────────────────────────────────────────────────────────────────────────
 E. OPERATIONS  (NEW)    Orchestrator · Scheduler · Resource/LLM Mgr ·     Stage 7
    "the OS"             Monitor · Reporter · Model Registry · Insight Bus
 ─────────────────────────────────────────────────────────────────────────────────
 D. SCIENCE              ai_scientist/{scientist,lab,campaign,             above core
    "the researchers"    meta_learner,meta_layer,proposal,evaluation,
                         reports,cloud,llm,dashboard}
 ─────────────────────────────────────────────────────────────────────────────────
 C. LEARNED MODELS       ai_scientist/{predictor,policy,dynamics,world}    above core
    "the intuition"      + knowledge graph + knowledge base
 ─────────────────────────────────────────────────────────────────────────────────
 B. GENERATION           engine_v2/{evolution,decision} + registry +       above core
    "the hypotheses"     capability + frontend
 ─────────────────────────────────────────────────────────────────────────────────
 A. SUBSTRATE  (R/O)     engine_v2/{runtime,scheduler,operator,state,ir,   READ-ONLY
    "the physics"        plan,context} · backends/{reference,sparse_
                         bitslice,dense_byte} · operators/{13 operators}
 ─────────────────────────────────────────────────────────────────────────────────
 0. MEMORY               ai_scientist/{db,graph} + executor + novelty      append-only
    "the record"
```

The invariant that makes the whole tower safe: **only Stratum A computes
energies, and it is deterministic and never modified.** Every layer above is a
*proposer* or an *analyzer*; none may override the Runtime's verdict. Stage 7
inherits this without exception — the Orchestrator schedules and governs, it
never scores a solution itself.

---

## 2. Design principles (inherited, non-negotiable)

1. **Correctness ⟩ compilation ⟩ tests ⟩ performance ⟩ readability** (Constitution decision order). The Orchestrator never trades down.
2. **Determinism / replay** (ADR-0004): every scheduled run is reproducible from `(instance, seed, plan, model-version)`. The Scheduler records all four.
3. **Honesty**: an unavailable tier (no Ollama, no API key, missing data) is *disclosed and skipped*, never faked. This already holds for `CloudScientist`/`LlmHypothesisGenerator`; Stage 7 makes it a platform-wide contract enforced by the Monitor.
4. **Append-only history** (§12): the DB and graph only grow. Model artifacts are versioned, never overwritten.
5. **The LLM proposes, the Runtime disposes** (§11): LLMs (local or cloud) generate ideas, hypotheses, operator drafts, and prose analysis. They never compute a score and never make a decision the Runtime should make.
6. **Bounded, budget-accounted autonomy** (§10 rule 1): every automated actor (a campaign, an LLM call, a model retrain) runs under a declared budget the Monitor enforces.

---

## 3. Component roles & responsibilities

### 3.1 Substrate (Stratum A — exists, read-only)

| Component | Role | Owns | Must-not |
|---|---|---|---|
| **Runtime** (`runtime.rs`) | Executes a `Plan` over a `SpinState`; owns ladder, RNG, iteration, event log, `maybe_adapt` controllers, and the `RunController` early-stop hook. | The trajectory, the per-step `StepEvent` log, `QualityMetrics`. | Know the operator's algorithm; branch on wall-clock in a trajectory-preserving op; be non-deterministic. |
| **Backends** (`reference/sparse_bitslice/dense_byte`) | ΔE, energies, flips, digests under one `SpinState` trait. | Field ledgers. | Drift (`audit()`==0 on integer substrates); diverge across backends. |
| **Operators** (13) | One physical law each, by capability. | Their own scratch. | Read couplings directly; misalign the RNG stream across backends. |

### 3.2 Generation (Stratum B — exists)

| Component | Role | Inputs | Outputs |
|---|---|---|---|
| **Decision Engine** (`decision.rs`) | Analyze instance structure; select backend; synthesize a utility plan (α·q̂+β·ĉ+γ·m̂+δ·l̂) with recorded rationale. | `ProblemIR` + `KnowledgeBase`. | `Plan` + per-step rationale. |
| **Evolution Engine** (`evolution.rs`) | Genetic search over operator *sequences*; utility fitness. | Capability pool + seeds (from Ideators). | Best `Schedule` + `Experience`. |
| **Frontend** (`frontend.rs`) | Lower external formats → `ProblemIR`. `rudy_maxcut_ir`, and `qubo_model_to_ir` (energy-exact bridge for BQP/QPLIB/BiqMac). | Family files. | `ProblemIR`. |

### 3.3 Learned models (Stratum C — exists)

Each is a **consumer of history** and a **contributor of intuition**. None
runs the Runtime for its verdict except to *train*.

| Model | Predicts | Trained on | Consumed by |
|---|---|---|---|
| **Predictor** (`predictor.rs`) | schedule → expected relative improvement | whole DB (ridge) | candidate **filter** (FilteredIdeator) |
| **Policy** (`policy.rs`) | next operator \| (features, prefix) | quality-filtered DB (supervised) + Runtime (REINFORCE) | `PolicyIdeator`, first-op preference for the Meta-Layer |
| **Dynamics** (`dynamics.rs`) | remaining improvement \| partial trajectory | captured `StepEvent` logs (ridge) | `EarlyStopController` (Runtime hook), plateau signal for the Meta-Layer |
| **World** (`world.rs`) | next observable state \| (state, op, temp, sweeps) | consecutive step pairs (ridge ×4) | imagined rollouts → schedule ranking, yield signal for the Meta-Layer |

### 3.4 Science (Stratum D — exists)

| Component | Role |
|---|---|
| **AI Scientist / Scientific Lab** (`scientist.rs`, `lab.rs`) | Four agents — HypothesisGenerator, ExperimentDesigner, Statistician, KnowledgeManager — run the discovery round: propose → design (novelty-filtered) → execute (via Runtime) → judge (Welch + CI) → record. |
| **Meta-Learner** (`meta_learner.rs`) | Mine statistically-supported rules from the DB (dominance/conditional/ordering/antipattern/useless/temperature/budget/backend); `publish()` consistent conditional facts to the graph. |
| **Meta-Learning Layer** (`meta_layer.rs`) | **The consensus bus.** Consolidate per-operator signals from Meta-Learner + Policy + World + Dynamics into one verdict (prefer/avoid/switch-early) with source attribution; publish to the graph; bias the ideator. |
| **Local LLM** (`llm.rs`) | Ollama/Qwen `Ideator`: reads the research brief, proposes reasoned operator sequences; graceful heuristic fallback. |
| **Cloud LLM** (`cloud.rs`) | Claude Messages API: periodic deep scientific analysis over reports + graph + aggregates; honest skip without a key. |
| **Reporter set** (`reports.rs`, `evaluation.rs`, `dashboard.rs`, `proposal.rs`) | Research reports (every N exp), self-evaluation (reproducibility + predictor accuracy), self-contained dashboard, operator-proposal drafts. |
| **Campaign Manager** (`campaign.rs`) | Today's driver: runs generations, stamps provenance, trains Predictor (+ optional Meta-Layer), reports, checkpoints, resumes. |

### 3.5 Operations (Stratum E — **Stage 7, new**)

| Component | Role | Owns | Must-not |
|---|---|---|---|
| **Platform Kernel / Orchestrator** | The always-on supervisor loop: pull the next job from the Scheduler, run it via the Campaign Manager, hand results to the Monitor and Reporter, repeat forever (or until budget/stop). | The run-loop, the shutdown/resume protocol. | Compute energies; bypass the Monitor's kill signal. |
| **Task Scheduler** | A priority job queue of campaigns/experiments with triggers, cadences, and per-job budgets. Decides *what to research next*. | The queue, job state, seeds. | Duplicate work already in the DB; exceed the global budget. |
| **Resource / LLM Manager** | Govern local Ollama (frequent, background) + cloud (rare, expensive): availability probing, rate/cost budgets, fallback, warm-pool. | Tokens/calls ledgers, tier availability. | Fabricate a result when a tier is down; exceed a period budget. |
| **Monitor / Health** | Continuously check invariants (determinism replay, golden regression, ledger audit=0), model health (held-out accuracy), campaign progress vs kill criteria, and resource usage; raise alerts; pause/kill jobs. | Metrics time-series, alerts. | Silence a violated invariant. |
| **Reporter** | The unified reporting bus: schedule and consolidate the report set (dashboard, research report, evaluation, deep analysis, proposals) and expose a single index. | Report cadence, artifact index. | Overwrite history; publish an unverified number without its provenance. |
| **Model Registry / Artifact Store** | Versioned, lineage-stamped storage of trained model weights (Policy/Dynamics/World/Predictor) with the DB-snapshot and metrics they were trained on. | Model versions, lineage. | Overwrite a version; serve a model without its metrics. |
| **Insight Bus** | The formalized publish/subscribe over the graph's consensus facts (Meta-Layer), so Policy/World/Evolution/LLM *subscribe* rather than each re-deriving. | Subscription routing. | Let a subscriber mutate another's facts. |

---

## 4. Data flows (the buses)

Six typed buses connect everything. Writers and readers are explicit so no
component reaches into another's internals.

```
 BUS                     WRITER(S)                         READER(S)
 ───────────────────────────────────────────────────────────────────────────────
 1 Experiment stream     Runtime→Executor→ExperimentDB     ALL analysis + all models
   (append-only)         (provenance-stamped rows)         (train), Monitor, Reporter
 2 Knowledge graph       Meta-Learner.publish,             LLM (ResearchMemory), Reports,
   (conditional facts)   Meta-Layer.publish,               Decision Engine, Insight Bus
                         KnowledgeManager
 3 Model artifacts NEW   model trainers → Model Registry   Ideators, controllers, Meta-Layer
 4 Insight/consensus     Meta-Layer (per-op verdicts)      MetaBiasedIdeator, Policy bias,
   (Insight Bus)                                           Evolution seeds, LLM prompt
 5 Reports / analyses    Reporter (MD/HTML/JSON)           humans, Cloud LLM, Monitor
 6 Control / commands    Scheduler→Orchestrator,           Campaign Manager, Runtime
   NEW                   Monitor→Scheduler (kill/pause),   (RunController), LLM Manager
                         Resource Mgr→tiers
```

Key property: **the experiment stream is the single source of truth.** Every
model, rule, report, and insight is *derived* from it and can be recomputed. If
a model version is ever in doubt, retrain from the append-only stream at the
recorded DB snapshot — nothing important lives only in weights.

---

## 5. The self-learning cycle

The closed loop the platform runs, one **generation** at a time (existing steps
marked ✓, Stage 7 additions marked ＋):

```
        ┌──────────────────────────────────────────────────────────────┐
        │                                                              ▼
 (1) Scheduler picks the next instance/regime to study  ＋
        │
        ▼
 (2) Decision Engine analyzes structure → backend + baseline plan   ✓
        │
        ▼
 (3) Ideators propose schedules:  AI-Scientist agents ✓ + Local LLM ✓
        │                          biased by the Insight Bus (Meta-Layer) ✓
        ▼
 (4) Predictor filters candidates ✓;  World model imagines & ranks them ＋
        │  (run only the promising few → compute saved)
        ▼
 (5) Evolution expands survivors into variants   ✓
        │
        ▼
 (6) RUNTIME executes them (the ONLY judge)   ✓
        │   Dynamics early-stop controller trims dead runs ＋
        ▼
 (7) Every result appended to the Experiment DB (provenance)   ✓
        │
        ▼
 (8) Statistician confirms/refutes; KnowledgeManager writes graph facts   ✓
        │
        ▼
 (9) Models RETRAIN on the grown history: Predictor ✓, Policy/World/Dynamics ＋→Registry
        │
        ▼
 (10) Meta-Learner mines rules ✓; Meta-Layer consolidates cross-model consensus ✓
        │   → publishes attributed facts to the graph → Insight Bus
        ▼
 (11) Reporter emits: dashboard (every gen) ✓, research report (every N) ✓,
        │  evaluation (reproducibility + predictor accuracy) ✓
        ▼
 (12) Cloud LLM (every M experiments) reads reports+graph → deep analysis
        │  + proposes MAJOR improvements / new operators ✓ → operator draft ✓
        ▼
 (13) Monitor checks invariants + progress; Scheduler updates the queue ＋
        └──────────────────────────────────────────────────────────────┘  (loop)
```

**The propagation example, concretely** (the user's scenario, mapped to modules):

1. **Dynamics** learns "metropolis_sweep plateaus by ~40% of budget" → `plateau_frac` low → `dynamics_plateau[metropolis]` small.
2. **Meta-Layer** consolidates it into a `switch-early` verdict and publishes `metropolis_sweep --plateaus-early--> true IF density<0.05` to the graph.
3. **Policy**, retrained next generation on data where long-metropolis runs under-performed, lowers `first_op_probs[metropolis]` for that regime.
4. **World** model's imagined rollouts stop crediting extra metropolis sweeps, so it ranks metropolis-heavy schedules lower → they are pruned before the Runtime.
5. **Cloud LLM** reads the report ("metropolis front-loads on sparse; the tail is wasted") and proposes a **fused operator** or an early-switch schedule; `OperatorProposal` drafts it.
6. **Evolution** generates variants of the endorsed shape; **Runtime** checks them; the cycle repeats with the new evidence.

No component *told* another what to do. Each learned from the shared record, and
the Meta-Layer made the agreement explicit and auditable.

---

## 6. Automation & the Task Scheduler

Today: campaigns run in CLI order, then the process exits. Stage 7 makes
scheduling first-class.

**Job model.** A `Job` = `{ instance-or-regime, budget (experiments/wall/tokens),
priority, trigger, seeds, model-version pin }`. Jobs are append-logged so the
queue is itself reproducible.

**Triggers / cadences** (all budget-accounted):

| Trigger | Fires | Action |
|---|---|---|
| every generation | — | retrain Predictor; dashboard refresh |
| every campaign | — | retrain Policy/World/Dynamics → Registry; Meta-Layer consolidate + publish |
| every `report_every` experiments | e.g. 5 000 | research report + operator-gap proposal |
| every `cloud_every` experiments | e.g. 10 000 | cloud deep analysis (if tier available) |
| coverage gap | a regime under-sampled | enqueue a campaign there |
| curiosity | novelty archive stalls | enqueue a high-temperature exploration campaign |
| regression | golden/replay check fails | **pause queue**, alert, no new jobs |
| new operator merged | Registry sees a new passport | enqueue campaigns that include it (does it get used?) |

**Scheduling policy.** Priority = `value_estimate / cost_estimate`, where value
comes from the Predictor's expected improvement and the coverage deficit, and
cost from the World model's imagined budget. This makes the platform spend the
Runtime's time where the models jointly predict the most learning per sweep —
and, crucially, the *actual* value is always measured, never assumed.

---

## 7. Local & cloud LLM management (the two tiers)

The Constitution's two-tier rule (§11/§12): a **frequent cheap** local model and
a **rare expensive** cloud model, both proposers only.

```
 ┌──────────────────────────── Resource / LLM Manager ───────────────────────────┐
 │                                                                                │
 │  LOCAL  (Ollama / Qwen2.5-Coder)          CLOUD  (Claude Messages API)         │
 │  role:  idea + hypothesis generation      role:  deep architecture review,     │
 │         in the background, per round             major-improvement proposals   │
 │  cadence: every ideation round            cadence: every ~10 000 experiments   │
 │  cost:  local compute, ~free              cost:   $ / tokens — budget-capped   │
 │  input: research brief (structure,        input:  cumulative reports + graph   │
 │         operator menu, graph facts,               + aggregates (NOT raw DB)     │
 │         Insight-Bus consensus)            output: prose analysis + proposals   │
 │  fallback: → deterministic heuristic      fallback: → skip, disclosed          │
 │  availability: probe `ollama`             availability: probe ANTHROPIC_API_KEY │
 └────────────────────────────────────────────────────────────────────────────────┘
 GOVERNANCE (Stage 7):
  • availability probe before every call → honest skip, never fabricate  (exists)
  • per-period budget: max calls + max tokens; Monitor enforces          ＋
  • warm pool / async: local runs in the BACKGROUND, decoupled from the  ＋
    experiment loop, so ideas queue up without blocking the Runtime
  • context discipline: LLMs read DISTILLED knowledge (reports + graph +  (exists)
    consensus), never the multi-million-row raw DB — bounded context
  • provenance: every LLM-originated hypothesis/proposal is tagged with   (exists)
    its tier + model, so its downstream results are attributable
```

**Why two tiers.** The local model is the tireless junior scientist — it never
sleeps, generates hundreds of reasoned ideas, and costs nothing but GPU time.
The cloud model is the visiting professor — consulted rarely, given only the
distilled state, and asked the expensive question: *"what's the mechanism, what's
an artifact, and what new operator or direction should we try?"* Neither ever
touches an energy value.

---

## 8. Monitoring & health

The Monitor is the platform's immune system. It watches four classes of signal
and has exactly one power: it can **pause the Scheduler and raise an alert.** It
never edits results.

| Class | Signal | Green | Action on red |
|---|---|---|---|
| **Correctness (hard)** | golden regression, backend replay bit-identity, `audit()` ledger drift | pass / ==0 | **halt queue**, alert — this is a defect, not a tuning issue |
| **Determinism** | re-run a sampled `(instance,seed,plan,model-ver)` → identical | identical | halt queue, alert (ADR-0004 breach) |
| **Model health** | Predictor held-out Spearman, World rank-correlation, Dynamics early-stop quality-Δ, Policy held-out win-rate | above floor | demote the model to advisory-only; keep running on the Runtime's verdict |
| **Progress / cost** | best-energy trend, novelty rate, RSS, wall, LLM tokens vs budget | improving / under budget | apply kill criteria; re-prioritize; throttle the offending tier |

The self-evaluation already implemented (`evaluation.rs`: rule reproducibility
across instances + leave-one-instance-out predictor accuracy) is the seed of the
model-health track. Stage 7 schedules it and adds the correctness/determinism
gates as continuous checks, not just pre-commit.

---

## 9. Reporting system

Layered, cadenced, provenance-stamped — every number carries how it was
produced. All artifacts are append-only; the Reporter maintains a single index.

| Artifact | Cadence | Audience | Exists |
|---|---|---|---|
| **Dashboard** (`dashboard.html`) | every generation | human, at a glance | ✓ |
| **Research report** (`report_*.md`) | every N experiments | human + local LLM memory | ✓ |
| **Evaluation report** | on demand / per phase | human — *is the platform learning?* | ✓ |
| **Deep analysis** (`deep_analysis_*.md`) | every M experiments | human — mechanism + directions | ✓ (cloud tier) |
| **Operator proposal** (`operator_proposal_*.md`) | on mined gap / LLM idea | human review before implementation | ✓ |
| **Consensus** (Insight Bus → graph → report) | every campaign | Policy/World/LLM + human | ✓ (Meta-Layer) |
| **Platform status** (queue, health, budgets) | continuous | human ops | ＋ Stage 7 |

The reporting invariant: **a report never publishes an unverified number.** A
claimed win carries its seeds, budget, and baseline; a skipped tier is stated;
an artifact of a probe (e.g. the cold-start World-model AVOID bias, §11) is
disclosed. Honesty is a reporting requirement, not a nicety.

---

## 10. Interactions — sequences

**A campaign tick** (Orchestrator drives one campaign):

```
Scheduler ──job──▶ Orchestrator ──▶ CampaignManager.run
                                     │  Decision Engine → plan
                                     │  Ideators (Lab agents + Local LLM),
                                     │    biased by Insight Bus
                                     │  Predictor filter + World-rank (＋)
                                     │  Evolution → schedules
                                     │  Runtime executes (Dynamics early-stop ＋)
                                     │  DB append (provenance)
                                     │  Statistician + KnowledgeManager → graph
                                     │  retrain models → Registry (＋)
                                     │  Meta-Layer consolidate + publish
                                     ▼
                     Reporter (dashboard, report?, proposal?, cloud?) 
                                     ▼
                     Monitor (invariants, health, budget) ──ok/kill──▶ Scheduler
```

**A cloud-review tick** (every M experiments, if the tier is available):

```
Reporter ── distilled(reports+graph+aggregates) ──▶ Resource Mgr (budget ok?)
                                                    │ yes
                                                    ▼
                                          Cloud LLM (Claude) → deep_analysis_*.md
                                                    │  proposals
                                                    ▼
                                          OperatorProposal draft + graph note
                                                    ▼
                                          Scheduler enqueues follow-up campaigns
```

**A monitor intervention** (a hard-correctness failure):

```
Monitor: golden regression FAILS ──▶ Scheduler.pause()
                                 ──▶ alert (status report + Stop-hook surfacing)
                                 ──▶ NO new jobs until a human/Claude-Code fix + green gate
```

---

## 11. What exists today vs. what Stage 7 adds

Honest gap table — so the design is a roadmap, not a claim of completion.

| Capability | Today (Stage 6) | Stage 7 adds |
|---|---|---|
| Run campaigns | ✓ `research_platform`, sequential, then exits | Persistent Orchestrator loop (service) |
| Choose what to study | CLI arg (`--file`) | Priority Scheduler with triggers, coverage/curiosity |
| Train the 4 models | Predictor per campaign; Policy/World/Dynamics only in bins + opt-in Meta-Layer | Scheduled retraining → **versioned Model Registry** |
| Model lineage | none (weights discarded) | Registry: version + DB-snapshot + metrics |
| World-model pre-filter of candidates | tested (`rank_correlation`), not in the loop | wire imagine→rank→run-top-few into ideation |
| Dynamics early-stop | wired into Runtime, demonstrated | wire into the executor's every-run path (config) |
| Cross-model consensus | ✓ Meta-Layer, opt-in `--shared-knowledge` | promote to default; formal Insight Bus |
| Local LLM | ✓ synchronous per round | background/async warm pool |
| Cloud LLM | ✓ cadenced deep analysis | budget governor + follow-up enqueue |
| Monitoring | tests + `/verify` + evaluation report | continuous invariant + health + budget Monitor |
| Reporting | dashboard/report/eval/analysis/proposal | scheduled + unified index + platform status |

Known artifacts to fix as part of Stage 7 (already disclosed in the code/memory):
- The Meta-Layer's World/Dynamics probes run single-operator schedules **from the all-zero cold start**, so ensemble/cluster/PT operators are under-valued and over-AVOIDed. Fix: warm-start the probe (a short thermal pass) before measuring per-operator yield.
- Consensus facts publish at support=1 → low graph-confidence → may not surface to the LLM until accumulated across campaigns. Fix: a dedicated confidence for consensus facts, or surface them by predicate regardless of support.

---

## 12. Build order (incremental, each step verifiable)

Every step preserves the green gate (fmt + clippy + full tests + golden
regression) and ships behind a flag before becoming default. Proposed ADRs in
brackets.

1. **Model Registry** — persist/version Policy/World/Dynamics/Predictor with lineage. [ADR-0009] Small, unblocks everything.
2. **Orchestrator + Scheduler (minimal)** — a persistent loop over a priority job queue; start with the existing trigger cadences made explicit. [ADR-0010]
3. **Monitor (hard gates first)** — continuous golden + replay + ledger checks; pause-on-red. Correctness before cleverness.
4. **Wire the loop tighter** — World-model pre-filter into ideation; Dynamics early-stop into the executor path; Meta-Layer to default. (Fix the cold-start probe bias here.)
5. **Resource / LLM Manager** — budgets, availability, async local pool. [ADR-0011]
6. **Reporter unification + platform status** — one index, scheduled cadences.
7. **Curiosity / coverage triggers** — the Scheduler starts choosing its own next questions.

Each step is independently useful; the platform is operable after step 2 and
self-directing after step 7.

---

## 13. Risks & failure modes (and how the design contains them)

| Risk | Containment |
|---|---|
| A learned model biases the search into a rut (feedback loop on its own outputs) | The Runtime always measures; the Scheduler keeps a random/curiosity fraction; the Monitor watches novelty rate; models are advisory, demotable. |
| Selection bias in the corpus (models fluent only where they already explored) | Permanent random-schedule control fraction logged into the DB; coverage triggers. |
| Cost blow-up (LLM tokens, compute) | Per-period budgets enforced by the Monitor; cloud is rare by cadence; local is free-ish and async. |
| A correctness regression slips in under automation | Continuous golden + replay + ledger gates halt the queue; no new science on a red tree. |
| Model weights become a hidden source of truth | Everything derivable from the append-only stream; Registry stores the snapshot to retrain from. |
| Over-claiming (a report asserts superiority) | Reporting invariant: seeds + budget + baseline or it doesn't ship; A/B vs UltimateSolver stays the arbiter of production claims. |

---

## 14. One-paragraph summary

Stage 7 is the **operating system** for the research organs Stage 6 built. A
persistent Orchestrator pulls jobs from a priority Scheduler; each job runs the
existing generation loop (Decision → Ideators biased by the Insight Bus →
Predictor/World filter → Evolution → Runtime → DB), retrains the four models
into a versioned Registry, consolidates their cross-model consensus via the
Meta-Layer, and hands artifacts to a scheduled Reporter and a continuous Monitor.
A two-tier LLM manager runs a free local idea-generator in the background and a
budgeted cloud reviewer periodically — both proposers, never judges. The
Runtime remains the single, deterministic, read-only source of truth; every
layer above only proposes, analyzes, and learns — under budget, with provenance,
and with the Monitor holding a hard line on correctness. The result is a lab
that runs itself: it decides what to study, learns from every run, writes down
what it found, proposes what to build next, and never lies about a number.
