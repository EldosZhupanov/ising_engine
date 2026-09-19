"""Exact, deliberately constructed CD001 witnesses; no speed or novelty claim."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import platform
import sys
import time
import unittest

HERE = Path(__file__).resolve().parent
FAMILIES = ('binary_square', 'unit_square', 'separable')
SIZES = tuple(range(1, 7))


def bits(mask, n):
    return tuple((mask >> i) & 1 for i in range(n))


def coefficients(family, b):
    a = tuple(1 if family == 'unit_square' else 1 << i for i in range(b))
    if family in ('binary_square', 'unit_square'):
        v = a + tuple(-w for w in a)
        unary = tuple(w*w for w in v)
        pairs = tuple((i, j, 2*v[i]*v[j]) for i in range(2*b) for j in range(i+1, 2*b))
    else:
        assert family == 'separable'
        unary = tuple(2*i+1 for i in range(b)) + (0,)*b
        pairs = tuple((i, b+j, -2*a[j]) for i in range(b) for j in range(b))
    return a, unary, pairs


def qubo(y, unary, pairs):
    return sum(h*v for h, v in zip(unary, y)) + sum(q*y[i]*y[j] for i, j, q in pairs)


def ising_coefficients(unary, pairs):
    # Exactly 4 * E_binary, s=2*x-1; independent native spin expression.
    constant = 2*sum(unary) + sum(q for _, _, q in pairs)
    fields = [2*h for h in unary]
    for i, j, q in pairs:
        fields[i] += q
        fields[j] += q
    return constant, fields, pairs


def native_ising(y, converted):
    constant, fields, pairs = converted
    spins = tuple(2*v-1 for v in y)
    return constant + sum(h*s for h, s in zip(fields, spins)) + sum(q*spins[i]*spins[j] for i, j, q in pairs)


def rank_one_interface(b, pairs):
    w = [[0]*b for _ in range(b)]
    for i, j, q in pairs:
        if i < b <= j:
            w[i][j-b] = q
    assert w[0][0] != 0
    assert all(w[i][j]*w[0][0] == w[i][0]*w[0][j] for i in range(b) for j in range(b))
    return 1


def inspect(family, b):
    a, unary, pairs = coefficients(family, b)
    converted = ising_coefficients(unary, pairs)
    states = [bits(mask, b) for mask in range(1 << b)]
    values = [sum(w*v for w, v in zip(a, x)) for x in states]
    responses, boundary_values, gaps = set(), set(), []
    unique_contexts = evaluations = 0
    for zm, z in enumerate(states):
        energies = []
        for xm, x in enumerate(states):
            direct = (values[xm]-values[zm])**2 if family != 'separable' else sum((2*i+1-2*values[zm])*x[i] for i in range(b))
            y = x+z
            assert qubo(y, unary, pairs) == direct
            assert native_ising(y, converted) == 4*direct
            energies.append(direct)
            evaluations += 1
        best = min(energies)
        winners = [i for i, e in enumerate(energies) if e == best]
        responses.add(winners[0])  # fixed smallest-mask tie break, not uniqueness
        boundary_values.add(values[zm])
        unique_contexts += len(winners) == 1
        gaps.append(sorted(energies)[1]-best)
        if family == 'binary_square':
            assert winners == [zm] and gaps[-1] == 1
        if family == 'unit_square':
            assert best == 0
        if family == 'separable':
            expected = sum((values[zm] > i) << i for i in range(b))
            assert winners == [expected]
    scale = max([abs(h) for h in unary]+[abs(q) for _, _, q in pairs])
    if family == 'binary_square':
        assert len(responses) == 1 << b
        assert scale == 1 << (2*b-1)
    else:
        assert len(responses) == b+1
    return dict(family=family, b=b, boundary_contexts=1 << b,
                interface_rank=rank_one_interface(b, pairs), distinct_scalar_inputs=len(boundary_values),
                distinct_chosen_responses=len(responses), unique_optimum_contexts=unique_contexts,
                minimum_gap=min(gaps), maximum_polynomial_coefficient=scale,
                normalized_minimum_gap=f'{min(gaps)}/{scale}',
                exhaustive_energy_evaluations=evaluations,
                closed_form_output_bits=b if family == 'binary_square' else 'NA')


class WitnessTests(unittest.TestCase):
    def test_three_independent_energies_and_nontrivial_ties(self):
        for family in FAMILIES:
            for b in (1, 2, 3):
                inspect(family, b)
        self.assertEqual(inspect('unit_square', 3)['unique_optimum_contexts'], 2)

    def test_normalization_does_not_hide_precision(self):
        row = inspect('binary_square', 4)
        self.assertEqual(row['normalized_minimum_gap'], '1/128')
        self.assertEqual(row['distinct_chosen_responses'], 16)

    def test_separable_and_coupled_rank_one_are_different(self):
        self.assertEqual(inspect('separable', 4)['distinct_chosen_responses'], 5)
        self.assertEqual(inspect('binary_square', 4)['distinct_chosen_responses'], 16)


def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def run():
    out = HERE/'cd001'
    if out.exists():
        raise RuntimeError('refusing existing witness output directory')
    out.mkdir()
    paths = (HERE/'witness.py', HERE/'MATH.md', HERE/'IMPLEMENTATION.md')
    manifest = {'kind':'deductive witness; construction already known before execution; not preregistration',
                'python':sys.version, 'platform':platform.platform(), 'families':FAMILIES, 'sizes':SIZES,
                'sources':{p.name:digest(p) for p in paths}}
    (out/'freeze.json').write_text(json.dumps(manifest, indent=2)+'\n')
    start = time.perf_counter()
    rows = []
    try:
        with (out/'rows.tsv').open('w') as f:
            writer = None
            for family in FAMILIES:
                for b in SIZES:
                    row = inspect(family, b)
                    if writer is None:
                        writer = csv.DictWriter(f, list(row), delimiter='\t', lineterminator='\n')
                        writer.writeheader()
                    writer.writerow(row); f.flush(); rows.append(row)
        assert {p.name:digest(p) for p in paths} == manifest['sources']
        result = {'rows':len(rows), 'energy_evaluations':sum(r['exhaustive_energy_evaluations'] for r in rows),
                  'verdict':'NO_GO_RANK_ONLY_RESPONSE_COUNT', 'elapsed_seconds_not_benchmark':time.perf_counter()-start,
                  'rows_sha256':digest(out/'rows.tsv'), 'novel_solver':False}
        (out/'complete.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result, indent=2))
    except Exception as exc:
        (out/'failure.txt').write_text(repr(exc)+'\n')
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--check', action='store_true')
    group.add_argument('--run', action='store_true')
    args = parser.parse_args()
    if args.check:
        unittest.main(argv=[sys.argv[0]])
    else:
        run()
