---
id: cd005-prior-art
kind: research-reference
status: active
authority_scope: prior-art-audit
created: 2026-09-19
immutable: false
---

# CD005 Prior-Art Matrix & Adversarial Literature Attack

Date: 2026-09-19.
Objective: Rigorously audit edge-restricted 2-opt escapes against existing QUBO metaheuristics and $k$-flip neighborhoods.

---

## 1. Prior-Art Mapping Matrix

| Proposed Component | Closest Literature Source | Equivalence / Overlap | Novelty Verdict |
|---|---|---|---|
| Pairwise ($r$-flip) Energy Update Formulas | Alidaee, Wang & Kochenberger (2023), "r-Flip Strategies for Quadratic Unconstrained Binary Optimization"; Glover et al. (2010) | Exact algebraic identity: $\Delta_{ij} = \Delta_i + \Delta_j + q_{ij} d_i d_j$ is fully derived in Alidaee (2023). | **NOT NOVEL** (Algebraic Identity) |
| Variable Neighborhood Search (1-opt + 2-opt) | Mladenović & Hansen (1997); Hao et al. (2022), "Metaheuristics for QUBO" | Systematic escalation from 1-flip to 2-flip neighborhood when trapped. | **NOT NOVEL** (Established Heuristic Family) |
| Edge-Only Filtering for 2-Flips | Boros, Hammer & Sun (1989); Alidaee et al. (2023), Section 3 | Observed that non-connected pairs have $\Delta_{ij} = \Delta_i + \Delta_j$. | **KNOWN OBSERVATION** |

---

## 2. Adversarial Scrutiny: What is NOT Novel

1. **The formula is not new:** The algebraic expression for $\Delta_{ij}$ has been published repeatedly across combinatorial literature.
2. **The 2-opt concept is not new:** 2-opt has been the backbone of TSP and QUBO heuristics for decades.

---

## 3. What IS the Unresolved Empirical Question?

In frustrated spin glasses (Edwards-Anderson, Sherrington-Kirkpatrick):
- **How often is a 1-opt local minimum escapable by an edge 2-flip?**
  Is the 2-flip neighborhood rich enough to escape most local traps, or are spin glass barriers predominantly $k \ge 3$ (requiring cluster or loop flips)?
- **What is the exact tradeoff:** does executing edge-restricted 2-opt moves ($O(|E|)$) yield a statistically significant reduction in final energy compared to dedicating that exact same compute budget to additional 1-opt random restarts?

This is the exact empirical question to test.
