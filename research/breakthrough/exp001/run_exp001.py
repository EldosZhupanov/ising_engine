"""Run the preregistered experiment once; no optional tuning switches."""
from pathlib import Path
import argparse
import csv
import hashlib
import json
import os
import platform
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
BASELINE = "fdec0df"
PREREG = "aee6ea8"
FAMILIES = ("cycle", "subdivided", "sparse", "dense", "field")
SIZES = (16, 64, 128)
INSTANCE_SEEDS = (1101, 1102, 1103)
SOLVER_SEEDS = (2101, 2102, 2103, 2104, 2105)
BUILD = Path(tempfile.gettempdir()) / "ising-breakthrough-exp001-build"
OUTPUT = HERE / "exp001"
ENV = {**os.environ, "RAYON_NUM_THREADS": "1"}
HEADER = "mode family n instance_seed solver_seed arm exchanges fingerprint energy target success time_target_ms time_best_ms elapsed_ms prep_ms solves residual_n residual_m table_evals degree_scans delta_evals gain_scans pair_queries neighbor_updates pair_moves single_moves pre_common_best common_improvement_sum pair_improvement_sum nominal_spin_proposals_upper prep_overrun state generator_ms audit_ms common_delta_evals common_gain_scans common_neighbor_updates common_single_moves observations".split()
TARGET_HEADER = "family n instance_seed fingerprint target state target_kind target_build_ms".split()
RESOURCE_HEADER = "process_elapsed_s process_user_s process_system_s peak_rss_kib launch_to_return_ms".split()


def command(args, **kwargs):
    return subprocess.run(args, cwd=ROOT, env=ENV, check=True, **kwargs)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def frozen_sources():
    command(["git", "merge-base", "--is-ancestor", PREREG, "HEAD"])
    frozen = command(["git", "show", f"{PREREG}:research/breakthrough/EXP001_PROTOCOL.md"], capture_output=True).stdout
    if frozen != (HERE / "EXP001_PROTOCOL.md").read_bytes():
        raise RuntimeError("preregistration bytes differ")
    command(["git", "diff", "--exit-code", BASELINE, "--", "src", "Cargo.toml", "Cargo.lock", "research/Cargo.toml"])
    if not Path("/usr/bin/time").is_file():
        raise RuntimeError("/usr/bin/time is required for CPU and peak RSS")


def build_and_check():
    frozen_sources()
    BUILD.mkdir(exist_ok=True)
    command(["cargo", "build", "--release", "--lib"])
    # Pin the exact library used for every child; source tree and binary hash are recorded.
    shutil.copyfile(ROOT / "target/release/libising_engine.rlib", BUILD / "libbaseline.rlib")
    args = ["--edition=2021", "-O", "-D", "warnings", str(HERE / "probe.rs"), "--extern", f"ising_engine={BUILD / 'libbaseline.rlib'}", "-L", f"dependency={ROOT / 'target/release/deps'}"]
    command(["rustfmt", "--edition", "2021", "--check", str(HERE / "probe.rs")])
    command(["rustc", *args, "--test", "-o", str(BUILD / "tests")])
    tests = command([str(BUILD / "tests")], text=True, capture_output=True)
    print(tests.stdout, end="", flush=True)
    command(["rustc", *args, "-o", str(BUILD / "probe")])
    # Synthetic, non-registered smoke instance: checks CLI framing and null/censor fields.
    smoke = command([str(BUILD / "probe"), "run", "fixed", "field", "8", "71", "83", "0", "1", "-9999"], text=True, capture_output=True)
    row = smoke.stdout.strip().split("\t")
    if len(row) != len(HEADER) or row[11] != "NA" or len(row[31]) != 8:
        raise RuntimeError(f"CLI output invalid: {smoke.stdout}")
    command(["python3", str(HERE / "analyze_exp001.py"), "--selftest"])
    assert len(list(design())) == 4200
    return tests.stdout


def design():
    for mode in ("fixed", "wall"):
        for family in FAMILIES:
            for n in (SIZES if mode == "fixed" else (128,)):
                for instance_seed in INSTANCE_SEEDS:
                    for solver_seed in SOLVER_SEEDS:
                        for exchanges in ((1, 8) if mode == "fixed" else (1,)):
                            offset = (solver_seed + exchanges) % 8
                            arms = [(i + offset) % 8 for i in range(8)]
                            if (instance_seed + solver_seed) % 2:
                                arms.reverse()
                            for arm in arms:
                                yield mode, family, n, instance_seed, solver_seed, arm, exchanges


def run():
    # Refusal before compilation or any new oracle/row. No accidental reruns.
    if OUTPUT.exists():
        raise RuntimeError(f"refusing existing output directory: {OUTPUT}")
    tests = build_and_check()
    OUTPUT.mkdir()  # also makes simultaneous invocations fail safely
    (OUTPUT / "tests.txt").write_text(tests)
    for name in ("probe.rs", "run_exp001.py", "analyze_exp001.py"):
        shutil.copyfile(HERE / name, OUTPUT / name)
    source_paths = [HERE / n for n in ("probe.rs", "run_exp001.py", "analyze_exp001.py", "EXP001_PROTOCOL.md")]
    source_paths += [BUILD / "libbaseline.rlib", BUILD / "probe"]
    (OUTPUT / "source.sha256").write_text("".join(f"{digest(p)}  {p}\n" for p in source_paths))
    environment = {"baseline_source": BASELINE, "prereg_commit": PREREG,
                   "instrument_commit": command(["git", "rev-parse", "HEAD"], text=True, capture_output=True).stdout.strip(),
                   "platform": platform.platform(), "python": platform.python_version(), "rayon_threads": 1,
                   "rustc": command(["rustc", "--version"], text=True, capture_output=True).stdout.strip(),
                   "cpu": command(["lscpu"], text=True, capture_output=True).stdout,
                   "resource_scope": "child process including generation/audit; GNU time user/system have 0.01s display resolution, peak RSS includes runtime/model",
                   "time_scope": "algorithm includes preprocessing, solve, lifting, common restoration and C; excludes process launch, generator and final audit",
                   "timing_resolution": "exploratory only; prior calibration is not renewed by this run",
                   "target_times": "observed upper bounds after each complete atomic solve/lift/common/C unit; NA is right censoring",
                   "compute_limit": "nominal spin proposals exclude presolve, finishing and kernel work; not exact total evaluation count"}
    (OUTPUT / "environment.txt").write_text(json.dumps(environment, indent=2))
    targets = {}
    with (OUTPUT / "targets.tsv").open("w") as f:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n"); writer.writerow(TARGET_HEADER)
        for family in FAMILIES:
            for n in SIZES:
                for seed in INSTANCE_SEEDS:
                    result = command([str(BUILD / "probe"), "target", family, str(n), str(seed)], text=True, capture_output=True)
                    fields = result.stdout.strip().split("\t")
                    if len(fields) != len(TARGET_HEADER):
                        raise RuntimeError("target record malformed")
                    targets[family, n, seed] = dict(zip(TARGET_HEADER, fields))
                    writer.writerow(fields); f.flush()
    # Targets now fixed and persisted before any candidate run.
    with (OUTPUT / "raw.tsv").open("w") as f, (OUTPUT / "run.log").open("w") as log:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n"); writer.writerow(HEADER + RESOURCE_HEADER)
        resource_file = OUTPUT / "last_resource.txt"
        for index, (mode, family, n, iseed, sseed, arm, exchanges) in enumerate(design(), 1):
            target = targets[family, n, iseed]
            args = ["/usr/bin/time", "-f", "%e\t%U\t%S\t%M", "-o", str(resource_file), str(BUILD / "probe"), "run", mode, family, str(n), str(iseed), str(sseed), str(arm), str(exchanges), target["target"]]
            t0 = time.perf_counter()
            try:
                result = command(args, text=True, capture_output=True)
                launch_ms = (time.perf_counter() - t0) * 1000
                fields = result.stdout.strip().split("\t")
                if len(fields) != len(HEADER):
                    raise RuntimeError(f"row width {len(fields)} != {len(HEADER)}")
                row = dict(zip(HEADER, fields))
                if row["fingerprint"] != target["fingerprint"] or row["target"] != target["target"]:
                    raise RuntimeError("instance or target drift")
                if n == 16 and float(row["energy"]) < float(target["target"]):
                    raise RuntimeError("candidate below exhaustive optimum")
                resource = resource_file.read_text().strip().split("\t")
                if len(resource) != 4:
                    raise RuntimeError("resource record malformed")
                writer.writerow(fields + resource + [f"{launch_ms:.6f}"]); f.flush()
            except Exception as exc:
                log.write(f"FAILED row {index}: {args!r}: {exc!r}\n")
                if isinstance(exc, subprocess.CalledProcessError):
                    log.write(exc.stdout + exc.stderr)
                log.flush()
                raise
            if index % 100 == 0:
                message = f"{index}/4200 rows complete; {mode} {family} n={n}"
                print(message, flush=True); log.write(message + "\n"); log.flush()
    (OUTPUT / "complete.json").write_text(json.dumps({"rows": 4200, "raw_sha256": digest(OUTPUT / "raw.tsv"), "targets_sha256": digest(OUTPUT / "targets.tsv")}, indent=2))
    command(["python3", str(HERE / "analyze_exp001.py"), str(OUTPUT)])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--check", action="store_true")
    group.add_argument("--run", action="store_true")
    args = parser.parse_args()
    if args.check:
        build_and_check()
    else:
        run()
