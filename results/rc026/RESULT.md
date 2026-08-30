# RC-026 — Were we measuring an engine part outside the engine?

**Registered decision: EMBEDDING HELPS, IMMATERIALLY.**

Partly, yes. The shape of the comparison was hiding something real: applying path
relinking repeatedly inside the search, at an unchanged local-search budget, is
reliably better than applying it once at the end. This is the **first significant
trend in five architecture cycles**. It is also strongly sublinear, and it does
not come close to the 1 % floor.

## Provenance

| item | value |
|---|---|
| preregistration | `research/PREREG_RC026_EMBEDDING_LADDER.md`, commit `4725682`, frozen before code |
| preregistration SHA-256 | `974526e14f1f6b0da7af…` |
| harness commit | `dea8117` |
| registered command | `cargo run --release --bin exp_rc026_embedding_ladder -- --dir benchmark_suite/data/gset --sweeps 50 --replicas 32 --seeds 301,302,303` |
| `raw.tsv` SHA-256 | `ac91d6bf8d3de316df48f83e716e13b6983d05ae09f9daf802337533696726f3` |
| `run.err` SHA-256 | `35330f3931bcd6c78bbc96953ae96be3758f5a11b0c537a4878649d22cf98a3d` |
| rows | 450, exactly as §2 froze; 30 instances × 5 rungs × 3 seeds |

## Both mandatory controls, first

**§3 control one — `k = 1` must reproduce RC-024.** `k = 1` *is* RC-024's
arrangement, differing only in seed.

| | mean gain |
|---|---:|
| RC-024, seeds 101/102/103 | +0.105 % |
| RC-026 `k = 1`, seeds 301/302/303 | **+0.0997 %** |

Positive and below 1 %, within half a hundredth of a percent of the earlier
figure on independent seeds. The ladder is readable.

**§3 control two — the control arm must not move across rungs.** The control is
the same schedule at the same seed for every `k`, so its energy must be identical
at all five. The harness compares **bits**, not values, and aborts on any
difference. It ran to completion, so no control energy moved in 450 comparisons:
nothing leaked between arms.

## Result

Per-instance gain is the mean over seeds 301/302/303. Every rung spends exactly
50 Metropolis sweeps; only the interleaving changes.

| k | sweeps per block | mean gain | median | wins | losses | ties |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 50 | +0.0997 % | +0.1054 % | 68 | **0** | 22 |
| 2 | 25 | +0.1041 % | +0.1012 % | 70 | **0** | 20 |
| 5 | 10 | +0.1117 % | +0.1141 % | 70 | **0** | 20 |
| 10 | 5 | +0.1412 % | +0.1493 % | 71 | **0** | 19 |
| 25 | 2 | **+0.1766 %** | +0.1661 % | 76 | **0** | 14 |

**Primary test — Spearman(gain, k) over 150 points: ρ = +0.312,
p = 1.03 × 10⁻⁴.** A trend, and in the registered direction.

Supporting figures, all registered as part of §3's required report:

- the trend is **broad, not carried by outliers**: the per-instance Spearman is
  positive on **26 of 30** instances and negative on 1;
- paired across instances, `k = 25` beats `k = 1` on **25 of 30**, Wilcoxon
  p = 1.1 × 10⁻⁵;
- across all 450 observations: **355 wins, 0 losses, 95 ties**.

Under the frozen decision table, ρ > 0 and p < 0.05 hold, but the `k = 25` mean
gain is 0.177 %, far below 1 %. That is the **EMBEDDING HELPS, IMMATERIALLY**
row, written down before the run.

## Why this does not reopen the line

The return on embedding is **strongly sublinear**. Fitting the five rung means:

> gain ∝ k^0.18

Twenty-five times as many relinking rounds buys **1.77×** the gain. Extrapolating
that exponent — and this is an extrapolation, not a measurement — reaching a
1 % mean gain would take on the order of **3.6 × 10⁵ rounds**.

That extrapolation is not merely large, it is pointed the wrong way, because the
cost per round is not constant. `relink_source` walks a path of length equal to
the Hamming distance `d` between two replicas, and re-evaluates every
still-differing site at every step: **Σᵢ₌₁..d i = d²/2 delta-energy evaluations
per source per round.** Embedding multiplies an already quadratic cost by `k`.
This is a property of the code, not a timing measurement — no wall time is
claimed anywhere in this record.

So the mechanism does respond to being embedded, exactly as the field's use of it
suggested. It responds at an exponent of 0.18 against a per-round cost quadratic
in Hamming distance. **The direction that helps is the direction that cannot be
afforded**, and that is a cleaner reason to stop than five nulls would have been.

## The invariant, after 630 trials

| cycle | corpus | wins | **losses** | trials |
|---|---|---:|---:|---:|
| RC-024 | G-Set | 69 | **0** | 90 |
| RC-025 | Biq Mac, weighted | 54 | **0** | 90 |
| RC-026 | G-Set, five embedding depths | 355 | **0** | 450 |
| **total** | | **478** | **0** | **630** |

Across two corpora, three preregistrations, nine seeds and five embedding
depths, `path_relink_sweep` has **never once finished worse than its control**.
That is the never-worse invariant, proven on synthetic endpoints before any of
these runs and enforced by rewinding each source to its best prefix. It is worth
recording plainly: the mechanism is completely reliable and reliably worth about
a tenth of a percent.

## What this closes

Five cycles have now asked, in order, whether the flat results were the
mechanism, the corpus, or the shape of the comparison:

| cycle | question | answer |
|---|---|---|
| RC-003, RC-023, RC-024 | is the mechanism inventory the gap? | effects an order of magnitude under the floor |
| RC-025 | is G-Set's degeneracy the reason? | no — the ladder of weight diversity is flat |
| **RC-026** | were we measuring it outside the search it belongs in? | **partly yes, and it changes nothing that matters** |

The third explanation is the only one of the three that turned out to carry a
real signal, and it is still immaterial. The architecture line has been asked its
three natural questions and has answered all three. **It does not need a sixth
cycle.**

## Prohibited claims (PREREG §5, honoured)

The candidate at `k = 25` performs twenty-five relinking rounds against the
control's zero. That is deliberately more logical work and **this experiment does
not price it**. RC-021 left this host `INSTRUMENT-INVALID`, and
`research/RC021_C10_DIAGNOSIS.md` measured that it cannot resolve a paired
wall-time difference below about 5 %. **No wall-time, throughput or equal-cost
claim is made or licensed here**; the quadratic-cost statement above is read off
the algorithm, not off a clock.

No claim about `UltimateSolver`, production routing, tuned path relinking, corpora
beyond G-Set, memory-form operators, or universal optimization. The `k^0.18` fit
is descriptive of five points and is not offered as a law. This result does not
retroactively make RC-003, RC-023, RC-024 or RC-025 positive — those measured
what they measured, in the arrangement they named.

## Artifacts

- `raw.tsv` — header plus exactly 450 rows, energies at full `f64` precision
- `run.err` — stderr of the registered invocation, retained verbatim
- this file
