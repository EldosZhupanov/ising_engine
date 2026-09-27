# MQLib adapter qualification — MQ-QUAL-001

Scope: engineering correctness of a standalone external-process adapter.
The [binding amendment](../EXTERNAL_COMPARISON_AMENDMENT_1.md) freezes the tiny
synthetic corpus and 36 calls. This is not a comparative experiment.

## Reproduction

```bash
bash scripts/setup_mqlib.sh
python3 -m unittest discover -s research/mqlib_qualification -p 'test_*.py' -v
# Requires the reviewed instrument to be committed. Choose a new directory.
python3 research/mqlib_qualification/qualify.py --output .cache/mqlib-reproduction
```

Setup requires Git, GNU make, g++, ar, Python 3 and flock. It installs nothing
system-wide. MQLib's unmodified MIT source is pinned to
`585496274af5abb0849d0d47e135496b4688680b` in `.cache/mqlib-qualification/upstream`.
Build warnings from upstream are retained in build.json, not suppressed.

## Contract and boundary

The [adapter](../../benchmarks/adapters/run_mqlib.py) takes JSON with exactly
`offset`, `linear`, `pairs`. Each pair `[i,j,J]` has zero-based `i<j`, no duplicate
or diagonal pair. It minimizes `c+Σh_i*x_i+ΣJ_ij*x_i*x_j`. Coefficients must be
finite, have magnitude at most 1e12 and permit lossless halving of pair weights.
Integers in this range are exactly representable in binary64; floats mean their
binary64 values. Nonfinite values, unknown fields and malformed states fail.

MQLib's QUBO input maximizes a symmetric matrix form, so export diagonal `-h`
and upper-triangle `-J/2`. The offset stays in the record. Output is independently
checked with exact rational arithmetic; reported objective must agree within
absolute 1e-9. Large or ill-conditioned inputs may fail this strict check;
passing small fixtures is not universal numerical certification.

Example standalone invocation, using a previously retained fixture:

```bash
python3 benchmarks/adapters/run_mqlib.py MODEL.json \
  --binary .cache/mqlib-qualification/upstream/bin/MQLib \
  --seed 101 --seconds 0.02 --output NEW_RESULT.json
```

Only `MERZ2002ONEOPT` is qualified. The generic CLI records the supplied binary's
hash; **it does not certify its origin**. The qualification driver additionally
verifies the pinned checkout, build manifest and committed instrument.
No MaxCut/Ising frontend, arbitrary CSR import, hyperheuristic, warm start or
production Rust harness integration is provided. Existing old adapters remain
unchanged; their omitted offsets and permissive parsing are separate debt.

Requested search runtime, measured process wall time, conversion/setup and total
adapter time are separate fields. The process limit is a watchdog, not a matched
competitive budget. Time-limited randomized search need not return identical
states on repeated executions. No speed or statistical comparison is made.
Upstream objective-history text is retained verbatim, not qualified as an
independently verified anytime curve. Only final candidates are verified.

## Evidence lifecycle

Before execution: protocol and instrument are committed and independently
reviewed. Per run: metadata.json, twelve models/exports/oracle outputs, 36 raw
candidate records, summary and reproduction README. Outputs are exclusive-create.
All-state oracle values must match exactly for these dyadic fixtures. Candidate
energy and optimum gap are separate; optimality is not an acceptance condition.
Existing run artifacts are never rewritten. Actual execution status is recorded
in memory/NOW.md and the retained run summary after qualification.

Next research work needs a separately frozen design and reconciliation with the
parent protocol's S3/X3 prerequisites. MQ-QUAL-001 cannot authorize a campaign.
