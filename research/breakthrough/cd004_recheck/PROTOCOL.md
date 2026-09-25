---
id: cd004-recheck-protocol
kind: research-protocol
status: binding
authority_scope: CD004-R only
created: 2026-09-25
immutable: true
---

# CD004-R: prospective exactness and utility check

The legacy CD004 result and its exactness defect were inspected before this
protocol. This is a new iteration, not retrospective preregistration of the
legacy experiment. No holdout rows below have been computed as of this commit.

## Object and arms

For `E(s)=-sum_{i<j} J_ij s_i s_j`, integer symmetric `J`, `h=0`, compare
two deterministic depth-first exact searches using the *same* variable order,
branch order (`+1`, then `-1`), initial incumbent, lower bound, and dynamic
incumbent update. Baseline is chronological pruning. Candidate additionally
stores a partial assignment `C` only when `LB(C)>=E_inc` and prunes a future
node when its fixed assignment contains `C`. The candidate must not skip an
unexplored sibling merely because a core was extracted. A cache hit is valid
after an incumbent improvement because the threshold can only decrease.

The exhaustive `2^N` evaluator is the correctness oracle. The saved full spin
witness must recompute to the reported energy. Original CD004 functions are
historical controls, not candidate code. Python standard library only.

## Frozen instances and budgets

Use the original CD004 graph generator and its coupling distributions for
`complete_SK` and `random_sparse`. For each family, `N in {8,10,12}` and
`seed in 1000..1019`, yielding 120 independent generated instances. Pair arms
within each instance. The initial incumbent is one deterministic random spin
start followed by the original 1-opt heuristic, with RNG seed
`4000000 + 10000*N + 100*family_index + seed` (`complete_SK=0`,
`random_sparse=1`). No parameter tuning or exclusions. Run once; serialize
every row including failures. The known seed-1 complete graph, an earlier
field-bearing `N=4` case, and random small exhaustive tests are calibration
only, excluded from the 120 rows.

## Decisions and analysis

Primary gate: all 120 baseline and candidate energies equal the independent
exhaustive optimum and all returned spin witnesses recompute exactly. One
failure is NO-GO for CD004-R correctness; stop utility interpretation. Secondary
metrics, if primary passes: paired node, lower-bound-call, and wall-clock ratios
candidate/baseline, plus count of time wins. Report medians and ranges by
family/size and overall. A *practical* speed result requires overall median
wall-time ratio <=0.90 and at least 90/120 paired time wins. This is a screen,
not a statistical significance claim; repeat timing on a different machine
before production. No graph-selector rule is fit to these rows. The user-facing
decision is one of exact-and-useful, exact-but-slow, or incorrect.

Execution environment, evaluated commit, Python version, raw JSONL rows,
analysis command, failures and code hashes will be recorded in a later result.
The legacy eight-spin counterexample is disclosed prior access. Any correction
to this protocol requires a linked prospective amendment, not an in-place edit.
