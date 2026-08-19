# RC-017 first held-in attempt — instrument abort record

**Status:** aborted; no RC-017 scientific claim and no held-out access.

## What happened

After commit `4d9a1b1` passed all pre-flight controls and independent review,
the preregistered held-in command was started once with seeds `5001–5008`.
It printed summaries through G14 and then panicked while beginning the next
corpus member at the per-pair guard requiring all 12 common-prefix `S1`
coordinates to be bit-identical.

The block writer runs only after all 30 instances return. Consequently the
panic created no `experiments/rc017` artifact. The already printed held-in
summaries nevertheless mean that `5001–5008` are no longer pristine and must
not be rerun or reused as a replacement held-in block.

The held-out seeds `6001–6008` were not run or inspected.

## Root cause

The high-confidence root cause is `S1[5] = energy_entropy`.
`Runtime::quality` builds an energy histogram in a randomized `HashMap` and
then evaluates the floating-point sum over `hist.values()`. Two independent
Runtime executions can therefore visit the same terms in different orders and
produce entropy values differing by one or more ULP even when the prefix state,
ledger and histogram are identical.

This invalidates the bit-level replay assumption for the deployed sensor. The
other 11 coordinates are derived from the identical prefix state/event or fixed
constants. The exact observed coordinate and ULP distance were not printed by
the original assertion; they may be confirmed only with pilot seeds.

## Binding disposition

1. RC-017 is paused as an invalid-instrument attempt. It has no scientific
   verdict.
2. Do not rerun seeds `5001–5008` and do not access `6001–6008`.
3. Any repair to Runtime entropy accumulation is trajectory-contract sensitive:
   entropy is available to adaptation logic, so even a one-ULP change could
   cross a threshold. It requires ADR-level review and cannot be introduced as
   a research-only patch under RC-017's read-only-core constraint.
4. A successor may proceed only after: pilot-only reproduction; a frozen
   deterministic entropy algorithm and replay tests; an explicit preregistered
   successor/amendment with fresh untouched held-in seeds; and independent
   review before any new science run.

