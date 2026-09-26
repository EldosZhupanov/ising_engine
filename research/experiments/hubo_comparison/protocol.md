# HUBO-C001 — same-kernel representation and external-baseline screen

2026-09-27. Binding on commit before candidate implementation or solve outcomes.
Locally preregistered exploratory screen, not a production/SOTA comparison.

## Question, scope and evidence

Does direct HUBO with the existing MSC kernel achieve better original-objective
quality than a shared-product quadratic reduction, and does any signal survive
comparison with retained OpenJij? HUBO-RG001 passed correctness; its results do
not imply performance. Breakthrough EXP001 was a different elimination study.

Canonical paths: this directory's generator/cases, worker, campaign, tests and
`research/examples/hubo_compare.rs`; dependency map is LogicBuilder -> HuboModel
-> FlatHuboModel -> QuantumField -> engine::step. Production files are read-only.
Reuse original engine; no scalar/MSC changes or UltimateSolver API additions.
The test-local Rosenberg implementation from RG001 is not the performance arm.

## Inputs and arms

`cases.json` SHA256 79026f9af3fe1846129da52f28e214b77be12fac7548e17ef96bed7330cde1a9.
Ten deterministic signed spin polynomials: n=32, 96 distinct random interactions
and 32 signed unit fields, degrees 3/4 (five each). Instance seeds 930001..930005
and 940001..940005. SplitMix64 generator is preserved, expands s=2x-1 exactly.
No optimum or reference solution is known/used. Denominator is sum absolute
spin coefficients (=128); lower original spin energy is better. This is synthetic
screening, not an external application corpus. No prior search/tuning on these
instances. Creating inputs before freezing is disclosed; no solve outcomes exist.

Solver seeds 950001..950010, each instance/arm/seed once: 400 cells.
Four arms: msc_native, msc_quad, oj_native, oj_quad. For quad, use installed
`dimod.make_quadratic` (0.12.22), BINARY, with pair sharing and strength
M=1+sum absolute degree>2 coefficients of the expanded original polynomial.
This sufficient conservative penalty is fixed, not tuned. It can disadvantage
a quadratic search; therefore a win licenses only this reduction/settings claim,
not rejection of stronger/tighter quadratizations. Include both native external
and quadratic external arms to prevent an ancilla-only comparison from appearing
as a competitive solver win. No post-hoc penalty or baseline replacement.

A common Python worker imports the same libraries before READY. After GO it
parses the input, emits all-zero original witness, builds/reduces the model,
constructs original-index-preserving integer variable labels, then invokes:

- MSC: experiment executable linked to unchanged engine, one slice/population,
  eight geometrically spaced temperatures and 64 lanes per temperature; one
  engine step then inspect all original-variable projections; no reset, ICM,
  quench, presolve or learned guidance. Random initial bits; initialize energy
  buffers from model. Native and quad have identical driver/settings.
- OpenJij 0.12.0: SASampler.sample_hubo BINARY, one read, 1000 sweeps, one thread,
  METROPOLIS, XORSHIFT, GEOMETRIC; repeat complete calls with
  seed=solver_seed+10*restart_index. Require restart_index<1,000,000
  (else instrument failure). This is injective across the ten seed cells in
  the practical range and remains below 2^31. No seedless fallback.
  The same polynomial sampler is used at degree <=2 for oj_quad.

Both use the same deterministic temperature endpoint rule: T_hot=max(1,
max over variables of sum incident absolute nonconstant coefficients)/ln(2),
T_cold=0.1. MSC uses eight temperatures between endpoints; OpenJij anneals over
1000 sweeps with beta_min=1/T_hot, beta_max=10. Common rule is applied to each
representation, so the result includes schedule changes induced by penalties.
These are fixed, untuned configurations, not claims about optimal settings.
Common wrapper/import access is equal; MSC child launch/serialization is charged.
Python NumPy/OpenJij/dimod versions and code/binary hashes are recorded.

## Budget, observation and independent checks

Single worker/core budget; RAYON_NUM_THREADS=1, OMP_NUM_THREADS=1, OPENBLAS_NUM_THREADS=1,
MKL_NUM_THREADS=1, PYTHONHASHSEED=0. CPU affinity unset; record host/load/versions.
No concurrent campaigns. WSL timing is exploratory. Each cell has an external
2.0-second deadline from immediately before GO is written; budget includes
input parsing, reduction, initialization, solver and witness scoring/reporting.
Interpreter/library imports before READY are excluded and measured separately.
Thus this is warm-runtime search quality, not cold-start end-to-end speed.
READY must arrive within 30s or campaign is instrument-invalid.

Only complete witness lines received by the supervisor by 2.0s count. Discard
late witnesses, retain timestamps/raw lines and termination metadata. Kill the
whole process group (including Rust child) at deadline; termination may take up
to 0.2s with no extra credited search. Initial all-zero is a legitimate fallback,
not silent success. Missing/invalid witness, malformed complete event, early
unexpected exit, failed import/solver or leaked child invalidates the campaign.
No reruns, exclusions or replacement cells; preserve failures and stop on first
instrument defect. A partial suite is INSTRUMENT_INVALID, not evidence of a win.

Each arm scores emitted candidate x against the original integer spin expression;
the supervisor independently recomputes both the spin and expanded-polynomial
energies. Save final 32-bit witness, received-event log, transform metadata,
source hashes, process statuses. A quadratic assignment need not have consistent
auxiliaries to be a valid projected original candidate; do not use penalized Q
energy for ranking final quality. Capture expanded variable/term counts and M.

Order: instances as listed, seeds ascending. Let A=msc_native, B=msc_quad,
C=oj_native, D=oj_quad. Cycle orders [ABCD,DCBA,CDAB,BADC] by
(instance_index*10+seed_index)%4 (indices start at zero). Across 100 blocks,
each arm occupies each position 25 times and each pair order is split 50/50. Analyze only
after all cells; do not adapt searches to partial outcomes.

## Metrics, decision and falsification

Primary comparisons: msc_native against each other arm. For every instance,
average ten paired normalized gains (E_other-E_native)/128. Independent unit is
instance, not seed. Report median and mean of these ten instance-level gains,
exact two-sided Wilcoxon signed-rank p (average ties, omit zero differences,
full sign enumeration) and Holm correction across three comparisons. Sign-rank
requires symmetric instance difference distributions/sign exchangeability; this
small synthetic sample does not establish external generalization.

CONTINUE only if all three comparisons have median instance gain >=0.01 AND
Holm p<0.05 AND each degree family's mean gain is positive, with all integrity
gates passed. Otherwise NOT_QUALIFIED_FOR_ADVANTAGE; nonsignificance is not
proof of equivalence or impossibility. Full saturation/ties is INCONCLUSIVE
for discrimination and must be reported. Never reverse direction into a universal
OpenJij win. No automatic tuning or new campaign after the outcome.

Secondary: per-arm/family seed energies, mean/sample SD/min/max/median/Q25/Q75/IQR,
paired win/tie/loss, transform cost/size, startup and deadline distributions,
number of solver returns/sweeps where observable. No optimum-success rate or
TTS99: no certified optimum targets exist. Improved-over-zero frequency is only
a descriptive search metric. Treat unobserved target hits as unavailable, not
as failures at an invented target. Completed sweeps/returns may differ at the equal wall budget and are
descriptive only; no speed claim follows from a comparison at unequal budgets.

Before qualification: tests for exact input expansion; canonical quadratic
minimum identity on small cases; engine/model energy and fixed-seed replay;
independent corruption detection; deadline/early-exit/missing/late-row handling;
exact rank/Holm reference cases. Smoke uses disjoint seeds 910001/910002 and
explicit small n=6 fixtures, never the main generator or main cases. Pre-data API preflight on one n=3 toy
returned energy -2 identically twice; no comparative measurements were made.

Freeze instrument/analysis with tests and independent review before main data.
Capture all source dependency hashes against Git, packages, binaries, OS/CPU/RAM,
compiler flags, load and exact invocations. Preserve results separately; new
scientific defects after holdout require a new protocol, not silent repair/rerun.

Primary-source baseline references (checked 2026-09-27):
https://tutorial.openjij.org/ (sample_hubo supports direct high-order sampling),
https://docs.dwavequantum.com/en/latest/ocean/api_ref_dimod/generated/dimod.make_quadratic.html
(explicit warning about insufficient penalty strength). No novelty is claimed for
native HUBO, substitution, PT or this factorial comparison.
