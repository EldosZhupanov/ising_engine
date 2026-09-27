# MQ-DIFFICULTY-001 — qualification of fixed-wrapper discrimination

2026-09-28. Binding prospective preregistration and narrowly scoped amendment.
Status at registration: **HYPOTHESIS; no search or timing observations collected**.
The [manifest](manifest.json) and [input index](instances.json) are part of this
preregistration. Their bytes, generator and input files freeze with this commit.
Manifest SHA-256: `0adfd1b47472877c99d332d66f27b90839fe8ad371695ed8778b35de8c64e3a3`.

## Authority, question and boundaries

User authorization: continue the reviewed next step after MQ-MST2-001, in the
context of the requested comparison. For this one diagnostic qualification,
explicitly supersede the [parent](../../EXTERNAL_COMPARISON_PROTOCOL.md)'s
S3/X3 entry prerequisites, selector/random/greedy arm set, BiqMac-only corpus,
held-in/held-out routing and mandatory TTS/best-known-gap endpoints. This is
**not** a Wave 1/2 experiment and does not satisfy or remove S3/X3 for those waves.
Use [NOW](../../../memory/NOW.md), as in the existing handoff amendment.
[MQ-DESIGN-002](../mqlib_screen/NEXT_DESIGN.md) motivates this separate synthetic
qualification branch; it does not confer execution permission by itself.

Question: do the four frozen wrappers produce meaningfully different delivered
solutions on these inputs at equal application cost? H1: at least 8 of 24
instances have a best-to-worst fixed-arm mean energy spread of at least 0.001L
at two seconds. This is an operational corpus gate, not a superiority test.
H2: at least two different arms each have at least four unique instance wins,
beating every other arm's seed mean by at least 0.001L. H2 tests whether this
corpus motivates further selection research, not whether any selector works.
Neither hypothesis asserts that larger inputs are hard or have known optima.

No learned KB, training, dynamic controller, parameter tuning, record attempt,
new optimizer or changes to production code, Cargo or public APIs. No automatic
confirmatory wave. Native model-path calibration and tiny MST2 qualification do
not license full-search speed claims on these larger inputs.

## Corpus, provenance and prior exposure

24 qualification-only files `instances/q00.json` through `q23.json`; exact hashes,
lengths, pair counts, L values and generator seeds in the manifest. Six instances
per n in {256,512} crossed with edge probability {1/16,1/2}. Edge decisions and
weights use the SHA-256 domain in `generate.py`, not process-global RNG. For each
included edge, integer w is selected from {-8,...,-1,1,...,8}. Write the exact
binary expansion of H(s)=sum w_ij s_i s_j, s_i=2x_i-1:

- offset c=sum w;
- h_i=-2 sum_j w_ij;
- pair J_ij=4w_ij, i<j, each pair stored once.

Thus E(x)=c+sum h_i x_i+sum J_ij x_i x_j. Normalization is
L=sum |h_i|+sum |J_ij|, **excluding c**. It is not an optimality gap.
All inputs belong to one generator family and remain qualification data forever.
No development/training/evaluation partition is claimed. No holdout is created
or opened; a future evaluation needs its own family-separated corpus/protocol.
The parent's BiqMac direction and its unresolved exposure/license/target issues
are unchanged. No third-party dataset is copied here; no license is inferred
for the repository or these generated files from MQLib's MIT license.

Generators 62001..62024 and search seeds 61101..61110 were checked against
tracked declarations/references and the task's burn/reservation map, with no
collision found. This is not proof about every historical derived RNG stream.
All four arms share a base seed label; their random streams differ. Earlier
engineering seeds 501/502 are reused only on already exposed tiny smoke inputs.
During seed checking an overly broad numeric grep incidentally printed part of
closed `research/cd005_equal_time/raw_q2.jsonl`; it was discarded as an irrelevant
numeric match. No reserved holdout was opened and no choice here uses its output.
Inputs were generated and algebra-tested before registration, without optimization.

## Frozen baseline definitions

Production source base is the manifest's full commit (945a72b...). Implementing
new orchestration must not alter its algorithms or the following configurations.
All builds record source and executable SHA-256; the committed instrument SHA
must precede any preflight observation. MQLib is pinned to
`585496274af5abb0849d0d47e135496b4688680b`, with unchanged native defaults.

1. `ultimate`: the completed-chunk policy in `research/examples/mqlib_compare.rs`:
   `UltimateSolver::new(hot,0.1,8,8,Some(chunk_seed)).with_2opt(true)`,
   hot=max(1,max_i(|h_i|+sum_j |J_ij|)),64 lanes,10 temperatures, one slice/pop.
   Repeated independent completed solves; no mid-solve witnesses or warm starts.
2. `v2_default`: same example's `DecisionEngine::default_plan(ir,registry,32,32,
   chunk_seed)`, standard registry, built-in ladder4.0..0.08, zero initialization.
   Publish the actual backend/operators/temperatures; no learned KB or controller.
   Repeated independent completed chunks, with no budget-dependent retuning.
3. `mqlib_merz`: native `Merz2002OneOpt`, continuing callback, as in the original
   screen. Publication on first callback and strictly improving objective only.
4. `mqlib_mst2`: native `Palubeckis2004bMST2`, same publication policy as MERZ.
   Callback always returns true; parent owns stopping. MQ-MST2-001 established
   that a callback overrides the native runtime cutoff. No cooperative deadline
   or false-then-true callback; no alteration of native search mathematics.

Rust chunk seed = base_seed*0x9e3779b97f4a7c15 + chunk_index modulo2^64.
MQLib `srand(base_seed)`. Both MQLib calls receive runtime3600s with callback;
this is not their delivered budget. Converter maximizes c-E: negate h on diagonal
and split -J/2 symmetrically. Verify each native objective equals c-E exactly
within1e-9. All input arithmetic/results fit exact integer precision in f64.
The wrapper may add no-search probe/affinity metadata without changing search.
These are configured wrappers, not tuned best-in-family methods or hyperheuristics.

## Application cost and output eligibility

One parent and one child pinned to CPU0; require observed affinity {0} for both,
including readiness messages. Require single-thread settings in manifest and
record actual process thread observations; unexpected runnable worker threads
invalidate the contract. Record host load and affinity, not only taskset command.
Sequential cells, no other benchmark/build/optimizer concurrently started.

Timer begins before conversion, serialization, tempfile creation and launch.
Include model construction, both representations built by the reused Rust
wrapper, search, restarts, output, parent parsing and independent validation.
Reading/hash-checking immutable corpus and preparing a common all-zero fallback
are outside the cell timer for every arm. All-zero E=c is available at time zero.
Build time and postrun audit are outside search time and separately recorded.

For every complete JSONL line record receipt time AND validation-completion time
using the parent's monotonic clock. Eligibility requires **both strictly before**
the checkpoint. Batch receipt timestamps cannot bypass validation cost. Scoring
uses the best eligible raw-polynomial-verified witness or the zero fallback.
This is a deliberate prospective tightening of the earlier receipt-only contract.
No subtraction of startup medians. No child-reported timestamps used for scoring.
Record setup/model-ready, receipt, validation, cutoff, kill and reap separately.

Hard-kill the process group at 2s, reap with at most5s allowance. Kernel scheduling
overshoot does not extend eligibility. Retain late/partial/raw stdout and stderr;
complete late witnesses are checked off-budget but cannot improve scores. Invalid
witness/energy, affinity mismatch, abnormal early exit, malformed output or
unexpected thread configuration => INVALID, retain artifacts and stop. A killed
incomplete line or no delivered solve at cutoff is censored/fallback-only, not an
invalid solution. Bound per-line output to1MiB and total stdout per cell to16MiB;
overflow is INVALID, not a silently truncated success.

## Mandatory instrument qualification before main

Instrument and executable provenance must be committed and independently reviewed
before preflight; no search on qualification inputs until every preflight gate
passes. The commands to implement are `build.py`, `run.py --phase preflight
--output NEW_DIRECTORY`, `run.py --phase main --preflight PREFLIGHT_DIRECTORY
--output NEW_DIRECTORY`, and `independent_audit.py RUN_DIRECTORY`. No existing
output may be overwritten. At registration these commands are an implementation
contract, **not a claim that an executable harness already exists**.

1. Tests: exhaustive tiny-model conversion and direct energy identity; malformed,
   nonfinite, duplicate and oversized messages; exact-cutoff, late, partial and
   post-kill behavior; delayed validation cannot earn early receipt credit;
   affinity/provenance mismatch; duplicate/missing cell; cap/abort preservation.
   Hand-constructed analysis cases test ties, insufficient complementarity and
   threshold boundaries. No qualification-instance optimization in unit tests.
2. Native no-search controls: q05,q11,q17,q23, all four worker modes; null_a,
   null_b and delay50ms, ten repetitions,250ms budget each (480 cells). Each uses
   the same input conversion, native parsing/model preparation, zero-state energy,
   output and parent-validation paths as main, omitting optimizer invocation.
   Delay after model preparation before witness emission, measured by child clock.
   Order fixture then repetition; rotate arms by(fixture_index+rep)%4 and modes
   by(fixture_index+rep+arm_index)%3. For each of16fixture/arm groups require all30
   valid control cells and on-time validated zero witnesses; abs(median(null_a)-
   median(null_b))<=10ms; median paired(delay-null_a) in[25,75]ms;
   median measured injected sleep in[45,75]ms. Across480cells cutoff overshoot
   p95<=25ms (sorted index ceil(.95N)-1) and max<=100ms. Fail any => stop, no main.
   These qualify delivery instrumentation, **not optimizer initialization**.
3. Actual callback smoke: four exposed MQ-QUAL-001 tiny inputs whose hashes are
   in manifest, seeds501/502, four arms,250ms (32cells). All valid with at least
   one eligible verified native witness, including all required config/affinity
   records, or stop. Retain every cell/failure. No claims from smoke quality.

No rescue reruns under this version, including failed preflight. A code change
needed after observations requires an amendment/new version naming exposure.
A single independently reviewed successful preflight licenses one main run only.
Actual full-search setup is charged and observed in main; preflight doesn't assert
that any optimizer will produce a witness on larger inputs before a checkpoint.

## Main schedule, caps and environment

Seeds are the ten explicit manifest values61101..61110;24×10×4=960cells.
Instance order q00..q23, then seed order; rotate the ordered four-arm list by
(instance_index+seed_index)%4. One2s trace per cell; descriptive checkpoints
250ms,500ms,1s,2s are prefixes of that run, not independently tuned budgets.
Nominal charged time:120s controls +8s smoke +1920s main=2048s.
Total elapsed cap for preflight plus main, excluding builds/offline audits:3000s.
Enforce remaining cumulative allowance before every cell, retain ABORT on cap;
no inference from incomplete schedule and no resuming/retrying selective cells.

Same WSL2 Linux6.6.114.1, AMD Ryzen7 170 host; full OS/CPU/RAM, Python, Rust and
C++ versions, flags, source hashes, upstream clean pin, binary hashes, observed
affinity/thread settings and load are recorded. Manifest records current toolchain.
Rust release/default features, empty RUSTFLAGS; C++11 -O2 with warnings denied.
Changed hardware, compiler, flags or source-base algorithms require prospective
amendment; available-memory/load fluctuations are recorded, not hidden exclusions.

## Analysis, falsification and report

For each instance, arm and checkpoint report all ten energies, fallback counts,
mean/median/std/min/max and paired win/tie/loss counts. Independently rescore every
complete witness and recompute checkpoints from raw bytes and parent timestamps.
No scoring from worker self-reported best alone. If any required cell is invalid,
missing or duplicated: **NO VERDICT**, preserve partial results without winners.

At2s compute each arm's mean over seeds. H1 gate uses (worst mean-best mean)/L;
>=8instances above margin => SUPPORTED on this qualification sample, else
INCONCLUSIVE (corpus NOT QUALIFIED by this rule). Publish best-to-second-best
spreads too: H1 can pass merely because one arm is weak. H2 counts only unique
winners with every other mean worse by>=0.001L; >=2arms each winning>=4instances
=> complementarity SUPPORTED on this sample, otherwise NOT ESTABLISHED. No
selection branch on this corpus if H2 fails. No cherry-picking instances/strata.

Report descriptive95% percentile bootstrap intervals for each pair's mean
normalized difference across instances:10000 resamples, deterministic SHA-256
namespace `MQ-DIFFICULTY-001/bootstrap/v1`, draw indices from the four successive8-byte
big-endian unsigned chunks of SHA-256(counter as unsigned decimal, starting0,
prefixed by namespace plus `/`),
rejection sampling integers below floor(2^64/24)*24 then modulo24. Use the same
resample index matrix for all pairs. Quantiles use linear interpolation at
.025/.975*(9999). Instance, not seed/checkpoint, is the resampling unit; the one
synthetic family and reused qualifications preclude broad generalization. These
intervals are descriptive, not extra post-hoc significance/selection gates.

No certified/predeclared targets exist: omit TTS99, success-to-optimum and relative
optimality gaps. Do not turn pooled best-observed endpoints into speed targets.
No superiority/SOTA/equivalence claim, even if one wrapper dominates this sample.
Retain metadata.json, raw per-cell records/streams, analysis.json, review/test logs
and reproduction README with exact commands. Independent read-only review audits
all source/data hashes, schedules, budgets, energies and statistics before result
interpretation. Publish failures/nulls and update NOW/catalogue/research ledger.
