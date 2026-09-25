# Anomalies & Empirical Phase Transitions: Fundamental AI Research

**Document ID:** `research/fundamental_ai/ANOMALIES.md`  
**Status:** Monitored Registry  
**Author:** Autonomous Research Scientist (`ising_engine`)  

---

## 1. Registered Anomalies from EXP-TEN-001

### ANOM-001: The 4-Body Parity Inversion Cliff
- **Detection Date:** 2026-09-12
- **Condition:** Model D (Sparse 4-Body Hyperedges), $N \in \{128, 256, 512\}$, transition between $\eta = 20\%$ and $\eta = 30\%$.
- **Manifestation:**
  Exact pattern recovery exhibits a discontinuous-like drop:
  - $N=128, P=13, \eta=20\%$: Exact Acc = **94.2%**
  - $N=128, P=13, \eta=30\%$: Exact Acc = **28.5%**
  - $N=256, P=26, \eta=20\%$: Exact Acc = **98.8%**
  - $N=256, P=26, \eta=30\%$: Exact Acc = **28.5%**
  - $N=512, P=51, \eta=20\%$: Exact Acc = **98.1%**
  - $N=512, P=51, \eta=30\%$: Exact Acc = **23.4%**
  - At $P/N \ge 0.25$ and $\eta \ge 30\%$: Exact Acc = **0.0%**.
- **Physical Mechanism:**
  For 4-body terms $s_i s_j s_k s_l$, if two spins in a hyperedge flip simultaneously due to noise, the product $(-1)^2 = +1$ remains invariant, but if one or three spins flip, the product is inverted. At $\eta \ge 25\%$, the binomial probability of odd-parity flips approaches $50\%$, which flips the effective sign of the local field, destabilizing the true attractor.
- **Classification:** `VERIFIED STATISTICAL MECHANICS EFFECT`.

---

### ANOM-002: Scaling Reversal: SNR Grows with $N$ at Fixed Capacity Ratio $P/N$
- **Detection Date:** 2026-09-12
- **Condition:** Model C (LowRankCP / Polynomial DAM $n=3$), $P = \alpha N$ at 40% noise corruption.
- **Manifestation:**
  As system size $N$ increases, recovery under heavy noise (40%) monotonically improves:
  - At $N=128, P=32$: Exact Acc = **53.8%**
  - At $N=256, P=64$: Exact Acc = **71.6%**
  - At $N=512, P=128$: Exact Acc = **93.7%**
- **Physical Mechanism:**
  Proven in `THEORY_SIGNAL_CROSSTALK.md`. The effective signal scales as $(1-2\eta)^2 N^2$, while total distractor variance scales as $P / N^2$. Therefore, at fixed $P/N = \alpha$:
  $$\text{SNR}_3 \propto (1 - 2\eta)^2 \sqrt{\frac{N}{\alpha}} \xrightarrow{N \to \infty} \infty$$
  Unlike Pairwise Hopfield where SNR is strictly $O(1)$ at linear load, degree-3 polynomial overlap suppresses Gaussian crosstalk by a factor of $\sqrt{N}$.
- **Classification:** `CONFIRMED POLYNOMIAL DAM SCALING PROPERTY`.

---

### ANOM-003: Frustration Trapping in Random Sparse Hypergraphs
- **Detection Date:** 2026-09-12
- **Condition:** Model B (Sparse 3-Body), $\eta \ge 35\%$, high pattern load.
- **Manifestation:**
  Greedy coordinate descent fails to terminate within 10 sweeps, frequently reaching the 50-sweep timeout, while dense pairwise and CP models terminate in 2–4 sweeps.
- **Physical Mechanism:**
  Random sparse hypergraphs possess high local variance in degree distribution and create frustrated loops with shallow local minima. The energy landscape lacks the smooth global funnel created by dense all-to-all tensor contractions.
- **Classification:** `CONFIRMED GRAPH TOPOLOGY BOTTLENECK`.

---

### ANOM-004: Odd-Degree Attractor Sign Collapse (Coordinate Sign-Erasure Trap)
- **Detection Date:** 2026-09-12
- **Condition:** Candidate Latent CP-3 Memory under $P \gg R$ on compositional feature grammars.
- **Manifestation:**
  When factors $\{u^r\}$ form an orthonormal basis representing latent coordinates ($R \ll P$), energy relaxation collapses cosine similarity to $\approx 0.32$, severely underperforming simple linear projection ($0.77$).
- **Physical Mechanism:**
  For odd polynomial energy $E(s) \propto -\sum_r (u^r \cdot s)^p$ with $p=3$, the effective field is $h_i \propto \sum_r u_i^r (u^r \cdot s)^2$. Because the squared overlap $(u^r \cdot s)^2 \ge 0$ is strictly positive, the driving force pushes in direction $+u^r$ regardless of whether the projection is positive or negative. The energy landscape has only attractors where all coordinates are positive, erasing half the latent space ($2^R \to 1$).
- **Classification:** `CONFIRMED MATHEMATICAL LIMITATION OF ODD-DEGREE HOMOGENEOUS ENERGIES`.

---

### ANOM-005: Inference Propagation Wavefront in Multi-Hop Energy Chains
- **Detection Date:** 2026-09-12
- **Condition:** Candidate Modern Trilinear Chain in EXP-TEN-004.
- **Manifestation:**
  Under synchronous parallel updates, reasoning accuracy displays a sharp temporal wavefront:
  - $T = 1$: Intermediate bridge $s_X$ attains 100.0% accuracy, but final target $s_C$ remains at 0.9% (chance).
  - $T = 2$: Signal wavefront reaches $s_C$, abruptly jumping final accuracy to 100.0%.
  - $T \ge 4$: Deep energy minimum is reached and preserved.
- **Physical Mechanism:**
  In a physical recurrent network, graph diameter dictates the minimal relaxation time required for information to flow from inputs to output variables. Multi-hop relational reasoning requires $T \ge K$ synchronous sweeps to establish global fixed points.
- **Classification:** `CONFIRMED RECURRENT INFERENCE COMPUTATION SCALING`.

---

### ANOM-006: Limit-Cycle Stalling vs Bidirectional Lyapunov Descent in Cyclic Networks
- **Detection Date:** 2026-09-12
- **Condition:** EXP-TEN-005 Part 2 (Closed Triad Relational Graphs, simultaneous noise $\eta = 0.30$).
- **Manifestation:**
  - Recurrent Directed Attention quickly jumps from $82.2\%$ ($T=1$) to $85.0\%$ ($T=2$), but then **permanently locks** at $85.0\%$ for all $T \ge 4$. Additional compute yields exactly zero gain ($0.0\%$).
  - Modern Trilinear Cycle Energy Relaxation continues to improve monotonically with compute: $T=1 (86.2\%) \to T=2 (96.6\%) \to T=4 (98.4\%) \to T=8 (\mathbf{99.1\%})$.
- **Physical Mechanism:**
  - Directed attention lacks a scalar potential function $E$. In a cyclic graph ($A \to B \to C \to A$), non-variational updates permit periodic orbits / limit cycles where conflicting noisy bits chase each other indefinitely around the ring.
  - Symmetrical trilinear energy relaxation enforces monotonic descent $\Delta E \le 0$ on a joint Lyapunov function. It couples forward and backward forces at each slot, breaking the cyclic asymmetry and funneling the configuration into the ground-state attractor.
- **Classification:** `CANDIDATE EMPIRICAL ADVANTAGE OVER SPECIFIC ASYMMETRIC ATTENTION BASELINE (PRIOR ART AUDIT PENDING)`.

---

### ANOM-007: Recurrent Attention Cyclic Error Amplification vs Lyapunov Invariance
- **Detection Date:** 2026-09-12
- **Condition:** EXP-TEN-006A (Cyclic Constraint Topologies $K \in \{3, 4, 5, 8, 16, 32\}$, inference compute $T \in \{1, 2, 4, 8, 16, 32\}$, noise $\eta \in \{0.10, 0.20, 0.30\}$).
- **Manifestation:**
  - As inference compute $T$ scales from $1 \to 32$, unrolled Recurrent Attention exhibits severe monotonic degradation across all cycle sizes:
    At $K=16, \eta=0.20$: $T=1 (78.8\%) \to T=4 (72.7\%) \to T=16 (63.3\%) \to T=32 (59.7\%)$ ($-19.1\%$ collapse).
    At $K=16, \eta=0.10$: $T=1 (88.6\%) \to T=16 (67.1\%) \to T=32 (62.3\%)$ ($-26.3\%$ collapse).
  - In stark contrast, `LearnedDualEnergy` exhibits exact Lyapunov stability:
    At $K=16, \eta=0.20$: $T=1 (79.7\%) \to T=4 (79.7\%) \to T=16 (79.6\%) \to T=32 (78.3\%)$.
    At $T=16$, energy relaxation outperforms recurrent attention by **$+16.3\%$** ($79.6\%$ vs $63.3\%$).
    At $T=32$, energy relaxation outperforms recurrent attention by **$+18.6\%$** ($78.3\%$ vs $59.7\%$).
- **Physical Mechanism:**
  - The tested recurrent attention is an unconstrained asymmetric non-conservative iterated map $s^{(t+1)} = f(s^{(t)})$. Its Jacobian $\frac{\partial f}{\partial s}$ on closed cyclic factor graphs has complex eigenvalues with modulus near or above unity. Without conservative anchoring or contractive damping, asymmetric circulation of signals around the cycle continually amplifies perturbation errors.
  - Energy relaxation strictly follows $\dot{s} \propto -\nabla_s E$. In continuous time, its Jacobian is the symmetric Hessian $H = \nabla^2 E \in \mathbb{R}^{d \times d}$, guaranteeing all continuous eigenvalues are purely real ($\text{Im}(\lambda) = 0$). In discrete time, stability depends on step size and curvature $\rho(I - \gamma H) \le 1$. Trajectories remain anchored near the observation rather than diverging.
- **Classification:** `EMPIRICAL SIGNAL: LYAPUNOV INERTIA VS ASYMMETRIC ATTENTION ERROR DIVERGENCE (NOVELTY UNRESOLVED)`.


