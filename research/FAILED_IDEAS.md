# Failed Ideas & Refuted Hypotheses — Ising Engine

Authority: Negative Results Register & Anti-Duplication Archive  
Rule: **NEVER DELETE AN ENTRY FROM THIS FILE.**  
Purpose: Prevent publication bias, avoid repeating dead ends, and enforce epistemic honesty.

---

## 1. Algorithmic Dead Ends & Refuted Mechanisms

### FAIL-001: Ensemble-Consensus Backbone Freezing (RC-001)
- **Idea**: Use replica ensemble consensus $c_i = |\langle s_i \rangle|$ to freeze "confident" spins or modulate local temperature $T_{\text{eff}}(i) = T \cdot \exp(\lambda c_i)$ with $\lambda < 0$.
- **Hypothesis**: Freezing agreed-upon spins speeds up ground state discovery on MaxCut.
- **Outcome**: **COMPLETELY REFUTED**. At $\lambda = -2$, the algorithm won 0 out of 132 non-tied paired instances against standard Metropolis ($p \approx 0$).
- **Mechanism**: The consensus was an artifact of shared initial states; freezing spins locked the ensemble into sub-optimal local basins.
- **Lesson**: Do not use ensemble agreement to freeze variables during thermal annealing.

### FAIL-002: Greedy Initialization for Thermal Solvers (RC-002)
- **Idea**: Warm-start parallel tempering replicas with greedy 1-opt solutions instead of random spins to provide an initial energy advantage.
- **Hypothesis**: Greedy initialization reduces time-to-solution across all temperatures.
- **Outcome**: **REFUTED**. Greedy starts collapsed ensemble diversity: from identical initial states, all replicas explored the same valley. At thermal depth ($T \ge 0.25$), initial quality was completely erased, and deep greedy starts were statistically harmful (greedy $\times 10$: $-0.138\%$, $p = 7.4 \times 10^{-4}$).
- **Lesson**: Thermal solvers require initial diversity, not initial greedy quality.

### FAIL-003: Non-Chronological Backjumping in QUBO Branch & Bound (CD004-R)
- **Idea**: Use conflict-directed backjumping in binary branch-and-bound to skip search nodes.
- **Hypothesis**: Achieves 30–50% node count reduction over chronological branch-and-bound without loss of optimality.
- **Outcome**: **REFUTED & FOUND UNSOUND (2026-09-25)**. The original prototype had a subtle soundness bug (exact counterexample discovered in recheck). The corrected, certified-core implementation visited identical nodes to chronological BnB and ran 4.876x slower due to certification overhead.
- **Lesson**: Backjumping in unconstrained optimization requires certified infeasibility cores; without them, branch cuts are either unsound or redundant.

### FAIL-004: 2-Opt Multi-Neighborhood Descent on Unweighted MIS QUBO (CD005-Q2)
- **Idea**: Apply 2-flip neighborhood search after 1-opt local descent to escape local traps in Maximum Independent Set QUBOs ($P=2$).
- **Hypothesis**: 2-opt escape increases final independent set cardinality at equal wall-clock time.
- **Outcome**: **REFUTED (NO-GO, 2026-09-25)**. Across 18 paired test cells on QOBLIB MIS graphs, 2-opt yielded 0 wins, 18 ties, 0 losses.
- **Theoretical Reason**: For the standard unweighted MIS QUBO with $P=2$, any 1-opt local minimum mathematically precludes the existence of a strictly improving 2-flip move.
- **Lesson**: 2-opt is algebraically ineffective on unweighted MIS QUBO with $P=2$. Do not deploy 2-opt on this specific formulation.

### FAIL-005: Pure Parallel Tempering on High-Dimensional LABS (LABS-Q001)
- **Idea**: Use pure Parallel Tempering + 1-opt local search to solve LABS instances ($N = 40, 50, 60$).
- **Hypothesis**: Reaches $\ge 8/10$ optimal hits at $N=50$ and $N=60$ within 10 seconds.
- **Outcome**: **FAILED QUALIFICATION (2026-09-23)**. Achieved 0/10 hits at $N=50$ and 0/10 hits at $N=60$, while specialized memetic algorithms (lMAts) achieved 5/10 hits at $N=50$.
- **Lesson**: Pure thermal fluctuations cannot cross the ultra-narrow entropy bottlenecks of the Bernasconi model. Genetic crossover and tabu recency memory are strictly required.
