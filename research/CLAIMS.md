# Formal Scientific Claims Register — Ising Engine

Authority: Project Claim Audit Table  
Rule: Prohibited from upgrading status without published, reproducible evidence.

---

## Claims Audit Matrix

| Claim ID | Formal Claim Statement | Direct Evidence | Independent Verification | Relevant Literature | Counter-Evidence / Observed Boundaries | Confidence | Status |
|---|---|---|---|---|---|---|---|
| `CLM-001` | Exact equality Market Split instances (`ms_03_*`) can be solved to 0 violations in $< 2.1$ s on commodity CPU. | `benchmarks/qoblib/submissions/01-marketsplit/` | Official ZIB verifier `check_marketsplit` (exit code 0) | Cornuéjols & Dawande (1998) | Only tested on 20-variable instances (`ms_03_*`); larger $m=12..15$ instances remain unbenchmarked. | High | **SUPPORTED** |
| `CLM-002` | Unweighted MIS QUBO ($P=2$) algebraically precludes strict 2-flip improvements from any 1-opt local optimum. | `research/cd005_equal_time/RESULT.md` | Formal algebraic proof & 18/18 tied cells | Pardalos & Jha (1992) | Holds strictly for unweighted $P=2$; does not apply to weighted MIS or other penalty multipliers. | High | **SUPPORTED** |
| `CLM-003` | QPBO max-flow presolve preserves global optimum when fixing weak persistencies. | `src/presolve/qpbo.rs` | `tests/test_qpbo.rs` (exhaustive brute-force ground truth) | Boros & Hammer (2002), Kolmogorov & Rother (2007) | Dense anti-ferromagnetic graphs yield 0 fixed variables. | High | **SUPPORTED** |
| `CLM-004` | Memetic PT + 2-opt compresses LABS $N=74$ energy to $E=357$. | `benchmarks/qoblib/world_records/checkpoint_N074.sol` | Official ZIB verifier `check_labs` (`VALID: E=357`) | Knauer (2004), Bernasconi (1987) | Knauer 2004 world record is $E=341$; current result is gap $+16$, not yet a new world record. | High | **SUPPORTED** |
| `CLM-005` | Non-chronological backjumping provides a node-count reduction in general QUBO branch-and-bound. | `research/breakthrough/cd004_recheck/` | Recheck 2026-09-25: 0/120 speedup over chronological BnB | Marques-Silva & Sakallah (1999) | Counterexample found in historical prototype; corrected version is 4.876x slower. | High | **FALSIFIED** |
| `CLM-006` | Freezing replica consensus variables speeds up ground-state convergence on MaxCut. | `research/RESEARCH_INVENTORY.md` (RC-001) | Paired Wilcoxon test on G-set ($p \approx 0$, 0/132 wins at $\lambda=-2$) | Boettcher & Percus (2001) | Replicas lock into sub-optimal local traps. | High | **FALSIFIED** |
| `CLM-007` | Ising Engine demonstrates "Quantum Advantage / 1000x Speedup" over physical quantum annealers. | None (derived from marketing/investor pitch) | None | D-Wave Advantage2 whitepapers, Albash & Lidar (2018) | No rigorous scaling advantage or equal-hardware temperature-calibrated proof exists. | Low | **UNVERIFIED** |
| `CLM-008` | Direct degree-4 HUBO evaluation in SIMD achieves lower Time-To-Solution than optimal quadratization on hard hypergraphs. | `src/solver/engine.rs` | Unit benchmarks; comprehensive benchmark pending | Freedman (2005), Ishikawa (2011) | Systematic cross-solver TTS comparison on standardized HUBO datasets not yet complete. | Medium | **UNVERIFIED** |
