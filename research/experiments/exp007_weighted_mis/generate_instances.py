#!/usr/bin/env python3
"""Deterministic EXP-007W weighted-MIS instances; no solver access."""

import hashlib
import json
from pathlib import Path


SPECS = ((64, 8), (64, 16), (96, 8), (96, 16), (128, 8), (128, 16))


def draw(label):
    return int.from_bytes(hashlib.sha256(label.encode("ascii")).digest()[:8], "big")


def make_instances():
    instances = []
    for index, (n, edge_percent) in enumerate(SPECS):
        weights = [1 + draw(f"EXP007W-v1-w-{index}-{v}") % 10 for v in range(n)]
        edges = [
            [u, v]
            for u in range(n)
            for v in range(u + 1, n)
            if draw(f"EXP007W-v1-e-{index}-{u}-{v}") % 100 < edge_percent
        ]
        instances.append({
            "name": f"er_n{n}_p{edge_percent:02d}_g{index}",
            "n": n,
            "edge_percent": edge_percent,
            "weights": weights,
            "edges": edges,
        })
    return {"experiment_id": "EXP-007W", "generator": "SHA-256-v1", "instances": instances}


if __name__ == "__main__":
    target = Path(__file__).with_name("instances.json")
    encoded = json.dumps(make_instances(), sort_keys=True, separators=(",", ":")) + "\n"
    if target.exists() and target.read_text() != encoded:
        raise SystemExit("existing instances.json differs from deterministic generator")
    target.write_text(encoded)
    print(hashlib.sha256(encoded.encode()).hexdigest(), target)
