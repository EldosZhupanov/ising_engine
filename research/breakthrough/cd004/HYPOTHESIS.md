---
id: cd004-hypothesis
kind: research-protocol
status: active
authority_scope: candidate-hypothesis
created: 2026-09-19
immutable: false
---

# CD004 Hypothesis: Native Soft-Conflict Learning and Non-Chronological Backjumping in Ising Optimization

## 1. Context & Motivation

Modern automated theorem provers (CDCL SAT solvers) routinely solve Boolean instances with millions of variables because they learn **Conflict Clauses** from dead ends, enabling exponential pruning ($2^{N-k}$) and non-chronological backjumping.

In contrast, discrete energy optimizers (Ising / QUBO / MaxCut) either:
1. Use stochastic local search (Simulated Annealing, Parallel Tempering, Tabu), which blindly wanders the landscape without ever learning which subcubes are provably sub-optimal; OR
2. Use classical Branch-and-Bound (BnB), which branches chronologically and throws away all learned failure structure upon backtracking; OR
3. Translate to MaxSAT CNF, which explodes in clause size and variable count when edge couplings $J_{ij}$ are non-unit reals.

## 2. The Tested Hypothesis (H14)

**Hypothesis H14 (Native Soft-Conflict Pruning Advantage):**
Let $s^* \in \{-1, +1\}^N$ be an incumbent solution with known energy $E^* = E(s^*)$.
When a partial assignment $s_F = \sigma_F$ yields a certified dual lower bound strictly exceeding the incumbent:
$$\text{LB}(s_F) > E^*$$
there exists an efficient polynomial-time deletion filter that extracts a **Minimal Soft-Conflict Core** $F^* \subseteq F$ such that:
$$\text{LB}(s_{F^*}) > E^*$$
Recording $(F^*, \sigma_{F^*})$ as a native soft no-good and non-chronologically backjumping to the second-highest decision level in $F^*$:
1. Provably eliminates all $2^{N - |F^*|}$ subcube configurations (where $|F^*| \ll |F|$);
2. Strictly reduces the number of branch evaluations compared to chronological Branch-and-Bound by a factor exponential in $(|F| - |F^*|)$.

## 3. Decisive Kill Condition

The hypothesis is **FALSIFIED (NO-GO)** if:
1. The extracted conflict cores are degenerate ($|F^*| \approx |F|$), yielding zero non-chronological backjumping advantage over standard chronological Branch-and-Bound; OR
2. The overhead of computing dual bounds and extracting conflict cores exceeds the saved combinatorial tree search on frustrated spin glass instances; OR
3. Soft conflict learning is already identically implemented in standard non-CNF branch-and-bound literature with verified negative outcomes on frustrated zero-field graphs.
