# PREREG — RC-014: causal horizon value of operator substitution

**Pre-registration under Constitution §13.** Written **before** any instrument
code exists. Everything below is binding: thresholds, seed lists, instances,
controls and both gates are fixed here and may not be renegotiated after seeing
results. A missed threshold is a KILL or ARCHIVE, not a revision.

**Prerequisite:** `RC014_PHASE0_AUDIT.md` (Phase 0, PASS-restricted).
**Plan:** `RC014_PLAN_CAUSAL_INSTRUMENT.md`.

**Amended before instrument code:** `PREREG_RC014_AMENDMENT_1.md`. Where the
amendment and this file differ, the amendment controls. The original text is
left intact so the correction is visible in git history rather than silently
rewritten.

Commit this file before the first line of instrument code. Its position in git
history is the evidence that the hypothesis preceded the data.

---

## 1. Hypothesis

**H-14.** At matched budget and preserved randomness, substituting one thermal
exploration operator for another changes the **horizon value** of a run by a
material amount, and the sign of that change depends on the instance's structural
regime.

**H-14-null.** The substitution is worth no more than re-seeding
(`ρ_I = |I| / d_seed < 0.5`) on every tested contrast.

---

## 2. Primary estimand

```
I_replace(k)  =  Y(P[k ← B]) − Y(P)
```

`Y` = best canonical energy at the end of the run (lower is better), so
`I_replace > 0` means the substitution was **worse**.

**Why substitution and not deletion.** Deletion conflates the operator's
contribution with one step less of budget — the extra-sweeps confound that
`--pt-ab --fair` was built to expose. Phase 0 §0.7 shows the existing
deletion-based ablation returning +100% on rows where the ablated schedule simply
contained no optimiser left. `I_delete` is recorded as a **secondary diagnostic
only**.

## 3. The substitution pair, and why it is exact

`metropolis_sweep` ↔ `gibbs_color_sweep`, at equal `sweeps`.

Both consume exactly `sweeps · n · r` draws of type `f64`
(`metropolis_sweep.rs:119` keeps the stream aligned with an explicit discarded
draw; `gibbs_color_sweep.rs:152` draws unconditionally). `rand 0.8.5` gives `f64`
the same two-word width in both. Therefore the generator state **after** the
substituted step is identical in both arms, and every downstream draw is
preserved — an exact counterfactual under the current single-stream Runtime, with
**no change to the read-only core**.

This is the only such pair available (Phase 0 §0.1 table). `random_flip_sweep`
draws `bool` (one word) and is excluded. All content-dependent operators are
excluded.

**Exactness needs a second condition, and the schedule is chosen to satisfy it.**
Draw-identity preserves the stream *position*, but the substitution changes the
*state* — so any later operator whose draw count depends on state would consume a
different number of draws and divergence would resume. Here the only downstream
operator is `greedy_descent`, which takes `_rng` and draws nothing, so the
condition holds trivially. Introducing a data-dependent operator after the
substitution point voids the exactness claim and requires ADR-0009.

**Machine-verified alignment, not assumed.** The harness records a per-step draw
counter and a generator-state fingerprint. Any arm whose post-step fingerprint
differs from its pair is **discarded, not analysed**.

## 4. Fixed experimental design

| Parameter | Value |
|---|---|
| Schedule | `[X, greedy_descent]`, `X ∈ {metropolis_sweep, gibbs_color_sweep}` |
| Sweeps | `[16, 16]` (matches `orchestrator.rs:591–596`, so the positive control shares the protocol) |
| Ladder | `temp_hi = 4.0`, `temp_lo = 0.1` (geometric across replicas) |
| Replicas | 32 |
| Backend | as `DecisionEngine::analyze(ir).select_backend()` decides; recorded, never overridden |
| Held-in seeds | 1001–1008 |
| Held-out seeds | 2001–2008 (**not inspected until Gate A conditions 1–4 are evaluated**) |

**Instances (6).** Canonical corpus is `platform_gset` (Phase 0 §0.3), plus one
weighted bridge:

| Instance | Band | Role |
|---|---|---|
| `gset/G22` | `density>=0.05` | **primary named contrast** — the band of the well-powered control |
| `gset/G1` | `density>=0.05` | replication in band |
| `gset/G11` | `density<0.05` | toroidal ±1 |
| `gset/G32` | `density<0.05` | toroidal ±1 |
| `gset/G43` | `density<0.05` | sparse |
| `biqmac/be100.1.sparse` | weighted | 145 distinct weights — the only family where the guide axis is identifiable (RC-004 typing rule 7) |

**Initialisation factorial** (RC-002; the prefix is identical in both arms, so
alignment is preserved):

| Arm | Endpoint |
|---|---|
| `legacy` — all-zeros | **compatibility** endpoint; comparable to the historical corpus |
| `diverse` — one `random_flip_sweep(1)` prepended | **reachability** endpoint |

**Binding rule:** conclusions may not be transferred between arms without
reporting the interaction `I_{operator × initial diversity}` explicitly. Changing
the production/default initialisation is **out of scope**.

**Temperature scope.** The ladder spans both sides of the RC-008 mixing crossover
(0.1 / 0.5), so every run contains cold non-mixing replicas by construction. No
flat-ladder claim is made in this cycle.

## 5. Contrast set and multiplicity

Declared set: **12 contrasts** = 6 instances × 2 initialisation arms.

- **Primary, named in advance:** `metropolis_sweep → gibbs_color_sweep` on
  `G22`, `legacy` init.
- **Secondary (11):** Benjamini–Hochberg at FDR 0.10 across the declared 12.
  An effect counts only if it survives BH.

No contrast may be added to this set after data are seen.

## 6. Thresholds

`d_seed` = standard deviation of `Y` across the 8 held-in seeds within an arm
(the scale-free null of RC-007).

An effect is **material** iff **both**:

1. `ρ_I = |I_replace| / d_seed ≥ 0.5`; and
2. `|I_replace| / |Y| ≥ 0.001` (0.1% relative).

An effect is **significant** iff a paired randomisation test over the 8 held-in
seeds gives `p < 0.05` **after BH** (§5).

**Degenerate-null flag (RC-007's `inf` failure).** If `d_seed < 10⁻⁹`, `ρ_I` is
not computed; the row is flagged `DEGENERATE_NULL` and excluded from the material
count. It is reported, never silently dropped.

**Equivalence** is claimed only from a bootstrap 95% CI inside ±0.1%, never from
`p > 0.05`.

**Timing** (cost only, not the estimand): conditions interleaved within each
repetition, medians taken (~9% host drift, `PERF.md`). The declared `cost_model`
is **forbidden** as a cost measure (RC-005: it hard-codes φ ≡ 1/3 and overcharges
cold regimes up to 2.1×). Budget matching here is exact by construction — equal
`sweeps` of equal-draw operators — so no calibration table is needed for the
primary estimand; measured wall-cost is recorded as an observable, not used to
adjust budgets. **No online budget adjustment of any kind.**

## 7. Controls — all four are Gate-A blocking

| Control | Requirement | Fails ⇒ |
|---|---|---|
| **Null** | `metropolis_sweep → metropolis_sweep` is **bit-identical**: same `Y`, same best state, same per-step draw counter and generator fingerprint. | harness bug |
| **Inert (a)** | Substitution at `sweeps = 0`: both arms consume 0 draws and change nothing; `I ≡ 0` exactly. | plumbing bug |
| **Inert (b)** | Substitution at a state that is a fixed point of both operators (post-`greedy_descent` local optimum, 0 further draws): `I ≡ 0` exactly. | attribution bug |
| **Synthetic positive** | An instance small enough to enumerate all `2ⁿ` states (`n ≤ 20`): both arms' outcomes are checked against brute force, and the attribution arithmetic is validated against the exact value. | arithmetic bug |
| **Real positive** | Re-run the Theory Engine protocol (`[metropolis_sweep, greedy_descent]`, sweeps `[16,16]`, 4.0→0.1) on `gset` `density>=0.05` with **current** code; establish a bootstrap CI for the deletion degradation; the instrument must reproduce that paired effect inside the CI. | instrument invalid |
| **RNG-shift detection** | A deliberate one-draw offset injected into the counterfactual arm must be **detected** by the fingerprint check, not absorbed. | alignment unverifiable |

The historical "≈5%" is **not** used as a target: Phase 0 §0.7 shows the recorded
distribution spans 0.6% (REFUTED) to 100% (schedule emptied). The control is
self-consistency against a freshly established CI, and this is recorded as a
weakening relative to a true historical control.

**Why the positive controls are blocking.** A set of only-zero checks is passed
perfectly by an instrument that reports ≈0 for everything — RC-012's exact failure
(a model predicting ≡0 at every step passed its module test), and the reason
RC-009 carries an explicit non-vacuity guard.

## 8. Recorded data unit

```
(S_t, O_t, B_t, U_t) → (S_{t+1}, C_t, Y_{t:H})
```

`S` = the frozen `S_1` sensor set (Phase 0 §0.8) · `O` = operator · `B` = budget ·
`U` = (draw counter, generator fingerprint) · `C` = **measured** wall-cost ·
`Y_{t:H}` = best energy at horizons `H ∈ {1, 2, end}` steps ahead.

Captured by a `RunController` implementation that always returns
`RunControl::Continue` — read-only, above the core, bit-identical by construction
(Phase 0 §0.6).

Recording horizon values, not only instantaneous ΔE, is mandatory: an
instantaneous-improvement target systematically under-values exploration and would
re-learn a myopic policy, discarding reachability.

## 9. Gate A — end of Phase 1 (binding)

All of:

1. Synthetic positive control recovered to analytic precision.
2. Real positive control reproduced inside its freshly established CI.
3. Null control bit-identical; both inert controls exactly zero; the RNG-shift
   test **detects** the injected offset.
4. The **primary named contrast** (G22, legacy) is material (§6) **and**
   significant, **or** at least one secondary contrast survives BH at FDR 0.10
   while being material.
5. Sign and magnitude replicate on held-out seeds 2001–2008.
6. The next falsifiable descendant was written into §12 of this file **before**
   the held-out seeds were run.

**Fail ⇒ STOP.** Do not build the Foundry, the DSL search, or the general
addressable-RNG ADR "to strengthen the signal". Diagnose first: instrument,
sensors, or absent causal structure.

## 10. Gate B — next cycle

1. The descendant is confirmed on new instances or a new structural regime;
2. the stated alternative explanation was tested and failed;
3. the effect survives at budget-matched substitution on the new instances.

**Gate A pass + Gate B fail ⇒ the instrument is validated and the specific
conclusion is refuted.** That is not grounds to remove the instrument.

## 11. Pre-declared alternative explanations

To be tested at Gate B, listed now so they cannot be invented afterwards:

- **A1 — regime, not operator.** The effect tracks the density band rather than
  the substitution; predicts the sign flips with the band and vanishes within a
  band.
- **A2 — initialisation artifact.** The effect exists only in the `legacy` arm and
  is an artifact of the all-zeros ensemble collapse (RC-002 / RC-008); predicts
  a large `I_{operator × initial diversity}`.
- **A3 — acceptance identity.** At these temperatures Gibbs and Metropolis
  acceptance probabilities are numerically close, so any difference is
  arithmetic noise; predicts `ρ_I < 0.5` everywhere and is the null.
- **A4 — the quench dominates.** `greedy_descent` erases the difference the first
  operator made; predicts `I_replace` shrinks toward 0 as the quench budget grows.

## 12. Falsifiable descendant

> **To be written before the held-out seeds are run. Left empty deliberately.
> An entry added after held-out results are seen invalidates Gate A condition 6.**

---

## 13. Scope limits, stated in advance

- **One substitution pair.** The single exactly draw-aligned pair the current
  Runtime permits. No claim is made about operators outside it.
- **No general counterfactual instrument.** Addressable randomness
  (`draw_kind` / `draw_index` / plan-defined logical slots) requires changing the
  Runtime contract and is deferred to an ADR (Phase 0 §0.1).
- **Cache-resident regime only.** At G-Set scale the whole ledger is ~0.5 MB
  (RC-013); nothing here transfers to `n ≥ 10⁵`.
- **Pairs, not schedules.** No claim about longer compositions; targeted triples
  are Phase 2.
- **The production solver is untouched**, as is the default initialisation.
