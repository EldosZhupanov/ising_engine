"""One-shot MQ-FIRST-CHUNK-001 coordinator; unchanged native search worker."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OLD = HERE.parent / 'mqlib_difficulty_qualification'
sys.path.insert(0, str(OLD))
import run as worker  # noqa: E402
import build as native_build  # noqa: E402
sys.path.pop(0)
assert Path(worker.__file__).resolve() == OLD / 'run.py'
assert Path(native_build.__file__).resolve() == OLD / 'build.py'
PREREG = '922a3a09f05c8ce28481de5bf98c43f7a73f0d95'
SOURCES = ['protocol.md', 'manifest.json', 'campaign.py', 'audit.py', 'test_campaign.py']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def manifest():
    return json.loads((HERE / 'manifest.json').read_text())


def source_record():
    hashes = {}
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    for name in SOURCES:
        path = HERE / name
        rel = str(path.relative_to(ROOT))
        registered = subprocess.check_output(['git', 'show', f'HEAD:{rel}'], cwd=ROOT)
        if registered != path.read_bytes():
            raise ValueError('uncommitted instrument: ' + rel)
        hashes[rel] = sha(path)
    for name in ['protocol.md', 'manifest.json']:
        path = HERE / name
        rel = str(path.relative_to(ROOT))
        if subprocess.check_output(['git', 'show', f'{PREREG}:{rel}'], cwd=ROOT) != path.read_bytes():
            raise ValueError('preregistration drift: ' + rel)
    return {'instrument_commit': head, 'registration_commit': PREREG,
            'source_hashes': hashes, 'native_build': native_build.verify()}


def schedule(m):
    return [dict(instance=item['id'], seed=seed, arm=arm, mode='search', budget=m['budget_seconds'], block='first_chunk')
            for item in m['instances'] for seed in m['seeds'] for arm in m['arms']]


def first_inc(r, budget):
    hits = [e for e in r['events'] if 'verified_energy' in e
            and e['received_seconds'] < budget and e['validated_seconds'] < budget]
    return max(hits[0]['received_seconds'], hits[0]['validated_seconds']) if hits else None


def analyze(records, m):
    cells = schedule(m)
    if len(records) != len(cells) or any(r['cell'] != c or r['status'] != 'VALID' or
        r['ready'] != 1 or r['kill_seconds'] > m['kill_request_deadline_seconds']
        for r, c in zip(records, cells)):
        return {'status': 'INVALID', 'reason': 'cell/schedule/termination gate'}
    table = []
    for r in records:
        ready = next(e['validated_seconds'] for e in r['events'] if e.get('message', {}).get('kind') == 'ready')
        raw_first = next((e for e in r['events'] if 'verified_energy' in e), None)
        t = first_inc(r, m['budget_seconds'])
        table.append({'instance': r['cell']['instance'], 'seed': r['cell']['seed'],
                      'arm': r['cell']['arm'], 'ready_seconds': ready,
                      'first_inc_receipt_seconds': raw_first['received_seconds'] if raw_first else None,
                      'first_inc_validated_seconds': raw_first['validated_seconds'] if raw_first else None,
                      'first_verified_inc_seconds': t, 'right_censored_at_seconds': m['budget_seconds'] if t is None else None,
                      'first_inc_before_2s': t is not None and t < 2})
    group = lambda name, arm: [x for x in table if x['instance'] == name and x['arm'] == arm]
    q12u = group('q12', 'ultimate'); q18u = group('q18', 'ultimate'); q18v = group('q18', 'v2_default')
    c1 = all(x['first_inc_before_2s'] for x in q12u) and all(not x['first_inc_before_2s'] for x in q18u)
    c2 = any(x['first_verified_inc_seconds'] is not None and 2 < x['first_verified_inc_seconds'] < 10 for x in q18u)
    control = all(x['first_inc_before_2s'] for x in q18v)
    readiness = {}
    for item in m['instances']:
        for arm in m['arms']:
            values = [x['ready_seconds'] for x in group(item['id'], arm)]
            readiness[item['id'] + '/' + arm] = {'median': statistics.median(values),
                'min': min(values), 'max': max(values)}
    return {'status': 'COMPLETE', 'cells': table, 'C1': c1, 'C2': c2, 'CONTROL': control,
            'readiness_seconds': readiness,
            'before_2s': {name: sum(x['first_inc_before_2s'] for x in table if x['arm'] == name) for name in m['arms']},
            'before_10s': {name: sum(x['first_verified_inc_seconds'] is not None for x in table if x['arm'] == name) for name in m['arms']}}


def main():
    start = time.perf_counter()
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    m = manifest(); sources = source_record()
    if platform.platform() != m['environment']['os'] or m['environment']['cpu'] not in Path('/proc/cpuinfo').read_text():
        raise ValueError('host drift')
    if sha(HERE.parent / 'mqlib_coarse_quality/manifest.json') != m['inherited_manifest_sha256']:
        raise ValueError('inherited manifest drift')
    models = {}
    for item in m['instances']:
        path = ROOT / item['file']
        if sha(path) != item['sha256']:
            raise ValueError('input hash drift')
        models[item['id']] = json.loads(path.read_text())
    os.sched_setaffinity(0, {0})
    with (HERE / '.run_started').open('x') as f:
        f.write(str(args.output.resolve()) + '\n')
    args.output.mkdir(parents=True, exist_ok=False)
    worker.write(args.output / 'metadata.json', {'experiment_id': m['id'], 'manifest': m,
        'sources': sources, 'instrument_commit': sources['instrument_commit'],
        'git_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=ROOT).strip()),
        'parent_affinity': sorted(os.sched_getaffinity(0)), 'host': platform.platform(),
        'cpu': Path('/proc/cpuinfo').read_text(), 'python': sys.version,
        'started_unix': time.time(), 'argv': sys.argv})
    (args.output / 'README.md').write_text('MQ-FIRST-CHUNK-001 raw diagnostic. See metadata.json, analysis.json, audit.json, review.json and cellNN records.\n')
    records = []; status = 'COMPLETE'; reason = ''; result = {'status': 'INVALID'}
    try:
        for i, cell in enumerate(schedule(m)):
            if time.perf_counter() - start + m['budget_seconds'] + 5 > m['process_cap_seconds']:
                status = 'ABORT'; reason = 'process cap'; break
            r = worker.run_cell(models[cell['instance']], cell, args.output / f'cell{i:02}')
            records.append(r)
            if r['status'] != 'VALID' or r['kill_seconds'] > m['kill_request_deadline_seconds']:
                status = 'INVALID'; reason = f'cell{i:02}'; break
            print(f'{i+1}/{len(schedule(m))}', flush=True)
        result = analyze(records, m)
    except BaseException as exc:
        status = 'ABORT'; reason = repr(exc); raise
    finally:
        elapsed = time.perf_counter() - start
        if elapsed > m['process_cap_seconds']:
            status = 'ABORT'; reason = 'process cap exceeded'
        if status != 'COMPLETE':
            result = {'status': 'INVALID', 'reason': reason}
        worker.write(args.output / 'analysis.json', result)
        worker.write(args.output / 'terminal.json', {'status': status, 'reason': reason,
            'cells': len(records), 'elapsed_seconds': elapsed, 'ended_unix': time.time()})
    print(json.dumps({'terminal': status, 'analysis': result['status'], 'cells': len(records)}))


if __name__ == '__main__':
    main()
