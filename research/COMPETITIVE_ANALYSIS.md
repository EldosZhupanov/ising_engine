# Ising Engine — Competitive & Architectural Analysis

**Reviewer stance:** independent scientific reviewer. No credit is given for intent,
documentation, or ambition. Every claim below is sourced from code in
`benchmark_suite/external/` (7 cloned repositories) or from this repository.
Where I could not verify something, it says so.

---

## 1. Executive Summary

**The solver is a research prototype. The laboratory around it is the contribution.**

Measured honestly, the Ising Engine's optimization core is one to two orders of
magnitude smaller and less engineered than the mature solvers it sits beside:

| | Ising Engine | CP-SAT | SCIP |
|---|---|---|---|
| Core solver LOC | `ultimate.rs` **868** | `cp_model_presolve.cc` alone **14,774** | `solve.c` + `scip_solve.c` **10,832** |
| Algorithmic plugins | **14 operators** | 13+ LNS generators + CDCL + LP + cuts | **174 plugins** (64 heur, 34 cons, 22 sepa, 18 presol, 16 branch, 12 prop, 8 nodesel) |
| Presolve | **674 LOC** (QPBO/roof duality) | 14,774 LOC | 18 dedicated presolvers |
| Explicit SIMD | **none** (see §9.1) | — | — |
| GPU | **none** | none | none |

Against the *Ising-specific* field the picture is better but still not leading:
OpenJij ships Swendsen–Wang cluster updates with union-find, continuous-time
Ising, simulated **quantum** annealing with Trotter slices, an OpenMP read-level
parallel sampler, and a real CUDA kernel. We have none of the quantum or GPU
paths.

Where we are genuinely ahead is **not the solver at all**. No system in
`external/` contains anything resembling an autonomous research loop: an
append-only experiment record with provenance, a conditional knowledge graph with
Welford-accrued confidence, a Popperian theory engine that ablates its own
mechanisms, concept discovery under an Occam gate, or bit-identical replay of any
recorded run. That gap is real, and it is the defensible research contribution.

**Verdict in one line:** we should stop implying solver superiority and publish
the autonomous-laboratory architecture, while treating the solver as an
instrument that must merely be *credible*, not *best*.

---

## 2. Competitive Comparison (from source)

### 2.1 D-Wave Neal — `dwave-neal/neal/src/cpu_sa.cpp` (321 LOC)

| Dimension | Finding (evidence) |
|---|---|
| Core algorithm | Single-spin-flip simulated annealing |
| Search strategy | Sequential sweep over all variables per beta |
| Neighborhood | Single spin flip only |
| Acceptance | Metropolis: `exp(-delta_energy[var]*beta) * RANDMAX > rand` (:142) |
| Temperature | Explicit `beta_schedule` vector × `sweeps_per_beta` (:89–96) — geometric by default, computed in Python |
| Parallelization | **None inside a solve.** `num_reads` run sequentially inside one `nogil` block (`simulated_annealing.pyx:135`) |
| SIMD | **None** |
| GPU | **None** |
| Threading | `thread_local uint64_t rng_state[2]` (:37) — safety only, not parallelism |
| Memory layout | `vector<vector<int>> neighbors`, `vector<vector<double>> neighbour_couplings` (`cpu_sa.h:14–19`) — **adjacency-of-vectors:每 row a separate heap allocation** |
| Cache friendliness | **Weak.** Pointer-chasing per neighbor list; no CSR |
| Presolve | None |
| Local search | Implicit (greedy accepted when `delta <= 0`, :133) |
| Restart | Via `num_reads` only |
| Adaptive | None — schedule fixed up front |
| Data structures | `char *state` (1 byte/spin), incremental `double *delta_energy` |
| Scalability | Fine to ~10⁴–10⁵ sparse; layout hurts beyond |
| Strong ideas | **(a)** Incremental delta-energy array updated O(deg) per accepted flip, never recomputed (:148–155). **(b)** `threshold = 44.36142 / beta` skips `exp()` when acceptance is impossible at 64-bit RNG resolution (:124–129) — a genuinely clever micro-optimization. **(c)** xorshift128+ `FASTRAND` macro instead of `std::mt19937` (:23–29) |
| Weak ideas | vector-of-vectors layout; no parallelism; no adaptivity |

### 2.2 OpenJij — `include/openjij/**` (97 C++ headers)

| Dimension | Finding (evidence) |
|---|---|
| Core algorithm | SA **and** SQA (transverse-field Ising, `transverse_ising.hpp`, `gamma`, `num_trotter_slices`) |
| Search strategy | Multiple *systems* × *updaters* (a genuine strategy matrix) |
| Neighborhood | `single_spin_flip`, **`swendsen_wang`** (cluster), `continuous_time_swendsen_wang`, `k_local`, `single_integer_move` |
| Acceptance | `dE <= 0 || exp(-beta*dE) > urd(rng)` (`single_spin_flip.hpp:80–82`) |
| Temperature | Schedule list; SQA adds `(1/2)·log(tanh(beta·gamma·(1-s)/num_trotter_slices))` (:148) |
| Parallelization | **OpenMP over reads**: `#pragma omp parallel for schedule(guided) num_threads(num_threads_)` (`sa_sampler.hpp:190,285`); also `parallel for reduction(+:energy)` (`polynomial.hpp:211`) |
| SIMD | **Implicit via Eigen** (`Eigen::Matrix<FloatType,Dynamic,Dynamic,RowMajor>`, `classical_ising.hpp:49–52`); vectorized dE update `system.dE += 4*spin(index)*…` (:84) |
| GPU | **Yes** — `system/gpu/chimera_cuda/kernel.cu`, `__global__ void metropolis`, shared-memory caches for spins, RNG and 6 coupling planes (:34–63). **Chimera-topology-specific, not general** |
| Threading | OpenMP |
| Memory layout | **Dense `MatrixXx` for classical Ising** (O(n²)) *and* `graph/csr_sparse.hpp` for sparse |
| Cache friendliness | Good for dense; CSR available |
| Presolve | None |
| Local search | Cluster moves subsume some of it |
| Restart | `num_reads` |
| Adaptive | Minimal; schedules are supplied |
| Data structures | Eigen matrices/vectors, `UnionFind` (`utility/union_find.hpp`) for clusters |
| Scalability | Dense path is O(n²) memory — a hard ceiling; sparse path better |
| Strong ideas | Cluster (Swendsen–Wang) updates with union-find; SQA/continuous-time; system×updater orthogonality; real GPU kernel with shared-memory tiling |
| Weak ideas | Dense matrix default; `std::uniform_real_distribution` + `std::exp` in the hot loop (slower than Neal's trick); GPU limited to Chimera |

### 2.3 OR-Tools CP-SAT — `ortools/sat/**` (1,582 C++ files)

| Dimension | Finding (evidence) |
|---|---|
| Core algorithm | CDCL SAT (`sat_solver.cc` 3,156; `clause.cc` 3,644) + LP relaxation (`linear_programming_constraint.cc` 3,014) + cutting planes (`cuts.cc` 3,037) + LNS |
| Search strategy | **Parallel portfolio of subsolvers** (`subsolver.h:44 class SubSolver`) with `SharedBoundsManager` (`synchronization.h:661`) — workers exchange bounds/solutions |
| Neighborhood | **13+ structure-aware LNS generators**: ConstraintGraph, DecompositionGraph, ArcGraph, RelaxationInduced, LocalBranchingLpBased, Routing{Path,FullPath,Random}, Rectangles/Packing, Scheduling{Interval,Precedence} |
| Acceptance | Exact (branch-and-bound with proofs), not stochastic |
| Temperature | N/A |
| Parallelization | Multi-worker portfolio + shared clause/bound synchronization |
| SIMD | Not a design axis |
| GPU | None |
| Threading | `num_workers`, `Synchronize()` per subsolver |
| Memory layout | Specialized literal/clause structures; watched literals |
| Presolve | **`cp_model_presolve.cc` 14,774 LOC** + `sat_inprocessing.cc` 3,383 (in-search simplification) |
| Local search | Feasibility jump + LNS |
| Restart | Standard CDCL restarts; LNS restarts |
| Adaptive | **Yes** — per-generator `difficulty` adapted from outcome, with `deterministic_time` accounting (`cp_model_lns.h:396–437`) |
| Data structures | Watched literals, trail, LP tableau |
| Scalability | Industrial; millions of variables |
| Strong ideas | Portfolio + shared learning; adaptive LNS difficulty; **deterministic time** accounting (reproducible parallel runs); enormous presolve |
| Weak ideas | Complexity; not designed for dense Ising |

### 2.4 SCIP — `src/scip/**` (685 C, 1,069 C/H)

| Dimension | Finding |
|---|---|
| Core algorithm | Branch-cut-and-price MILP/MINLP |
| Architecture | **Plugin registry**: 64 heuristics, 34 constraint handlers, 22 separators, 18 presolvers, 16 branching rules, 12 propagators, 8 node selectors = **174 plugins** |
| Adaptive | ALNS heuristic, adaptive diving, restarts |
| Presolve | 18 dedicated presolvers |
| Strong ideas | The plugin/registry architecture itself — the clearest prior art for our "operators selected by capability" idea, at 12× the scale |
| Weak ideas | C, heavy configuration surface |

### 2.5 dimod / dwave-ocean-sdk / PySCIPOpt

- **dimod** (146 py, 41 pyx, 21 C++): the *data model* — BQM/CQM/QM containers, samplers ABC, `AdjVectorBQM`-style storage. Not a solver. **This is the closest prior art to our `ProblemIR`**, and it is considerably richer (multiple views, symbolic math, constraint models).
- **dwave-ocean-sdk**: meta-package (6 py) — dependency aggregation only. No algorithms.
- **PySCIPOpt**: Cython bindings (129 py, 2 pyx). No algorithms.

### 2.6 Ising Engine (this repository)

| Dimension | Finding (evidence) |
|---|---|
| Core algorithm | PT + population annealing + 14 operators over a deterministic Runtime |
| Neighborhood | 14 operators incl. `houdayer_cluster`, `isoenergetic_cluster`, `replica_exchange`, `population_resample` — **comparable in kind to OpenJij's cluster set** |
| Parallelization | rayon (`solver/engine.rs`, `solver/cluster.rs`); read/replica level |
| SIMD | **NONE.** `core/simd_utils.rs` is **orphaned — not declared in any `mod.rs`, never compiled**; its own header calls it *"A theoretical implementation… a foundation for replacing the serial iteration in ultimate.rs"*. Only `-Ctarget-cpu=native -Ctarget-feature=+avx2,+fma` auto-vectorization is real |
| GPU | **NONE** |
| Memory layout | **Symmetric CSR** (`ir.rs:14–24`: `row_ptr`, `col_idx`, `weights`) — **better than Neal's vector-of-vectors**, comparable to OpenJij's CSR path |
| Presolve | 674 LOC QPBO/roof-duality |
| Adaptive | Runtime `maybe_adapt` controllers; Dynamics early-stop (opt-in) |
| Determinism | **Bit-identical replay, cross-backend verified >100k spins** — stronger than any competitor here (CP-SAT offers deterministic *time*, not bit-identical trajectories) |
| Scalability | Verified on G-Set n ≤ 2,000; **untested beyond** |

---

## 3. Architecture Comparison (whole platform)

The comparison inverts once we leave the solver.

| Subsystem | Ising Engine | Neal | OpenJij | CP-SAT | SCIP | dimod |
|---|---|---|---|---|---|---|
| Solver core | prototype | ✔ | ✔✔ | ✔✔✔ | ✔✔✔ | — |
| Problem IR | CSR | adj-vectors | Eigen/CSR | proto-based | LP/MPS | ✔✔ (richest) |
| Presolve | small | — | — | ✔✔✔ | ✔✔✔ | — |
| Plugin/operator registry | 14, capability-typed | — | system×updater | subsolvers | **174 plugins** | — |
| Parallel portfolio | — | — | reads | ✔✔✔ shared bounds | ✔✔ | — |
| GPU | — | — | ✔ (Chimera) | — | — | — |
| **Append-only experiment DB** | **✔ 109,758 runs** | — | — | — | — | — |
| **Knowledge graph (conditional, weighted)** | **✔** | — | — | — | — | — |
| **Theory engine (ablation-falsified)** | **✔** | — | — | — | — | — |
| **Concept discovery (Occam-gated)** | **✔** | — | — | — | — | — |
| **Scientific memory (structure-keyed recall)** | **✔** | — | — | — | — | — |
| **Bit-identical replay of any record** | **✔** | — | — | — | — | — |
| **Autonomous campaign supervisor** | **✔** | — | — | — | — | — |
| **Discovery center / gap mining** | **✔** | — | — | — | — | — |
| **Publication composer** | **✔** | — | — | — | — | — |
| **Workflow OS / research curriculum** | **✔** | — | — | — | — | — |
| Benchmark harness | small | — | — | internal | internal | — |
| Model registry (versioned, lineage) | **✔ 1,201 snapshots** | — | — | — | — | — |

**Unique to us (no counterpart in any cloned repo):** experiment DB with
provenance, knowledge graph, theory engine, concept discovery, scientific memory,
replay, campaign supervisor, discovery center, publication composer, workflow OS,
model registry.

**Everyone else is better at:** the solver, presolve, portfolio parallelism,
plugin breadth, and — critically — *validated benchmarking at scale*.

---

## 4. Feature Matrix — Scientific Workflow

| Capability | Us | Best competitor | Gap |
|---|---|---|---|
| Reproducible runs | bit-identical | CP-SAT deterministic time | **we lead** |
| Provenance per result | full (seed, campaign, gen) | none | **we lead** |
| Automated hypothesis generation | ✔ (LLM + evolution) | none | **we lead** |
| Automated falsification | ✔ (ablation) | none | **we lead** |
| Cross-instance rule reproduction | ✔ (evaluation) | none | **we lead** |
| Statistical rigor in reporting | Wilcoxon + power caveats | internal only | comparable |
| **Benchmark breadth** | G-Set only, n≤2000 | MIPLIB, SATLIB, TSPLIB, industrial | **we are far behind** |
| **Baseline coverage** | OpenJij, neal | full field | **we are far behind** |
| **Peer-reviewed validation** | none | decades | **we are far behind** |

---

## 5. AI Scientist Comparison

There is **no competitor** in `external/` with an AI-scientist layer. The honest
comparison class is therefore the literature, not these repos:

| System | What it does | How we differ |
|---|---|---|
| CP-SAT adaptive LNS | Adapts neighborhood `difficulty` from measured outcomes | Same *idea*, narrower scope; theirs is battle-tested at industrial scale |
| SCIP ALNS / adaptive diving | Bandit-like heuristic selection | Ours adds *conditional knowledge* and *causal ablation*, theirs adds *decades of tuning* |
| AutoML / SMAC / irace | Algorithm configuration | They optimize parameters; we attempt to *discover mechanisms and concepts* |
| "AI Scientist" LLM papers (2024–) | LLM writes/executes/reviews papers | We are grounded in a deterministic runtime with falsification — a genuine methodological advantage |

**Our defensible novelty:** the combination of (a) deterministic, replayable
execution, (b) append-only evidence, (c) conditional knowledge with confidence,
and (d) mandatory ablation before a mechanism becomes a theory. No cloned system
has any of it; the LLM-scientist literature has (d) rarely and (a) almost never.

---

## 6. Missing Capabilities

### 6.1 Solver / HPC (largest gap)
1. **Real SIMD** — we have literally none. AVX2/AVX-512 spin-flip kernels, bit-packed multi-replica updates.
2. **GPU execution** — OpenJij has CUDA; we have nothing. Multi-replica PT is embarrassingly parallel on GPU.
3. **Distributed execution** — no MPI/multi-node. SC/IPDPS submissions require it.
4. **Deterministic parallel time accounting** (CP-SAT has it) — required for fair parallel benchmarking.
5. **Presolve depth** — 674 LOC vs 14,774. Missing: probing, clique merging, coefficient tightening, symmetry detection.
6. **Portfolio with shared learning** — no cross-worker bound/solution sharing.
7. **Large-scale validation** — nothing above n=2,000.

### 6.2 Scientific method
8. **Bayesian experimental design** — the planner ranks by heuristic "expected new knowledge", not posterior information gain / EIG.
9. **Causal knowledge graph** — current edges are conditional-correlational; no do-calculus, no confounder adjustment (we *already refuted* one claim on a density/ruggedness confound — that is exactly the machinery we lack).
10. **Counter-example generation** — no adversarial instance synthesis to break a theory.
11. **Theory versioning/diff** — theories have no persistent store or lineage.
12. **Multi-objective** — energy only; no Pareto over (quality, time, energy-to-target, robustness).
13. **Time-to-target / ECDF analysis** — the field standard (runtime distributions); we report best-score only.
14. **Auto benchmark generation** — no controlled instance synthesis with known planted optima.
15. **Continual/active learning** — models retrain in batch; no online updating or acquisition-driven sampling.

### 6.3 Platform
16. **Runtime telemetry export** — `StepEvent`/`RunRecord` never serialized (already identified); blocks energy-landscape and trajectory analysis.
17. **Universal reduction engine** — the Application Explorer cites reductions but cannot *execute* them; a real reduction compiler (SAT/graph-coloring/knapsack → QUBO) would make T2 claims testable.
18. **Operator synthesis** — we mine *gaps* and draft proposals, but no operator is ever synthesized, compiled, and harness-verified. This is the single biggest unrealized idea in the platform.
19. **Hyperparameter intelligence** — no SMAC/BOHB-class tuner.
20. **Multi-user / distributed campaigns**, FPGA, quantum-hybrid backends.

---

## 7. Novel Research Opportunities (publishable)

1. **Falsification-driven algorithm discovery** — an autonomous loop that requires ablation before accepting a mechanism. Novel vs both AutoML and LLM-scientist work. *(Venue: NeurIPS/ICML)*
2. **Concept discovery under an Occam gate** — learning new *structural features* out-of-sample rather than tuning weights. We already have a real negative result (11-feature descriptor rejected). *(ICML/AAAI)*
3. **Operator synthesis from adjacency mining** — the `metropolis_gibbs` fusion proposal is a genuine machine-generated algorithmic hypothesis; synthesizing and verifying it would be a first. *(AAAI/GECCO)*
4. **Bit-identical replay as a reproducibility standard** for stochastic solver research. *(SC/IPDPS reproducibility track)*
5. **Cross-family strategy transfer** — 9/12 at ρ +0.747 is a real, if small, result worth scaling. *(ICML)*

---

## 8. Honest Critique

### 8.1 Weak architectural decisions
- **SIMD theater.** `.cargo/config.toml` advertises `+avx2,+fma`, and a file named `simd_utils.rs` exists — but it is **never compiled**. Anyone auditing this will read it as a performance claim unsupported by code.
- **Dual solver lineage.** `solver/ultimate.rs` (production) and `engine_v2` (research) are separate universes bridged only by an energy-exact frontend. Justified by ADR, but it means research findings do not automatically improve the shipped solver.
- **In-memory-only faculties.** Curiosity, planner agenda, memory recall, concept admissions, runtime events — all computed then discarded. We only just fixed recall.
- **Campaign = directory** with a single-writer constraint; no concurrency story for multi-node.

### 8.2 Technical debt
- Orphaned, never-compiled files: `ultimate_patch.rs`, `tabu.rs`, `logic_builder_patch{,_mux}.rs`, `qubo_model.rs`, `simd_utils.rs`.
- Pre-existing broken workspace member: `research/src/bin/test_engine.rs` (missing `energy_offset`).
- Debug profile cannot link (`.cargo/config.toml` forces `-Copt-level=3`); release-only workflow is undocumented outside the run skill.
- Ingest loads whole files into memory — will not survive the "millions of rows" the docs anticipate.

### 8.3 Missing scientific rigor
- **Benchmark scale is the fatal weakness for publication.** Our only significant head-to-head is `sparse_colorsweep_v01` vs `openjij_sa`, **n=23, p=2.7e-05** — real, but a *single* baseline on *one* family at n≤2000. Everything else is n=2–3 at p=0.25.
- **No comparison at all against CP-SAT, SCIP, Gurobi, or specialized MaxCut heuristics** (MQLib, Burer–Monteiro), despite CP-SAT and SCIP being cloned locally.
- No time-to-target distributions, no performance profiles, no ECDF — the standard currency of solver papers.
- No statistical correction for multiple comparisons across 18,570 runs of hypothesis testing.
- Baseline definition is internal; not tied to published G-Set best-known values in the reported figures.

### 8.4 Missing UX for researchers
- No CLI↔UI parity for the newer capabilities (replay/memory are UI/API-only).
- No notebook/Python API — the entire scientific-computing audience works in Python; we ship neither a `pip` package nor a `dimod`-compatible sampler interface. **This alone will block adoption.**
- No export to standard formats (no `dimod.BinaryQuadraticModel`, no MPS/LP round-trip).

---

## 9. Engineering Recommendations (ranked by value ÷ effort)

1. **Delete or implement `simd_utils.rs`.** Either ship real AVX2 kernels or remove the file. The current state is the single most damaging thing an expert reviewer would find.
2. **Ship a `dimod`-compatible Python sampler.** Instant comparability with the entire D-Wave ecosystem and a credible benchmarking story.
3. **Serialize the runtime event log.** Unlocks landscapes, trajectory analysis, and honest telemetry.
4. **Benchmark against CP-SAT and SCIP** — they are already cloned. Publish losses.
5. **Adopt time-to-target + ECDF reporting.**
6. **Bandit-based operator selection** with deterministic-time accounting (copy CP-SAT's proven design).
7. **GPU PT** — multi-replica is embarrassingly parallel; OpenJij proves the pattern.

---

## 10. Five-Year Roadmap

### Phase 1 — Credibility (month 1)
- Remove SIMD theater; implement one real AVX2 spin-flip kernel with a bit-identity test.
- `dimod`-compatible sampler + pip package.
- Benchmark vs CP-SAT/SCIP/neal/OpenJij on G-Set + MQLib, with TTT/ECDF.
- **Why:** without this, no venue takes the solver seriously. **Difficulty:** M. **Advantage:** entry ticket.

### Phase 2 — Instrumented science (months 2–4)
- Runtime event-log export → energy landscapes, trajectory scrubber.
- Curiosity/planner export → Discovery Center Stage 2.
- Theory persistence + curation (epistemic firewall), counter-example generator.
- **Why:** turns claims into inspectable evidence. **Difficulty:** M–L. **Advantage:** reproducibility leadership.

### Phase 3 — Real discovery (months 5–12)
- **Operator synthesis**: proposal → generated Rust → harness verification → registry admission, all gated by bit-identity and ablation.
- Bayesian experimental design (EIG-driven planner).
- Universal reduction engine (executable, not cited).
- **Why:** this is the actual thesis — machines inventing algorithms. **Difficulty:** XL. **Advantage:** unique.

### Phase 4 — Scale (year 2)
- GPU PT; MPI multi-node campaigns; deterministic parallel time.
- n ≥ 10⁵–10⁶ validation.
- **Why:** SC/IPDPS relevance. **Difficulty:** XL.

### Phase 5 — Platform (years 3–5)
- Causal knowledge graph with intervention semantics.
- Multi-user, marketplaces for operators/theories, quantum-hybrid backends.
- **Why:** becomes the reference laboratory. **Difficulty:** XL.

---

## 11. Final Verdict

**As a solver:** not competitive, and the gap is structural — 868 lines against
CP-SAT's 14,774-line presolve, no SIMD, no GPU, no distributed execution, and one
statistically significant head-to-head against a single baseline at n≤2,000.
Claims of superiority would not survive review, and the repository's own ROADMAP
already concedes evolved plans lose 5/5 to `UltimateSolver`.

**As an autonomous research laboratory:** genuinely novel. Nothing in the seven
cloned repositories — including two of the strongest solvers ever written —
contains an append-only evidence base, a conditional knowledge graph, an
ablation-driven theory engine, Occam-gated concept discovery, or bit-identical
replay. That combination is publishable.

**Strategic recommendation:** reposition. The paper is *"a falsification-driven
autonomous laboratory for algorithm discovery"*, with the Ising solver as the
substrate — not *"a fast Ising solver"*. Then close the credibility gap in
Phase 1 so the substrate cannot be used to dismiss the science.

The greatest present risk is not missing features. It is the mismatch between the
platform's rigorous internal honesty and a few external claims (SIMD, and any
implication of solver superiority) that the code does not support.
