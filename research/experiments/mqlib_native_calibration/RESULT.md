# MQ-NATIVE-001 — measured native model preparation, no solver comparison

2026-09-27. Closed result under the [frozen protocol](protocol.md).
**Registered overall FAIL; independent evidence audit PASS.**
The 50ms and 100ms points pass all four tested worker/shape combinations.
Neither 10ms nor 25ms passes the whole path. This is instrument evidence, not
an optimization result, solver speedup or full solver-initialization measurement.

## Evidence and chronology

- Protocol/fixtures/manifest: `fb568c8`, before candidate execution.
- Reviewed instrument: `a25c595`; committed-source rebuild and linked-library
  provenance are retained in [metadata](run001/metadata.json).
- [Two-worker schema smoke](smoke001/summary.json): 2/2 PASS;
  [offline smoke audit](smoke001/audit.json): PASS.
- One main run, [raw records](run001/raw.jsonl) and [all statistics](run001/summary.json),
  retained at `a435e7a`; elapsed47.507818597s versus240s campaign cap.
- [Offline audit](run001/audit.json) and [independent agent review](run001/verification.json):
  PASS. All1,666 messages,960cells,234 source/executable hashes and all16 groups'
  six gates independently checked. Agent review is not an external human replication.

All960 measurement records are VALID. There are810 two-line completions:
807 before the deadline and3 late. Another46 cells contain one complete line,
104 contain none; hence150 lack both required lines. Six complete lines are late.
No partial bytes or stderr. Missing/late delivery is retained censoring, not an
invalid energy or a discarded run. Every emitted diagnostic's four fixed-state
energies match the raw polynomial; every emitted incumbent is the prescribed zero
state. No optimizer or candidate selection runs in this experiment.

## Registered grid

Each cell below represents20 repeats ×3 modes (two identical nulls and a known
sleep B/4). Parent and child share observed CPU0; one worker thread. Cold-start
conversion, serialization, model creation, fixed-state checks, output and parent
validation are charged under the registered contract.

| Worker / artificial input | 10ms | 25ms | 50ms | 100ms |
|---|---|---|---|---|
| Rust / n8 sparse | FAIL | FAIL | PASS | PASS |
| Rust / n128 dense | FAIL | FAIL | PASS | PASS |
| MQLib C++ / n8 sparse | FAIL | FAIL | PASS | PASS |
| MQLib C++ / n128 dense | FAIL | FAIL | PASS | PASS |

All480 cells at50/100ms deliver both lines on time and pass their group gates.
The full registered grid still FAILS; passing subsets do not reverse that result.

At25ms, Rust/n8, Rust/n128 and C++/n8 each deliver60/60 completions but fail the
cutoff-overshoot gate. Their q95 overshoots are respectively1.279,2.008 and1.420ms,
above the frozen1.25ms threshold. C++/n128 additionally has only15/20,12/20 and0/20
on-time completions for nullA/nullB/delay. At10ms all four groups fail the deadline
gate, and the larger inputs also fail completion/positive-control gates. Do not
change thresholds after observing these outcomes.

## Exact interpretation and limits

SUPPORTED: on this host, these frozen diagnostic model-preparation paths meet
all six registered gates at50 and100ms. Rust constructs QuboModel/ProblemIR and
OperatorRegistry; C++ constructs QUBOInstance and explicit-assignment solutions.
Both check fixed assignments and emit an incumbent plus a diagnostic line.
The extra diagnostics are measured; this is not isolated native startup latency.

UltimateSolver's field allocation and other initialization inside `solve()`,
v2 operator initialization inside `Runtime::run()`, and all search remain outside
this boundary. Passing50ms here does not qualify those operations or larger inputs.
A future comparison must retain the same affinity/timing contract and validate its
actual search wrappers. Conditional on-time latency distributions do not estimate
uncensored runtimes. No Rust-versus-C++ speed ranking is inferred from different
conversion/construction work. No CPU-isolation guarantee is made beyond observed
affinity; background load and WSL environment are recorded.

The earlier Python calibration and MQ-SCREEN-001 stay unchanged. Neither this run
nor the Python echo retrospectively calibrates the old screen's unrecorded parent
placement. No real benchmark coefficients, reserved outcomes, new algorithm seeds
or untouched holdout were used. Production solvers, APIs and Cargo are unchanged.

## Verification and next gate

[Preflight](preflight.json):14 Python tests,2 Rust tests, release Rust/C++ builds,
strict targeted Clippy and formatting PASS. Tiny exhaustive checks include all16
assignments with fractional coefficients and nonzero offset. A mocked-process
regression ensures parent validation happens before kill and consumes the budget.
Initial review findings (deferred validation and missing build flags) were fixed
before committed instrument execution. The original C++ serializer is included
at compile time; only its unused renamed legacy entry point has the locally
scoped `-Wreturn-type` suppression. New probe code otherwise uses warnings denied.

Offline reproduction of the evidence audit (no solver runs):

```bash
python3 research/experiments/mqlib_native_calibration/independent_audit.py \
  research/experiments/mqlib_native_calibration/run001
```

Original invocation and source commit are retained in [reproduction.json](run001/reproduction.json).
It describes the historical run, not authorization to overwrite or rescue it.

Next, qualify the candidate stronger MQLib baseline (PALUBECKIS2004bMST2) with
small exact sign/offset/witness and callback checks before a new difficult-corpus
comparison. Then freeze difficulty qualification separately from untouched
holdout, predeclare targets and actual-wrapper cost curves. The303 inventoried
benchmark files are not established unseen; parent S3/X3 and redistribution gates
remain. No automatic optimization-comparison wave follows this calibration.
