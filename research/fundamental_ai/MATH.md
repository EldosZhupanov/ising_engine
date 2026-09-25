# Mathematical Foundations: Higher-Order Tensor Energy Dynamics

**Document ID:** `research/fundamental_ai/MATH.md`  
**Status:** Frozen Reference  
**Author:** Autonomous Research Scientist (`ising_engine`)  

---

## 1. Discrete State Energy Formulation

Let the discrete state vector be $s = (s_1, \dots, s_N)^T$ with binary spins $s_i \in \{-1, +1\}$.

The general $K$-th order energy polynomial is defined over ordered multi-indices:
$$E(s) = -\sum_{i=1}^N h_i s_i - \sum_{1 \le i < j \le N} J_{ij} s_i s_j - \sum_{1 \le i < j < k \le N} T_{ijk} s_i s_j s_k - \sum_{1 \le i < j < k < l \le N} Q_{ijkl} s_i s_j s_k s_l$$

Where:
- $h_i \in \mathbb{R}$ is the linear bias (external magnetic field);
- $J_{ij} \in \mathbb{R}$ is the pairwise coupling matrix (Ising / Hopfield);
- $T_{ijk} \in \mathbb{R}$ is the 3-body interaction tensor;
- $Q_{ijkl} \in \mathbb{R}$ is the 4-body interaction tensor.

---

## 2. Effective Local Field and Flip Delta Exact Theorem

### Theorem 1 (Single Spin Flip Energy Delta)
Let $s^{(m)}$ be the state obtained by flipping the $m$-th spin of state $s$:
$$s_i^{(m)} = \begin{cases} -s_m & \text{if } i = m \\ s_i & \text{if } i \ne m \end{cases}$$
The exact change in energy $\Delta E_m \equiv E(s^{(m)}) - E(s)$ is given by:
$$\Delta E_m = 2 s_m h_m^{\text{eff}}(s)$$
where the effective local field $h_m^{\text{eff}}(s)$ acting on spin $m$ is defined as:
$$h_m^{\text{eff}}(s) = h_m + \sum_{j \ne m} J_{mj} s_j + \sum_{\substack{j < k \\ j,k \ne m}} T_{mjk} s_j s_k + \sum_{\substack{j < k < l \\ j,k,l \ne m}} Q_{mjkl} s_j s_k s_l$$
with indices inside $T$ and $Q$ sorted into canonical ascending order.

### Proof:
The energy $E(s)$ can be partitioned into terms that contain $s_m$ and terms independent of $s_m$:
$$E(s) = E_{\setminus m}(s) - s_m \left[ h_m + \sum_{j \ne m} J_{mj} s_j + \sum_{\substack{j < k \\ j,k \ne m}} T_{mjk} s_j s_k + \sum_{\substack{j < k < l \\ j,k,l \ne m}} Q_{mjkl} s_j s_k s_l \right] = E_{\setminus m}(s) - s_m h_m^{\text{eff}}(s)$$
When $s_m \to -s_m$, $E_{\setminus m}(s)$ is invariant:
$$E(s^{(m)}) = E_{\setminus m}(s) - (-s_m) h_m^{\text{eff}}(s) = E_{\setminus m}(s) + s_m h_m^{\text{eff}}(s)$$
Subtracting $E(s)$ yields:
$$\Delta E_m = E(s^{(m)}) - E(s) = \left[E_{\setminus m}(s) + s_m h_m^{\text{eff}}(s)\right] - \left[E_{\setminus m}(s) - s_m h_m^{\text{eff}}(s)\right] = 2 s_m h_m^{\text{eff}}(s) \quad \blacksquare$$

### Corollary 1.1 (Greedy Energy Descent)
If spin $m$ is updated via:
$$s_m \leftarrow \text{sign}\left(h_m^{\text{eff}}(s)\right)$$
(with the convention $\text{sign}(0) = s_m$), then the energy change satisfies:
$$\Delta E_m \le 0$$
with equality if and only if $s_m = \text{sign}(h_m^{\text{eff}}(s))$. Hence, greedy asynchronous updates converge monotonically to a local minimum of $E(s)$.

---

## 3. Sparse Hyperedge Representation

To prevent $O(N^3)$ and $O(N^4)$ memory explosions, the sparse model represents higher-order interactions as explicit weighted hyperedges:
$$\mathcal{H}_3 = \{(i_e, j_e, k_e, w_e)\}_{e=1}^{M_3}, \quad 1 \le i_e < j_e < k_e \le N$$
$$\mathcal{H}_4 = \{(i_e, j_e, k_e, l_e, w_e)\}_{e=1}^{M_4}, \quad 1 \le i_e < j_e < k_e < l_e \le N$$

The effective local field at node $m$ becomes:
$$h_m^{\text{eff}}(s) = h_m + \sum_{j \in \mathcal{N}_2(m)} J_{mj} s_j + \sum_{e \in \mathcal{E}_3(m)} w_e \prod_{v \in e \setminus \{m\}} s_v + \sum_{e \in \mathcal{E}_4(m)} w_e \prod_{v \in e \setminus \{m\}} s_v$$
where $\mathcal{E}_k(m)$ is the incident hyperedge adjacency list of node $m$.
- Memory complexity: $O(N + M_2 + M_3 + M_4)$.
- Field computation complexity: $O(|\mathcal{E}_2(m)| + 2 |\mathcal{E}_3(m)| + 3 |\mathcal{E}_4(m)|)$.

---

## 4. Low-Rank Canonical Polyadic (CP) Factorized Tensor Dynamics

### 4.1. Factorization Definition
For order-3 tensor interactions, a symmetric rank-$R$ CP decomposition models:
$$T_{ijk} = \sum_{r=1}^R \lambda_r a_{ir} a_{jr} a_{kr}$$
where $A = [a_{ir}] \in \mathbb{R}^{N \times R}$ and $\lambda \in \mathbb{R}^R$.
- Parameter count: $N \times R + R = O(NR)$, avoiding $O(N^3)$.

### 4.2. Vectorized Field Contraction
The effective field for spin $m$ excluding self-interactions is:
$$h_m^{\text{eff}}(s) = \sum_{r=1}^R \lambda_r a_{mr} \sum_{\substack{j < k \\ j,k \ne m}} a_{jr} a_{kr} s_j s_k$$
Using the algebraic identity for any vector $z$:
$$\sum_{j < k} z_j z_k = \frac{1}{2} \left[ \left(\sum_j z_j\right)^2 - \sum_j z_j^2 \right]$$
Let $z_j^{(r)} = a_{jr} s_j$, and let $S_r = \sum_{j=1}^N a_{jr} s_j = a_r^T s$.
Excluding node $m$, let $S_{r, \setminus m} = S_r - a_{mr} s_m$.
Then:
$$\sum_{\substack{j < k \\ j,k \ne m}} a_{jr} a_{kr} s_j s_k = \frac{1}{2} \left[ S_{r, \setminus m}^2 - \sum_{j \ne m} a_{jr}^2 \right]$$
Notice that since $s_j^2 = 1$ for all binary spins, $(a_{jr} s_j)^2 = a_{jr}^2$ is completely independent of the state $s$!
Therefore, the term $\sum_{j \ne m} a_{jr}^2 = \|a_r\|_2^2 - a_{mr}^2$ is a constant precomputed at initialization!
This reduces the entire contraction across all $N$ spins to:
1. Compute $S_r = a_r^T s$ in $O(NR)$ operations.
2. For each spin $m$, compute $h_m^{\text{eff}}$ in $O(R)$ operations:
   $$h_m^{\text{eff}}(s) = \frac{1}{2} \sum_{r=1}^R \lambda_r a_{mr} \left[ (S_r - a_{mr} s_m)^2 - (\|a_r\|_2^2 - a_{mr}^2) \right]$$
Total computational cost for full sweep: $O(NR)$ instead of $O(N^3)$!

---

## 5. Learning Rules and Parameter Budgets

### 5.1. Pairwise Hebbian (Baseline M0)
For $P$ training patterns $\{\xi^\mu\}_{\mu=1}^P \in \{-1, +1\}^N$:
$$J_{ij} = \frac{1}{N} \sum_{\mu=1}^P \xi_i^\mu \xi_j^\mu \quad (i \ne j), \quad J_{ii} = 0$$
Total parameters: $K_0 = \frac{N(N-1)}{2} \approx \frac{N^2}{2}$.

### 5.2. Sparse Higher-Order Selection (Candidate M3)
Given parameter budget $K_{\text{target}} = K_0$:
1. Compute empirical correlations:
   $$C_{ijk} = \frac{1}{P} \sum_{\mu=1}^P \xi_i^\mu \xi_j^\mu \xi_k^\mu$$
2. Select the top $M_3 = K_{\text{target}}$ triplets with largest absolute correlation $|C_{ijk}|$, or sample with probability proportional to $|C_{ijk}|$.
3. Set weight:
   $$w_{ijk} = \frac{1}{N^2} \sum_{\mu=1}^P \xi_i^\mu \xi_j^\mu \xi_k^\mu$$
Parameter count: $K_3 = M_3 = K_0$ (strictly matched).

### 5.3. Low-Rank CP Learning (Candidate M_CP)
When storing $P$ patterns in rank-$P$ form:
Set $a_{ir} = \xi_i^r$ and $\lambda_r = \frac{1}{N^2}$.
Parameter count: $K_{\text{CP}} = N \times P$.
For $P \le 0.5 N$, $K_{\text{CP}} \le 0.5 N^2 \le K_0$.

---

## 6. Continuous State and Dynamical Relaxation

For continuous vector extensions $x \in \mathbb{R}^N$ with box constraint $x_i \in [-1, +1]$:
Energy:
$$E(x) = -\sum_i h_i x_i - \sum_{i < j} J_{ij} x_i x_j - \sum_{i < j < k} T_{ijk} x_i x_j x_k$$

### 6.1. Overdamped Langevin Dynamics (D4)
$$dx_i = \left( -\frac{\partial E}{\partial x_i} - \gamma x_i \right) dt + \sqrt{2 T} dW_i(t)$$
where:
$$-\frac{\partial E}{\partial x_i} = h_i + \sum_{j \ne i} J_{ij} x_j + \sum_{j < k, j,k \ne i} T_{ijk} x_j x_k$$

### 6.2. Inertial Dynamics (D5)
$$v_i^{(t+1)} = \beta v_i^{(t)} - (1-\beta) \nabla_i E(x^{(t)})$$
$$x_i^{(t+1)} = \text{clip}\left(x_i^{(t)} + v_i^{(t+1)}, -1.0, +1.0\right)$$
