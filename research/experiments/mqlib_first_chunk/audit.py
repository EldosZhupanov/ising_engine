"""Offline raw-evidence audit; never import the campaign/scoring module."""
import argparse
import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OLD = HERE.parent / 'mqlib_difficulty_qualification'
spec = importlib.util.spec_from_file_location('mq_immutable_audit_helpers', OLD / 'independent_audit.py')
old = importlib.util.module_from_spec(spec)
spec.loader.exec_module(old)
PREREG = '922a3a09f05c8ce28481de5bf98c43f7a73f0d95'
SOURCES = ['protocol.md', 'manifest.json', 'campaign.py', 'audit.py', 'test_campaign.py']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(Path(path).read_text(), object_pairs_hook=old.unique)


def expected(m):
    return [dict(instance=item['id'], seed=seed, arm=arm, mode='search', budget=10., block='first_chunk')
            for item in m['instances'] for seed in m['seeds'] for arm in m['arms']]


def readiness_summary(m, table):
    output = {}
    for item in m['instances']:
        for arm in m['arms']:
            values = [x['ready_seconds'] for x in table if x['instance'] == item['id'] and x['arm'] == arm]
            assert len(values) == len(m['seeds'])
            output[item['id'] + '/' + arm] = {'median': statistics.median(values),
                'min': min(values), 'max': max(values)}
    return output


def verify_binary_hashes(native, root=ROOT):
    for path, value in native['binaries'].items():
        assert sha(root / path) == value, 'native binary drift'


def provenance(meta):
    m = meta['manifest']; frozen = meta['sources']
    assert m == read(HERE / 'manifest.json') and m['id'] == 'MQ-FIRST-CHUNK-001'
    assert frozen['registration_commit'] == PREREG
    for name in ['protocol.md', 'manifest.json']:
        path = HERE / name; rel = str(path.relative_to(ROOT))
        assert subprocess.check_output(['git', 'show', f'{PREREG}:{rel}'], cwd=ROOT) == path.read_bytes()
    assert sha(HERE / 'manifest.json') in (HERE / 'protocol.md').read_text()
    assert set(frozen['source_hashes']) == {str((HERE / name).relative_to(ROOT)) for name in SOURCES}
    for path, value in frozen['source_hashes'].items():
        assert sha(ROOT / path) == value
        assert hashlib.sha256(subprocess.check_output(['git', 'show', f"{frozen['instrument_commit']}:{path}"], cwd=ROOT)).hexdigest() == value
    assert meta['instrument_commit'] == frozen['instrument_commit']
    native = frozen['native_build']
    old.provenance({'manifest': read(OLD / 'manifest.json'), 'build': native})
    assert m['inherited_manifest_sha256'] == sha(HERE.parent / 'mqlib_coarse_quality/manifest.json')
    assert m['frozen_worker_commit'] == 'f76991dde036f77fe7bb29cbf17efb1e20ec769c'
    for path, value in native['source_hashes'].items():
        assert sha(ROOT / path) == value
        assert hashlib.sha256(subprocess.check_output(['git', 'show', f"{native['git_commit']}:{path}"], cwd=ROOT)).hexdigest() == value
    verify_binary_hashes(native)
    for path, value in native['upstream_build']['hashes'].items():
        assert sha(ROOT / path) == value
    assert meta['host'] == platform.platform() and meta['parent_affinity'] == [0] and not meta['git_dirty']


def audit(folder):
    meta = read(folder / 'metadata.json'); provenance(meta)
    m = meta['manifest']; terminal = read(folder / 'terminal.json'); result = read(folder / 'analysis.json')
    plan = expected(m); assert len(plan) == 12 and m['budget_seconds'] == 10.0
    assert m['kill_request_deadline_seconds'] == 10.1 and m['process_cap_seconds'] == 180.0
    models = {}
    old_m = read(HERE.parent / 'mqlib_coarse_quality/manifest.json')
    for item in m['instances']:
        assert item == next(x for x in old_m['instances'] if x['id'] == item['id'])
        path = ROOT / item['file']; assert sha(path) == item['sha256']
        models[item['id']] = read(path)
    paths = sorted(folder.glob('cell*/record.json'))
    assert len(paths) == terminal['cells'] and len(paths) <= 12
    table = []; all_valid = True; witnesses = 0; late = 0
    for i, path in enumerate(paths):
        assert path.parent.name == f'cell{i:02}'
        r = read(path); cell = r['cell']; assert cell == plan[i]
        model = models[cell['instance']]
        assert r['fallback_energy'] == model['offset'] and r['parent_affinity'] == [0]
        assert r['status'] == ('INVALID' if r['errors'] else 'VALID')
        data = (path.parent / 'stdout.bin').read_bytes()
        assert data == b''.join(base64.b64decode(e['raw_base64']) + b'\n' for e in r['events']) + base64.b64decode(r['partial_base64'])
        assert len(data) == r['stdout_bytes']
        ready = []; first = None; raw_first = None; configs = 0; last_chunk = -1; last_energy = float('inf'); previous = 0.0
        for e in r['events']:
            assert e['received_seconds'] >= 0 and e['validated_seconds'] >= max(previous, e['received_seconds'])
            previous = e['validated_seconds']
            if 'error' in e:
                assert r['status'] == 'INVALID'; continue
            event = json.loads(base64.b64decode(e['raw_base64']), object_pairs_hook=old.unique)
            assert event == e['message']; old.message_schema(event, cell)
            if event['kind'] == 'ready':
                assert event['affinity'] == '0' and not ready
                ready.append(e['validated_seconds']); continue
            assert len(ready) == 1
            if event['kind'] == 'config':
                assert cell['arm'] == 'v2_default' and configs == 0
                assert event['seed'] == (cell['seed'] * 0x9e3779b97f4a7c15) % 2**64
                assert event['replicas'] == 32 and event['backend'] in ('SparseBitSlice', 'DenseByte')
                assert 1 <= len(event['operators']) <= 2 and len(event['temperatures']) == 32
                for j, v in enumerate(event['temperatures']):
                    old.close(v, 4 * (.08 / 4) ** (j / 31))
                configs += 1; continue
            assert cell['arm'] != 'v2_default' or configs == 1
            value = old.energy(model, event['state'])
            assert abs(value - e['verified_energy']) <= 1e-9
            old.verify_reported_energy(value, event, False, model['offset'])
            assert value < last_energy; last_energy = value
            assert type(event['chunk']) is int and event['chunk'] > last_chunk >= -1
            last_chunk = event['chunk']; witnesses += 1
            t = max(e['received_seconds'], e['validated_seconds'])
            if raw_first is None:raw_first = e
            if t >= 10:late += 1
            elif first is None:first = t
        if r['status'] == 'VALID':
            assert len(ready) == r['ready'] == 1 and configs == r['configs']
            assert 10 <= r['cutoff_seconds'] <= r['kill_seconds'] <= r['reap_seconds'] <= r['total_seconds']
            assert r['exit_code'] == -9 and r['reap_seconds'] - r['kill_seconds'] <= 5.1
            assert r['kill_seconds'] <= 10.1
            for obs in r['observations']:
                assert len(obs['tasks']) <= 2
                if obs['seconds'] >= ready[0]:assert all(t['affinity'] == '0' for t in obs['tasks'])
            table.append({'instance': cell['instance'], 'seed': cell['seed'], 'arm': cell['arm'],
                          'ready_seconds': ready[0], 'first_verified_inc_seconds': first,
                          'first_inc_receipt_seconds': raw_first['received_seconds'] if raw_first else None,
                          'first_inc_validated_seconds': raw_first['validated_seconds'] if raw_first else None,
                          'right_censored_at_seconds': 10. if first is None else None,
                          'first_inc_before_2s': first is not None and first < 2})
        else:all_valid = False
    complete = len(paths) == 12 and all_valid and terminal['status'] == 'COMPLETE'
    if complete:
        assert result['status'] == 'COMPLETE' and result['cells'] == table
        group = lambda instance, arm: [x for x in table if x['instance'] == instance and x['arm'] == arm]
        q12 = group('q12', 'ultimate'); q18 = group('q18', 'ultimate'); v18 = group('q18', 'v2_default')
        assert result['C1'] == (all(x['first_inc_before_2s'] for x in q12) and all(not x['first_inc_before_2s'] for x in q18))
        assert result['C2'] == any(x['first_verified_inc_seconds'] is not None and 2 < x['first_verified_inc_seconds'] < 10 for x in q18)
        assert result['CONTROL'] == all(x['first_inc_before_2s'] for x in v18)
        assert result['readiness_seconds'] == readiness_summary(m, table)
        for arm in m['arms']:
            assert result['before_2s'][arm] == sum(x['first_inc_before_2s'] for x in table if x['arm'] == arm)
            assert result['before_10s'][arm] == sum(x['first_verified_inc_seconds'] is not None for x in table if x['arm'] == arm)
        assert terminal['elapsed_seconds'] <= 180
    else:
        assert terminal['status'] != 'COMPLETE' and result['status'] == 'INVALID'
    return {'status': 'PASS', 'scope': 'artifact audit only; no optimizer replication',
            'registered_status': result['status'], 'cells': len(paths), 'witnesses': witnesses,
            'late_witnesses': late,
            'artifact_hashes': {str(path.relative_to(folder)): sha(path)
                for path in sorted(folder.rglob('*')) if path.is_file() and path.name not in ('audit.json', 'review.json')}}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('folder', type=Path)
    output = parser.parse_args().folder
    result = audit(output)
    with (output / 'audit.json').open('x') as file:
        json.dump(result, file, indent=2); file.write('\n')
    print({k: v for k, v in result.items() if k != 'artifact_hashes'})
