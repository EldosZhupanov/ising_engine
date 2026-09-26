"""Read-only HUBO-C001 audit; no optimizer calls or imports of campaign/worker.

Run with PYTHONHASHSEED=0 benchmark-env/bin/python3 <this file>.
Only audit/result.json is written. dimod is used solely to reconstruct the
registered deterministic reduction, not to search for a solution.
"""
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
from fractions import Fraction

import dimod

HERE = Path(__file__).resolve().parent
EXP = HERE.parent
ROOT = HERE.parents[3]
ARMS = ('msc_native', 'msc_quad', 'oj_native', 'oj_quad')
SEEDS = list(range(950001, 950011))
FREEZE = '23d300f545ce027e7ad56eac7e895e920b804712'
CASE_SHA = '79026f9af3fe1846129da52f28e214b77be12fac7548e17ef96bed7330cde1a9'


def check(condition, detail):
    if not condition:
        raise ValueError(detail)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def expand(spin_terms):
    coefficients = {}
    for weight, variables in spin_terms:
        for k in range(len(variables) + 1):
            for subset in itertools.combinations(variables, k):
                coefficients[subset] = coefficients.get(subset, 0) + weight * 2**k * (-1)**(len(variables)-k)
    return {key: value for key, value in coefficients.items() if value}


def energy(case, bits):
    check(isinstance(bits, str) and len(bits) == case['n'] and set(bits) <= {'0', '1'}, 'nonbinary witness')
    binary = [int(bit) for bit in bits]
    total = 0
    for weight, variables in case['spin_terms']:
        term = weight
        for index in variables:
            term *= 2*binary[index]-1
        total += term
    expanded = sum(weight for weight, variables in case['terms'] if all(binary[i] for i in variables))
    check(total == expanded, 'spin/expanded energy mismatch')
    return total


def dist(values):
    ordered = sorted(values)
    def quartile(f):
        position = (len(ordered)-1)*f
        i = math.floor(position)
        return ordered[i] + (ordered[math.ceil(position)]-ordered[i])*(position-i)
    return dict(mean=statistics.mean(values), median=statistics.median(values), sd=statistics.stdev(values),
                minimum=min(values), maximum=max(values), q25=quartile(.25), q75=quartile(.75),
                iqr=quartile(.75)-quartile(.25))


def exact_p(values):
    nonzero = [v for v in values if v]
    magnitudes = sorted(abs(v) for v in nonzero)
    ranks = []
    for value in nonzero:
        positions = [i+1 for i, magnitude in enumerate(magnitudes) if magnitude == abs(value)]
        ranks.append(Fraction(sum(positions), len(positions)))
    observed = abs(sum(rank if value > 0 else -rank for rank, value in zip(ranks, nonzero)))
    extreme = sum(abs(sum(sign*rank for sign, rank in zip(signs, ranks))) >= observed
                  for signs in itertools.product((-1, 1), repeat=len(ranks)))
    return Fraction(extreme, 2**len(ranks))


def same(actual, expected, path='summary'):
    if isinstance(expected, dict):
        check(isinstance(actual, dict) and actual.keys() == expected.keys(), path+' keys')
        for key in expected:
            same(actual[key], expected[key], path+'.'+key)
    elif isinstance(expected, list):
        check(isinstance(actual, list) and len(actual) == len(expected), path+' length')
        for i, (a, b) in enumerate(zip(actual, expected)):
            same(a, b, path+f'[{i}]')
    elif isinstance(expected, (int, float)) and not isinstance(expected, bool):
        check(type(actual) in (int, float) and math.isfinite(actual)
              and math.isclose(actual, expected, rel_tol=1e-13, abs_tol=1e-13), path+' number')
    else:
        check(actual == expected, path+' value')


def provenance(directory):
    env = read(directory/'environment.json')
    check(env['commit'] == FREEZE, 'wrong source freeze')
    names = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', FREEZE, '--',
             'src', 'Cargo.toml', 'Cargo.lock', '.cargo/config.toml', 'research/Cargo.toml',
             'research/examples/hubo_compare.rs', 'research/experiments/hubo_comparison'], cwd=ROOT, text=True).splitlines()
    check(set(env['sources']) == {p for p in names if '/run/' not in p and '/smoke/' not in p}, 'incomplete source set')
    for name, digest in env['sources'].items():
        blob = subprocess.check_output(['git', 'cat-file', 'blob', FREEZE+':'+name], cwd=ROOT)
        check(hashlib.sha256(blob).hexdigest() == digest == sha(ROOT/name), 'source hash '+name)
    for name, digest in env['baseline_hashes'].items():
        check(sha(name) == digest, 'baseline hash '+name)
    check(sha(env['engine']) == env['engine_sha256'], 'engine hash')
    check(sha(env['python']) == env['python_binary_sha256'], 'Python executable hash')
    check(env['packages'] == {'dimod': '0.12.22', 'numpy': '2.5.1', 'openjij': '0.12.0'}, 'package versions')
    check(read(directory/'complete.json')['commit'] == FREEZE, 'completion/source disagreement')
    return env


def representations(case):
    terms = case['terms']
    check({tuple(v): w for w, v in terms} == expand(case['spin_terms']), 'symbolic expansion mismatch')
    original = {tuple(v): w for w, v in terms}
    penalty = 1 + sum(abs(w) for w, v in terms if len(v) > 2)
    quadratic = dimod.make_quadratic(original, penalty, dimod.BINARY)
    for v in range(case['n']):
        quadratic.add_variable(v, 0)
    incident = [0]*case['n']
    for weight, variables in terms:
        for v in variables:
            incident[v] += abs(weight)
    qincident = {v: abs(w) for v, w in quadratic.linear.items()}
    for (u, v), weight in quadratic.quadratic.items():
        qincident[u] += abs(weight)
        qincident[v] += abs(weight)
    native = {'model_n': case['n'], 'model_terms': len(terms), 'penalty': 0,
              'hot': max(1, max(incident))/math.log(2)}
    quad = {'model_n': len(quadratic.variables), 'penalty': penalty,
            'model_terms': 1+sum(bool(w) for w in quadratic.linear.values())+sum(bool(w) for w in quadratic.quadratic.values()),
            'hot': max(1, max(qincident.values()))/math.log(2)}
    return native, quad, quadratic


def extension_energy(case, bits, quadratic):
    assignment = dict(enumerate(map(int, bits)))
    remaining = list(quadratic.info['reduction'].items())
    while remaining:
        ready = [(pair, info) for pair, info in remaining if all(v in assignment for v in pair)]
        check(bool(ready), 'cyclic/missing auxiliary definition')
        for (u, v), info in ready:
            assignment[info['product']] = assignment[u]*assignment[v]
        remaining = [item for item in remaining if item not in ready]
    value = quadratic.offset
    value += sum(weight*assignment[v] for v, weight in quadratic.linear.items())
    value += sum(weight*assignment[u]*assignment[v] for (u, v), weight in quadratic.quadratic.items())
    check(value == energy(case, bits), 'consistent quadratic extension differs from original')


def audit(part, cases, seeds):
    directory = EXP/part
    env = provenance(directory)
    check(env['smoke'] is (part == 'smoke'), 'smoke flag')
    expected = {f"{c['id']}_{s}_{a}.json" for c in cases for s in seeds for a in ARMS}
    actual = {p.name for p in directory.glob('*.json')} - {'environment.json', 'complete.json', 'summary.json'}
    check(actual == expected and read(directory/'complete.json')['cells'] == len(expected), 'cell set/completion')
    check({p.stem for p in directory.glob('*.sol')} == {Path(n).stem for n in expected}, 'witness set')
    models = {c['id']: representations(c) for c in cases}
    endpoints = {}; rows = []; total = 0; late = 0
    for case in cases:
        for seed in seeds:
            for arm in ARMS:
                path = directory/f"{case['id']}_{seed}_{arm}.json"
                row = read(path)
                archived_worker = row['command'][1]
                check(Path(archived_worker).parts[-4:] == ('research', 'experiments', 'hubo_comparison', 'worker.py'), 'worker path suffix')
                command = [env['python'], archived_worker, '--arm', arm, '--seed', str(seed), '--engine', env['engine']]
                check(row['command'] == command, 'command mismatch')
                check(not row['errors'] and row['returncode'] in (-15, -9) and not row['stderr'], 'error/exit/stderr')
                check(row['budget_s'] == 2 and 0 <= row['startup_s'] <= 30, 'budget/startup')
                check(2 <= row['stop_s'] <= row['termination_s'] <= 2.2, 'cutoff/termination')
                complete_lines = row['stdout'].split('\n')[:-1]
                check(complete_lines[0] == 'READY', 'READY missing')
                check([json.loads(line) for line in complete_lines[1:]] == [i['event'] for i in row['events']], 'stdout/events disagreement')
                times = [i['received_s'] for i in row['events']]
                check(times == sorted(times), 'nonmonotonic receipt time')
                candidates = []; meta = []
                for item in row['events']:
                    event = item['event']; received = item['received_s']
                    check(math.isfinite(received) and 0 <= received <= row['termination_s'], 'receipt time invalid')
                    if event['kind'] == 'inc':
                        check(type(event['energy']) is int and energy(case, event['bits']) == event['energy'], 'incumbent energy invalid')
                        check(type(event['steps']) is int and event['steps'] >= 0, 'work counter invalid')
                        if arm.startswith('oj_'):
                            check(event['steps'] % 1000 == 0, 'OpenJij work counter')
                        total += 1
                        if received <= 2:
                            candidates.append((event['energy'], event['bits'], received))
                        else:
                            late += 1
                    elif event['kind'] == 'meta':
                        meta.append(event)
                        check(0 <= event['transform_s'] <= received, 'transform duration invalid')
                    else:
                        raise ValueError('invalid complete event')
                first = row['events'][0]['event']
                check(first['kind'] == 'inc' and first['bits'] == '0'*case['n'] and first['steps'] == 0, 'fallback mismatch')
                check(len(meta) == 1 and candidates, 'metadata/endpoint missing')
                expected_meta = models[case['id']][int(arm.endswith('_quad'))]
                same({k: meta[0][k] for k in expected_meta}, expected_meta, 'transform metadata')
                best = min(candidates, key=lambda item: item[0])
                check(path.with_suffix('.sol').read_text() == best[1]+'\n', 'saved final witness')
                extension_energy(case, best[1], models[case['id']][2])
                endpoints[case['id'], seed, arm] = best[0]
                rows.append((case, seed, arm, row))
    stats = {a: dist([endpoints[c['id'], s, a] for c in cases for s in seeds]) for a in ARMS}
    report = {'cells': len(expected), 'integer_verified_incumbents': total, 'late_incumbents': late,
              'final_witnesses_and_consistent_extensions': len(expected), 'source_hashes_checked': len(env['sources']),
              'package_files_checked': len(env['baseline_hashes']), 'engine_python_hashes_checked': 2,
              'max_stop_s': max(r['stop_s'] for _, _, _, r in rows),
              'max_termination_s': max(r['termination_s'] for _, _, _, r in rows),
              'artifact_sha256': {p.name: sha(p) for p in sorted(directory.iterdir()) if p.is_file()}}
    if part == 'smoke':
        reconstructed = {'cells': len(expected), 'errors': [], 'stats': stats}
    else:
        comparisons = []; pvalues = []
        for arm in ARMS[1:]:
            differences = [[endpoints[c['id'], s, arm]-endpoints[c['id'], s, 'msc_native'] for s in seeds] for c in cases]
            gains = [Fraction(sum(d), len(seeds)*c['normalizer']) for d, c in zip(differences, cases)]
            pvalue = exact_p(gains); pvalues.append(pvalue)
            flat = [d for row in differences for d in row]
            comparisons.append(dict(other=arm, instance_gains=list(map(float, gains)), mean=float(statistics.mean(gains)),
                median=float(statistics.median(gains)), family_means=[float(statistics.mean(gains[:5])), float(statistics.mean(gains[5:]))],
                p=float(pvalue), wins=sum(v > 0 for v in flat), ties=flat.count(0), losses=sum(v < 0 for v in flat)))
        floor = Fraction(0)
        for rank, index in enumerate(sorted(range(3), key=lambda i: pvalues[i])):
            floor = max(floor, min(Fraction(1), pvalues[index]*(3-rank)))
            comparisons[index]['holm_p'] = float(floor)
        go = all(c['median'] >= .01 and c['holm_p'] < .05 and all(v > 0 for v in c['family_means']) for c in comparisons)
        verdict = 'CONTINUE' if go else 'INCONCLUSIVE' if all(c['ties'] == 100 for c in comparisons) else 'NOT_QUALIFIED_FOR_ADVANTAGE'
        diagnostics = {}
        for arm in ARMS:
            armrows = [r for _, _, a, r in rows if a == arm]
            metadata = [i['event'] for r in armrows for i in r['events'] if i['event']['kind'] == 'meta']
            diagnostics[arm] = {key: dist([r[key] for r in armrows]) for key in ('startup_s', 'stop_s', 'termination_s')}
            diagnostics[arm].update({key: dist([m[key] for m in metadata]) for key in ('model_n', 'model_terms', 'penalty', 'hot', 'transform_s')})
        reconstructed = {'cells': len(expected), 'errors': [], 'comparisons': comparisons, 'verdict': verdict, 'stats': stats,
            'incumbents': total, 'diagnostics': diagnostics,
            'family_stats': {f'd{degree}_{arm}': dist([endpoints[c['id'], s, arm] for c in cases if c['degree'] == degree for s in seeds]) for degree in (3, 4) for arm in ARMS},
            'improved_over_zero': {arm: sum(endpoints[c['id'], s, arm] < energy(c, '0'*c['n']) for c in cases for s in seeds)/100 for arm in ARMS},
            'energies': {c['id']: {arm: [endpoints[c['id'], s, arm] for s in seeds] for arm in ARMS} for c in cases}}
        schedules = ('ABCD', 'DCBA', 'CDAB', 'BADC')
        planned = [f"{c['id']}_{s}_{ARMS[ord(a)-65]}.json" for ci, c in enumerate(cases) for si, s in enumerate(seeds) for a in schedules[(ci*10+si)%4]]
        observed = sorted(expected, key=lambda name: (directory/name).stat().st_mtime_ns)
        check(observed == planned, 'filesystem write-order differs from frozen schedule')
        report.update(verdict=verdict, comparisons=comparisons, filesystem_write_order_matches=True,
                      observed_write_order=observed, order_evidence='Observed filesystem mtimes; no absolute event timestamps exist in cell records.')
    same(read(directory/'summary.json'), reconstructed)
    report['all_summary_fields_recomputed'] = True
    return report


def main():
    check(os.environ.get('PYTHONHASHSEED') == '0', 'set PYTHONHASHSEED=0 for reproducible dimod tie ordering')
    check(sha(EXP/'cases.json') == CASE_SHA, 'corpus hash')
    cases = read(EXP/'cases.json')
    manifest = read(EXP/'raw_sha256.json')
    check(len(manifest) == 822, 'raw manifest cardinality')
    for name, expected in manifest.items():
        check(sha(EXP/name) == expected, 'raw manifest hash '+name)
    check(len(cases) == 10 and [c['instance_seed'] for c in cases] == list(range(930001, 930006))+list(range(940001, 940006)), 'instance design')
    check(all(c['n'] == 32 and len(c['spin_terms']) == 128 and c['normalizer'] == 128 for c in cases), 'corpus dimensions')
    spin = [[1, [0,1,2]], [-1, [1,2,3,4]], [1, [5]], [-1, [0]]]
    smoke = {'id': 'smoke6', 'n': 6, 'spin_terms': spin, 'terms': [[w, list(v)] for v, w in sorted(expand(spin).items())], 'normalizer': 4}
    result = {'status': 'PASS', 'source_freeze': FREEZE, 'auditor_sha256': sha(__file__),
              'raw_manifest_entries_checked': len(manifest), 'raw_manifest_sha256': sha(EXP/'raw_sha256.json'),
              'smoke': audit('smoke', [smoke], [910001, 910002]), 'run': audit('run', cases, SEEDS),
              'limitations': [
                  'No optimizer or additional search cell was run. Verification is post-data and does not alter preregistration.',
                  'Native-arm endpoint ties do not certify global optima or prove equivalence; all-zero fallback was charged identically.',
                  'Advantage over the fixed conservative-penalty quadratic representations is not superiority over competitive quadratization methods.',
                  'Only ten synthetic instances, fixed untuned schedules, one WSL host, two-second warm runtime; no external or cold-start generalization.',
                  'Process-group cleanup is recorded by the frozen supervisor; no historical process census or absolute per-cell start timestamps were saved.',
                  'Filesystem write-order evidence is local and mutable; Git alone does not preserve execution mtimes.',
                  'Recorded package/source/executable hashes match current bytes; this is not an independently rebuilt reproducible-build attestation.',
                  'Consistent auxiliary extensions verified for every final witness; exhaustive main-instance auxiliary minimization was not performed.',
              ]}
    (HERE/'result.json').write_text(json.dumps(result, indent=2, sort_keys=True)+'\n')
    print(json.dumps({'status': result['status'], 'smoke_cells': result['smoke']['cells'], 'run_cells': result['run']['cells'],
                      'incumbents': result['run']['integer_verified_incumbents'], 'verdict': result['run']['verdict']}))


if __name__ == '__main__':
    main()
