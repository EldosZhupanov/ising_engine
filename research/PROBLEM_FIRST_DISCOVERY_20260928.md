# Problem-first discovery: looking for the other scissors

Date: 2026-09-28. Status: **research triage, no product or performance claim**.
Ising Engine development remains paused. This is a broad, non-solver search at
the owner's request. Sources are primary papers, project documentation, and
published engineering reports; most performance figures are author-reported.
There were no user interviews, reproduced external benchmarks, hardware trials,
or claims of exhaustive coverage of any field.

## The search question

For a real job, distinguish the desired decision/output `y` from the customary
intermediate object `R(x)`. Ask whether a smaller, task-specific representation
`S(x)` can produce `y` with the required error and safety properties. Example:
the customer wants a printable part, not necessarily a watertight triangle mesh.
This **does not imply** that `S` exists, preserves enough information, or is
novel. If the objective is approximate, assess *downstream decision loss*, not
only visual similarity or a proxy score.

Search operators, each already known somewhere, are: bypass an intermediate;
compute only the needed decision/certificate; observe only decision-relevant
inputs; reuse work across a sequence; replace an unreliable proxy by an
independent outcome oracle; specialize the representation to the physical or
organizational workflow. Their *application to a specific unsolved job* might
be valuable. The operators themselves are not a claim of invention.

For a claimed fivefold **end-to-end** speedup, at least 80% of baseline time
must be in work one can eliminate even if the replacement were free:
`S_max = 1/(1-p)`. In the real implementation, new setup, data movement,
verification and fallback shrink this bound. A result measured only on the
replaced stage does not qualify. [A 2026 simulation case study](https://arxiv.org/abs/2608.23075)
reports a 5.8x cheaper surrogate call but approximately tied full-solver cost;
this is author-reported, not our reproduction.

## Cross-domain opportunity map

Status key: **NO-GO** = broad formulation already has strong coverage or no
credible differentiator in this screen; **NARROW** = a testable unmet slice may
remain; **UNKNOWN** = insufficient accessible evidence. None is **SUPPORTED**
as a product or breakthrough. A first test is a falsifier, not an implementation
plan or preregistered experiment.

| Job and conventional path | Alternative question; strongest prior art | First falsifier and decision |
| --- | --- | --- |
| **Physical part from a scan:** point cloud → mesh reconstruction/repair → slicer → G-code. | Can slice-aligned maps go directly to toolpaths? [PrintAnything](https://arxiv.org/html/2607.27729v1) already does exactly this and publishes [code](https://github.com/Sangminhong/PrintAnything). Its reported 0.33 s vs fast DWG 0.37 s is **not** a 4–5x gain over the strongest cited fast comparator. The authors explicitly leave material physics/constraints out; real prints are on one printer and strength is checked manually. [Kerf](https://github.com/Khushiyant/kerf) already offers a G-code IR and bounded deposited-geometry checks, but says it does not verify process state. | **NARROW:** test whether a physics/printer-aware gate on direct-scan toolpaths predicts actual print failure or strength better than PrintAnything + standard slicer + existing printability checks. First qualify public raw scans, toolpaths, printer profiles and measured physical outcomes. If only synthetic geometry or one printer is available, physical-product claim stops. No ownership of PrintAnything's core idea. |
| **Reproduce a scientific claim:** paper → human reconstructs workflow → tools → result. | Can one check a *claim and its exact computational evidence* rather than generate a whole paper? [AutoMat](https://arxiv.org/abs/2605.00803) already benchmarks 85 computational-materials claims (best setting: author-reported 54.1% success). [AgentActionBench](https://arxiv.org/abs/2609.11117) evaluates reproduction processes across 150 papers. [SocSci-Repro-Bench](https://arxiv.org/abs/2606.11447) tests 221 social-science tasks with known reproducibility conditions. | **NARROW:** find one claim class with legal public input, a domain oracle and a materially missed failure under existing replay/checklist tools. Compare independent numerical/semantic validation to ordinary replay and human review at equal access and cost. If missing original data/protocol is the dominant failure, software alone cannot solve it. |
| **Agent says DONE:** accept narrative, read trace, or rely on CI. | Verify final task-specific outcome with a fresh oracle. This already overlaps [false-success detectors](https://arxiv.org/abs/2606.09863), [AgentClaimGuard](https://github.com/konoeph/AgentClaimGuard), [BeforeDone](https://github.com/rrrrrredy/beforedone), [AgentTrial](https://github.com/tang-vu/agenttrial) and [SWE-bench's artifact regrading](https://github.com/swe-bench/experiments). Repositories' descriptions establish existence, not reliability. | **NARROW, crowded:** before coding, establish at least 30 joinable real claims/artifacts/independent outcomes; compare to the trivial official-verdict rule, CI and cheap detectors. If these already catch the same false claims at lower cost, **NO-GO**. See [existing protocol sketch](HYPODIVE_TRIAGE.md#new-product-discovery-after-owner-pause--2026-09-28). |
| **Scientific notebook correctness:** rerun cells or trust displayed plots. | Enforce that displayed state matches a clean ordered run. [FlowBook](https://arxiv.org/abs/2605.01560) already tracks cell reads/writes and detects stale outputs. A [2026 observational study](https://arxiv.org/abs/2603.22726) reproduced only 2/19 attempted notebooks; the small attempted sample does not estimate population prevalence. | **NO-GO** for a generic notebook-state fixer. A separate narrow test would need the missing-data/dependency problem or claim-level numerical validity, with FlowBook and clean container rerun as baselines. |
| **Find code breakage:** run a large test suite after each change. | Select tests or run them earlier. [Meta's predictive selection](https://engineering.fb.com/2018/11/21/developer-tools/predictive-test-selection/) and [Google speculative testing](https://research.google/pubs/speculative-testing-at-google-with-transition-prediction/) are mature production examples; [Google's assessment](https://research.google/pubs/assessing-transition-based-test-selection-algorithms-at-google/) warns that simple historical predictors can beat clever ones. | **NO-GO** for generic AI test selection. Only a specific underserved environment with deployable artifacts, strong simple baseline and measured missed regressions could reopen it. |
| **Expensive scientific simulation:** compute a full field, then inspect one outcome. | Compute a quantity of interest or certified decision bound directly. [Goal-oriented error estimation](https://epubs.siam.org/doi/pdf/10.1137/15M1021982?download=true) and [certified reduced-order active learning](https://epubs.siam.org/doi/10.1137/22M1493318) establish the broad mathematics. | **UNKNOWN / established class:** select one real open workload and measure target-decision error, total latency and fallbacks versus the best domain solver. A general "solve less" theorem or universal 5x claim is unsupported. |
| **Repeated numerical solves:** restart a sparse solver for every nearby system. | Recycle subspaces/preconditioners. Existing [recycling survey](https://doi.org/10.1002/gamm.202000016) and [parametrized preconditioner work](https://doi.org/10.1137/20M1331123) are direct prior art. [Parth](https://doi.org/10.1145/3731179) illustrates stage vs total-time separation. | **NARROW, application-dependent:** qualify a real sequence, compute the unchanged-work floor, then compare against warm starts, tuned PETSc/hypre and existing recycling at identical true residuals. See [previous screen](HYPODIVE_TRIAGE.md#dual-track-discovery-update--2026-09-28). |
| **Measure before deciding:** collect a full data field, then optimize a decision. | Choose measurements for downstream decision value. This is already studied in [decision-focused data acquisition](https://arxiv.org/abs/2504.15062), including a drone reconnaissance/shortest-path example. | **UNKNOWN:** seek a particular high-cost measurement workflow with public outcomes and a customer who controls acquisition. Compare to cheap coverage/random and standard active-learning rules; count all measurement and error costs. No claim that the principle is new. |
| **Editable scientific figure:** infer original data/code from a published image. | Recover only the visual presentation, explicitly abstaining on original measurements. [SciFigure2Code](https://arxiv.org/abs/2609.08155) already makes that distinction and provides a 337-panel test set. | **NO-GO** as new research category. A product for figure editing would need distinct buyer evidence; it must never advertise recovery of unavailable original data. |

## Ranking for this owner, not a claim of market size

### First public-artifact qualification (read-only)

The most promising low-capital row deserves a stricter intake than a paper
abstract. This check inspected repository/dataset documentation and one
published benchmark gold JSON. It did not inspect original replication
packages, individual agent runs or traces, so it does not yet meet Gate A.

| Candidate source | What is publicly described | Specific obstacle to a claim-bound verifier |
| --- | --- | --- |
| [SocSci-Repro-Bench](https://github.com/malizad/SocSci-Repro-Bench) | README reports 221 tasks; the checked GitHub gold JSON contains 220 answer entries across 54 papers. Replication packages are referenced at Harvard Dataverse; benchmark license is CC BY 4.0. Gold includes explicit missing-material cases. | The README does not establish a joined corpus of **agent final claims and raw run artifacts** for each gold outcome; Harvard package availability and each original-data license need case-level checks. Its own published agent comparison is already a strong baseline. |
| [REPRO-Bench](https://github.com/uiuc-kang-lab/REPRO-Bench) and [dataset card](https://huggingface.co/datasets/chuxuan/REPRO-Bench) | 112 cases with original PDF, code/data, gold annotations and public reproduction reports; repo contains baseline agent runners. A subsequent [frozen five-case intake](experiments/repro_bench_intake/RESULT.md) found 4/5 numerical claim/report joins. | The dataset card reports ~183 GB total and a broken viewer. Only 2/5 selected claims had their exact data below the intake's 20 MB/file cap; Stata, manual report linkage, and unresolved reuse terms remain. No code was rerun and no verifier advantage was tested. |
| [AutoMat](https://github.com/JHU-CLSP/AutoMat) | Executable harness and claim-pack layout; repository says 74 claims currently published of an 85-claim study. | Dataset is gated and some paper PDFs must be acquired separately; evaluation uses an LLM-based holistic judge. Access and a genuinely independent numerical oracle are not established for our use. |
| [AgentActionBench task repository](https://github.com/KOU-199024/NLPCC-2026-Shared-Task-11) | Training/validation papers and rubrics; process-level action recording and result-matching criteria. | Test rubrics are withheld, and public training rubrics alone are not joined real-world false-completion cases. Running a competition-style agent is a different question from verifying an existing claim. |

For a reproducible **field-level check**, the published SocSci JSON at
[`benchmark/SocSci_Repro_Bench.json`](https://github.com/malizad/SocSci-Repro-Bench/blob/main/benchmark/SocSci_Repro_Bench.json)
was read without downloading replication packages. SHA-256 of the bytes read:
`af9ebdd013634011c0c7e66027d7081ec8af1a27be3accb3aa7d8b46f100e801`.
With Python `random.seed(20260928)` and five draws from paper IDs 1..54, the
sample is **18, 36, 44, 45, 51**, containing respectively **5, 4, 6, 4, 4**
gold-answer entries. Each sampled row has `id`, `domain`, `language`,
`paper_title`, `Repo`, `task_prompt`, `results`; none has an agent final claim,
trace or generated artifact. Across this exact JSON snapshot, 54 rows contain
**220** result entries, including **7** explicit missing-material answers:
six `No Data` and one `No data or code`. The
README's stated 221 tasks may count an item elsewhere or reflect a revision;
the one-entry discrepancy is **unresolved** and no count was silently coerced.
This source therefore passes a preliminary *task/gold-field* check, and fails
the *agent-claim-field* check for this file. Case-level repository links,
licenses, original data and true numerical reruns remain uninspected.

**Gate A status: IN PROGRESS / INCONCLUSIVE.** REPRO-Bench and SocSci-Repro-
Bench are plausible sources of *paper, materials and independent outcome*;
the checked SocSci JSON alone cannot be the original false-`DONE` corpus.
The next read-only check is a small, predeclared per-case sample from an openly
accessible source that may include actual agent outputs, noting which fields
join and which require restricted access. A missing field is reported as
missing rather than synthesized.

1. **Computational-claim verification in one domain** is the best low-capital
   *qualification* because public papers/artifacts and independent checkers may
   be available, and the owner's strongest transferable skill is designing
   falsifiable evidence. Competition is heavy; a generic product is ruled out.
   The next step is an artifact-availability table, not another framework.
2. **Direct scan-to-fabrication physical validity** is the clearest example of
   changing the task representation. It is a more radical domain change and
   could matter to users, but the core bypass is already published and the
   remaining physical claim needs printers, materials, scans and failure data.
   Keep it in exploratory research unless those resources are accessible.
3. **Target-specific scientific computation** could yield a mathematically
   deep, domain-specific contribution. Its generic form is mature; a candidate
   must identify one measurable end-user decision and show a nontrivial
   end-to-end gap after strong numerical controls.

These ranks reflect cost to *learn whether to proceed*, not expected revenue or
probability of a breakthrough. The other six rows are useful negative controls:
they show how quickly an attractive reformulation can be overtaken by existing
work. This search is broad but **not exhaustive**. No independent user demand,
novelty proof, 4–5x improvement, or commercially viable product has been
established.

## Prospective decision gates

* Gate A, **artifact access**: for each of the top three, list real public
  inputs, reproducible incumbent output, an independent outcome, legal reuse,
  and one prospective owner/user. If not joinable, retire or narrow.
* Gate B, **failure and cost floor**: classify at least 20 real failures or
  expensive runs before designing a model. Measure how much of the end-to-end
  cost the proposed change could eliminate. Reject a 5x target when the
  unchanged floor exceeds 20%.
* Gate C, **matched cheap control**: predeclare the outcome and compare against
  a simple deterministic rule and the best practical incumbent, including
  construction, execution, verification, human correction and failures.
* Gate D, **independent demand**: technical superiority is separate from
  willingness to adopt/pay. Obtain external workflow evidence before a product
  build. Public stars, papers and GitHub issues alone do not satisfy this gate.

No gate beyond desk research was executed in this record. The immediate task is
Gate A for the computational-claim domain; a failed Gate A is a successful
early rejection, not a reason to relabel internal synthetic data as external
evidence. Preserve the other tracks as alternatives, not simultaneous builds.
