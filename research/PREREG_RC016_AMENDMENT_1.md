# PREREG — RC-016 Amendment 1: census inference, qualification, and a model-conditional power rule

**Status:** binding amendment to `PREREG_RC016.md`, written and committed
**before any RC-016 instrument code exists and before any datum — pilot, held-in
or held-out — is collected.** Documentation only: no code was implemented and no
experiment was run.

**Reason.** A review of the committed pre-registration found four blocking issues.
All four are accepted. Two further corrections were then required, and both
correct claims that were wrong in my own text rather than merely imprecise:
the power rule is **model-conditional**, not assumption-free (§A6), and `K = 0`
does **not** by itself license the PAIR INTERCHANGEABLE verdict (§A4).

## Sections superseded

| original | status |
|---|---|
| §6.2 Population statistic | **SUPERSEDED** by §A1 |
| §6.3 Three verdicts | **SUPERSEDED** by §A3 |
| §6.4 Kill criteria | **SUPERSEDED** by §A4 |
| §8.2 "The cost arm is unpaired and non-causal" | **SUPERSEDED** by §A5 |
| §8.3 "Machine-checkable power rule" | **SUPERSEDED** by §A6 |
| §6.1 Per instance | **amended** — §A2 adds the `QUALIFIES` predicate |
| §8.1 Calibration | **amended** — §A5.3 adds the level-2 validity guard |

Everything not named above stands unchanged, including §1–§5, §7, §9, §10 and
§11: the primary estimand, the exact scope, `S₀`-not-`S₁`, the enumerated hashed
corpus, the seed lists, the functional and its secondaries, the controls, and the
RC-017 out-of-scope declaration.

---

## A1 — Cross-instance binomial inference is DELETED

§6.2 tested the positive/negative split across instances with an exact two-sided
binomial against `p = 0.5`. That test is removed entirely, for a reason stronger
than the dependence objection that prompted the review.

**There is no sample.** §4 enumerates and hashes the **entire** locally available
corpus — all 30 G-Set files — and every one is measured. A binomial against
`p = 0.5` asks whether a pattern of signs could arise by chance in a *sample* from
a population of instances. No population is defined and no sampling occurs: this
is a **census**. Once each instance's sign is established, the count of positive
versus negative instances is a **descriptive quantity of a finite corpus, not an
estimate**, and carries no cross-instance sampling variance to test.

The dependence objection is separately correct and independently sufficient: the
30 instances are stratified by `n`, `m` and topology (random all-`+1` versus
toroidal ±1) and are not exchangeable draws from one population, so pooling their
signs into one Bernoulli sequence mixes heterogeneous strata.

**Replacement.** The corpus statistic is reported descriptively: `K` = number of
qualifying instances (§A2), `k₊` positive, `k₋` negative, and the corpus fraction
`K/30`. **No p-value is computed across instances and none is licensed.** All
inferential burden rests on the per-instance tests, where genuine seed randomness
lives.

**Generalisation.** Conclusions apply to **these 30 hashed instances** and to
nothing beyond them.

## A2 — Qualification requires significance, not only effect size

§6.1's per-instance machinery is unchanged (exact two-sided sign-flip over
`2⁸ = 256`; `d_seed`; `DEGENERATE_NULL` below `1e-9`; materiality
`ρ ≥ 0.5` **and** `rel ≥ 0.1%`; A6 replication; BH at FDR 0.10 over the held-in
p-values of all 30 instances). What was missing is that the verdicts never
consulted the BH result.

```
QUALIFIES(instance) :=  BH-rejected at FDR 0.10 in the held-in 30-instance family
                    AND material   (ρ ≥ 0.5 AND rel ≥ 0.1%)
                    AND replicating (A6, on held-out)
```

**Why this was a live hole, not a formality.** Materiality is a pure
**effect-size** gate and A6 is a sign/ratio agreement rule; neither mentions a
p-value. At `k = 8` the exact sign-flip test rejects at `α = 0.05` only if at most
**12 of 256** sign assignments are as extreme, and the p-floor
`2/256 = 0.0078125` requires **all eight** paired differences to share a sign. An
instance with five of eight agreeing can carry a large mean, clear `ρ ≥ 0.5` and
`rel ≥ 0.1%`, replicate under A6, and still be statistically indistinguishable
from zero — and under the superseded §6.3 it would have confirmed the hypothesis.

`QUALIFIES` is used identically by every verdict below.

## A3 — Verdicts

| verdict | condition |
|---|---|
| **SIGN VARIES** | `k₊ ≥ 1` **and** `k₋ ≥ 1`. Report every matched group containing two opposite-signed qualifying members, with the exact `S₀` distance between them — the `S₀` counterexample. |
| **SIGN CONSTANT** | `K ≥ 6`, all qualifying instances share one sign, **and** they span **≥ 3 matched groups and both structural families** (random all-`+1`; toroidal ±1). |
| **PAIR INTERCHANGEABLE** | **all 30** instances are **non-degenerate** *and* **non-material** in **both** the held-in and the held-out blocks. See §A4 — `K = 0` alone is **not** sufficient. |

The structural-coverage requirement in SIGN CONSTANT replaces the deleted
binomial threshold. It is a **coverage** condition, not a significance one: it
exists so that "the sign is constant" cannot be declared from a narrow slice of
the corpus, which is precisely how RC-014's five-instance, two-family evidence
overreached.

## A4 — Kill criteria, and the `K = 0` disambiguation

The superseded §6.3 read `K = 0` as PAIR INTERCHANGEABLE. **That is wrong**:
`K = 0` is a statement about *qualification*, and qualification can fail for
reasons that have nothing to do with the operators being interchangeable.

`K = 0` is consistent with at least three distinct worlds:

1. no effect exists anywhere — genuinely interchangeable;
2. effects exist and are **material** but fail BH significance or fail A6
   replication — **underdetermined**, not interchangeable;
3. some instances are `DEGENERATE_NULL` — **saturated**, not interchangeable.

Only world 1 licenses the verdict, so PAIR INTERCHANGEABLE requires the positive
condition in §A3 — all 30 non-degenerate and non-material in both blocks — rather
than the absence of qualification.

**Kill criteria.**

1. **Q-INCONCLUSIVE** — any of:
   - `1 ≤ K < 6`;
   - `K ≥ 6` without the §A3 group/family coverage;
   - `K = 0` while **any** instance is material but non-significant or
     non-replicating (world 2);
   - `K = 0` while **any** instance is `DEGENERATE_NULL`, including an isolated
     one **below** the 50% threshold of criterion 2 (world 3).

   Report `K`, `k₊`, `k₋`, the coverage achieved, and the count of degenerate and
   of material-but-unqualified instances. **Declare no verdict.** No post-hoc
   relaxation of any bar.
2. **BENCHMARK-VALIDITY FINDING** — ≥ 50% of instances `DEGENERATE_NULL`. The
   configuration is saturated at this budget and cannot discriminate; the sign
   question is not answered; recorded as a third benchmark-validity result after
   RC-004's two.
3. **INSTRUMENT INVALID** — any control cited in §9 fails on re-run. Nothing
   measured is then evidence about the hypothesis, in either direction.

## A5 — The cost arm is a level-2 paired seed-block policy contrast

§8.2 declared the cost arm unpaired, non-causal and analysed by Mann–Whitney.
**That was wrong**, and the error is verified from source rather than conceded on
argument.

| claim | source | verified |
|---|---|---|
| Both arms instantiate the **same** generator from the **same** seed | `runtime.rs:196` — `ChaCha8Rng::seed_from_u64(plan.seed)` | ✅ |
| `greedy_descent` consumes **zero** randomness | `greedy_descent.rs:83` takes `_rng`; **0** occurrences of `rng.gen` in the file | ✅ |
| The draw-index → event-address map is **identical** in both operators | both build the traversal order from `state.coloring()` (`metropolis_sweep.rs:44`, `gibbs_color_sweep.rs:60`), a function of the **IR only** | ✅ |

Because `greedy_descent` draws nothing and the run then ends, **there is no
downstream consumer of randomness at all** — a differing stream position after the
substituted step is never read. And because both operators walk the same order,
draw index `i` drives the **same logical event** `(sweep, site, replica)` in both
arms for every `i ≤ min(k_A, k_B)·n·r`.

The superseded text claimed the arms "share only the prefix before the
substitution and diverge immediately". Two errors: the substitution *is* the first
step, so there is no prefix; and it conflated **state divergence** — the effect
being measured, and expected — with **randomness divergence**, which never reaches
the outcome.

### A5.1 Three levels, distinguished

| level | conditions | what it licenses |
|---|---|---|
| **1 — event-level exactness** | draw-identical pair, **equal** draw counts, identical event addressing | every random input drives the same event in both arms ⇒ the difference is attributable to **the decision rule alone**. A counterfactual claim. **This is `I_replace_work`.** |
| **2 — seed-block policy contrast** | same seed and stream, identical event addressing on the overlap, **unequal** draw counts, **no** stochastic downstream consumer | valid paired potential outcomes for the intervention *"replace the whole policy `(metropolis, 16 sweeps)` with `(gibbs, k′ sweeps)`"*. **Causal about the policy**, not about a single event, because the budget changed by construction. **This is `I_replace_cost`.** |
| **3 — unpaired comparison** | different seeds, or a stochastic downstream consumer breaking event addressing | a distributional comparison only; no pairing, no CRN variance reduction. **Not applicable to this design.** |

### A5.2 Analysis of the cost arm

- **Test:** the **exact paired two-sided sign-flip test over all `2⁸ = 256` sign
  assignments** — the same test as the primary. **Mann–Whitney, the `3/π` ARE
  inflation and all normal-theory power algebra are deleted.**
- **Null scale:** paired-difference `d_seed`; the same `DEGENERATE_NULL` rule.
- **Permitted language:** causal about the **policy**. **Forbidden:** attributing
  the difference to the decision rule alone, since the budget changed by
  construction.
- **`I_work` and `I_cost` are never differenced or ranked against each other** —
  they estimate different interventions.
- **CRN overlap, reported per instance:** `min(16, k′) / max(16, k′)`. Level-2
  pairing is valid at any overlap, but its variance reduction **degrades** as the
  fraction falls, so the fraction accompanies every reported result.

### A5.3 Guard added to §8.1

§8.1 stands — pilot seeds 3001–3008, interleaved conditions, medians, frozen
before the science run; `R = median_ms_gibbs / median_ms_metropolis ∈ [0.95, 1.05]`
⇒ equal sweeps is equal cost and no cost arm runs; **`cost_model` remains
forbidden** as a cost measure (RC-005). **Added:** level-2 validity is conditional
on there being **no stochastic operator downstream of the substitution point**.
That holds here and is verified above; it **must be re-verified** if the schedule
ever changes.

## A6 — Power rule: MODEL-CONDITIONAL, and specified exactly

The superseded §8.3 was wrong twice, and both errors are mine.

**Error 1 — it was not a power calculation.** It formed a single shifted dataset
and asked whether that one realisation rejects. A single draw estimates nothing;
power is a **rejection rate over the sampling distribution**.

**Error 2 — "assumption-free" was false.** Bootstrapping centred residuals under
an added constant assumes an **additive location-shift** alternative and assumes
the **pilot residual distribution represents the science run's**. Those are
modelling commitments. The rule is **model-conditional** and is labelled so
wherever it is reported.

### A6.1 `Y_ref`, defined and frozen

```
Y_ref(instance) := | median over pilot seeds 3001–3008 of
                    Y( metropolis policy at its calibrated sweep count ) |
```

Computed in the calibration pass and **frozen before any science seed runs.**

### A6.2 The procedure

Per instance, before the science run:

1. From the pilot, obtain the 8 paired differences `d_i` at the calibrated sweep
   counts.
2. **Centre:** `e_i = d_i − mean(d)`.
3. **Shift:** `Δ = 0.001 · Y_ref` — the materiality bar itself, so the arm is
   required to be powered for exactly the effect the primary calls material.
4. **Bootstrap:** draw **100 000** datasets. Each draws 8 residuals from
   `{e_i}` **with replacement** and adds `Δ`. Randomness comes from a
   **`ChaCha8Rng` with a pre-declared fixed seed**, recorded in this amendment as
   `POWER_BOOTSTRAP_SEED = 20260819`, so the computation is deterministic and
   replayable.
5. **Test:** on each bootstrap dataset, the **exact** two-sided paired sign-flip
   test over all `2⁸ = 256` assignments, at `α = 0.05`.
6. **POWERED** iff the rejection rate over the 100 000 datasets is **`≥ 0.80`**;
   otherwise **UNDERPOWERED**.

### A6.3 Resolution and reference facts

Monte-Carlo error at 100 000 datasets and a rejection rate near 0.80 is
`SE = √(0.8·0.2/100000) = 0.00126`, so the 95% interval half-width is
**0.00248** — the 0.80 threshold is resolvable to about ±0.0025 and is not
limited by bootstrap noise.

Exact facts about the test at `k = 8`: `α · 2⁸ = 12.8`, so rejection requires at
most **12 of 256** assignments to be as extreme; the p-floor is
`2/256 = 0.0078125`, attained only when **all eight** paired differences share a
sign.

### A6.4 Stated assumptions

The POWERED/UNDERPOWERED determination is valid **only under** both of:

- **Empirical additive location shift** — the alternative differs from the null by
  a constant `Δ` added to every paired difference, leaving the residual
  distribution's shape and scale unchanged.
- **Pilot representativeness** — the pilot's residual distribution at the
  calibrated sweep counts is the distribution the science run will exhibit.

Neither is verifiable in advance. Both are recorded so that a power verdict is
never presented as distribution-free.

### A6.5 UNDERPOWERED arms are NOT RUN

Where the determination is UNDERPOWERED, the science cost arm is **not executed**.
The pre-data determination is reported instead:

> *instance X: cost arm not run — UNDERPOWERED at k = 8 seeds; pilot paired sd
> `s_pair` = …; shift `Δ` = …; bootstrap rejection rate = … < 0.80.*

Running an arm whose result may not be interpreted invites exactly the informal
reading the prohibition intends to prevent. Any `n_required` figure is
**advisory only**, explicitly labelled as resting on a normal approximation, and
may never gate a verdict. Seed counts are frozen at 8 and are not increased to
chase power.

---

## What is now weaker, and what is stronger

**Weaker.** No cross-instance p-value exists. The corpus verdict is a census
statement about 30 enumerated, hashed instances and generalises to nothing beyond
them. SIGN CONSTANT additionally requires structural coverage the corpus may not
supply. PAIR INTERCHANGEABLE requires a positive condition over all 30 instances
in both blocks and will therefore be reached rarely. The power rule is
model-conditional and says so.

**Stronger.** Qualification now includes significance, closing a hole through
which a five-of-eight instance could have confirmed the hypothesis. The cost arm
recovers paired CRN analysis — more power *and* a more accurate epistemic label
than the unpaired version it replaces. The power rule is an actual rejection rate
rather than a single draw. `K = 0` can no longer be silently read as
interchangeability. And the level-1 / level-2 / level-3 taxonomy states precisely
what each estimand does and does not license, which the original conflated.

## Facts verified against source for this amendment

| claim | source | verified |
|---|---|---|
| Same generator from the same seed in both arms | `runtime.rs:196` | ✅ |
| `greedy_descent` consumes zero randomness | `greedy_descent.rs:83`; 0 × `rng.gen` | ✅ |
| Traversal order is a function of the IR only | `metropolis_sweep.rs:44`, `gibbs_color_sweep.rs:60` | ✅ |
| `α·2⁸ = 12.8` ⇒ reject iff ≤ 12 of 256; p-floor `2/256 = 0.0078125` | computed | ✅ |
| Bootstrap SE at `n = 100 000`, `p = 0.80` is `0.00126` (95% half-width `0.00248`) | computed | ✅ |
| Superseded section numbers exist as cited | `PREREG_RC016.md` §6.1–6.4, §8.1–8.3 | ✅ |
