# PREREG RC-027 — Does mechanism value depend on run length?

**Status:** binding. Written before the RC-027 harness and before any RC-027
datum. Nothing below may change after the first registered invocation. A
prospective correction requires a separately committed amendment; this file
remains immutable.

## §1 Question and prior evidence

RC-023 through RC-026 varied mechanism, corpus and the arrangement of a
mechanism inside a search. Every comparison used 50 local-search sweeps. The
architecture line is therefore closed at that budget, but it has not tested the
possibility that candidate and control curves separate only on longer runs.

> As the common local-search budget grows, do the candidate and control curves
> diverge, converge, or stay parallel?

The two comparisons are the unchanged pairs from RC-025:

| pair | control | candidate | prior 50-sweep reading |
|---|---|---|---|
| `synthesis` | `metropolis_sweep` | `path_relink_sweep` | +0.105 %, never worse in RC-024 |
| `memory_form` | `history_field` | `tabu_sweep` | no detectable difference in RC-023 |

No operator is retuned and no new mechanism is introduced. This is the last
budget-axis test, not a seventh mechanism cycle.

## §2 Feasibility probe and frozen design

Before this document was written, the top rung was timed once in an untracked,
throwaway clone on G11, seed 401. The four budgets and both pairs took about
6.8 seconds of computation; the projected registered run is about ten minutes.
Only elapsed time was used. The probe's scores are discarded, are not RC-027
data, are not analyzed, and do not enter any threshold or decision below.

The registered design is:

- **corpus:** exactly the 30 files named `G` followed only by decimal digits in
  `benchmark_suite/data/gset/`, lexicographically sorted;
- **budget ladder:** sweeps exactly **50, 200, 800, 3200**;
- **replicas:** 32;
- **temperature ladder:** `temp_hi = 4.0`, `temp_lo = 0.1`;
- **seeds:** exactly **401, 402, 403**, fresh for this measurement;
- **pairs:** the two rows in §1, with one schedule step per arm and the same
  sweep budget, replicas, temperatures and seed inside each pair;
- **harness:** the new private binary `exp_rc027_budget_ladder`, which refuses
  every command other than §7.

The run produces **30 × 4 × 3 × 2 = 720 paired observations**. There is no
pilot subset, alternative top rung, tuning run, early stopping or secondary
extension.

## §3 Estimand and primary tests

For each `(pair, instance, sweeps, seed)`, let `E_C` be the control's canonical
best energy and `E_P` the candidate's. The paired relative gain is

`g = (E_C − E_P) / abs(E_C)` when `abs(E_C) > 1e-12`, otherwise `g = 0`.

Positive favours the candidate. For each `(pair, instance, sweeps)`, average `g`
over the three seeds. This gives 120 points per pair.

**Primary test, separately for each pair:** Spearman rank correlation between
those 120 per-instance mean gains and `log(sweeps)`, two-sided at α = 0.05.
Ranks use average ranks for ties. Since logarithm is monotone, its base cannot
change the ranks. The reported p-value is the conventional large-sample
Student-t approximation

`t = rho * sqrt((n - 2) / (1 - rho^2))`, `df = n - 2 = 118`,

with a two-sided Student-t tail evaluated via the regularized incomplete beta;
`|rho| = 1` has p = 0. No unregistered subgroup or alternative trend test may
replace it.

The report also gives, per pair and budget, mean and median gain, and
win/loss/tie counts at `1e-9`, plus the top-rung mean against the 1 %
materiality floor. These are descriptive and do not repair a failed primary
test.

## §4 Mandatory 50-sweep controls

The first rung is the design every preceding architecture cycle used. The
ladder is interpretable only if it reproduces both prior readings on fresh
seeds:

1. `synthesis`: mean gain at 50 sweeps is strictly positive and strictly below
   1 %; this is RC-024's qualitative result.
2. `memory_form`: absolute mean gain at 50 sweeps is below 1 %, and a two-sided
   Wilcoxon signed-rank test over the 30 per-instance mean gains has `p >= 0.05`;
   this is RC-023's null. Zeros are dropped, absolute nonzero gains receive
   average ranks, and the p-value uses the normal approximation with tie
   correction and no continuity correction.

Failure of either control makes the whole run **INSTRUMENT INVALID**. It is
published, but no convergent/divergent/parallel reading is licensed.

## §5 Frozen decision table and correctness obligations

The global invalidity row is evaluated first. Otherwise each pair is classified
independently and both classifications are published.

| condition | registered reading |
|---|---|
| a mandatory control fails; a registered operator is absent; corpus membership/count or raw Rudy structure is wrong; frontend energy disagrees with the raw edge list; an arm is missing; a score or gain is non-finite; a best state does not rescore exactly; a row is missing or duplicated; or path relinking is worse than its paired Metropolis control | **INSTRUMENT INVALID**; publish the available evidence, no scientific ladder result |
| `rho > 0` and `p < 0.05` | **DIVERGE**; gain grows with budget and the 50-sweep closure was budget-dependent. The architecture line may reopen only under a new preregistration |
| `rho < 0` and `p < 0.05` | **CONVERGE**; the mechanism buys earlier convergence rather than a better long-run optimum |
| `p >= 0.05` or `rho = 0` | **PARALLEL**; budget does not explain the flat mechanism results and the architecture closure is final |

The following obligations must be proven before the run:

1. The loader inspects raw text before the frontend: header `n` and declared
   edge count parse, every nonempty body row has exactly three fields, endpoints
   are distinct and in `1..=n`, weights are finite, and physical row count equals
   the header. This is load-bearing because `parse_rudy` silently drops invalid
   edges.
2. On two independent assignments, the frontend energy equals the negative cut
   evaluated directly from the validated raw edges.
3. For every pair and rung, both arms lower to one step with exactly the frozen
   sweeps, replicas, temperatures and seed; no rung activates early stopping.
4. Every reported `best_state` is rescored against the fresh `ProblemIR` and is
   exactly equal to the reported score. The `synthesis` candidate is never
   greater than its paired control.
5. The complete key set contains every frozen tuple exactly once and therefore
   exactly 720 rows. All gains and energies are finite.
6. Each run-time guard above has a non-vacuous unit test. Before execution, its
   controlling branch is independently mutated in a scratch clone; exactly the
   named test must fail, the source is restored, and its SHA-256 must match.
7. Both development and release tests, all binaries, clippy, formatting, golden
   regression, memory validation and immutable hashes pass before the run.

## §6 Frozen artifacts and prohibited claims

The result lives under `results/rc027/`:

- `raw.tsv` — header plus exactly 720 registered rows when the instrument
  completes;
- `run.err` — stderr from the sole registered invocation, retained verbatim;
- `RESULT.md` — provenance, guard/mutation evidence, controls, both primary
  results, the frozen classifications, hashes and limitations.

No wall-time, throughput or equal-cost claim is licensed. RC-021 remains
`INSTRUMENT-INVALID`, and the candidate mechanisms do different logical work
from their controls. The feasibility probe establishes only practicality.

No claim about `UltimateSolver`, production routing, production integration,
tuned variants, corpora beyond these 30 G-Set instances, a universal optimizer,
or model training. A DIVERGE result licenses only a new preregistration; it does
not itself license production use. No high-diversity extension from RC-025 may
be run or used to repair this result.

## §7 Exact registered run

After the harness commit, mutations and every §5 gate pass, the sole registered
command is:

```text
cargo run --release --bin exp_rc027_budget_ladder -- \
  --dir benchmark_suite/data/gset --budgets 50,200,800,3200 \
  --replicas 32 --seeds 401,402,403
```

Any argument differing in value, order, spelling or count is refused. Results
from that invocation are published whether positive, null, adverse, partial or
instrument-invalid. It is never repeated to rescue an outcome.
