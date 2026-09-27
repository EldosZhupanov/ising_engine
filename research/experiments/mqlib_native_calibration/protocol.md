# MQ-NATIVE-001 — native model preparation, no search

2026-09-27. Binding prospective preregistration; NOT RUN.
Scoped successor to [MQ-CAL-001](../mqlib_timing_calibration/RESULT.md) and
[affinity design](../mqlib_screen/NEXT_DESIGN.md). User authorized implementing
and running native no-search probes. Only instrument qualification is admitted;
parent external-comparison S3/X3, untouched-data and adapter gates remain.

## Estimand and exact boundary

Measure delivered completion of native model preparation, not optimization or
full solver initialization. Rust uses the exact Model/prepare body and model
energy methods from committed mqlib_compare.rs (byte-checked before build): JSON
parse, QuboModel/CSR plus ProblemIR construction, and standard OperatorRegistry.
The probe never calls solve, Runtime::run or a search operator. In particular,
UltimateSolver's internal field/allocation inside solve is OUTSIDE this boundary.
C++ uses pinned MQLib585496274af5abb0849d0d47e135496b4688680b QUBOInstance parser,
QUBOSimpleSolution with explicit assignments and PopulateFromAssignments, and
Stream::Report serialization from the original bridge (compile-time reuse).
It never constructs Merz2002OneOpt or any search heuristic.

Before optional delay, both probes evaluate four deterministic assignments from
the constructed model: all0,all1,index%2,index%3==0. These are validation probes,
not search; no best state is selected. Emit the all-zero state through the original
incumbent serialization plus a diagnostic JSON line containing all four energies,
relative model-ready time, actual sleep time and observed affinity. Extra checking
and diagnostic delivery cost is included and disclosed. This is a diagnostic
native path, not a claim of uninstrumented solver startup cost.

Rust uses safe std::time::Instant for relative phases; no cross-language absolute
clock-domain claim. Parent uses monotonic_ns. There is no child-to-parent delivery
lag gate because a shared absolute monotonic timestamp is not available through
this safe Rust interface. Do not copy that gate or pretend it was measured.

## Frozen cells and resource contract

[Manifest](manifest.json) freezes two artificial models, hashes, workers,
budgets10/25/50/100ms,20repeats and three modes A/B/D. A/B have identical worker
logic; D sleeps B/4. Order is worker,fixture,budget,repeat with deterministic arm
rotation. No stochastic seed or real corpus. **960cells,44.4s nominal budget**,
240s total cap; exceeding it is INCOMPLETE. No retries or corpus changes.
Both parent and child CPU0, one thread; record observed masks in parent and child
and reject mismatches. Same contract must be used by any dependent benchmark.
This does not retroactively calibrate MQ-SCREEN-001's unrecorded parent placement.

Parent timer starts before serialization/conversion/tempfile/launch. Rust JSON
and MQLib exported QUBO conversion are charged; common immutable disk fixture
read and postrun audit are outside. Receipt must be strictly before the deadline.
A completion requires BOTH complete lines before cutoff; its timestamp is the
later line receipt. Keep late lines, partial bytes and stderr. Two-line emission
is part of this contract; never use the earlier state line alone for admission.
Remain alive until killed at deadline. Record cutoff,kill,reap and total times.
Parent validates every complete line, every raw-polynomial probe energy and zero
state independently. Finite domain/schema/energy/time/affinity mismatch,
unexpected exit, premature EOF, changed sources or malformed output => INVALID,
stop, retain. Absent/late completion without corruption is valid censoring.

## Gate per worker, fixture and budget

Require all60 valid records,>=19/20 on-time complete pairs in each mode,
>=19 complete A/B and D/A repeat pairs. Let d=B/4, t=last-line receipt from start.
All six conditions must pass:

1. Completion and paired counts above.
2. All deadline overshoots nonnegative; nearest-rank q95<=max(1ms,0.05B).
3. abs(median(tB-tA))<=max(2ms,0.1B).
4. abs(median(tD-tA)-d)<=max(2ms,0.2d) AND at least18 of20 paired blocks have
   on-time tD>tA; missing pairs count against18.
5. All emitted D diagnostics report actual sleep>=d; nearest-rank q95 oversleep
   <=max(1ms,0.1d), with>=19 on-time D completions.
6. Every emitted diagnostic has observed affinity exactly CPU0 and nonnegative
   model-ready duration. Parent observed mask and child OS-observed mask are[0].

q95 index=ceil(.95N)-1; even medians averaged. No p-value/speed ranking. Report
all16 worker/shape/budget outcomes; a budget qualifies the complete tested path
only if all four worker/shape combinations pass. No interpolation or admission
of solver search, unseen larger inputs, changed affinity or different binaries.
Report distributions/counts of on-time latencies, paired differences, readiness,
sleep, cutoff and teardown; conditional latencies are not uncensored runtimes.
Overall FAIL if any registered group fails; INVALID/INCOMPLETE remain distinct.

## Gates before execution and evidence

Commit protocol+manifest before instrument execution. Byte-check reused Rust
prepare body and pinned C++ include, verify linked library/build provenance.
Build isolated Rust research example/C++probe, no production or Cargo changes.
Tests must exhaustively compare tiny-model energies, exercise serialization and
MQLib sign/offset mapping, and fabricated censoring/pair/affinity/schema cases.
Independent instrument review precedes commit and any smoke.

Two separate schema smokes, one per worker on n8 at200ms,A mode, check both
complete lines/state/diagnostics/termination, not main timing thresholds. Preserve
all failures. Main requires both smoke passes from identical source/binary hashes.
Use fixed artificial state probes only, no new optimizer runs. Output directories
must be new. Capture protocol/instrument commit, binaries, source/dependency hashes,
host/compiler/env/affinities, raw bytes and exact commands; assert before/after.
Independent postrun auditor must not import the analyzer; reconstruct completions,
all energies/timestamps/counts and all six gates. No rescue rerun after failure.
No automatic optimization comparison follows. Changed protocol/instrument after
outcomes needs a new prospective version, retaining previous data.
