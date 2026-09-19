---
id: cd004-kill-test
kind: research-protocol
status: active
authority_scope: falsification-protocol
created: 2026-09-19
immutable: false
---

# CD004 Kill-Test Protocol: Native Soft-Conflict Learning vs Chronological Branch-and-Bound

## 1. Objective
Measure whether extracting minimal soft-conflict cores and non-chronologically backjumping achieves a decisive reduction in visited search nodes compared to standard chronological Branch-and-Bound on frustrated Ising spin glasses.

## 2. Benchmark Suite
- **Instance Sizes $N$:** $\{12, 14, 16, 18\}$.
- **Graph Topologies:**
  1. `2D_lattice`: 2D torus with bimodal $\pm 1$ couplings (Edwards-Anderson model, highly frustrated).
  2. `random_sparse`: Erdős-Rényi $G(N, p=0.4)$ with random integer couplings $J_{ij} \in \{-2, -1, 1, 2\}$.
  3. `complete_SK`: Complete graph $K_N$ with random couplings (Sherrington-Kirkpatrick model).

## 3. Evaluators
1. **Chronological BnB (Baseline):**
   Branches on $s_v \in \{-1, +1\}$. Computes $\text{LB}_0(F, \sigma_F)$. If $\text{LB}_0 > E^*$, backtracks immediately to depth $|F| - 1$.
2. **Conflict-Driven BnB (Candidate CDCL-Ising):**
   When $\text{LB}_0(F, \sigma_F) > E^*$, executes the deletion filter to isolate the minimal conflict core $F^* \subseteq F$. Backjumps non-chronologically to $\max_{u \in F^* \setminus \{v_{\text{last}}\}} \text{depth}(u)$.

## 4. Primary Metrics
- `nodes_bnb`: Number of nodes explored by Chronological BnB.
- `nodes_cdcl`: Number of nodes explored by Conflict-Driven BnB.
- `node_reduction`: $(1 - \text{nodes_cdcl} / \text{nodes_bnb}) \times 100\%$.
- `mean_core_ratio`: Average $|F^*| / |F|$ over all conflict events.

## 5. Falsification Rule
- **Kill Condition (NO-GO):** If Conflict-Driven BnB fails to achieve at least a $25\%$ reduction in visited search nodes on frustrated graphs, OR if mean core ratio exceeds $0.85$ (cores are too large to permit meaningful backjumping), the hypothesis of a native CDCL advantage in Ising is **FALSIFIED (NO-GO)**.
- **Success Condition (PASS):** CDCL-Ising achieves $\ge 30\%$ node reduction with mean core ratio $\le 0.70$.
