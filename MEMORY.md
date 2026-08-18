# MEMORY.md — SUPERSEDED

> **This document is retained for history only. Do not plan from it.**
>
> - **Direction** → `research/ISING_ENGINE_CONSTITUTION.md` (single source of truth)
> - **Status / priorities** → `ROADMAP.md`
> - **What is in flight** → `memory/CURRENT_TASK.md`
> - **Index of everything** → `memory/INDEX.md`
>
> It sits outside the governance order defined in `CLAUDE.md`
> (Constitution → SOUL → ADR → ROADMAP → CLAUDE) and contradicts it in three
> places, corrected inline below.

## Long-Term Project Memory

### Core Goal

Build the fastest open-source CPU Ising/QUBO/HUBO solver written in Rust.

> **Still true, but incomplete.** The project is now two things: that production
> engine (Stage 0, frozen, 100%) *and* an autonomous research platform growing
> toward a Research Foundation Model. See `ROADMAP.md`.

---

## Current Architecture

Scalar family:
- parallel_tempering.rs
- adaptive.rs
- cluster.rs
- tabu.rs

MSC family:
- types.rs
- engine.rs
- ultimate.rs

These two families must remain independent.

> **Correction.** The scalar/legacy family is **frozen**, not merely
> independent — never merged, never rewritten (`ROADMAP.md` Stage 0). This
> listing also omits `src/engine_v2` (~20.6k lines) and its `ai_scientist/`
> modules, which is where all current work happens.

---

## Current Priorities

1. SIMD layout
2. FlatHuboModel
3. Branchless kernels
4. Vectorizable math
5. Benchmark improvements

> **SUPERSEDED — this priority list is not current.** Use `ROADMAP.md`
> "Next 10" (item 1: growth campaign toward 500k experiments). SIMD is item 7
> of 10 there, not #1.
>
> The claim also misdescribes the code. `src/core/simd_utils.rs` is **declared
> in no `mod.rs`** (`src/core/mod.rs` lists only `csr_matrix`, `hubo`, `anls`) —
> it is orphaned and **never compiled**, and its own header calls it "a
> theoretical implementation". There are no explicit SIMD intrinsics anywhere;
> only `-Ctarget-cpu=native +avx2,+fma` auto-vectorization. What *did* land is
> adaptive dispatch in `SparseBitSlice::apply_flips` (1.6–4.6× at high flip
> count, bit-identical); explicit `#[target_feature]` AVX2/512 remains undone.

---

## Permanent Decisions

- f64 energies
- 64 replicas
- Rust stable
- UltimateSolver is primary solver
- server_api.rs calls UltimateSolver only

> **Correction.** "64 replicas" is permanent only for the **production** MSC
> path (64 replicas per `u64` bit-slice). `engine_v2` is replica-count agnostic;
> all recent research ran at 32. The other four remain accurate.

---

## Never Forget

Correctness > Architecture > Performance > API > Readability

