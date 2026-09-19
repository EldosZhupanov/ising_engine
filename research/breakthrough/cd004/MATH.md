---
id: cd004-math
kind: research-protocol
status: active
authority_scope: mathematical-derivation
created: 2026-09-19
immutable: false
---

# CD004 Mathematical Derivation: Native Soft-Conflict Learning in Ising Optimization

## 1. Problem Formulation

Let $G = (V, E)$ be an undirected graph with $|V| = N$ spins $s_i \in \{-1, +1\}$.
The Ising Hamiltonian is:
$$E(s) = - \sum_{(u, v) \in E} J_{uv} s_u s_v - \sum_{v \in V} h_v s_v$$

Let an incumbent solution $s_{\text{inc}} \in \{-1, +1\}^N$ be known with upper-bound energy:
$$E^* = E(s_{\text{inc}})$$

A search node is defined by a partial assignment $(F, \sigma_F)$, where $F \subseteq V$ is the set of fixed variables and $\sigma_F \in \{-1, +1\}^{|F|}$.
The free variables are $R = V \setminus F$.

---

## 2. Fast Certified Dual Lower Bound $\text{LB}(F, \sigma_F)$

The conditional energy of any completion $s_R \in \{-1, +1\}^{|R|}$ is:
$$E(s_F = \sigma_F, s_R) = E_F(\sigma_F) - \sum_{(u, v) \in E(R)} J_{uv} s_u s_v - \sum_{u \in R} h_u^{\text{eff}}(\sigma_F) s_u$$
where:
$$E_F(\sigma_F) = - \sum_{(u, v) \in E(F)} J_{uv} \sigma_u \sigma_v$$
$$h_u^{\text{eff}}(\sigma_F) = h_u + \sum_{v \in F: (u, v) \in E} J_{uv} \sigma_v$$

### Fast Certified Bound Construction
For any completion $s_R$:
$$- s_u s_v \ge -1 \implies - J_{uv} s_u s_v \ge - |J_{uv}|$$
$$- h_u^{\text{eff}} s_u \ge - |h_u^{\text{eff}}|$$

Therefore, a provable lower bound is:
$$\text{LB}_0(F, \sigma_F) = E_F(\sigma_F) - \sum_{(u, v) \in E(R)} |J_{uv}| - \sum_{u \in R} |h_u^{\text{eff}}(\sigma_F)|$$

### Property (Monotonicity & Invariance)
$$\forall s_R \in \{-1, +1\}^{|R|}, \quad E(\sigma_F, s_R) \ge \text{LB}_0(F, \sigma_F)$$
If $\text{LB}_0(F, \sigma_F) > E^*$, then no completion in the subcube $\{s : s_F = \sigma_F\}$ can achieve an energy as low as the incumbent $E^*$.

---

## 3. Minimal Soft-Conflict Core Extraction (Deletion Filter)

When a soft conflict occurs ($\text{LB}_0(F, \sigma_F) > E^*$), the set $F$ may contain irrelevant decisions made high in the tree.
We seek a minimal subset $F^* \subseteq F$ that still satisfies the conflict condition:
$$\text{LB}_0(F^*, \sigma_{F^*}) > E^*$$

### Algorithm: Greedy Deletion Filter
```python
def extract_minimal_core(F, sigma_F, E_star):
    core = list(F)
    for v in list(F):
        candidate = [u for u in core if u != v]
        if compute_LB(candidate, sigma_F) > E_star:
            core = candidate  # v was unnecessary, eliminate it
    return core
```
*Complexity:* At most $|F|$ evaluations of the $O(E + V)$ lower bound, which takes $O(|F| \cdot (|E| + |V|))$ time.

---

## 4. Non-Chronological Backjumping & Nogood Pruning

In chronological Branch-and-Bound:
- If a node at depth $d = |F|$ is pruned, the solver backtracks to depth $d - 1$ (the immediate parent).
- It explores the sibling branch, often repeating the exact same failed sub-structure!

In Native Conflict-Driven Optimization:
- The minimal core $F^*$ has decision levels $\{L(v) : v \in F^*\}$.
- Let $d_{\max} = \max_{v \in F^*} L(v)$ and $d_{\text{jump}} = \max_{v \in F^* \setminus \{v_{\max}\}} L(v)$ (the second-highest decision level).
- **Theorem:** All search subtrees between $d_{\text{jump}}$ and $d_{\max}$ that do not change $F^*$ are provably sub-optimal.
- The solver **backjumps directly to $d_{\text{jump}}$**, skipping $2^{d_{\max} - d_{\text{jump}}}$ branches without evaluating them!
