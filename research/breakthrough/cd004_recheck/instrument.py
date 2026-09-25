#!/usr/bin/env python3
"""CD004-R exact core-learning witness; the historical witness is unchanged."""

import argparse
import importlib.util
import itertools
import json
import math
import random
import statistics
import sys
import time
from pathlib import Path


LEGACY = Path(__file__).resolve().parents[1] / "cd004" / "witness.py"
_spec = importlib.util.spec_from_file_location("cd004_legacy", LEGACY)
legacy = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(legacy)


def exact_oracle(J, h):
    """Independent exhaustive evaluator, including an attaining assignment."""
    n = len(h)
    best_energy = math.inf
    best_state = None
    for state in itertools.product((-1, 1), repeat=n):
        energy = -sum(h[i] * state[i] for i in range(n))
        energy -= sum(J[i][j] * state[i] * state[j]
                      for i in range(n) for j in range(i + 1, n))
        if energy < best_energy:
            best_energy, best_state = energy, state
    return best_energy, best_state


def direct_energy(J, h, state):
    """Primary-gate evaluator independent of the legacy search evaluator."""
    energy = 0.0
    for i, spin in enumerate(state):
        energy -= h[i] * spin
        for j in range(i + 1, len(state)):
            energy -= J[i][j] * spin * state[j]
    return energy


def solve(J, h, initial_state, learn):
    """Same DFS in both arms; candidate additionally caches certified cores."""
    n = len(h)
    best_state = tuple(initial_state)
    best_energy = legacy.compute_energy(J, h, best_state, n)
    counters = {"nodes": 0, "lb_calls": 0, "cache_hits": 0, "cores": 0}
    learned = []  # (tuple[(variable, sign)], incumbent at certification)

    def bound(variables, signs):
        counters["lb_calls"] += 1
        return legacy.compute_lower_bound(J, h, n, variables, signs)

    def core_for(signs):
        variables = list(range(len(signs)))
        core = list(zip(variables, signs))
        # Single-pass deletion yields an inclusion-minimal core for this order.
        for variable in variables:
            trial = [(i, s) for i, s in core if i != variable]
            if bound([i for i, _ in trial], [s for _, s in trial]) >= best_energy:
                core = trial
        core_variables = [i for i, _ in core]
        core_signs = [s for _, s in core]
        assert bound(core_variables, core_signs) >= best_energy
        return tuple(core)

    def recurse(signs):
        nonlocal best_state, best_energy
        counters["nodes"] += 1
        if learn:
            for core, threshold in learned:
                if all(i < len(signs) and signs[i] == s for i, s in core):
                    assert best_energy <= threshold
                    counters["cache_hits"] += 1
                    return
        depth = len(signs)
        if bound(list(range(depth)), signs) >= best_energy:
            if learn:
                learned.append((core_for(signs), best_energy))
                counters["cores"] += 1
            return
        if depth == n:
            energy = legacy.compute_energy(J, h, signs, n)
            if energy < best_energy:
                best_energy, best_state = energy, tuple(signs)
            return
        # Both siblings must be considered unless their own certified core matches.
        recurse(signs + [1])
        recurse(signs + [-1])

    start = time.perf_counter_ns()
    recurse([])
    wall_ns = time.perf_counter_ns() - start
    assert legacy.compute_energy(J, h, best_state, n) == best_energy
    return {"energy": best_energy, "state": list(best_state),
            **counters, "wall_ns": wall_ns}


def run(output):
    if output.exists():
        raise FileExistsError(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    failures = 0
    # Disclosed calibration case: never part of the 120-row holdout.
    warm_J, warm_h = legacy.generate_frustrated_instance(8, "complete_SK", 1)
    random.seed(10001)
    warm_state, _ = legacy.find_heuristic_incumbent(
        warm_J, warm_h, 8, num_restarts=1)
    solve(warm_J, warm_h, warm_state, learn=False)
    solve(warm_J, warm_h, warm_state, learn=True)
    with output.open("x", encoding="utf-8") as stream:
        for family_index, family in enumerate(("complete_SK", "random_sparse")):
            for n in (8, 10, 12):
                for seed in range(1000, 1020):
                    J, h = legacy.generate_frustrated_instance(n, family, seed)
                    random.seed(4_000_000 + 10_000 * n + 100 * family_index + seed)
                    start_state, _ = legacy.find_heuristic_incumbent(
                        J, h, n, num_restarts=1)
                    row = {"family": family, "n": n, "seed": seed,
                           "initial_energy": direct_energy(J, h, start_state)}
                    try:
                        optimum, oracle_state = exact_oracle(J, h)
                        runs = {"baseline": [], "candidate": []}
                        for repetition in range(3):
                            order = (("baseline", "candidate") if
                                     (seed + repetition) % 2 == 0 else
                                     ("candidate", "baseline"))
                            for arm in order:
                                runs[arm].append(solve(
                                    J, h, start_state, learn=arm == "candidate"))
                        results = {}
                        valid = direct_energy(J, h, oracle_state) == optimum
                        for arm, repeated in runs.items():
                            first = {k: v for k, v in repeated[0].items()
                                     if k != "wall_ns"}
                            valid &= all({k: v for k, v in result.items()
                                          if k != "wall_ns"} == first
                                         for result in repeated)
                            state = first["state"]
                            valid &= (len(state) == n and all(s in (-1, 1) for s in state)
                                      and direct_energy(J, h, state) == optimum
                                      and first["energy"] == optimum)
                            results[arm] = {**first,
                                            "wall_samples_ns": [r["wall_ns"] for r in repeated],
                                            "wall_ns": statistics.median(
                                                r["wall_ns"] for r in repeated)}
                        row.update({"optimum": optimum, "valid": bool(valid), **results})
                    except Exception as error:
                        row.update({"valid": False,
                                    "error": f"{type(error).__name__}: {error}"})
                    failures += not row["valid"]
                    stream.write(json.dumps(row, sort_keys=True) + "\n")
                    stream.flush()
    print(json.dumps({"rows": 120, "failures": failures, "output": str(output)}))
    if failures:
        raise SystemExit(1)


def analyze(input_path):
    rows = [json.loads(line) for line in input_path.read_text().splitlines()]
    if len(rows) != 120 or not all(row["valid"] for row in rows):
        raise ValueError("Correctness gate failed or row count incomplete")
    def summary(subset):
        values = {}
        for metric in ("nodes", "lb_calls", "wall_ns"):
            ratios = [row["candidate"][metric] / row["baseline"][metric]
                      for row in subset]
            values[metric] = {"median_ratio": statistics.median(ratios),
                              "min_ratio": min(ratios), "max_ratio": max(ratios)}
        values["time_wins"] = sum(row["candidate"]["wall_ns"]
                                  < row["baseline"]["wall_ns"] for row in subset)
        values["count"] = len(subset)
        return values
    groups = {f"{family}_{n}": summary([r for r in rows if r["family"] == family
                                        and r["n"] == n])
              for family in ("complete_SK", "random_sparse") for n in (8, 10, 12)}
    overall = summary(rows)
    return {"overall": overall, "groups": groups,
            "decision": ("exact-and-useful" if overall["wall_ns"]["median_ratio"] <= 0.90
                         and overall["time_wins"] >= 90 else "exact-but-slow")}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("run").add_argument("--output", type=Path, required=True)
    sub.add_parser("analyze").add_argument("--input", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        run(args.output)
    else:
        print(json.dumps(analyze(args.input), sort_keys=True, indent=2))
