#!/usr/bin/env bash
set -euo pipefail
cd /home/eldos/ising_engine
cargo build --release --bin exp_cd003_market_residual
python3 research/experiments/cd003_market_residual/study.py run /home/eldos/ising_engine/target/release/exp_cd003_market_residual /home/eldos/ising_engine/research/experiments/cd003_market_residual/results/run001
