# PREREG — RC-021 Amendment 1: the gate is an engineering acceptance rule, not a statistical certification

**Status:** binding amendment to `research/PREREG_RC021_HOST_INSTRUMENT.md`
(committed `b090923`). Written **before any RC-021 instrument code exists and
before any RC-021 datum.** Documentation only: no code, no run, no
`experiments/rc021/`.

**Reason.** A post-commit statistical audit found three defects. All three are
verified below and all three are **accepted**. The root defect is §A1: RC-021's
90 measurements are **clustered by construction**, so the Clopper–Pearson
binomial certification the pre-registration built its justification on does not
apply to the protocol it justifies.

**Nothing about the frozen design changes.** The counts, the bound, the seeds,
the journal design, the taxonomy, the provenance requirements and every
non-claim stand exactly as committed. What changes is **what the document is
permitted to say the numbers mean.**

## Clauses superseded — and only these

| clause | status |
|---|---|
| §7.1, "RC-021 adopts the **0.95 target**, so the host must be certified at…" | **SUPERSEDED** by §A3 |
| §7.1, every use of "certifies", "certified rate", "clears" as a property of the protocol | **SUPERSEDED** by §A2 |
| §7.1, the row "what a PASS **certifies** … `p ≥ 0.9317`" | **SUPERSEDED** by §A4 |
| §8, the `HOST-NOT-QUALIFIED` cell "does not meet the inherited bound at the certified rate" | **SUPERSEDED** by §A4 |
| §9, kill criterion 9 (leave-one-out as a verdict-fixing criterion) | **SUPERSEDED** by §A5 |
| §9, preamble clause "criteria 7 and 9 fix the verdict rather than halting a run in progress" — false for criterion 9 once §A5 withdraws it | **SUPERSEDED** by §A5; read as "criterion 7 fixes the verdict" |
| §0, "One question, one estimand" and "the per-measurement rate at which `spread` stays within the inherited bound", insofar as either reads as a population parameter | **RE-LABELLED** by §A2 and narrowed by §A4 |
| `PROJECT_PLAN.md` Stage S1b, "the **minimum** sample size clearing it … margin (lower bound `0.9317` against `0.9034`)" | **SUPERSEDED** by §A2 |

Everything else in the pre-registration is unchanged and remains binding.

---

## §A1 — Pseudoreplication: the i.i.d. model is not established

### The finding, verified

The Clopper–Pearson interval requires **90 independent and identically
distributed Bernoulli trials**. RC-021's 90 measurements are neither, and the
pre-registration's own design section says why.

**They are not independent.** The 90 are nested in **6 sessions**, each a separate
process invocation with its own thermal trajectory, its own cache population and
its own 10-minute idle history (§4.1, §4.2). Measurements inside one session share
all of it. A session is a **cluster**, of size 15.

**They are not identically distributed.** The three phases are **deliberately
heterogeneous**. §4.1 defines them as cold, warmed steady-state, and sustained
load, and states that they "are *defined* by cumulative thermal and cache state".
A design whose stated purpose is to create three different conditions cannot
then treat its measurements as draws from one distribution. If
`P(spread ≤ 0.09)` differs by phase — which is precisely what phase C exists to
detect — then **there is no single scalar `p` to certify**, and the parameter the
interval is about does not exist.

### How badly this bites, quantified

For clustered Bernoulli data the variance inflates by the design effect
`deff = 1 + (m − 1)·ICC` with cluster size `m = 15`:

| intra-session ICC | design effect | effective `N` | still ≥ the document's own minimum of 63? |
|---|---|---|---|
| 0.00 | 1.00 | 90.0 | yes |
| 0.02 | 1.28 | 70.3 | yes |
| **0.05** | **1.70** | **52.9** | **no** |
| 0.10 | 2.40 | 37.5 | no |
| 0.20 | 3.80 | 23.7 | no |
| 0.30 | 5.20 | 17.3 | no |

An intra-session correlation of **0.05** already drops the effective sample size
below the `N = 63` that §7.1 itself computed as the minimum; the exact crossing is
at `ICC ≈ 0.031`. This design has **structural reasons to expect non-zero
clustering — of unknown magnitude**: measurements inside a session share a thermal
trajectory, a cache state and a process.

**Two disclaimers, because this table is easy to over-read.**

1. **No ICC is estimated here and none may be assumed.** The point is not that a
   particular correction should be applied; it is that the uncorrected interval is
   not a statement about this protocol.
2. **The comparison against `N = 63` is circular, and that is stated rather than
   hidden.** `63` is itself a Clopper–Pearson figure, derived inside the very
   i.i.d. model §A1 rejects, so this row measures a rejected model against its own
   yardstick. It **illustrates the fragility** of that model; it is **not an
   independent argument.** The independent argument is the first part of §A1 — no
   single scalar `p` exists — and it stands with this table struck entirely.

Two further reasons the table **understates** the damage, both safe for the
conclusion: the Kish design effect corrects variance but not degrees of freedom,
and with 6 clusters the cluster-robust `df ≈ 5` would undercover badly on its own;
and with phase as a fixed effect a single scalar ICC is not the right
decomposition at all.

### What would have been required

A defensible population-level inference would need a model with **session as a
random effect and phase as a fixed effect**, and an interval derived from it.
That is not bolted on here: with 6 clusters there is very little information to
estimate a session-level variance, so such a model would be weakly identified,
and adopting one now would be choosing an analysis after seeing the design's
weakness. **A future cycle that wants a population claim must pre-register that
model before its data.** RC-021 does not make a population claim at all (§A3).

## §A2 — The arithmetic is retained as an idealised sensitivity calculation

§7.1's tables are **not deleted**, because they did real work: they are why the
counts are what they are, and deleting them would hide the reasoning. They are
**re-labelled**.

Every figure in §7.1 — the `P(session)` row, the `p ≥ 0.9023` and `p ≥ 0.9467`
thresholds, the entire Clopper–Pearson table, `N = 63` as the tabular minimum, and
the power tables — is hereby designated:

> **IDEALISED i.i.d. SENSITIVITY CALCULATION.** It answers the question *"if the
> 90 measurements were 90 independent Bernoulli trials with a common success
> probability, what would follow?"* They are not, so **nothing in that table is a
> property of the RC-021 protocol.** The figures are retained to document how the
> counts were chosen and to bound the best case, and for **no other purpose.**

**Binding prohibitions.** No RC-021 document, commit message, report or summary
may state or imply that:

- the protocol certifies `p ≥ 0.9317`, or any value of `p`;
- a PASS establishes a confidence bound on the host's true failure rate;
- `N = 90` gives 95% confidence of anything about this host;
- the power figures are the protocol's power. They are the power of an idealised
  i.i.d. gate, and remain **upper bounds** for the additional reason already
  recorded in §7.1 (rules 1 and 4 unmodelled) **and now for this reason too.**

The arithmetic keeps a **narrower** design-rationale status: it supplies the
**floor `N ≥ 63` above which `6 × 3 × 5` was chosen**. It is not the reason the
counts are 90. §7.1 already established this and said so in terms — *"**What may
not be said:** that 90 was forced by the arithmetic. It was not"* — after an
earlier draft's contrary framing was called evasive. **This amendment must not
re-inflate the arithmetic's design credit at the moment it deflates its
inferential credit**, and the distinction is recorded here so it cannot.

## §A3 — What the gate actually is

RC-021's gate is a **frozen engineering acceptance rule**: a deterministic,
pre-registered predicate over the six journals. It is not an estimator and not a
hypothesis test.

The rule is **unchanged** from §7.2 — all six sessions `COMPLETED`, failures
`≤ 2` of 90, no session contributing more than 1, every control passed — and it
retains every property that made it worth having: it is fixed before data, its
denominator cannot shrink, it admits no post-hoc exclusion, and it cannot be
satisfied by one lucky session.

What it loses is only the claim that passing it *certifies a rate*. An acceptance
test says: **this apparatus, under this protocol, behaved this way.** That is a
smaller claim than the pre-registration made, and it is the claim RC-021 can
actually support.

### The internal witness that the gate was never that test

The pre-registration contains, in its own tables, a proof that the gate does not
implement the confidence-bound test it was narrated as implementing:

> At `N = 90` and `k = 3`, §7.1's idealised lower bound is **`0.9161`**, which
> **exceeds** the required `0.9023`. A host with 3 failures would therefore
> "certify" above the requirement on the document's own arithmetic — and the
> frozen gate, which allows `k ≤ 2`, **rejects it**.

A rule that rejects an outcome its own certification arithmetic would accept is
not that arithmetic's test. This was visible before any datum existed, and it is
recorded here rather than left for a reader to notice after a verdict.

## §A4 — Verdict semantics, restated exactly

The two Class II verdicts of §8 keep their names and their consequences. Their
**meaning** is narrowed to what the protocol supports.

**`HOST-QUALIFIED` means exactly:**

> The host and instrument **passed the frozen engineering qualification protocol**
> of this pre-registration, on this host configuration, on this occasion.

It does **not** mean that the true per-measurement success rate is at or above any
value, that a confidence bound was established, or that the host will behave this
way again.

**`HOST-NOT-QUALIFIED` means exactly:**

> The host and instrument **did not pass this protocol on this host
> configuration**.

It does **not** mean that the true `p` is below `0.9023`, that the host failed an
exact confidence-bound test, that the host is unsuitable in general, or that a
different protocol would also reject it. §A3's `k = 3` witness shows directly why
this distinction is not pedantic: a rejected host may be one the idealised
arithmetic would have accepted.

**§11 is unchanged in force but read through this narrowing.** A FAIL still
closes wall-time measurement on **this host configuration** under **this
protocol**, and still does not license lowering the threshold and retrying. It
was already scoped to the configuration; it is now also explicitly scoped to the
protocol.

> **This scoping describes what a FAIL *means*. It adds no permitted response
> beyond the two §11 enumerates** — a materially different measuring station, or
> an explicit move to a comparison budget making no wall-time claim. In
> particular it does **not** license writing a fresh protocol for the same host
> and trying again: that is retry-until-pass under another name, and §2 forbids
> it independently.

**§8's `HOST-NOT-QUALIFIED` row** is amended: the "what may be claimed" cell reads
*"this host and instrument did not pass this protocol on this configuration"*, and
the phrase "at the certified rate" is struck.

## §A5 — Leave-one-session-out becomes descriptive only

### The finding, verified

§9 criterion 9 instructed: recompute the verdict six times, each omitting one
session; if any recomputation flips `HOST-NOT-QUALIFIED` to `HOST-QUALIFIED`, the
run is `HOST-NOT-QUALIFIED`.

**It is not executable as written.** The verdict rules of §7.2 are defined for
**six sessions and 90 measurements**. On five sessions and 75 measurements,
**rules 1 and 2 are undefined**: rule 1 requires *all six* to complete, so it is
unsatisfiable or must be silently re-scoped; rule 2 allows `≤ 2` of **90**, and on
75 the allowance is neither obviously `2` nor obviously rescaled
(`2 × 75/90 = 1.67`). Rules 3 and 4 do transfer cleanly — rule 3 is stated per
session, and rule 4's controls live outside the sessions in their own journals —
but two undefined conjuncts make the **composite predicate undefined**, which is
sufficient.

**And it was outcome-vacuous even where it was defined.** Its antecedent — *"if
any recomputation turns `HOST-NOT-QUALIFIED` into `HOST-QUALIFIED`"* —
presupposes the derived verdict is already `HOST-NOT-QUALIFIED`, and its
consequent asserts *"the run is `HOST-NOT-QUALIFIED`"*. It could not change a
verdict in any world. Its only operative content was the instruction to publish
the table, which §A5 keeps. **§A5 is therefore a correction, not a concession**,
and §9's preamble claim that criterion 9 "fixes the verdict" was already false
before this amendment.

### The replacement

Leave-one-session-out is **removed as a kill criterion** and becomes a
**descriptive sensitivity table, published with every verdict and never changing
it**:

For each session `s ∈ 1..6`, report over the remaining 5 sessions and their 75
measurements **four quantities**: the **failure count**; the **maximum
per-session failure count**; the **median** of `paired_spread`; and its **95th
percentile**. Report the same four for the full 90 alongside.

**Estimator, fixed here so the table is reproducible:** both quantiles use the
**nearest-rank** definition — the smallest value at or above which at least
`⌈q·n⌉` observations lie — with **no interpolation**, since nearest-rank and
linear interpolation differ materially at `p95` on `n = 75`. **Rows with status
`LOST`** carry no valid `paired_spread`: they are **excluded from the two
quantiles** and their exclusion count is reported beside them, while they
continue to count as failures in the failure counts and under §7.2. The two
treatments differ deliberately, and both are stated.

**Binding:** this table is **descriptive**. The verdict is the §7.2 predicate on
all six sessions and 90 measurements, full stop. No cell of this table may change
it, and the table may not be used to argue that a verdict "would have been
different" — the frozen denominator exists precisely to forbid that argument.

Its purpose is honest disclosure: a reader can see whether the outcome was
concentrated in one session, which is information, not a decision rule.

**§9 renumbering is deliberately avoided.** Criterion 9 is retained as a numbered
slot reading *"withdrawn by Amendment 1 §A5; see the descriptive table"*, so that
references to criteria 10 and 11 elsewhere in the document remain correct.

## §A6 — Explicitly unchanged

- **6 sessions × 3 phases × 5 pairs = 90**, and the phase order A → B → C.
- The **inherited `0.09`** bound. Not lowered, not reinterpreted.
- The **fixed denominator of 90**; lost, truncated and below-floor measurements
  still count as failures.
- **No retries**, no discarded-and-rerun sessions, no seventh session, and
  `INCONCLUSIVE-UNDERPOWERED` still terminal with zero restarts and gated on a
  pre-abort `EXTERNAL-CAUSE` row.
- The **journal-first design**: per-session journals, created and fsynced before
  the first measurement, append-only, overwrite-refusing, parent directory
  fsynced; observation artifact only on success.
- **Seeds `31001–31004`**, band `[31001, 31099]`, and the disjointness gate.
- The whole of §8's taxonomy apart from the one amended cell, §10's provenance
  list, §1's non-claims, §11's successor rule, and §12's implementation plan.
- The **six idle intervals** of §4.1, including controls → session 1.

## Internal consistency check

| question | resolution |
|---|---|
| Does any claim of statistical certification survive? | No — §7.1's figures are re-labelled idealised in §A2, and §A2 lists the prohibited phrasings explicitly |
| Is the arithmetic deleted? | No — retained as design rationale and as a best-case bound, with its status stated |
| Does the gate change? | No — §7.2's predicate is byte-for-byte the rule it was |
| Can a verdict now claim something about true `p`? | No — §A4 fixes both verdicts to protocol-passage statements |
| Is leave-one-out executable? | It is no longer executed as a predicate; it is a descriptive table of four named quantities, with a fixed quantile estimator and a stated `LOST` rule, over a defined subset |
| Can the descriptive table change a verdict? | No — prohibited in §A5 |
| Do counts, bound, seeds or design move? | No — §A6 |
| Does this amendment license any new claim? | No. It **removes** a claim the pre-registration was not entitled to make and licenses nothing in exchange |
