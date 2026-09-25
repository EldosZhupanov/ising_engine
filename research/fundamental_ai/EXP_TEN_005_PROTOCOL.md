# Experimental Protocol: EXP-TEN-005
## Autonomous Energy Functional Search: Resolving the Sign-Erasure Pathology and Cyclic Constraint Dynamics

**Experiment ID:** `EXP-TEN-005`  
**Document ID:** `research/fundamental_ai/EXP_TEN_005_PROTOCOL.md`  
**Status:** Frozen & Approved for Execution  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  

---

## 1. Scientific Motivation & Context

In `EXP-TEN-001` through `EXP-TEN-004`, we established:
1. **Model C Reduction (NR-003):** Homogeneous cubic tensor memory ($R=P$) reduces to degree-3 Polynomial DAM and collapses under correlation ($\rho=0.6$, clusters).
2. **Sign-Erasure Pathology (NR-004):** In latent compression ($P \gg R$), odd-degree potentials $E = -\frac{1}{p} \sum_r (u^r \cdot s)^p$ with $p=3$ yield local fields $h_i \propto \sum_r u_i^r (u^r \cdot s)^2$. Because $(u^r \cdot s)^2 \ge 0$, the sign of the latent coordinate is erased, collapsing test cosine similarity to $0.322$ where linear SVD achieves $0.773$.
3. **DAG Unrolling Reduction (Test G):** While Modern Trilinear Energy demonstrated inference-time compute scaling ($T=1: 0.9\% \to T=2: 100\%$) on 2-hop relational reasoning, the acyclic chain $A \xrightarrow{R_1} B \xrightarrow{R_2} C$ is a DAG, making 2-step relaxation functionally reducible to an unrolled 2-layer Attention / Modern Hopfield forward pass.

To discover a **genuine, non-reducible computational primitive for AI**, we must address two foundational frontiers:
1. **Frontier 1 (Latent Energy Functional Search):** Can higher-order energy potentials cure the Sign-Erasure Pathology and match or beat Linear SVD subspace projection on unseen compositional grammar?
2. **Frontier 2 (Cyclic Constraint Satisfaction):** Can energy dynamics solve closed-loop cyclic constraints ($A \xrightarrow{R_1} B \xrightarrow{R_2} C \xrightarrow{R_3} A$) where feed-forward attention architectures fail because no topological causal ordering exists?

---

## 2. Mathematical Formulations

### 2.1 Part 1: Latent-Configuration Energy Potentials ($P \gg R$)

Let $U \in \mathbb{R}^{R \times N}$ be the latent factor matrix extracted from training data, with row vectors $u^r \in \mathbb{R}^N$.
Let $a_r(s) = u^r \cdot s = \sum_{i=1}^N u_i^r s_i$ denote the overlap of spin state $s \in \{-1, +1\}^N$ onto factor $r$.

We formulate and evaluate the following functional energy forms:

1. **Cubic Potential ($p=3$, Baseline Reference):**
   $$E_3(s) = -\frac{1}{3} \sum_{r=1}^R \lambda_r (u^r \cdot s)^3 \implies h_i^{(3)}(s) = \sum_{r=1}^R \lambda_r u_i^r (u^r \cdot s)^2$$
   *Pathology:* $(u^r \cdot s)^2 \ge 0$ unconditionally erases the sign of $(u^r \cdot s)$.

2. **Quartic Potential ($p=4$, Even-Degree Polynomial):**
   $$E_4(s) = -\frac{1}{4} \sum_{r=1}^R \lambda_r (u^r \cdot s)^4 \implies h_i^{(4)}(s) = \sum_{r=1}^R \lambda_r u_i^r (u^r \cdot s)^3$$
   *Sign Property:* $(u^r \cdot s)^3$ is an **odd function** of overlap: $\text{sign}((u^r \cdot s)^3) = \text{sign}(u^r \cdot s)$.
   *Non-linear Contrast:* Overlaps $> 1$ are super-linearly amplified; small noise overlaps are suppressed.

3. **Sextic Potential ($p=6$, High-Degree Even Polynomial):**
   $$E_6(s) = -\frac{1}{6} \sum_{r=1}^R \lambda_r (u^r \cdot s)^6 \implies h_i^{(6)}(s) = \sum_{r=1}^R \lambda_r u_i^r (u^r \cdot s)^5$$
   *Sign Property:* Odd function; extreme non-linear separation.

4. **Log-Cosh Potential (Non-Polynomial Smooth Saturation):**
   $$E_{\text{log-cosh}}(s; \beta) = -\frac{1}{\beta} \sum_{r=1}^R \lambda_r \ln \cosh(\beta (u^r \cdot s)) \implies h_i^{\text{lc}}(s) = \sum_{r=1}^R \lambda_r u_i^r \tanh(\beta (u^r \cdot s))$$
   *Sign Property:* $\tanh(-\beta x) = -\tanh(\beta x)$ preserves sign; bounded in $[-1, +1]$; avoids runaway gradient blowup.

5. **Rational Saturating Potential (Robust Cauchy/Lorentzian):**
   $$E_{\text{rat}}(s; \gamma) = -\sum_{r=1}^R \lambda_r \frac{(u^r \cdot s)^2}{1 + \gamma (u^r \cdot s)^2} \implies h_i^{\text{rat}}(s) = \sum_{r=1}^R \lambda_r u_i^r \frac{2 (u^r \cdot s)}{(1 + \gamma (u^r \cdot s)^2)^2}$$
   *Sign Property:* Odd function; suppresses distant outliers.

6. **Dual Latent-Configuration Model (Explicit Latent Optimization):**
   $$E(s, z) = \frac{1}{2} \|z\|_2^2 + \alpha \|z\|_1 - \sum_{r=1}^R z_r (u^r \cdot s)$$
   *Dynamics:*
   $$z_r^{(t+1)} = \text{SoftThreshold}(u^r \cdot s^{(t)}, \alpha)$$
   $$h_i^{(t+1)} = \sum_{r=1}^R u_i^r z_r^{(t+1)}, \quad s_i^{(t+1)} = \text{sign}(h_i^{(t+1)})$$

7. **Linear SVD Subspace Baseline:**
   $$s^* = \text{sign}\left(V V^T s\right)$$

---

### 2.2 Part 2: Cyclic Constraint Satisfaction (Beyond Directed Attention)

Consider a relational knowledge graph where facts form a closed cycle:
$$\text{Triad}: \quad e_A \xrightarrow{r_1} e_B \xrightarrow{r_2} e_C \xrightarrow{r_3} e_A$$

During inference:
- All three entities $s_A, s_B, s_C$ are **simultaneously corrupted** by bit-flip noise $\eta \in \{0.15, 0.30, 0.45\}$.
- The system must recover the exact triplet $(e_A, e_B, e_C)$ using the cyclic constraint structure.

#### Evaluated Architectural Paradigms:

1. **`Feedforward_Attention_1Pass` (Standard Directed Transformer Layer):**
   Computes updates in a fixed arbitrary order (e.g. $A \to B \to C$):
   $$\hat{s}_B = \text{SoftmaxAttention}(s_A, r_1)$$
   $$\hat{s}_C = \text{SoftmaxAttention}(\hat{s}_B, r_2)$$
   $$\hat{s}_A = \text{SoftmaxAttention}(\hat{s}_C, r_3)$$
   *Failure Mode:* Early errors in $\hat{s}_B$ cascade into $\hat{s}_C$ and destroy $\hat{s}_A$.

2. **`Feedforward_Attention_2Pass` (2-Layer Cascade):**
   Repeats the directed pass a second time.

3. **`Recurrent_Attention_T` (Unrolled Attention without Energy):**
   Applies directed updates cyclically for $T$ steps without an energy Lyapunov function.

4. **`Pairwise_Hopfield_Cycle`:**
   Pairwise couplings on the triangle:
   $$E_{\text{pair}} = - s_A^T W_{AB} s_B - s_B^T W_{BC} s_C - s_C^T W_{CA} s_A$$

5. **`Modern_Trilinear_Cycle_Energy` (Candidate Energy Primitive):**
   The joint energy is the sum of 3 trilinear relational potentials on the cycle:
   $$E(s_A, s_B, s_C) = E(s_A, r_1, s_B) + E(s_B, r_2, s_C) + E(s_C, r_3, s_A)$$
   where each potential is:
   $$E(s_{\text{in}}, r, s_{\text{out}}) = -\frac{1}{\beta} \ln \sum_{\mu=1}^M \exp\left(\beta \frac{(s_{\text{in}} \cdot \xi_1^\mu)(r \cdot \xi_R^\mu)(s_{\text{out}} \cdot \xi_2^\mu)}{N_E \sqrt{N_R}}\right)$$
   The effective field on each slot receives **bidirectional constraint forces**:
   $$h_A = \nabla_{s_A} E = h_{\text{forward}}(r_1, s_B) + h_{\text{backward}}(s_C, r_3)$$
   $$h_B = \nabla_{s_B} E = h_{\text{forward}}(r_2, s_C) + h_{\text{backward}}(s_A, r_1)$$
   $$h_C = \nabla_{s_C} E = h_{\text{forward}}(r_3, s_A) + h_{\text{backward}}(s_B, r_2)$$
   Discrete spin dynamics synchronously or asynchronously flips spins to minimize $E(s_A, s_B, s_C)$.

---

## 3. Preregistered Hypotheses

- **$H_{1\text{a}}$ (Even-Power Sign Preservation):**
  Quartic ($p=4$) and Sextic ($p=6$) potentials will achieve test cosine similarity $> 0.70$ on unseen compositional grammar, overcoming the Sign-Erasure failure of Cubic ($p=3$, cosine $0.322$).
- **$H_{1\text{b}}$ (Non-Polynomial Saturation):**
  Log-Cosh energy ($\tanh$) will equal or exceed Linear SVD subspace projection on unseen grammar, providing bounded gradient dynamics without polynomial overflow.
- **$H_{2\text{a}}$ (Energy Superiority on Cyclic Constraints):**
  On simultaneous 3-slot corruption ($\eta \ge 0.30$), `Modern_Trilinear_Cycle_Energy` with $T \ge 4$ will achieve strictly higher Exact Triad Recovery than `Feedforward_Attention_1Pass` and `Feedforward_Attention_2Pass` ($p < 0.001$, Wilcoxon signed-rank test).
- **$H_{2\text{b}}$ (Trilinear vs Pairwise on Cycles):**
  `Pairwise_Hopfield_Cycle` will suffer catastrophic crosstalk ($< 10\%$ recovery), confirming the necessity of higher-order trilinear relational binding.
- **$H_0$ (Null Hypothesis):**
  No energy functional outperforms Linear SVD on unseen grammar, and feedforward attention cascades equal or outperform energy relaxation on cyclic constraint satisfaction.

---

## 4. Experimental Protocols & Parameters

### Protocol 1: Latent Representation Reconstruction ($P \gg R$)
- Total patterns: $P = 1,000$ compositional sentences from context-free grammar.
- Dimension: $N = 128$.
- Latent rank: $R = 24$.
- Train/Test Split: 800 train / 200 test (test sentences strictly held out).
- Noise corruption on test queries: $\eta \in \{0.0, 0.15, 0.30\}$.
- Metrics:
  - Mean Cosine Similarity to ground-truth clean test sentence: $\cos(\hat{s}, s^{\text{clean}})$.
  - Bit Error Rate (BER): $\frac{1}{N} d_H(\hat{s}, s^{\text{clean}})$.
  - Exact Recovery Rate: fraction of test instances with $d_H = 0$.

### Protocol 2: Cyclic Constraint Satisfaction
- Entities: $M_E = 32$, $N_E = 64$.
- Relations: 3 distinct relations $r_1, r_2, r_3$, $N_R = 32$.
- Triad Knowledge Base: $M = 32$ closed triads $(e_i, r_1, e_{\pi_1(i)}), (e_{\pi_1(i)}, r_2, e_{\pi_2(\pi_1(i))}), (e_{\pi_2(\pi_1(i))}, r_3, e_i)$.
- Corruption: All three entities $(s_A, s_B, s_C)$ corrupted with noise $\eta \in \{0.15, 0.30, 0.45\}$.
- Relaxation Steps: $T \in \{1, 2, 4, 8, 16\}$.
- Seeds: 20 independent seeds per condition.
- Metrics:
  - Exact Triad Recovery: all 3 entities simultaneously recovered with $d_H = 0$.
  - Individual Entity Recovery: mean entity accuracy across the 3 slots.
  - Energy Monotonicity: verification that $E(t+1) \le E(t)$.

---

## 5. Execution Plan & Quality Gates

1. **Protocol Preregistration:** Commit this document `EXP_TEN_005_PROTOCOL.md`.
2. **Implementation:**
   - Add new energy potentials ($p=4$, $p=6$, Log-Cosh, Rational, Dual Latent) to `src/models.rs` and `src/learning.rs`.
   - Add cyclic relational constraint network and baseline cascades to `src/models.rs`.
   - Unit tests in `tests/energy_law_tests.rs`.
3. **Execution:**
   - Implement standalone executable benchmark `benchmarks/exp_ten_005.rs`.
   - Run benchmark across all conditions, streaming structured TSV to `EXP_TEN_005_RAW.tsv`.
4. **Analysis & Publication:**
   - Publish full results, tables, statistical significance tests, and phase transitions in `EXP_TEN_005_RESULT.md`.
   - Update `NOVELTY_LEDGER.md`, `NEGATIVE_RESULTS.md`, `ANOMALIES.md`, `RESEARCH_GRAPH.md`, and `NEXT.md`.
