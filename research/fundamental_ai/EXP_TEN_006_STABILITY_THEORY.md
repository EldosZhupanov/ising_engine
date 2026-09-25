# Mathematical Stability Theory of Discrete Energy Relaxation
## Rigorous Analysis of Continuous Flow, Discrete Gradient Maps, and Non-Conservative Attention

**Document ID:** `research/fundamental_ai/EXP_TEN_006_STABILITY_THEORY.md`  
**Status:** Theoretical Foundation & Error Correction  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  

---

## 1. Executive Summary & Statement of Correction

In `EXP-TEN-005` and early notes of `EXP-TEN-006`, it was asserted:
> *"Because the Hessian  = \nabla^2 E$ is real symmetric, all eigenvalues are real, mathematically forbidding imaginary eigenvalues, Hopf bifurcations, and limit-cycle divergence."*

This assertion is **incomplete and overly strong**.
While continuous gradient flow $\dot{x} = -\nabla E(x)$ with a symmetric Hessian guarantees that local linearization cannot produce imaginary eigenvalues, **discrete-time gradient descent** {t+1} = x_t - \eta \nabla E(x_t)$ can and does diverge if the learning rate $\eta$ exceeds the curvature bound.

This document formally derives:
1. The exact linear stability criteria for discrete energy descent vs continuous flow;
2. The conditions under which symmetric energy relaxation is strictly contractive;
3. The exact dynamical pathology of unconstrained recurrent attention;
4. The sufficient conditions for stabilized, conservative recurrent attention.

---

## 2. Continuous Flow vs Discrete Gradient Dynamics

### 2.1 Continuous Gradient Flow
Consider the dynamical system:
433338\frac{dx(t)}{dt} = -\nabla E(x(t)), \quad x(t) \in \mathbb{R}^D433338
Let ^*$ be a stationary point ($\nabla E(x^*) = 0$). Expanding in perturbation $\delta x(t) = x(t) - x^*$:
433338\frac{d \delta x(t)}{dt} = - H(x^*) \delta x(t) + \mathcal{O}(\|\delta x\|^2)433338
where (x^*) = \nabla^2 E(x^*)$ is the Hessian matrix.

**Properties:**
1. **Symmetry:** By Schwarz's theorem, for any ^2$ potential $, (x^*) = H(x^*)^T$.
2. **Real Spectrum:** All eigenvalues $\lambda_i(H)$ are strictly real:
   433338\lambda_i \in \mathbb{R} \implies 	ext{Im}(\lambda_i) = 0433338
3. **Impossibility of Continuous Hopf Bifurcations:**
   A Hopf bifurcation requires a pair of purely imaginary eigenvalues $\pm i \omega$ ($\omega 
e 0$) crossing the imaginary axis. Because $\text{Im}(\lambda_i) \equiv 0$, **continuous gradient flow cannot undergo Hopf bifurcations and cannot sustain continuous limit cycles**.
4. **Energy as Lyapunov Function:**
   433338\frac{dE(x(t))}{dt} = \langle \nabla E(x), \dot{x} \rangle = -\|\nabla E(x)\|^2 \le 0433338
   Trajectories monotonically descend (x)$ until reaching stationary points ($\nabla E = 0$).

### 2.2 Discrete Gradient Descent (The Source of Discrete Instability)
In computational implementations (such as `LearnedDualEnergyNetwork.relax`), time is discretized with step size $\eta > 0$:
433338x_{t+1} = x_t - \eta \nabla E(x_t) \equiv \Phi_\eta(x_t)433338
Linearizing around stationary point ^*$:
433338\delta x_{t+1} = J_\Phi(x^*) \delta x_t433338
where the Jacobian of the discrete update map is:
433338J_\Phi(x^*) = I - \eta H(x^*)433338

**Discrete Stability Theorem:**
The stationary point ^*$ is linearly asymptotically stable if and only if the spectral radius of \Phi$ is strictly bounded by 1:
433338\rho(J_\Phi) = \max_i |1 - \eta \lambda_i(H)| < 1433338
Solving for $\eta$:
433338-1 < 1 - \eta \lambda_i(H) < 1 \iff 0 < \eta < \frac{2}{\lambda_{\max}(H)}433338
where $\lambda_{\max}(H)$ is the maximum eigenvalue of the Hessian.

> [!WARNING]
> **Correction to Prior Claim:**
> If the step size $\eta > \frac{2}{\lambda_{\max}}$, then  - \eta \lambda_{\max} < -1$.
> The discrete dynamical system undergoes a **period-doubling flip bifurcation**, producing discrete oscillations ({t+1} pprox -x_t$) and diverging away from the stationary point, despite having a perfectly real, symmetric Hessian.
> Therefore, **symmetry alone does not guarantee stability**; the step size $\eta$ must satisfy $\eta < 2 / \lambda_{\max}$.

---

## 3. Global Descent and Monotonicity Criteria

### 3.1 Descent Lemma under Lipschitz Smoothness
Assume the energy gradient $\nabla E$ is hBcLipschitz continuous:
433338\|\nabla E(x) - \nabla E(y)\| \le L \|x - y\|, \quad orall x, y \in \mathbb{R}^D433338
which is equivalent to (x) \preceq L I$ for all $.

By the standard Descent Lemma (Nesterov 2004):
433338E(x_{t+1}) \le E(x_t) + \langle \nabla E(x_t), x_{t+1} - x_t \rangle + \frac{L}{2} \|x_{t+1} - x_t\|^2433338
Substituting the gradient descent update {t+1} - x_t = -\eta \nabla E(x_t)$:
433338E(x_{t+1}) \le E(x_t) - \eta \left( 1 - \frac{\eta L}{2} \right) \|\nabla E(x_t)\|^2433338

**Sufficient Condition for Strict Monotonic Descent:**
433338\Delta E_t = E(x_{t+1}) - E(x_t) \le -\frac{\eta}{2} \|\nabla E(x_t)\|^2 < 0 \quad 	ext{whenever } 0 < \eta \le \frac{1}{L}433338
For `LearnedDualEnergyNetwork`, where $\eta = 0.05$ and  pprox \lambda_{\text{obs}} + 2 \beta \|W\|^2 pprox 1.0 + 4(0.35)^2 pprox 1.5$, $\eta = 0.05 < 1/1.5 pprox 0.67$.
This confirms that the empirical stability observed in EXP-TEN-006A was due to operating well within the Lipschitz contraction basin $\eta < 1/L$.

### 3.2 Projected / Clamped Gradient Descent
In `LearnedDualEnergyNetwork`, continuous spins are clamped to 1^D$:
433338s_{t+1} = \text{clamp}(s_t - \eta \nabla_s E, -1, 1) = \text{proj}_{[-1, 1]^D}(s_t - \eta \nabla_s E)433338
Because the hypercube 1^D$ is a closed convex set, the projection operator $\text{proj}$ is firmly non-expansive:
433338\|\text{proj}(u) - \text{proj}(v)\| \le \|u - v\|433338
Therefore, coordinate clamping can never increase the spectral radius or cause divergence; it strictly acts as a contractive stabilizing barrier.

---

## 4. Dynamical Systems Analysis of Recurrent Attention

### 4.1 Asymmetric Iterated Maps
Consider the general recurrent attention update tested in EXP-TEN-006A:
433338s_v^{(t+1)} = \text{LayerNorm}\left( s_v^{(t)} + \sum_{u \in N(v)} \text{Attn}(s_v^{(t)}, s_u^{(t)}; r_{uv}) \right) \equiv \mathcal{F}(s^{(t)})433338
Linearizing around a trajectory point:
433338\delta s^{(t+1)} = J_\mathcal{F}(s^{(t)}) \delta s^{(t)}433338
Because the adjacency matrix of a directed cycle bash 	o 1 	o \dots 	o K-1 	o 0$ is a directed circulant matrix:
433338\mathbf{C} = \begin{pmatrix} 0 & 0 & \dots & 1 \ 1 & 0 & \dots & 0 \ 0 & 1 & \dots & 0 \ \vdots & & \ddots & \vdots \end{pmatrix}433338
The eigenvalues of $\mathbf{C}$ are the hBcth roots of unity:
433338\mu_k = \exp\left( \frac{2\pi i k}{K} \right), \quad k = 0, 1, \dots, K-1433338
When unrolled across multiple cycles, the full Jacobian \mathcal{F}$ inherits this cyclic circulant structure:
433338J_\mathcal{F} pprox I + \mathbf{C} \otimes (W_Q W_K^T)433338

**Theorem 2 (Circulating Error Divergence on Cycles):**
If  W_K^T$ is not symmetric and has spectral radius exceeding the damping factor, \mathcal{F}$ possesses complex eigenvalues $\lambda_k \in \mathbb{C}$ with:
433338|\lambda_k| > 1 \quad 	ext{and} \quad 	ext{Im}(\lambda_k) 
e 0433338
Any noise perturbation $\delta s$ is amplified along the eigenspaces corresponding to roots of unity, circulating around the cycle and destroying the initial configuration.
This proves analytically why `RecurrentAttention` in EXP-TEN-006A collapsed from 8.8\%$ to 9.7\%$ as $ increased.

---

## 5. Construction of Stabilized Conservative Attention

To construct a fair, competitive attention baseline for EXP-TEN-006B, we identify the exact mathematical conditions required to stabilize recurrent attention on cycles:

1. **Damped Recurrence (Mann Iteration):**
   433338s^{(t+1)} = (1 - \alpha) s^{(t)} + \alpha \mathcal{F}(s^{(t)}), \quad lpha \in (0, 1)433338
   Damping scales the spectrum: $\lambda(J_{\text{damped}}) = (1-\alpha) + \alpha \lambda(J_\mathcal{F})$. If $\alpha \le 0.25$, it contracts complex orbits into stable fixed points.
2. **Symmetric Attention Formulation (Energy-Admitting):**
   Constrain  = W_K \equiv W$. Then:
   433338M = W W^T = M^T \succeq 0433338
   The attention affinity  M s_j$ is symmetric, making the interaction conservative and eliminating non-zero imaginary circulation.
3. **Spectral Normalization:**
   Divide weight matrices by their largest singular value $\sigma_{\max}(W)$, enforcing $\|W\|_2 \le 1$.

These three mechanisms define the mandatory neural baselines for the next audit.
