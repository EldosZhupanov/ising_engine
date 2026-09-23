# LAYA-001: Semantic Decisions with Verified Discrete Reconciliation — Pilot Results

- **Protocol:** [`PROTOCOL.md`](PROTOCOL.md) (Preregistered in `26d4c0a`)
- **Instrument:** Commit `96b2ace` (Clean worktree execution)
- **Status:** Terminal Pilot Complete
- **Date:** 2026-09-23

---

## 1. Executive Summary

The preregistered LAYA-001 pilot successfully evaluated the end-to-end integration between the open-weights **Laya-1B System 1 Decision Model** and the **UltimateSolver** (incorporating CD005 Edge 2-Opt and exact presolve).

Across 24 independent synthetic manufacturing decision groups (192 binary choices), the raw neural model suffered from systematic constraint violations (26 logical contradictions). Reconciling the semantic logits via our penalized Ising/QUBO Hamiltonian completely eliminated all violations, corrected 19 erroneous decisions without damaging a single valid prediction, and raised perfect group-level decision accuracy from **25.0% to 62.5%** (a 2.5× improvement).

All mathematical correctness gates passed with zero mismatches.

---

## 2. Quantitative Results Ledger

| Metric | Raw Laya ($p \ge 0.5$) | All-Zero Baseline | Greedy Feasible Pass | UltimateSolver (Ours) | Reduced Exact (Oracle) |
|---|---|---|---|---|---|
| **Bit Accuracy** | 84.38% (162/192) | 70.31% (135/192) | 91.15% (175/192) | **94.27% (181/192)** | **94.27% (181/192)** |
| **Constraint Violations** | **26 violations** | 0 | 0 | **0 violations** | **0 violations** |
| **Feasible Groups** | 10 / 24 (41.7%) | 24 / 24 (100%) | 24 / 24 (100%) | **24 / 24 (100%)** | **24 / 24 (100%)** |
| **Perfect Groups (8/8 bits)** | 6 / 24 (25.0%) | 1 / 24 (4.2%) | 11 / 24 (45.8%) | **15 / 24 (62.5%)** | **15 / 24 (62.5%)** |
| **Bits Recovered (vs Raw)** | 0 (baseline) | 30 | 19 | **19** | **19** |
| **Bits Damaged (False Reversal)** | 0 (baseline) | 57 | **6 damaged** | **0 damaged** | **0 damaged** |
| **Max Energy Gap to Oracle** | 58.0267 | 9.4119 | 3.0152 | **0.0000** | **0.0000** |

---

## 3. Key Findings

1. **Perfect Energy Optimization (Zero Gap to Brute-Force):**
   Across all 24 instances, `UltimateSolver` found states whose energy identically matched the exact brute-force oracle (`max_energy_gap = 0.0000`).
2. **Elimination of "Greedy Damage":**
   A standard greedy post-processing heuristic was able to restore feasibility, but at the cost of damaging 6 correctly predicted bits. The global Ising optimization avoided all 6 false damages by considering collective multi-variable trade-offs.
3. **Presolve Complete Graph Elimination:**
   The production `full_presolve` engine completely resolved all 8 variables analytically in all 24 groups (`full_fixed_per_group = 8/8`), reducing the residual conflict search space from 6,144 enumerated states to **0**.
4. **Descriptive Baseline (Brier Score):**
   Raw Laya probability calibration Brier score on this synthetic domain was $0.1009$.

---

## 4. Methodological Boundaries and Disclosures

- **Synthetic Domain:** Sentences and constraints were generated from literal templates (manufacturing operations). Generalization to unstructured, noisy real-world text requires external human-annotated corpora.
- **Model Calibration:** Laya was executed uncalibrated in zero-shot mode without domain fine-tuning.
- **Timing:** Total inference time was 130.0 seconds (dominated by PyTorch CPU forward passes on 192 prompts). Rust solver execution was sub-second across all 24 groups.
