# Prior Art Audit: Higher-Order Tensor Energy Dynamics in AI and Neural Associative Memory

**Document ID:** `research/fundamental_ai/PRIOR_ART.md`  
**Status:** Frozen Reference  
**Scope:** Foundational Literature, Mathematical Equivalences, and Classification of Novelty Claims  
**Author:** Autonomous Research Scientist (`ising_engine`)  

---

## 1. Executive Summary and Evaluation Framework

This audit provides a rigorous, exhaustive review of literature relevant to higher-order tensor energy dynamics, associative memories, continuous and discrete energy-based inference, and combinatorial optimization.

Every analyzed mechanism, mathematical formulation, and architecture is assigned one of four definitive labels:
- **`KNOWN`**: Explicitly published, analyzed, or proven in existing scientific literature.
- **`DERIVED`**: Follows straightforwardly from standard combinations of established mathematical or physical theorems without requiring new conceptual primitives.
- **`POSSIBLY NOVEL`**: A specific structural, algorithmic, or architectural configuration not currently documented in literature that introduces a distinct computational regime.
- **`NOT NOVEL`**: Previously claimed or perceived innovations that are identical or mathematically isomorphic to established prior art.

---

## 2. Exhaustive Prior Art Inventory

### 2.1. Classical Discrete Hopfield Networks
- **Key Citations:**
  - Hopfield, J. J. (1982). *Neural networks and physical systems with emergent collective computational abilities*. PNAS, 79(8), 2554–2558.
  - Hopfield, J. J. (1984). *Neurons with graded response have collective computational properties like those of two-state neurons*. PNAS, 81(10), 3088–3092.
  - Amit, D. J., Gutfreund, H., & Sompolinsky, H. (1985). *Storing infinite numbers of patterns in a spin-glass model of neural networks*. Physical Review Letters, 55(14), 1530.
- **Mathematical Formulation:**
  State $s_i \in \{-1, +1\}$ for $i \in \{1, \dots, N\}$.
  Pairwise energy function:
  $$E(s) = -\frac{1}{2} \sum_{i \ne j} J_{ij} s_i s_j - \sum_i h_i s_i$$
  Hebbian outer-product rule for $P$ patterns $\{\xi^\mu\}_{\mu=1}^P \in \{-1, +1\}^N$:
  $$J_{ij} = \frac{1}{N} \sum_{\mu=1}^P \xi_i^\mu \xi_j^\mu \quad (i \ne j), \quad J_{ii} = 0$$
  Asynchronous update: $s_i \leftarrow \text{sign}\left(\sum_j J_{ij} s_j + h_i\right)$.
- **Theoretical Limits:**
  Critical capacity $\alpha_c = \frac{P_{\max}}{N} \approx 0.138$ (AGS theory). Beyond $0.138 N$, spin-glass catastrophic breakdown occurs. Gardner limit for optimal unconstrained weights is $P \le 2N$. Spurious local minima grow exponentially with $N$.
- **Classification:** `KNOWN` (foundational baseline).

---

### 2.2. Classical Higher-Order Hopfield Networks & Polynomial Spin Glasses
- **Key Citations:**
  - Peretto, P., & Niez, J. J. (1986). *Long range interactions in neural networks*. Biological Cybernetics, 54(1), 53–63.
  - Psaltis, D., & Cheol Park, C. (1986). *Nonlinear discriminant functions and associative memories*. Neural Networks for Computing, AIP Conf. Proc., 151(1), 370–375.
  - Gardner, E. (1987). *Multiconnected neural networks*. J. Phys. A: Math. Gen., 20(11), 3453.
  - Baldi, P., & Venkatesh, S. S. (1987). *Number of stable points for spin-glasses and neural networks of higher orders*. Physical Review Letters, 58(9), 913.
  - Abbott, L. F., & Arian, Y. (1987). *Storage capacity of generalized networks*. Physical Review A, 36(10), 5091.
- **Mathematical Formulation:**
  For $p$-spin interactions ($p \ge 3$):
  $$E_p(s) = -\frac{1}{p!} \sum_{i_1 \ne i_2 \ne \dots \ne i_p} J_{i_1 i_2 \dots i_p} s_{i_1} s_{i_2} \dots s_{i_p}$$
  Higher-order Hebbian rule:
  $$J_{i_1 \dots i_p} = \frac{1}{N^{p-1}} \sum_{\mu=1}^P \xi_{i_1}^\mu \dots \xi_{i_p}^\mu$$
- **Theoretical Limits:**
  Storage capacity scales as:
  $$P_c^{(p)} \approx \alpha_p \frac{N^{p-1}}{p! \ln N}$$
  Number of parameters scales as $\binom{N}{p} \sim \frac{N^p}{p!}$.
  Therefore, the **capacity per parameter** is:
  $$\frac{\text{Bits Stored}}{\text{Parameters}} = \frac{P_c \cdot N}{\binom{N}{p}} = \frac{O(N^p / \ln N)}{O(N^p)} = O\left(\frac{1}{\ln N}\right) \le O(1)$$
  Dense higher-order interactions drastically increase parameter count to $O(N^p)$, yielding no asymptotic gain in parameter efficiency.
- **Classification:** `KNOWN` (mathematical capacity and scaling bounds are completely established).

---

### 2.3. Modern Hopfield Networks & Dense Associative Memory (DAM)
- **Key Citations:**
  - Krotov, D., & Hopfield, J. J. (2016). *Dense Associative Memory for Pattern Recognition*. NeurIPS 2016.
  - Demircigil, M., Heubeck, J., Ramsauer, H., et al. (2017). *Model of associative memory with huge storage capacity*. arXiv:1702.01929.
  - Krotov, D., & Hopfield, J. J. (2020). *Large Associative Memory Problem in Neurobiology and Machine Learning*. arXiv:2008.06996.
  - Ramsauer, H., Schäfl, B., Lehner, J., et al. (2020). *Hopfield Networks is All You Need*. ICLR 2021.
- **Mathematical Formulation:**
  Instead of explicit tensor weights, the energy is defined as a non-linear function $F: \mathbb{R} \to \mathbb{R}$ of the pattern overlaps:
  $$E(s) = -\sum_{\mu=1}^P F\left(\sum_{i=1}^N \xi_i^\mu s_i\right)$$
  - For $F(x) = x^n$, expanding $F(\xi^\mu \cdot s)$ recovers symmetric higher-order tensor interactions:
    $$E(s) = -\sum_{i_1, \dots, i_n} \left(\sum_{\mu=1}^P \xi_{i_1}^\mu \dots \xi_{i_n}^\mu\right) s_{i_1} \dots s_{i_n}$$
  - For $F(x) = \exp(x)$ (Demircigil et al., 2017; Ramsauer et al., 2020), storage capacity is exponential: $P_c \sim 2^{N/2}$ or $c^N$.
  - For continuous states $x \in \mathbb{R}^d$:
    $$E(x) = -\text{lse}\left(\beta X^T x\right) + \frac{1}{2} \|x\|^2$$
    Update step:
    $$x^{(t+1)} = X \text{softmax}\left(\beta X^T x^{(t)}\right)$$
    This update is mathematically identical to the attention mechanism of Transformers ($V \cdot \text{softmax}(K^T Q)$)!
- **Key Distinction from our Proposed Experiment:**
  Modern Hopfield / Attention computes the contraction against stored prototypes $X \in \mathbb{R}^{N \times P}$ on the fly. It maintains $O(N P)$ parameters, but requires storing all training exemplars explicitly (non-parametric in memory size). It does not store factorized structural hyperedges or sparse combinatorial relations between variables independent of exemplars.
- **Classification:** `KNOWN` (equivalence of continuous DAM and attention is fully proven; claiming attention equivalence as novel would be `NOT NOVEL`).

---

### 2.4. Boltzmann Machines & Higher-Order Boltzmann Machines
- **Key Citations:**
  - Ackley, D. H., Hinton, G. E., & Sejnowski, T. J. (1985). *A learning algorithm for Boltzmann machines*. Cognitive Science, 9(1), 147–169.
  - Sejnowski, T. J. (1986). *Higher-order Boltzmann machines*. AIP Conf. Proc., 151(1), 398–403.
- **Mathematical Formulation:**
  Stochastic distribution over configurations $s \in \{-1, +1\}^N$:
  $$P(s) = \frac{1}{Z} \exp(-E(s) / T)$$
  Higher-order energy:
  $$E(s) = -\sum_i h_i s_i - \sum_{ij} J_{ij} s_i s_j - \sum_{ijk} T_{ijk} s_i s_j s_k$$
  Contrastive learning rule:
  $$\Delta T_{ijk} = \eta \left( \langle s_i s_j s_k \rangle_{\text{clamped}} - \langle s_i s_j s_k \rangle_{\text{free}} \right)$$
- **Theoretical Limits:**
  Suffers from the partition function $Z$ estimation problem. Sampling $\langle s_i s_j s_k \rangle_{\text{free}}$ via MCMC is exponentially slow due to deep metastable barriers in higher-order landscapes.
- **Classification:** `KNOWN`.

---

### 2.5. Energy-Based Models (EBMs) & Equilibrium Propagation
- **Key Citations:**
  - LeCun, Y., Chopra, S., Hadsell, R., Ranzato, M., & Huang, F. (2006). *A tutorial on energy-based learning*. Predicting Structured Data.
  - Scellier, B., & Bengio, Y. (2017). *Equilibrium Propagation: Bridging the gap between energy-based models and backpropagation*. Frontiers in Computational Neuroscience, 11, 24.
  - Du, Y., & Mordatch, I. (2019). *Implicit generation and modeling with energy-based models*. NeurIPS 2019.
- **Mathematical Formulation:**
  Continuous energy $E_\theta(s, x)$.
  Equilibrium state: $s^* = \arg\min_s E_\theta(s, x)$ via $\frac{ds}{dt} = -\nabla_s E_\theta(s, x)$.
  Gradient of loss $\mathcal{L}(s^*, y)$ with respect to parameter $\theta$:
  $$\frac{\partial \mathcal{L}}{\partial \theta} = \lim_{\beta \to 0} \frac{1}{\beta} \left( \frac{\partial E_\theta}{\partial \theta}(s^\beta, x) - \frac{\partial E_\theta}{\partial \theta}(s^0, x) \right)$$
  where $s^\beta$ is the state relaxed under the nudged energy $E_\theta + \beta \mathcal{L}$.
- **Classification:** `KNOWN` (Equilibrium Propagation proves local energy learning matches backpropagation).

---

### 2.6. Predictive Coding
- **Key Citations:**
  - Rao, R. P., & Ballard, D. H. (1999). *Predictive coding in the visual cortex*. Nature Neuroscience, 2(1), 79–87.
  - Friston, K. (2005). *A theory of cortical responses*. Philosophical Transactions of the Royal Society B, 360(1456), 815–836.
  - Millidge, B., Tschantz, A., & Buckley, C. L. (2021). *Predictive coding: a theoretical and experimental review*. arXiv:2107.12979.
- **Mathematical Formulation:**
  Hierarchical generative model where prediction errors $\varepsilon_l = x_l - g(W_l x_{l+1})$ are minimized by gradient descent on free energy $F = \sum_l \frac{1}{2 \sigma_l^2} \|\varepsilon_l\|^2$.
- **Classification:** `KNOWN`.

---

### 2.7. Higher-Order Ising / HOBO / PUBO and Quadratization
- **Key Citations:**
  - Rosenberg, I. G. (1975). *Reduction of bivalent maximization to the quadratic case*. Cahiers du Centre d'Études de Recherche Opérationnelle, 17, 71–74.
  - Boros, E., & Hammer, P. L. (2002). *Pseudo-boolean optimization*. Discrete Applied Mathematics, 123(1-3), 155–225.
  - Dattani, N. (2019). *Quadratization in discrete optimization and quantum annealing*. arXiv:1901.04405.
  - Biamonte, J. D. (2008). *Non-perturbative k-body to two-body commutable Hamiltonian reduction*. Phys. Rev. A 77, 052331.
- **Mathematical Formulation:**
  Higher-Order Binary Optimization (HOBO) minimizes:
  $$E(s) = \sum_{k=1}^K \sum_{i_1 < \dots < i_k} C_{i_1 \dots i_k} \prod_{m=1}^k s_{i_m}, \quad s_i \in \{0, 1\} \text{ or } \{-1, +1\}$$
  Standard reduction (Rosenberg): replace product $x_i x_j$ by auxiliary variable $w \in \{0, 1\}$ using penalty:
  $$P(x_i, x_j, w) = M (x_i x_j - 2 x_i w - 2 x_j w + 3 w), \quad M > 0$$
- **Known Pathologies of Quadratization:**
  1. Blowup in variable count: $O(N^2)$ or $O(N^3)$ auxiliary variables.
  2. Energy landscape distortion: Large penalty coefficients $M$ create deep artificial valleys and high energy barriers, freezing local search and causing slow Markov chain mixing.
  3. Spectral gap degradation: Minimum spectral gap in quantum annealing shrinks exponentially with the addition of auxiliary penalty constraints.
- **Relevance to our Experiment:** Native higher-order dynamics (avoiding quadratization) is a well-motivated direction in optimization, but its utility as a **neural associative memory primitive** requires empirical proof.
- **Classification:** `KNOWN` (HOBO and quadratization penalties); native higher-order relaxation dynamics is `DERIVED`.

---

### 2.8. p-Bit Networks & Oscillator Ising Machines
- **Key Citations:**
  - Camsari, K. Y., Faria, R., Datta, S., et al. (2019). *Stochastic p-bits for invertible logic*. Physical Review X, 9(2), 021011.
  - Borders, W. A., et al. (2019). *Integer factorization using stochastic magnetic tunnel junctions*. Nature, 573(7774), 390–393.
  - Wang, T., & Roychowdhury, J. (2019). *OIM: Oscillator-based Ising machines for solving combinatorial optimization problems*. UCNC 2019.
  - Inagaki, T., et al. (2016). *A coherent Ising machine for 2000-node optimization problems*. Science, 354(6312), 603–606.
- **Formulation:**
  Stochastic activation via magnetic tunnel junctions: $s_i(t) = \text{sign}\left(\tanh(\beta I_i(t)) - \text{rand}(-1, 1)\right)$. Higher-order logic gates (e.g. 3-bit full adder) implemented as native 3-terminal devices.
- **Classification:** `KNOWN` (hardware physical dynamics).

---

### 2.9. Tensor Networks & Tensor Decompositions
- **Key Citations:**
  - Kolda, T. G., & Bader, B. W. (2009). *Tensor decompositions and applications*. SIAM Review, 51(3), 455–500.
  - Oseledets, I. V. (2011). *Tensor-train decomposition*. SIAM J. Sci. Comput., 33(5), 2295–2317.
  - Novikov, A., et al. (2015). *Tensorizing neural networks*. NeurIPS 2015.
  - Stoudenmire, E., & Schwab, D. J. (2016). *Supervised learning with tensor networks*. NeurIPS 2016.
- **Mathematical Formulation:**
  Canonical Polyadic (CP) decomposition of order-3 tensor:
  $$T_{ijk} \approx \sum_{r=1}^R \lambda_r a_{ir} b_{jr} c_{kr}$$
  Symmetric CP decomposition:
  $$T_{ijk} \approx \sum_{r=1}^R \lambda_r a_{ir} a_{jr} a_{kr}$$
  Parameter scaling: reduces dense $O(N^3)$ parameters to $O(N R)$.
  Contraction of effective field with state $s$:
  $$h_i^{\text{eff}} = \sum_{j, k} T_{ijk} s_j s_k = \sum_{r=1}^R \lambda_r a_{ir} \left(\sum_j a_{jr} s_j\right) \left(\sum_k a_{kr} s_k\right) = \sum_{r=1}^R \lambda_r a_{ir} (a_r^T s)^2$$
  Computation of all $N$ local fields drops from $O(N^3)$ to $O(N R)$!
- **Classification:** `KNOWN` in multilinear algebra; application to higher-order energy field contraction is `DERIVED`.

---

### 2.10. Hypergraph Neural Networks & Sigma-Pi Units
- **Key Citations:**
  - Rumelhart, D. E., McClelland, J. L., et al. (1986). *Parallel Distributed Processing*, Vol 1. (Sigma-Pi units).
  - Feng, Y., You, H., Zhang, Z., et al. (2019). *Hypergraph neural networks*. AAAI 2019.
  - Yadati, N., et al. (2019). *HyperGCN: A new method of training graph convolutional networks on hypergraphs*. NeurIPS 2019.
- **Mathematical Formulation:**
  Sigma-Pi unit: $y_i = \sigma\left(\sum_k w_{ik} \prod_{j \in S_k} x_j\right)$. Hypergraph Laplacian and incidence matrices $H \in \{0, 1\}^{|V| \times |E|}$.
- **Classification:** `KNOWN`.

---

### 2.11. Compositional Associative Memory & Vector Symbolic Architectures
- **Key Citations:**
  - Smolensky, P. (1990). *Tensor product variable binding and the representation of symbolic structures in connectionist systems*. Artificial Intelligence, 46(1-2), 159–216.
  - Plate, T. A. (1991, 2003). *Holographic Reduced Representations*. IEEE Trans. Neural Netw.
  - Kanerva, P. (2009). *Hyperdimensional computing: An introduction to computing in distributed representation with high-dimensional random vectors*. Cognitive Computation, 1(2), 139–159.
  - Gayler, R. W. (2003). *Vector Symbolic Architectures answer Jackendoff’s challenges for cognitive science*.
- **Mathematical Formulation:**
  Smolensky: predicate binding via tensor product $r \otimes f \in \mathbb{R}^{d_1 \times d_2}$.
  Plate HRR: circular convolution $x \circledast y$, circular correlation $x \odot y$.
  Kanerva HDC: binding via element-wise multiplication $x \odot y$, bundling via superposition $\sum x_i$.
  Higher-order energy binding of relation $(e_1, r, e_2)$:
  $$E(e_1, r, e_2) = - \sum_{i, j, k} T_{ijk} (e_1)_i r_j (e_2)_k$$
- **Classification:** `KNOWN` (tensor product binding and HDC); energy minimization completion over tensor bindings is `DERIVED`.

---

### 2.12. Neural Cellular Automata (NCA)
- **Key Citations:**
  - Mordvintsev, A., Randazzo, E., Niklasson, E., & Levin, M. (2020). *Growing neural cellular automata*. Distill, 5(2), e23.
- **Formulation:**
  Local update rule: $x_{i,j}^{(t+1)} = x_{i,j}^{(t)} + \Delta x(N(x_{i,j}^{(t)}))$. Demonstrates robust attractor dynamics, pattern regeneration from 50% damage, and self-repair via local consensus.
- **Classification:** `KNOWN`.

---

## 3. Comparative Taxonomy & Novelty Assessment Table

| Conceptual Component | Prior Art Status | Governing Literature | What is Known | What Must NOT Be Claimed as Novel | Where Genuine Investigation Lives |
|---|---|---|---|---|---|
| **Pairwise Hopfield Memory** | `KNOWN` | Hopfield (1982), Amit et al. (1985) | Capacity $\approx 0.138N$, Hebbian learning, spurious states | Hopfield dynamics, quadratic energy | Baseline comparison only |
| **Dense Higher-Order Hopfield ($O(N^p)$)** | `KNOWN` | Peretto & Niez (1986), Psaltis (1986), Gardner (1987) | Capacity $O(N^{p-1})$, Hebbian tensor formula, parameter count $O(N^p)$ | Claiming higher-order energy increases capacity *per parameter* | Confirming whether parameter normalization destroys the gain |
| **Continuous Modern Hopfield / DAM** | `KNOWN` / `NOT NOVEL` | Krotov & Hopfield (2016), Demircigil (2017), Ramsauer (2020) | Exponential capacity via non-linear overlap $F(\sum \xi_i s_i)$, exact equivalence to Transformer Self-Attention | Any claim that 'non-linear energy recovers attention' or that DAM is unstudied | We are explicitly NOT testing sample-prototype attention; we are testing explicit structural hyperedges |
| **Rosenberg Quadratization Penalties** | `KNOWN` | Rosenberg (1975), Boros & Hammer (2002), Dattani (2019) | Penalty gadgets create high barrier landscapes and auxiliary variables | Claiming quadratization adds variables is a novel discovery | Quantifying the exact barrier height differences between native HOBO and QUBO |
| **Sparse Higher-Order Hyperedge Energy Memory** | `POSSIBLY NOVEL` | Minimal / Fragmented (mostly hypergraph GNNs or coding theory) | Sparse parity check codes (LDPC), hypergraph cuts | Sparse hypergraphs as general data structures | **Can a budgeted set of $O(N^2)$ hyperedges outperform $O(N^2)$ pairwise edges in capacity and basin radius?** |
| **Symmetric CP Factorized Energy Contraction** | `DERIVED` | Kolda & Bader (2009), Novikov (2015) | CP rank factorization, $O(NR)$ parameterization and contraction | Tensor CP decomposition itself | **Does CP rank $R$ provide an optimal attractor landscape under gradient descent or Hebbian projection?** |
| **Compositional Completion via 3-Body Attractors** | `DERIVED` / `POSSIBLY NOVEL` | Smolensky (1990), Plate (2003) | TPR bindings, HRR, energy minimizations | Tensor representation of relations | **Can asymmetric 3-body relaxation $(A, R, ? \to B)$ outperform pairwise cross-attention or MLP at equal parameter budget?** |

---

## 4. Methodological Invariants for our Fundamental Experiment

To maintain scientific integrity and prevent false claims:

1. **Parameter Normalization Invariant:**
   A candidate model $M_k$ with $K_k$ parameters must NEVER be compared to baseline $M_0$ with $K_0$ parameters without explicitly reporting metrics per parameter:
   $$\text{Efficiency} = \frac{\text{Bits Recovered at Threshold}}{\text{Total Trainable/Stored Parameters}}$$
   If $M_1$ (third-order) uses $\binom{N}{3}$ parameters and $M_0$ uses $\binom{N}{2}$ parameters, claiming $M_1$ stores more patterns is trivial and meaningless.
2. **Fixed Compute Invariant:**
   Inference dynamics must record floating point operations (FLOPs), hyperedge evaluations, and wall-clock time.
3. **Falsification-First Protocol:**
   Before claiming that higher-order tensors provide an advantage, we must test whether:
   - The same advantage is obtained by simply adding a small MLP or low-rank pairwise matrix.
   - The advantage disappears when pattern correlations are removed.
   - The advantage is an artifact of small $N \le 128$.
4. **Negative Result Documentation:**
   If sparse higher-order interactions perform worse than pairwise Hopfield at equal parameter count, this is recorded immediately as a core scientific finding.
