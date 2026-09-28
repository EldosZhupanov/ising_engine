# MQ-FIRST-CHUNK-001 — first delivered witness on exposed QUBO inputs

2026-09-28. Binding prospective diagnostic, written after the complete
[MQ-QUALITY-002 result](../mqlib_coarse_quality/closure/RESULT.md). The 60
dense-512 Ultimate runs that produced no witness by 2 s are known; this is an
informed mechanism probe, **not** an independent confirmation or a continuation
of the closed comparison. [Manifest](manifest.json) fixes the cells and hashes. SHA-256: `0b5689121281f3c2208f536249c13b838b87727dabfc03f12ed9205a47ff3282`.

## Question and limits

Does the unchanged Ultimate wrapper eventually deliver its first independently
validated witness on one previously exposed dense 512-variable input, and when?
Compare its first-witness delivery against an exposed sparse input of the same
size and against the unchanged `v2_default` wrapper as a small delivery-path
control. The instrument cannot measure time *inside* `UltimateSolver::solve()`;
it observes only readiness and completed output. No kernel-speed, solver-quality,
selector, S3/X3, holdout, generalization or optimality claim follows.

Use only old qualification files q12 (n=512,p=1/16) and q18 (n=512,p=1/2), in
that order, with their exact hashes in the manifest. Both were disclosed and
searched in MQ-QUALITY-002. Reuse old seed labels 61101, 61102, 61103 explicitly;
these are diagnostic replications on exposed cells, not independent test seeds.
Use `ultimate` and `v2_default` unchanged, in that order, for each
(instance, seed). Fixed order: q12 × seeds ascending × arms in order, then q18
likewise, **12 cells total**. No adaptive ordering or tuning.

## Measurement and falsifiers

Each cell gets a 10.0 s wall budget. Parent timer begins before conversion,
temporary file and process launch. Parent and child CPU0, inherited one-thread
environment and existing Rust release worker. The native worker is pinned at
`f76991dde036f77fe7bb29cbf17efb1e20ec769c`, its binary and dependency
hashes bound before execution. Ultimate uses
`new(hot, 0.1, 8, 8, chunk_seed).with_2opt(true)`; v2 uses its default
32-replica/32-sweep plan. The original `run_cell` helper and worker are not
edited; no production/Cargo/source change or new dependency is allowed.

Record readiness, first raw `inc` receipt, and first independent validation
completion. A witness counts at time `max(receipt, validation)` only if both
timestamps are **strictly below 10 s**. If none arrives, record right censoring
at 10 s; a late or partial message does not count. Verify its binary state and
raw QUBO energy `c+Σhx+ΣJxx` against the original input at absolute tolerance
1e-9. Full stdout, stderr, event times, source/binary hashes, host, CPU masks,
thread observations and kill/reap times are retained. All 12 cells must be
VALID, exactly scheduled, ready observed, child killed/reaped and kill requested
by 10.1 s. Any failure makes the registered diagnostic INVALID; preserve it and
do not retry this version.

Report the full 12-cell first-witness table, readiness medians/ranges, and
counts with first verified `inc` before 2 s and before 10 s. Predeclared
mechanism checks, **descriptive only**:

1. `C1`: q12/Ultimate first `inc` before 2 s on all three seeds, while
   q18/Ultimate does not deliver by 2 s on all three. Failure to reproduce
   either side weakens the delivery-pattern explanation.
2. `C2`: q18/Ultimate delivers at least one first verified `inc` in (2,10) s.
   If not, report three censored bounds, not an invented solve duration.
3. `CONTROL`: q18/v2 delivers first verified `inc` before 2 s on all three
   seeds. A failure challenges the host/path control, not an optimizer result.

Even a positive C2 supports only eventual completed output from the *whole*
worker call; if it fails, possible causes include a long first call, nonfinite
reported energy or another unobserved internal condition. A traceback or
`INVALID` cell blocks interpretation. No p-value or speed ratio is defined.

## Execution governance

Preregister and commit this file and manifest before instrument code or data.
Implement a separate one-shot coordinator and independently coded offline
auditor. Fabricated tests must reject late validation, invalid raw energy,
missing/duplicate cells, stale source/binary hashes and failed termination.
Independent read-only instrument review precedes the one run; independent raw
audit and review precede interpretation. Full actual CPU time is at most about
120 s plus setup/cleanup, with a hard registered total process cap of 180 s.
No rescue rerun or sampling of other inputs under this protocol. Retain a null,
negative or invalid result and update the canonical research ledger.
