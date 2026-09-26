#!/usr/bin/env python3
"""Exact algebraic triage witnesses; not a solver-performance experiment.

Run without arguments to replay. --output creates a new JSON record exclusively.
Uses only the Python standard library and integer/rational arithmetic.
"""

import argparse
import hashlib
import itertools
import json
from pathlib import Path
import subprocess


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def transitivity():
    rows = []
    for a, b, c in itertools.product((0, 1), repeat=3):
        violations = int(a and b and not c) + int(a and c and not b) + int(b and c and not a)
        penalty = a*b + a*c + b*c - 3*a*b*c
        require(penalty == violations, "triangle polynomial differs from logical relation")
        rows.append({"bits": [a, b, c], "penalty": penalty, "energy": -2*a-2*b+3*c+8*penalty})
    require(sum(row["penalty"] == 0 for row in rows) == 5, "not five partitions")
    best = min(row["energy"] for row in rows)
    winners = [row["bits"] for row in rows if row["energy"] == best]
    require(best == -2 and winners == [[0, 1, 0], [1, 0, 0]], "wrong exact partition")
    return {"states": 8, "minimum": best, "winners": winners, "rows": rows}


def rounding():
    rows = []
    for q0, q1 in itertools.product((0, 1), repeat=2):
        direct = (50*(q0+q1)-49)**2
        polynomial = 2401 - 2400*q0 - 2400*q1 + 5000*q0*q1
        require(direct == polynomial, "rounding QUBO disagrees with reconstruction")
        rows.append({"bits": [q0, q1], "loss_times_2500": direct})
    require(rows[0]["loss_times_2500"] == 2401 and min(r["loss_times_2500"] for r in rows) == 1,
            "correlated rounding witness failed")
    return {"states": 4, "rows": rows, "scope": "two fixed weights 0.49, input (1,1), grid {0,1}"}


def sat_clause():
    # Independently expand each signed clause's product of false-literal factors.
    count = 0
    for signs in itertools.product((False, True), repeat=3):
        poly = {(): 1}
        for i, positive in enumerate(signs):
            factor = {(): 1, (i,): -1} if positive else {(i,): 1}
            expanded = {}
            for term, coefficient in poly.items():
                for other, scale in factor.items():
                    key = tuple(sorted(set(term + other)))
                    expanded[key] = expanded.get(key, 0) + coefficient*scale
            poly = expanded
        for bits in itertools.product((0, 1), repeat=3):
            energy = sum(coefficient * all(bits[i] for i in term) for term, coefficient in poly.items())
            unsatisfied = not any(bool(bits[i]) == signs[i] for i in range(3))
            require(energy == int(unsatisfied), "clause expansion changes SAT semantics")
            count += 1
    return {"signed_clauses": 8, "state_checks": count, "scope": "distinct literals, degree three"}


def qec_degeneracy():
    # Conditional X-only channel given syndrome (0,1), probabilities in twentieths.
    masses = {0b100: 7, 0b010: 1, 0b001: 6, 0b111: 6}
    stabilizer_x = 0b110
    require((stabilizer_x & 0b111).bit_count() % 2 == 0, "stabilizers do not commute")
    require(sum(masses.values()) == 20, "conditional probabilities not normalized")
    require(all(error.bit_count() % 2 == 1 for error in masses), "wrong ZZZ syndrome")
    classes = {}
    for error, mass in masses.items():
        label = min(error, error ^ stabilizer_x)
        classes[label] = classes.get(label, 0) + mass
    best_error = max(masses, key=masses.get)
    map_class = min(best_error, best_error ^ stabilizer_x)
    ml_class = max(classes, key=classes.get)
    require(map_class != ml_class and classes[map_class] == 8 and classes[ml_class] == 12,
            "MAP versus logical-ML counterexample failed")
    return {"map_error": format(best_error, "03b"), "map_class_mass": "2/5",
            "logical_ml_class_mass": "3/5", "scope": "[[3,1]] distance-one toy, not threshold evidence"}


def cd003_precision():
    rows = []
    for k in range(5, 9):
        b = k*k
        # Every boundary coefficient is nonzero: powers of two, then b-k ones.
        boundary = [1 << i for i in range(k)] + [1]*(b-k)
        fields = {0}
        for coefficient in boundary:
            fields |= {theta + coefficient for theta in fields}
        expected = (1 << k) + b-k
        require(fields == set(range(expected)), "not all claimed fields attainable")
        # u=(1,2,...,2^k): every k+1-bit internal state has distinct value q.
        internal_values = range(1 << (k+1))
        for theta in sorted(fields):
            energies = [(q-theta)**2 for q in internal_values]
            require(min(energies) == 0 and energies.count(0) == 1 and energies[theta] == 0,
                    "conditional response not uniquely theta")
        rows.append({"k": k, "boundary_size": b, "factor_precision_log2K": k-1,
                     "interface_rank": 1, "unique_responses": expected,
                     "energy_evaluations": expected*len(internal_values)})
    return {"rows": rows, "energy_evaluations": sum(r["energy_evaluations"] for r in rows),
            "scope": "finite witnesses; asymptotic argument is in HYPODIVE_TRIAGE.md"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    source = Path(__file__).resolve()
    root = source.parents[2]
    report = {"schema": 1, "kind": "deterministic algebraic checks; no application benchmark",
              "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
              "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
              "checks": {"transitivity": transitivity(), "rounding": rounding(),
                         "sat_clause": sat_clause(), "qec_degeneracy": qec_degeneracy(),
                         "cd003_precision": cd003_precision()}, "verdict": "PASS"}
    serialized = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        with args.output.open("x") as stream:
            stream.write(serialized)
    else:
        print(serialized, end="")


if __name__ == "__main__":
    main()
