---
id: cd003-kill-test
kind: research-protocol
status: active
authority_scope: falsification-protocol
created: 2026-09-19
immutable: false
---

# CD003 Kill-Test Protocol: Precision-Rank Bounded Response Witness

## 1. Objective
Experimentally test and verify the Precision-Rank Bounded Response bound across an exact combinatorial grid of boundary sizes $b$, integer weight limits $K$, and interface ranks $r$.

## 2. Experimental Grid
- **Boundary size $b$:** $\{1, 2, 3, 4, 5, 6, 8, 10\}$.
- **Precision bound $K$:** $\{1, 2, 3, 4, 8, 16, 32\}$.
- **Interface rank $r$:** $\{1, 2\}$.
- **Coefficient families:**
  1. `unit_positive`: $v_i = 1$ for all $i$ ($K=1$, rank 1). Expected image size: $b + 1$.
  2. `ternary`: $v_i \in \{-1, 0, +1\}$ randomly chosen ($K=1$, rank 1). Bound: $2b + 1$.
  3. `bounded_integer`: $v_i \in \{-K, \dots, K\}$ ($K \in \{2, 3, 4\}$). Bound: $2bK + 1$.
  4. `binary_powers`: $v_i = 2^i$ ($K = 2^{b-1}$, H11 baseline). Expected: $2^b$.
  5. `rank_2_bipartite`: $r=2$, $V \in \{-1, 0, 1\}^{b \times 2}$. Bound: $(2b+1)^2$.

## 3. Metrics Evaluated
1. **$N_{\text{context}}$:** Total number of raw boundary configurations ($2^b$).
2. **$N_{\boldsymbol{\theta}}$:** Number of unique projection values $|\{V^T z : z \in \{0, 1\}^b\}|$.
3. **$N_{\text{resp}}$:** Number of unique conditional ground-state internal assignments $\{x^*(z) : z \in \{0, 1\}^b\}$.
4. **$B_{\text{theoretical}}$:** The theoretical upper bound $(2bK + 1)^r$.
5. **Violation check:** $\text{Is } N_{\text{resp}} \le N_{\boldsymbol{\theta}} \le B_{\text{theoretical}}$?

## 4. Falsification Rule
- **Kill Condition 1 (FALSIFIED):** If ANY instance produces $N_{\text{resp}} > N_{\boldsymbol{\theta}}$ or $N_{\text{resp}} > B_{\text{theoretical}}$, the mathematical proof contains a fatal flaw.
- **Kill Condition 2 (TRIVIAL):** If all instances with $K=O(1)$ collapse to a constant number of states ($N_{\text{resp}} \le 2$), meaning the interface cannot transfer meaningful context.
- **Success Condition (PASS):** $N_{\text{resp}}$ scales as $\Theta(b \cdot K)$ for rank 1 and $\Theta(b^2)$ for rank 2, confirming that low precision strictly prevents exponential blowup while preserving non-trivial information transfer.
