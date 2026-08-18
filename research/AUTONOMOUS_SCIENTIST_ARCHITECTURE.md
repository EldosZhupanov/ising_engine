# AUTONOMOUS_SCIENTIST_ARCHITECTURE.md

> The platform re-evaluated against one question only: *does it form hypotheses,
> test them, refute itself, and set its own agenda — without a human in the loop?*
> PART 4 scores what exists. PART 7 designs the target. Governed by SOUL
> §"An AI Scientist worthy of the name" and Constitution §12–13.

---

# PART 4 — Subsystem scorecard (against the autonomous-discovery vision)

Score = how much of the *autonomous-scientist* job this subsystem actually does
today (not code quality, not benchmark score). E = Exists & works · P = Partial ·
D = Designed-not-built · M = Missing.

| Subsystem | Module(s) | State | Score /10 | Honest assessment |
|---|---|---|---|---|
| **Experiment Runner** | `executor.rs`, `Runtime`, `BatchExecutor` | E | **9** | The crown jewel. Deterministic, cross-backend bit-identical, delegated-through. Everything real depends on it. |
| **Operator Registry** | `registry.rs` (14 ops + passports) | E | **8** | Capability-selected, not name-selected. Solid — but a *fixed* set. The discovery frontier (new operators) lives here and is closed. |
| **Statistics** | `stats.rs`, `benchmark/stats.rs` | E | **8** | Welch/Wilcoxon/paired-t/bootstrap present and used. Adequate for the science. |
| **Knowledge Memory** | `db.rs`, `graph.rs`, `memory_os.rs` | E | **7** | Append-only DB + conditional evidence-weighted graph + structure-keyed recall. Strong design; **under-fed** (recent findings bypass it — see gaps PART 3) and low-confidence (support≈1 until many campaigns). |
| **Investigator / Theory** | `theory.rs`, `investigate_operator` | P | **6** | Real Popperian ablation (mechanism→prediction→Runtime ablation→refutation). But mostly single-shot / human-triggered; confidence stays low without many-instance runs. |
| **Research Platform (loop)** | `campaign.rs`, `orchestrator.rs` | E | **7** | The closed generational loop exists and is resumable. Runs; but its *ideation* is narrow (schedule search), and it does not yet pursue *open* questions. |
| **Planner** | `planner.rs` | P | **6** | Picks next *instance* by expected new knowledge. Good — but plans instances, not *questions/hypotheses*. |
| **Executive (Chief Scientist)** | `executive.rs` | P | **6** | Decides what to retrain, local-vs-cloud routing, publish-vs-gather. Real and wired (`--executive`). Governs *operations*, not a *research agenda*. |
| **Hypothesis Generator** | `lab.rs`, `llm.rs`, `proposal.rs`, `curiosity.rs` | P | **5** | Generates *schedule* hypotheses and *prose* operator drafts. Cannot generate a falsifiable *scientific question* about mechanism, nor operator *code*. |
| **Learned Models** | `predictor/policy/dynamics/world.rs` | P | **5** | Four small models (ridge/MLP) that work and transfer weakly. Honestly labelled as pre-foundation. Fine for now. |
| **Red Team / Self-Critic** | (spread across `theory`, `evaluation`, `novelty`) | P | **4** | Ablation + reproducibility audit + novelty exist, but there is **no dedicated adversarial agent** that tries to *break every new finding* (confounds, leakage, artifacts) the way this session's human red-teaming did. |
| **Publication Pipeline** | `reports.rs`, `dashboard.rs` | P | **4** | Renders reports + a dashboard. Does not assemble a *paper-grade, self-contained, falsification-first writeup* of a finding with its evidence and refutations. |
| **Families / Instance Generation** | `families.rs` | P | **4** | TSP/SAT/coloring/MIS/partition, proven correct. But generation is *fixed-parameter*; no **controlled generation** (vary one landscape axis all-else-equal) — the thing O2/causal science needs. |
| **Research Agenda** | — | M | **1** | **Missing.** No persistent, self-rewriting list of open questions ranked by expected knowledge gain. The Planner picks instances; nothing picks *questions*. |
| **Algorithm/Operator Generator** | `proposal.rs` (prose only) | D | **2** | **The mission-critical hole.** Proposes operators as text; writes no code; nothing compiles/tests/keeps a synthesized operator. |

**Where the platform is strong:** the *substrate and the discipline* — runner,
determinism, registry, append-only knowledge, ablation. These are exactly the parts
that do not depreciate (Constitution §1).

**Where it is hollow (against the vision):** the *generative and self-directing*
parts — a real hypothesis/question generator, a self-rewriting agenda, an adversarial
self-critic, controlled instance generation, an operator *synthesizer*, and a
paper-grade publication step. The platform can *run* science handed to it; it cannot
yet *decide what science to do* or *invent the objects it studies*.

## PART 4b — Remaining named subsystems (completing the scorecard)

| Subsystem | Module(s) | State | Score /10 | Assessment |
|---|---|---|---|---|
| **Evolution Engine** | `evolution.rs` | P | **4** | Genetic search over *sequences of the 14 fixed operators* (`Genome = Vec<usize>`). This is hyper-heuristics (KNOWN); it discovers *orderings*, never new dynamics. Solid engineering, wrong altitude for "discovery." |
| **Lab Agent** | `lab.rs` (HypothesisGenerator, ExperimentDesigner, Statistician, KnowledgeManager) | P | **5** | Multi-agent scaffold exists and runs; but its hypotheses are *schedule* hypotheses, and its "designs" are not pre-registered with explicit failure criteria. The skeleton of the right thing. |
| **Question Generator** | (none dedicated; latent in `curiosity.rs`, `planner.rs`) | M | **2** | No component emits a *falsifiable scientific question* with success/failure criteria. Curiosity emits surprise signals; the Planner emits instances. The question layer is missing. |
| **Novelty Detector** | `novelty.rs` | P | **4** | Detects novelty of *schedules/behaviors* against history — but not against the **external literature**. It cannot tell KNOWN from NEW (the exact failure this whole audit corrects). Needs a literature-grounded Reviewer on top. |
| **Foundry (instance generation)** | `families.rs` | P | **4** | Correct fixed-parameter generators (proven vs brute force). Cannot yet do *controlled* generation (vary one landscape axis all-else-equal) — the capability O2/causal science requires. |
| **Generator (operator code)** | `proposal.rs` | D | **2** | Writes prose operator drafts, *no code*, by design. The single most mission-critical hole. |
| **Paper Writer / Publication** | `reports.rs`, `dashboard.rs` | P | **4** | Renders reports and a dashboard; does not assemble a falsification-first, literature-aware, reproducible writeup of a finding. |

**Aggregate reading.** Substrate & discipline layers average ~8/10; the *generative,
self-directing, literature-aware* layers average ~3/10. The platform is a superb
laboratory with a weak scientist inside it. Every low score is a component that decides
*what to study* or *invents what it studies* — exactly the mission.

---

# PART 7 — Design of the Autonomous Scientist

## Design principles (inherited, non-negotiable)

1. **Delegate all computation to the deterministic Runtime.** Agents *generate and
   analyse*; the `BatchExecutor` owns execution (Constitution §7). Never reach around it.
2. **Append-only, source-attributed knowledge.** Every claim is conditional, evidenced,
   and reproducible from a seed. No bare facts, no deletions (§12).
3. **Falsification is the product.** An agent's output is judged by what it *refutes*.
   Nothing is "confirmed" without a survived attempt to break it (§13).
4. **Discovery over selection.** The system's purpose is to *pose questions and invent
   objects*, not to optimise a human-given target.
5. **Every synthesized artifact passes the verification firewall.** New operators enter
   only through capability passports + bit-identical cross-backend checks + golden regression.

## The loop (what "autonomous" means, concretely)

```
             ┌────────────────────── RESEARCH AGENDA (self-rewriting) ──────────────────────┐
             │  ranked open questions  ·  expected-knowledge-gain  ·  refutation debts       │
             └───────────┬───────────────────────────────────────────────────────┬─────────┘
                         │ picks the next QUESTION (not just instance)             │ updates
                         ▼                                                         │
   ┌──────────────┐   ┌───────────────┐   ┌────────────────┐   ┌───────────────┐  │
   │ IDEA          │→ │ EXPERIMENT     │→ │ INSTANCE        │→ │ EXPERIMENT     │  │
   │ GENERATOR     │  │ DESIGNER       │  │ FOUNDRY         │  │ RUNNER         │  │
   │ (hypotheses,  │  │ (metrics,      │  │ (controlled     │  │ (deterministic │  │
   │  operator     │  │  controls,     │  │  generation:    │  │  Runtime,      │  │
   │  code)        │  │  success/fail, │  │  vary ONE axis  │  │  BatchExecutor)│  │
   │               │  │  pre-register) │  │  all-else-equal)│  │                │  │
   └──────────────┘   └───────────────┘   └────────────────┘   └──────┬────────┘  │
                                                                       ▼           │
   ┌──────────────┐   ┌───────────────┐   ┌────────────────┐   ┌───────────────┐  │
   │ PUBLICATION   │← │ REVIEWER       │← │ RED TEAM        │← │ ANALYZER +     │  │
   │ (paper-grade  │  │ (is it novel   │  │ (confounds,     │  │ STATISTICIAN   │  │
   │  writeup +    │  │  vs LITERATURE │  │  leakage,       │  │ (Welch/Wilcox/ │  │
   │  evidence +   │  │  & our graph?) │  │  artifacts,     │  │  bootstrap,    │──┘
   │  refutations) │  │                │  │  ablation)      │  │  effect sizes) │
   └──────┬───────┘   └───────────────┘   └────────────────┘   └───────────────┘
          ▼
   KNOWLEDGE GRAPH  +  EXPERIMENT DB  +  SCIENTIFIC MEMORY  (append-only)  ──► feeds AGENDA
```

## Agent roster (target ← today)

| Agent | Responsibility | Maps to / becomes |
|---|---|---|
| **Research Agenda** *(new)* | Persistent, ranked, **self-rewriting** list of open questions + refutation debts; the true "what next." | new `agenda.rs` on top of `graph`+`memory_os`+`curiosity` |
| **Idea Generator** | Emit *falsifiable questions* and *candidate operator programs*, not just schedules/prose. | extend `lab`+`llm`+`proposal` → add code-emitting `OperatorSynthesizer` |
| **Experiment Designer** | Turn a question into a **pre-registered** design: metrics, controls, independent/dependent vars, success & failure criteria. | extend `lab.rs` (add pre-registration record) |
| **Instance Foundry** *(new capability)* | **Controlled generation**: produce instance families varying ONE landscape axis all-else-equal (the O2/causal engine). | extend `families.rs` with parameterised landscape targeting |
| **Experiment Runner** | Deterministic execution. | `executor.rs` / `Runtime` — **keep as-is** |
| **Analyzer + Statistician** | Effect sizes, CIs, proper tests; no eyeballing. | `stats.rs` — wire into the loop's verdict |
| **Red Team** *(elevate)* | Adversarially attack every finding: confounds, leakage, between-group artifacts, selection bias; demand an ablation. | promote scattered checks → dedicated `redteam.rs` |
| **Reviewer** | Is it **novel vs the literature and our own graph**, or a re-discovery? Tag KNOWN/NEW. | new `reviewer.rs` seeded with a literature table (gaps PART 5) |
| **Publication** | Paper-grade, falsification-first writeup with evidence + kept refutations. | extend `reports.rs` |
| **Operator/Algorithm Generator** | Synthesize, compile, verify, ablate, and *keep or kill* new operators. | close the loop `proposal.rs` → `OperatorSynthesizer` + verification firewall |
| **Executive / Chief Scientist** | Allocate compute, retrain stale models, route local/cloud, set priorities. | `executive.rs` — **keep, extend to own the Agenda** |
| **Knowledge Graph / Memory / DB** | Append-only, conditional, source-attributed store; structure-keyed recall. | `graph`/`memory_os`/`db` — **keep, and actually feed** |

## The two structural additions that unlock the mission

1. **A Research Agenda that rewrites itself.** Without it, "autonomous" is a loop that
   repeats a fixed task. With it, the system chooses questions by expected knowledge
   gain, carries *refutation debts* (claims at support≈1 that need re-testing), and
   retires closed questions. This is the difference between a cron job and a scientist.

2. **An Operator Synthesizer behind the verification firewall.** This is the only
   addition that turns *selection* into *discovery*. It must: (a) sample operator
   programs over a typed sub-API of `SpinState`; (b) compile in a sandbox; (c) reject
   any that fail bit-identical cross-backend checks or the golden regression; (d) test
   behavioral **non-redundancy** vs the existing 14; (e) hand survivors to the Theory
   Engine for ablation; (f) register only the ones that *contribute causally*. Anything
   less is hyper-heuristics (KNOWN).

Everything else on the roster already exists in some form; these two are the load-bearing
new walls. The roadmap builds toward them deliberately, evidence-gated, in that order.
