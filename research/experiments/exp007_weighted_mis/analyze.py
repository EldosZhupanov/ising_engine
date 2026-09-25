#!/usr/bin/env python3
"""Independent EXP-007W validator for frozen weighted-MIS outcomes."""

import argparse
import hashlib
import json
import math
from collections import defaultdict
from pathlib import Path


EXPERIMENT_ID = "EXP-007W"
INPUT_SHA256 = "3c901eaaa0098cc273427e6942227fb5c0c5010f459423e5686819d94878e520"
SPECS = ((64, 8), (64, 16), (96, 8), (96, 16), (128, 8), (128, 16))
PENALTY = 11


def load_graphs(path):
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != INPUT_SHA256:
        raise ValueError("frozen instance SHA-256 mismatch")
    data = json.loads(raw)
    if data["experiment_id"] != EXPERIMENT_ID or data["generator"] != "SHA-256-v1":
        raise ValueError("wrong dataset identity")
    if len(data["instances"]) != len(SPECS):
        raise ValueError("wrong graph count")
    result = {}
    for g, (graph, (n, p)) in enumerate(zip(data["instances"], SPECS)):
        name = f"er_n{n}_p{p:02}_g{g}"
        if graph["name"] != name or graph["n"] != n or graph["edge_percent"] != p:
            raise ValueError(f"wrong graph metadata: {name}")
        weights = graph["weights"]
        edges = [tuple(edge) for edge in graph["edges"]]
        if len(weights) != n or any(type(w) is not int or not 1 <= w <= 10 for w in weights):
            raise ValueError(f"invalid weights: {name}")
        if any(u >= v or u < 0 or v >= n for u, v in edges) or edges != sorted(set(edges)):
            raise ValueError(f"invalid edges: {name}")
        result[name] = graph
    return result


def direct(graph, state):
    if len(state) != graph["n"] or any(bit not in "01" for bit in state):
        raise ValueError("invalid binary state")
    weight = sum(w for w, bit in zip(graph["weights"], state) if bit == "1")
    collisions = sum(state[u] == "1" and state[v] == "1" for u, v in graph["edges"])
    return weight, collisions, -weight + PENALTY * collisions


def flip_energy(graph, state, positions):
    state = list(state)
    for index in positions:
        state[index] = "0" if state[index] == "1" else "1"
    return direct(graph, "".join(state))[2]


def exact_sign_p(wins, losses):
    n = wins + losses
    if not n:
        return 1.0
    return sum(math.comb(n, k) for k in range(wins, n + 1)) / (2 ** n)


def check_probe(row, graph, index):
    errors = []
    s = row["start_index"]
    if row["evaluation_id"] != EXPERIMENT_ID or row["graph"] != graph["name"]:
        errors.append("wrong probe identity")
    if row["seed"] != 9_000_000 + 1_000 * index + s:
        errors.append("wrong probe seed")
    state = row["state"]
    try:
        _, _, energy = direct(graph, state)
    except ValueError as exc:
        return [str(exc)], False
    fixed = row["fixed"]
    if any(len(item) != 2 or type(item[0]) is not int or type(item[1]) is not int
           or not 0 <= item[0] < graph["n"] or item[1] not in (0, 1)
           for item in fixed) or len({item[0] for item in fixed}) != len(fixed):
        return ["invalid fixed-spin list"], False
    is_fixed = {i for i, value in fixed}
    if any(int(state[i]) != value for i, value in fixed):
        errors.append("state violates fixed spin")
    if abs(row["model_energy"] - energy) > 1e-9 or abs(row["direct_energy"] - energy) > 1e-9:
        errors.append("probe energy mismatch")
    singles = [flip_energy(graph, state, (i,)) - energy
               for i in range(graph["n"]) if i not in is_fixed]
    pairs = [(flip_energy(graph, state, (u, v)) - energy, u, v)
             for u, v in graph["edges"] if u not in is_fixed and v not in is_fixed]
    minimum_single = min(singles, default=None)
    minimum_pair = min((item[0] for item in pairs), default=None)
    for name, actual in (("min_single_delta", minimum_single), ("min_pair_delta", minimum_pair)):
        reported = row[name]
        if (actual is None) != (reported is None) or (actual is not None and abs(reported - actual) > 1e-9):
            errors.append(f"{name} mismatch")
    if minimum_single is not None and minimum_single < -1e-9:
        errors.append("not a 1-opt minimum")
    improving = min((item for item in pairs if item[0] < -1e-9), default=None)
    reported = row["improving_pair"]
    if improving is None:
        if reported is not None:
            errors.append("false improving pair")
    elif reported is None or (reported["u"], reported["v"]) != improving[1:] or abs(reported["delta"] - improving[0]) > 1e-9:
        errors.append("improving pair mismatch")
    if not row["valid"]:
        errors.append("harness marked probe invalid")
    return errors, improving is not None


def check_cell(row, graph, index):
    errors = []
    c = row["campaign"]
    if row["evaluation_id"] != EXPERIMENT_ID or row["graph"] != graph["name"]:
        errors.append("wrong cell identity")
    if row["n"] != graph["n"] or row["edges"] != len(graph["edges"]):
        errors.append("wrong graph metadata")
    by_seed = {}
    for arm_name in ("baseline", "candidate"):
        arm = row[arm_name]
        if arm["completed"] != len(arm["solutions"]) or arm["completed"] < 1 or not arm["valid"]:
            errors.append(f"invalid or empty {arm_name} arm")
        best = None
        weights = {}
        for k, sol in enumerate(arm["solutions"]):
            seed = 7_000_000 + 100_000 * index + 10_000 * c + k
            if sol["seed"] != seed:
                errors.append(f"wrong {arm_name} seed {k}")
            try:
                weight, collisions, energy = direct(graph, sol["state"])
            except ValueError as exc:
                errors.append(f"{arm_name} solve {k}: {exc}")
                continue
            if sol["weight"] != weight or sol["collisions"] != collisions or collisions or not sol["valid"]:
                errors.append(f"invalid {arm_name} solution {k}")
            if abs(sol["model_energy"] - energy) > 1e-9 or energy != -weight:
                errors.append(f"energy mismatch in {arm_name} solve {k}")
            if not 0 <= sol["elapsed_ms"] <= 2000.000001 or sol["duration_ms"] < 0:
                errors.append(f"late solution counted in {arm_name} solve {k}")
            best = weight if best is None else max(best, weight)
            weights[seed] = weight
        if arm["best_weight"] != best:
            errors.append(f"best weight mismatch in {arm_name}")
        by_seed[arm_name] = weights
    b, a = row["baseline"]["best_weight"], row["candidate"]["best_weight"]
    delta = a - b if a is not None and b is not None else None
    if row["delta"] != delta or not row["valid"]:
        errors.append("paired cell mismatch")
    shared = set(by_seed["baseline"]) & set(by_seed["candidate"])
    return errors, delta, [by_seed["candidate"][seed] - by_seed["baseline"][seed] for seed in shared]


def analyze(structural_path, raw_path, data_path):
    graphs = load_graphs(data_path)
    structural = [json.loads(line) for line in structural_path.read_text().splitlines()]
    cells = [json.loads(line) for line in raw_path.read_text().splitlines()]
    expected_keys = {(name, c) for name in graphs for c in range(10)}
    if len(structural) != 60 or {(r["graph"], r["start_index"]) for r in structural} != expected_keys:
        raise ValueError("expected 60 unique structural rows")
    if len(cells) != 60 or {(r["graph"], r["campaign"]) for r in cells} != expected_keys:
        raise ValueError("expected 60 unique paired cells")
    by_graph = defaultdict(lambda: {"wins": 0, "ties": 0, "losses": 0, "differences": [], "probe_hits": 0})
    invalid_probes = []
    invalid_cells = []
    fixed_by_graph = {}
    for row in structural:
        name = row["graph"]
        g = list(graphs).index(name)
        errors, hit = check_probe(row, graphs[name], g)
        frozen_fixed = tuple(tuple(item) for item in row["fixed"])
        if name in fixed_by_graph and frozen_fixed != fixed_by_graph[name]:
            errors.append("fixed list changed within graph")
        fixed_by_graph[name] = frozen_fixed
        if errors:
            invalid_probes.append({"graph": name, "start_index": row["start_index"], "reasons": errors})
        else:
            by_graph[name]["probe_hits"] += int(hit)
    wins = ties = losses = 0
    completed = {"baseline": 0, "candidate": 0}
    late = {"baseline": 0, "candidate": 0}
    paired = []
    for row in cells:
        name = row["graph"]
        g = list(graphs).index(name)
        errors, delta, same_seed = check_cell(row, graphs[name], g)
        for arm in completed:
            completed[arm] += row[arm]["completed"]
            late[arm] += row[arm]["late_completions"]
        if errors:
            invalid_cells.append({"graph": name, "campaign": row["campaign"], "reasons": errors})
            continue
        paired.extend(same_seed)
        by_graph[name]["differences"].append(delta)
        if delta > 0:
            wins += 1
            by_graph[name]["wins"] += 1
        elif delta < 0:
            losses += 1
            by_graph[name]["losses"] += 1
        else:
            ties += 1
            by_graph[name]["ties"] += 1
    p_value = exact_sign_p(wins, losses)
    hit_graphs = sum(group["wins"] > 0 for group in by_graph.values())
    structural_hits = sum(group["probe_hits"] for group in by_graph.values())
    go = (not invalid_probes and not invalid_cells and wins >= 10 and losses <= 3
          and hit_graphs >= 3 and p_value < 0.05)
    return {
        "experiment_id": EXPERIMENT_ID,
        "structural_rows": len(structural),
        "structural_hits": structural_hits,
        "mechanism_h1": "SUPPORTED" if structural_hits and not invalid_probes else "NO-GO",
        "invalid_probes": invalid_probes,
        "cells": len(cells), "wins": wins, "ties": ties, "losses": losses,
        "win_graphs": hit_graphs, "sign_test_p_one_sided": p_value,
        "decision_h2": "GO" if go else "NO-GO",
        "invalid_cells": invalid_cells,
        "completed": completed, "late_completions": late,
        "same_seed_pairs": len(paired),
        "same_seed_wins": sum(x > 0 for x in paired),
        "same_seed_ties": sum(x == 0 for x in paired),
        "same_seed_losses": sum(x < 0 for x in paired),
        "fixed_counts": {name: len(items) for name, items in fixed_by_graph.items()},
        "by_graph": dict(by_graph),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--structural", type=Path, required=True)
    parser.add_argument("--raw", type=Path, required=True)
    parser.add_argument("--data", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(analyze(args.structural, args.raw, args.data), indent=2, sort_keys=True))
