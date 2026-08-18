# RC-009 — Does the capability passport match reality?

**Status: CONFIRMED (no discrepancy found).** A verification gap was closed;
the model was already correct.

## Problem

ADR-0001 makes the capability passport **load-bearing**: operators are selected
by capability, never by name, so a wrong `Constraints` field silently changes
every routing decision the Decision Engine makes.

All 18 registered operators declare `supports_sparse: true` **and**
`supports_dense: true` — a universal claim about state representation.

**The gap:** `tests/test_dense_byte_verification.rs` contains exactly two tests —
one verifying the DenseByte *backend primitives* against the f64 oracle over
thousands of checks, one timing. It exercises **zero operators**. So "every
operator runs correctly on DenseByte" was verified for none of them.

## Hypothesis

At least one operator violates its declared passport — either misbehaving on a
backend it claims to support, or over-declaring `needs_replicas`.

## Mathematical reasoning

None. This is a conformance audit, not a derivation.

## Experimental setup

`tests/test_operator_backend_passport.rs`. For every operator in the standard
registry, on **every backend its passport claims**:

1. it does not panic,
2. `audit()` reports zero ledger drift,
3. energies are **identical** to the reference oracle (the ADR-0004 bit-identity
   firewall, extended to DenseByte for the first time).

Plus: every operator declaring `needs_replicas: false` is run at **R = 1**.

**Vacuity guard.** Cross-backend agreement is worthless if the operators did
nothing — the trap that produced RC-007's confounded first pass and the retracted
Law 2's empty control. The suite records a post-warm-up baseline and **fails if
half the operators are inert**. A randomising warm-up runs first so
replica-coupled operators have genuine diversity to act on (from the all-zeros
init every replica is identical — RC-002 — which makes `replica_exchange` and the
cluster operators structurally inert).

## Dataset

A 12-variable hand-built instance: integral (so `SparseBitSlice` accepts it),
mixed-sign, non-zero offset, non-trivial linear terms, and frustrated enough to
exercise cluster and extremal operators rather than trivially converging.

## Measurements

- **18 operators × 3 backends: 0 failures.** No panic, zero drift, energies
  identical to the oracle throughout.
- `needs_replicas: false` accurate for every operator declaring it.
- **0 of 18 inert** — every operator changed the energy, so the agreement is
  real evidence rather than shared inaction.

## Counterexamples / refutations

None. The hypothesis (some operator violates its passport) was **refuted**.

## Final conclusion

The passport model matches reality. The universal `supports_dense` claim is now
verified rather than assumed.

## Practical consequences

Permanent suite `tests/test_operator_backend_passport.rs` (2 tests), running under
`cargo test --workspace` and therefore inside the CI `test` job. Any newly
registered operator is audited automatically — a growth-safe gate, since the
library is expected to expand.

## Open questions

- `needs_temperature` and `needs_integer` are **not** audited. The former is hard
  to falsify behaviourally (an operator may read a temperature and ignore it);
  the latter is exercised only indirectly by `SparseBitSlice` acceptance.
- The audit uses one small instance. Backend divergence that appears only at
  scale (e.g. above `DENSE_N_LIMIT = 4096`) would not be caught.

## Reproduction

```bash
cargo test --release --test test_operator_backend_passport -- --nocapture
```
Expected: 2 tests pass, and the line
`audited 18 operators on all declared backends; 0 left the energy unchanged`.
