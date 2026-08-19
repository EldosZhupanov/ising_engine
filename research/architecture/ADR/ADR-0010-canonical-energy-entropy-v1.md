---
id: ADR-0010
title: Canonical energy-entropy v1 restores adaptive replay
status: proposed
date: 2026-08-19
tier: architecture
edges:
  - [Canonical-Energy-Entropy-v1, implements, ADR-0004]
  - [Canonical-Energy-Entropy-v1, implements, CONSTITUTION#11]
  - [Canonical-Energy-Entropy-v1, motivated_by, RC017-Abort]
  - [RC017-Successor, requires, Canonical-Energy-Entropy-v1]
  - [Runtime-Quality, realized_by, Canonical-Energy-Entropy-v1]
---

# ADR-0010 — Canonical energy-entropy v1

## Context

The first RC-017 held-in attempt exposed that `Runtime::quality` groups exact
energy bits in a randomized `HashMap` and folds `hist.values()` into an `f64`.
Floating addition is order-dependent, so independently executed identical
prefixes can emit entropy values differing by one or more ULP. RC-017 aborted
before writing an artifact; its record is `research/RC017_ABORT_RECORD.md`.

This is not merely a diagnostic discrepancy. Entropy participates in Runtime
collapse adaptation and is passed to `RunController`; it can therefore change
Skip/Continue/Switch/Stop decisions. The existing implementation contradicts
ADR-0004 and Constitution §11. Canonicalising the reduction changes legacy
event bits and may change opt-in adaptive trajectories, so it is an explicit
sensor-contract migration rather than a silent refactor.

## Decision

Define `canonical-energy-entropy/v1`:

1. Empty input returns positive `0.0`.
2. A histogram bin is identified by exact `f64::to_bits()`. Thus `+0.0` and
   `-0.0` remain distinct, as do distinct NaN payloads. Runtime energy ledgers
   are expected to be finite; non-finite semantics are retained only to make
   the helper total and testable.
3. Energy bits are sorted in ascending raw-`u64` order and run-length counted.
4. For each distinct key, in that order, compute
   `p = count as f64 / n as f64`, then `-(p * p.log2())`, and add it to one
   left-to-right accumulator. No parallel reduction, reassociation, FMA, or
   multiplication by the number of equal-count bins is permitted.
5. Runtime uses one shared implementation and publishes the constant
   `ENERGY_ENTROPY_ALGORITHM = "canonical-energy-entropy/v1"` for provenance.
6. Legacy unordered values are named `legacy-unordered/v0`; old artifacts and
   trained models must not be relabelled as v1 or claimed bit-comparable.
7. Any future change to bin identity, key order, term formula, `log2`
   assumption, or fold order requires a new version and ADR.

The initial implementation may allocate one sorted scratch vector per quality
sample because the legacy implementation already allocates a histogram there.
A reusable Runtime buffer is allowed only as a separately measured follow-up:
it expands Runtime ownership and is not required to restore correctness.

## Evidence and acceptance gates

Before implementation, the frozen pilot protocol in
`research/RC017_ENTROPY_PILOT.md` must reproduce at least one case with equal
state, ledger and histogram digests but unequal legacy entropy bits. After the
change the exact same matrix must produce one entropy bit pattern per seed.

Required regressions:

- exact golden bits for a deliberately unequal histogram;
- invariance under reverse, rotation and seeded permutations;
- explicit exact-bit bin semantics;
- complete independent-Runtime event-stream replay;
- collapse-threshold boundary replay;
- adaptation-disabled state, energy and operator compatibility;
- RC-017 full 12-coordinate prefix identity and burned-seed refusal.

Full fmt, check, release tests, strict clippy and all-bin build gates apply.
Because this path can affect adaptive trajectories, Runtime throughput is
measured before/after. A regression greater than 1% rejects this implementation
or requires a separately justified design.

### Evidence outcome

Both frozen pre-change diagnostics returned `NOT REPRODUCED`:

- prefix-only matrix:
  `731ce544a9f2e081e093bde0445a1708678760581d3eab00925621ab7e720a9c`;
- exact full-plan context matrix:
  `40d43ce1da6d1e1a602cf0caeeb72b92ca55ab0291805322859d7eb093f35f3e`.

All eight exact-context pilot pairs matched on all 12 S1 coordinates. The
acceptance gate was not met. This ADR remains proposed and unimplemented; it
does not license a Runtime change.

Independent review found that the first exact-context harness allowed full
plans to finish. Amendment 2 repeated the matrix with a controller stopping
after the prefix event; the valid artifact has SHA-256
`40d43ce1da6d1e1a602cf0caeeb72b92ca55ab0291805322859d7eb093f35f3e`
and again found zero mismatches. The acceptance decision is unchanged.

## Consequences

- New entropy event bits become stable for a fixed build/target and identical
  input. No cross-platform `libm` identity is claimed.
- Opt-in adaptive/controller trajectories may differ from legacy v0; this is
  the correction of a non-replayable sensor, not a performance refactor.
- RNG draw order, operators, backend state and mean-energy reduction are not
  changed.
- RC-017 cannot resume until this ADR is accepted, its evidence is recorded,
  and a successor is preregistered with wholly fresh science seeds.

## Alternatives rejected

- Keep `HashMap` and compare with tolerance: violates bit-identical replay.
- Deterministic hasher: bucket iteration is not a durable numerical contract.
- Sort only multiplicities or multiply repeated entropy terms: changes the
  canonical floating operation sequence.
- `BTreeMap` as the specification: deterministic but freezes a container rather
  than the numerical algorithm and adds node allocation/pointer traversal.
- Rational entropy or lookup tables: changes the numerical definition and is
  unnecessary for this repair.
