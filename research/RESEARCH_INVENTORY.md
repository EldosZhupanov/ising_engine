# Research Inventory — complete register of research cycles

**Purpose:** nothing in this programme may exist only in conversation. This file
is the index, the validation matrix, the revocation register, the reproduction
guide and the dependency graph. Each cycle also has its own record in
`research/RC0*.md`.

Convention: **CONFIRMED** / **REFUTED** / **PARTIAL** refers to the *pre-registered
hypothesis*, not to whether the cycle was useful. Several of the most valuable
cycles refuted their own hypothesis.

---

## 1. Inventory

### RC-001 — Ensemble-Consensus Thermostat
- **Motivation** — `delta_e_into` returns ΔE for all replicas at once and `spin()`
  is an O(1) bit read, so the ensemble is a free per-site estimator. All 14
  operators then in the library treated replicas as independent chains.
- **Hypothesis (pre-registered)** — per-site temperature
  `T_eff(i,r) = T_r·exp(λ·c_i)` with consensus `c_i = |⟨s_i⟩|` beats Metropolis;
  success required **p < 0.05 AND median relative improvement > 0.1%**.
- **Method** — 3 arms (λ = −1, 0, +1) at identical budget/replicas/ladder/seeds,
  paired on (instance, seed); then a λ dose–response ladder. Two-sided Wilcoxon
  on **scale-free relative** differences.
- **Data** — 30 G-Set × 8 seeds = 240 paired obs; dose arms 150 each.
- **Derivation** — λ = 0 ⇒ `exp(0) = 1` exactly ⇒ bit-identical to
  `metropolis_sweep`: the null lives *inside* the algorithm and is unit-tested.
- **Result** — λ<0 (backbone freezing) **refuted with a large effect**: 0 of 132
  non-tied pairs at λ = −2, p ≈ 0. λ>0 significant (p = 1.55e-5) but median
  **+0.043%**, less than half the pre-registered bar.
- **Status — REFUTED** on its own criterion. Declined to switch to the mean,
  which would have passed.
- **Implication** — closes backbone-freezing for thermal MaxCut; the mechanism is
  that ensemble consensus is largely a shared-initialization artifact.
- **Limitations** — PT ladder dilutes `c_i` (hot replicas randomise it); measured
  9× attenuation, predicted in advance.
- **Follow-up** — RC-002 (why consensus is an artifact).

### RC-002 — Initialization erasure → the quality/diversity dissociation
- **Motivation** — `executor.rs:124` is `vec![0u8; ir.n]`: **all 18,570 recorded
  runs start all-zeros with every replica identical**. Seeds vary the RNG stream,
  never the starting point.
- **Hypothesis (pre-registered)** — an erasure threshold T\* exists and
  **T\* < 4.0**, so the standard configuration is in the erased regime.
- **Method** — 3-arm factorial separating the two things an initialization can
  supply: zeros (poor/zero-diversity), random (poor/max-diversity), greedy
  (good/zero-diversity), at identical total sweeps, swept over 6 temperatures.
  Equivalence claimed only via **bootstrap 95% CI inside ±0.1%**, never p > 0.05.
- **Data** — 30 G-Set × 8 seeds = 240 pairs per cell.
- **Result** — **Diversity is never erased**: +0.455% at T = 0.1 (183W/27L) and
  still **+0.088% at temp_hi = 4.0** (p = 8.5e-5). **Quality is erased** at every
  T ≥ 0.25 (CI inside ±0.1%) and is *harmful* at depth (greedy×10: −0.138%,
  p = 7.4e-4). Confirmatory 2×2: random+greedy ≈ random alone.
- **Status — PARTIAL.** H2a supported; H2b/H2c/H2d refuted. The framing (a single
  threshold) was wrong; the dissociation is the real result.
- **Derivation/mechanism** — verified from source, not inferred:
  `greedy_descent` takes `_rng` (`greedy_descent.rs:83`) — it ignores the RNG, so
  from identical starts all replicas follow the same trajectory into the same
  optimum. Diversity collapse is provable.
- **Implication** — the warm-start class supplies *quality*, which is exactly the
  component this substrate discards. Explains why `Warmstart` and
  `ExactInference` capabilities have zero implementations and were never missed.
- **Limitations** — one operator family; the all-zeros cost (0.09–0.46%) is
  **reported, not fixed**: changing it breaks bit-identical replay of the whole
  corpus (ADR-0004).
- **Follow-up** — RC-008 reconciled this with RC-003's ergodic premise.

### RC-003 — Architectural assumption audit
- **Motivation** — RC-001/002 were operator research with statistics around it.
- **Method** — audit 10 assumptions against Neal, OpenJij, CP-SAT, SCIP and this
  engine; attempt to kill each candidate architecture *mathematically* first.
- **Result** — three architectures killed before any code: entropy-as-control
  (`dS/dβ = −β·Var(E) < 0` ⇒ bijection ⇒ a renamed cooling schedule);
  interleaved roof-duality persistency (idempotent, or unsound — and its only
  non-trivial form was already refuted by RC-001); belief propagation (de
  Almeida–Thouless instability in the RSB phase). The survivor (runtime move
  synthesis) became RC-007.
- **Status — CONFIRMED** (the audit found A10 unbroken by every system).
- **Implication** — **A10 — "the move set is human-designed and fixed at compile
  time" — is broken by nobody.**
- **Follow-up** — RC-004, RC-007.

### RC-004 — The architecture space, and two impossibility proofs
- **Result** — a 7-axis space with typing rules; **all 18 operators occupy one
  cell**. Two proofs: **E = 2V − |E|** on unweighted instances (energy ≡ violation
  count, so energy-guided and constraint-guided search are one method written two
  ways), and the temperature/entropy bijection.
- **Status — CONFIRMED (proofs).**
- **Implication — benchmark degeneracy.** G-Set cannot test the guide axis *in
  principle*; later work found it also cannot separate frustration from
  clustering. This is identifiability, not power: more G-Set runs cannot fix it.
- **Follow-up** — any guide/frustration experiment must use `biqmac`.

### RC-005 — Flip density: a complexity parameter the cost model omits
- **Motivation** — every `cost_model` is a pure function of instance *shape*.
- **Hypothesis (pre-registered)** — true cost carries **flip density φ = flips/r**
  (= acceptance rate) with a regime change at **φ = 1/3**; declared work constant
  while actual ms varies **> 2×**. Refuted if ms is flat (< 1.2×; host drift ~9%).
- **Derivation** — `sparse_bitslice.rs:265` dispatches on flip count: scattered
  `O(deg·flips)` below the threshold, dense `O(deg·r)` above.
- **Method** — temperatures **interleaved within each repetition**, medians taken,
  so monotonic host drift cannot manufacture a monotonic trend.
- **Data** — G1, G11, G22, G32; `metropolis_sweep`, r = 32, 20 sweeps.
- **Result — CONFIRMED.** Declared work constant; actual ms spread **1.61–2.09×**.
  The φ = 1/3 critical point is *visible*: on both instances that cross it, cost
  turns over (G11 11.448→11.025 ms; G32 20.543→19.974). Corrected law
  `W = W_scan + W_flip·min(φ,⅓)`, **R² 0.93–0.97**; flip term **57–79%** of cost
  at saturation.
- **Evidence that it is a mechanism, not a fit** — the `min` cap is *inert* on the
  two instances that never reach φ = ⅓ (capped R² identical to uncapped to 4 d.p.)
  and lifts R² by 5–8 points on the two that cross it.
- **Implication** — the declared model is the true law with **φ ≡ ⅓ hard-coded**,
  so it **overcharges by up to 2.1%, worst in the cold regime**. The Decision
  Engine ranks by utility *per unit cost*, so low-temperature refinement is
  systematically penalised.
- **Limitations** — one operator, one backend; wall-ms as cost proxy.
- **Status: OPEN, UNFIXED** — correcting `cost_model` changes plan ranking and
  therefore trajectories (ADR-0004).

### RC-006 — The gradient-ledger invariant
See `RC006_GRADIENT_LEDGER.md`. **CONFIRMED**, exact identity, residual
`0.000000e0` over 200 checks on 5 instance families including 145-distinct-weight
biqmac. Promoted to `tests/test_gradient_ledger_invariant.rs`.

### RC-007 — The operator monoid is effectively commutative
- **Hypothesis (pre-registered)** — ρ = d_order/d_seed ≈ 1 for most pairs but
  **large** for asymmetric pairs, since a schedule should be dominated by
  whichever operator runs last.
- **Method** — `d_seed` (same schedule, different seeds) is the null, making ρ
  scale-free. 8 operators, all 28 pairs, 6 instances × 6 seeds.
- **Result — REFUTED.** 28/28 pairs have ρ < 1.4; max **1.32**. Swapping two
  operators changes the outcome no more than re-seeding does.
- **Instrument confound, caught and corrected** — the first pass reported 57%
  commuting with **8 `inf` rows**: from all-zeros every replica is identical, so
  `replica_exchange` and `houdayer_cluster` are inert and deterministic operators
  have zero seed variance (ρ divides by zero). With a randomising sweep prepended:
  100% commuting, zero `inf`.
- **Exact commutation that survives correction** —
  `greedy_descent × replica_exchange` and `steepest_descent × replica_exchange`
  have `d_order` **exactly 0** against a non-zero null: deterministic descent acts
  independently per replica, so permuting and descending provably commute.
- **Implication** — the Evolution Engine's *ordering* search explores distinctions
  that largely do not exist at pair level. A mechanism for the recorded result
  that evolved plans lose **5/5** to `UltimateSolver`.
- **Limitations** — **pairs only**; longer schedules untested. Equal budgets, one
  ladder, final energy only.
- **Status: OPEN, UNFIXED** (narrowing the search space is trajectory-changing).

### RC-008 — Ensemble collapse at low T; audit of RC-003's ergodic premise
- **Motivation** — RC-003 used the ergodic theorem; RC-002 found surviving memory
  of the initial condition. Those are in tension and the premise was never tested.
- **Method** — two-chain coupling: independent inits, independent RNG streams,
  **flat** ladders (a PT ladder would confound chain mixing with replica exchange).
  Absolute overlap throughout, because Z₂ gauge symmetry makes signed overlap
  average to zero for reasons unrelated to mixing.
- **Result** — **at T = 0.1, `q_AA` = 1.0000 exactly**: all 16 replicas collapse to
  a *single configuration*, the chains stay 0.22–0.24 apart, energies never agree,
  unchanged 200→800 sweeps. Broken ergodicity. **At T ≥ 0.5 chains mix by 200
  sweeps.** Crossover between 0.1 and 0.5.
- **Criterion error, caught and fixed** — the first pass said "not mixed"
  everywhere because the energy test was an absolute `1e-3`, *below the sampling
  noise of a 16-replica mean*. Replaced with a 2σ bound on the difference of two
  means.
- **Status — PARTIAL** (predicted broken ergodicity at low T: yes; predicted a
  clean high-T crossover: yes; but the audit's value was reconciliation).
- **Implication** — RC-003's premise was **sound where applied** (its sweeps were
  mostly T ≥ 0.5): a scope caveat, not a refutation. **RC-002 is reconciled, not
  contradicted**: its ladders always spanned down to 0.1, so cold non-mixing
  replicas preserved the memory. Both correct; different objects.
- **Limitations** — R = 16 too small for a tight overlap tolerance, so the
  MIXED/not-mixed verdict flickers at T ≥ 0.5; that is instrument noise and is
  **not** reported as signal. One instance, one operator.

### RC-009 — Capability passport vs reality
See `RC009_BACKEND_PASSPORT.md`. **CONFIRMED (no discrepancy).** 18 operators × 3
backends, 0 failures, **0 of 18 inert**. Closed the gap that
`test_dense_byte_verification.rs` exercised zero operators.

### RC-010 — World model "Spearman 0.975" audit
- **Hypothesis (pre-registered)** — ρ **collapses** once the trivially-learnable
  noise-vs-greedy gap is removed, showing the number carried ~one bit.
- **Method** — three candidate sets against one trained model: A reproduce (4/5
  in training), B held out (0/9, easy gap intact), C homogeneous (0/9, every
  candidate ends in `greedy_descent`, easy gap removed).
- **Result — REFUTED.** A 0.9747 → B 0.8984 → C **0.8485**. Narrowing the ranking
  task **7.5×** (spread 286% → 38%) costs only 0.05 of ρ. The model genuinely ranks.
- **My forensic error, corrected** — I first argued 0.975 required n = 9 (untied
  Spearman grid) and so could not have come from the n = 5 test.
  `evaluation.rs:110` averages ranks over ties, making intermediate values
  reachable; arm A returns 0.9747. The claim *did* come from that test.
- **Finding that stands** — **overstated, not wrong**: honest held-out value
  **≈0.85–0.90**, not 0.975; the gap is training overlap. `world.rs`'s test gates
  only at `rho >= 0.5`, so 0.975 was observed, never enforced.
- **Status — PARTIAL** (the claim is inflated; the model is sound).
- **Limitations** — n = 9, **one** instance, 3 seeds; the A>B>C ordering is
  plausibly within noise.

### RC-011 — The predictor's LOO Spearman cannot measure transfer
- **Result — a PROOF.** `predictor::feature_row` has **no instance×schedule
  interaction terms**, so `ŷ = w·x_inst + w·x_sched`. `evaluate_predictor` scores
  each fold over rows of *one* held-out instance, which all share that signature,
  so `w·x_inst` is a **constant offset** — and Spearman, being rank-based, is
  **provably invariant** to it.
- **Empirical confirmation (18,570-row DB)** — `predict(sigA,s) − predict(sigB,s)`
  max deviation **1.110e-16** (machine ε) over 7 very different schedules; **0**
  signature violations across 23 instances; metric reproduced at **0.7467**.
- **Status — CONFIRMED (the metric is invariant); the *claim* it supported is
  REVOKED.**
- **Sound part** — the fold structure is correct: `leave_one_instance_out`
  excludes the whole instance, so there is no row-level leakage despite ~800
  rows/instance.
- **NOT claimed** — the predictor is fine; ranking schedules is what `filter`
  uses it for. **The portability 9/12 claim is separate and untouched.**
- **Follow-up** — to make it transfer-sensitive: add interaction terms, or score
  *across* folds (predicted-vs-actual instance-mean improvement, where instance
  features actually vary). Both behaviour-changing; recorded, not applied.

### RC-012 — Dynamics early-stop audit
- **Motivation** — named by RC-011's final audit as the highest-value target: the
  only learned model that *acts* (predict < ε ⇒ Stop), so its errors are silent.
- **Hypothesis (pre-registered)** — false stops concentrate on thermal/shifted
  conditions; refuted if every firing's actual remaining < ε.
- **Method** — offline replay of the exact deployed rule (ε=0.005, min_frac=0.5)
  over controller-free captured trajectories; actual remaining computable because
  the full trajectory is known. 4 conditions × 5 seeds.
- **Result — CONFIRMED, then superseded twice.** (1) 7/20 firings false, all
  thermal, worst discards 1.86% ≈ 3.7×ε. (2) The model predicts ≡0 at every step
  (16.2% actually remaining at D's step 0), so the ε-gate is vacuous — the
  controller is behaviourally "stop at min_frac". (3) **The deployed bootstrap
  cannot fit at all**: 2-step schedule × 3 seeds = 6 rows < fit's 20-row floor ⇒
  `--early-stop` always "skips honestly". An unreachable feature.
- **Status — CONFIRMED (all three layers demonstrated).**
- **Implication** — ROADMAP prior-Next-10 item 2 annotated; fix must couple
  layers 2+3 (fixing only the bootstrap ships a silent 50% truncation).
- **Limitations** — layers 1–2 measured on the module-test training corpus,
  because the deployed path yields no model; re-measure if training is enriched.
- **Follow-up** — target reweighting / plateau-stratified training; then re-run
  this audit binary unchanged.

### RC-013 — Kernel floors (hardware characterisation)
- **Motivation** — ADR-0003 asserted, never measured; two open decisions (AVX2
  SAXPY, `simd_utils.rs`) hinge on which floor binds.
- **Hypotheses (pre-registered)** — P1: scan intercept compute-bound (≥50%
  explained by the acceptance floor) and flat (<+50%) across L2→DRAM. P2:
  random-graph flip cost rises ≥2× across the ladder while ring stays flat.
- **Method** — three measured floors (STREAM/gather/acceptance) + kernel grid:
  {ring, random} × ledger 0.5→134 MB × two φ points; ring-vs-random at equal
  n/degree/dynamics isolates locality at identical FLOPs; interleaved reps,
  medians (host drift rule).
- **Result — P1 REFUTED** (floor explains ~40%; random intercept +75%);
  **P2 CONFIRMED** (2.1× flip-cost rise; ring flat; 1.9× total at 134 MB).
- **Implications** — ADR-0003 quantified (~2× at DRAM, ~0 in cache);
  benchmark-scale caveat #3 (G-Set is entirely cache-resident, 0.5 MB ledger);
  SIMD evidence leans delete `simd_utils.rs` (scope-limited to engine_v2).
- **Limitations** — φ_lo drifts with n ⇒ scan/flip split indicative only at
  small φ_lo; WSL2 wall-clock; one operator/backend.
- **Status — PARTIAL** (one prediction each way; both informative).

### RC-014 — Substitution beats deletion (exact-counterfactual instrument)
- **Motivation** — the Theory Engine certifies `metropolis_sweep` AND
  `gibbs_color_sweep` as causal "exploration" on the same condition, with
  near-identical deletion degradations (+8.1% / +7.5%) and the same ablated
  baseline (−8496.00). Deletion measures distance from a greedy-only schedule,
  so it cannot separate two operators of the same capability even in principle.
- **Hypothesis (pre-registered)** — an equal-work substitution changes the run's
  horizon value by more than re-seeding (`ρ ≥ 0.5`, rel ≥ 0.1%), and its sign is
  regime-dependent.
- **Method** — `I_replace_work = Y(gibbs) − Y(metropolis)` at equal sweeps and
  equal draws. Exact under the Runtime's single stream because the pair is
  draw-identical (`sweeps·n·r` `f64` each, measured at the operator boundary) and
  the only downstream operator draws nothing. 6 instances × 2 initialisation arms
  × 8 held-in + 8 held-out seeds; exact sign-flip test over 2⁸; BH at FDR 0.10 on
  the 11 secondaries; primary contrast named in advance.
- **Result — PARTIAL.** First clause CONFIRMED: Metropolis beats Gibbs on all
  five G-Set instances, both arms, both seed sets, 0.17–2.41% at ρ = 1.6–9.6;
  **10/10 contrasts replicate** under the numeric A6 rule. Second clause NOT
  ESTABLISHED: `be100.1` reverses sign in all four cells but is
  `DEGENERATE_NULL` in one arm, ρ < 0.5 in the other, fails replication, and is
  confounded weighted×dense.
- **Novelty, stated honestly** — the *direction* is KNOWN (Peskun ordering:
  Metropolis dominates heat-bath; Metropolis accepts every `ΔE ≤ 0` move with
  probability 1, heat-bath with `1/(1+exp(−|h|/T)) < 1`). **The contribution is
  methodological**: deletion-based ablation reports two operators as equivalently
  causal where substitution separates them cleanly.
- **Instrument validation** — an independent reference implementation, sharing no
  code with the operators, reproduces both **exactly on 648/648 fixtures**; the
  draw-alignment probe returns identical generator state and detects an injected
  one-word shift; null and both inert controls are exact. The first fixture grid
  was **vacuous** (32 replicas × 16 sweeps saturates every small instance to its
  optimum, so every reference difference was 0) — recorded, and the grid widened
  without touching the pass criterion.
- **Limitations** — one pair, one schedule, 16 sweeps, cache-resident scale,
  pairs not schedules. The real-positive CI (+2.52% [+2.43, +2.64]) does **not**
  contain the historical 2.1%; exact historical reproduction was never available
  because `ExperimentDb` records no code version (Phase 0 §0.2).
- **Descendant D-14 — REFUTED.** Density-vs-weighting ran as pre-registered and
  the two arms point opposite ways: the unweighted-dense arm is zero within noise
  and `DEGENERATE_NULL` in every cell, the weighted-sparse arm is positive but
  not significant (p = 1.000 / 0.375). **Mechanism of the refutation:** dense
  n = 100 instances **saturate** — the Metropolis arm returns the same energy for
  all eight seeds, so `d_seed` collapses and `ρ` is undefined. The `be100.1` sign
  reversal was therefore measured against a zero-variance arm and is
  **WITHDRAWN as unmeasurable**, not kept as an open observation. Gate B is not
  reached; nothing was widened to rescue it.
- **Non-gating temperature curve.** Tie mass (`ΔE = 0` proposals) is **13–34%** on
  the two toroidal ±1 lattices against **0–5%** elsewhere and 0.004% on the
  145-weight `be100.1` — the Amendment 2 A2.2 exploratory prediction confirmed at
  n = 6. The `T → 0` shrinkage holds where tie mass is low. **Anomaly:** G43
  *reverses* at T = 0.1 (I = −44.13, ρ = 5.31, p = 0.0078) with only 2.3% ties, so
  neither the tie mechanism nor the downhill-acceptance gap covers it. Recorded as
  an anomaly, not a finding; it is the natural next pre-registration.
- **Follow-up** — adding a substitution arm to `theory.rs` (trajectory-changing ⇒
  open decision, NOT applied); the matched multi-instance set (Amendment 2 A2.4)
  re-pointed at G43's cold reversal rather than at density-vs-weighting.
  Record: `RC014_COUNTERFACTUAL_SUBSTITUTION.md`.

### RC-015 — The cold difference between Metropolis and heat-bath is one line about ties
- **Origin** — RC-014 §10's unexplained cold reversal on G43, and the fact that
  its dismissal of tie handling ("only 2.3% ties") was wrong on three counts: a
  fraction was compared where a **count** was required (2.3% of 16 000 proposals
  is 368 ties/replica ⇒ ~184 divergent decisions); off-tie behaviour at T = 0.1
  was never checked (the acceptance ratio is **1.0000** at ΔE = +1 and +2, so ties
  are the ONLY remaining channel there); and the tie statistic was
  **post-treatment**, counted along one arm's trajectory.
- **Method** — three draw-identical arms: **M** (tie ⇒ accept), **M½** (tie ⇒ flip
  w.p. ½, Metropolis elsewhere), **G** (heat-bath). `I_tie = Y(M½) − Y(M)`,
  `I_off = Y(G) − Y(M½)`, `I_full = I_tie + I_off`. Matched G-Set triples that
  already exist — G43/G44/G45 (n=1000, m=9990) and G22/G23/G24 (n=2000, m=19990),
  all weights +1 — so no synthetic generator was needed. Flat T = 0.1, 8 held-in +
  8 held-out seeds, pre- and post-quench reported separately.
- **Result — CONFIRMED**, held-in and held-out, on every instance where the effect
  exists. **Tie handling carries 94–115% of every material `I_full`**; the off-tie
  residual never reaches materiality (max ρ 0.504, always at rel < 0.06%). G43
  −44.125 → −40.875 (share 0.963 → 1.058), G23 +57.0 → +54.75, G24 +45.0 → +49.1;
  all three replicate under the A6 rule (ratios 0.926 / 0.961 / 1.092). The
  decomposition closes **exactly** in all 24 cells. Same before and after
  `greedy_descent`, so the divergence exists at the quench's starting point.
- **Gradient (non-gating, pre-registered)** — the tie share must fall as T rises;
  measured **0.963 → 0.740 → 0.440** at T = 0.1 → 0.5 → 2.0, and `I_full` flips
  sign between 0.1 and 0.5. The reversal is specifically cold.
- **Implication for the operator library** — `metropolis_sweep.rs:117`'s `d <= 0.0`
  folds ties into the downhill branch. That one convention accounts for
  essentially the whole cold difference between two registry entries declared as
  different physical laws. Tie handling is a design axis this repository had never
  named. Making it an explicit parameter is trajectory-changing ⇒ open decision.
- **NOT explained — the sign.** G43 negative, G23/G24 positive; matched siblings
  disagree (G43 −44.1 vs G44 −0.25 vs G45 −5.4 at identical n, m, weights); tie
  *count* does not predict it (G43 has the fewest ties in set A and the largest
  effect). Declared open so no later answer can be presented as anticipated.
- **Novelty** — the mechanism is **LIKELY KNOWN** (it follows from the acceptance
  functions). What was not known here is that it accounts for ~100% of a material,
  replicating, instance-specific difference on real instances, with a monotone
  temperature gradient and a sign nothing measured predicts.
- **Limitations** — flat T = 0.1 (gradient on G43 only), one schedule, legacy init,
  six unweighted instances, cache-resident scale.
- **Descendant D-15 — REFUTED, and the refutation deflates the practical claim.**
  `Y` is **non-monotone** in the tie probability `q` on G43 with an interior
  optimum at `q = 0.75`, and the shape (verdict *and* argmin) **replicates
  exactly** on held-out; G23 and G24 are monotone to `q = 1`. Per Amendment 1
  A1.2, fixed before the data, this refutes D-15 and leaves H-15 untouched.
  **But the optimum does not pay:** `q = 0.75` beats `metropolis_sweep`
  materially and replicably (−51.8 / −45.4, ρ 6.2 / 3.2, p .008 / .016) and does
  **not** beat the heat-bath tie rule (−9.25 p .219 held-in, −2.13 p .617
  held-out, no replication). The curve is flat over `[0.5, 0.75]`. So the earlier
  suggestion of a `neutral_move_rate` policy variable is **not supported**; what
  is supported is per-instance *selection between two operators that already
  exist*. Solid across all three: `q = 0` is 3.0–3.1% worse than the best point
  everywhere — refusing neutral moves is the one clearly wrong setting, and
  `q = 0.25` recovers almost all of it.
- **Power limitation recorded** — Amendment A1.3's bootstrap-CI equivalence branch
  **never fired**: at 8 seeds the 95% CI is always wider than ±0.1% (half-widths
  0.09–0.21%), so every equivalence verdict rests on the non-materiality branch.
  Eight seeds cannot establish ±0.1% equivalence by CI.
- **Follow-up** — what sets the sign (open, no candidate); the neutral-network
  observables declared in Amendment 1 A1.6; decomposing G11/G32, which carry
  13–34% tie mass but whose RC-014 effects were measured on a **ladder**, so under
  the A1.1 identity that is a different quantity and needs its own
  pre-registration (A1.5). Record: `RC015_TIE_HANDLING.md`.

### Non-cycle records
`AXIOMS_OF_OPTIMIZATION.md` (eight axioms, validated against six historical
breakthroughs; our operators destroy none), `CHANNEL_EXHAUSTION.md` (**contains a
retracted law — see §2**), `RELATIONAL_PRIMITIVE.md` (Gauge Degeneracy theorem;
the relational primitive it proposes was later refuted by Representation
Invariance), `COMPETITIVE_ANALYSIS.md` (source-level comparison vs 7 cloned
solvers).

---

## 2. Truth maintenance — revocation register

Nothing is deleted. Each entry keeps its original context and names the
experiment that overturned it.

| # | Claim | Status | Why wrong | Disproved by |
|---|---|---|---|---|
| 1 | **"Easy-Information Law"** — ensemble information concentrates on the easy sub-problem, depleted on frustrated cores | **REVOKED** | Two independent fatal flaws: the control group was **empty** (G-Set has zero unfrustrated triangles, so "frustrated triangle" = "any triangle"), and a **degree confound** fully explains it (69.3 vs 26.3 mean local degree, forced by definition; ratio 0.505 ≈ ½). A density effect in a frustration costume. | `CHANNEL_EXHAUSTION.md` §7, three-way control split |
| 2 | "76% of pairwise covariance is emergent" | **CORRECTED** | That was 76% unexplained by *one* variable. Adding local degree absorbs half; sampling noise (~28% at R=32) covers most of the rest. | RC-004 §8.3 two-factor η² (0.42–0.55) |
| 3 | **Relational state primitive** as a source of power | **REVOKED** | Representation Invariance: a bijection or quotient by group G gains ≤ log₂\|G\| bits ⇒ **≤ 1 bit for Z₂**, independent of n. Its other half ("frustration = relational inconsistency") is a restatement of MaxCut. | `CHANNEL_EXHAUSTION.md` §1 |
| 4 | "Channel exhaustion" — retention is open but drained | **REVOKED** | Depended entirely on the Easy-Information Law. **Retention is simply open and unexplained**; the 2×-signal/+0.06%-gain puzzle is reopened. | follows from #1 |
| 5 | "ρ = 0.975 requires n = 9, so it did not come from that test" | **REVOKED (my error)** | Assumed *untied* Spearman. `evaluation.rs:110` averages ranks over ties, making intermediate values reachable at n = 5; the reproduce arm returns 0.9747. | RC-010 |
| 6 | ROADMAP: "cross-family transfer is real (… predictor LOO Spearman +0.747)" | **REVOKED in part** | The Spearman is *provably invariant* to instance features, so it cannot evidence transfer. **The portability 9/12 half stands.** | RC-011 |
| 7 | ROADMAP: World model "Spearman 0.975" | **CORRECTED** | Inflated by 4/5 candidates being verbatim training members; honest held-out ≈0.85–0.90. Not fabricated — the model genuinely ranks. | RC-010 |
| 8 | RC-007 first pass: "57% of pairs commute, 8 `inf` rows" | **REVOKED (instrument)** | All-zeros init makes replica-coupled operators inert and deterministic operators zero-variance ⇒ ρ divides by zero. Corrected: 100% commuting, 0 `inf`. | RC-007 `--diverse` re-run |
| 9 | RC-008 first pass: "not mixed at every temperature" | **REVOKED (instrument)** | Energy criterion was an absolute `1e-3`, below the sampling noise of a 16-replica mean. | RC-008 2σ re-run |
| 10 | Root `MEMORY.md`/`CONTEXT.md`: "SIMD is priority #1", "SIMD-first architecture" | **SUPERSEDED** | `src/core/simd_utils.rs` is declared in no `mod.rs` — orphaned, never compiled. Live plan is ROADMAP "Next 10", where SIMD is item 7. | marked inline in both files |

---

## 3. Validation matrix

| RC | Status | Math proof | Benchmarked | Tests | Doc | CI-gated |
|---|---|---|---|---|---|---|
| RC-001 | REFUTED (own criterion) | null-by-construction (λ=0) | 240+150 pairs | 4 unit | ✅ | via test suite |
| RC-002 | PARTIAL | mechanism from source | 240/cell × 6 T | — | ✅ | — |
| RC-003 | CONFIRMED | ✅ 3 kills | — | — | ✅ | — |
| RC-004 | CONFIRMED | ✅ E=2V−\|E\|, dS/dβ | measurement | — | ✅ | — |
| RC-005 | CONFIRMED | derivation + R² 0.93–0.97 | 4 instances | — | ✅ | — |
| RC-006 | CONFIRMED | ✅ exact identity | 200 checks, 5 families | **3** | ✅ | ✅ |
| RC-007 | REFUTED | — | 28 pairs × 36 runs | — | ✅ | — |
| RC-008 | PARTIAL | — | 6 T × 800 sweeps | — | ✅ | — |
| RC-009 | CONFIRMED | — | 18 ops × 3 backends | **2** | ✅ | ✅ |
| RC-010 | REFUTED (own prediction) | — | 3 sets × 9 cands | — | ✅ | — |
| RC-011 | CONFIRMED | ✅ invariance proof | 18,570-row DB | — | ✅ | — |
| RC-012 | CONFIRMED (3 layers) | — | 4 cond × 5 seeds + deployed config verbatim | — | ✅ | — |
| RC-013 | PARTIAL (P1 refuted, P2 confirmed) | — | 2 topo × 5 sizes × 2 φ + 3 floors | — | ✅ | — |
| RC-014 | PARTIAL (clause 1 confirmed, clause 2 REFUTED by D-14) | draw-identity measured | 6 inst × 2 arms × 16 seeds + D-14 + T-curve | — | ✅ | — |
| RC-015 | CONFIRMED | acceptance-ratio derivation | 6 inst × 3 arms × 16 seeds + gradient | — | ✅ | — |

"CI-gated" = an automated gate fails if the property regresses. RC-006 and RC-009
became tests and therefore run in the CI `test` job.

---

## 4. Reproduction

Every experiment is an executable in `src/bin/`. All are deterministic.

| RC | Command | Expected |
|---|---|---|
| 001 | `cargo run --release --bin exp_consensus -- --dose --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 --seeds 5 --temp-hi 1.0 --temp-lo 0.1` | λ=−2 row: 0/132 wins; λ=+2: mean +0.10%, p≈1.5e-5 |
| 002 | `cargo run --release --bin exp_initialization -- --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 --seeds 8` (+ `--deep`) | B DIFFERS at every T; C ERASED at T ≥ 0.25 |
| 004 | `cargo run --release --bin exp_field_capacity -- --dir benchmark_suite/data/gset` | mean\|m\| ratio ≈0.95–0.99; cov ratio ≈2.3–2.8 |
| 005 | `cargo run --release --bin exp_cost_model -- --file benchmark_suite/data/gset/G11` | spread ≥1.6×; turnover past φ=1/3 |
| 006 | `cargo run --release --bin exp_gradient_invariant -- --file benchmark_suite/data/gset/G1` | `worst absolute residual: 0.000000e0`, VERIFIED |
| 007 | `cargo run --release --bin exp_commutator -- --dir benchmark_suite/data/gset --sweeps 30 --seeds 6 --instances 6 --diverse` | 28/28 commuting, max ρ ≈1.32 |
| 008 | `cargo run --release --bin exp_mixing_time -- --file benchmark_suite/data/gset/G11 --replicas 16 --sweeps 800 --step 50` | T=0.1: q_AA = 1.0000; T ≥ 0.5: q_AB ≈ q_AA |
| 009 | `cargo test --release --test test_operator_backend_passport -- --nocapture` | 2 pass; `0 left the energy unchanged` |
| 010 | `cargo run --release --bin exp_world_model_audit` | A 0.9747 / B 0.8984 / C 0.8485 |
| 011 | `cargo run --release --bin exp_predictor_audit` | max deviation 1.110e-16; 0 violations; 0.7467 |
| 012 | `cargo run --release --bin exp_dynamics_audit` | 20 firings / 7 false (all thermal); Q2 all-zero predictions; Q3 `fit returned NONE (rows < 20)` |
| 015 | `cargo run --release --bin exp_tie_handling -- --controls` (then `--science`, `--holdout`, `--gradient`) | 3 controls pass; G43 I_full=−44.125 share 0.963; gradient 0.963→0.740→0.440 |
| 014 | `cargo run --release --bin exp_counterfactual -- --controls` (then `--science`, `--holdout`, `--d14`, `--tempcurve`) | 648/648 agreement; G22/legacy I=+37.875 ρ=1.737 p=0.0078; D-14a DEGENERATE_NULL; G43 T=0.1 I=−44.13 |
| 013 | `cargo run --release --bin exp_kernel_floors` | ring rows flat across ladder; random flip ns 2.1× rise; 1.9× ring-vs-random at 134 MB |

**Interpretation guide.** A *scale-free* metric (relative difference, ρ = ratio to
a null) is used wherever instances differ in magnitude. "Equivalence" is claimed
only from a bootstrap CI inside a pre-registered bound, never from p > 0.05.
Timing results use interleaved conditions and medians, because this host drifts
~9% on identical binaries (`PERF.md`).

---

## 5. Dependency graph — which component each result touches

```
RC-001 consensus thermostat ──▶ operators/ (2 registered: consensus_freeze/seek)
                             └─▶ knowledge: backbone-freezing closed

RC-002 quality/diversity ─────▶ executor.rs:124 init  [COST QUANTIFIED, NOT FIXED]
                             ├─▶ capability system: Warmstart/ExactInference justified empty
                             └─▶ every later cycle: the all-zeros confound

RC-003 assumption audit ──────▶ architecture: A10 identified as universally unbroken
RC-004 architecture space ────▶ BENCHMARK VALIDITY: G-Set degenerate on 2 axes
                             └─▶ any guide/frustration experiment must use biqmac

RC-005 flip density ──────────▶ SCHEDULER / DecisionEngine utility model  [OPEN]
                             └─▶ operator cost_model (all 18)             [OPEN]

RC-006 gradient ledger ───────▶ AUDIT: O(n) cross-check   [TEST LANDED]
RC-007 commutativity ─────────▶ EVOLUTION ENGINE ordering search           [OPEN]
RC-008 mixing/collapse ───────▶ PT assumptions; scope of RC-003's ergodic premise
                             └─▶ reconciles RC-002 (ladder preserved the memory)

RC-009 backend passport ──────▶ BACKEND ROUTING / ADR-0001 selection  [TEST LANDED]
RC-010 world model ───────────▶ World-model INTERPRETATION (ROADMAP corrected)
RC-011 predictor ─────────────▶ Predictor INTERPRETATION (ROADMAP corrected)
RC-012 dynamics ──────────────▶ EARLY-STOP CONTROLLER: unreachable as shipped;
                             └─▶ ε-gate vacuous ⇒ "stop at min_frac"      [OPEN]
RC-013 kernel floors ─────────▶ ADR-0003 quantified (locality ~2× at DRAM)
                             ├─▶ simd_utils.rs decision: evidence leans DELETE
                             └─▶ BENCHMARK SCALE: G-Set ledger is cache-resident
```

---

## 6. Code-integration decisions

For each verified invariant: does it become a test, an assertion, or nothing?

| Result | Decision | Rationale |
|---|---|---|
| **RC-006** gradient-ledger identity | **unit + regression test** (`tests/test_gradient_ledger_invariant.rs`, 3 tests) | Exact identity, cheap, backend-independent. Covers construction, 200 sequential flips, and a real G-Set instance. |
| RC-006 as a **runtime debug assertion** | **NOT done** | Would sit in the read-only core hot path; needs a bit-identity A/B before any insertion (ADR-0004). Recorded as an open question. |
| **RC-009** passport conformance | **property-style regression test** (`tests/test_operator_backend_passport.rs`, 2 tests) | Iterates the *registry*, so newly registered operators are audited automatically — the library is expected to grow. |
| RC-009 vacuity guard | **assertion inside the test** | Fails if ≥half the operators are inert, so the suite cannot pass vacuously. |
| **RC-001** λ=0 ≡ metropolis | **unit test** (in `ensemble_thermostat.rs`) | The null lives inside the algorithm; bit-identity must not silently break. |
| **RC-005** corrected cost law | **NOT a test** | Asserting it would fix wall-clock ratios into the suite, and this host drifts ~9%. It is a *finding about the model*, and fixing the model is trajectory-changing. |
| **RC-007** commutativity | **NOT a test** | ρ ≈ 1.3 vs 1.0 is within noise at this power; a threshold test would be flaky. The actionable part is narrowing the Evolution search space, which needs approval. |
| **RC-008** mixing thresholds | **NOT a test** | Depends on instance, R and sweep budget; a fixed threshold would be brittle. Diagnostic binary retained instead. |
| **RC-010 / RC-011** metric audits | **NOT tests** — documentation corrections | They audit *claims*, not code behaviour. RC-011's proof is structural: it would only change if interaction terms were added, which is itself the proposed remedy. |
| **RC-004** `E = 2V − |E|` | **NOT a test** | A statement about instance families, not about code. Recorded in `OPEN_PROBLEMS` so no future experiment repeats the mistake. |

## 7. CI coverage

`.github/workflows/ci.yml` runs four jobs — `lint`, `test`, `docs`, `regression`.
Two gaps were found and closed this session:

- **CI never compiled the `research/` workspace member.** `cargo metadata` shows
  `workspace_default_members` = the root package alone, so `cargo build
  --all-targets` skipped it entirely — the structural reason a broken target
  there (E0063) survived. Fixed by adding `--workspace` to build/clippy/test and
  `--all` to fmt.
- **No documentation gate existed**, which is how 18 rustdoc errors accumulated
  across 9 files. Added a `docs` job with `RUSTDOCFLAGS: -D warnings`.

RC-006 and RC-009 land in the `test` job automatically. All 7 `run:` commands in
the workflow were extracted and executed locally: **all pass**.

**Not added as CI gates, deliberately:** the experiment binaries. They take
minutes, and RC-005/007/008 depend on wall-clock or low-power statistics that
would make CI flaky. They are reproduction commands (§4), not gates.

## 8. Final audit

**Which discoveries changed the architecture?** Strictly, **none yet**. Two landed
as permanent tests (RC-006, RC-009), which changes the *verification* surface, not
the runtime. Four findings that *would* change architecture — RC-005's cost model,
RC-007's degenerate ordering search, RC-011's two remedies — are all
**trajectory-changing under ADR-0004** and are recorded as open decisions rather
than applied unilaterally.

**Which changed only documentation?** RC-003, RC-004, RC-008, RC-010, RC-011 —
plus the ROADMAP corrections and the supersession of root `MEMORY.md`/`CONTEXT.md`.

**Which hypotheses were disproven?** RC-001 (on its own pre-registered bar),
RC-002's threshold framing, RC-007, RC-010, and my own forensic claim inside
RC-010. Also the Easy-Information Law and the relational primitive — both revoked
in §2. **Six of eleven cycles refuted something I had asserted.**

**What remains unverified?**
- The 2×-signal/+0.06%-gain puzzle: **reopened and unexplained** since the
  Easy-Information Law was revoked.
- `needs_temperature` and `needs_integer` passport fields (RC-009 audited neither).
- RC-007 beyond pairs; RC-005 beyond one operator/backend; RC-008 beyond one
  instance.
- The `Dynamics` early-stop model — never audited.
- `src/core/simd_utils.rs`: orphaned, never compiled; delete-or-implement undecided.

**Single highest-value next experiment (as of RC-011).** Audit **`Dynamics`** — the last
unaudited learned model, and the only one whose errors are *silent*: a wrong
remaining-improvement prediction halts a run early and costs quality with no
observable failure. The other three models are advisory (`filter`, ranking);
Dynamics *acts*. Same method as RC-010/011: ask first what its reported metric is
mathematically capable of detecting.

**Done — RC-012.** All four learned models are now audited: Predictor (metric
provably transfer-blind), Policy (unaudited claim: wins 5/5 — the one remaining
gap), World (inflated but genuine), Dynamics (three-layer failure). Next
highest-value: the **Policy "wins 5/5"** claim, the last headline number with no
audit; and the open-decision backlog in `memory/OPEN_PROBLEMS.md` §0, which now
has five entries awaiting the maintainer.
