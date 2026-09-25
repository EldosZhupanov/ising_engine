# PROJECT_STATE — Factual State of Ising Engine

Date: 2026-09-25  
Repository: `https://github.com/eldos/ising_engine`  
Authority Scope: Fact-based baseline audit. No marketing, no unverified claims.

---

## 1. Architectural Architecture & Crate Structure

The repository is organized as a Cargo workspace containing the core engine (`ising_engine`) and a `research` workspace member.

### Solver Families & Boundaries (Invariant per `AGENTS.md`)
The codebase strictly isolates two solver families to prevent cross-contamination:

| Dimension | Scalar Family | MSC / Vectorized Family (`UltimateSolver`) |
|---|---|---|
| **Main Files** | `src/solver/parallel_tempering.rs`, `adaptive.rs`, `cluster.rs`, `tabu.rs`, `icm.rs`, `population_annealing.rs` | `src/solver/ultimate.rs`, `src/solver/engine.rs`, `src/solver/types.rs` |
| **Data Representation** | `QuboModel` + `CsrMatrix` (compressed sparse row) | `HuboModel` / `FlatHuboModel` (CSR-style contiguous flat arrays) |
| **State Ownership** | Independent `Replica` instances per temperature | `QuantumField`: 5D Structure-of-Arrays (SoA) layout |
| **Optimization Target** | Pairwise quadratic Ising/QUBO | High-order binary optimization (HUBO) up to degree 4 + transverse field SQA |
| **Execution Model** | Rayon thread-parallel replicas | Vectorized SIMD (AVX2) inner loop across 64 replicas per variable |

### Presolve and Preprocessing Modules
- `src/presolve/qpbo.rs`: Exact Roof Duality / QPBO via Dinic's max-flow algorithm with weak persistency detection using Tarjan's Strongly Connected Components (SCC) on the Boros-Hammer implication network $N_f$. Exhaustively tested in `tests/test_qpbo.rs`.
- `src/presolve/mod.rs`: Probing, variable clamping, and cascade reductions.

### Compiler Modules
- `src/compiler/logic_builder.rs`: Penalty-based mapping of boolean logic gates (AND, OR, NOT, XOR, Half-Adder, Full-Adder, 2x2 Multiplier) into quadratic (QUBO) and cubic/quartic (HUBO) polynomials with exact zero ground-state energy for satisfied assignments.

### Engine V2 Experimental Framework
- `src/engine_v2/`: Experimental operator-based abstraction framework (`dense_byte`, `sparse_bitslice`, `reference` backends; operators for Metropolis, cluster, tabu, path-relinking, ensemble thermostat).
- `src/engine_v2/ai_scientist/`: Structural scaffolding for autonomous hypothesis generation and evaluation (`campaign`, `concept`, `curiosity`, `policy`, `world`, `llm`).

---

## 2. Implemented Algorithms vs Claimed/Stubbed Features

### A. Actually Implemented & Verified in Code
1. **Vectorized High-Order HUBO Engine (`src/solver/engine.rs`)**:
   - Degree-2, degree-3, and degree-4 energy delta evaluation in single-cycle loops.
   - Byte-per-replica layout: 64 replicas laid out contiguously per variable, enabling AVX2 packed vectorization.
   - 8-lane interleaved Xoshiro256++ PRNG (Blackman & Vigna, ACM TOMS 2021) generating independent uniform streams via mantissa bit-trick.
2. **Parallel Tempering (Replica Exchange)**:
   - Metropolis sweeps with exact Boltzmann swap acceptance criterion $\min(1, \exp((\beta_i - \beta_j)(E_j - E_i)))$.
   - Verified energy tracking without O(N) recomputation per step.
3. **Adaptive Temperature Ladder Tuning**:
   - Acceptance uniformization (Rathore-Chopra-de Pablo 2005 / Kofke 2002).
   - Feedback-optimized round-trip ladder (Katzgraber-Trebst-Huse-Troyer 2006).
4. **Population Annealing (PA-PT)**:
   - Multi-population resampling based on Effective Sample Size (ESS) thresholding.
   - Post-resample decorrelation sweeps (Wang, Machta & Katzgraber 2015).
5. **Isoenergetic Cluster Moves (ICM)**:
   - Houdayer / Zhu-Ochoa-Katzgraber cluster algorithm for pairwise spin models.
6. **Path-Relinking Finisher**:
   - Elite pool construction with diversity constraints and greedy path-relinking (Wang-Lü-Glover-Hao 2012).
7. **Local Search & Exact 2-Opt Quenching**:
   - Deterministic 1-opt local search.
   - $O(N^2)$ deterministic 2-opt search (used in LABS and Market Split).
   - Edge-restricted 2-opt escape on graph edges.
8. **Exact Roof Duality / QPBO Presolver**:
   - Dinic max-flow, Tarjan SCC decomposition on implication network, weak persistency variable fixing.
9. **Time-to-Solution (TTS) & Heavy-Tailed Restarts**:
   - Exact Luby restart schedules and bootstrap confidence interval computation.

### B. Claimed in Documentation or Investor Pitch, but Stubbed or Absent in Code
1. **"GNN Hybrid Optimization / Learned Search Policy"**:
   - Claimed in `gnn_design_spec.md` and investor presentations as a Graph Neural Network guiding search.
   - **Fact in code**: `UltimateSolver.gnn_heuristic_probs` is simply an `Option<Vec<f64>>` representing initial variable flip probabilities. There is no neural network forward pass, no PyTorch/ONNX runtime, and no trained model in the engine. `src/bin/gnn_hybrid_demo.rs` is a synthetic scaffold.
2. **"Quantum Advantage / 1000x Speedup over D-Wave Quantum Hardware"**:
   - Claimed in early marketing text (`YC_APPLICATION_FALL_2026.md`, `investor_proofs_*.rs`).
   - **Fact in code**: Comparisons compared CPU runtimes against estimated D-Wave minor-embedding latency or simulated annealing baselines. No rigorous equal-hardware, temperature-corrected, or scaling-advantage proof against actual QPU execution exists.
3. **"Autonomous Self-Writing AI Scientist"**:
   - Claimed in `src/engine_v2/ai_scientist/` as an autonomous closed-loop discovery system.
   - **Fact in code**: The modules consist of data structures and heuristic dispatch rules, but do not autonomously discover novel mathematical theorems or write novel Rust kernels outside human-guided preregistrations.
4. **"Exact Branch and Bound / Backjump Speedup (CD004)"**:
   - Historically claimed 33–54% node reduction via backjumping.
   - **Fact in code**: Recheck on 2026-09-25 (`research/breakthrough/cd004_recheck/RESULT.md`) revealed an algorithmic bug in the original prototype (counterexample found); the corrected certified version showed 0/120 speedup over chronological BnB (median wall ratio 4.876x slower).

---

## 3. Benchmark Reproducibility Status

| Benchmark | Dataset / Class | Reference / Baseline | Replicated Result in Engine | Official Verifier Status |
|---|---|---|---|---|
| **QOBLIB Class 01** | Market Split (`ms_03_*`) | Q-Bridge GPU SA (4.3–4.4s) | 6/6 instances solved to exact 0 violations in 0.70s–2.02s | **VALID** (`check_marketsplit`) |
| **QOBLIB Class 02** | LABS ($N \in [67..74]$) | Knauer 2004 22-year records | $N=74 \to E=357$ (gap +16), $N=70 \to E=319$ (gap +24) | **VALID** (`check_labs`) |
| **QOBLIB Class 07** | Max Independent Set | Gurobi / Official BKV | Exact optimum on `sloane_1dc_*`, 99.3% on `socfb-haverford76` | **VALID** (`check_stableset`) |
| **G-Set MaxCut** | Graphs $G1 \dots G80$ | Published heuristic bounds | Matches or approaches published cuts on classical instances | Internal verifier |
| **Sherrington-Kirkpatrick**| $N=256, 512, 1024$ | Parisi asymptotic limit $-0.76321$ | Converges to $e \approx -0.73 \dots -0.74$ via multi-start quenches | Mathematically bounded |
| **Roof Duality Presolve** | Posiform random instances | Brute-force exhaustive ground states | 100% preservation of optimal energy under variable fixing | **PASS** (`test_qpbo.rs`) |

---

## 4. Unconfirmed Scientific Claims

1. **"Scaling Advantage over Classical Solvers"**:
   - Status: **UNVERIFIED / INSUFFICIENT EVIDENCE**.
   - No asymptotic scaling advantage ($O(N^a)$ vs $O(N^b)$) has been proven against Gurobi, Biq Mac, KaMIS, or specialized branch-and-bound. Observed advantages are constant-factor throughput improvements from AVX2 memory locality and multi-replica parallelism.
2. **"World Record Beaten in LABS ($N \ge 67$)"**:
   - Status: **UNCONFIRMED**.
   - Knauer 2004 world records remain unbroken: $N=74$ is at 357 (record 341, gap +16); $N=67$ is at 289 (record 241, gap +48).
3. **"General 2-Opt Advantage on Unweighted MIS"**:
   - Status: **REFUTED (NO-GO, 2026-09-25)**.
   - For unweighted MIS QUBO with penalty $P=2$, any 1-opt local minimum algebraically precludes strict 2-flip improvements.

---

## 5. Scientifically Most Promising Components

1. **Native SIMD High-Order HUBO Kernel**:
   - Direct optimization of 3rd and 4th order terms in AVX2 without ancilla binarization penalties.
   - Reduces variable inflation and avoids large energy penalty barriers required by degree reduction (e.g. Freedman / Ishikawa).
2. **Exact Roof Duality / Implication Network Presolver**:
   - Mathematically sound, polynomial-time reduction for submodular/partially submodular components via Dinic max-flow.
3. **Adaptive Feedback-Tuned Population Annealing**:
   - Systematic resampling coupled with KTHT round-trip feedback provides high ergodic mobility across deep spin-glass valleys.
4. **Memetic 2-Opt Tabu Search on Autocorrelation Landscapes**:
   - Demonstrated large energy reduction on rugged non-convex landscapes (LABS gap compressed from +112 to +16).

---

## 6. Potentially Novel Scientific Contributions

1. **5D SIMD Memory Layout (`QuantumField`) with Interleaved 8-Lane Vector RNG**:
   - Contiguous replica packing enabling branchless AVX2 auto-vectorization across 64 replicas simultaneously with zero per-sweep heap allocations.
2. **Empirical Quality vs Diversity Dissociation in Spin-Glass Annealing (RC-002)**:
   - Experimental proof that warm-start quality is rapidly erased at thermal depth, while diversity survives and dictates ground-state hit rates.
3. **Direct Integration of Exact Roof-Duality Persistency with Multi-Replica HUBO Annealing**:
   - Combining max-flow presolve with vectorized population annealing.

---

## 7. Identified Benchmark Leakage & Methodological Risks

1. **Hardcoded Parameter Specialization**:
   - Some historical benchmark scripts contain hardcoded temperature ladders and sweep counts specifically tuned to known instances (e.g., G-set graphs).
2. **Problem-Specific Prior Leakage**:
   - In `labs_record_hunter.rs`, initial sequences are biased with leading 1s (`prefix_ones`), which exploits known mathematical properties of Barker/LABS sequences rather than general solver search.
3. **Target-Hitting vs Fixed-Budget Bias**:
   - Halting upon reaching a known target energy without reporting failure rate or timeout distribution overestimates solver efficiency (censored run distortion).
4. **Hardware Normalization**:
   - Comparisons between local CPU runtimes and cloud GPU / QPU baselines must explicitly account for core count, thermal conditions, compiler flags, and memory bandwidth.
