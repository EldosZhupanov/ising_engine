# Research Cycle 001 — Ensemble-Consensus Thermostats

**Status:** hypotheses registered *before* implementation and *before* any run.
This file is pre-registration: the falsification criteria below were fixed in
advance, so a negative result cannot be re-labelled as a success afterwards.

---

## 1. Analysis — what this substrate has that nobody exploits

Reading `src/engine_v2/state.rs`, the `SpinState` contract exposes something no
competitor's data model does:

```rust
fn delta_e_into(&self, site: usize, out: &mut [f64]);   // ΔE at one site, ALL replicas
fn spin(&self, site: usize, replica: usize) -> bool;    // O(1) bit read
fn overlap(&self, a: usize, b: usize) -> f64;           // cheap on bit planes
```

Every operator in the library — all 14 — consumes only the *first* of these
per-replica and treats replicas as **independent chains**. `ReplicaExchange`
couples them at the level of whole configurations; `HoudayerClusterMove` and
`IsoenergeticClusterMove` couple exactly two of them through overlap domains.

**Nobody uses the ensemble as a per-site statistical estimator.**

Compare: D-Wave Neal (`cpu_sa.cpp`) runs `num_reads` sequentially and shares
nothing between them. OpenJij's `single_spin_flip.hpp` updates one system at a
time. CP-SAT shares *bounds* between workers, not *variable-level marginals*.

Yet at every site the ensemble already tells us

  m_i = ⟨s_i⟩ over replicas  ∈ [−1, 1],   c_i = |m_i| ∈ [0, 1]

for the cost of R bit reads — the same order as the ΔE loop we are already
running. **The information is free and it is being discarded.**

This connects directly to our own recorded finding
(`state-dependent-operator-selection`): the best operator is state-dependent and
the frontier is a *joint temperature × operator* policy. c_i is a per-site state
descriptor. A thermostat driven by c_i is that joint policy, pushed down to the
variable level.

## 2. Hypotheses generated

Six ideas were generated; all six are evaluated in §3, one is implemented.

| # | Idea | Core principle | Novelty | Cost |
|---|---|---|---|---|
| **H1** | **Ensemble-Consensus Thermostat (ECT)** — per-site temperature modulated by ensemble consensus | information-theoretic | **high** | **zero asymptotic** |
| H2 | Conditional pair-flip (order-2 barrier escape via flip-read-unflip) | second-order neighborhood | medium (≈ 2-flip / Kernighan–Lin) | ×2 |
| H3 | Overlap-directed valley transport (cluster replicas, push the worst to the complement) | RSB geometry | high | O(R²n) |
| H4 | Fisher-information site scheduling (sweep by Var_r[ΔE_i]) | information geometry | high | +sort |
| H5 | Wang–Landau entropic acceptance | density of states | low (port) | histogram |
| H6 | Renormalization: contract strong couplings to super-spins, solve coarse, project | hierarchical | high | large |

**H1 is selected** for cycle 001 because it has the best novelty-to-cost ratio,
zero asymptotic overhead, and — decisively — **an embedded null control**
(§4).

## 3. Scientific evaluation — H1, Ensemble-Consensus Thermostat

### Definition

At site i, with ensemble magnetization m_i and consensus c_i = |m_i|, replica r
at ladder temperature T_r proposes a flip accepted with probability

    P = min(1, exp(−ΔE / T_eff)),      **T_eff(i, r) = T_r · exp(λ · c_i)**

λ ∈ ℝ is the single parameter.

- **λ = 0** ⇒ `exp(0) = 1` exactly ⇒ **T_eff ≡ T_r** ⇒ the operator is
  *bit-identical* to `MetropolisSweep`. The null hypothesis is inside the
  algorithm.
- **λ < 0** ⇒ consensus sites run **colder**. Interpretation: *backbone
  freezing.* Where every replica agrees, treat the assignment as discovered
  structure and protect it; spend thermal noise on the undecided sites.
- **λ > 0** ⇒ consensus sites run **hotter**. Interpretation: *curiosity.* Where
  every replica agrees, the ensemble holds **zero information about the
  alternative branch**; no amount of further independent sampling will ever test
  it. The maximally informative move is precisely to attack the consensus.

These two readings are **opposite and both a priori plausible**. That is what
makes this an experiment rather than an implementation.

### Problem solved
Single-spin thermal dynamics allocates noise uniformly over variables, but
frustration is heterogeneous: rigid ferromagnetic motifs and frustrated loops get
the same temperature. ECT gives every variable its own thermostat, driven by a
statistic that is free.

### Scientific intuition
The free energy is F = E − TS. In an ensemble at fixed T, the entropy is carried
almost entirely by the low-|m| (liquid) sites; the high-|m| (condensed) sites
carry the energy. A single global T is a compromise between descending in E and
retaining S. Splitting T by site is the smallest possible generalization that
lets the two be steered independently.

### Expected advantages
- Zero asymptotic overhead; same complexity class as Metropolis.
- Adapts *online and per-variable*, with no schedule to tune and no extra state.
- Reduces to a verified operator at λ = 0, so it can never be worse than
  Metropolis by construction error — only by the λ term being wrong.

### Expected weaknesses
- **Breaks detailed balance.** A site-dependent temperature is not the Gibbs
  measure of any Hamiltonian, so the operator must be typed `BehaviorChanging`
  and its output canonically re-scored. It is an optimizer, not a sampler.
- **Ladder confound (important, stated up front).** On a PT ladder, replicas sit
  at different temperatures; the hot replicas are near-random and *dilute* c_i
  toward 0. ECT's signal is therefore weakest exactly where the ladder is widest.
  This is a genuine limitation and is not designed around in cycle 001.
- Meaningless at R = 1 (c ≡ 1, so it degenerates to a constant rescale of T).
  Declared honestly via `needs_replicas: true`.

### Complexity
- **Time:** O(sweeps · (n + 2m) · R) — identical to `MetropolisSweep`, plus R
  bit reads per site, which is the same order as the ΔE loop already performed.
- **Memory:** O(R) scratch. **No additional allocation over Metropolis.**
- **Scalability:** identical to Metropolis; no new bottleneck.

### When it should outperform
Instances with a strong backbone — a large set of variables taking the same value
in nearly all good solutions (structured / low-density / planted instances). λ<0
should protect it; λ>0 should be able to break out of a *wrong* backbone.

### When it should fail
Instances with no backbone (dense random ±J at high frustration): c_i is then a
noise estimate, and modulating T by noise strictly adds variance to a
well-calibrated Metropolis chain. **Prediction: ECT ≈ Metropolis or worse on
dense random instances.**

### Falsifiable hypothesis (pre-registered)

> **H1.** There exists a sign of λ for which the Ensemble-Consensus Thermostat
> attains a lower mean best energy than `metropolis_sweep` (λ = 0) at *identical
> sweep budget, identical replica count, identical temperature ladder and
> identical seeds*, with p < 0.05 under a two-sided Wilcoxon signed-rank test on
> paired (instance, seed) results.

### Success criteria
p < 0.05 **and** a median relative improvement > 0.1 %, on ≥ 20 paired
observations.

### Failure criteria
p ≥ 0.05, or an improvement of the wrong sign. **Both signs of λ failing is a
publishable refutation**, and closes off a plausible research direction.

## 4. Experimental design (fixed in advance)

| Element | Setting |
|---|---|
| Arms | `metropolis_sweep` (λ = 0, **null**), `consensus_freeze` (λ = −1), `consensus_seek` (λ = +1) |
| Instances | G-Set (real benchmark graphs), all available |
| Seeds | fixed set, **identical across arms** (paired design) |
| Budget | identical `sweeps` and `replicas` for every arm |
| Ladder | identical `temp_hi` / `temp_lo` for every arm |
| Metric | canonical best energy from `ExperimentOutcome::score` (the canonical scorer, not the operator's own claim) |
| Statistic | two-sided **Wilcoxon signed-rank** on paired (instance, seed) |
| Baseline | the null arm is the *same operator at λ = 0*, and is verified bit-identical to `metropolis_sweep` by unit test |
| Confidence | p < 0.05 |
| Expected outcome | genuinely uncertain; both signs are defensible |

Execution goes through `RuntimeExecutor` / `BatchExecutor` — the canonical
harness the platform already uses — so no bespoke measuring device is
introduced.

---

## 5. Results

Harness `src/bin/exp_consensus.rs`, 30 G-Set instances, `RuntimeExecutor`,
canonical scorer, paired on (instance, seed), identical budget/replicas/ladder
per arm. `sweeps=100`, `replicas=32`.

### 5.0 A statistical error I made and corrected

The first analysis ran Wilcoxon on **raw energies** pooled across instances.
G-Set energies span an order of magnitude, so the signed ranks were dominated by
the largest graphs — a scale confound in my own instrument, not in the operator.
The reported numbers below rank the **scale-free relative differences**. The
correction changed conclusions materially (λ>0 on the wide ladder went from
p = 0.107 to p = 1.3e-3), which is precisely why it is recorded rather than
silently fixed.

### 5.1 Three-arm trial, 240 paired observations

| Ladder | Arm | mean rel | median rel | W/L/T | n_eff | Wilcoxon p |
|---|---|---|---|---|---|---|
| 4.0→0.1 | freeze λ=−1 | +0.0127% | 0.0000% | 103/90/47 | 193 | 3.43e-1 |
| 4.0→0.1 | seek λ=+1 | +0.0573% | 0.0000% | 109/82/49 | 191 | **1.30e-3** |
| 1.0→0.1 | freeze λ=−1 | −0.0791% | −0.0525% | 48/143/49 | 191 | **2.03e-11** |
| 1.0→0.1 | seek λ=+1 | +0.0339% | 0.0000% | 113/83/44 | 196 | **7.57e-3** |

### 5.2 Dose–response, 150 paired observations per λ

A single significant point is not a mechanism. If the consensus term is real,
effect size must move with λ and pass through zero at λ = 0 — where the operator
is *provably* Metropolis (unit-tested bit-identity).

| λ | narrow ladder (1.0→0.1) | | | wide ladder (4.0→0.1) | | |
|---|---|---|---|---|---|---|
| | mean rel | W/L | p | mean rel | W/L | p |
| **−2.0** | **−0.7317%** | **0/132** | **~0** | −0.0798% | 44/79 | 1.16e-4 |
| −1.0 | −0.0745% | 30/86 | 3.11e-7 | +0.0062% | 60/61 | 9.88e-1 |
| −0.5 | −0.0054% | 62/64 | 6.97e-1 | −0.0009% | 57/64 | 7.10e-1 |
| *0* | *— (exact null)* | | | *—* | | |
| +0.5 | +0.0258% | 65/55 | 2.54e-1 | +0.0205% | 63/58 | 5.24e-1 |
| +1.0 | +0.0429% | 76/49 | 6.95e-3 | +0.0429% | 65/55 | 4.77e-2 |
| **+2.0** | **+0.1029%** | 79/48 | **1.55e-5** | **+0.0931%** | 78/51 | **1.36e-3** |
| +3.0 | +0.0863% | 79/47 | 5.28e-5 | +0.0652% | 83/50 | 9.12e-3 |

### 5.3 Verdict against the pre-registered criterion

> Success required **p < 0.05 AND median relative improvement > 0.1%**.

At the best setting (λ = +2.0, narrow ladder): p = 1.55e-5 ✓, **median rel =
+0.0426% ✗** — less than half the threshold. The *mean* clears 0.1%; the
**median, which is what was pre-registered, does not**.

**H1 is therefore NOT satisfied. The pre-registered hypothesis fails on effect
size.** Switching to the mean because it flatters the result is exactly what
pre-registration exists to prevent, and it is declined here.

### 5.4 What was nevertheless established

The mechanism is real, confirmed on three independent axes:

1. **Sign symmetry through an exact zero.** Effects are opposite either side of
   λ = 0, where the operator is bit-identical to Metropolis by unit test — so the
   zero point is analytic, not fitted.
2. **Monotone dose–response with an interior optimum** near λ ≈ +2 in both
   ladder conditions independently.
3. **Ladder-dilution confirmed as predicted.** §3 predicted *before running* that
   a wide ladder randomizes hot replicas, shrinking c_i and attenuating the
   effect. The λ=−2 harm collapses from −0.73% (narrow) to −0.08% (wide) — a 9×
   attenuation in the predicted direction.

### 5.5 The strongest finding is the negative one

**Backbone freezing is decisively harmful.** At λ = −2 on the narrow ladder the
operator won **0 of 132** non-tied pairs. Protecting the variables the ensemble
agrees on — the intuitive move, and the premise of backbone-guided local search —
is not merely neutral here but catastrophic, and it gets *worse* precisely as the
consensus signal gets *stronger*.

Mechanistic reading: consensus in a thermal ensemble is substantially an artifact
of **shared initialization and insufficient mixing**, not of true backbone
structure. Cooling those sites converts a sampling artifact into a hard
constraint and freezes the ensemble into whichever basin it started in.

The counterintuitive direction — heating what the ensemble agrees on, because
agreement means zero information about the alternative branch — is the one that
helps. It is the optimizer-level analogue of the platform's own curiosity
principle, and its effect is real, replicated, and **small (~0.04–0.10%)**.

## 6. Knowledge extracted

- **Refuted:** ensemble-consensus freezing (λ<0) as an optimization principle for
  MaxCut under thermal dynamics. Large effect, p ≈ 0, replicated in two ladder
  conditions. Do not revisit without a mixing-diagnostic that separates true
  backbone from initialization artifact.
- **Supported but immaterial:** anti-consensus heating (λ>0), optimum λ ≈ +2,
  ~0.09% mean gain, p ≈ 1e-5 — below the materiality bar set in advance.
- **Method:** ensemble consensus c_i is a free per-site state descriptor. It
  carries real signal, but the signal is contaminated by ladder width. Any future
  use must condition on replica-temperature spread.
- **Instrument:** pooled Wilcoxon on raw energies across heterogeneous instances
  is scale-confounded. Rank relative differences.

## 7. Descendants for cycle 002

1. **Consensus restricted to the cold sub-ladder** — compute c_i over only the
   coldest k replicas, removing the dilution the data just quantified. Directly
   addresses the one confound that is now measured rather than assumed.
2. **Mixing-corrected consensus** — weight c_i by per-site flip frequency to
   separate "agreed because converged" from "agreed because never moved". This is
   the direct test of §5.5's mechanistic reading.
3. **H4 (Fisher-information site scheduling)** — Var_r[ΔE_i] rather than |⟨s_i⟩|
   as the per-site descriptor; a second-moment statistic where c_i is a first.

## 8. Reproduction

```bash
cargo test --release --lib engine_v2::operators::ensemble_thermostat   # 4/4, incl. λ=0 bit-identity
cargo run --release --bin exp_consensus -- --dose \
    --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 --seeds 5 \
    --temp-hi 1.0 --temp-lo 0.1
```

Deterministic: same seeds ⇒ same trajectories ⇒ same table.
Gates at time of recording: **313 tests passed / 0 failed**, clippy clean, fmt clean.
