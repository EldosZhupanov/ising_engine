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

---

# ER-001 Builder handoff — 2026-09-27

## 1. Objective

Mode C: build a verified small real-data entity-reconciliation pipeline after
APP-01 triage. [Result](experiments/entity_resolution/RESULT.md) separates
implemented behavior, development annotation quality and native search fidelity.

## 2. Strongest Claims

ER-C1 REPRODUCED engineering: one complete-score partitioner, exact up to eight
records, with independently verified cubic encoding and truth-free input boundary.
ER-C2 REPRODUCED in this development sample: nine of twelve groups require
consistency repair; exact/greedy F1=0.705314 versus pairwise0.677083.
ER-C3 NOT SUPPORTED: additional search benefit. Greedy equals exact on all twelve
blocks; registered H2 fails. Native reaches an optimal witness in120/120 cells,
which establishes H3 fidelity only. No semantic guarantee or novel method.

## 3. Canonical Implementation

[core.py](experiments/entity_resolution/core.py) owns score/partition/objective
semantics and the small JSON CLI. [prepare.py](experiments/entity_resolution/prepare.py)
owns pinned WDC intake. [pilot.py](experiments/entity_resolution/pilot.py) owns
run/analysis, invoking unchanged Q002 supervision and native `hubo_compare.rs`.
The older synthetic LAYA-001 and its bridge are preserved, not silently repurposed.

## 4. Architecture Decisions

Research-only standard-library Python; reuse native MSC through its existing
example boundary. No Rust production/API/dependency migration; both solver families
intact. Exhaustive set partitions are the strong optimum control at this size.
Routine scoped choices need no new ADR. Laya runtime/training deferred explicitly.

## 5. Invariants

Label-free blocking/scoring/search; complete within-block edge semantics; canonical
partition ties; triangle penalty excludes inconsistent global minima; exact integer
binary/spin energy equality with factor8; every event verified; late witnesses
never credited; fallback never counted as native success; frozen inputs/sources,
command/seed identity, sequential clocks, exact raw inventories and failure retention.

## 6. Test Evidence

Fourteen Python tests PASS, unchanged Rust example2/2 PASS, release build and
targeted Clippy PASS. Input regeneration matches SHA exactly. Independent final
preflight PASS after seed/inventory/environment/F1-boundary issues were repaired
before any empirical outcome. No full production cargo rerun: no production change.

## 7. Empirical Evidence

[Main archive](experiments/entity_resolution/run/main/summary.json):120 cells,
1,344 checked incumbents; [smoke](experiments/entity_resolution/run/smoke/summary.json):
one separate cell/five incumbents. H1PASS, H2FAIL, H3PASS. Five selected fallback
endpoints have separate on-time native optimal witnesses. Equal-energy ties cause
native-wrapper F1 variation; they do not overturn H2. No statistics of new worlds
are inferred from repeated seeds.

## 8. Assumptions

WDC identifier-derived reference labels; no model trained; fixed lexical scores.
Only72/2,841 offers and87/8,471 source-positive pairs enter selected blocks.
Cross-block pairs unresolved. Test/validation archive members remain unread.
CPU kernel and Python scorer, one worker; no GPU/hardware-energy claim.

## 9. Known Weaknesses

Development selection, small blocks, no source-disjoint holdout and no calibrated
probabilities. Greedy already solves these objectives. No demonstrated Laya value,
presolve benefit, industrial scaling, end-to-end latency gain or field-wide novelty.
Independent auditor checks the recorded runtime binary; this is not a clean-machine
rebuild demonstration. Dataset archive remains external; source URL and hashes
support reconstruction rather than redistributing raw text.

## 10. Simplest Plausible Alternative

Deterministic greedy merging suffices here and chooses the same twelve partitions
as exact enumeration. Better semantic quality than naive threshold/closure may
come entirely from this simple consistency treatment, without expensive search.

## 11. Suggested Kill-Tests

Run the preserved [independent auditor](experiments/entity_resolution/independent_audit.py)
for `main` and `smoke`; it recomputes without importing the pilot. Mutate energies,
seed identity, labels, source hashes or late timestamps in a disposable copy:
verification must refuse it. For any future gain, keep the same scorer/decoder
controls and separate blocking coverage from within-block F1; use new data and
new protocol. Do not retune the opened ER-001 sample into a success.

## 12. Freeze Point

Branch `feat/solver-research-upgrades`; evaluated code/protocol `96b0c23`, results
`a91481c`. Config/seeds/claims: [protocol](experiments/entity_resolution/protocol.md);
source/input/environment hashes: [environment](experiments/entity_resolution/run/main/environment.json);
raw manifest: [manifest](experiments/entity_resolution/run/main/raw_manifest.json).
Independent reviewer/source/report: [audit](experiments/entity_resolution/independent_audit.json),
**PASS**, all147 main raw hashes,203 source/Git hashes and the binary, all120 cells
and1,344 incumbents. Source script and reproduction commands are retained.
Builder verdict **READY FOR FALSIFICATION; independent artifact audit PASS**.
Scientific decision: capability retained, added-search-value claim not supported.
Next design: fresh entity-disjoint scorer/blocking qualification with identical
simple/exact decoders. No automatic new campaign or training.
