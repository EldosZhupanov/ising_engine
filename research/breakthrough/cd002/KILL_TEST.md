---
id: cd002-kill-test
kind: research-protocol
status: active
authority_scope: falsification-protocol
created: 2026-09-19
immutable: false
---

# CD002 Kill-Test Protocol: Adversarial Gauge Cycle Witness

## 1. Objective
Test whether combinatorial discrete MAP optimization on frustrated permutation cycles ($S_d$) achieves an unrefuted accuracy advantage over classical Spectral Permutation Synchronization (Pachauri et al. 2013) and Loopy Message Passing.

## 2. Experimental Grid
- **Cycle lengths $K$:** $\{3, 4, 5, 6\}$.
- **Permutation dimension $d$:** $\{3, 4, 5\}$.
  - For $d=3$, $|S_3| = 6$. Full search space for $K=4$ is $6^4 = 1,296$ configurations.
  - For $d=4$, $|S_4| = 24$. Full search space for $K=3$ is $24^3 = 13,824$ configurations.
  - For $d=5$, $|S_5| = 120$. Full search space for $K=3$ is $120^3 = 1,728,000$ (or branch-and-bound / dynamic programming on cycle).
- **Corruption types:**
  1. Transposition $(0, 1)$ on single edge;
  2. Full derangement / cyclic shift on single edge;
  3. Dense multi-edge corruption ($2$ corrupted edges out of $K$).

## 3. Evaluators
1. **Raw / Identity Baseline:** Assumes $\Pi_{uv} = I$.
2. **Spectral Permutation Sync (Pachauri 2013):** Top-$d$ eigenvectors of $\mathbf{R}$ + Hungarian rounding.
3. **Exact Discrete MAP (Ising/Gauge):** Exact enumeration of discrete permutation states maximizing Gauge Hamiltonian $\sum_{(u, v)} \text{Tr}(P_u \Pi_{uv} P_v^T)$.
4. **Loopy Min-Sum BP:** Message passing on the factor graph over $S_d$.

## 4. Falsification Rule
- **Outcome A (NO-GO):** Spectral Sync achieves $\ge$ accuracy of Discrete MAP across all grid cells, OR Discrete MAP exhibits symmetric tie degeneracy (multiple ground states due to gauge frustration) that prevents deterministic error correction without prior knowledge of which edge was corrupted.
- **Outcome B (PASS / GO):** Discrete MAP decisively breaks through the spectral breakdown threshold, isolating the adversarial edge and achieving strictly higher node recovery accuracy ($> 15\%$ margin) where Spectral Sync collapses.
