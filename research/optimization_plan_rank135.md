# Implementation Plan: Rank 1/3/5 Architecture Fixes

Sequenced AFTER the running publication benchmark completes (its binary and
CPU must not be disturbed). Constraint hierarchy: correctness > golden
regression compatibility > performance. Nothing lands without the full gate
(tests + clippy + fmt + golden + measured A/B).

## Fix A (Rank 3) — acceptance fast path. Golden-SAFE by construction.

In `engine.rs` Phase 1 inner loop:
1. Skip `fast_exp` when `total_delta <= 0.0` (accept unconditionally —
   decision identical, exp value unused). Per-lane `select`, or a per-variable
   aggregate-mask branch when ALL lanes auto-accept/reject cheaply.
2. rng_buf is an INDEXED buffer (not a stream), so skipping loads for
   auto-accepted lanes cannot desynchronize anything.
Expected: large win on sparse instances where acceptance FLOPs dominate
(review Rank 3: ~1280 acceptance FLOPs vs ~320 physics FLOPs at deg 5).
Validation: bit-identical trajectories (golden byte-compare) + A/B on G-Set.

## Fix B (Rank 5) — zero-copy replica exchange (identity swap).

Replace byte-wise `spins.swap()` loops with a per-(t,p)-cell indirection:
`cell_of[logical_cell] -> physical chunk index`, swapped in O(1) per accepted
replica-lane... NOTE: lanes swap independently (per-r mask), so full-chunk
pointer swap only works when the whole lane-block swaps. Two options:
  B1. Keep per-lane granularity: swap 64-byte lane groups via masked
      SIMD loads/stores (still O(n) but line-utilization 64/64 instead of
      1/64, branchless via mask expansion). Trajectory-identical.
  B2. Restructure exchange to whole-cell swaps (changes semantics — REJECT).
Choose B1. Validation: golden byte-identical (the swap outcome is the same
bytes, only the copy mechanism changes) + perf counter A/B.

## Fix C (Rank 1) — budget-aware depth/width allocation. OPT-IN.

New optional field on `UltimateSolver` (e.g. `depth_bias: Option<...>` or
`auto_allocate: bool = false` via Default) — DEFAULT OFF so production
behavior and goldens are untouched. When enabled with a wall budget or a
total-sweep budget: choose num_temps (and effective lane usage) so per-chain
schedule length ≥ c·n sweeps before widening (literature: schedule length
scales with n; population width is the residual dimension).
The benchmark bridge (`solve_instance --auto-alloc`) opts in explicitly.
Validation: cannot break goldens (off by default); A/B TTS on G-Set
large + BQP dense to confirm no dense regression; report both configs.

## Explicitly deferred

- Rank 2 (RNG width reduction): changes accepted-move randomness →
  trajectory-breaking; needs its own golden regeneration decision.
- Rank 6 (MSC bit-packing): architectural rewrite; separate effort.

## Gate (all must pass before claiming anything)

cargo test (full) · clippy -D warnings · fmt · golden byte-compare ·
selftest.py · A/B benchmark: rerun the G-Set-large + bqp-dense subset with
the SAME harness (fresh raw file, same seeds/budgets) comparing
old-vs-new engine and vs OpenJij; report measured deltas only.
