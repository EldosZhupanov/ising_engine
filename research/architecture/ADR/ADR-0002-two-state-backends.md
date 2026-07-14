---
id: ADR-0002
title: Two state backends (DenseByte, SparseBitSlice) behind one interface
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [DenseByte, implements, SpinState-API]
  - [SparseBitSlice, implements, SpinState-API]
  - [SpinState-API, implements, CONSTITUTION#9]
  - [SparseBitSlice, requires, Integer-Fields]
  - [Integer-Fields, requires, Quantization-Pass]
  - [Quantization-Pass, requires, ADR-0005]
  - [SparseBitSlice, motivated_by, BENCH-GSET-2026-06]
  - [SparseBitSlice, motivated_by, ADR-0003]
  - [Backend-Selection-Pass, uses, SpinState-API]
---

# ADR-0002 — Why two state backends exist

## Context

One state layout cannot serve both regimes, for a physical reason:

- **Dense instances** (ORLIB/Biq Mac, density ≳ 5%): a flip changes the field
  of *every* other variable. Bandwidth goes to streaming the coupling row; the
  spin layout is secondary. The existing byte-per-replica engine (1 spin = 1
  i8 × 64 interleaved replicas, f64 couplings) is measured-strong here — it
  wins this regime in BENCH-GSET-2026-06.
- **Sparse instances** (G-Set, deg ≪ n): a flip touches only deg neighbors.
  The cost is dominated by per-spin state and field traffic — and here the
  byte layout wastes 8× storage and the f64 machinery wastes bandwidth on
  arithmetic precision the (integer-coefficient) instances don't need. This is
  the regime the engine lost, across the board, to vanilla single-thread SA.

One layout must lose one regime. Migrating dense to a bit-sliced layout would
also forfeit our strongest verified asset: the bit-identical trajectory history
of the dense engine.

## Decision

Two concrete backends behind the single `SpinState` interface (Constitution
§9):

- **`DenseByte`** — the existing engine, preserved bit-for-bit as the dense
  workhorse and permanent regression anchor.
- **`SparseBitSlice`** — 1 bit/spin/replica (one 64 B cache line = 512
  replicas of one spin), integer couplings via the certified quantization
  pass, i16/i32 field planes, chromatic synchronous updates.

A **backend-selection lowering pass** chooses per instance (initial rule:
density/integrality; refined by measurement). Operators are written against
the interface only.

## Evidence

- BENCH-GSET-2026-06: dense wins + total large-sparse loss — the two-regime
  split is measured on 499 instances, not hypothesized.
- Adversarial review Rank 1: G60 received ~60 sweeps/chain vs ~15,756 for
  OpenJij — sweep starvation is a *layout-and-geometry* problem.
- Cache arithmetic (ADR-0003): G60 (n=7000) at 512 bit-sliced replicas =
  ~7.9 MB total hot state — fully L3-resident on the target machine; the byte
  layout cannot achieve this.
- Prior art: multi-spin coding (1980s) and Fujitsu Digital Annealer prove the
  bit-parallel substrate at scale; Isakov et al. 2015 `an_ms` codes are the
  fastest known CPU SA. Their limitation (shared randomness / same-J
  restrictions) is fixed by our per-replica JIT acceptance RNG.

## Consequences

- Every operator must be layout-agnostic; layout-specific tricks live in
  kernels below the interface.
- Two backends to maintain and verify. Accepted: the alternative is losing a
  regime permanently.
- The interface is the sacred boundary — a future GPU/FPGA backend must slot
  in with zero operator changes (Constitution §9 test).

## Alternatives rejected

- **Single bit-sliced layout for everything**: forfeits measured dense wins
  and the golden trajectory history; dense flips invalidate all fields anyway,
  so bit-slicing buys little there.
- **Single byte layout + more threads for sparse**: cannot close an ~8×
  state-bandwidth and ~260× sweep-depth gap with 16 logical cores.
- **CSR-float sparse engine (OpenJij shape)**: matches the competitor instead
  of beating it; leaves the cache-line-as-ensemble advantage (ADR-0003) on the
  table.
