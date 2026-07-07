#!/usr/bin/env python3
"""Offline self-test of the Benchmark Hub: parsers, energy conventions, the
YAML-subset loader, and the registry. Runs with no network and no downloads.

    python3 benchmark_suite/scripts/selftest.py

Conventions are checked against tiny instances with known optima, mirroring
the Rust test-suite (tests/test_benchmark.rs) so the two languages provably
share the same QUBO mapping.
"""

import itertools
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ising_bench import miniyaml, registry  # noqa: E402
from ising_bench.parsers import (  # noqa: E402
    parse_biqmac_sparse,
    parse_maxcut_auto,
    parse_orlib_bqp,
    parse_qplib,
    parse_rudy,
    parse_snap_edgelist,
)

FAILURES = []


def check(cond, msg):
    if cond:
        print(f"  ok  {msg}")
    else:
        print(f"FAIL  {msg}")
        FAILURES.append(msg)


def brute_min(qubo):
    return min(
        qubo.energy(x) for x in itertools.product((0, 1), repeat=qubo.n)
    )


def main() -> int:
    print("== rudy / MaxCut mapping ==")
    tri = parse_rudy("3 3\n1 2 1\n2 3 1\n1 3 1\n", "tri")
    check(tri.n == 3 and tri.m == 3, "triangle parses")
    check(tri.cut_value([1, 0, 0]) == 2.0, "triangle cut value")
    q = tri.to_qubo()
    check(q.maximize, "maxcut qubo is maximize-flagged")
    check(abs(brute_min(q) + 2.0) < 1e-9, "min energy = -maxcut = -2")
    c4 = parse_rudy("4 4\n1 2 1\n2 3 1\n3 4 1\n4 1 1\n", "c4")
    check(abs(brute_min(c4.to_qubo()) + 4.0) < 1e-9, "4-cycle maxcut = 4")

    print("== DIMACS ==")
    d = parse_maxcut_auto("c comment\np edge 3 2\ne 1 2 5\ne 2 3 1\n", "d")
    check(d.n == 3 and d.m == 2, "dimacs parses via auto-sniff")
    check(abs(brute_min(d.to_qubo()) + 6.0) < 1e-9, "dimacs maxcut = 6")

    print("== SNAP edge list ==")
    s = parse_snap_edgelist("# comment\n10 20\n20 30\n30 10\n10 10\n20 10\n", "s")
    check(s.n == 3 and s.m == 3, "snap remap+dedupe (triangle)")
    check(abs(brute_min(s.to_qubo()) + 2.0) < 1e-9, "snap maxcut = 2")

    print("== OR-Library BQP ==")
    # Q symmetric, one triangle listed -> off-diag counts twice:
    # max x'Qx = x1 + x2 + 2*3*x1x2 = 8 at (1,1).
    v = parse_orlib_bqp("1\n2 3\n1 1 1\n2 2 1\n1 2 3\n", "toy")
    check(len(v) == 1, "orlib problem count")
    check(abs(brute_min(v[0]) + 8.0) < 1e-9, "orlib symmetric-Q double count (max 8)")
    check(v[0].maximize, "orlib is maximize-flagged")

    print("== Biq Mac sparse ==")
    # MINIMIZE x'Qx, symmetric double: E = x1 + x2 - 6 x1x2, min -4 at (1,1).
    b = parse_biqmac_sparse("2 3\n1 1 1\n2 2 1\n1 2 -3\n", "bm")
    check(not b.maximize, "biqmac sparse is minimize")
    check(abs(brute_min(b) + 4.0) < 1e-9, "biqmac minimize + double count (min -4)")

    print("== QPLIB ==")
    # Real QBN layout: no constraint-count line (adaptive format). Listed
    # quad entries carry a ½ factor and are NOT symmetric-doubled, so pair
    # coeff = -6/2 = -3: min (x1 + x2 - 3 x1x2) = -1 at (1,1).
    qp = parse_qplib(
        "toy name\nQBN type\nminimize sense\n2 n\n1 nq\n1 2 -6 q\n"
        "0 bd\n2 nb\n1 1 b\n2 1 b\n0 c\n",
        "qp",
    )
    check(not qp.maximize, "qplib minimize sense")
    check(abs(brute_min(qp) + 1.0) < 1e-9, "qplib half-factor pair coeff (min -1)")
    try:
        parse_qplib("c n\nQBL t\nminimize s\n2 n\n", "bad")
        check(False, "qplib refuses constrained")
    except ValueError:
        check(True, "qplib refuses constrained")
    try:
        parse_qplib("c n\nQCN t\nminimize s\n2 n\n", "bad")
        check(False, "qplib refuses non-binary (continuous vars)")
    except ValueError:
        check(True, "qplib refuses non-binary (continuous vars)")

    print("== miniyaml ==")
    doc = miniyaml.load(
        "a: 1\nb:\n  - x\n  - 2\nc:\n  d: 'q # not comment'\n  e: true # comment\n"
    )
    check(doc == {"a": 1, "b": ["x", 2], "c": {"d": "q # not comment", "e": True}},
          "yaml subset round-trip")

    print("== registry ==")
    names = registry.dataset_names()
    check(len(names) >= 6, f"registry has {len(names)} datasets")
    for n in names:
        ds = registry.dataset(n)
        check("citation" in ds and "license" in ds, f"{n} carries license+citation")

    print()
    if FAILURES:
        print(f"{len(FAILURES)} FAILURES")
        return 1
    print("all self-tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
