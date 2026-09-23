"""Rebuild the frozen, logging-only instrumented external specialist."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile


def build(destination):
    here = Path(__file__).resolve().parent
    meta = json.loads((here / "reference.json").read_text())
    for name, key in [("reference.tar.gz", "archive_sha256"),
                      ("reference_logging.patch", "patch_sha256")]:
        assert hashlib.sha256((here / name).read_bytes()).hexdigest() == meta[key]
    destination.mkdir(parents=True, exist_ok=False)
    with tarfile.open(here / "reference.tar.gz") as archive:
        archive.extractall(destination, filter="data")
    source = destination / "solvers/lMAts-lRRts/src"
    with (here / "reference_logging.patch").open() as patch:
        subprocess.run(["patch", "-p1"], cwd=source, stdin=patch, check=True)
    subprocess.run([
        "make", "lMAts",
        f"CFLAGS=-O3 -std=gnu99 -DNDEBUG -D__RUN_IN_LINUX__ -DlMAts -I{source}/libs/dcmt0.6.1/include",
        "LDFLAGS=-lpthread -lm",
    ], cwd=source, check=True)
    print(source / "lMAts")


if __name__ == "__main__":
    build(Path(sys.argv[1]).resolve())
