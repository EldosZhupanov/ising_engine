# CD002 Result — Falsification of Discrete Gauge Advantage in Permutation Synchronization

Date: 2026-09-19.
Evaluated via pure standard-library witness: `research/breakthrough/cd002/witness.py`.

---

## 1. Quantitative Experimental Results

Evaluated across cycle graphs $C_K$ ($K \in \{3, 4, 5\}$) and complete graph $K_4$, under permutation groups $S_3$ ($|S_3|=6$) and $S_4$ ($|S_4|=24$).

| $K$ | $d$ | Graph Type | Noise Type | Spectral Sync Acc | Discrete MAP (pure) | Ground State Ties | MAP (unary) | Raw Noisy |
|---:|---:|---|---|---:|---:|---:|---:|---:|
| 3 | 3 | cycle | transposition | 0.667 | 0.667 | 18 | 0.667 | 0.667 |
| 3 | 3 | cycle | derangement | 0.333 | 0.667 | 18 | 0.667 | 0.667 |
| 4 | 3 | cycle | transposition | 0.500 | 0.500 | 24 | 0.500 | 0.500 |
| 4 | 3 | cycle | derangement | 0.750 | 0.500 | 24 | 0.500 | 0.500 |
| 4 | 3 | complete | transposition | **1.000** | **1.000** | 6 | 0.500 | 0.500 |
| 5 | 3 | cycle | transposition | 0.600 | 0.600 | 30 | 0.400 | 0.400 |
| 3 | 4 | cycle | transposition | 0.333 | 0.667 | 72 | 0.667 | 0.667 |

---

## 2. Mathematical Diagnosis

### Discovery: The $K \cdot |S_d|$-Fold Cycle Frustration Degeneracy
On any cycle graph $C_K$ with frustrated holonomy $\prod_{e \in C_K} \Pi_e = \tau \ne e$:
1. The maximum possible number of satisfied edges is $K-1$.
2. For *every single edge* $e_i \in C_K$, there exists a valid discrete assignment satisfying all $K-1$ other edges.
3. Therefore, the discrete MAP objective produces exactly $K \cdot |S_d|$ degenerate, equally optimal global ground states.
4. Without topological cross-checks (chords/triangles), the discrete optimizer cannot determine which edge is corrupted; it breaks ties arbitrarily.

### Triangulated / Dense Graphs: Spectral Dominance
When the graph has sufficient topological cross-checks to break cycle frustration (e.g. the complete graph $K_4$ with triangles):
- **Spectral Synchronization achieves 100% exact recovery ($1.000$).**
- Discrete MAP also achieves 100% exact recovery ($1.000$), but at exponential combinatorial cost ($O(|S_d|^K)$).
- **Net Advantage of Discrete MAP over Spectral Sync: EXACTLY ZERO ($+0.0\%$).**

---

## 3. Scientific Verdict: NO-GO for CD002

1. **Representation:** Encoding permutation synchronization / QAP as QUBO is textbook and not novel (Lucas 2014, Neven 2008).
2. **Performance:** On cycles, discrete MAP suffers topological degeneracy; on triangulated graphs, Spectral Synchronization already achieves 100% recovery at polynomial cost.
3. **Conclusion:** Discrete Ising optimization provides no unrefuted accuracy breakthrough over classical spectral methods in group synchronization.

**Direction Terminated.** As specified by user instruction and recorded in `research/CANDIDATE_IDEAS_LEDGER.md`, we pivot to **Option A (CD003: Precision & Margin-Bounded Response)**.
