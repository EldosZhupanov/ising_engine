---
id: cd003-hypothesis
kind: research-protocol
status: active
authority_scope: candidate-hypothesis
created: 2026-09-19
immutable: false
---

# CD003 Hypothesis: Precision-Rank Bounded Response Programs in Discrete Optimization

## 1. Context & Problem Statement

In CD001/H11, the hypothesis that low interface rank alone guarantees polynomial compression of conditional ground states was falsified by the counterexample:
$$E(x, z) = \left( \sum_{i=0}^{b-1} 2^i (x_i - z_i) \right)^2$$
which produced $2^b$ unique conditional ground states $x^*(z) = z$ despite having cross-coupling matrix rank 1.

Crucially, this counterexample required exponential coefficient bit-growth ($2^i$), meaning the normalized minimum spectral gap shrank as $\Delta_{\min} = 1/4^{b-1}$.
In digital hardware, neuromorphic systems, and real-world combinatorial models, precision is bounded: coefficients are stored as small integers ($\{-K, \dots, K\}$ with $K = O(1)$).

## 2. The Tested Hypothesis (H13)

**Hypothesis H13 (Precision-Rank Bounded Response Theorem):**
Let an Ising/QUBO model be partitioned into internal variables $x \in \{0, 1\}^n$ and boundary variables $z \in \{0, 1\}^b$, with cross-coupling interaction $x^T J_{xz} z$ having rank $r$.
If the boundary projection vectors $V \in \mathbb{Z}^{b \times r}$ have integer coefficients bounded by $\|V\|_\infty \le K$, then:
1. The effective boundary context $\boldsymbol{\theta}(z) = V^T z \in \mathbb{Z}^r$ can take at most:
   $$N_{\boldsymbol{\theta}} \le \prod_{j=1}^r (2 b K + 1) = (2 b K + 1)^r$$
   distinct scalar vectors.
2. The number of unique conditional ground-state responses $\{x^*(z) : z \in \{0, 1\}^b\}$ across the interface is strictly bounded by:
   $$N_{\text{resp}} \le (2 b K + 1)^r$$
3. For constant rank $r = O(1)$ and constant precision $K = O(1)$, $N_{\text{resp}} = O(b^r)$ is **strictly polynomial in boundary size $b$**, provably rescuing structural compression from the exponential tree-width barrier ($2^b$).

## 3. Decisive Kill Condition

The hypothesis is **FALSIFIED (NO-GO)** if:
1. There exists any constructed or random instance with rank $r$ and coefficients bounded by $K$ where the number of unique conditional ground states exceeds $(2 b K + 1)^r$; OR
2. The precomputed response function $F_A(\boldsymbol{\theta})$ requires exponential bit complexity to evaluate, rendering the polynomial state bound computationally vacuous; OR
3. The result is already fully stated as an existing named theorem in discrete optimization, eliminating scientific priority.
