"""Download engine: mirror failover, verification, unpacking, filename
normalization, SHA256 manifests, metadata.json and per-dataset README.md.

Layout after a run (everything under the data directory)::

    data/
      <dataset>/
        <normalized files...>       # parseable instance files
        metadata.json               # per-file sha256/bytes/source/status
        README.md                   # origin, license, citation
        best_known.csv              # (qplib only) verified objectives
      _archives/                    # raw downloaded archives (kept for audit)

Everything is idempotent: files already present with a non-zero size are not
re-downloaded unless ``force=True``. After one successful run the whole suite
works fully offline.
"""

from __future__ import annotations

import csv
import fnmatch
import gzip
import hashlib
import io
import json
import re
import shutil
import subprocess
import tarfile
import time
import urllib.request
import zipfile
from pathlib import Path

from . import registry as reg
from .parsers import PARSERS

USER_AGENT = "ising-engine-benchmark-hub/1.0 (research use)"


# ---------------------------------------------------------------------------
# Primitives
# ---------------------------------------------------------------------------


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _fetch_urllib(url: str, dest: Path, timeout: int) -> None:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=timeout) as resp, open(dest, "wb") as out:
        status = getattr(resp, "status", 200)
        if not 200 <= status < 300:
            raise OSError(f"HTTP {status}")
        shutil.copyfileobj(resp, out)


def _fetch_curl(url: str, dest: Path, timeout: int) -> None:
    r = subprocess.run(
        [
            "curl", "-sSL", "--fail", "--max-time", str(timeout),
            "-o", str(dest), "-w", "%{http_code}", url,
        ],
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        raise OSError(f"curl exit {r.returncode}: {r.stderr.strip()[:200]}")
    code = r.stdout.strip()[-3:]
    if not code.startswith("2"):
        raise OSError(f"HTTP {code}")


def _looks_like_html_error(path: Path) -> bool:
    """Detect an HTML page saved where a data file was expected (some
    servers answer 200/300 with an error page — e.g. OR-Library over
    plain http). Every registered dataset is plain text or a binary
    archive, never HTML."""
    with open(path, "rb") as fh:
        head = fh.read(256).lstrip().lower()
    return head.startswith((b"<!doctype", b"<html", b"<head", b"<?xml")) or b"<html" in head[:64]


def fetch(urls, dest: Path, timeout: int = 60) -> str:
    """Download ``dest`` from the first URL that works (urllib, then curl as
    a fallback per URL). Atomic: writes ``dest.part`` then renames. Returns
    the URL that succeeded; raises OSError listing every failure."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".part")
    errors = []
    for url in urls:
        for method in (_fetch_urllib, _fetch_curl):
            try:
                method(url, tmp, timeout)
                if tmp.stat().st_size == 0:
                    raise OSError("empty response")
                if _looks_like_html_error(tmp):
                    raise OSError("got an HTML page instead of data")
                tmp.rename(dest)
                return url
            except Exception as exc:  # noqa: BLE001 - collect and continue
                errors.append(f"{url} [{method.__name__}]: {exc}")
                tmp.unlink(missing_ok=True)
    raise OSError("all sources failed:\n  " + "\n  ".join(errors))


def normalize_name(name: str) -> str:
    """Filesystem-safe, consistent instance filename: basename, spaces and
    odd characters mapped to ``_``, single ``.gz`` suffix stripped (files are
    stored decompressed)."""
    base = Path(name).name
    if base.endswith(".gz"):
        base = base[:-3]
    return re.sub(r"[^A-Za-z0-9._+-]", "_", base)


def _write_normalized(data: bytes, member_name: str, dataset_dir: Path) -> Path:
    out = dataset_dir / normalize_name(member_name)
    with open(out, "wb") as fh:
        fh.write(data)
    return out


def unpack(archive: Path, dataset_dir: Path, select: str) -> list:
    """Extract regular files matching ``select`` (fnmatch on the basename)
    from a .tar.gz/.tgz/.zip/.gz archive into ``dataset_dir`` with normalized
    flat filenames. Returns the written paths."""
    written = []
    name = archive.name.lower()
    if name.endswith((".tar.gz", ".tgz", ".tar")):
        with tarfile.open(archive) as tf:
            for m in tf.getmembers():
                if not m.isfile():
                    continue
                if not fnmatch.fnmatch(Path(m.name).name, select):
                    continue
                data = tf.extractfile(m).read()
                written.append(_write_normalized(data, m.name, dataset_dir))
    elif name.endswith(".zip"):
        with zipfile.ZipFile(archive) as zf:
            for m in zf.infolist():
                if m.is_dir():
                    continue
                if not fnmatch.fnmatch(Path(m.filename).name, select):
                    continue
                written.append(_write_normalized(zf.read(m), m.filename, dataset_dir))
    elif name.endswith(".gz"):
        with gzip.open(archive, "rb") as fh:
            written.append(_write_normalized(fh.read(), archive.name, dataset_dir))
    else:
        raise ValueError(f"unsupported archive type: {archive.name}")
    return written


# ---------------------------------------------------------------------------
# Per-dataset download strategies
# ---------------------------------------------------------------------------


def _record(files: list, path: Path, source: str, parser: str) -> None:
    files.append(
        {
            "file": path.name,
            "sha256": sha256_file(path),
            "bytes": path.stat().st_size,
            "source_url": source,
            "parser": parser,
            "status": "ok",
        }
    )


def _strategy_files(ds: dict, dataset_dir: Path, opts: dict, files: list, errors: list):
    names = ds["files"][: opts["limit"]] if opts["limit"] else ds["files"]
    for fname in names:
        urls = [t.format(file=fname) for t in ds["url_templates"]]
        is_gz = fname.endswith(".gz")
        target = dataset_dir / normalize_name(fname)
        raw = dataset_dir / Path(fname).name if is_gz else target
        if target.exists() and target.stat().st_size > 0 and not opts["force"]:
            _record(files, target, "cached", ds["parser"])
            continue
        try:
            used = fetch(urls, raw, opts["timeout"])
            if is_gz:
                with gzip.open(raw, "rb") as fh:
                    data = fh.read()
                raw.unlink()
                with open(target, "wb") as fh:
                    fh.write(data)
            _record(files, target, used, ds["parser"])
        except OSError as exc:
            errors.append({"file": fname, "error": str(exc)})


def _strategy_archive(ds: dict, dataset_dir: Path, opts: dict, files: list, errors: list):
    arch_dir = dataset_dir.parent / "_archives"
    specs = ds["archives"][: opts["limit"]] if opts["limit"] else ds["archives"]
    for spec in specs:
        arch = arch_dir / spec["name"]
        try:
            if not (arch.exists() and arch.stat().st_size > 0) or opts["force"]:
                used = fetch(spec["urls"], arch, opts["timeout"])
            else:
                used = "cached"
            for path in unpack(arch, dataset_dir, spec.get("select", "*")):
                _record(files, path, f"{used} :: {spec['name']}", spec.get("parser", ds["parser"]))
        except (OSError, ValueError) as exc:
            errors.append({"file": spec["name"], "error": str(exc)})


def _strategy_qplib_index(ds: dict, dataset_dir: Path, opts: dict, files: list, errors: list):
    idx_path = dataset_dir / "instancedata.csv"
    try:
        if not (idx_path.exists() and idx_path.stat().st_size > 0) or opts["force"]:
            fetch([ds["index_url"]], idx_path, opts["timeout"])
    except OSError as exc:
        errors.append({"file": "instancedata.csv", "error": str(exc)})
        return
    with open(idx_path, newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))
    picks = [
        r
        for r in rows
        if r["ncons"] == "0" and r["nbinvars"] == r["nvars"] and int(r["nvars"]) > 0
    ]
    if opts["limit"]:
        picks = picks[: opts["limit"]]
    best = []
    for r in picks:
        name = r["name"]
        fname = f"{name}.qplib"
        target = dataset_dir / fname
        try:
            if not (target.exists() and target.stat().st_size > 0) or opts["force"]:
                used = fetch(
                    [ds["instance_url_template"].format(file=name)], target, opts["timeout"]
                )
            else:
                used = "cached"
            _record(files, target, used, ds["parser"])
            if r.get("solobjvalue"):
                best.append((fname, r["objsense"], r["solobjvalue"]))
        except OSError as exc:
            errors.append({"file": fname, "error": str(exc)})
    if best:
        with open(dataset_dir / "best_known.csv", "w", encoding="utf-8") as fh:
            fh.write("instance,objsense,best_known_native\n")
            for name, sense, val in best:
                fh.write(f"{name},{sense},{val}\n")


_STRATEGIES = {
    "files": _strategy_files,
    "archive": _strategy_archive,
    "qplib_index": _strategy_qplib_index,
}


# ---------------------------------------------------------------------------
# Orchestration
# ---------------------------------------------------------------------------


def _dataset_readme(name: str, ds: dict, files: list, errors: list) -> str:
    lines = [
        f"# {name}",
        "",
        ds.get("description", ""),
        "",
        f"- **Origin:** {ds.get('official_url', 'n/a')}",
        f"- **License:** {ds.get('license', 'unspecified')}",
        f"- **Citation:** {ds.get('citation', 'n/a')}",
        f"- **Format:** {ds.get('format', 'n/a')} (parser `{ds.get('parser')}`)",
        f"- **Files:** {len(files)} downloaded, {len(errors)} failed",
        "",
        "Downloaded and verified by `benchmark_suite/scripts/download_all_benchmarks.py`.",
        "Integrity manifest (SHA256 per file) is in `metadata.json`.",
        "If you publish results on these instances, cite the origin above.",
        "",
    ]
    return "\n".join(lines)


def download_dataset(
    name: str,
    data_dir: Path,
    timeout: int = 60,
    limit: int = 0,
    force: bool = False,
) -> dict:
    """Download + verify one dataset. Returns its metadata dict (also written
    to ``<data_dir>/<name>/metadata.json``)."""
    ds = reg.dataset(name)
    dataset_dir = Path(data_dir) / name
    dataset_dir.mkdir(parents=True, exist_ok=True)
    opts = {"timeout": timeout, "limit": limit, "force": force}
    files, errors = [], []
    _STRATEGIES[ds["strategy"]](ds, dataset_dir, opts, files, errors)
    meta = {
        "dataset": name,
        "description": ds.get("description"),
        "license": ds.get("license"),
        "citation": ds.get("citation"),
        "official_url": ds.get("official_url"),
        "downloaded_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "status": "ok" if files and not errors else ("partial" if files else "unavailable"),
        "files": files,
        "errors": errors,
    }
    with open(dataset_dir / "metadata.json", "w", encoding="utf-8") as fh:
        json.dump(meta, fh, indent=2)
    with open(dataset_dir / "README.md", "w", encoding="utf-8") as fh:
        fh.write(_dataset_readme(name, ds, files, errors))
    return meta


def download_all(
    data_dir: Path,
    only=None,
    exclude=None,
    timeout: int = 60,
    limit: int = 0,
    force: bool = False,
    log=print,
) -> dict:
    """Download every registered dataset (or the ``only`` subset). Returns
    ``{dataset: metadata}``."""
    results = {}
    for name in reg.dataset_names():
        if only and name not in only:
            continue
        if exclude and name in exclude:
            continue
        log(f"── {name} …")
        meta = download_dataset(name, data_dir, timeout=timeout, limit=limit, force=force)
        log(
            f"   {meta['status']}: {len(meta['files'])} files"
            + (f", {len(meta['errors'])} errors" if meta["errors"] else "")
        )
        results[name] = meta
    return results


def verify_dataset(name: str, data_dir: Path) -> list:
    """Re-hash every file against metadata.json and re-parse it with its
    registered parser. Returns a list of problem strings (empty = clean)."""
    dataset_dir = Path(data_dir) / name
    meta_path = dataset_dir / "metadata.json"
    problems = []
    if not meta_path.exists():
        return [f"{name}: no metadata.json (not downloaded)"]
    with open(meta_path, encoding="utf-8") as fh:
        meta = json.load(fh)
    for rec in meta["files"]:
        path = dataset_dir / rec["file"]
        if not path.exists():
            problems.append(f"{name}/{rec['file']}: missing")
            continue
        if sha256_file(path) != rec["sha256"]:
            problems.append(f"{name}/{rec['file']}: sha256 mismatch")
            continue
        parser = PARSERS[rec["parser"]]
        try:
            with open(path, encoding="utf-8", errors="replace") as fh:
                parsed = parser(fh.read(), rec["file"])
            for inst in parsed if isinstance(parsed, list) else [parsed]:
                inst.validate()
        except (ValueError, StopIteration, KeyError) as exc:
            problems.append(f"{name}/{rec['file']}: parse/validate failed: {exc}")
    return problems
