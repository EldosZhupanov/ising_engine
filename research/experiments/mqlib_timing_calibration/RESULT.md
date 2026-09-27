# MQ-CAL-001 — only 100ms qualifies both echo shapes

Date: 2026-09-27. Registered overall outcome: **FAIL**. Evidence audit: **PASS**.
[Protocol](protocol.md) `2d9cb9c`; instrument `3aaac7c`; retained raw/audit
artifacts `689fbe2`. One schema smoke and one main run; no rescue repeats.

## Measured result

All **480/480 measurement records VALID**, collected in23.711729142s against
22.2s summed nominal budgets and120s campaign cap. Here VALID means intact
measurement/censoring, not completion before the deadline. **236 complete worker
messages** were retained and independently rescored:234on-time,2late;244cells
had no complete message. The separate200ms smoke had one valid witness.

Each table count is on-time messages out of20repeats. A/B are identical zero-delay
workers; D adds one quarter of the budget as sleep. All seven gates must pass.

| Artificial input | Budget | A | B | D | Registered gate result |
|---|---:|---:|---:|---:|---|
| n8 sparse | 10ms | 0/20 | 0/20 | 0/20 | FAIL |
| n8 sparse | 25ms | 0/20 | 0/20 | 0/20 | FAIL |
| n8 sparse | 50ms | 20/20 | 20/20 | 20/20 | PASS |
| n8 sparse | 100ms | 20/20 | 20/20 | 20/20 | PASS |
| n128 dense | 10ms | 0/20 | 0/20 | 0/20 | FAIL |
| n128 dense | 25ms | 0/20 | 0/20 | 0/20 | FAIL |
| n128 dense | 50ms | 20/20 | 20/20 | 14/20 | FAIL |
| n128 dense | 100ms | 20/20 | 20/20 | 20/20 | PASS |

The complete gate breakdown, distributions, actual sleep, receipt lag and
teardown evidence remain in [summary](run001/summary.json) and [raw](run001/raw.jsonl).
At n128/50ms the positive-delay arm fails the completion/pair/sign/sleep-count
gates; this is not an energy correctness failure. Shorter-budget rows also have
censoring and, in some cells, deadline-overshoot failures. No absent observation
was treated as a completed runtime equal to its budget.

## Scope of the decision

Only the **tested100ms point** passed for both artificial shapes on this host,
under this exact cold-start Python echo implementation and its frozen tolerances.
There is no interpolation to untested times or admission of native solver budgets.
The overall grid FAIL is retained alongside its passing cells, not replaced by a
success label. No timing threshold, fixture, arm or source changed after data.

Workers parse a serialized artificial model, construct its zero vector and emit
one independently checked message; they perform no optimization. Parent and
worker are pinned toCPU0. Startup, serialization, file read and parsing contribute
to the cost. These results neither diagnose a universal clock-resolution limit
nor establish Rust/C++ startup cost, speedup, solver hardness or solver quality.
A different runtime or persistent-worker contract requires a new scoped protocol.

## Verification and reproduction

Independent agent review checked the instrument before execution and then ran the
[separate auditor](independent_audit.py), which imports no instrument code. It
verified nine source/executable hashes per run, committed source, fixture hashes,
exact schedule, raw-byte conservation, states/energies, monotonic ordering,
receipt eligibility, kill/reap timestamps and all seven gates. [Audit](run001/independent_audit.json):
PASS. Porting its repository-root lookup from an absolute path to a relative one
preserved byte-identical audit output. No workers were rerun by the reviewer.

Main raw SHA256:
`0c29db636ac43c32fcac734c422e6eed045fd8e98a8a96ad86b825f45af71f06`.
Smoke raw SHA256:
`d4634f26620772eb522090095fd40a11fcff8d3083ca47ce95fa5fa4075e0fc5`.

Twelve fabricated-record tests passed, covering every gate, invalid-row precedence,
malformed messages, clocks, exact-cutoff/late observations, EOF and missing pairs.
Python syntax and diff checks passed. Source changes are isolated research Python;
no production solver/Cargo changes and no Rust test rerun are claimed. Review
findings/fixes and verification commands are in [verification.json](verification.json).

Read-only validation on the matching executable environment:

```bash
python3 research/experiments/mqlib_timing_calibration/independent_audit.py research/experiments/mqlib_timing_calibration/smoke001
python3 research/experiments/mqlib_timing_calibration/independent_audit.py research/experiments/mqlib_timing_calibration/run001
python3 -m unittest discover -s research/experiments/mqlib_timing_calibration -p 'test_*.py' -v
```

Fresh reproduction, retaining new directories and accepting possible timing drift:

```bash
python3 research/experiments/mqlib_timing_calibration/calibrate.py --smoke --output /tmp/mq-cal-smoke-new
# Require schema smoke PASS; main checks matching source/executable fingerprints.
python3 research/experiments/mqlib_timing_calibration/calibrate.py --smoke-record /tmp/mq-cal-smoke-new --output /tmp/mq-cal-main-new
```

The runner requires the registered OS/Python/CPU/clock configuration. Metadata
records actual commands, thread environment, RAM, affinity, executable hashes and
load. Host or implementation changes require a new prospective calibration version;
old runtime numbers need not reproduce on different machines. Raw artifacts never
get overwritten by reproduction.

## Next task

Prepare a separate **native no-search readiness calibration** using the actual
Rust/C++ input conversion/model construction and output paths on artificial data.
Do not assume the Python echo's startup distribution transfers to native workers,
and do not silently switch to warm-start timing. That probe needs a new frozen
protocol and instrument review. Real solver comparison remains gated by adapter
qualification, verified targets and an untouched split; MQ-CAL-001 waives none
of those requirements or the parent's S3/X3. No real/reserved instance was opened.
