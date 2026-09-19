---
id: cd004-prior-art
kind: research-reference
status: active
authority_scope: prior-art-audit
created: 2026-09-19
immutable: false
---

# CD004 Prior-Art Matrix & Adversarial Literature Attack

Date: 2026-09-19.
Objective: Rigorously audit native soft-conflict learning against MaxSAT core algorithms, Valued Constraint Satisfaction (WCSP), and Branch-and-Bound solvers.

---

## 1. Prior-Art Mapping Matrix

| Proposed Component | Closest Literature Source | Equivalence / Overlap | Remaining Distinction |
|---|---|---|---|
| Conflict-Driven Clause Learning (CDCL) | Marques-Silva & Sakallah (1999); Moskewicz et al. (Chaff, 2001) | Exact logical foundation: extract minimal core from failure and backjump non-chronologically. | CDCL operates exclusively on hard Boolean clauses (UNSAT). Soft optimization has no empty clause. |
| Core-Guided MaxSAT | Morgado et al. (2014); Narodytska & Bacchus (2014); Martins et al. (Open-WBO, 2014) | Identifies unsatisfiable cores to reformulate the optimization problem. | Requires CNF SAT encoding and cardinality constraints. Does not operate directly on native quadratic spin matrices $J_{ij}$. |
| Soft Nogoods in Valued CSP (WCSP) | Schiex, Fargier & Verfaillie (1995); Cooper et al. (2010); ToulBar2 solver | Maintains soft local consistency ($EDAC$) and records lower-bound violations. | WCSP solvers use specialized hyper-edge tables. Our setting is direct pairwise Ising/QUBO with continuous/integer couplings. |
| Branch-and-Cut for MaxCut / Ising | BiqMac (Rendl, Rinaldi, Wiegele 2010); BiqCrunch (Krislock et al. 2014) | Solves exact MaxCut using Semidefinite Programming (SDP) bounds in a branch-and-bound tree. | BiqMac relies on heavy SDP ($O(N^3)$ per node) and standard chronological branching. It does NOT extract minimal conflict cores or backjump non-chronologically. |
| Roof Duality & QPBO Probing | Boros & Hammer (2002); Rother et al. (2007) | Proves 1-variable persistencies ($s_i = \pm 1$) via network flow. | Probing fixes individual variables unconditionally ($k=1$). It does not learn multi-variable conflict cores ($k \ge 2$) from incumbent bounding. |

---

## 2. Adversarial Scrutiny: What is NOT Novel

1. **The CDCL Backjumping Principle is Known:**
   Learning minimal failure sets and jumping back to the second-highest decision level is the standard 1-UIP rule in SAT solvers since 1996. Applying it to optimization is a natural extension.
2. **Deletion Filtering is Known:**
   The greedy deletion filter for extracting minimal unsatisfiable subsets (MUS) was formalized by Bakker et al. (1993) and Junker (QuickXplain, 2004).

---

## 3. What IS the Unresolved Empirical Question?

In hard, frustrated spin glass instances (e.g. Sherrington-Kirkpatrick or 2D/3D $\pm J$ Edwards-Anderson models):
- **Does the deletion filter extract small, actionable cores ($|F^*| \ll |F|$), or is frustration so diffuse that the core is nearly the entire branch ($|F^*| \approx |F|$)?**
- If $|F^*| = |F| - 1$ or $|F|$, non-chronological backjumping provides **almost zero savings**, while paying the computational overhead of repeated lower-bound evaluations.
- If $|F^*| \ll |F|$ (e.g. $|F^*| \le |F|/2$), backjumping skips entire subtrees of size $2^{|F| - |F^*|}$, dramatically outperforming standard Branch-and-Bound.

This is the exact empirical question to test.
