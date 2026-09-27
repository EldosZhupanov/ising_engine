#!/usr/bin/env python3
"""Strict standalone minimum-QUBO bridge. Not wired to the legacy Rust harness.

Input JSON: {"offset": c, "linear": [h...], "pairs": [[i,j,J],...]}, i<j.
The sole qualified native heuristic is MERZ2002ONEOPT. No implicit MaxCut or
hyperheuristic conversion, missing-seed fallback, or candidate repair is allowed.
"""
import argparse
import csv
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import subprocess
import tempfile
import time

UPSTREAM_SHA = "585496274af5abb0849d0d47e135496b4688680b"
HEURISTIC = "MERZ2002ONEOPT"


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def number(value):
    if type(value) not in (int, float):
        raise ValueError("expected finite numeric coefficient")
    if abs(value) > 1e12:
        raise ValueError("coefficient exceeds supported magnitude")
    if not math.isfinite(value):
        raise ValueError("expected finite numeric coefficient")
    return float(value)


def validate_model(model):
    if not isinstance(model, dict) or set(model) != {"offset", "linear", "pairs"}:
        raise ValueError("model requires exactly offset, linear, pairs")
    if not isinstance(model["linear"], list) or not 1 <= len(model["linear"]) <= 100000:
        raise ValueError("invalid linear dimension")
    number(model["offset"])
    for h in model["linear"]:
        number(h)
    if not isinstance(model["pairs"], list):
        raise ValueError("pairs must be a list")
    seen = set()
    for pair in model["pairs"]:
        if not isinstance(pair, list) or len(pair) != 3:
            raise ValueError("pair requires i,j,J")
        i, j, value = pair
        if type(i) is not int or type(j) is not int or not 0 <= i < j < len(model["linear"]):
            raise ValueError("pair requires valid ordered distinct indices")
        if (i, j) in seen:
            raise ValueError("duplicate pair")
        seen.add((i, j))
        q = number(value)
        if (q / 2) * 2 != q:
            raise ValueError("pair cannot be halved losslessly")
    return model


def load_model(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON key")
            result[key] = value
        return result
    return validate_model(json.loads(Path(path).read_text(), object_pairs_hook=unique))


def export_qubo(model):
    validate_model(model)
    entries = [(i + 1, i + 1, -number(h)) for i, h in enumerate(model["linear"])]
    entries += [(i + 1, j + 1, -number(v) / 2) for i, j, v in model["pairs"]]
    return f"{len(model['linear'])} {len(entries)}\n" + "".join(
        f"{i} {j} {v:.17g}\n" for i, j, v in entries)


def exact_energy(model, state):
    """Independent raw polynomial evaluation, no solver buffers/converted Q."""
    validate_model(model)
    if len(state) != len(model["linear"]) or any(type(x) is not int or x not in (0, 1) for x in state):
        raise ValueError("invalid binary state")
    return (Fraction(model["offset"])
            + sum(Fraction(h) * x for h, x in zip(model["linear"], state))
            + sum(Fraction(v) * state[i] * state[j] for i, j, v in model["pairs"]))


def parse_output(stdout, model, seconds, input_path):
    lines = [line.strip() for line in stdout.splitlines() if line.strip()]
    if len(lines) != 3 or lines[1] != "Solution:":
        raise ValueError("expected one summary and one complete solution")
    row = next(csv.reader([lines[0]], strict=True))
    if len(row) != 6 or row[1] != HEURISTIC or row[2] != str(input_path):
        raise ValueError("invalid summary identity")
    budget, objective, runtime = map(float, (row[0], row[3], row[4]))
    if not all(math.isfinite(v) for v in (budget, objective, runtime)) or runtime < 0:
        raise ValueError("invalid summary metrics")
    # Upstream prints the requested limit at default stream precision (6 digits).
    if budget != float(format(seconds, '.6g')):
        raise ValueError("wrong reported requested budget")
    tokens = lines[2].split()
    if any(t not in ("0", "1") for t in tokens):
        raise ValueError("nonbinary output; no clamping allowed")
    state = [int(t) for t in tokens]
    energy = exact_energy(model, state)
    expected = Fraction(model["offset"]) - energy
    if abs(Fraction(row[3]) - expected) > Fraction(1, 10**9):
        raise ValueError("upstream objective disagrees with independent energy")
    return {"state": state, "energy": float(energy), "energy_exact": str(energy),
            "reported_max_objective": objective, "reported_runtime_seconds": runtime,
            "history_raw": row[5]}


def run(model, binary, seed, seconds, hard_timeout=5.0):
    validate_model(model)
    if type(seed) is not int or not 0 <= seed <= 65535:
        raise ValueError("MQLib seed must be an integer in [0,65535]")
    if (type(seconds) not in (int, float) or not math.isfinite(seconds) or seconds <= 0
            or type(hard_timeout) not in (int, float) or not math.isfinite(hard_timeout)
            or hard_timeout <= seconds):
        raise ValueError("require 0 < requested seconds < finite hard timeout")
    binary = Path(binary).resolve(strict=True)
    start = time.perf_counter()
    text = export_qubo(model)
    record = {"status": "INVALID", "model": model, "seed": seed,
              "requested_seconds": seconds, "hard_timeout_seconds": hard_timeout,
              "heuristic": HEURISTIC, "binary": str(binary), "binary_sha256": sha256(binary),
              "input_qubo": text, "input_sha256": hashlib.sha256(text.encode()).hexdigest()}
    with tempfile.TemporaryDirectory(prefix="ising-mqlib-") as temp:
        path = Path(temp) / "input.qubo"
        path.write_text(text)
        cmd = [str(binary), "-h", HEURISTIC, "-fQ", str(path), "-s", str(seed),
               "-r", str(seconds), "-ps"]
        record["command"] = cmd
        record["conversion_setup_seconds"] = time.perf_counter() - start
        launched = time.perf_counter()
        try:
            completed = subprocess.run(cmd, capture_output=True, text=True, timeout=hard_timeout,
                                       cwd=binary.parent.parent)
            record.update(stdout=completed.stdout, stderr=completed.stderr, exit_code=completed.returncode)
            record["process_wall_seconds"] = time.perf_counter() - launched
            if completed.returncode:
                raise ValueError("MQLib exited unsuccessfully")
            record.update(parse_output(completed.stdout, model, seconds, path))
            record["status"] = "VALID"
        except subprocess.TimeoutExpired as exc:
            record.update(status="TIMEOUT", error="hard process timeout", exit_code=None,
                          stdout=(exc.stdout or b"").decode(errors="replace") if isinstance(exc.stdout, bytes) else exc.stdout or "",
                          stderr=(exc.stderr or b"").decode(errors="replace") if isinstance(exc.stderr, bytes) else exc.stderr or "")
        except (ValueError, OSError, csv.Error) as exc:
            record["error"] = str(exc)
        record["total_wall_seconds"] = time.perf_counter() - start
    return record


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("model", type=Path)
    p.add_argument("--binary", type=Path, required=True)
    p.add_argument("--seed", type=int, required=True)
    p.add_argument("--seconds", type=float, required=True)
    p.add_argument("--hard-timeout", type=float, default=5.0)
    p.add_argument("--output", type=Path, required=True, help="new JSON file; never overwritten")
    a = p.parse_args()
    # Exclusive create BEFORE running prevents an accidental duplicate run.
    with a.output.open("x") as dest:
        try:
            result = run(load_model(a.model), a.binary, a.seed, a.seconds, a.hard_timeout)
        except (ValueError, OSError) as exc:
            result = {"status": "INVALID", "error": str(exc)}
        json.dump(result, dest, indent=2, allow_nan=False)
        dest.write("\n")
    return 0 if result["status"] == "VALID" else 2


if __name__ == "__main__":
    raise SystemExit(main())
