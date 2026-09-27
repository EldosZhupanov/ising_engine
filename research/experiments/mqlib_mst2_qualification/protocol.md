# MQ-MST2-001 — exact witness and callback qualification

2026-09-28. Binding prospective protocol; NOT RUN.
This prospectively extends only the narrow engineering exception in
[external comparison amendment1](../../EXTERNAL_COMPARISON_AMENDMENT_1.md)
to the MST2 qualification below. It does not amend old evidence or waive parent
S3/X3, comparison, untouched-holdout or independent review gates. The user
explicitly authorized this next correctness task. No competitive experiment.

## Question and hypothesis

H1 (engineering): the pinned PALUBECKIS2004bMST2 bridge preserves the explicit
minimum polynomial E=c+sum(h*x)+sum(i<j,J*x*x), emits verifiable global incumbents,
and honors latched cooperative callback stop or parent-enforced termination.
PASS requires every exact identity, parser/corruption test and registered cell's
mode-specific gate below. A single mismatch/failure => FAIL, retained without
repair or result-selected retry. Optimum attainment is not required or evidence
of relative strength. No p-values, speed ranking or stochastic comparison.

## Data and implementation boundary

[Manifest](manifest.json) freezes the12 previously exposed artificial MQ-QUAL-001
models n1..8, coefficients, case hashes, three engineering-test seeds101/102/103,
and upstream585496274af5abb0849d0d47e135496b4688680b (MQLib MIT). These are not unseen
data. No real corpus, research holdout or new optimizer design. Baseline/reference
is exhaustive raw-polynomial evaluation in exact rational arithmetic, checked
against the existing upstream-loader/recomputation oracle for all544 assignments.
Reuse the strict exporter read-only: diagonal -h, upper pair -J/2, objective
F=-E+c. Original MERZ adapter, original builds/runs and production code unchanged.
New C++ callback wrapper and Python driver only; separate cache executable.
No public API changes, new dependencies or Rust kernel changes.

## Frozen cells and termination

12models ×3seeds ×4modes =144cells in model,seed,mode manifest order.
Cooperative modes use a2s end-to-end monotonic watchdog; watchdog mode uses100ms.
300s campaign cap. Conversion, model construction, launch and waiting are charged
to the process watchdog. No performance admission is inferred from these limits.
Parent and child observed CPU0, one thread; reject affinity mismatch. Retain exact
commands, environment, upstream/compiler/flags/library/binary/source hashes,
raw stdout/stderr/partial line, actual wall duration, process status and all emitted
records. Prefix trials and failures remain evidence if the campaign is incomplete.
Commit protocol, then reviewed instrument before any upstream qualification run.

Callback emits every reported incumbent with sequential call index, objective,
binary state, decision to continue, own relative monotonic elapsed time, upstream
reported runtime (diagnostic only), and observed child affinity. Both Report
overloads forward to the same implementation. Stop is latched: after false it
never returns true again. Constructor may invoke callback again after false from
STS; this is allowed. After cooperative return emit final stored best, callback
count and stop status. Parent independently verifies every complete emitted state.
No fallback, clamping, optimum substitution or silent stream filtering.

- **first:** return false at the first call and all subsequent calls. At least1
  callback, no true returns, exactly1 final record, exit0 before watchdog.
- **third:** return true for calls1/2, false from call3 onward. At least3 callbacks,
  exactly1 final record, exit0 before watchdog. This checks genuine continuation.
- **deadline:** use callback's own monotonic elapsed time, beginning immediately
  before heuristic construction, to latch false at>=20ms. Never use upstream's
  gettimeofday runtime for the decision. Require a false callback, final record,
  exit0 before watchdog; no claim of immediate polling or20ms end-to-end runtime.
- **watchdog:** callback always returns true. MQLib's supplied20ms runtime argument
  is bypassed when a callback exists. Parent kills at100ms; require expected
  timeout/SIGKILL,>=1 complete valid callback and no final record. A trailing
  partial line is retained censoring, not accepted as a candidate. Timeout in any
  cooperative mode, early normal exit in watchdog mode, or malformed complete
  line is FAIL. Oversize stdout (>2MiB), schema/domain mismatch, nonfinite value,
  wrong polynomial energy or unlatched stop is FAIL.

Callback objectives must be nondecreasing; final stored state/objective must equal
the final callback's incumbent. Objective tolerance is absolute1e-9 against exact
rational raw energy; dyadic oracle identities must be exact. Relative monotonic
callback stamps are nonnegative/nondecreasing. Upstream runtime is recorded,
finite/nonnegative but is not a trusted clock. Parent captures complete stdout
at termination; this does NOT qualify per-message delivery times/anytime curves.

## Verification and evidence gates

Before execution: review the source dependency map and protocol, then test exact
sign/factor/offset mutations, missing/nonbinary/extra records, callback latch,
mode-specific timeout/exit behavior, and source-pin/build verification. Build with
strict warnings for new wrapper; preserve upstream warnings/build record.
No optimizer runs in unit tests. Independent read-only instrument review and
committed freeze precede the single main run; no unregistered optimizer smoke.

All544 oracle states and144cells must be valid for overall PASS. A watchdog cell
is valid only on its expected termination path; it is not a failed solve. Record
best emitted energy and tiny exhaustive gap descriptively, never compare modes
or solvers. Keep every candidate and rejected output if a failure occurs. Stop
on first failure, publish FAIL/INCOMPLETE as applicable. No rescue rerun.

Metadata and reproduction README bind source/build provenance and invocations.
Postrun independent audit rechecks raw records, every candidate's exact polynomial
energy, exhaustive oracles, termination/callback rules and source hashes without
importing the adapter's validator. Separate reviewer confirms bounded conclusion.
No claim of exact rerun reproducibility without an independent execution: a raw
artifact audit is not an optimizer replication. Time-budget runs may vary.

If PASS, this admits only the specific standalone bridge and tested callback
contract on these tiny inputs. Larger dimensions, runtime scaling, stronger
baseline status and actual matched-budget streaming remain unqualified. Next
prepare a new difficult-corpus comparison after its own gates; no automatic
comparison campaign follows this engineering qualification.
