---
id: cd003-prior-art
kind: research-reference
status: active
authority_scope: prior-art-audit
created: 2026-09-19
immutable: false
---

# CD003 Prior-Art Matrix & Adversarial Literature Attack

Date: 2026-09-19.
Objective: Rigorously audit the Precision-Rank Bounded Response theorem against known literature in threshold logic, multi-parametric programming, and computational complexity.

---

## 1. Prior-Art Mapping Matrix

| Field | Mathematical Mechanism | Equivalence / Overlap | Remaining Distinction |
|---|---|---|---|
| Subset Sum / Additive Combinatorics | Image size of $\sum v_i z_i$ for $z \in \{0, 1\}^b$ with bounded integers $v_i \in [-K, K]$ | **EXACT IDENTITY** for the 1D image size: $|\{\mathbf{v}^T z\}| \le 2bK+1$ is an elementary additive combinatorial fact (Freiman 1973; Tao & Vu 2006). | The image size formula itself is elementary; novelty cannot be claimed on this lemma. |
| Threshold Logic / Perceptrons | Weights required to realize Boolean functions: Muroga (1971); Hastad (1994) | Hastad proved that certain linear threshold functions require weights of magnitude $2^{\Omega(n)}$. | In our context, this explains *why* H11 was able to realize $2^b$ responses: it exploited the Hastad exponential weight regime. Bounding $K$ rules out that regime. |
| Multi-Parametric Programming (mp-MIQP) | Critical regions of $\min_x [c_x + \theta^T a_x]$: Bemporad et al. (2002); Baotic et al. (2006) | Critical regions in continuous $\theta \in \mathbb{R}^r$ are convex polyhedra forming the lower envelope of hyperplanes. | Known in continuous control; our theorem restricts the parameter domain to the discrete image of $\{0, 1\}^b$ under linear projection $V^T$. |
| Variable Elimination / Treewidth | Bucket elimination table size: Dechter (1999) | Standard elimination requires $O(2^w)$ table entries for boundary size $w$. | Bypasses the $2^w$ tabular cost by replacing the $2^w$ raw configurations with $(2wK+1)^r$ effective scalar parameters $\boldsymbol{\theta}$. |
| Fast Multipole Method (FMM) | Low-rank kernel factorization $K(x, z) \approx \sum u_k(x) v_k(z)$: Greengard & Rokhlin (1987) | FMM compresses $O(N^2)$ pairwise interactions to $O(N)$ via truncated multipole expansions. | FMM operates on continuous potentials ($1/r$); our setting is discrete Ising/QUBO with exact integer coefficients. |

---

## 2. Adversarial Scrutiny: What is NOT Novel

1. **The Pigeonhole Bound on Image Size is Elementary:**
   The fact that $\sum_{i=1}^b v_i z_i$ takes at most $2bK + 1$ integer values when $|v_i| \le K$ is high-school arithmetic. Claiming this lemma as an algorithmic invention would be rejected by any reviewer.

2. **The Lower Envelope of Affine Functions is Known:**
   The fact that $\min_x [E(x) + \theta \cdot a_x]$ is a concave piecewise-linear function of $\theta$ is elementary convex analysis (Rockafellar 1970) and standard multi-parametric programming.

---

## 3. What IS the Defensible Scientific Value?

The genuine scientific contribution is **Resolving the Mechanism of Structural Response Compression**:
1. **Resolution of H11:** It cleanly explains why the H11 rank-only claim failed (it required $K = 2^{b-1}$, entering the exponential regime) and proves the exact boundary where structural compression *does* hold ($K \le \text{poly}(b)$).
2. **Elimination Boundary Theorem:** It establishes that low-rank boundary elimination in discrete graphical models does *not* scale as $2^w$ (treewidth), but as $O((wK)^r)$. When boundary coupling is mediated through small integer weights (e.g. collective magnetization or balance constraints), the effective elimination complexity is **strictly polynomial**.
