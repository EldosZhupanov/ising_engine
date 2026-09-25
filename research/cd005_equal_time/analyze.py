#!/usr/bin/env python3
"""Frozen independent MIS feasibility/energy analysis for CD005-Q."""

import argparse
import json
from collections import defaultdict
from pathlib import Path

from fetch import EVALUATION_ID, FILES, git_blob_sha1


def read_graph(path):
    header = None
    edges = set()
    for raw in path.read_text().splitlines():
        parts = raw.split()
        if not parts or parts[0] == "c":
            continue
        if parts[0] == "p" and len(parts) == 4 and parts[1] == "edge":
            if header is not None:
                raise ValueError(f"duplicate header: {path}")
            header = (int(parts[2]), int(parts[3]))
        elif parts[0] == "e" and len(parts) == 3 and header is not None:
            u, v = int(parts[1]) - 1, int(parts[2]) - 1
            if not 0 <= u < header[0] or not 0 <= v < header[0] or u == v:
                raise ValueError(f"invalid edge: {path}")
            edge = tuple(sorted((u, v)))
            if edge in edges:
                raise ValueError(f"duplicate edge: {path}")
            edges.add(edge)
        else:
            raise ValueError(f"invalid DIMACS line: {path}: {raw}")
    if header is None or len(edges) != header[1]:
        raise ValueError(f"header edge count mismatch: {path}")
    return header[0], edges


def analyze(raw_path, data_dir):
    graphs = {}
    for name, expected in FILES.items():
        path = data_dir / name
        if git_blob_sha1(path.read_bytes()) != expected:
            raise ValueError(f"blob mismatch: {name}")
        graphs[name] = read_graph(path)
    rows = [json.loads(line) for line in raw_path.read_text().splitlines()]
    expected_keys = {(name, campaign) for name in FILES for campaign in range(3)}
    keys = {(row["graph"], row["campaign"]) for row in rows}
    if len(rows) != 18 or keys != expected_keys:
        raise ValueError("Expected exactly 18 unique registered cells")
    wins = ties = losses = 0
    family = defaultdict(lambda: {"wins": 0, "ties": 0, "losses": 0,
                                  "failures": 0, "differences": []})
    paired_seed_differences = []
    completed = {"baseline": 0, "candidate": 0}
    late = {"baseline": 0, "candidate": 0}
    invalid_cells = []
    for row in rows:
        if row["evaluation_id"] != EVALUATION_ID:
            raise ValueError("unexpected evaluation iteration")
        name, campaign = row["graph"], row["campaign"]
        graph_index = list(FILES).index(name)
        n, edges = graphs[name]
        if row["n"] != n or row["edges"] != len(edges):
            raise ValueError(f"graph metadata mismatch: {name}")
        by_seed = {}
        cell_errors = []
        for arm_name in ("baseline", "candidate"):
            arm = row[arm_name]
            if arm["completed"] != len(arm["solutions"]):
                raise ValueError("completed count mismatch")
            if arm["completed"] < 1 or not arm["valid"]:
                cell_errors.append(f"invalid or empty {arm_name} arm")
            completed[arm_name] += arm["completed"]
            late[arm_name] += arm["late_completions"]
            best = None
            sizes = {}
            for k, sol in enumerate(arm["solutions"]):
                seed = 5_000_000 + 100_000 * graph_index + 10_000 * campaign + k
                state = sol["state"]
                if sol["seed"] != seed or len(state) != n or any(c not in "01" for c in state):
                    cell_errors.append(f"invalid state/seed in {arm_name} solve {k}")
                    continue
                size = state.count("1")
                collisions = sum(state[u] == "1" and state[v] == "1" for u, v in edges)
                energy = -size + 2 * collisions
                if collisions or sol["size"] != size or sol["collisions"] != collisions or not sol["valid"]:
                    cell_errors.append(f"MIS infeasible in {arm_name} solve {k}")
                if abs(sol["model_energy"] - energy) > 1e-9:
                    cell_errors.append(f"QUBO energy mismatch in {arm_name} solve {k}")
                if sol["elapsed_ms"] > 5000.0:
                    cell_errors.append(f"late solution counted in {arm_name} solve {k}")
                sizes[seed] = size
                best = size if best is None else max(best, size)
            if arm["best_size"] != best:
                cell_errors.append(f"best size mismatch in {arm_name}")
            by_seed[arm_name] = sizes
        base_best = row["baseline"]["best_size"]
        cand_best = row["candidate"]["best_size"]
        delta = cand_best - base_best if base_best is not None and cand_best is not None else None
        if row["delta"] != delta:
            cell_errors.append("paired delta mismatch")
        if not row["valid"]:
            cell_errors.append("harness marked cell invalid")
        if cell_errors:
            family[name]["failures"] += 1
            invalid_cells.append({"graph": name, "campaign": campaign, "reasons": cell_errors})
            continue
        for seed in set(by_seed["baseline"]) & set(by_seed["candidate"]):
            paired_seed_differences.append(by_seed["candidate"][seed] - by_seed["baseline"][seed])
        family[name]["differences"].append(delta)
        if delta > 0:
            wins += 1
            family[name]["wins"] += 1
        elif delta < 0:
            losses += 1
            family[name]["losses"] += 1
        else:
            ties += 1
            family[name]["ties"] += 1
    win_families = sum(group["wins"] > 0 for group in family.values())
    decision = ("GO" if not invalid_cells and wins >= 5 and losses <= 1
                and win_families >= 2 else "NO-GO")
    return {"cells": len(rows), "wins": wins, "ties": ties, "losses": losses,
            "win_graphs": win_families, "decision": decision,
            "invalid_cells": invalid_cells,
            "by_graph": dict(family), "completed": completed, "late_completions": late,
            "same_seed_pairs": len(paired_seed_differences),
            "same_seed_wins": sum(x > 0 for x in paired_seed_differences),
            "same_seed_ties": sum(x == 0 for x in paired_seed_differences),
            "same_seed_losses": sum(x < 0 for x in paired_seed_differences)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", type=Path, required=True)
    parser.add_argument("--data-dir", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(analyze(args.raw, args.data_dir), indent=2, sort_keys=True))
