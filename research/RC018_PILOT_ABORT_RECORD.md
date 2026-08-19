# RC-018 pilot — invalid-instrument abort record

**Status:** the pilot block ran to completion, but the instrument did not
implement a mandatory pre-registered guard. **RC-018 has no scientific verdict of
any kind, Gate A is not evaluated, and held-in and held-out are prohibited.**

This document records what happened. It proposes nothing.

## 1. Chronology, exact

| when | what |
|---|---|
| 2026-08-19T22:30:24+05:00 | `5f38bef` pre-registration committed |
| 2026-08-19T22:35:10+05:00 | `96dc2af` Amendment 1 — estimands repaired before instrumentation |
| 2026-08-19T22:37:28+05:00 | `44501b6` Amendment 2 — paired timing windows, corrected run counts |
| 2026-08-19T22:45:49+05:00 | `ce9e307` instrument committed; controls run and passed |
| — | independent code review returned PASS; pilot authorised |
| 2026-08-19 22:56:40+05:00 | pilot artifacts written; process exit status **0** |
| — | independent review of the pilot identified the §5 defect below |

The pre-registration and both amendments precede the instrument, and the
instrument precedes the data. That chronology is intact; it is not what failed.

## 2. What was produced

| artifact | sha256 | data rows |
|---|---|---|
| `experiments/rc018/rc018_pilot_designL.tsv` | `ba8c3c468f14d0700d8dbe937522499701564264de143f1cd7874488db8a90d6` | **3,456** |
| `experiments/rc018/rc018_pilot_designT.tsv` | `bc09e08fc6f06d88a6fcef4fd2f6d0e43fb1da878a14a01fd588a9d4f77aba66` | **6,912** |

Row counts match Amendment 2 §B2 exactly (each file carries one further header
line). Every seed appearing in either file is from `7001–7008` and no other. All
6,912 Design T observations are `degenerate = false`.

**Controls PASS.** Both at instrument commit time and again inside the pilot run:
P3 corpus hashes (all six match `PREREG_RC016.md` §4), P2/P1′ synthetic estimator
recovery and exact `k′`, N2′ zero-slope null, N1 same-code null (spread 0.013 ≤
0.09), N3′ paired vs uninterrupted matching state, energy bits and RNG probe for
both operators, P4 φ recovered independently to `|d| = 0.000000`.

The controls passing is precisely why this record is necessary: a passing control
set did not detect the defect, because the defect is a guard that was never
implemented rather than a guard that failed.

## 3. Binding invalidity — prereg §5 was not implemented

`PREREG_RC018_COST_IDENTIFICATION.md` §5 requires, as a host guard on the science
loop:

> **Drift sentinel.** A fixed sentinel condition is executed first and last in
> every repetition. If sentinel median drifts by more than **9%** across the
> session, the session is discarded and re-run; a discarded session is recorded in
> the results document with its reason, never silently dropped.

and, in the same section, that *"the instrument records host load context
alongside each session for the record."*

**Neither was implemented.** `src/bin/exp_cost_identification.rs` lines 917–1012
construct a deterministic per-repetition permutation of the condition list and
execute it. There is no sentinel condition, no first-and-last execution, no
session-level drift comparison, no 9% rejection, and no host-load capture. The
two artifact schemas confirm this from the data side: neither header carries a
sentinel, session, drift or host-load field, so the guard leaves no evidence and
could not be reconstructed after the fact.

**Aggravating fact, recorded rather than omitted.** The doc comment immediately
above the science loop states *"with a sentinel first and last."* The comment
asserts the guard; the code does not contain it. This is the defect class RC-012
recorded — a documented feature that silently does nothing — reintroduced here in
a new instrument, and it is the reason the divergence survived both the author's
own checks and an independent code review.

**Consequence.** The 9% drift bar of §5 was never applied to the science block.
Whether host drift did or did not affect these timings is **not known and cannot
be established from these artifacts**, because the quantity that would answer it
was never recorded. The instrument is therefore invalid for RC-018's purposes
under kill criterion **K1** ("any control in §4 fails: instrument invalid; no cost
claim; cycle ends"), and the pilot is an invalid-instrument run.

**No Gate A verdict exists.** Held-in (`8001–8008`) and held-out (`9001–9008`)
are prohibited.

## 4. Exploratory-only calculation — NOT a result

Everything in this section is **exploratory**. It is recorded for completeness of
the account, and it is **not** a scientific finding, **not** a Gate A evaluation,
and **not** usable as evidence for or against any hypothesis. It rests on data
from an instrument that failed its validity precondition.

Two further reasons it is not the frozen computation, stated so the numbers are
not mistaken for protocol output:

1. The instrument implements no analysis mode, so the frozen estimator of §6 was
   never executed; these figures come from an ad-hoc read-only script.
2. §6.4's confidence interval requires the deterministic `ChaCha8Rng` stream
   seeded by the §2 formula. The script used a different generator, so the A1 and
   A2 figures are not the pre-registered interval even in form.

Computed on the pilot artifacts:

| check | exploratory outcome |
|---|---|
| D1 strict Design L monotonicity `T(4) < T(8) < T(16) < T(24)` | **96/96** cells |
| A1 slope CI excludes zero | 12/12 cells |
| A2 CI half-width ≤ 20% of `\|b\|` | 12/12 cells |
| A3 max residual ≤ 10% of fitted `T(24)` | 12/12 cells |
| A4 `k′` identical across all eight seeds | **stable on G14, G22, G43; unstable on G1, G11, G32** |

The A4 instability is per-seed rounding movement, not a large spread: G1 gave
`k′ ∈ {12, 13}`, G11 `{9, 10}`, G32 `{9, 10}`.

Taken at face value this pattern would be **3 of 6** instances satisfying all four
criteria, against §7's requirement of **≥ 5 of 6 for both operators**. That would
correspond to kill criterion **K4** (`NOT IDENTIFIED`).

**K4 is not activated and must not be recorded as activated.** K1 precedes it:
instrument validity is a precondition for any verdict, and it failed. A kill
criterion reached through an invalid instrument is not a result, and treating this
pattern as `NOT IDENTIFIED` would attribute to the data a conclusion that the
instrument was not entitled to produce.

## 5. Seed disposition

- **`7001–7008` are burned.** They were executed and their per-instance timings
  were written to disk and read. They are not reusable as a pilot block for
  RC-018 or for any successor.
- **`8001–8008` (held-in) and `9001–9008` (held-out) are untouched.** No mode
  reached them, no artifact names them, and the gate refused nothing because
  nothing requested them.
- The prior-cycle blocks remain forbidden as recorded in the pre-registration
  §2: RC-016's `1001–3008`, RC-017's `4001–4008`, the burned `5001–5008` and the
  reserved `6001–6008`.

## 6. What this record does not do

It proposes no repair, names no replacement seeds, and recommends no next action.
Those are decisions for the maintainer, and making them here would let an
invalid-instrument run set the direction of the cycle that replaces it.

The raw artifacts are retained unmodified, under their recorded hashes, as the
evidence for this account.
