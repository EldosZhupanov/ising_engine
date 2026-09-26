"""LABS-Q002: frozen equal-deadline comparison using the retained Q001 supervisor."""
import argparse
import hashlib
import importlib.util
import itertools
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
Q001 = ROOT / "research/labs_qualification"
SUPERVISOR_SHA = "d36cbd1f1c462255f7405fa43da0a49488f6c1ff8ac5f9d8f20e726656dfa785"
CASES_SHA = "dfa43a17e8e6966faac1572e95cb758d54068c75005e45536637d6c26bfb1b1a"
TARGETS = {40: 108, 50: 153, 60: 218}
SEEDS = list(range(740001, 740011))
ARMS = ("memetic", "lmats")
BUDGET = 10.0
SOURCE_PATHS = [
    "research/experiments/labs_q002/" + name
    for name in ("campaign.py", "test_campaign.py", "protocol.md", "cases.json")
] + ["src/bin/labs_record_hunter.rs", "Cargo.lock", "Cargo.toml", ".cargo/config.toml",
     "benchmarks/qoblib/check_labs.rs"] + [
    "research/labs_qualification/" + name for name in (
        "campaign.py", "reference.json", "reference.tar.gz", "reference_logging.patch", "build_reference.py")]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def git_digest(commit, path):
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("invalid source commit")
    contents = subprocess.check_output(["git", "cat-file", "blob", f"{commit}:{path}"],
                                      cwd=ROOT, stderr=subprocess.DEVNULL)
    return hashlib.sha256(contents).hexdigest()


def validate_provenance(directory):
    env = json.loads((directory / "environment.json").read_text())
    completion = json.loads((directory / "complete.json").read_text())
    if (env["smoke"] is not False or env["seeds"] != SEEDS or env["budget"] != BUDGET
            or env["targets"] != {str(n): target for n, target in TARGETS.items()}):
        raise ValueError("manifest design mismatch")
    if completion["cells"] != 60 or completion["commit"] != env["commit"]:
        raise ValueError("completion/source freeze mismatch")
    if set(env["source_sha256"]) != set(SOURCE_PATHS):
        raise ValueError("source manifest is incomplete")
    for path, recorded in env["source_sha256"].items():
        if git_digest(env["commit"], path) != recorded:
            raise ValueError(f"source hash differs from frozen commit: {path}")
    if set(env["binaries"]) != {*ARMS, "checker"} or set(env["binary_sha256"]) != {*ARMS, "checker"}:
        raise ValueError("binary manifest is incomplete")
    if any(not re.fullmatch(r"[0-9a-f]{64}", value) for value in env["binary_sha256"].values()):
        raise ValueError("invalid binary digest")
    return env


if digest(Q001 / "campaign.py") != SUPERVISOR_SHA:
    raise RuntimeError("retained Q001 supervisor changed")
spec = importlib.util.spec_from_file_location("q001_supervisor", Q001 / "campaign.py")
q001 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q001)


def quartile(values, fraction):
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    left = math.floor(position)
    right = math.ceil(position)
    return ordered[left] + (ordered[right] - ordered[left]) * (position - left)


def distribution(values):
    if not values:
        return None
    q25, q75 = quartile(values, .25), quartile(values, .75)
    return {"mean": statistics.mean(values), "median": statistics.median(values),
            "sample_sd": statistics.stdev(values) if len(values) > 1 else None,
            "min": min(values), "max": max(values), "q25": q25, "q75": q75,
            "iqr": q75 - q25}


def wilcoxon_exact(differences):
    nonzero = [value for value in differences if value != 0]
    if not nonzero:
        return 1.0
    absolute = sorted(abs(value) for value in nonzero)
    # Doubled average ranks keep tied-rank enumeration exact in integers.
    ranks = {value: sum(i + 1 for i, item in enumerate(absolute) if item == value)
             * 2 // absolute.count(value) for value in set(absolute)}
    weights = [ranks[abs(value)] for value in nonzero]
    observed = abs(sum(rank if value > 0 else -rank
                       for rank, value in zip(weights, nonzero)))
    extreme = sum(abs(sum(sign * rank for sign, rank in zip(signs, weights))) >= observed
                  for signs in itertools.product((-1, 1), repeat=len(weights)))
    return extreme / (2 ** len(weights))


def holm(pvalues):
    adjusted, floor = {}, 0.0
    ordered = sorted(pvalues, key=pvalues.get)
    for i, key in enumerate(ordered):
        floor = min(1.0, max(floor, (len(ordered) - i) * pvalues[key]))
        adjusted[key] = floor
    return adjusted


def restart_tts99(hits):
    if hits == 0:
        return None
    if hits == 10:
        return BUDGET
    return BUDGET * max(1, math.ceil(math.log(.01) / math.log1p(-hits / 10)))


def checker_valid(checker, n, energy):
    match = re.search(rf"LABS k={n} E\(S\)=(\d+) ", checker.get("stdout", ""))
    return (checker.get("returncode") in (0, 20)
            and match is not None and int(match[1]) == energy)


def command_for(arm, binary, n, seed):
    return ([binary, "--qualify", str(n), str(seed)] if arm == "memetic" else
            [binary, str(n), str(seed), "3600", "1", "0"])


def valid_endpoint(row, arm, n, seed):
    if (row["arm"], row["n"], row["seed"], row["target"], row["budget"]) != (
            arm, n, seed, TARGETS[n], BUDGET):
        raise ValueError("cell identity mismatch")
    if row["errors"]:
        raise ValueError("solver/supervisor reported errors")
    if row["stop"] not in ("deadline", "optimum"):
        raise ValueError("unexpected stop")
    stopped = row["stop_seconds"]
    if not math.isfinite(stopped) or stopped < 0 or (
            row["stop"] == "deadline" and not BUDGET <= stopped <= BUDGET + .1):
        raise ValueError("budget enforcement failure")
    if any(event["eligible"] and event["seconds"] > min(BUDGET, stopped) for event in row["events"]):
        raise ValueError("credited witness arrived after stop")
    best = q001.best_event(row)  # Re-evaluates every sequence from its bits.
    if best is None or not checker_valid(row.get("checker", {}), n, best["energy"]):
        raise ValueError("missing witness or independent checker failure")
    if best["energy"] < TARGETS[n]:
        raise ValueError("energy below proven target")
    if row["stop"] == "optimum" and best["energy"] != TARGETS[n]:
        raise ValueError("optimum stop without an optimum witness")
    return best


def analyze(directory):
    directory = Path(directory)
    summary = {"budget_seconds": BUDGET, "arms": {}, "paired": {}, "failures": [],
               "scope": "these implementations/settings/host; no general speed or SOTA claim"}
    env = None
    try:
        env = validate_provenance(directory)
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        summary["failures"].append(f"provenance: {error}")
    for arm in ARMS:
        summary["arms"][arm] = {}
        for n, target in TARGETS.items():
            energies, hit_times, failures = [], [], []
            for seed in SEEDS:
                try:
                    row = json.loads((directory / f"{n}_{seed}_{arm}.json").read_text())
                    best = valid_endpoint(row, arm, n, seed)
                    if env is not None and row["command"] != command_for(arm, env["binaries"][arm], n, seed):
                        raise ValueError("cell command differs from frozen design")
                    energies.append(best["energy"])
                    hit_times.append(best["seconds"] if best["energy"] == target else None)
                except (OSError, ValueError, AssertionError, KeyError, TypeError) as error:
                    energies.append(None)
                    hit_times.append(None)
                    failures.append(seed)
                    summary["failures"].append(f"{n}/{seed}/{arm}: {error}")
            good = [value for value in energies if value is not None]
            hits = energies.count(target)
            summary["arms"][arm][str(n)] = {
                "energies": energies, "hits": hits, "runs": 10, "failures": failures,
                "success_rate": hits / 10, "wilson95": q001.wilson(hits, 10),
                "distribution": distribution(good),
                "relative_gaps": [None if e is None else (e-target)/target for e in energies],
                "hit_seconds": hit_times, "non_hits_right_censored_at": BUDGET,
                "restart_tts99_plugin_seconds": restart_tts99(hits),
                "hit_threshold_met": hits >= 8 and not failures}
    expected_files = {f"{n}_{seed}_{arm}.json" for n in TARGETS for seed in SEEDS for arm in ARMS}
    actual_files = {path.name for path in directory.glob("*_*.json") if path.name[0].isdigit()}
    if actual_files != expected_files:
        summary["failures"].append("cell file set differs from the registered design")
    pvalues = {}
    for n in TARGETS:
        candidate = summary["arms"]["memetic"][str(n)]["energies"]
        baseline = summary["arms"]["lmats"][str(n)]["energies"]
        pairs = [(a, b) for a, b in zip(candidate, baseline) if a is not None and b is not None]
        gains = [(b - a) / b for a, b in pairs]
        pvalues[str(n)] = wilcoxon_exact([b - a for a, b in pairs]) if len(pairs) == 10 else 1.0
        summary["paired"][str(n)] = {
            "memetic_wins": sum(a < b for a, b in pairs), "ties": sum(a == b for a, b in pairs),
            "lmats_wins": sum(a > b for a, b in pairs), "invalid_pairs": 10-len(pairs),
            "median_relative_gain": statistics.median(gains) if gains else None,
            "wilcoxon_exact_p": pvalues[str(n)]}
    adjusted = holm(pvalues)
    valid = not summary["failures"]
    for n, result in summary["paired"].items():
        result["holm_p"] = adjusted[n]
        result["local_superiority_gate"] = bool(valid and result["holm_p"] < .05
            and result["median_relative_gain"] >= .05)
    summary["verdict"] = ("INSTRUMENT_INVALID" if not valid else
        "QUALIFIED" if all(cell["hit_threshold_met"] for cell in summary["arms"]["memetic"].values())
        else "NOT_QUALIFIED")
    return summary


def execute(command, arm, n, target, seed, budget, out, checker):
    row = q001.supervise(command, n, target, budget)
    row.update(arm=arm, seed=seed, load_after=os.getloadavg())
    path = out / f"{n}_{seed}_{arm}.json"
    path.write_text(json.dumps(row, indent=2) + "\n")
    best = q001.best_event(row)
    if best is None:
        row["errors"].append("no eligible witness")
    else:
        witness = out / f"{n}_{seed}_{arm}.sol"
        witness.write_text(best["bits"] + "\n")
        row["checker"] = q001.check_witness([checker, str(n), str(witness), str(best["energy"])])
        if not checker_valid(row["checker"], n, best["energy"]):
            row["errors"].append("independent checker failure")
    path.write_text(json.dumps(row, indent=2) + "\n")
    print(json.dumps({"n": n, "seed": seed, "arm": arm,
                      "energy": None if best is None else best["energy"],
                      "errors": row["errors"]}), flush=True)
    return row


def run(args):
    if digest(HERE / "cases.json") != CASES_SHA:
        raise RuntimeError("cases changed")
    if subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"],
                               cwd=ROOT, text=True):
        raise RuntimeError("tracked source must be clean before execution")
    binaries = {key: str(Path(getattr(args, key)).resolve()) for key in (*ARMS, "checker")}
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    source_hashes = {path: digest(ROOT / path) for path in SOURCE_PATHS}
    for path, current in source_hashes.items():
        if git_digest(commit, path) != current:
            raise RuntimeError(f"source is not frozen at HEAD: {path}")
    targets, seeds, budget = design(args.action)
    env = {"commit": commit,
        "binaries": binaries, "binary_sha256": {k: digest(v) for k, v in binaries.items()},
        "source_sha256": source_hashes,
        "platform": platform.platform(), "cpu": Path("/proc/cpuinfo").read_text(),
        "memory": Path("/proc/meminfo").read_text(), "initial_load": os.getloadavg(),
        "python": platform.python_version(), "rustflags_env": os.environ.get("RUSTFLAGS"),
        "compiler_versions": {tool: subprocess.check_output([tool, "--version"], text=True)
                              for tool in ("rustc", "cargo", "gcc")},
        "targets": targets, "seeds": seeds, "budget": budget,
        "thread_environment": {key: "1" for key in ("RAYON_NUM_THREADS", "OMP_NUM_THREADS",
                                                      "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS")},
        "smoke": args.action == "smoke"}
    out = Path(args.output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / "environment.json").write_text(json.dumps(env, indent=2) + "\n")
    rows = []
    for ni, (n, target) in enumerate(targets.items()):
        for si, seed in enumerate(seeds):
            for arm in ARMS if (ni+si) % 2 == 0 else ARMS[::-1]:
                for path, expected in env["source_sha256"].items():
                    if digest(ROOT / path) != expected:
                        raise RuntimeError(f"source changed during campaign: {path}")
                if any(digest(path) != env["binary_sha256"][key] for key, path in binaries.items()):
                    raise RuntimeError("binary changed during campaign")
                command = command_for(arm, binaries[arm], n, seed)
                before = os.getloadavg()
                row = execute(command, arm, n, target, seed, budget, out, binaries["checker"])
                row["load_before"] = before
                (out / f"{n}_{seed}_{arm}.json").write_text(json.dumps(row, indent=2) + "\n")
                rows.append(row)
    (out / "complete.json").write_text(json.dumps({"commit": env["commit"], "cells": len(rows)}) + "\n")
    if args.action == "run":
        (out / "summary.json").write_text(json.dumps(analyze(out), indent=2) + "\n")
    elif any(row["errors"] for row in rows):
        raise RuntimeError("smoke integrity gate failed; inspect preserved rows")


def design(action):
    if action == "smoke":
        return {20: 26}, [820001, 820002], 1.0
    return TARGETS, SEEDS, BUDGET


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    for action in ("run", "smoke"):
        p = sub.add_parser(action)
        for key in (*ARMS, "checker", "output"):
            p.add_argument("--"+key, required=True)
    p = sub.add_parser("analyze")
    p.add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.action == "analyze":
        print(json.dumps(analyze(args.directory), indent=2))
    else:
        run(args)
