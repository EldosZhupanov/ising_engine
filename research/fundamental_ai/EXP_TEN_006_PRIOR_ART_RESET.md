# Prior-Art Reset & Adversarial Literature Audit: EXP-TEN-006
## Formal Algebraic Deconstruction of Cyclic Constraint Energy Dynamics

**Document ID:** `research/fundamental_ai/EXP_TEN_006_PRIOR_ART_RESET.md`  
**Status:** Canonical Scientific Reset  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  

---

## 1. Motivation & Governance

Following the empirical findings of `EXP-TEN-006A`, this document performs a mandatory, exhaustive mathematical audit against four established foundational literature families:
1. **Permutation & Group Synchronization** (Pachauri et al. 2013; Singer 2011; DeepCORD 2026);
2. **Loopy Belief Propagation & Factor Graph Inference** (Pearl 1988; Yedidia et al. 2003; Satorras et al. 2021);
3. **Deep Equilibrium Models & Implicit Fixed-Point Reasoning** (Bai et al. 2019; Gao et al. 2023);
4. **Looped / Recurrent Transformers & Conservative Self-Attention** (NeurIPS 2025; Energy-Based Transformers 2025).

The explicit objective is to test whether the cyclic constraint problem and energy relaxation proposed in EXP-TEN-006 are algebraically reducible to established prior art, and to eliminate unsubstantiated claims of novelty.

---

## 2. Family A: Permutation & Group Synchronization

### 2.1 Formal Algebraic Mapping
In the mathematical literature on **Group Synchronization** (Singer 2011; Pachauri, Kondor & Singh, NIPS 2013):
- Let $ be a group (compact Lie group like (3)$ or finite group like the symmetric group $).
- Let each node  \in V$ in a graph $\mathcal{G} = (V, \mathcal{E})$ have an unknown ground-truth group element ^* \in G$.
- Along each directed edge  \in \mathcal{E}$, we observe a noisy relative group measurement:
  433065R_{uv} pprox g_v^* (g_u^*)^{-1}433065
- For any cycle  	o v_1 	o \dots 	o v_{K-1} 	o v_0$, exact mathematical consistency (zero holonomy / curvature) is defined by the **cycle consistency condition**:
  433065R_{v_{K-1}, v_0} \dots R_{v_1, v_2} R_{v_0, v_1} = e_G433065
  where $ is the identity element of $.

### 2.2 Direct Mapping to EXP-TEN-006
In `PermutationWorld` of EXP-TEN-006:
- Group:  = S_d$ (the symmetric group on  = 16$ elements, represented by permutation matrices $\Pi \in \mathbb{R}^{d 	imes d}$).
- State of node $:  \in \{-1, +1\}^d$. The ground-truth state is ^* = \Pi_v^* s_0^*$, where $\Pi_v^* \in S_d$ acts on a base feature vector ^*$.
- Edge relation {uv}$: specifies the relative permutation $\pi_r \in S_d$ mapping  	o s_v$:
  433065s_v = \pi_r(s_u) \iff s_v = \Pi_r s_u433065
- Therefore, the edge relation is identically the relative measurement:
  433065\Pi_r = \Pi_v^* (\Pi_u^*)^{-1}433065
- The cyclic constraint $\pi_{r_{K-1}} \circ \dots \circ \pi_{r_0} = 	ext{id}$ is identically the standard **cycle-consistency constraint** in permutation synchronization.

> [!CAUTION]
> **Definitive Reduction Verdict:**
> **EXP-TEN-006A TASK = SPECIAL CASE OF PERMUTATION / GROUP SYNCHRONIZATION OVER $.**
> The problem of recovering consistent node configurations from noisy states under cyclic permutation relations is algebraically isomorphic to the classical **Permutation Synchronization Problem** introduced by Pachauri, Kondor & Singh (NIPS 2013).

### 2.3 Existing Solvers in Synchronization Literature
The permutation synchronization community has extensively studied and solved this problem with guaranteed recovery bounds:
1. **Spectral Permutation Synchronization (Pachauri et al. 2013):**
   Constructs the block matrix $\mathbf{R} \in \mathbb{R}^{nd 	imes nd}$ where block $ is $\Pi_{uv}$. The top $ eigenvectors of $\mathbf{R}$ provide an exact continuous spectral relaxation of the node group elements, which are then rounded to valid permutation matrices via the Hungarian algorithm or Birkhoff polytope projection.
2. **Iteratively Reweighted Least Squares (IRLS) Synchronization (Wang & Singer 2013; Chatterjee & Govindu 2013):**
   Handles heavy outlier noise and corrupted edges via robust loss functions $\rho(R_{uv} g_u g_v^{-1})$.
3. **Cycle-Edge Message Passing (CEMP) (Leonardos et al. 2021):**
   Passes messages along triangles and cycles to identify inconsistent/frustrated edges and filter corrupted measurements.
4. **Learning Transformation Synchronization (2019) / Learning Iterative Robust Synchronization (2021):**
   Trains neural networks to predict reweighting factors for edge consistency.
5. **DeepCORD (2026):**
   Learns distributed solvers for factor graph optimization directly on matrix Lie groups.

**Conclusion for EXP-TEN-006:**
Any claim that our cyclic constraint task represents a novel AI domain is **FALSIFIED**. It is permutation synchronization. However, exploring whether a *parameter-shared local energy potential* can solve permutation synchronization without building the global  	imes nd$ Gram matrix is an open and valid question.

---

## 3. Family B: Loopy Belief Propagation & Factor Graph Inference

### 3.1 Factor Graph Formulation
The cyclic permutation consistency problem can be formulated as maximum a posteriori (MAP) inference on a factor graph:
433065P(s | x) \propto \exp(-E(s; x)) = \prod_{v \in V} \psi_v(s_v; x_v) \prod_{(u, v) \in \mathcal{E}} \psi_{uv}(s_u, s_v; r_{uv})433065
where:
- $\psi_v(s_v; x_v) = \exp\left(-\frac{\lambda_{\text{obs}}}{2} \|s_v - x_v\|^2\right)$ (unary observation potential);
- $\psi_{uv}(s_u, s_v; r_{uv}) = \exp\left(\beta s_v^T \Pi_{r_{uv}} s_u\right)$ (pairwise constraint potential).

### 3.2 Min-Sum / Max-Product Equivalence
In classical probabilistic graphical models:
- **Min-Sum Algorithm:** Computes min-marginals via recursive message passing:
  433065m_{u 	o v}(s_v) = \min_{s_u} \left[ \phi_{uv}(s_u, s_v) + \phi_u(s_u) + \sum_{w \in N(u) \setminus \{v\}} m_{w 	o u}(s_u) ight]433065
- On loopy graphs (such as cycles), standard BP is not guaranteed to converge, frequently exhibiting limit cycles or oscillatory non-convergence.
- **Continuous Proximal Relaxation:**
  If spin states  \in \{-1, +1\}^d$ are relaxed to the continuous hypercube 1^d$, MAP inference becomes continuous energy minimization:
  433065\min_{s \in [-1, 1]^{nd}} E(s) = \sum_v \frac{\lambda_{\text{obs}}}{2} \|s_v - x_v\|^2 - \sum_{(u, v) \in \mathcal{E}} s_v^T \Pi_{r_{uv}} s_u433065
- **Gradient Update:**
  433065s_v^{(t+1)} = 	ext{proj}_{[-1, 1]^d} \left( s_v^{(t)} - \gamma 
abla_{s_v} E(s^{(t)}) ight)433065
  where $\nabla_{s_v} E = \lambda_{\text{obs}}(s_v - x_v) - \sum_{u \in N_{\text{in}}(v)} \Pi_{uv} s_u - \sum_{w \in N_{\text{out}}(v)} \Pi_{wv}^T s_w$.

> [!IMPORTANT]
> **Equivalence Result:**
> The continuous energy relaxation in EXP-TEN-006 is functionally a **continuous projected gradient descent on the Bethe-like free energy of a cyclic factor graph**.
> When relations are known, this is continuous MAP relaxation; when potentials are parameterized by $\theta$, it is equivalent to a **Factor Graph Neural Network (FGNN)** or **Neural Enhanced Belief Propagation (NEBP)** performing continuous inference.

---

## 4. Family C: Deep Equilibrium Models & Implicit Reasoning

### 4.1 Fixed-Point Formulation
In **Deep Equilibrium Models (DEQ)** (Bai, Kolter & Koltun 2019; Gao et al. 2023):
Inference is defined as the fixed point of a non-linear mapping:
433065x^* = F_\theta(x^*; \text{input})433065
In `LearnedDualEnergyNetwork`, inference is defined as the stationary point of energy descent:
433065s^* = 	ext{clamp}\left( s^* - \gamma 
abla_s E_\theta(s^*, z^*; x), -1, 1 \right)433065
433065z^* = z^* - \gamma 
abla_z E_\theta(s^*, z^*; x)433065
Letting  = (s, z)$, this can be written identically as a fixed-point equation:
433065y^* = \mathcal{T}_\gamma(y^*), \quad \mathcal{T}_\gamma(y) \equiv 	ext{proj}(y - \gamma 
abla_y E_\theta(y))433065

### 4.2 What Is Actually Different Between DEQ and Energy Relaxation?
| Dimension | General DEQ (Bai et al. 2019) | Energy Relaxation (EXP-TEN-006) |
|---|---|---|
| **Operator Structure** | Arbitrary neural network \theta(y)$ | Conservative gradient vector field: \theta(y) = -\nabla_y E_\theta(y)$ |
| **Jacobian Symmetry** | Asymmetric: $\text{Jac}(F) \ne \text{Jac}(F)^T$ | Symmetric (Hessian): $\text{Jac}(\nabla E) = \nabla^2 E = (\nabla^2 E)^T$ |
| **Continuous Eigenvalues** | Complex: $\lambda \in \mathbb{C}$ (permits limit cycles & chaos) | Real: $\lambda \in \mathbb{R}$ (no imaginary component) |
| **Lyapunov Function** | None in general; requires custom contractivity regularizer | Exact scalar potential \theta(y)$ acts as native Lyapunov function |
| **Convergence Guarantee** | Requires spectral radius $\rho(\text{Jac}) < 1$ | Monotonic descent $\dot{E} \le 0$ under continuous flow |

**Conclusion:**
Our energy model is a **Conservative / Potential Deep Equilibrium Model**. It is not a distinct computational universe from DEQs; it is the *integrable, conservative Hamiltonian/gradient subclass* of implicit fixed-point models.

---

## 5. Family D: Recurrent / Looped Transformers & Conservative Attention

### 5.1 The Myth of Non-Conservative Attention
A common claim in early neural EBM discussions is that *"Self-Attention cannot have a Lyapunov or energy function because attention weights are directed."*
**This claim is mathematically false.**

Recent theoretical literature (NeurIPS 2025; Energy-Based Transformers 2025) and Modern Hopfield Networks (Ramsauer et al. 2020) prove that **Self-Attention admits an exact scalar Lyapunov energy under specific algebraic symmetry constraints**:

#### Theorem 1 (Energy Formulation of Self-Attention):
Consider a single-layer self-attention map:
433065A(X) = 	ext{softmax}\left( rac{X W_Q W_K^T X^T}{\sqrt{d}} ight) X W_V433065
If the following conditions are satisfied:
1. **Bilinear Matrix Symmetry:**  \equiv W_Q W_K^T$ is symmetric ( = M^T$).
2. **Tied Value Projection:**  = I$ (or aligned with $).
3. **Log-Sum-Exp Potential:**
   Then the attention update is the exact gradient of the Modern Hopfield energy:
   433065E(X) = -\sum_i 	ext{LogSumExp}_j \left( rac{X_i M X_j^T}{\sqrt{d}} ight) + rac{1}{2} \|X\|_F^2433065
   433065\nabla_{X_i} E(X) = X_i - A(X)_i433065
   The residual update {t+1} = X_t - \gamma 
abla E(X_t) = (1-\gamma)X_t + \gamma A(X_t)$ is **exact gradient descent on a scalar Lyapunov function**.

### 5.2 Why the EXP-TEN-006A Recurrent Attention Baseline Failed
In `EXP-TEN-006A`, the baseline `RecurrentAttentionGraph` was implemented as:
433065W_Q, W_K, W_V \sim 	ext{i.i.d. random matrices}433065
433065M = W_Q W_K^T 
e M^T \quad (\text{asymmetric})433065
433065s_v^{(t+1)} = 	ext{LayerNorm}(s_v^{(t)} + 	ext{Attn}(s_v^{(t)}))433065
1. The projection matrix  W_K^T$ was completely unconstrained and asymmetric.
2. The recurrence had **zero damping** ($\alpha = 1.0$), forcing full step updates.
3. LayerNorm added a non-conservative spherical projection.

Therefore, the hBc19.1\%$ collapse of `RecurrentAttention` in EXP-TEN-006A proved only that an **unconstrained, undamped, asymmetric recurrent update diverges on cyclic graphs**.
It did **NOT** prove that recurrent attention *in general* cannot solve cyclic graphs. A symmetric, damped attention network ( = M^T, \alpha \le 0.5$) is conservative and would avoid cyclic error divergence.

---

## 6. Synthesis & Novelty Matrix

| Architecture Component | Prior Art Source | Algebraic / Conceptual Overlap | What Remains Distinct in Our Proposal | Novelty Classification |
|---|---|---|---|:---:|
| **Cyclic Permutation Constraint Task** | Pachauri, Kondor & Singh (NIPS 2013) | Exact isomorphism to Permutation Synchronization over $ | Local parameter-shared inference without  	imes nd$ Gram matrix | **KNOWN PROBLEM** |
| **Iterative Relaxation on Graph** | Loopy BP / Min-Sum / FGNN (Yedidia 2003) | Continuous gradient descent on factor graph energy | Explicit dual latent variables $ coupling observable spins | **KNOWN + USEFUL** |
| **Implicit Fixed-Point Inference** | DEQ (Bai et al. 2019), Implicit GNNs | ^* = rg\min E_\theta(x) \leftrightarrow x^* = F_\theta(x^*)$ | Restriction to symmetric Jacobian $\nabla^2 E = (\nabla^2 E)^T$ | **DERIVED EXTENSION** |
| **Stabilized Cyclic Recurrence** | Fully Looped Transformer (2026), Damped Attention | Cyclic stability achieved via contractive damping & residual coupling | Energy formulation guarantees Lyapunov descent naturally | **KNOWN ALTERNATIVE** |
| **Dual Latent Variable Energy (, z$)** | Dual EBMs, Hopfield dual variables (Krotov 2021) | Observable-latent coupling with non-polynomial potentials | Joint factor graph potential learned end-to-end | **POSSIBLY NOVEL COMBINATION** |

---

## 7. Directives for EXP-TEN-006B

1. **Retract Overclaims:** EXP-TEN-006A did not discover a "new computational primitive"; it observed that conservative energy relaxation avoids the cyclic circulation divergence of unconstrained asymmetric recurrent attention.
2. **Mandatory Classical Baselines:** EXP-TEN-006B must include Spectral Permutation Synchronization (Pachauri 2013) and Loopy Min-Sum Message Passing.
3. **Mandatory Stabilized Neural Baselines:** EXP-TEN-006B must include Damped Recurrent Attention ({t+1} = (1-\alpha)x_t + \alpha F(x_t)$) and Symmetric Conservative Attention.
4. **Mandatory Operational Audit (=0$ & Rescue Rate):** Test whether `LearnedDualEnergy` actively corrects errors (Rescue Rate $> 0$) or merely possesses inert stability.
