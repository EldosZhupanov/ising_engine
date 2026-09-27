# MQ-CAL-001 — no-search delivery timing calibration

Date: 2026-09-27. **Binding prospective protocol; NOT RUN.**
This is a scoped prospective instrument-qualification amendment to the
[parent](../../EXTERNAL_COMPARISON_PROTOCOL.md) and
[MQ-DESIGN-002](../mqlib_screen/NEXT_DESIGN.md). The user's instruction to perform
inventory and prepare calibration authorizes preparation. This protocol admits
only the no-search measurement control described here after instrument review;
it does not open an external solver comparison or waive S3/X3 for one. No
BiqMac, Gset, reserved seed, learned model, or optimizer is used.

## Question and estimand

Can a cold-start echo instrument distinguish a known added delay and receive
complete valid messages within candidate short deadlines, with bounded scheduler
and delivery error? This qualifies this **Python echo measurement path only**.
It cannot qualify native solver startup/model construction, select a solver,
prove speedup or authorize all native 10ms runs. Passing is a necessary
instrument check, not sufficient native workload calibration. A failed cell
means this proxy configuration failed that budget, not a universal clock limit.

## Frozen inputs and design

[manifest.json](manifest.json) fixes both artificial JSON fixtures, hashes,
CPU0, host/Python version, budgets10/25/50/100ms, 20 repeat blocks, and rotating
arm order. Fixtures are n8 sparse and n128 dense; all-zero state has energy0.
No random numbers or research seeds; repeat indices0..19 are scheduling labels.
Each fixture × budget × repeat has three arms: A and B run exactly identical
zero-delay worker logic; D adds budget/4 sleep before emission. There are
2×4×20×3 = **480cells**,22.2s nominal summed budgets; total cap120s including
all overhead. Cap exceedance is INCOMPLETE, preserve all data, no replacement.

The worker is the same Python executable in every arm. It reads the exact input
path written by the parent, parses JSON, constructs the zero vector, directly
recomputes its polynomial energy, then optionally sleeps. No search is allowed.
Record monotonic_ns immediately before/after sleep and before a single complete
JSONL message is written and flushed. Keep the child alive without producing more
output until the parent terminates it at deadline. Reuse no persistent worker.

Timer starts before input serialization/tempfile/launch. Parent uses a monotonic
clock; same-host child timestamps are diagnostic and checked against parent's
clock domain. Use taskset CPU0 and one thread, no concurrent calibration workers.
Common immutable fixture disk read and post-run audit are outside the clock;
worker file read, parsing and construction are inside. Record raw bytes,
receipt timestamp, setup time, cutoff, kill/reap time, exit, stderr and hostload.
Only complete lines received strictly before budget qualify. Retain partial and
late output. Parent recomputes zero-state energy independently. Unexpected exit,
invalid state/energy/schema, non-monotonic timestamps or source mutation invalidates
the instrument: stop and retain failure. A deadline miss without corruption is a
valid censored observation, never silently imputed as completion at deadline.

## Frozen decision rule per fixture and budget

Let B be budget, d=B/4 and t the first complete on-time receipt from clock start.
Report each arm's completion fraction, timing distribution, deadline overshoot
(cutoff-B), delivery lag(receipt-child_pre_emit), actual injected sleep duration,
and within-block A-B/D-A differences. Late/absent receipts remain censored.

PASS requires **all** of:

1. All60 cells valid; each arm has >=19/20 on-time receipts.
2. At least19 complete A/B pairs and19 complete D/A pairs; publish their counts.
3. Nearest-rank q95 of all60 deadline overshoots <=max(1ms,0.05B), none negative.
4. Nearest-rank q95 delivery lag for all on-time messages <=max(1ms,0.05B),
   none negative; missing/late messages separately reported under conditions1–2.
5. abs(median(tB-tA)) <=max(2ms,0.1B) on complete paired blocks.
6. abs(median(tD-tA)-d) <=max(2ms,0.2d) on complete paired blocks, and
   >=18 of20 blocks show on-time tD>tA (missing pair counts against18).
7. For emitted D messages, actual sleep duration>=d and nearest-rank q95
   oversleep<=max(1ms,0.1d). Require>=19 such on-time D messages.

q95 uses sorted index ceil(0.95*N)-1. Medians use the arithmetic midpoint for an
even count. These are engineering tolerances fixed before observation, not
statistical superiority claims. Conditional latency summaries are not uncensored
population estimates; acceptance additionally requires the high completion rates.
Report all eight fixture/budget cells. A grid point qualifies both fixture shapes
only if both pass. No monotonic interpolation, relaxed thresholds or best-of-retry.
An overall FAIL does not erase independently reported per-cell outcomes.

## Instrument, audit and stop gates

Before main: commit this protocol/manifest/fixtures, then commit instrument after
independent review. Tests must catch exact-boundary/late acceptance, premature
EOF, invalid state, bad timestamp, missing pairs and quantile/count/positive/null
threshold failures on fabricated records. Unit-test fake clocks may not tune
criteria from real calibration outcomes. A one-cell schema smoke uses B=0.2s and
no delay, separate from registered cells; it checks valid parsing/termination,
not the registered timing thresholds. Preserve smoke and all failures.

Record exact instrument commit, dirty status, dependency/source hashes, Python
executable/hash, taskset version, kernel/CPU/affinity/clock info and all actual
commands. Assert committed bytes and fixture hashes before/after. No package
installation or production solver changes. A source change during collection
invalidates the run. No pilot timing on registered cells before freeze.

Independent post-run audit must rescore every witness, validate all timing/count
rules and recompute each gate directly from raw events. Store metadata.json,
raw.jsonl, summary.json and reproduction command in a fresh run directory.
A failed or inconclusive calibration receives the same retained evidence as a
pass; changed instrumentation/criteria require a new prospective amendment and
new output directory. No automatic solver benchmark follows this calibration.
