---
id: candidate-ideas-ledger
kind: reference
status: active
authority_scope: research-backlog
created: 2026-09-19
immutable: false
---

# Candidate Breakthrough Ideas Ledger (CD Research Backlog)

This document is the persistent repository backlog for all candidate breakthrough
hypotheses formulated under the 5-step cross-domain research protocol:
1. Cross-domain mechanism mapping;
2. Precise boundary identification (impossible, expensive, or unverified);
3. Inter-domain transfer with concrete equations;
4. Adversarial prior-art attack and falsification;
5. Minimal decisive experiment (witness) in Ising/QUBO before any heavy solver build.

If an active direction is falsified or rejected under prior-art scrutiny, the
investigation seamlessly pivots to the next candidate in this ledger without loss
of knowledge.

---

## 1. Candidate Queue

### [CD002 / Option C] Discrete Gauge & Permutation Synchronization for Relational AI Coherence
* **Status:** **FALSIFIED / NO-GO** (2026-09-19)
* **Outcome:** Proved exact $K \cdot |S_d|$-fold ground state degeneracy on cycles; zero accuracy advantage over classical Spectral Synchronization on triangulated graphs. Fully documented in `research/breakthrough/cd002/`.

---

### [CD003 / Option A] Precision & Margin-Bounded Response Programs (H11 Extension / Resolution)
* **Status:** **CONFIRMED THEOREM / COMPLETED** (2026-09-19)
* **Outcome:** Proved and verified that $N_{\text{resp}} \le (2bK + 1)^r$. Resolves the H11 paradox: low rank bounds response count polynomially if and only if bit-precision $p \ll b/r$. Compression ratio reaches $\le 0.016$ at $b=8$. Fully documented in `research/breakthrough/cd003/`.
* **Cross-Domain Origin:** Computational Complexity + Parametric Programming + Model Reduction.
* **The Problem:** CD001/H11 proved that low interface rank alone does *not* bound the number of conditional ground-state responses polynomially: an exact counterexample generated $2^b$ unique responses with rank 1.
* **The Missing Assumption:** The counterexample required exponential bit precision in coefficients ($2^i$), causing the normalized spectral gap to shrink as $1/2^b$. In physical hardware and digital architectures, precision is bounded (e.g. 8-bit or 16-bit integers).
* **The Hypothesis:** Under bounded precision (coefficients in $\{-K, \dots, K\}$ with $K = O(\text{poly}(b))$) OR bounded margin ($\Delta \ge 1/\text{poly}(b)$), the number of unique ground-state responses across an interface of rank $r$ is strictly bounded by $O(\text{poly}(b, K))$.
* **The Boundary:** Lower bounds on threshold logic / perceptrons (Muroga 1971; Hastad 1994) show that some linear threshold functions require weights of size $2^{O(n)}$.
* **Cheapest Kill-Test:** Exact enumeration of small bit-width Ising interfaces ($b \in \{2, 3, 4\}$, weights in $\{-2, -1, 0, 1, 2\}$) to determine whether $2^b$ unique responses can be sustained with small integer weights.

---

### [CD004 / Option B] Dual-Bound Certificates & Conflict Pruning from Proof Theory (H03 Extension)
* **Status:** Queued
* **Cross-Domain Origin:** Automated Theorem Proving (SAT/CDCL) + Mathematical Optimization (Lagrangian Duality).
* **The Problem:** Modern SAT solvers (CDCL) solve million-variable instances by learning conflict clauses from unit-propagation failures, pruning vast subtrees ($2^{N-k}$). In soft optimization (Ising/QUBO/MaxSAT), there are no hard unsatisfiable clauses—every configuration is feasible with a soft energy value.
* **The Boundary:** Heuristic local search (tabu, simulated annealing) explores blindly without learning certified forbidden subcubes; branch-and-bound requires expensive linear/SDP relaxations at every tree node.
* **The Transfer:** Compute fast, certified dual lower bounds (via 1-flip / 2-flip local dual certificates or tree decompositions) on a subcube $C$. If $\text{LB}(C) \ge E_{\text{incumbent}}$, prune $C$ permanently as a certified "soft conflict".
* **Cheapest Kill-Test:** Benchmark dual-bound pruning efficiency on small dense MaxCut instances ($N \in [20, 30]$) against pure branch-and-bound.

---

### [CD005 / Option D] Gauge-Aligned Cross-Curvature Barrier Audit & 2-Opt Escapes (H07)
* **Status:** Queued
* **Cross-Domain Origin:** Differential Geometry / Statistical Mechanics.
* **The Problem:** Metropolis/Glauber dynamics get trapped in 1-flip local minima for exponential time $O(e^{\Delta E / T})$ when negative-curvature directions require simultaneous coordinated 2-spin or cycle flips.
* **The Hypothesis:** Closed-form $O(1)$ evaluation of pairwise Hessian/curvature blocks allows instantaneous detection of negative 2-opt escape directions without exhaustive $O(N^2)$ candidate evaluation.
* **Cheapest Kill-Test:** Measure escape rates from deep 1-opt local traps on frustrated 2D/3D spin glasses with bimodal $\pm J$ couplings.

---

### [CD006 / Option E] Amortized Exact Conditioning over Treewidth-Bounded Subgraphs (H01 / H02 / H10)
* **Status:** Queued
* **Cross-Domain Origin:** Knowledge Compilation + Graph Theory + Dynamic Programming.
* **The Problem:** Exact bucket elimination cost scales as $O(n \cdot 2^w)$ where $w$ is treewidth. Most real-world graphs have high treewidth ($w \approx O(n)$), making global elimination impossible.
* **The Transfer:** Identify localized, low-treewidth subgraphs attached to a high-treewidth core. Compile exact conditional response tables once for the low-treewidth peripherals, and optimize only the reduced core.
* **Cheapest Kill-Test:** Reduction ratio on benchmark graphs with heavy tree-like fringes (e.g. Chimera/Pegasus boundaries, power grids).
