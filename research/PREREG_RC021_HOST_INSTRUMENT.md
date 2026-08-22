# PREREG — RC-021: host and timing-instrument qualification

**Status:** binding pre-registration. Written **before any RC-021 instrument
code exists and before any RC-021 datum of any kind.** Documentation only: this
commit creates no binary, runs nothing, and produces no measurement.

**Constitution §13** requires pre-registration to precede data in git history.
**ADR-0004** requires a recorded result to be reproducible from its inputs.

---

## §0. One question, one estimand

RC-021 qualifies a **measuring station**, not an optimizer.

> **Can this host and a new fail-closed timing instrument measure short
> wall-time intervals reproducibly, to a precision fixed in advance, without
> selecting lucky quiet moments, and while preserving complete evidence under
> every abort?**

### Primary estimand

For one deterministic sentinel workload executed twice under identical
conditions, the **paired wall-time spread**

```
spread = max(t_first, t_last) / min(t_first, t_last) − 1
```

The object of study is the **distribution of `spread` across a frozen protocol**,
summarised by the single statistic of §7: the per-measurement rate at which
`spread` stays within the inherited bound.

**Wall time is the product quantity and is never displaced.** Process CPU time,
CPU frequency, temperature, involuntary context switches and similar values may
be recorded as **diagnostic channels only**, when they are reliably obtainable.
They may not be substituted for wall time in any statistic, threshold or verdict,
and a diagnostic channel that is unavailable is recorded as unavailable rather
than silently omitted (§6, fallback rules).

No secondary indicator may be introduced after data exist, and none may replace
the primary spread.

## §1. Scope, and the claims RC-021 may never make

RC-021 **must not** produce, imply, or be cited for any of:

- marginal wall cost `b(i,o)` of any operator;
- any Metropolis/Gibbs ratio, `k′`, equal-cost budget, or cost comparison;
- any energy, cut, best state, quality or outcome quantity;
- any operator ranking, preference, or default;
- sensor sufficiency, scheduler or policy claims;
- superiority over `UltimateSolver` or any external solver;
- any external benchmark, product, or breakthrough claim.

**The one statement a PASS licenses, in full:**

> On the fixed host configuration and under the fixed protocol recorded here, the
> timing instrument satisfied a reproducibility criterion fixed in advance, and
> may therefore be used by a separate, newly pre-registered cost cycle.

That sentence is the maximum. It is a statement about a **measuring station**,
not about optimization, and it licenses no number in any downstream model.

## §2. Standing constraints, inherited and binding

- **RC-020 is closed** (`RC020_PILOT_ABORT_RECORD.md`), Gate A returned **NO
  VERDICT**, and it is **not repaired in place** and not re-run.
- **Burned, never reusable:** `5001–5008` (RC-017), `7001–7008` (RC-018),
  `10001–10008` (RC-020).
- **Reserved and unopened, and they do not pass to RC-021:** `6001–6008`,
  `8001–8008`, `9001–9008`, `11001–11008`, `12001–12008`.
- **In use by earlier cycles:** `1001–1008`, `2001–2008`, `3001–3008`,
  `4001–4008`.
- **No prior science seed is used by RC-021 in any mode.**
- **The `0.09` bound may not be lowered retrospectively.** It originates in
  `PERF.md`'s recorded same-code drift for this host class and was frozen by
  RC-020 before its data. RC-021 inherits it as a *requirement to be tested*,
  not as a number to be tuned.
- RC-021 may fix its own acceptance criterion **only here, before code and
  data, with the arithmetic justification of §7**.
- The number of sessions and blocks is **fixed here** (§4) and may not change.
- **No retries. No discarded-and-rerun sessions. No "repeat in a quieter
  moment."** Every planned execution stays in the denominator whatever it
  returns.

## §3. Seed and workload audit

### 3.1 Audit of everything that already exists

Performed by `rg` over `src/bin/exp_*.rs` and the canonical research documents
before any new number was chosen.

| family | values | where declared |
|---|---|---|
| RC-014/015/016 held-in | `1001–1008` | `exp_counterfactual.rs:61`, `exp_tie_handling.rs:60`, `exp_sensor_sufficiency.rs:62` |
| held-out | `2001–2008` | `exp_counterfactual.rs:62`, `exp_tie_handling.rs:61`, `exp_sensor_sufficiency.rs:63` |
| RC-016 pilot | `3001–3008` | `exp_sensor_sufficiency.rs:65` |
| RC-017 pilot / held-in / held-out | `4001–4008` / `5001–5008` **burned** / `6001–6008` reserved | `PREREG_RC017.md` §3 |
| RC-018 pilot / held-in / held-out | `7001–7008` **burned** / `8001–8008` / `9001–9008` | `exp_cost_identification.rs:53–55` |
| RC-020 pilot / held-in / held-out | `10001–10008` **burned** / `11001–11008` / `12001–12008` | `exp_marginal_cost.rs:40–42` |
| control / internal | `20001`, `991`, `993` | `exp_marginal_cost.rs:53`, `:56`, `:57` respectively |
| bootstrap and permutation bases | `20260819` (`exp_sensor_sufficiency.rs:79`), `20260820` (`:81`), `20260901` (`PREREG_RC017.md` §3), `20261001` (`exp_cost_identification.rs:72`), `20261101` (`:73`), `20261201` (`exp_marginal_cost.rs:59`), `20261301` (`:60`) | as cited |

**Derived permutation ranges, including one that is easy to miss.**
`exp_cost_identification.rs:978` uses `PERM_BASE_SEED + 1000 + rep`, not
`PERM_BASE_SEED + rep`, so with `PERM_BASE_SEED = 20261101` and
`REPETITIONS = 9` it additionally occupies **`20262101–20262109`**.

```
occupancy = {991, 993} ∪ [1001, 12008] ∪ {20001} ∪ [20260819, 20262109]
```

The `+1000` offset was missed in an earlier draft of this table and found by
independent review. It is recorded because it demonstrates the audit's real
failure mode: a base constant is easy to enumerate, a call-site offset is not.
**An implementation must derive occupancy from call sites, not from constant
declarations.**

### 3.2 RC-021's own seeds

RC-021 produces **no science outcome**, so its seeds are **not** named pilot,
held-in or held-out. They are **diagnostic and control seeds** and are described
as such wherever they appear.

| constant | value | purpose |
|---|---|---|
| `RC021_SENTINEL_SEED` | **31001** | the single deterministic sentinel workload; every sentinel execution uses it, so a paired spread compares identical work and identical numbers and measures only the host |
| `RC021_LOAD_SEED` | **31002** | the sustained-load workload of phase C |
| `RC021_SYNTH_SEED` | **31003** | synthetic generator for the positive and negative controls of §6 |
| `RC021_ORDER_SEED` | **31004** | base for any deterministic ordering permutation, `31004 + session_index` |

**Reserved band: `[31001, 31099]` belongs to RC-021 and to nothing else.**

### 3.3 Disjointness — machine-checkable

The instrument must, **before any measurement**, enumerate every family of §3.1
and assert that no RC-021 seed is a member, and that the four RC-021 seeds are
pairwise distinct. Failure is **Class I** (`INSTRUMENT-INVALID`, §8), never a
qualification outcome.

Proof by interval, checkable by inspection: the largest prior science value is
`12008`; the control seed is `20001`; the lowest bootstrap-derived value is
`20260819` and the highest is `20262109`. `31001…31004` lie strictly inside the
open interval `(20001, 20260819)` and are therefore disjoint from every family.
The nearest occupied value below is `20001` (gap 11 000) and above is `20260819`
(gap 20 229 815).

**No fallback to an old, burned or reserved seed is permitted under any
condition**, including instrument failure, host rejection, or a missing seed
constant. A missing or unparsable RC-021 seed is a refusal, not a default.

### 3.4 Workload — representative but outcome-blind

The sentinel is an existing, already-used control workload, and **every one of
its parameters is frozen numerically here** rather than deferred to
implementation. The values are **inherited** from RC-020's frozen sentinel, not
newly chosen; each row cites its own line:

| parameter | frozen value | source |
|---|---|---|
| operator | `metropolis_sweep` | inherited |
| instance | `G11` | `:104`, `CORPUS[1]` |
| temperature | **0.5** | `exp_marginal_cost.rs:79`, `SENTINEL_TEMP` |
| sweeps per sentinel execution | **8** | `:80`, `SENTINEL_WINDOW` |
| replicas | **32** | `:32`, `REPLICAS` |
| initialisation | all-zeros | inherited |
| untimed prefix before each sentinel | **4 sweeps** | `:34`, `PREFIX_SWEEPS` |

The instance file **must be** hash-verified against `PREREG_RC016.md` §4 before
any measurement; no such verification has occurred, since no RC-021 code exists.

**Binding on the workload:**

- it **stores no** energy, best state, cut, quality or operator delta, and the
  instrument has no code path that can compute one;
- it **never compares two operators** and never selects a "better" one — a single
  operator is fixed for the sentinel;
- phase C's sustained load uses the same single operator with `RC021_LOAD_SEED`;
- there is **exactly one** load phase (phase C, §4.1), so no ordering among load
  phases arises.

## §4. Host protocol

### 4.1 Three conditions, in frozen order

| phase | condition | why it exists |
|---|---|---|
| **A** | **cold / initial** — the first measurements of a freshly started process | the state in which a run begins; RC-020's controls were measured here |
| **B** | **warmed steady-state** — after a fixed warmup, before sustained load | separates first-touch and cache effects from load effects |
| **C** | **sustained representative load** — sentinel pairs taken around a sustained load block | the condition under which RC-020's session actually failed |

**Frozen numerically, so nothing is chosen at implementation time:**

| parameter | frozen value | justification |
|---|---|---|
| phase-A position | the first 5 pairs of a freshly started process, after **zero** warmup | "cold" is defined by the absence of prior work |
| phase-B warmup | **32 sentinel-equivalents** (32 × 8 = 256 sweeps) with `RC021_SENTINEL_SEED`, executed and discarded before phase B's pairs | large enough that first-touch and cache population are complete; it is 32× one sentinel |
| phase-C load block | **256 sentinel-equivalents** (2048 sweeps) with `RC021_LOAD_SEED`, executed immediately before each phase-C pair | 8× phase B's warmup, so phase C differs from phase B in cumulative work by a factor, not a margin; it satisfies §6 P7 by a wide margin |
| minimum idle interval **before every session, including session 1** | **10 minutes**, with each actual interval recorded per §10 | sessions must be comparable in initial thermal state, and a floor on the five inter-session gaps alone would leave session 1 uncovered |

**Session 1 is covered explicitly, because the controls precede it.** N3 executes
one whole session's protocol — five phase-C load blocks of 2048 sweeps each — P2
runs an abort test, and §9 criterion 10 runs 120 sentinel executions, all
immediately before qualification begins. Without this clause session 1's phase A
would be the only phase A in the run that starts on a host that has just done
seconds of sustained work, while sessions 2–6 start after ten idle minutes. That
is exactly the systematic warm-session-1 effect §7.2 rule 3 would then punish, and
§11 makes the resulting FAIL terminal. **The floor therefore applies to six
intervals, not five:** the gap between the last control execution and session 1,
and each of the five gaps between consecutive sessions.

The load-block size is set from duration reasoning and the P7 floor, **not** from
any RC-020 outcome. A prior control diagnostic — not outcome data — puts one
8-sweep sentinel at roughly 2 ms on this host class, which places the load block
in the region of half a second and the phase-B warmup near 60 ms. If the
implementation measures materially different durations, the frozen counts stand;
they are counts of work, not of time.

**The order A → B → C is fixed and is NOT randomised.** The phases are *defined*
by cumulative thermal and cache state, so permuting them would destroy the thing
they name. This is stated in advance precisely so that observing a phase effect
later cannot be re-described as a randomisation choice. Ordering **within** a
phase, where any freedom exists, uses `RC021_ORDER_SEED + session_index`.

### 4.2 Fixed counts — the minimum is derived in §7; the value is a choice above it

| quantity | value |
|---|---|
| independent host sessions | **6** (six separate process invocations) |
| phases per session | **3** (A, B, C) |
| paired measurements per phase | **5** |
| paired measurements per session | **15** |
| **total paired measurements** | **90** |
| **measured** sentinel executions | 180 (two per pair) — warmup and load-block sentinel-equivalents are additional and are not measurements |

**Every one of the 90 stays in the denominator.** A measurement that fails, is
degenerate, or is lost is counted against the total; it is never dropped and
never re-run.

### 4.3 Fixed environment, identical for all six sessions

- CPU affinity: a fixed, recorded CPU set, identical in every session.
- Thread count: fixed and recorded.
- WSL processor and memory configuration: fixed, recorded, and **not changed
  between sessions**. A configuration change invalidates the whole run and
  restarts the count from zero — it does not produce a partial result.
- Mains power; sleep and suspend disabled.
- **No concurrent `cargo`, agent, model, download or build task during
  measurement.** The instrument records what it can observe of concurrent load;
  it does not police it, and an unobserved violation is a protocol violation by
  the operator, recorded as such.
- No session may be skipped, repeated, or replaced because it "looks bad".

### 4.4 What RC-021 may conclude about cause — and may not

RC-021 **does not establish thermal throttling** and must never state it as
cause. It may report only:

- whether a **workload-dependent drift** is observable across phases A, B, C;
- whether the spread is **bursty** rather than systematic;
- whether the station qualifies.

If the design cannot separate a thermal explanation from a bursty-host or
CPU-contention explanation, the record must say **"H1, H2 and H3 are not
distinguished by these data"** and stop there. RC-020's record already reached
that position; RC-021 may not quietly improve on it.

## §5. Two artifacts, and failure-safe evidence

RC-020's fatal design flaw was a single artifact: §5 required a discard to be
recorded while §10.1 forbade writing unless the session completed, and the
instrument obeyed one and violated the other, losing every discard row in memory.

The exposure is wider than the two paths named in the abort record. A read of
`run_pilot` performed for this document finds **four** paths on which a populated
buffer is lost — the `HOST-UNSTABLE` return (`exp_marginal_cost.rs:1280`), the
twice-failed repetition return (`:1288`), a failing `create_dir_all` (`:1299`),
and a failure of the buffer-persisting `fs::write` (`:1301`) — plus a fifth,
`:1253`, which returns before the buffer exists and so loses nothing, and a sixth,
a failure of the separate provenance write at `:1329` after the observation
artifact has already been written.

The two write-adjacent paths are the most instructive: at `:1299` and `:1301` the
buffer is **complete** and is still lost. Any design in which evidence lives in
memory until one final write has this class of hole by construction, and
enumerating the instances does not remove it.

**RC-021 removes the class, not the instances**, by never holding evidence in
memory across a measurement.

### 5.1 The guard journal — always written

- **One journal per session**, at
  `experiments/rc021/rc021_journal_s{session}.tsv` with `session ∈ 1..6`.
  A single fixed path cannot work: §4.2 requires six separate process
  invocations, and a refuse-if-exists rule on one path would make session 2
  unstartable. Controls that need their own journals use disjoint prefixes —
  `rc021_journal_n3.tsv` for N3 and `rc021_journal_p2.tsv` for P2 — so no control
  can collide with, overwrite, or be mistaken for a qualification session.
- Created and **flushed to disk before the first measurement of its session**,
  with the parent directory itself fsynced after creation so that a crash
  immediately after `create` leaves an empty journal rather than no file — the
  misclassification §5.2 forbids.
- **Append-only.** Every row is written and **flushed and fsynced before the
  next measurement begins**. Nothing accumulates in memory across measurements.
- It must survive: control failure, sentinel failure, host rejection, deliberate
  abort, panic or error path, and an incomplete session.
- **Overwrite is impossible:** if that session's path already exists, the
  instrument refuses to start **that session**. A journal is never truncated,
  rotated or replaced.
- **A failing `fsync` is `JOURNAL-INVALID` (§8) and aborts immediately.** Evidence
  that cannot be durably written is not evidence, and continuing would produce
  measurements whose record may not survive.
- A **partially written final line** is expected after a crash. The reader treats
  a final line with the wrong field count as **truncated**, counts it in the
  denominator as `status = LOST`, and never repairs or discards it silently.

**Journal schema — frozen, in this order:**

```
schema_version   run_uuid   repo_commit   prereg_commit   host_fingerprint
phase   session   block   measurement_index   monotonic_offset_ms
sentinel_first_ms   sentinel_last_ms   paired_spread   timer_resolution_ms
load_avg_start   load_avg_end   cpu_set   thread_count
diag_cpu_time   diag_ctx_switches   diag_freq   diag_flags
status   reason
```

The three `diag_*` value columns carry a diagnostic reading **or the literal
`NA`**, and `diag_flags` records which channels were available. **No energy, cut,
best-state, quality or operator-delta column exists, and none may be added.**

**The `status` column's domain is closed and enumerated**, because three
different vocabularies would otherwise all be called "status":

| value | meaning |
|---|---|
| `OK` | a completed paired measurement; `paired_spread` is valid |
| `LOST` | a measurement that did not complete, whose row is truncated, **or** whose sentinel duration fell below the §6 P8 resolution floor |
| `SESSION-OPEN` | lifecycle row, written before the first measurement |
| `SESSION-CLOSE-COMPLETED` | lifecycle row, all planned measurements written |
| `SESSION-CLOSE-ABORTED` | lifecycle row, session ended early |
| `EXTERNAL-CAUSE` | see §8; must be written and fsynced **before** the abort it justifies |

No other value may appear. The six **run-level** statuses of §8 are a separate
vocabulary and never appear in this column; the run status is *derived* from the
journals by §7.2, never written by the measuring loop.

### 5.2 Session lifecycle — four states, unambiguous

| state | how it is recognised in the journal |
|---|---|
| `NOT STARTED` | no journal file exists for that session id |
| `STARTED` | a header and a `SESSION-OPEN` row exist; no `SESSION-CLOSE-*` row |
| `COMPLETED` | a `SESSION-CLOSE-COMPLETED` row, and a measurement count equal to §4.2 |
| `ABORTED` | a `SESSION-CLOSE-ABORTED` row, **or** `STARTED` with no close row — a crash is `ABORTED`, never `NOT STARTED` |

### 5.3 The observation artifact — only on success

A separate `experiments/rc021/rc021_observations.tsv` may be written **only after
all six sessions reach `COMPLETED`**. It is derived from the journal and adds no
new measurement. If RC-021 needs no artifact beyond the journal, none is written;
the journal is the evidence of record either way.

### 5.4 Integrity

The results document records, for **each of the six session journals**, for the
**N3 and P2 control journals**, and for the observation artifact if it exists: the **SHA-256**, the **row count**, and the
**count of rows by status**. A run any of whose journal SHA-256 values is absent
from the record is **`JOURNAL-INVALID`** (§8) — this is a status, not merely a
statement that the run "is not evidence".

## §6. Controls

Each control gives its input, expected result, exact pass/fail, failure class,
and behaviour when a diagnostic channel is unavailable.

| id | control | input | pass iff | class on failure |
|---|---|---|---|---|
| **N1** | same-code / same-work null | one sentinel workload, two executions, phase B | `spread ≤ 0.09` | `INSTRUMENT-INVALID` if the two executions are not bit-identical in work; otherwise the observation is data, not a control failure |
| **N2** | back-to-back null without representative load | two sentinel executions with no load between them, phase A | recorded; **no pass threshold** — this is the reference condition, and a failure here is *data about the host*, routed to §7, not a control failure | — |
| **N3** | fixed-protocol replay on diagnostic seeds | **one session's** protocol re-executed with the same seeds and the same build, **before the six qualification sessions begin**, into its own journal `rc021_journal_n3.tsv` | identical **work** — sentinel execution count, sweep count, seed — and identical journal schema; wall times will differ and that is expected | `INSTRUMENT-INVALID` |
| **P1** | injected slowdown must be detected | a sentinel deliberately given extra work | the guard flags it; the injected spread must exceed `0.09` by construction | `INSTRUMENT-INVALID` |
| **P2** | deliberate abort after a journal write | kill the process after ≥1 measurement row | the journal on disk contains that row and the session reads back as `ABORTED` | `JOURNAL-INVALID` |
| **P3** | taxonomy test | every terminal program path exercised | each maps to **exactly one** §8 status; none maps to zero or two | `INSTRUMENT-INVALID` |
| **P4** | schema guard | the frozen header | contains no `energy`, `cut`, `best`, `quality`, `delta`, `objective` column | `INSTRUMENT-INVALID` |
| **P5** | seed refusal and disjointness | every family of §3.1 | each is refused by value; RC-021's four seeds are members of none and pairwise distinct | `INSTRUMENT-INVALID` |
| **P6** | provenance gate | this file | tracked, clean, committed, and its commit strictly precedes the instrument commit and every artifact | `REFUSED-BEFORE-MEASUREMENT` |
| **P7** | positive workload duration | the phase-C load block | its duration exceeds **1000 ×** the measured timer resolution | `INSTRUMENT-INVALID` |
| **P8** | per-measurement resolution floor | every sentinel execution | its duration is at least **100 ×** the measured timer resolution — the multiple RC-020 froze as `DEGENERATE_RESOLUTION_MULT` (`exp_marginal_cost.rs:66`), inherited here rather than newly chosen. A sentinel below the floor makes that measurement `LOST` | `INSTRUMENT-INVALID` if it holds for **any** whole phase; otherwise the individual measurement counts as a failure under §7.2 |

**N3 and P2 never enter the denominator.** They run before the qualification
sessions, write to their own journal paths, and none of their measurements is one
of the 90. The verdict is derived from the six session journals alone.

**Anti-vacuity, binding.** An instrument that returns `PASS` for everything, or
zero for every measurement, **must not pass this control set**. P1 and P3 exist
for exactly that purpose: a guard that cannot fail on demand is not a guard, and
a taxonomy in which no path can reach a failing status is not a taxonomy. If
either can be satisfied by a stub, the control set is defective and the cycle
stops.

**Diagnostic-channel fallback.** If a `diag_*` channel cannot be read reliably,
the instrument writes `NA`, sets the corresponding `diag_flags` bit, and
**continues**. No diagnostic channel is required for the primary statistic, and
the absence of one is never a qualification failure — but it is never silently
omitted either.

**Availability under WSL2 is unverified and is recorded here as a hypothesis, not
a plan.** A read of the current instrument performed for this document finds it
measures only load average (`exp_marginal_cost.rs:418`), thread count (`:425`),
CPU affinity (`:436`) and timer resolution (`:398`). For the three
diagnostic channels this document names, the prior is:

| channel | plausible route | confidence it is obtainable in WSL2 |
|---|---|---|
| process CPU time | `/proc/self/stat` or `getrusage` | **medium** |
| involuntary context switches | `getrusage(RUSAGE_SELF)` | **medium** |
| CPU frequency | `/sys/.../cpufreq/scaling_cur_freq` | **medium** — WSL2's virtualised kernel often does not expose real host `cpufreq` nodes |
| temperature | host thermal sensors | **low** — no direct hwmon access; would need a Windows-side helper |

Temperature is therefore **not** in the frozen schema, and no verdict may ever
depend on a channel whose availability was not established first. An
implementation step must determine availability empirically **before** relying on
any of them, and must not add a dependency to obtain one (§9 criterion 11).

## §7. Statistical decision rule — fixed before any datum

### 7.1 The arithmetic that fixes the numbers

Performed before choosing the counts, using **only**: the inherited `0.09`
bound, exact binomial arithmetic, and the structure a successor cost cycle would
inherit from RC-020 §5 (nine repetitions, at most two discarded). **No RC-020
timing measurement is used.**

Let `p` be the probability that one paired measurement satisfies
`spread ≤ 0.09`.

**The model is a deliberate simplification, and its direction is stated.**
RC-020 §5 is not a plain "nine repetitions, at most two discarded": a failing
repetition is **re-executed once**, and the session dies on a *twice-failed*
repetition — which is in fact what ended RC-020, rather than the discard counter.
The binomial below models the no-retry case. A retry can only raise session
survival for a given `p`, so the requirement derived here is **stricter than a
retrying successor would need**. The correspondence is therefore conservative and
approximate, **not exact**, and this document does not claim otherwise.

A nine-repetition session tolerating at most two failures, without retry,
survives with probability

```
P(session) = Σ_{k=0}^{2} C(9,k) (1−p)^k p^(9−k)
```

| `p` | 0.80 | 0.85 | 0.90 | 0.925 | 0.95 | 0.99 |
|---|---|---|---|---|---|---|
| `P(session)` | 0.7382 | 0.8591 | 0.9470 | 0.9749 | 0.9916 | 0.9999 |

- `P(session) ≥ 0.95` requires **`p ≥ 0.9023`**.
- `P(session) ≥ 0.99` requires **`p ≥ 0.9467`**.

RC-021 adopts the **0.95 target**, so the host must be certified at
**`p ≥ 0.9023`**.

**Sample size.** With `N` paired measurements and `k` failures, the exact
one-sided 95% Clopper–Pearson lower bound on `p` is:

| `N` | `k=0` | `k=1` | `k=2` | `k=3` |
|---|---|---|---|---|
| 30 | 0.9050 | 0.8514 | 0.8047 | 0.7614 |
| 60 | 0.9513 | 0.9234 | 0.8988 | 0.8758 |
| **90** | 0.9673 | 0.9484 | **0.9317** | 0.9161 |
| 120 | 0.9753 | 0.9611 | 0.9485 | 0.9367 |

**The minimum, stated honestly.** The smallest `N` that clears `0.9023` at
`k ≤ 2` is **`N = 63`** (lower bound `0.9034`); `N = 60` gives `0.8988` and does
not clear it. An earlier draft of this document said `N = 90` was "the smallest of
the tabulated sizes", with the table chosen as 30/60/90/120 so that 90 was the
answer. **Independent review called that evasive and it was.** The correction is
recorded rather than quietly repaired.

**`N = 90` is therefore a choice above the minimum, and the reasons are:**

- it factors as `6 sessions × 3 phases × 5 pairs`, so the protocol of §4 has equal
  weight in every phase of every session — 63 does not factor into the design;
- it buys margin: a `0.9317` lower bound against a `0.9023` requirement, versus
  `0.9034` at `N = 63`, which clears by `0.0011`;
- extra measurements are conservative in the safe direction: more evidence, a
  stricter bound, and no additional freedom for the analyst.

**What may not be said:** that 90 was forced by the arithmetic. It was not.

**Power — the gate must be able to pass a good host and fail a bad one.** The
gate is the **conjunction** of §7.2's rules 1–4, not rule 2 alone, so the power
figures must be computed for the conjunction. An earlier draft tabulated rule 2
and narrated the result as the gate's power; independent review caught it, and
the error was in the **unsafe direction** — it understated the chance of a false
`HOST-NOT-QUALIFIED`, and §11 makes a FAIL terminal.

With `90 = 6 sessions × 15 measurements`, rule 3 excludes the two-failure
configurations that fall inside one session. Of the `C(90,2) = 4005` two-failure
placements, `6 × C(15,2) = 630` are same-session and are rejected, leaving `3375`.

| true `p` | rule 2 alone | **rules 2 ∧ 3 (used)** | cost of rule 3 |
|---|---|---|---|
| 0.99 | 0.9381 | **0.9120** | −0.0260 |
| 0.98 | 0.7312 | **0.6886** | −0.0426 |
| 0.95 | 0.1664 | **0.1492** | −0.0173 |

A genuinely reliable host (`p = 0.99`) passes **91.2%** of the time; a host at
`p = 0.95` passes **14.9%**. The gate is neither automatic-pass nor
automatic-fail, and it discriminates — but see the operating-point note below
before reading `p = 0.95` as "marginal" in any absolute sense.

**Rules 1 and 4 are not modelled and lower these figures further.** Rule 1
requires all six sessions to reach `COMPLETED`, and rule 4 requires every control
to pass; neither has a probability model here, because neither is a property of
`p`. **The tabulated values are therefore upper bounds on the gate's true
power**, and a `HOST-NOT-QUALIFIED` verdict is correspondingly more likely than
the table alone suggests. This is stated rather than left to be discovered.

**The gate demands materially more than the rate it certifies, and that is
deliberate.** Two different quantities are easy to conflate, so both are stated:

| quantity | value |
|---|---|
| what a PASS **certifies** (95% lower bound at `k ≤ 2`, `N = 90`) | `p ≥ 0.9317` |
| what a successor session **needs** (§7.1) | `p ≥ 0.9023` |
| what the gate **effectively demands** to pass with better than even odds | `p ≈ 0.975` |

Power of the full rules-2∧3 gate across the range:

| true `p` | 0.9023 | 0.9317 | 0.95 | 0.97 | 0.98 | 0.99 |
|---|---|---|---|---|---|---|
| P(pass) | **0.0048** | 0.0442 | 0.1492 | 0.4522 | 0.6886 | 0.9120 |

**A host that merely meets the `0.9023` requirement will essentially never be
qualified** — 1 run in 200. This is not an oversight and is not corrected by
loosening `k`: at `N = 90`, `k ≤ 3` still certifies `0.9161 ≥ 0.9023` and only
lifts the same figure to `0.0130`. The `≤ 2 of 90` rule simply *is* a demand for
a much quieter host than the bare successor requirement.

It is accepted because RC-021 qualifies a **measuring station**, not a single
run: a host at `p = 0.9023` gives a successor only a 95% chance per session, and
a cost cycle needs many sessions. Demanding `p ≈ 0.98` of a station is
proportionate. But the consequence must be read plainly — **`HOST-NOT-QUALIFIED`
is the likely outcome for any host that is merely adequate**, and under §11 that
verdict is terminal for wall-time measurement on this configuration. Where the
earlier text called `p = 0.95` "a marginal host", that host in fact *exceeds* the
stated requirement; it is marginal against this gate, not against the science.

**A second consequence, decided consciously rather than discovered.** Under
§8.2's final branch, a single unrelated crash with no preceding `EXTERNAL-CAUSE`
row fails rule 1, which yields a published Class II `HOST-NOT-QUALIFIED` and,
through §11, closes wall-time measurement on this configuration. That is
deliberately conservative — an instrument whose sessions do not complete is not a
reliable instrument — but it is sharper than the power table alone implies.

**Why RC-021 is not split into a calibration block and a descendant.** A split
is the correct structure when a threshold cannot be justified before data, since
it lets calibration fix the number and a committed descendant fix the test. That
condition does not hold here: the per-measurement threshold is **not chosen by
this document at all** — it is the inherited `0.09` (§2) — and the minimum sample
size follows from the arithmetic above. Nothing about the decision rule is left to
be selected after seeing data, so a descendant would add ceremony without adding
protection. This justification is self-contained and cites no source outside the
committed repository.

### 7.2 The decision rule

**Primary statistic:** the number of the **90** paired measurements whose
`paired_spread` exceeds `0.09`.

**Aggregation:** across all six sessions and all three phases, pooled. Phase and
session are recorded and reported **descriptively**; see §7.3.

**`HOST/INSTRUMENT QUALIFIED` iff all four hold:**

1. all six sessions reach `COMPLETED` (§5.2);
2. **failures ≤ 2 of 90**, where a failure is `paired_spread > 0.09`;
3. **no single session contributes more than 1 failure** — so a pass cannot rest
   on five good sessions carrying one bad one, and cannot rest on one lucky
   session at all;
4. every mandatory control of §6 passed.

**`HOST/INSTRUMENT NOT QUALIFIED`** otherwise, provided the instrument and
journal were valid — that is a **result**, published as such.

**Treatment of incomplete measurements.** A measurement that is degenerate,
truncated, or lost counts as a **failure** for rule 2. The denominator is fixed
at 90 and never shrinks. This is deliberately the conservative direction: an
instrument that loses measurements is not a reliable instrument.

**Multiplicity.** Exactly **one** contrast is tested — rule 2 against the `0.09`
bound. Phase and session comparisons are **descriptive only** and are not tested
contrasts, so no correction applies and none may be introduced later by
promoting a descriptive comparison to a test.

### 7.3 Reported separately, never as a verdict

- **Within-block spread:** the distribution of `paired_spread` inside each phase.
- **Between-session stability:** the per-session failure counts and the spread
  quantiles per session.

These are published because a host that passes rule 2 while showing a strong
phase effect is qualified *and* interesting. Neither may change the verdict.

### 7.4 Prohibitions

No observation is dropped post hoc. No threshold is moved. No session is
excluded. The verdict is a function of the journal alone, and re-deriving it from
the committed journal must reproduce it exactly.

## §8. Taxonomy — every terminal path has exactly one status

| status | class | is it a result? | what may be claimed | next cycle | seeds consumed |
|---|---|---|---|---|---|
| `REFUSED-BEFORE-MEASUREMENT` | I | **no** | nothing; the run did not start | re-attempt permitted after the refusal is fixed | none |
| `INSTRUMENT-INVALID` | I | **no** | nothing about the host | instrument must be repaired under a fresh pre-registration | RC-021 diagnostic seeds only |
| `JOURNAL-INVALID` | I | **no** | nothing at all — evidence integrity failed | as above | RC-021 diagnostic seeds only |
| `HOST-NOT-QUALIFIED` | II | **yes, publish** | this host configuration does not meet the inherited bound at the certified rate | see §11 FAIL | RC-021 diagnostic seeds only |
| `HOST-QUALIFIED` | II | **yes, publish** | exactly the sentence in §1 | see §11 PASS | RC-021 diagnostic seeds only |
| `INCONCLUSIVE-UNDERPOWERED` | I | **no** | nothing | **terminal within RC-021. No restart is permitted.** See §8.1 | RC-021 diagnostic seeds only |

### §8.1 `INCONCLUSIVE-UNDERPOWERED`, defined so it cannot be abused

An earlier draft of this document allowed an externally caused abort to void the
run and restart it from session 1 with the counter reset. **Independent review
rejected that, and the rejection is accepted.** An uncapped void-and-restart
contradicts §2 ("No retries. No discarded-and-rerun sessions… Every planned
execution stays in the denominator") and reinstates precisely the bias that
`RC020_PILOT_ABORT_RECORD.md` makes binding on every successor: repeating until a
guard admits the run selects an atypically quiet moment. A route around a method
rule is a violation of it, however carefully worded.

**The rule, replacing it:**

1. `INCONCLUSIVE-UNDERPOWERED` is **terminal within RC-021**. The cycle ends. No
   session is repeated, no counter is reset, and no seventh session exists.
2. Resuming the qualification question requires a **fresh pre-registration**,
   written before any new datum, exactly as a fresh cycle would be.
3. It is reachable **only** when a machine-checkable condition holds: an
   `EXTERNAL-CAUSE` row (§5.1) was written **and fsynced to the session journal
   before** the abort it justifies, naming the cause. An abort with no such
   preceding row is **not** externally caused, whatever anyone says afterwards.
4. The record must state that the run is underpowered and that **no
   qualification verdict exists**, never that the host "would have passed".

This makes the escape hatch checkable from committed bytes rather than from
operator testimony, and caps it at zero restarts. A reader can verify eligibility
by inspecting the journal alone.

### §8.2 A crash maps to exactly one status

RC-020's record shows how easily a crash acquires three plausible classifications.
The rule here is deterministic and leaves the operator no discretion:

```
if a mandatory control of §6 failed          -> INSTRUMENT-INVALID   (Class I)
else if a journal integrity rule failed      -> JOURNAL-INVALID      (Class I)
else if a preceding EXTERNAL-CAUSE row exists-> INCONCLUSIVE-UNDERPOWERED (Class I)
else                                         -> the session is ABORTED, its
                                                unwritten measurements count as
                                                failures, and §7.2 decides the
                                                run status
```

The tests are evaluated **in this order**, each is decidable from the journals and
the control log, and exactly one branch is taken. No crash is classified by
inspecting how the numbers were trending when it happened.

**RC-020's defect, closed.** Each situation below yields one journal-level status
and one contribution to the run-level status, and neither column is a narrative:

| situation | journal-level status | contribution to the run-level status |
|---|---|---|
| a repeated sentinel failure within a session | each row `OK`, `paired_spread > 0.09` | each counts as one failure under §7.2 rule 2 |
| more failures than §7.2 allows | rows `OK` | `HOST-NOT-QUALIFIED` |
| double-counting a retried repetition | **not reachable** — RC-021 has no retry loop; every row carries a unique `measurement_index` and the denominator is the fixed 90 | none |
| crash after the first journal row | written rows stand; `SESSION-CLOSE-ABORTED` absent, so the reader classifies the session `ABORTED` | §8.2's ordered test decides; by default the unwritten measurements count as failures |
| a missing diagnostic channel | `NA` + `diag_flags` bit, row still `OK` | **none** — never a failure |
| an incomplete session | unwritten measurements have no rows | those measurements count as failures; §7.2 rule 1 then fails, giving `HOST-NOT-QUALIFIED` |
| a control fails mid-run | the control log records it | `INSTRUMENT-INVALID`; measurements already written are retained as evidence but **no verdict is derived from them** |
| a journal SHA-256 missing from the record, or a failing fsync | — | `JOURNAL-INVALID` |
| the pre-registration is untracked, dirty, or committed after the instrument | no journal is created | `REFUSED-BEFORE-MEASUREMENT` |

**No Class I status may ever be written up as a Class II result**, and no Class II
verdict may be recorded while any Class I condition holds.

## §9. Kill criteria

Each of these ends RC-021 with the consequence it names. Criteria 1–6, 8, 10
and 11 stop the cycle immediately; criteria 7 and 9 fix the verdict rather than
halting a run in progress.

1. The journal does **not** survive a deliberate abort (P2 fails).
2. Any terminal program path maps to zero or to more than one §8 status.
3. Controls do not distinguish an injected slowdown (P1 fails).
4. Any provenance requirement of §10 fails.
5. Any forbidden, burned or reserved seed reaches execution.
6. An entire phase's sentinel executions fall below the **100 ×** timer-resolution
   floor of §6 P8 — the instrument is then resolving noise, not work. (A single
   measurement below the floor is `LOST` and counts as a failure; it does not stop
   the cycle.)
7. The host does not qualify within the fixed six sessions — this is
   `HOST-NOT-QUALIFIED`, **not** a licence for a seventh session.
8. Qualification would require repeating until a pass occurs.
9. **Leave-one-out instability.** After the verdict is derived, recompute it six
   times, each time omitting one session and its 15 measurements. If any
   recomputation turns `HOST-NOT-QUALIFIED` into `HOST-QUALIFIED`, the run is
   `HOST-NOT-QUALIFIED` and the leave-one-out table is published with it. This is
   an explicit post-verdict audit, distinct from §7.2 rule 3: rule 3 constrains
   how failures may be *distributed*, this constrains how much the verdict may
   *depend* on any one session.
10. **Diagnostic overhead perturbs the primary workload beyond a bound fixed in
    advance.** An earlier draft expressed this as "1% of the inherited `0.09`
    budget", i.e. a relative-time bound of `0.0009` — smaller than this host's own
    observed paired spread, hence unmeasurable, and an unmeasurable kill criterion
    is one that gets waived. Independent review flagged it and the criterion is
    replaced by a measurable one:

    > **Procedure, fixed here.** Before the qualification sessions, execute **30
    > sentinel pairs with diagnostics enabled and 30 with diagnostics disabled**
    > — 60 pairs, hence **120 sentinel executions** — interleaved, in phase-B
    > conditions. Take the **median** sentinel duration of each arm. The criterion
    > fails iff `|median_on − median_off| / median_off > 0.01`: a **1% bound on
    > the sentinel's own duration**, not on the spread budget.

    At the recorded timer resolution of roughly 20 ns, 1% of a ~2 ms sentinel is
    ~20 µs, about 1000× the resolution, so the bound is measurable. If it fails,
    diagnostics are removed, not tolerated. These 120 executions are a control and
    never enter the denominator of 90.
11. **A new heavy dependency, or any change to `Runtime`, the solver, or the
    operators, would be required.**

**Criteria 10 and 11 stop the work and escalate.** They may not be resolved
inside an implementation step: they require an explicit architecture decision
(an ADR) taken separately. A successor that quietly adds a crate or edits the
read-only core to make RC-021 work has violated this pre-registration.

## §10. Provenance

Recorded for every RC-021 run, in the journal header and in the results document:

`prereg SHA` · `instrument SHA` · exact command line · UTC and local timestamps ·
OS, WSL and kernel versions · CPU-visible topology · WSL processor and memory
configuration · CPU affinity · thread count · Rust version · build profile and
flags · sentinel corpus hash · **SHA-256 and row count of each of the six session
journals and of the N3 and P2 control journals** (§5.4) · observation artifact
SHA-256 and row count if one exists · exit status.

**Additionally required, because each is a first-order determinant of the
measurement and each was missing from an earlier draft:**

- the four RC-021 seed values of §3.2 — they are the run's determinism inputs;
- the **six measured idle intervals** of §4.1 — the gap between the last control
  execution and session 1, and each of the five gaps between consecutive
  sessions — against the 10-minute floor. The phases are *defined* by cumulative
  thermal and cache state, so this cooling time is not an environmental footnote
  but a parameter of the experiment;
- the **diagnostic-availability determination** of §6 — which channels were found
  obtainable, and by which route — since it gates what the schema can carry;
- the measured **timer resolution** of each session;
- the **diagnostic-overhead measurement** of §9 criterion 10;
- for any `INCONCLUSIVE-UNDERPOWERED` outcome, the `EXTERNAL-CAUSE` row and its
  journal offset, so eligibility is checkable from bytes (§8.1).

**Chronology, binding.** This pre-registration must be committed **strictly
before** the instrument code, before the controls are run, and before any
qualification datum exists. The instrument verifies this and **refuses**
otherwise (§6 P6).

## §11. Successor rule

**A PASS does not open Stage S3 and does not demonstrate cost identification.**
It licenses exactly one thing:

> writing a **fresh pre-registration** for a new marginal-cost cycle, with new
> science blocks of its own.

That successor inherits nothing from RC-020 except the closed record, and it may
not reuse RC-020's seeds, thresholds, instrument or descendant.

**A FAIL closes wall-time measurement on this host configuration.** It does
**not** license lowering the threshold and retrying, and it does not license a
seventh session. The permitted responses are: a materially different measuring
station, or an explicit move to a comparison budget that makes **no wall-time
claim at all** — chosen and pre-registered separately.

## §12. Implementation plan — future work, not this commit

No code is written by this commit. When implementation is separately authorised:

1. Write the journal layer first: create, header, append, flush, fsync, and the
   refusal to overwrite an existing path.
2. Add the §5.2 lifecycle rows and the reader that classifies `NOT STARTED`,
   `STARTED`, `COMPLETED`, `ABORTED`, including a truncated final line.
3. Implement P2 as an executable deliberate-abort test and prove the journal
   survives process exit.
4. Implement the seed constants, the §3.3 disjointness gate and the §10
   provenance gate, with the decisions as pure functions and unit tests.
5. Implement the sentinel and the paired-spread measurement; assert the schema
   carries no outcome column.
6. Implement the §4 host protocol — phases, counts, affinity, environment
   capture — with the diagnostic channels and their `NA` fallbacks.
7. Implement the remaining controls N1, N3, P1, P3, P4, P5, P7, with P1 and P3
   constructed so they can fail on demand.
8. Implement the §7 decision rule and the §8 taxonomy as pure functions over a
   journal, unit-tested on synthetic journals that must both pass and fail.
9. Independent review of the instrument against this document.
10. Only then, on a prepared host, execute the six qualification sessions —
    once, with no retries.
