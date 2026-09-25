# HYPODIVE Builder Handoff — CD004-R

## 1. Objective

Mode C research evidence: test whether explicit certified soft-conflict cores
repair CD004 exactness and improve matched chronological branch-and-bound on
small integer Ising graphs. Success required 120/120 exactness and the registered
practical-speed gate. No production solver or external benchmark change.

## 2. Strongest Claims

C1, deterministic counterexample: the historical CD004 backjump solver can
return a nonoptimal incumbent; [result](RESULT.md) and calibration test.
C2, scoped empirical result: corrected core caching is exact on all 120 frozen
instances, but has 0 node wins and 0 time wins. C2 does not generalize to all
graph families or implementations. No novelty or production-speed claim.

## 3. Canonical Implementation

[`instrument.py`](instrument.py) implements one DFS with optional certified
core cache; [`test_instrument.py`](test_instrument.py) checks it. The historical
[`../cd004/witness.py`](../cd004/witness.py) remains unchanged and is the
source of the original energy and lower-bound functions. There are no callers
in `src/` to migrate.

## 4. Architecture Decisions

No ADR: isolated research prototype. Direct core storage preserves exactness;
legacy target-only backjumping failed. Reverse this no-integration decision only
after a new exact implementation beats a matched baseline at equal cost.

## 5. Invariants

Every core is a subset of the current branch with `LB(core)>=incumbent` at
certification; later incumbents only decrease. Both siblings are explored
unless their own bound or a saved core prunes them. Every returned spin vector
has length N, valid signs and independently recomputed ground-state energy.
Enforced by `instrument.py` and the 120-row primary gate.

## 6. Test Evidence

`python3 -m unittest discover -s research/breakthrough/cd004_recheck -p
'test_*.py' -v`: 2 tests PASS (one legacy counterexample, 80 small random-field
cases). `python3 -m py_compile` PASS. Holdout command completed 120 rows,
0 failures; raw and analysis hashes are in [result](RESULT.md). No root `src/`
files changed, so Rust quality gates are not applicable.

## 7. Empirical Evidence

[Raw JSONL](raw.jsonl) and [generated analysis](analysis.json) from one paired
120-instance campaign. 60 initial incumbents were nonoptimal; both methods
found the exact optimum in 120/120. Candidate/baseline median ratios: nodes
1.000, bound calls 3.946, wall time 4.876; 0/120 time wins. No holdout fitting.

## 8. Assumptions

Integer symmetric zero-field couplings from the legacy SK and random-sparse
generators; Python 3.14.4 and standard library. Exhaustive search at N<=12 is
the ground-truth oracle. Three timing repetitions alternate order per the
[amendment](AMENDMENT_1.md). User authorized scoped local research; no external
publication or deployment.

## 9. Known Weaknesses

Small synthetic graphs, simple lower bound, one-start incumbent. Python wall
times do not estimate optimized Rust performance. The old seven benchmark
cases and counterexamples were known before this new protocol. Generator setup
exceptions could abort a run before a failure row; none occurred. Unrelated
untracked workspace files prevent a globally clean working tree.

## 10. Simplest Plausible Alternative

Chronological branch-and-bound with the same lower bound and incumbent is
exact and faster here. It uses fewer bound calls and visits the same nodes.

## 11. Suggested Kill-Tests

For any future CD004 redesign: first exhaustively compare energies on a
nonoptimal-incumbent small case; then test whether a valid learned constraint
reduces total work against the same baseline. A node reduction without exactness
or equal-cost advantage rejects the claim.

## 12. Freeze Point

Branch `feat/solver-research-upgrades`; protocol commit `506e61d`, amendment
`827fa51`, evaluated code/analysis commit
`da2ba4c2cace116eecc7d4200a78fdb3a1077409`, results commit
`eed279604039a60592264b222c040dcb50570caa`. Configuration and seed
manifest: [protocol](PROTOCOL.md) + [amendment](AMENDMENT_1.md). Canonical code:
`instrument.py`; claim IDs C1/C2 above. Raw: `raw.jsonl`; analysis:
`instrument.py analyze --input research/breakthrough/cd004_recheck/raw.jsonl`.
Test command and environment: §6/§8. Evidence hashes: [result](RESULT.md).
Separate manifest: N/A; SHA-256 hashes in result, immutable protocol hashes in
`memory/BINDING_SHA256`. Weaknesses: §9. Builder verdict: scoped NO-GO for the
current acceleration mechanism; package READY FOR INDEPENDENT FALSIFICATION.
