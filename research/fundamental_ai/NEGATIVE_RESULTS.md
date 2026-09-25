# Negative Results Ledger: Fundamental AI Research

**Document ID:** `research/fundamental_ai/NEGATIVE_RESULTS.md`  
**Status:** Active Scientific Registry  
**Author:** Autonomous Research Scientist (`ising_engine`)  

---

## 1. Protocol for Negative Results

In this project, negative results and falsifications are valued equally with positive findings. They prevent circular research, eliminate unpromising model spaces, and establish rigorous boundaries on what higher-order interactions can and cannot achieve.

---

## 2. Catalog of Falsified Hypotheses

### NR-001: Sparse Hyperedges with Budget-Matched Parameters Fail to Break the Capacity Barrier
- **Hypothesis:** Sparse 3-body hyperedges $(i, j, k, w)$ constrained to the exact parameter budget of pairwise Hopfield ($M_3 = \binom{N}{2}$) will exceed the classical extensive capacity limit ($P > 0.138N$) due to higher-order separation.
- **Experimental Test:** EXP-TEN-001 ($N \in \{128, 256, 512\}$, $P/N \in \{0.10, 0.25, 0.50, 1.00\}$).
- **Outcome:**
  - At $P/N = 0.10$ ($P < 0.138N$): Model B expands the basin of attraction (86.7% recovery at 30% noise vs 47.7% for Pairwise Hopfield).
  - At $P/N = 0.25$ ($P > 0.138N$): Model B collapses catastrophically (exact recovery drops to $2.7\%$ at $N=256$ and $0.1\%$ at $N=512$).
- **Root Cause Analysis:**
  Dense pairwise models have $O(N^2)$ edges connecting all pairs. A sparse 3-body model with $O(N^2)$ hyperedges only connects a tiny fraction $\frac{\binom{N}{2}}{\binom{N}{3}} \approx \frac{3}{N}$ of all available triplets. At $P > 0.14N$, the random crosstalk across uncoupled or weakly coupled triplets overwhelms the sparse signal.
- **Verdict:** `CONCLUSIVELY FALSIFIED`. Parameter-matched random/degree-regular sparse hyperedges cannot scale associative capacity beyond $O(N)$.

---

### NR-002: Sparse 4-Body Hyperedges Suffer from Parity-Induced Catastrophic Basin Collapse
- **Hypothesis:** 4-body hyperedge interactions $s_i s_j s_k s_l$ will exhibit deeper energy wells and greater robustness to noise than 3-body or 2-body interactions.
- **Experimental Test:** Model D in EXP-TEN-001.
- **Outcome:**
  - At $\eta \le 20\%$ noise: Model D achieves near-perfect recovery (up to 99.8%).
  - At $\eta \ge 30\%$ noise: Exact recovery drops abruptly to $0.0\%$.
- **Root Cause Analysis:**
  The interaction energy contains products of four spins. Under independent Bernoulli noise with flip probability $\eta$, the expected product of 4 spins changes sign with probability:
  $$\mathbb{P}(\text{parity flip}) = \binom{4}{1} \eta (1-\eta)^3 + \binom{4}{3} \eta^3 (1-\eta) \approx 4\eta - 12\eta^2 + 16\eta^3$$
  For $\eta = 0.30$, this probability exceeds $45\%$, completely destroying the correlation signal and creating inverted local fields.
- **Verdict:** `CONCLUSIVELY FALSIFIED` for wide attraction basins.

---

### NR-003: Model C (LowRankCP with $R=P$) is NOT a Novel Architecture
- **Hypothesis:** Factorized CP 3-body tensor energy relaxation represents a novel computational primitive for AI.
- **Adversarial Reduction Test:** Test A (DAM Reduction) & Test B (Universal Hopfield Reduction).
- **Outcome:**
  Algebraic proof proves that Model C is identically equivalent to Polynomial Dense Associative Memory ($n=3$) of Krotov & Hopfield (2016). The strong retrieval performance at $P \approx N$ is the well-known $O(N^2)$ capacity scaling of polynomial DAM, not a new architectural mechanism.
- **Verdict:** `FALSIFIED AS NOVELTY`. Retained as a verified reference baseline.

---

### NR-004: Odd-Degree Polynomial Energy Models ($p=3$) Suffer from Sign-Erasure Pathology Under Latent Representation ($P \gg R$)
- **Hypothesis:** When $P \gg R$, low-rank CP 3-body tensor energy relaxation will learn a compact generative basis and reconstruct unseen grammatical patterns better than linear projection or 1-NN.
- **Experimental Test:** EXP-TEN-003 ($N=128$, $K=9$, $P_{\text{train}} \in \{128, 192\}$, $R \in \{4, 8, 16, 32\}$, $\eta \in \{15\%, 30\%\}$).
- **Outcome:**
  - `B1_LinearSVD` achieves $0.773$ cosine similarity on unseen grammatical patterns.
  - `Candidate_LatentCP3` achieves only $0.322$ cosine similarity on unseen patterns, performing worse than linear projection and worse than pairwise Hopfield ($0.453$).
- **Root Cause Analysis:**
  For odd polynomial energy $E(s) \propto -\sum_r (u^r \cdot s)^p$ with $p=3$, the effective field is $h_i \propto \sum_r u_i^r (u^r \cdot s)^2$. Because $(u^r \cdot s)^2 \ge 0$ is strictly non-negative, the driving field erases the sign of the latent coordinates $(u^r \cdot s)$, pushing in direction $+u^r$ regardless of whether the true projection was positive or negative.
- **Verdict:** `CONCLUSIVELY FALSIFIED`. Odd-degree polynomial energy models are mathematically incapable of serving as latent coordinate representations. Higher-order latent models must use even-degree polynomials ($p \in \{4, 6\}$) or asymmetric multi-role bindings.

---

### NR-005: Linear Trilinear Tensor Products (Hebbian TPR) Suffer from Multi-Hop Error Amplification
- **Hypothesis:** Linear trilinear Hebbian binding $T = \sum_\mu \xi_A^\mu \otimes \xi_R^\mu \otimes \xi_B^\mu$ can reliably transmit relational state across a multi-hop reasoning chain.
- **Experimental Test:** EXP-TEN-004 ($M_E = 32$, $M_R = 4$, $N_E = 64$, 2-hop composition).
- **Outcome:** Exact 2-hop recovery is $0.0\% - 0.3\%$.
- **Root Cause Analysis:** Linear summation of distractor facts produces Gaussian interference variance $\approx M \sigma^2$. At $M/N = 0.50$, this creates an irreducible $\approx 8\%$ bit error rate at hop 1. The corrupted intermediate state degrades the second hop multiplicatively, resulting in total compositional collapse.
- **Verdict:** `CONCLUSIVELY FALSIFIED`. Linear tensor product representations cannot support deep multi-hop relational inference without non-linear contrast separation.

---

### NR-006: Pairwise Networks Suffer Total Binding Collapse in Multi-Relational Triples
- **Hypothesis:** Standard pairwise Hopfield couplings between entity and relation slots can solve relational composition via linear cue combination.
- **Experimental Test:** EXP-TEN-004 (`B0_PairwiseChain`).
- **Outcome:** Exact 2-hop recovery is $0.0\%$ across all steps and noise levels.
- **Root Cause Analysis:** Pairwise models linearly sum signals $W_{AX} s_A + W_{RX} s_R$. This independently activates all objects associated with entity $A$ across all relations, and all objects associated with relation $R$ across all entities. The absence of multiplicative conjunctive gating causes catastrophic superposition crosstalk.
- **Verdict:** `CONCLUSIVELY FALSIFIED`. Higher-order (at least 3-way) conjunctive binding is strictly necessary for relational role-filler binding.

---

### NR-007: Polynomial Power-Overflow Pathology in High-Degree Latent EBMs ($p \ge 4$)
- **Hypothesis:** Higher-degree homogeneous even polynomials ($p=4, 6$) will match or exceed linear subspace projection in latent representation learning ($P \gg R$).
- **Experimental Test:** EXP-TEN-005 Part 1 ($N=128, R=24, P_{\text{train}}=800, P_{\text{test}}=200$).
- **Outcome:** Quartic ($p=4$) achieves $0.395$ and Sextic ($p=6$) achieves $0.402$ cosine similarity—beating Cubic ($p=3, 0.325$), but falling far short of Linear SVD ($0.924$) and Saturating EBMs (`LogCosh`, $0.630$).
- **Root Cause Analysis:** Monomial field derivatives $f(x) = x^3$ or $x^5$ produce extreme dynamic range disparities: a dominant latent coordinate with overlap $4.0$ exerts force $4^3 = 64$, completely crushing a secondary coordinate with overlap $1.0$ (force $1^3 = 1$). This creates winner-take-all distortion across multiple distributed latent coordinates.
- **Verdict:** `CONCLUSIVELY FALSIFIED`. High-degree polynomial potentials cannot serve as multi-factor distributed latent representations. Saturated potentials ($\tanh$, rational) or dual-variable latent models are mandatory.

---

### NR-008: Limit-Cycle Stalling of Unrolled Directed Attention on Cyclic Constraint Systems
- **Hypothesis:** Deeply unrolling directed Transformer attention ($T \in \{1, 2, 4, 8, 16\}$) can solve cyclic relational constraints on closed triads ($A \to B \to C \to A$) under simultaneous multi-slot noise.
- **Experimental Test:** EXP-TEN-005 Part 2 ($M_E=32, N_E=64, N_R=32$, simultaneous noise $\eta = 0.30$).
- **Outcome:** Recurrent Attention plateaus at $85.0\%$ exact triad recovery at $T=2$ and fails to improve at $T=4, 8, 16$ ($85.0\%$). In contrast, Energy Relaxation climbs monotonically to $99.1\%$ ($p < 10^{-12}$).
- **Root Cause Analysis:** In this specific directed architecture, unconstrained unrolled attention lacked contractive damping or a global scalar Lyapunov potential. Errors introduced in one slot circulated cyclically to adjacent slots.
- **Verdict:** `EMPIRICAL DEFECT OF SPECIFIC BASELINE`. The specific RecurrentAttention implementation tested in EXP-TEN-005/006A degraded as recurrent depth increased under this protocol. This demonstrates instability of this particular unrolled baseline, not an impossibility theorem for all attention mechanisms, since symmetric or damped attention can admit Lyapunov functions.

---

### NR-009: Finite-Difference Gradient Vanishing in Unrolled Energy Relaxation
- **Hypothesis:** Local energy interaction parameters $\theta$ (1,872 weights) can be trained end-to-end via unrolled BPTT with finite-difference gradient estimation to learn permutation constraint satisfactions on cyclic graphs.
- **Experimental Test:** EXP-TEN-006A ($K \in \{3, 4, 5\}$, 60 graphs, 20 epochs, 4 unroll steps).
- **Outcome:** Training loss stalled completely ($1.0185 \to 1.0184$). The network maintained Lyapunov stability in inference, but failed to optimize discrete permutation operators, yielding $0.0\%$ exact graph solves ($d_H = 0$).
- **Root Cause Analysis:** For a small relaxation step $\gamma = 0.05$, a single weight perturbation $\epsilon = 10^{-4}$ causes an attenuated state change of $O(10^{-7})$ at $T=4$. The finite difference $\Delta L / \epsilon$ vanishes, starving the weights of learning signals.
- **Verdict:** `METHODOLOGICAL IMPLEMENTATION DEFECT`. Finite-difference unrolled training used in EXP-TEN-006A exhibited vanishing signal due to numerical attenuation across small relaxation steps. This is an implementation artifact of numerical finite differences, not a fundamental failure of analytical BPTT or energy-based learning.

---

### NR-010: Failure of Continuous Energy and Neural Relaxation on Discrete Group Synchronization (Inertia-Erosion Dilemma)
- **Hypothesis:** A shared local continuous energy-based model or contractive message-passing neural network can learn to perform active error correction (Net Rescue $> +30\%$) on cyclic permutation synchronization constraints ($G = S_d$).
- **Experimental Test:** EXP-TEN-006B (50 test cycles, $K \in \{3, 6\}$, $d=16$, noise $20\%$, trained with Exact Analytical BPTT and Exact Analytical EqProp, compared against Oracle Linear EBM, Oracle Contractive GNN, and classical baselines).
- **Outcome:**
  - BPTT-Trained $E_{\ln\cosh}$: Net Rescue = **$-0.2\%$** (Inert, Rescue Rate $0.0\%$, Damage Rate $0.2\%$).
  - EqProp-Trained $E_{\ln\cosh}$: Net Rescue = **$-0.2\%$** (Inert, Rescue Rate $0.0\%$, Damage Rate $0.2\%$).
  - Oracle Contractive GNN: Net Rescue = **$+1.7\%$** (Erosion, Rescue Rate $20.0\%$, Damage Rate $18.3\%$, Accuracy drops from $80.1\% \to 69.4\%$).
  - Oracle Linear EBM: Net Rescue = **$+3.7\%$** (Erosion, Rescue Rate $22.6\%$, Damage Rate $18.9\%$, Accuracy drops from $79.1\% \to 68.8\%$).
  - Classical Spectral Synchronization: Net Rescue = **$+40.1\%$** (Rescue Rate $43.8\%$, Damage Rate $3.8\%$, Accuracy rises to $85.1\%$).
  - Classical Loopy Min-Sum BP: Net Rescue = **$+40.6\%$** (Rescue Rate $54.9\%$, Damage Rate $14.3\%$, Accuracy $79.1\%$).
- **Root Cause Analysis:**
  Continuous local relaxation on discrete constraint graphs faces the **Inertia-Erosion Dilemma**:
  1. *Inertia Trap ($\ln\cosh$ potential):* The gradient $\nabla_s \ln\cosh(\beta s_v^T W s_u) \propto \tanh(\beta s_v^T W s_u) \to 0$ as $s_v \to 0$. The interaction force vanishes at the zero boundary, creating an insurmountable barrier that prevents bits from flipping sign under observation anchoring ($\lambda_{\text{obs}}$).
  2. *Erosion Trap (Linear EBM / GNN):* When the interaction force is linear, corrupted nodes transmit corrupted continuous fields into clean nodes without discrete winner-take-all pruning. Symmetrical error diffusion damages clean bits at nearly the exact rate it rescues corrupt bits, degrading net graph accuracy.
  3. *Why Classical Solvers Dominate:* Spectral Synchronization projects directly onto the global harmonic eigenspace of the connection matrix $\mathbf{R}$, bypassing iterative local diffusion; Loopy Min-Sum uses discrete $\max$-marginalization to dynamically suppress noise and enforce discrete group constraints.
- **Verdict:** `CONCLUSIVELY FALSIFIED`. Continuous local energy relaxation and continuous neural message passing cannot replace discrete group synchronization algorithms for permutation constraint satisfaction.



