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

