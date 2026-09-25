# Next 10 Scientific Experiments — Ising Engine

Authority: Prioritized Research Backlog  
Ranking Criterion: **MAXIMUM INFORMATION GAIN** (not promotional appeal).  
Rule: Each experiment must have a frozen hypothesis, exact baseline, metric, falsification threshold, and hardware cost estimate.

---

## 1. EXP-001: Native SIMD 4th-Order HUBO vs. Ishikawa Ancilla Quadratization

- **Hypothesis**: Direct AVX2 evaluation of degree-4 binary polynomials in `FlatHuboModel` achieves lower Time-To-Solution ($\text{TTS}_{99}$) than Ishikawa/Freedman quadratization solved on the same CPU cores by standard quadratic solvers.
- **Why It Matters**: Standard combinatorial solvers force all cubic and quartic interactions into quadratic form by introducing ancilla variables and penalty terms. Proving that direct 4th-order evaluation is faster eliminates ancilla inflation and penalty barriers across all higher-order optimization.
- **Benchmark Suite**: Synthetic and physics-derived 3-XOR and 4-spin glass instances ($N \in [50, 100, 200, 500]$), degree-4 density $\rho \in [0.01, 0.1]$.
- **Baseline**: Ishikawa quadratization + `QuboModel` (Parallel Tempering) and Gurobi / Biq Mac.
- **Primary Metric**: Time-To-Solution ($\text{TTS}_{99}$) and peak memory bandwidth.
- **Estimated Computational Cost**: ~2 CPU hours (10 seeds $\times$ 20 instances $\times$ 30 s).
- **Falsification Criterion**: Direct HUBO yields $\ge 10\%$ worse median $\text{TTS}_{99}$ or fails to find ground states reached by quadratization.
- **Expected Information Gain**: **VERY HIGH**. Settles the fundamental architectural premise of `UltimateSolver` and `HuboModel`.

---

## 2. EXP-002: Roof Duality (QPBO) Persistency Yield Across All 10 QOBLIB Classes

- **Hypothesis**: Exact QPBO max-flow presolve reduces problem dimensionality by fixing $\ge 15\%$ of binary variables in at least 3 of the 10 official QOBLIB problem classes within $< 100$ ms.
- **Why It Matters**: Polynomial-time reductions are mathematically exact and risk-free. Mapping exactly which QOBLIB classes contain submodular structures establishes where presolve should be permanently activated.
- **Benchmark Suite**: All public instances from QOBLIB classes 01 through 10.
- **Baseline**: Raw, unreduced instances solved without QPBO.
- **Primary Metric**: Persistency fraction ($\frac{N_{\text{fixed}}}{N_{\text{total}}}$), presolve wall-clock runtime (ms), and residual instance solution time.
- **Estimated Computational Cost**: ~15 CPU minutes.
- **Falsification Criterion**: QPBO fixes $< 5\%$ of variables across all tested QOBLIB classes, or presolve time exceeds the time saved during search.
- **Expected Information Gain**: **HIGH**. Establishes the exact domain boundaries of roof duality on standardized benchmarks.

---

## 3. EXP-003: Simulated Quantum Annealing (Trotter Slices) vs. Classical Parallel Tempering at Matched CPU Budget

- **Hypothesis**: Multi-slice Path-Integral Monte Carlo (SQA) achieves higher ground-state hit rates than classical Parallel Tempering on energy landscapes featuring tall, thin potential barriers (quantum tunneling advantage).
- **Why It Matters**: The engine implements Trotter slices (`num_slices > 1`), but classical SQA requires simulating $M$ coupled replicas per temperature. We must determine if quantum tunneling simulation justifies the $M$-fold computational cost on finite graphs.
- **Benchmark Suite**: Deceptive spin-glass bimodal barriers and ferromagnetically coupled clusters ($N \in [64, 128, 256]$).
- **Baseline**: Classical Parallel Tempering (single slice, $M=1$) allocated equal total spin-flips and equal wall-clock time.
- **Primary Metric**: Ground-state hit probability $P_{\text{succ}}$ at matched CPU time.
- **Estimated Computational Cost**: ~3 CPU hours (8 seeds $\times$ 15 instances $\times$ 60 s).
- **Falsification Criterion**: Classical PT achieves equal or higher $P_{\text{succ}}$ than SQA across all barrier heights at equal wall-clock time.
- **Expected Information Gain**: **VERY HIGH**. Eliminates ungrounded quantum advantage claims and rigorously defines SQA applicability.

---

## 4. EXP-004: Targeted VNS Strike on Unbroken LABS World Records ($N=74, 70, 69$)

- **Hypothesis**: Memetic Parallel Tempering with Variable Neighborhood Search (2-opt quench + 3..5 spin parity-preserving perturbations) breaks at least one official 22-year-old world record ($E < E_{\text{BKV}}$) within 12 hours of compute.
- **Why It Matters**: A verified world record on a 22-year-old open problem (Knauer 2004) is undeniable external proof of algorithmic effectiveness. Current results are only +16 away on $N=74$ ($E=357$ vs record 341).
- **Benchmark Suite**: LABS lengths $N=74$ (target $< 341$), $N=70$ (target $< 295$), $N=69$ (target $< 274$).
- **Baseline**: Current best checkpoints ($E=357, 319, 310$) and published Knauer records.
- **Primary Metric**: Exact verified energy $E = \sum C_d^2$ confirmed by official ZIB `check_labs`.
- **Estimated Computational Cost**: ~12 CPU hours on 4 cores.
- **Falsification Criterion**: Energy fails to improve below current checkpoints ($357, 319, 310$).
- **Expected Information Gain**: **HIGH**. Proves whether multi-core memetic VNS can beat supercomputer cluster records from 2004.

---

## 5. EXP-005: Attack on Officially Unsolved QOBLIB Class 01 Market Split Instances ($m=12..15$)

- **Hypothesis**: The exact quadratic formulation + 2-opt tabu quench solves at least one of the 41 officially unsolved QOBLIB Market Split instances to exact equality (0 violations).
- **Why It Matters**: QOBLIB explicitly lists 41 instances with "no known solution". Finding a verified solution for any of them is an immediate world-first result for the QOBLIB leaderboard.
- **Benchmark Suite**: Instances `ms_12_100_003` through `ms_15_200_003` from QOBLIB Class 01.
- **Baseline**: Published QOBLIB status (UNSOLVED / no known feasible state).
- **Primary Metric**: Feasibility (0 constraint violations) verified by `check_marketsplit`.
- **Estimated Computational Cost**: ~4 CPU hours.
- **Falsification Criterion**: 0 feasible solutions found across all 41 instances within 10 minutes per instance.
- **Expected Information Gain**: **VERY HIGH**. Tests whether our exact equality formulation scales beyond $m=3$.

---

## 6. EXP-006: Feedback-Optimized vs. Geometric Ladder Tuning in Spin-Glass Ground-State Discovery

- **Hypothesis**: Katzgraber et al.'s round-trip feedback ladder achieves higher ground-state hit rates than a standard geometric temperature ladder on dense Sherrington-Kirkpatrick spin glasses.
- **Why It Matters**: Temperature schedule tuning is often treated as a secondary heuristic. Rigorously testing whether feedback optimization produces statistically significant gains on dense spin glasses clarifies the value of the pre-run tuning phase.
- **Benchmark Suite**: 30 SK disorder realizations at $N=128, 256$.
- **Baseline**: Geometric temperature ladder ($T_{\text{min}} = 0.1, T_{\text{max}} = 3.0$).
- **Primary Metric**: Paired Wilcoxon signed-rank test on best-found energy and round-trip flow rate.
- **Estimated Computational Cost**: ~1 CPU hour.
- **Falsification Criterion**: Feedback-optimized ladder yields $p \ge 0.05$ or fails to improve median energy over geometric ladder.
- **Expected Information Gain**: **MEDIUM-HIGH**. Validates `UltimateSolver::FeedbackOptimized` against simpler baselines.

---

## 7. EXP-007: Generalization of CD005: 2-Opt Efficacy on Weighted vs. Unweighted Graph QUBOs

- **Hypothesis**: While 2-opt improvements are algebraically precluded on unweighted MIS QUBO ($P=2$, CD005), 2-opt search provides statistically significant energy gains on weighted MaxCut and weighted MIS where coupling coefficients $J_{ij}$ vary continuously.
- **Why It Matters**: CD005 showed a no-go on unweighted MIS, but the exact boundary where 2-opt becomes viable across graph problems must be mapped.
- **Benchmark Suite**: Random Erdős-Rényi graphs with uniform edge weights $w \in [-10, 10]$ vs unweighted graphs.
- **Baseline**: 1-opt local descent.
- **Primary Metric**: Median percentage energy reduction attributable to 2-opt moves beyond 1-opt local minima.
- **Estimated Computational Cost**: ~45 CPU minutes.
- **Falsification Criterion**: 2-opt yields 0 improvements on weighted graphs, showing a broader degeneracy than currently proven.
- **Expected Information Gain**: **MEDIUM-HIGH**. Restores theoretical clarity to local search operators.

---

## 8. EXP-008: Population Annealing Resampling (ESS) vs. Independent Multi-Start Parallel Tempering

- **Hypothesis**: Population Annealing with ESS-triggered resampling achieves higher ground-state discovery rates than running $K$ independent Parallel Tempering trajectories under identical total sweep and CPU time budgets.
- **Why It Matters**: Population Annealing introduces complex resampling and communication overhead. We must test whether collective population reweighting outperforms trivial embarassingly parallel independent PT runs.
- **Benchmark Suite**: Edwards-Anderson 3D spin glasses ($L=6, 8$) and QOBLIB MaxCut instances.
- **Baseline**: $K$ independent PT runs across identical seeds.
- **Primary Metric**: Success probability $P_{\text{succ}}$ and energy distribution variance.
- **Estimated Computational Cost**: ~2 CPU hours.
- **Falsification Criterion**: Independent PT runs match or exceed PA-PT success rate at equal core-hours.
- **Expected Information Gain**: **HIGH**. Answers whether population-level interaction is necessary for hard spin glasses.

---

## 9. EXP-009: Isoenergetic Cluster Moves (ICM) Acceleration Boundary Across Graph Densities

- **Hypothesis**: Isoenergetic Cluster Moves (ICM) significantly accelerate spin-glass relaxation only below a critical edge density $\rho_c \approx \frac{2d}{N}$; on dense graphs ($\rho > 0.2$), cluster percolation encompasses the entire graph, reducing ICM to trivial spin inversions.
- **Why It Matters**: ICM is often cited as a cluster acceleration panacea. Identifying the exact percolation threshold where ICM fails prevents running expensive cluster decomposition on dense topologies.
- **Benchmark Suite**: Random graphs with densities $\rho \in [0.01, 0.05, 0.1, 0.2, 0.5]$ at $N=100$.
- **Baseline**: Parallel Tempering without ICM.
- **Primary Metric**: Average cluster size as fraction of $N$ and energy decorrelation time.
- **Estimated Computational Cost**: ~1 CPU hour.
- **Falsification Criterion**: ICM accelerates dense graphs without giant cluster percolation.
- **Expected Information Gain**: **MEDIUM**. Establishes the topological gating rules for ICM activation.

---

## 10. EXP-010: Path-Relinking vs. Simple Local Search on Elite Populations

- **Hypothesis**: Greedy path-relinking between pairs in an elite solution pool uncovers lower-energy states than applying random perturbation + local search to the same elite pool.
- **Why It Matters**: Path-relinking assumes that the trajectory between two high-quality local minima contains valleys with shared features. We must test whether this structured recombination beats simple multi-start perturbation.
- **Benchmark Suite**: G-Set MaxCut ($G22, G35, G48$) and QOBLIB MIS.
- **Baseline**: Randomized 2-flip kick + 1-opt local search on the same elite pool.
- **Primary Metric**: Paired difference in best-energy improvement.
- **Estimated Computational Cost**: ~1 CPU hour.
- **Falsification Criterion**: Randomized kicks match or exceed path-relinking improvements with $p \ge 0.05$.
- **Expected Information Gain**: **MEDIUM**. Disentangles structured memetic relinking from generic diversification.
