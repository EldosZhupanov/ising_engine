"""Portable, independent Q002 Phase B audit. No project imports or solver calls.

Run only after both campaigns finish. The output is exclusive-create evidence;
existing audit records are never overwritten. Runtime binaries are not accessed:
their recorded hashes are checked against the committed C001 reference.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import subprocess

BASE = Path(__file__).resolve().parents[1]
ROOT = BASE.parents[2]
FREEZE = '5e9ea62234b6891a73fc420bf92b797964ad5c53'
PROTOCOL = '9438f15810bff607c52fa2a7310407ba417bde17'
C001 = '23d300f545ce027e7ad56eac7e895e920b804712'
C001_RAW = 'a196745c0672505d3ed6388e65e69cf2d0901215'
REFERENCE_SHA = '74126b4e67eb497b2c6929d9e50ea68fb74ecc451dd36d093c95f0fec329b165'
CASES_SHA = '79ce0d4291902cfa0169d1f89d2e373b9a8684d35420b2b89b24dc1277417803'
PREFIX = 'research/experiments/hubo_corpus_qualification/'
OLD = 'research/experiments/hubo_comparison/'
ARMS = ('msc_native', 'oj_native')
THREADS = dict(RAYON_NUM_THREADS='1', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1',
               MKL_NUM_THREADS='1', PYTHONHASHSEED='0')


def check(condition, label):
    if not condition:
        raise ValueError(label)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def blob(commit, path):
    return subprocess.check_output(['git', 'show', commit + ':' + path], cwd=ROOT)


def read(path):
    return json.loads(path.read_bytes())


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def same(actual, expected, label='summary'):
    """Exact schema/integers, tolerance only for independently derived floats."""
    if type(expected) is dict:
        check(type(actual) is dict and actual.keys() == expected.keys(), label + ': keys')
        for key in expected:
            same(actual[key], expected[key], label + '.' + key)
    elif type(expected) is list:
        check(type(actual) is list and len(actual) == len(expected), label + ': length')
        for i, (a, b) in enumerate(zip(actual, expected)):
            same(a, b, label + '[' + str(i) + ']')
    elif type(expected) is float:
        check(finite(actual) and math.isclose(actual, expected, rel_tol=2e-13, abs_tol=1e-12), label)
    else:
        check(type(actual) is type(expected) and actual == expected, label)


def distribution(values):
    ordered = sorted(values)
    size = len(values)
    mean = math.fsum(values) / size
    def percentile(p):
        position = (size - 1) * p
        left = int(position)
        weight = position - left
        return ordered[left] * (1 - weight) + ordered[min(left + 1, size - 1)] * weight
    q25, q75 = percentile(.25), percentile(.75)
    return dict(mean=mean, median=percentile(.5),
                sd=math.sqrt(math.fsum((v - mean) ** 2 for v in values) / (size - 1)),
                minimum=ordered[0], maximum=ordered[-1], q25=q25, q75=q75, iqr=q75-q25)


def expand(spin_terms):
    polynomial = Counter()
    for coefficient, indices in spin_terms:
        for mask in range(1 << len(indices)):
            variables = tuple(v for j, v in enumerate(indices) if mask & (1 << j))
            factor = (2 ** len(variables)) * ((-1) ** (len(indices) - len(variables)))
            polynomial[variables] += coefficient * factor
    return [[value, list(key)] for key, value in sorted(polynomial.items()) if value]


def score(case, bits):
    check(type(bits) is str and len(bits) == case['n'] and not set(bits) - {'0', '1'}, 'witness encoding')
    # Spin parity and Boolean monomial support use different evaluation paths.
    spin = sum(w * (-1 if sum(bits[v] == '0' for v in vs) % 2 else 1)
               for w, vs in case['spin_terms'])
    binary = sum(w for w, vs in case['terms'] if all(bits[v] == '1' for v in vs))
    check(spin == binary, 'spin/binary energy disagreement')
    return spin


def cases(smoke):
    if smoke:
        spin = [[1, [0, 1, 2]], [-1, [1, 2, 3, 4]], [1, [5]], [-1, [0]]]
        return [dict(id='smoke6', n=6, spin_terms=spin, terms=expand(spin), normalizer=4)]
    data = blob(FREEZE, PREFIX + 'cases.json')
    check(sha(data) == CASES_SHA, 'binding input bytes')
    result = json.loads(data)
    manifest = json.loads(blob(FREEZE, PREFIX + 'inputs.json'))
    check(manifest['cases_sha256'] == CASES_SHA, 'input manifest hash')
    check(manifest['solver_seeds'] == list(range(970001, 970011)), 'seed manifest')
    check(sha(blob(FREEZE, OLD + 'generate_cases.py')) == manifest['generator_sha256'], 'generator pin')
    check(len(result) == len(manifest['instances']) == 12, 'twelve cases')
    for case, entry in zip(result, manifest['instances']):
        encoded = (json.dumps(case, sort_keys=True, separators=(',', ':')) + '\n').encode()
        check(sha(encoded) == entry['sha256'], 'per-case hash ' + case['id'])
        for name in ['id', 'n', 'degree', 'instance_seed', 'normalizer']:
            check(case[name] == entry[name], 'input manifest identity ' + name)
        check(case['normalizer'] == 4 * case['n'], 'normalizer')
    return result


def environment(env, smoke):
    check(env['commit'] == FREEZE and env['smoke'] is smoke, 'frozen commit/mode')
    reference_bytes = blob(C001_RAW, OLD + 'run/environment.json')
    check(sha(reference_bytes) == REFERENCE_SHA, 'C001 reference SHA')
    reference = json.loads(reference_bytes)
    check(env['packages'] == {'numpy': '2.5.1', 'openjij': '0.12.0', 'dimod': '0.12.22'}, 'package versions')
    for name in ['packages', 'platform', 'rustc', 'rustflags', 'config', 'python_binary_sha256', 'engine_sha256']:
        check(env[name] == reference[name], 'environment continuity: ' + name)
    models = lambda e: set(line for line in e['cpu'].splitlines() if line.startswith('model name'))
    check(models(env) == models(reference) and models(env), 'CPU model continuity')
    check(env['thread_env'] == THREADS, 'thread settings')
    check(Counter(env['baseline_hashes'].values()) == Counter(reference['baseline_hashes'].values()), 'package code hashes')
    check(env['worker'] == str(Path(env['root']) / (OLD + 'worker.py')), 'archived worker path')
    check('MemTotal:' in env['memory'] and len(env['load']) == 3 and all(finite(v) for v in env['load']), 'RAM/load metadata')
    check(type(env['created_unix_ns']) is int and env['created_unix_ns'] > 0, 'capture time')
    check(bool(env['controller_python']) and bool(env['python_version']), 'Python version metadata')
    scopes = ['src', 'Cargo.toml', 'Cargo.lock', '.cargo/config.toml', 'research/Cargo.toml',
              'research/examples/hubo_compare.rs']
    scopes += [OLD + name for name in ['worker.py', 'campaign.py', 'generate_cases.py']]
    scopes += [PREFIX + name for name in ['phase_b.py', 'test_phase_b.py', 'protocol.md', 'cases.json', 'inputs.json']]
    paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', FREEZE, '--', *scopes], cwd=ROOT, text=True).splitlines()
    check(set(paths) == set(env['sources']), 'complete source inventory')
    for path in paths:
        check(sha(blob(FREEZE, path)) == env['sources'][path], 'source hash: ' + path)
    for path in [OLD + 'worker.py', 'research/examples/hubo_compare.rs']:
        check(blob(FREEZE, path) == blob(C001, path), 'retained worker/driver settings')
    check(blob(FREEZE, PREFIX + 'protocol.md') == blob(PROTOCOL, PREFIX + 'protocol.md'), 'binding protocol unchanged')
    return len(paths)


def inventory(directory):
    check(not (directory / 'invalid.json').exists(), 'retained invalid marker')
    all_paths = list(directory.rglob('*'))
    check(not any(p.is_symlink() for p in all_paths), 'symlink in raw evidence')
    files = {str(p.relative_to(directory)) for p in all_paths if p.is_file()}
    manifest = read(directory / 'raw_sha256.json')
    check(files == set(manifest) | {'raw_sha256.json', 'summary.json'}, 'raw inventory')
    for name, expected in manifest.items():
        check(not Path(name).is_absolute() and '..' not in Path(name).parts, 'unsafe raw path')
        check(sha((directory / name).read_bytes()) == expected, 'raw SHA: ' + name)
    return {name: sha((directory / name).read_bytes()) for name in sorted(files)}


def audit_row(case, row):
    check(row['errors'] == [] and row['survivors'] == [], 'recorded process failure')
    check(type(row['pid']) is int and row['pid'] > 0, 'PID')
    check(row['returncode'] in [-15, -9], 'termination signal')
    for key in ['budget_s', 'startup_s', 'stop_s', 'termination_s', 'load1']:
        check(finite(row[key]) and row[key] >= 0, 'finite timing/load: ' + key)
    check(row['budget_s'] == 2 and 0 <= row['startup_s'] <= 30, 'startup/budget')
    check(2 <= row['stop_s'] <= row['termination_s'] <= 2.2, 'deadline tolerance')
    stamps = row['clocks']
    names = ['boot_ns', 'go_ns', 'stop_ns', 'term_ns', 'finish_ns']
    check(all(type(stamps[k]) is int and stamps[k] > 0 for k in names), 'monotonic clock encoding')
    check(all(stamps[a] <= stamps[b] for a, b in zip(names, names[1:])), 'monotonic clock order')
    if 'kill_ns' in stamps:
        check(type(stamps['kill_ns']) is int and stamps['term_ns'] <= stamps['kill_ns'] <= stamps['finish_ns'], 'SIGKILL clock')
    if row['returncode'] == -9:
        check('kill_ns' in stamps, 'SIGKILL evidence')
    for name, begin, end in [('startup_s', 'boot_ns', 'go_ns'), ('stop_s', 'go_ns', 'stop_ns'), ('termination_s', 'go_ns', 'finish_ns')]:
        check(row[name] == (stamps[end] - stamps[begin]) / 1e9, 'derived clock: ' + name)
    for key in ['boot_unix_ns', 'go_unix_ns', 'finish_unix_ns']:
        check(type(stamps[key]) is int and stamps[key] > 0, 'UTC clock')
    check(stamps['boot_unix_ns'] <= stamps['go_unix_ns'] <= stamps['finish_unix_ns'], 'UTC clock order')
    # Round-trip original bytes. Only newline-terminated events count as complete.
    raw = row['stdout'].encode('utf-8', errors='surrogateescape')
    row['stderr'].encode('utf-8', errors='surrogateescape')
    lines = raw.split(b'\n')
    check(lines[0] == b'READY', 'READY framing')
    parsed = [json.loads(line) for line in lines[1:-1]]
    check(parsed == [record['event'] for record in row['events']], 'all complete stdout events retained')
    best, meta, receipts = None, [], []
    counts = dict(incumbents=0, late_incumbents=0, partial_tails=int(bool(lines[-1])))
    for record in row['events']:
        receipt, event = record['received_s'], record['event']
        check(finite(receipt) and 0 <= receipt <= row['termination_s'], 'receipt bounds')
        receipts.append(receipt)
        check(type(event) is dict, 'event object')
        if event.get('kind') == 'inc':
            check(type(event['energy']) is int, 'integer energy')
            actual = score(case, event['bits'])
            check(actual == event['energy'], 'reported objective vs independent energy')
            counts['incumbents'] += 1
            counts['late_incumbents'] += receipt > 2
            if receipt <= 2 and (best is None or actual < best[0]):
                best = (actual, event['bits'], receipt)
        elif event.get('kind') == 'meta':
            for name in ['model_n', 'model_terms', 'penalty']:
                check(type(event[name]) is int, 'integer model metadata')
            check((event['model_n'], event['model_terms'], event['penalty']) == (case['n'], len(case['terms']), 0), 'native model metadata')
            check(finite(event['transform_s']) and 0 <= event['transform_s'] <= receipt, 'preparation receipt')
            incidence = [sum(abs(w) for w, vs in case['terms'] if v in vs) for v in range(case['n'])]
            check(finite(event['hot']) and math.isclose(event['hot'], max(1, max(incidence)) / math.log(2), rel_tol=1e-14), 'native temperature')
            meta.append(event)
        else:
            raise ValueError('unrecognized complete event')
    check(receipts == sorted(receipts), 'receipt order')
    check(best is not None and len(meta) == 1, 'eligible witness and unique metadata')
    return best, meta[0], counts


def summary(cs, seeds, endpoints, rows, smoke):
    result = dict(cells=len(rows), incumbents=sum(count['incumbents'] for _, _, _, count in rows))
    if smoke:
        result['verdict'] = 'SMOKE_PASS'
    else:
        instances, groups = {}, {}
        for case in cs:
            values = [[endpoints[case['id'], seed, arm] for seed in seeds] for arm in ARMS]
            a, b = values
            spreads = [max(v) - min(v) for v in values]
            differing = sum(x != y for x, y in zip(a, b))
            qualifies = max(spreads) >= 2 and differing >= 2
            zero = score(case, '0' * case['n'])
            instances[case['id']] = dict(spreads=spreads, differing_pairs=differing, qualifies=qualifies,
                stable_separation=max(spreads) == 0 and differing == len(seeds),
                msc_wins=sum(x < y for x, y in zip(a, b)), ties=len(seeds)-differing,
                msc_losses=sum(x > y for x, y in zip(a, b)),
                normalized_msc_minus_oj=[(x-y)/case['normalizer'] for x, y in zip(a, b)],
                arms={arm: dict(energies=v, distinct=len(set(v)), improved_over_zero=sum(e < zero for e in v)/len(v),
                                **distribution(v)) for arm, v in zip(ARMS, values)})
            groups.setdefault('n%d_d%d' % (case['n'], case['degree']), []).append(qualifies)
        check(len(groups) == 4 and all(len(v) == 3 for v in groups.values()), 'four complete strata')
        strata = {name: dict(qualified=sum(v), total=3, passes=sum(v) >= 2) for name, v in groups.items()}
        count = sum(s['passes'] for s in strata.values())
        result.update(instances=instances, strata=strata, passing_strata=count,
                      verdict='VARIATION_PRESENT' if count >= 3 else 'MIXED' if count else 'NO_QUALIFIED_VARIATION')
    result['diagnostics'] = {}
    for arm in ARMS:
        selected = [(row, meta, counts) for item, row, meta, counts in rows if item['arm'] == arm]
        metrics = {key: distribution([row[key] for row, _, _ in selected]) for key in ['startup_s', 'stop_s', 'termination_s']}
        metrics.update({key: distribution([meta[key] for _, meta, _ in selected]) for key in ['hot', 'transform_s']})
        metrics['incumbents'] = sum(counts['incumbents'] for _, _, counts in selected)
        result['diagnostics'][arm] = metrics
    return result


def audit(directory, smoke):
    hashes = inventory(directory)
    env = read(directory / 'environment.json')
    source_count = environment(env, smoke)
    cs = cases(smoke)
    check(read(directory / 'cases.json') == cs, 'archived cases')
    for case in cs:
        check(expand(case['spin_terms']) == case['terms'], 'exact input polynomial expansion')
    seeds = [980001, 980002] if smoke else list(range(970001, 970011))
    schedule = []
    for ci, case in enumerate(cs):
        for si, seed in enumerate(seeds):
            order = ARMS if (ci+si) % 2 == 0 else ARMS[::-1]
            for arm in order:
                schedule.append(dict(sequence=len(schedule), case=case['id'], seed=seed, arm=arm))
    check(read(directory / 'plan.json') == schedule, 'complete balanced schedule')
    check(read(directory / 'complete.json') == dict(commit=FREEZE, cells=len(schedule)), 'completion marker')
    expected = {'environment.json', 'cases.json', 'plan.json', 'complete.json', 'summary.json', 'raw_sha256.json'}
    expected.update('cells/%04d%s' % (i, ext) for i in range(len(schedule)) for ext in ['.json', '.sol'])
    check(set(hashes) == expected, 'no hidden cells or exclusions')
    by_id = {case['id']: case for case in cs}
    rows, endpoints, totals = [], {}, Counter()
    previous = None
    for item in schedule:
        path = directory / ('cells/%04d.json' % item['sequence'])
        row = read(path)
        check(row['identity'] == item, 'sequence identity')
        expected_command = [env['python'], env['worker'], '--arm', item['arm'], '--seed', str(item['seed']), '--engine', env['engine']]
        check(row['command'] == expected_command, 'archived command')
        best, meta, counts = audit_row(by_id[item['case']], row)
        check(path.with_suffix('.sol').read_bytes() == (best[1] + '\n').encode(), 'earliest best eligible final witness')
        if previous is not None:
            check(previous['finish_ns'] <= row['clocks']['boot_ns'], 'sequential nonoverlapping workers')
            check(previous['finish_unix_ns'] <= row['clocks']['boot_unix_ns'], 'sequential UTC evidence')
        previous = row['clocks']
        endpoints[item['case'], item['seed'], item['arm']] = best[0]
        totals.update(counts)
        rows.append((item, row, meta, counts))
    recomputed = summary(cs, seeds, endpoints, rows, smoke)
    same(read(directory / 'summary.json'), recomputed)
    return dict(status='PASS', cells=len(schedule), cases=len(cs), **totals,
                source_files_verified=source_count, raw_hashes=hashes, recomputed_summary=recomputed,
                first_clocks=rows[0][1]['clocks'], last_clocks=rows[-1][1]['clocks'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--main', type=Path, default=BASE / 'phase_b_run')
    parser.add_argument('--smoke', type=Path, default=BASE / 'phase_b_smoke')
    parser.add_argument('--output', type=Path, default=Path(__file__).with_suffix('.json'))
    args = parser.parse_args()
    check(not args.output.exists(), 'refuse to overwrite independent audit')
    smoke = audit(args.smoke, True)
    main_result = audit(args.main, False)
    check(smoke['last_clocks']['finish_ns'] <= main_result['first_clocks']['boot_ns'], 'smoke precedes main')
    report = dict(status='PASS', frozen_source=FREEZE, binding_protocol=PROTOCOL,
                  audited_utc=datetime.now(timezone.utc).isoformat(), auditor_sha256=sha(Path(__file__).read_bytes()),
                  independence='Python standard library only; no C001/Q002 implementation imports or runtime binary access',
                  limits='Recorded process/receipt evidence and recorded binary/package hashes verified; no retrospective observation of live processes or re-execution of solvers.',
                  smoke=smoke, main=main_result)
    with args.output.open('x') as stream:
        stream.write(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(status='PASS', smoke_cells=smoke['cells'], main_cells=main_result['cells'],
                          incumbents=main_result['incumbents'], verdict=main_result['recomputed_summary']['verdict'],
                          output=str(args.output))))


if __name__ == '__main__':
    main()
