# RC-025 — Is it the mechanisms, or is it the corpus?

**Registered decision: NO EVIDENCE THE CORPUS EXPLAINS THE FLAT RESULTS.**

Three cycles found their mechanism immaterial on G-Set. This asked whether G-Set
was the reason. It was not. The mechanism hypothesis is closed on the evidence
available, and the next cycle must look somewhere that neither the mechanism nor
the corpus explains.

## Provenance

| item | value |
|---|---|
| preregistration | `research/PREREG_RC025_CORPUS_OR_MECHANISM.md`, commit `339e74f`, frozen before code |
| preregistration SHA-256 | `292a33a750ec273855bf…` |
| amendment | `research/PREREG_RC025_AMENDMENT_1.md`, commit `c3b4835`, committed **before any datum** |
| amendment SHA-256 | `1aa5aff00c3e1cbb36a1…` |
| harness commit | `c3b48355c6cd1636c2737480ddcf13e35aaf41da` |
| registered command | `cargo run --release --bin exp_rc025_corpus_ladder -- --dir benchmark_suite/data/biqmac --sweeps 50 --replicas 32 --seeds 201,202,203` |
| `raw.tsv` SHA-256 | `f40bdba33086587708da1e6536507a15fc40f5747d630c90f38c9fabecabf278` |
| `run.err` SHA-256 | `d9803625573c741917268a7cdf61867c5f7eb12622a19e4be899563c1afdf2b2` |
| rows | 540, exactly as §3 froze; 90 instances, 3 seeds, 2 pairs |

The first invocation of the registered command **refused before measuring
anything** — `pm1s yielded 0 files, not the 10 PREREG §2 froze`. The family names
in §2 lacked their size suffix. `raw.tsv` was empty, no experiment ran, and the
correction was committed as a separate amendment before the run. See §"What the
refusal was worth" below.

## The mandatory control, first

§4 requires rung 1 to reproduce what G-Set gave before any reading of the ladder
is licensed. Rung 1 is G-Set's degenerate class inside this corpus: 1–2 distinct
weights, where `E = 2V − |E|` makes energy-guided and constraint-guided search
the same method.

| pair | rung-1 mean gain | verdict |
|---|---:|---|
| memory form | **−0.68 %** | below 1 %: immaterial, as on G-Set |
| synthesis | **+0.39 %** | below 1 %: immaterial, as on G-Set |

Both clear the control. The ladder is readable.

## Result

Per-instance gain is the mean over seeds 201/202/203. Positive favours the
candidate.

### memory form — `tabu_sweep` against `history_field`

| rung | distinct weights | mean | median | pos / neg |
|---|---:|---:|---:|---:|
| 1 | 1–2 | −0.675 % | −0.117 % | 5 / 20 |
| 2 | 10 | −0.189 % | −0.101 % | 4 / 26 |
| 3 | 21 | −0.271 % | −0.129 % | 10 / 16 |

**Overall: 19 wins, 62 losses, 9 ties**; mean −0.379 %; sign test p = 1.8 × 10⁻⁶.

**Primary test — Spearman(gain, distinct weights) over 90 instances:
ρ = +0.074, p = 0.489.** No trend.

### synthesis — `path_relink_sweep` against `metropolis_sweep`

| rung | distinct weights | mean | median | pos / neg |
|---|---:|---:|---:|---:|
| 1 | 1–2 | +0.393 % | +0.182 % | 20 / 0 |
| 2 | 10 | +0.028 % | +0.004 % | 16 / 0 |
| 3 | 21 | +0.124 % | +0.023 % | 18 / 0 |

**Overall: 54 wins, 0 losses, 36 ties**; mean +0.181 %; sign test p = 1.1 × 10⁻¹⁶.

**Primary test: ρ = −0.098, p = 0.360.** No trend, and the point estimate points
the wrong way — the effect is *largest* on the degenerate rung.

Under the frozen decision table, both pairs land on `p ≥ 0.05`: **NO EVIDENCE THE
CORPUS EXPLAINS THE FLAT RESULTS.**

## What this changes

**The corpus is exonerated, and that is the point of having run it.**
`OPEN_PROBLEMS.md` §1 established by proof — not by sample size — that G-Set
cannot separate energy-guided from constraint-guided search. That remains true
and remains a real limitation of G-Set. It is simply **not the reason** these two
mechanisms are immaterial: breaking the bijection by moving from 1 to 21 distinct
weights, at three densities, does not make either mechanism matter more.

Four cycles now agree:

| cycle | corpus | mechanism | effect |
|---|---|---|---|
| RC-003 | G-Set | covariance-mined collective moves | ≈ +0.06 % |
| RC-023 | G-Set | hard prohibition vs soft decaying memory | none detectable |
| RC-024 | G-Set | endpoint-guided path relinking | +0.105 % |
| **RC-025** | **Biq Mac, weighted** | **both of the above** | **+0.18 % and −0.38 %** |

RC-022's census identified a real gap between our operator corpus and the human
one. That gap is now measured on two corpora and it does not pay. The honest
reading is not that the census was wrong but that **occupying a cell is not the
same as benefiting from it**, and our search for value in the mechanism
inventory has run out of road.

## Two things worth more than the null

**Hard prohibition is not neutral on weighted instances — it is mildly harmful.**
RC-023 measured no difference on G-Set. Here `tabu_sweep` loses 62 of 81 decided
instances, at every rung, sign test p = 1.8 × 10⁻⁶. The magnitude is small
(−0.38 %) and immaterial, but the *sign* is now established where G-Set could not
establish it. Tenure was fixed at 10 and never tuned, in both cycles; this says
nothing about a tuned tabu, and says clearly that the untuned form costs
something on a corpus with real weights.

**Path relinking still never loses.** 54 wins and **0 losses** in 90 instances
here, after 69 wins and 0 losses in 90 on G-Set. Across 180 paired trials on two
corpora, the candidate has not once finished worse than its control. That is the
never-worse invariant, proven on synthetic endpoints before either run and
enforced by rewinding each source to its best prefix. It is a reliable mechanism
that reliably buys about a tenth of a percent.

## Exploratory, and not to be read as findings

Within each density ladder separately, Spearman gives:

| ladder | memory form | synthesis |
|---|---:|---:|
| 10 % | ρ = −0.179, p = 0.343 | ρ = −0.149, p = 0.432 |
| 50 % | ρ = +0.043, p = 0.824 | ρ = +0.245, p = 0.192 |
| 90 % | **ρ = +0.604, p = 0.0004** | **ρ = −0.654, p = 0.0001** |

These six tests were **not registered**. Two of them reach p < 0.001, at the same
density, **in opposite directions**, which is why the registered pooled test is
null: they cancel. Six unregistered tests producing two strong and mutually
contradictory signals is the exact shape of a garden of forking paths, and
`memory/OPEN_PROBLEMS.md` §0d and §0e record what happened the last two times a
perfect-looking split was promoted without its own preregistration. Recorded so
the numbers are not lost; **not** offered as evidence of anything.

## A defect in this preregistration, recorded rather than tidied away

§2 registered a "secondary, deliberately unmatched extension" over `t2g`, `t3g`
and `ising2.5/3.0`, reaching 200–1104 distinct weights. §3 and §7 then froze a
540-row run that **excludes it**. The preregistration is internally inconsistent,
and the extension was not run.

It is **not** being run now. The primary result is known, and adding an
unregistered arm after seeing a null is precisely the move the whole
preregistration procedure exists to forbid. §2 itself said the extension "may
never establish" a conclusion on its own, because weight diversity is confounded
there with density and topology. Anyone wanting that range must preregister it
fresh.

## What the refusal was worth

The harness stopped the first run instead of measuring. The families in §2 were
named without their size suffix — `pm1s` rather than `pm1s_100` — and `pm1s`,
`pm1d` and `g05` each exist at more than one size in this collection. A matcher
that accepted a bare prefix would have drawn `pm1s_80` and `pm1s_100` into one
rung, mixed two problem sizes inside a comparison that holds size constant,
still returned ten files, and produced a plausible number with nothing
complaining.

Finding it at run time was one run too late, so every registered family name is
now resolved against the corpus in a test: a wrong name, a lost file, or a rung
whose measured weight diversity has drifted fails `cargo test`. Three mutations
confirm it, and the first of them survived every test that existed before.

## Prohibited claims (PREREG §6, honoured)

**No wall-time claim of any kind.** RC-021 left this host `INSTRUMENT-INVALID`
and `research/RC021_C10_DIAGNOSIS.md` measured why: it cannot resolve a paired
wall-time difference below about 5 %. RC-025 measures paired solution quality at
identical seeds and logical budget, which needs no qualified host.

No claim about `UltimateSolver`, production routing, tuned variants of either
mechanism, corpora beyond Biq Mac and G-Set, or universal optimization. This
result does not make RC-003, RC-023 or RC-024 into positive results, and it does
not license retiring G-Set for reasons other than the one tested here — §1's
proof that G-Set cannot separate the guide axis stands untouched, because RC-025
tested whether that degeneracy explains *these* flat results, not whether the
degeneracy exists.

## Artifacts

- `raw.tsv` — header plus exactly 540 rows, energies at full `f64` precision
- `run.err` — stderr of the registered invocation, retained verbatim
- this file
