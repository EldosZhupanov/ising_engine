# HYPODIVE Builder Handoff — CD001

## 1. Objective

Mode C: cross-domain mechanism map and minimum exact Ising test of a proposed
structural compression assumption. Success is a defensible falsification decision.
No production integration, benchmark repetition or claim of revolutionary AI.

## 2. Strongest Claims

C1 THEORY: rank-one interface alone does not bound conditional answer count
polynomially. Proven by the family in [MATH](breakthrough/h11/MATH.md).
C2 ENGINEERING: the isolated witness produced 18 rows with three objective forms
agreeing over 16,380 exact assignments. [Result](breakthrough/h11/RESULTS.md).
C3 NOVELTY: exponential energy weighting over a fixed finite set is known Hedge;
parameter-cell memory is known geometry; no new solver identified in this cycle.
[Prior art](PRIOR_ART_MATRIX.md). No global priority assertion.

## 3. Canonical Implementation

[Standalone witness](breakthrough/h11/witness.py): coefficients -> inspect -> run.
Tests and the CLI share this construction but use independent direct, expanded
QUBO and Ising energy evaluators. No caller migration or production changes.

## 4. Architecture Decisions

Standard-library exact integer witness outside src. No ADR needed for this
isolated test. Generic solver baseline would obscure the elementary identity
solution; exhaustive minimization provides the exact control.

## 5. Invariants

Expanded QUBO = direct polynomial; native Ising = four times binary energy;
rectangular cross coefficient matrix nonzero with all 2x2 minors zero; unique
main-family optimum x=z and gap1; tie representatives separated from uniqueness;
source/config hashes unchanged throughout run; existing output refused.

## 6. Test Evidence

`python3 research/breakthrough/h11/witness.py --check`: 3 passed.
`python3 research/breakthrough/h11/witness.py --run`: completed once, 18 rows.
Independent mathematical/source review completed, including separate small
enumeration. Independent finished-code review completed PASS on retry (after a service usage
limit): tests repeated, all source/raw hashes and all18 rows/counts checked, no
HIGH/MEDIUM findings. Two LOW wording clarifications applied to unfrozen reports. Cargo not applicable: no src or
Cargo edits. Documentation validation results are recorded in the cycle ledger.

## 7. Empirical Evidence

[Raw rows](breakthrough/h11/cd001/rows.tsv),
[completion](breakthrough/h11/cd001/complete.json). Constructed cases, no holdout,
no random seeds, no p-values, no performance estimates. At b6 response counts
are64/7/7 for coupled-binary/unit/separable families, respectively.

## 8. Assumptions

Exact finite integer QUBO; z is fixed boundary; deterministic lowest-mask tie
breaking; powers-of-two coefficients have growing bit length. Normalizing the
maximum binary-polynomial coefficient shrinks the gap exponentially. No guarantee
at fixed analog precision. Copying the boundary is allowed and is the strongest
simple competitor, not leakage hidden from a claimed benchmark.

## 9. Known Weaknesses

No frozen evaluated Git commit for this
new witness: independently reviewed files remain a working-tree addition. The hash freeze
records integrity, not historical preregistration. Earlier EXP001/EXP002 drafts,
fundamental_ai and RC027 work remain unrelated and preserved. Literature coverage
is scoped; no proof of an unoccupied scientific direction.

## 10. Simplest Plausible Alternative

Squared equality penalty already known; identity map solves the family. Thus
any apparent speedup against a generic Ising solver would be explained by the
construction. The useful output is rejection of a structural assumption.

## 11. Suggested Kill-Tests

Verify arbitrary b algebraically; inspect conversion and highest cross coefficient;
try to infer hardness from answer count (identity falsifies that inference);
check that unit-weight degeneracy is not reported as unique answers. Fixed-rank
bounded-margin bounds are different claims and cannot rescue rank-only H11.

## 12. Freeze Point

Branch feat/solver-research-upgrades; base HEAD 526ddd3; evaluated/result commit:
frozen in this CD001 commit (independently reviewed working-tree evidence).
[Content freeze](breakthrough/h11/cd001/freeze.json) names exact script/MATH/plan
hashes, environment and b1..6 across three families; no seeds/holdout.
Rows hash c41968e84e64beee0c91c7910fa003affc93eb0b30e3ca7c4579ccec1f5a37ae.
Claim IDs C1–C3; reproduction uses --check or a separate copy of the frozen
witness because --run refuses existing output. Builder verdict **READY: independently reviewed and Git-frozen**.
Scientific decision on rank-only H11 is NO-GO; no production/publication GO.
