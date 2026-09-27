"""MQ-CAL-001 instrument; no solver imports or real benchmark access."""
import argparse
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import platform
import selectors
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
ARMS = ['null_a', 'null_b', 'delay']
SOURCES = ['protocol.md', 'manifest.json', 'echo_n8.json', 'echo_n128.json',
           'worker.py', 'calibrate.py', 'test_calibrate.py']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, obj):
    Path(path).write_text(json.dumps(obj, indent=2, allow_nan=False) + '\n')


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def fingerprint():
    head = git('rev-parse', 'HEAD')
    hashes = {}
    for name in SOURCES:
        path = HERE / name
        rel = str(path.relative_to(ROOT))
        if subprocess.check_output(['git', 'show', head + ':' + rel], cwd=ROOT) != path.read_bytes():
            raise ValueError('uncommitted instrument: ' + rel)
        hashes[rel] = sha(path)
    for path in [Path(sys.executable).resolve(), Path(shutil.which('taskset')).resolve()]:
        hashes[str(path)] = sha(path)
    return hashes


def validate_event(model, raw, receipt_ns, start_ns, budget_ns):
    event = json.loads(raw)
    if not isinstance(event, dict) or set(event) != {'state', 'energy', 'sleep_start_ns', 'sleep_end_ns', 'pre_emit_ns'}:
        raise ValueError('wrong event schema')
    x = event['state']
    if not isinstance(x, list) or len(x) != len(model['linear']) or any(type(v) is not int or v != 0 for v in x):
        raise ValueError('worker must return exact all-zero vector')
    energy = model['offset']
    for i, h in enumerate(model['linear']):
        energy += h * x[i]
    for i, j, w in model['pairs']:
        energy += w * x[i] * x[j]
    if type(event['energy']) not in (float, int) or not math.isfinite(event['energy']) or event['energy'] != energy:
        raise ValueError('wrong energy')
    stamps = [event[k] for k in ['sleep_start_ns', 'sleep_end_ns', 'pre_emit_ns']]
    if any(type(v) is not int for v in stamps) or not (start_ns <= stamps[0] <= stamps[1] <= stamps[2] <= receipt_ns):
        raise ValueError('clock domain or timestamp order violation')
    return {'raw': raw, 'receipt_ns': receipt_ns,
            'eligible': receipt_ns - start_ns < budget_ns,
            'latency_ns': receipt_ns - start_ns,
            'delivery_lag_ns': receipt_ns - stamps[2],
            'sleep_ns': stamps[1] - stamps[0], 'verified_energy': energy}


def close_errors(returncode, killed, eof, cutoff_ns, budget_ns, partial):
    errors = []
    if eof and cutoff_ns < budget_ns:
        errors.append('premature EOF')
    if returncode != -9 or not killed:
        errors.append('unexpected exit: worker must remain alive until deadline')
    if partial and cutoff_ns < budget_ns:
        errors.append('incomplete early output')
    return errors


def run_cell(model, fixture, repeat, arm, budget, cpu, campaign_deadline_ns):
    start = time.monotonic_ns()
    budget_ns = round(budget * 1e9)
    delay_ns = budget_ns // 4 if arm == 'delay' else 0
    rec = {'fixture': fixture, 'repeat': repeat, 'arm': arm, 'budget_ns': budget_ns,
           'delay_ns': delay_ns, 'cpu': cpu, 'start_ns': start, 'events': [], 'errors': []}
    proc = None
    pending = b''
    raw = bytearray()
    eof = False
    with tempfile.TemporaryDirectory(prefix='mq-cal-') as tmp:
        try:
            path = Path(tmp) / 'input.json'
            path.write_text(json.dumps(model))
            cmd = ['taskset', '-c', str(cpu), sys.executable, str(HERE / 'worker.py'), str(path), str(delay_ns)]
            rec['command'] = cmd
            env = {**os.environ, 'RAYON_NUM_THREADS': '1', 'OMP_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1'}
            rec['setup_ns'] = time.monotonic_ns() - start
            with (Path(tmp) / 'stderr').open('wb') as err:
                proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=err, env=env)
                os.set_blocking(proc.stdout.fileno(), False)
                with selectors.DefaultSelector() as selector:
                    selector.register(proc.stdout, selectors.EVENT_READ)
                    while True:
                        remaining = min(start + budget_ns, campaign_deadline_ns) - time.monotonic_ns()
                        if remaining <= 0:
                            break
                        if not selector.select(remaining / 1e9):
                            continue
                        chunk = os.read(proc.stdout.fileno(), 65536)
                        receipt = time.monotonic_ns()
                        if not chunk:
                            eof = True
                            break
                        raw.extend(chunk)
                        pending += chunk
                        if len(raw) > 1000000:
                            raise ValueError('oversized output')
                        while b'\n' in pending:
                            line, pending = pending.split(b'\n', 1)
                            rec['events'].append(validate_event(model, line.decode(), receipt, start, budget_ns))
                cutoff = time.monotonic_ns()
                rec['cutoff_ns'] = cutoff - start
                rec['campaign_interrupted'] = cutoff >= campaign_deadline_ns
                rec['eof'] = eof
                rec['killed_at_deadline'] = cutoff >= min(start + budget_ns, campaign_deadline_ns) and proc.poll() is None
                rec['kill_sent_ns'] = None
                if proc.poll() is None:
                    rec['kill_sent_ns'] = time.monotonic_ns()
                    proc.kill()
                proc.wait(timeout=3)
                rec['reaped_ns'] = time.monotonic_ns()
                rec['kill_reap_ns'] = rec['reaped_ns'] - rec['kill_sent_ns'] if rec['kill_sent_ns'] is not None else None
                rest = proc.stdout.read() or b''
                receipt = time.monotonic_ns()
                raw.extend(rest)
                pending += rest
                for line in pending.split(b'\n')[:-1]:
                    rec['events'].append(validate_event(model, line.decode(), receipt, start, budget_ns))
                pending = pending.split(b'\n')[-1]
                proc.stdout.close()
                rec['exit_code'] = proc.returncode
                rec['errors'] += close_errors(proc.returncode, rec['killed_at_deadline'], eof, rec['cutoff_ns'], budget_ns, pending)
                if len(rec['events']) > 1:
                    rec['errors'].append('more than one worker message')
        except (OSError, ValueError, subprocess.TimeoutExpired) as exc:
            rec['errors'].append(str(exc))
        finally:
            if proc is not None and proc.poll() is None:
                proc.kill()
                proc.wait(timeout=3)
            if proc is not None and proc.stdout is not None:
                proc.stdout.close()
        rec['stderr'] = (Path(tmp) / 'stderr').read_text(errors='replace') if (Path(tmp) / 'stderr').exists() else ''
    rec['raw_hex'] = raw.hex()
    rec['partial_hex'] = pending.hex()
    rec['total_ns'] = time.monotonic_ns() - start
    rec['status'] = 'INVALID' if rec['errors'] else 'VALID'
    return rec


def q95(values):
    return sorted(values)[math.ceil(.95 * len(values)) - 1] if values else None


def describe(values):
    if not values:
        return {'n': 0}
    x = sorted(values)
    def quantile(p):
        z = (len(x) - 1) * p
        lo = int(z)
        hi = min(lo + 1, len(x) - 1)
        return x[lo] * (1 - z + lo) + x[hi] * (z - lo)
    return {'n': len(x), 'mean': statistics.mean(x), 'median': statistics.median(x),
            'std': statistics.pstdev(x), 'min': min(x), 'max': max(x),
            'q25': quantile(.25), 'q75': quantile(.75), 'q95_nearest_rank': q95(x)}


def evaluate_group(rows, budget_ns):
    expected = set(itertools.product(range(20), ARMS))
    indexed = {(r['repeat'], r['arm']): r for r in rows}
    if len(rows) != 60 or set(indexed) != expected or any(r['status'] != 'VALID' for r in rows):
        return {'status': 'INVALID', 'reason': 'missing/duplicate/invalid records'}
    on = {key: r['events'][0] for key, r in indexed.items() if r['events'] and r['events'][0]['eligible']}
    latencies = {a: [e['latency_ns'] for (i, arm), e in on.items() if arm == a] for a in ARMS}
    null = [on[i, 'null_b']['latency_ns'] - on[i, 'null_a']['latency_ns'] for i in range(20) if (i, 'null_a') in on and (i, 'null_b') in on]
    positive = [on[i, 'delay']['latency_ns'] - on[i, 'null_a']['latency_ns'] for i in range(20) if (i, 'null_a') in on and (i, 'delay') in on]
    over = [r['cutoff_ns'] - budget_ns for r in rows]
    lag = [e['delivery_lag_ns'] for e in on.values()]
    d = budget_ns // 4
    sleep = [r['events'][0]['sleep_ns'] for r in rows if r['arm'] == 'delay' and r['events']]
    on_time_sleep_count = sum(a == 'delay' for i, a in on)
    oversleep = [s - d for s in sleep]
    gates = {
        'completion': all(len(v) >= 19 for v in latencies.values()),
        'pairs': len(null) >= 19 and len(positive) >= 19,
        'deadline': min(over) >= 0 and q95(over) <= max(1000000, .05 * budget_ns),
        'delivery': bool(lag) and min(lag) >= 0 and q95(lag) <= max(1000000, .05 * budget_ns),
        'null': bool(null) and abs(statistics.median(null)) <= max(2000000, .1 * budget_ns),
        'positive': bool(positive) and abs(statistics.median(positive) - d) <= max(2000000, .2 * d) and sum(v > 0 for v in positive) >= 18,
        'sleep': on_time_sleep_count >= 19 and min(oversleep) >= 0 and q95(oversleep) <= max(1000000, .1 * d),
    }
    return {'status': 'PASS' if all(gates.values()) else 'FAIL', 'gates': gates,
            'latency_ns_by_arm': {a: describe(v) for a, v in latencies.items()},
            'completion_by_arm': {a: len(v) / 20 for a, v in latencies.items()},
            'deadline_overshoot_ns': describe(over), 'delivery_lag_ns': describe(lag),
            'null_difference_ns': describe(null), 'positive_difference_ns': describe(positive),
            'actual_delay_sleep_ns': describe(sleep), 'oversleep_ns': describe(oversleep),
            'on_time_delay_sleep_count': on_time_sleep_count,
            'late_messages': sum(bool(r['events']) and not r['events'][0]['eligible'] for r in rows),
            'absent_messages': sum(not r['events'] for r in rows)}


def analyze(rows, manifest):
    expected = {(f['file'], round(b * 1e9), i, a) for f in manifest['fixtures'] for b in manifest['budgets_seconds'] for i in range(20) for a in ARMS}
    keys = [(r['fixture'], r['budget_ns'], r['repeat'], r['arm']) for r in rows]
    if any(r['status'] != 'VALID' for r in rows):
        return {'status': 'INVALID', 'cells': len(rows)}
    if len(keys) != len(expected) or set(keys) != expected:
        return {'status': 'INCOMPLETE', 'cells': len(rows)}
    groups = []
    for f in manifest['fixtures']:
        for b in manifest['budgets_seconds']:
            ns = round(b * 1e9)
            group = evaluate_group([r for r in rows if r['fixture'] == f['file'] and r['budget_ns'] == ns], ns)
            groups.append({'fixture': f['file'], 'budget_ns': ns, **group})
    return {'status': 'PASS' if all(g['status'] == 'PASS' for g in groups) else 'FAIL',
            'cells': len(rows), 'groups': groups,
            'qualified_echo_budget_ns': [round(b * 1e9) for b in manifest['budgets_seconds'] if all(g['status'] == 'PASS' for g in groups if g['budget_ns'] == round(b * 1e9))],
            'scope': 'Python echo only; not native solver budget admission'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--smoke', action='store_true')
    parser.add_argument('--smoke-record', type=Path)
    args = parser.parse_args()
    start = time.monotonic_ns()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    if platform.platform() != manifest['environment']['platform'] or sys.version != manifest['environment']['python']:
        raise ValueError('environment differs from frozen manifest; new calibration version required')
    if manifest['cpu'] not in os.sched_getaffinity(0):
        raise ValueError('registered CPU unavailable')
    os.sched_setaffinity(0, {manifest['cpu']})
    cpu_model = next(line.split(':', 1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name'))
    if cpu_model != manifest['environment']['cpu_model'] or vars(time.get_clock_info('monotonic')) != manifest['environment']['monotonic_clock']:
        raise ValueError('CPU or monotonic clock differs from frozen manifest')
    hashes = fingerprint()
    models = {}
    for f in manifest['fixtures']:
        if sha(HERE / f['file']) != f['sha256']:
            raise ValueError('fixture hash mismatch')
        models[f['file']] = json.loads((HERE / f['file']).read_text())
    if not args.smoke:
        if args.smoke_record is None:
            raise ValueError('main requires retained smoke')
        sm = json.loads((args.smoke_record / 'metadata.json').read_text())
        ss = json.loads((args.smoke_record / 'summary.json').read_text())
        if ss['status'] != 'SMOKE_PASS' or ss['raw_sha256'] != sha(args.smoke_record / 'raw.jsonl') or sm['hashes'] != hashes:
            raise ValueError('smoke gate or provenance mismatch')
    args.output.mkdir(parents=True, exist_ok=False)
    metadata = {'git_commit': git('rev-parse', 'HEAD'), 'git_status': git('status', '--short'),
                'hashes': hashes, 'manifest': manifest, 'smoke': args.smoke, 'argv': sys.argv,
                'started_unix': time.time(), 'platform': platform.platform(), 'python': sys.version,
                'python_executable': sys.executable, 'affinity': sorted(os.sched_getaffinity(0)),
                'cpuinfo': Path('/proc/cpuinfo').read_text(), 'clock': vars(time.get_clock_info('monotonic')),
                'taskset': subprocess.check_output(['taskset', '--version'], text=True), 'load_start': os.getloadavg()}
    metadata['meminfo'] = Path('/proc/meminfo').read_text()
    metadata['thread_environment'] = {'RAYON_NUM_THREADS': '1', 'OMP_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1'}
    write(args.output / 'metadata.json', metadata)
    jobs = []
    if args.smoke:
        jobs = [(manifest['fixtures'][0]['file'], 0, 'null_a', .2)]
    else:
        for fi, f in enumerate(manifest['fixtures']):
            for bi, b in enumerate(manifest['budgets_seconds']):
                for i in range(20):
                    rotate = (fi + bi + i) % 3
                    jobs.extend((f['file'], i, a, b) for a in ARMS[rotate:] + ARMS[:rotate])
    deadline = start + round(manifest['hard_campaign_cap_seconds'] * 1e9)
    rows = []
    with (args.output / 'raw.jsonl').open('x') as stream:
        for name, i, arm, b in jobs:
            if time.monotonic_ns() >= deadline:
                break
            row = run_cell(models[name], name, i, arm, b, manifest['cpu'], deadline)
            rows.append(row)
            stream.write(json.dumps(row, allow_nan=False) + '\n')
            stream.flush()
            if len(rows) % 60 == 0:
                print(f'completed {len(rows)}/{len(jobs)}', flush=True)
            if row['status'] != 'VALID' or row.get('campaign_interrupted'):
                break
    if args.smoke:
        ok = len(rows) == 1 and rows[0]['status'] == 'VALID' and len(rows[0]['events']) == 1
        summary = {'status': 'SMOKE_PASS' if ok else 'SMOKE_FAIL', 'cells': len(rows)}
    else:
        summary = analyze(rows, manifest)
    try:
        if fingerprint() != hashes:
            raise ValueError('source or executable changed')
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        summary = {'status': 'INVALID', 'reason': str(exc), 'cells': len(rows)}
    summary['elapsed_seconds'] = (time.monotonic_ns() - start) / 1e9
    if summary['elapsed_seconds'] >= manifest['hard_campaign_cap_seconds'] and summary['status'] != 'INVALID':
        summary['status'] = 'INCOMPLETE'
    summary['raw_sha256'] = sha(args.output / 'raw.jsonl')
    summary['load_end'] = os.getloadavg()
    write(args.output / 'summary.json', summary)
    write(args.output / 'reproduction.json', {'argv': sys.argv, 'git_commit': metadata['git_commit'], 'output_must_be_fresh': True})
    (args.output / 'README.md').write_text('# MQ-CAL-001 retained run\n\nInstrument commit `' + metadata['git_commit'] + '`.\n\n'
        'No-search Python echo only. See ../protocol.md, metadata.json and reproduction.json.\n'
        'Reproduce the recorded argv with a new --output directory; main requires a passing\n'
        '--smoke-record from the same instrument and executable hashes. Never overwrite this run.\n'
        'Independent verification must recompute raw witnesses, timestamps and all seven gates.\n')
    print(json.dumps(summary, indent=2))
    return 0 if summary['status'] in ('PASS', 'FAIL', 'SMOKE_PASS') else 1


if __name__ == '__main__':
    raise SystemExit(main())
