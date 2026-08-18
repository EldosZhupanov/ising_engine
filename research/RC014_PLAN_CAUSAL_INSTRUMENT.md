# RC-014 — Action Plan: the Exact-Counterfactual Instrument

> **Status: PLAN + Phase 0 EXECUTED (2026-08-19).** Results:
> `RC014_PHASE0_AUDIT.md` — **PASS, restricted**. Pre-registration:
> `PREREG_RC014.md`. No experiment has been run; this file remains the plan and
> the gate structure, and becomes an `RC014_*.md` record only after Gate A.
>
> Two Phase-0 answers **corrected this document in place** (§2 row 0.3, §3.2);
> both corrections are marked inline and sourced to the audit.
>
> **Governance.** Subordinate to `ISING_ENGINE_CONSTITUTION.md` (direction),
> `SOUL.md`, the ADRs, and `CLAUDE.md`. It changes no direction and proposes no
> amendment. It implements Constitution §13 (pre-registration with a binding kill
> criterion) and ADR-0004 (bit-identical replay) literally.
>
> **What this plan replaces.** `ROADMAP.md` "Next 10" item 1 — the growth campaign
> toward 500k — is **deprioritised, not cancelled**, on the evidence of RC-004
> (G-Set is unidentifiable on two axes), RC-011 (the predictor's metric is provably
> blind to instance features), and RC-013 (the whole recorded corpus is
> cache-resident, one regime). Growing a corpus that provably cannot answer the
> open questions adds rows, not knowledge. Coverage replaces count.

---

## 0. The one experiment

> At a fixed Runtime version, with addressable randomness, at measured-equal
> budget, and with sensors frozen in advance: determine the **causal horizon value
> of replacing a baseline operator**; validate the instrument against an analytic
> and a historical positive control; then establish whether the current sensors
> are sufficient to choose an operator, or whether there exist states that are
> indistinguishable to the controller yet require opposite actions.

Three outcomes, all publishable: the instrument is validated and a causal law
follows; a specific hypothesis is refuted; or the direction dies inside two weeks
of instrument work.

**Why this and not operator synthesis.** The reachability probe can return an
uninformative zero if the search language is wrong. The sufficiency question
cannot: either a sufficient statistic exists (strong), or a hierarchy of minimal
counterexamples is constructed (also strong). Impossibility/identifiability
results are also this project's demonstrated genre — RC-003, RC-004 and the Gauge
Degeneracy theorem are the strongest items in the corpus.

---

## 1. Standing constraints (non-negotiable, inherited)

1. **The production core is read-only.** No change to `src/solver/ultimate.rs`,
   the legacy solvers, `server_api.rs`, or the golden regression path. Instrument
   work happens above the `engine_v2` Runtime, never inside it, unless Phase 0
   proves otherwise — in which case it becomes an ADR, not a patch.
2. **ADR-0004.** Any trajectory-changing behaviour is opt-in, flagged, and A/B'd
   at identical seeds. The default path stays bit-identical.
3. **Constitution §13.** Instances, seeds, budgets, baselines, success thresholds
   and the kill criterion are fixed **before** implementation, in a committed
   file (deliverable D-0.9).
4. **Host discipline (`PERF.md`).** ~9% same-session timing drift. Every timing
   measurement interleaves conditions within each repetition and takes medians.
   Absolute ms are never compared across sessions.
5. **No new subsystem until a gate passes.** This plan authorises exactly one
   instrument. Foundry, DSL search and dataset growth are gated behind it.
6. **No new governance documents.** The doc layer is at diminishing returns. This
   file is the last planning artifact before code.

---

## 2. Phase 0 — Feasibility audit (no code changes)

**Bounded by answers, not by time.** "The data does not exist" is a complete
answer that triggers the recorded fallback. If any item turns into archaeology,
record the fallback and move on — Phase 0 must not become a research project.

| # | Question | Method | Fallback / kill |
|---|---|---|---|
| **0.1** | Does `engine_v2`'s Runtime preserve the correspondence of random events when a step is removed or replaced? | Read the RNG path in `runtime.rs` / `executor.rs` / the operator implementations. Determine whether draws are consumed as a **stream** (position-dependent) or from an **indexed** buffer. Note: `optimization_plan_rank135.md` states the production `rng_buf` is indexed, not a stream — engine_v2 is unverified. | **KILL/REDIRECT:** if exact addressing requires changing the Runtime contract, stop and open an ADR. Do not patch the read-only core to fit this experiment. Phase 3 (DSL gate) becomes the useful work while that decision is pending. |
| **0.2** | Does `ExperimentDb` record the **code version** of each row? | Inspect the 22-field schema against the required provenance list (§2.1). | If absent: the old corpus is a source for *statistical analysis of stored results*, **not** for guaranteed rerun candidates. Record this and proceed — no rerun-based design. |
| **0.3** | Which corpus is canonical? | ~~`platform_gset` 18,570 · `platform_grow` 48,653 · `platform_grow_div` 42,535 = **109,758**~~ **ANSWERED, and the premise was wrong**: `platform_gset` is a byte-identical *prefix* of both other campaigns, so 109,758 triple-counts it. **Distinct rows = 72,618**, and 23,965 record ids collide between the two tails with zero identical rows. See Phase 0 §0.3. | Canonical corpus = `platform_gset` (18,570). Union requires a `(campaign, id)` composite key. Three documents quoting 109,758/110k need correction. |
| **0.4** | Are stored plan/instance/seed data sufficient to re-execute a chosen row? | Attempt one manual re-execution of a recent row end-to-end. | If it fails: rerun candidates are drawn only from rows written by the current code version. |
| **0.5** | What exactly does `--investigate` compute today? | Read `theory.rs::investigate_operator` and commit `8ec5c43` ("make `--investigate` a marginal-contribution test, not vs-nothing"). | **Do not build a second marginal-contribution harness.** If it already computes `I_replace`, extend it; the instrument becomes a generalisation, not a parallel system. |
| **0.6** | Can transition observation be added without violating the read-only core? | Identify the seam: `BatchExecutor` / operator `report` path. | If not, ADR. |
| **0.7** | Can the historical `metropolis_sweep` ≈5% ablation protocol be reconstructed? | Recover instance, schedule, initialisation, baseline, budget, seed list from the Theory Engine's stored artifacts. | **Fallback, recorded now:** if unreconstructable, run a fresh ablation with current code, establish a bootstrap CI, and require the instrument to reproduce **that**. Weaker (self-consistency, not historical) but valid. Do not skip the control. |
| **0.8** | Freeze the sensor maps. | Extract from deployed code: `S_0` = sensors the Decision Engine actually reads (`InstanceStats` / `InstanceSignature`); `S_1` = Runtime/controller sensors (acceptance, overlap, diversity, round-trip); `S_2` = a minimal permitted extension, defined **before** any result. | The maps are frozen artifacts in the pre-registration. Choosing a deliberately poor map and "proving" insufficiency is the failure mode this prevents. |
| **0.9** | Write the pre-registration. | `research/PREREG_RC014.md`, committed **before the first line of instrument code**. Contents in §10. | Across thirteen cycles the project has never produced pre-registration as an *artifact* — "pre-registered" appears in prose written afterwards. A committed file makes the discipline checkable by git history instead of by assertion. This closes CHIEF_SCIENTIST F4 (self-confirmation) structurally. |

### 2.1 Required provenance fields (audit target for 0.2)

git commit · dirty-tree status (+ patch hash) · crate version · registry version and
full operator ID set · serialized execution plan · backend · rustc version · target
features · RNG algorithm/version · instance content hash · lowering/pass versions ·
seed · thread layout.

Without commit or artifact hash, "same seed ⇒ same trajectory" holds only *for a
known code version* and the record does not identify that version.

---

## 3. Phase 1 — Minimal counterfactual instrument

Deliberately small: 2–3 operators, one schedule, existing instances
(G-Set + `biqmac`, the only locally available weighted family — RC-004 typing rule
7 makes any guide/frustration question uninformative elsewhere).

### 3.1 Addressable randomness (the load-bearing requirement)

Common-random-numbers design, not merely a deterministic RNG:

```
U = f(seed, run, phase, logical_slot, operator, site, replica, draw_kind, draw_index)
```

- **Logical slots are defined by the plan, not by execution dynamics.** A removed
  step leaves an **empty slot**; subsequent operators receive the same exogenous
  draws as in the original run.
- `draw_kind` + local `draw_index` are required: an operator may take several
  draws per site, branches may consume different counts, and
  population/resampling and replica exchange have a different event dimensionality
  than a site sweep.
- Repeated instances of the same operator need a stable identity.

**Validation:** a deliberate off-by-one shift in the addressing must be *detected*
by a test (§3.4), not silently absorbed.

### 3.2 The three estimands — and which one is primary

| Estimand | Definition | Question it answers |
|---|---|---|
| `I_delete` | `Y(P \ k) − Y(P)` | value of the step *together with its budget* |
| `I_cost-null` | `Y(P[k ← noop_c]) − Y(P)` | effect of the computation at preserved wall-cost |
| **`I_replace`** | **`Y(P[k ← B_c]) − Y(P)`** | **advantage over the baseline operator at equal cost** |

**`I_replace` is primary** — it is the question a scheduler actually faces.
*(Corrected by Phase 0 §0.5: this plan originally stated that `--investigate`
already adopted this semantics. It does not — `theory.rs:315–329` **deletes** the
operator, retaining only the rest of the schedule. The replacement arm must be
built.)* `I_delete` conflates causal
contribution with one step less of budget: exactly the extra-sweeps confound that
`--pt-ab --fair` was built to kill. `I_cost-null` is a diagnostic control only —
it preserves time spent, not useful algorithmic work.

**Budget matching is calibrated before the main run and frozen:** interleaved
measurements, median wall time, calibrated per instance/regime, stored as a frozen
table. **The declared `cost_model` must not be used** — RC-005 showed it is the
true law with φ ≡ 1/3 hard-coded and overcharges cold regimes up to 2.1×. Nor may
sweeps be adjusted online against noisy wall-clock: that injects a
non-deterministic policy into the experiment itself.

### 3.3 The data unit

```
(S_t, O_t, B_t, U_t) → (S_{t+1}, C_t, Y_{t:H})
```

`S` = frozen sensor vector · `O` = operator · `B` = budget · `U` = addressed
randomness · `C` = **measured** cost · `Y_{t:H}` = best result over horizons `H`,
not only instantaneous ΔE.

**Why the horizon term is not optional.** Instantaneous ΔE reward systematically
under-values exploration: an operator may worsen current energy while opening a
region local moves cannot reach. Recording only ΔE would re-learn the same myopic
policy and discard reachability.

**Why this data unit matters beyond this cycle.** RC-011 proved the predictor's
LOO Spearman is *provably invariant* to instance features because the model is
additive and every row in a fold shares one instance signature. At transition
level the state varies *within* a run, so the constant offset disappears. This is
the remedy RC-011 named, obtained structurally rather than by adding interaction
terms — it retires one of the five open decisions instead of leaving it pending.

### 3.4 Controls — four kinds, all required

| Control | Requirement |
|---|---|
| **Null** | The null intervention is **bit-identical** to the unmodified run. |
| **Inert** | Removing a provably inert step yields exactly zero. (RC-007: from all-zeros, `replica_exchange` and `houdayer_cluster` are inert — a ready-made inert case.) |
| **Synthetic positive** | A small system where replacing an operator gives an **analytically pre-computable** effect. Validates the attribution arithmetic independently of any historical measurement. |
| **Real positive** | Reproduces the known `metropolis_sweep` ablation within its reconstructed CI (or the fresh-ablation CI, per fallback 0.7). |

**The positive controls are the point.** A set of only-zero checks is passed
perfectly by an instrument that reports ≈0 for everything — which is precisely
RC-012's failure (a model predicting ≡0 at every step passed its module test) and
what RC-009's vacuity guard ("0 of 18 inert") exists to prevent.

The historical ≈5% is **not a universal constant** — it depends on instance,
schedule, initialisation, baseline and budget. Fix the expected interval in the
pre-registration; the control passes if the new instrument reproduces the prior
*paired* effect inside it. Otherwise a different intervention semantics will be
misread as a broken instrument.

### 3.5 Initialisation as a factor, not a fix

All 18,570 recorded runs start all-zeros with every replica identical
(`executor.rs:124`). RC-002 measured the cost at 0.09–0.46% and showed thermal
dynamics erases initial *quality* but never initial *diversity*. Since diversity
**is** reachability, running a reachability experiment on that initialisation
without accounting for it would confound the primary object.

| Arm | Purpose |
|---|---|
| legacy all-zeros | compatibility with the historical corpus |
| diverse random | reachability without initial ensemble collapse |

Measure `I_{operator × initial diversity}`. Pre-registered rules: the *compatibility*
endpoint is legacy init; the *reachability* endpoint is diverse init; transferring
a conclusion between them is forbidden without the interaction test; **replacing
the production/default initialisation is out of scope.**

Note RC-008: at T = 0.1 the ensemble collapses completely (`q_AA = 1.0000`, all
replicas one configuration); chains mix at T ≥ 0.5. Temperatures for this cycle
must be chosen on both sides of that crossover, or reported as single-regime.

### 3.6 Attribution order

1. singleton ablations;
2. one **pre-selected** pair interaction;
3. targeted triples (Phase 2) — chosen from pairs whose `d_order` was largest in
   RC-007 (the ρ ≈ 1.32 end), not exhaustively;
4. Monte-Carlo Shapley/Owen only for suspect groups;
5. **efficiency check**: the attributions must sum to the total effect within
   error.

Plain leave-one-out is insufficient: with substitutable operators
`v({A,B}) ≫ v({A}), v({B})` and both LOO contributions can be small. RC-007
(28/28 pairs at ρ < 1.4) makes this the expected regime here, and it is why the
coalition function `v(C)` is the unit of analysis. Full Shapley is exponential and
is not promised for long schedules.

---

## 4. The gates

### Gate A — end of Phase 1

All of:

1. **Synthetic positive control** recovered to analytic precision.
2. **Real positive control** reproduced inside its pre-registered interval.
3. **Null control** exactly zero; **inert control** exactly zero; the deliberate
   RNG-shift test **detects** the shift.
4. At least one studied effect exceeds the pre-registered threshold relative to
   `d_seed` — **under multiplicity control**: either the specific
   (operator, instance-class) contrast is named in advance, or Benjamini–Hochberg
   is applied across the pre-declared set and the effect survives. An
   "at-least-one-of-many" criterion without correction is a garden of forking
   paths and will fire by chance.
5. Sign and magnitude replicate on held-out seeds.
6. The next falsifiable descendant was **written down before** the held-out
   results were viewed.

**Failing Gate A ⇒ stop.** Do not build the Foundry "to strengthen the signal".
Decide first whether the problem is the instrument, the sensors, or the absence of
usable causal structure.

### Gate B — end of Phase 2

1. The descendant is confirmed on new instances or a new structural regime;
2. the alternative explanation was tested and failed;
3. the effect survives budget-matched baseline replacement.

**Gate A pass + Gate B fail ⇒ the instrument is useful and the specific scientific
conclusion is refuted.** That is not grounds to remove the instrument. Record the
refutation and pick the next descendant.

### 4.1 Statistical requirements (both gates)

`ρ_I = |I| / d_seed` is the materiality scale, **not** the uncertainty of the
intervention effect — a counterfactual pair sharing addressed randomness has far
lower variance than a seed-to-seed comparison. Required jointly:

- absolute (or relative) effect size;
- bootstrap CI;
- paired randomisation test;
- normalisation against seed noise;
- **an explicit degenerate-null flag** — RC-007's first pass produced 8 `inf` rows
  because deterministic operators have zero seed variance and `ρ` divided by zero;
- replication on independent instances and seeds.

Rank scale-free relative differences, never raw energies pooled across
heterogeneous instances. Claim equivalence only from a bootstrap CI inside a
pre-registered bound, never from `p > 0.05`.

---

## 5. Phase 2 — descendant and first sufficiency probe

Only after Gate A. Falsifiable descendant on new instances; targeted pair/triple
interactions (tests RC-007's stated "pairs only" limitation); Gate B; first check
of whether the frozen `S_0` sensors suffice to choose an operator.

---

## 6. Phase 3 — DSL reconstruction gate

Sequential, not parallel — a solo project has one attention budget, and two
"parallel" tracks are serial plus context-switching. **Exception:** if Phase 0.1
blocks Phase 1 behind an ADR-level Runtime decision, this becomes the useful work
while that decision is pending.

**Purpose: validate the search language before searching.** A negative result
from operator search is worthless if the language could not express the known
operators.

Minimum basis to reconstruct — distinct mechanisms, not all 18 at once:
Metropolis · greedy descent · random flip · replica exchange · cluster move ·
resampling/history-based.

**Equivalence is machine-checkable, scoped by operator class.** Build the explicit
transition matrix `K(x,y)` of the DSL program on a small system (single-flip
kernels are sparse — `n + 1` non-zeros per row — so `n ≈ 20` is tractable; cluster
and replica operators have wider support and need smaller `n`):

| Class | Criterion |
|---|---|
| Reversible thermal (Metropolis, Gibbs) | normalisation `Σ_y K(x,y) = 1`; detailed balance `π_β(x)K(x,y) = π_β(y)K(y,x)` with `π_β ∝ e^{−βE}`; kernel equality to the reference within tolerance |
| Non-reversible (DEO exchange, extremal optimisation) | kernel equality only — **detailed balance must not be required**, these violate it by design |
| Deterministic (greedy, steepest descent) | functional equality on all enumerated states |

Irreducibility and aperiodicity are checked where expected. An empirical
post-burn-in histogram remains an integration test, never the definition — a wrong
kernel can carry the right stationary distribution.

**Kill:** with adequate search budget and the target enabled, failure to
reconstruct a behavioural equivalent of Metropolis invalidates any negative result
from novel-operator search. The DSL is revised **once**; after a second failure
the direction is frozen.

---

## 7. Phase 4 — Instance Foundry (gated)

Only after: the coupling map, computational feasibility, planted optima, separate
verification of barrier geometry, and `biqmac` as the external bridge.

### 7.1 The axes are not independent — and this project has the proof

"Vary one axis, all else equal" is **mathematically impossible** for part of the
proposed axis list:

- clustering ↔ degree: coupled by definition. This is exactly what killed the
  Easy-Information Law — triangle edges have mean local degree 69.3 vs 26.3,
  "forced by definition" (`CHANNEL_EXHAUSTION` §7.2).
- spectral gap ↔ modularity: linked by Cheeger's inequality — nearly one axis in
  two coordinate systems.
- treewidth ↔ density: a dense graph is not of small width.
- barrier geometry ↔ frustration: not independent for Ising landscapes.

A naive Foundry reproduces the degree confound *synthetically*, where the empty
control group that exposed it on G-Set will no longer be empty to warn you.

### 7.2 Required before any generator code

| Coupling | Evidence | Permitted design |
|---|---|---|
| hard | theorem / identity | independent variation forbidden |
| bounded | only a range is reachable | matched design inside the range |
| soft | statistical correlation | balancing / matching |
| practically independent | confirmed by generation | admissible causal contrast |

The map must not be pairwise only — constraints can appear only for triples. After
the algebraic pass, a computational feasibility solver:

```
find G₀, G₁ :  X_k(G₀) ≈ X_k(G₁)  (controlled, explicit tolerances)
               X_j(G₀) ≠ X_j(G₁)  (contrasted)
```

**First Foundry carries 2–3 axes with an automatically verified contrast, not
eleven.** Start with: weight heterogeneity at matched topology; planted basin
multiplicity; controllable barrier construction; possibly modular coupling between
planted subproblems.

**Planted optimum ≠ known barrier geometry.** A planted instance gives ground
truth for the answer, not necessarily for the structure of paths to it. These are
verified separately.

`biqmac` (`be100.1.sparse`, 145 distinct weights, −100…100) is the external bridge
proving a conclusion is not a generator artifact.

---

## 8. Phase 5 — sufficient-statistic counterexamples

The long-term scientific thesis.

For each frozen level `S_0 ⊂ S_1 ⊂ S_2`:

```
S_i(A) ≈ S_i(B)   and   argmax_O V(O, A) ≠ argmax_O V(O, B) ?
```

1. counterexample found at `S_0` but disappearing after one added sensor ⇒ the
   result is **the minimal missing observable**;
2. counterexample surviving even at the rich `S_2` ⇒ observable compression is
   **fundamentally insufficient** in this class — the stronger result.

**Dependency, stated explicitly:** `V(O, S)` is the horizon value from §3.3. Phase
5 cannot begin before the horizon value is validated. The sensor maps come from
deployed code (§2, 0.8), frozen before any result — otherwise the counterexample
is a strawman of one's own construction.

---

## 9. Dependency graph

```
Phase 0  RNG / version / corpus audit  ──(0.1 kill ⇒ ADR)──▶ Phase 3 becomes the useful work
    │
    ▼
Phase 1  addressable randomness → transition records → I_replace → controls
    │
   Gate A ──fail──▶ STOP. Diagnose instrument vs sensors vs absent structure.
    │
    ▼
Phase 2  descendant · targeted triples · Gate B · first sufficiency probe
    │
    ├──────────────▶ Phase 3  DSL reconstruction gate (sequential)
    │
    ├──────────────▶ Phase 4  Foundry (after the coupling map)
    │
    ▼
Phase 5  sufficient-statistic counterexamples   [requires validated horizon value]
```

---

## 10. Pre-registration contents (deliverable D-0.9)

`research/PREREG_RC014.md`, committed **before** instrument code:

1. Hypothesis and the primary estimand (`I_replace`).
2. Instances, seed lists, budgets, temperatures (both sides of the RC-008
   crossover), operators, baseline operator.
3. The frozen budget calibration table.
4. Frozen sensor maps `S_0` / `S_1` / `S_2`, extracted from deployed code.
5. Positive-control intervals (synthetic analytic value; historical or fresh
   ablation CI).
6. The named contrast **or** the pre-declared set plus the FDR procedure.
7. The `d_seed` threshold and the degenerate-null rule.
8. Gate A and Gate B criteria verbatim.
9. The factorial initialisation design and the transfer prohibition between arms.
10. Canonical corpus declaration (§0.3) and whether reruns are admissible.

---

## 11. Explicitly out of scope

- Growing the dataset to 500k. Coverage first; count later, if ever.
- Any change to the production solver, `server_api.rs`, or default initialisation.
- New website panels, new LLM agents, new abstract subsystems.
- New hand-written operators without a specific hypothesis.
- New governance documents.
- Full Shapley over long schedules.
- Higher-order replica statistics. Deferred deliberately: three cycles (RC-001
  consensus, RC-003 covariance, AXIOMS §8.3–8.6) each found real-but-immaterial
  statistical signal, the 2×-signal / +0.06%-gain puzzle is **reopened and
  unexplained** since the Easy-Information Law was retracted, and the
  `I_emergent = I(ensemble) − I(J)` subtraction was already performed and partly
  corrected (local degree absorbs about half of the "76% emergent"; sampling noise
  most of the remainder). Adding a fourth, more expensive statistic before
  explaining why the first three did not cash out increases observability without
  controllability.

---

## 12. Risk register

| Risk | Containment |
|---|---|
| Instrument reports ≈0 for everything and passes all zero-checks | Two positive controls are Gate-A blocking (§3.4) |
| Gate A fires by chance across many contrasts | Named contrast or BH-FDR across the pre-declared set (§4) |
| `I_delete` semantics smuggles in a budget confound | `I_replace` is primary; frozen calibration; declared `cost_model` forbidden (RC-005) |
| Randomness desynchronises ⇒ "counterfactual" is just another seed | Logical slots defined by the plan; empty slot on removal; shift-detection test |
| Reachability measured on an initialisation that suppresses diversity | Factorial legacy/diverse arms + interaction (RC-002, RC-008) |
| Foundry rebuilds the degree confound synthetically | Coupling map + feasibility solver before generator code |
| Sensor map chosen to make insufficiency easy | Maps frozen from deployed code before any result |
| Phase 0 becomes archaeology | Bounded by answers; every item has a recorded fallback |
| Planning replaces doing | This is the last planning artifact. Gates are binding. |

---

## 13. Immediate next actions

1. ~~**Phase 0.1:** read the `engine_v2` Runtime RNG path.~~ **DONE
   (2026-08-19).** Answer: **NO** — single `ChaCha8Rng` stream
   (`runtime.rs:196`, `:296`). Narrow exception found:
   `metropolis_sweep ↔ gibbs_color_sweep` consume identical draws, so that one
   substitution is exact today with no core change. General addressability → ADR.
2. ~~**Write `research/PREREG_RC014.md`.**~~ **DONE** — written before any
   instrument code exists, per §10.
3. **Commit** the uncommitted paths (RC001–RC013, `memory/`, the `exp_*`
   binaries, the two new tests, `website/`, and the three RC-014 files).
   Pre-registration is only evidence if its position in git history precedes the
   data.
4. **Phase 1** may start: the restricted instrument per `PREREG_RC014.md`.

---

*This plan can kill itself in two weeks. That is its purpose. A plan that cannot
return a fatal answer is a construction schedule, not an experiment.*
