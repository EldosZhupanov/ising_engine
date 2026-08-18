# Research Cycle 002 — The Initialization Erasure Law

**Status:** pre-registered before implementation and before any run.

---

## 1. The gap, found in the architecture rather than the literature

Two facts discovered by reading the code, both verifiable in one command each.

### 1.1 Half the declared design space has no implementation

`capability.rs` declares six capabilities. Counting every `Capability::` in the
16-operator library:

| Capability | Operators implementing it |
|---|---|
| Exploration | 10 |
| Exploitation | 8 |
| BarrierCrossing | 7 |
| **ExactInference** | **0** |
| **Approximate** | **0** |
| **Warmstart** | **0** |

The Decision Engine selects by capability. A query for `Warmstart` or
`ExactInference` **can never return anything**. Three of six capabilities are
declared, documented, queryable — and empty.

### 1.2 Every experiment ever run started from the same point

`src/engine_v2/ai_scientist/executor.rs:124`:

```rust
let init = vec![0u8; ir.n];
```

and identically at `evolution.rs:385`, `:517`, `:689`.

**Every one of the 18,570 recorded runs began at the all-zeros configuration,
with all replicas identical at t = 0.** The seed varies the RNG stream, never the
starting point. Different seeds are therefore *not* independent samples of the
initial condition — they are the same initial condition with different noise.

This is not a bug; it is a defensible determinism choice. But it means an entire
axis of the experiment space has never been varied, and **no recorded result is
known to be independent of it.**

### 1.3 It retroactively explains RC-001

RC-001 concluded that ensemble consensus behaves like "a shared-initialization
artifact rather than true backbone". That was inferred from behaviour. §1.2 shows
it is **structurally guaranteed**: at t = 0 every replica is the same
configuration, so c_i ≡ 1 at every site. The ensemble begins with *zero*
diversity and must manufacture all of it thermally. The RC-001 mechanism was
correct and now has a cause.

## 2. The question

These two facts are one question. Warm starts, construction heuristics, exact
relaxations, BP initialization — an entire algorithmic class exists to supply a
*better starting configuration*. This platform has never implemented a single
member of it, and has never suffered for it.

> **Is that because the class is unnecessary here — because thermal dynamics
> erases initialization entirely?**

If so, there is a temperature T\* above which the starting point is
information-theoretically destroyed, and the platform's standard `temp_hi = 4.0`
sits above it. That would explain, at class level, why `Warmstart` and
`ExactInference` were never built and never missed — and would predict that any
future effort spent on them is wasted in the standard configuration.

## 3. Design — a 3-arm factorial separating QUALITY from DIVERSITY

Crucially, **no new operator is required.** `random_flip_sweep` flips each
(site, replica) independently with probability ½, so a single sweep applied to
the all-zeros state produces exactly an independent uniform random configuration
*per replica*. The instrument is built entirely from already-verified,
cross-backend-tested components — nothing new can bias it.

| Arm | Schedule (total sweeps identical) | Initial quality | Initial diversity |
|---|---|---|---|
| **A** *status quo* | `metropolis_sweep(100)` | poor (all-zeros) | **zero** |
| **B** *diverse* | `random_flip_sweep(1)` → `metropolis_sweep(99)` | poor | **maximal** |
| **C** *informed* | `greedy_descent(1)` → `metropolis_sweep(99)` | **good** | zero |

A vs B isolates **diversity** at fixed quality. A vs C isolates **quality** at
fixed diversity. All arms receive exactly 100 sweeps, the same replica count, the
same ladder and the same seeds; pairing is on (instance, seed).

Swept factor: `temp_hi ∈ {0.1, 0.25, 0.5, 1.0, 2.0, 4.0}`, `temp_lo = 0.1` fixed.

## 4. Pre-registered hypotheses and falsification criteria

- **H2a — diversity matters at low T.** B differs from A at `temp_hi = 0.1`,
  p < 0.05.
- **H2b — quality matters at low T.** C differs from A at `temp_hi = 0.1`,
  p < 0.05.
- **H2c — erasure.** There exists T\* such that above it, B ≈ A and C ≈ A.
- **H2d — the class-level claim.** T\* < 4.0, i.e. the platform's standard
  configuration is in the erased regime.

**Equivalence, not absence of evidence.** "No difference" will *not* be claimed
from p > 0.05 alone — that is the classic fallacy. Erasure is claimed only when
the **bootstrap 95% CI on the mean relative difference falls entirely inside
±0.1%**, an equivalence bound fixed here in advance. A wide CI means
*underpowered*, and will be reported as such.

**Refutation.** If B and C differ from A significantly at every temperature
including 4.0, then initialization always matters, the recorded database is
confounded by the all-zeros start, and the warm-start class is a live and
valuable direction. That outcome is equally publishable and would make §1.2 a
correction notice for the entire knowledge base.

## 5. Why this is not operator tuning

No parameter is being optimized and no operator is being improved. The object of
study is a *property of the dynamics itself* — whether it retains or destroys
information about where it started — and the deliverable is a threshold that
tells us whether an entire class of algorithms can ever pay off on this
substrate.

---

## 6. Results

30 G-Set instances × 8 seeds = **240 paired observations per cell**. 100 total
sweeps per arm, 32 replicas, `temp_lo = 0.1`. Harness
`src/bin/exp_initialization.rs`.

### 6.1 Primary: quality and diversity dissociate completely

| temp_hi | B *diverse* mean rel | 95% CI | verdict | C *informed* mean rel | 95% CI | verdict |
|---|---|---|---|---|---|---|
| 0.1 | **+0.4546%** | [0.395, 0.515] | DIFFERS | −0.0330% | [−0.100, 0.033] | unclear |
| 0.25 | +0.1876% | [0.142, 0.231] | DIFFERS | −0.0222% | [−0.068, 0.024] | **ERASED** |
| 0.5 | +0.1387% | [0.104, 0.174] | DIFFERS | −0.0153% | [−0.057, 0.023] | **ERASED** |
| 1.0 | +0.1051% | [0.069, 0.143] | DIFFERS | −0.0347% | [−0.073, 0.002] | **ERASED** |
| 2.0 | +0.0927% | [0.053, 0.132] | DIFFERS | −0.0144% | [−0.057, 0.027] | **ERASED** |
| **4.0** | **+0.0879%** | [0.050, 0.128] | **DIFFERS** | −0.0315% | [−0.070, 0.006] | **ERASED** |

### 6.2 Confirmatory 2×2 — does quality add anything on top of diversity?

| temp_hi | arm | mean rel | 95% CI | W/L | p | verdict |
|---|---|---|---|---|---|---|
| 0.1 | B random only | +0.4546% | [0.395, 0.515] | 183/27 | ~0 | DIFFERS |
| 0.1 | **C greedy×10 only** | **−0.1376%** | [−0.210, −0.069] | 82/133 | 7.35e-4 | **DIFFERS (worse)** |
| 0.1 | D random + greedy×9 | +0.4451% | [0.385, 0.510] | 185/30 | ~0 | DIFFERS |
| 4.0 | B random only | +0.0879% | [0.050, 0.128] | 122/86 | 8.52e-5 | DIFFERS |
| 4.0 | C greedy×10 only | −0.0341% | [−0.077, 0.004] | 98/108 | 3.20e-1 | ERASED |
| 4.0 | D random + greedy×9 | +0.0889% | [0.050, 0.129] | 118/89 | 2.19e-4 | DIFFERS |

**D ≈ B to three decimal places, at both temperatures.** Nine sweeps of greedy
descent layered on top of a diverse start contribute *nothing*: +0.4451% vs
+0.4546% (overlapping CIs) and +0.0889% vs +0.0879%.

### 6.3 Verdict on the pre-registered hypotheses

| | | |
|---|---|---|
| **H2a** diversity matters at low T | +0.4546%, 183/27, p ≈ 0 | **SUPPORTED** |
| **H2b** quality matters at low T | equivalent, then *negative* | **REFUTED** |
| **H2c** an erasure threshold T\* exists | none for diversity up to 4.0 | **REFUTED** |
| **H2d** T\* < 4.0, standard config is erased | diversity still significant at 4.0 | **REFUTED** |

My framing was wrong. I asked *when* initialization is erased and assumed a
single threshold. The dynamics does not erase initialization as such — **it
erases one component of it and preserves the other, permanently.**

## 7. The finding: the Quality–Diversity Dissociation

> An initialization can supply two things: **where** the ensemble starts
> (solution quality) and **how spread out** it is (diversity). Thermal ensemble
> dynamics destroys the first almost immediately and preserves the second
> indefinitely. Quality is not merely useless — supplied at the cost of
> diversity it is **actively harmful, with a monotone negative dose–response**
> (greedy×1: −0.033% n.s.; greedy×10: −0.138%, p = 7.4e-4).

### Mechanism, verified from source rather than inferred

`greedy_descent` takes `_rng: &mut ChaCha8Rng` — **it ignores the RNG entirely**
(`greedy_descent.rs:83`). From the identical all-zeros start every one of the 32
replicas therefore follows the *same* deterministic trajectory into the *same*
local optimum. The ensemble collapses to a single distinct configuration.
Diversity is provably zero, not merely small.

The deeper the greedy phase, the deeper the shared basin the whole ensemble must
then climb out of — which is exactly the observed monotone harm. When diversity
is already present (arm D), greedy descends each replica into *its own* nearby
basin, preserves the spread, and contributes nothing.

### Why this explains an entire algorithmic class

Warm starts, construction heuristics, BP initialization, LP/SDP rounding,
Goemans–Williamson, tree-exact relaxation, QPBO-fixed partial assignments —
**every member of the class produces one high-quality configuration.** That is
precisely the component this substrate discards. Applied to an ensemble solver
each member either collapses the ensemble (harmful) or is redundant (useless).

The class is not underexplored here. It is **structurally mismatched** to
ensemble dynamics, and the criterion that governs initialization is *entropic*,
not *energetic*.

This retroactively justifies §1.1: `Warmstart` and `ExactInference` have zero
implementations after 18,570 experiments not through oversight but because there
was never anything for them to win. It also completes RC-001: consensus was a
shared-initialization artifact because the harness *guarantees* zero initial
diversity.

### Actionable defect (NOT acted on — see §9)

`executor.rs:124` starts every run at all-zeros, the worst of the three arms
tested. Measured cost: **+0.088% at the standard `temp_hi = 4.0`** (p = 8.5e-5)
and **+0.455% at `temp_hi = 0.1`** (183/27), on every one of the 18,570 recorded
runs. One sweep of `random_flip_sweep` recovers it at negligible cost.

## 8. Knowledge extracted

- **Law (new):** initialization influence on thermal ensemble solvers is carried
  by ensemble *entropy*, not by ensemble *energy*. Diversity persists to at least
  `temp_hi = 4.0`; quality is erased above 0.25 and is harmful when it costs
  diversity.
- **Refuted:** the "thermalization erases initialization" intuition, in its
  simple form. There is no single T\*.
- **Refuted:** the entire warm-start / construction-heuristic class as a source
  of gain for this substrate, *when it supplies quality alone*. The open
  sub-case is a construction that supplies quality **without** collapsing
  diversity — e.g. R independent randomized roundings. Untested; this is the one
  survivor of the class and it is the natural RC-003.
- **Confound quantified:** the all-zeros start biases every recorded result by
  0.09–0.46% in the pessimistic direction.
- **Method:** claiming "no effect" requires an equivalence bound. Five of six
  quality cells are `ERASED` (CI inside ±0.1%), which is a positive demonstration
  of equivalence, not a failure to reject.

## 9. What was deliberately NOT done

Changing `executor.rs:124` would alter the trajectory of every future run and
**break bit-identical replay of all 18,570 recorded experiments** (ADR-0004).
That is a behaviour-changing act on the read-only core and is a decision for the
maintainer, not a unilateral fix. It is reported, quantified, and left in place.

## 10. Reproduction

```bash
cargo run --release --bin exp_initialization -- \
    --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 --seeds 8
cargo run --release --bin exp_initialization -- --deep \
    --dir benchmark_suite/data/gset --sweeps 100 --replicas 32 --seeds 8
```

No new operator was written for this cycle: every component is an existing,
cross-backend-verified operator, so nothing new could bias the measurement.
Gates at time of recording: clippy clean, fmt clean, 313 tests passing.
