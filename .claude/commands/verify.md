Run the full verification loop, phase by phase. If a phase fails, STOP and
fix before continuing to the next phase.

Phase 1 — format + lint:
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings

Phase 2 — full test suite (includes golden regression + determinism):
  cargo test
  (golden gate: tests/test_regression_golden.rs must pass byte-identical)

Phase 3 — release build, all binaries stay compilable:
  cargo build --release --bins

Phase 4 — benchmark-infrastructure self-tests (parsers + conventions):
  python3 benchmark_suite/scripts/selftest.py

Phase 5 — performance changes ONLY (never claim speedups without this):
  - Preserve the previous binary: cp target/release/solve_instance <tmp>/baseline
  - A/B with identical seeds and fixed budgets:
    benchmark-env/bin/python3 benchmark_suite/scripts/ab_engine_compare.py \
        --baseline <tmp>/baseline --optimized target/release/solve_instance
  - REQUIRE: all energy pairs bit-identical; keep only >1% measured speedups.

Output format:

VERIFICATION REPORT
===================
fmt/clippy:  PASS/FAIL
tests:       PASS/FAIL (N passed)
golden:      PASS/FAIL (bit-identical)
release:     PASS/FAIL
selftest:    PASS/FAIL
A/B (perf):  xN.NN geomean, energies identical: YES/NO  (or: not applicable)
Overall:     READY / NOT READY

Issues to fix:
1. ...
