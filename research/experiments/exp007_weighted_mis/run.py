#!/usr/bin/env python3
"""One-shot frozen EXP-007W run with pre-run input hash and provenance."""

import argparse
import hashlib
import json
import os
import platform
import shlex
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


EXPERIMENT_ID = "EXP-007W"
INPUT_SHA256 = "3c901eaaa0098cc273427e6942227fb5c0c5010f459423e5686819d94878e520"
SOURCE_PATHS = (
    "src/bin/exp007_weighted_mis.rs",
    "research/experiments/exp007_weighted_mis/protocol.md",
    "research/experiments/exp007_weighted_mis/instances.json",
    "research/experiments/exp007_weighted_mis/generate_instances.py",
    "research/experiments/exp007_weighted_mis/analyze.py",
    "research/experiments/exp007_weighted_mis/test_tools.py",
    "research/experiments/exp007_weighted_mis/run.py",
    "research/experiments/exp007_weighted_mis/commands.sh",
)


def command_output(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def now():
    return datetime.now(timezone.utc).isoformat()


def cpu_name():
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("model name"):
            return line.split(":", 1)[1].strip()
    return platform.processor()


def ram_bytes():
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("cannot read total RAM")


def verify_input(path, expected=INPUT_SHA256):
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != expected:
        raise ValueError(f"frozen input hash mismatch: {digest}")
    return digest


def run(output):
    root = Path(__file__).resolve().parents[3]
    data = Path(__file__).with_name("instances.json")
    digest = verify_input(data)
    for source in SOURCE_PATHS:
        subprocess.run(["git", "ls-files", "--error-unmatch", source], cwd=root,
                       stdout=subprocess.DEVNULL, check=True)
    subprocess.run(["git", "diff", "--quiet", "HEAD", "--", *SOURCE_PATHS],
                   cwd=root, check=True)
    binary = root / "target/release/exp007_weighted_mis"
    if not binary.is_file():
        raise FileNotFoundError(binary)
    binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    command = [str(binary), "--data", str(data),
               "--structural", str(output / "structural.jsonl"),
               "--raw", str(output / "raw.jsonl")]
    commit = command_output("git", "rev-parse", "HEAD", cwd=root)
    metadata = {
        "experiment_id": EXPERIMENT_ID,
        "git_commit": commit,
        "git_dirty": bool(command_output("git", "status", "--porcelain", cwd=root)),
        "source_paths_clean": True,
        "binary_sha256": binary_sha256,
        "benchmark_dataset": {
            "name": "EXP-007W deterministic vertex-weighted ER graphs",
            "git_commit": command_output("git", "rev-parse", "08f24a9", cwd=root),
            "url": "research/experiments/exp007_weighted_mis/instances.json",
            "sha256": digest,
        },
        "environment": {
            "os": platform.platform(),
            "cpu": cpu_name(),
            "logical_cpus": os.cpu_count(),
            "ram_bytes": ram_bytes(),
            "rust_version": command_output("rustc", "--version"),
            "profile": "release",
            "rustflags": os.environ.get("RUSTFLAGS", ""),
            "threads": os.cpu_count(),
            "env_vars": {"RAYON_NUM_THREADS": os.environ.get("RAYON_NUM_THREADS")},
        },
        "execution": {
            "launcher_command_line": "bash research/experiments/exp007_weighted_mis/commands.sh <new-output-directory>",
            "command_line": shlex.join(command),
            "seeds": [7_000_000 + 100_000 * g + 10_000 * c
                      for g in range(6) for c in range(10)],
            "structural_seeds": [9_000_000 + 1_000 * g + s
                                 for g in range(6) for s in range(10)],
            "solve_seed_rule": "7_000_000 + 100_000*g + 10_000*c + k",
            "timeout_seconds": 2.0,
            "start_time_iso": now(),
            "end_time_iso": None,
            "exit_code": None,
        },
    }
    metadata_path = output / "metadata.json"
    metadata_path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
    (output / "README.md").write_text(
        "# EXP-007W run artifacts\n\n"
        "Protocol: `../protocol.md`. Inputs: `../instances.json`. "
        "Structural probes: `structural.jsonl`; timed cells: `raw.jsonl`; "
        "independent summary: `analysis.json`; environment: `metadata.json`.\n\n"
        "From repository root, run `bash research/experiments/exp007_weighted_mis/commands.sh "
        "<new-output-directory>` to replay. The run uses wall-clock deadlines, so "
        "timing and best-found weights can vary with machine load; independent "
        "feasibility and energy checks must always pass.\n"
    )
    with (output / "console.log").open("w") as log:
        process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True, bufsize=1)
        for line in process.stdout:
            print(line, end="", flush=True)
            log.write(line)
            log.flush()
        returncode = process.wait()
    metadata["execution"]["end_time_iso"] = now()
    metadata["execution"]["exit_code"] = returncode
    metadata_path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
    if (output / "structural.jsonl").exists() and (output / "raw.jsonl").exists():
        analyzer = [sys.executable, str(Path(__file__).with_name("analyze.py")),
                    "--structural", str(output / "structural.jsonl"),
                    "--raw", str(output / "raw.jsonl"), "--data", str(data)]
        analysis = subprocess.run(analyzer, cwd=root, text=True, capture_output=True)
        (output / "analysis.stdout").write_text(analysis.stdout)
        (output / "analysis.stderr").write_text(analysis.stderr)
        if analysis.returncode == 0:
            (output / "analysis.json").write_text(analysis.stdout)
    if returncode:
        raise RuntimeError(f"instrument exited {returncode}; artifacts retained in {output}")
    if not (output / "analysis.json").exists():
        raise RuntimeError(f"independent analyzer failed; artifacts retained in {output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    run(parser.parse_args().output)
