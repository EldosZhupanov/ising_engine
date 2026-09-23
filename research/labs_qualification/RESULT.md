# LABS-Q001: Qualification and Baseline Comparison — Final Results

- **Protocol:** [`PROTOCOL.md`](PROTOCOL.md)
- **Evaluated Commit:** `cd04ff13137264a01f5e03f90bc093f5be46f112`
- **Date:** 2026-09-23
- **Design:** Single-worker, equal 10.0-second wall budget per cell, 10 independent seeds (730001..730010) across $N \in \{40, 50, 60\}$.
- **Independent Verification:** All 60 final solutions checked with official ZIB QOBLIB verifier (`check_labs.rs`).
- **Qualification Verdict:** **NOT QUALIFIED** (`pt_qualified: false`).

---

## 1. Executive Summary

The preregistered LABS-Q001 campaign was executed across 60 sequential runs (30 runs for our Parallel Tempering prototype `pt`, 30 runs for the established specialist `lMAts` by Bošković & Brest).

1. **Protocol Integrity:** 60/60 runs completed strictly within budget. Zero checker failures, zero malformed witnesses, zero budget-violation errors (`errors: []` on all rows).
2. **$N=40$ Performance:** Our `pt` prototype hit the exact Packebusch & Mertens theoretical global optimum ($E=108$) in **7 out of 10 runs** (median 108.0, times ranging from 1.63 s to 9.78 s). The baseline `lMAts` achieved 10/10 hits. Paired score: 7 ties, 3 `lMAts` wins.
3. **Scaling Boundary ($N=50, 60$):**
   - At $N=50$ (optimum 153), `pt` reached best energy 161 (median 185.0, 0/10 hits), whereas `lMAts` hit optimum 153 in 5/10 runs (median 157.0).
   - At $N=60$ (optimum 218), neither arm hit the global optimum within 10 seconds; however, `lMAts` approached much closer (median 256.0, best 238) than `pt` (median 282.0, best 270).
4. **Qualification Standard:** The operational threshold required $\ge 8/10$ optimum hits at each length. Because `pt` achieved 7/10 at $N=40$ and 0/10 at $N=50, 60$, the prototype is **not qualified** to reliably challenge the $N=67$ world record under current algorithmic parameters.

---

## 2. Quantitative Results Ledger

| Metric / Dimension | $N = 40$ (Target: 108) | $N = 50$ (Target: 153) | $N = 60$ (Target: 218) |
|---|---|---|---|
| **PT Optimum Hits** | **7 / 10** (70.0%) | 0 / 10 (0.0%) | 0 / 10 (0.0%) |
| **PT Wilson 95% CI** | [0.397, 0.892] | [0.000, 0.278] | [0.000, 0.278] |
| **PT Energy Median (Range)** | **108.0** [108, 116] | 185.0 [161, 201] | 282.0 [270, 290] |
| **PT Time to Optimum** | 1.63 s – 9.78 s (mean 4.03 s) | N/A | N/A |
| **lMAts Optimum Hits** | **10 / 10** (100.0%) | **5 / 10** (50.0%) | 0 / 10 (0.0%) |
| **lMAts Wilson 95% CI** | [0.722, 1.000] | [0.237, 0.763] | [0.000, 0.278] |
| **lMAts Energy Median (Range)** | **108.0** [108, 108] | **157.0** [153, 161] | **256.0** [238, 266] |
| **lMAts Time to Optimum** | 0.45 s – 8.17 s (mean 2.83 s) | 1.64 s – 6.01 s (mean 3.75 s) | N/A |
| **Paired Score (PT vs lMAts)** | 0 PT wins / 7 ties / 3 lMAts wins | 0 PT wins / 0 ties / 10 lMAts wins | 0 PT wins / 0 ties / 10 lMAts wins |

---

## 3. Scientific Findings & Lessons

1. **Confirmation of PT Correctness:**
   The corrected Boltzmann replica-exchange formula (`exchange_cost`) works robustly: on $N=40$, `pt` consistently reaches the global minimum ($E=108$) in 70% of runs, proving that the $O(N)$ incremental delta engine and PT ladder navigate the rugged landscape effectively on moderate dimensions.
2. **Mechanism Gap on Rugged Landscapes:**
   The comparison clearly illustrates the finding from architecture census RC-022: on larger problem sizes ($N \ge 50$), pure physical thermal sampling (Parallel Tempering) with local 1-opt quenching gets trapped in deep golf-course metastable valleys. The specialized genetic recombination (crossover) and tabu recency memory in `lMAts` allow it to bridge across valleys far more effectively than purely thermal swaps.
3. **Implications for the $N=67$ World Record:**
   Blindly launching our current PT solver on $N=67$ for days would be scientifically naive: since `lMAts` is decisively superior at $N=50$ and $N=60$, attempting to beat Knauer's record ($E=241$) requires integrating population crossover or tabu memory into our solver first.
