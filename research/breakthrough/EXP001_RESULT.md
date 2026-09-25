# EXP001: continuation screens, not a breakthrough

Date: 2026-09-12. Immutable on commit. Protocol: [EXP001_PROTOCOL.md](EXP001_PROTOCOL.md).
Preregistration `aee6ea8`; instrument `526ddd3`; production source `fdec0df` unchanged.
The run completed once: 3,600 fixed-exchange rows, 600 wall-budget rows and 45
precomputed targets. No failed or rerun rows. Every final assignment passed
canonical QUBO and independent native Ising energy checks inside the instrument.
The instrument passed 9 Rust and 3 Python tests plus independent review before data.

Raw SHA256: `10b7905fc2fb37aa73275aa6b8e8c6ef3ea6779b1e3ff01baca17bd7e17f65c4`.
Targets SHA256: `8c34c2e483092fe3a7cd61a2aa8f76ca8da801e6ab05272fb8c0fde9ce79f1e3`.
Artifacts: [raw](exp001/raw.tsv), [targets](exp001/targets.tsv),
[environment](exp001/environment.txt), [summary](exp001/summary.tsv),
[all descriptive ablations](exp001/paired_ablations.tsv),
[primary screens](exp001/primary_screens.json), [closure](exp001/complete.json).

## Registered primary outcome

Quality advantage is (E_baseline-E_candidate)/max(1,abs(reference target)),
equally averaged across three instance means. It is not an optimum gap reduction.
Time ratios use restricted mean observed target time, including failures at 100ms.
All eight arms must satisfy the budget for a paired block to count.

| Family | Valid blocks / 15 | H01 A+B quality | H01 target-time ratio | H07 C quality | H07 target-time ratio |
|---|---:|---:|---:|---:|---:|
| cycle | 15 | +10.418% | 0.00107 | +5.800% | 0.0590 |
| subdivided | 14 | +8.466% | 0.0297 | +3.863% | 0.1035 |
| sparse | 15 | +0.114% | 0.6933 | +4.049% | 0.2378 |
| dense | 0 | inconclusive | inconclusive | inconclusive | inconclusive |
| field | 15 | +0.506% | 0.7148 | +2.711% | 0.2594 |

Both H01 and H07 receive the frozen **CONTINUE_SCREEN** verdict. H01 passes
only on subdivided among non-cycle families. Sparse/field H01 fail because some
instances get worse despite a positive family average. H07 passes subdivided,
sparse and field with positive quality advantage in every retained instance.
Cycle is only a known exact-method positive control. Dense has 101/120 individual
wall rows outside budget; no complete valid block exists. It is an instrument
limitation, not a negative mathematical result.

## What was learned

A+B reduces subdivided n=128 to 32 variables, cycle to zero, sparse/field to
117, 121 or 127 variables, and dense not at all. This tracks the structural
mechanism. Degree-2-only B already supplies almost all subdivided improvement;
C supplies no further improvement after A+B on the n=128 subdivided wall rows.

The common full-variable 1-opt pass improves none of the 4,200 final solve units'
aggregated records. Thus its correction does not account for candidate gains
on these instances. Pair refinement improves 28/30 sparse n=128 fixed runs and
27/30 field runs, but 0/30 for both families at n=16. The measured two-spin
barriers are real relative to these particular one-spin endpoints.

At n=128, increasing fixed exchanges from 1 to 8 shrinks C's quality advantage:
sparse 5.997% to 1.394%; field 3.696% to 1.119%; dense 1.498% to 0.263%.
This is evidence that baseline search duration matters, not grounds to hide
the one-exchange result or select the favorable rung.

## Anomalies and immediate falsification priorities

**ANOMALY — REQUIRES INVESTIGATION: short-restart baseline.** The wall suite
restarts after one exchange. Existing fixed-eight baseline has mean energy
-2446.8 on dense n=128, better than wall C's -2404.667; recorded median fixed-eight
elapsed is about 28ms versus roughly 108ms wall C. This cross-suite descriptive
comparison does not establish a calibrated speed claim, but it directly refutes
interpreting the screen as superiority over a well-allocated baseline budget.
Sparse fixed-eight baseline (-710.933) is already near wall baseline (-711.733).
Next performance test must include longer uninterrupted baseline calls.

**ANOMALY — REQUIRES INVESTIGATION: easy non-cycle family.** The subdivided
core is a ring plus opposite matching, a Mobius ladder. Grouping opposite
vertices produces a strip with four states per position and twisted end bonds.
Its boundary width stays bounded as n grows. Hence a second exact positive
control may explain H01's sole non-cycle pass. An independent transfer oracle
and random cubic cores are required before broader structural claims.

**ANOMALY — REQUIRES INVESTIGATION: large target-time ratios.** Large-instance
targets are feasible baseline outputs, not certified optima. Earlier arrival at
these relatively accessible levels can amplify ratios. Do not describe 0.03 as
33-fold acceleration to optimum. Validate objective/targets, replay fixed seeds,
and challenge the signal with stronger baseline budgets before more mechanisms.

## Cost and limits

End-to-end algorithm time includes transformation, lifting, all refinement and
incumbent handling. Wall C uses almost the same nominal proposals as baseline
on sparse (+about 1.6%) and field (-about 0.6%), and process CPU medians are about
0.10s in both arms. This excludes a gross compute-budget multiplication, but
internal kernel evaluation counts remain unavailable; GNU CPU time is coarse.
Per-stage counts, CPU, RSS, elapsed distributions, seed variance and n scaling
are in the raw/summary artifacts. Fixed exchanges are not equal-cost evidence.
No GPU implementation/comparison was run. No production speed or universal
solver claim is licensed by this exploratory host or three instances per family.

Next: [EXP002 validation](EXP002_PROTOCOL.md), then a separately registered
strong-baseline and random-core challenge. No retuning or rerun of EXP001.
