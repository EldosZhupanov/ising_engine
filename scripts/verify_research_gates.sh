#!/usr/bin/env bash
set -euo pipefail

echo "================================================================================"
echo "          ISING ENGINE: SELECTED RESEARCH QUALITY GATES                       "
echo "================================================================================"

echo ">>> 1. Type Check (cargo check)..."
cargo check

echo ">>> 2. Formatting Check (cargo fmt --check)..."
cargo fmt --check

echo ">>> 3. Linter & Dead-Code Invariants (cargo clippy)..."
cargo clippy --all-targets -- -D warnings

echo ">>> 4. Unit, Integration & Golden Tests (cargo test --release)..."
cargo test --release

echo ">>> 5. Binaries Build Check (cargo build --release --bins)..."
cargo build --release --bins

echo ">>> 6. Git Diff Invariants..."
git diff --check
git diff --cached --check

echo ">>> 7. Test standalone fundamental-AI crate and audit its retained raw TSV..."
cargo test --manifest-path research/fundamental_ai/Cargo.toml
python3 scripts/audit_exp006_raw.py

echo ">>> 8. Build and exercise local QOBLIB checkers..."
rustc --edition=2021 -O benchmarks/qoblib/check_labs.rs -o target/release/check_labs
rustc --edition=2021 -O benchmarks/qoblib/check_marketsplit.rs -o target/release/check_marketsplit
rustc --edition=2021 -O benchmarks/qoblib/check_stableset.rs -o target/release/check_stableset

# N=74 lies beyond the checker's optimum table: this verifies the sequence's
# claimed energy (357), not global optimality.
target/release/check_labs 74 benchmarks/qoblib/world_records/checkpoint_N074.sol 357 \
    | grep -q "Energy cross-check OK: E(S)=357"
target/release/check_marketsplit \
    benchmarks/qoblib/marketsplit/instances/ms_03_050_002.dat \
    benchmarks/qoblib/marketsplit/solutions/ms_03_050_002.sol \
    | grep -q "VALID: Solution successfully verified"
if target/release/check_marketsplit \
    benchmarks/qoblib/marketsplit/unsolved_instances/ms_13_050_003.dat \
    benchmarks/qoblib/marketsplit/rejected_candidates/ms_13_050_003.sol >/dev/null; then
    echo "ERROR: Market Split checker accepted a known rejected candidate" >&2
    exit 1
else
    status=$?
    if [ "$status" -ne 21 ]; then
        echo "ERROR: Market Split checker returned unexpected status $status" >&2
        exit 1
    fi
fi
target/release/check_stableset \
    benchmarks/qoblib/instances/sloane_1dc_64.gph \
    benchmarks/qoblib/solutions/sloane_1dc_64.sol \
    | grep -q "VALID: Solution successfully verified"

echo "================================================================================"
echo "          SELECTED ROOT, FUNDAMENTAL-AI AND CHECKER GATES PASSED               "
echo "================================================================================"
