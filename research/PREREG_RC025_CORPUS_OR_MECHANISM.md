# PREREG RC-025 — Is it the mechanisms, or is it the corpus?

**Status:** binding. Written before candidate code and before any RC-025 datum.
Nothing below may change after the first run. A prospective correction requires
a separately committed amendment; the original remains immutable.

## §1 Question and prior evidence

Three preregistered cycles added mechanisms that the external corpus uses and we
lacked, and all three returned an effect an order of magnitude under the 1 %
materiality floor:

| cycle | mechanism | effect |
|---|---|---|
| RC-003 | collective moves mined from population covariance | ≈ **+0.06 %** |
| RC-023 | hard prohibition with aspiration, against soft decaying bias | **no detectable difference** |
| RC-024 | endpoint-guided path relinking | **+0.105 %**, 69 wins / 0 losses / 90 |

Every one of them was measured on G-Set. `memory/OPEN_PROBLEMS.md` §1 records,
from before those cycles, that G-Set is degenerate on two axes — and one of them
is not a power problem but an **identifiability** problem:

> Every G-Set instance is unweighted (J ∈ {−1,+1}). For J ∈ {±1}, with V violated
> and S satisfied edges, `E = V − S` and `V + S = |E|`, so `E = 2V − |E|` — an
> affine bijection. Ranking by violation count **is** ranking by energy.

So on G-Set, energy-guided and constraint-guided search are the same method
written two ways, and the landscape a memory or a recombination operator has to
exploit is the most degenerate one available. **Three flat results admit two
readings, and only one of them has ever been tested.**

> Do the mechanisms fail because they have nothing to offer, or because the
> corpus cannot express what they offer?

The Biq Mac collection answers this in a way G-Set cannot: it spans distinct edge
weights from **1 to 1104** inside one collection, at one size scale, in one file
format — and it contains G-Set's own degenerate class as its bottom rung, so the
comparison carries its own control.

## §2 Corpus — frozen

Only Biq Mac families with **no self-loops** are used. A self-loop is a linear
term, not an edge; `be*` and `gka*` carry them and are QUBO rather than MaxCut
instances. Mixing model classes would confound the axis under test, so they are
**excluded**, and this exclusion is fixed here, before any result is seen.

The primary ladder holds **density constant** and varies only weight diversity.
Three density-matched ladders, `n = 100` throughout, first **10** files of each
family in lexicographic order:

| ladder | rung 1 | rung 2 | rung 3 |
|---|---|---|---|
| **10 % density** | `pm1s` — 2 weights | `pw01` — 10 weights | `w01` — 21 weights |
| **50 % density** | `g05_100` — 1 weight | `pw05` — 10 weights | `w05` — 21 weights |
| **90 % density** | `pm1d` — 2 weights | `pw09` — 10 weights | `w09` — 21 weights |

**90 instances.** Rung 1 crosses no bijection: at 1–2 distinct weights `E = 2V −
|E|` holds and the guide axis is unidentifiable, exactly as on G-Set. Rungs 2
and 3 break it. The ladder therefore crosses a **qualitative** boundary, not
merely a quantitative one, which is why a 1 → 21 span is enough.

A **secondary, deliberately unmatched extension** reaches further up the weight
axis and is reported separately: `t2g` (9 files, 200 weights, 4 % density),
`t3g` (9, 373, 5 %), `ising2.5` and `ising3.0` (first 5 each, 814–1104 weights,
100 % density). **Weight diversity is confounded with density and topology
here**, so it may support a conclusion drawn from the primary ladder and may
never establish one on its own. This limitation is written down now, not after
the numbers arrive.

## §3 Mechanisms, budget and seeds — frozen

The operators are used **exactly as their own cycles registered them**. No
retuning, no new parameter, no code change to either mechanism. The RC-023 twin
pair and the RC-024 pair are re-run unchanged on the new corpus:

| pair | candidate | control | from |
|---|---|---|---|
| **memory form** | `tabu_sweep` | `history_field` | RC-023 |
| **synthesized moves** | `path_relink_sweep` | `metropolis_sweep` | RC-024 |

- **Sweeps:** 50. **Replicas:** 32. **Temperatures:** `temp_hi = 4.0`,
  `temp_lo = 0.1`. Identical between arms, identical to RC-023 and RC-024.
- **Seeds:** exactly `201, 202, 203`. Fresh, so this is a new measurement and not
  a re-analysis of RC-023's or RC-024's data.
- **Harness:** new private research binary `exp_rc025_corpus_ladder`, restricted
  to these frozen values, which refuses any other argument.

The run produces **90 instances × 3 seeds × 2 pairs = 540 paired observations**.
No tuning run, no pilot subset, no alternative budget.

## §4 Estimand and decision rule

For each (pair, instance, seed) let `E_C` be the control's canonical best energy
and `E_P` the candidate's. The paired relative gain is

`g = (E_C − E_P) / abs(E_C)` when `abs(E_C) > 1e-12`, otherwise `g = 0`.

Positive favours the candidate. Per instance, `g` is the mean over the three
seeds.

**Primary test, per pair:** Spearman rank correlation between per-instance `g`
and that instance's number of distinct edge weights, over the 90 primary-ladder
instances, two-sided. The hypothesis is that the mechanisms' value **grows with
weight diversity**; a rank correlation states exactly that and assumes nothing
about the shape.

**Mandatory control, per pair:** rung 1 (`pm1s`, `g05_100`, `pm1d` — 30
instances at 1–2 distinct weights) must reproduce what G-Set gave, i.e. mean
gain **< 1 %**. Rung 1 is G-Set's degenerate class inside this corpus. If rung 1
shows a material effect, then something other than weight diversity differs
between the corpora and **no reading of the ladder is licensed**.

The report must include all 540 raw energies and gains, per-rung mean/median
gain and win/loss/tie counts at 1e-9, the Spearman ρ and p per pair, and the
secondary extension reported apart from the primary ladder.

The outcomes are frozen:

| condition | registered reading |
|---|---|
| rung-1 mean gain ≥ 1 % for either pair, or a non-finite score, or a missing or duplicated row | **INSTRUMENT INVALID**; publish no scientific result |
| ρ > 0, p < 0.05, and rung-3 mean gain ≥ 1 % | **THE CORPUS WAS THE LIMIT**; the mechanism gap RC-022 identified is live again and G-Set is retired as the arbiter for architecture cycles |
| ρ > 0, p < 0.05, but rung-3 mean gain < 1 % | **WEIGHT DIVERSITY MATTERS, IMMATERIALLY**; the corpus is part of the story but does not rescue the mechanisms |
| p ≥ 0.05 | **NO EVIDENCE THE CORPUS EXPLAINS THE FLAT RESULTS**; the mechanism hypothesis is closed on the evidence available, and the next cycle must look somewhere neither the mechanism nor the corpus explains |

All four outcomes are publishable. The run occurs once. **No parameter, seed,
family, or rung may be changed to rescue a null or adverse result**, and no
instance may be added to or removed from the ladder after a number is seen.

## §5 Correctness obligations before the run

1. The loader is proven correct on this corpus: for every instance used, the
   `ProblemIR` reached through the frontend must agree with an independent
   evaluation of the same file, and every file must be confirmed to contain no
   self-loop. A silently mis-parsed weight would fabricate the very axis under
   test.
2. Both backends agree: candidate and control are bit-identical between
   `ReferenceState` and `SparseBitSlice` at identical seeds on a sample of the
   corpus.
3. Zero canonical ledger drift on every reported state; a fresh IR rescore equals
   the reported energy.
4. The declared weight diversity of every rung is **measured from the files**,
   not asserted from this document, and the harness refuses to run if a rung's
   measured diversity disagrees with the value registered in §2.
5. The harness refuses any argument that differs from §7's frozen command.
6. `cargo check`, release **and** dev tests, all binaries, clippy with warnings
   denied, fmt, golden regression, and the memory-doc gate pass before the run.

## §6 Frozen artifacts and prohibited claims

The result lives under `results/rc025/`:

- `raw.tsv` — exactly 540 data rows plus header;
- `run.err` — stderr of the registered invocation, retained even if empty;
- `RESULT.md` — decision, complete summary, hashes and prohibited claims.

**No wall-time claim of any kind.** RC-021 left this host `INSTRUMENT-INVALID`,
and `research/RC021_C10_DIAGNOSIS.md` measured why: this machine cannot resolve a
paired wall-time difference below about 5 %. RC-025 measures paired solution
quality at identical seeds and logical budget, which needs no qualified host.

No claim about `UltimateSolver`, production routing, tuned variants of either
mechanism, corpora outside Biq Mac and G-Set, or universal optimization. A
finding that the corpus was the limit licenses **one** thing: a new
preregistration of the architecture line on a non-degenerate corpus. It does not
retroactively make RC-003, RC-023 or RC-024 positive results — those measured
what they measured, on the corpus they named.

## §7 Exact run

After the harness commit and all §5 gates pass, the sole registered command is:

```text
cargo run --release --bin exp_rc025_corpus_ladder -- \
  --dir benchmark_suite/data/biqmac --sweeps 50 --replicas 32 \
  --seeds 201,202,203
```

Any argument different from these frozen values must be refused. Results are
published from that invocation whether the outcome is positive, null, adverse,
or instrument-invalid.
