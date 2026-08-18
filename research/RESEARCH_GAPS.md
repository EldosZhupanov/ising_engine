# RESEARCH_GAPS.md — Where We Went Off Course, and the True Gap

> Companion to `RESEARCH_HISTORY.md`. This document does the uncomfortable work:
> naming where the research became local optimization instead of autonomous
> discovery (PART 3), auditing what is already **known science** so we do not
> reproduce it (PART 5), and identifying the **smallest genuinely-new problem**
> this platform could actually solve (PART 6). Governed by SOUL: *truth has
> priority over confirmation.*

---

# PART 3 — Where we drifted into benchmark/local optimization

## The core drift, stated plainly

The mission (SOUL, Constitution §1–3) is **an autonomous system that does science on
optimization** and grows toward a **Research Foundation Model**. The *last several
sessions of research* — the "universal law," the structural/multidimensional
descriptors, the operator predictor, the adaptive oracle — were **human-directed,
one-off empirical studies** whose object was *predict/optimize/classify*, not
*build the autonomous scientist*. Each was honest and well-executed; collectively
they were a detour.

Four concrete symptoms, each verifiable in the repo:

1. **The findings never entered the knowledge system.** The recent CLI modes
   (`--op-benchmark`, `--structural`, `--pt-ab`, `--adaptive-oracle`) `println!`
   their results; they do **not** call `observe_if` / `flush_append`. The
   conclusions live in commit messages and in the assistant's memory files — *not*
   in the append-only `ExperimentDb` / `KnowledgeGraph` the whole platform exists
   to grow. By the project's own rule (Constitution §13: *an unrecorded run didn't
   happen*), most of R1–R4 and C1–C2 are, formally, **off the record**.

2. **The human became the loop.** Hypotheses, experiment designs, statistics,
   red-teaming, and re-planning were done *by hand in Python*. The platform already
   has agents for exactly these roles (`lab`, `planner`, `theory`, `stats`,
   `scientist`, `executive`). We bypassed the autonomous scientist to *be* the
   scientist — the opposite of the mission.

3. **The work reproduced known fields without citing them.** `rg` for FunSearch,
   ELA, hyper-heuristics, algorithm-selection, SATzilla, Vizier, Nevergrad returns
   **zero hits** anywhere in the repo. We independently re-derived Exploratory
   Landscape Analysis, the Algorithm Selection Problem, and Adaptive Operator
   Selection — decades-old fields — and treated each re-derivation as discovery.

4. **The objective quietly became a scalar to predict.** "Find the universal law,"
   "predict the operator," "fit the descriptor," "classify the instance." Every one
   of these collapses open-ended discovery into a supervised-learning target. That
   is a comfortable, well-defined, *closed* problem — and it is not the mission.

## The local optima that trapped us (each is useful, none is the destination)

| Local optimum | Why it attracts | Why it is not the destination |
|---|---|---|
| **One universal law** | A single clean scalar feels like a Law of Nature | Optimization has *no* free lunch; a universal operator law is a priori unlikely, and each version was refuted (R1–R4) |
| **Predict the operator** | Well-posed supervised target, easy to score | This is the Algorithm Selection Problem (Rice 1976) — solved-in-principle, and it selects among *given* algorithms; it discovers nothing |
| **Fit a descriptor** | ELA features are cheap and plentiful | Hand-features overfit (R4); ELA already exists; more features ≠ more science |
| **Classify instances** | Clusters look like understanding | Taxonomy is not mechanism; the platform is meant to explain *why*, and to *act*, not to label |
| **Beat a benchmark** | A number that goes up is legible and motivating | The Benchmark Track is *evidence*, a means; SOUL is explicit that "another solver depreciates immediately." UltimateSolver already wins 5/5 — we don't need another point in the heuristic zoo |

**The unifying error:** all five convert an *open-ended, self-directed discovery
problem* into a *closed, human-specified optimization problem*. That is precisely
the transformation the mission forbids: we are supposed to be building the thing
that *poses* the problems, not hand-solving one problem it might pose.

## What was *not* drift (keep this)

The honesty was real and is the project's soul working correctly: every claim was
falsified hard, confounds were hunted and found (R1's density=ruggedness, R4's
between-family leak), negative results were kept. The *method* was right; the
*agent* was wrong (human, not platform) and the *object* was wrong (a scalar, not
a scientist).

---

# PART 5 — Known-science audit (do NOT rediscover)

The repo cites **no external literature**. Below, each recent research thread is
mapped to the field that already owns it. **KNOWN** = do not spend compute
reproducing it; use it as a prior and cite it.

| Our thread | Established field / landmark | Verdict |
|---|---|---|
| Structural + landscape features (ruggedness, autocorrelation, entropy, FDC) | **Exploratory Landscape Analysis** (Mersmann 2011); fitness-landscape analysis (Weinberger 1990; Stadler); Fitness-Distance Correlation (Jones & Forrest 1995) | **KNOWN** |
| descriptor → best algorithm | **Algorithm Selection Problem** (Rice 1976); **SATzilla** (Xu 2008); per-instance algorithm selection / **ASlib** | **KNOWN** |
| State → best next operator; explore→refine schedules | **Adaptive Operator Selection** (Fialho 2010, credit assignment); **Hyper-heuristics** (Burke, Ochoa); memetic algorithms | **KNOWN** |
| Automatic parameter/schedule tuning | **irace** (López-Ibáñez); **SMAC** (Hutter); **Google Vizier**; **Nevergrad** | **KNOWN** |
| Evolving operator *sequences* | Hyper-heuristic genetic programming; **AutoML** pipelines | **KNOWN** |
| Thermal dominance / PT subtleties | Simulated annealing (Kirkpatrick 1983); parallel tempering (Swendsen–Wang, Marinari) | **KNOWN** |
| Learned world-model ranking of schedules | Model-based RL; **AutoRL** (Parker-Holder 2022) | **KNOWN (form)** |
| **Discovering new *code/operators* by search** | **AutoML-Zero** (Real 2020); **FunSearch** (Romera-Paredes 2023); **AlphaTensor / AlphaDev** (Fawzi 2022, Mankowitz 2023); **EUREKA** (Ma 2023) | **KNOWN (as method), NOT DONE HERE (for Ising operators)** |

**Implication.** Roughly everything in R1–R4 and C1–C3 is a *replication* of known
fields. That is not worthless — replication on a clean deterministic substrate has
value — but it must be **labelled KNOWN and not mistaken for contribution**. The
one row where the *method exists but has not been applied in our domain* is the
last: search-based discovery of new operators.

## PART 5b — Rediscovery ledger (Our work → Known literature → Difference)

Brutally honest. The "Difference" column is what, if anything, we added beyond the
prior art — usually *only* the deterministic substrate, not the idea.

| Our work | Known literature it reproduces | The actual difference (what we added) |
|---|---|---|
| `--structural` landscape features (ruggedness, autocorrelation, minima-entropy, funnel, frustration, spectral gap) | **Exploratory Landscape Analysis** (Mersmann 2011; Kerschke); fitness-landscape analysis (Weinberger 1990; Stadler); FDC (Jones & Forrest 1995) | None conceptually. Our feature set is a subset of ELA. Only novelty: exact, replayable computation on a deterministic substrate. |
| descriptor → best operator (H1/R4) | **Algorithm Selection Problem** (Rice 1976); **SATzilla** (Xu 2008); **ASlib**; per-instance selection | None. We re-ran per-instance algorithm selection and got the standard result (hand-features overfit OOD). |
| State → best next operator; explore→refine (C2/H2) | **Adaptive Operator Selection** (Fialho 2010); **Hyper-heuristics** (Burke, Ochoa); memetic explore-exploit; SA cooling schedules | None. The "state-dependent operator" is the founding premise of AOS. Our local twist: showing the *temp ladder already is such a policy* on this engine. |
| `--pt-ab --fair` equal-work PT vs Metropolis (R3) | Parallel tempering theory (Marinari; Katzgraber tuning); equal-work benchmarking discipline | None conceptually; a careful equal-work replication that overturns a naive PT claim we ourselves made. |
| Evolving operator *sequences* (`evolution.rs`) | Hyper-heuristic genetic programming; sequence-based AutoML | None. Sequencing a fixed operator pool = hyper-heuristics. |
| Learned World-model ranking of schedules (`world.rs`) | Model-based RL; **AutoRL** (Parker-Holder 2022) | Form is standard; our Spearman-0.975 imagined-vs-real is a clean but expected result. |
| Predictor/Policy cross-family transfer (C3) | Meta-learning for algorithm selection; transfer in ASlib | The *result* (transfer works) is expected; the honest measurement on one substrate is the only addition. |
| Self-refutation via Runtime ablation (C5, `theory.rs`) | Popperian methodology; ablation studies; causal-contribution testing | The *integration* — an always-on ablation gate inside an autonomous loop — is where the only defensible originality lives (see PART 6). |
| Operator *code* discovery | **AutoML-Zero** (Real 2020); **FunSearch** (2023); **AlphaTensor/AlphaDev** (2022–23); **EUREKA** (2023) | **Method known; NOT attempted here.** This is the gap, not a rediscovery. |

## PART 5c — 4-level novelty verdict on every CONFIRMED result

Scale: **KNOWN** (textbook / landmark result) · **LIKELY KNOWN** (almost certainly in
the literature, we haven't found the exact citation) · **POSSIBLY NOVEL** (a specific
combination we can't place, low confidence) · **TRULY NOVEL** (defensible contribution).
No confirmed *scientific* result reaches TRULY NOVEL. That is the honest headline.

| Finding | Claim | Verdict | Why |
|---|---|---|---|
| **C1** | Thermal operators dominate all classes | **KNOWN** | Simulated annealing as a strong general baseline (Kirkpatrick 1983). |
| **C2** | Best operator is state-dependent; explore→refine; ladder masks it | **KNOWN** (mechanism) / **POSSIBLY NOVEL** (the *specific* observation that this engine's replica temp-ladder is an implicit dynamic policy that erases operator-switching value ≤0.23%) | AOS/hyper-heuristics own the mechanism; the ladder-equivalence framing is a narrow, engine-specific observation, not a general law. |
| **C3** | Cross-family transfer of schedule quality | **KNOWN** (form) / **LIKELY KNOWN** (specifics) | Transfer in per-instance algorithm selection is established; our numbers are a local measurement. |
| **C4** | Deterministic bit-identical cross-backend replay | **LIKELY KNOWN** as engineering; **POSSIBLY NOVEL** as a *research substrate* (exact reproducible fitness enabling exact ablation/oracle experiments) | Reproducible builds/RNG are standard; using determinism as the *epistemic* foundation of an autonomous optimizer-scientist is uncommon and is our real edge. |
| **C5** | The loop can refute its own proposals via Runtime ablation | **KNOWN** (Popper, ablation) / **POSSIBLY NOVEL** (as an *always-on integrated* self-refutation gate in an autonomous optimization-research loop) | The ideas are old; the packaged, continuous, append-only self-refutation system is where a defensible claim could be built — but only if it runs autonomously at scale (it does not yet). |

**Verdict in one line:** every *scientific* finding is KNOWN or LIKELY KNOWN. The only
POSSIBLY-NOVEL material is *architectural* — determinism-as-epistemics (C4) and
always-on self-refutation (C5) — i.e., the *scientist*, not the *optimization results*.
That is precisely why the mission is the scientist, not the solver.

---

# PART 6 — The true research gap (the smallest genuinely-new thing)

**Question:** what is the smallest unsolved problem this platform — with its
deterministic runtime, operator trait, ablation/Theory engine, and append-only
knowledge — could realistically solve that is *not* a benchmark, a solver, or a
predictor?

### The gap: **autonomous synthesis of novel, verified optimization operators**

Not selecting among 14 hand-written operators (Algorithm Selection — KNOWN). Not
sequencing them (hyper-heuristics — KNOWN). But **searching the space of operator
*programs*** over the `SpinState`/`Operator` API to discover a *new* local-search
dynamic that (a) compiles and runs on the verified substrate, (b) is *not
behaviorally redundant* with any existing operator (the engine can already test
this — R2/C5), and (c) survives an ablation on the Runtime (Theory Engine).

This is **FunSearch / AutoML-Zero applied to physics-inspired optimization
operators** — a method that exists, in a domain where (to our knowledge) it has not
been done, on a substrate uniquely suited to it because **determinism makes the
fitness signal exact and every discovery reproducible.**

Why this is the right gap:
- It is **discovery, not selection** — it obeys the mission.
- The platform is **90% of the way there**: it has the runtime, the operator
  contract, capability passports, an evolution engine (currently over sequences),
  a redundancy/novelty test, and an ablation engine. The missing piece is a
  *program search space* for operator bodies and a *safe compile/execute harness*
  — `proposal.rs` today deliberately stops at prose.
- It produces **falsifiable, publishable artifacts**: "here is an operator the
  system invented, here is the ablation proving it contributes, here is the
  structural regime where it wins, and here is why it is not any known move."
- The negative result is equally publishable: "unbounded operator search over this
  API yields only re-discoveries of Metropolis/cluster moves — the operator space
  is effectively closed," which would be a real scientific statement.

### Candidate directions, ranked

| # | Direction | Novelty | Difficulty | Sci. value | Eng. effort |
|---|---|---|---|---|---|
| **1** | **Operator synthesis** (FunSearch-style program search over the SpinState API, verified + ablated) | **High** | High | **High** | High |
| 2 | **Causal landscape science** (O2): controlled instance generation varying ONE landscape axis all-else-equal; run the Theory Engine to establish *causal* operator↔structure laws (not correlational) | Medium-High | Medium | Medium-High | Medium |
| 3 | **Autonomous mechanism catalogue**: let the loop run truly unattended for N weeks and produce an append-only, self-refuted catalogue of *conditional* operator mechanisms — the "cumulative science" artifact itself | Medium | Medium | Medium-High | Medium (mostly wiring existing agents) |
| 4 | **Learned joint (temp×operator) online controller** (O5) A/B vs the built-in ladder | Low-Medium | Medium | Medium | Medium |
| 5 | Nonlinear ELA transfer test (O1) | Low | Low | Low | Low |

**Recommendation.** Direction **2 first** (it is the causal foundation everything
else needs and is mostly design + the existing Theory Engine), then **1** (the true
novel contribution) built on the controlled instances and redundancy tests #2
matures. #3 is the standing background campaign that makes the platform *be* the
scientist regardless of the headline result.

---

## The one sentence that should govern the next year

> We are not looking for a law that predicts the best operator; we are building the
> scientist that **invents** operators and **proves** — by ablation on a
> deterministic substrate — which of its own inventions are real.
