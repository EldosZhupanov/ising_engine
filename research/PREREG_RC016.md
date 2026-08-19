# PREREG — RC-016: does the first-slot operator choice carry per-instance information?

**Pre-registration under Constitution §13.** Written and committed **before any
RC-016 instrument code exists and before any science datum is collected.**
Thresholds, corpus, seeds, arms, verdicts, kill criteria and the power rule are
binding and may not be renegotiated after seeing results. A missed threshold is a
KILL, not a revision.

**Lineage.** RC-014 (`RC014_COUNTERFACTUAL_SUBSTITUTION.md`, Gate A passed) built
and validated the exact-substitution instrument; RC-015
(`RC015_TIE_HANDLING.md`) localised the cold Metropolis/heat-bath difference to
tie handling. This cycle asks the question those two left open, after an
independent cross-examination narrowed it substantially.

**Scope of the narrowing.** Four objections were conceded outright and are
reflected below: the unit of analysis (§4), the licensed conclusion (§3), the
sufficiency target `S₀` rather than `S₁` (§3.2), and the equal-cost/equal-work
separation (§8). Two more changed the design: the verdict labels (§6) and the
status of the primary functional (§7).

---

## 1. Hypotheses

**H-16.** At the deployed ladder, the sign of `I_replace_work` for the ordered
pair (`metropolis_sweep`, `gibbs_color_sweep`) in the **first slot** varies across
G-Set instances.

**H-16-null.** The sign is the same on every instance where the effect is
material.

**Why this is not already answered.** RC-014 measured 5 G-Set instances at the
deployed ladder and found all positive. Those 5 span only **two structural
families** (random all-`+1`: G1, G22, G43; toroidal ±1: G11, G32), and 5
same-sign instances against a fair-coin null give an exact binomial
`p = 2·(1/2)⁵ = 0.0625` — **not significant two-sided**. Sign constancy is
therefore the hypothesis under test, **not** an established premise.

## 2. Primary estimand

```
I_replace_work(instance) = Y(gibbs_color_sweep) − Y(metropolis_sweep)
```

substituted in the **first slot** of `[X@16, greedy_descent@16]`; positive means
the substitution was worse. Equal sweeps, equal draws.

**Exactness holds under the two RC-014 Phase-0 conditions, both source-verified:**

1. **Draw-identical pair.** Both operators build the same colour order over all
   `n` sites and take exactly one `f64` per `(sweep, site, replica)`
   unconditionally — `metropolis_sweep.rs:119` keeps its count data-independent
   with an explicit discarded draw (`let _u: f64 = rng.gen();` in the `d <= 0.0`
   branch); `gibbs_color_sweep.rs:152` draws unconditionally.
2. **No stochastic downstream operator.** The only later operator is
   `greedy_descent`, which takes `_rng` and draws nothing.

The Runtime threads one `ChaCha8Rng` (`runtime.rs:196`, `:296`), so **only** pairs
satisfying both conditions are exact. Anything wider is blocked by **ADR-0009**.

## 3. Exact scope of the licensed conclusion

Quotable verbatim; nothing beyond it may be claimed.

> In the deployed ladder configuration — geometric `4.0 → 0.1`, 32 replicas —
> with the schedule `[X@16, greedy_descent@16]`,
> `X ∈ {metropolis_sweep, gibbs_color_sweep}`, from all-zeros initialisation, on
> unweighted G-Set, under the best-of-32 canonical-energy functional: the sign of
> `I_replace_work` for this **one ordered pair in the first slot** does / does not
> vary across instances.

### 3.1 What is explicitly NOT licensed

- **The other 16 registered operators.** One pair is not the library.
- **Capability selection.** Verified from source: `metropolis_sweep.rs:66-68` and
  `gibbs_color_sweep.rs:86-88` declare **identical** capability sets
  `{Exploration, Exploitation}`. The experiment therefore lives entirely *inside
  one capability* and cannot speak to selection *between* capabilities.
- **The Evolution ordering search**, which searches over *sequences*; ordering was
  addressed by RC-007, whose own stated limitation is "pairs only".
- **The Policy operator head**, which selects among all operators at arbitrary
  trajectory points.
- Multi-slot schedules · other budgets · other ladders · weighted instances ·
  `n ≥ 10⁵` (RC-013: G-Set is entirely cache-resident).

### 3.2 This tests `S₀` sufficiency, not `S₁`

Verified from source at `runtime.rs:522` (`fn quality`). Under legacy all-zeros
init every replica is identical (RC-002), so at `t = 0`, when the **first**
operator would be chosen:

| `S₁` runtime sensor | value at `t = 0` | discriminating across instances? |
|---|---|---|
| `energy_entropy` | energy histogram has **one** bin ⇒ `p = 1` ⇒ `−1·log₂1` = **0** | no |
| `diversity` | `overlap = 1` for every pair ⇒ `(1−1)/2` = **0** | no |
| `acceptance`, `recent_acceptance` | **0** — no step has run | no |
| `iteration`, `frac_elapsed` | **0** | no |
| `temperatures`, `num_replicas` | plan constants, identical across instances | no |
| `best_energy` = `mean_energy` | `E(all-zeros)` = the instance `offset` | a scalar already implicit in the instance |

> **`S₁` collapses to `S₀` plus one scalar at the first-slot decision point.**

`S₀` is the frozen `FeatureRegistry::v0` (`feature_registry.rs:46-72`, marked
FROZEN in source): `log_n = ln(n+1)/10`, `density`, `clustering`,
`mean_degree/10`, `degree_cv` — the shared vocabulary every faculty encodes
against. **A counterexample here is an `S₀` counterexample.** Calling it an `S₁`
result would be false: `S₁`'s extra sensors are not in play.

## 4. Corpus — enumerated before any run

**Unit of analysis is the INSTANCE**, never `(instance, arm)`. The diverse-init
arm is removed from the primary entirely and appears only as a secondary
robustness column (§7).

All 30 locally available G-Set files, listed with `n`, `m`, density, weight set
and a content hash. The hash is recorded because RC-014 Phase 0 §0.2 established
that `ExperimentDb` stores `instance_id` as a **name**, not a hash, so a changed
benchmark file is otherwise undetectable.

| # | instance | n | m | density | weights | sha256[:12] |
|---|---|---|---|---|---|---|
| 1 | `G1` | 800 | 19176 | 0.0600 | +1 | `73bf704d8ffc` |
| 2 | `G2` | 800 | 19176 | 0.0600 | +1 | `732d57480a01` |
| 3 | `G3` | 800 | 19176 | 0.0600 | +1 | `999e49b5e093` |
| 4 | `G11` | 800 | 1600 | 0.0050 | ±1 | `c2a760d2926d` |
| 5 | `G12` | 800 | 1600 | 0.0050 | ±1 | `a8628108d95d` |
| 6 | `G13` | 800 | 1600 | 0.0050 | ±1 | `44af0d3aa232` |
| 7 | `G14` | 800 | 4694 | 0.0147 | +1 | `dc769b978a40` |
| 8 | `G15` | 800 | 4661 | 0.0146 | +1 | `2f1808f074bc` |
| 9 | `G16` | 800 | 4672 | 0.0146 | +1 | `5a70eec4649a` |
| 10 | `G22` | 2000 | 19990 | 0.0100 | +1 | `9baeee06eb14` |
| 11 | `G23` | 2000 | 19990 | 0.0100 | +1 | `3669c719ebbc` |
| 12 | `G24` | 2000 | 19990 | 0.0100 | +1 | `9aff2abd74d1` |
| 13 | `G32` | 2000 | 4000 | 0.0020 | ±1 | `9760fce6b601` |
| 14 | `G33` | 2000 | 4000 | 0.0020 | ±1 | `4791e1bd9ac2` |
| 15 | `G34` | 2000 | 4000 | 0.0020 | ±1 | `e84c77938fcd` |
| 16 | `G35` | 2000 | 11778 | 0.0059 | +1 | `3df35a2abbe8` |
| 17 | `G36` | 2000 | 11766 | 0.0059 | +1 | `022423749f4e` |
| 18 | `G43` | 1000 | 9990 | 0.0200 | +1 | `9af5445b4b06` |
| 19 | `G44` | 1000 | 9990 | 0.0200 | +1 | `929e7687b9a0` |
| 20 | `G45` | 1000 | 9990 | 0.0200 | +1 | `e1514f22a23c` |
| 21 | `G48` | 3000 | 6000 | 0.0013 | +1 | `2c2daba39d1f` |
| 22 | `G49` | 3000 | 6000 | 0.0013 | +1 | `01733a64e25e` |
| 23 | `G50` | 3000 | 6000 | 0.0013 | +1 | `d3f5f31c5089` |
| 24 | `G51` | 1000 | 5909 | 0.0118 | +1 | `23b7111e929f` |
| 25 | `G52` | 1000 | 5916 | 0.0118 | +1 | `48ab066f1a3b` |
| 26 | `G53` | 1000 | 5914 | 0.0118 | +1 | `10ebfc718012` |
| 27 | `G55` | 5000 | 12498 | 0.0010 | +1 | `7537bbb613a6` |
| 28 | `G60` | 7000 | 17148 | 0.0007 | +1 | `b6480c1716ec` |
| 29 | `G63` | 7000 | 41459 | 0.0017 | +1 | `a1d08e1eed7a` |
| 30 | `G70` | 10000 | 9999 | 0.0002 | +1 | `0d965a2ff144` |

**Matched groups — identical `n` AND `m`, so `S₀` is near-constant within a group
by construction.** These are where an `S₀` counterexample would live at
near-zero sensor distance:

| group | n | m | members |
|---|---|---|---|
| A | 800 | 1600 | G11, G12, G13 (toroidal ±1) |
| B | 800 | 19176 | G1, G2, G3 |
| C | 1000 | 9990 | G43, G44, G45 |
| D | 2000 | 4000 | G32, G33, G34 (toroidal ±1) |
| E | 2000 | 19990 | G22, G23, G24 |
| F | 3000 | 6000 | G48, G49, G50 |

**Six matched groups, 18 of 30 instances.** `clustering` and `degree_cv` still
vary slightly within a group; the exact `S₀` distance is computed and reported for
every within-group pair that yields a counterexample.

## 5. Fixed design

| Parameter | Value |
|---|---|
| Schedule | `[X@16, greedy_descent@16]`, `X ∈ {metropolis_sweep, gibbs_color_sweep}` |
| Ladder | geometric `temp_hi = 4.0`, `temp_lo = 0.1` — the deployed configuration |
| Replicas | 32 |
| Initialisation | legacy all-zeros (`executor.rs`, `run_one`) |
| Backend | as `DecisionEngine::analyze(ir).select_backend()` decides; recorded, never overridden |
| **Held-in seeds** | **1001–1008** |
| **Held-out seeds** | **2001–2008** — not inspected until §9's controls and the held-in criteria are evaluated |
| **Pilot seeds (cost calibration only)** | **3001–3008** — disjoint from both, so calibration cannot contaminate either |

## 6. Analysis, verdicts, kill criteria

### 6.1 Per instance

Exact two-sided sign-flip test over all `2⁸ = 256` assignments (p floor
`2/256 = 0.0078`). `d_seed` = sample standard deviation of the reference
(`metropolis`) arm. `DEGENERATE_NULL` if `d_seed < 1e-9` — the flag exists
because RC-007's first pass produced `inf` rows by dividing by a zero null.

**Materiality** = `ρ = |I| / d_seed ≥ 0.5` **AND** `rel = |I| / |Y| ≥ 0.001`.

**Replication (RC-014 rule A6)** = all three of: matching sign; the held-out
effect independently material; `0.5 ≤ |I_out / I_in| ≤ 2.0`. If `I_in = 0` or
either arm is `DEGENERATE_NULL`, replication fails.

**Multiplicity:** Benjamini–Hochberg at **FDR 0.10 across the 30-instance
family**. There is **no primary named instance**: the question is about the
*population*, so naming one instance would answer a different question.

### 6.2 Population statistic

Among instances that are **material AND replicating**, count positive versus
negative signs and test with the **exact two-sided binomial against `p = 0.5`**.
This, not the per-instance tests, is what answers H-16.

### 6.3 Three verdicts — distinct labels, never merged

| verdict | condition | what it means |
|---|---|---|
| **SIGN VARIES** | ≥ 1 material, replicating instance of **each** sign | H-16 confirmed. Report every matched group containing two opposite-signed material members, with the exact `S₀` distance between them — the `S₀` counterexample. |
| **SIGN CONSTANT** | all material, replicating instances share one sign **and** the exact binomial `p < 0.05` (attainable only with ≥ 6 such instances: 6 same-sign gives `p = 0.0312`, 5 gives `p = 0.0625`) | H-16 refuted for this pair/slot/configuration. |
| **PAIR INTERCHANGEABLE** | **no** instance reaches materiality | The sign is *undefined*, not uniform. This **bounds what any selector could gain** for this pair at this configuration. It is **NOT** SIGN CONSTANT and must never be reported as such. |

### 6.4 Kill criteria

1. **Q-INCONCLUSIVE** — fewer than **6** instances are both material and
   replicating. The population binomial cannot reach `p < 0.05` below 6, so the
   sign question is unanswerable. Reported as inconclusive. **No post-hoc
   lowering of the materiality bar.**
2. **BENCHMARK-VALIDITY FINDING** — ≥ **50%** of instances are
   `DEGENERATE_NULL`. The configuration is saturated at this budget and cannot
   discriminate; the sign question is not answered, and this is recorded as a
   third benchmark-validity result after RC-004's two.
3. **INSTRUMENT INVALID** — any control in §9 fails on re-run. Then nothing
   measured is evidence about H-16, in either direction.

## 7. Functional and secondaries

**Primary functional: best-of-32 canonical energy at end of run.** This is
declared as a **choice**, not asserted as the scheduler's target. Two defensible
targets exist and differ: the Decision Engine synthesises plans by a composite
utility `α·q̂ + β·ĉ + γ·m̂ + δ·l̂`, while `server_api` returns the single best
solution, which is what `RunRecord.best_energy` holds and the golden regression
protects. Best-of-32 is chosen because it is the *delivered* quantity and because
every prior cycle used it (comparability).

**Secondaries, pre-declared, computed for every cell, always reported:**
(a) ensemble **mean** energy; (b) best energy **before** `greedy_descent`;
(c) measured wall cost per arm; (d) the cost ratio `R` (§8); (e) the diverse-init
arm.

**Binding rules against dilution:** secondaries are **excluded from the
multiplicity family** — they are the same contrast measured differently, not new
contrasts, and adding them to BH would be double-counting. A secondary may
**qualify** the primary verdict ("holds under best-of-32, reverses under the
ensemble mean") but may **never** replace or re-rank it. If a secondary
disagrees with the primary, the disagreement is the headline and the primary
verdict stands as stated.

## 8. Cost: equal work versus equal measured cost

The tension is **structural, not a gap**. `I_replace_work` is exact *because* the
arms are draw-identical; equalising measured cost requires **unequal sweep
counts** ⇒ unequal draw counts ⇒ the stream desynchronises at the substitution
point ⇒ the comparison **cannot** be an exact counterfactual. Both properties are
unobtainable simultaneously.

### 8.1 Calibration first — it may dissolve the problem

A calibration pass on the **pilot seeds 3001–3008**, conditions **interleaved
within each repetition** and summarised by the **median** (the `PERF.md` host-drift
protocol; this host drifts up to ~9% within a session). Frozen per instance
**before** any science seed runs. Report

```
R(instance) = median_ms(gibbs) / median_ms(metropolis)
```

> **If `R ∈ [0.95, 1.05]`, equal sweeps IS equal cost within tolerance on that
> instance, and the exact estimand already answers the scheduler's question there.
> No cost arm is run for that instance.**

This is plausible rather than wishful: `gibbs_color_sweep` evaluates its
exponential unconditionally, while `metropolis_sweep` evaluates `exp` only on the
uphill branch — and at cold rungs nearly every proposal *is* uphill.

**`cost_model` is FORBIDDEN as the cost measure** (RC-005: shape-only, hard-codes
`φ ≡ 1/3`, overcharges cold regimes up to 2.1×). Only measured wall time, from
the frozen table. **No online budget adjustment of any kind.**

### 8.2 The cost arm is unpaired and non-causal

Run only where `R ∉ [0.95, 1.05]`, with integer sweep counts read from the frozen
table.

| | `I_replace_work` | `I_replace_cost` |
|---|---|---|
| status | **exact counterfactual** | **A/B, NOT a counterfactual** |
| draws | identical | **different** |
| pairing by seed | valid (common random numbers) | **INVALID** — arms share only the prefix and diverge immediately |
| test | exact paired sign-flip over `2⁸` | **unpaired** two-sided Mann–Whitney |
| null scale | paired-difference `d_seed` | **per-arm** sd, computed separately |
| language permitted | causal | comparative only |

**Three binding prohibitions.** (i) The paired sign-flip test is **forbidden** on
the cost arm — applying it there manufactures causality that does not exist.
(ii) The two estimands are **never differenced or ranked against each other**:
the cost arm has no common random numbers, so its variance is structurally
larger and `|I_cost| < |I_work|` is uninterpretable. (iii) No causal verb may
appear in any sentence reporting `I_replace_cost`.

### 8.3 Machine-checkable power rule for the cost arm — fixed before data

No subjective language is permitted; "within noise" is not a verdict. The rule is
a formula over quantities measured in the **pilot**, evaluated **before** the
science run.

**Discreteness check.** With `n₁ = n₂ = 8`, Mann–Whitney has
`C(16,8) = 12870` arrangements, so the minimum attainable two-sided p is
`2/12870 = 1.554e-4`. `α = 0.05` is attainable; discreteness is not a barrier.

**Minimum detectable effect.** For a two-sample test at two-sided `α = 0.05` and
power `1 − β = 0.80`, with per-arm standard deviation `s`, the normal
approximation requires

```
n ≥ 2·(z_0.975 + z_0.80)² · s²/Δ²
  = 2·(1.959964 + 0.841621)² · s²/Δ²
  = 15.6978 · s²/Δ²
```

Mann–Whitney's asymptotic relative efficiency versus the t-test under normality
is `3/π = 0.95493`, so the requirement is inflated by `1/0.95493 = 1.04720`:

```
n_required(instance) = ceil( 16.4387 · s² / Δ² )
```

with `Δ = 0.001 · |Y_ref|` — **the materiality bar itself**, so the cost arm is
required to be powered for exactly the effect size the primary calls material.
`s` is the pooled per-arm sample standard deviation from the pilot seeds
3001–3008 at the calibrated sweep counts, frozen before the science run.

Equivalently, the achieved minimum detectable effect at `n = 8` is

```
MDE(instance) = s · sqrt(16.4387 / 8) = 1.4335 · s
```

**Decision rule, per instance, evaluated before the science run:**

- `MDE(instance) = 1.4335·s ≤ 0.001 · |Y_ref|` ⇒ the cost arm is **POWERED**; run it and
  report its Mann–Whitney result.
- `MDE(instance) = 1.4335·s > 0.001 · |Y_ref|` ⇒ the cost arm is **UNDERPOWERED**; it is
  still run and reported, but its verdict line is fixed in advance to
  `UNDERPOWERED: MDE = <value> exceeds the materiality bar <value>; this arm
  cannot detect a material effect`, and **no conclusion of any kind may be drawn
  from it** — neither presence nor absence of an effect.

Seed counts are **not** increased to chase power: 8 held-in seeds are frozen by
this pre-registration. `n_required` is reported so a future cycle knows what it
would cost.

## 9. Controls

The instrument's controls already passed in RC-014 and RC-015 and are **cited,
not re-derived**: the operator-boundary draw-alignment probe with injected-shift
detection; null substitution bit-identity; both inert controls; the synthetic
arithmetic positive; the synthetic operator positive (648/648 exact agreement
against an independent reference).

**They are nonetheless re-run at the head of RC-016**, because the corpus,
instance sizes and code state differ. Any failure triggers kill criterion 3
(§6.4): the instrument is invalid and nothing is evidence about H-16.

## 10. Out of scope — declared now so it cannot become a post-hoc pivot

**RC-017 — `S₁` sufficiency**, via a **mid-run** substitution
`[metropolis_sweep@16, X@16, greedy_descent@16]`. Both arms share an identical
prefix, so they reach slot 2 at the same stream position and the same state; the
substituted pair is draw-identical; `greedy_descent` draws nothing. **Exactness
holds and no ADR is required.** At slot 2 the `S₁` runtime sensors are
non-degenerate, so that design tests what RC-016 provably cannot (§3.2). It is
**not** part of this cycle.

Also out of scope: any Runtime change; any `theory.rs` change; any new production
operator; any change to the default initialisation; the growth campaign; the
Foundry; the DSL. The five open decisions in `memory/OPEN_PROBLEMS.md` §0 are
untouched by this cycle.

## 11. Facts verified against source for this pre-registration

| Claim | Source | Verified |
|---|---|---|
| Single `ChaCha8Rng` threaded through every operator | `runtime.rs:196`, `:296` | ✅ |
| Metropolis keeps its draw count data-independent | `metropolis_sweep.rs:119` | ✅ |
| Gibbs draws unconditionally, one per (site, replica) | `gibbs_color_sweep.rs:152` | ✅ |
| Both operators declare **identical** capabilities `{Exploration, Exploitation}` | `metropolis_sweep.rs:66-68`, `gibbs_color_sweep.rs:86-88` | ✅ |
| `energy_entropy` and `diversity` are 0 at all-zeros | `runtime.rs:522` (`fn quality`) | ✅ |
| `S₀` = five frozen scalars | `feature_registry.rs:46-72` | ✅ |
| 30 G-Set files, 6 matched `(n, m)` groups, hashes | enumerated §4 | ✅ |
