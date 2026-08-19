# External projects backlog

**Status:** deferred integration register  
**Updated:** 2026-08-20  
**Rule:** none of the projects below is a dependency or a current ROADMAP task.
Return to this file only after the active causal-instrument and equal-cost work is
closed. Every adoption requires its own preregistered comparison or ADR; popularity
is not evidence of benefit to Ising Engine.

This file preserves the external projects discussed in agent sessions and the
projects already identified in the research corpus. It is deliberately separate
from `ROADMAP.md`: this is a comparison queue, not permission to expand scope.

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

