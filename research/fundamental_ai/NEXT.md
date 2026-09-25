# Research Roadmap: Fundamental AI Energy Dynamics

**Document ID:** `research/fundamental_ai/NEXT.md`  
**Status:** Live Working Sequence  
**Current Date:** September 12, 2026  
**Author:** Autonomous Research Scientist (`ising_engine`)  

---

## 1. Immediate Action Queue

```
[EXP-TEN-001: Associative Memory Scaling] ──────────► COMPLETED (EXP_TEN_001_RESULT.md)
                     │
                     ▼
[EXP-TEN-002: Hard Falsification of Model C] ──────► COMPLETED (EXP_TEN_002_RESULT.md)
   ├── Model C ≡ Polynomial DAM (n=3)
   └── Collapses on correlated data; 1-NN reaches 100%
                     │
                     ▼
[EXP-TEN-003: True Latent Compression (P >> R)] ───► COMPLETED (EXP_TEN_003_RESULT.md)
   ├── Distributed factors confirmed (MaxSim=0.355, PR=70.4)
   ├── Sign-Erasure Pathology discovered for odd-degree p=3
   └── Linear SVD Subspace beats CP-3 on unseen grammar (0.773 vs 0.322)
                     │
                     ▼
[EXP-TEN-004: Relational Compositional Reasoning] ──► COMPLETED (EXP_TEN_004_RESULT.md)
   ├── Inference-time compute scaling proved (T=1: 0.9% -> T=2: 100.0%)
   ├── Pairwise Hopfield collapses completely (0.0%)
   └── Linear TPR suffers multi-hop error amplification (0.0%)
                      │
                      ▼
[EXP-TEN-005: Energy Law Search & Cyclic Dynamics] ──► COMPLETED (EXP_TEN_005_RESULT.md)
   ├── Even powers (p=4) cure sign erasure; LogCosh & Dual Latent reach 0.63-0.88 cos
   ├── Recurrent directed attention stalls at limit cycles on cycles (85.0%)
   └── Modern Trilinear Cycle Energy achieves 99.1% (p < 1e-12; 16x error reduction)
                      │
                      ▼
[EXP-TEN-006A: Trainable Dual Energy Net Pilot] ─────► AUDITED & FALSIFIED AS REASONING
   ├── Overclaim Retracted: Model was inert (Rescue Rate = 0.7%, identical to zero-interaction)
   ├── Task reduced to Permutation Synchronization over S_d (Pachauri et al. 2013)
   └── Spectral Sync (89.4%) and Loopy Min-Sum (95.1%) vastly outperform candidate (79.6%)
                      │
                      ▼
[EXP-TEN-006_RESET: Prior-Art Reset & Audits] ───────► COMPLETED
   ├── EXP_TEN_006_PRIOR_ART_RESET.md (Algebraic reductions to Synchronization, BP, DEQ)
   ├── EXP_TEN_006_STABILITY_THEORY.md (Rigorous discrete Lyapunov & step-size bounds)
   └── EXP_TEN_006_NOVELTY_REVIEW.md (Component-level novelty matrix & retraction)
                      │
                       ▼
[EXP-TEN-006B: Rigorous Energy-Based Synchronization] ► COMPLETED (EXP_TEN_006B_RESULT.md)
   ├── Exact Analytical BPTT & EqProp Implemented & Mathematically Verified
   ├── Pilot Gate FAILED: Net Rescue <= +3.7% across all continuous models (EBM, GNN)
   ├── Null Hypothesis H_0 Confirmed: Classical Spectral Sync (+40.1%) & Loopy Min-Sum (+40.6%) dominate
   └── Discovered the Inertia-Erosion Dilemma of continuous relaxation on discrete constraint graphs
```

---

## 2. Answers to Priority Research Questions

1. **Does Model C do anything beyond 1-NN retrieval on structured data?**
   - **NO. Conclusively Falsified.** On correlated ($\rho=0.6$) and clustered data, Model C collapses to $0.0\%$, while trivial 1-NN lookup achieves $85\% - 100\%$ recovery (`EXP_TEN_002_RESULT.md`).
2. **Can $P \gg R$ factors generalize to unseen combinations?**
   - **Linear SVD ($0.924$) and Dual Latent EBMs ($0.883$) do, but homogeneous odd-degree CP-3 fails ($0.325$).** Odd-degree polynomial potentials square the overlap, unconditionally erasing coordinate signs (`EXP_TEN_003_RESULT.md`, `EXP_TEN_005_RESULT.md`).
3. **Does Continuous Energy Relaxation perform genuine relational computation beyond Baselines?**
   - **NO. Conclusively Falsified on Discrete Permutation Constraints (`EXP_TEN_006B_RESULT.md`).**
     - $E_{\ln\cosh}$ is **INERT** (Rescue Rate $= 0.0\%$, Net Rescue $= -0.2\%$) due to the vanishing gradient barrier at $s_v = 0$.
     - Linear EBMs and Contractive GNNs suffer **INFORMATION EROSION** (Damage Rate $= 18.9\%$, Rescue Rate $= 22.6\%$, Net Rescue $= +3.7\%$) because continuous diffusion lacks discrete winner-take-all pruning, dropping overall accuracy from $79\% \to 69\%$.
     - Classical Spectral Synchronization (**$+40.1\%$**) and Loopy Min-Sum BP (**$+40.6\%$**) decisively outperform all continuous neural models under matched budgets.


