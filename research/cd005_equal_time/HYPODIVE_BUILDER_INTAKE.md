# CD005-Q: Builder intake

Mode C — research evidence. The user's selected next action is to qualify the
already integrated CD005 edge-restricted two-spin escape on held-out applied
instances at equal wall-clock cost. The closest prior evidence is
[`../breakthrough/cd005/RESULTS.md`](../breakthrough/cd005/RESULTS.md): synthetic
spin glasses and operation counts. The existing SK binary compares the toggle
at equal sweeps, and QOBLIB MIS examples use CD005 without an off arm.

Strongest testable claim: enabling `UltimateSolver::with_2opt(true)` yields
better feasible maximum-independent-set solutions than the same solver with
the option off when both receive the same wall-clock search window. This is a
scoped empirical claim, not a theorem, solver-wide result, or world record.

Canonical path: a standalone `src/bin/exp_cd005_mis_qualification.rs` harness
calling the existing `UltimateSolver`, plus a standard-library fetch/analysis
script. No solver family, public API, dependency or production benchmark is
changed. The six QOBLIB graph filenames and Git blobs were selected from
repository metadata before reading their contents. No results from those six
graphs have been accessed. The protocol fixes budget, seeds, inclusion and
analysis before the harness or graph data are run.

Invariants: identical QUBO, temperature, sweeps, exchanges and seed stream in
both arms; only the CD005 toggle differs. Each arm receives five seconds; only
solutions completed before its deadline count. Each counted solution has a
verified spin vector, zero MIS collisions and QUBO energy consistent with its
independently counted size. Any invalid solution stops the quality claim.
Calibration uses only previously evaluated local graphs. A result with no
paired benefit is a valid NO-GO screen, not a reason to tune on the holdout.
