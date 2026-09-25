# Mathematical Theory: EXP-TEN-006
## Trainable Dual-Variable Energy Networks on Cyclic Factor Graphs

**Document ID:** `research/fundamental_ai/EXP_TEN_006_THEORY.md`  
**Status:** Theoretical Foundation & Derivation  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Mathematical Physicist (`ising_engine`)  

---

## 1. Mathematical Formulation

### 1.1 Configuration and Latent Spaces
Consider a graph of variables $G = (V, \mathcal{E})$ where each node $v \in V$ possesses:
1. An **observable configuration state** $s_v \in \mathbb{R}^d$ (with spins $s_{v, i} \in [-1, +1]$).
2. A **continuous latent state** $z_v \in \mathbb{R}^m$.
3. An optional noisy conditioning input $x_v \in \mathbb{R}^d$.

Each directed edge $e = (u, v) \in \mathcal{E}$ carries a relation code $r_{uv} \in \mathbb{R}^{d_r}$.

### 1.2 The Joint Energy Functional
We define the parameter-shared energy functional:
$$E_\theta(s, z; x, G) = E_{\text{obs}}(s; x) + E_{\text{latent}}(z) + E_{\text{rel},\theta}(s, z; G)$$

where:
1. **Observation / Anchoring Potential:**
   $$E_{\text{obs}}(s; x) = \frac{\lambda_{\text{obs}}}{2} \sum_{v \in V} \|s_v - x_v\|_2^2$$
   Penalizes divergence from observed input $x_v$.
2. **Latent Regularization:**
   $$E_{\text{latent}}(z) = \sum_{v \in V} \left( \frac{1}{2} \|z_v\|_2^2 + \alpha \|z_v\|_1 \right)$$
   Provides quadratic confinement and $L_1$ sparsity on continuous latents.
3. **Local Parameter-Shared Relational Interaction Potential:**
   Instead of storing raw exemplars $\xi^\mu$ (as in Hopfield/DAM), the local interaction is parameterized by trainable tensor weights $\theta$:
   $$E_{\text{rel},\theta}(s, z; G) = \sum_{(u, v) \in \mathcal{E}} \Psi_\theta(s_u, z_u, r_{uv}, s_v, z_v)$$

### 1.3 Parametric Interaction Laws $\Psi_\theta$
To avoid polynomial power overflow (NR-007) and sign erasure (NR-004), we formulate and analyze three candidate interaction potentials:

#### Candidate 1: Bilinear Latent Gauge Potential (Dual Bilinear)
$$\Psi_\theta^{\text{bilinear}}(s_u, z_u, r, s_v, z_v) = - z_u^T W(r) z_v - s_u^T U z_u - s_v^T U z_v$$
where $W(r) = \sum_{k=1}^{d_r} r_k W_k$ is a bilinear relation tensor slice.
- Minima in $z$: $z_v^* = (I)^{-1} (U^T s_v + \sum_{u} W(r_{uv})^T z_u)$.
- Smooth, convex in $z$, quadratic energy.

#### Candidate 2: Saturating Non-Polynomial Relational Potential (Log-Cosh EBM)
$$\Psi_\theta^{\text{logcosh}}(s_u, r, s_v) = -\frac{1}{\beta} \sum_{k=1}^K \ln \cosh \left( \beta \left[ s_u^T U_k s_v + (s_u \odot s_v)^T V_k r \right] \right)$$
- Non-linear gating: derivative is $\tanh(\beta \cdot)$, bounded in $[-1, +1]$.
- Invariant to unbounded energy explosions.

#### Candidate 3: Factorized Gated Trilinear Potential (Candidate Primitive)
Let $q_k(u, r) = (U_k s_u) \odot (R_k r)$ be the relational query vector for slot $k$.
$$\Psi_\theta^{\text{gated}}(s_u, r, s_v) = - \sum_{k=1}^K \phi\left( \frac{q_k(u, r) \cdot (V_k s_v)}{\sqrt{d}} \right)$$
where $\phi(y) = \ln(1 + e^{\beta y}) / \beta$ (softplus) or $\ln \cosh(\beta y)$.

---

## 2. Dynamics & Relaxation

### 2.1 Gradient Fields
The relaxation dynamics follow continuous gradient descent on the joint energy landscape:
$$\tau_s \frac{d s_v}{d t} = - \nabla_{s_v} E_\theta(s, z; x, G)$$
$$\tau_z \frac{d z_v}{d t} = - \nabla_{z_v} E_\theta(s, z; x, G)$$

For discrete spin updates ($s_{v, i} \in \{-1, +1\}$), the local effective field is:
$$h_{v}(s, z) = - \nabla_{s_v} E_\theta(s, z) = \lambda_{\text{obs}} x_v + \sum_{u \in \mathcal{N}_{\text{in}}(v)} \nabla_{s_v} \Psi_\theta(s_u, r_{uv}, s_v) + \sum_{w \in \mathcal{N}_{\text{out}}(v)} \nabla_{s_v} \Psi_\theta(s_v, r_{vw}, s_w)$$
And discrete updates flip spins according to:
$$s_{v, i}^{(t+1)} = \text{sign}\left( h_{v, i}(s^{(t)}, z^{(t)}) \right)$$

### 2.2 Proof of Monotonic Lyapunov Descent
**Theorem 1 (Monotonic Energy Descent):**
For any symmetric relational potential $\Psi(u, v) = \Psi(v, u)$ or continuous relaxation with step size $\gamma \le \frac{2}{L_{\max}}$ where $L_{\max} = \|\nabla^2 E\|_2$:
$$\frac{d E_\theta}{d t} = \sum_{v \in V} \left( \nabla_{s_v} E \cdot \dot{s}_v + \nabla_{z_v} E \cdot \dot{z}_v \right) = - \sum_{v \in V} \left( \frac{1}{\tau_s} \|\nabla_{s_v} E\|^2 + \frac{1}{\tau_z} \|\nabla_{z_v} E\|^2 \right) \le 0$$
Equality holds if and only if the system has reached a critical point (local minimum or saddle point).

**Corollary 1.1 (Impossibility of Periodic Limit Cycles):**
Because $E_\theta(s, z)$ is bounded below (by quadratic regularization on $z$ and compact bounds on $s$) and $\frac{d E}{d t} \le 0$, the autonomous dynamical system possesses **no periodic orbits, non-trivial limit cycles, or strange attractors**.
Every trajectory converges to a stationary equilibrium point $\lim_{t \to \infty} (s(t), z(t)) = (s^*, z^*)$.

---

## 3. Why Directed Attention Stalls: The Asymmetric Limit Cycle Theorem

### 3.1 Directed Attention as an Asymmetric Iterated Map
In a cyclic graph $v_0 \to v_1 \dots \to v_{K-1} \to v_0$, unrolled directed attention applies:
$$s_{v+1}^{(t+1)} = \mathcal{T}_\theta(s_v^{(t)}, r_{v, v+1})$$
The global Jacobian matrix $J = \frac{\partial s^{(t+1)}}{\partial s^{(t)}}$ has block cyclic structure:
$$J = \begin{pmatrix} 0 & 0 & \dots & J_{K-1, 0} \\ J_{0, 1} & 0 & \dots & 0 \\ 0 & J_{1, 2} & \dots & 0 \\ \vdots & & \ddots & \vdots \end{pmatrix}$$
Because $J$ is asymmetric and cyclic, its eigenvalues $\lambda_k$ lie on a circle in the complex plane:
$$\lambda_k = \rho e^{i 2\pi k / K}$$
When $\rho \approx 1$ (which occurs when attention is confident but noisy), the system undergoes a **Hopf bifurcation**, producing stable limit cycles of period $K$.
Any error introduced at node $v$ circulates with phase shift $2\pi/K$, endlessly oscillating and preventing convergence.

### 3.2 Symmetrical Energy Relaxation as Limit-Cycle Destroyer
In the Energy Network, the effective Jacobian is the Hessian of the scalar potential:
$$J_{\text{energy}} = - H = - \nabla^2 E_\theta$$
Because $H = H^T$ is **strictly real and symmetric**, all its eigenvalues are **strictly real**:
$$\lambda_k(H) \in \mathbb{R}$$
The imaginary part is identically zero ($\text{Im}(\lambda) = 0$).
Therefore, **Hopf bifurcations are mathematically impossible** in the energy dynamics. Oscillatory limit cycles cannot exist.

---

## 4. Equilibrium Training: Backpropagation Through Equilibrium

To train $\theta$ without storing full unrolled trajectories $T \in \{1, \dots, 32\}$, we apply the **Implicit Function Theorem at Equilibrium**:
At equilibrium $(s^*, z^*)$, the stationary condition holds:
$$\nabla_s E_\theta(s^*, z^*; x) = 0, \quad \nabla_z E_\theta(s^*, z^*; x) = 0$$
Let $\mathcal{L}(s^*, y)$ be the loss with respect to ground-truth clean state $y$.
The exact total gradient with respect to parameter $\theta$ is:
$$\frac{d \mathcal{L}}{d \theta} = \frac{\partial \mathcal{L}}{\partial s^*} \left( - H_{s^*, s^*}^{-1} \frac{\partial^2 E_\theta}{\partial s^* \partial \theta} \right)$$
Alternatively, in **Equilibrium Propagation (Scellier & Bengio 2017)**:
1. **Free Phase:** Relax network to equilibrium $(s^0, z^0) = \arg\min E_\theta(s, z; x)$.
2. **Nudged Phase:** Add loss penalty to energy $E_\beta = E_\theta + \beta \mathcal{L}(s, y)$, and relax to nudged equilibrium $(s^\beta, z^\beta)$.
3. **Parameter Update:**
   $$\nabla_\theta \mathcal{L} = \lim_{\beta \to 0} \frac{1}{\beta} \left( \frac{\partial E_\theta(s^\beta, z^\beta)}{\partial \theta} - \frac{\partial E_\theta(s^0, z^0)}{\partial \theta} \right)$$
This update is **strictly local** and requires $O(1)$ memory with respect to inference depth $T$.
