#!/usr/bin/env python3
"""OpenJij bridge for the Ising Engine benchmark harness.

Reads a QUBO in the harness line format and runs OpenJij's SASampler at the
matched sweep budget, emitting a JSON reply on stdout:

    {"available": true, "state": [0,1,...], "wall_ms": 12.3, "energy": -4.0}

If OpenJij (or a dependency) is missing, prints
    {"available": false, "reason": "..."}
and exits 0 so the Rust harness records the solver as skipped, never faked.

Input format (one token per field):
    P <n> <sweeps> <seed>
    L <i> <coeff>          # linear term  h_i
    Q <i> <j> <coeff>      # quadratic pair coeff, i < j
"""
import sys
import json
import time


def read_model(path):
    n = sweeps = seed = 0
    Q = {}
    with open(path) as fh:
        for line in fh:
            p = line.split()
            if not p:
                continue
            if p[0] == "P":
                n, sweeps, seed = int(p[1]), int(p[2]), int(p[3])
            elif p[0] == "L":
                Q[(int(p[1]), int(p[1]))] = float(p[2])
            elif p[0] == "Q":
                Q[(int(p[1]), int(p[2]))] = float(p[3])
    return n, sweeps, seed, Q


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"available": False, "reason": "no input file"}))
        return
    try:
        import openjij as oj
    except Exception as exc:  # noqa: BLE001 - report any import failure
        print(json.dumps({"available": False, "reason": "import openjij: %s" % exc}))
        return

    n, sweeps, seed, Q = read_model(sys.argv[1])
    sampler = oj.SASampler()
    t0 = time.perf_counter()
    try:
        res = sampler.sample_qubo(Q, num_sweeps=max(1, sweeps), num_reads=1, seed=seed)
    except TypeError:
        # Older/newer API without seed kwarg.
        res = sampler.sample_qubo(Q, num_sweeps=max(1, sweeps), num_reads=1)
    wall_ms = (time.perf_counter() - t0) * 1000.0

    sample = res.first.sample
    state = [int(sample.get(i, 0)) for i in range(n)]
    print(json.dumps(
        {"available": True, "state": state, "wall_ms": wall_ms,
         "energy": float(res.first.energy)}
    ))


if __name__ == "__main__":
    main()
