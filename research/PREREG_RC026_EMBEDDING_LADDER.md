# PREREG RC-026 — Were we measuring an engine part outside the engine?

**Status:** binding. Written before candidate code and before any RC-026 datum.
Nothing below may change after the first run. A prospective correction requires a
separately committed amendment; the original remains immutable.

## §1 Question and prior evidence

Four cycles measured a mechanism and found it immaterial:

| cycle | corpus | mechanism | effect |
|---|---|---|---|
| RC-003 | G-Set | covariance-mined collective moves | ≈ +0.06 % |
| RC-023 | G-Set | hard prohibition vs soft decaying memory | none detectable |
| RC-024 | G-Set | endpoint-guided path relinking | +0.105 %, 69 W / 0 L |
| RC-025 | Biq Mac, weighted | both of the above | +0.18 % and −0.38 % |

RC-025 exonerated the corpus: breaking G-Set's `E = 2V − |E|` degeneracy changes
nothing. So the flat results are not explained by the instances.

**All four share one property that none of them tested.** Each scored the
mechanism as a *solo operator over the whole budget*. In the field, path
relinking does not run once at the end of a search — it lives inside GRASP and
scatter search, applied **repeatedly**, interleaved with the local search that
keeps feeding it fresh endpoints. RC-024 gave it one round after fifty sweeps.

> Does endpoint-guided path relinking pay when it is embedded in the search —
> applied many times at the same total local-search budget — rather than once at
> the end?

If the answer is yes, four cycles were measuring an engine part outside the
engine. If no, the solo-measurement hypothesis is closed with them.

The mechanism under test is **path relinking, not memory**. RC-025 showed the
untuned tabu form is mildly harmful; embedding a mechanism that costs something
is a weaker question than embedding the one that has never once lost a paired
trial in 180 of them. One question, cleanly.

## §2 Design — frozen

`path_relink_sweep` runs a Metropolis prefix and then exactly one relinking
round. A schedule that names it *k* times therefore performs **k relinking
rounds**, and the Runtime instantiates each operator once per plan
(`runtime.rs`, register-once/use-many), so the relinker's state persists across
all k blocks — the mechanism is embedded in one search, not restarted k times.

| arm | schedule | Metropolis sweeps | relink rounds |
|---|---|---:|---:|
| control | `[metropolis_sweep]`, sweeps `[50]` | 50 | 0 |
| candidate, k | `[path_relink_sweep] × k`, sweeps `[50/k] × k` | 50 | k |

**k ∈ {1, 2, 5, 10, 25}**, giving block sizes 50, 25, 10, 5, 2. Every value
divides 50 exactly, so the Metropolis budget is **identical at every rung** and
the only thing the ladder varies is how finely the mechanism is interleaved.

`k = 1` is RC-024's arrangement exactly, so the bottom rung is this experiment's
own control, on fresh seeds.

- **Corpus:** all 30 files named `G*` in `benchmark_suite/data/gset/`, the corpus
  RC-023 and RC-024 used. `README.md` and `metadata.json` are not instances.
- **Replicas:** 32. **Temperatures:** `temp_hi = 4.0`, `temp_lo = 0.1`.
- **Seeds:** exactly `301, 302, 303`.
- **Harness:** new private research binary `exp_rc026_embedding_ladder`,
  restricted to these frozen values, which refuses any other argument.

The run produces **30 × 5 × 3 = 450 paired observations**. No tuning run, no
pilot, no alternative budget or k.

## §3 Estimand and decision rule

For each (instance, k, seed) let `E_C` be the control's canonical best energy and
`E_P` the candidate's. The paired relative gain is

`g = (E_C − E_P) / abs(E_C)` when `abs(E_C) > 1e-12`, otherwise `g = 0`.

Per (instance, k), `g` is the mean over the three seeds — 150 points.

**Primary test:** Spearman rank correlation between `g` and `k` over those 150
points, two-sided. The hypothesis is that the mechanism's value **grows with how
finely it is embedded**; a rank correlation states that and assumes no shape.

**Mandatory control:** the `k = 1` rung must reproduce RC-024 qualitatively — mean
gain strictly positive and below 1 %. `k = 1` *is* RC-024's design, differing only
in seed. If it does not reproduce, something other than embedding differs between
the two runs and **no reading of the ladder is licensed**.

**Second mandatory control:** the control arm is the same schedule at the same
seed for every k, so for each (instance, seed) its energy must be **bit-identical
across all five rungs**. Any variation means state is leaking between arms and
the run is invalid.

The report must include all 450 raw energies and gains, per-k mean/median gain
and win/loss/tie counts at 1e-9, and the Spearman ρ and p.

The outcomes are frozen:

| condition | registered reading |
|---|---|
| `k = 1` mean gain ≤ 0 or ≥ 1 %; a control energy differing across k; a non-finite score; a missing or duplicated row | **INSTRUMENT INVALID**; publish no scientific result |
| ρ > 0, p < 0.05, and `k = 25` mean gain ≥ 1 % | **EMBEDDING WAS THE LIMIT**; four cycles measured the mechanism outside the search it belongs in, and the architecture line reopens under a new preregistration |
| ρ > 0, p < 0.05, but `k = 25` mean gain < 1 % | **EMBEDDING HELPS, IMMATERIALLY**; the shape of the comparison matters and still does not reach the floor |
| p ≥ 0.05 | **NO EVIDENCE EMBEDDING EXPLAINS THE FLAT RESULTS**; the solo-measurement hypothesis is closed with the mechanism and the corpus, and the next question must be none of the three |

All four outcomes are publishable. The run occurs once. No parameter, seed, k, or
instance may be changed to rescue a null or adverse result.

## §4 Correctness obligations before the run

1. A schedule naming one operator k times produces k steps that share **one**
   operator instance, so the mechanism's memory persists across blocks. Proven by
   a test that fails if the instances are separate.
2. For every registered k, the candidate's schedule delivers exactly 50 Metropolis
   sweeps in total and exactly k relinking rounds. Proven by a test.
3. The control schedule is byte-identical at every k, and the harness refuses at
   run time if a control energy differs across rungs for the same instance and
   seed.
4. Zero canonical ledger drift on every reported state.
5. The harness refuses any argument that differs from §6's frozen command.
6. `cargo check`, release **and** dev tests, all binaries, clippy with warnings
   denied, fmt, golden regression, and the memory-doc gate pass before the run.

## §5 Frozen artifacts and prohibited claims

The result lives under `results/rc026/`: `raw.tsv` (450 rows plus header),
`run.err`, and `RESULT.md`.

**The candidate at `k = 25` performs twenty-five relinking rounds against the
control's zero.** That is deliberately more logical work, and this experiment
does not price it. **No wall-time, throughput or equal-cost claim of any kind**:
RC-021 left this host `INSTRUMENT-INVALID` and `research/RC021_C10_DIAGNOSIS.md`
measured that it cannot resolve a paired wall-time difference below about 5 %.

No claim about `UltimateSolver`, production routing, tuned path relinking, other
corpora, memory-form operators, or universal optimization. A finding that
embedding was the limit licenses **one** thing: a new preregistration of the
architecture line with embedded comparisons. It does not retroactively make
RC-003, RC-023, RC-024 or RC-025 positive results — those measured what they
measured, in the arrangement they named.

## §6 Exact run

After the harness commit and all §4 gates pass, the sole registered command is:

```text
cargo run --release --bin exp_rc026_embedding_ladder -- \
  --dir benchmark_suite/data/gset --sweeps 50 --replicas 32 \
  --seeds 301,302,303
```

Any argument different from these frozen values must be refused. Results are
published from that invocation whether the outcome is positive, null, adverse, or
instrument-invalid.
