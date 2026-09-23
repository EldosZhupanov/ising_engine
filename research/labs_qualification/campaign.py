"""Frozen single-worker, equal-deadline LABS qualification and independent analysis."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import selectors
import signal
import statistics
import subprocess
import time

TARGETS = {40: 108, 50: 153, 60: 218}
SEEDS = list(range(730001, 730011))
BUDGET = 10.0


def energy(bits, n):
    if not isinstance(bits, str) or len(bits) != n or set(bits) - {"0", "1"}:
        raise ValueError("invalid sequence")
    spins = [1 - 2 * int(v) for v in bits]
    return sum(sum(spins[i] * spins[i + d] for i in range(n - d)) ** 2
               for d in range(1, n))


def incumbent(line, n, elapsed, budget):
    if not line.startswith("INC "):
        return None
    match = re.fullmatch(r"INC ([0-9]+) ([01]+)", line)
    if match is None:
        raise ValueError("malformed incumbent")
    claimed, bits = int(match[1]), match[2]
    actual = energy(bits, n)
    if claimed != actual:
        raise ValueError("energy mismatch")
    return {"energy": actual, "bits": bits, "seconds": elapsed,
            "eligible": elapsed <= budget}


def supervise(command, n, target, budget):
    """Receipt-time cutoff includes startup; drain output after killing without credit."""
    start = time.monotonic()
    row = {"command": command, "n": n, "target": target, "budget": budget,
           "events": [], "errors": [], "stdout": "", "stderr": ""}
    env = dict(os.environ, OMP_NUM_THREADS="1", OPENBLAS_NUM_THREADS="1",
               MKL_NUM_THREADS="1", RAYON_NUM_THREADS="1")
    try:
        proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                env=env, start_new_session=True)
    except OSError as exc:
        row.update(errors=[str(exc)], stop="launch_failure", returncode=None,
                   elapsed=time.monotonic() - start)
        return row
    selector = selectors.DefaultSelector()
    chunks = {"stdout": [], "stderr": []}
    for name, stream in [("stdout", proc.stdout), ("stderr", proc.stderr)]:
        os.set_blocking(stream.fileno(), False)
        selector.register(stream, selectors.EVENT_READ, name)
    pending = b""
    stop = None
    stopped_at = None

    def kill(reason):
        nonlocal stop, stopped_at
        if stop is None:
            stop = reason
            stopped_at = time.monotonic() - start
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    try:
        while selector.get_map():
            elapsed = time.monotonic() - start
            if stop is None and elapsed >= budget:
                kill("deadline")
            for key, _ in selector.select(timeout=max(0, min(0.01, budget - elapsed))
                                           if stop is None else 0.01):
                chunk = os.read(key.fileobj.fileno(), 65536)
                received = time.monotonic() - start
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                chunks[key.data].append(chunk)
                if key.data != "stdout":
                    continue
                pending += chunk
                while b"\n" in pending:
                    line, pending = pending.split(b"\n", 1)
                    try:
                        event = incumbent(line.decode("ascii").strip(), n, received, budget)
                        if event is not None:
                            # Output drained after a stop is evidence, never a new endpoint.
                            event["eligible"] &= stop is None
                            row["events"].append(event)
                            if event["energy"] < target:
                                raise ValueError("energy below proven optimum")
                            if event["eligible"] and event["energy"] == target:
                                kill("optimum")
                    except (ValueError, UnicodeError) as exc:
                        row["errors"].append(str(exc))
                        kill("invalid_witness")
            if stop is not None and time.monotonic() - start > budget + 2:
                row["errors"].append("output drain timeout")
                break
        if stop is None:
            stop = "unexpected_exit"
            row["errors"].append("solver exited before external stop")
        proc.wait(timeout=2)
    finally:
        if proc.poll() is None:
            os.killpg(proc.pid, signal.SIGKILL)
            proc.wait()
        selector.close()
        proc.stdout.close()
        proc.stderr.close()
    row.update(stdout=b"".join(chunks["stdout"]).decode("utf8", errors="replace"),
               stderr=b"".join(chunks["stderr"]).decode("utf8", errors="replace"),
               stop=stop, stop_seconds=stopped_at, returncode=proc.returncode,
               elapsed=time.monotonic() - start)
    if stop == "deadline" and stopped_at > budget + 0.1:
        row["errors"].append("deadline enforcement exceeded tolerance")
    return row


def best_event(row):
    valid = []
    for event in row["events"]:
        assert event["energy"] == energy(event["bits"], row["n"])
        assert math.isfinite(event["seconds"]) and event["seconds"] >= 0
        if event["eligible"]:
            assert event["seconds"] <= row["budget"]
            valid.append(event)
    return min(valid, key=lambda e: e["energy"]) if valid else None


def wilson(hits, count):
    z = 1.959963984540054
    p = hits / count
    center = (p + z * z / (2 * count)) / (1 + z * z / count)
    radius = z * math.sqrt(p * (1 - p) / count + z * z / (4 * count * count)) / (1 + z * z / count)
    return [center - radius, center + radius]


def analyze(directory):
    summary = {"budget_seconds": BUDGET, "arms": {}, "paired": {},
               "scope": "quality at a shared wall cap; no speed or SOTA claim"}
    for arm in ["pt", "lmats"]:
        summary["arms"][arm] = {}
        for n, target in TARGETS.items():
            values, failures, hit_times = [], [], []
            for seed in SEEDS:
                path = directory / f"{n}_{seed}_{arm}.json"
                if not path.exists():
                    values.append(None); failures.append(seed); continue
                row = json.loads(path.read_text())
                assert (row["arm"], row["n"], row["seed"], row["target"], row["budget"]) == (arm, n, seed, target, BUDGET)
                best = best_event(row)
                bad = row["errors"] or best is None or row.get("checker", {}).get("returncode") not in [0, 20]
                if bad:
                    failures.append(seed); values.append(None)
                else:
                    assert best["energy"] >= target
                    values.append(best["energy"])
                    if best["energy"] == target:
                        hit_times.append(best["seconds"])
            good = [v for v in values if v is not None]
            hits = sum(v == target for v in values)
            summary["arms"][arm][str(n)] = {
                "energies": values, "hits": hits, "runs": 10,
                "wilson95": wilson(hits, 10), "failures": failures,
                "median": statistics.median(good) if good else None,
                "range": [min(good), max(good)] if good else None,
                "relative_gaps": [None if v is None else (v-target)/target for v in values],
                "observed_hit_seconds": hit_times,
                "qualified": hits >= 8 and not failures}
    for n in TARGETS:
        a = summary["arms"]["pt"][str(n)]["energies"]
        b = summary["arms"]["lmats"][str(n)]["energies"]
        pairs = [(x, y) for x, y in zip(a, b) if x is not None and y is not None]
        summary["paired"][str(n)] = {"pt_wins": sum(x < y for x, y in pairs),
            "ties": sum(x == y for x, y in pairs), "lmats_wins": sum(x > y for x, y in pairs),
            "invalid_pairs": 10 - len(pairs)}
    summary["pt_qualified"] = all(v["qualified"] for v in summary["arms"]["pt"].values())
    return summary


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def check_witness(command):
    try:
        cp = subprocess.run(command, capture_output=True, text=True, timeout=5)
        return {"returncode": cp.returncode, "stdout": cp.stdout, "stderr": cp.stderr}
    except (OSError, subprocess.TimeoutExpired) as exc:
        def text(value):
            return value.decode("utf8", errors="replace") if isinstance(value, bytes) else (value or "")
        return {"returncode": None, "stdout": text(getattr(exc, "stdout", "")),
                "stderr": text(getattr(exc, "stderr", "")), "error": str(exc)}


def run(args):
    out = Path(args.output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    repo = Path(__file__).resolve().parents[2]
    status = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=repo, text=True)
    if status:
        raise RuntimeError("tracked source must be clean")
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    binaries = {key: str(Path(getattr(args, key)).resolve()) for key in ["pt", "lmats", "checker"]}
    environment = {"commit": commit, "binaries": binaries,
        "binary_sha256": {k: digest(v) for k, v in binaries.items()},
        "platform": platform.platform(), "python": platform.python_version(),
        "cpu": Path("/proc/cpuinfo").read_text(), "initial_load": os.getloadavg(),
        "seeds": SEEDS, "targets": TARGETS, "budget": BUDGET,
        "compiler_versions": {tool: subprocess.check_output([tool, "--version"], text=True)
                              for tool in ["rustc", "cargo", "gcc"]},
        "source_sha256": {str(p.relative_to(repo)): digest(p) for p in
            [Path(__file__), repo / "src/bin/qoblib_labs_challenge.rs", repo / "Cargo.lock",
             repo / "benchmarks/qoblib/check_labs.rs",
             *[repo / "research/labs_qualification" / name for name in
               ["PROTOCOL.md", "reference.json", "reference.tar.gz", "reference_logging.patch",
                "build_reference.py", "test_campaign.py"]]]}}
    (out / "environment.json").write_text(json.dumps(environment, indent=2) + "\n")
    for ni, (n, target) in enumerate(TARGETS.items()):
        for si, seed in enumerate(SEEDS):
            arms = ["pt", "lmats"] if (ni + si) % 2 == 0 else ["lmats", "pt"]
            for arm in arms:
                assert digest(binaries[arm]) == environment["binary_sha256"][arm]
                cmd = ([binaries[arm], "--qualify", str(n), str(seed)] if arm == "pt" else
                       [binaries[arm], str(n), str(seed), "3600", "1", "0"])
                row = supervise(cmd, n, target, BUDGET)
                row.update(arm=arm, seed=seed, load=os.getloadavg())
                path = out / f"{n}_{seed}_{arm}.json"
                path.write_text(json.dumps(row, indent=2) + "\n")
                best = best_event(row)
                if best is not None:
                    witness = out / f"{n}_{seed}_{arm}.sol"
                    witness.write_text(best["bits"] + "\n")
                    row["checker"] = check_witness([binaries["checker"], str(n), str(witness), str(best["energy"])])
                    if row["checker"]["returncode"] not in [0, 20] or f"E(S)={best['energy']}" not in row["checker"]["stdout"]:
                        row["errors"].append("official checker failure")
                path.write_text(json.dumps(row, indent=2) + "\n")
                print(json.dumps({"n": n, "seed": seed, "arm": arm,
                    "energy": best["energy"] if best else None, "errors": row["errors"]}), flush=True)
    (out / "summary.json").write_text(json.dumps(analyze(out), indent=2) + "\n")
    (out / "complete.json").write_text(json.dumps({"commit": commit, "cells": 60}) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="action", required=True)
    p = sub.add_parser("run")
    for name in ["pt", "lmats", "checker", "output"]:
        p.add_argument("--" + name, required=True)
    p = sub.add_parser("analyze")
    p.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.action == "run":
        run(args)
    else:
        print(json.dumps(analyze(args.directory), indent=2))
