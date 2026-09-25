#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    printf 'usage: %s NEW_OUTPUT_DIRECTORY\n' "$0" >&2
    exit 2
fi

repo_root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$repo_root"
cargo build --release --bin exp007_weighted_mis
python3 research/experiments/exp007_weighted_mis/run.py --output "$1"
