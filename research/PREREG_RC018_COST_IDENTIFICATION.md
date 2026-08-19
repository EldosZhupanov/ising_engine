# PREREG — RC-018: identification of the two-term cost model, and its efficacy

**Status:** binding pre-registration. Written **before any RC-018 instrument code
exists and before any RC-018 datum — pilot, held-in or held-out — is collected.**
Documentation only; this commit changes no `src`.

**Provenance chronology, which this document is the first link of.** The order is
(1) this file committed; (2) instrument written; (3) controls run; (4) pilot;
(5) held-in; (6) descendant frozen; (7) held-out. Any artifact whose mtime
precedes this file's commit timestamp is not RC-018 evidence and may not be cited
as such. Every artifact is recorded with its SHA-256 in the results document.

---

## 1. What this cycle measures, and what it does not

RC-018 measures **wall time only.** It computes, stores and reports no solution
quality, no energy, no operator comparison on outcome. An operator that is slower
here is not thereby worse; an operator that is faster is not thereby better. The
word "cost" in this document means measured wall-clock milliseconds and nothing
else.

`cost_model` is **forbidden as a measure** of cost (RC-005). It is the object
under test, never the instrument.

### The two questions are separate and are gated separately

RC-005 established that the shipped shape-only `cost_model` is blind to flip
density φ, with a measured error spread of 1.64×–2.09× across four instances. It
did **not** establish that the proposed replacement is correct. RC-018 therefore
asks two questions, in order, and the second is only reached if the first passes:

- **Identification (Gate A).** Is the timing law `T(k) = a + b·k` separable — can
  `a` (the k-independent floor) and `b` (the per-sweep slope) be estimated
  distinctly, with a stable integer `k′`, from timings at ≥3 budgets?
- **Efficacy (Gate B).** Conditional on identification, does
  `W = W_scan + W_flip·min(φ, 1/3)` predict held-out wall time **better than the
  shipped shape-only model**, by a pre-registered margin?

**These are not the same claim and must never be reported as one.** Identification
can pass while efficacy fails: the law can be separable and the proposed formula
still wrong. Efficacy cannot be evaluated at all if identification fails, because
without separable coefficients there is no quantity to compare. A results document
that reports Gate B without a passed Gate A is invalid.

### Why this cycle needs no trajectory-change approval

`cost_model` is a **predictor** consumed by Decision Engine ranking; it is not
read on the Runtime execution path. RC-018 adopts nothing, edits no `cost_model`,
and changes no plan ranking. **Precondition, verified before the instrument is
written, not assumed:** if inspection shows `cost_model` is read on the execution
path, this cycle stops and is re-pre-registered. Adoption of any cost model
remains the maintainer's decision (`memory/OPEN_PROBLEMS.md` §0 #1).

## 2. Frozen design and provenance

| item | frozen value |
|---|---|
| corpus | a frozen 6-instance subset of the hashed 30 in `PREREG_RC016.md` §4: **G1, G11, G14, G22, G32, G43** |
| hash verification | the instrument must verify each file against the `sha12` recorded for it in `PREREG_RC016.md` §4 and refuse on mismatch |
| operators | `metropolis_sweep` **and** `gibbs_color_sweep`, each fitted **separately** |
| budgets `k` | **4, 8, 16, 24** sweeps — four points, so the linear fit has 2 residual degrees of freedom |
| replicas | 32 |
| initialisation | legacy all-zeros |
| ladder | geometric `4.0 -> 0.1` |
| pilot seeds | `7001–7008` |
| held-in seeds | `8001–8008` |
| held-out seeds | `9001–9008` |
| CI bootstrap seed | `20261001 + 2*instance_index + block` |
| order-randomisation seed | `20261101 + repetition_index` |
| repetitions | 9 per `(instance, operator, k, seed)` cell, median taken |
| timing | `Instant::now()` at the call site; the profiler aggregates by operator name and is **not** usable here |

`G1`, `G22` and `G32` carry RC-005-measured φ ranges (0.064–0.196, 0.067–0.259,
0.139–0.432 respectively); **G32 is the only one measured to cross the φ = 1/3
regime boundary.** The φ regime of `G11`, `G14` and `G43` is **unknown before
data** and is recorded, not predicted. No instance may be added, dropped or
substituted after any datum is seen.

### Seed independence — binding

All four RC-018 blocks are disjoint from every prior cycle. The following are
**forbidden** to RC-018 in every mode:

- `1001–1008`, `2001–2008`, `3001–3008` — RC-016;
- `4001–4008` — RC-017 pilot;
- `5001–5008` — **burned** by the RC-017 abort; never reusable;
- `6001–6008` — reserved to RC-017's successor and uninspected.

**No RC-017 datum, artifact, digest or printed summary may be reused, re-analysed
or cited as evidence in RC-018.** RC-017's instrument aborted without a verdict;
its data are not a baseline, not a control and not a prior. The instrument must
hard-refuse any run naming a forbidden seed and exit non-zero.

## 3. Execution order — interleaving and randomisation

Monotone host drift must not be able to manufacture a slope, and block structure
must not be able to hide one.

1. **Interleaved within repetition.** Inside one repetition every
   `(operator, k)` condition is executed once before any condition is executed a
   second time. A drift that is monotone across the repetition therefore lands on
   every condition approximately equally.
2. **Randomised order within repetition.** The order of conditions inside a
   repetition is a deterministic permutation seeded by
   `20261101 + repetition_index`, so the ordering is replayable but is not the
   same in every repetition. This breaks any residual coupling between a
   condition's position and a periodic host effect.
3. **Median over repetitions**, never mean — one descheduled repetition must not
   move the estimate.
4. Both properties are asserted by a control (§4), not merely intended.

## 4. Mandatory controls — positive and null

Every control below must pass **before any pilot datum is analysed**, and the
whole set must be re-run before held-in. Any failure kills the cycle as an invalid
instrument, with no cost claim of any kind.

**Null controls** (must show *no* effect):

- **N1 — same-code null.** The identical operator and configuration timed as two
  distinct labelled conditions. The measured ratio must lie within the host drift
  bar of §5. A null pair that differs is a broken harness.
- **N2 — zero-budget null.** `k = 0` yields the `k`-independent floor only; the
  fitted `b` from a set of zero-budget cells must be statistically
  indistinguishable from zero.
- **N3 — non-invasiveness.** A run's final energy and state digest are
  **bit-identical** with timing instrumentation enabled and disabled. Timing must
  not perturb the thing being timed.

**Positive controls** (must show a *known* effect):

- **P1 — budget positive.** Measured time at `k = 24` exceeds that at `k = 4` by
  a factor whose lower bound is pre-registered as **> 2.0** on every instance. A
  harness that cannot see a 6× budget difference cannot see anything smaller.
- **P2 — synthetic slope recovery.** Against a synthetic workload with a
  *constructed, known* `(a, b)`, the fitting procedure recovers both within 5%.
  This validates the estimator independently of the Runtime.
- **P3 — corpus hash positive.** All six instances match their `PREREG_RC016.md`
  §4 hashes.

## 5. Host, load and drift guards

- **Drift bar.** The recorded same-code drift on this host is **~9%**
  (`PERF.md`). Any effect claimed by RC-018 must exceed it; any control null that
  exceeds it fails.
- **HOST-UNSTABLE flag.** If N1's same-code spread exceeds **9%**, the run is
  flagged `HOST-UNSTABLE` and **aborts before science**. It is not down-weighted,
  not annotated, not proceeded-past.
- **Drift sentinel.** A fixed sentinel condition is executed first and last in
  every repetition. If sentinel median drifts by more than **9%** across the
  session, the session is discarded and re-run; a discarded session is recorded
  in the results document with its reason, never silently dropped.
- **DEGENERATE-TIMING flag.** Any cell whose median duration is below **100×**
  the measured timer resolution is marked `DEGENERATE-TIMING`. Such cells are
  excluded from fitting and reported as excluded with their count. If more than
  **2 of 6** instances are degenerate at `k = 4`, the budget grid is invalid and
  the cycle stops — the grid is **not** silently widened.
- No other process is to be started by the instrument during timing; the
  instrument records host load context alongside each session for the record.

## 6. Estimation — fixed before data

For each `(instance, operator, block)`:

1. Take the **median** over the 9 repetitions for each `(k, seed)` cell.
2. Fit `T(k) = a + b·k` by **ordinary least squares over the four k values**, on
   the per-seed medians. One fit per seed; eight fits per cell.
3. The reported `a`, `b` are the **medians over the eight seeds**.
4. The CI on `b` is the deterministic **percentile 95%** bootstrap over the eight
   per-seed slopes, **100 000** resamples, seeded by the §2 formula. It is
   descriptive and never changes a gate outcome except where §7 names it
   explicitly.
5. `a` and `b` are estimated **separately for `metropolis_sweep` and
   `gibbs_color_sweep`.** A single pooled fit is forbidden: the whole point is
   that the two operators may have different slopes and a common floor.

## 7. Gate A — identification

Gate A passes for an `(instance, operator)` cell iff **all four** hold:

- **A1 — slope non-null.** The 95% CI on `b` excludes zero.
- **A2 — slope precision.** The CI half-width on `b` is **≤ 20%** of `|b|`.
- **A3 — linearity.** The maximum absolute residual of the four-point fit is
  **≤ 10%** of the fitted `T(k_max)`. A larger residual means `T(k)` is not
  affine in `k` on this grid and the two-term decomposition does not describe it.
- **A4 — stable integer `k′`.** With `k_ref = 16`,
  `k′ = round(k_ref · b_M / b_G)` must be **identical across all eight pilot
  seeds** and identical across the bootstrap's central 95%. A `k′` that moves
  with the seed is not an equal-cost budget.

**Gate A passes for the cycle** iff it passes on **≥ 5 of the 6 instances for
both operators.** Otherwise the verdict is `NOT IDENTIFIED`.

`NOT IDENTIFIED` is a real, publishable outcome: it states that whole-policy
timing on this grid cannot separate the `k`-independent floor from the per-sweep
slope, and it therefore **confirms that no equal-cost arm is constructible** by
this route. In that case Gate B is **not evaluated** and no efficacy claim of any
kind is made.

## 8. Gate B — efficacy on held-out

Reached only if Gate A passed.

1. Freeze `W_scan` and `W_flip` from **held-in only**, by the §6 procedure. They
   are written and committed in the descendant document before held-out is
   touched.
2. Predict held-out wall time on `9001–9008` with (i) the frozen proposed model
   and (ii) the shipped shape-only `cost_model`, both **unmodified after seeing
   held-out**.
3. The metric is **median absolute relative error (MdARE)** against measured wall
   time, per instance, pooled over operators and budgets.

**Gate B passes** iff the proposed model's MdARE is lower than the shape-only
model's by **≥ 5 percentage points** on **≥ 5 of the 6 instances**.

**Gate B fails** otherwise, and the verdict is
`EFFICACY NOT ESTABLISHED` — a first-class result meaning the proposed
replacement is not demonstrably better than the model RC-005 refuted. In that
case no adoption is recommended and **no additional term, interaction or
regime split may be added to the formula in this cycle.**

## 9. Kill criteria

Each is a genuine two-sided decision with a fixed threshold, and each can PASS or
FAIL on real data.

| # | Condition | Consequence |
|---|---|---|
| K1 | any control in §4 fails | instrument invalid; **no cost claim**; cycle ends |
| K2 | N1 spread > 9% (`HOST-UNSTABLE`) | abort before science; re-run on a quiet host |
| K3 | > 2 of 6 instances `DEGENERATE-TIMING` at `k = 4` | budget grid invalid; **stop**; do not widen the grid |
| K4 | Gate A fails (§7) | verdict `NOT IDENTIFIED`; Gate B not evaluated; equal-cost arm confirmed unconstructible |
| K5 | Gate A passes, Gate B fails (§8) | verdict `EFFICACY NOT ESTABLISHED`; no adoption; no formula extension |
| K6 | held-out accessed before the descendant is committed | protocol violation; held-out block burned; cycle ends without verdict |

A cycle ending at K4 or K5 is **not** a failed cycle. Both are informative
negative results and are written up with the same weight as a pass.

## 10. Prohibitions

- **No adoption.** RC-018 does not change `cost_model`, any operator, the
  Decision Engine, the scheduler, or plan ranking. **Changing the scheduler or
  the Decision Engine's cost input on the strength of this result requires a
  separate, accepted ADR** — a passed Gate B is evidence for such an ADR, never a
  licence to make the change.
- **No core edits.** `runtime.rs`, `scheduler.rs`, `state.rs`, the backends,
  `registry.rs` and `operator.rs` are read-only (`CLAUDE.md` §3.2). No `unsafe`.
- **No RC-017 reuse**, per §2. No access to `5001–5008` or `6001–6008`.
- **No quality, energy, or outcome claims.** RC-018 measures milliseconds.
- **Equal sweeps is not equal cost**, and equal cost is not established by this
  cycle unless Gate A passes; even then `k′` is a measured budget ratio, not a
  deployment recommendation. No deployment-default, throughput, or
  scheduler-utility claim may be derived from any RC-018 result.
- **No post-hoc tuning.** Coefficients are not refitted after seeing held-out;
  thresholds are not moved; instances, seeds, budgets and repetitions are not
  widened without a committed amendment written before the new data.
- **No ADR-0009 or ADR-0010 work** in this cycle.

## 11. What a pass would and would not license

A passed Gate A licenses exactly: *on this grid, for these six instances and two
operators, whole-policy timing separates into an affine floor and slope, and a
stable integer equal-sweep-ratio exists.*

A passed Gate B licenses exactly: *the two-term flip-density model predicts wall
time on held-out better than the shipped shape-only model by the stated margin.*

Neither licenses a claim that the proposed model is **correct**, that it is the
best available model, that either operator should be preferred, or that the
scheduler should be changed. The last of these requires an ADR.

---

# Amendment 1 — repair of the estimands before instrumentation

**Status:** binding amendment to the pre-registration above. Written **before any
RC-018 instrument code exists and before any RC-018 datum.** Documentation only.

**Reason.** Independent review returned FAIL with seven binding defects. All seven
are verified below and all seven are **accepted**. The root defect is §A7: the
body above froze a single geometric ladder, along which φ varies continuously, so
a whole-ladder timing yields exactly **one** aggregate φ per cell and the φ law is
**not identifiable at all** by the design as written. Every other defect either
follows from that or is an independent estimator error.

## Clauses superseded — and only these

| clause | status |
|---|---|
| §2, the `budgets k` row and the single-ladder assumption | **SUPERSEDED** by §A1 |
| §4, control **P1** | **SUPERSEDED** by §A6 |
| §4, control **N2** | **SUPERSEDED** by §A5 |
| §5, `DEGENERATE-TIMING` (scope) | **EXTENDED** by §A1.4 |
| §6, estimation procedure | **EXTENDED** by §A2 (§6 continues to govern Design L) |
| §7, criterion **A4** | **SUPERSEDED** by §A4 |
| §8, in its entirety | **SUPERSEDED** by §A3 |
| §9, kill criterion **K3** | **EXTENDED** by §A1.4 |

Everything else stands: §1's timing-only scope and the identification/efficacy
separation, §2's corpus, seeds and hash verification, §3's interleaving and
randomisation, §4's N1/N3/P2/P3, §5's drift bar and `HOST-UNSTABLE`, §6's median
and bootstrap conventions, §7's A1–A3 and the ≥5-of-6 rule, §9's K1/K2/K4/K5/K6,
§10's prohibitions, §11's licensing limits.

## A7 — Two designs, named and kept apart

The cycle now has **two disjoint measurement designs**. Which design supplies
which gate is fixed here and may not be crossed.

**Design L — ladder.** The geometric ladder `4.0 -> 0.1`, budgets
`k ∈ {4, 8, 16, 24}`, per operator. Yields whole-policy `T_o(k)`. **Design L is
the sole data source for Gate A.**

**Design T — isothermal.** A **constant** temperature per cell, from the frozen
set

```
T_cell ∈ {0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0}
```

eight cells, per operator. Within one cell the acceptance rate — and therefore
φ — is stationary rather than swept, which is precisely what makes the φ law
identifiable. **Design T is the sole data source for Gate B.**

**Binding consequence, stated because it is easy to violate:** Gate A's `k′` is
established on the ladder and is **not** validated by Gate B, and Gate B's φ law
is established isothermally and is **not** validated by Gate A. Neither transfers
to the other's regime without a new pre-registration. In particular `T_cell = 8.0`
lies **above** the ladder's hot end of `4.0`, so the φ law is fitted over a wider
φ range than the ladder ever traverses; any use of the fitted law to explain
ladder timings is extrapolation and is prohibited in this cycle.

## A1 — Design T, specified

1. **Budgets, two per cell.** Each isothermal cell is timed at
   `k ∈ {4, 12}`. Per-sweep time is the **difference quotient**

   ```
   ms_per_sweep(i, o, T_cell, s) = [ T_meas(12) − T_meas(4) ] / 8
   ```

   Differencing removes the `k`-independent floor **by construction**, so Design T
   never depends on estimating an intercept. This is the same lesson as §A6: an
   intercept that is not modelled must be cancelled, not ignored.

2. **φ, defined and measured.** `φ = accepted flips / (r · n)` per sweep, averaged
   over the eight sweeps in the differenced window `(4, 12]`. Per RC-005, φ **is**
   the acceptance rate; the instrument records it from the operator's own
   acceptance counters.

3. **New control — φ recovery (P4).** Measured φ must agree with the acceptance
   rate independently computed from `StepEvent` to within 1% relative, on every
   pilot cell. Failure means the quantity being modelled is not the quantity being
   measured, and kills the cycle under K1.

4. **`DEGENERATE-TIMING`, extended.** A Design T cell is degenerate when
   `T_meas(12) − T_meas(4)` is below **100×** the measured timer resolution; a
   Design L cell is degenerate as already defined in §5. **K3, extended:** if more
   than **2 of 6** instances are degenerate at `k = 4` (Design L) **or** more than
   **2 of the 8** temperature cells are degenerate on any instance (Design T), the
   corresponding grid is invalid and that design stops. Neither grid may be
   silently widened.

5. **Run count, stated honestly.** Design L is `6 × 2 × 4 × 8 × 9 = 3,456` runs;
   Design T is `6 × 2 × 8 × 2 × 8 × 9 = 13,824` runs; **17,280 short runs total**
   across pilot, held-in and held-out blocks combined. This is recorded so the
   cost of the cycle is visible before it is authorised, not discovered during it.

## A2 — The exact Gate B estimator, with shape normalisation

The shipped shape-only work is

```
S_i = (n_i + 2·m_i) · r
```

with `m_i = num_pairs` and `r = 32`, exactly as `work_per_sweep` computes it.
`S_i` is dimensionless and instance-only; it carries no temperature and no
operator.

**Response and regressor**, per observation `(i, o, T_cell, s)`:

```
y = ms_per_sweep(i, o, T_cell, s) / S_i        [ms per unit shape-work]
x = min( φ(i, o, T_cell, s), 1/3 )             [clipped flip density]
```

**Estimator.** Ordinary least squares of `y` on `x`, **fitted separately for each
operator**, pooled over the held-in observations of that operator:
`6 instances × 8 temperatures × 8 seeds = 384 observations per operator`.

```
ŷ(o) = c_scan(o) + c_flip(o) · x
```

The intercept is `W_scan` and the slope is `W_flip`, both in ms per unit
shape-work. Pooling instances is what identifies the law as instance-independent;
pooling operators is **forbidden**, since the whole hypothesis is that the two
operators have different flip costs. Coefficients are frozen from **held-in
only** and written into the descendant document before held-out is touched.

**The clip at 1/3 is part of the frozen model**, not a fitting choice: it encodes
the `apply_flips` dense/scattered dispatch at `flips·3 ≥ r`.

## A3 — Gate B, restated exactly

**Baseline conversion — the defect this repairs.** The shipped model emits the
abstract, dimensionless `S_i`. Comparing it to milliseconds without a scale would
make the baseline lose on units alone, which would rig the test. The baseline is
therefore given its **own free scale parameter**, fitted on held-in by the same
rule:

```
λ(o) = median over the 384 held-in observations of o of  ( ms_per_sweep / S_i )
ms_hat_shape(i, o, T_cell) = λ(o) · S_i
```

`λ(o)` has no temperature and no φ dependence — that is exactly the blindness
RC-005 measured — but the baseline is otherwise a fair steelman.

**Proposed prediction:**

```
ms_hat_prop(i, o, T_cell, s) = S_i · [ c_scan(o) + c_flip(o) · min(φ, 1/3) ]
```

**Evaluation set, exactly.** Per `(instance, operator)`: the held-out block gives
`8 temperatures × 8 seeds = 64` observations. There are `6 × 2 = 12` such cells,
hence **768 held-out observations in total**. Degenerate cells (§A1.4) are
excluded and their count reported; if exclusions reduce any `(instance, operator)`
cell below **48** of its 64 observations, that cell is reported `INSUFFICIENT` and
counts as a failure for that cell rather than being silently dropped.

**Metric.** Median absolute relative error over exactly those 64 observations:

```
MdARE(i, o, model) = median_{64 obs}  | ms_hat − ms_meas | / ms_meas
```

**Gate B passes** iff

```
MdARE(i, o, proposed) ≤ MdARE(i, o, shape) − 0.05
```

on **at least 10 of the 12** `(instance, operator)` cells. Otherwise the verdict
is `EFFICACY NOT ESTABLISHED`, with the consequences already fixed in §8 of the
body and in K5: no adoption, and **no additional term, interaction or regime
split may be added to the formula in this cycle.**

## A4 — `k′` solves equal total time, not equal slope

The body's `k′ = round(16 · b_M / b_G)` equates **slopes** and silently assumes
`a_M = a_G`. That is the same `k`-independent-floor error the cycle exists to
expose. Equal cost means equal **total** time:

```
a_M + b_M · 16  =  a_G + b_G · k′
```

hence

```
k′ = round( ( a_M − a_G + 16 · b_M ) / b_G )
```

with `a_o, b_o` from Design L. **Criterion A4, restated:** this integer must be
identical across all eight pilot seeds and across the central 95% of the
bootstrap, and must satisfy `k′ ≥ 1`. A `k′` that moves with the seed, or that is
non-positive, is not an equal-cost budget and fails A4.

## A5 — N2 replaced by a valid zero-slope null

The body's N2 proposed to fit a slope from `k = 0` cells alone. With every
regressor value identical the OLS slope is undefined; the control could not have
passed or failed meaningfully.

**N2′ — synthetic zero-slope null.** A synthetic workload whose duration is
constructed to be **independent of the `k` label** is executed under the four
distinct labels `k ∈ {4, 8, 16, 24}` and fitted by the §6 procedure. The 95% CI on
the fitted `b` must **contain zero**. This is a genuine two-sided null: a harness
that manufactures slope from labels alone fails it.

## A6 — P1 was not a harness positive

`T(k) = a + b·k` with `a > 0` gives `T(24)/T(4) < 6` always, and the ratio tends
to 1 as `a` dominates. The body's "> 2.0 on every instance" therefore tested each
instance's `a/b` ratio, not the harness, and a perfectly sound harness could fail
it on a floor-dominated instance.

**P1′ — synthetic known-effect positive.** Against a synthetic workload with
**constructed, known** `a` and `b`, the §6 fitting procedure recovers both within
**5%**, and recovers the implied `k′` of §A4 exactly. This is the cycle's
known-effect positive control and it is independent of the Runtime.

**D1 — real-instance monotonicity, a diagnostic.** On every real Design L cell the
medians must satisfy `T(4) < T(8) < T(16) < T(24)` **strictly**. This is a valid
requirement — `b > 0` implies strict monotonicity regardless of `a` — and it fails
loudly on a broken harness. The **ratio** `T(24)/T(4)` is **reported, not
thresholded**; any threshold on it may only be set from the §A6 synthetic
calibration, never chosen after seeing real data.

## Internal consistency check

| item | resolution |
|---|---|
| Gate A data source | Design L only; §6 estimation unchanged and still applies to it |
| Gate B data source | Design T only; §A2 estimator |
| φ identifiable? | yes — 8 isothermal cells per (instance, operator), vs 1 aggregate point under the superseded design |
| intercept handling | Design L models it (`a`); Design T cancels it by differencing |
| operators pooled? | never — separate fits in both designs |
| baseline fairness | `λ(o)` free scale fitted on held-in only |
| observation count | Gate B: exactly 64 per cell, 768 total; `INSUFFICIENT` below 48 |
| `k′` | solves equal total time; `≥ 1`; seed-stable |
| controls | N1, N3, P2, P3 stand; N2′, P1′ replace N2, P1; P4 added; D1 is a diagnostic |
| kill criteria able to fail | K1–K6 stand, K3 extended to both grids; A4 and Gate B both have concrete two-sided thresholds |
| adoption | still prohibited; scheduler change still requires a separate accepted ADR (§10) |

---

# Amendment 2 — paired timing windows and corrected run accounting

**Status:** binding amendment to this pre-registration as amended by Amendment 1.
Written **before any RC-018 instrument code exists and before any RC-018 datum.**
Documentation only.

**Reason.** Independent review returned two further binding defects. Both are
verified below and both are **accepted**.

## Clauses superseded — and only these

| clause | status |
|---|---|
| §A1.1, the difference quotient over two budgets | **SUPERSEDED** by §B1 |
| §A1.2, the φ averaging window | **SUPERSEDED** by §B1.3 |
| §A1.4, `DEGENERATE-TIMING` for Design T | **SUPERSEDED** by §B3 |
| §A1.5, run count | **SUPERSEDED** by §B2 |
| §4, control **N3** | **EXTENDED** by §B1.4 |

Nothing else changes. Design L is untouched. §A2's estimator, §A3's Gate B
metric, evaluation set of 64 observations per cell and 0.05 margin, §A4's `k′`
equation, §A5's N2′, §A6's P1′ and D1, and every threshold anywhere in this
document stand exactly as written. **This amendment changes no threshold.**

## B1 — Design T is a paired nested execution

### The defect, verified

§A1.1 defined per-sweep time as `[T_meas(12) − T_meas(4)] / 8` without saying
that the two timings come from the **same** run. Read literally it permits two
**independent** executions at budgets 12 and 4. That would be invalid for three
independent reasons:

1. The two runs would occupy **different RNG positions and different states**, so
   their difference is not the cost of any single trajectory's sweeps 5–12.
2. The subtraction would combine **two independent wall-time noises**, inflating
   variance and admitting negative differences.
3. The `k`-independent floor would cancel only in expectation, not per
   observation, so the intercept-free property §A1.1 claimed by construction
   would not actually hold.

**The finding is accepted.**

### The specification

Each Design T observation is **one paired nested execution**: a single Runtime
execution, one state, one RNG stream, at one constant `T_cell`.

1. **Prefix — sweeps 1–4.** Executed normally. A timing boundary is taken at the
   end of sweep 4.
2. **Measurement window — sweeps 5–12, exactly eight sweeps.** `incremental_ms`
   is measured **directly around this window**: one clock read immediately before
   sweep 5 and one immediately after sweep 12.

   ```
   ms_per_sweep(i, o, T_cell, s) = incremental_ms / 8
   ```

   **This is a direct measurement, never a subtraction of two independent wall
   times.** No arithmetic combines timings from different executions anywhere in
   Design T.

3. **φ from the same window only.** `φ = accepted / proposals`, counted over
   **sweeps 5–12 exclusively**. Proposals and acceptances from the prefix are not
   included, and no acceptance figure from any other execution may enter. This
   keeps the regressor and the response measured over the identical interval of
   the identical trajectory — the property that makes the pairing meaningful.

4. **Prefix timing is diagnostic only.** The prefix interval may be timed and
   recorded as `prefix_ms` for host-behaviour diagnostics. It is **not** a Gate B
   response, is **not** used in any fit, and may not be subtracted from anything.
   The Gate B response is the incremental window and nothing else.

5. **N3′ — non-invasiveness under pairing.** The paired instrumented trajectory
   must be **bit-identical** to an **uninterrupted 12-sweep trajectory** at the
   same instance, operator, temperature and seed, on all three of: final state
   digest, energy bits, and RNG probe. This supersedes §4's N3 by testing the
   specific hazard pairing introduces — that a mid-run timing boundary or
   checkpoint perturbs the run it is measuring. A mismatch kills the cycle under
   K1.

## B2 — Run count, corrected and per block

### The defect, verified

§A1.5 computed `6 × 2 × 4 × 8 × 9 = 3,456` using **eight seeds, which is one
block**, and then labelled the sum "across pilot, held-in and held-out blocks
combined". Those numbers are **per block**, so the stated total understated the
cycle by a factor of three. **The finding is accepted.**

### Terminology, fixed

- **Operator execution** — one Runtime execution of one plan at one budget. The
  unit of Design L.
- **Paired trajectory** — one Runtime execution carrying a 4-sweep prefix and an
  8-sweep measurement window, yielding exactly one `incremental_ms` and one φ.
  The unit of Design T. A paired trajectory is **one** Runtime execution, not two.

Both are Runtime executions; the two names distinguish what each unit yields.

### The corrected accounting

Per block — each block has eight seeds:

| design | factors | per block |
|---|---|---|
| Design L | `6 instances × 2 operators × 4 budgets × 8 seeds × 9 repetitions` | **3,456 operator executions** |
| Design T | `6 instances × 2 operators × 8 temperatures × 8 seeds × 9 repetitions` | **6,912 paired trajectories** |
| **total** | | **10,368 Runtime executions per block** |

Across the three science blocks — pilot, held-in, held-out:

```
3 × 10,368 = 31,104 Runtime executions
```

**Controls are additional and are not included in this figure.** The controls of
§4 (N1, N3′, P2, P3), §A1.3 (P4), §A5 (N2′) and §A6 (P1′, D1) carry their own
executions, several of them synthetic rather than Runtime, and are accounted for
separately in the results document.

Design T is now **6,912** paired trajectories per block rather than the 13,824
executions implied by the superseded two-budget scheme, because pairing folds
both budgets into a single execution. The corrected cycle total of **31,104** is
recorded here so the true cost is visible before authorisation.

## B3 — Positive-duration and denominator guards

A Design T observation is `DEGENERATE-TIMING`, excluded from fitting and counted
in the exclusion report, unless **both** hold:

1. `incremental_ms > 0` **strictly.** A non-positive interval is physically
   impossible and indicates timer wrap, insufficient resolution, or a clock
   source that is not monotonic. It is never clamped, floored, or treated as a
   small positive value.
2. `incremental_ms ≥ 100 × timer_resolution`, with the resolution measured by the
   instrument and recorded.

**Denominator safety, stated explicitly.** §A3's MdARE divides by `ms_meas`, and
`ms_meas = incremental_ms / 8`. Guard 1 makes that denominator strictly positive
for every observation that reaches the metric, so MdARE is always well-defined and
no observation can be silently rescued by a clamp.

Design L's degeneracy rule and §A1.4's `K3` counting rule are unchanged in every
other respect: more than **2 of 6** instances degenerate at `k = 4` invalidates
Design L, and more than **2 of the 8** temperature cells degenerate on any
instance invalidates Design T. The §A3 `INSUFFICIENT` rule — a
`(instance, operator)` cell falling below 48 of its 64 observations fails rather
than being dropped — continues to apply unchanged.

## Internal consistency check

| item | resolution |
|---|---|
| Design T unit | one paired trajectory = one Runtime execution, one state, one RNG stream |
| Gate B response | `incremental_ms / 8`, measured directly around sweeps 5–12 |
| subtraction of independent runs | **eliminated** — no cross-execution arithmetic remains in Design T |
| φ window | sweeps 5–12 only, same trajectory, same interval as the response |
| prefix timing | diagnostic only; never a response, never subtracted |
| intercept | cancelled by construction — the window excludes the startup floor rather than estimating it |
| N3 | superseded by N3′: paired instrumented vs uninterrupted 12-sweep, exact state/energy/RNG |
| Design L | untouched by this amendment |
| run count | 10,368 per block; 31,104 across three blocks; controls separate |
| degeneracy | `incremental_ms > 0` and `≥ 100×` resolution; MdARE denominator provably positive |
| thresholds | **none changed by this amendment** |
