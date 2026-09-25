# Fundamental AI Research: Higher-Order Tensor Energy Dynamics

**Directory:** `research/fundamental_ai/`  
**Status:** Active Autonomous Investigation  
**Governance:** Governed by `ising_engine` scientific invariants and adversarial falsification protocols.  

---

## 1. Executive Mission

The objective of this research track is to rigorously investigate whether higher-order tensor energy dynamics ($p \ge 3$) can serve as a genuine, scalable computational primitive for artificial intelligence—distinct from standard feed-forward Transformers, MLPs, classical pairwise Hopfield networks, and established Dense Associative Memories (DAM).

We operate strictly under an **Adversarial Falsification Methodology**:
- We do **not** attempt to prove our ideas work.
- We actively construct reductions to prior art (Tests A through H in `NOVELTY_LEDGER.md`).
- We enforce strict **parameter budget normalization** ($\text{Capacity per Parameter}$ and $\text{Capacity per Byte}$).
- We mandate comparisons against strong classical and modern baselines (Pseudoinverse Hopfield, Polynomial DAM, Modern Hopfield, and Nearest-Neighbor Oracles).

---

## 2. Directory Architecture

```
research/fundamental_ai/
├── README.md                      # Primary overview and architectural map
├── PRIOR_ART.md                   # Exhaustive literature audit (17 paradigms categorized)
├── NOVELTY_LEDGER.md              # Living adversarial ledger and reduction proofs
├── HYPOTHESES.md                  # Preregistered binding mathematical hypotheses
├── MATH.md                        # Formal analytical derivations and energy theorems
├── THEORY_SIGNAL_CROSSTALK.md     # Exact SNR and statistical mechanics interference analysis
├── EXPERIMENT_PROTOCOL.md         # Frozen protocol for EXP-TEN-001
├── EXP_TEN_001_RESULT.md          # Full empirical results and falsification verdicts
├── EXP_TEN_002_PROTOCOL.md        # Protocol for hard falsification of Model C vs strong baselines
├── NEGATIVE_RESULTS.md            # Catalog of conclusively falsified hypotheses
├── ANOMALIES.md                   # Monitored empirical anomalies and scaling transitions
├── RESEARCH_GRAPH.md              # Mermaid knowledge topology of hypotheses, theories, and outcomes
├── NEXT.md                        # Ordered research queue and immediate next actions
├── Cargo.toml                     # Standalone Rust workspace configuration
├── src/                           # High-performance Rust energy engine
│   ├── lib.rs                     # Library root
│   ├── types.rs                   # SpinState, ContinuousState, Hyperedges, CPTensor3
│   ├── models.rs                  # Models M0 (Pairwise), M1/M2 (Dense), M3 (Sparse), M_CP (LowRank)
│   ├── learning.rs                # Hebbian, budgeted hyperedge selection, CP factorization
│   ├── dynamics.rs                # Dynamics D0 (Greedy), D1 (Glauber), D2 (Parallel), D3 (SA), D4 (Langevin), D5 (Momentum)
│   └── experiment.rs              # Parameter evaluation and statistical metric collection
├── tests/
│   └── brute_force_math_tests.rs  # 6/6 Brute-force float-level verification tests
└── benchmarks/
    └── exp_ten_001.rs             # Multi-threaded release benchmark executable
```

---

## 3. Core Models Under Investigation

1. **Model A (M0 — Baseline Pairwise Hopfield):**
   $$E(s) = -\sum_{i < j} J_{ij} s_i s_j, \quad K_A = \frac{N(N-1)}{2}$$
2. **Model B (M3 — Budget-Matched Sparse 3-Body Hyperedge Memory):**
   $$E(s) = -\sum_{e \in \mathcal{E}_3} w_e s_i s_j s_k, \quad |\mathcal{E}_3| = K_A$$
3. **Model C (M_CP — Symmetric CP 3-Body Tensor Memory / Polynomial DAM $n=3$):**
   $$E(s) = -\frac{1}{6 N^2} \sum_{r=1}^P (\xi^r \cdot s)^3 + O(N), \quad K_C = N \times P$$
4. **Model D (M3 — Budget-Matched Sparse 4-Body Hyperedge Memory):**
   $$E(s) = -\sum_{e \in \mathcal{E}_4} w_e s_i s_j s_k s_l, \quad |\mathcal{E}_4| = K_A$$

---

## 4. Summary of Empirical Findings (EXP-TEN-001)

- **Classical Limit Reproduced:** Pairwise Hopfield collapses at $P > 0.138N$ across all $N \in \{128, 256, 512\}$.
- **Sparse Hyperedge Falsification:** Fixing hyperedges to $O(N^2)$ budget fails to scale capacity beyond $O(N)$.
- **Parity Cliff Discovered:** 4-body interactions suffer a catastrophic collapse at noise $\ge 30\%$ due to even-degree sign invariance.
- **Model C Performance & Novelty Classification:** Model C achieves 100% recovery up to $P \approx N$ and noise 20%, with SNR scaling as $\sqrt{N}$. However, adversarial reduction proves this is **algebraically isomorphic to Polynomial Dense Associative Memory ($n=3$)**, classifying it as `KNOWN MECHANISM`.
