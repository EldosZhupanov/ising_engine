# MQ-SCREEN-001 — fixed solver comparison

Date 2026-09-27. Status: binding prospective preregistration and narrow amendment
to [external comparison protocol](../../EXTERNAL_COMPARISON_PROTOCOL.md) and
[amendment 1](../../EXTERNAL_COMPARISON_AMENDMENT_1.md).

## Authorization and scope

The user explicitly requested immediate comparison after MQ-QUAL-001: «да и
сразу сравни». For THIS descriptive fixed-solver screen only, supersede the
parent's S3/X3 prerequisites, mandatory selector/random/greedy arms, BiqMac corpus,
held-in/held-out routing and TTS/best-known-optimum endpoints. Those requirements
remain unchanged for subsequent selector/advantage campaigns. No claim of S3,
learned selection, new algorithm, record, or general competitive superiority.
The historical protocol is preserved; no previously burned data are reused.

Question: at the same delivered-solution wall budget on these fresh synthetic
weighted QUBOs, how do restarted UltimateSolver, a fixed engine_v2 default plan
and MQLib MERZ2002ONEOPT compare? This tests configured wrappers, not the best
possible settings or the complete MQLib portfolio/hyperheuristic.

## Frozen corpus and arms

[manifest.json](manifest.json) fixes eight JSON files and SHA-256, two instances
per cell of n in {64,128} × edge probability {.1,.7}. Python Random generator
seeds41001..41008, independent integer fields [-3,3], nonzero pair weights
uniformly chosen from {-5..-1,1..5}, offsets -4..3. The actual JSON bytes, not
the recipe, are authoritative. No optimization outcomes were inspected before
this protocol. No real corpus, previously opened Gset or reserved holdout is used.

All arms minimize the same raw binary polynomial. Initial available incumbent is
the all-zero state of energy c for all arms, explicitly tagged as fallback.

1. `ultimate`: unmodified UltimateSolver, repeated independent chunks. Each uses
   `new(hot,0.1,8,8,seed).with_2opt(true)`, default64 lanes,10temperatures,
   one slice/population, no ICM/path-relinking. hot=max(1,max_i(|h_i|+Σ_j|J_ij|)).
   Publish strictly improving completed solves; no access to mid-solve witnesses.
2. `v2_default`: unmodified DecisionEngine::default_plan(ir,standard_registry,
   32,32,seed), built-in ladder4.0..0.08, all-zero initialization, no learned KB,
   adaptation/controller or training. Repeat independent chunks, publish best
   completed RunRecord. Record actual chosen operators/backend once per process.
3. `mqlib`: unchanged upstream585496274af5abb0849d0d47e135496b4688680b,
   native MERZ2002ONEOPT. A small callback bridge emits improving states/objectives
   to stdout. Native algorithm is unchanged. Callback mode is separately checked
   on disjoint tiny smoke fixtures before the comparison; no qualification claim
   about other heuristics. Independent objective checks apply to every witness.

For the two Rust wrappers, chunk seed = base_seed*0x9e3779b97f4a7c15+chunk_index
with u64 wrapping arithmetic. MQLib uses the base seed directly via srand.
Matching base seed pairs runs; it does not imply matching random streams.

## Budget, ordering and validity

Seeds51001..51010; 8×10×3=240 cells, sequential (no concurrent optimization).
Each cell: 2.0 seconds measured by the parent monotonic clock BEFORE adapter
conversion/input serialization/process launch. Include process startup, model
construction, search, restart and emission overhead. Common disk read of the
raw immutable fixture and postrun independent audit are outside this clock.
Each child uses taskset CPU0 and RAYON_NUM_THREADS=1, OMP_NUM_THREADS=1,
OPENBLAS_NUM_THREADS=1. All cells use the same host. No resource advantage claim.
Host: WSL2 Linux6.6.114.1, AMD Ryzen7 170; full runtime environment/compiler/flags
and executable/source hashes retained. Rust release, existing default features;
upstream g++ -O2. No build time is charged as search time.

Parent timestamps complete received JSONL lines; only lines received strictly
before deadline can improve the primary incumbent. Late/partial output is
retained and labelled but never accepted. Parent terminates the child at deadline
and waits for process exit; teardown is recorded separately, not search budget.
Delivery time rather than inner algorithm time defines the estimand. Early
successful finish is allowed but not filled with extra budget. Unexpected exit,
invalid state/energy, setup failure or incomplete case is an instrument failure:
preserve artifacts and stop, do not silently count as a loss or select a retry.
No completed native solve before cutoff is disclosed as fallback-only, not a
provenance failure or hidden timeout. Every accepted witness is independently
rescored from integer raw coefficients. Report completion/fallback counts.

Order: instance index then seed index; rotate the three arms by
(instance_index+seed_index)%3. This balances within-block ordering; host load is
recorded. No outcome-based scheduling, tuning, filtering or post-hoc subgroup.

## Outcomes and analysis

Primary contrast per instance: mean over ten seeds of
D=(E_mqlib-E_ultimate)/L, L=Σ|h_i|+Σ|J_ij| frozen in manifest (offset excluded).
Positive favors Ultimate. H1: mean of eight instance contrasts >=0.001 AND
two-sided exact paired sign-flip test p<0.05 AND 95% instance bootstrap percentile
interval lower endpoint>0. Sign-flip enumerates all2^8 sign patterns of means;
The sign-flip null assumes symmetric/exchangeable paired instance differences;
it is not a population-generalization guarantee. Bootstrap10000 draws with Python Random seed52001. Eight instances, not80seeds,
are the units for inference. This narrow synthetic-screen criterion does not
establish generalization, novel selection, or a strong-specialist/SOTA win.
Failure to meet it is INCONCLUSIVE for H1, not equivalence. If any primary
cell is invalid, H1 has NO VERDICT.

Secondary/descriptive only: v2 vs both arms; per-instance/overall wins/ties/losses,
mean, median, population std, min/max, quartiles of final energies and paired
normalized differences; per-instance incumbent traces and time-to-best-delivered.
No known optima: no optimum success-rate or TTS99; report pairwise win fractions
and fallback frequency with correct labels. No pooling raw energies across
instances to manufacture an overall energy advantage. Optimistic post-hoc oracle
choosing the lowest per-instance mean and best fixed arm are diagnostic only,
selected on these same data, never reported as deployable selector performance.

## Execution gates and stop conditions

Before main: independently review and commit protocol, fixtures and instrument;
exhaustive tiny QuboModel/IR/parser spectrum tests, wrapper callback smokes and
deadline/late-output/corruption tests must pass. Smoke uses only the first four
MQ-QUAL-001 tiny fixtures, seeds501,502, all three arms,0.25s cap (24 cells),
outside the screen data. Preserve smoke artifacts and every failure. Independent
postrun audit of all witnesses, hashes, budget eligibility and recomputed summary.
Store metadata.json, raw.jsonl, summary.json and reproduction README. No overwrite.
Protocol changes after data require a new prospective amendment/version; no rescue
rerun. Commit and document negative results with the same care as positive ones.
