---
id: ADR-0001
title: The engine is built from Operators, not Algorithms
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [Operator-API, implements, CONSTITUTION#3]
  - [Operator-API, implements, CONSTITUTION#8]
  - [Operator-API, refines, BLUEPRINT#L3]
  - [Scheduler, uses, Operator-API]
  - [Operator-API, motivated_by, MQLib-Fragmentation]
  - [Operator-API, anchored_by, BENCH-GSET-2026-06]
---

# ADR-0001 — Why Operators instead of Algorithms

## Context

The optimization literature ships *algorithms*: SA, PT, ICM, tabu, PA — each a
closed loop owning its own state, moves, schedule, and bookkeeping. Our own
repository reproduced this pattern (UltimateSolver plus four independent legacy
solvers), and so does the field's best zoo: MQLib contains 37 heuristics with
**zero shared substrate** — every one re-implements spin storage, delta-energy
maintenance, and RNG. Improvements to one benefit none of the others.

The decisive internal evidence: the full 2026-06 benchmark campaign
(BENCH-GSET-2026-06, 15,280 runs) showed the engine winning dense QUBO and
losing all large sparse G-Set instances. The root cause was not a bad
algorithm — it was an algorithm-shaped architecture: ensemble geometry (64×10)
was a *constant inside a monolith*, unreachable by any per-instance decision.

## Decision

Every optimization dynamic is an **operator**: a typed transformation
`apply(state, ctx, budget) → report` over a shared `SpinState`, with a declared
cost model. Operators know nothing about each other; all composition happens in
the scheduler through state and reports. Named methods from the literature
("simulated annealing", "PT-ICM") are *schedules* — versioned plans over the
operator set — not code paths.

## Evidence

- BENCH-GSET-2026-06: the fixed-geometry loss demonstrates that
  algorithm-embedded constants cannot adapt; only a plan-level decision can.
- MQLib (Dunning-Gupta-Silberholz 2018): 37× duplicated substrate is the
  measured cost of the algorithm-shaped alternative at ecosystem scale.
- LLVM precedent: passes over shared IR outlived every monolithic compiler
  they competed with; the composition economics compound the same way here —
  one optimized kernel (e.g. the deployed XOR-SUB sign trick) benefits every
  operator that touches spins, instead of one solver.

## Consequences

- New capability enters as an operator, a pass, or a schedule — never a new
  solver struct (Constitution §4.1).
- Every operator inherits the substrate's verification (golden replay, A/B)
  and its performance work for free.
- Cost: designing the `SpinState`/`Operator` contracts well is hard, and a bad
  contract taxes everyone. Contract changes go through ADRs.
- Legacy solvers are preserved as-is (regression anchors), not ported en masse.

## Alternatives rejected

- **Best-of-breed monolith** (keep improving UltimateSolver): every new
  dynamic multiplies internal coupling; the 64×10 failure mode recurs in new
  forms; nothing transfers.
- **Plugin algorithms sharing only the model API** (dimod/Ocean shape): sharing
  the *problem* without sharing the *execution substrate* is exactly the
  MQLib outcome — proven fragmentation.
