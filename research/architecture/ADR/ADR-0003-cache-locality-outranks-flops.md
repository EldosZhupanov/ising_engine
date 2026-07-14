---
id: ADR-0003
title: Cache locality outranks FLOPS in all layout and kernel decisions
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [Cache-First-Design, implements, CONSTITUTION#6]
  - [SparseBitSlice, motivated_by, Cache-First-Design]
  - [Reorder-Pass, implements, Cache-First-Design]
  - [Cache-First-Design, validated_by, EXP-PERF-2026-06]
  - [Cache-First-Design, anchored_by, HW-RYZEN-PROFILE]
---

# ADR-0003 — Why cache locality outranks FLOPS

## Context

The intuitive model of a fast solver counts arithmetic: flips per second,
exp() evaluations, multiply-adds. The measured reality of the target machine
(HW-RYZEN-PROFILE: Ryzen 7, AVX2, 64 B lines, L1 32 K / L2 512 K / L3 16 MB
shared) is that arithmetic is nearly free and *data movement is the budget*:

- A 256-bit integer add issues 3–4× per cycle; an L3 miss costs ~hundreds of
  cycles — one miss buys thousands of vector operations.
- The 2026-06 asm-driven optimization campaign (EXP-PERF-2026-06, cumulative
  ×1.37, every step A/B-verified bit-identical) drew its wins overwhelmingly
  from traffic elimination, not arithmetic: the JIT acceptance RNG removed a
  per-step f64 buffer (memory traffic), the j_tau==0 fast path skipped loads,
  the XOR-SUB identity removed an instruction *sequence* LLVM emitted because
  the ISA lacks i8 multiply. Meanwhile lane-halving — a pure "less arithmetic"
  change — measured ×0.98 and was reverted.
- The sparse hot loop (field scatter: `h_j ± 2·q_ij` over neighbors) has
  arithmetic intensity of ~1 add per load. It is bandwidth, period.

## Decision

In every layout, kernel, and scheduling decision, **bytes-moved is optimized
before operations-executed**:

1. State layouts are chosen so working sets are *resident by design* in a
   named cache level: threshold tables in L1, work-unit slices in L2, whole
   hot state in L3 (ensemble width R is derived from the 16 MB budget, not
   chosen independently).
2. One 64 B cache line is treated as the fundamental unit of computation —
   bit-slicing makes one line = 512 replicas of one spin, converting bandwidth
   into ensemble width.
3. Graph reordering (BFS/degeneracy) is a mandatory lowering pass: it converts
   random neighbor access into near-streaming access that hardware prefetch
   can serve.
4. Arithmetic is *added* to save traffic when profitable (recompute over
   reload), never the reverse.

## Evidence

- EXP-PERF-2026-06: the accepted/rejected pattern above — traffic cuts won,
  the arithmetic-only cut lost.
- Cache arithmetic for the sparse backend: G60 fully L3-resident at R=512
  (~7.9 MB) versus a byte layout that cannot fit — an 8× state-density
  difference from layout alone.
- HW-RYZEN-PROFILE (measured, not assumed): no AVX512, no i8 multiply, slow
  gathers — the FLOPS-first design space is partly *unavailable* on this
  machine regardless of preference.

## Consequences

- Performance reviews ask "what lines does this touch?" before "what does this
  compute?".
- Profiling priority: cache/bandwidth counters over instruction counts (on
  WSL2 without perf counters: A/B wall-clock with layout-varied experiments).
- Risk: over-fitting layouts to one cache geometry. Mitigation: sizes are
  parameters derived at startup from measured topology, never constants.

## Alternatives rejected

- **FLOPS-first (maximize vector arithmetic throughput)**: refuted by
  EXP-PERF-2026-06 on our own hot paths.
- **GPU-style oversubscription to hide latency**: 16 logical CPU threads
  cannot hide hundreds-of-cycles misses the way 10k GPU threads can; on CPUs
  the miss must be *prevented*, not hidden.
