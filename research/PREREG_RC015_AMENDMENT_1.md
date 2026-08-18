# PREREG — RC-015 Amendment 1: exact ratio, and D-15's semantics

**Status:** binding amendment to `PREREG_RC015.md`, written and committed
**before D-15 runs**. It corrects one rounded claim, fixes the semantics of what
D-15 can and cannot refute, and makes one criterion machine-checkable. **D-15
itself is unchanged and runs exactly as pre-registered.**

**Reason:** an independent review found that (a) the acceptance-ratio table
states as `1.0000` a quantity that is not 1, (b) D-15's monotonicity requirement
is stronger than the causal claim it is meant to test, and (c) "reproduces the
`G` arm within seed noise" is not yet a machine criterion.

---

## A1.1 — The exact ratio, which is cleaner than the rounded table

`PREREG_RC015.md` §1 tabulates the Metropolis : heat-bath acceptance ratio as
`1.0000` at `ΔE = +1` and `+2`. Those are **rounded**, not exact. With

```
p_M(ΔE) = min(1, e^{−ΔE/T})            (ties fall in the accept branch)
p_G(ΔE) = 1 / (1 + e^{ΔE/T})           (uniform in sign — derived from
                                        gibbs_color_sweep.rs for x = 0 and x = 1)
```

the ratio has a single closed form, **verified numerically to 1e-12 across
`T ∈ {0.1, 0.5, 2.0}` × `ΔE ∈ {−2,−1,0,+1,+2}`:**

> **p_M(ΔE) / p_G(ΔE) = 1 + e^{−|ΔE|/T}**

It is symmetric in the sign of `ΔE`, and **exactly 2 at `ΔE = 0`, at every
temperature**. Concretely:

| T | ratio at `\|ΔE\| = 1` | ratio at `\|ΔE\| = 2` | ratio at `ΔE = 0` |
|---|---|---|---|
| 0.1 | 1.0000454 | 1.000000002 | **2** |
| 0.5 | 1.1353353 | 1.0183156 | **2** |
| 2.0 | 1.6065307 | 1.3678794 | **2** |

**The conclusion is unchanged and now stated exactly rather than by rounding:**
the tie channel's ratio is temperature-*independent* at 2, while every other
channel's ratio decays to 1 as `e^{−Δ_min/T}`. `Δ_min` is measured (per-arm
census) as **1.0** on these instances. So the cold dominance of ties is not an
approximation — it is the `T → 0` limit of a closed form.

This also explains the measured gradient (share 0.963 → 0.740 → 0.440 at
T = 0.1 → 0.5 → 2.0) as the off-tie ratio climbing 1.0000454 → 1.135 → 1.607
while the tie ratio stays pinned at 2.

*Corrected in `RC015_TIE_HANDLING.md` and `PREREG_RC015.md` §1 alongside this
amendment.*

## A1.2 — What D-15 can and cannot refute

`PREREG_RC015.md` §9 predicts `Y` monotone in the tie-acceptance probability `q`.
That is **stronger than the causal claim H-15 makes**, and the two must not be
conflated.

The flip decisions are nested in `q` **for a single neutral proposal in a fixed
state**. They are *not* nested along a trajectory: after the first divergent
decision the states differ, so the later `ΔE = 0` set, the later fields and the
whole path differ. A neutral-diffusion network can perfectly well have an
**interior optimum** — `q = 0.25` or `0.75` better than either endpoint.

**Binding semantics, fixed before the data:**

- Non-monotonicity refutes **D-15**, the monotone dose–response hypothesis.
- Non-monotonicity does **not** refute **H-15**, the causal decomposition, which
  is already CONFIRMED held-in and held-out by the three-arm test.
- A **reproducible** interior optimum means tie handling is a **non-linear control
  parameter**, not an absent channel — a stronger practical result than
  monotonicity, because it implies the library needs a `neutral_move_rate` policy
  variable rather than a binary Metropolis/heat-bath choice.
- An interior optimum is only reportable as such if it **replicates**; a
  single-seed-set wiggle is noise (§A1.4).

## A1.3 — "reproduces the `G` arm" becomes machine-checkable

`PREREG_RC015.md` §9's clause "the `q = 0.5` arm does not reproduce the `G` arm's
`Y` within seed noise" is judgement, not a criterion. Fixed now:

> **Equivalence holds iff** the paired difference `Y(q=0.5) − Y(G)` satisfies
> **either** a percentile bootstrap 95% CI (2000 resamples, seeded) lying entirely
> inside **±0.1% of `|Y|`**, **or** the established non-materiality rule
> (`ρ < 0.5` **and** `rel < 0.1%`).

Both are pre-existing thresholds in this programme; neither is introduced to fit
a result. Failure of equivalence means `M½` and `G` do **not** share only the tie
rule, which would itself be a finding about the decomposition.

## A1.4 — Held-out must replicate the curve's SHAPE

`PREREG_RC015.md` §9 is silent on how the held-out arm validates a *curve*.
Fixed now: the held-out run must reproduce

1. the **monotonicity verdict** (monotone vs interior-optimum), and
2. the **position of the optimum** in `q` where one exists,

not merely the sign of the endpoints. A shape that does not replicate is reported
as **not replicated**, and no interior optimum is claimed from one seed set.

## A1.5 — G11/G32 are a different regime and need their own pre-registration

RC-015 §9 follow-up 4 proposes decomposing G11/G32 next. Their RC-014 effects were
measured **on a temperature ladder (4.0 → 0.1)**, not at flat `T = 0.1`. Under the
A1.1 identity the ladder mixes ratios from 1.607 (hot rungs) to 1.0000454 (cold
rungs), so the tie share there is a *different quantity* from the one RC-015
measured. That decomposition is a **separate cycle with its own pre-registration**
and may not be folded into RC-015's verdict.

## A1.6 — After the curve: the neutral network, not the tie count

Declared now so it is not a post-hoc pivot. RC-015 established that tie *count*
does not predict the sign (G43 has the fewest ties in set A and the largest
effect). The next observables are properties of the **neutral network actually
traversed**, per arm:

- run lengths of consecutive neutral moves;
- number of distinct states reached via neutral moves;
- recurrence of neutral transitions (return to an already-visited state);
- the energy of the first non-neutral exit after a neutral run;
- replica overlap immediately before and after each neutral run.

None is measured yet, and none is claimed to predict anything.

## A1.7 — Scope unchanged

No Runtime change, no `theory.rs` change, no production operator, no published
fact altered, no `neutral_move_rate` added to production. The curve and its
held-out replication come first.
