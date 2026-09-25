# Experiment Protocol: EXP-TEN-001
## Associative Binary Pattern Completion Under Noise Corruption

**Document ID:** `research/fundamental_ai/EXPERIMENT_PROTOCOL.md`  
**Status:** Preregistered & Frozen Prior to Code Execution  
**Author:** Autonomous Research Scientist (`ising_engine`)  

---

## 1. Scientific Objective

Quantitatively determine whether higher-order tensor energy dynamics (3-body sparse, 4-body sparse, or low-rank CP factorized) outperform classical pairwise Hopfield/Ising memory in associative recall, noise tolerance, and capacity per parameter, or whether any apparent advantage is nullified by parameter normalization.

---

## 2. Experimental Design Matrix

### 2.1. Factorial Grid
- **System Dimensions ($N$):** $128, 256, 512$
- **Pattern Load ($P/N$):** $0.10, 0.25, 0.50, 1.00$
  - For $N=128$: $P \in \{13, 32, 64, 128\}$
  - For $N=256$: $P \in \{26, 64, 128, 256\}$
  - For $N=512$: $P \in \{51, 128, 256, 512\}$
- **Noise Corruption ($\eta$):** $10\%, 20\%, 30\%, 40\%$ (independent Bernoulli bit flips)
- **Random Seeds:** 20 independent seeds per configuration (`seed = 1001 .. 1020`)
- **Total Trials per Model:** $3 \times 4 \times 4 \times 20 = 960$ trials.

### 2.2. Competing Models
1. **Model A (Baseline: Pairwise Hopfield / Ising):**
   - Dense pairwise Hebbian matrix $J_{ij} = \frac{1}{N} \sum_{\mu=1}^P \xi_i^\mu \xi_j^\mu$.
   - Parameters: $K_A = \frac{N(N-1)}{2}$.
2. **Model B (Candidate: 3-Body Sparse Tensor Memory):**
   - Sparse 3-body hyperedges $(i, j, k, w)$ selected by correlation magnitude.
   - Budget strictly matched to baseline: $M_3 = K_A = \frac{N(N-1)}{2}$.
   - Parameters: $K_B = K_A$.
3. **Model C (Candidate: Low-Rank CP 3-Body Tensor Memory):**
   - Symmetric CP factorization $T_{ijk} = \sum_{r=1}^P \lambda_r \xi_{ir} \xi_{jr} \xi_{kr}$.
   - Vectorized contraction $O(NR)$ per sweep.
   - Parameters: $K_C = N \times P$.
4. **Model D (Candidate: 4-Body Sparse Tensor Memory):**
   - Sparse 4-body hyperedges $(i, j, k, l, w)$ selected with budget $M_4 = K_A$.
   - Parameters: $K_D = K_A$.

---

## 3. Strict Execution Protocol

1. **Shared Pseudo-Random Number Generation:**
   For each (seed, $N, P$):
   - Generate $P$ i.i.d. Rademacher patterns $\xi^\mu \in \{-1, +1\}^N$ with probability $0.5$.
   - For each pattern $\mu \in \{1, \dots, P\}$:
     - Generate corrupted probe $s^{(0)}$ by flipping each bit with probability $\eta$.
     - Feed the exact same probe $s^{(0)}$ to Model A, Model B, Model C, and Model D.
2. **Inference Dynamics:**
   - Dynamics: D0 Sequential Greedy Coordinate Descent.
   - Sweeps: Maximum 50 sweeps.
   - Termination: When a full sweep produces zero bit flips (exact local energy minimum reached) or at 50 sweeps.
3. **Metrics Recorded per Probe:**
   - Exact match: $1$ if $s^* == \xi^\mu$, else $0$.
   - Overlap: $m = \frac{1}{N} \sum_{i=1}^N s_i^* \xi_i^\mu \in [-1, 1]$.
   - Final Energy: $E(s^*)$.
   - Convergence Sweeps: number of sweeps to reach attractor.
   - Flip Count: total bit flips.
   - Wall-clock time (microseconds).

---

## 4. Analytical Quality Gates

Before results are published:
- **Brute-Force Mathematical Verification:** A dedicated unit test must verify that $\Delta E_i = 2 s_i h_i^{\text{eff}}$ identically matches $E(s^{(i)}) - E(s)$ on small instances ($N=8, 12$) for all models.
- **Energy Monotonicity Check:** Verify that every greedy step satisfies $\Delta E \le 0$.
- **Statistical Significance:** Paired Wilcoxon signed-rank test on exact recovery across the 20 seeds.
