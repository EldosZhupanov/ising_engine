"""Independent Q002 Phase A audit; imports no project compiler or solver."""
import ast
import copy
import hashlib
import itertools
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

BASE = Path(__file__).resolve().parents[1]
ROOT = BASE.parents[2]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def add(poly, key, coefficient):
    value = poly.get(key, 0) + coefficient
    if value:
        poly[key] = value
    else:
        poly.pop(key, None)


def read_terms(records, limit, degree=4, canonical=False):
    require(type(limit) is int and limit > 0, 'invalid variable count')
    poly, keys = {}, []
    for coefficient, indices in records:
        require(type(coefficient) is int, 'noninteger coefficient')
        require(type(indices) in (list, tuple), 'invalid monomial container')
        require(all(type(i) is int and 0 <= i < limit for i in indices), 'invalid index')
        key = tuple(sorted(indices))
        require(len(key) <= degree and len(set(key)) == len(key), 'invalid degree/duplicate index')
        if canonical:
            require(coefficient != 0 and tuple(indices) == key and key not in poly,
                    'noncanonical certificate monomial')
        keys.append(key)
        add(poly, key, coefficient)
    if canonical:
        require(keys == sorted(keys), 'noncanonical certificate order')
    return poly


def selected_pair(poly):
    counts = {}
    for term in poly:
        if len(term) > 2:
            for i in range(len(term)):
                for j in range(i + 1, len(term)):
                    pair = (term[i], term[j])
                    counts[pair] = counts.get(pair, 0) + 1
    require(bool(counts), 'unexpected substitution after quadratic completion')
    largest = max(counts.values())
    return min(pair for pair, count in counts.items() if count == largest)


def substitute(poly, affected, u, v, z, penalty):
    result = {term: value for term, value in poly.items() if term not in affected}
    for term, value in affected.items():
        add(result, tuple(sorted(tuple(i for i in term if i != u and i != v) + (z,))), value)
    for term, factor in (((u, v), 1), ((u, z), -2), ((v, z), -2), ((z,), 3)):
        add(result, term, factor * penalty)
    return result


def objective(poly, bits):
    return sum(value for term, value in poly.items() if all(bits[i] == 1 for i in term))


def audit_model(n, original, certificate, masks):
    mode = certificate['mode']
    require(mode in ('local', 'global') and certificate['original_n'] == n, 'model identity')
    global_m = 1 + sum(abs(w) for term, w in original.items() if len(term) > 2)
    require(type(certificate['global_penalty']) is int and certificate['global_penalty'] == global_m,
            'global penalty metadata')
    current = dict(original)
    plans = []
    for index, step in enumerate(certificate['steps']):
        require(all(type(step[k]) is int for k in ('u', 'v', 'z', 'penalty', 'positive', 'negative')),
                'noninteger step field')
        u, v, z = step['u'], step['v'], step['z']
        require(0 <= u < v < z and z == n + index, 'nonfresh auxiliary or pair')
        require((u, v) == selected_pair(current), 'most-frequent/lex pair rule')
        affected = {term: value for term, value in current.items()
                    if len(term) > 2 and u in term and v in term}
        recorded = read_terms(step['affected'], z, canonical=True)
        require(recorded == affected and bool(affected), 'affected polynomial')
        p = sum(w for w in affected.values() if w > 0)
        q = -sum(w for w in affected.values() if w < 0)
        require((step['positive'], step['negative']) == (p, q), 'signed bounds')
        local_m = 1 + max(p, q)
        penalty = local_m if mode == 'local' else global_m
        require(step['penalty'] == penalty and 1 <= local_m <= global_m, 'penalty certificate')
        # g contains neither selected pair variable; both possible invalid-z gaps
        # are >= M-max(P,N) >= 1 for every old-variable assignment.
        require(penalty - p >= 1 and penalty - q >= 1, 'conditional minimum margin')
        previous_l1 = sum(abs(w) for term, w in current.items() if len(term) > 2)
        current = substitute(current, affected, u, v, z, penalty)
        require(sum(abs(w) for term, w in current.items() if len(term) > 2) <= previous_l1,
                'higher-order coefficient bound increased')
        plans.append((u, v, z, tuple(sorted(affected.items()))))
    require(type(certificate['n']) is int and certificate['n'] == n + len(plans), 'expanded size')
    require(current == read_terms(certificate['terms'], certificate['n'], 2, canonical=True),
            'final polynomial mismatch')
    require(all(len(term) <= 2 for term in current), 'unreduced term')
    require(sum(abs(w) for w in current.values()) < 2**53, 'f64 exact-energy bound')
    for mask in masks:
        bits = [(mask >> i) & 1 for i in range(n)]
        expected = objective(original, bits)
        for step in certificate['steps']:
            bits.append(bits[step['u']] * bits[step['v']])
        require(objective(current, bits) == expected, 'consistent extension mismatch')
    return plans


def fixtures_check():
    tree = ast.parse((BASE / 'test_lowering.py').read_text())
    fixtures = next(ast.literal_eval(node.value) for node in tree.body
                    if isinstance(node, ast.Assign)
                    and any(isinstance(t, ast.Name) and t.id == 'FIXTURES' for t in node.targets))
    original_count = expanded_count = 0
    for n, terms in fixtures:
        original = read_terms(terms, n)
        global_m = 1 + sum(abs(w) for term, w in original.items() if len(term) > 2)
        for mode in ('local', 'global'):
            poly, z = dict(original), n
            while any(len(term) > 2 for term in poly):
                u, v = selected_pair(poly)
                affected = {t: w for t, w in poly.items() if len(t) > 2 and u in t and v in t}
                p = sum(w for w in affected.values() if w > 0)
                q = -sum(w for w in affected.values() if w < 0)
                poly = substitute(poly, affected, u, v, z,
                                  1 + max(p, q) if mode == 'local' else global_m)
                z += 1
            for bits in itertools.product((0, 1), repeat=n):
                outcomes = [objective(poly, bits + extra)
                            for extra in itertools.product((0, 1), repeat=z-n)]
                expected = objective(original, bits)
                require(min(outcomes) == expected and outcomes.count(expected) == 1,
                        'independent fixture minimum')
            original_count += 2**n
            expanded_count += 2**z
    return original_count, expanded_count


def main():
    output = Path(__file__).with_name('audit_phase_a.json')
    require(not output.exists(), 'refuse to overwrite independent audit')
    raw_bytes = (BASE / 'phase_a.json').read_bytes()
    raw = json.loads(raw_bytes)
    require(raw['status'] == 'PASS' and raw['phase_b'] == 'NOT_RUN', 'Phase A status/scope')
    source_commit = raw['source_commit']
    require(source_commit == '20176618178b7c4839a35747918881b8235d3fd5', 'unexpected source freeze')
    source_checks = {}
    for name, expected in raw['source_sha256'].items():
        path = BASE / name
        frozen = subprocess.check_output(['git', 'show', source_commit + ':' + str(path.relative_to(ROOT))], cwd=ROOT)
        require(sha(path.read_bytes()) == sha(frozen) == expected, 'source hash mismatch: ' + name)
        source_checks[name] = expected
    manifest = json.loads((BASE / 'inputs.json').read_text())
    cases_bytes = (BASE / 'cases.json').read_bytes()
    require(sha(cases_bytes) == manifest['cases_sha256'], 'case-file hash')
    generator = BASE.parent / 'hubo_comparison/generate_cases.py'
    frozen_generator = subprocess.check_output(['git', 'show', source_commit + ':' + str(generator.relative_to(ROOT))], cwd=ROOT)
    require(sha(generator.read_bytes()) == sha(frozen_generator) == manifest['generator_sha256'], 'generator hash')
    cases = json.loads(cases_bytes)
    require(len(cases) == len(manifest['instances']) == len(raw['static_cases']) == 12, 'case count')
    rows = []
    for case, entry, record in zip(cases, manifest['instances'], raw['static_cases']):
        require(case['id'] == record['id'] == entry['id'] and sha(encoded(case)) == entry['sha256'], 'case identity/hash')
        for key in ('n', 'degree', 'instance_seed', 'normalizer'):
            require(case[key] == entry[key], 'case metadata: ' + key)
        n = case['n']
        original = read_terms(case['terms'], n)
        spin_expansion = {}
        for coefficient, indices in case['spin_terms']:
            for size in range(len(indices) + 1):
                for subset in itertools.combinations(indices, size):
                    add(spin_expansion, tuple(sorted(subset)), coefficient * 2**size * (-1)**(len(indices)-size))
        require(original == spin_expansion, 'exact spin-to-binary expansion')
        masks = [0, (1 << n)-1, int('10' * ((n+1)//2), 2), int('01' * ((n+1)//2), 2)]
        masks += [int.from_bytes(hashlib.sha256((case['id'] + ':' + str(i)).encode()).digest(), 'big') for i in range(8)]
        plans = {}
        for mode in ('local', 'global'):
            certificate = record['certificates'][mode]
            require(certificate['mode'] == mode, 'mode label')
            require(sha(json.dumps(certificate, sort_keys=True).encode()) == record[mode + '_sha256'], 'certificate hash')
            plans[mode] = audit_model(n, original, certificate, masks)
        require(plans['local'] == plans['global'], 'local/global pair-plan mismatch')
        local, glob = record['certificates']['local'], record['certificates']['global']
        penalties = [step['penalty'] for step in local['steps']]
        expected_fields = {'original_n': n, 'expanded_n': local['n'], 'products': len(penalties),
                           'global_penalty': local['global_penalty'], 'local_penalty_min': min(penalties),
                           'local_penalty_max': max(penalties), 'local_terms': len(local['terms']),
                           'global_terms': len(glob['terms']), 'nested_products': sum(s['v'] >= n for s in local['steps'])}
        require(all(record[k] == v for k, v in expected_fields.items()), 'summary metadata mismatch')
        rows.append({'id': case['id'], **expected_fields, 'models_verified': 2, 'extensions_checked': 24})
        print(json.dumps({'id': case['id'], 'status': 'PASS'}), flush=True)
    fixture_counts = fixtures_check()
    require(fixture_counts == (raw['exhaustive_original_checks_both_modes'], raw['exhaustive_expanded_states_both_modes']), 'fixture counts')
    first, record = cases[0], raw['static_cases'][0]
    original = read_terms(first['terms'], first['n'])
    rejected = []
    for mutation in ('duplicate_final_term', 'incorrect_penalty', 'wrong_pair'):
        certificate = copy.deepcopy(record['certificates']['local'])
        if mutation == 'duplicate_final_term':
            certificate['terms'].append(copy.deepcopy(certificate['terms'][0]))
        elif mutation == 'incorrect_penalty':
            certificate['steps'][0]['penalty'] -= 1
        else:
            certificate['steps'][0]['u'], certificate['steps'][0]['v'] = certificate['steps'][0]['v'], certificate['steps'][0]['u']
        try:
            audit_model(first['n'], original, certificate, [])
        except ValueError:
            rejected.append(mutation)
        else:
            raise ValueError('mutation accepted: ' + mutation)
    report = {'status': 'PASS', 'scope': 'Independent exact-algebra/static-certificate audit; no compiler import or stochastic solver call',
              'phase_b_in_source_artifact': raw['phase_b'], 'source_commit': source_commit,
              'phase_a_sha256': sha(raw_bytes), 'auditor_sha256': sha(Path(__file__).read_bytes()),
              'audit_time_utc': datetime.now(timezone.utc).isoformat(), 'source_hashes_verified': source_checks,
              'generator_sha256_verified': manifest['generator_sha256'], 'cases_verified': len(rows),
              'models_verified': 2*len(rows), 'substitution_steps_verified': 2*sum(r['products'] for r in rows),
              'consistent_extensions_verified': 24*len(rows), 'exact_spin_expansions_verified': len(rows),
              'exhaustive_fixture_original_checks_both_modes': fixture_counts[0],
              'exhaustive_fixture_expanded_states_both_modes': fixture_counts[1],
              'mutation_checks_rejected': rejected, 'cases': rows}
    require(sha((BASE / 'phase_a.json').read_bytes()) == sha(raw_bytes), 'source artifact changed during audit')
    with output.open('x') as stream:
        stream.write(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'status': 'PASS', 'models_verified': report['models_verified'], 'audit_file': str(output)}))


if __name__ == '__main__':
    main()
