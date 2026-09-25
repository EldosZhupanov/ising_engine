# RC-027 — Does mechanism value depend on run length?

**Registered outcome:** `synthesis` **CONVERGES**; `memory_form` stays
**PARALLEL**. Both mandatory 50-sweep controls passed. Increasing the budget
does not reopen the architecture line.

## Provenance

| item | value |
|---|---|
| preregistration | `research/PREREG_RC027_BUDGET_AXIS.md`, commit `0180102`, frozen before harness code and data |
| preregistration SHA-256 | `88bc04d1477bf5d464f99241f1fea31531501d1ffddd1729ac9c9182d0efd667` |
| instrument commit | `fdec0dff24595828cedf08d793d7d9b62a5a2d0a` |
| instrument source SHA-256 | `3e3e5cfcc2bdaca8254af20b914695bd2a2f660ae1f89a2d6afb95ba64d432b8` |
| registered command | `cargo run --release --bin exp_rc027_budget_ladder -- --dir benchmark_suite/data/gset --budgets 50,200,800,3200 --replicas 32 --seeds 401,402,403` |
| `raw.tsv` SHA-256 | `a5131c0b20e925795b340e69c47e602f4dcb72f7dc93ade2e3101ba800a74361` |
| `run.err` SHA-256 | `68d594ca6b1eec85ad621f00d0b44917c32270b7db2163e9a979b85c1cb15377` |
| execution | one registered invocation, exit 0; no pilot, repeat, rescue run or secondary extension |

A concurrent repository commit (`69067b7`) incorporated the already-written
initial harness together with an unrelated benchmark audit. It followed the
binding preregistration and preceded the final instrument commit. No RC-027
datum existed at either commit; `fdec0df` is the exact revision that passed the
pre-run gates and produced the artifacts above.

## Instrument validity and mandatory controls

The raw file has one header and exactly 720 data rows. All 720 tuple keys are
unique and cover 30 instances × 4 budgets × 3 seeds × 2 pairs. Every score and
gain is finite, every reported state passed an exact canonical rescore, and the
raw Rudy guard accepted the declared edge count and endpoints of all 30 files.

Both controls frozen in PREREG §4 passed:

| control | registered requirement | observed |
|---|---|---|
| `synthesis`, 50 sweeps | mean gain strictly between 0 and 1 % | **+0.105242 %** |
| `memory_form`, 50 sweeps | `abs(mean) < 1 %` and Wilcoxon `p >= 0.05` | mean **+0.202385 %**, `p = 0.873354` |

Path relinking was never worse than its paired Metropolis control at any rung:
**268 wins, 0 losses, 92 ties** across the 360 RC-027 synthesis observations.

## Registered primary results

Spearman uses the 120 preregistered per-instance mean gains per pair, average
ranks for ties, and the frozen two-sided Student-t approximation with 118
degrees of freedom.

| pair | Spearman rho | p | frozen classification |
|---|---:|---:|---|
| `synthesis` | **−0.260843** | **0.004009** | **CONVERGE** |
| `memory_form` | +0.011410 | 0.901565 | **PARALLEL** |

The synthesis curve moves in the opposite direction from the only reopening
condition: path relinking helps most at the shortest budget and its gain shrinks
significantly as the run grows. Memory form has no registered trend at all.

## Complete descriptive ladder

Wins/losses/ties use the frozen `1e-9` threshold and count the 90 raw paired
observations at each pair/budget.

| pair | sweeps | mean gain | median gain | W / L / T |
|---|---:|---:|---:|---:|
| synthesis | 50 | +0.105242 % | +0.079104 % | 69 / 0 / 21 |
| synthesis | 200 | +0.094932 % | +0.066357 % | 70 / 0 / 20 |
| synthesis | 800 | +0.062155 % | +0.052418 % | 65 / 0 / 25 |
| synthesis | 3200 | +0.066190 % | +0.044195 % | 64 / 0 / 26 |
| memory form | 50 | +0.202385 % | 0.000000 % | 40 / 41 / 9 |
| memory form | 200 | −0.048699 % | −0.013140 % | 26 / 45 / 19 |
| memory form | 800 | −0.062471 % | −0.022446 % | 29 / 47 / 14 |
| memory form | 3200 | −0.025894 % | 0.000000 % | 30 / 42 / 18 |

Neither top rung approaches the 1 % materiality floor. No unregistered subgroup
or alternative trend test was run.

## Guard and mutation evidence

Before execution, each named guard family was broken in turn. Each exact test
invocation failed, the source was restored, and the restored SHA-256 matched.

| mutated guard | mutation | test killed |
|---|---|---|
| frozen command | argument-count equality inverted | `only_the_preregistered_command_is_accepted` |
| corpus identity/count | 30-file comparison inverted | `only_the_exact_thirty_gset_instances_are_accepted` |
| raw Rudy structure | declared/body edge-count comparison inverted | `raw_rudy_structure_is_checked_before_the_frontend` |
| frontend/raw energy | exact-energy comparison inverted | `frontend_energy_must_match_the_raw_edges` |
| equal frozen budget | schedule-sweep comparison inverted | `every_arm_has_exactly_the_frozen_budget_and_parameters` |
| never-worse | strict worsening test changed to reject ties | `bad_outcomes_and_path_regressions_are_refused` |
| row uniqueness/completeness | `HashSet::insert` sense inverted | `duplicate_or_missing_rows_are_refused` |
| mandatory controls | positive synthesis boundary weakened to admit zero | `both_fifty_sweep_controls_are_mandatory` |
| operator availability | path-relink operator name replaced by an absent name | `every_registered_operator_exists` |

The final harness has ten tests: the nine mutation tripwires plus an actual
30-file preflight through raw validation and frontend-energy equivalence. The
execution constructor is shared by production and test and asserts that early
stopping is disabled.

## What survives

RC-027 closes the last cheap alternative explanation for the architecture
results. The mechanism inventory was not rescued by a different memory form
(RC-023), a non-degenerate corpus (RC-025), embedding shape (RC-026), or a
64-fold longer local-search budget here. For path relinking the honest positive
claim is narrower: it buys **earlier convergence**, with a never-worse guarantee,
not a better long-run optimum. The memory-form comparison stays flat.

Under the frozen table, the architecture closure is final. Reopening it would
require a new question and a new preregistration, not another budget or
mechanism rung added after these results.

## Prohibited claims, honoured

This is paired quality at equal logical sweep budgets. The candidates do not
perform equal logical work to their controls, RC-021 left the host
`INSTRUMENT-INVALID`, and the feasibility probe was not a timing experiment.
Therefore this record makes **no wall-time, throughput or equal-cost claim**.

It makes no claim about `UltimateSolver`, production routing or integration,
tuned variants, corpora beyond these 30 G-Set instances, universal optimization
or model training. The RC-025 high-diversity extension was not run. The complete
published evidence is `raw.tsv`, `run.err`, and this record.
