# CONTEXT.md — Ising Engine Project Context

## Project

High-performance Rust implementation of an Ising/QUBO/HUBO optimization engine.

Primary goal:
Build the fastest and most accurate open-source CPU Ising Solver.

Language:
- Rust 1.95+

Target platforms:
- Linux
- Windows
- WSL

---

## Architecture

The project consists of two completely independent solver families.

### Scalar Family

Files:

- solver/parallel_tempering.rs
- solver/adaptive.rs
- solver/cluster.rs
- solver/tabu.rs

Characteristics:

- QuboModel
- CsrMatrix
- Sequential / Rayon
- Legacy architecture
- Stable API

This family must remain untouched unless explicitly requested.

---

### MSC Family

Files:

- solver/types.rs
- solver/engine.rs
- solver/ultimate.rs

Characteristics:

- QuantumField
- HuboModel
- Multi-Spin Coding
- Population Annealing
- Parallel Tempering
- SIMD-first architecture

Only this family is optimized.

---

## Data Models

QuboModel

- legacy
- pairwise interactions

HuboModel

- Edge2
- Edge3
- Edge4

FlatHuboModel

- CSR-like flattened representation
- Read-only
- Generated once before solving

QuantumField

Owns every spin.

No duplicated spin storage is allowed.

---

## Memory Layout

Current optimization target:

Variable-major layout

[variable]
    [slice]
        [temperature]
            [population]
                [replica]

Replica count:

64 replicas

Future SIMD layout:

Vec<i8>

One contiguous cache line per variable.

---

## Solver Pipeline

Compiler

↓

HuboModel

↓

FlatHuboModel

↓

QuantumField

↓

UltimateSolver

↓

Best Solution

---

## Optimization Priorities

1. Correctness

2. SIMD Vectorization

3. Memory Locality

4. Parallel Tempering

5. Population Annealing

6. Branchless Algorithms

7. Reduced Allocations

---

## Hot Path

Performance critical files:

solver/engine.rs

solver/types.rs

Rules:

- no allocations
- no HashMap
- no String
- no format!
- no clone()
- no variable bit shifts
- no hidden heap usage

---

## Public Entry Point

HTTP

↓

server_api.rs

↓

UltimateSolver::solve()

This API should remain stable.

---

## Current Goals

Phase 0

Fix build environment.

Phase 1

SIMD byte-per-replica layout.

Phase 2

FlatHuboModel.

Phase 3

Branchless acceptance.

Phase 4

Vectorizable fast_exp.

Phase 5

Bulk RNG generation.

Phase 6

Compiler optimization flags.

Phase 7

Cleanup.

---

## Success Criteria

The project is considered successful when:

- all tests pass
- all binaries build
- SIMD loops are vectorized
- benchmark improves
- numerical correctness is preserved
- architecture remains clean

