"""Phase A static certificates only: no annealing or optimization outcomes."""
import hashlib
import json
import platform
from pathlib import Path
import subprocess
import sys

from lowering import lower
from test_lowering import FIXTURES, energy

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SOURCE_NAMES = ('lowering.py', 'test_lowering.py', 'verify_phase_a.py',
                'prepare_inputs.py', 'cases.json', 'inputs.json', 'protocol.md')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_record(terms, variable_limit, maximum_degree=4):
    keys = []
    result = {}
    for weight, variables in terms:
        assert type(weight) is int and weight != 0
        assert isinstance(variables, (list, tuple))
        assert all(type(v) is int and 0 <= v < variable_limit for v in variables)
        key = tuple(variables)
        assert len(key) <= maximum_degree and key == tuple(sorted(set(key)))
        assert key not in result, 'duplicate certificate monomial'
        keys.append(key)
        result[key] = weight
    assert keys == sorted(keys), 'noncanonical certificate order'
    return result


def check_certificate(n, terms, result):
    # Replay the algebra independently rather than trusting compiler energies.
    assert result['original_n'] == n and result['mode'] in ('local', 'global')
    poly = {}
    for w, vs in terms:
        key = tuple(sorted(vs))
        poly[key] = poly.get(key, 0) + w
    poly = {t: w for t, w in poly.items() if w}
    global_m = 1 + sum(abs(w) for t, w in poly.items() if len(t) > 2)
    for index, step in enumerate(result['steps']):
        u, v, z = (step[k] for k in ('u', 'v', 'z'))
        assert all(type(step[k]) is int for k in ('u', 'v', 'z', 'penalty', 'positive', 'negative'))
        assert 0 <= u < v < z and z == n+index
        affected = {t: w for t, w in poly.items() if len(t) > 2 and u in t and v in t}
        assert affected and affected == canonical_record(step['affected'], z)
        p = sum(max(0, w) for w in affected.values())
        q = sum(max(0, -w) for w in affected.values())
        assert (step['positive'], step['negative']) == (p, q)
        expected = 1+max(p, q) if result['mode'] == 'local' else global_m
        assert step['penalty'] == expected and expected <= global_m
        for term, weight in affected.items():
            del poly[term]
            new_term = tuple(sorted(set(term)-{u, v} | {z}))
            poly[new_term] = poly.get(new_term, 0) + weight
        for term, multiplier in [((u, v), 1), ((u, z), -2), ((v, z), -2), ((z,), 3)]:
            poly[term] = poly.get(term, 0) + expected*multiplier
        poly = {t: w for t, w in poly.items() if w}
    assert poly == canonical_record(result['terms'], result['n'], 2)
    assert all(len(t) <= 2 for t in poly)
    assert result['n'] == n+len(result['steps']) and result['global_penalty'] == global_m
    # Sufficient conservative bound for exact integer f64 energies if later used.
    assert sum(abs(w) for w in poly.values()) < 2**53
    for mask in (0, (1<<n)-1, 0xAAAAAAAAAAAAAAAA, 0x123456789ABCDEF):
        bits = [(mask>>i)&1 for i in range(n)]
        original = energy(terms, bits)
        for step in result['steps']:
            bits.append(bits[step['u']]*bits[step['v']])
        assert energy(result['terms'], bits) == original


def main():
    if not __debug__:
        raise SystemExit('assertions must be enabled')
    output = HERE/'phase_a.json'
    if output.exists():
        raise SystemExit('refuse to overwrite Phase A record')
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    hashes = {name: digest(HERE/name) for name in SOURCE_NAMES}
    for name, sha in hashes.items():
        rel = str((HERE/name).relative_to(ROOT))
        data = subprocess.check_output(['git', 'show', commit+':'+rel], cwd=ROOT)
        if hashlib.sha256(data).hexdigest() != sha:
            raise ValueError('source not frozen: '+name)
    tests = subprocess.run([sys.executable, '-m', 'unittest', 'discover', '-s', str(HERE), '-p', 'test_*.py'],
                           capture_output=True, text=True)
    if tests.returncode:
        raise RuntimeError(tests.stdout+tests.stderr)
    expanded = original = 0
    for n, terms in FIXTURES:
        for mode in ('local', 'global'):
            result = lower(n, terms, mode)
            check_certificate(n, terms, result)
            original += 2**n
            expanded += 2**result['n']
    manifest = json.loads((HERE/'inputs.json').read_text())
    assert digest(HERE/'cases.json') == manifest['cases_sha256']
    rows = []
    for case in json.loads((HERE/'cases.json').read_text()):
        local = lower(case['n'], case['terms'])
        glob = lower(case['n'], case['terms'], 'global')
        for result in (local, glob):
            check_certificate(case['n'], case['terms'], result)
        plan = lambda r: [(s['u'], s['v'], s['z'], s['affected']) for s in r['steps']]
        assert plan(local) == plan(glob)
        penalties = [s['penalty'] for s in local['steps']]
        rows.append({'id': case['id'], 'original_n': case['n'], 'expanded_n': local['n'],
                     'products': len(penalties), 'global_penalty': local['global_penalty'],
                     'local_penalty_min': min(penalties), 'local_penalty_max': max(penalties),
                     'local_terms': len(local['terms']), 'global_terms': len(glob['terms']),
                     'nested_products': sum(s['v'] >= case['n'] for s in local['steps']),
                     'certificates': {'local': local, 'global': glob},
                     'local_sha256': hashlib.sha256(json.dumps(local, sort_keys=True).encode()).hexdigest(),
                     'global_sha256': hashlib.sha256(json.dumps(glob, sort_keys=True).encode()).hexdigest()})
    output.write_text(json.dumps({'status': 'PASS', 'phase_b': 'NOT_RUN', 'source_commit': commit,
                                 'source_sha256': hashes, 'python': sys.version, 'platform': platform.platform(),
                                 'test_stdout': tests.stdout, 'test_stderr': tests.stderr,
                                 'exhaustive_original_checks_both_modes': original,
                                 'exhaustive_expanded_states_both_modes': expanded,
                                 'static_cases': rows,
                                 'scope': 'Exact reduction checks and static sizes; no search or performance claim'},
                                indent=2) + '\n')
    print(json.dumps({'status': 'PASS', 'static_cases': len(rows), 'phase_b': 'NOT_RUN'}))


if __name__ == '__main__':
    main()
