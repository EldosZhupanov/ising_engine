# MQ-DIFFICULTY-001 — main not admitted

2026-09-28. Closed registered version. **Preflight FAIL; independent artifact
audit/review PASS. Main NOT RUN. H1 and H2 UNTESTED / INCONCLUSIVE.**

[Protocol](../protocol.md) `253d94a`; instrument
`f76991dde036f77fe7bb29cbf17efb1e20ec769c`.
[Raw data](../preflight001/README.md), [analysis](../preflight001/analysis.json),
[executable audit](../preflight001/audit.json) and
[independent reviewer record](../preflight001/review.json).

## What ran

One retained preflight, no retry:480 no-search native controls and32 short optimizer
smoke cells on already exposed tiny fixtures. All512records VALID as instrument
records; validity does not imply successful admission or an on-time witness.
1,029 complete messages,509 independently rescored witnesses, one late witness.
Of480controls,477have a complete witness and476have an on-time validated witness;
three have no complete witness. All32smoke cells have an on-time native witness.
Recorded elapsed time132.719519s;3000s cap not approached.

| Registered gate | Result |
|---|---|
| Control groups | 13/16 PASS; overall FAIL |
| q11 / MST2-labelled no-search mode | null-median difference10.961274ms exceeds10ms |
| q23 / MERZ-labelled delay mode | 8/10 on-time; two absent witnesses |
| q23 / MST2-labelled delay mode | 8/10 on-time; one absent and one late witness |
| Cutoff overshoot | PASS: p95=2.035656ms, maximum7.719838ms |
| Actual tiny callback smoke | PASS:32/32 on-time verified witnesses |

The late MST2-labelled **no-search** witness was received at248.796ms but finished
independent validation at254.451ms. Excluding it enforces the registered rule;
receipt alone would have wrongly credited this cell. Arm labels in controls
select preparation paths, not optimizer invocations.

No960-cell main run or main guard was created. The24generated inputs are still
qualification-only, not known hard inputs or an evaluation holdout. Four inputs
were used in no-search timing controls; all24coefficients/hashes were available
for generator verification. No selector training or reserved evaluation was used.
The search seeds61101..61110 were reserved but not executed in main.

## Interpretation and limits

The whole preregistered delivery-control gate failed. Do not waive its failed
groups, raise its threshold or retry until it happens to pass. Neither corpus
hypothesis has optimization observations, so this does not falsify H1/H2 or show
that any solver is weak, strong, slower or faster.

Descriptive diagnosis only: for q23, C++ parent setup medians ranged120.78–147.47ms
across the two labelled paths and three modes. This measured interval includes
conversion/serialization/tempfile work. It consumes a substantial portion of250ms;
it is not pure C++ startup or isolated parsing time. Host scheduling, conversion,
I/O and validation were not separately randomized causal interventions. Additional
null differences on q23 can be described, but its first reported gate reason is
missing probe witnesses. No single cause is established.

This does **not** establish inability to compare final solution quality at2s.
The registered design coupled its primary2s endpoint to reliable250ms diagnostic
controls. That conservative all-or-nothing admission failed; a different scientific
question requires a new prospective version, not retrospective partial admission.
No speed, generalization, S3/X3, learned-selection, record or superiority claim.

## Reproduction and engineering evidence

Instrument source/executable provenance and exact commands are in
[metadata.json](../preflight001/metadata.json). Independent read-only review
verified208source hashes, two executable hashes and1,540artifact hashes; no
optimizer rerun. Audit means artifact consistency, not independent repetition
of the stochastic experiment or an external human review.

Offline check, without optimization or changing raw data:

```bash
PYTHONPATH=research/experiments/mqlib_difficulty_qualification python3 - <<'PY'
from pathlib import Path
from independent_audit import audit
r = audit(Path('research/experiments/mqlib_difficulty_qualification/preflight001'))
print({k: v for k, v in r.items() if k != 'artifact_hashes'})
PY
```

Use the instrument commit and recorded commands for a separately labelled clean-
checkout replication; do not remove the one-shot guard to overwrite this run.
This closure lives in a subdirectory to leave the frozen instrument's direct
source-file inventory unchanged. Prior protocols/results remain byte-identical.

25Python tests (also independently rerun),2Rust algebra/parser tests,
`cargo check --workspace --all-targets`, targeted research-example Clippy with
warnings denied, `cargo fmt --all --check`, strict C++ build and diff checks PASS.
Production solver families, APIs and Cargo unchanged. No hosted-CI PASS claim.
A local catalogue-update script initially failed before staging the new README
entry; it was repaired and the unpublished raw-record commit amended only after
the memory gate passed. Raw measurements/instrument were not edited or rerun.

## Next decision

Design a **separate coarse2s final-quality qualification** on the entire frozen
24-input qualification set, preserving this exposure history and all four
configurations. Do not inspect outcomes to select a favorable subset. Treat
subsecond speed resolution as a different optional question; do not make a2s
quality comparison contingent on unsupported10ms discrimination without a reason
linked to that estimand. Freeze new admission rules before new observations.
Keep conversion and independent validation charged, verify actual deadlines,
report readiness/fallback/load uncertainty, and retain the current failure.
No automatic retry or main run under MQ-DIFFICULTY-001 is permitted.
