# HUBO-Q002 — endpoint-variation qualification and certified local penalties

2026-09-27. Binding on commit before reduction implementation or solver outcomes.
This is a new diagnostic protocol; HUBO-C001 remains closed and unchanged.

## Question, hypothesis and scope

HYPOTHESIS: at least three of four fresh size/degree strata exhibit measurable
stochastic endpoint variation under the two retained native solvers at two seconds.
This is an operational measurement gate, not a test of solver superiority,
statistical power, intrinsic hardness or global optimality. C001 had constant,
identical native energies in all ten seed repetitions of every n=32 instance.

Phase A prepares inputs and a proved, testable local-penalty reduction for future
quadratic controls. Phase B evaluates only the two unchanged native methods.
No quadratic performance campaign, production integration, Laya training or
record hunt is part of Q002. Finishing Phase A does not mean Phase B succeeded.

## Source/dependency map and do-not-touch scope

Reuse the byte-pinned C001 input generator (SHA in inputs.json), MSC research
driver `research/examples/hubo_compare.rs` and unchanged production
LogicBuilder -> FlatHuboModel -> QuantumField -> engine::step. Reuse mathematical
worker settings, but create a new task-local controller/analysis; do not modify
frozen C001 source/raw. New pure-Python reduction and its tests stay in this
directory. No dependency install or public API/solver-family change.

## Inputs and prior access

`cases.json` SHA256 **79ce0d4291902cfa0169d1f89d2e373b9a8684d35420b2b89b24dc1277417803**.
`inputs.json` lists every ID, original size, degree, instance seed, canonical
per-case hash and normalizer. Preparation invokes C001's fixed SplitMix64
construction at larger n; no optimum or solve outcome has been read.

| Stratum | n | Degree | Instance seeds (inclusive) |
|---|---:|---:|---|
| n64_d3 | 64 | 3 | 961001–961003 |
| n64_d4 | 64 | 4 | 962001–962003 |
| n128_d3 | 128 | 3 | 963001–963003 |
| n128_d4 | 128 | 4 | 964001–964003 |

Each instance has 3n distinct signed interactions and n signed unit fields;
expand s=2x−1 exactly. Normalizer is 4n. Fresh random inputs use the same synthetic
family as C001, so they are not an external application corpus. Retain all twelve
cases and four strata regardless of outcome; no filtering by winner/direction.
All seeds exposed in Q002 are diagnostic data, never a later confirmatory holdout.

## Phase A: local penalties and invariant

Input is a canonical map from sorted, distinct nonnegative variable IDs to exact
integer coefficients, merging duplicates and deleting zero coefficients. Include
the empty monomial for constants. Reject invalid types/indices/degrees >4.

At each step choose the most frequent variable pair among current degree>2
monomials, breaking ties lexicographically. Replace this pair only in those
higher-order terms; leave existing degree<=2 terms, including earlier penalties,
unchanged. Create a fresh increasing auxiliary ID z, merge/drop-zero terms.
For the affected part uv*g, let P=sum positive coefficients and N=sum absolute
negative coefficients. Add the Rosenberg penalty

    M * (u*v - 2*u*z - 2*v*z + 3*z).

Local M=1+max(P,N). Global control uses the constant
M_global=1+sum absolute original degree>2 coefficients. Both use identical pair
plans; coefficient differences occur only in their quadratic penalties. Record
each pair, new ID, affected coefficients and M for independent certificate checks.

Proof obligation: g is independent of u,v and −N<=g<=P on Boolean assignments.
If uv=1, invalid z=0 has gap M−g>=1 relative to z=1. If uv=0, invalid z=1 has gap
g+M*(3−2u−2v)>=1. Thus min_z F_new=F_old for every assignment of all old variables,
including prior auxiliaries. Composing these minima preserves the full objective;
the auxiliaries form an acyclic construction, not an assumed orthogonal basis.
Current higher-order coefficient L1 cannot increase under substitution/merging,
so M_local<=M_global and the global control is sufficient too.

Correctness gate: exhaustive original/auxiliary minimization on small signed,
shared-pair, nested-auxiliary, mixed/constant and already-quadratic fixtures;
also verify each substitution's conditional minimum across all current old
variables. Local/global pair plans must agree, M_local<=M_global at every step,
consistent auxiliary extension must recover original energy, and an insufficient
penalty mutation must be rejected. Deterministic generator replay and malformed
input rejection are required. These are correctness checks, not performance data.
No speed claim from reduced M. Large integer coefficients must not be silently
rounded during any eventual f64 conversion; reject nonexact representation.

## Phase B: arms, budgets, environment and order

Two arms A=msc_native, B=oj_native; no algorithm or parameter tuning. Settings
are C001 source `23d300f`: MSC one slice/population, eight geometric temperatures,
64 lanes/temperature, one engine step before scoring projections; OpenJij0.12.0
SASampler.sample_hubo BINARY, one read,1000 sweeps,one thread,METROPOLIS,XORSHIFT,
GEOMETRIC. For both T_hot=max(1,max incident absolute coefficient sum)/ln2 and
T_cold=0.1. OpenJij restart seed=solver_seed+10*restart_index, index<1,000,000;
no seedless fallback. Common Python worker imports dimod0.12.22/OpenJij/NumPy2.5.1
before READY. Record exact hashes/versions at source freeze.

Solver seeds 970001–970010. Twelve instances × ten seeds × two arms =240 cells;
480 seconds total credited budgets. No adaptive budget extension. Two-second
external deadline includes parse/init/model preparation/search/scoring/IPC;
pre-READY import measured separately. All-zero initial witness is valid fallback.
Only complete original-objective witnesses received by2.0s are credited. Kill
whole process group at deadline; terminate within0.2s tolerance. Record all late
complete events and raw partial tail; independently check every complete witness.
READY limit30s. Missing/invalid witness/metadata, early exit, leaked process,
malformed complete event or changed source invalidates the whole campaign;
preserve failing row and stop, no replacements/reruns.

Instances in inputs.json order, seeds ascending; AB when (instance_index+seed_index)
is even, BA otherwise. Each instance therefore has five AB and five BA blocks.
One worker; RAYON/OMP/OPENBLAS/MKL threads=1, PYTHONHASHSEED=0, no affinity pinning,
no concurrent research campaign. Expected host is the C001 AMD Ryzen7 170/WSL2
host; record actual OS/CPU/RAM/rustc/config/RUSTFLAGS/load and binaries before any
cell. Unexpected host/version change requires prospective amendment, not silent
substitution. Warm-time exploratory quality only; no cold latency claim.

## Primary decision and secondary metrics

For each instance, using ten final energies per arm, define spread=max−min.
An instance is VARIATION_PRESENT iff (spread_A>=2 OR spread_B>=2) AND at least
two of ten paired seed endpoints differ. The either-arm rule is symmetric.
A stratum passes iff at least two of its three instances meet that condition.
Overall: at least3/4 passing strata -> VARIATION_PRESENT;1/4 or2/4 -> MIXED;
0/4 -> NO_QUALIFIED_VARIATION. Integrity failure overrides all with
INSTRUMENT_INVALID. Report per-instance and per-stratum counts, not only aggregate.

Important counterexample: equal outcome multisets with permuted paired seeds can
pass this gate. Therefore passing is NOT evidence of different solver performance
distributions or statistical power. Stable, different arm endpoints can fail
the variation gate; report these as a descriptive stable-separation category,
not an exception to the criterion. Passing permits proposing a new independent
measurement, not selecting a winner. No p-value or superiority conclusion from
this qualification. All later claims need a new prospective design/holdout.

Secondary: all energies/witnesses/receipts, per-instance arm mean/median/sampleSD,
min/max/Q25/Q75/IQR; paired wins/ties/losses, normalized differences, number of
distinct endpoints, improved-over-zero fraction, preparation/startup/deadline
distributions, emitted incumbent counts. No certified target exists: optimum hit
rate/TTS99 unavailable. Constant endpoints are not optimality certificates.

## Quadratic control and schedule follow-up requirements

Phase A's local/global reduction uses one deterministic pair plan, enabling a
future within-kernel penalty ablation. Keep the existing dimod global control
as a separate reference; do not call the new construction equivalent to dimod's
tie-breaking or a strongest-known reduction. Penalty correctness does not establish
search quality. In a future comparison record actual coefficient ranges and
temperature endpoints and cross local/global penalties with representation-derived
versus common-endpoint schedules. That factorial comparison needs its own frozen
protocol; no post-hoc schedule choice or C001 replacement is authorized here.

## Before execution and publication

Freeze the new instrument/analysis after independent review and deterministic
tests. Controller must retain source hashes and portable raw-artifact verification
separately from optional current-runtime matching; retain explicit sequence IDs,
start/finish timestamps and process-stop evidence rather than relying on mtimes.
Smoke uses only a fixed explicit n=6 toy and seeds980001/980002. No main generator
at invalid small sizes. Source freeze precedes smoke and main outcomes; preserve
all raw/errors, independent analysis and limitation disclosures. Phase A result
must explicitly say whether Phase B has run. No automatic new campaign after Q002.

## Verified primary references and external-corpus triage

- [dimod make_quadratic documentation](https://docs.dwavequantum.com/en/latest/ocean/api_ref_dimod/generated/dimod.make_quadratic.html):
  strength controls substitution penalties; insufficient strength can change minima.
- [D-Wave reformulation documentation](https://docs.dwavequantum.com/en/latest/quantum_research/reformulating.html):
  established higher-order reduction techniques. No novelty claim for substitution.
- [QOBLIB at checked commit16a166ee](https://github.com/ZIB-AOPT/QOBLIB/tree/16a166ee67c24c112551c803aab5394743b815b5/10-topology):
  class10 is Topology Design/Graph Golf, not a generic fourth-order HUBO corpus.
  Its README defines a degree-constrained minimum-diameter graph problem; using it
  here would first require a separately verified formulation. The old class label
  in conversation is not a benchmark specification. This task does not claim all
  QOBLIB formats or every external HUBO corpus has been exhausted.

References checked2026-09-27. These are official code/documentation sources,
not a claim of a newly verified peer-reviewed theorem or algorithmic novelty.
