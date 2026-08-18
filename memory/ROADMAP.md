# memory/ROADMAP.md — pointer

**Authoritative source: `../ROADMAP.md`.** It carries the full stage table with
completion percentages, per-stage detail, the confirmed/refuted ledger, and the
technical-debt list. Do not restate it here — it will drift.

Direction (as opposed to status) is `../research/ISING_ENGINE_CONSTITUTION.md`.

## Where the project is

Two things at once: a **production optimization engine** (Stage 0, 100%, frozen)
and an **autonomous research platform** growing toward a Research Foundation
Model (Stage 11, ~8–10%). Stages 1–10 sit between 50% and 95%.

The frontier is Stage 11, and `../ROADMAP.md` is explicit that it is gated on
**data volume, not architecture**: 18,570 experiments recorded against
milestones of 500k → 1M → 5M → 20M. Building a GNN/Transformer before the data
exists would over-fit; the path is to grow the dataset first.

## Current priorities — "Next 10"

Item 1 is the active task (see [CURRENT_TASK.md](CURRENT_TASK.md)).

1. **Growth campaign** — long curiosity-driven `--service` toward 500k
2. Distributed `BatchExecutor` behind the same trait
3. Model Registry consumption (warm-start / A-B model versions)
4. Monitor → live gate with alerting + auto-pause
5. Reconstruct-and-serve models (`--serve-model`)
6. SAT/TSP campaigns; k>2 gadgets; job-shop scheduling
7. Explicit `#[target_feature]` AVX2/512 for the dense inner SAXPY
8. World-filtered ideation default-on once rank-correlation is validated
9. Theory confidence at scale (`--investigate` across the full G-Set)
10. Foundation-model readiness gate when the dataset crosses 500k

## On item 7 (SIMD) — read this before believing any older document

`../MEMORY.md` and `../CONTEXT.md` are **superseded** and claim SIMD is priority
#1 with a "SIMD-first architecture". That is not the current plan and does not
describe the code. What actually exists:

- `SparseBitSlice::apply_flips` adaptively dispatches the field update
  (scattered for sparse flips, contiguous/vectorizable for dense; crossover
  ≈ `r/3`, measured). Bit-identical. **1.6–4.6× faster at high flip count.**
- Explicit `#[target_feature]` AVX2/512 for the dense inner SAXPY is **not done**.
- `src/core/simd_utils.rs` is orphaned and never compiled — see
  [OPEN_PROBLEMS.md](OPEN_PROBLEMS.md).
