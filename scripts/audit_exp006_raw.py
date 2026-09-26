#!/usr/bin/env python3
"""Independently summarize EXP-TEN-006A-R rows without model code dependencies."""

from __future__ import annotations

import argparse
import csv
import hashlib
from collections import defaultdict
from pathlib import Path
from statistics import mean


DEFAULT_RAW = (
    Path(__file__).resolve().parent.parent
    / "research/fundamental_ai/EXP_TEN_006_AUDIT_RAW.tsv"
)
METRICS = ("mean_accuracy", "rescue_rate", "damage_rate", "net_rescue", "exact_solve")


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="") as stream:
        reader = csv.DictReader(stream, delimiter="\t")
        rows = list(reader)
    required = {"cycle_k", "noise", "step", "seed", "instance_id", "model", *METRICS}
    if not rows or not required.issubset(rows[0]):
        raise ValueError("missing rows or required TSV columns")
    keys = [
        (r["cycle_k"], r["noise"], r["step"], r["seed"], r["model"])
        for r in rows
    ]
    if len(set(keys)) != len(keys):
        raise ValueError("duplicate condition/seed/model row")
    dimensions = {
        field: {r[field] for r in rows}
        for field in ("cycle_k", "noise", "step", "seed", "model")
    }
    expected = 1
    for values in dimensions.values():
        expected *= len(values)
    if expected != len(rows):
        raise ValueError(f"incomplete Cartesian design: {len(rows)} rows, expected {expected}")
    for row in rows:
        for metric in METRICS:
            value = float(row[metric])
            if not 0 <= value <= 1 and metric != "net_rescue":
                raise ValueError(f"out-of-range {metric}: {value}")
        if row["exact_solve"] not in ("0.0000", "1.0000"):
            raise ValueError("exact_solve must be Boolean")
    return rows


def summarize(path: Path) -> None:
    rows = read_rows(path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    print(f"sha256={digest} rows={len(rows)}")
    dimensions = {
        field: sorted({r[field] for r in rows})
        for field in ("cycle_k", "noise", "step", "seed", "model")
    }
    print("dimensions=" + " ".join(f"{k}:{len(v)}" for k, v in dimensions.items()))
    print(f"cartesian_expected={len(rows)} complete=True")

    slice_rows = [
        r for r in rows if r["cycle_k"] == "16" and r["noise"] == "0.20" and r["step"] == "16"
    ]
    by_model: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in slice_rows:
        by_model[row["model"]].append(row)
    for model, sample in sorted(by_model.items()):
        metrics = " ".join(f"{m}={mean(float(r[m]) for r in sample):.5f}" for m in METRICS)
        print(f"K16_T16 model={model} n={len(sample)} {metrics}")

    exact_rows = [r for r in rows if float(r["exact_solve"]) == 1]
    exact_graphs = {
        (r["model"], r["cycle_k"], r["noise"], r["seed"], r["instance_id"])
        for r in exact_rows
    }
    print(f"exact_positive_rows={len(exact_rows)} distinct_model_graphs={len(exact_graphs)}")
    for graph in sorted(exact_graphs):
        print("exact_graph=" + ",".join(graph))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", nargs="?", type=Path, default=DEFAULT_RAW)
    summarize(parser.parse_args().raw)
