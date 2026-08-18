# Concept Discovery Engine + Serena / Graphiti / Obsidian — Architecture & Audit

> Chief-Scientist design memo. Audit-first, per the mandate: no subsystem is added
> until the five-question duplication test proves it is not a rebuild. Governed by
> the Constitution (determinism ADR-0004, minimal deps, self-contained,
> recomputable-from-stream STAGE_8 §3) and the standing principle: **never
> duplicate; always extend; always integrate.** No code is written until the
> representation-centralization prerequisite (§4) lands.

---

## 1. The duplication audit (the gate)

Each proposed integration against: (1) equivalent exists? (2) extend? (3) reuse?
(4) replace? (5) introduces duplication? — with a verdict.

### 1.1 Concept Discovery Engine — **BUILD** (the one true new layer)
1. Equivalent? **No.** Meta-Learner mines rules *over* fixed features; Predictor
   fits *over* fixed features; Curiosity buckets *over* fixed features. **Nothing
   invents features.** The system's most important finding to date — "density is a
   dead predictor, ruggedness is the driver" — was made by a *human*, because no
   faculty can nominate a new structural axis. That is the gap.
2. Extend? It **sits upstream** of all faculties and feeds them a grown vocabulary.
3. Reuse? Heavily — Predictor + Meta-Learner as the **fitness test**, ExperimentDb
   as **evidence**, the `--structural` probes (ruggedness/frustration/autocorr/
   spectral/entropy) as the **primitive concept library**, KnowledgeGraph as the
   **publication target**.
4. Replace? No — it makes the others *smarter inputs*, replaces nothing.
5. Duplication? **Only if** it re-mines rules / ranks instances / runs ablations.
   It must do **none** of those. It proposes → tests → admits *features*. Clean.

**Verdict: build it — upstream, reuse-only, no duplication.**

### 1.2 Serena — **ADOPT AS DEVELOPER TOOLING, NOT A PRODUCT SUBSYSTEM**
1. Equivalent? Partially — `.claude/context/{tree,modules,symbols,impls}` and
   `INDEX.md` already index the repo; the `Explore` agent already searches it.
   Serena adds LSP-grade semantic navigation on top.
2. Extend/reuse? It is an **MCP server** — a Claude-Code *tool*, not Rust code. It
   integrates with **none** of the scientist's subsystems.
3. Duplication (in the product)? None — because it never enters the product.
4. **Honest reframing:** Serena makes *Claude* navigate faster; it does **not** make
   *the scientist* more intelligent. It is orthogonal to the mission. Useful, but do
   not file it under "cognitive architecture of the scientist" — it is IDE-grade
   tooling for the human/agent editing the repo.

**Verdict: adopt as an MCP navigation aid (config, not code). Clearly labelled as
dev tooling. It is not part of the scientist.**

### 1.3 Graphiti — **STOP. REFACTOR THE EXISTING GRAPH INSTEAD.**
1. Equivalent? **Yes, substantially.** The mandate's Graphiti ladder
   (Experiment→Hypothesis→Concept→Theory→Evidence→Contradiction→Refutation→Derived
   Concept→Research Program) is **already the STAGE_8 knowledge ladder**, and the
   existing `KnowledgeGraph` already stores conditional, evidence-weighted,
   source-attributed facts with Welford confidence, over an **append-only, timestamped,
   recomputable** stream (ExperimentDb). Contradictions/refutations are already
   first-class (Theory Engine keeps refuted theories).
2. Extend? Yes — the ladder's **missing rungs** (typed `Concept` and `ResearchProgram`
   nodes above facts) are an *extension of the existing graph*, which is exactly what
   the Concept Discovery Engine needs anyway.
3. Reuse? The existing graph is reusable and already governed by the Constitution.
4. Replace? No.
5. **Duplication? Severe — and constitutionally disqualifying.** Graphiti is a
   **Python service + Neo4j database + LLM-based entity extraction**. Adopting it as
   "the layer above the graph" means: a second knowledge store, in a second language/
   runtime, behind a database server, populated by **non-deterministic LLM
   extraction**. That violates ADR-0004 (determinism/replay), the minimal-dependency
   rule, the self-contained/honest-skip principle, and STAGE_8 §3 (*everything
   recomputable from the stream*). It re-creates the exact silo problem this project
   already suffered — at 10× the surface.

**Verdict: STOP. Do not adopt Graphiti. Capture its genuine value — a typed
abstraction ladder with temporal contradiction/refutation tracking — by *extending
the existing deterministic KnowledgeGraph* with `Concept` / `Theory` / `ResearchProgram`
node types over its existing append-only timeline. Refactor, do not bolt on.**

### 1.4 Obsidian vault — **ADOPT AS A GENERATED VIEW ONLY**
1. Equivalent? Partially — `reports.rs` emits markdown, `dashboard.rs` emits HTML,
   `dataset.rs` exports the corpus. A browsable, Dataview-queried vault over *all*
   experiments/theories/concepts does not yet exist.
2. Extend/reuse? **Yes** — extend `reports.rs`/`dataset.rs` to *emit vault-shaped
   markdown* (frontmatter Dataview can index). Git-native, deterministic,
   self-contained; adds **no runtime dependency** to the Rust product (Obsidian is a
   viewer over markdown the engine already produces).
3. Duplication? **Only if the vault becomes a second source of truth.** Contained by
   making it **one-directional**: stores → vault (regenerable, never authored-in).
4. Replace? It can replace ad-hoc report scatter with one coherent, queryable view.

**Verdict: adopt as a *generated, one-directional view* of the append-only stores.
Refuted theories stay visible; nothing disappears; markdown stays Git-native.
Never a store — always a projection.**

---

## 2. Architecture proposal

Two moves, in order. The first is a prerequisite refactor; the second is the new layer.

### 2.1 Prerequisite: centralize the representation (extend, don't add)
Today the "vocabulary of thought" is the fixed `InstanceSignature{n, density,
clustering, mean_degree, degree_cv}`, and density/degree features are recomputed
**ad hoc in ~10 modules** (scientist, curiosity, memory_os, campaign, evaluation,
dashboard, knowledge, cloud, capability, research_log). While it is fixed-width and
scattered, **no admitted concept can propagate**. So:

- Promote the representation to a single **`FeatureRegistry`** — a *versioned,
  extensible* ordered list of named, deterministic `Concept` functions
  `ProblemIR → f64` (the current 5 scalars become the seed concepts v0). One place
  computes features; every faculty reads the registry, not a hardcoded struct.
- This is an **extension/refactor** of the existing representation, not a new brain.
  It replaces the scattered computations and the fixed `InstanceSignature` width.

### 2.2 The Concept Discovery Engine (upstream cognitive layer)
A single loop, reusing existing organs:

```
 ExperimentDb (evidence, existing)
        │  propose
        ▼
 CANDIDATE CONCEPT  = a deterministic ProblemIR→f64 program, composed from a
   (feature program)  PRIMITIVE LIBRARY = the existing --structural probes
                      (ruggedness, frustration, autocorrelation, spectral gap,
                       minima-entropy, funnel) + graph statistics + compositions
        │  test (REUSE, do not rebuild)
        ▼
 FITNESS = does admitting this concept improve the scientist's understanding?
   • Predictor: higher leave-one-FAMILY-out held-out skill (evaluate_predictor)
   • Meta-Learner: fewer / higher-confidence rules (knowledge COMPRESSION)
   • Curiosity: lower model-disagreement / explains a standing anomaly
   MINUS an Occam complexity penalty
        │  gate (the R4 discipline: OOD improvement or reject — no feature spam)
        ▼
 ADMIT → append to FeatureRegistry (new vocabulary version) → publish a Concept
         node in the KnowledgeGraph (source-attributed, evidence-weighted)
        │
        ▼
 ALL faculties re-fit against the new vocabulary on their next cycle → the whole
 scientist is now smarter, permanently. Rejected concepts are kept (dead-end memory).
```

The engine **never** mines operator rules (Meta-Learner), ranks instances (Planner),
or runs ablations (Theory Engine). It only grows the vocabulary those organs run on.

---

## 3. Dependency diagram

```
                         ┌─────────────────────────────────────────┐
                         │        CONCEPT DISCOVERY ENGINE          │
                         │  propose → test → gate(OOD) → admit      │
                         └───────┬───────────────┬───────────┬──────┘
              reads evidence     │       fitness │           │ publishes
                    ┌────────────┘        (reuse)│           │
                    ▼                     ┌───────┴───────┐   ▼
             ExperimentDb ───────────────▶ Predictor      │  KnowledgeGraph
             (append-only)                │ Meta-Learner  │  (extended: +Concept,
                    ▲                      │ Curiosity     │   +Theory, +Program nodes)
                    │                      └───────┬───────┘        │
                    │ runs                         │ all read       │ projects to
        ┌───────────┴──────────┐                   ▼                ▼
        │ Runtime / Evolution  │◀──── FeatureRegistry (v0→vN) ──▶  OBSIDIAN VAULT
        │ Campaign / Orchestr. │      (single shared vocabulary)   (generated view,
        └──────────────────────┘         ▲   ▲   ▲   ▲             one-directional)
          Planner · Executive ───────────┘   │   │   │
          Memory (structure-keyed) ──────────┘   │   │
          World · Policy · Dynamics ─────────────┘   │
                                                     │
   SERENA (MCP) ── dev-time semantic navigation for Claude; OUTSIDE the product ─┘(none)
   GRAPHITI ── REJECTED (duplication); its value folded into the extended KnowledgeGraph
```

Single spine: `FeatureRegistry` is the one vocabulary; the Concept Engine grows it;
every organ consumes it; the graph records it; the vault projects it.

---

## 4. Integration plan (per existing subsystem — all read the registry)

| Subsystem | Change (extend only) | Duplication risk |
|---|---|---|
| **Predictor** | feature vector width becomes `registry.len()` (was 5); refit against current vocab version | none — same model, dynamic width |
| **Meta-Learner** | rule conditions may reference any admitted concept, not just density/clustering | none |
| **Curiosity** | coverage buckets + anomaly computed over the richer vocabulary | none |
| **Scientific Memory** | structural-signature index uses the full registry (finally closes STAGE_8 Pillar II) | none — completes a partial feature |
| **Theory Engine** | mechanism conditions can cite admitted concepts (e.g. "frustration>x") | none |
| **Planner / Curiosity** | expected-info-gain now includes "regimes thin in a *new* concept" | none |
| **KnowledgeGraph** | +typed nodes: `Concept`, `Theory`, `ResearchProgram` above facts (the Graphiti value, natively) | none — extension |
| **Campaign / Orchestrator** | one extra stage per cycle: run Concept Discovery in the `learn` phase | none |
| **reports.rs / dataset.rs** | additionally emit the Obsidian vault (frontmatter markdown) | none — new export target |

Concept Discovery slots into the orchestrator's existing `analyze()/learn()` stage —
**no new loop, no new planner.**

---

## 5. Migration strategy (evidence-gated, reversible)

1. **Centralize** the ~10 scattered feature computations behind `FeatureRegistry`
   with the current 5 scalars as v0. Behavior-preserving; golden regression + all
   cross-backend bit-identity must pass unchanged. *This is the whole first PR.*
2. **Version** the representation: models record which vocabulary version they were
   trained on (extend the Model Registry, which already versions weights/lineage).
   A model is only compared against models of the *same* vocab version — preserving
   reproducibility (ADR-0004).
3. **Seed** the primitive concept library from the existing `--structural` probes
   (already written, already tested).
4. **Admit** concepts only via leave-one-**family**-out improvement + Occam penalty —
   the exact discipline that killed the naive 11-feature descriptor (R4). No concept
   enters on in-sample fit.
5. **Project** to the Obsidian vault from the stores (one-directional).
6. Graphiti is **not migrated** — its ladder is realized as graph node types (step 1
   of the Concept Engine's publication path).

Reversibility: every step is behind a flag; v0 registry == today's behavior exactly.

---

## 6. Risk analysis

| Risk | Severity | Containment |
|---|---|---|
| **Representation refactor breaks determinism** (10 call sites) | High | v0 registry is bit-identical to today; golden + cross-backend gate every step; behind a flag |
| **Concept overfitting / feature spam** (the R4 failure) | High | Admission requires leave-one-FAMILY-out OOD improvement + complexity penalty; rejected concepts recorded, not retried |
| **Model reproducibility across vocab versions** | Med | Vocabulary-version stamping in the Model Registry; never compare across versions |
| **Graphiti/Neo4j silo + non-determinism** | High | **Rejected outright**; value captured by extending the deterministic graph |
| **Serena mistaken for scientist cognition** | Low | Labelled dev-tooling; never wired into the product |
| **Obsidian vault becomes a second source of truth** | Med | One-directional generation only; regenerable; never authored-in |
| **Concept Engine drifts into re-mining rules** | Med | Hard boundary: it emits *features* only; rule-mining stays in Meta-Learner |
| **Compute cost of concept search** | Med | Reuse Curiosity's budget/λ; concept proposals are cheap (deterministic feature evals), gated by the existing executor |

---

## 7. Implementation roadmap (no code until §5.1 is agreed)

- **P0 — Representation centralization.** `FeatureRegistry` (v0 = current 5 scalars);
  replace scattered computations; all faculties read it. *Kill criterion:* any golden/
  cross-backend regression → revert. *Ships:* identical behavior, one vocabulary.
- **P1 — Graph abstraction rungs.** Add `Concept`/`Theory`/`ResearchProgram` node
  types to the existing graph (the Graphiti value, natively). *Ships:* the ladder.
- **P2 — Concept admission on a fixed candidate set.** Wire the `--structural` probes
  as candidate concepts; admit only those passing OOD improvement + Occam. *Ships:*
  the scientist admits (or rejects) ruggedness/frustration **by its own test** —
  autonomously re-deriving the human finding.
- **P3 — Concept *proposal* (search).** Compose primitives into novel candidate
  concepts; the engine invents features no human wrote. *Ships:* open-ended vocabulary
  growth. *Kill criterion:* if no proposed concept ever beats the seed set OOD, the
  representation is complete — a real, publishable negative result.
- **P4 — Obsidian projection + Serena tooling.** Generated vault; MCP navigation.
- **P5 — Loop integration.** Concept Discovery runs in the orchestrator `learn` stage
  every cycle; the scientist grows its vocabulary continuously.

Order rationale: **the representation must be centralized before anything can consume
a new concept** (P0 is load-bearing); the ladder (P1) is where concepts live; admission
on a known-good set (P2) validates the fitness test before trusting open-ended search
(P3); tooling/views (P4) are cosmetic; continuous integration (P5) is last because it
amplifies whatever P2–P3 produce.

---

## 7b. Exact files to modify (P0 — representation centralization) + duplicate retirement

**Feature representation (the P0 refactor — v0 must stay bit-identical):**
- `src/engine_v2/decision.rs` — `InstanceStats::analyze(ir)` is the canonical IR→features
  computation (keep as the single extractor; becomes the registry's v0 evaluator).
- `src/engine_v2/ai_scientist/predictor.rs` — `InstanceSignature` (line 17) + `features()`
  (line 37): make the signature registry-backed and give it one canonical
  `to_features()`; remove the hardcoded width assumption.
- `src/engine_v2/ai_scientist/policy.rs` — `features()`/`N_FEATS` (line 57): delete the
  private encoder; call the shared `to_features()`.
- `src/engine_v2/ai_scientist/memory_os.rs` — `feats()->[f64;5]` (line 30): delete the
  private encoder; call the shared one.
- Construction sites to route through one constructor (`from_stats`/`from_ir`), not to
  rewrite: `campaign.rs:352`, `curiosity.rs:65`, `evaluation.rs:161/425`,
  `executive.rs:516/546`, `planner.rs:282`, `scientist.rs:457`, `policy.rs` tests.
- `src/engine_v2/ai_scientist/model_registry.rs` — add a **vocabulary-version** stamp
  next to the existing weight/lineage versioning (reproducibility across vocab versions).

**Duplicate retirement (my own prior additions — honor "never a second store/reasoner"):**
- `src/engine_v2/ai_scientist/research_log.rs` (`ResearchLog`) — **retire**: it is a
  second knowledge store (`experiments.db`/`knowledge.graph`) parallel to
  `CampaignManager` (`ai_experiments.txt`/`knowledge_graph.txt`). Route the CLI research
  modes' persistence through `CampaignManager` instead.
- `src/engine_v2/ai_scientist/reasoner.rs` (`GraphReasoner`) — **retire/collapse**: it
  duplicates Meta-Learner (rules) + Theory Engine (prediction/refutation) + Curiosity
  (anomaly/coverage) + Planner (info-gain). Keep only a thin `--reason` CLI *view* that
  delegates to those, or remove it entirely.
- `src/bin/research_platform.rs` — the `--reason` handler and the `--op-benchmark`
  persistence block: repoint from `ResearchLog` to `CampaignManager`.
- `src/engine_v2/ai_scientist/mod.rs` — drop the `research_log`/`reasoner` re-exports on
  retirement.

**Graph abstraction rungs (P1):**
- `src/engine_v2/ai_scientist/graph.rs` — extend `KnowledgeGraph` with typed node kinds
  (`Concept`, `Theory`, `ResearchProgram`, `Evidence`, `Refutation`, `Prediction`,
  hypothesis/experiment lineage edges) over its existing append-only, source-attributed
  triples — no new persistence layer.

**Concept Discovery Engine (P2–P3, after P0):** one new module
`src/engine_v2/ai_scientist/concept.rs` (upstream faculty; proposes/tests/admits
*features* only), wired into `orchestrator.rs::tick()`'s `learn` stage and reusing
`evaluation.rs::evaluate_predictor` (OOD test) + `meta_learner.rs` (compression) as
fitness. It emits into the FeatureRegistry + graph; it never mines rules, ranks
instances, or ablates.

## 8. One-paragraph verdict

The mission-aligned addition is the **Concept Discovery Engine**, and it is only
possible after the **representation is centralized** into one extensible
`FeatureRegistry` that every faculty already-present consumes — a refactor, not a
rebuild. Of the three external tools: **Obsidian** is adopted as a one-directional
generated view (safe, Git-native); **Serena** is adopted as dev-time navigation
tooling that is *not part of the scientist*; and **Graphiti is rejected** because it
duplicates the KnowledgeGraph + Scientific Memory and imports a non-deterministic
Python/Neo4j silo that violates the Constitution — its genuine value (a typed
abstraction ladder with refutation history) is captured by *extending the existing
deterministic graph*. The result is one coherent cognitive architecture in which the
scientist expands its own vocabulary and becomes permanently smarter after each cycle —
not a collection of bolted-on tools.
