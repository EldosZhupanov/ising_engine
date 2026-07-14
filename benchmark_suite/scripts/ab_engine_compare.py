#!/usr/bin/env python3
"""A/B validation: baseline engine binary vs. optimized engine binary.

Runs BOTH binaries on the same instances with IDENTICAL fixed budgets
(sweeps × exchanges — no calibration, so the work per run is exactly equal)
and identical seeds. Because Fixes A–D are bit-identical by construction,
every (instance, seed) must produce the SAME energy; any mismatch fails the
run loudly. Wall-clock ratio is the speedup. CPU utilization comes from
/usr/bin/time; cache counters from `perf stat` when the kernel exposes them
(often unavailable under WSL2 — reported as n/a, not faked).

Usage:
  benchmark-env/bin/python3 benchmark_suite/scripts/ab_engine_compare.py \
      --baseline PATH --optimized PATH [--seeds 3]
"""

import argparse
import json
import shutil
import statistics as st
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
DATA = REPO / "benchmark_suite/data"

# (label, file, fmt, problem, sweeps, exchanges) — fixed equal work.
CASES = [
    ("bqp100.1  dense n=100", DATA / "orlib/bqp100.txt", "orlib", 1, 15, 200),
    ("bqp500.1  dense n=500", DATA / "orlib/bqp500.txt", "orlib", 1, 15, 60),
    ("bqp1000.1 dense n=1000", DATA / "orlib/bqp1000.txt", "orlib", 1, 15, 25),
    ("gka5c     dense n=100", DATA / "biqmac/gka5c.sparse", "biqmac", None, 15, 200),
    ("QPLIB_3506 n=496", DATA / "qplib/QPLIB_3506.qplib", "qplib", None, 15, 80),
    ("G14  sparse n=800", DATA / "gset/G14", "rudy", None, 15, 120),
    ("G22  sparse n=2000", DATA / "gset/G22", "rudy", None, 15, 60),
    ("G60  sparse n=7000", DATA / "gset/G60", "rudy", None, 15, 15),
]


def run(binary, path, fmt, problem, sweeps, exchanges, seed):
    cmd = ["/usr/bin/time", "-v", str(binary), "--file", str(path), "--format", fmt,
           "--sweeps", str(sweeps), "--exchanges", str(exchanges), "--seed", str(seed)]
    if problem is not None:
        cmd += ["--problem", str(problem)]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True, timeout=600)
    rec = json.loads(out.stdout)
    cpu_pct = None
    for line in out.stderr.splitlines():
        if "Percent of CPU" in line:
            cpu_pct = line.split(":")[-1].strip()
    return rec["energy"], rec["wall_ms"], cpu_pct


def perf_counters(binary, path, fmt, problem, sweeps, exchanges):
    """Cache counters via perf stat, if available."""
    if not shutil.which("perf"):
        return "perf unavailable (WSL2 kernel)"
    cmd = ["perf", "stat", "-e", "cache-misses,cache-references,instructions,cycles",
           str(binary), "--file", str(path), "--format", fmt,
           "--sweeps", str(sweeps), "--exchanges", str(exchanges), "--seed", "1"]
    if problem is not None:
        cmd += ["--problem", str(problem)]
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
        wanted = [l.strip() for l in out.stderr.splitlines()
                  if any(k in l for k in ("cache-misses", "cache-references",
                                          "instructions", "cycles"))]
        return "; ".join(wanted) if wanted else "perf produced no counters"
    except (subprocess.SubprocessError, OSError) as exc:
        return f"perf failed: {exc}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", required=True)
    ap.add_argument("--optimized", required=True)
    ap.add_argument("--seeds", type=int, default=3)
    args = ap.parse_args()

    mismatches = 0
    rows = []
    for (label, path, fmt, problem, sweeps, exchanges) in CASES:
        if not path.exists():
            print(f"skip {label}: file missing")
            continue
        walls = {"base": [], "opt": []}
        cpus = {"base": None, "opt": None}
        for seed in range(1, args.seeds + 1):
            eb, wb, cb = run(args.baseline, path, fmt, problem, sweeps, exchanges, seed)
            eo, wo, co = run(args.optimized, path, fmt, problem, sweeps, exchanges, seed)
            if eb != eo:
                print(f"ENERGY MISMATCH {label} seed {seed}: base {eb} vs opt {eo}")
                mismatches += 1
            walls["base"].append(wb)
            walls["opt"].append(wo)
            cpus = {"base": cb, "opt": co}
        mb, mo = st.mean(walls["base"]), st.mean(walls["opt"])
        rows.append((label, mb, mo, mb / mo, cpus))
        print(f"{label:26s} base {mb:9.1f} ms  opt {mo:9.1f} ms  speedup ×{mb/mo:.2f}  "
              f"cpu base {cpus['base']} opt {cpus['opt']}")

    print("\n-- perf counters on G60 (baseline, then optimized) --")
    g60 = next(c for c in CASES if "G60" in c[0])
    for name, b in (("baseline", args.baseline), ("optimized", args.optimized)):
        print(f"{name}: {perf_counters(b, g60[1], g60[2], g60[3], g60[4], g60[5])}")

    geo = 1.0
    for (_, _, _, s, _) in rows:
        geo *= s
    geo = geo ** (1 / len(rows)) if rows else float("nan")
    print(f"\ngeometric-mean speedup ×{geo:.2f} over {len(rows)} instances, "
          f"{args.seeds} seeds each")
    print("energy identity:", "FAILED" if mismatches else
          f"OK — all {len(rows) * args.seeds} pairs bit-identical")
    return 1 if mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
