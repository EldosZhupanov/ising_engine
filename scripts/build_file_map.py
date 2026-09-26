#!/usr/bin/env python3
"""Emit a reproducible source/research inventory without reading generated trees."""

from __future__ import annotations

import csv
import hashlib
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = "memory/FILE_MAP.tsv"
IGNORED_EVIDENCE_ROOTS = (
    "archive",
    "experiments",
    "research/experiments",
    "benchmark_suite/data",
    "benchmark_suite/results",
)


def paths_from_command(*args: str) -> set[str]:
    output = subprocess.run(
        args, cwd=ROOT, check=True, capture_output=True
    ).stdout
    return {part.decode("utf-8", "surrogateescape") for part in output.split(b"\0") if part}


def catalogued_markdown() -> dict[str, tuple[str, str]]:
    rows: dict[str, tuple[str, str]] = {}
    pattern = re.compile(r"^\| `([^`]+\.md)` \|[^|]+\|[^|]+\|\s*([^|]+)\s*\|")
    for line in (ROOT / "memory/CATALOG.md").read_text().splitlines():
        match = pattern.match(line)
        if match:
            rows[match.group(1)] = (match.group(2).strip(), "catalogued")
    return rows


def project_for(path: str) -> str:
    if path.startswith("research/fundamental_ai/"):
        return "fundamental_ai"
    if path.startswith("research/laya_semantic/"):
        return "laya_semantic"
    if path.startswith("research/labs_qualification/") or "labs" in path.lower():
        return "LABS"
    if "marketsplit" in path.lower() or "market_split" in path.lower():
        return "MarketSplit"
    if path.startswith("src/engine_v2/"):
        return "engine_v2"
    if path.startswith(("src/", "tests/", "benches/")):
        return "ising_engine"
    if path.startswith("website/"):
        return "website"
    if path.startswith(("benchmark_suite/", "benchmarks/", "gset/")):
        return "benchmark_hub"
    if path.startswith("research/") or path.startswith("results/"):
        return "research_program"
    if path.startswith("experiments/"):
        return "generated_experiments"
    if path.startswith("archive/"):
        return "archive"
    if path.startswith((".agents/", ".claude/")):
        return "agent_tooling"
    if path.startswith("memory/") or path.endswith(".md"):
        return "project_governance"
    return "miscellaneous"


def kind_for(path: str) -> str:
    if path.endswith(".md"):
        return "document"
    if path.endswith((".rs", ".py", ".ts", ".tsx", ".mjs", ".sh")):
        return "source_or_test"
    if path.endswith((".dat", ".gph", ".rudy", ".mc")):
        return "instance"
    if path.endswith(".sol"):
        return "candidate_or_witness"
    if path.endswith((".tsv", ".csv", ".json", ".jsonl", ".txt", ".log")):
        return "data_or_result"
    return "asset_or_config"


def lifecycle_for(path: str, tracking: str, catalog: dict[str, tuple[str, str]]) -> str:
    if tracking == "deleted_tracked":
        return "SUPERSEDED"
    if path in {
        "src/core/lattice.rs",
        "src/bin/qoblib_unsolved_marketsplit_hunter.rs",
        "src/bin/qoblib_lll_screening.rs",
        "src/bin/qoblib_lambda_lattice_solver.rs",
        "src/bin/qoblib_lattice_ultimate_hybrid.rs",
        "tests/test_lattice.rs",
    }:
        return "UNKNOWN"  # tracked experimental code, not qualified solver evidence
    if path in catalog:
        return {
            "active": "CURRENT",
            "binding": "CURRENT",
            "closed": "HISTORICAL",
            "historical": "HISTORICAL",
            "proposed": "UNKNOWN",
            "reference": "CURRENT",
            "superseded": "SUPERSEDED",
        }.get(catalog[path][0], "UNKNOWN")
    if path.startswith("benchmarks/qoblib/marketsplit/rejected_candidates/"):
        return "FAILED"
    if path.startswith("benchmarks/qoblib/world_records/") and path.endswith(".sol"):
        return "CURRENT"
    if tracking == "ignored_evidence" or path.startswith("archive/"):
        return "HISTORICAL"
    if tracking == "untracked":
        return "UNKNOWN"
    if path.startswith(("src/", "tests/", "benchmark_suite/ising_bench/")):
        return "CURRENT"
    return "UNKNOWN"


def evidence_for(path: str) -> str:
    if path.startswith("research/experiments/hubo_corpus_qualification/"):
        return "HUBO-Q002 Phase A: 24 exact reduction certificates; Phase B NOT RUN; see RESULT_PHASE_A.md"
    if path.startswith("research/experiments/hubo_comparison/") or path == "research/examples/hubo_compare.rs":
        return "HUBO-C001: 400 valid cells; native representation benefit scoped; 100 native OpenJij ties; see RESULT.md"
    if path.startswith("research/experiments/hubo_representation_gate/") or path == "tests/test_hubo_representation.rs":
        return "HUBO-RG001 exact gate PASS; performance untested; see RESULT.md"
    if path.startswith("research/experiments/labs_q002/"):
        return "Q002 NOT_QUALIFIED; independent 60-witness/881-incumbent audit PASS; see RESULT.md"
    if path.startswith("benchmarks/qoblib/marketsplit/solutions/") and not (ROOT / path).exists():
        return "checker-rejected file moved to rejected_candidates; see PROJECTS.md"
    if path.startswith("benchmarks/qoblib/marketsplit/rejected_candidates/"):
        return "checker exit 21; see verification.txt"
    if path.startswith("benchmarks/qoblib/marketsplit/solutions/") and path.endswith(".sol"):
        return "check_marketsplit exit 0 in 2026-09-25 audit"
    if path.startswith("benchmarks/qoblib/world_records/") and path.endswith(".sol"):
        return "independent autocorrelation energy matched; checker optimum table absent for N>=67"
    if path == "benchmarks/qoblib/upstream/":
        return "incomplete external clone; broken Git HEAD; not a benchmark source"
    if path.startswith("results/rc027/"):
        return "raw hash matched; see RESULT.md"
    if path.startswith("research/breakthrough/exp001/"):
        return "raw/target hashes matched; four derived outputs replayed byte-for-byte"
    if path.startswith("benchmarks/qoblib/marketsplit/unsolved_instances/"):
        return "official byte match; contains public # Solution label"
    if path == "research/fundamental_ai/EXP_TEN_006_AUDIT_RAW.tsv":
        return "3360 rows; all non-timing fields replayed from current source; see EXP006A_RAW_RECHECK.md"
    if path.startswith("research/fundamental_ai/") and path.endswith(".rs"):
        return "standalone cargo test passed; claim not assessed"
    if path.startswith(("src/", "tests/")):
        return "workspace cargo test --release passed; claim not assessed"
    return "not independently reviewed in this audit"


def sha256(path: Path) -> str:
    if not path.is_file():
        return ""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> None:
    tracked = paths_from_command("git", "ls-files", "-z")
    untracked = paths_from_command("git", "ls-files", "--others", "--exclude-standard", "-z")
    ignored_roots = tuple(root for root in IGNORED_EVIDENCE_ROOTS if (ROOT / root).exists())
    ignored = (
        paths_from_command("rg", "--files", "--hidden", "--no-ignore", "-0", *ignored_roots)
        if ignored_roots
        else set()
    ) - tracked - untracked
    all_paths = (tracked | untracked | ignored) - {MANIFEST}
    catalog = catalogued_markdown()
    writer = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
    writer.writerow(("path", "tracking", "project", "kind", "lifecycle", "sha256", "evidence", "byte_duplicate_of"))
    first_by_hash: dict[str, str] = {}
    for path in sorted(all_paths):
        full_path = ROOT / path
        tracking = (
            "deleted_tracked" if path in tracked and not full_path.exists()
            else "tracked" if path in tracked
            else "untracked" if path in untracked
            else "ignored_evidence"
        )
        digest = sha256(full_path)
        duplicate = first_by_hash.setdefault(digest, path) if digest else ""
        writer.writerow((
            path,
            tracking,
            project_for(path),
            kind_for(path),
            lifecycle_for(path, tracking, catalog),
            digest,
            evidence_for(path),
            duplicate if duplicate and duplicate != path else "-",
        ))


if __name__ == "__main__":
    main()
