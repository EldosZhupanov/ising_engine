# Breakthrough research state

**Resumed 2026-09-12 through the goal control**, which is active after the prior block. Preregistration committed as `aee6ea8`; the isolated prototype is implemented and independently reviewed PASS before data.

Project handoff authority remains [memory/NOW.md](../../memory/NOW.md). This file is the requested research ledger, not a second project-task authority.

Baseline: unchanged `UltimateSolver` from `fdec0df`, with existing QPBO/probing, random initialization, PT and 1-opt finish. No experimental candidate has yet established superiority. Best validated method remains the baseline; no breakthrough claim.

Completed: [algorithm audit](AUDIT.md), [10 hypotheses](HYPOTHESES.md), [prospective symmetry-scope correction](GAUGE_SCOPE.md). Independent Explorer checked historical algorithms and cached branches. `engine_v2` is explicitly covered, including state, kernels, registry, runtime, selector and research models.

| Experiment | Hypotheses | Design | Status |
|---|---|---|---|
| EXP001 | H01 + H07 | full 2³ factorial: leaf response, degree-2 response, 2-opt refinement | preregistered `aee6ea8`; prototype: 9 mathematical + 3 analysis tests pass; independent instrument review PASS; no data |

Previously weak/rejected directions: consensus freezing (RC-001), generic covariance/co-flip move synthesis (RC-003), hard-vs-soft memory spelling (RC-023), naive relinking additions (RC-024–026). Their scoped negative records are preserved. No new hypothesis is experimentally rejected yet.

Promising untested: conditional elimination, exact tree moves, gauge-conditioned distributions, constraint-tangent moves. Unexplained candidate outcomes: none yet. Audit anomaly: published “marginal vacuity” corollary is broader than its premises; exact counterexample derived. Benchmark anomaly: calibration null drift is large, so small timing advantages will not be claimed.

Next: commit the reviewed prototype before data; run all 4,200 frozen rows, analyze failures and compute accounting, independently review results. Then select H02 or H04 based on information gained, not the most flattering result.
