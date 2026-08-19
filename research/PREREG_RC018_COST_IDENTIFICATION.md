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
