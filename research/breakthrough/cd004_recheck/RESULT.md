---
id: cd004-recheck-result
kind: research-record
status: closed
authority_scope: CD004-R findings
created: 2026-09-25
immutable: true
---

# CD004-R: exactness restored, acceleration claim withdrawn

## Decision

The original CD004 `solve_conflict_driven_bnb` is **not an exact solver**. Its
reported 33.1–54.4% node reductions cannot support the earlier algorithmic
advantage or production-integration claim. A separate corrected prototype with
certified learned cores is exact on the registered 120 small instances, but its
tree size is identical to chronological branch-and-bound and it is slower.
This is **NO-GO for the current CD004 acceleration mechanism**, not a theorem
that all conflict-learning approaches to Ising optimization are ineffective.

## Why the original result failed

The historical witness in [`../cd004/witness.py`](../cd004/witness.py) returns a
non-chronological jump target from a pruned descendant and propagates it past
unexplored sibling branches. It does not store a learned nogood or prove that
those skipped siblings contain the same conflicting assignment. For the
zero-field `complete_SK`, `N=8`, generator seed `1`, and one 1-opt start seeded
`10001`, the initial energy is -10. Exhaustive enumeration and chronological
branch-and-bound find -12; the historical conflict-driven function returns -10.
This is asserted in `test_instrument.py`.

A post-discovery read-only replay of all seven published CD004 benchmark cases
found that each 20-restart initial incumbent already equalled the exhaustive
optimum. This replay was not saved as a row-level artifact; the registered
counterexample and recheck are the preserved decisive evidence. Thus their
final-energy agreement could not test whether the
backjumping search recovers a missed optimum. The earlier protocol also
specified a mean-core-ratio success threshold of <=0.70, whereas all seven
reported ratios were >=0.780; its stated success condition was unmet. These
observations do not alter the original files or their historical chronology.

## Prospective recheck

The [protocol](PROTOCOL.md) was committed as `506e61d`; the [prospective
amendment](AMENDMENT_1.md) as `827fa51`, both before holdout access. The
algorithm and analysis froze at `da2ba4c2cace116eecc7d4200a78fdb3a1077409`.
The [raw 120 rows](raw.jsonl) and [analysis](analysis.json) froze separately at
`eed279604039a60592264b222c040dcb50570caa`. Python 3.14.4, standard
library only. The data-generating graph distributions, sizes, seeds, paired
instances and search configuration, three timing repetitions, alternate order,
and exclusions are in the
protocol and amendment. The known counterexample and 80 small random field
cases were calibration, not holdout.

| Registered outcome | Observation |
|---|---:|
| Correctness vs independent `2^N` oracle | 120/120 both arms exact; 0 invalid rows |
| Cases whose one-start incumbent missed optimum | 60/120 |
| Candidate rows with at least one cache hit | 102/120; 1,095 hits total |
| Candidate/baseline visited-node ratio | 1.000 median, min, max; 0/120 node wins |
| Candidate/baseline lower-bound-call ratio | 3.946 median; 1.597–5.151 range |
| Candidate/baseline median-of-three wall-time ratio | 4.876 median; 2.062–9.349 range |
| Candidate paired wall-time wins | 0/120 |

The registered practical-speed gate required median wall ratio <=0.90 and at
least 90/120 wins. It failed decisively. Wall time is a small-Python-instance
screen, not a claim about optimized Rust or larger graphs. The baseline and
candidate visit exactly the same nodes in this run; the cache replaces some
bound checks but core extraction adds many more. No graph-structure selector
was fitted, and no selector result follows from these rows.

## Reproduce and limits

Run `python3 -m unittest discover -s research/breakthrough/cd004_recheck -p
'test_*.py' -v`, then `python3 research/breakthrough/cd004_recheck/instrument.py
analyze --input research/breakthrough/cd004_recheck/raw.jsonl`. The analysis
refuses incomplete or invalid rows. To rerun the deterministic experiment, use
the `run --output NEW_PATH` command from the frozen source commit; wall times
will vary. Original source SHA-256:
`aaf705c09c5f95d945a4968eeb48a431a01ec6138175fabc927b3a244840f81f`.
Frozen instrument SHA-256:
`0626a2ee6818ce45e37cda6c510915b809b3a26b8bd2eb4f29824b42c37dbc1c`.
Raw SHA-256:
`4ebeaf20b7ea97c221ff0dc58f1b79c720bc80c98d9d717366798e6e6bba53de`.
Analysis SHA-256:
`c146a3da652ca94f2db2a130f6d6ba8f6a7d962625bbca3b3c44a20b8ff20a05`.

The study covers only integer-coupling, zero-field SK and sparse random graphs
at N=8, 10, 12, with a single 1-opt incumbent. It does not test stronger
bounds, clause propagation, other branching orders, larger cores, CD003 or
CD005, Laya, quantum dynamics, or physical-material instances. The old witness
failure is an exact counterexample independent of these empirical limits.
