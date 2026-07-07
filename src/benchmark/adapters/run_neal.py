#!/usr/bin/env python3
"""dwave-neal bridge for the Ising Engine benchmark harness.

Runs neal.SimulatedAnnealingSampler at the matched sweep budget and emits the
same JSON contract as run_openjij.py. Prints {"available": false, ...} and
exits 0 when the library is absent, so the harness skips it honestly.
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
        import neal
    except Exception as exc:  # noqa: BLE001
        print(json.dumps({"available": False, "reason": "import neal: %s" % exc}))
        return

    n, sweeps, seed, Q = read_model(sys.argv[1])
    sampler = neal.SimulatedAnnealingSampler()
    t0 = time.perf_counter()
    try:
        res = sampler.sample_qubo(Q, num_sweeps=max(1, sweeps), num_reads=1, seed=seed)
    except TypeError:
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
