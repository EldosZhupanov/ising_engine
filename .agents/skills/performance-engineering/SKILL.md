---
name: performance-engineering
description: Rules for profiling, hardware measurement, SIMD vectorization, and distinguishing algorithmic advances from microarchitectural tuning.
---

# Performance Engineering Protocol

This skill enforces engineering discipline and measurement rigor on all performance claims within Ising Engine.

## Core Principles

1. **Profile Before Optimizing**:
   - Never optimize based on intuition. Always obtain a baseline profile using `perf`, `criterion`, or hardware counters.
   - Measure:
     - CPU utilization across cores;
     - Cache miss rates (L1d, L2, LLC);
     - Memory bandwidth and allocation rate in hot loops;
     - Branch misprediction rate;
     - Instruction retirement (IPC).

2. **Algorithmic Improvement vs Implementation Optimization**:
   - **Algorithmic Improvement**: Reduces search tree size, improves energy landscape mobility, decreases flip count to ground state, or reduces asymptotic complexity. Must be evaluated in hardware-independent units:
     - Number of spin flips / sweeps to reach target;
     - Number of visited nodes;
     - Ground-state hit probability per sweep.
   - **Implementation Optimization**: Increases spin-flips per second through SIMD, cache blocking, register reuse, or compiler intrinsics. Must be reported as throughput (e.g. mega-flips per second, ns per flip).
   - *It is strictly forbidden to label a compiler/SIMD throughput improvement as an algorithmic breakthrough.*

3. **Hot-Loop Invariants**:
   Inside `step()`, `calculate_delta_e`, or any inner sweep loop:
   - Zero heap allocations (`Vec::new`, `Box`, `String`, captured closures);
   - Zero `HashMap` or dynamic pointer lookups;
   - Sequential, cache-aligned memory access;
   - All `unsafe` blocks must have explicit `// SAFETY:` proofs documenting pointer validity and bounds invariants.

4. **Vectorization Evidence**:
   For any claim of vectorization or SIMD efficiency:
   ```bash
   RUSTFLAGS='-C target-cpu=native -C llvm-args=-pass-remarks=loop-vectorize' \
     cargo build --release --lib 2>&1 | rg 'vectorized loop'
   objdump -d target/release/libising_engine.rlib | rg -c 'ymm|zmm'
   cargo bench --bench benchmark
   ```
