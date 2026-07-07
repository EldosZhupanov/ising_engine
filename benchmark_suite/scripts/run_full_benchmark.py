#!/usr/bin/env python3
"""Publication-grade full benchmark: Ising Engine (UltimateSolver) vs.
OpenJij SASampler over EVERY locally downloaded instance.

Protocol upgrades over compare_openjij.py:
  - all ~500 instances from gset / orlib / biqmac / qplib / dimacs_maxcut /
    snap (multi-problem files expanded; unsupported files listed, not
    silently skipped);
  - seeds: small (n<200) 1..20, medium (n<1000) 1..10, large 1..5 —
    identical lists for both solvers;
  - ROBUST calibration: adaptive two-point affine fit per (instance,
    solver) followed by a measured verification run at the chosen knob;
    if the verification misses the wall-clock budget by >5% the knob is
    proportionally corrected and re-verified (max 3 rounds). Requested
    budget vs. actual mean wall time is printed per instance;
  - crash-safe incremental JSONL writes and RESUME: instances whose full
    seed set already exists in the output are skipped, so the run can be
    chunked or restarted freely;
  - canonical scoring: both solvers' raw states are re-scored with the
    same ising_bench.Qubo energy function;
  - graphs the engine cannot read natively (SNAP edge lists, DIMACS
    'e'-format) are materialized ONCE as rudy files under
    results/converted/ and BOTH sides consume the converted file, so the
    instance is bit-identical across solvers.

Solvers run untouched at defaults: no tuning, no algorithm changes.

Usage (inside benchmark-env):
  benchmark-env/bin/python3 benchmark_suite/scripts/run_full_benchmark.py \
      [--classes small,medium,large] [--datasets gset,orlib,...] [--dry-run]
"""

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO / "benchmark_suite"))

import openjij  # noqa: E402

from ising_bench import load_dataset, list_datasets  # noqa: E402
from ising_bench.parsers import parse_rudy  # noqa: E402

ENGINE_BIN = REPO / "target/release/solve_instance"
RESULTS = REPO / "benchmark_suite/results"
RAW = RESULTS / "raw/full_benchmark.jsonl"
CONVERTED = RESULTS / "converted"

CLASSES = {  # name -> (budget seconds, seed count)
    "small": (1.0, 20),
    "medium": (3.0, 10),
    "large": (8.0, 5),
}


def size_class(n: int) -> str:
    if n < 200:
        return "small"
    if n < 1000:
        return "medium"
    return "large"


# ---------------------------------------------------------------------------
# Instance enumeration
# ---------------------------------------------------------------------------


def graph_to_rudy(qubo_meta_dataset, fname, out_path):
    """Materialize a Graph instance as a rudy file (deterministic)."""
    from ising_bench import load_dataset as ld

    g = [x for x in ld(qubo_meta_dataset, instances=[fname])][0]
    with open(out_path, "w", encoding="utf-8") as fh:
        fh.write(f"{g.n} {len(g.edges)}\n")
        for (u, v, w) in g.edges:
            fh.write(f"{u + 1} {v + 1} {w:g}\n")


def enumerate_instances(datasets):
    """Yield dicts describing every runnable instance; collect unsupported."""
    CONVERTED.mkdir(parents=True, exist_ok=True)
    entries, unsupported = [], []
    for dataset in datasets:
        meta = json.load(open(REPO / "benchmark_suite/data" / dataset / "metadata.json"))
        for rec in meta["files"]:
            fname, parser = rec["file"], rec["parser"]
            path = REPO / "benchmark_suite/data" / dataset / fname
            try:
                qubos = load_dataset(dataset, instances=[fname], as_qubo=True)
            except (ValueError, StopIteration) as exc:
                unsupported.append((dataset, fname, f"parse failed: {exc}"))
                continue
            if parser in ("orlib_bqp",):
                for k, q in enumerate(qubos, start=1):
                    entries.append(dict(dataset=dataset, file=fname, problem=k,
                                        fmt="orlib", path=path, qubo=q))
            elif parser == "biqmac_sparse":
                entries.append(dict(dataset=dataset, file=fname, problem=None,
                                    fmt="biqmac", path=path, qubo=qubos[0]))
            elif parser == "qplib":
                entries.append(dict(dataset=dataset, file=fname, problem=None,
                                    fmt="qplib", path=path, qubo=qubos[0]))
            elif parser in ("rudy", "maxcut_auto", "dimacs", "snap_edgelist"):
                # Engine natively reads rudy only; convert anything else.
                head = open(path, encoding="utf-8", errors="replace").read(2048)
                first = next((l.split() for l in head.splitlines() if l.split()), [""])
                is_rudy = parser == "rudy" or (
                    parser == "maxcut_auto" and first[0] not in ("c", "p", "e")
                )
                if is_rudy:
                    use, q = path, qubos[0]
                else:
                    conv = CONVERTED / f"{dataset}__{fname}.rudy"
                    if not conv.exists():
                        graph_to_rudy(dataset, fname, conv)
                    # Re-parse from the converted file so BOTH sides consume
                    # the identical bytes.
                    q = parse_rudy(open(conv, encoding="utf-8").read(), fname).to_qubo()
                    q.best_known = qubos[0].best_known
                    use = conv
                entries.append(dict(dataset=dataset, file=fname, problem=None,
                                    fmt="rudy", path=use, qubo=q))
            else:
                unsupported.append((dataset, fname, f"unsupported parser {parser}"))
    return entries, unsupported


# ---------------------------------------------------------------------------
# Solver drivers (identical to compare_openjij, kept self-contained)
# ---------------------------------------------------------------------------


def run_engine(ent, sweeps, exchanges, seed, timeout):
    cmd = [str(ENGINE_BIN), "--file", str(ent["path"]), "--format", ent["fmt"],
           "--sweeps", str(sweeps), "--exchanges", str(exchanges), "--seed", str(seed)]
    if ent["problem"] is not None:
        cmd += ["--problem", str(ent["problem"])]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True, timeout=timeout)
    rec = json.loads(out.stdout)
    state = [1 if c == "1" else 0 for c in rec["state"]]
    return {"energy": ent["qubo"].energy(state), "wall_s": rec["wall_ms"] / 1000.0}


def run_openjij(sampler, Q, qubo, num_sweeps, seed):
    t0 = time.perf_counter()
    res = sampler.sample_qubo(Q, num_sweeps=num_sweeps, num_reads=1, seed=seed)
    wall = time.perf_counter() - t0
    sample = res.first.sample
    state = [int(sample.get(i, 0)) for i in range(qubo.n)]
    return {"energy": qubo.energy(state), "wall_s": wall}


def qdict(qubo):
    Q = {}
    for i, l in enumerate(qubo.linear):
        if l != 0.0:
            Q[(i, i)] = l
    Q.update(qubo.quad)
    return Q


# ---------------------------------------------------------------------------
# Robust calibration: adaptive two-point affine fit + verified correction
# ---------------------------------------------------------------------------


def calibrate(run_at, k_min, k_probe, budget, tol=0.05, rounds=3):
    """Find integer knob k so run_at(k) ≈ budget within tol.

    run_at(k) -> wall seconds. Affine model t = a + b·k fitted from a tiny
    probe and an adaptive second probe near 60% of budget (so the slope is
    never noise/overhead dominated), then a measured verification run with
    proportional correction until within tol. Returns (k, verified_t).
    """
    t1 = run_at(k_probe)
    slope0 = max(t1 / k_probe, 1e-9)
    k2 = max(k_probe * 4, int(0.6 * budget / slope0))
    t2 = run_at(k2)
    b = (t2 - t1) / max(k2 - k_probe, 1)
    if b <= 0:
        b = max(t2 / k2, 1e-9)
    a = max(t1 - b * k_probe, 0.0)
    k = max(k_min, int((budget - a) / b))
    t = run_at(k)
    for _ in range(rounds):
        if abs(t - budget) / budget <= tol:
            break
        k = max(k_min, int(k * budget / max(t, 1e-9)))
        t = run_at(k)
    return k, t


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--classes", default="small,medium,large")
    ap.add_argument("--datasets", default=",".join(list_datasets()))
    ap.add_argument("--dry-run", action="store_true", help="enumerate + estimate only")
    args = ap.parse_args()
    classes = set(args.classes.split(","))
    datasets = args.datasets.split(",")

    entries, unsupported = enumerate_instances(datasets)
    for e in entries:
        e["iname"] = e["qubo"].name if e["problem"] is None else f"{e['file']}.{e['problem']}"
        e["class"] = size_class(e["qubo"].n)
    entries = [e for e in entries if e["class"] in classes]
    entries.sort(key=lambda e: (e["qubo"].n, e["iname"]))

    RAW.parent.mkdir(parents=True, exist_ok=True)
    (RESULTS / "unsupported.txt").write_text(
        "\n".join(f"{d}/{f}: {r}" for d, f, r in unsupported) or "(none)\n",
        encoding="utf-8",
    )

    est = sum(CLASSES[e["class"]][0] * CLASSES[e["class"]][1] * 2 + 6 for e in entries)
    print(f"{len(entries)} instances selected ({len(unsupported)} unsupported listed); "
          f"estimated measured time ≈ {est/3600:.1f} h", flush=True)
    if args.dry_run:
        for e in entries[:10]:
            print(" ", e["class"], e["iname"], "n=", e["qubo"].n)
        return

    # ---- resume state: the JSONL is the source of truth ------------------
    # done[(inst, solver, seed)] = True; knobs[(inst, solver)] = calibrated
    # knob recorded with every row, so a partially-run instance resumes its
    # REMAINING seeds under the identical budget without re-calibrating.
    done, knobs = set(), {}
    if RAW.exists():
        for line in open(RAW, encoding="utf-8"):
            try:
                r = json.loads(line)
            except json.JSONDecodeError:
                continue
            done.add((r["instance"], r["solver"], r["seed"]))
            if "knob" in r:
                knobs[(r["instance"], r["solver"])] = r["knob"]

    def weight(ent):  # expected wall-seconds for an instance (runs + calib)
        b, k = CLASSES[ent["class"]]
        return b * k * 2 + 6.0

    total_weight = sum(weight(e) for e in entries)
    total_runs = sum(CLASSES[e["class"]][1] * 2 for e in entries)

    def write_progress(ent, done_runs, session_weight, session_elapsed, done_weight):
        remaining = max(total_weight - done_weight, 0.0)
        rate = session_elapsed / session_weight if session_weight > 0 else 1.0
        eta_h = remaining * rate / 3600
        body = (
            f"updated: {time.strftime('%Y-%m-%d %H:%M:%S')}\n"
            f"dataset: {ent['dataset']}\ninstance: {ent['iname']} (n={ent['qubo'].n})\n"
            f"runs completed: {done_runs}/{total_runs} ({100*done_runs/total_runs:.1f}%)\n"
            f"elapsed this session: {session_elapsed/60:.1f} min\n"
            f"ETA: {eta_h:.2f} h\n"
        )
        tmp = RESULTS / "progress.txt.tmp"
        tmp.write_text(body, encoding="utf-8")
        tmp.replace(RESULTS / "progress.txt")

    def write_checkpoint(ckpt):
        tmp = RAW.parent / "checkpoint.json.tmp"
        tmp.write_text(json.dumps(ckpt, indent=1), encoding="utf-8")
        tmp.replace(RAW.parent / "checkpoint.json")

    ckpt = {}
    for (inst, solver, seed) in done:
        ckpt.setdefault(inst, {}).setdefault(solver, []).append(seed)

    sampler = openjij.SASampler()
    out = open(RAW, "a", encoding="utf-8")
    t_start = time.time()
    session_weight = 0.0
    done_runs = len(done)
    done_weight = sum(
        weight(e) * min(1.0, sum(1 for s in ("Ultimate", "OpenJij")
                                 for sd in range(1, CLASSES[e["class"]][1] + 1)
                                 if (e["iname"], s, sd) in done)
                        / (CLASSES[e["class"]][1] * 2))
        for e in entries
    )

    for idx, ent in enumerate(entries, 1):
        qubo = ent["qubo"]
        budget, n_seeds = CLASSES[ent["class"]]
        seeds = list(range(1, n_seeds + 1))
        missing = [
            sd for sd in seeds
            if any((ent["iname"], s, sd) not in done for s in ("Ultimate", "OpenJij"))
        ]
        if not missing:
            continue
        Q = qdict(qubo)
        timeout = budget * 10 + 120

        # Calibrate only the solvers with no stored knob for this instance.
        try:
            k_eng = knobs.get((ent["iname"], "Ultimate"))
            if k_eng is None:
                k_eng, _ = calibrate(
                    lambda k: run_engine(ent, 15, k, 999, timeout)["wall_s"],
                    k_min=2, k_probe=2, budget=budget)
            k_oj = knobs.get((ent["iname"], "OpenJij"))
            if k_oj is None:
                k_oj, _ = calibrate(
                    lambda k: run_openjij(sampler, Q, qubo, k, 999)["wall_s"],
                    k_min=100, k_probe=300, budget=budget)
        except (subprocess.SubprocessError, json.JSONDecodeError, OSError) as exc:
            print(f"[{idx}/{len(entries)}] {ent['iname']}: CALIBRATION FAILED: {exc}",
                  flush=True)
            continue

        walls = {"Ultimate": [], "OpenJij": []}
        for seed in missing:
            try:
                runs = {}
                if (ent["iname"], "Ultimate", seed) not in done:
                    runs["Ultimate"] = run_engine(ent, 15, k_eng, seed, timeout)
                if (ent["iname"], "OpenJij", seed) not in done:
                    runs["OpenJij"] = run_openjij(sampler, Q, qubo, k_oj, seed)
            except (subprocess.SubprocessError, json.JSONDecodeError, OSError) as exc:
                print(f"[{idx}/{len(entries)}] {ent['iname']}: RUN FAILED (seed {seed}): {exc}",
                      flush=True)
                break
            # Persist IMMEDIATELY per seed: a crash never loses a finished run.
            for solver, r in runs.items():
                walls[solver].append(r["wall_s"])
                out.write(json.dumps({
                    "class": ent["class"], "dataset": ent["dataset"],
                    "instance": ent["iname"], "n": qubo.n, "nnz": qubo.nnz,
                    "maximize": qubo.maximize, "best_known": qubo.best_known,
                    "solver": solver, "seed": seed,
                    "energy": r["energy"],
                    "native": qubo.native_objective(r["energy"]),
                    "wall_s": r["wall_s"], "budget_s": budget,
                    "knob": k_eng if solver == "Ultimate" else k_oj,
                }) + "\n")
                done.add((ent["iname"], solver, seed))
                ckpt.setdefault(ent["iname"], {}).setdefault(solver, []).append(seed)
                done_runs += 1
                run_w = budget
                session_weight += run_w
                done_weight += run_w
            out.flush()
            write_checkpoint(ckpt)
            write_progress(ent, done_runs, session_weight,
                           time.time() - t_start, done_weight)
        if walls["Ultimate"] and walls["OpenJij"]:
            mu = sum(walls["Ultimate"]) / len(walls["Ultimate"])
            mo = sum(walls["OpenJij"]) / len(walls["OpenJij"])
            diff = 100.0 * abs(mu - mo) / budget
            elapsed = (time.time() - t_start) / 60
            print(f"[{idx}/{len(entries)}] {ent['class']:6s} {ent['iname']:24s} n={qubo.n:<6d}"
                  f" budget={budget:.0f}s engine={mu:.2f}s openjij={mo:.2f}s diff={diff:.1f}%"
                  f"  ({elapsed:.0f} min elapsed)", flush=True)
    out.close()

    # ---- final verification: every expected triple exactly once ----------
    counts = {}
    for line in open(RAW, encoding="utf-8"):
        try:
            r = json.loads(line)
        except json.JSONDecodeError:
            continue
        key = (r["instance"], r["solver"], r["seed"])
        counts[key] = counts.get(key, 0) + 1
    missing_triples, duplicate_triples = [], []
    for e in entries:
        for s in ("Ultimate", "OpenJij"):
            for sd in range(1, CLASSES[e["class"]][1] + 1):
                c = counts.get((e["iname"], s, sd), 0)
                if c == 0:
                    missing_triples.append((e["iname"], s, sd))
                elif c > 1:
                    duplicate_triples.append((e["iname"], s, sd, c))
    ver = [
        f"expected runs: {total_runs}",
        f"recorded unique: {total_runs - len(missing_triples)}",
        f"missing: {len(missing_triples)}",
        f"duplicated (last kept at analysis): {len(duplicate_triples)}",
    ]
    if missing_triples[:20]:
        ver.append("first missing: " + "; ".join(f"{i}/{s}/seed{sd}" for i, s, sd in missing_triples[:20]))
    (RESULTS / "verification.txt").write_text("\n".join(ver) + "\n", encoding="utf-8")
    print("\n".join(ver), flush=True)
    print("run complete" if not missing_triples else "run INCOMPLETE — restart to resume")
    sys.exit(0 if not missing_triples else 3)


if __name__ == "__main__":
    main()
