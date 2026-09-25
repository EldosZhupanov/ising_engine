# Experimental Protocol: EXP-TEN-003
## True Latent Factorization ($P \gg R$): Generative Structure vs. Exemplar Memorization

**Experiment ID:** `EXP-TEN-003`  
**Document ID:** `research/fundamental_ai/EXP_TEN_003_PROTOCOL.md`  
**Status:** Frozen & Approved for Execution  
**Date:** September 12, 2026  
**Author:** Autonomous Research Scientist & Adversarial Reviewer (`ising_engine`)  

---

## 1. Scientific Objective & Hypotheses

In `EXP-TEN-001` and `EXP-TEN-002`, rank-$P$ tensor memory ($R = P$) was proved to be identical to degree-3 Polynomial Dense Associative Memory (Krotov & Hopfield 2016). When $R = P$, each factor $u^r = \xi^r$ is an explicit copy of a training memory, making the system an exemplar-retrieval device that collapses under correlations.

`EXP-TEN-003` transitions from **exemplar memorization** ($R = P$) to **true latent compression** ($P \gg R$). We investigate whether a low-rank CP 3-body energy model ($R \ll P$) can discover a compact generative representation of a combinatorial dataset, evaluate factor reuse and entropy, and determine whether energy dynamics can reconstruct **unseen valid combinations** that are absent from the training set.

### Formal Hypotheses:
- **$H_1$ (Distributed Latent Representation):** When $P \gg R$, the learned factors $u^r$ do not correspond 1-to-1 with training exemplars ($\max_\mu |\cos(u^r, \xi^\mu)| \ll 1.0$), and the participation ratio $\text{PR}_r \gg 1$, proving distributed factor reuse.
- **$H_2$ (Generative Basin Formation):** The energy landscape formed by rank-$R$ 3-body factors possesses local minima at valid grammar points that were *never present* in $\mathcal{D}_{\text{train}}$.
- **$H_0$ (Exemplar / Linear Bound Null):** The energy dynamics of rank-$R$ CP-3 cannot reconstruct unseen valid combinations better than a standard linear subspace projection $\hat{s} = \text{sign}(U_R U_R^T s_{\text{noisy}})$, and cannot surpass 1-NN retrieval on seen data.

---

## 2. Experimental Design & Independent Variables

### 2.1 Generative Data World: Compositional Feature Grammar
To objectively measure whether a model learns a generative rule rather than memorizing exemplars, we define a controlled combinatorial grammar:
- **System Size:** $N = 128$ binary spins ($s_i \in \{-1, +1\}$).
- **Latent Dimension:** $K = 8$ independent latent binary features $z \in \{-1, +1\}^K$.
- **Total Grammatical Universe:** $|\mathcal{W}| = 2^K = 256$ valid patterns (and scaled to $K=10 \implies 1024$ patterns).
- **Generator Matrix:** $G \in \mathbb{R}^{N \times K}$, generated with orthonormal columns ($G^T G = I_K$).
- **Valid Pattern Generation:** $\xi(z) = \text{sign}(G z)$.
- **Dataset Split:**
  - $\mathcal{D}_{\text{train}}$: $P_{\text{train}} \in \{128, 192\}$ patterns sampled from $\mathcal{W}$ ($50\%$ to $75\%$ of the universe).
  - $\mathcal{D}_{\text{unseen}}$: Systematically held-out valid patterns from $\mathcal{W} \setminus \mathcal{D}_{\text{train}}$ ($64$ to $128$ unseen patterns).

### 2.2 Latent Rank R
- $R \in \{4, 8, 16, 32\}$, ensuring $R \ll P$ ($R/P \in [0.02, 0.25]$).

### 2.3 Evaluated Architectures & Controls
1. **`Candidate_LatentCP3`:**
   $$E(s) = -\frac{1}{6 N^2} \sum_{r=1}^R \lambda_r (u^r \cdot s)^3$$
   where $u^r \in \mathbb{R}^N$ are rank-$R$ factors learned via symmetric tensor power decomposition / top spectral components, and dynamics follow discrete greedy coordinate descent ($s_i \leftarrow \text{sign}(h_i)$).
2. **`B0_Pairwise_LowRank`:**
   Pairwise Hopfield with rank-$R$ weight matrix $W = \sum_{r=1}^R u^r (u^r)^T$.
3. **`B1_LinearSVD`:**
   Direct linear projection followed by sign: $\hat{s} = \text{sign}(U_R U_R^T s_{\text{noisy}})$. Single-step feedforward projection baseline.
4. **`B2_1NN_Oracle`:**
   Exemplar lookup: $\arg\max_{\mu \in \mathcal{D}_{\text{train}}} (\xi^\mu \cdot s_{\text{noisy}})$. Cannot generate unseen patterns by design!
5. **`B3_ModernHopfield`:**
   Continuous softmax attention over $\mathcal{D}_{\text{train}}$ ($\beta = 1.5$). Exemplar-weighted combination baseline.

### 2.4 Noise Levels & Trials
- Corruption noise: $\eta \in \{15\%, 30\%\}$.
- Statistical repetitions: 20 independent seeds per condition.

---

## 3. Metrics & Diagnostics

1. **Factor-Memory Mutual Similarity:**
   $$\text{MaxSim}(u^r) = \max_{\mu=1 \dots P} \frac{|u^r \cdot \xi^\mu|}{\|u^r\| \|\xi^\mu\|}, \quad \overline{\text{MaxSim}} = \frac{1}{R} \sum_{r=1}^R \text{MaxSim}(u^r)$$
2. **Participation Ratio (Factor Reuse):**
   $$a_{r\mu} = \frac{(u^r \cdot \xi^\mu)^2}{N^2}, \quad p_{r\mu} = \frac{a_{r\mu}}{\sum_\nu a_{r\nu}}, \quad \text{PR}_r = \frac{1}{\sum_{\mu=1}^P p_{r\mu}^2}$$
3. **Normalized Factor Entropy:**
   $$q_{\mu r} = \frac{a_{r\mu}}{\sum_s a_{s\mu}}, \quad H_\mu = -\sum_{r=1}^R q_{\mu r} \ln q_{\mu r}, \quad \overline{H}_{\text{norm}} = \frac{1}{P \ln R} \sum_{\mu=1}^P H_\mu$$
4. **Seen Reconstruction Accuracy:**
   Percentage of noisy patterns from $\mathcal{D}_{\text{train}}$ converged exactly ($d_H = 0$) to their ground truth.
5. **Unseen Generative Generalization Accuracy:**
   Percentage of noisy patterns from $\mathcal{D}_{\text{unseen}}$ converged exactly ($d_H = 0$) to the unseen ground truth.
6. **Cosine Fidelity:**
   Mean cosine similarity $\frac{1}{N} s_{\text{final}} \cdot \xi_{\text{ground\_truth}}$ for both seen and unseen patterns.

---

## 4. Adversarial Falsification Criteria

- If $\overline{\text{MaxSim}} > 0.85$, the factors are failing to generalize and are merely memorizing a subset of $R$ training patterns.
- If `Candidate_LatentCP3` achieves **$0\%$ unseen accuracy**, the energy landscape has no minima at unseen grammatical points, and the model cannot perform generative generalization.
- If `Candidate_LatentCP3` unseen accuracy is $\le$ `B1_LinearSVD`, the non-linear 3-body energy dynamics add zero generative computational value over a simple linear projection.
