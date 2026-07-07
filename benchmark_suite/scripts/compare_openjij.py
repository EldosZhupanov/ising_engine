#!/usr/bin/env python3
"""Fair head-to-head: Ising Engine (UltimateSolver) vs. OpenJij SASampler.

Protocol (scientific comparison, no tuning):
  - identical instances, parsed once; BOTH solvers' returned states are
    re-scored with the single canonical energy function (ising_bench.Qubo),
    so the objective is identical by construction;
  - identical seed list per instance;
  - EQUAL WALL-CLOCK BUDGET per size class. The engine's one "sweep"
    updates 640 SIMD chains while OpenJij's updates one, so equal-sweeps
    would be meaningless; each solver's budget knob (engine: exchanges,
    OpenJij: num_sweeps in a single deep anneal — its quality-optimal use)
    is calibrated per instance with a short probe so both consume ≈ the
    same wall time. Probes are excluded from measurements;
  - solvers run at library defaults otherwise: engine
    UltimateSolver::new(5.0, 0.02, ...), OpenJij SASampler auto schedule.

Reference for gaps: verified best-known objective (QPLIB official index)
when available, else the best energy found by ANY run of EITHER solver on
that instance (documented per row).

Outputs under benchmark_suite/results/:
  raw/openjij_vs_ultimate.jsonl     one JSON record per run
  tables/summary.csv                per (instance, solver) aggregates
  plots/*.png                       gap and time figures
  report_openjij.md                 full report

Run inside benchmark-env:
  benchmark-env/bin/python3 benchmark_suite/scripts/compare_openjij.py \
      --classes small,medium,large
"""

import argparse
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO / "benchmark_suite"))

import openjij  # noqa: E402

from ising_bench import load_dataset  # noqa: E402

ENGINE_BIN = REPO / "target/release/solve_instance"
RESULTS = REPO / "benchmark_suite/results"

# (class, dataset, file, problem-or-None, format) — progressively larger.
MANIFEST = [
    # small: n = 50..125, T = 1 s
    ("small", "orlib", "bqp50.txt", 1, "orlib"),
    ("small", "orlib", "bqp50.txt", 2, "orlib"),
    ("small", "orlib", "bqp100.txt", 1, "orlib"),
    ("small", "orlib", "bqp100.txt", 2, "orlib"),
    ("small", "biqmac", "g05_100.0", None, "rudy"),
    ("small", "biqmac", "pm1s_100.3", None, "rudy"),
    ("small", "dimacs_maxcut", "sg3dl051000.mc", None, "rudy"),
    # medium: n = 250..800, T = 3 s
    ("medium", "orlib", "bqp250.txt", 1, "orlib"),
    ("medium", "orlib", "bqp500.txt", 1, "orlib"),
    ("medium", "qplib", "QPLIB_3565.qplib", None, "qplib"),
    ("medium", "qplib", "QPLIB_3705.qplib", None, "qplib"),
    ("medium", "qplib", "QPLIB_3506.qplib", None, "qplib"),
    ("medium", "gset", "G14", None, "rudy"),
    ("medium", "gset", "G1", None, "rudy"),
    # large: n = 1000..7000, T = 8 s
    ("large", "orlib", "bqp1000.txt", 1, "orlib"),
    ("large", "orlib", "bqp2500", 1, "orlib"),
    ("large", "gset", "G22", None, "rudy"),
    ("large", "gset", "G55", None, "rudy"),
    ("large", "gset", "G60", None, "rudy"),
]

TIME_BUDGET = {"small": 1.0, "medium": 3.0, "large": 8.0}
SEEDS = {"small": [1, 2, 3, 4, 5], "medium": [1, 2, 3, 4, 5], "large": [1, 2, 3]}


def load_qubo(dataset, fname, problem):
    """Canonical Qubo for scoring + the OpenJij Q-dict built from it."""
    insts = load_dataset(dataset, instances=[fname], as_qubo=True)
    if problem is not None:
        insts = [q for q in insts if q.name.endswith(f".{problem}")]
    qubo = insts[0]
    Q = {}
    for i, l in enumerate(qubo.linear):
        if l != 0.0:
            Q[(i, i)] = l
    for (i, j), w in qubo.quad.items():
        Q[(i, j)] = w
    return qubo, Q


# ---------------------------------------------------------------------------
# Solver drivers (each returns dict: energy, wall_s, work, extra)
# ---------------------------------------------------------------------------


def run_engine(path, fmt, problem, qubo, sweeps, exchanges, seed):
    cmd = [
        str(ENGINE_BIN), "--file", str(path), "--format", fmt,
        "--sweeps", str(sweeps), "--exchanges", str(exchanges), "--seed", str(seed),
    ]
    if problem is not None:
        cmd += ["--problem", str(problem)]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True)
    rec = json.loads(out.stdout)
    state = [1 if c == "1" else 0 for c in rec["state"]]
    return {
        "energy": qubo.energy(state),  # canonical re-score
        "wall_s": rec["wall_ms"] / 1000.0,  # internal solve time (no proc startup)
        "work": f"sweeps={sweeps}x{exchanges}",
    }


def run_openjij(sampler, Q, qubo, num_sweeps, seed):
    t0 = time.perf_counter()
    res = sampler.sample_qubo(Q, num_sweeps=num_sweeps, num_reads=1, seed=seed)
    wall = time.perf_counter() - t0
    sample = res.first.sample
    state = [int(sample.get(i, 0)) for i in range(qubo.n)]
    return {
        "energy": qubo.energy(state),  # canonical re-score
        "wall_s": wall,
        "work": f"num_sweeps={num_sweeps}",
    }


# ---------------------------------------------------------------------------
# Calibration: hit the wall-clock budget with each solver's budget knob
# ---------------------------------------------------------------------------


def calibrate_engine(path, fmt, problem, qubo, budget_s):
    """Two-point probe (exchanges 4 and 12) separates fixed setup overhead
    from the marginal per-exchange cost, then sizes `exchanges` (sweeps fixed
    at 15) so total wall time ≈ budget. One-point probing over-estimates the
    marginal cost and silently under-fills the budget."""
    t1 = run_engine(path, fmt, problem, qubo, 15, 4, seed=999)["wall_s"]
    t2 = run_engine(path, fmt, problem, qubo, 15, 12, seed=999)["wall_s"]
    per_exchange = max((t2 - t1) / 8, 1e-5)
    overhead = max(t1 - 4 * per_exchange, 0.0)
    exchanges = max(2, min(20000, round((budget_s - overhead) / per_exchange)))
    return 15, exchanges


def calibrate_openjij(sampler, Q, qubo, budget_s):
    """Two-point probe (500 and 2500 sweeps), same overhead separation, then
    size num_sweeps (single deep anneal) so wall time ≈ budget."""
    t1 = run_openjij(sampler, Q, qubo, 500, seed=999)["wall_s"]
    t2 = run_openjij(sampler, Q, qubo, 2500, seed=999)["wall_s"]
    per_sweep = max((t2 - t1) / 2000, 1e-9)
    overhead = max(t1 - 500 * per_sweep, 0.0)
    return max(100, min(50_000_000, round((budget_s - overhead) / per_sweep)))


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--classes", default="small,medium,large")
    ap.add_argument("--out", default=str(RESULTS))
    args = ap.parse_args()
    classes = set(args.classes.split(","))

    results_dir = Path(args.out)
    (results_dir / "raw").mkdir(parents=True, exist_ok=True)
    raw_path = results_dir / "raw" / "openjij_vs_ultimate.jsonl"

    sampler = openjij.SASampler()
    rows = []
    raw_fh = open(raw_path, "a", encoding="utf-8")
    for cls, dataset, fname, problem, fmt in MANIFEST:
        if cls not in classes:
            continue
        path = REPO / "benchmark_suite/data" / dataset / fname
        if not path.exists():
            print(f"skip {dataset}/{fname}: not downloaded", file=sys.stderr)
            continue
        try:
            qubo, Q = load_qubo(dataset, fname, problem)
        except (ValueError, StopIteration) as exc:
            print(f"skip {dataset}/{fname}: parse failed: {exc}", file=sys.stderr)
            continue
        budget = TIME_BUDGET[cls]
        iname = qubo.name if problem is None else f"{fname}.{problem}"
        print(f"[{cls}] {iname} n={qubo.n} nnz={qubo.nnz} budget={budget}s", flush=True)

        sweeps, exchanges = calibrate_engine(path, fmt, problem, qubo, budget)
        oj_sweeps = calibrate_openjij(sampler, Q, qubo, budget)
        print(f"  calibrated: engine 15x{exchanges}, openjij {oj_sweeps} sweeps", flush=True)

        inst_rows = []
        for seed in SEEDS[cls]:
            for solver, runner in (
                ("Ultimate", lambda s=seed: run_engine(path, fmt, problem, qubo, sweeps, exchanges, s)),
                ("OpenJij", lambda s=seed: run_openjij(sampler, Q, qubo, oj_sweeps, s)),
            ):
                r = runner()
                inst_rows.append(
                    {
                        "class": cls,
                        "dataset": dataset,
                        "instance": iname,
                        "n": qubo.n,
                        "nnz": qubo.nnz,
                        "maximize": qubo.maximize,
                        "best_known": qubo.best_known,
                        "solver": solver,
                        "seed": seed,
                        "energy": r["energy"],
                        "native": qubo.native_objective(r["energy"]),
                        "wall_s": r["wall_s"],
                        "work": r["work"],
                        "budget_s": budget,
                    }
                )
        # Persist per instance so a crash never loses completed measurements.
        for r in inst_rows:
            raw_fh.write(json.dumps(r) + "\n")
        raw_fh.flush()
        rows.extend(inst_rows)
        e_u = min(r["energy"] for r in inst_rows if r["solver"] == "Ultimate")
        e_o = min(r["energy"] for r in inst_rows if r["solver"] == "OpenJij")
        print(f"  best E: Ultimate {e_u:.1f} | OpenJij {e_o:.1f}", flush=True)

    raw_fh.close()
    print(f"\nwrote {len(rows)} runs -> {raw_path}")


if __name__ == "__main__":
    main()
