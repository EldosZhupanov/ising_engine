#!/usr/bin/env python3
"""Fetch the frozen official QOBLIB MIS holdout and verify Git blob IDs."""

import argparse
import hashlib
import json
import urllib.request
from pathlib import Path


COMMIT = "2b400f43c197bb0eb9bc9802efa2b28b818ab63c"
FILES = {
    "C125-9.gph": "8a4d512bc71a16a458b6aae3f2820cf6bd04127d",
    "brock200-2.gph": "d1141099281fdd4ba084cee7c7ec7962d9e6cc96",
    "hamming6-4.gph": "5448b16bfe4b459aa5e4643545d0083f410707aeb798",
    "sloane_1zc_128.gph": "9388d86f8183ceb0baf24e76fb09192f84bf7d89f",
    "johnson8-4-4.gph": "14378e8d32d4e0d04349a1e72a94c447cebe329",
    "football.gph": "a71393c17349b56abacbc9035aab65712e856fae62",
}


def git_blob_sha1(data):
    return hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()


def fetch(output):
    output.mkdir(parents=True, exist_ok=True)
    manifest = {"commit": COMMIT, "files": {}}
    for name, expected in FILES.items():
        path = output / name
        if path.exists():
            data = path.read_bytes()
        else:
            url = (f"https://raw.githubusercontent.com/ZIB-AOPT/QOBLIB/{COMMIT}/"
                   f"07-independentset/instances/{name}")
            req = urllib.request.Request(url, headers={"User-Agent": "IsingEngine-CD005-Q"})
            with urllib.request.urlopen(req, timeout=30) as response:
                data = response.read()
        actual = git_blob_sha1(data)
        if actual != expected:
            raise ValueError(f"{name}: blob {actual}, expected {expected}")
        if not path.exists():
            path.write_bytes(data)
        manifest["files"][name] = {"git_blob_sha1": actual,
                                   "sha256": hashlib.sha256(data).hexdigest(),
                                   "bytes": len(data)}
    manifest_path = output / "manifest.json"
    if manifest_path.exists():
        if json.loads(manifest_path.read_text()) != manifest:
            raise ValueError("Existing manifest differs from pinned data")
    else:
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps(manifest, sort_keys=True))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    fetch(parser.parse_args().output)
