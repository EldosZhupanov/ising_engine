# MQ-QUALITY-002 — prospective admission revision for two-second quality

2026-09-28. Binding preregistration and narrowly scoped prospective amendment.
[Manifest](manifest.json) SHA-256:
`284eaab2640322eb99346a99cc58575ef1ad327d6e4211f4cacdfe45ea27de84`.
Status: HYPOTHESIS; no observations under this version at registration.

## Why this is a new version

[MQ-DIFFICULTY-001](../mqlib_difficulty_qualification/closure/RESULT.md) failed its
250ms preflight:13/16 control groups passed. Its main search never ran. That
FAIL remains unchanged and its guard, entrypoint, raw files and source are frozen.
Observed delivery failures motivate this revision; it is not an independent
redesign uninformed by those data. In particular **the primary two-second H1/H2
question is unchanged**. We revise admission and remove descriptive short-budget
comparisons; we do not pretend to introduce a new primary endpoint.

The user authorized this continuation after the retained failure. For this
qualification only, prospectively supersede the
[parent protocol](../../EXTERNAL_COMPARISON_PROTOCOL.md)'s S3/X3 entry conditions,
mandatory selector/random/greedy arms, BiqMac corpus, holdout routing and mandatory
TTS/best-known-gap endpoints. Do not run old MQ-DIFFICULTY-001's main or treat its
failed preflight as successful. This separate entrypoint does not establish S3/X3
or authorize a selector/production/competitive-advantage wave. NOW is the handoff.

## Unchanged hypotheses, inputs and algorithms

Retain all24 qualification inputs and their exact SHA-256 from the predecessor,
without outcome-based filtering or regeneration: q00..q23, n256/512 crossed with
p1/16 or1/2, six per stratum. The manifest contains their root-relative paths and
hashes. Inputs commit253d94a; one generator family, wholly qualification-only.
Four inputs were exposed to no-search controls and all coefficients to generator
verification; no main optimizer outcomes existed. No unseen holdout, optima or
family-generalization claim. No new third-party data/license is inferred.

Transfer reserved-but-unused main search seeds61101..61110 from the closed version;
this is an explicit exception to the parent's reserved-block prohibition for
this continuation alone. Do not call these fresh independent replications.
Engineering seed501 and probe seed0 reuse exposed fixtures only.

The same four fixed wrappers, with no tuning: Ultimate completed chunks
`new(hot,0.1,8,8,seed).with_2opt(true)`; v2 default32replicas/32sweeps and built-in
ladder4.0..0.08; native MQLib MERZ2002ONEOPT and PALUBECKIS2004bMST2. All details,
chunk-seed rule, default settings and conversions are inherited exactly from
[the predecessor's frozen baseline definitions](../mqlib_difficulty_qualification/protocol.md#frozen-baseline-definitions).
Workers are the unchanged Rust/C++ sources at full commit
`f76991dde036f77fe7bb29cbf17efb1e20ec769c`; production source base945a72b and MQLib
`585496274af5abb0849d0d47e135496b4688680b`. Hash all dependencies and actual binaries.
No operator additions, learned KB/controller, training, warm starts, API or Cargo
changes. A new Python coordinator may call the old run_cell helper, but must not
invoke or monkey-patch its old main/admission/build globals. New scoring/auditor
belong in this directory; old source/evidence stay unchanged.

Raw objective E(x)=c+sum h_i*x_i+sum J_ij*x_i*x_j; minimize, i<j once. Independently
rescore each candidate from the original input, absolute tolerance1e-9. MQLib
maximizes c-E using -h diagonal and -J/2 off-diagonal. L=sum|h|+sum|J| excludes c.

- H1: at least8/24 instances have (worst fixed-arm seed mean minus best mean)/L
  >=0.001 at2s. This operational discrimination gate can pass because one arm is
  weak; also report best-to-second-best spread. It is not a superiority test.
- H2: at least two arms each have at least four unique instance wins over every
  other arm mean by>=0.001L. If absent, do not pursue selector work on this corpus
  from these data. It is not a learned-selector result.

No change to these hypotheses, margins, corpus or algorithm parameters follows
from the failed timing controls. No known targets => no TTS, optimality gap or
success-to-optimum. No short-time speed ratios or crossing-time claims.

## Cost and correctness contract

One parent and one child pinned to observedCPU0; inherited thread environment
RAYON_NUM_THREADS=OMP_NUM_THREADS=OPENBLAS_NUM_THREADS=1. Retain observed child
thread masks/counts (up to main plus one Rayon worker for Rust search, one thread
otherwise). No concurrent benchmark/build jobs. Same host/toolchain/profile/flags
as manifest; reject changes prospectively, record RAM/load fluctuations.

Use unchanged run_cell accounting: timer before conversion/serialization/tempfile,
launch, model preparation, search, output and parent parsing/raw-energy checking.
Shared immutable-input read/hash check and zero fallback setup are outside for
all arms. Zero vector/E=c is the fallback at t=0. No hidden conversion subtraction.
A witness is eligible only if **both** parent receipt and validation completion
are strictly before2s. Raw traces are retained for auditing; no published
250ms/500ms/1s energy comparisons from this version.

Kill process group at deadline, allow up to5s reaping; record actual timestamps.
No late witness gets credit, even if received before2s but validated later.
Per-cell kill-request overshoot (kill_time-2s) >100ms is an instrument failure,
not extra scoring allowance. This100ms resource-release bound is inherited from
the earlier maximum gate, not an asserted timing-resolution precision. No
median-null-difference threshold or injected50ms-resolution gate is applied.
These previously failed questions are outside this version's claimed scope.

Invalid state/energy/schema, source/affinity/thread mismatch, abnormal early exit,
failed setup or overflow => INVALID, retain and stop. Keep the existing1MiB line
and16MiB stream bounds. Missing completed solve or killed partial line at cutoff
is fallback-only/censored, not an invalid answer, provided readiness and other
contract checks pass. Report fallback counts prominently for every arm.

## Admission sufficient for this narrower scope

Existing exact-spectrum checks, frozen worker/source equality tests, and all32
successful old tiny smokes remain engineering evidence. They are not reused as
new observations or as evidence that the old preflight passed.

Before observations: commit this preregistration, then independently review and
commit the new coordinator/auditor/tests. Bind actual binaries and new source
hashes in a separate provenance record, with no mutation of old files.

Run exactly20 new admission cells, budget2s each, in this fixed order:

1. q05,q11,q17,q23, each of the four arms in manifest order, `null_a`, seed0:
   16native no-search readiness cells. Require one independently verified zero
   witness, ready/affinity record, and validation before2s in every cell.
2. Exposed tiny mixed fixture case03 (hash in manifest), seed501, each of the
   four arms in manifest order: four actual-search smokes. Require >=1 eligible
   native witness and correct ready/config metadata per arm.

All20must be valid and meet the100ms kill bound. Any failure ends this version;
no selected retry. Audit/review the complete raw artifacts before main. This is
minimal end-to-end liveness/correctness admission, **not a low-noise timing
calibration**, a statistically estimated reliability guarantee or larger-input
optimizer-initialization guarantee. Main still charges/records full initialization.

Fabricated regressions must cover2s exact boundary, delayed validation, absent/
late/partial output, invalid/duplicate cells, cap/abort retention, failed-admission
refusal, registration/source drift and all gate thresholds. Independently
recalculate final energies, primary counts and descriptive intervals. Reusing
immutable helpers is allowed; the auditor must not call runner scoring or trust
self-reported objectives. No real qualification optimizer runs in unit tests.

## Main, limits and reporting

One main run:24×10×4=960cells,2s each. Order instance q00..q23, seed61101..61110,
rotate the four-arm list by(instance_index+seed_index)%4 as before. Matching seed
labels pair blocks, not RNG streams. No adaptive order, restarts outside the fixed
wrapper, per-instance tuning or outcome-dependent exclusions. One-shot guards
and exclusive output directories, retained even on abort. Reproduction must be
labelled separately; no overwriting or rerunning this version to improve a verdict.

Nominal search/application time40s admission +1920s main=1960s. Total elapsed cap
3000s across both phase processes, including phase setup/output/analysis, excluding
builds/offline audits/reviews and idle gaps between phases. Check before each cell
with a5s cleanup allowance and after analysis; partial/invalid campaigns have
NO VERDICT. First invalid cell stops the phase and is retained. Any postobservation
code/design correction requires a new prospective amendment/version.

Report all ten per-instance energies, mean/median/std/min/max, fallback counts,
paired wins/ties/losses and normalized differences at2s only. H1 qualifying>=8
=> SUPPORTED_ON_QUALIFICATION, otherwise INCONCLUSIVE/NOT_QUALIFIED. H2 qualifying
>=2arms each>=4wins => SUPPORTED_ON_QUALIFICATION, otherwise NOT_ESTABLISHED.
Neither label asserts optimizer novelty, general competitive superiority or
statistical equivalence. Do not pool raw energies across differently scaled inputs.

For all six pairwise mean normalized differences, keep the exact predecessor's
descriptive95% instance bootstrap:10000 resamples, same SHA-256 namespace
`MQ-DIFFICULTY-001/bootstrap/v1`, counter0, four8-byte big-endian chunks/digest,
rejection below floor(2^64/24)*24, modulo24, shared draws across pairs, linearly
interpolated percentiles .025/.975. Keep this algorithm unchanged despite new ID;
bootstrap is not an independent replication or confirmatory test. One generator
family and24instances limit generalization; seeds are not the independent units.

Retain metadata.json, complete raw stdout/stderr/records, terminal/analysis/audit,
exact commands and reproduction README. Independent read-only reviews before
execution and interpretation; publish null/negative results and all protocol
changes. No automatic selection study, held-out wave, record hunt or production
routing follows a positive qualification. Update NOW/catalogue/roadmap/ledger.
