# Experiment Protocol: EXP-TEN-002
## Hard Adversarial Falsification of Model C vs. Modern Associative Baselines

**Document ID:** `research/fundamental_ai/EXP_TEN_002_PROTOCOL.md`  
**Status:** Preregistered & Binding  
**Author:** Autonomous Research Scientist & Adversarial Scientific Reviewer (`ising_engine`)  

---

## 1. Scientific Core Question

Does Model C (Low-Rank CP 3-body Tensor Memory) possess any computational, architectural, or dynamical property that distinguishes it from known Polynomial Dense Associative Memory (Krotov & Hopfield 2016), or can its entire behavior be explained by:
1. Exact algebraic identity with Polynomial DAM ($n=3$);
2. Trivial 1-Nearest-Neighbor (1-NN) Hamming retrieval;
3. Sub-optimal approximation to Continuous Modern Hopfield / Softmax Attention?

---

## 2. Experimental Arms & Baselines

All models are evaluated on the exact same probe vectors, random seeds, and parameter/compute budgets.

1. **`B0_Hebbian` (Classical Hopfield):**
   - Pairwise Hebbian outer product: $J = \frac{1}{N} X X^T - I$.
2. **`B1_Pseudoinverse` (Projection Hopfield):**
   - Projection rule: $W = X (X^T X)^{-1} X^T$ (Kanter & Sompolinsky 1987).
   - Known capacity: $P \le N$ with near-zero crosstalk for linearly independent patterns.
3. **`B2_NearestNeighbor` (1-NN Hamming Oracle):**
   - Direct memory lookup: $s^* = \arg\max_\mu (\xi^\mu \cdot s)$.
   - Baseline question: Does energy relaxation achieve anything that a simple dot-product argmax cannot?
4. **`B3_PolynomialDAM` (Exact Polynomial DAM, $n=3$):**
   - Energy: $E(s) = -\frac{1}{6 N^2} \sum_\mu (\xi^\mu \cdot s)^3$.
   - Update: $s_i \leftarrow \text{sign}\left(\sum_\mu \xi_i^\mu (\xi^\mu \cdot s)^2\right)$.
5. **`B4_ModernHopfield` (Softmax / Continuous DAM):**
   - Energy: $E(x) = -\frac{1}{\beta} \text{lse}(\beta X^T x) + \frac{1}{2} \|x\|^2$.
   - Update: $x^{(t+1)} = X \text{softmax}(\beta X^T x^{(t)})$.
6. **`Candidate_ModelC` (CP 3-Body Tensor Memory):**
   - Our tensor coordinate-descent implementation with explicit self-interaction subtraction.

---

## 3. Memory Structure Battery (7 Distinct Datasets)

Testing only on i.i.d. random memories is a fatal flaw in associative memory evaluation because random vectors are nearly orthogonal in high dimensions ($m \approx 1/\sqrt{N}$). Real data has structure, correlations, and clusters.

1. **Suite A: Random i.i.d. Rademacher Patterns:** $\mathbb{P}(\xi_i = \pm 1) = 0.5$.
2. **Suite B: Correlated Memories:** Random walk Markov tree of memories with inter-pattern correlation $\rho \in \{0.3, 0.6, 0.8\}$.
3. **Suite C: Clustered Memories:** $K = 4$ cluster prototypes; memories are generated with $15\%$ random mutations around prototypes.
4. **Suite D: Low-Rank Subspace Memories:** Memories drawn from a low-dimensional generative subspace of rank $k \ll P$.
5. **Suite E: Adversarially Close Pairs:** Pairs of patterns with minimal Hamming distance $d_H \in \{1, 2, 3\}$.
6. **Suite F: Biased / Non-Zero Mean Patterns:** Average magnetization $m = \mathbb{E}[\xi_i] \in \{0.2, 0.4, 0.6\}$.
7. **Suite G: Compositional Attribute Triples:** Binary vectors formed by concatenating disjoint attribute slots $(A, B, C)$.

---

## 4. Falsification Decision Logic

| Experimental Outcome | Scientific Interpretation | Official Verdict |
|---|---|---|
| `Candidate_ModelC` matches `B3_PolynomialDAM` on all suites within $\epsilon < 10^{-6}$ | Model C is an algebraic re-parameterization of degree-3 DAM | **`KNOWN MECHANISM`** (No novelty) |
| `B2_NearestNeighbor` matches or beats `Candidate_ModelC` at equal noise | Relaxation dynamics performs trivial 1-NN lookup without basin advantage | **`FALSIFIED AS DYNAMICAL ADVANTAGE`** |
| `B1_Pseudoinverse` achieves higher capacity and noise tolerance at equal parameter count ($O(N^2)$) | Linear projection outperforms polynomial tensor Hebbian learning | **`FALSIFIED AS OPTIMAL CAPACITY`** |
| `Candidate_ModelC` demonstrates unique error correction on non-i.i.d. suites not achieved by B2 or B3 | Model C contains an unappreciated dynamical property | **`POTENTIALLY NOVEL ANOMALY`** |
