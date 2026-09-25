# Novelty Ledger: Adversarial Reduction and Prior Art Deconstruction

**Document ID:** `research/fundamental_ai/NOVELTY_LEDGER.md`  
**Status:** Living Adversarial Audit  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  

---

## 1. Governance and Adversarial Protocol

Every mathematical model, learning rule, and architectural candidate proposed in `research/fundamental_ai` must undergo formal reduction tests before any claim of novelty is entertained.

The default scientific hypothesis for any candidate is:
$$\mathcal{H}_{\text{null}}: \text{The candidate is algebraically, functionally, or computationally isomorphic to prior art.}$$

### Mandatory Reduction Gates:
- **Test A (DAM Reduction):** Can energy be factored as $E(s) = -\sum_\mu F(\xi^\mu \cdot s)$? If yes $\implies$ Dense Associative Memory family (Krotov & Hopfield 2016; Demircigil 2017).
- **Test B (Universal Hopfield Reduction):** Does the update decompose into $\text{Similarity} \to \text{Separation} \to \text{Projection}$? If yes $\implies$ Universal Hopfield family.
- **Test C (Attention Reduction):** Can the field update be rewritten as $V \cdot \text{softmax}(K^T Q)$ or polynomial attention? If yes $\implies$ Transformer attention equivalent.
- **Test D (Kernel Retrieval):** Is output equivalent to Nadaraya-Watson kernel regression $\sum_\mu K(x, \xi^\mu) \xi^\mu$?
- **Test E (Nearest-Neighbor Memory):** Does a trivial 1-NN Hamming oracle match or exceed candidate accuracy at lower computational cost?
- **Test F (Parameter Storage vs Compression):** If rank $R = P$ and each factor corresponds to one training exemplar, it is **explicit memory storage**, NOT latent compression.
- **Test G (Transformer Reduction):** Is the recurrence isomorphic to an Energy Transformer / NRGPT / recurrent attention layer?
- **Test H (Fast-Weight Reduction):** Are dynamic weights isomorphic to Hebbian fast weights or test-time meta-learning?

---

## 2. Adversarial Deconstruction of Implemented Models

### Model C: Low-Rank CP 3-Body Tensor Memory (`LowRankCPMemory`)
- **Hypothesis ID:** `HYP-C-001`
- **Mathematical Form:**
  $$E(s) = -\sum_{i < j < k} T_{ijk} s_i s_j s_k, \quad T_{ijk} = \sum_{r=1}^P \lambda_r a_{ir} a_{jr} a_{kr}, \quad a_r = \xi^r$$
  Local field:
  $$h_i^{\text{eff}}(s) = \sum_{r=1}^P \lambda_r a_{ir} \sum_{j < k, j,k \ne i} a_{jr} a_{kr} s_j s_k \approx \frac{1}{2} \sum_{r=1}^P \lambda_r \xi_i^r (\xi^r \cdot s)^2$$
- **Closest Prior Art:**
  1. Krotov & Hopfield (2016) *Dense Associative Memory for Pattern Recognition* (polynomial interaction function $F(x) = x^n$ with $n=3$).
  2. Psaltis & Cheol Park (1986) *Nonlinear discriminant functions and associative memories*.
  3. Gardner (1987) *Multiconnected neural networks*.
- **Algebraic Reduction:**
  For $a_r = \xi^r$, the CP tensor contraction $\sum_{i < j < k} \xi_i^r \xi_j^r \xi_k^r s_i s_j s_k$ is the exact 3rd elementary symmetric polynomial of the vector $z_i = \xi_i^r s_i$.
  By Newton-Girard identities, up to diagonal self-interaction corrections of order $O(N)$, this is identically:
  $$E(s) = -\frac{1}{6 N^2} \sum_{r=1}^P (\xi^r \cdot s)^3 + O(N)$$
  This is algebraically identical to Polynomial Dense Associative Memory with power $n=3$.
- **Functional Reduction:**
  The update step $s_i \leftarrow \text{sign}(h_i^{\text{eff}})$ computes $h_i^{\text{eff}} \propto \sum_{r=1}^P \xi_i^r (\xi^r \cdot s)^2$.
  This passes **Test A** (DAM), **Test B** (Universal Hopfield with polynomial separation $f(u) = u^2$), and **Test D** (Kernel retrieval with quadratic polynomial kernel $K(x, y) = (x \cdot y)^2$).
- **Computational Reduction:**
  Contraction cost is $O(NP)$ per sweep. Exactly matches polynomial DAM.
- **What is Actually Different?**
  Nothing conceptually. The CP formulation implements the exact coordinate-descent discrete energy dynamics of a degree-3 Dense Associative Memory using tensor algebraic factorizations.
- **Novelty Confidence:** `NOT NOVEL` (Pure reproduction of established Polynomial DAM physics).
- **Experimental Status:** Validated as high-precision baseline / reference implementation, NOT as an architectural discovery.

---

### Model B: Sparse 3-Body Hyperedge Memory (`SparseHyperedgeMemory`)
- **Hypothesis ID:** `HYP-B-001`
- **Mathematical Form:**
  $$E(s) = -\sum_{e=(i, j, k) \in \mathcal{E}_3} w_e s_i s_j s_k, \quad |\mathcal{E}_3| = \binom{N}{2}$$
  Sparse hyperedges selected with degree-regular budget matched to pairwise Hopfield.
- **Closest Prior Art:**
  1. Dilute / Sparse Hopfield networks (Derrida, Gardner, Zippelius 1987).
  2. Hypergraph cuts and LDPC parity check matrices.
- **Algebraic Reduction:**
  Cannot be reduced to a function of scalar overlaps $\sum_\mu F(\xi^\mu \cdot s)$ because the hyperedge adjacency matrix is sparse and topological.
- **Functional Reduction:**
  Local message passing on a 3-uniform hypergraph.
- **What is Actually Different?**
  Tests whether a topological constraint (fixing parameter budget to $O(N^2)$ via sparse hyperedges) preserves higher-order noise suppression.
- **Empirical Verdict from EXP-TEN-001:**
  - At $P/N = 0.10$, expands attraction basin (holding 86.7% at 30% noise vs 47.7% for Pairwise).
  - At $P/N \ge 0.25$, **collapses** to $\approx 2\%$ recovery.
- **Novelty Confidence:** `DERIVED / POSSIBLY NOVEL TOPOLOGY`, but `EMPIRICALLY LIMITED` as an associative memory.

---

### Model D: Sparse 4-Body Hyperedge Memory (`SparseHyperedgeMemory`)
- **Hypothesis ID:** `HYP-D-001`
- **Mathematical Form:**
  $$E(s) = -\sum_{e=(i, j, k, l) \in \mathcal{E}_4} w_e s_i s_j s_k s_l, \quad |\mathcal{E}_4| = \binom{N}{2}$$
- **Closest Prior Art:**
  4-spin glass models, PUBO/HOBO optimization.
- **Algebraic Reduction:**
  Topological 4-uniform hypergraph.
- **Empirical Verdict from EXP-TEN-001:**
  Extreme threshold behavior: near-perfect recall (99.8%) at low noise (10%), but sharp catastrophic cliff at $\ge 30\%$ noise due to parity multiplication sensitivity.
- **Novelty Confidence:** `DERIVED`.

---

## 3. Novelty Summary Matrix

| Candidate / Mechanism | Reduction Status | Governing Literature | Legitimate Scientific Value |
|---|---|---|---|
| **Model C (LowRankCP, R=P)** | **Fully Reduced to Polynomial DAM ($n=3$)** | Krotov & Hopfield (2016) | High-performance exact reference engine for polynomial energy dynamics. |
| **Model B (Sparse 3-Body, budgeted)** | **Irreducible to overlap DAM; topological hypergraph** | Derrida et al. (1987) | Proves sparse hyperedges widen basins at low $P$, but fail to scale capacity at equal parameter count. |
| **Model D (Sparse 4-Body, budgeted)** | **Irreducible to overlap DAM; 4-uniform hypergraph** | HOBO literature | Reveals sharp parity thresholding in higher-order sparse energy landscapes. |
| **Candidate Latent CP-3 ($P \gg R$)** | **Defeated by Linear SVD Subspace; Sign-Erasure Pathology** | Subspace PCA / Anandkumar (2014) | Proves that odd-degree ($p=3$) energy dynamics erase coordinate signs; proves even/odd exponent selection rule. |
| **Candidate Modern Trilinear Chain** | **Reduced to Recurrent Modern Hopfield / Softmax Attention** | Ramsauer et al. (2020), Smolensky (1990) | Demonstrates physical inference-time compute scaling (T=1: 0.9% -> T=2: 100%), but reduced to recurrent softmax attention on DAGs. |
| **Non-Polynomial / Dual Latent EBMs ($P \gg R$)** | **Overcomes Sign-Erasure & Power Overflow; Matches Linear SVD** | Olshausen & Field (1996), Krotov (2021) | Resolves sign-erasure; proves that bounded nonlinearities ($\tanh$) or dual latent variables are mandatory for multi-factor representation. |
| **Modern Trilinear Cycle Energy Network (EXP-TEN-005)** | **Specialized Triad Case of Permutation Synchronization** | Pachauri et al. (2013), Loopy BP | Outperformed specific asymmetric recurrent attention baseline on fixed triads, but does not generalize to a scalable learning rule. |
| **Learned Dual-Variable Energy Network (EXP-TEN-006A)** | **FALSIFIED AS ACTIVE REASONER (Inertia Illusion)** | Pachauri et al. (2013), DEQ | Falsified: Rescue Rate $= 0.7\%$, identical to zero-interaction ablation ($p=1.000$). "Stability" was an artifact of remaining stuck at noisy inputs while asymmetric attention diverged. |
| **Learned Analytic Energy & Contractive GNN (EXP-TEN-006B)** | **FALSIFIED ON DISCRETE CONSTRAINTS (Inertia-Erosion Dilemma)** | Pachauri et al. (2013), Loopy Min-Sum BP | Decisively falsified: Net Rescue $\le +3.7\%$ (Pilot Gate failed), defeated by classical Spectral Synchronization ($+40.1\%$) and Loopy Min-Sum BP ($+40.6\%$). Continuous relaxations cannot replace discrete group synchronization. |



