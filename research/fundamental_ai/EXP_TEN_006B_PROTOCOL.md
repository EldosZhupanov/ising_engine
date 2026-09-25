# Experimental Protocol: EXP-TEN-006B
## Rigorous Learned Constraint Dynamics: From Permutation Synchronization to Cross-Topology Factor Graphs

**Experiment ID:** `EXP-TEN-006B`  
**Document ID:** `research/fundamental_ai/EXP_TEN_006B_PROTOCOL.md`  
**Status:** Approved & Frozen for Implementation  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  

---

## 1. Core Scientific Hypothesis & Objectives

Following the falsification in `EXP-TEN-006A-R` (where the candidate energy model exhibited only bash.7\%$ Rescue Rate and was statistically indistinguishable from a zero-interaction baseline), `EXP-TEN-006B` investigates the fundamental question:

> **Core Hypothesis ($):**  
> A shared, local computational rule learned from random initialization (without hardcoded transition tables or oracle knowledge) can perform **active error correction (Net Rescue $> +30\%$)** on loopy constraint systems, scale OOD to unseen graph sizes ( = 16, 32, 64$), and remain competitive with classical Spectral Synchronization, Loopy Min-Sum, and stabilized neural baselines under matched computational budgets.

### Primary Falsification Criteria:
- **Null Hypothesis ($):**  
  No learned local rule (energy, contractive, or message-passing) achieves Net Rescue $> +10\%$ on unseen graph instances; classical Spectral Synchronization (9.4\%$) and Loopy Min-Sum (5.1\%$) remain strictly superior across all parameter-matched settings.
- **Inertia Falsification:**  
  If Net Rescue $\le +5\%$, the model is classified as **INERT** (not reasoning).
- **Contraction Equivalence Falsification:**  
  If a contractive non-energy map ^{(t+1)} = F_\theta(s^{(t)})$ matches scalar energy dynamics, the scalar energy hypothesis is **REDUCED TO CONTRACTION**.

---

## 2. Benchmark Task: Group & Permutation Synchronization

### 2.1 Formal Definition
Let $\mathcal{G} = (V, \mathcal{E})$ be a graph with $|V| = N$ nodes and edge set $\mathcal{E}$.
Let $ be a group with identity $. Each node  \in V$ has an unknown true group element ^* \in G$.
Along each edge  \in \mathcal{E}$, a relative relation measurement {uv} pprox g_v^* (g_u^*)^{-1}$ is observed.

We evaluate across three group families:
1. ** = \mathbb{Z}_2$:** Binary spin parity synchronization ( \in \{-1, +1\}$, {uv} \in \{-1, +1\}$,  = R_{uv} s_u$).
2. ** = \mathbb{Z}_4$:** Modulo-4 cyclic group ( \in \{0, 1, 2, 3\}$ encoded as 2D orthogonal phasors).
3. ** = S_d$ (=16$):** Permutation synchronization (each relation is a permutation $\pi_r \in S_d$).

### 2.2 Graph Distributions
- **F1 (Single Cycle):**  \in \{3, 4, 5, 8, 16, 32, 64\}$.
- **F2 (Intersecting Cycles):** Figures-of-eight and wheel graphs with shared bridging chords.
- **F3 (Random Regular Factor Graphs):** 3-regular Ramanujan graphs with loops and chords.
- **F8 (Frustrated Graphs):** Graphs where a fraction {\text{frust}} \in \{0.10, 0.20\}$ of edge constraints are corrupted to create non-zero holonomy.

---

## 3. Evaluated Architecture Families

All candidate architectures are parameterized by a local shared rule $\theta$ applied uniformly across all nodes/edges:

1. **Family A: Shared Scalar Energy Model (\theta$)**
   460634E_\theta(s; x, \mathcal{G}) = \frac{\lambda_{\text{obs}}}{2} \sum_v \|s_v - x_v\|^2 - \sum_{(u, v) \in \mathcal{E}} \Phi_\theta(s_u, s_v; r_{uv})460634
   where $\Phi_\theta(s_u, s_v; r) = \sum_{k=1}^m \ln\cosh\left( \beta (s_u^T W_k^{(1)}[r] + s_v^T W_k^{(2)}[r]) \right)$.
   Update: ^{(t+1)} = \text{clamp}\left( s_v^{(t)} - \gamma \nabla_{s_v} E_\theta, -1, 1 \right)$.

2. **Family B: Dual-Variable Energy Model (\theta(s, z)$)**
   Includes auxiliary continuous latents  \in \mathbb{R}^m$ per node:
   460634E_\theta(s, z; x, \mathcal{G}) = E_\theta(s) + \frac{1}{2}\|z\|^2 - \sum_{(u, v)} z_u^T U_r z_v460634

3. **Family C: Contractive Non-Energy Model (\theta$)**
   Direct non-conservative update:
   460634s_v^{(t+1)} = (1 - \alpha) s_v^{(t)} + \alpha \tanh\left( \mathcal{M}_\theta(s_v^{(t)}, \{s_u, r_{uv}\}_{u \in N(v)}) \right)460634

4. **Family D: Learned Factor Graph Message Passing (Neural Min-Sum)**
   Parameterized message updates along edges:
   460634m_{u 	o v}^{(t+1)} = (1 - \alpha) m_{u 	o v}^{(t)} + \alpha \text{MLP}_\theta\left( x_u, m_{w 	o u}^{(t)}, r_{uv} \right)460634

---

## 4. Training Regimes: Eliminating Finite-Difference Failures

Finite-difference BPTT is strictly banned as a primary training method.

### 4.1 Analytic Unrolled BPTT (TRAIN-A)
Exact analytic gradients computed via reverse-mode automatic differentiation through unrolled relaxation steps:
460634\frac{\partial \mathcal{L}}{\partial \theta} = \sum_{t=1}^T \frac{\partial \mathcal{L}}{\partial s^{(t)}} \frac{\partial s^{(t)}}{\partial \theta}460634
where $\frac{\partial s^{(t)}}{\partial s^{(t-1)}} = I - \gamma \nabla_s^2 E(s^{(t-1)})$ and $\frac{\partial s^{(t)}}{\partial \theta} = -\gamma \nabla_\theta \nabla_s E(s^{(t-1)})$.

### 4.2 Analytical Equilibrium Propagation (TRAIN-C)
Free phase relaxation to fixed point ^0 = \arg\min_s E_\theta(s; x)$.  
Weakly clamped nudged phase to ^\beta = \arg\min_s \left[ E_\theta(s; x) + \frac{\beta}{2} \|s - s^*\|^2 \right]$.  
Exact gradient update:
460634\Delta \theta = \frac{\eta_{\text{train}}}{\beta} \left( \nabla_\theta E_\theta(s^0) - \nabla_\theta E_\theta(s^\beta) \right)460634

---

## 5. Strong Baselines (Matched Compute & Parameters)

### Classical Solvers:
- **B0 (Identity / No-Op):** Raw noisy input state (=0$).
- **B2 (Spectral Synchronization):** Leading eigenvector relaxation of the block connection matrix $\mathbf{R}$ (Pachauri et al. 2013).
- **B5 (Loopy Min-Sum):** Factor graph message passing on discrete/continuous coordinates.

### Neural Baselines:
- **N0 (Unconstrained Recurrent Attention):** Asymmetric unrolled attention cascade.
- **N1 (Damped Recurrent Attention):** Mann iteration with $\alpha = 0.25$.
- **N4 (Symmetric Energy Attention):** Conservative attention with  \equiv W_K$.

---

## 6. Formal Metrics & Decision Rule

At every step  \in \{0, 1, 2, 4, 8, 16, 32\}$:
1. **Rescue Rate:** $\mathbb{P}(\hat{s}_i^{(t)} = s_i^* \mid x_i 
e s_i^*)$
2. **Damage Rate:** $\mathbb{P}(\hat{s}_i^{(t)} 
e s_i^* \mid x_i = s_i^*)$
3. **Net Rescue:** $\text{Rescue Rate} - \text{Damage Rate}$
4. **Constraint Satisfaction:** Fraction of edges satisfying  = \pi_r(s_u)$.
5. **Distance to Optimum:** $\frac{1}{Nd} \sum_{v, i} |\hat{s}_{v, i} - s_{v, i}^*|$.

### Decision Gate:
1. **Pilot Gate:** In training on short graphs ( \le 8$), if candidate Net Rescue does not exceed **$+10\%* at =8$, halt and diagnose before running large benchmarks.
2. **Benchmark Gate:** Candidate proceeds to claim status only if Net Rescue $> +30\%$ and candidate is competitive with Spectral Synchronization (9.4\%$) and Loopy Min-Sum (5.1\%$) across 3 graph topologies.
