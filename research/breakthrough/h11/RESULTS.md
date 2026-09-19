# H11/CD001 result — rank alone does not bound conditional answer count

Date 2026-09-17. Exact constructed witness, not a held-out benchmark. All
construction details and expected outcome were known algebraically before running.
Source/config integrity: [freeze](cd001/freeze.json).
Raw observations: [18 rows](cd001/rows.tsv); [completion](cd001/complete.json).
Rows SHA256: c41968e84e64beee0c91c7910fa003affc93eb0b30e3ca7c4579ccec1f5a37ae.

Commands run:
- python3 research/breakthrough/h11/witness.py --check: 3 tests pass.
- python3 research/breakthrough/h11/witness.py --run: 18 rows, 16,380 exact energy
  evaluations, no mismatches among direct square/expanded QUBO/native Ising.

| Boundary bits b | Coupled binary powers: distinct answers | Same rank, unit weights | Same rank, separable thresholds | Normalized coupled gap |
|---:|---:|---:|---:|---:|
| 1 | 2 | 2 | 2 | 1/2 |
| 2 | 4 | 3 | 3 | 1/8 |
| 3 | 8 | 4 | 4 | 1/32 |
| 4 | 16 | 5 | 5 | 1/128 |
| 5 | 32 | 6 | 6 | 1/512 |
| 6 | 64 | 7 | 7 | 1/2048 |

Every interface has rank1. In the main family every boundary has a unique optimum
with unnormalized gap1. Unit weights introduce degeneracy (only all-zero and
all-one boundaries have unique optima); its counts use deterministic tie-breaking.
Separable thresholds have unique optima. The separable family is an analytical
structural contrast, not a single-component ablation: unary coefficients, rows of
the cross interface, and boundary-only terms also differ from the main family.
These comparisons do not isolate the causal effect of interior couplings and do
not support an empirical claim about random Ising instances.

Verdict **NO-GO** for rank-only response-count compression. Mathematical proof
holds for every b>=1; finite enumeration is a consistency check of the implementation.
The same witness has identity-map optimizer O(b); exponential answer count does
not imply hardness. Constant normalized precision/margin is outside this rejection.
No new solver, no end-to-end speedup, no demonstrated AI application, no claim of
scientific priority. The square-penalty encoding is known.

Independent mathematical/source review by mechanism_review confirmed formulas,
rank conventions, precision caveat, non-hardness and known Hedge/MPC connections.
That reviewer independently enumerated b1..6 before receiving implementation.
The first finished-code review attempt hit a service usage limit. On retry,
independent review completed with PASS and no HIGH/MEDIUM findings: 3 tests
repeated, all frozen source/raw hashes matched, 18 unique rows and 16,380 counts
checked against formulas; --run was not repeated. Two LOW wording clarifications
were applied here and in the intake without changing frozen artifacts. Production
source unchanged. Git freeze completed in this CD001 commit; content hashes verified.
