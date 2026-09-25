# Formal Novelty Review & Adversarial Teardown: EXP-TEN-006
## Rigorous Falsification of Candidate Claims and Prior-Art Synchronization Audit

**Document ID:** `research/fundamental_ai/EXP_TEN_006_NOVELTY_REVIEW.md`  
**Status:** Canonical Scientific Review  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  
**Audit Data:** `research/fundamental_ai/EXP_TEN_006_AUDIT_RAW.tsv` (4,410 recorded evaluations)

---

## 1. Executive Scientific Verdict: The "Inertia Illusion" Falsification

The critical audit `EXP-TEN-006A-R` conclusively falsifies the hypothesis that `LearnedDualEnergyNetwork` was performing intelligent relational reasoning on cyclic topologies.

### Definitive Empirical Proof:
1. **The Model Was Not "Thinking" (Rescue Rate $\approx 0\%$):**
   - Under initial noise $\eta = 0.20$, the query state starts at an accuracy of **9.7\%*.
   - As relaxation steps increase from =0$ to =16$, `Candidate_LearnedEnergy` reaches **9.6\%*.
   - **Rescue Rate:** Only **bash.7\%* of corrupted bits were corrected.
   - **Damage Rate:** **bash.3\%* of correct bits were corrupted.
   - **Net Rescue:** **$+0.4\%* (statistically indistinguishable from zero).
2. **Ablation Equivalence to Zero-Interaction Energy ( = 1.000$):**
   - An ablation network with **all learned interaction weights set to zero** (`Ablation_ZeroInteraction`, $\Psi \equiv 0$, retaining only observational anchoring $\lambda_{\text{obs}}$) achieves **9.7\%* accuracy at =16$.
   - Paired bootstrap test between Candidate and Zero Interaction:
     440332\Delta \text{Acc} = -0.10\% \quad [95\% \text{ CI: } -0.23\%, +0.04\%], \quad \text{McNemar } \chi^2 = 0.00, \; p = 1.0000440332
   - The learned relation potential $\Psi_\theta$ was completely inert. The candidate network appeared "stable" only because it was frozen near the initial input.
3. **The "Breakthrough" Over Attention Was an Artifact of a Fragile Baseline:**
   - In `EXP-TEN-006A`, the apparent victory of energy (9.6\%$) over `RecurrentAttention` (2.9\%$) was merely:
     *An inert frozen model staying at 0\%$ beats an undamped, asymmetric recurrent model that actively circulates errors and degrades to 9\%$.*
4. **Classical Synchronization Solves What Neural Attention and Inert Energy Could Not:**
   - **Classical Spectral Permutation Synchronization** (Pachauri, Kondor & Singh, NIPS 2013) actively repairs the cycle, climbing from 9.7\%$ to **9.4\%* (Rescue Rate: **5.0\%*, Net Rescue: **$+53.2\%*,  = 0.0089$ over Candidate).
   - **Loopy Min-Sum Belief Propagation** climbs from 9.7\%$ to **5.1\%* (Rescue Rate: **5.0\%*, Net Rescue: **$+82.7\%*).

---

## 2. Comprehensive Component-Level Novelty Matrix

| Component | Closest Known Prior Art | Mathematical Overlap | Experimental Overlap | Key Functional Difference | Epistemic Novelty Status |
|---|---|---|---|---|:---:|
| **1. Cyclic Permutation Constraints** | Pachauri, Kondor & Singh (*Permutation Synchronization*, NIPS 2013); Singer (2011) | **100% Identical:** {uv} pprox g_v g_u^{-1}$ on =S_d$; cycle holonomy $\prod R = e$. | Synchronization on synthetic graphs and 3D point clouds | None. Our synthetic task is identically Permutation Synchronization. | **NOT NOVEL** |
| **2. Global Scalar Energy Function** | Hopfield (1982); LeCun et al. (*A Tutorial on Energy-Based Learning*, 2006) | Joint Gibbs/Bethe free energy (s, z; x)$ | Continuous relaxation of MRFs and Hopfield nets | Parameterized non-polynomial potentials ($\ln\cosh$) | **KNOWN + USEFUL** |
| **3. Bidirectional Relaxation** | Loopy Belief Propagation (Pearl 1988); Bethe Free Energy minimization (Yedidia 2003) | Gradient descent on Bethe-like continuous factor graph | Message passing and variational MAP inference | Continuous coordinate updates instead of discrete messages | **KNOWN + USEFUL** |
| **4. Dual Latent Variables (, z$)** | Dual EBMs; Continuous Hopfield Duals (Krotov 2021); Auxiliary MRF variables | Legendre-Fenchel transformation coupling latents to observables | Sparse coding (Olshausen & Field 1996) | Joint continuous relaxation with node-shared weights | **DERIVED EXTENSION** |
| **5. Size-Independent Generalization** | GNNs (Kipf & Welling 2017); RUN-CSP / ANYCSP (2020); Deep Equilibrium Graph Models | Parameter sharing of local potentials across arbitrary graph sizes | Evaluating =3$ trained models on =16, 32$ | Local energy rules are naturally graph-size agnostic | **NOT NOVEL** |
| **6. Iterative Inference-Time Compute** | Recurrent neural networks; Deep Equilibrium Models (Bai et al. 2019); Modern Hopfield | Fixed-point iteration {t+1} = \Phi(x_t)$ | Testing  \in \{1, 2, 4, 8, 16, 32\}$ | Standard recurrent unrolling | **NOT NOVEL** |
| **7. Equilibrium Propagation** | Scellier & Bengio (2017); Laborieux et al. (2021) | Exact fixed-point gradient $\nabla_\theta \mathcal{L} \propto \nabla_\theta E^\beta - \nabla_\theta E^0$ | Training physical neuromorphic and energy networks | Applied to factor graph permutation potentials | **KNOWN + USEFUL** |
| **8. Local Parameter-Shared Learning** | Relational GNNs (Schlichtkrull 2018); Factor Graph NNs (Satorras 2021) | Shared relation matrices $ across edges | Knowledge graph link prediction | Energy formulation rather than feed-forward message aggregation | **DERIVED EXTENSION** |
| **9. Lyapunov Stability Guarantee** | Classical Dynamical Systems (Lyapunov 1892); Cohen-Grossberg (1983) | $\dot{E} = \langle \nabla E, \dot{s} \rangle = -\|\nabla E\|^2 \le 0$ | Attractor networks and convex optimization | Discrete stability requires step size bound $\eta < 2/\lambda_{\max}$ | **NOT NOVEL** |
| **10. Conservative vs Non-Conservative Attention** | Modern Hopfield (Ramsauer 2020); Energy-Based Transformers (2025); NeurIPS 2025 | Symmetric bilinear form  W_K^T = (W_Q W_K^T)^T$ admits exact scalar potential | Stabilized attention in looped language models | Shows that standard attention can be conservative if symmetrized | **KNOWN ALTERNATIVE** |

---

## 3. Quantitative Summary of Audit Results (K=16, $\eta=0.20$)

| Model / Baseline | Class | =0$ | =16$ | $\Delta(T_{16} - T_0)$ | Rescue Rate | Damage Rate | Net Rescue | Verdict |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **Candidate_LearnedEnergy** | Neural EBM | 9.7\%$ | 9.6\%$ | hBc0.1\%$ | bash.7\%$ | bash.3\%$ | $+0.4\%$ | **INERT** |
| **Ablation_ZeroInteraction** | Ablation | 9.7\%$ | 9.7\%$ | bash.0\%$ | bash.0\%$ | bash.0\%$ | $+0.0\%$ | **INERT REFERENCE** |
| **Ablation_RandomSymmetric** | Ablation | 9.7\%$ | 9.4\%$ | hBc0.3\%$ | bash.9\%$ | bash.6\%$ | $+0.3\%$ | **INERT REFERENCE** |
| **N0_Unconstrained_Attn** | Recurrent NN | 9.7\%$ | 2.9\%$ | **hBc16.8\%* | 9.4\%$ | 8.5\%$ | $+0.8\%$ | **DIVERGENT** |
| **N1_Damped_Attn_0.25** | Stabilized NN | 9.7\%$ | 1.8\%$ | hBc7.9\%$ | 3.3\%$ | 3.2\%$ | $+0.1\%$ | **DAMPED DIVERGENCE** |
| **N4_Symmetric_Energy_Attn** | Conservative NN | 9.7\%$ | 6.5\%$ | hBc13.2\%$ | 9.7\%$ | 1.7\%$ | hBc2.0\%$ | **UNSTABLE STEPSIZE** |
| **SYNC1_Spectral_Sync** | Classical Sync | 9.7\%$ | **9.4\%* | **$+9.7\%* | **5.0\%* | **.8\%* | **$+53.2\%* | **ACTIVE SOLVER** |
| **SYNC5_Loopy_MinSum** | Factor Graph BP | 9.7\%$ | **5.1\%* | **$+15.4\%* | **5.0\%* | **.3\%* | **$+82.7\%* | **SUPERIOR SOLVER** |

---

## 4. Final Scientific Synthesis

1. **Retraction of Claims:**
   - The claim of discovering a "first irreducible computational primitive" or "breakthrough over Transformer attention" is **UNRESERVEDLY RETRACTED**.
   - The observed stability in EXP-TEN-006A was an instance of the **Inertia Fallacy**: mistaking an unresponsive model (bash.7\%$ rescue rate) for superior reasoning simply because the competitor was actively divergent.
2. **What Actually Remains Valid:**
   - We have established a rigorous, mathematically verified testbed for **Permutation Synchronization on Graphs**.
   - We have proved why unconstrained recurrent attention fails on cyclic graphs (circulant Jacobian eigenvalues on the unit circle).
   - We have demonstrated that classical Spectral Synchronization and Loopy Min-Sum are the true standards to beat.
3. **Condition for EXP-TEN-006B:**
   - Any future neural energy model must prove that it achieves **Net Rescue Rate $> +30\%*, demonstrating genuine inference compute scaling that competes with Spectral Synchronization (9.4\%$) and Loopy Min-Sum (5.1\%$), rather than boasting inert stability over broken baselines.
