# HUBO-C001 Builder Handoff

## 1. Objective

Mode C: reproducible research evidence. Compare native and quadratic input using
unchanged MSC and independent OpenJij at matched warm-runtime budgets. Scientific
success is the conjunction in [protocol.md](protocol.md); engineering success is
a complete, independently checked result including an adverse/null outcome.
No production changes, Laya training or world-record search.

## 2. Strongest Claims

C1 — SUPPORTED empirical: MSC native improves endpoint energy over the frozen
dimod/MSC quadratic control, 100/100 paired cells, median instance gain
0.13203125, Holm p=0.005859375. Scope is this corpus, budget and settings.

C2 — INCONCLUSIVE competitive advantage: MSC native and OpenJij native tie
100/100. The registered overall decision is NOT_QUALIFIED_FOR_ADVANTAGE.
No equivalence or optimum claim follows. [Result](RESULT.md) and
[raw summary](run/summary.json) contain all three preregistered contrasts.

## 3. Canonical Implementation

`generate_cases.py` -> `cases.json`; `campaign.py` supervises `worker.py`, which
uses either `research/examples/hubo_compare.rs` -> unchanged `engine::step`,
or OpenJij `SASampler.sample_hubo`. Quadratic arms use dimod.make_quadratic.
`test_campaign.py` and Rust example tests enforce instrumentation invariants.
Old seed-dropping benchmark adapters are not used. No production migration.

## 4. Architecture Decisions

Keep experiment orchestration outside production, original polynomial as the
objective source, and constants outside the native model. No new architectural
invariant/ADR. The simple alternative, comparing only a quadratic external
baseline, was rejected because it cannot separate representation from solver.

## 5. Invariants

- Spin and expanded 0/1 energy agree: independent `score`, input-expansion tests.
- Quadratic minimum matches original: exhaustive n=6 auxiliary enumeration.
- MSC incremental energies match integer recomputation: fixed-seed Rust test.
- Original witnesses determine quality: all events decoded/recomputed, including
  late events; only receipts within two seconds count.
- Errors are preserved and stop the suite: launch/early-exit/tail/deadline tests.
- Process group terminates within the registered 0.2-second tolerance; no witness
  after two seconds is credited: group termination and process-group test.
- Inferential unit is instance: non-vacuous primary aggregation and Holm tests.

## 6. Test Evidence

[instrument_checks.json](instrument_checks.json): 14 Python tests, two Rust
example tests, example Clippy/build, fmt, memory validation and diff check PASS.
Independent pre-data review re-ran Python/Rust tests and accepted the final
implementation. Smoke: 8/8 valid, independently checked 25 incumbents and eight
final witnesses. Production full-suite rerun N/A: no production source changes.
Final independent audit PASS is preserved in [audit/result.json](audit/result.json); result/editorial checks are separate
from the frozen instrument and do not add solver observations.

## 7. Empirical Evidence

Raw commit `a196745`, [manifest](raw_sha256.json), [run/](run/), [smoke/](smoke/).
400 main cells, 3,487 incumbents, 400 final witnesses; no missing/replacement cell.
Analysis: `benchmark-env/bin/python3 research/experiments/hubo_comparison/campaign.py analyze research/experiments/hubo_comparison/run`.
Paired normalized gains are averaged over ten seeds within ten instances;
exact sign-rank and three-comparison Holm correction are frozen. All native
endpoints coincide, so the native contrast does not discriminate.

## 8. Assumptions

Synthetic signed spin coefficients and expansion are exact integers; global
penalty suffices but is not tight. Statistical sign exchangeability/symmetric
differences is assumed, not established by ten synthetic instances. Warm worker
imports are excluded equally; Rust child startup is charged. Host timing is
exploratory, single-worker, unpinned WSL2. Optimum targets are unavailable.

## 9. Known Weaknesses

Small n=32 corpus and two seconds; both native methods have zero endpoint
variation across seeds within each instance. Untuned schedules, a conservative
global penalty and 92–154 expanded variables limit generalization. Same rule
for temperature endpoints produces different numeric schedules after reduction.
The driver is not the complete UltimateSolver. No full system-image provenance:
source and named package/binary hashes are recorded, not every OS dependency.
Frozen analyzer validates original absolute command paths; full independent audit
also requires recorded runtime paths and local mtimes. Portable full audit replay
is not established. Timing endpoints need not replay
bit-identically. No confirmatory campaign beyond this screen is implied.

## 10. Simplest Plausible Alternative

Both native solvers reach the same easy-at-this-budget plateau. Conservative
penalties and auxiliaries make the quadratic configurations harder to search.
That explains the observations without a new or superior MSC algorithm.
Global optimality of the common plateau is unknown.

## 11. Suggested Kill-Tests

First recompute every original witness and exact per-instance contrast without
importing `campaign.py`; a mismatch refutes C1/C2 evidence integrity. Check raw
receipts, full cell set, source/package hashes and the frozen conjunction.
On a future, separately registered corpus, a tight quadratic control matching
native quality would narrow C1 further; consistent native separation would be
new evidence, not a reinterpretation of these 100 ties. Do not tune/retry these
exposed cases to manufacture a win.

## 12. Freeze Point

- Branch: `feat/solver-research-upgrades`.
- Protocol: `feb9a76`; evaluated code/analysis: `23d300f545ce027e7ad56eac7e895e920b804712`.
- Results commit: `a196745`; all raw SHA-256 values: `raw_sha256.json`.
- Config and seed/split manifest: `protocol.md`, `cases.json`, `run/environment.json`.
- Canonical implementation/test commands: sections 3 and 6, `instrument_checks.json`.
- Primary claims: C1/C2, section 2; limitations: sections 8–10.
- Independent review/audit: `audit/`; no external publication or model training.

Treat these strongest claims as falsifiable hypotheses. Do not defend them.
Find the cheapest decisive kill-test first. This package closes this Builder
cycle; further research requires a new prospective protocol and independent data.
