# PREREG — RC-016 Amendment 2: no equivalence claim, and two exactness corrections

**Status:** binding amendment to `PREREG_RC016.md` as amended by
`PREREG_RC016_AMENDMENT_1.md`, written and committed **before any RC-016
instrument code exists and before any datum — pilot, held-in or held-out — is
collected.** Documentation only: no code implemented, no experiment run.

**Reason.** A review found that Amendment 1's `PAIR INTERCHANGEABLE` verdict
licenses an **equivalence** claim from the **absence of a material point
estimate**. The finding is accepted, and it is stronger than stated: the verdict
violates a method rule this programme already recorded, and RC-015 has already
*measured* that the design cannot support the claim. Two exactness defects in
Amendment 1's power section are also confirmed, the second with a sharper
operational consequence than the review stated.

## Amendment 1 clauses superseded

| clause | status |
|---|---|
| §A3, the `PAIR INTERCHANGEABLE` row | **SUPERSEDED** by §B1 |
| §A4, kill criteria — the "only world 1 licenses the verdict" sentence and the verdict name | **SUPERSEDED** by §B2 (the `K = 0` disambiguation and Q-INCONCLUSIVE routing are **preserved verbatim in substance**) |
| §A6.2 step 3, `Δ = 0.001 · Y_ref` described as "the materiality bar itself" | **SUPERSEDED** by §B3 |
| §A6.3, the p-floor claim | **SUPERSEDED** by §B4 |
| §A6.4, stated assumptions | **extended** by §B5 |

Everything else in `PREREG_RC016.md` and Amendment 1 stands, including the
primary estimand, the exact scope, `S₀`-not-`S₁`, the enumerated hashed corpus,
the deletion of cross-instance binomial inference, the `QUALIFIES` predicate, the
SIGN VARIES and SIGN CONSTANT verdicts with their structural coverage, and the
level-1/2/3 taxonomy with the cost arm as a level-2 paired seed-block contrast.

---

## B1 — `PAIR INTERCHANGEABLE` is replaced by `NO MATERIAL EFFECT OBSERVED`

### The finding, and why it is decisive

Amendment 1 §A3 licensed `PAIR INTERCHANGEABLE` when all 30 instances are
non-degenerate and **non-material** in both blocks. "Non-material" is the negation
of a threshold on a **point estimate**:
`NOT( ρ = |I|/d_seed ≥ 0.5 AND rel = |I|/|Y| ≥ 0.001 )` (`PREREG_RC016.md` §6.1).

Failing to exceed an effect-size threshold is **absence of a detected effect**. It
is not evidence that the effect is absent, and "interchangeable" asserts exactly
that. Establishing equivalence requires an equivalence procedure — a TOST, or a
confidence interval lying wholly inside a pre-registered equivalence margin —
which no clause of this pre-registration declares for this verdict.

**Two facts make this decisive rather than pedantic.**

1. **It violates a method rule this programme already recorded.**
   `memory/RESEARCH.md:291`, among the rules earlier cycles produced:
   > *Claiming "no difference" requires an **equivalence bound**, not `p > 0.05`.*

   Amendment 1 reintroduced precisely the error that rule exists to prevent, one
   step removed — via a point-estimate threshold instead of a p-value.

2. **RC-015 already measured that this design cannot support the claim.**
   `RC015_TIE_HANDLING.md` §10.5 records that at 8 seeds the bootstrap 95% CI on
   the paired difference had **half-widths of 0.09–0.21%**, against a materiality
   bar of **±0.1%**. The interval is therefore **wider than the bar on most
   instances**, so at `k = 8` an equivalence verdict at this margin is **not
   attainable even in principle**, whatever the data show.

### The correction

The verdict is renamed and demoted to a description of what was observed:

| verdict | condition | what it means |
|---|---|---|
| **NO MATERIAL EFFECT OBSERVED** | all 30 instances **non-degenerate** and **non-material** in **both** the held-in and the held-out blocks | **A descriptive statement about this design's sensitivity, and nothing more.** No material effect was detected at `ρ ≥ 0.5` and `rel ≥ 0.1%` with 8 paired seeds per block. |

**Explicitly forbidden under this verdict**, in the report and in any downstream
citation of it:

- any claim that the two operators are **equivalent**, **interchangeable**, or
  **indistinguishable**;
- any claim that the effect **is** zero, negligible, or absent;
- any claim that **bounds what a selector could gain** — Amendment 1's phrasing,
  now withdrawn, because a bound on achievable gain is an equivalence claim in
  other words.

**Mandatory accompaniment.** The verdict is reportable only together with the
design's measured sensitivity: the per-instance bootstrap 95% CI half-width on the
paired difference, expressed as a percentage of `|Y|`, so a reader can see what
"no material effect observed" does and does not exclude.

**Out of scope, declared now so it is not a later pivot.** A genuine equivalence
verdict would require a pre-registered equivalence margin, a TOST or CI-inside-
margin procedure, and — per RC-015 §10.5's measurement — materially more than 8
seeds per block. None is added here, and no equivalence verdict is available to
RC-016 in any outcome.

## B2 — `K = 0` disambiguation preserved, with the verdict renamed

Amendment 1 §A4's substance is retained in full; only the verdict name and the
sentence licensing it change.

`K = 0` remains consistent with three distinct worlds — (1) no effect exists,
(2) effects are material but fail BH significance or A6 replication, (3) some
instances are `DEGENERATE_NULL` — and only world 1 is compatible with the
descriptive verdict. **Amendment 1's sentence "Only world 1 licenses the verdict"
is superseded**: world 1 licenses only the *descriptive* statement of §B1, never
an equivalence claim.

**Kill criteria, unchanged in substance.** Q-INCONCLUSIVE still fires on any of:
`1 ≤ K < 6`; `K ≥ 6` without the §A3 group/family coverage; `K = 0` while any
instance is material but non-significant or non-replicating; `K = 0` while any
instance is `DEGENERATE_NULL`, **including an isolated one below** the 50%
threshold of the benchmark-validity criterion. The benchmark-validity criterion
(≥ 50% degenerate) and the instrument-invalid criterion stand verbatim.

## B3 — `Δ` is the BINDING materiality threshold, not the relative component alone

**Confirmed.** Amendment 1 §A6.2 step 3 sets `Δ = 0.001 · Y_ref` and calls it
"the materiality bar itself". It is not: materiality is a **conjunction**,

```
material  ⟺  ρ = |I| / d_seed ≥ 0.5   AND   rel = |I| / |Y| ≥ 0.001
```

so `0.001 · |Y_ref|` is only the **`rel` component**. The `ρ` component requires
`|I| ≥ 0.5 · d_seed`, a different quantity in different units. Powering at the
`rel` component alone **under-powers the arm whenever `0.5 · d_seed` exceeds
`0.001 · |Y_ref|`**, which is unconstrained a priori.

**Correction.** The shift used in the power bootstrap is the **binding** threshold:

```
Δ(instance) = max( 0.001 · |Y_ref(instance)| ,  0.5 · d_seed_pilot(instance) )
```

where `d_seed_pilot` is the sample standard deviation of the **reference
(metropolis) arm** over the pilot seeds 3001–3008 at the calibrated sweep count —
matching `PREREG_RC016.md` §6.1's definition of `d_seed` as an arm-level, not a
paired, standard deviation. Both components are computed in the calibration pass
and **frozen before any science seed runs**, and both are reported alongside the
`Δ` actually used, so the binding component is visible.

Everything else in §A6.2 is unchanged: centre the pilot paired differences,
resample residuals with replacement, add `Δ`, 100 000 bootstrap datasets from
`POWER_BOOTSTRAP_SEED = 20260819`, exact `2⁸` paired sign-flip per dataset at
`α = 0.05`, POWERED iff the rejection rate is `≥ 0.80`.

## B4 — The p-floor requires all eight differences nonzero, and ties can make rejection impossible

**Confirmed, with a stronger consequence than the review stated.**

Amendment 1 §A6.3 asserts the p-floor `2/256 = 0.0078125` is "attained only when
all eight paired differences share a sign". Incomplete: they must also all be
**nonzero**. A zero difference is unchanged by a sign flip, so it makes pairs of
sign assignments produce **identical** statistics — ties in the permutation
distribution that inflate the tail count.

With `z` zero differences among 8, the extreme `|mean|` is attained by
`2^z · 2 = 2^{z+1}` assignments, so

```
p_floor(z) = 2^{z+1} / 256
```

| `z` | p_floor | can reject at α = 0.05? |
|---|---|---|
| 0 | 0.00781 | yes |
| 1 | 0.01562 | yes |
| 2 | 0.03125 | yes |
| **3** | **0.06250** | **NO** |
| 4 | 0.12500 | no |
| 5 | 0.25000 | no |

> **An instance with three or more zero paired differences cannot reject at
> `α = 0.05` regardless of the magnitude of the remaining five.**

**This is not a hypothetical.** On unweighted G-Set the energies are integers, so
paired differences are integers and exact zeros are reachable. Verified from the
recorded corpus: every RC-014 held-in mean, multiplied by 8, is an integer —
`+37.88 → 303`, `+50.13 → 401`, `+4.75 → 38`, `+17.75 → 142`, `+28.13 → 225`, and
so on for all ten G-Set contrasts — confirming that the per-seed differences lie
on the integer lattice. RC-015 recorded exact `I = +0.0000` cells.

**Required reporting field, added.** For every instance and every arm, report
`z` — the count of zero paired differences — alongside the p-value. Where `z ≥ 3`
the instance is reported as **`TIE-BLOCKED`**: it cannot reach significance by
construction, so it can never satisfy `QUALIFIES`, and it must not be silently
counted as a non-significant instance. `TIE-BLOCKED` instances are routed to
Q-INCONCLUSIVE by the same rule as material-but-unqualified ones (§B2), never to
`NO MATERIAL EFFECT OBSERVED`.

## B5 — One assumption added to §A6.4: the bootstrap works on a continuum, the data on a lattice

Amendment 1 §A6.4 states two modelling assumptions (empirical additive location
shift; pilot representativeness). A third is added, and it follows directly from
§B4.

Centring produces `e_i = d_i − mean(d)`, which is generally **not** an integer,
and adding a real-valued `Δ` yields non-integer `d′_i`. The bootstrap therefore
generates data on a **continuum** and essentially never produces the exact zeros
that the **integer-valued** science data can and will produce.

> **Consequence: the bootstrap power estimate is an OVER-estimate.** Real data can
> be tie-blocked (§B4); simulated data effectively cannot.

The procedure is **not** changed — that would exceed a minimal correction — but
the direction of the bias is recorded, and a POWERED determination is therefore an
**upper bound** on achievable power, never a guarantee. Any instance that turns
out `TIE-BLOCKED` in the science run is reported as such irrespective of its
pre-data POWERED determination.

---

## What is now weaker, and what is stronger

**Weaker.** RC-016 can no longer return any equivalence or interchangeability
conclusion in any outcome; the strongest available null result is the descriptive
`NO MATERIAL EFFECT OBSERVED`, and it must be reported with the design's measured
sensitivity attached. Power determinations are upper bounds rather than
guarantees.

**Stronger.** `Δ` now covers the binding component of a conjunctive materiality
bar rather than one of its two halves, so an arm cannot be declared POWERED while
being blind to the `ρ` component. Tie-blocking is detected and routed rather than
silently mistaken for a null, which closes a path by which an instance that
*cannot* be significant would have been counted as *not* significant. And the
programme's own recorded rule on equivalence bounds is honoured rather than
re-violated.

## Facts verified against source for this amendment

| claim | source | verified |
|---|---|---|
| Materiality is a conjunction of `ρ` and `rel` | `PREREG_RC016.md` §6.1 (line 192) | ✅ |
| `d_seed` is the **reference-arm** sd, not the paired sd | `PREREG_RC016.md` §6.1 (line 188) | ✅ |
| The programme already requires an equivalence bound for "no difference" | `memory/RESEARCH.md:291` | ✅ |
| At `k = 8` the bootstrap CI half-width is 0.09–0.21% against a ±0.1% bar | `RC015_TIE_HANDLING.md` §10.5 (line 264) | ✅ |
| `p_floor(z) = 2^{z+1}/256`; `z ≥ 3` cannot reject at `α = 0.05` | computed | ✅ |
| Per-seed paired differences lie on the integer lattice | every RC-014 held-in mean × 8 is an integer | ✅ |
| Amendment 1 clause labels §A3, §A4, §A6.2–§A6.4 exist as cited | `PREREG_RC016_AMENDMENT_1.md` | ✅ |
