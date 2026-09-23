"""Canonical LAYA-001 data, encoding, baseline and verification functions (stdlib)."""
import itertools
import math
import random

POSITIVE = (
    'The {job} operation is running now.',
    'The {job} operation has started and is still in progress.',
    'The {job} operation was paused earlier, but it has resumed and is running now.',
    'The plan was to cancel {job}, but the cancellation was reversed; it is running now.',
)
NEGATIVE = (
    'The {job} operation has not started yet.',
    'The {job} operation finished yesterday and is not running now.',
    'The {job} operation was running earlier, but it has been stopped.',
    'The {job} operation is scheduled for tomorrow; it is not running now.',
)


def states(n):
    return [[(mask >> i) & 1 for i in range(n)] for mask in range(1 << n)]


def violations(x, rules):
    return sum(x[i] * (x[j] if kind == 'exclude' else 1 - x[j])
               for kind, i, j in rules)


def corpus():
    groups = []
    for group in range(24):
        rng = random.Random(9242300 + group)
        rules = []
        for i, j in itertools.combinations(range(8), 2):
            if rng.random() < (0.20 if group % 2 == 0 else 0.45):
                rules.append(('exclude' if rng.random() < 0.6 else 'implies', i, j))
        truth = rng.choice([x for x in states(8) if violations(x, rules) == 0])
        messages = [rng.choice(POSITIVE if bit else NEGATIVE).format(job=f'job_{i}')
                    for i, bit in enumerate(truth)]
        groups.append({'id': group, 'seed': 9242300 + group, 'truth': truth,
                       'messages': messages, 'rules': rules})
    return groups


def probabilities(values):
    if not values or len(values) > 16:
        raise ValueError('require 1..16 probabilities')
    if any(not isinstance(p, (int, float)) or isinstance(p, bool)
           or not math.isfinite(p) or p < 0 or p > 1 for p in values):
        raise ValueError('probability must be finite in [0,1]')
    return [min(1 - 1e-4, max(1e-4, p)) for p in values]


def encode(values, rules, seed):
    p = probabilities(values)
    h = [math.log((1 - v) / v) for v in p]
    penalty = 1 + sum(abs(v) for v in h)
    pairs = []
    seen = set()
    for kind, i, j in rules:
        if kind not in ('exclude', 'implies') or not (0 <= i < j < len(p)) or (i, j) in seen:
            raise ValueError('invalid or duplicate rule')
        seen.add((i, j))
        if kind == 'implies':
            h[i] += penalty
        pairs.append((i, j, penalty if kind == 'exclude' else -penalty))
    return {'linear': h, 'pairs': pairs, 'offset': -sum(math.log(1-v) for v in p),
            'seed': seed}, penalty


def direct_energy(x, p, rules, penalty):
    return -sum(math.log(v if b else 1-v) for b, v in zip(x, p)) + penalty * violations(x, rules)


def qubo_energy(x, req):
    return req['offset'] + sum(a*b for a, b in zip(x, req['linear'])) + sum(w*x[i]*x[j] for i,j,w in req['pairs'])


def greedy(p, rules):
    x = [0] * len(p)
    for i in sorted(range(len(p)), key=lambda i: (-p[i], i)):
        if p[i] > 0.5:
            x[i] = 1
            if violations(x, rules):
                x[i] = 0
    return x


def verify_and_score(group, values, bridge):
    p = probabilities(values)
    req, penalty = encode(values, group['rules'], 1000 + group['id'])
    spectrum = [direct_energy(x, p, group['rules'], penalty) for x in states(len(p))]
    if len(bridge['energy_spectrum']) != len(spectrum):
        raise ValueError('spectrum length mismatch')
    for value in bridge['energy_spectrum']:
        require_finite(value)
    if any(abs(a-b) > 1e-8 for a,b in zip(spectrum, bridge['energy_spectrum'])):
        raise ValueError('cross-language encoding mismatch')
    optimum = min(spectrum)
    for arm in ('exact', 'reduced_exact', 'ultimate'):
        x = bridge[arm]['state']
        if len(x) != len(p) or any(type(v) is not int or v not in (0,1) for v in x):
            raise ValueError('invalid returned assignment')
        require_finite(bridge[arm]['energy'])
        if abs(direct_energy(x, p, group['rules'], penalty) - bridge[arm]['energy']) > 1e-8:
            raise ValueError('solver energy mismatch')
    for arm in ('exact', 'reduced_exact'):
        if abs(bridge[arm]['energy'] - optimum) > 1e-8:
            raise ValueError('exact optimum mismatch')
    raw = [int(v >= 0.5) for v in p]
    arms = {'raw': raw, 'all_zero': [0]*len(p), 'greedy': greedy(p, group['rules'])}
    arms.update({a: bridge[a]['state'] for a in ('exact', 'ultimate', 'reduced_exact')})
    truth = group['truth']
    scores = {}
    for name, x in arms.items():
        scores[name] = {'state': x, 'correct': sum(a==b for a,b in zip(x,truth)),
                        'group_correct': x == truth, 'violations': violations(x,group['rules']),
                        'recovered': sum(r != t and a == t for r,a,t in zip(raw,x,truth)),
                        'damaged': sum(r == t and a != t for r,a,t in zip(raw,x,truth)),
                        'energy_gap': direct_energy(x,p,group['rules'],penalty) - optimum}
    return scores


def require_finite(value):
    if isinstance(value, bool) or not isinstance(value, (float, int)) or not math.isfinite(value):
        raise ValueError('nonfinite or nonnumeric energy')
