"""Exact integer substitution with local/global certified product penalties.

Experimental compiler only. No stochastic search, float conversion or solver API.
"""
from collections import Counter
from itertools import combinations


def canonical(n, terms):
    if type(n) is not int or n < 1:
        raise ValueError('n must be a positive integer')
    poly = {}
    for weight, variables in terms:
        if type(weight) is not int:
            raise ValueError('integer coefficients required')
        if not isinstance(variables, (list, tuple)):
            raise ValueError('variable indices must be a list or tuple')
        if any(type(v) is not int or not 0 <= v < n for v in variables):
            raise ValueError('invalid variable index')
        key = tuple(sorted(variables))
        if len(key) > 4 or len(set(key)) != len(key):
            raise ValueError('distinct indices and degree <=4 required')
        poly[key] = poly.get(key, 0) + weight
    return {key: value for key, value in sorted(poly.items()) if value}


def lower(n, terms, mode='local'):
    if mode not in ('local', 'global'):
        raise ValueError('unknown penalty mode')
    poly = canonical(n, terms)
    global_m = 1 + sum(abs(w) for term, w in poly.items() if len(term) > 2)
    steps = []
    next_id = n
    while any(len(term) > 2 for term in poly):
        counts = Counter(pair for term in poly if len(term) > 2
                         for pair in combinations(term, 2))
        u, v = min(counts, key=lambda pair: (-counts[pair], pair))
        affected = [(term, w) for term, w in sorted(poly.items())
                    if len(term) > 2 and u in term and v in term]
        positive = sum(w for _, w in affected if w > 0)
        negative = sum(-w for _, w in affected if w < 0)
        local_m = 1 + max(positive, negative)
        if local_m > global_m:
            raise ValueError('higher-order coefficient bound increased')
        penalty = local_m if mode == 'local' else global_m
        z = next_id
        for term, weight in affected:
            del poly[term]
            key = tuple(sorted([i for i in term if i not in (u, v)] + [z]))
            poly[key] = poly.get(key, 0) + weight
        for key, weight in [((u, v), penalty), ((u, z), -2*penalty),
                            ((v, z), -2*penalty), ((z,), 3*penalty)]:
            poly[key] = poly.get(key, 0) + weight
        poly = {key: w for key, w in sorted(poly.items()) if w}
        steps.append({'u': u, 'v': v, 'z': z, 'penalty': penalty,
                      'positive': positive, 'negative': negative,
                      'affected': [[w, list(term)] for term, w in affected]})
        next_id += 1
    return {'n': next_id, 'original_n': n, 'mode': mode, 'global_penalty': global_m,
            'terms': [[w, list(term)] for term, w in sorted(poly.items())], 'steps': steps}
