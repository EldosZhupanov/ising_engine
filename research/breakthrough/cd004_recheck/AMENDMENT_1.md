---
id: cd004-recheck-amendment-1
kind: research-protocol-amendment
status: binding
authority_scope: CD004-R only
created: 2026-09-25
immutable: true
supersedes: PROTOCOL.md timing and witness-verification procedure only
---

# CD004-R prospective instrument correction

Independent read-only review of the code occurred before any holdout row was
generated. On the disclosed non-holdout `N=8, complete_SK, seed=1` case, a
single baseline-first timing was unstable. The review also found that the raw
row gate did not independently recompute returned spin witnesses.

For every holdout instance, run three paired repetitions. Alternate arm order
according to `(seed + repetition) % 2`: even means baseline first, odd means
candidate first. Record all six raw timings and use the median of three for
each arm in the specified ratio analysis. Before the holdout, warm both arms
once on the already disclosed `N=8, complete_SK, seed=1` calibration case.
No holdout-dependent exclusion or rerun is permitted.

Independently recompute every returned state with a direct energy evaluator
distinct from the legacy energy function used by the searches. Reject a row if
a state has wrong length or a sign outside `{-1,+1}`, if any repetition differs
in energy/state/counters from its paired repeats, or if direct energy differs
from the exhaustive optimum. The `solve()` internal assertion is insufficient
as a primary gate because it uses the legacy evaluator and can be disabled.

The original primary correctness and practical-speed thresholds remain. Runtime
is still only a small-instance screen, not a production performance claim.
