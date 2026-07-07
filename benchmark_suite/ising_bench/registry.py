"""Access to the benchmark registry (configs/benchmark_registry.yaml)."""

from __future__ import annotations

from pathlib import Path

from . import miniyaml

_SUITE_ROOT = Path(__file__).resolve().parent.parent
REGISTRY_PATH = _SUITE_ROOT / "configs" / "benchmark_registry.yaml"
DEFAULT_DATA_DIR = _SUITE_ROOT / "data"

_cache = None


def load() -> dict:
    """The parsed registry (cached)."""
    global _cache
    if _cache is None:
        _cache = miniyaml.load_file(REGISTRY_PATH)
        _validate(_cache)
    return _cache


def dataset_names() -> list:
    return list(load()["datasets"].keys())


def dataset(name: str) -> dict:
    ds = load()["datasets"].get(name)
    if ds is None:
        raise KeyError(
            f"unknown dataset {name!r}; registered: {', '.join(dataset_names())}"
        )
    return ds


_REQUIRED = ("description", "kind", "license", "citation", "format", "parser",
             "official_url", "strategy")


def _validate(regy: dict) -> None:
    """Fail fast on a malformed registry rather than mid-download."""
    from .parsers import PARSERS

    datasets = regy.get("datasets")
    if not isinstance(datasets, dict) or not datasets:
        raise ValueError("registry: missing 'datasets' mapping")
    for name, ds in datasets.items():
        for key in _REQUIRED:
            if key not in ds:
                raise ValueError(f"registry: {name} missing required key {key!r}")
        if ds["parser"] not in PARSERS:
            raise ValueError(f"registry: {name} names unknown parser {ds['parser']!r}")
        strat = ds["strategy"]
        if strat == "files" and not (ds.get("files") and ds.get("url_templates")):
            raise ValueError(f"registry: {name} strategy 'files' needs files+url_templates")
        if strat == "archive":
            for a in ds.get("archives") or []:
                if not (a.get("name") and a.get("urls")):
                    raise ValueError(f"registry: {name} archive entries need name+urls")
                if a.get("parser") and a["parser"] not in PARSERS:
                    raise ValueError(
                        f"registry: {name} archive {a['name']} unknown parser {a['parser']!r}"
                    )
        if strat == "qplib_index" and not (
            ds.get("index_url") and ds.get("instance_url_template")
        ):
            raise ValueError(
                f"registry: {name} strategy 'qplib_index' needs index_url+instance_url_template"
            )
