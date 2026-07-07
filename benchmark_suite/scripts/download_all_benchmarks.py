#!/usr/bin/env python3
"""Download, verify and organize every public benchmark dataset registered in
configs/benchmark_registry.yaml — the single command behind the Benchmark Hub.

    python3 benchmark_suite/scripts/download_all_benchmarks.py            # everything
    python3 benchmark_suite/scripts/download_all_benchmarks.py --only gset,orlib
    python3 benchmark_suite/scripts/download_all_benchmarks.py --list
    python3 benchmark_suite/scripts/download_all_benchmarks.py --validate

Each dataset ends up under benchmark_suite/data/<name>/ with normalized
instance files, metadata.json (SHA256 manifest + provenance), and a README.md
with origin/license/citation. Mirrors are tried in order; failures are
recorded per file, never fatal to the rest of the run. Re-runs are
incremental (verified files are not re-fetched). After one successful run,
loading (`ising_bench.load_dataset`) is fully offline.

Stdlib only — no pip required.
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ising_bench import registry as reg  # noqa: E402
from ising_bench.download import download_all  # noqa: E402
from ising_bench.validators import validate_all  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--only", help="comma-separated dataset subset")
    ap.add_argument("--exclude", help="comma-separated datasets to skip")
    ap.add_argument("--data-dir", default=str(reg.DEFAULT_DATA_DIR))
    ap.add_argument("--timeout", type=int, default=60, help="per-download seconds")
    ap.add_argument(
        "--limit", type=int, default=0, help="max files/archives per dataset (0 = all)"
    )
    ap.add_argument("--force", action="store_true", help="re-download existing files")
    ap.add_argument("--list", action="store_true", help="list registered datasets and exit")
    ap.add_argument(
        "--validate",
        action="store_true",
        help="validate already-downloaded data (hashes + parses) and exit",
    )
    args = ap.parse_args()

    if args.list:
        for name in reg.dataset_names():
            ds = reg.dataset(name)
            print(f"{name:15s} {ds['kind']:8s} {ds['description']}")
        return 0

    data_dir = Path(args.data_dir)

    if args.validate:
        reports = validate_all(data_dir)
        bad = [r for r in reports if r["status"] == "problems"]
        return 1 if bad else 0

    only = set(args.only.split(",")) if args.only else None
    exclude = set(args.exclude.split(",")) if args.exclude else None
    unknown = (only or set()) - set(reg.dataset_names())
    if unknown:
        print(f"unknown dataset(s): {', '.join(sorted(unknown))}", file=sys.stderr)
        return 2

    results = download_all(
        data_dir,
        only=only,
        exclude=exclude,
        timeout=args.timeout,
        limit=args.limit,
        force=args.force,
    )
    print()
    hard_fail = 0
    for name, meta in results.items():
        print(f"{name:15s} {meta['status']:12s} {len(meta['files'])} files")
        if meta["status"] == "unavailable":
            hard_fail += 1
    print(f"\ndata directory: {data_dir}")
    print("next: --validate to re-verify, or ising_bench.load_dataset(name) to use")
    return 1 if hard_fail else 0


if __name__ == "__main__":
    sys.exit(main())
