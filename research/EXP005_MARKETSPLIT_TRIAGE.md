# EXP-005 Market Split: falsification-first triage

Date: 2026-09-25. Scope: fast audit of the claim that the 41 selected QOBLIB
Market Split instances have no publicly known feasible assignments. This is a
prospective finding linked to, but does not edit, `protocol.md`.

## Decision

**NO-GO for a first-solution or world-record claim on these 41 input files.**
The central premise in protocol section 3 is contradicted by the instance data:
each of the 41 local `.dat` files includes a `# Solution:` comment containing a
binary assignment. An independent integer recomputation gives `Ax = b` for all
41. The local compiled QOBLIB `check_marketsplit` returns exit code 0 for all
41 assignments. This does not show that the search code read those comments.
It shows that a newly found feasible assignment on these files cannot be
claimed as the first publicly known one.

Primary-source examples:

- https://raw.githubusercontent.com/ZIB-AOPT/QOBLIB/main/01-marketsplit/instances/ms_12_100_003.dat
- https://raw.githubusercontent.com/ZIB-AOPT/QOBLIB/main/01-marketsplit/instances/ms_13_050_003.dat
- https://raw.githubusercontent.com/ZIB-AOPT/QOBLIB/main/01-marketsplit/solutions/README.md

The curated solutions table does not list all 41 selected instances, but the
same official repository publishes their assignments in the instance comments.
The official solutions README explicitly says all supplied problems are feasible.
Lattice basis reduction for Market Split itself is prior art:
https://pubsonline.informs.org/doi/10.1287/ijoc.12.3.192.12635.

## Executable kill-test

The audit extracted only the `# Solution:` vector from each of the 41 files,
verified the vector length and Boolean domain, recomputed every row sum with
integer arithmetic, and passed each vector to
`target/release/check_marketsplit <instance.dat> <01-string>`. Observed:
**41/41 exact residual-zero assignments; 41/41 checker exit code 0**.
All 41 local input files were fetched again from the official `main` raw URLs
and compared byte for byte: **41/41 identical**, with no fetch errors.
The strongest simple competing explanation for any claimed first solution is
the public answer already packaged with the benchmark. This lookup is an
answer-key baseline, not a fair solver baseline.

## Algorithm and evidence limits

`src/bin/qoblib_lattice_ultimate_hybrid.rs` does call `UltimateSolver::solve`
in arm 0 of a four-arm Rayon search. Arms 1–3 run separate integer methods.
No matched ablation establishes that arm 0 improves time-to-feasibility. A
single run or shared incumbent cannot attribute progress to UltimateSolver.
The local LLL implementation checks `Ax0=b` and `Av=0` for extracted vectors;
dimension `n-m` alone does not prove that the vectors form a saturated integer
kernel basis or that the Boolean assignment is representable as `x0+Vλ`.

The identity `Φ(λ)=Σ_j x_j(x_j-1)` is nonnegative for integer `x` and reaches
zero exactly for Boolean `x`. Evaluating one coordinate delta uses O(1)
arithmetic after gradient maintenance; applying the move updates O(r) gradient
entries and O(n) coordinates. The claimed 50M–100M steps/s, 200× gain, quantum
tunneling, and high chance of solving overnight have no matched measurements
here. Having 100 Boolean coordinates in one integer point does not reduce the
problem to 20 independent variables: kernel moves can alter all coordinates.

The protocol is currently an untracked draft, as are the new lattice and
hybrid sources. No evaluated commit, environment manifest, frozen raw run,
or matched seed/budget comparison was available in this audit. The running
hybrid process is a 600-second single-instance run; it is not evidence of an
indefinite 41-instance campaign.

## Next justified experiment

Preserve the original files and the public answer vectors as a separate
verification oracle. Give every solver only a comment-stripped `(A,b)` input.
Under a new, prospective protocol, compare LLL+integer search, the hybrid
with UltimateSolver, and the hybrid without arm 0 on identical instances,
seeds, hardware, and wall-clock budgets. Include a strong specialized exact
baseline and report time-to-first-feasible with timeout censoring. Independently
check every returned assignment. The question is algorithmic utility on a
known-feasible benchmark, not discovery of a previously unknown solution.

Stop condition for this triage: the public-answer premise has failed. Reopen
novelty assessment only for a different, verified open problem or a properly
qualified algorithmic advantage.
