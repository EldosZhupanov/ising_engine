# Adversarial Architecture Review — Ising Engine vs. "Beat Every Classical Solver"

Read-only review, 2026-07. Evidence: kernel source (`src/solver/engine.rs`,
`types.rs`, `ultimate.rs`), the 499-instance equal-wall-clock benchmark
(`benchmark_suite/results/`), and PERF.md. Host: 16 logical cores, AVX2, WSL2.
No code was changed; no optimization applied.

**Measured headline that motivates the ranking** (from the full benchmark's
calibration data, same host, same seconds):

| instance | engine effective flip-attempts/s | OpenJij (1 core) | resource ratio |
|---|---|---|---|
| G60 (n=7000, deg≈5 sparse) | ≈34 M/s (≈10 cores + AVX2) | ≈14 M/s | **2.4× throughput from ~40× the silicon** |
| bqp1000 (deg≈99 dense) | ≈2.0 G lane-edge/s | ≈0.13 G edge/s | ≈16× — wins dense decisively |

The engine's architecture is a dense-QUBO weapon. Its losses are exactly
where the items below predict.

---

## Rank 1 — Fixed width-vs-depth chain allocation (algorithmic scalability)

**What:** `NUM_REPLICAS = 64` (hard constant, `types.rs:17`) × `num_temps: 10`
(default, `ultimate.rs:72`) regardless of n and of the time budget. At n=7000
under an 8 s budget, calibration yields 15×4 = 60 sweeps *total* per chain —
an annealing schedule ~250× shorter than OpenJij's 15,756 sweeps in the same
wall time.

**Evidence:** benchmark: engine loses **every** G-Set instance n≥800
(G60 mean gap 1.03% vs 0.05%); wins every dense BQP. A 640-chain population
with a 60-step schedule cannot anneal; this is not a constant-factor issue.

**Why it blocks the goal:** state-of-the-art CPU solvers scale schedule depth
with n/budget. Without budget-aware width↔depth allocation, no amount of
kernel tuning wins large sparse instances.

## Rank 2 — RNG buffer memory traffic (roofline / bandwidth)

**What:** `StepScratch.rng_buf` holds one **f64 per spin-site per step**
(`engine.rs:435`, filled at `:567`, consumed at `:603`). That is 8 bytes of
random stream per 1 byte of spin state — written then read, every step.

**Evidence:** at n=7000: spins = 4.5 MB, rng_buf = 35.8 MB/step ⇒ ≈72 MB/step
of pure RNG traffic vs ≈9 MB of spin traffic. The sweep is bandwidth-bound at
large n and ~90% of the bytes moved are random numbers. A u32 (or f32)
compare-threshold halves-to-quarters this; generating lanes in-register
(the vectorized xoshiro already exists, `Xoshiro256PlusPlusX8`) removes the
round-trip through memory entirely.

## Rank 3 — Acceptance path dominates sparse instances (SIMD utilization)

**What:** per variable, the accept loop (`engine.rs:591-611`) evaluates
`fast_exp` (≈20 FLOPs, polynomial — good, branchless, vectorizable) for **all
64 lanes always**, plus a rng_buf load, even for lanes with Δ≤0 (auto-accept)
and even though at low temperature the vast majority of proposals are
rejected identically. For deg-5 graphs the delta kernel is ~320 mul-adds but
acceptance is ~1,280 FLOPs + 512 B of RNG loads — **acceptance, not physics,
is the sparse hot loop**.

**Evidence:** the 2.4×-for-40×-resources number above; dense instances (where
the edge kernel amortizes acceptance) win 16×.

Secondary: every edge iteration converts i8→f64 and FMAs into `delta[64]`
(`engine.rs:210-213`); integer accumulation (i16/i32) with one final convert
would raise lanes/instruction ~4× for the ±1/unit-weight graphs (G-Set,
torus).

## Rank 4 — Parallel scalability ceiling (Amdahl)

**What:** Phase-1 parallelism grain = one rayon task per (temperature ×
population) cell: `num_temps × num_pops = 10` tasks (`engine.rs:553-559`) on
a 16-core host ⇒ ≥6 cores idle by construction. Phase 2 (replica exchange,
`engine.rs:625-685`), round-trip bookkeeping, PA resampling, ICM, and the
polish/finishers are fully serial on the main thread.

**Why it blocks the goal:** the ceiling binds tighter exactly when it matters
(more cores, larger machines). No intra-cell (over-variables or over-lane-
blocks) parallel dimension exists to fill the gap, and there is no NUMA
placement story for multi-socket deployment (moot on this host, fatal on a
2-socket server where `spins` is one interleaved allocation).

## Rank 5 — Replica-exchange swap: strided byte swaps (cache & branches)

**What:** an accepted swap physically exchanges spins **one byte at a time**
across two cells with a per-lane branch inside the triple loop
(`engine.rs:654-665`): stride-64 access touching a full cache line per useful
byte, unpredictable branch on `swap_mask[r]` in the innermost position, all
serial. Cost O(pairs × n × 64) bytes moved per step at ~1/64 line utilization.

**Fix direction (not applied):** swap replica *identity* (index/pointer
indirection or per-cell lane permutation), not data — the standard zero-copy
PT exchange. Also removes the Rank-4 serial section's largest term.

## Rank 6 — Byte-per-replica layout is 8× memory vs. multi-spin coding

**What:** deliberate SIMD-first design (`types.rs` docs): 1 spin = 1 byte.
Competitive CPU annealers (Isakov-style MSC) pack 64 replicas/u64word, an 8×
footprint and bandwidth reduction, trading convert-free FMA for popcount/mask
arithmetic. At n=10⁵–10⁶ (the "beat everyone" regime) the byte layout blows
L2/L3: spins alone at n=10⁶ = 640 MB × plus 8× that in RNG stream (Rank 2).

**Evidence:** today's benchmark tops out at n=10⁴ where this is latent; the
roofline math says it becomes the wall next.

## Rank 7 — Temperature-ladder *count* does not scale

**What:** adaptive ladders tune β *positions* (acceptance-uniform default —
good, +57% round trips per PERF.md) but `num_temps = 10` is fixed; replica-
exchange theory wants the number of rungs to grow (≈√(specific heat)·√n) to
hold swap acceptance constant. At n=7000 with 10 rungs the β-gaps are large;
round-trip time inflates.

**Evidence:** PERF.md ladder study is at n=40; no large-n ladder data exists
— itself a gap. Indirect: all large-instance losses (Rank 1 confounds this;
they compound).

## Rank 8 — Population management copies (PA path)

**What:** PA resampling (`apply_resample`) clones whole spin blocks per
cloned member and the post-resample relaxation runs serially; `num_pops = 1`
in the production default, so today this is dormant — but it is the scaling
path for the PA-PT mode. Copy-on-resample of byte-fat replicas (Rank 6)
multiplies.

## Rank 9 — Checked and NOT bottlenecks (for the record)

- **Branch prediction, Phase 1:** accept/flip/energy update are branchless
  (`engine.rs:599-610`); `fast_exp` is clamp+polynomial (no table, no
  branch); `is_clamped` is per-variable, predictable. Clean.
- **False sharing:** Phase-1 rayon chunks are disjoint slices; per-cell
  energy arrays are 512 B (line-aligned by size); Phase 2 is single-threaded.
  No evidence of false sharing.
- **Hot-loop allocation:** `StepScratch` pre-allocates; sweep makes zero
  heap allocations (verified by construction, `engine.rs:388-447`).
- **RNG quality/vectorization:** xoshiro256++ ×8 lanes, fill outside the
  hot loop — sound design, it is the *volume* (Rank 2), not the generator.

---

## Summary ranking

| # | bottleneck | axis | binds when | measured evidence |
|---|---|---|---|---|
| 1 | fixed 64×10 chain grid, no depth/width budget allocation | scheduling | large sparse, any budget | loses all G-Set ≥800; 60 sweeps/chain |
| 2 | f64-per-site RNG stream through memory | bandwidth | n ≳ 10³ | 8:1 RNG:spin byte ratio |
| 3 | acceptance FLOPs + i8→f64 per edge-lane | SIMD util | sparse graphs | 2.4× throughput from 40× resources |
| 4 | 10-task parallel grain; serial Phase 2/finishers | scalability | ≥16 cores | 6+ idle cores by construction |
| 5 | byte-wise branchy replica swaps | cache/branches | every step | 1/64 line utilization, serial |
| 6 | 8× footprint vs MSC packing | memory | n ≳ 10⁵ | roofline projection |
| 7 | ladder count fixed at 10 | temp scheduling | large n | no large-n ladder data (gap) |
| 8 | PA resample deep copies | population mgmt | PA mode | dormant (num_pops=1) |

The composite adversarial verdict: **the engine is architecturally optimized
for the dense mid-size regime it already wins, and structurally incapable of
winning the large sparse regime without Rank 1 + 3 + 5 changes.** Ranks 2/6
set the wall for the n≥10⁵ ambitions beyond that.
