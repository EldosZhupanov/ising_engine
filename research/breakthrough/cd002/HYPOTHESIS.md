---
id: cd002-hypothesis
kind: research-protocol
status: active
authority_scope: candidate-hypothesis
created: 2026-09-19
immutable: false
---

# CD002 Hypothesis: Discrete Gauge Synchronization vs Spectral Rounding in Frustrated Multi-Step Reasoning

## 1. Context & Motivation

In multi-step reasoning architectures, language models generate sequential tokens or thoughts where intermediate entities must maintain global consistency across logical cycles (e.g. variable bindings, entity coreference, or permutation alignments over a domain of size $d$).

Earlier investigation in `research/fundamental_ai/EXP_TEN_006B` proved the **Inertia-Erosion Dilemma**:
- Continuous neural relaxations (EBMs with $\ln\cosh$, continuous Hopfield, contractive GNNs) fail on discrete permutation constraints: they either stall at zero crossings ($\text{Rescue Rate} = 0.0\%$) or diffuse errors symmetrically, damaging clean bits ($\text{Damage Rate} \approx 19\%$).
- Meanwhile, classical discrete algorithms—**Spectral Synchronization** (Pachauri et al. 2013) and **Loopy Min-Sum Belief Propagation**—achieved $+40.1\%$ and $+40.6\%$ net rescue.

## 2. The Tested Hypothesis (H12)

**Hypothesis H12 (Discrete Synchronization Advantage):**
Under adversarial/structured corruptions where cycle holonomy is frustrated (i.e. $\prod_{e \in \text{cycle}} \Pi_e \ne I$) and the spectral gap of the measurement Gram matrix collapses, exact/combinatorial discrete Ising optimization of the non-abelian gauge Hamiltonian recovers the true underlying alignment with strictly higher accuracy than Spectral Synchronization, without suffering the continuous Inertia-Erosion failure.

## 3. Mathematical Mechanism

Let $\mathcal{G} = (V, \mathcal{E})$ be a reasoning graph with nodes $v \in V$ and edges $(u, v) \in \mathcal{E}$.
Each node has a hidden permutation $P_v^* \in S_d$.
Along each edge, we observe a noisy relative permutation measurement $\Pi_{uv} \in S_d$.
Node configurations are binary assignment matrices $X_v \in \{0, 1\}^{d \times d}$ satisfying:
$$\sum_j X_{v, ij} = 1, \quad \sum_i X_{v, ij} = 1 \quad \forall v \in V$$

The discrete Gauge Hamiltonian is:
$$H(X) = - \sum_{(u, v) \in \mathcal{E}} \text{Tr}\left( X_u \Pi_{uv} X_v^T \right)$$

In full unconstrained QUBO form, the row/column stochasticity constraints are added as quadratic penalties:
$$E_{\text{QUBO}}(X) = - \sum_{(u, v) \in \mathcal{E}} \sum_{i, j, k} X_{u, ik} \Pi_{uv, kl} X_{v, il} + \lambda \sum_{v \in V} \left[ \sum_i \left( \sum_j X_{v, ij} - 1 \right)^2 + \sum_j \left( \sum_i X_{v, ij} - 1 \right)^2 \right]$$

## 4. The 5-Step Cross-Domain Protocol

1. **Mechanism Map:** Cross-domain alignment across Physics (Lattice Gauge Theory / Spin Glasses), Computer Vision (Multi-Image Matching), and AI Reasoning (Cycle-Consistent Bindings).
2. **Boundary:** Spectral methods require a spectral gap ($\lambda_1 - \lambda_2 \gg \sigma\sqrt{nd}$); Min-Sum BP oscillates on frustrated loops; continuous EBMs suffer zero-crossing inertia.
3. **Transfer:** Map multi-step reasoning consistency directly to the discrete Gauge Hamiltonian without continuous relaxation.
4. **Prior-Art Attack:** Falsify claims of algorithmic novelty against Pachauri et al. (2013), Lucas (2014), Singer (2011), and Leonardos et al. (2021).
5. **Decisive Kill-Test:** A lightweight, exact Python witness comparing Spectral Sync, Loopy Min-Sum, and Discrete Optimization on frustrated adversarial cycles ($K \in \{3, 4, 5\}$, $d \in \{3, 4\}$).
