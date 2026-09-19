---
id: cd002-math
kind: research-protocol
status: active
authority_scope: mathematical-derivation
created: 2026-09-19
immutable: false
---

# CD002 Mathematical Framework: Gauge Synchronization on Graphs

## 1. Problem Formulation

Let $\mathcal{G} = (V, \mathcal{E})$ be a connected graph with $|V| = K$ nodes.
Let $S_d$ denote the symmetric group on $d$ symbols, with identity $e = (0, 1, \dots, d-1)$.
Each node $v \in V$ is assigned a ground-truth permutation $P_v^* \in S_d$.

For each directed edge $(u, v) \in \mathcal{E}$, the ground-truth relative measurement is:
$$\Pi_{uv}^* = P_u^* (P_v^*)^{-1}$$
so that $P_u^* = \Pi_{uv}^* P_v^*$.

### Cycle Consistency (Zero Holonomy)
For any directed cycle $C = (v_0, v_1, \dots, v_{K-1}, v_0)$, consistency requires:
$$\prod_{k=0}^{K-1} \Pi_{v_k, v_{k+1}} = e$$

### Adversarial Frustration
We inject an adversarial corrupting permutation $\tau \ne e$ onto a single edge $(v_{K-1}, v_0)$, creating non-zero cycle holonomy:
$$\Pi_{v_{K-1}, v_0}^{\text{obs}} = \tau \Pi_{v_{K-1}, v_0}^* \implies \oint_C \Pi^{\text{obs}} = \tau \ne e$$

The measurement graph is now **frustrated**: no assignment $P_0, \dots, P_{K-1}$ can satisfy all $K$ edge relations simultaneously! Exactly at least one edge must be violated.

---

## 2. Competing Estimators

### Estimator 1: Spectral Permutation Synchronization (Pachauri et al. 2013)
Let $\mathbf{R} \in \mathbb{R}^{Kd \times Kd}$ be the block matrix where block $(u, v)$ is the $d \times d$ permutation matrix $\Pi_{uv}^{\text{obs}}$, with $\Pi_{vv} = I_d$ and $\Pi_{vu} = \Pi_{uv}^T$.
1. Compute the top $d$ eigenvectors $U \in \mathbb{R}^{Kd \times d}$ of $\mathbf{R}$.
2. For each node $v$, extract the $d \times d$ block $U_v$.
3. Project $U_v$ onto $S_d$ by solving the linear assignment problem:
   $$\widehat{P}_v^{\text{spec}} = \arg\max_{P \in S_d} \text{Tr}\left( P^T U_v \right)$$
   using the Hungarian algorithm (or minimum-weight bipartite matching).

### Estimator 2: Discrete Maximum A Posteriori (MAP) Gauge Hamiltonian
The maximum agreement configuration minimizes the number of violated edges:
$$\widehat{P}^{\text{discrete}} = \arg\min_{P_0, \dots, P_{K-1} \in S_d} \sum_{(u, v) \in \mathcal{E}} d_{\text{perm}}\left( P_u, \Pi_{uv}^{\text{obs}} P_v \right)$$
where $d_{\text{perm}}(A, B) = \frac{1}{2} (d - \text{Tr}(A B^T))$ is the Hamming distance between permutations.

Equivalently, this maximizes the Gauge alignment energy:
$$\mathcal{E}(P) = \sum_{(u, v) \in \mathcal{E}} \text{Tr}\left( P_u \Pi_{uv}^{\text{obs}} P_v^T \right)$$

### Estimator 3: Cycle-Edge Message Passing (CEMP / Triangle Filter)
For each edge $e = (u, v)$, compute the consensus over all 2-hop paths $u \to w \to v$:
$$S_{uv} = \frac{1}{|N(u) \cap N(v)|} \sum_{w} \text{Tr}\left( \Pi_{uv}^{\text{obs}} (\Pi_{uw} \Pi_{wv})^T \right)$$
Filter edges with low cycle consistency.

---

## 3. The Kill-Test Metric

On a cycle graph of length $K$, there are $K$ edges, $K-1$ of which are uncorrupted and 1 is corrupted by $\tau$.
- Ground truth has $K-1$ satisfied edges and 1 violated edge (the corrupted one).
- Any spurious or smeared solution satisfies $\le K-2$ edges or corrupts multiple clean nodes.
- **Node Recovery Accuracy:**
  $$\text{Acc}(\widehat{P}, P^*) = \frac{1}{K} \sum_{v=0}^{K-1} \mathbb{I}\left( \widehat{P}_v (P_v^*)^{-1} = g_0 \right)$$
  (measured up to a global gauge shift $g_0 \in S_d$).

**Kill Condition:**
If Spectral Synchronization recovers the exact ground truth with $100\%$ accuracy despite the adversarial cycle corruption, OR if Discrete MAP minimization produces degenerate tie-breaks that offer no advantage over Spectral Sync, the claim of a "discrete Ising advantage" on this task is **FALSIFIED (NO-GO)**.
