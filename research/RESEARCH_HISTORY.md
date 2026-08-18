# RESEARCH_HISTORY.md — Everything We Have Learned

> **Purpose.** The single reconstructed record of every major research conclusion
> in this project, separated into **CONFIRMED**, **REFUTED**, and **OPEN**, with
> the evidence and the reason each survived or failed. Governed by SOUL §2
> (*honesty over optimism*) and Constitution §13 (*negative results are results*).
> A finding not written here did not happen.
>
> **Provenance note.** Findings tagged `[platform]` were produced by the autonomous
> loop and are in the append-only stores. Findings tagged `[hand]` were produced by
> human-directed CLI experiments (`--op-benchmark`, `--structural`, `--pt-ab`,
> `--adaptive-oracle`) analysed in Python; **most are NOT yet in the ExperimentDb /
> KnowledgeGraph** — recovering them into the stores is Phase 0 work (see the
> autonomous-scientist roadmap).
>
> **KNOWN vs NEW.** Per Constitution §13 and the discovery mandate, every claim is
> tagged for its standing in the *external literature*, which this repo had never
> cited before this document. `KNOWN` = reproduces an established result; `NEW?` =
> plausibly novel but unverified against literature; `LOCAL` = an empirical fact
> about *this engine/substrate*, not a general claim.

---

## Reading guide

Each entry: **Question · Hypothesis · Experiment · Evidence · Why it survived/failed ·
Confidence · Literature standing · Future work.**

Confidence scale: High / Moderate / Low, with the reason for the ceiling.

---

# CONFIRMED

## C1 — Thermal (Metropolis-class) operators dominate every tested problem class `[hand]`
- **Question.** Is there a single operator family that is best across problem classes?
- **Hypothesis (L1).** A thermal operator is the (tied-)best single operator on every instance.
- **Experiment.** `--op-benchmark` (absolute per-instance [0,1] normalized mean best
  energy, 3 seeds) over 43 real BiqMac spin glasses + 5 synthetic families
  (coloring, MIS, partition, max2sat, TSP) — ~75 instances, 6 classes.
- **Evidence.** 0 failures / ~75. The four thermal ops (metropolis, gibbs,
  extremal_metropolis, history_field) hold the top-4 mean ranks (0.000–0.012); a
  cliff to the 5th (extremal_optimization, 0.369).
- **Why it survived.** Every falsification attempt (find a non-thermal op that is
  uniquely best) failed; the margin over non-thermal ops is large on hard instances.
- **Confidence.** High (as a fact about this substrate); the metric's artifacts are
  documented (R2/R4 below).
- **Literature standing.** **KNOWN.** That temperature-driven acceptance (simulated
  annealing / Metropolis) is a strong general-purpose baseline is decades-old
  (Kirkpatrick 1983). The *contribution here is negative-space*: it kills the search
  for a more exotic universal operator.
- **Future work.** Stop re-confirming it; treat "thermal base" as a prior, not a result.

## C2 — Operator value is STATE-dependent (explore→refine), masked by the temperature ladder `[hand]`
- **Question.** Does the best operator depend on the problem, or on *where the search is*?
- **Hypothesis (H2).** An oracle picking the best *next* operator from the current
  state beats the best static single operator, and the selection changes across phases.
- **Experiment.** `--adaptive-oracle`: deterministic-replay greedy oracle over
  operator schedules (score `prefix+[op]` by re-running; identical prefix ⇒ identical
  state) vs best single op at equal budget, on real BiqMac. Run with the default temp
  ladder (4.0→0.1) and at fixed temperature (T=0.3, T=0.5).
- **Evidence.** With the ladder: oracle picks one thermal op all phases (looks static),
  0/10 long-budget wins. **At fixed temperature a clear, robust transition appears:**
  T=0.3 → extremal_metropolis (cold start) → steepest_descent (refine), unanimous by
  late phases; T=0.5 → thermal (early) → **isoenergetic_cluster** (late). The ladder
  performs explore→refine *internally*, masking schedule-level switching.
- **Why it survived.** The transition reproduces at two independent temperatures with
  *different* refinement operators, ruling out a fixed-operator artifact. `isoenergetic_
  cluster` — an op that scores worse than random *solo/cold* (see R2) — is optimal as a
  warm late-phase move, the definitive demonstration of state-dependence.
- **Confidence.** Moderate-high qualitatively; Moderate on magnitude (greedy = myopic
  lower bound; oracle peeks at true energy; BiqMac + engine_v2 only; modest budgets).
- **Literature standing.** **KNOWN.** This is Adaptive Operator Selection / hyper-heuristics
  / memetic explore-then-exploit (Fialho credit assignment; Burke & Ochoa hyper-heuristics;
  cooling schedules). The *local* new fact is that *this engine's temp ladder already is a
  dynamic policy*, so operator-switching adds little over it (0.04–0.23%).
- **Future work.** Reframe from "which operator" to the joint (temperature × operator ×
  trigger-state) policy; A/B a learned online controller vs the built-in ladder.

## C3 — Cross-family transfer of learned schedule-quality is real `[platform]`
- **Question.** Does knowledge learned on one problem family predict on another?
- **Experiment.** Portability harness; Predictor (ridge) leave-one-instance-out; Policy
  trained on MaxCut applied to non-MaxCut.
- **Evidence.** Portability 9/12; Predictor LOO Spearman +0.747; MaxCut-policy 12/15 on
  non-MaxCut; World-model imagined-vs-real ranking Spearman 0.975.
- **Confidence.** Moderate (small models, ~18.5k experiments; the number to beat, not a harvest).
- **Literature standing.** **KNOWN** in form (per-instance algorithm selection transfers;
  SATzilla, ELA), **LOCAL** in specifics. Notably the transfer metric is *rank* (Spearman),
  consistent with H1 below.
- **Future work.** The SOUL thesis (foundation model) rests on this scaling; grow the dataset.

## C4 — Determinism / bit-identical replay holds across backends `[platform]`
- **Experiment.** Golden regression + cross-backend cross-checks (SparseBitSlice exact
  integer, >100k checks; DenseByte bit-identical, 4.8× vs oracle).
- **Evidence.** Passes byte-identical; every adaptive feature bit-identical on replay (ADR-0004).
- **Confidence.** High. **Literature standing.** N/A (engineering guarantee). This is the
  project's genuine structural asset — it is what makes the oracle experiments (C2) even possible.

## C5 — The platform can refute its own proposals (Popperian loop works) `[platform]`
- **Evidence.** The loop's own proposed operator `extremal_metropolis` was tested and
  found weak on G-Set (worst-solutions 8×) and recorded as such; evolved plans lose 5/5
  to UltimateSolver at equal budget; the Theory Engine's ablation shows metropolis_sweep
  is causal (ablation costs ~5%) while redundant operators are refuted.
- **Confidence.** High that the *mechanism* exists; Low that it runs *autonomously at
  scale* (most refutations were human-triggered or single-shot).
- **Literature standing.** The self-refutation discipline is the project's distinctive
  value; **NEW?** as an integrated always-on system (see gaps doc), KNOWN as isolated ideas.

---

# REFUTED (kept as knowledge)

## R1 — There is a single scalar structural predictor of operator competitiveness `[hand]`
- **Hypothesis.** density / weight_cv / lin_coup predicts which operator is competitive.
- **Experiment.** `--structural` features vs `--op-benchmark` scores; Spearman on real BiqMac.
- **Evidence.** density↔extremal_optimization = **−0.14** on real BiqMac (claimed −0.93 did
  not replicate); the real driver is **ruggedness (+0.92)**. The −0.93 was **cross-family**,
  i.e. density acting as a proxy for *which synthetic family* — the family confound the
  original claim believed it had survived. Within a single synthetic family (MIS) density
  works (−0.80) *only because* there density↔ruggedness are coupled (−0.55); on real data
  they decouple (−0.12) and density dies.
- **Why it failed.** Confounding: density was a proxy for landscape ruggedness.
- **Confidence.** High (confound isolated on real data).
- **Literature standing.** **KNOWN.** Landscape-feature confounding is a core lesson of
  Exploratory Landscape Analysis (Mersmann 2011); structural graph features being weak vs
  sampled landscape features is expected.
- **Future work.** Causal test: variance-controlled families varying ruggedness all-else-equal.

## R2 — "random_flip is universally the worst operator" `[hand]`
- **Evidence.** FALSE on 33/43 BiqMac + all max2sat: five *ensemble* operators
  (replica_exchange, houdayer, isoenergetic, population_resample, elite_broadcast) score
  **worse than random noise** when run solo/cold. True only on easy instances.
- **Why it failed.** Instance-selection artifact; ensemble ops are state-conditional (need a
  warm ensemble). This is the seed of C2.
- **Confidence.** High. **Literature standing.** **KNOWN** (population/cluster methods require
  a populated, thermalized ensemble). **LOCAL** specifics.

## R3 — Parallel tempering (thermal+exchange) is universally the best *algorithm* `[hand]`
- **Hypothesis (L4).** PT beats pure Metropolis at equal budget, especially on hard glasses.
- **Experiment.** `--pt-ab --fair` (pure gets PT's *total* sweep count): 25 real BiqMac.
- **Evidence.** PT wins 5, pure wins 4, 17 ties; sign-test **p=1.0**; mean rel_gain
  **negative** (−0.0049); pure wins on gka by 6–7%. The prior "+13% PT on gka2b" was the
  extra-sweeps confound the `--fair` flag exposes.
- **Why it failed.** Not statistically distinguishable from a wash once work is equalized; the
  engine already spreads pure-Metropolis replicas across the ladder (temperature diversity
  without exchange).
- **Confidence.** Moderate-high (short-budget regime; PT literature favors long runs / low T).
- **Literature standing.** **KNOWN** subtlety (equal-work accounting overturns naive PT wins).

## R4 — A multidimensional landscape descriptor predicts operators better than one scalar `[hand]`
- **Hypothesis (H1).** The full 11-dim descriptor (density, degree stats, ruggedness,
  minima-entropy, funnel, autocorrelation, frustration, spectral-gap) beats ruggedness-alone
  out-of-sample.
- **Experiment.** Ridge, **leave-one-family-out** CV, 169 instances / 7 families.
- **Evidence.** Pooled OOD Spearman *looked* like a win (+0.62 vs +0.12) but that was a
  **between-family confound** (the vector separates families by mean difficulty). **Within-
  held-out-family** OOD Spearman: ruggedness-alone (+0.314) **beats** the full descriptor
  (+0.185); a λ-sweep shows full only matches ruggedness at λ=300 where it has shrunk the
  extra dims to ≈0. In-sample R²=0.92, OOD R²=−4.7 (overfitting).
- **Why it failed.** These 11 features add overfitting, not transferable signal. **Important
  correction:** this refutes *these features*, not "vectors are useless" — orthogonality
  (ruggedness 0.016 vs frustration 0.96 on one instance) is not informativeness.
- **Confidence.** Moderate-high (LOFO, λ-swept, confound-decomposed); linear model only (no
  nonlinear model tested — a genuine open door).
- **Literature standing.** **KNOWN** methodology (ELA + per-instance algorithm selection); the
  refutation is a healthy replication of "hand-features overfit."

## R5 — (Meta) The recent research program was on the project's true path `[reconstruction]`
- **Claim being refuted.** That hunting for a universal law / a descriptor→operator predictor
  *is* the mission.
- **Evidence.** SOUL §"ultimate goal" and Constitution §2–3 define the mission as an
  autonomous *science* platform → foundation model, not a landscape-analysis result. The
  recent work (R1–R4, C1–C2) is human-directed empirical landscape analysis — valuable, honest,
  but it is a **local optimum** (see RESEARCH_GAPS.md PART 3) and largely reproduces known
  fields (ELA, algorithm selection, AOS).
- **Confidence.** High (documented against the governing charters).
- **Future work.** Redirect to autonomous discovery; see the gaps and architecture docs.

---

# OPEN

## O1 — Does a *nonlinear* model over landscape features transfer within-family OOD?
- The one door R4 left open: ridge is linear; a tree/GBM could extract interactions. Gated on
  a real ML environment. Low priority unless the target is first made non-degenerate.
- **Literature standing.** KNOWN framing (ELA + ML); worth at most a single decisive test.

## O2 — Causal (not correlational) role of ruggedness
- All ruggedness results are correlational (ruggedness ≈ hardness). **Open experiment:**
  variance-controlled synthetic families where ruggedness is varied *all-else-equal*, to move
  from correlation to causation. This is the highest-value *classical* experiment remaining
  and the natural bridge to autonomous instance generation. **NEW?** as a controlled design.

## O3 — Can the platform DISCOVER a genuinely new operator (not select among 14)?
- Today `evolution` searches *sequences* of 14 hand-coded operators; `proposal.rs` writes prose,
  **no code**. The discovery loop is **open**. Whether evolutionary program search over the
  SpinState API can synthesize a novel, verified, non-redundant operator is untested. **This is
  the real frontier** (see RESEARCH_GAPS.md PART 6). **NEW?** for physics-inspired optimization.

## O4 — Does the foundation-model thesis scale?
- Cross-family transfer (C3) is a seed at ~18.5k experiments. Whether it holds to 500k/1M/5M is
  the SOUL bet. Open, and gated on the dataset-growth campaign, not on architecture.

## O5 — Joint (temperature × operator) online policy vs the built-in ladder
- C2 shows the ladder already does explore→refine. Open: can a *learned* online controller beat
  the hand-tuned ladder at equal budget on held-out instances? The honest arbiter of the whole
  dynamic-policy thesis. **KNOWN** field (AutoRL / AOS), **LOCAL** arbiter.

---

## One-paragraph synthesis

The engine robustly does thermal-based local search whose *best move is state-dependent* —
both **known** results. The single-scalar and multi-scalar predictor hunts **failed** for
sound, confound-driven reasons and **mostly reproduce established fields** (ELA, algorithm
selection, AOS) the repo had never cited. The project's genuine, un-depreciated assets are the
**deterministic substrate**, the **append-only knowledge discipline**, and the **self-refutation
machinery** — none of which is a benchmark score. The mission (SOUL) is autonomous, cumulative
optimization *science*; the recent work, however rigorous, was a local optimum away from it. The
open questions that matter (O2, O3, O5) are about *discovery and causation*, not prediction.
