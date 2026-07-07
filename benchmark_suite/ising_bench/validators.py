"""Dataset validators.

Two levels:

- structural (`Graph.validate` / `Qubo.validate`, invoked by every parser) —
  index ranges, finite weights, shape consistency;
- dataset-level (`validate_dataset`) — re-hash each file against the SHA256
  manifest and re-parse it, plus per-format sanity checks (declared edge
  counts, connectivity of the id remap, etc.).

`validate_all` produces a machine-readable report for CI or audits.
"""

from __future__ import annotations

import json
from pathlib import Path

from . import registry as reg
from .download import verify_dataset


def validate_dataset(name: str, data_dir: Path = None) -> dict:
    """Validate one downloaded dataset. Returns a report dict with
    ``status`` (clean / problems / missing) and the problem list."""
    data_dir = Path(data_dir) if data_dir else reg.DEFAULT_DATA_DIR
    problems = verify_dataset(name, data_dir)
    missing = bool(problems) and "no metadata.json" in problems[0]
    return {
        "dataset": name,
        "status": "missing" if missing else ("clean" if not problems else "problems"),
        "problems": problems,
    }


def validate_all(data_dir: Path = None, log=print) -> list:
    """Validate every registered dataset; writes ``validation_report.json``
    into the data directory and returns the report list."""
    data_dir = Path(data_dir) if data_dir else reg.DEFAULT_DATA_DIR
    reports = []
    for name in reg.dataset_names():
        rep = validate_dataset(name, data_dir)
        log(f"{name}: {rep['status']}" + (f" ({len(rep['problems'])})" if rep["problems"] else ""))
        for p in rep["problems"][:10]:
            log(f"  - {p}")
        reports.append(rep)
    out = Path(data_dir) / "validation_report.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", encoding="utf-8") as fh:
        json.dump(reports, fh, indent=2)
    return reports
