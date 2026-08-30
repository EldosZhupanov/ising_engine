# External projects backlog

**Status:** deferred integration register  
**Updated:** 2026-08-27  
**Rule:** none of the projects below is a dependency or a current ROADMAP task.
Return to this file only after the active causal-instrument and equal-cost work is
closed. Every adoption requires its own preregistered comparison or ADR; popularity
is not evidence of benefit to Ising Engine.

This file preserves the external projects discussed in agent sessions and the
projects already identified in the research corpus. It is deliberately separate
from `ROADMAP.md`: this is a comparison queue, not permission to expand scope.

**Sections §1–§7 are the original comparison queue and their numbering is
referenced by the immutable `EXTERNAL_COMPARISON_PROTOCOL.md`; it must not
change.** §8 onward were added on 2026-08-27 from an external literature review
and cover a different question: not *"which baseline do we compare against"* but
*"which external mechanism or algorithm family occupies an architectural cell we
do not"*. §8 holds the research leads, §9–§11 the reference material, §12 the
refusals with their reasons, and §13 a candidate roadmap correction that is a
**hypothesis, not a decision**.

## 1. Priority queue

| Priority | Project | Official source | Intended use here | First comparison / kill criterion |
|---|---|---|---|---|
| P0 | **ASlib scenarios** | <https://github.com/coseal/aslib_data> | Standard external baseline for per-instance algorithm selection; prevents presenting a known problem as a new field. | Export one frozen Ising selector dataset to ASlib form and compare against standard selectors. Stop if our format cannot preserve budgets/censoring without changing the estimand. |
| P0 | **DACBench** | <https://github.com/automl/DACBench> | Baseline and evaluation protocol for dynamic algorithm configuration—the closest established field to our runtime operator controller. | Encode one small Ising control environment and compare regret/sample efficiency. Stop if state/action timing cannot be represented faithfully. |
| P0 | **SMAC3** | <https://github.com/automl/SMAC3> | Strong model-based configuration baseline. | Equal evaluation budget against our planner on frozen mixed discrete/continuous schedules. Our method must improve held-out quality/cost, not merely find a different schedule. |
| P0 | **Nevergrad** | <https://github.com/facebookresearch/nevergrad> | Broad derivative-free portfolio baseline, including discrete and mixed spaces. | Compare under identical objective-call budget and seeds. No claim if wall-time/evaluation budgets differ. |
| P0 | **Optuna** | <https://github.com/optuna/optuna> | Practical HPO/search baseline with pruning and distributed studies. | Use it as the product-grade baseline for schedule/configuration tuning; stop integration if adapter overhead dominates the evaluated solver budget. |
| P0 | **BiqMac corpus** | local `benchmark_suite/biqmac/` and `benchmark_suite/README.md` | Weighted BQP/MaxCut instances. Required for questions that G-Set cannot identify (weight-aware guides, frustration controls). | Use before any new guide/frustration claim. A G-Set-only result is insufficient by construction. |
| P1 | **MQLib** | <https://github.com/MQLib/MQLib> | Classical MaxCut/QUBO heuristic baseline and instance collection. | Compare solution quality versus wall time on a frozen common corpus. Reject comparisons based on iteration count alone. |
| P1 | **Google OR-Tools** | <https://github.com/google/or-tools> | Industrial exact/CP-SAT baseline for scheduling, routing and constraint problems. | Small/medium instances with known bounds and identical time limits. Its role is an external reality check, not a library to copy into the solver. |
| P1 | **OpenJij** | <https://github.com/OpenJij/OpenJij> | Open Ising/QUBO annealing baseline. | Compare reproducible CPU samplers on shared Ising instances and budgets; record conversion and initialization costs. |
| P1 | **IOHprofiler / IOHexperimenter** | <https://github.com/IOHprofiler/IOHexperimenter> | Experiment logging, benchmarking discipline and anytime-performance analysis. | Prototype an exporter rather than replacing ExperimentDb. Adopt only if it adds a standard analysis we cannot reproduce cheaply. |
| P1 | **flacco** | <https://github.com/mlr-org/flacco> | Exploratory Landscape Analysis baseline. It marks which feature ideas are already known. | Compare frozen S0/S1 features with established ELA features on held-out instances; no novelty claim for merely adding descriptors. |
| P2 | **SALib** | <https://github.com/SALib/SALib> | Sobol/Morris sensitivity analysis for coupled Foundry axes and policy parameters. | Use only after an axis-feasibility map exists. Stop if the generator cannot vary inputs independently enough for the selected method. |

## 2. LLM and product integration candidates

| Project | Official source | Decision | Concrete future experiment |
|---|---|---|---|
| **Soup** | <https://github.com/MakazhanAlpamys/Soup> | **Keep as the leading applied integration candidate.** Soup performs LoRA/QLoRA training, sweeps, evaluation and adapter merging; our engine should be an outer-loop discrete optimizer, never a replacement for gradient training. | Train or collect 8–20 compatible LoRA adapters. Measure individual utility and pairwise interference, formulate subset/discrete-weight selection as QUBO, and compare against Soup's CMA-ES, greedy, random, Optuna and exhaustive search where feasible. Same real-evaluation budget is mandatory. |
| **Hugging Face PEFT** | <https://github.com/huggingface/peft> | Use through Soup or a thin evaluation harness; do not add it to the Rust core. | Source compatible LoRA adapters and define a reproducible merge/evaluation corpus. Record base model, revision, adapter hashes and licenses. |
| **Hugging Face Evaluate** | <https://github.com/huggingface/evaluate> | Candidate standardized evaluator for the Soup pilot. | Freeze task metrics before optimization so the optimizer cannot select its own judge. |
| **MiroFish** | <https://github.com/666ghj/MiroFish> | **Market/scenario research only.** Multi-agent simulations may expose customer objections and deployment scenarios, but cannot validate solver quality or forecast demand reliably. | Give it a frozen product description and stakeholder personas; compare its objections with real interviews. Kill the use if it produces only ungrounded narratives or changes materially with prompt wording. |

## 3. Knowledge and agent tooling

These decisions are already justified in
`research/CONCEPT_DISCOVERY_AND_INTEGRATIONS.md`; this table prevents accidental
re-litigation.

| Project | Official source | Frozen decision |
|---|---|---|
| **Obsidian** | <https://obsidian.md/> | **Adopt later as a generated, one-way Markdown view only.** Git-tracked repository files remain the source of truth. Never create a second hand-edited vault or require Obsidian at runtime. |
| **Serena** | <https://github.com/oraios/serena> | **Optional developer tooling.** Useful for semantic navigation by coding agents; not part of the optimization product and not evidence of scientific capability. |
| **Graphiti** | <https://github.com/getzep/graphiti> | **Do not integrate.** It substantially duplicates the existing deterministic knowledge graph. Preserve useful typed-node ideas natively instead of adding Neo4j/runtime nondeterminism. |

## 4. Additional configuration baselines already identified by the literature audit

These are baseline obligations, not immediate integrations:

- **irace** — <https://github.com/MLopez-Ibanez/irace>; racing-based automatic
  configuration.
- **pymoo** — <https://github.com/anyoptimization/pymoo>; multi-objective
  optimization and Pareto-front baselines.
- **CMA-ES / pycma** — <https://github.com/CMA-ES/pycma>; continuous optimizer
  required when comparing adapter merge weights or schedule parameters.
- **Ray Tune** — <https://github.com/ray-project/ray>; distributed experiment
  execution baseline only if local Optuna/SMAC evaluation throughput becomes the
  measured bottleneck.

Do not install all of these. For each experiment choose the smallest baseline set
that covers the claim. A typical future comparison is:

```text
random + greedy + one established specialist + our method
```

Adding five similar tuners usually spends compute without increasing evidential
value.

## 5. Platform-source policy

- **GitHub** supplies versioned code and issue evidence. Pin commit SHAs; never
  benchmark against a moving default branch.
- **Hugging Face** supplies models, adapters and datasets. Pin repository revision,
  file hashes, dataset split and license.
- **Reddit/Discord** supply problem reports and language used by potential users.
  They are hypothesis sources only—never performance, novelty or market evidence.
- **Papers/official documentation** determine prior art and experimental contracts.
  Prefer primary sources over summaries and promotional pages.

## 6. Activation order after the current main work

1. Finish RC-020 marginal-cost identification with a valid instrument.
2. Run a separately preregistered equal-wall-cost operator comparison.
3. Benchmark the selector/controller against ASlib/DACBench conventions and
   SMAC3/Nevergrad/Optuna under equal budgets.
4. Expand the corpus with BiqMac and one external solver baseline (OpenJij or
   MQLib).
5. Only then run the Soup/LoRA applied pilot.
6. Use MiroFish for product-message stress testing and Obsidian as a generated
   research view after the technical evidence is stable.

## 7. Explicit non-goals

- Do not encode billions of continuous LLM weights directly as QUBO.
- Do not replace PyTorch/gradient descent with the Ising engine.
- Do not claim algorithm selection, dynamic configuration, ELA or HPO as new
  fields.
- Do not adopt an external dependency merely because it has stars or an active
  community.
- Do not let external tools change frozen seeds, metrics, budgets or held-out
  routing after an experiment starts.

---

# §8. Research leads — HIGH PRIORITY

Added 2026-08-27. Each lead is a **hypothesis with a falsification test**, not a
plan. Nothing here is scheduled, and nothing here changes `ROADMAP.md`.

Every URL below was fetched and checked on 2026-08-27. Claims that could not be
confirmed against a primary source are marked `UNVERIFIED` in place.

The internal anchors these leads attach to:

- **RC-004's seven axes** (`research/RC004_ARCHITECTURE_SPACE.md` §1, immutable):
  **Σ** state · **G** guide · **D** domain · **K** control · **P** population ·
  **M** moves · **T** time. RC-004's finding: *all 18 of our operators are the
  single point* `(config, energy, binary, temperature, R, local, sweeps)`.
- **The eight axioms** (`research/AXIOMS_OF_OPTIMIZATION.md` §1): Ax1
  representability · Ax2 solution-carrying state · Ax3 scalar total order · Ax4
  runtime-derived direction · Ax5 space invariance · Ax6 sequential dependence ·
  Ax7 instance isolation · Ax8 locality. §4 records which are protected: **Ax3 is
  theorem-protected on unweighted instances** (RC-004's `E = 2V − |E|`), **Ax5 is
  obstructed**, **Ax4/Ax7 are cost-obstructed**, and **§5 names Ax2 as the
  attackable one**.
- **The Z₂ corollary** (`research/RELATIONAL_PRIMITIVE.md` §40–44): any algorithm
  whose state is a set of per-variable marginals carries **exactly zero**
  information about a MaxCut solution — marginal-state methods are *provably
  vacuous*, not merely weak. **An Ax2 attack routed through marginals is already
  dead.** The relational primitive is the surviving route.

---

## §8.A — MQLib architectural census

**Status:** HIGH PRIORITY
**Source:** repository <https://github.com/MQLib/MQLib> (92★, MIT, C++);
Dunning, Gupta & Silberholz, *"What Works Best When? A Systematic Evaluation of
Heuristics for Max-Cut and QUBO"*, **INFORMS Journal on Computing 30(3), 2018**.
**Internal connection:** RC-004 (immutable); `AXIOMS_OF_OPTIMIZATION.md`; the
existing §1 P1 row for MQLib, which reserves it as a *benchmark* baseline only.

**Why it matters.** RC-004 concluded that all eighteen operators occupy one cell,
and used that to explain why every cycle produced ≈0.06 %. That conclusion was
drawn **from our own operator library**. A corpus of dozens of independently
designed human heuristics for exactly our problems is the cheapest available
attempt to refute it.

**Hypothesis.** The one-cell result is a property of *our* corpus, not of
MaxCut/QUBO heuristics in general; mapping MQLib's heuristics onto the seven axes
will populate cells we do not occupy.

**Cheap falsification test.** Read-and-classify only. For each MQLib heuristic
record its `(Σ, G, D, K, P, M, T)` tuple from its published description and
source, with a one-line justification per axis and the source line. No execution,
no benchmark run, no dependency. Output: a census table plus the count of
distinct cells.

**Possible outcomes, all of which are results:**

| Outcome | Reading |
|---|---|
| MQLib populates cells we do not occupy | The taxonomy discriminates and RC-004's finding is about *our* corpus. Those cells become synthesis targets. |
| MQLib collapses into our cell too | Either a real structural convergence of the whole field — a much stronger and more publishable claim than RC-004 alone — or the taxonomy is too coarse. The two must then be separated deliberately. |
| Heuristics cannot be classified cleanly | The taxonomy needs revision **before** it is used to steer generation. This outcome invalidates §8.B and §8.C until fixed. |

**Evidence required before any implementation follows.** A completed census with
per-axis justification, reviewed against RC-004's own table (which already places
WalkSAT, belief propagation, LTGA/GOMEA, simulated bifurcation and MemComputing
outside our cell, and Houdayer/ICM **inside** it).

**What NOT to do.** Do not add MQLib as a dependency; do not run it as a
benchmark inside this lead — §1's P1 row already governs that separately and has
its own kill criterion. **Do not tune the classification to produce new cells.**
A collapse result is as valuable as a spread result and must be reported as
found.

**Potential future integration.** If cells are found empty for us but occupied by
humans, they become the target set for §8.B and §8.C. If §1's P1 benchmark
comparison later runs, the census gives it an architectural axis to report along.

---

## §8.B — Architecture-space quality diversity

**Status:** HIGH PRIORITY
**Source:** mechanism from MAP-Elites as used in OpenEvolve
(<https://github.com/algorithmicsuperintelligence/openevolve>, 7.3k★, Apache-2.0,
825 commits, Python, drives evolution of code in other languages **including
Rust**; requires an OpenAI-compatible LLM endpoint).
**Internal connection:** `src/engine_v2/evolution.rs` (genetic search over
operator sequences); RC-004; `curiosity.rs` (surprise = coverage deficit +
cross-seed anomaly + model disagreement).

**Why it matters.** Quality-diversity search needs a **behavioural descriptor**,
and the descriptor is what determines what "new" means. Published LLM-evolution
systems use ad-hoc descriptors — code length, runtime, hand-picked features.
RC-004 and `AXIOMS_OF_OPTIMIZATION.md` give us a *derived* coordinate system for
architectural novelty that, as far as this review found, no external system has.
Our own evolution engine currently selects on fitness alone, and ROADMAP records
that its evolved plans **lose 5/5 to `UltimateSolver`** at equal budget — a
fitness-only search inside one cell.

**Hypothesis.** Separating `quality(candidate)` from
`architectural_novelty(candidate)`, and keeping an archive keyed by architectural
cell rather than by rank, finds qualitatively different candidates that a
fitness-ranked search cannot reach — because a marginally-better intra-cell
candidate currently outranks a worse candidate in an empty cell.

**Cheap falsification test.** Before writing any search: take the existing
experiment corpus and compute, retrospectively, what a cell-keyed archive would
have retained versus what fitness ranking retained. If the two archives are the
same set, the descriptor adds nothing on the data we already have and the lead
stops there.

**Evidence required before implementation.** (1) §8.A must have shown the
taxonomy discriminates. (2) The retrospective archive test must show divergence.
(3) A formal archive contract — `cell -> best verified candidate` — where
"verified" means correctness against the `ReferenceState` oracle, a recorded
provenance stamp, and evidence in the append-only `ExperimentDb`. An archive
entry without those three is not an entry.

**What NOT to do.** Do not integrate OpenEvolve as a framework. Do not treat a
candidate as interesting because its fitness is marginally higher. Do not let a
cell be claimed by a candidate that has not passed the correctness firewall.

**Potential future integration.** The descriptor is a small internal addition to
`evolution.rs` plus a scoring term in `curiosity.rs`, not a new subsystem.

---

## §8.C — Axiom-targeted operator synthesis

**Status:** HIGH PRIORITY (blocked on §8.A and §8.B)
**Source:** internal — `AXIOMS_OF_OPTIMIZATION.md` §5; RC-003's runtime move
synthesizer (+0.06 %, the only cycle to move an axis).
**Internal connection:** `src/engine_v2/registry.rs` capability passports;
`ai_scientist/proposal.rs` (ROADMAP records it as *operator-proposal drafter,
**no auto-code***).

**Why it matters.** The generic instruction *"generate a better MaxCut
operator"* is exactly the search that produced 0.06 % for three cycles. The
axioms give a strictly more specific target.

**Hypothesis.** A generator constrained to a named axis value or a named axiom
produces structurally different candidates from a generator optimising fitness,
and the difference is measurable in the §8.B descriptor before any benchmark is
run.

**Required candidate record — a candidate without all ten fields is not a
candidate:** target axis/cell · target axiom · mechanism rationale · expected
regime · falsifiable prediction · generated implementation or patch · correctness
status · benchmark evidence · ablation result · final verdict.

**Cheap falsification test.** Hand-write **three** candidates against three
different targets, with no LLM involved, and check whether the pipeline —
descriptor, correctness firewall, ablation, verdict — can actually carry them end
to end. If the pipeline cannot process a hand-written candidate, it certainly
cannot process a generated one.

**Evidence required before implementation.** A sandbox with an enforced
correctness firewall (`ReferenceState` f64 oracle and `SparseBitSlice` exact
cross-check are already the mechanism), the golden regression passing unchanged,
and a written ADR for the sandbox boundary.

**What NOT to do.**
- **Do not target Ax2 through per-variable marginals** — `RELATIONAL_PRIMITIVE.md`
  proves that route carries zero information on MaxCut.
- **Do not target Ax3 on unweighted instances** — RC-004's `E = 2V − |E|` is a
  theorem, not an obstacle.
- LLM-generated code lives **only** in the research sandbox. `UltimateSolver`,
  every frozen legacy solver, and the read-only `engine_v2` core are untouchable
  (CLAUDE.md §4, ADR-0001, ADR-0004).
- No candidate is admitted on fitness alone, and no ablation result is discarded.

---

## §8.D — Negative experience returned to ideation

**Status:** HIGH PRIORITY (smallest of the five; verified gap)
**Source:** mechanism from ReEvo's reflection step
(<https://github.com/ai4co/reevo>, 294★, MIT, NeurIPS 2024, arXiv 2402.01145) and
RefineEvo's bidirectional positive/negative experience pool
(<https://github.com/samwu-learn/RefineEvo> — see §12 for why the repository
itself is refused).
**Internal connection:** `ai_scientist/proposal.rs`, `curiosity.rs`,
`memory_os.rs`, `graph.rs`, `theory.rs`.

**Why it matters.** We already keep honest negatives as first-class knowledge —
`extremal_metropolis` weak on G-Set, evolved plans losing 5/5, the refuted
universal laws. **Verified on 2026-08-27:** `proposal.rs`, `curiosity.rs` and
`memory_os.rs` contain **zero** occurrences of `refut`/`negative`/`fail`. The
refutations are stored but are **not read back at ideation time**. The system can
therefore re-propose something it has already refuted, and its own record would
not stop it.

**Hypothesis.** Returning *"what was already tried, under which conditions, and
why it failed"* to the ideator measurably reduces re-proposal of refuted
mechanisms.

**Cheap falsification test.** Purely retrospective, no code: replay the recorded
proposal history against the recorded refutations and count how many proposals
were already-refuted mechanisms. **If the count is zero, this lead is dead** —
the gap is theoretical and not worth code.

**Evidence required before implementation.** A non-zero re-proposal count, plus a
check that `KnowledgeGraph::condition_holds` can express the *conditions* under
which a mechanism failed — a bare "X failed" is exactly the unconditional fact
`graph.rs` forbids (CLAUDE.md §10).

**What NOT to do.** Do not adopt ReEvo or RefineEvo as frameworks; neither
targets MaxCut or Ising (ReEvo: TSP/CVRP/BPP/MKP/OP/DPP; RefineEvo:
TSP/CVRP/BPP/MKP). Take the retrieval idea, not the code.

**Potential future integration.** A retrieval call in the ideation path, sourced
from the existing conditional, evidence-weighted graph. No new store.

---

## §8.E — Stage-11 foundation-model / GNN evidence gate

**Status:** HIGH PRIORITY — this is a **gate**, and it blocks Stage 11
**Source:** the Nature Machine Intelligence exchange, all four items verified:
Schuetz, Brubaker & Katzgraber, *"Combinatorial optimization with physics-inspired
graph neural networks"*, **Nat. Mach. Intell. 4, 367–377 (2022)**;
Angelini & Ricci-Tersenghi, *"Modern graph neural networks do worse than classical
greedy algorithms…"* <https://www.nature.com/articles/s42256-022-00589-y>;
Boettcher, *"Inability of a graph neural network heuristic to outperform greedy
algorithms…"* <https://www.nature.com/articles/s42256-022-00587-0>; and the
authors' replies <https://www.nature.com/articles/s42256-022-00590-5> and
<https://www.nature.com/articles/s42256-022-00588-z>.
Plus, and closer to our problem than any of the above:
**MaxCut-Bench** — <https://github.com/ankurnath/MaxCut-Bench>, arXiv 2406.11897,
*"A Benchmark for Maximum Cut: Towards Standardization of the Evaluation of
Learned Heuristics for Combinatorial Optimization"*, whose reported finding is
that **tabu search outperforms every evaluated learned heuristic** on objective
value, scalability and generalisation, with the single exception of ANYCSP.
`UNVERIFIED`: no official public code repository for PI-GNN itself was located.
**Internal connection:** ROADMAP Stage 11 (🔬 10 %); `dataset.rs`; Next-10 items 1
and 10.

**Why it matters.** ROADMAP currently gates the foundation model on dataset size:
*"when the dataset crosses 500k, re-evaluate whether a shared GNN/Transformer
representation is finally earned."* Scale is a necessary milestone. It is **not**
evidence that a richer representation is needed, and the published record on our
own problem class is at best contested and at worst negative.

**The gate — replaces "500k reached → build GNN":**

1. Demonstrate that the **current scalar structural features are the bottleneck**
   — a learning curve that has flattened against features, not against data.
2. Fix a **held-out-family protocol in advance**, in writing.
3. Compare four arms: current scalar representation · a trivial-but-strong
   classical baseline · a GNN representation · a richer graph representation.
4. Fix the **effect size and robustness criterion before** looking at results.
5. **Reject the richer model** unless it shows a convincing out-of-family gain.

**What NOT to do.** Do not treat 500k as the trigger. Do not run the comparison
without the classical baseline arm — that arm is the one the published literature
says usually wins. Do not build the model to justify the dataset.

---

# §9. Watch list — mechanism sources, not dependencies

None of these is proposed for adoption. Each is listed for **one specific
mechanism** worth reading, so that a future session does not re-derive it. All
metrics checked 2026-08-27.

| Project | Verified source | Read it for | Do not |
|---|---|---|---|
| **OpenEvolve** | <https://github.com/algorithmicsuperintelligence/openevolve> — 7.3k★, Apache-2.0, 825 commits, Python; evolves code in other languages incl. **Rust**; needs an OpenAI-compatible endpoint | MAP-Elites archive; evaluator architecture; program database; code-mutation representation; the sandbox/evaluation boundary | Adopt the framework before §8.B's retrospective test says the descriptor divides anything. Its headline results (circle packing, kernel speedups) are **self-reported, not peer-reviewed** |
| **FunSearch** | <https://github.com/google-deepmind/funsearch> — 1.1k★, Apache-2.0/CC-BY-4.0, **14 commits**; accompanies *Nature* (2023) "Mathematical discoveries from program search with large language models" | Islands; migration between islands; population-diversity maintenance; program database | Expect a runnable system. The repo **explicitly excludes** the LLM, the sandbox and the distributed infrastructure. Reference code only. Possible later use: one island per architectural cell |
| **AlphaEvolve** | Paper arXiv **2506.13131** (whitepaper 16 Jun 2025); blog <https://deepmind.google/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/>; results-only repo <https://github.com/google-deepmind/alphaevolve_results> | Evaluator contracts; candidate database; parallel candidate evaluation; code-level evolution patterns | Treat as an available system. No implementation is published — the repository holds results, not the agent |
| **LLaMEA** | <https://github.com/XAI-liacs/LLaMEA> — 122★, MIT, 452 commits, Python; first released **Nov 2024**; paper in IEEE Trans. Evolutionary Computation, 2025 | Niching; diversity preservation; unified-diff / patch-style mutation; the separation of algorithm *structure* from *parameter* tuning | **Do not call it a successor to AlphaEvolve** — it predates AlphaEvolve (May 2025) by six months. Its own README's "fully-open alternative" is positioning, not chronology. Note also its home turf is **BBOB/IOH continuous** black-box optimisation; transfer to discrete Ising is a research question, not a given |
| **ReEvo** | <https://github.com/ai4co/reevo> — 294★, MIT, Python, NeurIPS 2024, arXiv 2402.01145 | Reflection *before* the next generation; feeding failure evidence into later synthesis (see §8.D) | Adopt. Targets TSP/CVRP/BPP/MKP/OP/DPP — **no MaxCut, no Ising** |
| **HSEvo** | <https://github.com/datphamvn/HSEvo> — AAAI-25, arXiv 2412.14995; ships baselines for EoH, FunSearch and ReEvo | Two explicit population-diversity metrics: Shannon–Wiener Diversity Index and Cumulative Diversity Index; and its finding that ReEvo's reflection improves objective score but **not** diversity | Add a diversity metric because it is published. First state what it would measure **in our architectural space** — a metric over code tokens is not a metric over cells |
| **autoresearch** | <https://github.com/hugoferreira/autoresearch> — 3★, **Go**, 217 commits, **license TBD**, early/experimental; drives Claude Code or Codex | Its ontology: Goal · Hypothesis · Experiment · Observation · Conclusion · Lesson · **Frontier** · **Instrument**. Two of these we do not have as first-class objects: `Instrument = command + parser → measured value`, and `Frontier = best supported conclusions for the current goal`, which shows whether the search has **stalled** | Take a dependency — 3★, license TBD, and Go. Compare its statistics against `src/benchmark/stats.rs`: it uses **BCa bootstrap 95 % CI (2000 seeded resamples)** for single samples and **percentile bootstrap on fractional delta + two-tailed Mann–Whitney U** against baseline. Its `Instrument` concept is the generalisation of what RC-020/RC-021 built by hand |

---

# §10. External algorithm families to map into the RC-004 space

This section exists because the review that produced §8 initially proposed only
*meta-tools for discovering algorithms* and no *competing algorithm families*.
For RC-004's question, the families matter more.

**RC-004 has already classified some of these** (`RC004_ARCHITECTURE_SPACE.md`
§1, immutable). Those rows are recorded here so they are not re-opened:

| Family | RC-004's own classification | Consequence |
|---|---|---|
| Simulated bifurcation | `config · energy · **continuous** · none · R · **ODE** · sweeps` | Already known to differ on **D**, **K** and **M**. The open question is empirical, not taxonomic |
| MemComputing | `config+memory · energy · continuous · none · 1 · ODE · **event**` | Differs on Σ, D, K, M, T |
| WalkSAT / focused walk | `config · **violation** · binary · **none** · 1 · local · **event**` | Differs on G, K, T |
| Belief propagation | `**field** · energy · **prob** · none · field · resample · sweeps` | Differs on Σ, D, P, M — **but see the Z₂ corollary in §8: provably vacuous on MaxCut** |
| LTGA / GOMEA | `config · energy · binary · **none** · R · **synthesized** · sweeps` | Differs on K and M |
| **Houdayer / isoenergetic cluster moves** | **inside our cell** | **Do not re-open as a candidate new cell.** RC-004 names Houdayer and ICM among the eighteen |

Candidates to classify, all verified 2026-08-27:

| Family | Verified source | Why it is a candidate | First question |
|---|---|---|---|
| **MQLib** | <https://github.com/MQLib/MQLib> — 92★, MIT, C++; INFORMS J. Comput. 30(3), 2018 | The main human-designed MaxCut/QUBO corpus; also ships an ML hyper-heuristic for per-instance algorithm selection | §8.A. Note the hyper-heuristic is prior art for our selector work — see §7's non-goal against claiming algorithm selection as a new field |
| **Simulated bifurcation** | <https://github.com/bqth29/simulated-bifurcation-algorithm> — 166★, MIT, Python/PyTorch, 598 commits; implements **bSB, dSB, HbSB, HdSB**; cites Goto et al. *Sci. Adv.* 2019 and 2021, Kanao & Goto *Commun. Phys.* 2022 | RC-004 already places it in a different cell. This is a runnable open implementation of the variants | Does the measured behaviour match the predicted cell, i.e. does an axis change produce an effect our intra-cell variation cannot? |
| **QQA4CO / PQQA** | <https://github.com/Yuma-Ichikawa/QQA4CO>, ICLR 2025; arXiv 2409.02135 *"Optimization by Parallel Quasi-Quantum Annealing with Gradient-Based Sampling"* | Continuous relaxation + gradient sampling on GPU, with a communication term between parallel runs. Candidate `D = continuous`, `M = gradient`, and a **P** coupling we do not have. Bundles PA, SA, iSCO and neural/continuous approaches as one comparable corpus | Is the parallel-run communication term a genuinely new **P** value, or replica exchange under another name? |
| **MaxCut-Bench** | <https://github.com/ankurnath/MaxCut-Bench>, arXiv 2406.11897 | Unified interface over forward/reversible greedy, tabu, extremal optimisation, S2V-DQN, ECO-DQN, ANYCSP — several families in one place, **on our exact problem** | Which of these are outside our cell, and does its reported "tabu beats every learned heuristic except ANYCSP" survive our own protocol? Feeds §8.E directly |
| **peapods** | <https://github.com/PeaBrane/peapods>, arXiv 2602.19045 — *"peapods: A Rust-Accelerated Monte Carlo Package for Ising Spin Systems"*; Python interface, **Rust** backend | Implements Metropolis, Gibbs, Swendsen–Wang, Wolff, PT, and **three** replica cluster moves — Houdayer ICM, **Jörg**, and **CMR** — plus site/link overlap statistics, overlap histograms, cluster-size distributions, equilibration diagnostics and integrated autocorrelation times | RC-004 already puts Houdayer/ICM inside our cell, so the cluster moves are **not** the lead. The lead is the **diagnostics**: overlap order parameters and integrated autocorrelation time are measurements we do not take. Being Rust, it is the one corpus readable without a language boundary |
| **D-Wave Hybrid** | <https://github.com/dwavesystems/dwave-hybrid> — 90★, Apache-2.0, Python, 1180 commits; `EnergyImpactDecomposer`, `RacingBranches`, `InterruptableTabuSampler`, `SplatComposer`; **D-Wave hardware optional** | Problem **decomposition** plus a heterogeneous solver portfolio. Decomposition is not a value on any of our seven axes | Is decomposition an eighth axis, or is it Ax1 (representability) being changed? Answer before treating it as a cell |
| **RL4CO** | <https://github.com/ai4co/rl4co> — 899★, MIT, PyTorch; NeurIPS 2023 GLFrontiers workshop, KDD 2025 | Named in the review as Stage-11 literature | **Domain mismatch, recorded so it is not re-checked:** covers TSP, CVRP, scheduling and EDA. **MaxCut and Ising are not listed.** Reference for Ax4/Ax7 attacks only |
| **DIFUSCO** | <https://github.com/Edward-Sun/DIFUSCO>, NeurIPS 2023, arXiv 2302.08224 | Discrete-diffusion solver; the review's diffusion representative | Ships **TSP and MIS** models, **not MaxCut**. Read for the Ax4/Ax7 mechanism — direction from a learned score rather than from evaluating the objective — not as a baseline |

**Rule for this section.** Classification comes before comparison, and comparison
before adoption. A family that has not been placed on the seven axes cannot be
argued to be architecturally new, however well it benchmarks.

---

# §11. Benchmark and scientific-infrastructure references

Kept separate so that infrastructure is never mistaken for scientific novelty.
§1 already carries Nevergrad, Optuna, OpenJij and IOHexperimenter as **baseline**
rows with their own kill criteria; the notes below are additional and do not
replace them.

| Reference | Verified source | Read it for | Constraint |
|---|---|---|---|
| **IOHexperimenter** | <https://github.com/IOHprofiler/IOHexperimenter> (§1, P1) | The experimental contract itself: logging format, **budget semantics**, reproducibility guarantees | §1's rule stands — prototype an exporter, never replace `ExperimentDb` |
| **OpenJij / dwave-neal** | <https://github.com/OpenJij/OpenJij> (§1, P1) | External Ising/QUBO annealing baselines | Adapters only. Record conversion and initialisation cost as part of the budget |
| **Optuna** | <https://github.com/optuna/optuna> (§1, P0) | Storage architecture; worker coordination; a control experiment for HPO questions | **Do not take its samplers or pruning.** ADR-0004 requires bit-identical replay from a seed; pruning and asynchronous sampling are not compatible with that as-is. Any use must state how determinism is preserved |
| **Ray** | <https://github.com/ray-project/ray> (§4) | Reference only for a future distributed `BatchExecutor`: worker model, retries, scheduling, fault tolerance | Do not add a Python/Ray dependency to the Rust core. ADR-0004 applies to the distributed executor exactly as it does to the local one |

---

# §12. Rejected for now — with the reason, so it is not re-researched

| Project | Verified source | Verdict | Reason |
|---|---|---|---|
| **Crewplane** | <https://github.com/crewplaneai/crewplane> — 35★, Apache-2.0, 87 commits; Markdown + YAML workflow DAGs over Claude Code, Codex, Gemini | **No current need** | Orca already provides orchestration, supervised workers, decision gates and persistent run state — `orchestration task-create / dispatch / worker-start / send / gate-create`, exercised in this repository on 2026-08-27 |
| **duet-agents** | <https://github.com/JonathanRosado/duet-agents> — **1★**, MIT, 47 commits; tmux/psmux FIFO inboxes | **No current need** | An additional messaging layer does not address the real bottleneck. Its README states it has **no recovery** from a crash and incomplete Windows rejoin |
| **tmux-team** | <https://github.com/wkh237/tmux-team> | **No current need** | Same reason, with less functionality |
| **RefineEvo (repository)** | <https://github.com/samwu-learn/RefineEvo> — **2★, 2 commits, no license file**; claims ICML 2026; targets TSP/CVRP/BPP/MKP | **Reference paper/idea only** | No license, no maintenance history, and **no MaxCut or Ising in scope**. The bidirectional experience pool is carried into §8.D as an idea; the repository is not a dependency candidate |
| **PiEvo** | <https://github.com/amair-lab/PiEvo> — 38★, MIT, **7 commits**, Python, AutoGen-based; claims ICML 2026, arXiv 2602.06448 | **Reference only** | The concept — evolving *principles*, not only hypotheses, under uncertainty minimisation — is interesting for the Theory Engine. Its experiments are in scientific-discovery benchmarks including ChEMBL; **domain mismatch** with an optimisation engine's current priorities |
| **Nevergrad as a MaxCut/QUBO baseline** | <https://github.com/facebookresearch/nevergrad> | **Rejected in that role** | It is a derivative-free portfolio, not a MaxCut/QUBO solver. §1 already scopes it correctly as a *derivative-free portfolio baseline under identical objective-call budget*; using it as a MaxCut arm would produce a comparison nobody is entitled to interpret |

**The operational fix that these three orchestration tools do not provide.**
On 2026-08-27 a Claude session and a Codex session wrote into **the same working
tree simultaneously** during RC-021 Step 7. The work merged without loss, but by
luck rather than by design. The correct fix is procedural and needs no new
software: **one agent = one isolated Orca worktree** (`orca worktree create
--agent`). Claude and Codex must not hold the same working tree at the same time.

---

# §13. Candidate roadmap correction — a research hypothesis, not a decision

**Status:** UNPROVEN. Recorded as a candidate gate so it is not lost and not
silently adopted. **`ROADMAP.md` and `PROJECT_PLAN.md` are unchanged.**

The current sequence (ROADMAP "Next 10" items 1, 2 and 10) prioritises dataset
growth toward 500k experiments and a distributed `BatchExecutor`.

**The hypothesis:** *before expensive scaling, test whether **architectural
diversity** — not sample count — is the binding constraint on new knowledge.*

The alternative sequence, offered for evaluation only:

1. Map MQLib and the §10 families into the RC-004 architectural space.
2. Attempt to **falsify** RC-004's one-cell finding.
3. Validate or revise the seven-axis taxonomy and the axiom list against that census.
4. Identify cells and mechanisms that are genuinely uncovered.
5. Redirect Curiosity and the generation objective toward structural novelty (§8.B).
6. **Only then** decide whether 500k growth is still the highest-value next step.
7. Scale the dataset once the experiment generator produces sufficiently diverse information.

**What this hypothesis does *not* claim.** It does **not** claim dataset growth
is useless. A further 400k experiments inside one apparent architectural cell
would still improve calibration, and the Predictor, World and Policy estimates
that depend on it — those are real gains and ROADMAP's Next-10 item 1 remains
justified on its own terms. The narrower and testable claim is only about
*where the next unit of compute buys the most new knowledge*.

**What would settle it.** §8.A's census is the deciding experiment and costs no
compute. If MQLib and the §10 families populate cells we do not occupy, this
sequence has a case. If they collapse into our cell, RC-004's finding is stronger
than we thought, structural novelty is not available cheaply, and scaling is the
better use of compute — which would **confirm** the current roadmap rather than
correct it.

**Do not remove or reorder any current roadmap priority on the strength of this
section.** It becomes actionable only through §8.A's result and, if that result
supports it, a written ADR.
