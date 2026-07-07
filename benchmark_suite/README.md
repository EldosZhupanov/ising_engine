# Benchmark Hub

Reproducible acquisition, verification, and loading of public Ising / QUBO /
MaxCut benchmark datasets for the Ising Engine. Pure Python stdlib — no pip,
no third-party packages.

## One command

```bash
python3 benchmark_suite/scripts/download_all_benchmarks.py
```

Downloads every registered dataset with mirror failover, unpacks archives,
normalizes filenames, writes SHA256 manifests (`metadata.json`), and creates
a per-dataset `README.md` with origin, license, and citation. Re-runs are
incremental. Useful flags: `--only gset,orlib`, `--limit N`, `--force`,
`--list`, `--validate`.

After one successful run everything works **fully offline**.

## Datasets

| name | kind | contents |
|---|---|---|
| `gset` | MaxCut | Stanford G-Set (rudy graphs, n=800…20000) |
| `orlib` | QUBO | Beasley OR-Library BQP (n=50…2500, multi-problem files) |
| `biqmac` | mixed | Biq Mac library (rudy/ising MaxCut + GKA/Beasley sparse BQP) |
| `qplib` | QUBO | QPLIB unconstrained all-binary instances (23, with best-known objectives from the official index) |
| `dimacs_maxcut` | MaxCut | Optsicom MaxCut sets (incl. DIMACS torus / spin-glass) |
| `snap` | MaxCut | SNAP real-world graphs as unweighted MaxCut |

The registry (`configs/benchmark_registry.yaml`) is the single source of
truth: license, citation, format, parser, official URL, and mirrors per
dataset. Add a dataset by adding a registry entry — no code changes needed
unless it introduces a new file format.

## Python API

```python
import sys; sys.path.insert(0, "benchmark_suite")
from ising_bench import load_dataset, list_datasets, validate_all

graphs = load_dataset("gset")            # Graph objects
qubos  = load_dataset("gset", as_qubo=True)  # coerced to minimization QUBOs
inst   = qubos[0]
inst.energy(x)          # E(x) = offset + Σ l_i x_i + Σ_{i<j} q_ij x_i x_j
inst.maximize           # True when the native objective is -E (MaxCut, BQP)
inst.best_known         # native-units optimum, when published (QPLIB)
inst.meta               # dataset, file, source_url, sha256, license, citation
```

Energy conventions match the Rust engine (`src/benchmark/instances.rs`)
exactly; the shared toy-instance tests in `scripts/selftest.py` and
`tests/test_benchmark.rs` pin the mapping in both languages.

## Layout

```
benchmark_suite/
├── configs/benchmark_registry.yaml   # dataset registry (source of truth)
├── scripts/download_all_benchmarks.py# the single command
├── scripts/selftest.py               # offline parser/convention tests
├── ising_bench/                      # stdlib-only package
│   ├── __init__.py    load_dataset / list_datasets / validate_*
│   ├── models.py      Graph / Qubo standardized objects
│   ├── parsers.py     rudy, DIMACS, SNAP, ORLIB, Biq Mac sparse, QPLIB
│   ├── download.py    mirrors, verify, unpack, hash, metadata
│   ├── validators.py  hash + re-parse audits (validation_report.json)
│   ├── registry.py    registry access + fail-fast schema check
│   └── miniyaml.py    YAML-subset loader (environment has no PyYAML)
└── data/                             # downloaded (gitignored)
```

## Verification

- `scripts/selftest.py` — parsers and energy conventions against tiny
  instances with known optima (mirrors the Rust test suite).
- `--validate` — re-hashes every downloaded file against its manifest and
  re-parses it with its registered parser; writes
  `data/validation_report.json`.

## Citations

Please cite the original sources when publishing results; each dataset's
`data/<name>/README.md` and the registry carry the full reference:
Helmberg & Rendl 2000 (G-Set), Beasley 1990 (OR-Library), Wiegele 2007
(Biq Mac), Furini et al. 2019 (QPLIB), Martí, Duarte & Laguna 2009
(Optsicom MaxCut), Leskovec & Krevl 2014 (SNAP).
