# EXP-007W frozen research handoff

## 1. Objective

Test the marginal value of the existing CD005 strict two-flip finisher on
vertex-weighted MIS, first as post-presolve 1-opt escape headroom and then as
best feasible weight at equal two-second wall-clock budgets.

## 2. Strongest Claims

- H1 supported on this corpus: 58/60 post-presolve, post-1-opt states had an
  independently verified improving free edge-pair move; no variables were
  fixed by presolve.
- H2 practical NO-GO: 0 wins, 60 ties, 0 losses for the best result in 60
  matched-time cells, p=1.0 by the registered exact sign test.
- Secondary only: 268 of 5,934 same-seed completed-solve pairs improved and
  none worsened; this did not change the campaign best.

## 3. Canonical Implementation

[generate_instances.py](generate_instances.py) and [instances.json](instances.json)
define the six frozen inputs. [exp007_weighted_mis.rs](../../../src/bin/exp007_weighted_mis.rs)
calls the existing production toggle and records full states.
[analyze.py](analyze.py) independently checks every state and delta;
[run.py](run.py) and [commands.sh](commands.sh) verify input SHA-256, launch once
and write metadata. Production solver files and public APIs were untouched.

## 4. Architecture Decisions

Keep this study in a standalone binary and research scripts. Use the existing
`QuboModel`/`CsrMatrix` bridge into `UltimateSolver`; do not move weighted-MIS
policy into solver families. Use vertex rewards `1..=10` and fixed edge penalty
11 so any complete 1-opt local minimum is independent. Retain production
presolve and decomposition in both arms.

## 5. Invariants

Both arms use identical model, solver parameters and solve-seed stream; only
`with_2opt` differs. A solution counts only if it finishes within its own
two-second deadline. Every counted state must be binary, edge-feasible and
independently recompute to energy `-weight`. All 60 cells and all 60 structural
starts are retained, including failures. The [protocol](protocol.md) specifies
the exact GO/NO-GO gate and forbids retries and posthoc exclusions.

## 6. Test Evidence

Before any holdout solver access, 3 Python tampering/analysis tests and the
full required Rust gates passed. The two-vertex calibration exercised a strict
weighted two-flip escape. A read-only reviewer found and helped correct RNG
draw order, sign-test tail and input-hash preflight before data access, then
reported no remaining HIGH/MEDIUM issue. The final source was frozen at
`f25e2c3`.

## 7. Empirical Evidence

One run at evaluated commit `f25e2c38c2480fd7b7f127bdb47738d26347437b`.
Raw, structural, analysis, metadata, log, README and
[SHA256SUMS](results/run001/SHA256SUMS) are under [run001](results/run001/).
Independent direct-energy review reproduced 58/60 structural hits, 60/60
ties, 5,934 shared-seed pairs and 268 candidate gains with no exclusions or
invalid solutions. Full table and hashes: [RESULT.md](RESULT.md).

## 8. Assumptions

The synthetic Erdős–Rényi graphs are small (64–128 vertices) with 8% or 16%
edge probability and deterministic positive vertex weights. They are not a
representative sample of real weighted-MIS applications. Wall-clock results
depend on this four-logical-CPU WSL2 host. `git_dirty=true` in metadata comes
from unrelated untracked files; the experiment source paths were clean.

## 9. Known Weaknesses

No exact graph optima or specialized MWIS baseline were measured. The same
observed best weight in every campaign could reflect an easy corpus; it is
not proof of reaching the global optimum. The primary gate is a practical
pilot decision, not an equivalence or general superiority test. The raw
artifact hashes were added in the reporting commit after the raw freeze,
and the generated README's two relative links were corrected post-run; both
documentation deviations are disclosed in [RESULT.md](RESULT.md).

## 10. Simplest Plausible Alternative

Keep `with_2opt(false)` for these instances. It reached the same observed
best feasible weight in every registered cell at the same wall-clock budget.
The candidate's improvement to some individual solutions is real but did
not improve the best-of-time endpoint.

## 11. Suggested Kill-Tests

On a new, independently sourced weighted-MIS corpus, first confirm nonzero
post-presolve pair headroom. Then compare 1-opt alone, CD005 and a specialized
weighted-MIS solver under one pre-registered budget, independent validator
and at least 10 seeds per graph. If the best-of-time result still ties, stop
integration; do not tune on these six graphs. This is a suggestion, not an
authorization for another campaign.

## 12. Freeze Point

Protocol and inputs: `08f24a9`; instrument and analysis: `f25e2c3`; frozen
raw outcomes: `5b9b391`. Claims H1/H2 and limitations: [RESULT.md](RESULT.md).
Replay data analysis without solving:

```bash
python3 research/experiments/exp007_weighted_mis/analyze.py \
  --structural research/experiments/exp007_weighted_mis/results/run001/structural.jsonl \
  --raw research/experiments/exp007_weighted_mis/results/run001/raw.jsonl \
  --data research/experiments/exp007_weighted_mis/instances.json
```

Remaining: no production integration; choose a new external graph family only
under a new protocol.
