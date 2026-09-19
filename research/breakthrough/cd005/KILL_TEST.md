---
id: cd005-kill-test
kind: research-protocol
status: active
authority_scope: falsification-protocol
created: 2026-09-19
immutable: false
---

# CD005 Kill-Test Protocol: Edge-Restricted 2-Opt Escapes in Frustrated Spin Glasses

## 1. Objective
1. Mathematically verify that at 1-opt minima, no non-edge pair ever has $\Delta_{uv} < 0$ (Theorem 1).
2. Measure the escape rate from 1-opt local minima into lower-energy basins via edge 2-flips.
3. Quantify final energy improvement and speedup over exhaustive $O(N^2)$ pair scanning.

## 2. Benchmark Suite
- **Instance Sizes $N$:** $\{16, 36, 64, 100\}$.
- **Topologies:**
  1. `2D_torus`: Edwards-Anderson spin glass ($L \times L$, bimodal couplings $J_{ij} \in \{-1, +1\}$).
  2. `sparse_ER`: Erdős-Rényi $G(N, p)$ with average degree $\langle d \rangle \in [4, 8]$.

## 3. Evaluators
1. **Pure 1-Opt Descent (Baseline):** Greedily flips single spins until all $\Delta_i \ge 0$.
2. **Edge-Restricted 2-Opt Interleaved (Candidate):**
   - When 1-opt hits local minimum, scan $E$ for $\Delta_{uv} = \Delta_u + \Delta_v - 4 J_{uv} s_u s_v < 0$.
   - Execute the most improving pair move and resume 1-opt until both 1-opt and 2-opt have zero improving moves.
3. **Exhaustive 2-Opt Verification Oracle:** Scans all $\binom{N}{2}$ pairs to verify Theorem 1.

## 4. Primary Metrics
- `theorem_violations`: Any non-edge with $\Delta_{uv} < 0$ (must be strictly 0).
- `escape_rate`: Fraction of 1-opt minima that admit an improving edge 2-flip.
- `mean_barrier`: Average intermediate barrier $B_{uv} = \min(\Delta_u, \Delta_v)$ tunneled.
- `energy_gain`: $E_{\text{1-opt}} - E_{\text{2-opt}}$ (positive means lower energy found).
- `scan_ratio`: $|E| / \binom{N}{2}$ (theoretical speedup factor).

## 5. Falsification Rule
- **Kill Condition (NO-GO):** If escape rate is $< 5\%$ (1-opt minima are almost never 2-opt escapable), OR if final energy gain is $\le 0.1\%$ across all benchmarks, the 2-opt neighborhood is practically inert.
- **Success Condition (PASS):** Theorem 1 verified with 0 violations, escape rate $\ge 30\%$, and consistent energy reduction on frustrated spin glasses.
