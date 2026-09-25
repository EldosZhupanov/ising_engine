# Scientific Knowledge Base — Ising Engine

Authority: Empirical & Theoretical Verified Facts  
Last Updated: 2026-09-25

---

## 1. Thermodynamic & Algorithmic Principles

### 1.1 Quality vs. Diversity Dissociation (RC-002)
- **Verified Finding**: Initialization quality (e.g. greedy local search starts) is rapidly erased at thermal depth ($T \ge 0.25$), whereas initial diversity survives across temperatures up to $T = 4.0$.
- **Mechanism**: Greedy descent algorithms without stochastic perturbation collapse all identical initial replicas into the same trajectory, eliminating diversity. Replicas must be initialized with independent stochastic noise or distinct topological perturbations.

### 1.2 Backjump & Exact BnB Limits (CD004-R)
- **Verified Finding**: A non-chronological backjumping mechanism without certified core validity produces unsound branch cuts (counterexample documented in `research/breakthrough/cd004_recheck/`).
- **Performance**: A sound, certified-core BnB implementation requires node-validation overhead that yielded 0/120 speedup over standard chronological branch-and-bound (median wall ratio 4.876x slower).

### 1.3 2-Opt Invariant on Unweighted MIS QUBO (CD005)
- **Verified Theorem**: For the standard unweighted Maximum Independent Set QUBO formulation with quadratic penalty $P = 2$:
  $$H(x) = -\sum_{i \in V} x_i + 2 \sum_{(u,v) \in E} x_u x_v$$
  any 1-opt local minimum (where all single-flip $\Delta E \ge 0$) algebraically precludes the existence of a strictly improving 2-flip move ($\Delta E_{p,q} < 0$).
- **Implication**: Multi-neighborhood 2-opt escape on this specific formulation cannot improve upon 1-opt descent. 2-opt search is only effective on problems with non-degenerate quadratic interactions (e.g. LABS, spin glasses, weighted MaxCut).

### 1.4 Roof Duality & Weak Persistency (QPBO)
- **Verified Theorem (Hammer-Hansen-Simeone 1984, Boros-Hammer 2002)**: Solving min-cut on the implication posiform network $N_f$ yields polynomial-time weak persistency. If a variable $x_i$ and its complement $\bar{x}_i$ lie in different strongly connected components of the residual graph, the optimal value of $x_i$ in the unconstrained global problem is guaranteed to equal its residual cut assignment.
- **Engine Status**: Fully implemented via Dinic's algorithm (`src/presolve/qpbo.rs`) and verified 100% optimum-preserving across exhaustive tests.

---

## 2. Problem Landscape Properties

### 2.1 Low Autocorrelation Binary Sequences (LABS / Bernasconi Model)
- **Energy Landscape**: Extreme ruggedness with golf-course valleys. Standard Simulated Annealing gets trapped in exponentially many 1-opt local traps.
- **Memetic Search Dynamics**: Combining multi-replica parallel tempering with $O(N^2)$ deterministic 2-opt quenching and tabu search compresses the gap to 22-year-old world records:
  - $N=74$: $E=357$ (record 341, gap +16);
  - $N=70$: $E=319$ (record 295, gap +24);
  - $N=67$: $E=289$ (record 241, gap +48).
- **All solutions verified by official ZIB checker `check_labs`**.

### 2.2 Market Split (Cornuéjols & Dawande 1998)
- Exact quadratic equality formulation $\min \sum_k (\sum_j A_{kj} x_j - b_k)^2$ creates sharp quadratic valleys.
- Classical 2-opt quench + dynamic tabu search achieves 0 constraint violations in 0.70s–2.02s on benchmark instances (`ms_03_*`), beating GPU simulated annealing baselines (4.3–4.4s).
