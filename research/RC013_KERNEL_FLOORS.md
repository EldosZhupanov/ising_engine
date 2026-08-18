# RC-013 — Which hardware floor does the sweep kernel sit on?

**Status: P1 REFUTED, P2 CONFIRMED.** ADR-0003 ("cache locality outranks FLOPS")
converted from an assertion into a measurement, with a within-size control.

## Motivation

Two open decisions hinge on where the kernel's cost actually comes from:
ROADMAP item 7's remaining `#[target_feature]` AVX2 SAXPY, and
delete-or-implement for the orphaned `src/core/simd_utils.rs`. Neither can be
decided from IPC intuition; both need to know which floor binds.

## Method

Three measured floors on this host (AMD Ryzen 7 170; L1d 32K/core, L2 512K/core,
L3 16M shared; WSL2):

| floor | value |
|---|---|
| STREAM triad | **16.2 GB/s** |
| random 8 B gather (independent, MLP allowed) | **5.13 ns/access** |
| acceptance compute (ChaCha8 draw + branch + exp, cache-resident) | **3.61 ns/(site,rep)** |

Kernel: `MetropolisSweep` on `SparseBitSlice` (ledger `fields[site*r+rep]`,
8·n·r bytes, r=32). Grid: topology {ring i±1,i±2 (prefetch-friendly), random
~4-regular (prefetch-hostile)} × n {2k…524k: ledger 0.5→134 MB, L2→L3→DRAM} ×
flat T {0.05, 8.0}. Two φ points per cell give the RC-005 decomposition
`ns = scan + flip·φ`. **The ring-vs-random contrast at equal n, degree, and
dynamics isolates locality with FLOPs identical by construction.**

## Results

```
topo       n     ledger   phi_lo  phi_hi   ns_lo   ns_hi   scan    flip
ring     2048     0.5MB    0.272   0.930   10.42   14.39   8.77    6.04
ring   524288   134.2MB    0.316   0.930   11.16   14.74   9.33    5.82
random   2048     0.5MB    0.102   0.943    9.21   14.54   8.56    6.34
random  32768     8.4MB    0.125   0.943   11.06   17.95  10.00    8.43
random 131072    33.6MB    0.146   0.943   13.86   24.76  11.87   13.67
random 524288   134.2MB    0.224   0.944   17.99   27.67  14.98   13.45
```

**P2 — CONFIRMED.** Random-graph flip cost 6.34 → 13.45 ns (**2.1×**, meeting
the pre-registered ≥2× bar) across L2→DRAM; ring flip cost flat (6.04 → 5.82).
At 134 MB, random costs **1.9× ring** end-to-end at both temperatures. That
factor is pure locality: same n, same degree, same dynamics, same arithmetic.

**P1 — REFUTED, both criteria.** The acceptance floor explains only **~40%** of
the best-case scan intercept (3.61 of 8.6–8.8 ns; bar was ≥50%), and the random
topology's intercept rises **+75%** down the ladder (bar was <+50%). The scan is
not acceptance-math-bound: ~5 ns/(site,rep) of ledger-read/mask/dispatch
overhead dominates even fully cache-resident.

**Limitation on the decomposition.** φ_lo drifts with n (0.102 → 0.224 for
random, because larger sizes get fewer sweeps and are less converged at
T=0.05), so the intercept extrapolates outside its measured range at DRAM sizes.
The raw ns columns are model-free and are the load-bearing numbers; the
scan/flip split is indicative only where φ_lo is small.

## Consequences

1. **ADR-0003, quantified:** locality is worth ~2× at DRAM scale and ~0 at cache
   scale, at identical FLOPs. The ADR's decision rule survives measurement.
2. **Benchmark-scale caveat (new, and it generalises the G-Set findings):** at
   G-Set scale (n ≤ 2000, r=32) the entire ledger is **0.5 MB — cache-resident**.
   Every performance number derived on G-Set (DenseByte's 4.8×, RC-005's
   coefficients, all 18,570 recorded runs) lives in the cache regime and may not
   transfer to the n ≥ 10⁵ scaling ambition (ROADMAP Phase 4). Third entry in
   the benchmark-validity series after the two G-Set degeneracies.
3. **SIMD decision input (RC-005/RC-007-style open item, not applied):** neither
   dominant cost is the f64 SAXPY that `simd_utils.rs` and ROADMAP item 7 target
   — in-cache the kernel is bookkeeping+RNG-bound (and the floor math is only
   40% of that), at scale it is gather-bound where vector width is irrelevant.
   Evidence leans **delete `simd_utils.rs`**; the scaling lever is layout/
   locality, not vector width. Scope: measured on engine_v2's `SparseBitSlice`;
   `ultimate.rs`'s MSC kernel was NOT measured and this evidence does not reach it.

## Reproduction

```bash
cargo run --release --bin exp_kernel_floors
```
Expected: floors ≈ {16 GB/s, 5 ns, 3.6 ns} (host-dependent); ring rows flat
across the ladder; random rows rising ~2× ; flip ns 2.1× rise for random only.
