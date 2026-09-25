# Experimental Protocol: EXP-TEN-004
## Relational Compositional Generalization: Inference-Time Computation via Energy Relaxation

**Experiment ID:** `EXP-TEN-004`  
**Document ID:** `research/fundamental_ai/EXP_TEN_004_PROTOCOL.md`  
**Status:** Frozen & Approved for Execution  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Reviewer (`ising_engine`)  

---

## 1. Scientific Objective & Hypotheses

Previous experiments (`EXP-TEN-001`, `EXP-TEN-002`, `EXP-TEN-003`) proved that:
1. Higher-order polynomial memory with $R=P$ is identical to degree-3 Polynomial DAM and collapses under correlation.
2. Odd-degree ($p=3$) homogeneous latent factor models suffer from the Sign-Erasure Pathology.
3. Exemplar retrieval models (1-NN, Modern Hopfield) are fundamentally incapable of out-of-distribution compositional generalization.

`EXP-TEN-004` investigates whether **higher-order tensor energy relaxation** can serve as an **inference-time computational primitive** for relational reasoning on systematically held-out compositions.

### Core Question:
Does higher-order energy relaxation allow the network to dynamically trade compute for accuracy at inference time ($T \in \{1, 2, 4, 8, 16\}$), solving multi-hop relational compositions ($A \xrightarrow{R_1} B \xrightarrow{R_2} C$) where the composite query $(A, R_1 \circ R_2, ?)$ is **strictly absent** from training memory?

### Hypotheses:
- **$H_1$ (Inference-Time Compute Scaling):** In an energy relaxation chain with an unclamped intermediate bridge variable $s_X$, test accuracy on held-out 2-hop compositions scales monotonically with relaxation steps $T$:
  $$\text{Acc}(T=1) \ll \text{Acc}(T=2) < \text{Acc}(T=4) \to 100\%$$
- **$H_2$ (Trilinear Binding Superiority over Pairwise):** 3-body trilinear coupling $T(A, R, B)$ eliminates the pairwise binding crosstalk / superposition catastrophe that corrupts pairwise models ($W_{AR} + W_{RB} + W_{AB}$), maintaining high fidelity as the number of relations and entities scales.
- **$H_0$ (Static Retrieval Null):** An energy model cannot solve multi-hop compositions better than a 2-step cascaded 1-NN oracle or simple feedforward association.

---

## 2. Experimental Design & Relational Microworld

### 2.1 The Relational Microworld: $\mathcal{G} = (\mathcal{E}, \mathcal{R}, \mathcal{F})$
- **Entities:** $M_E = 32$ distinct entities $\{e_1, \dots, e_{M_E}\}$, each encoded as an orthogonal or pseudo-orthogonal spin vector $s_{e} \in \{-1, +1\}^{N_E}$ ($N_E = 64$).
- **Relations:** $M_R = 4$ distinct relations $\{r_1, \dots, r_{M_R}\}$, each encoded as a spin vector $s_r \in \{-1, +1\}^{N_R}$ ($N_R = 32$).
- **Knowledge Base (1-Hop Facts):**
  - $\mathcal{F}_1 = \{(e_i, r_1, e_{\pi_1(i)})\}$ where $\pi_1$ is a permutation on $\mathcal{E}$.
  - $\mathcal{F}_2 = \{(e_i, r_2, e_{\pi_2(i)})\}$ where $\pi_2$ is a permutation on $\mathcal{E}$.
  - Total 1-hop training facts: $|\mathcal{F}_{\text{train}}| = 2 \times M_E = 64$ facts.
- **Held-Out 2-Hop Test Suite (Strictly Unseen):**
  - Composition: $r_{12} = r_1 \circ r_2$.
  - Target facts: $\mathcal{F}_{\text{test}} = \{(e_i, r_{12}, e_{\pi_2(\pi_1(i))})\}$.
  - **Zero Shot Invariant:** No composite fact $(e_i, r_{12}, e_k)$ is EVER present in the training set or model parameters!

### 2.2 Inference Architecture: The Energy Relaxation Chain
To answer a 2-hop query $e_i \xrightarrow{r_1} ? \xrightarrow{r_2} e_k$:
The network state is partitioned into 5 slots:
$$s = [s_A ; s_{R1} ; s_X ; s_{R2} ; s_C]$$
where:
- $s_A$ is clamped to entity $e_i$ ($N_E$ spins).
- $s_{R1}$ is clamped to relation $r_1$ ($N_R$ spins).
- $s_X$ is **unclamped and initialized to random noise** ($N_E$ spins).
- $s_{R2}$ is clamped to relation $r_2$ ($N_R$ spins).
- $s_C$ is **unclamped and initialized to random noise** ($N_E$ spins).

The system energy is:
$$E(s_X, s_C) = E_{\text{hop1}}(s_A, s_{R1}, s_X) + E_{\text{hop2}}(s_X, s_{R2}, s_C)$$

### 2.3 Evaluated Models & Baselines
1. **`Candidate_TrilinearTensorChain`:**
   Each hop uses 3-body trilinear coupling:
   $$E_{\text{trilinear}}(s_A, s_R, s_B) = -\frac{1}{N_E^2 N_R} \sum_{\mu=1}^P (s_A \cdot \xi_A^\mu) (s_R \cdot \xi_R^\mu) (s_B \cdot \xi_B^\mu)$$
   The effective fields on unclamped variables are:
   $$h_X = \sum_{\mu} \xi_X^\mu (s_A \cdot \xi_A^\mu)(s_{R1} \cdot \xi_{R1}^\mu) + \sum_{\nu} \xi_X^\nu (s_C \cdot \xi_C^\nu)(s_{R2} \cdot \xi_{R2}^\nu)$$
   $$h_C = \sum_{\nu} \xi_C^\nu (s_X \cdot \xi_X^\nu)(s_{R2} \cdot \xi_{R2}^\nu)$$
2. **`B0_PairwiseHopfieldChain`:**
   Standard pairwise Hopfield couplings between slots:
   $$E_{\text{pairwise}} = - s_A^T W_{AR1} s_{R1} - s_A^T W_{AX} s_X - s_{R1}^T W_{R1X} s_X - s_X^T W_{XR2} s_{R2} - s_X^T W_{XC} s_C - s_{R2}^T W_{R2C} s_C$$
3. **`B1_Cascaded_1NN`:**
   Two-stage feedforward oracle: $\hat{X} = \text{1NN}(A, R_1)$, then $\hat{C} = \text{1NN}(\hat{X}, R_2)$.
4. **`B2_Direct_1NN` (Flat Baseline):**
   Attempts to match $(A, R_{12})$ directly in training database. Bound to $0.0\%$ by definition.
5. **`B3_Cascaded_ModernHopfield`:**
   Two-stage continuous softmax attention.

### 2.4 Inference Steps & Conditions
- Evaluation steps: $T \in \{1, 2, 4, 8, 16\}$.
- Input corruption noise on query $(s_A, s_{R1}, s_{R2})$: $\eta \in \{0.0, 0.15, 0.30\}$.
- 20 independent seeds.

---

## 3. Metrics & Independent Adversarial Review

1. **Exact 2-Hop Accuracy:** % of held-out test queries where final $s_C$ matches $e_{\pi_2(\pi_1(i))}$ with zero bit errors ($d_H = 0$).
2. **Intermediate Variable Settling Accuracy:** % of queries where $s_X$ settles exactly to the true latent entity $e_{\pi_1(i)}$.
3. **Inference Compute Scaling Curve:** Plot of accuracy vs $T$.
4. **Adversarial Gate:**
   - If `Candidate_TrilinearTensorChain` accuracy at $T=1$ is equal to $T=8$, it is NOT doing iterative energy relaxation.
   - If `Candidate_TrilinearTensorChain` cannot tolerate noise on $A$ better than `B0_PairwiseHopfieldChain`, trilinear binding provides no robustness advantage.
