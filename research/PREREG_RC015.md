# PREREG — RC-015: is the cold reversal a tie-handling effect?

**Pre-registration under Constitution §13.** Written and committed **before any
RC-015 instrument code exists**. Thresholds, instances, seeds, arms, observables
and the gate are binding and may not be renegotiated after seeing results.

**Origin.** RC-014 §10 found `gibbs_color_sweep` decisively better than
`metropolis_sweep` on **G43 at flat T = 0.1** (`I = −44.13`, ρ = 5.31,
p = 0.0078, all eight seeds) and recorded it as an unexplained anomaly, having
dismissed the tie mechanism on the grounds that G43's tie mass is "only 2.3%".

**That dismissal was wrong, and correcting it is what motivates this cycle.**

---

## 1. Why the RC-014 dismissal does not follow

**Error 1 — a fraction was compared where a count was required.** 2.3% of
`n × sweeps = 1000 × 16 = 16 000` proposals per replica is **368 tied proposals**.
Metropolis flips at every one; heat-bath flips at half. That is **≈184 divergent
decisions per replica**, which is ample to place replicas in different basins.
Comparing G43's 2.3% against G11/G32's 33% answered "which instance has more
ties", not "are there enough ties here to matter".

**Error 2 — off-tie behaviour at T = 0.1 was not checked.** With integer
couplings and `T = 0.1`, computed directly:

| `ΔE` | Metropolis accept | heat-bath flip | ratio |
|---|---|---|---|
| +1 | 4.540e−05 | 4.540e−05 | **1.0000** |
| +2 | 2.061e−09 | 2.061e−09 | **1.0000** |
| **0** | **1.0** | **0.5** | **2.0** |
| < 0 | ≈1 | ≈1 | ≈1 |

At `T = 0.1` the two kernels are **numerically indistinguishable off ties**. So
essentially *all* of the observed M−G difference at that temperature must flow
through the `ΔE = 0` channel. The tie mechanism is not merely admissible there —
it is the only remaining channel. (At `T = 2.0` the +1 ratio is 1.61, so ties are
*not* dominant; this predicts a gradient, tested non-gating in §7.)

**Error 3 — the tie statistic was post-treatment.** RC-014's `tie_fraction`
counts `ΔE = 0` proposals **along a Metropolis trajectory only**. After the first
divergent decision the heat-bath arm is on a different trajectory with a
different tie profile, so 2.3% describes one arm's realised path, not the
landscape. Conditioning on it is post-treatment. **RC-015 records the `ΔE` class
counts separately for every arm.**

---

## 2. Hypothesis

**H-15.** The cold reversal on G43 is produced by **tie handling**, not by any
general difference between the kernels: Metropolis's unconditional acceptance of
neutral moves walks replicas along the zero-field network into basins from which
`greedy_descent` finishes worse, while heat-bath's half-rate resampling leaves
them where the quench does better.

**H-15-null.** The reversal survives when tie handling is equalised.

---

## 3. Design — three arms, one draw per proposal

| Arm | `ΔE < 0` | `ΔE = 0` | `ΔE > 0` |
|---|---|---|---|
| **M** — `metropolis_sweep` (existing) | draw, accept | **draw, accept** | draw, accept iff `u < e^{−ΔE/T}` |
| **M½** — new, experiment-local | draw, accept | **draw, flip iff `u < ½`** | draw, accept iff `u < e^{−ΔE/T}` |
| **G** — `gibbs_color_sweep` (existing) | draw, flip iff `u < σ` | draw, flip iff `u < ½` | draw, flip iff `u < σ` |

All three consume **exactly one `f64` per `(sweep, site, replica)`**, so all three
are mutually draw-identical and every pairwise comparison is an exact
counterfactual under the current stream RNG (RC-014 Phase 0 §0.1, conditions 1
and 2 — the only downstream operator is `greedy_descent`, which draws nothing).

**Decomposition:**

```
I_tie  = Y(M½) − Y(M)     tie handling alone
I_off  = Y(G)  − Y(M½)    everything other than ties
I_full = Y(G)  − Y(M)     the RC-014 anomaly,  I_full = I_tie + I_off
```

**Null by construction (RC-001's discipline).** The new operator is parameterised
by tie mode; with `TieMode::Accept` it must be **bit-identical to
`metropolis_sweep`** — same energies, same best state, same generator position.
This is a Gate-blocking control: it makes a construction error impossible to
mistake for an effect. M½ is registered into an **experiment-local** registry
copy; the production registry stays at 18 operators (Constitution §13: prototypes
do not touch production).

---

## 4. Instances — a matched set that already exists

No synthetic generator is needed; G-Set contains exact matched triples
(verified from file headers, all weights `+1`):

| Set | Instances | n | m | Role |
|---|---|---|---|---|
| **A** | **G43, G44, G45** | 1000 | 9990 | the anomaly and its two exact siblings |
| **B** | G22, G23, G24 | 2000 | 19990 | size check at 2× n |

Set A separates *"a peculiarity of the G43 realisation"* from *"a property of the
family"*. Set B separates *family* from *size dependence*.

## 5. Fixed parameters

| Parameter | Value |
|---|---|
| Schedule | `[X@16, greedy_descent@16]`, `X ∈ {M, M½, G}` |
| Ladder | **flat T = 0.1** (where the anomaly lives) |
| Replicas | 32 · **Init** legacy all-zeros · **Backend** as the Decision Engine selects |
| Held-in seeds | 1001–1008 · **Held-out** 2001–2008, not inspected until the gate's controls and held-in criteria are evaluated |
| Statistics | exact two-sided sign-flip over 2⁸; `d_seed` = sample sd of the reference arm; `DEGENERATE_NULL` if `d_seed < 1e-9`; materiality ρ ≥ 0.5 **and** rel ≥ 0.1% |
| Multiplicity | **G43 is the primary named contrast** at α = 0.05 unadjusted; G44, G45 and set B are secondary under Benjamini–Hochberg at FDR 0.10 |

## 6. The gate (binding)

**Controls, all blocking:**

1. `TieMode::Accept` is bit-identical to `metropolis_sweep`.
2. All three arms leave the generator at the same position after the substituted
   step (operator-boundary probe), and an injected one-word shift is detected.
3. The decomposition closes: `I_full − (I_tie + I_off)` is exactly `0`.

**Verdict on G43 (primary), held-in then replicated held-out:**

- **H-15 CONFIRMED** iff `|I_tie| ≥ 0.80 · |I_full|` **and** the residual `I_off`
  does **not** reach ρ ≥ 0.5.
- **H-15 REFUTED** iff `I_off` is material (ρ ≥ 0.5 and rel ≥ 0.1%) **and**
  replicates on the held-out seeds under the RC-014 A6 rule.
- **Otherwise PARTIAL**, reported as such with both components, with no
  post-hoc adjustment of the 80% bar.

**Reported separately, always:** the outcome **before** `greedy_descent` and
**after** it. H-15's mechanism claim is specifically about where the quench
starts, so a tie effect that exists only post-quench is different evidence from
one already present pre-quench, and the two must not be merged.

## 7. Required observables

Recorded for **every arm separately** — not inferred from one arm's trajectory:

- counts of `ΔE < 0`, `ΔE = 0`, `ΔE > 0` proposals;
- the minimum non-zero `|ΔE|` encountered;
- best energy **before** `greedy_descent` and **after**;
- the gain attributable to `greedy_descent`;
- number of distinct final best-states across seeds, and pairwise Hamming
  distance between arms' best-states;
- the generator fingerprint after the substituted step.

**Not claimed** (Amendment-1 A2 discipline — the seam does not expose it):
per-invocation wall cost, and full-ensemble replica overlap. `RunRecord` exposes
only the best state, so state-diversity is reported over best-states across
seeds, and is labelled as such rather than presented as ensemble overlap.

**Non-gating gradient prediction.** If the mechanism is tie handling, the share
`|I_tie| / |I_full|` must **fall as T rises**, because off-tie acceptance ratios
diverge (1.0000 at T=0.1, 1.14 at T=0.5, 1.61 at T=2.0 for `ΔE=+1`). Measured at
`T ∈ {0.1, 0.5, 2.0}` on G43. Reported, not gating.

## 8. Scope

No Runtime change. No `theory.rs` change. No new *production* operator — M½ lives
in the experiment's local registry. No dataset growth, no Foundry, no DSL. The
production solver and default initialisation are untouched. RC-014's Gate B
remains failed and is not reopened by this cycle.

## 9. Falsifiable descendant

> **To be written before the held-out seeds are run. Left empty deliberately.**
