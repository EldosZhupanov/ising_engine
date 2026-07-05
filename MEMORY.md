# MEMORY.md

## Long-Term Project Memory

### Core Goal

Build the fastest open-source CPU Ising/QUBO/HUBO solver written in Rust.

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

---

## Current Priorities

1. SIMD layout
2. FlatHuboModel
3. Branchless kernels
4. Vectorizable math
5. Benchmark improvements

---

## Permanent Decisions

- f64 energies
- 64 replicas
- Rust stable
- UltimateSolver is primary solver
- server_api.rs calls UltimateSolver only

---

## Never Forget

Correctness > Architecture > Performance > API > Readability

