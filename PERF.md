# PERF.md

## Performance Tracking

Environment: Linux / WSL2, x86_64, `-Ctarget-cpu=native +avx2,+fma`.

⚠️ Methodology note: this host exhibits same-code criterion drift of up to
~9% within a session (measured directly, e.g. 158.8 → 166 ms on identical
binaries). Absolute ms figures are NOT comparable across sessions; only
back-to-back A/B measurements on the same host state are trustworthy, and
solution-quality/TTS should be preferred over raw runtime for algorithmic
claims. A proper TTS harness (optimal stopping + bootstrap CIs) is the
top remaining measurement-infrastructure item.

## Benchmark groups (see benches/benchmark.rs)

- `ultimate_kernel/*_mixed` — full annealing on seeded mixed-sign,
  presolve-immune, connected instances (self-asserted at setup).
- `overheads/presolve_short_circuit_200_vars` — whole-solver overhead on a
  fully-presolvable instance (RNG fill + PT bookkeeping, ~zero sweep math).
- `overheads/presolve_only_200_vars` — the O(nnz) persistency pass alone.

## Recent same-host reference (post continuous-mode wave 1)

| Benchmark | Time |
|---|---|
| ultimate_solver_200_vars_mixed | ~156 ms |
| presolve_short_circuit_200_vars | ~60 ms |
| presolve_only_200_vars | ~32 µs |

Quality: n=18 dense mixed-sign, low budget (3 sweeps × 5 exchanges),
200 seeded instances → 100% reached the exhaustive optimum, mean relative
gap 0.0000 (quality is saturated at this size; harder instances needed to
differentiate finisher/incumbent contributions).

## Ladder comparison (round-trip rate, KTHT objective)

SK spin glass, n=40, nt=12, 8 instances, 3000 production steps, 300 tuning
steps/iter × 8 iters. Higher round-trip count = better ladder.

| Ladder | Round trips | vs geometric |
|---|---|---|
| geometric (fixed) | 36607 | baseline |
| **acceptance-uniform** (default) | **57618** | **+57.4%** |
| feedback-optimized (KTHT, opt-in) | 52523 | +43.5% |

Both adaptive ladders beat geometric substantially; acceptance uniformization
wins on SK instances, so it remains the default (per "keep whichever performs
better"). Feedback optimization is faithful KTHT, convergent, and opt-in via
`LadderMode::FeedbackOptimized`. CRITICAL: feedback needs an adequate tuning
budget — at ~10 tuning steps the flow estimate collapses the ladder (−99.9%
round trips); a round-trip-count guard now skips redistribution when the
sample is too thin.

## Measurements

Date | Change | Effect
-----|--------|-------
2026-07 | Basis/Trotter/clamp correctness fixes | correctness (see tests)
2026-07 | Presolve (first-order persistency) + decomposition | instance-dependent search-space reduction
2026-07 | 8-lane vectorizable RNG | overhead floor −4% (isolated A/B)
2026-07 | DEO non-reversible PT + incumbent + 1-opt polish + data-driven β | quality/anytime; runtime within drift envelope
2026-07 | ICM, elite/path-relinking, PA diagnostics, TTS framework | quality/instrumentation (opt-in)
2026-07 | Exact probing persistency (replaces first-order on production path) | strictly stronger reduction, exhaustively verified
2026-07 | Round-trip tracking + feedback-optimized ladder | +43.5% round trips vs geometric (acceptance +57.4% remains default)
