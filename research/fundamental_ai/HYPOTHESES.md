# Mathematical Hypotheses: Higher-Order Tensor Energy Dynamics

**Document ID:** `research/fundamental_ai/HYPOTHESES.md`  
**Status:** Preregistered & Binding  
**Author:** Autonomous Research Scientist (`ising_engine`)  

---

## 1. Primary Research Question

Can higher-order tensor energy interactions ($p \ge 3$) serve as an efficient, robust, and scalable computational primitive for neural memory and reasoning, or is any perceived advantage merely a trivial byproduct of an inflated parameter budget?

---

## 2. Formal Hypotheses

### Hypothesis H0 (The Universal Null Hypothesis)
- **Statement:** At an equal parameter budget $K$, higher-order models (sparse hyperedges or low-rank CP factorizations) exhibit memory capacity, basin radius, and noise recovery that are less than or equal to optimal pairwise models. Any empirical advantage of dense tensors is entirely explained by the $O(N^p)$ parameter count.
- **Falsification Criterion:** Falsified if a 3-body or 4-body model with parameter count $K \le \binom{N}{2}$ achieves statistically significant higher recovery rate ($p < 0.01$, Wilcoxon signed-rank test across $\ge 20$ seeds) at noise levels $\ge 20\%$ across sizes $N \in \{128, 256, 512\}$.

### Hypothesis H1 (Sparse Hyperedge Advantage)
- **Mechanism:** Random pairwise connections induce dense pairwise crosstalk $\frac{1}{N} \sum_{\mu \ne \nu} \xi_i^\mu \xi_j^\mu \xi_j^\nu$. In contrast, sparse 3-body hyperedges $(i, j, k)$ sample triplets of spins. When hyperedges are selected along maximal correlation cliques, the signal scales faster than the uncorrelated crosstalk background.
- **Prediction:** A sparse 3-body model with $M = \binom{N}{2}$ hyperedges will exhibit larger basin radii (successful convergence from $> 30\%$ bit noise) than a dense pairwise Hopfield model with the same number of parameters.
- **Alternative Explanation / Falsification:** If sparse sampling leaves isolated spins or disjoint hypergraph components, convergence will degrade, causing H1 to be decisively falsified.

### Hypothesis H2 (Low-Rank CP Overlap Sharpening)
- **Mechanism:** In a symmetric rank-$R$ CP tensor $T_{ijk} = \sum_{r=1}^R \lambda_r a_{ir} a_{jr} a_{kr}$, the local effective field is:
  $$h_i^{\text{eff}} = \sum_{r=1}^R \lambda_r a_{ir} (a_r^T s)^2$$
  The quadratic factor $(a_r^T s)^2$ acts as a non-linear sharpening operator that exponentially suppresses background patterns whose overlap $|a_r^T s| \ll N$, while magnifying the true attractor with overlap $|a_r^T s| \approx N$.
- **Prediction:** The CP-factorized model achieves superior capacity-per-parameter ($P / (NR) \gg 0.138$) and near-zero crosstalk noise compared to pairwise Hebbian memory.
- **Falsification Criterion:** If spurious local minima or non-convex energy plateaus trap the dynamics when $R > 0.14 N$, H2 is falsified.

### Hypothesis H3 (Native Higher-Order vs. Quadratization Barrier)
- **Mechanism:** Rosenberg quadratization of 3-body and 4-body terms introduces auxiliary slack variables $w$ and high penalty multipliers $M \gg 1$. This creates deep non-physical local energy minima and ill-conditioned Hessian spectra. Native higher-order dynamics operates directly on the uncorrupted physical energy surface.
- **Prediction:** Native 3-body and 4-body greedy and Glauber dynamics will solve 3-SAT and parity constraints in fewer flips and with higher probability of finding valid satisfying assignments than quadratized QUBO solvers.
- **Falsification Criterion:** If native higher-order relaxation encounters high polynomial barriers and fails to find ground states with equal or better probability than QUBO, H3 is falsified.

### Hypothesis H4 (Compositional Relational Binding)
- **Mechanism:** A 3-body tensor can represent a relation $(A, R, B)$ as a single cohesive energy well $E(A, R, B) = -1$. Pairwise graphs require auxiliary mediator nodes or decompose $(A, R, B)$ into pairwise cliques $(A-R, R-B, A-B)$, which produce spurious binding (ghost combinations) when multiple relations share entities.
- **Prediction:** In multi-relation completion tasks ($A + R + ? \to B$), the 3-body energy network will avoid crosstalk errors on held-out combinations where pairwise networks fail.
- **Falsification Criterion:** If 3-body networks suffer from similar ghost attractors under Hebbian tensor storage, H4 is falsified.

---

## 3. Decision Matrix & Verdict Ledger

Every hypothesis will be evaluated against experimental data and classified into one of four immutable verdicts:
- **`SUPPORTED`**: Hypothesis prediction confirmed with $p < 0.01$ under rigorous parameter normalization.
- **`WEAK SIGNAL`**: Measurable trend observed, but effect size is below practical utility ($< 5\%$ gain) or inconsistent across $N$.
- **`INCONCLUSIVE`**: Variance across random seeds dominates, or compute limits prevent decisive testing.
- **`FALSIFIED`**: Baseline matches or outperforms candidate at equal parameter budget, or predicted mechanism fails.
