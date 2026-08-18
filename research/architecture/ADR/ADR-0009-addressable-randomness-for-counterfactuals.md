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

Until acceptance and implementation, the **general** counterfactual instrument is
blocked: for an arbitrary intervention, reporting `I_delete`, `I_cost-null`, or
`I_replace` as causal would be invalid. Phase 3's DSL reconstruction gate is the
plan-authorized independent fallback.

### Amendment, 2026-08-19 — the blocking claim is narrowed by measurement

*Added after this ADR was authored, sourced to `research/RC014_PHASE0_AUDIT.md`
§0.1. The original sentence blocked RC-014 Phase 1 unconditionally; that is
correct in general and wrong for one specific case, which the audit found.*

A substitution is an **exact** counterfactual under the current stream RNG when
**both** conditions hold:

1. **The substituted pair is draw-identical** — same number of draws, same draw
   width. Verified for `metropolis_sweep` ↔ `gibbs_color_sweep`: both call
   `ensure_order`, which pushes every site exactly once in colour order, so
   `|order| = n`; both then draw exactly one `f64` per `(sweep, site, replica)`
   unconditionally — `metropolis_sweep.rs:119` keeps its count data-independent
   with an explicit discarded draw, `gibbs_color_sweep.rs:152` is unconditional
   by construction. Total `sweeps·n·r` `f64` draws in both, and the traversal
   order matches. Under `rand 0.8.5` an `f64` is two 32-bit words in both cases.
   The generator state after the step is therefore identical.
2. **Every downstream operator's draw count is state-independent.** Condition 1
   preserves the stream *position*, but the substitution changes the *state*, so
   a later data-dependent operator (cluster, population resampling, extremal,
   move synthesis) would consume a different number of draws and divergence
   resumes. In `PREREG_RC014.md` the schedule is `[X, greedy_descent]` and
   `greedy_descent` takes `_rng` — it draws nothing — so condition 2 holds
   trivially.

`random_flip_sweep` is excluded despite an equal draw *count*: it draws `bool`
(one word), not `f64` (two words).

**Consequence.** RC-014 Phase 1 and Gates A/B proceed **restricted to that one
pair under that one schedule**, and the pre-registration is scoped accordingly
(`PREREG_RC014.md` §3, §13). This ADR remains required for anything wider: a
second substitution pair, a data-dependent downstream operator, deletion rather
than substitution, or any claim about longer schedules.

Both conditions are machine-checked rather than assumed — the harness records a
per-step draw counter and generator fingerprint, and Gate A blocks on a
null-substitution being bit-identical and on a deliberately injected one-draw
offset being *detected* (`PREREG_RC014.md` §7).

## Alternatives rejected

- **Reuse the same seed for each complete schedule.** Deterministic, but a
  removal changes stream position downstream and therefore changes the null.
- **Reseed once per operator name.** Repeated instances collide, and replacement
  still cannot preserve site/replica/draw correspondence.
- **Run operators directly above Runtime.** Bypasses the canonical execution
  boundary and produces a second, scientifically incomparable execution path.
- **Treat seed variance statistically.** Estimates a noisy schedule A/B, not the
  exact counterfactual instrument RC-014 pre-registered.
