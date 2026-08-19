# RC-017 entropy pilot — frozen diagnostic protocol

**Status:** frozen before diagnostic code and before any pilot output.

This protocol diagnoses the RC-017 abort. It is not an operator comparison and
must never compute or store a Metropolis-vs-Gibbs horizon effect.

## Frozen matrix

- instance: G15, the first corpus member reached after the printed G14 row;
- seeds: `4001–4008` only;
- initial state: all zeros, 32 replicas;
- temperatures: geometric `4.0 -> 0.1`;
- plan: prefix-only `metropolis_sweep@16`;
- repetitions: 16 independently constructed Runtime executions per seed.

For every execution record only: seed, repetition, state digest, exact ledger
energy-bit-vector digest, sorted `(energy_bits,count)` histogram digest, and
`StepEvent[0].energy_entropy.to_bits()`.

The diagnostic mode must reject seeds `5001–5008` and `6001–6008`. It writes
only below `experiments/rc017_entropy_pilot/`, never below the RC-017 science
artifact names.

## Verdicts

- `REPRODUCED`: for at least one seed, two executions have identical state,
  ledger and histogram digests but different entropy bits.
- `NOT REPRODUCED`: no such pair exists. Stop; do not widen instances, seeds or
  repetitions without a committed amendment.

After canonical v1 is implemented, rerun this exact matrix. Every seed must
have one entropy bit pattern, with state/ledger/histogram digests unchanged.
Any mismatch rejects the repair.

The burned held-in seeds `5001–5008` are never reusable. The original held-out
seeds `6001–6008` remain uninspected but are reserved from the successor to keep
the new cycle wholly independent.

## Recorded pre-change result

The frozen prefix-only matrix returned `NOT REPRODUCED`: every seed had exactly
one entropy bit pattern across 16 independently constructed Runtime runs.

- artifact: `experiments/rc017_entropy_pilot/legacy_v0.tsv`
- rows: 128 data rows plus header
- SHA-256: `731ce544a9f2e081e093bde0445a1708678760581d3eab00925621ab7e720a9c`

No Runtime repair is licensed by this result.

### Amendment 1 — preserve the failing execution context

Written after the prefix-only null result and before Amendment-1 diagnostic
code or output. The original failure compared the event produced inside two
full plans whose operator maps differ after the common prefix. The prefix-only
harness removed that context, so it did not test the observed failure exactly.

One final diagnostic is allowed without widening the evidence search:

- same G15, same pilot seeds `4001–4008`, same all-zero/32-replica/ladder;
- execute the exact full M and G plans from RC-017 once per seed;
- print the `to_bits()` of all 12 prefix coordinates and identify every unequal
  coordinate; record state and ledger digests after an independently executed
  prefix as a control;
- do not compute, print or store final horizon energies or `delta`;
- verdict `REPRODUCED` iff at least one pilot seed has unequal S1 bits.

If this exact-context diagnostic is also `NOT REPRODUCED`, stop RC-017 diagnosis:
the original failure is not reproducible on the frozen pilot block and no
Runtime change is permitted.

## Amendment 1 result

`NOT REPRODUCED`. Every one of the eight pilot seeds had zero mismatched S1
coordinates between the exact full M/G execution contexts.

- artifact: `experiments/rc017_entropy_pilot/exact_context_v0.tsv`
- rows: 8 data rows plus header
- SHA-256: `40d43ce1da6d1e1a602cf0caeeb72b92ca55ab0291805322859d7eb093f35f3e`

The entropy hypothesis is therefore not established by the frozen pilot
evidence. Per the kill criterion, ADR-0010 is not accepted, Runtime is not
changed, and RC-017 diagnosis stops here.
