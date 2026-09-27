# MQ-MST2-001 — standalone MST2 witness/termination qualification PASS

2026-09-28. Closed engineering result under the [protocol](protocol.md).
**Registered PASS; independent raw evidence audit PASS.** H1 is SUPPORTED only
for the specific pinned bridge and twelve exposed artificial fixtures n1..8.
No comparative-strength, speed, arbitrary-size or independent-rerun claim.

## Evidence

Protocol `f6fa9e9`, reviewed instrument `f738f4d`, retained raw/audit `d47c40c`.
One registered run; no optimizer smoke, retry or outcome-selected rerun.
[Summary](run001/summary.json), [source/build/environment provenance](run001/metadata.json),
[executable offline audit](run001/audit.json) and
[independent reviewer record](run001/verification.json) retain the evidence.

| Registered check | Observed result |
|---|---|
| Exhaustive input/objective identity | 544/544 exact matches |
| Cells:12 models ×3seeds ×4modes | 144/144 VALID |
| Callback witnesses independently rescored | 1,606 valid |
| Cooperative final witnesses independently rescored | 108 valid |
| Expected parent watchdog terminations | 36/36 SIGKILL; no final record |
| Raw/source hashes checked by independent audit | 325 raw artifacts;11 frozen sources;6 upstream build artifacts |

All1,714 emitted callback/final states obey the original minimum polynomial and
MQLib sign/pair-factor/offset mapping. Final stored incumbents match the final
callback. Every complete record passes strict schema/domain checks. No stderr,
partial trailing line, cell error or missing callback was observed.
135/144 cells attain the tiny exhaustive minimum;9 do not. This descriptive count
is not an acceptance criterion or a comparison between stopping modes/solvers.

## Callback behavior actually exercised

| Mode | Cells | Callback count per cell | Termination |
|---|---:|---:|---|
| first | 36 | 1–2 | exit0, latched false |
| third | 36 | 3–4 | first two calls continue; then latched false and exit0 |
| deadline | 36 | 5–11 | own monotonic callback clock reaches20ms; then exit0 |
| watchdog | 36 | 22–43 | callback always continues; parent kills at100ms watchdog |

Thirty cells contain another callback after false. This confirms the need to
keep stop latched: returning false inside upstream STS can be followed by its
outer Report call. Callback-based termination overrides MQLib's runtime-limit
check; the wrapper therefore uses its own steady clock, and the parent uses a
monotonic watchdog. Upstream gettimeofday runtime is diagnostic only. The20ms
callback threshold is not a strict process/end-to-end deadline.

The campaign elapsed6.325021969s under its300s cap. This is descriptive execution
metadata, not a throughput, latency or competitive measurement. Parent/child
observed CPU0 and one-thread settings were verified. This driver collects stdout
at termination, so it does not qualify per-message receipt timestamps or an
anytime quality curve. Previous50/100ms model-preparation qualification cannot
be transferred to this optimizer or to larger inputs.

## Verification and scope

[Preflight](preflight.json):13 fabricated/mock tests PASS; no optimizer runs in
unit tests. They exercise sign/pair-factor/offset corruption, nonbinary states,
malformed schemas, stop latching, own-clock decisions and timeout retention.
Two initial review findings were fixed before execution: oversized numeric JSON
now becomes a validation failure, and explicit failure retains priority over a
coincident campaign cap. Strict C++ `-Wall -Wextra -Werror` build and Python
syntax checks PASS. No Rust/Cargo changes; no unrelated Cargo suite rerun.

The first reviewer's service usage limit is retained in preflight. A separate
read-only reviewer completed final instrument review, repeated all13 fabricated
tests and later reran the independent offline auditor. This is independent
artifact validation, not independent execution/replication of the optimizer.

Original MERZ adapter/builds, old research records, production kernels, APIs and
Cargo remain unchanged. Upstream MQLib is unmodified at
`585496274af5abb0849d0d47e135496b4688680b`; the new executable/cache are separate.
The tiny cases were already exposed correctness data, never claimed as holdout.
No real dataset, selector training, comparative wave or record hunt was run.

Offline verification without optimizer execution:

```bash
python3 research/experiments/mqlib_mst2_qualification/independent_audit.py \
  research/experiments/mqlib_mst2_qualification/run001
```

Historical invocation/commit are in [reproduction.json](run001/reproduction.json).
Never overwrite evidence or retry for better quality. A future independent
replication needs a fresh output and explicit experimental scope; time-budget
search need not return identical witnesses.

## Next gate

The candidate baseline now has a qualified tiny-input objective/witness/callback
contract. Whether MST2 is stronger than MERZ or our solvers remains untested.
Next freeze difficulty qualification separately from untouched holdout, define
predeclared targets and baseline configurations, and review actual benchmark
wrappers with common parent-receipt/deadline semantics. Resolve parent S3/X3,
corpus provenance/licensing and holdout gates before a comparison wave. The303
inventoried files are still not established unseen. No automatic comparison
campaign is authorized by this result.
