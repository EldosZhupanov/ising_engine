"""Ising Engine Benchmark Hub — reproducible acquisition and loading of
public Ising/QUBO/MaxCut benchmark datasets.

Typical use::

    # once, with network:
    #   python3 benchmark_suite/scripts/download_all_benchmarks.py

    from ising_bench import load_dataset, list_datasets

    for inst in load_dataset("gset"):          # Graph objects
        qubo = inst.to_qubo()                  # standardized minimization QUBO
        ...

    for qubo in load_dataset("orlib"):         # Qubo objects (maximize flag set)
        ...

Everything after download works fully offline; loading touches only the
local data directory and the SHA256-verified files listed in each dataset's
``metadata.json``. Pure stdlib — no third-party dependencies.
"""

from __future__ import annotations

import csv
import json
from pathlib import Path

from . import registry as reg
from .models import Graph, Qubo  # noqa: F401 (public API)
from .parsers import PARSERS
from .validators import validate_all, validate_dataset  # noqa: F401 (public API)

__all__ = [
    "load_dataset",
    "list_datasets",
    "dataset_info",
    "Graph",
    "Qubo",
    "validate_dataset",
    "validate_all",
]


def list_datasets() -> list:
    """Names of every registered dataset."""
    return reg.dataset_names()


def dataset_info(name: str) -> dict:
    """Registry entry (license, citation, format, URLs) for a dataset."""
    return dict(reg.dataset(name))


def _best_known(dataset_dir: Path) -> dict:
    """Optional per-instance best-known objectives (native units)."""
    path = dataset_dir / "best_known.csv"
    out = {}
    if path.exists():
        with open(path, newline="", encoding="utf-8") as fh:
            for row in csv.DictReader(fh):
                try:
                    out[row["instance"]] = float(row["best_known_native"])
                except (KeyError, ValueError):
                    continue
    return out


def load_dataset(name: str, data_dir=None, instances=None, as_qubo=False) -> list:
    """Load a downloaded dataset as standardized objects.

    Args:
        name: registered dataset name (see ``list_datasets()``).
        data_dir: override the default ``benchmark_suite/data`` directory.
        instances: optional iterable of file names to load (subset).
        as_qubo: coerce every instance to a minimization ``Qubo``.

    Returns:
        list of ``Graph``/``Qubo`` objects, each carrying provenance in
        ``.meta`` (dataset, file, source_url, sha256).

    Raises:
        FileNotFoundError: dataset not downloaded yet.
    """
    ds = reg.dataset(name)
    dataset_dir = (Path(data_dir) if data_dir else reg.DEFAULT_DATA_DIR) / name
    meta_path = dataset_dir / "metadata.json"
    if not meta_path.exists():
        raise FileNotFoundError(
            f"dataset {name!r} is not downloaded; run "
            f"benchmark_suite/scripts/download_all_benchmarks.py --only {name}"
        )
    with open(meta_path, encoding="utf-8") as fh:
        meta = json.load(fh)
    wanted = set(instances) if instances else None
    best = _best_known(dataset_dir)
    out = []
    for rec in meta["files"]:
        fname = rec["file"]
        if wanted and fname not in wanted:
            continue
        parser = PARSERS[rec["parser"]]
        with open(dataset_dir / fname, encoding="utf-8", errors="replace") as fh:
            parsed = parser(fh.read(), fname)
        for inst in parsed if isinstance(parsed, list) else [parsed]:
            inst.meta.update(
                dataset=name,
                file=fname,
                source_url=rec.get("source_url"),
                sha256=rec.get("sha256"),
                license=ds.get("license"),
                citation=ds.get("citation"),
            )
            if fname in best and isinstance(inst, Qubo):
                inst.best_known = best[fname]
            out.append(inst.to_qubo() if as_qubo else inst)
    return out
