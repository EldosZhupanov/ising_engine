"""MQ-NATIVE-001: native model preparation only; never optimization."""
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

import importlib.util
import build

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
ARMS = ['null_a', 'null_b', 'delay']
WORKERS = ['rust_model', 'mqlib_model']
spec = importlib.util.spec_from_file_location('qualified_mq_adapter', ROOT/'benchmarks/adapters/run_mqlib.py')
mq = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mq)
sha = build.sha
git = build.git


def write(path, obj):
    Path(path).write_text(json.dumps(obj, indent=2, allow_nan=False)+'\n')


def fingerprint():
    rec = build.verify()
    hashes = dict(rec['source_hashes'])
    for b in rec['binaries'].values():
        hashes[b['path']] = b['sha256']
    for p in [Path(sys.executable).resolve(), Path(shutil.which('taskset')).resolve()]:
        hashes[str(p)] = sha(p)
    hashes[str((build.CACHE/'build.json').relative_to(ROOT))] = sha(build.CACHE/'build.json')
    return hashes


def probe_states(n):
    return [[0]*n, [1]*n, [i%2 for i in range(n)], [int(i%3==0) for i in range(n)]]


def unique(pairs):
    d = {}
    for k, v in pairs:
        if k in d:
            raise ValueError('duplicate JSON key')
        d[k] = v
    return d


def validate_event(model, worker, raw, receipt_ns, start_ns, budget_ns):
    event = json.loads(raw, object_pairs_hook=unique)
    if not isinstance(event, dict):
        raise ValueError('event must be an object')
    kind = event.get('kind')
    numeric = lambda v: type(v) in (int, float) and math.isfinite(v)
    energies = [model['offset'] + sum(h*v for h,v in zip(model['linear'],x))
                + sum(w*x[i]*x[j] for i,j,w in model['pairs'])
                for x in probe_states(len(model['linear']))]
    if worker == 'mqlib_model':
        energies = [model['offset']-e for e in energies]
    if kind == 'inc':
        keys = {'kind', 'state', 'objective'} if worker == 'mqlib_model' else {'kind', 'state', 'energy', 'chunk'}
        if set(event) != keys or event['state'] != [0]*len(model['linear']) or any(type(v) is not int for v in event['state']):
            raise ValueError('wrong incumbent schema or nonzero/noninteger state')
        e = event['objective'] if worker == 'mqlib_model' else event['energy']
        if not numeric(e) or e != energies[0]:
            raise ValueError('wrong zero-state energy')
        if worker == 'rust_model' and (type(event['chunk']) is not int or event['chunk'] != 0):
            raise ValueError('wrong chunk')
    elif kind == 'diagnostic':
        if set(event) != {'kind', 'energies', 'ready_ns', 'sleep_ns', 'affinity'}:
            raise ValueError('wrong diagnostic schema')
        if not isinstance(event['energies'], list) or len(event['energies']) != 4 or any(not numeric(v) for v in event['energies']) or event['energies'] != energies:
            raise ValueError('wrong fixed-state energies')
        if any(type(event[k]) is not int or event[k] < 0 for k in ['ready_ns','sleep_ns']):
            raise ValueError('invalid relative duration')
        if event['affinity'] != '0':
            raise ValueError('worker affinity mismatch')
        # Relative child phases must fit inside elapsed parent receipt interval.
        if event['ready_ns'] + event['sleep_ns'] > receipt_ns-start_ns:
            raise ValueError('relative phase durations exceed parent elapsed time')
    else:
        raise ValueError('unknown message kind')
    if type(receipt_ns) is not int or receipt_ns < start_ns:
        raise ValueError('invalid receipt time')
    return {'raw': raw, 'receipt_ns': receipt_ns, 'kind': kind,
            'eligible': receipt_ns-start_ns < budget_ns, 'latency_ns': receipt_ns-start_ns,
            'decoded': event}


def completion(row):
    events = row['events']
    if len(events) != 2 or {e['kind'] for e in events} != {'inc','diagnostic'}:
        return None
    if not all(e['eligible'] for e in events):
        return None
    return max(e['latency_ns'] for e in events)


def close_errors(returncode, killed, eof, cutoff_ns, budget_ns, partial):
    errors = []
    if eof and cutoff_ns < budget_ns:
        errors.append('premature EOF')
    if returncode != -9 or not killed:
        errors.append('unexpected exit: worker must remain alive until deadline')
    if partial and cutoff_ns < budget_ns:
        errors.append('incomplete early output')
    return errors


def run_cell(model, worker, fixture, repeat, arm, budget, cpu, campaign_deadline_ns):
    start = time.monotonic_ns()
    budget_ns = round(budget * 1e9)
    delay_ns = budget_ns // 4 if arm == 'delay' else 0
    rec = {'worker': worker, 'parent_affinity': sorted(os.sched_getaffinity(0)), 'fixture': fixture, 'repeat': repeat, 'arm': arm, 'budget_ns': budget_ns,
           'delay_ns': delay_ns, 'cpu': cpu, 'start_ns': start, 'events': [], 'errors': []}
    proc = None
    pending = b''
    raw = bytearray()
    eof = False
    with tempfile.TemporaryDirectory(prefix='mq-native-') as tmp:
        try:
            path = Path(tmp) / 'input.json'
            path.write_text(mq.export_qubo(model) if worker == 'mqlib_model' else json.dumps(model))
            cmd = ['taskset', '-c', str(cpu), str(build.BINS[worker]), str(path), str(delay_ns)]
            rec['command'] = cmd
            env = {**os.environ, 'RAYON_NUM_THREADS': '1', 'OMP_NUM_THREADS': '1', 'OPENBLAS_NUM_THREADS': '1'}
            rec['setup_ns'] = time.monotonic_ns() - start
            with (Path(tmp) / 'stderr').open('wb') as err:
                proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=err, env=env)
                rec['child_affinity'] = sorted(os.sched_getaffinity(proc.pid))
                if rec['parent_affinity'] != [0] or rec['child_affinity'] != [0]:
                    raise ValueError('OS affinity mismatch')
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
                            rec['events'].append(validate_event(model, worker, line.decode(), receipt, start, budget_ns))
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
                    rec['events'].append(validate_event(model, worker, line.decode(), receipt, start, budget_ns))
                pending = pending.split(b'\n')[-1]
                proc.stdout.close()
                rec['exit_code'] = proc.returncode
                rec['errors'] += close_errors(proc.returncode, rec['killed_at_deadline'], eof, rec['cutoff_ns'], budget_ns, pending)
                kinds = [e['kind'] for e in rec['events']]
                if len(kinds) != len(set(kinds)) or len(kinds) > 2:
                    rec['errors'].append('duplicate/excess worker messages')
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
    indexed = {(r['repeat'], r['arm']): r for r in rows}
    if len(rows) != 60 or set(indexed) != set(itertools.product(range(20), ARMS)) or any(r['status'] != 'VALID' for r in rows):
        return {'status': 'INVALID', 'reason': 'missing/duplicate/invalid records'}
    on = {k: completion(r) for k, r in indexed.items() if completion(r) is not None}
    lat = {a: [v for (_, arm), v in on.items() if arm == a] for a in ARMS}
    null = [on[i,'null_b']-on[i,'null_a'] for i in range(20) if (i,'null_a') in on and (i,'null_b') in on]
    positive = [on[i,'delay']-on[i,'null_a'] for i in range(20) if (i,'null_a') in on and (i,'delay') in on]
    over = [r['cutoff_ns']-budget_ns for r in rows]
    d = budget_ns//4
    diagnostics = [e['decoded'] for r in rows for e in r['events'] if e['kind']=='diagnostic']
    sleeps = [e['decoded']['sleep_ns'] for r in rows if r['arm']=='delay' for e in r['events'] if e['kind']=='diagnostic']
    oversleep = [s-d for s in sleeps]
    gates = {
        'completion': all(len(v)>=19 for v in lat.values()) and len(null)>=19 and len(positive)>=19,
        'deadline': min(over)>=0 and q95(over)<=max(1000000,.05*budget_ns),
        'null': bool(null) and abs(statistics.median(null))<=max(2000000,.1*budget_ns),
        'positive': bool(positive) and abs(statistics.median(positive)-d)<=max(2000000,.2*d) and sum(v>0 for v in positive)>=18,
        'sleep': len(lat['delay'])>=19 and bool(oversleep) and min(oversleep)>=0 and q95(oversleep)<=max(1000000,.1*d),
        'affinity': all(r['parent_affinity']==[0] and r['child_affinity']==[0] for r in rows)
                    and all(v['affinity']=='0' and v['ready_ns']>=0 for v in diagnostics),
    }
    return {'status': 'PASS' if all(gates.values()) else 'FAIL', 'gates': gates,
            'on_time_completions': {a: len(v) for a,v in lat.items()},
            'latency_ns_by_arm': {a: describe(v) for a,v in lat.items()},
            'null_difference_ns': describe(null), 'positive_difference_ns': describe(positive),
            'deadline_overshoot_ns': describe(over), 'actual_delay_sleep_ns': describe(sleeps),
            'oversleep_ns': describe(oversleep), 'model_ready_ns': describe([d['ready_ns'] for d in diagnostics]),
            'kill_reap_ns': describe([r['kill_reap_ns'] for r in rows]),
            'total_ns': describe([r['total_ns'] for r in rows]),
            'complete_but_late': sum(len(r['events'])==2 and completion(r) is None for r in rows),
            'without_both_lines': sum(len(r['events'])<2 for r in rows)}


def jobs(manifest, smoke=False):
    if smoke:
        return [(w,manifest['fixtures'][0]['file'],0,'null_a',.2) for w in WORKERS]
    result = []
    for wi,w in enumerate(WORKERS):
        for fi,f in enumerate(manifest['fixtures']):
            for bi,b in enumerate(manifest['budgets_seconds']):
                for i in range(20):
                    rotate = (wi+fi+bi+i)%3
                    result.extend((w,f['file'],i,a,b) for a in ARMS[rotate:]+ARMS[:rotate])
    return result


def analyze(rows, manifest):
    keys = [(r['worker'],r['fixture'],r['repeat'],r['arm'],r['budget_ns']) for r in rows]
    expected = [(w,f,i,a,round(b*1e9)) for w,f,i,a,b in jobs(manifest)]
    if any(r['status']!='VALID' for r in rows):
        return {'status':'INVALID','cells':len(rows)}
    if keys != expected:
        return {'status':'INCOMPLETE','cells':len(rows)}
    groups = []
    for w in WORKERS:
        for f in manifest['fixtures']:
            for b in manifest['budgets_seconds']:
                ns = round(b*1e9)
                group = evaluate_group([r for r in rows if r['worker']==w and r['fixture']==f['file'] and r['budget_ns']==ns],ns)
                groups.append({'worker':w,'fixture':f['file'],'budget_ns':ns,**group})
    return {'status':'PASS' if all(g['status']=='PASS' for g in groups) else 'FAIL',
            'cells':len(rows),'groups':groups,
            'qualified_model_budget_ns':[round(b*1e9) for b in manifest['budgets_seconds'] if all(g['status']=='PASS' for g in groups if g['budget_ns']==round(b*1e9))],
            'scope':'native diagnostic model preparation only; not full solver initialization or search'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--smoke',action='store_true')
    parser.add_argument('--smoke-record',type=Path)
    args = parser.parse_args()
    manifest = json.loads((HERE/'manifest.json').read_text())
    if platform.platform()!=manifest['environment']['platform'] or sys.version!=manifest['environment']['python']:
        raise ValueError('frozen environment mismatch')
    cpu_model = next(l.split(':',1)[1].strip() for l in Path('/proc/cpuinfo').read_text().splitlines() if l.startswith('model name'))
    if cpu_model!=manifest['environment']['cpu_model'] or vars(time.get_clock_info('monotonic'))!=manifest['environment']['monotonic_clock']:
        raise ValueError('CPU/clock mismatch')
    os.sched_setaffinity(0,{0})
    if sorted(os.sched_getaffinity(0))!=[0]:
        raise ValueError('parent affinity mismatch')
    hashes = fingerprint()
    models = {}
    for f in manifest['fixtures']:
        if sha(HERE/f['file'])!=f['sha256']:
            raise ValueError('fixture mismatch')
        models[f['file']] = mq.load_model(HERE/f['file'])
    if not args.smoke:
        if args.smoke_record is None:
            raise ValueError('requires retained two-worker smoke')
        sm = json.loads((args.smoke_record/'metadata.json').read_text())
        ss = json.loads((args.smoke_record/'summary.json').read_text())
        sr = [json.loads(l) for l in (args.smoke_record/'raw.jsonl').read_text().splitlines()]
        if ss['status']!='SMOKE_PASS' or ss['raw_sha256']!=sha(args.smoke_record/'raw.jsonl') or sm['hashes']!=hashes or not smoke_ok(sr):
            raise ValueError('smoke/provenance mismatch')
    args.output.mkdir(parents=True,exist_ok=False)
    meta = {'git_commit':git('rev-parse','HEAD'),'protocol_commit':'fb568c8',
            'git_status':git('status','--short'),'hashes':hashes,'manifest':manifest,
            'build':build.verify(),'smoke':args.smoke,'argv':sys.argv,
            'smoke_record':str(args.smoke_record) if args.smoke_record else None,
            'platform':platform.platform(),'python':sys.version,'python_executable':sys.executable,
            'affinity':sorted(os.sched_getaffinity(0)), 'started_unix':time.time(),
            'cpuinfo':Path('/proc/cpuinfo').read_text(),'meminfo':Path('/proc/meminfo').read_text(),
            'clock':vars(time.get_clock_info('monotonic')),
            'taskset':subprocess.check_output(['taskset','--version'],text=True),
            'thread_environment':{k:'1' for k in ['RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS']},
            'load_start':os.getloadavg()}
    write(args.output/'metadata.json',meta)
    start = time.monotonic_ns()
    deadline = start+round(manifest['hard_campaign_cap_seconds']*1e9)
    rows = []
    with (args.output/'raw.jsonl').open('x') as stream:
        for w,name,i,a,b in jobs(manifest,args.smoke):
            if time.monotonic_ns()>=deadline:
                break
            row = run_cell(models[name],w,name,i,a,b,0,deadline)
            rows.append(row)
            stream.write(json.dumps(row,allow_nan=False)+'\n')
            stream.flush()
            if len(rows)%60==0:
                print(f'completed {len(rows)}/960',flush=True)
            if row['status']!='VALID' or row.get('campaign_interrupted'):
                break
    summary = {'status':'SMOKE_PASS' if smoke_ok(rows) else 'SMOKE_FAIL','cells':len(rows)} if args.smoke else analyze(rows,manifest)
    try:
        if fingerprint()!=hashes:
            raise ValueError('instrument changed')
    except (OSError,ValueError,subprocess.CalledProcessError) as exc:
        summary = {'status':'INVALID','cells':len(rows),'reason':str(exc)}
    summary['elapsed_seconds'] = (time.monotonic_ns()-start)/1e9
    if summary['elapsed_seconds']>=manifest['hard_campaign_cap_seconds'] and summary['status']!='INVALID':
        summary['status']='INCOMPLETE'
    summary['raw_sha256']=sha(args.output/'raw.jsonl')
    summary['load_end']=os.getloadavg()
    write(args.output/'summary.json',summary)
    write(args.output/'reproduction.json',{'argv':sys.argv,'git_commit':meta['git_commit'],'fresh_output_required':True})
    (args.output/'README.md').write_text('# MQ-NATIVE-001 retained run\n\nInstrument `'+meta['git_commit']+'`.\n\nNative diagnostic model preparation only; no search. See ../protocol.md.\n'
        'Reproduce reproduction.json argv with a fresh output; main requires both-worker\nsmoke from identical frozen sources/binaries. Never overwrite evidence.\n')
    print(json.dumps({k:v for k,v in summary.items() if k!='groups'},indent=2))
    return 0 if summary['status'] in ('PASS','FAIL','SMOKE_PASS') else 1


def smoke_ok(rows):
    return (len(rows)==2 and [r['worker'] for r in rows]==WORKERS
            and all(r['status']=='VALID' and completion(r) is not None for r in rows))


if __name__=='__main__':
    raise SystemExit(main())
