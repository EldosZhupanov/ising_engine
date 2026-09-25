# Experimental Protocol: EXP-TEN-006
## Trainable Dual-Variable Energy Networks: Learning Local Constraints for Cyclic Consistency and Structural OOD Scaling

**Experiment ID:** `EXP-TEN-006`  
**Document ID:** `research/fundamental_ai/EXP_TEN_006_PROTOCOL.md`  
**Status:** Frozen & Approved for Execution  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  

---

## 1. Scientific Motivation & Core Hypotheses

`EXP-TEN-005` demonstrated that hand-crafted Modern Trilinear Energy relaxation achieves $99.1\%$ exact recovery on closed triad constraints where recurrent Transformer attention stalls at $85.0\%$ in an oscillating limit cycle.
However, `EXP-TEN-005` used explicit exemplar vectors $(s \cdot \xi^\mu)$ in the energy function.

`EXP-TEN-006` asks the foundational question:
**Can a neural network autonomously LEARN local energy interaction parameters $\theta$ from data and solve unseen cyclic constraint problems through inference-time relaxation, without storing training exemplars and without handcrafted relation potentials?**

### Preregistered Hypotheses:
- **$H_1$ (Structural OOD Size Generalization):**
  When trained *only* on short cycles ($K \in \{3, 4, 5\}$), a learned local energy network $\Psi_\theta(s_u, r, s_v)$ can be transferred directly to solve much larger unseen cycles ($K \in \{8, 16, 32\}$) without any retraining, outperforming GNN and Recurrent Attention by at least $+15\%$ exact recovery at $K=16$.
- **$H_2$ (Inference-Time Compute Scaling on OOD):**
  On unseen cycle sizes ($K \ge 8$), test accuracy scales monotonically with inference relaxation steps:
  $$\text{Acc}(T=1) < \text{Acc}(T=4) < \text{Acc}(T=16) \le \text{Acc}(T=32)$$
  while unrolled Recurrent Attention plateaus early ($T \ge 4$) due to cyclic error amplification.
- **$H_3$ (Limit-Cycle Elimination):**
  Recurrent Attention trajectories on noisy cycles exhibit periodic limit cycles ($\|s_{t+K} - s_t\| \approx 0$ with $\|s_{t+1} - s_t\| > 0$), whereas Learned Energy trajectories converge monotonically to fixed points ($\dot{E} \le 0$).
- **$H_0$ (Null Hypothesis):**
  A learned energy network fails to train or performs no better than standard GNN message passing and unrolled attention under equal parameter count ($\pm 10\%$) and matched inference FLOPs.

---

## 2. Experimental Task: Cyclic Permutation Consistency

### 2.1 The Permutation Relational World
Let each node in a graph $G = (V, \mathcal{E})$ represent a feature vector $s_v \in \mathbb{R}^d$ ($d = 16$).
Let relations $r \in \{1, \dots, R_{\max}\}$ ($R_{\max} = 8$) represent permutations $\pi_r \in S_d$ that shuffle spin coordinates:
$$s_{v+1, i} = s_{v, \pi_r(i)}$$
A closed cycle $v_0 \xrightarrow{r_0} v_1 \xrightarrow{r_1} \dots \xrightarrow{r_{K-1}} v_0$ is mathematically consistent if and only if the composition of permutations along the cycle satisfies the identity on the ground-truth pattern:
$$\pi_{r_{K-1}} \circ \dots \circ \pi_{r_0} = \text{id}$$

### 2.2 Task Conditions
1. **Inputs:** Noisy versions of all node spins $x_v = s_v^{\text{clean}} \odot \epsilon_v$, where $\epsilon_{v, i} = -1$ with probability $\eta \in \{0.10, 0.20, 0.30, 0.40\}$, along with relation identifiers $\{r_e\}$.
2. **Target:** Recover the exact clean state $(s_0^*, s_1^*, \dots, s_{K-1}^*)$.
3. **Evaluation Splits:**
   - **Split A (IID Short):** $K \in \{3, 4, 5\}$, unseen graph instances.
   - **Split B (OOD Size):** $K \in \{8, 16, 32\}$, unseen relations and unseen cycle lengths.
   - **Split C (Frustrated Cycle):** Cycles where one relation is corrupted such that no perfect solution exists; measure minimum constraint violation.

---

## 3. Evaluated Models (Strictly Budget-Matched $\pm 10\%$)

Let parameter budget be fixed to $\approx 2,500$ parameters across all models.

1. **`B1_Recurrent_Attention` (Directed Cascade):**
   - Query-Key-Value projection of node and relation states:
     $$m_v^{(t)} = \text{Softmax}\left( \frac{Q(s_v^{(t)}) K(r_{uv}, s_u^{(t)})^T}{\sqrt{d}} \right) V(s_u^{(t)})$$
     $$s_v^{(t+1)} = \text{LayerNorm}(s_v^{(t)} + \text{MLP}(m_v^{(t)}))$$
   - Parameter count matched to $\approx 2,500$.
2. **`B2_GNN_MessagePassing` (Edge-Conditioned GNN):**
   - Standard Graph Neural Network:
     $$m_{uv} = \text{MLP}_{\text{msg}}(s_u, r_{uv})$$
     $$s_v^{(t+1)} = \text{GRU}(s_v^{(t)}, \sum_u m_{uv})$$
3. **`B3_Direct_Dynamics` (Learned Update Operator without Energy):**
   - Direct field prediction: $s^{(t+1)} = s^{(t)} + \mathcal{F}_\theta(s^{(t)}, x, G)$.
4. **`Candidate_LearnedDualEnergy` (Dual-Variable Energy Network):**
   - Joint Energy Functional:
     $$E_\theta(s, z; x, G) = \frac{\lambda_{\text{obs}}}{2} \sum_v \|s_v - x_v\|^2 + \sum_v \left(\frac{1}{2}\|z_v\|^2 + \alpha \|z_v\|_1\right) - \sum_{(u, v) \in \mathcal{E}} \Psi_\theta(s_u, z_u, r_{uv}, s_v, z_v)$$
   - Local relation potential:
     $$\Psi_\theta = \sum_{k=1}^m \ln \cosh\left( s_u^T W_k^{(1)} r + s_v^T W_k^{(2)} r + z_u^T U_k z_v \right)$$
   - Effective field derived analytically from $\nabla_s E_\theta$.
   - Relaxation steps: $T \in \{1, 2, 4, 8, 16, 32\}$.

---

## 4. Training Procedure & Metrics

### 4.1 Training Regimes
- **EXP-TEN-006A:** Unrolled BPTT (5 unrolled steps during training) with Adam optimizer, batch size 32, learning rate $10^{-3}$, 500 training epochs on short cycles ($K \in \{3, 4, 5\}$).
- **EXP-TEN-006B:** Equilibrium Propagation / Implicit Differentiation training without unrolling history.

### 4.2 Metrics
1. **Exact Graph Solve Rate:** Fraction of graphs where all nodes $v \in V$ are recovered with zero bit errors ($d_H = 0$).
2. **Mean Node Accuracy:** Average cosine similarity across all nodes.
3. **Inference Compute Scaling Slope:** $\Delta \text{Acc} = \text{Acc}(T=16) - \text{Acc}(T=1)$.
4. **Limit Cycle Frequency:** Percentage of trajectories where $\|s_{t+p} - s_t\| < 10^{-4}$ for $p \in \{2, 3, 4\}$ while $\|s_{t+1} - s_t\| > 10^{-2}$.
5. **Monotonic Energy Descent:** Fraction of steps where $\Delta E \le 0$.

---

## 5. Adversarial Quality Gates

1. Parameter count of all models must match within $\pm 10\%$.
2. Inference FLOPs must be reported per step.
3. No model may access ground-truth labels during inference.
4. If `B2_GNN_MessagePassing` or `B1_Recurrent_Attention` matches `Candidate_LearnedDualEnergy` on $K=16$ with equal compute, hypothesis $H_1$ is **FALSIFIED**.
