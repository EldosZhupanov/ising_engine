#!/usr/bin/env python3
"""Local Graft compatibility diagnostic; no model/API or performance claims.

Run after `bash scripts/graft.sh setup`. Each run creates a new disposable Git
snapshot under .cache and writes its raw results there. It never edits source
files, agent configuration, or an earlier result.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
REVISION = "80692e5ad0bc8e8f7e1edea648247d90b9f76820"
CLI = ROOT / ".cache/graft-toolchain" / REVISION / "dist/cli.js"
ENV = dict(os.environ, DO_NOT_TRACK="1", CI="1", GRAFT_NO_GITIGNORE="1",
           GRAFT_NO_IGNORE="1", GRAFT_REFRESH="hash")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    if not CLI.is_file():
        raise SystemExit("Run bash scripts/graft.sh setup first")
    case_path = Path(__file__).with_name("cases.json")
    cases = json.loads(case_path.read_text())
    work = ROOT / ".cache/graft-qualification"
    work.mkdir(parents=True, exist_ok=True)
    run_dir = Path(tempfile.mkdtemp(prefix="run-", dir=work))
    snapshot = run_dir / "snapshot"
    snapshot.mkdir()
    manifest = {}
    names = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT).decode().split("\0")
    for name in sorted(set(names)):
        src = ROOT / name
        if (not name or name.startswith("benchmarks/qoblib/upstream/")
                or src.is_symlink() or not src.is_file()):
            continue
        dest = snapshot / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)
        manifest[name] = digest(src)
    (run_dir / "source_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    probe = snapshot / "src/qualification_probe.rs"
    probe.write_text("pub fn graft_probe_original() {}\n")
    adjacent = snapshot / "src/qualification_probe.rs.sol"
    adjacent.write_text("contract GraftExcludedSolution {}\n")

    def git(*args):
        return subprocess.check_output(["git", *args], cwd=snapshot, text=True)

    git("init", "-q")
    git("add", "--all")
    # Preserve tracked files even when a current ignore rule covers their path.
    tracked = subprocess.check_output(["git", "ls-files", "-z", "--cached"], cwd=ROOT).decode().split("\0")
    force_paths = [p for p in tracked if p in manifest]
    subprocess.run(["git", "add", "-f", "--pathspec-from-file=-", "--pathspec-file-nul"],
                   input=("\0".join(force_paths) + "\0").encode(), cwd=snapshot, check=True)
    git("-c", "user.name=Graft fixture", "-c", "user.email=fixture@localhost",
        "commit", "-qm", "Local compatibility fixture")

    def graft(*args):
        p = subprocess.run(["bash", str(ROOT / "scripts/graft.sh"), *args],
                           cwd=snapshot, env=dict(ENV, ISING_GRAFT_ROOT=str(snapshot)), capture_output=True, text=True,
                           timeout=120)
        if p.returncode:
            raise RuntimeError(f"Graft {args}: {p.returncode}: {p.stderr[-2000:]}")
        return p.stdout

    def ask(query):
        return json.loads(graft("ask", query, "--json", "-n", str(cases["top_k"])))

    def graph():
        return json.loads((snapshot / ".cache/graft-index/.graph/wiring.json").read_text())

    def symbols():
        return {n["name"] for n in graph()["nodes"] if n["kind"] != "file"}

    report = {
        "upstream_revision": REVISION, "version": graft("--version").strip(),
        "node": subprocess.check_output(["node", "--version"], text=True).strip(),
        "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "cases_sha256": digest(case_path), "runner_sha256": digest(Path(__file__)),
        "manifest_sha256": digest(run_dir / "source_manifest.json"),
        "scope": "16 symbol lookups and 4 document probes; development diagnostic, not agent QA or speed benchmark",
        "adapter_sha256": digest(ROOT / "scripts/graft.sh"),
        "scope_rule": "Explicit visible supported code paths; recomputed on each adapter call; excludes .sol",
        "cases": [], "checks": {},
    }
    built = graft("build")
    (run_dir / "build.log").write_text(built)
    original_graph = graph()
    report["index"] = {"nodes": len(original_graph["nodes"]),
                       "edges": len(original_graph["edges"]),
                       "files": sum(n["kind"] == "file" for n in original_graph["nodes"])}
    report["checks"]["solution_files_excluded"] = not any(
        n["path"].endswith(".sol") for n in original_graph["nodes"])
    before_rejection = digest(snapshot / ".cache/graft-index/.graph/wiring.json")
    reserved = [["--only-dir", "research"], ["--only-dir=research"],
                ["--include-dir", "research"], ["--follow-submodules"],
                ["--follow-nested-repos"], [str(run_dir)]]
    rejected = []
    for args in reserved:
        p = subprocess.run(["bash", str(ROOT / "scripts/graft.sh"), "build", *args],
                           cwd=snapshot, env=dict(ENV, ISING_GRAFT_ROOT=str(snapshot)),
                           capture_output=True, text=True, timeout=30)
        rejected.append(p.returncode != 0)
    report["checks"]["scope_overrides_rejected"] = all(rejected) and (
        digest(snapshot / ".cache/graft-index/.graph/wiring.json") == before_rejection)
    valid_pointers = True
    for case in cases["cases"]:
        result = ask(case["query"])
        paths = []
        for hit in result["hits"]:
            match = re.fullmatch(r"(.+):L(\d+)-L(\d+)", hit["pointer"])
            if not match:
                # A file hit legitimately has no symbol span.
                p = snapshot / hit["pointer"]
                valid_pointers &= p.is_file()
                paths.append(hit["pointer"])
                continue
            name, start, end = match.groups()
            p = snapshot / name
            valid_pointers &= p.is_file() and 1 <= int(start) <= int(end) <= len(p.read_text().splitlines())
            paths.append(name)
        # Strong cheap symbol baseline: declaration search. Documents use literal
        # search over Markdown. Retain all matches, and report top-k separately.
        visible = git("ls-files", "--cached", "--others", "--exclude-standard").splitlines()
        if case["kind"] == "code":
            pattern = r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:struct|enum|trait|type|fn|def|class)\s+" + re.escape(case["query"]) + r"\b"
            files = [p for p in visible if p.endswith((".rs", ".py"))]
            cmd = ["rg", "--files-with-matches", pattern, "--", *files]
        else:
            files = [p for p in visible if p.endswith(".md")]
            cmd = ["rg", "--files-with-matches", "--fixed-strings", case["query"], "--", *files]
        baseline = subprocess.run(cmd, cwd=snapshot, capture_output=True, text=True, timeout=15)
        if baseline.returncode not in (0, 1):
            raise RuntimeError(baseline.stderr)
        matches = sorted(x.removeprefix("./") for x in baseline.stdout.splitlines())
        report["cases"].append({**case, "graft_hit": case["path"] in paths,
                                "graft": result, "rg_matches": matches,
                                "rg_hit": case["path"] in matches,
                                "rg_top_k_hit": case["path"] in matches[:cases["top_k"]]})
    report["checks"]["returned_spans_exist"] = bool(valid_pointers)
    before = probe.stat()
    probe.write_text("pub fn graft_probe_modified() {}\n")
    os.utime(probe, ns=(before.st_atime_ns, before.st_mtime_ns))
    ask("graft_probe_modified")
    report["checks"]["same_size_mtime_edit_refreshed"] = (
        "graft_probe_modified" in symbols() and "graft_probe_original" not in symbols())
    probe.write_text("pub fn graft_probe_staged() {}\n")
    git("add", "src/qualification_probe.rs")
    ask("graft_probe_staged")
    report["checks"]["staged_edit_refreshed"] = "graft_probe_staged" in symbols()
    untracked = snapshot / "src/qualification_untracked.rs"
    untracked.write_text("pub fn graft_probe_untracked() {}\n")
    ask("graft_probe_untracked")
    report["checks"]["untracked_code_found"] = "graft_probe_untracked" in symbols()
    ignored = snapshot / ".cache/qualification_ignored.rs"
    ignored.write_text("pub fn graft_probe_ignored() {}\n")
    ask("graft_probe_ignored")
    report["checks"]["ignored_code_excluded"] = "graft_probe_ignored" not in symbols()
    untracked.unlink()
    ask("graft_probe_untracked")
    report["checks"]["deleted_code_removed"] = "graft_probe_untracked" not in symbols()
    report["checks"]["snapshot_sources_unchanged"] = all(
        (snapshot / p).is_file() and digest(snapshot / p) == h for p, h in manifest.items())
    unexpected = set(git("ls-files", "--others", "--exclude-standard").splitlines())
    report["checks"]["no_unexpected_visible_files"] = not unexpected
    report["checks"]["temporary_lists_cleaned"] = not list(
        (snapshot / ".cache/graft-index").glob("files.*"))
    report["unexpected_visible_files"] = sorted(unexpected)
    # Confirm that the production checkout was not changed by fixture operations.
    report["checks"]["working_sources_unchanged"] = all(
        (ROOT / p).is_file() and digest(ROOT / p) == h for p, h in manifest.items())
    report["counts"] = {kind: {key: sum(bool(c[key]) for c in report["cases"] if c["kind"] == kind)
                               for key in ("graft_hit", "rg_hit", "rg_top_k_hit")}
                        for kind in ("code", "document")}
    (run_dir / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"run_dir": str(run_dir), "index": report["index"],
                      "counts": report["counts"], "checks": report["checks"]}, indent=2))
    if not all(report["checks"].values()):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
