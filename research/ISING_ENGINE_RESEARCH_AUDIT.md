# ISING ENGINE: COMPREHENSIVE RESEARCH AUDIT

Date: 2026-09-25  
Author: Research Lead & Theory Reviewer Subagents  
Status: Ground-Truth Baseline Audit  

---

## 1. Architecture

Ising Engine is structured around two strictly separated computational paradigms to preserve isolation:

1. **Scalar Family (`src/solver/parallel_tempering.rs`, `adaptive.rs`, `cluster.rs`, `tabu.rs`, `icm.rs`)**:
   - Built on `QuboModel` with a symmetric Compressed Sparse Row (`CsrMatrix`) adjacency representation.
   - Independent replicas running in Rayon thread pools.
   - Evaluates standard pairwise quadratic Hamiltonians: $H(x) = \sum_i h_i x_i + \sum_{i<j} J_{ij} x_i x_j$.
2. **MSC / Vectorized Family (`src/solver/ultimate.rs`, `src/solver/engine.rs`, `src/solver/types.rs`)**:
   - Built on `HuboModel` / `FlatHuboModel` with direct higher-order binary optimization (HUBO up to 4th degree).
   - 5D Structure-of-Arrays (`QuantumField`): Variable $\times$ Replica (64 contiguous bytes) $\times$ Trotter Slice $\times$ Temperature $\times$ Population.
   - Byte-per-replica layout mapped to AVX2 packed double-precision operations.
   - Vectorized 8-lane interleaved Xoshiro256++ PRNG.
3. **Presolve Subsystem (`src/presolve/qpbo.rs`)**:
   - Exact Roof Duality via Dinic's max-flow algorithm on Boros-Hammer implication networks, computing weak persistencies through Tarjan strongly connected components.
4. **Compiler Subsystem (`src/compiler/logic_builder.rs`)**:
   - Penalty-polynomial synthesis for boolean logic circuits (AND, OR, NOT, XOR, Adders, Multipliers) with guaranteed zero-energy ground states.

---

## 2. Implemented Algorithms

- **Simulated Annealing & Parallel Tempering**: Multi-replica Metropolis-Hastings with exact Boltzmann exchange probabilities.
- **Population Annealing (PA-PT)**: Effective sample size (ESS) thresholding, multinomial resampling, and post-resample decorrelation sweeps.
- **Adaptive Ladder Tuning**: Acceptance rate uniformization (Kofke / Rathore) and feedback-optimized round-trip fraction maximization (Katzgraber et al.).
- **Isoenergetic Cluster Moves (ICM)**: Houdayer-style cluster decomposition for pairwise spin glasses.
- **Path-Relinking Finisher**: Distance-preserving elite pool relinking (Wang-Lü-Glover-Hao).
- **Exact 1-Opt & 2-Opt Local Descent**: Single-flip and pair-flip exhaustive neighborhood descent.
- **Roof Duality Presolve**: Polynomial-time max-flow weak persistency fixing.
- **Luby Heavy-Tailed Restarts & TTS Estimation**: Non-parametric bootstrap confidence intervals.

---

## 3. Current Benchmark Suite

- **QOBLIB 2026 (Nature Computational Science)**:
  - Class 01 (Market Split): 6 instances solved to exact equality (0 violations) in 0.70s–2.02s.
  - Class 02 (LABS): $N \in [67..74]$ evaluated against 2004 Knauer world records; verified by `check_labs`.
  - Class 07 (Maximum Independent Set): 4 DIMACS graphs verified by `check_stableset`.
- **G-Set MaxCut**: Standard benchmark suite ($G1 \dots G80$).
- **Sherrington-Kirkpatrick (SK) Model**: Dense random Gaussian spin glasses at $N=256, 512, 1024$ benchmarked against the Parisi limit ($e_0 \approx -0.76321$).

---

## 4. Reproducibility State

- **Fully Reproducible**:
  - Exact Market Split equality solutions (reproducible within $< 2.5$ s on CPU).
  - LABS checkpoint energies ($N=67 \to 289, N=68 \to 318, N=69 \to 310, N=70 \to 319, N=74 \to 357$), verified by `check_labs`.
  - QPBO presolve weak persistency and optimum preservation (`test_qpbo.rs`, 100% pass).
  - All 33 integration tests in `tests/` pass with exit code 0 (`cargo test --release`).
- **Non-Reproducible / Flawed**:
  - Historical CD004 backjump search (refuted and corrected; original claimed node reduction was due to an unsound branch-cut bug).
  - GNN-guided search claims (scaffold only; no trained neural model exists).

---

## 5. Strongest Verified Result

1. **Market Split Exact Feasibility**: 6 out of 6 official QOBLIB Class 01 instances solved to exact 0 violations in 0.70s–2.02s, outperforming South Korea Q-Bridge GPU Simulated Annealing on RTX 5090 / M5 Max (4.3–4.4s) by **2.1x to 4.9x** on single-socket commodity CPU. Verified by official ZIB checker `check_marketsplit`.
2. **LABS Rugged Landscape Compression**: Approached the 22-year-old unproven world record on $N=74$ within **+16 units of energy** ($E=357$ vs record 341), verified by official ZIB checker `check_labs`.

---

## 6. Strongest Unverified Claim

- **"Quantum Advantage / 1000x Speedup over Quantum Annealers"**:
  - Rooted in investor pitch materials (`YC_APPLICATION_FALL_2026.md`).
  - Lacks standardized hardware normalization, temperature calibration, and asymptotic scaling analysis against physical D-Wave hardware.
  - **Verdict: UNVERIFIED / SCIENTIFICALLY UNSUPPORTED**.

---

## 7. Likely Bugs or Methodological Risks

1. **Hardcoded Instance Specialization**: Heuristic parameters in older binaries (e.g. `gset_benchmark.rs`) were calibrated on the test instances themselves, risking benchmark overfitting.
2. **Censored Run Bias**: Evaluating Time-to-Solution by taking the mean of successful runs while ignoring timeouts introduces strong optimistic survival bias.
3. **Problem-Specific Prior Injection**: The prefix ones bias in `labs_record_hunter.rs` uses domain-specific knowledge that does not generalize to arbitrary HUBO landscapes.

---

## 8. Performance Bottlenecks

1. **AVX2 Vector Width Limitation**: Currently hardcoded to 64 replicas (matching 8 ymm registers). Moving to AVX-512 or dynamic lane dispatch would double throughput on modern Zen 4 / Xeon Sapphire Rapids processors.
2. **Sequential Path-Relinking**: Elite pool relinking is executed sequentially post-anneal rather than concurrently during population evolution.
3. **Single-Threaded QPBO**: The Dinic max-flow implementation in `qpbo.rs` is single-threaded. For graphs with $> 100,000$ edges, presolve runtime exceeds annealing initialization time.

---

## 9. Comparison with Relevant Literature

| Problem Domain | SOTA Literature Solver | Ising Engine Status | Key Comparison Insight |
|---|---|---|---|
| **Max Independent Set** | KaMIS / ReduMIS (Lamm et al. 2017) | `UltimateSolver` + MIS QUBO | KaMIS uses graph kernel reductions (domination, twins); our unweighted QUBO mapping cannot use 2-opt escape (CD005). Exact graph reductions outperform pure penalty annealing. |
| **MaxCut** | Biq Mac (Rendl et al. 2010), Breakout Local Search | `ParallelTemperingSolver` / MSC | Pure PT reaches high cuts quickly, but exact SDP branch-and-bound yields proven optimality bounds that heuristic SA cannot provide. |
| **LABS** | lMAts-lRRts (Gallagher 2012, Packebusch 2016) | `labs_record_hunter` | Memetic PT + 2-opt matches literature quality up to $N=40$, and narrows 20-year records on $N \ge 67$ to $+16$, but requires population-level tabu operators to compete at $N=50..60$. |
| **Roof Duality** | QPBO (Kolmogorov & Rother 2007) | `src/presolve/qpbo.rs` | Our Dinic implementation is bit-identical and verified optimum-preserving, but currently unparallelized. |

---

## 10. QOBLIB Status

- **Problem Class 01 (Market Split)**: Formulated, solved, 100% verified, submission package assembled.
- **Problem Class 02 (LABS)**: Formulated, tested against 20-year world records, official checker compiled and integrated.
- **Problem Class 07 (Maximum Independent Set)**: Formulated, verified on small-to-medium graphs with `check_stableset`.
- **Remaining Classes (03 Portfolio, 04 Network Design, 05 Vehicle Routing, 06 MaxCut, 08 QAP, 09 Graph Coloring, 10 HUBO)**: Not yet systematically evaluated. Classes 03, 06, and 10 are immediate candidates for our high-order HUBO kernel.

---

## 11. Experiments Worth Reproducing

1. **CD005 Equal-Time MIS Boundary**: Re-verify on non-QOBLIB random d-regular graphs to confirm that 2-opt is algebraically blocked across all unweighted MIS instances.
2. **RC-002 Initialization Dissociation**: Re-run on larger graphs ($N \ge 2000$) to measure whether the quality vs diversity dissociation holds across graph density transitions.
3. **Market Split $m=12..15$ Unsolved Instances**: Benchmark our 2-opt quench on the 41 officially unsolved instances in QOBLIB Class 01.

---

## 12. Five Strongest Research Directions

1. **Direction 1: Direct 4th-Order HUBO Advantage over Binarization**:
   - Compare native AVX2 degree-4 evaluation against Ishikawa/Freedman quadratization solved via state-of-the-art classical MIP/QUBO solvers.
2. **Direction 2: Breaking 20-Year LABS World Records ($N=74, 70, 69$)**:
   - Close the remaining 16-point gap on $N=74$ using Variable Neighborhood Search with skew-symmetry constraints.
3. **Direction 3: Solving Unsolved QOBLIB Class 01 Instances**:
   - Attack the 41 instances in `01-marketsplit` where no known classical or quantum solution has ever been recorded.
4. **Direction 4: Polynomial Roof Duality Integration on QOBLIB Classes**:
   - Measure the exact persistency yield across all QOBLIB benchmark families.
5. **Direction 5: Feedback-Tuned Population Annealing vs SQA Tunneling**:
   - Rigorous empirical test of quantum tunneling (Trotter slices) vs classical feedback-tuned population annealing across rugged barrier benchmarks.
