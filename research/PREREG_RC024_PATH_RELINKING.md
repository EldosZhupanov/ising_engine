# PREREG RC-024 — Does endpoint-guided path relinking buy material quality?

**Status:** binding. Written before candidate code and before any RC-024 datum.
Nothing below may change after the first run. A prospective correction requires
a separately committed amendment; the original remains immutable.

## §1 Question and prior evidence

RC-022 found synthesized moves — crossover, path relinking and recombination —
in 9 of 20 external MaxCut/QUBO heuristic families. RC-003 already tested one
form: collective subsets mined from population covariance. That mechanism beat
single flips by about 0.06%, a reproducible but immaterial effect. RC-024 does
not repeat it.

The untested form is **endpoint-guided path relinking**: use two complete
population members to synthesize an ordered sequence of moves through the
configurations between them, and retain the best intermediate state.

> At a fixed single-flip prefix, does one deterministic population path-
> relinking round produce a material paired quality improvement over the prefix
> alone?

This is a mechanism/quality test, not an efficiency test. RC-021 closed Class I
`INSTRUMENT-INVALID`; therefore no wall-time, throughput or equal-cost claim is
licensed.

## §2 Candidate and control — frozen mechanism

Both arms begin from the same initialization and run the existing
`metropolis_sweep` for 50 sweeps with the same seed, temperatures and 32
replicas. The candidate consumes the identical Metropolis RNG stream and then
performs exactly one deterministic relinking round:

1. Rank replicas by current canonical energy, breaking ties by replica index.
2. The lowest-energy replica is the target and is never modified.
3. The sources are the worst `max(1, R/4)` replicas, in worst-first order.
4. For each source, form the set of variables on which source and target differ.
5. At each path step, evaluate the current exact single-flip `delta_e` for every
   still-differing variable in that source. Flip the variable with the lowest
   `delta_e`; ties choose the lowest variable index. This strictly reduces the
   Hamming distance to the target by one.
6. Remember the lowest-energy prefix, endpoints included. After reaching the
   target, reverse the suffix after that prefix, leaving the source at the best
   point encountered. A source can therefore never finish worse than it began.

The candidate is named `path_relink_sweep`. A disabled constructor exists only
as a correctness control and must be bit-identical to `metropolis_sweep`; it is
not registered as a third experiment arm.

The implementation is isolated to `engine_v2`, uses `SpinState` only, and does
not call or modify the scalar-family `solver::elite::path_relink`. The scalar
implementation is precedent, not shared architecture.

## §3 Frozen corpus, seeds and logical budget

- **Instances:** all 30 files named `G*` in `benchmark_suite/data/gset/`, sorted
  lexicographically. `README.md` and `metadata.json` are not instances. No
  subset may be selected after results are seen.
- **Seeds:** exactly `101, 102, 103`.
- **Replicas:** 32.
- **Temperature ladder:** `temp_hi = 4.0`, `temp_lo = 0.1`, unchanged between
  arms.
- **Base budget:** 50 Metropolis sweeps in each arm.
- **Candidate addition:** one relinking round after the 50-sweep prefix. It is
  deliberately extra logical work. The experiment asks whether the mechanism
  has material quality value at all; it does not identify its marginal cost.
- **Harness:** new private research binary `exp_rc024_path_relink`, restricted
  to these two arms and frozen values. Its output is one TSV row per
  instance × seed plus a deterministic summary.

The run produces exactly 90 paired observations. No tuning run, warm-up result,
pilot subset or alternative source fraction is permitted.

## §4 Estimand and decision rule

For each instance/seed pair let `E_C` be the control's canonical best energy and
`E_P` the candidate's. The paired relative gain is

`g = (E_C - E_P) / abs(E_C)` when `abs(E_C) > 1e-12`, otherwise `g = 0`.

Positive is better for path relinking. The report must include all 90 raw
energies and gains, mean gain, median gain, strict win/loss/tie counts at
`1e-9`, and a two-sided Wilcoxon signed-rank test of the paired gains against
zero. Ties are handled by the repository's existing implementation.

The outcomes are frozen:

| condition | registered reading |
|---|---|
| any `E_P > E_C + 1e-9`, backend disagreement, nonzero ledger drift, non-finite score, missing row or duplicate pair | **INSTRUMENT INVALID**; publish no scientific result |
| `p < 0.05`, mean gain ≥ 1%, and median gain > 0 | **MATERIAL QUALITY SIGNAL**; path relinking earns a separately preregistered cost/replication study |
| `p < 0.05` but either materiality condition fails | **WORKS BUT IMMATERIAL**; the mechanism moves quality, but does not clear the project's 1% floor |
| `p ≥ 0.05` | **NO EVIDENCE OF VALUE** at this corpus and budget |

All outcomes are publishable. The run occurs once; no parameter may be changed
to rescue a null or adverse result.

## §5 Correctness obligations before the run

1. Disabled relinking is bit-identical to `metropolis_sweep`, including final
   state, canonical energies, accepted/proposed counts and RNG continuation.
2. The enabled operator is deterministic and bit-identical between
   `ReferenceState` and `SparseBitSlice` at identical seeds.
3. Every retained state has zero canonical ledger drift; a fresh IR rescore
   equals the reported energy.
4. Synthetic endpoints prove lowest-index tie-breaking, best-prefix retention,
   strict Hamming progress and the never-worse invariant.
5. The registry exposes the candidate through its capability passport without
   changing Runtime, public solver APIs or either backend.
6. `cargo check`, release tests, all binaries, clippy with warnings denied, fmt,
   golden regression, memory integrity and diff checks pass before the run.

## §6 Frozen artifacts and prohibited claims

The result lives under `results/rc024/`:

- `raw.tsv` — exactly 90 data rows plus header;
- `run.err` — stderr from the registered invocation, retained even if empty;
- `RESULT.md` — decision, complete summary, hashes and prohibited claims.

No claim may be made about wall time, equal cost, `UltimateSolver`, production
auto-routing, universal optimization, tuned path relinking, other corpora, or
model training/video generation. A material quality signal licenses only a new
preregistered cost/replication study; it does not license product integration.

## §7 Exact run

After the candidate commit and all §5 gates pass, the sole registered command
is:

```text
cargo run --release --bin exp_rc024_path_relink -- \
  --dir benchmark_suite/data/gset --sweeps 50 --replicas 32 \
  --seeds 101,102,103
```

Any argument different from these frozen values must be refused. Results are
published from that invocation whether the outcome is positive, null, adverse,
or instrument-invalid.
