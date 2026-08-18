---
id: ADR-0009
title: Addressable randomness for exact counterfactual execution
status: proposed
date: 2026-08-19
tier: architecture
edges:
  - [Addressable-Randomness, requires, ADR-0004]
  - [Exact-Counterfactual-Instrument, requires, Addressable-Randomness]
  - [Addressable-Randomness, conflicts_with, Runtime-Stream-RNG]
  - [ADR-0009, motivated_by, RC-014]
---

# ADR-0009 — Addressable randomness for exact counterfactual execution

## Context

RC-014 requires common random numbers that remain attached to the same logical
events when one scheduled operator is removed or replaced.  The current
`engine_v2::Runtime` creates one `ChaCha8Rng` from `Plan::seed` and passes the
same mutable stream to every `Operator::apply`.  Operators consume different,
and sometimes data-dependent, numbers of draws.  A schedule intervention
therefore changes the stream position seen by every downstream step.

Consequently, the current `TheoryEngine` comparison is deterministic but is not
an exact counterfactual: after the intervention it compares both a changed
operator schedule and changed downstream random events.  This is the same seed
variance that RC-007 uses as its null.

No existing seam above the Runtime can repair the correspondence.  The RNG is
private to `Runtime`; `BatchExecutor` accepts only complete schedules; and the
`Operator` contract receives `&mut ChaCha8Rng` without a logical event address.
Driving operators directly from the research layer would bypass the Runtime and
violate the read-only execution boundary.

## Proposed decision

Add an opt-in, versioned counterfactual execution mode whose exogenous random
events are addressed by stable plan coordinates:

```text
U = f(seed, run, phase, logical_slot, operator_instance,
      site, replica, draw_kind, draw_index)
```

The plan, rather than execution dynamics, assigns `logical_slot` and stable
operator-instance identities.  Removing a step retains an empty slot.  Replacing
a step does not renumber later slots.  Default execution remains byte-identical
to the current stream-RNG path.

The concrete interface is deliberately undecided by this proposal.  Before
acceptance, a prototype must compare at least these alternatives:

1. a Runtime-owned random-event provider passed through a new operator context;
2. per-slot ChaCha substreams derived by a stable, domain-separated seed hash;
3. counter-based words produced directly from the full event address.

The selected design must cover variable-draw operators, replica exchange,
resampling, clusters, repeated instances of one operator, and branches without
allowing consumption order to become an implicit address.

## Required evidence before acceptance

1. The default Runtime and golden regression remain bit-identical.
2. A null intervention is bit-identical in counterfactual mode.
3. Removing an inert step changes neither downstream random events nor results.
4. A deliberate one-field address shift is detected by a test.
5. Replacing one logical slot leaves every downstream event address unchanged.
6. Synthetic and real positive controls from RC-014 Gate A are recovered.
7. No operator allocates in its hot loop as a consequence of the interface.

## Consequences

If accepted, this changes a load-bearing Runtime/operator boundary and requires
updates to every operator implementation, the executor seam, replay provenance,
and the golden/cross-backend verification suites.  It must therefore be a
separate architectural change, not an incidental part of the RC-014 experiment.

Until acceptance and implementation, RC-014 Phase 1 and Gates A/B are blocked:
reporting `I_delete`, `I_cost-null`, or `I_replace` as causal would be invalid.
Phase 3's DSL reconstruction gate is the plan-authorized independent fallback.

## Alternatives rejected

- **Reuse the same seed for each complete schedule.** Deterministic, but a
  removal changes stream position downstream and therefore changes the null.
- **Reseed once per operator name.** Repeated instances collide, and replacement
  still cannot preserve site/replica/draw correspondence.
- **Run operators directly above Runtime.** Bypasses the canonical execution
  boundary and produces a second, scientifically incomparable execution path.
- **Treat seed variance statistically.** Estimates a noisy schedule A/B, not the
  exact counterfactual instrument RC-014 pre-registered.
