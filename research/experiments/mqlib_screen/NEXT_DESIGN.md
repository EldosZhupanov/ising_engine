# MQ-DESIGN-002 — difficulty qualification and cost curves

Status: **PROPOSED DESIGN, NOT AN EXECUTABLE PREREGISTRATION**. 2026-09-27.
Canonical successor design to [MQ-SCREEN-001](RESULT.md). No new solver runs, instance downloads, holdout coefficient/outcome inspection
or model training have been performed; the metadata inventory below is complete.

## What the completed screen establishes

All three configured wrappers returned identical final energy in every pair.
The corpus/budget did not discriminate final quality. Calling the problems easy
or the values globally optimal would require additional evidence. Early arrival
at the eventual common endpoint is a descriptive observation on exposed data,
not a preregistered target-time advantage. Do not turn the user's selected timing
ratios into a performance claim or use those endpoints as unseen targets.

## Authority and admission

The [parent protocol](../../EXTERNAL_COMPARISON_PROTOCOL.md), its
[handoff amendment](../../EXTERNAL_COMPARISON_AMENDMENT_1.md) and PROJECT_PLAN
S3/X3 gates remain binding. The MQ-SCREEN-001 exception applies only to that
completed screen. This design neither waives S3/X3 nor starts a new wave.
Before qualification execution, produce a separate prospective amendment
specifically authorizing corpus/instrument qualification or satisfy the existing
gates. Before any selector-advantage evaluation, reconcile all parent
requirements explicitly; design review alone cannot open execution.

## Questions separated

1. Measurement: can the harness resolve delivered-result costs in the proposed
   budget range without hiding startup, conversion, initialization or teardown?
2. Corpus qualification: do fixed, independently validated methods differ
   meaningfully on a predeclared sample at fixed costs?
3. Selection, later: does a frozen controller improve unseen results over the
   strongest fixed choice selected using development data only?

A yes to question 2 is not a yes to question 3. A single dominating method can
make a benchmark informative while leaving no motivation for algorithm selection.

## Corpus and leakage controls

Start with the parent's BiqMac direction; first inventory availability, format,
source provenance, instance families, licenses and target certificates without
running optimizers. Larger synthetic signed-weight QUBOs are a separate proposed
stratum, not a replacement silently relabelled BiqMac. Size alone is not a
hardness certificate. Do not reuse the eight MQ-SCREEN cases as unseen data.

Before outcomes: freeze the candidate universe, exact file hashes and a
family/group split into **qualification**, **development/training**, and
**untouched evaluation**. Near-duplicates, gauge/isomorphic versions and variants
from one generator family stay in the same group. Static metadata may support
splitting; holdout optimization outcomes may not. An independent reviewer checks
provenance and family separation before any qualification run.

Design target: at least 24 qualification instances and at least 24 distinct
final evaluation instances, distributed over predeclared size/density/family
strata. Actual counts and lists must be frozen after availability review; no
power guarantee is implied. Ten independent seeds for qualification; reserve
at least 30 per instance for a later confirmatory study, with its own power and
compute justification. No seed list is assigned here: first check the burn and
reservation records, then enumerate disjoint seeds in the binding manifest.

Qualification may reject a whole predeclared stratum under frozen criteria.
Retain all outcomes and exclusions. Do not retain only instances on which our
solver wins. A failed qualification stops; expanding the search for a better
corpus requires a new version disclosing all previously observed instances.
Any final claim covers the resulting target distribution, not arbitrary QUBO.

## Fixed competitors before learned selection

- Keep the already qualified native MQLib MERZ2002ONEOPT as continuity control.
- Add a **small, fixed candidate set of stronger/specialized MQLib heuristics**
  only after exact upstream IDs, versions and capability/output semantics are
  verified. Each adapter receives the same exact tiny-state and deadline gates.
  Do not call a candidate stronger until development measurements support it.
- Keep current Ultimate and v2 default configurations as continuity controls.
  Define a bounded, equal configuration-selection budget for candidate Ultimate,
  v2 and external configurations; include configuration failures and total cost.
- Pick one static configuration per **solver/backend family** on development
  data under a frozen normalized objective/tie rule, then choose a global
  best-fixed baseline on those development data. Freeze these choices before
  evaluation. Dataset-family routing is a separate selector, not this baseline;
  do not select configurations per evaluation instance after observing outcomes.
- A later selector comparison includes the parent's random-choice and greedy
  best-fixed baselines, plus production Ultimate and the specialist. A
  test-set best-of-all oracle is only an optimistic diagnostic.

Every registry plan records its actual operators/backend and seed. The previous
v2 default result supplies no evidence about learned KB or dynamic scheduling.

## Two timing experiments, never mixed

**Primary application contract:** delivered-result wall time starts before
conversion/serialization/launch, exactly as MQ-SCREEN-001. Preserve complete
receipt timestamps and strict cutoff, all-zero fallback, failures and late
output. For the prospective native calibration and its dependent benchmark,
**parent CPU0 + child CPU0**, one worker per arm, is the chosen contract. All
parent-side serialization, validation and delivery contention stays in the
application budget; no dedicated uncharged controller core is supplied. Verify
and record actual affinity of both processes (requested taskset is insufficient).
Freeze this same contract in both instruments; an affinity change requires a new
prospective calibration before timing claims. Rotate/randomize block order using
a frozen schedule and record load. Never subtract a measured startup median from
individual times or claim it was free.

**Optional separate kernel experiment:** a persistent initialized worker may
measure search after a ready barrier, with a fresh RNG/state per trial. Report
conversion, initialization and reset costs separately. Do not compare this
warm-worker time to a cold-start MQLib time. It needs its own instrument,
capability/serialization tests and prospective scope; no production API change
is implicitly authorized.

Before assigning very short budgets, use a no-search fixture/echo control,
identical-arm null and known-delay positive controls with the same launch,
serialization and receipt path. Freeze calibration design and acceptance
thresholds before calibration data. Candidate sub-100ms budgets are admitted
only if receipt/deadline error and injected-delay detection pass that gate;
otherwise report insufficient resolution rather than a speed ranking.

Proposed cold-start curve grid: 0.01,0.025,0.05,0.1,0.25,0.5,1,2 seconds,
plus a separate larger-instance block at 0.25,0.5,1,2,5,10 seconds. The grid and
applicable strata are frozen after calibration and before qualification search.
Collect one immutable anytime trace up to each block's maximum budget and derive
cutoff incumbents from it: these are **prefixes of one fixed configuration**, not
independently budget-tuned runs. Point out this distinction for schedule-dependent
algorithms. Do not choose the most favorable crossing time post hoc.

## Targets, censoring and inference

A target must be committed before evaluating that instance: published certified
optimum where available, or a cited best-known feasible witness rescored from the
original coefficients. Label the latter best-known, never optimal. An independent
reference run may define a target only with its cost, data access and role fixed
prospectively, isolated from the evaluated arms. If no justified target exists,
omit time-to-target for that instance; do not invent an optimum from pooled runs.

Primary qualification outcome proposed at **2 seconds**, common to both grids:
normalized final-energy differences using L=sum(abs(h))+sum(abs(J)), excluding
the constant. Proposed informativeness gate: at least one third of qualification
instances have a best-to-worst fixed-arm mean spread >=0.001 in this scale.
This diagnostic gate alone cannot establish superiority or selector value; it
can be triggered by a weak arm. A separate proposed complementarity screen asks
whether at least two non-dominated arms each win on at least one sixth of the
qualification instances by this margin. Report uncertainty and all ties; these
are development decisions, not confirmatory p-values. Freeze or revise these
proposals before data, never afterwards. No complementarity signal means prefer
a best-fixed baseline and stop the selector branch on that corpus.

For later confirmation choose **one** primary quality/cost endpoint and effect
threshold before evaluation. Other budget points are descriptive or receive a
predeclared multiple-comparison correction. Independent units are instances or
families as sampled, not all seeds or correlated checkpoints. Retain distributions
and paired/family uncertainty; quantify limits from a small number of families.

Target misses are right-censored, not observed completions at the timeout.
Report hit fraction and a predeclared restricted mean time-to-target over the
common horizon if justified; do not impute uncensored means or claim a finite
TTS99 from zero hits. Any restart-based TTS99 needs its own independence and
fixed-attempt-budget assumptions with probability uncertainty.

## Concrete next implementation gate

Prepare a read-only provenance/family/target inventory of the proposed corpus
and a bounded list of exact baseline configurations; do not open reserved
holdouts. Then freeze a no-search timing-calibration protocol and its acceptance
criteria. The next runnable qualification manifest must contain every instance
hash, split, seed, arm configuration, target provenance, budget/order, host and
compiler setting, total cost cap, kill rule and analysis command. Independent
review must confirm governance authorization, calibration and adapter gates
before execution. No further solver operators, record hunts or training are
justified by the current screen.

## Metadata inventory completed — 2026-09-27

[Local census](corpus_inventory.json), [source/baseline record](source_inventory.json)
and [reproducer](inventory_corpus.py) retain the scoped inventory. No instance
coefficients or reserved results were opened. The actual ignored cache is
`benchmark_suite/data/biqmac/`, not the parent's `benchmark_suite/biqmac/`.
303 data files occupy18,453,803bytes:80`be*.sparse`,45`gka*.sparse`,130other
Rudy names and48Ising names. Header inspection of125sparse files and several
named prior uses are recorded; **no file is currently established unseen**.
The minimum seed-exclusion list is retained, not clearance for new arbitrary seeds.

[Official BiqMac metadata](https://biqmac.aau.at/biqmaclib.html) identifies the
`be` family as Billionnet–Elloumi, distinct from Beasley's `bqp` family. The local
Beasley label is not authoritative. The BQP contract is minimization of symmetric
x'Qx; corresponding OR-Library maximization data require sign provenance.
Its target documentation includes optima and bounds, so a linked table is not a
blanket optimality certificate. Target tables/witnesses have not been ingested.
An explicit redistribution license was not verified; do not republish raw data
under the engine's code license. Metadata/source URL and hashes can be retained.

The pinned MQLib factory verifies additional candidates `PALUBECKIS2004bMST2`
(iterated tabu) and `KATAYAMA2000` (genetic/k-opt), with native default parameters
and callback constructors. Both still need adapter correctness/deadline gates;
neither is claimed stronger from code reading. Existing `MERZ2002ONEOPT`
remains the qualified continuity control. No new dependencies installed.

The [MQ-CAL-001 protocol](../mqlib_timing_calibration/protocol.md) now freezes a
no-search Python echo timing control with artificial inputs. It qualifies only
that receipt/deadline path, not native solver performance. Its
[result](../mqlib_timing_calibration/RESULT.md) is overall FAIL: only100ms passes
both echo shapes. The next scoped task is a native no-search readiness design;
corpus targets and untouched split clearance remain separate qualification items.

## Prospective affinity clarification — 2026-09-27

Source inspection confirms MQ-SCREEN-001 pins its child toCPU0 but does not
explicitly set or record the parent's affinity mask. MQ-CAL-001 explicitly pins
both. Thus MQ-CAL-001 is not a retrospective calibration of the old screen's
scheduling contract. Do not infer the old parent's actual placement from absence
of an explicit pin; inherited affinity was not established by those records.
The completed screen and calibration records remain immutable and unchanged.

For the next native experiment and its eventual comparison, choose the shared
CPU0 contract above. Record parent/child observed masks, worker thread settings,
input conversion, model readiness, emission and parent receipt; reject a contract
mismatch. Controller work is part of the single-core application cost. This
choice supplies no claim that it is the fastest or least noisy arrangement.

The echo's roughly tens-of-milliseconds receipt latency is a measurement of
process startup plus parsing/construction/output/delivery, not an estimate of
clock error. No phase-isolation experiment established Python startup alone as
the entire cause. Neither that latency nor a passing100ms gate is transferable
to native startup or an unmeasured speed ratio. Native qualification must use the
actual per-arm initialization/output path; do not replace it with a generic echo
and assume equivalent overhead. No new execution is authorized by this design.

## Native preparation calibration completed — 2026-09-27

[MQ-NATIVE-001](../mqlib_native_calibration/RESULT.md) ran once after protocol
`fb568c8` and instrument `a25c595`:960valid cells, overallFAIL, independent audit
PASS. All four Rust/C++ × artificial-shape groups pass50/100ms; all10/25ms groups
fail. This is diagnostic model preparation, including two-line delivery and timed
parent validation; it excludes initialization inside solve/run and any search.
Do not transfer admission to full optimizer startup, larger inputs or old screen.
Next qualify PALUBECKIS2004bMST2's exact conversion/witness/callback path, then
freeze the difficult-corpus comparison and actual-wrapper budget grid. Corpus
qualification and untouched holdout remain separate; no extra easy-screen rerun.

## MST2 bridge qualification completed — 2026-09-28

[MQ-MST2-001](../mqlib_mst2_qualification/RESULT.md) PASS:544exact identities,
144valid callback/process cells; independent raw audit PASS. Protocol `f6fa9e9`,
instrument `f738f4d`, raw `d47c40c`. This admits the tiny-input state/objective and
latched-stop contract only; relative baseline strength and larger-input behavior
remain unmeasured. Its parent captures output at termination, not per-message
receipt. The comparison instrument needs common delivered-result semantics.
Next freeze difficulty qualification separately from untouched holdout, targets,
baseline configurations and actual-wrapper budgets after S3/X3/provenance gates.
No new corpus is automatically unseen and no comparison wave ran here.

## Concrete synthetic qualification branch — 2026-09-28

[MQ-DIFFICULTY-001](../mqlib_difficulty_qualification/protocol.md) freezes a
separate24-input synthetic qualification, four fixed wrappers including the
now tiny-input-qualified MST2, and ten seeds. All one-generator-family inputs
are qualification-only; no future held-out population is claimed here. Its narrow
prospective parent-protocol exception does not satisfy S3/X3 or replace BiqMac
Wave2. No targets => no TTS endpoint. Main remains gated by committed/reviewed
instrument and successful preflight, with no optimizer execution at registration.

Eligibility there requires both parent receipt and independent validation to
complete before a checkpoint, prospectively tightening the receipt-only design
above. The manifest freezes a250ms-to2s prefix grid, current source configurations,
operational difficulty/complementarity rules and a3000s cumulative cap. The
larger-corpus full-search timing and discrimination outcomes remain unknown.

## Delivery qualification failed — 2026-09-28

[MQ-DIFFICULTY-001 closure](../mqlib_difficulty_qualification/closure/RESULT.md)
records512valid preflight cells and13/16passing controls, overallFAIL;
independent audit/review PASS. Main did not run. Its frozen250ms control gate
cannot be waived; nothing is established about the24inputs' optimization hardness.

Next proposed scope: one coarse2s delivered-and-validated final-quality endpoint,
same four fixed configurations and entire24-input qualification family. Preserve
the current exposure; no holdout claim. Separate quality-at-budget admission from
short-time speed-resolution claims. Do not merely relax a failed threshold to
rescue this version. Freeze a new protocol and review before observations; report
conversion/validation cost, fallback rates and scheduling uncertainty explicitly.
No automatic search, controller training or comparative wave follows this design.
