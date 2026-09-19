---
id: cd002-prior-art
kind: research-reference
status: active
authority_scope: prior-art-audit
created: 2026-09-19
immutable: false
---

# CD002 Prior-Art Matrix & Adversarial Literature Attack

Date: 2026-09-19.
Objective: Prove that the mathematical elements of CD002 are already known, locate the exact boundary, and determine whether any genuine, unrefuted contribution remains.

---

## 1. Prior-Art Mapping Matrix

| Proposed Component | Closest Literature Source | Equivalence / Overlap | Novelty Verdict |
|---|---|---|---|
| Permutation Synchronization on Graphs | Pachauri, Kondor & Singh (NIPS 2013), "Solving the multi-way matching problem by permutation synchronization" | Identical mathematical problem: recover node permutations $P_v \in S_d$ from noisy pairwise measurements $\Pi_{uv}$. | **NOT NOVEL** (Algebraic Identity) |
| Spectral Relaxation of Permutations | Singer (2011), "Angular synchronization by eigenvectors and semidefinite programming"; Bandeira et al. (2016) | Computes top eigenvectors of the block Gram matrix $\mathbf{R}$, rounds via Hungarian algorithm. | **NOT NOVEL** (Standard Baseline) |
| Permutation Matching as QUBO | Lucas (2014), "Ising formulations of many NP-problems", Section 3; Neven et al. (2008) | Formulates matching and Quadratic Assignment (QAP) as binary quadratic polynomials with penalty $\lambda (\sum X_{ij} - 1)^2$. | **NOT NOVEL** (Textbook Encoding) |
| Gauge Invariance in Spin Glasses | Toulouse (1977), "Theory of the frustration effect in spin glasses: II. Frustration on lattices"; Fradkin et al. (1978) | Maps gauge transformations $\sigma_i \to g_i \sigma_i$ and link variables $J_{ij} \to g_i J_{ij} g_j^{-1}$. Holonomy $\prod J$ determines frustration. | **NOT NOVEL** (Classical Physics) |
| Cycle Consistency in AI / NLP | Zhou et al. (2015), "Multi-image matching via fast alternating minimization"; Wang et al. (2019), "Deep Permutation Synchronization" | Enforces cycle consistency for entity alignment, visual correspondence, and graph neural networks. | **NOT NOVEL** (Established Subfield) |

---

## 2. Adversarial Scrutiny: What is NOT Novel

1. **The Representation is Not Novel:**
   Encoding permutations as binary indicator matrices $X \in \{0, 1\}^{d \times d}$ with row/column constraints in QUBO is standard since Lucas (2014). Calling this an "Ising reasoning breakthrough" is a marketing repackaging of textbook QAP-QUBO.

2. **The Problem is Not Novel:**
   Multi-way permutation synchronization has been studied for over a decade in computer vision and signal processing (Pachauri 2013, Chen & Candes 2018, Leonardos 2021).

3. **Discrete NP-Hardness is Known:**
   Solving the discrete Gauge Hamiltonian / QAP directly is NP-hard. Spectral methods (Pachauri 2013) were introduced *specifically* because the exact combinatorial problem is computationally intractable on large graphs!

---

## 3. What Question Remains Potentially Unresolved?

The only valid, non-trivial engineering question is:
**Does the discrete combinatorial optimizer actually beat Spectral Synchronization in recovery accuracy when the spectral gap collapses under adversarial, frustrated cycle corruption?**

- Under i.i.d. noise, Spectral Synchronization has proven statistical optimality (Bandeira et al. 2016).
- But under **frustrated cycle attacks** (where an adversarial fraction of edges creates a non-trivial holonomy loop), does Spectral Synchronization hallucinate or smear the permutation, whereas exact discrete minimization isolates the corrupted edge?
- And does the discrete landscape of frustrated gauge models exhibit spin-glass replica symmetry breaking, making local discrete search as blind as continuous relaxation?

If exact discrete minimization fails or is out-competed by simple cycle filtering (e.g. Cycle-Edge Message Passing, Leonardos 2021), the hypothesis is **DEAD (NO-GO)**.
