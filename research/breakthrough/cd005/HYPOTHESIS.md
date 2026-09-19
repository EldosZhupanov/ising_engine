---
id: cd005-hypothesis
kind: research-protocol
status: active
authority_scope: candidate-hypothesis
created: 2026-09-19
immutable: false
---

# CD005 Hypothesis: Edge-Restricted 2-Opt Escape and Cross-Curvature Barrier Auditing in Frustrated Spin Glasses

## 1. Context & Motivation

In discrete energy optimization (Ising / QUBO), local search algorithms (Glauber dynamics, Metropolis, 1-opt greedy descent) terminate at 1-opt local minima where:
$$\Delta_i = E(s \oplus e_i) - E(s) \ge 0 \quad \forall i \in V$$
In frustrated landscapes (spin glasses), these 1-opt local minima are separated from the global optimum by coordinated multi-spin flips.
Exhaustive evaluation of all 2-spin flip combinations requires checking $\binom{N}{2} = O(N^2)$ pairs, which becomes a computational bottleneck on large graphs ($N \ge 1000$).

## 2. The Tested Hypothesis (H15)

**Hypothesis H15 (Edge-Completeness of 2-Opt Escapes):**
1. **Mathematical Completeness:** At any 1-opt local minimum ($\Delta_i \ge 0$), the 2-spin flip energy delta between variables $u$ and $v$ satisfies:
   $$\Delta_{uv} = \Delta_u + \Delta_v + 4 J_{uv} s_u s_v$$
   If $(u, v) \notin E$ (meaning $J_{uv} = 0$), then $\Delta_{uv} = \Delta_u + \Delta_v \ge 0$.
   Therefore, **an improving 2-spin flip ($\Delta_{uv} < 0$) can exist IF AND ONLY IF $(u, v) \in E$ and $J_{uv} s_u s_v < 0$ (the edge is currently frustrated)**.
2. **Computational Speedup:** Scanning strictly the frustrated graph edges $E_{\text{frust}} = \{ (u, v) \in E : J_{uv} s_u s_v < 0 \}$ is **provably complete** for finding all improving 2-flip escapes, reducing complexity from $O(N^2)$ to $O(|E_{\text{frust}}|) \le O(|E|)$.
3. **Escaping Local Traps:** Incorporating edge-restricted 2-opt escape into local search significantly increases the probability of reaching the true ground state and lowers final residual energy on frustrated spin glass benchmarks.

## 3. Decisive Kill Condition

The hypothesis is **FALSIFIED (NO-GO)** if:
1. There exist any non-edge pairs $(u, v) \notin E$ that yield $\Delta_{uv} < 0$ at a 1-opt minimum (mathematical contradiction); OR
2. In frustrated spin glasses, 1-opt local minima have zero improving edge 2-flips (i.e. all barriers require $k \ge 3$ flips), rendering the 2-opt neighborhood completely inert; OR
3. The computational time to evaluate $E_{\text{frust}}$ exceeds the time to simply perform additional random 1-opt restarts to equivalent energy.
