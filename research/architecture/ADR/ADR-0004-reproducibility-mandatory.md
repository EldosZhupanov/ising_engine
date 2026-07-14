---
id: ADR-0004
title: Bit-identical reproducibility is mandatory, not optional
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [Bit-Identical-Determinism, implements, CONSTITUTION#4]
  - [Golden-Regression, implements, Bit-Identical-Determinism]
  - [AB-Harness, implements, Bit-Identical-Determinism]
  - [Integer-Fields, uses, Bit-Identical-Determinism]
  - [Bit-Identical-Determinism, validated_by, EXP-PERF-2026-06]
  - [Bit-Identical-Determinism, motivated_by, WSL2-Timing-Drift]
---

# ADR-0004 — Why reproducibility is mandatory

## Context

Stochastic optimizers are conventionally excused from reproducibility: "it's
randomized, results vary." This excuse destroys three things this project
cannot function without:

1. **Correctness verification.** A performance change to a stochastic engine
   that "seems about as good" is unverifiable — quality differences of 0.1%
   need thousands of runs to detect statistically, but a single golden replay
   detects *any* trajectory change instantly. Bit-identical replay is the
   cheapest, strongest correctness instrument available.
2. **Honest measurement.** The benchmark host (WSL2) shows up to ~9% same-host
   timing drift (WSL2-Timing-Drift, PERF.md). If correctness were entangled
   with timing-sensitive comparisons, no A/B would be trustworthy. Because
   energies must be *bit-identical* under identical seeds, the correctness
   half of every A/B is timing-independent by construction; only the speed
   half needs back-to-back runs.
3. **Scientific claims.** Every result in the claim-anchored evidence base
   must be replayable from (problem, plan, seeds). A non-reproducible result
   is an anecdote.

The 2026-06 campaign proved the discipline pays: seven optimizations
(cumulative ×1.37) merged with zero quality risk, because each was proven to
produce byte-identical trajectories (EXP-PERF-2026-06); the one regression
(lane-halving, ×0.98) was caught and reverted by the same gate.

## Decision

Determinism is a **load-bearing architectural property** (Constitution §4.5):

1. Given (problem, plan, seeds, thread layout), trajectories are bit-identical
   across runs. The golden regression must pass byte-identical; the A/B
   harness asserts energy equality before any speed claim.
2. Changes that alter rounding or ordering — FP contraction (`mul_add`),
   reordered float reductions, reduced-precision RNG compares, hot-loop
   iteration-order changes — are trajectory changes: rejected by default,
   admissible only as explicitly typed behavior-changing operators.
3. The sparse backend uses **integer fields** partly *for this reason*:
   integer arithmetic has no rounding, no reassociation hazard — determinism
   stops being a tax and becomes free (and faster).
4. Adaptive control must also replay: controllers read sensors and seeded
   streams only (Constitution §11), so even self-modifying runs are exact
   experiments.

## Evidence

- EXP-PERF-2026-06: 7 merges, 1 catch-and-revert, zero quality incidents —
  the gate working as designed.
- WSL2-Timing-Drift: ~9% same-host drift makes timing-based correctness
  checking impossible here; bit-identity is the only robust instrument.
- The dense engine's golden trajectory history is now a permanent regression
  anchor (ADR-0002) — an asset that exists *only because* of this rule.

## Consequences

- Some legitimate optimizations (FMA contraction, float re-association) are
  forbidden on trajectory paths — a real, accepted performance cost on the
  float backend; largely voided on the integer backend.
- Parallel schedules must be deterministic (fixed work-unit decomposition per
  plan), constraining some dynamic load-balancing designs.
- Every new backend/operator inherits the burden of proof: replay first, then
  merge.

## Alternatives rejected

- **Statistical equivalence gating** (distributions match ⇒ accept): needs
  enormous sample sizes for small effects, is timing-contaminated on this
  host, and silently admits compounding subtle biases.
- **Determinism only in tests, "fast mode" in production**: two codepaths,
  the verified one not being the shipped one — worthless verification.
