# PREREG — RC-016 Amendment 3: a deterministic sensitivity CI, and a scoped power caveat

**Status:** binding amendment to `PREREG_RC016.md` as amended by Amendments 1 and
2, written and committed **before any RC-016 instrument code exists and before any
datum — pilot, held-in or held-out — is collected.** Documentation only: no code
implemented, no experiment run.

**Reason.** Two final blocking findings in `PREREG_RC016_AMENDMENT_2.md`. Both are
independently verified below and both are **accepted**.

## Amendment 2 clauses superseded — and only these

| clause | status |
|---|---|
| §B1, the "Mandatory accompaniment" paragraph (lines 83–86) | **SUPERSEDED** by §C1 |
| §B5, the blockquote "the bootstrap power estimate is an OVER-estimate" and the sentence "a POWERED determination is an **upper bound** on achievable power" | **SUPERSEDED** by §C2 |
| §B5's summary line in "What is now weaker": "Power determinations are upper bounds rather than guarantees" | **SUPERSEDED** by §C2 |

Nothing else changes. §B1's rename of the verdict to **`NO MATERIAL EFFECT
OBSERVED`**, its prohibitions on equivalence/interchangeability/gain-bounding
language, §B2's `K = 0` disambiguation, §B3's binding `Δ`, §B4's tie-floor and
`TIE-BLOCKED` status, and §B5's first two modelling assumptions all stand, as does
everything in `PREREG_RC016.md` and Amendment 1 not previously superseded.

---

## C1 — The sensitivity CI, specified deterministically

### The finding, verified

§B1 requires the verdict to be reported "only together with … the per-instance
bootstrap 95% CI half-width on the paired difference, expressed as a percentage of
`|Y|`" (Amendment 2, lines 83–86). That sentence names **no algorithm** (percentile,
basic, or BCa), **no resample count**, **no seed**, and **no block handling** —
it does not say whether held-in and held-out are pooled or reported separately.

Two implementers following it would produce different numbers from the same data,
and the quantity would not be replayable. That is not merely untidy: it conflicts
with **ADR-0004**, under which a recorded result must be reproducible from its
inputs. **The finding is accepted.**

### The specification

The sensitivity CI is computed **per instance and per block, never pooled**:

1. **Blocks are separate.** Held-in and held-out each get their own CI. They are
   never combined, because they are separate pre-registered seed sets and pooling
   would destroy the replication logic that A6 depends on.
2. **Resample unit.** The **8 paired differences** `d_i` of that block, drawn
   **with replacement**, 8 at a time.
3. **Statistic.** The **mean** paired difference `Ī` of each resample.
4. **Interval.** The **percentile** 95% CI: the 2.5th and 97.5th percentiles of
   the resampled means. Not basic, not BCa — percentile, named explicitly so the
   choice is not left to the implementer.
5. **Resamples.** Exactly **100 000**.
6. **Seed.** A `ChaCha8Rng` seeded deterministically:

   ```
   CI_BOOTSTRAP_BASE_SEED = 20260820

   seed(instance, block) = CI_BOOTSTRAP_BASE_SEED
                         + 2 · (zero-based corpus index of the instance)
                         + (0 for held-in, 1 for held-out)
   ```

   The corpus index is the position in the **enumerated, hashed corpus table of
   `PREREG_RC016.md` §4**, which fixes the order: `G1` = 0, `G2` = 1, `G3` = 2,
   `G11` = 3, …, `G70` = 29.

   **Verified:** over 30 instances × 2 blocks this yields **60 distinct seeds**
   spanning `20260820 … 20260879`, and **none collides** with
   `POWER_BOOTSTRAP_SEED = 20260819` (Amendment 1 §A6.2), so the sensitivity CI
   and the power bootstrap can never share a stream.

7. **Reported quantities.** For each instance and block, all three of:
   - the CI **lower** bound,
   - the CI **upper** bound,
   - the **half-width**, expressed **relative to `|mean Y of the reference
     (metropolis) arm|` in that block**, in percent.

   The reference-arm mean is stated explicitly so "relative to `|Y|`" — ambiguous
   in §B1 — has one meaning.

### What this CI may and may not be used for

**May:** quantify the sensitivity that accompanies a `NO MATERIAL EFFECT
OBSERVED` verdict, so a reader can see what that verdict does not exclude.

**May NOT — binding prohibition:**

- it is **not** an input to `QUALIFIES` (Amendment 1 §A2), which remains exactly
  BH rejection **and** materiality **and** A6 replication;
- it is **not** an equivalence test and does **not** license any equivalence,
  interchangeability or indistinguishability claim, no matter how narrow the
  interval turns out to be — Amendment 2 §B1's prohibitions are unaffected;
- it does **not** enter the multiplicity family;
- it may **not** be compared against the materiality bar to produce a verdict of
  any kind.

It is a **descriptive** statistic reported alongside a descriptive verdict.

## C2 — The power caveat is scoped, not a bound

### The finding, verified

Amendment 2 §B5 states that the continuum bootstrap makes the power estimate "an
**OVER-estimate**", that "a POWERED determination is an **upper bound** on
achievable power", and summarises that "Power determinations are upper bounds
rather than guarantees" (lines 200, 205, 216).

**That over-claims, and the finding is accepted.** The argument in §B5 establishes
one channel only: the bootstrap operates on a continuum and therefore essentially
never produces the exact-zero differences that can make real, integer-valued data
`TIE-BLOCKED` (§B4). Holding everything else fixed, removing a tie can only reduce
the permutation tail count and so can only make rejection easier — so that channel
is **directionally optimistic**.

But it is not the only channel. The bootstrap resamples **centred pilot
residuals**, and the science-run distribution may differ from the pilot's in shape,
scale or location. If the science run's residual dispersion is *smaller* than the
pilot's, real power would be *higher* than the estimate. Nothing in the design
constrains that mismatch, so **no bound in either direction is established for the
estimate as a whole**, and a claim of "upper bound" is not licensed for an unknown
lattice distribution.

### The replacement statement

> **Scoped caveat.** The bootstrap generates data on a continuum while the science
> data lie on the integer lattice, so it essentially never produces the exact-zero
> paired differences that can render a real instance `TIE-BLOCKED` (§B4). **With
> respect to tie-blocking specifically, and holding all else equal, the estimate is
> therefore optimistic.** Other distribution mismatch between the pilot and the
> science run — in shape, scale or location — is unconstrained by this design and
> **can move power in either direction**. No bound on the overall power estimate is
> claimed.

**Operational consequence, unchanged:** a `POWERED` determination is not a
guarantee, and any instance that turns out `TIE-BLOCKED` in the science run is
reported as such irrespective of its pre-data determination (§B4, §B5). What is
withdrawn is only the *directional bound* on the estimate as a whole.

The two modelling assumptions of §A6.4 as extended by §B5 — empirical additive
location shift, and pilot representativeness — stand unchanged, and this scoped
caveat is the third item on that list.

---

## What is now weaker, and what is stronger

**Weaker.** The power estimate carries no directional bound; it is
model-conditional with one channel identified as optimistic and the rest
unconstrained.

**Stronger.** The sensitivity CI is now a determined, replayable quantity — one
algorithm, one resample count, one seed per (instance, block), one denominator —
rather than an instruction each implementer would resolve differently. Held-in and
held-out are reported separately, so the replication logic is not silently pooled
away. And the CI is fenced off from qualification and from equivalence, so adding
it cannot reopen the claim Amendment 2 closed.

## Facts verified against source for this amendment

| claim | source | verified |
|---|---|---|
| §B1 names no algorithm, resample count, seed or block handling | `PREREG_RC016_AMENDMENT_2.md` lines 83–86 | ✅ |
| §B5 uses strict bound language at three sites | same file, lines 200, 205, 216 | ✅ |
| The seed formula yields 60 distinct seeds, `20260820…20260879` | computed over 30 instances × 2 blocks | ✅ |
| No collision with `POWER_BOOTSTRAP_SEED = 20260819` | computed | ✅ |
| The corpus order is fixed and enumerated | `PREREG_RC016.md` §4, 30 hashed rows | ✅ |
| `QUALIFIES` is unchanged by this amendment | `PREREG_RC016_AMENDMENT_1.md` §A2 | ✅ |
