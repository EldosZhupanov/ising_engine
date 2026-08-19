# PREREG — RC-016 Amendment 4: withdrawal of the equal-cost arm

**Status:** binding amendment to `PREREG_RC016.md` as amended by Amendments 1–3.
Written **after the canonical calibration and before any held-in or held-out seed
has been run.** Documentation only: no instrument code changed by this commit.

**Why an amendment is required rather than a note.** Calibration was pre-registered
as the step that decides whether the equal-cost arm runs (Amendment 1 §A5.3). It has
now run, and it answers that question in a way the pre-registration did not
anticipate: the escape hatch is closed on every instance, and the arm the
pre-registration would therefore mandate is **not identifiable by the design that
mandates it**. Recording that silently in a results write-up would be a post-hoc
protocol change. It is recorded here, before the data, instead.

---

## D0 — Provenance of the pilot data, recorded in two separate parts

These two events are distinct and are **not** merged into a single narrative.

**Part 1 — an unauthorised execution during a read-only review.** During a
read-only decision review that explicitly forbade running calibration, a
`worker_done` message was composed in a shell string that contained the calibration
command inside backticks. Inside double quotes, bash performed command substitution
and **executed it**. The run completed and wrote a calibration table. Its **stdout
was displayed**, so per-instance pilot quantities were exposed at that moment. The
artifact was **not read** and was **deleted**; no canonical artifact survived, and
the repository was verified clean at `e65ac10`. The cause was a quoting defect, not
a judgement call, and the honest characterisation is that the read-only constraint
was violated.

**Part 2 — the canonical, maintainer-authorised run.** The maintainer subsequently
instructed that the canonical calibration be run. It was run, controls passed, and
it produced the artifact cited in §D1. This is the run of record.

**The consequence, stated plainly and not minimised.** Pilot information was
**observed before the canonical run**, and observation cannot be undone. Two
properties limit the damage and are stated as facts, not as exoneration:

1. The pilot seeds `3001–3008` are **disjoint** from held-in `1001–1008` and
   held-out `2001–2008`, so no science datum was touched by either execution.
2. The instrument is deterministic, so the canonical run reproduces the earlier
   numbers exactly; the exposure conveyed the same information the authorised run
   was always going to publish, only earlier and out of order.

What is **not** claimed: that the exposure was harmless in principle. Anyone
relying on this record should know that the calibration numbers were seen by the
implementing agent before the authorised freeze, and should weigh the amendments
written afterwards — including this one — with that knowledge.

## D1 — The calibration of record

| field | value |
|---|---|
| artifact | `experiments/rc016/rc016_calibration.tsv` |
| sha256 | `2905eefd54c90c9b39dd80c77f7762904d24cfe845c8f44a183e3bbf4531e424` |
| size / mtime | 2061 bytes · 2026-08-19 20:04:49 +0500 |
| rows | 30, in the `PREREG_RC016.md` §4 corpus order |
| controls | ALL PASS; operator positive 648/648 against the independent reference |
| pilot seeds | 3001–3008, interleaved conditions, medians |

`R` is defined in the instrument as `median_ms(gibbs) / median_ms(metropolis)` —
measured wall time at the call site. `cost_model` is forbidden as a cost measure
(RC-005; Amendment 1 §A5.3).

**Summary of record:**

- **`R` band `[0.95, 1.05]`: 0 of 30 instances.** Observed range **1.0535 (G3) to
  1.6830 (G12)**. The heat-bath arm is between 5% and 68% slower in wall time.
- **`cost_arm_needed`: 30 of 30.**
- **`powered`: 4 of 30** — G48, G49, G50, G63. **26 of 30 underpowered**, with
  rejection rates as low as 0.0520 (G13).

**A finding of this review, not present in the coordinator's summary.** Three of
the four `powered` instances — **G48, G49, G50** — have `d_seed_pilot` **exactly
0.000000**. That is precisely the `DEGENERATE_NULL` condition of the materiality
rule (`d_seed < 1e-9`, instrument line 403). Their `power = 1.0000` is an
**artifact of zero dispersion**: the bootstrap resamples centred residuals that are
all identically zero, so every resampled dataset is the constant shift `Δ` and the
sign-flip test rejects with certainty. It is not evidence of sensitivity. The power
rule and the materiality rule therefore **disagree on the same three cells** — one
calls them maximally powered, the other calls them degenerate.

**Corrected count: exactly 1 of 30 instances (G63) is genuinely powered.** Every
statement below that depends on power uses this corrected count. The raw `powered`
column is retained in the artifact unaltered; it is interpreted here, not edited.

## D2 — The equal-cost arm is WITHDRAWN

Amendment 1 §A5.3 provided that if `R` fell inside `[0.95, 1.05]` no cost arm was
needed, and otherwise an equal-cost arm at `k′` sweeps would be run. The first
branch is now dead on all 30 instances. The second branch is withdrawn, for three
independent reasons, each sufficient alone.

**1. `k′` from a total-cost ratio does not identify equal operator cost.**
Whole-policy wall time is
`T(k) = a + b·k`,
where `b·k` is the swept operator and `a` is the **`k`-independent greedy
finisher** that every plan runs at fixed budget `SWEEPS`. `R` is a ratio of
**totals**, `T_G(16)/T_M(16)`, so it is contaminated by `a`, which is common to
both arms and cancels from no ratio of this form. Setting
`k′ = round(16 / R)` equalises *whole-policy* time, not *operator* time; the
implied per-sweep ratio is recovered only as `b_G/b_M`, which requires `a` and `b`
to be separated by timing at ≥2 values of `k`. That was never pre-registered, and
the calibration as frozen does not contain it. **`k′` is therefore not identified
by the quantity the design uses to compute it.** This is the same defect class as
RC-005's cost-model blindness, and it is why `cost_model` is already forbidden.

**2. The 8 pilot seeds cannot both select `k′` and estimate its power.** Using the
same pilot block to choose `k′` and then to certify the arm's sensitivity is
selection on the outcome. Splitting the block does not rescue it: the exact
paired sign-flip test at the pre-registered α needs **≥6** paired observations
before it can reject at all (`p_floor = 2/2^K`), so a power-bearing subset consumes
at least 6 of 8, leaving ≤2 for selection — too few to estimate a ratio. The seed
sets are frozen by pre-registration and **may not be extended** to escape this.

**3. The arm would be underpowered where it is not degenerate.** With the corrected
count, **1 of 30** instances can detect an effect of size `Δ`. An arm that is
uninformative on 29 of 30 cells is not worth the protocol risk of running it.

**Binding effect:** the equal-cost arm **must not be run in RC-016**, on any
instance, including the four flagged `powered`. `cost_arm_needed = true` in the
frozen artifact is retained as the honest record of what the superseded rule
computed; it no longer authorises anything.

## D3 — The surviving estimand, and what may not be claimed from it

**`I_replace_work` under EQUAL SWEEPS remains the sole primary estimand of
RC-016**, exactly as specified in `PREREG_RC016.md` §2 and Amendment 1 §A2, with
`QUALIFIES` = BH rejection ∧ materiality ∧ A6 replication, unchanged.

**Binding prohibitions.** Because RC-016 now measures no cost quantity at all, no
result of this cycle may be stated as, or paraphrased into:

- an **equal-cost** or cost-normalised comparison of the two operators;
- a **deployment-optimal** choice, recommendation, or default;
- a **scheduler-utility** claim, or any claim about what a scheduler, portfolio, or
  policy *should* select;
- any wall-time, throughput, or efficiency superiority in either direction.

RC-016 answers "does replacing the operator at equal sweeps change the outcome",
and only that. The cost question is not answered, not partially answered, and not
bounded. It is left open, and any future cycle addressing it must first identify
`a` and `b` separately by timing at ≥2 values of `k` on frozen seeds disjoint from
all three RC-016 blocks.

## D4 — Finding 5 resolved: the SIGN VARIES / SIGN CONSTANT asymmetry

The ambiguity is resolved explicitly, because as written it made `SIGN VARIES`
unreachable below `K = 6` and thereby silently converted a real pattern into a
non-result.

**`SIGN VARIES`** requires **`k₊ ≥ 1` and `k₋ ≥ 1`** among qualified, material
instances — at least one qualifying instance of each sign. **There is no `K ≥ 6`
floor and no structural-coverage requirement.** Rationale: a single qualified,
material instance of each sign is already a constructive existence proof that the
sign is state-dependent, and an existence claim needs one witness per side, not a
population.

**`SIGN CONSTANT`** requires **`K ≥ 6`** qualified material instances **and** the
pre-registered structural coverage. Rationale: this is a universal claim over the
corpus, and a universal claim does need a population and needs the corpus spanned.

**Every other qualified-material pattern routes to `Q-INCONCLUSIVE`** — including
`K ≥ 1` all of one sign but `K < 6`, and any `K ≥ 6` that fails structural
coverage. `TIE-BLOCKED` continues to route to `Q-INCONCLUSIVE` per Amendment 2 §B4.

The asymmetry is deliberate and is recorded as such: the two verdicts are claims of
different logical type, and holding an existence claim to a universal claim's
evidentiary bar is what produced the defect.

## D5 — The novelty label, frozen before the verdict

Frozen now, before any science datum, exactly as the Peskun label was frozen before
RC-014's write-up, so the framing cannot be chosen after seeing the result.

**KNOWN, and not claimed as novel by RC-016:** per-instance algorithm selection;
algorithm portfolios; dynamic algorithm configuration (DAC); the general finding
that the best algorithm depends on the instance, and that features can predict it.
This is an established field with its own benchmarks and literature.

**What an `S₀` opposite-action matched pair would be:** a **domain-specific causal
counterexample** and a **methodological result** — evidence that a *specific frozen
feature set* (`FeatureRegistry::v0`, five scalars) maps two instances to
near-identical descriptors while the causally correct action differs in sign, under
an exact paired counterfactual. Its contribution is the **exactness of the
counterfactual** and the **identification of a specific sufficiency failure**, not
the discovery that algorithm selection is instance-dependent.

**Binding:** RC-016 may not claim to have discovered instance-dependent algorithm
selection, feature-based selection, or DAC. A literature check against the
algorithm-selection benchmarks remains outstanding and may only **narrow** this
label, never widen it.

## D6 — Machine-checkable gates (specification; implementation is a separate step)

This commit is documentation-only and changes no `src`. The following gates are
**binding on the next implementation step**, and are specified tightly enough to be
mechanically checked. Until they are implemented, held-in **must not** be run.

**Gate 1 — Amendment 4 precedes science.** Before any held-in or held-out mode
executes, the instrument must verify all of:

1. `research/PREREG_RC016_AMENDMENT_4.md` exists and is **tracked** by git
   (`git ls-files --error-unmatch`);
2. it has **no uncommitted modifications** (`git diff --quiet HEAD --` on that path);
3. its **committing commit date is strictly later than the mtime of**
   `experiments/rc016/rc016_calibration.tsv` — i.e. the amendment was written
   *after* the calibration it interprets, not back-dated.

Failure of any condition ⇒ **refuse to run and exit non-zero**, printing which
condition failed. This must be a hard refusal, not a warning — RC-012 established
that a flag which silently does nothing is itself a defect.

**Gate 2 — the cost arm is unreachable.** RC-016 must expose **no** mode, flag, or
code path that runs an equal-cost or `k′` arm. If a cost-arm entry point exists, it
must be removed or made to refuse unconditionally with a pointer to §D2.

**Gate 3 — degenerate power is not silently trusted.** Any instance reported
`powered` while `d_seed_pilot < 1e-9` must be printed with an explicit
`DEGENERATE-POWER` marker, so the G48/G49/G50 artifact of §D1 cannot be read as
sensitivity by a later reader.

---

## What is now weaker, and what is stronger

**Weaker.** RC-016 measures no cost quantity; the equal-cost comparison it once
promised is withdrawn and left open. Power is far thinner than the design assumed —
1 genuinely powered instance of 30, not 4. And the pilot data were observed out of
order before the authorised freeze.

**Stronger.** The cycle no longer contains an arm that its own design cannot
identify, so it cannot publish a cost claim resting on a confounded ratio. The
`SIGN VARIES` bar no longer makes the pattern unreachable. The novelty label is
fixed before the data rather than chosen after. And the disagreement between the
power rule and the materiality rule is recorded as a finding instead of being
averaged away.

## Facts verified against source for this amendment

| claim | source | verified |
|---|---|---|
| `R = median_ms(gibbs)/median_ms(metropolis)`, wall time at call site | instrument `Calibration.r`, `calibrate_one` | ✅ |
| 0/30 in the `R` band; range 1.0535–1.6830 | `rc016_calibration.tsv`, all 30 rows | ✅ |
| 30/30 `cost_arm_needed`; 4/30 `powered` | same artifact | ✅ |
| G48, G49, G50 have `d_seed_pilot` exactly 0.000000 | same artifact, rows 20–22 | ✅ |
| `DEGENERATE_NULL` is `d_seed() < 1e-9` | instrument line 403 | ✅ |
| Exact sign-flip needs `K ≥ 6` to reject at α = 0.05 | `p_floor = 2/2^K`, Amendment 2 §B4 | ✅ |
| Pilot seeds disjoint from both science blocks | instrument `PILOT`/`HELD_IN`/`HELD_OUT` | ✅ |
| Controls ALL PASS, 648/648 operator positive | canonical calibration stdout | ✅ |
| artifact sha256 as tabled | `sha256sum` | ✅ |
