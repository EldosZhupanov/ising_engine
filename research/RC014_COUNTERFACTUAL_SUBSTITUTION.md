# RC-014 — Substitution beats deletion: two operators the ablation engine calls equivalent are not

**Status: Gate A PASSED.** Pre-registered in `PREREG_RC014.md`
(+ Amendments 1 and 2; §4 and follow-up 3 carry Amendment 2's corrections),
committed before instrument code; descendant D-14 committed before the held-out
arm. Phase 0 audit: `RC014_PHASE0_AUDIT.md`. Instrument:
`src/bin/exp_counterfactual.rs`.

**Convention** (as in the rest of the register): CONFIRMED / REFUTED / PARTIAL
refers to the *pre-registered hypothesis*, not to whether the cycle was useful.

---

## 1. The headline, and it is methodological

The platform's Theory Engine has both of these recorded as facts, on the same
condition, from its own ablations:

```
metropolis_sweep  --theory-explains--> exploration IF density>=0.05
    ablation: full -9240.33, ablated -8496.00, degradation +8.1%  UPHELD  27/30
gibbs_color_sweep --theory-explains--> exploration IF density>=0.05
    ablation: full -9180.67, ablated -8496.00, degradation +7.5%  UPHELD  26/30
```

Two operators, the same capability, the same condition, near-identical
degradations, both "genuinely good", confidences 0.90 and 0.87. Note that **both
ablate to the same −8496.00**: deletion measures each operator's distance from a
greedy-only schedule, which is nearly the same for both, so it cannot separate
them even in principle.

Substitution can, and does. At equal sweeps and equal draws:

> **`metropolis_sweep` beats `gibbs_color_sweep` on every G-Set instance tested,
> in both initialisation arms, in both seed sets — by 0.17% to 2.41% of the
> energy scale, at ρ = 1.6–9.6 against the seed-variance null.**

**The finding is not "Metropolis is better" — that direction is LIKELY KNOWN
(§4). The finding is that deletion-based ablation reported the two as
equivalently causal while substitution separates them cleanly.** `I_delete` answers "would the
schedule collapse without this?"; `I_replace` answers "should I use this one or
that one here?" — and only the second is the question a scheduler faces.

---

## 2. Result

`I_replace_work = Y(gibbs) − Y(metropolis)` at equal sweeps and equal draws;
positive ⇒ the substitution was worse ⇒ Metropolis wins. 8 seeds per arm, exact
two-sided sign-flip test over all 2⁸ = 256 assignments (p floor = 0.0078).

| Instance | n | density | arm | I (held-in) | ρ | rel | p | I (held-out) | ρ | ratio | replicates |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **G22** | 2000 | 0.010 | legacy | **+37.88** | 1.74 | 0.29% | 0.008 | **+53.38** | 4.57 | 1.41 | **yes** |
| G22 | 2000 | 0.010 | diverse | +50.13 | 2.77 | 0.38% | 0.008 | +48.75 | 3.15 | 0.97 | yes |
| G1 | 800 | 0.060 | legacy | +30.88 | 2.70 | 0.27% | 0.008 | +23.63 | 3.66 | 0.77 | yes |
| G1 | 800 | 0.060 | diverse | +20.13 | 1.62 | 0.17% | 0.016 | +24.38 | 1.89 | 1.21 | yes |
| G11 | 800 | 0.005 | legacy | +4.75 | 4.44 | 0.87% | 0.016 | +5.75 | 3.86 | 1.21 | yes |
| G11 | 800 | 0.005 | diverse | +13.00 | 7.10 | 2.36% | 0.008 | +11.00 | 7.28 | 0.85 | yes |
| G32 | 2000 | 0.002 | legacy | +17.75 | 6.81 | 1.31% | 0.008 | +22.50 | 7.25 | 1.27 | yes |
| G32 | 2000 | 0.002 | diverse | +30.75 | 9.59 | 2.25% | 0.008 | +33.00 | 5.64 | 1.07 | yes |
| G43 | 1000 | 0.020 | legacy | +28.13 | 1.85 | 0.43% | 0.016 | +15.25 | 1.84 | 0.54 | yes |
| G43 | 1000 | 0.020 | diverse | +31.00 | 3.40 | 0.47% | 0.008 | +30.25 | 1.74 | 0.98 | yes |
| `be100.1` | 100 | 0.991 | legacy | **−48.38** | DEGEN | 0.48% | 0.008 | **−47.00** | DEGEN | 0.97 | **no** |
| `be100.1` | 100 | 0.991 | diverse | −19.50 | 0.31 | 0.18% | 1.000 | −65.00 | 0.46 | 3.33 | **no** |

**Ten of ten G-Set contrasts replicate** under the amendment's numeric rule (A6:
matching sign, held-out independently material, ratio in [0.5, 2.0]). Primary
named contrast G22/legacy: material and significant at α = 0.05 unadjusted; the
other eleven reject under Benjamini–Hochberg at FDR 0.10, except
`be100.1/diverse`.

**Status: CONFIRMED** for H-14's first clause (the substitution is worth more
than re-seeding), **NOT ESTABLISHED** for its second clause (regime-dependent
sign) — see §5.

---

## 3. Gate A, condition by condition

| # | Condition | Result |
|---|---|---|
| 1 | Synthetic arithmetic **and** synthetic operator positives pass | **PASS.** Arithmetic: effects `[1,2,3,4]`, mean 2.5, `d_seed = √(500/3)` to 1e-12; a constant-`full` table sets `DEGENERATE_NULL` instead of dividing by zero. Operator: an independent reference written against `ProblemIR` alone reproduces both production operators **exactly on 648/648 fixtures**; accepted non-vacuous fixture `n=12, seed=1, sweeps=1, replicas=2`, `I = +1.0`. |
| 2 | Real deletion positive inside its fresh CI | **PASS with a recorded discrepancy** — see §3.1. |
| 3 | Null + both inert exact; alignment probe matches; injected shift detected | **PASS.** Null bit-identical; `I(sweeps=0)` exactly `0.0`; inert `replica_exchange` deletion leaves state and canonical energy identical. Alignment: `0x51554038a6a5245b` from both generators; the one-`u32` shift arm gives `0x8c452cd251554038`, whose low half is the aligned value's high half — a one-word offset, **detected**. |
| 4 | Primary contrast material and significant | **PASS.** G22/legacy: ρ = 1.737 ≥ 0.5, rel = 0.288% ≥ 0.1%, p = 0.0078 < 0.05. |
| 5 | Replication on held-out seeds (A6) | **PASS.** sign matches; held-out ρ = 4.567 and rel = 0.405% independently material; ratio 53.375/37.875 = **1.41** ∈ [0.5, 2.0]. |
| 6 | Descendant written before held-out was run | **PASS,** verifiable: commit `b2cefc2` (descendant) precedes the held-out execution. |

### 3.1 The real-positive discrepancy, recorded rather than smoothed

Fresh deletion of `metropolis_sweep` on G22, current code, 8 seeds:
**+2.5223%**, bootstrap CI95 **[+2.4299%, +2.6377%]** — a positive degradation
whose CI excludes zero, so the Runtime pipeline reproduces the Theory Engine
protocol qualitatively.

But the historical recorded figure for that band is **2.1%**, and the fresh CI
**does not contain it**. This was foreseen: Phase 0 §0.2 established that
`ExperimentDb` records no code version and no seed list, so exact historical
reproduction was never available and the pre-registration adopted
self-consistency as the fallback (§7). The registry has grown 14 → 18 operators
and the executor has changed since those rows were written. **The control passes
as a pipeline check; it is not evidence of historical identity, and is not
reported as such.**

---

## 4. What is KNOWN here, and what is not

Stated before the confirmation was sought (`PREREG_RC014.md` §12, committed
2026-08-19 before the held-out run).

**LIKELY KNOWN — the direction.** *(Corrected by Amendment 2 A2.1; this section
originally said KNOWN and cited Peskun ordering as predicting the result. It does
not.)* Metropolis accepts every `ΔE ≤ 0` move with probability 1; heat-bath
accepts the same move with `1/(1+exp(−|h|/T)) < 1`. Peskun ordering (Peskun 1973;
Tierney 1998) bounds the asymptotic variance of ergodic averages for *reversible*
kernels sharing a *common stationary distribution*. What is measured here is a
**finite-budget best-of-32-replicas** energy after `greedy_descent`, on a ladder
spanning 4.0 → 0.1 — not an ergodic average, not at stationarity, and not the
functional Peskun bounds. The theorem is therefore **consistent with** the
direction and explains the local acceptance advantage, but does **not** predict
this observable. On `RESEARCH_GAPS.md` §5c's four-level scale the direction is
**LIKELY KNOWN**: finite-budget Metropolis-vs-heat-bath comparisons are common in
the annealing literature, but we have not located the exact citation and no
theorem we can name covers `Y`. The cycle's stated contribution is unchanged and
remains the methodological one below.

**NOT KNOWN, and the actual deliverable — the method.** That the platform's own
ablation engine cannot distinguish two operators it separately certifies as
causal, and that an equal-work substitution does, is a statement about the
instrument rather than about Ising. It is the concrete form of the argument that
`I_delete` carries a budget confound (`--pt-ab --fair`'s lesson at operator
granularity) and it is directly actionable: `theory.rs`'s ablation is a deletion.

**Also new here, and small:** the magnitude at a short 16-sweep budget
(0.17–2.41%), which is above the project's 1% materiality bar on the two
toroidal instances (G11, G32) and below it on the denser ones.

---

## 5. The sign reversal: observed twice, NOT a result

`be100.1` reverses the sign in both arms and both seed sets (−48.4, −19.5,
−47.0, −65.0 — Gibbs better). It is nonetheless recorded as an **observation**,
for three reasons fixed in advance:

1. **It fails this pre-registration's own materiality bar.** `legacy` is
   `DEGENERATE_NULL` — the Metropolis arm has zero seed variance on this
   instance, so ρ is undefined and the row is excluded from the material count by
   the rule written to prevent RC-007's `inf` failure. `diverse` has ρ = 0.31 and
   0.46 (both < 0.5) with p = 1.000 and 0.4375.
2. **It fails the A6 replication rule.** `legacy` is degenerate in both arms;
   `diverse` swings 3.33× between seed sets, outside [0.5, 2.0].
3. **It is confounded on two axes at once.** `be100.1` is both **weighted** (145
   distinct weights) and **dense** (0.991 against 0.002–0.06 for every G-Set
   instance). Attributing the reversal to weighting would repeat exactly the
   error that killed the Easy-Information Law: a density effect in another
   costume (`CHANNEL_EXHAUSTION.md` §7.2).

**D-14, pre-registered:** the sign is governed by **density, not weighting** —
an unweighted instance at density ≈ 0.99 must give `I < 0`, and a weighted
instance at G-Set-like density must give `I > 0`. Refuted if the signs follow
weighting instead, or if neither reaches materiality.

---

## 6. Scope — what this cycle does NOT license

- **One substitution pair, one schedule.** `metropolis_sweep ↔ gibbs_color_sweep`
  under `[X, greedy_descent]`. Exactness requires two conditions (Phase 0 §0.1):
  the pair is draw-identical, **and** no downstream operator's draw count depends
  on state. A second pair, a stochastic downstream operator, or deletion instead
  of substitution is blocked by **ADR-0009**.
- **No general counterfactual instrument** was built. The Runtime still threads
  one `ChaCha8Rng` (`runtime.rs:196`, `:296`).
- **Cache-resident regime only** — at G-Set scale the whole ledger is ~0.5 MB
  (RC-013); nothing here transfers to n ≥ 10⁵.
- **Pairs, not schedules.** Longer compositions untested (RC-007's limitation
  inherited).
- **16 sweeps.** A budget-dependence claim needs the sweep sweep, not this run.
- The production solver and the default initialisation are untouched.

---

## 7. Reproduction

```bash
cargo run --release --bin exp_counterfactual -- --controls   # Gate A controls
cargo run --release --bin exp_counterfactual -- --science    # held-in, 8 seeds
cargo run --release --bin exp_counterfactual -- --holdout    # held-out, 8 seeds
```

Expected: alignment probe EQUAL + shift DETECTED; 648/648 operator agreement;
G22/legacy `I=+37.875, ρ=1.737, p=0.0078` held-in and `+53.375, ρ=4.567` held-out.

---

## 8. Follow-ups this cycle generated

1. **`theory.rs` ablation is a deletion.** Adding a substitution arm would let the
   Theory Engine distinguish operators it currently certifies as equivalently
   causal. Trajectory-changing (it changes which facts are published) ⇒ an open
   decision under ADR-0004, not applied here.
2. **D-14** — the density-vs-weighting disentanglement, instances fixed.
3. **Secondary mechanism test** (non-gating, sharpened by Amendment 2 A2.2): the
   advantage must shrink as `T → 0` **provided the mass of `ΔE = 0` proposals is
   small** — at zero local field the kernels differ by a constant (Metropolis
   accepts, heat-bath flips w.p. ½) that cooling never closes. The tie fraction is
   a required co-observable; a flat or growing advantage at low `T` is
   uninterpretable without it. Flat ladders at `T ∈ {2.0, 0.5, 0.1}`.
4. **`be100.1` produces zero seed variance under Metropolis.** Worth its own look:
   a degenerate null on a dense n=100 instance suggests saturation, which would
   make this instance a poor discriminator for any operator comparison.

---

## 9. D-14 — REFUTED, and the refutation withdraws the `be100.1` observation

Run exactly as pre-registered (`PREREG_RC014.md` §12, committed before the
held-out arm), instances resolved per Amendment 2 A2.3. Held-in seeds, both arms.

| Arm | Instance | density | I | ρ | rel | p |
|---|---|---|---|---|---|---|
| **D-14a** unweighted, dense | synthetic J∈{±1}, n=100 | 0.9905 | **+0.0000** | **DEGEN** | 0.000% | 1.000 |
| D-14a | (diverse init) | 0.9905 | **+0.2500** | **DEGEN** | 0.062% | 1.000 |
| **D-14b** weighted, sparsest | `gka8a` | 0.0614 | **+71.75** | 0.354 | 1.624% | 1.000 |
| D-14b | (diverse init) | 0.0614 | **+33.38** | 0.547 | 0.745% | 0.375 |

**Predicted:** D-14a `I < 0` and D-14b `I > 0` if the sign follows **density**;
the reverse if it follows **weighting**.

**Observed:** D-14a is **zero within noise and degenerate**; D-14b is **positive**
but not significant (p = 1.000 / 0.375) and marginal on materiality (ρ = 0.354
fails, 0.547 passes).

**Verdict — REFUTED** on the pre-registered clause: the refutation condition
"`D-14a` positive" fires (+0.25, diverse arm). Stated without lawyering: **the two
arms point in opposite directions.** D-14b supports density; D-14a, such as it is,
leans weighting — and is degenerate. Neither hypothesis survives.

### 9.1 The mechanism of the refutation, and what it costs

Every cell of D-14a is `DEGENERATE_NULL` — the Metropolis arm returns the **same
energy for all eight seeds**. So is every `be100.1` cell, in the Gate-A run and at
all three flat temperatures (§10). The common factor is not weighting: it is
**dense, n = 100**. At density ≈ 0.99 with 32 replicas, 16 sweeps and a greedy
finisher, the search **saturates** — the outcome stops depending on the seed, the
null scale `d_seed` collapses to zero, and `ρ` becomes undefined.

**Consequence, and it is the point of running D-14:** the `be100.1` sign reversal
that motivated this descendant was measured **against a zero-variance arm**. It is
therefore **withdrawn** — not held open as an observation, but recorded as
**unmeasurable with this design**. The dense n = 100 corner is outside the
instrument's discriminating range, and the correct response is to say so rather
than to keep an unexplained reversal in reserve.

This also answers RC-014 follow-up 4 ("`be100.1` produces zero seed variance under
Metropolis — worth its own look") in the affirmative and closes it: it is
saturation, it generalises to any dense n = 100 instance weighted or not, and it
disqualifies the instance as a discriminator.

### 9.2 Correction to Amendment 2 A2.3 — my density figures were header-based

A2.3 reported BiqMac densities computed from each file's header edge count `m`.
The frontend accumulates duplicate pairs, so the **loaded** instance has fewer
distinct pairs and a lower density: `gka8a` is **403 header lines → 304 pairs,
density 0.0614, not 0.0814**. Re-measured over all 125 instances on loaded pair
counts, every header-based density in A2.3 was high by 20–35%.

**A2.3's conclusion is unaffected:** still **zero** weighted BiqMac instances at
loaded density ≤ 0.06, and `gka8a` is still the sparsest. But D-14b's instance is
much closer to the pre-registered `≤ 0.06` clause than the amendment claimed —
0.0614 against a 0.06 bar — so the recorded deviation is marginal rather than
substantial. Corrected here rather than left standing.

---

## 10. Temperature curve — the sharpened prediction, and one anomaly it does not cover

Non-gating (Amendment 2 A2.2). Flat ladders, legacy arm, held-in seeds, with the
`ΔE = 0` tie mass as the required co-observable.

| Instance | density | T=2.0 | T=0.5 | T=0.1 | tie mass 2.0 → 0.1 |
|---|---|---|---|---|---|
| G22 | 0.010 | +43.88 (p .008) | +32.75 (p .023) | **+1.88** (p .711) | 5.2% → 2.6% |
| G1 | 0.060 | +24.00 (p .016) | +35.25 (p .008) | **+6.50** (p .156) | 2.5% → 1.5% |
| G11 | 0.005 | +3.25 (p .391) | +4.50 (p .055) | +1.75 (p .125) | **33.5% → 13.1%** |
| G32 | 0.002 | −3.25 (p .648) | +29.50 (p .008) | +12.00 (p .008) | **33.1% → 13.1%** |
| G43 | 0.020 | +12.50 (p .172) | +15.88 (p .016) | **−44.13** (ρ 5.31, p .008) | 5.2% → 2.3% |
| `be100.1` | 0.991 | −67.75 (DEGEN) | −26.63 (DEGEN) | **0.00** (DEGEN) | 0.004% → 0.000% |

**Tie mass: the A2.2 exploratory prediction is CONFIRMED.** The two toroidal ±1
lattices carry **13–34%** zero-field proposals against **0–5%** everywhere else —
an order of magnitude — exactly as predicted from their degree-4 integer fields.
`be100.1`, with 145 distinct weights, has essentially **no** ties (0.004%).
Reported as hypothesis-generating at n = 6, never as a test.

**The `T → 0` prediction is broadly supported where tie mass is low.** G22 falls
+43.9 → +1.9 and loses significance; G1 falls to +6.5; `be100.1` falls to exactly
0. All three have tie mass ≤ 5%.

**And one anomaly no hypothesis on the table predicts.** **G43 reverses sign at
T = 0.1 — I = −44.13, ρ = 5.31, p = 0.0078** — material, significant, and
replicating across all eight seeds. Its tie mass is **2.3%**, so the tie mechanism
cannot explain it, and the downhill-acceptance gap predicts the advantage should
*shrink toward zero*, not invert. Gibbs is decisively better than Metropolis on
G43 at low temperature.

This is the sharpest open question the cycle produced, it is **not** pre-registered,
and it is recorded as an anomaly rather than a finding. The obvious next
pre-registration is whether the cold reversal is specific to G43 or appears across
a matched density band — which is precisely what Amendment 2 A2.4's matched set
should be pointed at, in preference to re-litigating density-vs-weighting.

---

## 11. Where RC-014 stands after the descendant

| Claim | Status |
|---|---|
| The instrument works and is validated | **Stands.** 648/648 reference agreement; alignment measured; all controls pass. |
| Substitution separates operators deletion cannot | **Stands.** 10/10 G-Set contrasts replicate on the pre-registered ladder. |
| Direction (Metropolis > Gibbs on the 4.0→0.1 ladder) | **Stands, LIKELY KNOWN** (Amendment 2 A2.1). |
| Regime-dependent sign (H-14 clause 2) | **REFUTED via D-14.** |
| The `be100.1` reversal | **WITHDRAWN** — measured against a saturated, zero-variance arm. |
| The `T → 0` mechanism | **Supported where tie mass is low; contradicted on G43.** |
| Gate B | **Not reached.** Its condition 1 (descendant confirmed) failed. |

**Gate B is not passed, and nothing is being widened to rescue it.** Per the plan,
a Gate-A pass with a Gate-B failure means the instrument is useful and the specific
scientific conclusion is refuted — which is exactly what happened. The instrument
stays; `theory.rs` semantics stay unchanged; no new fact type is published.
