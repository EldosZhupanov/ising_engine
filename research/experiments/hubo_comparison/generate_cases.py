"""HUBO-C001 input generation only; no solver or optimum access."""
import itertools
import json
from pathlib import Path


class SplitMix:
    def __init__(self, seed):
        self.state = seed

    def next(self):
        mask = (1 << 64) - 1
        self.state = (self.state + 0x9E3779B97F4A7C15) & mask
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & mask
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & mask
        return z ^ (z >> 31)


def make_case(degree, seed, n=32):
    rng = SplitMix(seed)
    fields = [1 if rng.next() & 1 else -1 for _ in range(n)]
    edges = {}
    while len(edges) < 3 * n:
        variables = set()
        while len(variables) < degree:
            variables.add(rng.next() % n)
        key = tuple(sorted(variables))
        if key not in edges:
            edges[key] = 1 if rng.next() & 1 else -1
    spin_terms = [[w, list(v)] for v, w in sorted(edges.items())]
    spin_terms += [[w, [v]] for v, w in enumerate(fields)]
    poly = {}
    for weight, variables in spin_terms:
        for length in range(len(variables) + 1):
            for key in itertools.combinations(variables, length):
                poly[key] = poly.get(key, 0) + weight * 2**length * (-1)**(len(variables)-length)
    return {'id': f'd{degree}_{seed}', 'n': n, 'degree': degree,
            'instance_seed': seed, 'spin_terms': spin_terms,
            'terms': [[w, list(k)] for k, w in sorted(poly.items()) if w],
            'normalizer': sum(abs(w) for w, _ in spin_terms)}


if __name__ == '__main__':
    cases = [make_case(degree, seed) for degree, first in [(3, 930001), (4, 940001)]
             for seed in range(first, first+5)]
    out = Path(__file__).with_name('cases.json')
    if out.exists():
        raise SystemExit('refuse to overwrite frozen cases')
    out.write_text(json.dumps(cases, indent=2) + '\n')
