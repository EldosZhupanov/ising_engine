# PREREG — RC-020: direct identification of marginal wall cost

**Status:** binding pre-registration, written **before any RC-020 instrument code
exists and before any RC-020 datum — pilot, held-in or held-out.** Documentation
only. No binary is implemented by this commit and nothing is run.

**Constitution §13** requires pre-registration to precede data in git history;
this file is the first RC-020 artifact of any kind. **ADR-0004** requires a
recorded result to be reproducible from its inputs; every quantity below is
either directly measured or derived by a procedure fixed here.

## 0. RC-020 is a fresh successor, not a repair of RC-018

RC-018 is **closed**. Its pilot is an invalid-instrument run under its own K1
(`RC018_PILOT_ABORT_RECORD.md`), it is not repaired, not re-run, and none of its
data, seeds, artifacts or verdicts enter RC-020 as evidence, baseline or prior.
RC-020 does not inherit RC-018's estimand, its two-design split, its Gate B, or
its thresholds.

**Disclosure of what was observed, because it bears on this design.** RC-018's
pilot output was seen before this document was written. Two structural lessons —
not values — are carried forward, and both are named here so the reader can
discount them:

1. A criterion defined on the **integer rounding** of a well-identified
   continuous quantity is brittle: it can fail permanently when the true value
   sits near a rounding boundary, no matter how precise the estimate. RC-020
   therefore states its criteria on continuous quantities with explicit precision
   requirements, and never on integer stability.
2. A guard that is asserted in prose but absent from executable code is
   undetectable by a passing control set. RC-020 requires every guard to be
   executable and to leave a field in the artifact.

No RC-018 numeric value informs any threshold below. Thresholds are set from
`PERF.md`'s recorded host drift and from timer resolution, both of which predate
RC-018.

## 1. The single question, and what is deliberately not asked

**Primary estimand — marginal wall cost.** For an operator `o` on instance `i`,
the wall-clock cost of one additional sweep,

```
b(i, o) = d(wall time) / d(sweeps)     [milliseconds per sweep]
```

**measured directly**, never fitted through an intercept. A window of exactly `W`
sweeps is timed inside a running trajectory and `b̂ = window_ms / W`. The
`k`-independent startup floor is excluded **by construction** because it lies
outside every timed window. There is no regression, no intercept parameter, and
therefore no floor/slope separability problem.

**Gate A, and only Gate A, is in scope.** RC-020 asks whether `b` is identifiable
to a pre-registered precision, and whether it is stationary in window length.

**Explicitly out of scope, and unclaimable from any RC-020 result:**

- any **efficacy** comparison of a cost model against measurement — RC-005's
  `W = W_scan + W_flip·min(φ,⅓)` is neither tested nor validated here;
- any **equal-cost** comparison of two operators, or any equal-cost budget;
- any quality, energy, cut or outcome quantity — RC-020 stores none;
- any deployment, scheduler, throughput or superiority claim;
- any change to `cost_model`, the Decision Engine, the scheduler, or any operator.

`cost_model` is **forbidden as a measure of cost** (RC-005). It is not read,
compared against, or reported by this cycle.

### 1.1 Equal-cost FEASIBILITY, reached only after a valid Gate A

If and only if Gate A passes, RC-020 additionally reports one **feasibility**
statement, computed from the same calibration data and from nothing else:

```
k'_real(i) = ( b_M(i) / b_G(i) ) · k_ref ,   k_ref = 16
```

with its CI. This is a **feasibility assessment about future work**, not an
equal-cost comparison and not a budget. It answers only: *is the ratio determined
sharply enough that an equal-cost design could be constructed at all?* Three
outcomes are pre-registered in §7.3. **No RC-020 document may state that two
operators were compared at equal cost.**

## 2. Frozen design

| item | frozen value |
|---|---|
| corpus | **G1, G11, G14, G22, G32, G43**, verified against the `sha256[:12]` recorded for each in `PREREG_RC016.md` §4 |
| operators | `metropolis_sweep` and `gibbs_color_sweep`, measured and reported **separately**; never pooled |
| replicas | 32 |
| initialisation | legacy all-zeros |
| temperature | **isothermal** per cell, `T_cell ∈ {0.1, 0.5, 2.0}` |
| prefix | 4 sweeps, untimed, discarded |
| timed windows | three consecutive, `W ∈ {4, 8, 16}` sweeps, each **directly timed** |
| window-order arms | **A** = (4, 8, 16), **B** = (16, 8, 4) |
| warmup | one full discarded trajectory per `(instance, operator)` before the first recorded repetition of a session |
| repetitions | 9 per cell |
| sentinel | fixed condition, executed **first and last in every repetition** |
| pilot seeds | **10001–10008** |
| held-in seeds | **11001–11008** — reserved, **not opened by RC-020** |
| held-out seeds | **12001–12008** — reserved, **not opened by RC-020** |
| CI bootstrap seed | `20261201 + 2·corpus_index + arm_code` |
| bootstrap resamples | 100 000, percentile 95% |
| drift bound | **9%**, frozen; see §5 |
| degenerate bound | window median below **100 ×** measured timer resolution |
| host load bound | 1-minute load average **< 2.0** at repetition start |

### 2.1 Seeds — binding

RC-020 uses **pilot `10001–10008` only.** Gate A is a calibration gate and needs
no science block. `11001–11008` and `12001–12008` are frozen and reserved for
whatever the descendant of §9 specifies; RC-020 must not reach them, and the
instrument must refuse any mode that names them.

**Forbidden to RC-020 in every mode**, with refusal and a non-zero exit:

`1001–1008`, `2001–2008`, `3001–3008` (RC-016); `4001–4008` (RC-017 pilot);
`5001–5008` (**burned** by the RC-017 abort); `6001–6008` (reserved to RC-017's
successor); **`7001–7008` (burned by the RC-018 pilot)**; `8001–8008` and
`9001–9008` (RC-018's reserved blocks, which RC-020 must not open).

### 2.2 Why three window lengths and two orders

Timing three consecutive windows inside one trajectory gives three direct
estimates of `b` with no subtraction and no intercept. But consecutive windows
occupy **different positions** in the trajectory, so window length is confounded
with position. Arm **B** reverses the order, decorrelating the two across arms.
A length effect that survives both orders is a length effect; one that reverses
with the order is a position effect. This is stated in advance because
discovering it afterwards would be a post-hoc explanation.

## 3. Execution protocol

1. **One trajectory per observation.** One state, one `ChaCha8Rng`, one operator
   instance. The prefix and all three windows are consecutive `apply` calls on
   the same state and the same stream. **No arithmetic anywhere in RC-020
   combines timings from different executions.**
2. **Direct timing.** Each window is bracketed by one clock read before its first
   sweep and one after its last. Window durations are never reconstructed by
   subtracting two totals.
3. **Warmup.** One complete trajectory per `(instance, operator)` is executed and
   **discarded** before the first recorded repetition of a session, so
   first-touch page faults and cache population do not land in recorded data.
4. **Interleaving.** Within one repetition every `(instance, operator, T_cell,
   arm)` condition is executed once before any condition is executed a second
   time.
5. **Randomised order.** The order within a repetition is a deterministic
   Fisher–Yates permutation seeded `20261301 + repetition_index`.
6. **Sentinel, executable.** A fixed sentinel condition — `metropolis_sweep` on
   G11 at `T = 0.5`, `W = 8` — is executed **first and last in every
   repetition**, and **both durations are written to the artifact**. A sentinel
   that leaves no field is treated as not executed (§5).
7. **Affinity and environment.** The instrument pins itself to a fixed set of
   **4 logical CPUs** for the whole session and records the set. It starts no
   other process during timing. It records, per repetition: 1-minute load
   average at start and end, the CPU set, the timer resolution it measured, and
   the thread count.
8. **Median over repetitions**, never mean.

## 4. Mandatory controls

All must pass before any recorded datum is retained. Failure ⇒ **K1**, invalid
instrument.

**Null controls:**

- **N1 same-code null.** One condition timed under two distinct labels; the
  measured ratio must lie within the §5 drift bound.
- **N2 zero-slope null.** A synthetic workload constructed to be independent of
  the window-length label, executed under all three labels `W ∈ {4, 8, 16}`; the
  estimated dependence of `b̂` on `W` must have a CI containing zero.
- **N3 non-invasiveness.** A trajectory carrying the prefix, three timed windows
  and both sentinel executions must be **bit-identical** — state digest, energy
  bits, and RNG probe — to an uninterrupted trajectory of the same total sweep
  count at the same seed. Instrumentation must not perturb what it measures.

**Positive controls:**

- **P1 synthetic marginal recovery.** Against a synthetic workload with a
  **constructed, known** per-unit cost, `b̂` recovers it within **5%**, and
  recovers the implied `k'_real` within **5%**. Independent of the Runtime.
- **P2 corpus hashes.** All six files match `PREREG_RC016.md` §4.
- **P3 sentinel liveness.** A deliberately injected slowdown in the sentinel
  condition must be **detected** by the §5 drift rule, not absorbed. A guard that
  cannot fail on demand is not a guard — this is the control whose absence made
  the RC-018 pilot invalid.
- **P4 window-length positive.** On real cells, `W = 16` windows must take
  strictly longer than `W = 4` windows of the same trajectory arm. Reported as a
  strict inequality only; **no ratio threshold**, because the ratio is a property
  of the instance rather than of the harness.

## 5. Host, drift and degeneracy — numerically frozen before any run

- **Drift bound: 9%**, the same-code drift recorded for this class of host in
  `PERF.md`. Frozen here, before any RC-020 execution, and not adjustable
  afterwards under any circumstance.
- **Sentinel rule.** For each repetition, `spread = max/min − 1` over the first
  and last sentinel medians. If `spread > 0.09` the **repetition is discarded**,
  its reason is written to the artifact, and it is re-executed at most once. If
  the re-execution also exceeds the bound the **session is discarded** and the
  discard is recorded — never silently dropped.
- **`HOST-UNSTABLE`.** More than **2 of 9** repetitions discarded in a session ⇒
  the session is `HOST-UNSTABLE` and **aborts before any verdict**.
- **`HOST-LOADED`.** A repetition beginning at 1-minute load average ≥ **2.0** is
  marked and discarded before execution, not after.
- **`DEGENERATE-TIMING`.** A window observation is degenerate unless **both**
  `window_ms > 0` strictly — a non-positive interval is impossible and is never
  clamped — and `window_ms ≥ 100 × timer_resolution`. Degenerate observations are
  excluded from estimation and their count reported. More than **2 of the 3**
  window lengths degenerate on any `(instance, operator, T_cell)` cell invalidates
  that cell's grid; more than **2 of 6** instances so invalidated stops the cycle.
- **`UNSTABLE`.** A cell whose three window-length estimates disagree beyond the
  §7 stationarity bound is routed to `UNSTABLE` and is **not** counted toward
  Gate A, and is **not** reported as a null. The two routings are distinct.

## 6. Estimation, fixed before data

Per `(instance, operator, T_cell, arm, window W, seed)`:

1. `b̂ = window_ms / W`, the **median** over the 9 recorded repetitions.
2. Per `(instance, operator, T_cell, arm, W)`: the reported `b̂` is the **median
   over the eight pilot seeds**.
3. The CI on `b̂` is the deterministic **percentile 95%** bootstrap over the eight
   per-seed values, 100 000 resamples, seeded by §2's formula.
4. Per `(instance, operator)`: `b` is the median over the three `T_cell` values
   and both arms, reported with the range across them.
5. `k'_real(i) = (b_M(i) / b_G(i)) · 16`, its CI obtained by resampling the
   paired per-seed values jointly, same seed formula, same resample count.

## 7. Gate A — calibration only

### 7.1 Per-cell criteria

A `(instance, operator, T_cell, arm)` cell **qualifies** iff all three hold:

- **A1 precision.** The CI half-width on `b̂` is **≤ 10%** of `b̂`.
- **A2 stationarity in window length.** The three estimates at `W ∈ {4, 8, 16}`
  lie within **±10%** of their median. Failure routes the cell to `UNSTABLE`.
- **A3 positivity.** `b̂ > 0` with a CI excluding zero.

### 7.2 Cycle verdict

- **`IDENTIFIED`** iff ≥ **5 of 6** instances qualify for **both** operators, in
  **all three** `T_cell` values and **both** arms.
- **`NOT IDENTIFIED`** otherwise, with the failing criterion named per cell.

`NOT IDENTIFIED` is a **scientific null and a first-class result**: it states
that marginal wall cost is not determinable to 10% by direct windowed timing on
this host and grid. It is not a failure of the cycle.

### 7.3 Feasibility statement, only after `IDENTIFIED`

- **`FEASIBLE`** — the CI on `k'_real(i)` has width **< 1.0 sweep** on ≥ 5 of 6
  instances.
- **`FEASIBLE AT REDUCED RESOLUTION`** — width in `[1.0, 3.0)` sweeps.
- **`NOT FEASIBLE`** — width ≥ 3.0 sweeps.

Deliberately **absent:** any requirement that a rounded integer be stable. The
criterion is stated on the continuous quantity and its width, per §0.

## 8. Kill criteria — invalid instrument versus scientific null

The distinction is binding, and conflating the two is itself a protocol
violation.

**Class I — INSTRUMENT INVALID. No result of any kind; no verdict may be
reported.**

| # | condition |
|---|---|
| K1 | any control of §4 fails, including **P3 sentinel liveness** |
| K2 | the sentinel is absent from the artifact schema, or any repetition lacks both sentinel fields |
| K3 | `HOST-UNSTABLE`: more than 2 of 9 repetitions discarded |
| K4 | degenerate-timing bounds of §5 exceeded |
| K5 | any forbidden seed reaches execution, or held-in/held-out is opened |
| K6 | any provenance requirement of §10 fails |

**Class II — SCIENTIFIC NULL. The instrument is valid; the answer is negative and
is published as a result.**

| # | condition |
|---|---|
| K7 | Gate A returns `NOT IDENTIFIED` (§7.2) |
| K8 | Gate A passes and feasibility is `NOT FEASIBLE` (§7.3) |

A Class I outcome may **never** be written up as a Class II result, and no
Class II verdict may be recorded while any Class I condition holds. `UNSTABLE`
cells (§5) belong to neither class on their own: they are excluded from
qualification and reported with their count.

## 9. Falsifiable descendant — deliberately blank and frozen

**This section is intentionally empty and is frozen in that state.**

No descendant may be written until Gate A has returned a verdict on the pilot
block. When written, it must name its candidate set and an exact numeric pass/
fail threshold using only quantities RC-020 actually measured, and must be
committed **before** any held-in seed is opened.

The held-in and held-out blocks (§2.1) exist so that a descendant has somewhere
to go. They are not RC-020's to spend.

## 10. Provenance and fail-closed requirements

The instrument must **refuse and exit non-zero**, naming the failed condition, if
any of the following does not hold. Each is a gate, not a message.

1. This file exists, is **tracked**, is **clean** against `HEAD`, and has been
   **committed**.
2. Its committing commit is **strictly older** than every RC-020 artifact.
3. All six corpus files match their `PREREG_RC016.md` §4 hashes.
4. The requested seed block is exactly `10001–10008`; every seed in §2.1's
   forbidden list is refused by value.
5. Controls of §4 have executed and passed **in the same process invocation**,
   before any recorded datum is retained. A separate `--controls` invocation does
   not satisfy this.
6. The sentinel executed first and last in the repetition, and both durations are
   present in the artifact row.
7. Any mode naming `11001–11008` or `12001–12008` is refused unconditionally
   while §9 remains blank.

### 10.1 Artifact schema, exact

One TSV per session, written only on successful completion, under
`experiments/rc020/`. Columns, in this order:

```
instance  corpus_index  operator  temp  arm  window_w  seed  repetition
window_ms  b_hat  sentinel_first_ms  sentinel_last_ms  sentinel_spread
timer_resolution_ms  load_avg_start  load_avg_end  cpu_set  thread_count
discarded  discard_reason  degenerate  ci_seed
```

No energy, cut, quality or outcome column exists, and none may be added. Every
guard of §5 has a column here, so a guard that did not run is visible in the
data rather than only in the code.

The results document records each artifact's **SHA-256**, its row count, and the
commit that produced it.

## 11. Implementation plan — for a future step, not this one

No binary is implemented by this commit. When implementation is authorised:

1. Create `src/bin/exp_marginal_cost.rs` with the §2 frozen constants and the
   six-instance corpus with its hashes.
2. Implement the provenance gates of §10 as a pure decision over gathered facts,
   plus a thin git wrapper, and unit-test the decision without a git tree.
3. Implement the paired trajectory: prefix, then three consecutive directly timed
   windows on one state and one RNG stream.
4. Implement the sentinel as executable code writing both durations, and the
   discard/re-execute/abort logic of §5.
5. Implement the environment protocol: CPU pinning, load-average capture, timer
   resolution measurement, warmup.
6. Implement controls N1–N3 and P1–P4, with **P3 constructed so it can fail on
   demand**, and wire them to run in-process before any datum is retained.
7. Implement the §6 estimators and the §7 routing, including the distinct
   `UNSTABLE` path.
8. Implement the §10.1 schema exactly, with every guard column populated.
9. Add unit tests: forbidden-seed refusal, no cross-execution arithmetic, no
   outcome columns, sentinel presence, Class I versus Class II routing, and the
   frozen constants matching this document.
10. Run controls only. Report. **Do not run the pilot in the same step.**

## 12. What this pre-registration does not license

It does not license an equal-cost comparison, an efficacy claim about any cost
model, a scheduler or Decision Engine change, a repair or re-run of RC-018, or
opening any reserved seed block. A passed Gate A licenses exactly one sentence:
*marginal wall cost is directly measurable on this grid to the stated precision,
and the operator ratio is determined to the stated width.*
