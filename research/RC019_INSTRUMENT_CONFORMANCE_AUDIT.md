# RC-019 — instrument conformance audit through RC-018

**Status:** read-only audit, complete for the clauses in scope. No code was
edited, no experiment or control was executed, and no raw artifact was touched.
Every classification below is anchored to an executable `file:line` or to the
absence of one.

**What this audit asks.** For each pre-registered cycle that has an instrument:
is every binding guard named by the document actually present in executable code,
and does it leave evidence? It does **not** re-examine any statistical result.

---

## 1. Method, and the positive control that licenses it

The method is three steps, applied per clause: locate the binding sentence in the
pre-registration; search the instrument for an **executable** occurrence, not a
comment; check whether the artifact schema carries a field recording that the
guard ran.

**A method that cannot rediscover a known defect proves nothing**, so the audit
was gated on reproducing one. The known defect is the RC-018 drift sentinel,
recorded in `RC018_PILOT_ABORT_RECORD.md`.

| step | result |
|---|---|
| clause | `PREREG_RC018_COST_IDENTIFICATION.md:150-151` — "A fixed sentinel condition is executed first and last in every repetition" |
| executable occurrence | **none.** The only occurrence in `src/bin/exp_cost_identification.rs` is the doc comment at `:917` |
| artifact evidence | **none.** Neither `rc018_pilot_designL.tsv` nor `rc018_pilot_designT.tsv` carries a sentinel, session, drift or host-load field |

The method reproduced the defect. It also independently flagged the second §5
clause — `:160`, "the instrument records host load context alongside each
session" — with zero occurrences of any host-load capture. The audit is therefore
licensed to report on the remaining cycles.

## 2. Scope

Four pre-registration families with instruments, plus the positive control.

| cycle | instrument |
|---|---|
| RC-014 | `src/bin/exp_counterfactual.rs` |
| RC-015 | `src/bin/exp_tie_handling.rs` |
| RC-016 | `src/bin/exp_sensor_sufficiency.rs` |
| RC-017 | `src/bin/exp_sensor_sufficiency.rs`, `--rc017-*` modes |
| RC-018 | `src/bin/exp_cost_identification.rs` (positive control) |

**Clause classes audited:** mandatory controls, control-blocking (gating), and
provenance/descendant gates. **Not audited:** estimator definitions, statistical
thresholds, corpus and seed clauses, and analysis code. Their absence from this
document is not a finding about them.

### RC-017's placement, corrected

RC-017's instrument is **not** a separate historical binary requiring retrieval
from `4d9a1b1`. It is in the current lineage, inside `exp_sensor_sufficiency.rs`,
under the `--rc017`, `--rc017-controls`, `--rc017-science`, `--rc017-holdout` and
`--rc017-verdict` flags. That file therefore carries the instruments of **two**
cycles. This is recorded because an auditor following the abort record's
chronology would otherwise look for a binary that does not exist.

## 3. Conformance matrix

Classifications: **I+E** implemented and evidenced · **I-NE** implemented but not
evidenced · **ABSENT** · **VACUOUS** (present as text or output, without
executable effect) · **N/A**.

| cycle | binding clause | class | evidence |
|---|---|---|---|
| RC-014 | six Gate-A controls (`PREREG_RC014.md` §7) | **I+E** | `exp_counterfactual.rs:322` `probe_draw_alignment`, `:377` `synthetic_arithmetic_positive`, `:418` `null_control`, `:443` `inert_a`, `:469` `inert_b`, `:654` `synthetic_operator_positive`, `:766` `real_positive`; invoked `:1176-1204`, each printing PASS/FAIL |
| RC-014 | RNG-shift detection must be **detected**, not absorbed (§7) | **I+E** | `:348-371`, `shift_detected = pa != pc`, blocking via the returned `aligned && shift_detected` |
| RC-014 | "Controls — all four are Gate-A blocking" (§7 heading) | **ABSENT** | `:1172` gates the control block on `flag("--controls") \|\| !(flag("--science") \|\| flag("--holdout") \|\| flag("--transitions"))`. With `--science` alone the condition is false and the entire control block is skipped; `:1217` then runs the science block. No executable guard makes controls a precondition |
| RC-014 | held-out requires the falsifiable descendant already written | **VACUOUS** | `:1225-1226` prints "Gate A condition 6: the falsifiable descendant must already be written" and proceeds unconditionally. No existence, tracked, clean or chronology check |
| RC-015 | three blocking controls (`PREREG_RC015.md:125-131`) | **I+E** | `exp_tie_handling.rs:426` `control_null`, `:453` `control_alignment` (carrying the injected one-word shift), decomposition closure asserted as `c.closes == 0.0` |
| RC-015 | "Controls, all blocking" (`:125`) | **ABSENT** | `:800` carries the identical condition to RC-014's; `:842` runs `--science` with the control block skipped |
| RC-016 | controls precede every science mode (`PREREG_RC016.md` §9) | **I+E** | `exp_sensor_sufficiency.rs:3360-3366`, unconditional `if !run_controls(&reg) { std::process::exit(1); }` executed before any mode dispatch, with the superseded pattern named in the comment at `:3357-3359` |
| RC-016 | calibration frozen before any science seed (§8.1) | **I+E** | `require_frozen_calibration`, invoked at the head of `--science` and `--holdout` |
| RC-016 | held-out descendant gate | **I+E** | `descendant_gate`, five independent conditions, refusing with a non-zero exit |
| RC-017 | controls precede every science mode | **I+E** | `:3296-3301`, unconditional `if !run_controls_rc017(&reg) { std::process::exit(1); }` |
| RC-017 | pre-registration provenance gate on held-in and held-out (`PREREG_RC017.md` §3) | **I+E** | `require_prereg_rc017` at `:3306` and `:3319`, each refusing with exit 2 |
| RC-018 | §5 drift sentinel | **ABSENT** | §1 above |
| RC-018 | §5 host load context | **ABSENT** | §1 above |

### A cross-cutting observation on evidence

For RC-014 and RC-015 the blocking property is absent from code **and** unrecorded
in the artifacts: no schema carries a field asserting that controls ran before a
given science block. The two facts compound — there is neither a guard nor a
trace.

For RC-016 and RC-017 the guard is in code and cannot be bypassed by flag
selection, so an artifact field would be redundant to the guarantee. Their
classification does not depend on one.

## 4. Procedural weakness versus invalidated claim

These are different, and the distinction is the point of this document.

### Procedural weakness — RC-014 and RC-015

Both cycles' controls **exist, are complete against their own pre-registered
lists, and are individually blocking when executed**: each returns a boolean that
forces a non-zero exit. What is missing is that the instrument does not make
running them a *precondition* of the science modes. "Blocking" is therefore an
operator convention rather than a machine guarantee.

**This does not invalidate RC-014's or RC-015's results.** The audit found no
control that is wrong, missing, or vacuous in either cycle; it found that the
sequencing is unenforced. Their published records state that the controls were run
and passed, and nothing here contradicts that.

**What can no longer be established from the artifacts alone** is that controls
preceded every science invocation, because no artifact records it. The claims
stand on the records; they do not stand on the instruments' own guarantees.

**Materially different, and more than procedural: RC-014's held-out descendant
condition is VACUOUS.** A printed reminder is not a gate. The held-out block was
reachable without the descendant existing, so the chronology that RC-014's A6
replication logic depends on is asserted by the record rather than enforced by the
instrument. This is recorded as the strongest finding of the audit that does not
invalidate a result.

### RC-016 — stronger, and the core claim is unaffected

RC-016 is the only family in scope whose controls gate every mode unconditionally,
and it additionally carries a frozen-calibration precondition and a five-condition
descendant gate. No conformance gap was found in any clause audited.

**RC-016's core equal-sweep claim is therefore valid as far as this audit
reaches**: nothing in the instrument's guard structure undermines it. The audit
scope is the qualifier — statistical and estimand clauses were not examined, so
this is an absence of conformance findings, not an independent re-validation of
the result.

### RC-017 — no conformance finding; the abort is unexplained by this audit

RC-017's guards are implemented and enforced. The audit therefore does **not**
supply a conformance explanation for the RC-017 abort recorded in
`RC017_ABORT_RECORD.md`. That abort remains undiagnosed on the evidence available.

### RC-018 — the one invalidated claim, already recorded

The RC-018 pilot is an invalid-instrument run under kill criterion K1, with no
Gate A verdict, as recorded in `RC018_PILOT_ABORT_RECORD.md`. This audit
reproduces the underlying defect as its positive control and adds nothing to that
disposition.

## 5. The pattern, stated as fact

Unconditional control gating appears in exactly the two later instruments,
RC-016 and RC-017. The two earlier ones, RC-014 and RC-015, share one condition
shape that skips controls whenever a science mode is invoked without
`--controls`; the RC-016 code comments name that shape as a defect it corrected.
RC-018, written after all four, contains a different instance of the same class —
a guard asserted in a doc comment and absent from the code.

Counting the `--early-stop` defect recorded in RC-012, this class — a documented
guard with no executable effect — has now occurred in four places in this
repository.

## 6. Confidence, and the check this audit did not perform

**High** on every code fact: each `file:line` was read directly, and each ABSENT
classification rests on an exhaustive search of the named file rather than on
failure to find.

**Medium** on the practical severity of the RC-014 and RC-015 gating gaps. The
decisive fact was not established here: whether the reproduction commands cited in
`RESEARCH_INVENTORY.md` pass `--controls` together with `--science`. If they do,
the gap is documentary in effect; if they do not, the affected runs have no
machine evidence that controls preceded them. That check was outside this audit's
scope and its absence is recorded rather than resolved.

## 7. What this document does not do

It proposes no repair, recommends no backport, retracts no result, and names no
next cycle. It records what is in the code and what is not.
