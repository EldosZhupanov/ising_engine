# memory/PERFORMANCE.md — pointer

**Authoritative source: `../PERF.md`** — measurement log, benchmark groups,
ladder comparison, and the dated change table. Add new measurements *there*.

Protocol lives in `../CLAUDE.md` §9 and `../VERIFY.md`.

## Read this before quoting any number

**The host exhibits same-code timing drift of up to ~9% within a session** —
measured directly, 158.8 → 166 ms on identical binaries. Consequences:

- **Absolute ms figures are not comparable across sessions.** Ever.
- Only **back-to-back A/B on the same host state** is trustworthy.
- Prefer **solution quality / TTS** over raw runtime for algorithmic claims.
- A proper TTS harness (optimal stopping + bootstrap CIs) is the top remaining
  measurement-infrastructure item.

## The rules that reject work by default

From `../CLAUDE.md` §4 and ADR-0004:

- Keep only **measured >1% wins**; revert regressions immediately.
- Any change that alters trajectories is **behaviour-changing and rejected by
  default** unless explicitly approved — this includes FP contraction
  (`mul_add`), reduced-precision RNG comparisons, and reordered float sums,
  however much faster they are.
- A/B at identical seeds via
  `benchmark_suite/scripts/ab_engine_compare.py` (asserts bit-identical
  energies) or `bin/ab_evolved_vs_ultimate.rs`.
- `tests/test_regression_golden.rs` must pass unchanged.
- No `unsafe` for micro-optimization.

## Standing results worth remembering

- `DenseByte` backend: **4.8× vs the f64 oracle**, bit-identical.
- Adaptive ladder: **acceptance-uniform +57.4%** round trips vs geometric (the
  default); feedback-optimized KTHT +43.5% (opt-in). Feedback needs an adequate
  tuning budget — at ~10 tuning steps the flow estimate collapses the ladder
  (−99.9% round trips); a round-trip-count guard now skips redistribution when
  the sample is too thin.
- `SparseBitSlice::apply_flips` adaptive dispatch: **1.6–4.6× at high flip
  count**, no regression at low; `random_flip_sweep` −29% end-to-end.
- `evaluate_predictor` rewritten to additive sufficient statistics:
  **217.6 → 12.1 ms/call (~18×)**, Spearman preserved at 0.7467.
- **No GPU. No explicit SIMD intrinsics.** Only `-Ctarget-cpu=native
  +avx2,+fma` auto-vectorization via `.cargo/config.toml`. See
  [OPEN_PROBLEMS.md](OPEN_PROBLEMS.md) §2.
