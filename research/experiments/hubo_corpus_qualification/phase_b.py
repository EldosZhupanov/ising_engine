"""HUBO-Q002 native-only variation qualification. No solver tuning or p-values.

Offline analysis uses raw files and the frozen Git objects, not installed solver
packages or absolute runtime paths. --runtime additionally matches this machine.
"""
import argparse
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import platform
import selectors
import signal
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OLD = HERE.parent / 'hubo_comparison'
spec = importlib.util.spec_from_file_location('c001_helpers', OLD / 'campaign.py')
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
ARMS = ['msc_native', 'oj_native']
SEEDS = list(range(970001, 970011))
CASE_HASH = '79ce0d4291902cfa0169d1f89d2e373b9a8684d35420b2b89b24dc1277417803'
THREAD_ENV = dict(RAYON_NUM_THREADS='1', OMP_NUM_THREADS='1',
                  OPENBLAS_NUM_THREADS='1', MKL_NUM_THREADS='1', PYTHONHASHSEED='0')
SOURCE_SCOPES = ['src', 'Cargo.toml', 'Cargo.lock', '.cargo/config.toml',
                 'research/Cargo.toml', 'research/examples/hubo_compare.rs']
SOURCE_SCOPES += [str((OLD / f).relative_to(ROOT)) for f in
                  ['worker.py', 'campaign.py', 'generate_cases.py']]
SOURCE_SCOPES += [str((HERE / f).relative_to(ROOT)) for f in
                  ['phase_b.py', 'test_phase_b.py', 'protocol.md', 'cases.json', 'inputs.json']]
dump, digest = base.dump, base.digest


def inputs(smoke=False):
    if smoke:
        return [base.smoke_case()], [980001, 980002]
    if digest(HERE / 'cases.json') != CASE_HASH:
        raise ValueError('input hash mismatch')
    cs = json.loads((HERE / 'cases.json').read_text())
    manifest = json.loads((HERE / 'inputs.json').read_text())
    if [c['id'] for c in cs] != [c['id'] for c in manifest['instances']] or manifest['solver_seeds'] != SEEDS:
        raise ValueError('input order/seeds mismatch')
    return cs, SEEDS


def plan(cs, seeds):
    return [{'sequence': k, 'case': c['id'], 'seed': s, 'arm': a}
            for k, (c, s, a) in enumerate(
                (c, s, a) for ci, c in enumerate(cs) for si, s in enumerate(seeds)
                for a in (ARMS if (ci + si) % 2 == 0 else ARMS[::-1]))]


def supervise(command, case):
    """Nonblocking input/output; kill session at GO+2 s, preserve every raw byte."""
    boot = time.monotonic_ns()
    stamps = {'boot_ns': boot, 'boot_unix_ns': time.time_ns()}
    row = dict(command=command, budget_s=2.0, startup_s=None, stop_s=None,
               termination_s=None, events=[], errors=[], stdout='', stderr='',
               returncode=None, pid=None, survivors=[], load1=os.getloadavg()[0], clocks=stamps)
    try:
        p = subprocess.Popen(command, cwd=ROOT, stdin=subprocess.PIPE,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             start_new_session=True, env=dict(os.environ, **THREAD_ENV))
    except OSError as exc:
        row['errors'].append('launch failure: ' + str(exc))
        stamps.update(finish_ns=time.monotonic_ns(), finish_unix_ns=time.time_ns())
        return row
    row['pid'] = p.pid
    sel = selectors.DefaultSelector()
    for name in ['stdout', 'stderr']:
        sel.register(getattr(p, name), selectors.EVENT_READ, name)
    os.set_blocking(p.stdin.fileno(), False)
    outputs = {'stdout': bytearray(), 'stderr': bytearray()}
    pending = b''
    buffered = b''
    start = None
    stop = None
    try:
        while True:
            limit = (start + 2_000_000_000) if start is not None else (boot + 30_000_000_000)
            remaining = (limit - time.monotonic_ns()) / 1e9
            if remaining <= 0:
                if start is None:
                    row['errors'].append('READY timeout')
                stop = time.monotonic_ns()
                break
            if p.poll() is not None:
                row['errors'].append('unexpected early exit')
                stop = time.monotonic_ns()
                break
            for key, _ in sel.select(min(remaining, .025)):
                if key.data == 'stdin':
                    try:
                        written = os.write(p.stdin.fileno(), pending[:65536])
                        pending = pending[written:]
                    except BlockingIOError:
                        continue
                    if not pending:
                        sel.unregister(p.stdin)
                        p.stdin.close()
                        p.stdin = None
                    continue
                data = os.read(key.fileobj.fileno(), 65536)
                received = time.monotonic_ns()
                if not data:
                    sel.unregister(key.fileobj)
                    continue
                outputs[key.data].extend(data)
                if key.data == 'stderr':
                    continue
                buffered += data
                while b'\n' in buffered:
                    line, buffered = buffered.split(b'\n', 1)
                    if start is None:
                        if line != b'READY' or buffered:
                            raise ValueError('unexpected output before GO')
                        start = time.monotonic_ns()
                        stamps.update(go_ns=start, go_unix_ns=time.time_ns())
                        pending = (json.dumps(case) + '\n').encode()
                        sel.register(p.stdin, selectors.EVENT_WRITE, 'stdin')
                    else:
                        row['events'].append({'received_s': (received - start) / 1e9,
                                              'event': json.loads(line)})
    except Exception as exc:
        row['errors'].append(str(exc))
        stop = time.monotonic_ns()
    finally:
        sel.close()
        if p.stdin is not None:
            p.stdin.close()
            p.stdin = None
        stamps['term_ns'] = time.monotonic_ns()
        try:
            os.killpg(p.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            out, err = p.communicate(timeout=.1)
        except subprocess.TimeoutExpired:
            stamps['kill_ns'] = time.monotonic_ns()
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                out, err = p.communicate(timeout=.1)
            except subprocess.TimeoutExpired as exc:
                out, err = exc.output or b'', exc.stderr or b''
                row['errors'].append('final drain timeout')
                try:
                    p.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    row['errors'].append('unreaped process')
        tail_time = time.monotonic_ns()
        if start is not None:
            try:
                row['events'].extend(base.parse_tail(buffered + out, (tail_time - start) / 1e9))
            except Exception as exc:
                row['errors'].append('invalid tail: ' + str(exc))
        outputs['stdout'].extend(out)
        outputs['stderr'].extend(err)
        row['survivors'] = base.active_group(p.pid)
        if row['survivors']:
            row['errors'].append('live group survivors')
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    end = time.monotonic_ns()
    stamps.update(stop_ns=stop, finish_ns=end, finish_unix_ns=time.time_ns())
    row.update(returncode=p.returncode,
               stdout=outputs['stdout'].decode('utf-8', errors='surrogateescape'),
               stderr=outputs['stderr'].decode('utf-8', errors='surrogateescape'))
    if start is not None:
        row.update(startup_s=(start - boot) / 1e9, stop_s=(stop - start) / 1e9,
                   termination_s=(end - start) / 1e9)
        if end - start > 2_200_000_000:
            row['errors'].append('termination exceeded tolerance')
    return row


def validate(case, row):
    best = base.validate(case, row)  # Independently recomputes BOTH original forms.
    clocks = row['clocks']
    ordered = ['boot_ns', 'go_ns', 'stop_ns', 'term_ns', 'finish_ns']
    if any(type(clocks.get(k)) is not int or clocks[k] <= 0 for k in ordered):
        raise ValueError('invalid clocks')
    if [clocks[k] for k in ordered] != sorted(clocks[k] for k in ordered):
        raise ValueError('clock order')
    for field, left, right in [('startup_s', 'boot_ns', 'go_ns'),
                               ('stop_s', 'go_ns', 'stop_ns'),
                               ('termination_s', 'go_ns', 'finish_ns')]:
        if row[field] != (clocks[right] - clocks[left]) / 1e9:
            raise ValueError('clock/timing disagreement')
    if not 0 <= row['startup_s'] <= 30 or row['survivors'] or type(row['pid']) is not int or row['pid'] <= 0:
        raise ValueError('startup/process evidence')
    if 'kill_ns' in clocks and not clocks['term_ns'] <= clocks['kill_ns'] <= clocks['finish_ns']:
        raise ValueError('kill clock')
    if any(type(clocks.get(k)) is not int or clocks[k] <= 0 for k in ['boot_unix_ns', 'go_unix_ns', 'finish_unix_ns']):
        raise ValueError('missing wall-clock evidence')
    # Full output/event correspondence catches omitted late or malformed lines.
    lines = row['stdout'].split('\n')[:-1]
    if not lines or lines[0] != 'READY' or [json.loads(x) for x in lines[1:]] != [x['event'] for x in row['events']]:
        raise ValueError('stdout/event mismatch')
    times = [x['received_s'] for x in row['events']]
    if times != sorted(times):
        raise ValueError('nonmonotonic receipts')
    meta = next(x['event'] for x in row['events'] if x['event']['kind'] == 'meta')
    incidence = [0] * case['n']
    for w, vs in case['terms']:
        for v in vs:
            incidence[v] += abs(w)
    if (meta['model_n'], meta['model_terms'], meta['penalty']) != (case['n'], len(case['terms']), 0):
        raise ValueError('native model metadata')
    if not math.isclose(meta['hot'], max(1, max(incidence)) / math.log(2), rel_tol=1e-14):
        raise ValueError('temperature metadata')
    return best


def classify(cs, seeds, data):
    instances = {}
    strata = {}
    for c in cs:
        aa, bb = [[data[c['id'], s, a] for s in seeds] for a in ARMS]
        sa, sb = max(aa) - min(aa), max(bb) - min(bb)
        different = sum(a != b for a, b in zip(aa, bb))
        passes = (sa >= 2 or sb >= 2) and different >= 2
        info = dict(spreads=[sa, sb], differing_pairs=different, qualifies=passes,
                    stable_separation=sa == sb == 0 and different == len(seeds),
                    msc_wins=sum(a < b for a, b in zip(aa, bb)), ties=len(seeds) - different,
                    msc_losses=sum(a > b for a, b in zip(aa, bb)),
                    normalized_msc_minus_oj=[(a - b) / c['normalizer'] for a, b in zip(aa, bb)],
                    arms={a: dict(energies=vals, distinct=len(set(vals)),
                                 improved_over_zero=sum(e < base.score(c, '0' * c['n']) for e in vals) / len(vals),
                                 **base.distribution(vals)) for a, vals in zip(ARMS, [aa, bb])})
        instances[c['id']] = info
        strata.setdefault(f'n{c["n"]}_d{c["degree"]}', []).append(passes)
    counts = {s: dict(qualified=sum(v), total=len(v), passes=sum(v) >= 2) for s, v in strata.items()}
    passed = sum(v['passes'] for v in counts.values())
    verdict = 'VARIATION_PRESENT' if passed >= 3 else ('MIXED' if passed else 'NO_QUALIFIED_VARIATION')
    return dict(verdict=verdict, passing_strata=passed, strata=counts, instances=instances)


def source_paths(commit):
    return subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', commit, '--', *SOURCE_SCOPES],
                                   cwd=ROOT, text=True).splitlines()


def verify_sources(env, runtime=False):
    paths = source_paths(env['commit'])
    required = {p for p in SOURCE_SCOPES if p != 'src'} | {'src/solver/engine.rs', 'src/solver/types.rs'}
    if not required.issubset(paths) or set(paths) != set(env['sources']):
        raise ValueError('incomplete source freeze')
    for path, sha in env['sources'].items():
        blob = subprocess.check_output(['git', 'show', env['commit'] + ':' + path], cwd=ROOT)
        if hashlib.sha256(blob).hexdigest() != sha:
            raise ValueError('source freeze mismatch: ' + path)
        if runtime and digest(ROOT / path) != sha:
            raise ValueError('current source changed: ' + path)


def runtime_matches(env):
    verify_sources(env, runtime=True)
    if digest(env['engine']) != env['engine_sha256'] or digest(Path(env['python']).resolve()) != env['python_binary_sha256']:
        raise ValueError('runtime binary mismatch')
    for p, sha in env['baseline_hashes'].items():
        if digest(p) != sha:
            raise ValueError('runtime package mismatch: ' + p)


def reference_env():
    blob = subprocess.check_output(['git', 'show',
        'a196745c0672505d3ed6388e65e69cf2d0901215:research/experiments/hubo_comparison/run/environment.json'], cwd=ROOT)
    if hashlib.sha256(blob).hexdigest() != '74126b4e67eb497b2c6929d9e50ea68fb74ecc451dd36d093c95f0fec329b165':
        raise ValueError('C001 environment reference changed')
    return json.loads(blob)


def verify_environment(env):
    """Validate archived settings without accessing any installed runtime path."""
    old = reference_env()
    for name in ['packages', 'platform', 'rustc', 'rustflags', 'config', 'python_binary_sha256', 'engine_sha256']:
        if env.get(name) != old[name]:
            raise ValueError('unexpected environment change requiring amendment: ' + name)
    cpu = lambda e: sorted(set(s for s in e['cpu'].splitlines() if s.startswith('model name')))
    if cpu(env) != cpu(old):
        raise ValueError('unexpected host change')
    if sorted(env['baseline_hashes'].values()) != sorted(old['baseline_hashes'].values()):
        raise ValueError('baseline package code changed')
    if env.get('thread_env') != THREAD_ENV:
        raise ValueError('thread configuration changed')
    if env.get('worker') != str(Path(env['root']) / 'research/experiments/hubo_comparison/worker.py'):
        raise ValueError('archived worker path mismatch')
    if type(env.get('smoke')) is not bool:
        raise ValueError('invalid campaign mode')


def capture(smoke, python, engine):
    commit = base.git('rev-parse', 'HEAD')
    package_code = """import importlib.metadata as m, openjij,dimod,numpy,hashlib,json,sys
from pathlib import Path
paths=set()
for module in [openjij,dimod,numpy]:
 paths.update(p for p in Path(module.__file__).parent.rglob('*') if p.suffix in ('.py','.so'))
print(json.dumps(dict(packages={k:m.version(k) for k in ['numpy','openjij','dimod']},
 python_version=sys.version,baseline_hashes={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)})))"""
    env = json.loads(subprocess.check_output([str(python), '-c', package_code], text=True,
                                           env=dict(os.environ, **THREAD_ENV)))
    env.update(commit=commit, sources={p: digest(ROOT / p) for p in source_paths(commit)},
               smoke=smoke, root=str(ROOT), python=str(python.absolute()), engine=str(engine.absolute()),
               worker=str(OLD / 'worker.py'), engine_sha256=digest(engine),
               python_binary_sha256=digest(python.resolve()), platform=platform.platform(),
               cpu=Path('/proc/cpuinfo').read_text(), memory=Path('/proc/meminfo').read_text(),
               load=os.getloadavg(), rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
               rustflags=os.getenv('RUSTFLAGS'), config=(ROOT / '.cargo/config.toml').read_text(),
               thread_env=THREAD_ENV, controller_python=sys.version, created_unix_ns=time.time_ns())
    verify_environment(env)
    for rel in ['research/experiments/hubo_comparison/worker.py', 'research/examples/hubo_compare.rs']:
        frozen = subprocess.check_output(['git', 'show', '23d300f:' + rel], cwd=ROOT)
        if hashlib.sha256(frozen).hexdigest() != digest(ROOT / rel):
            raise ValueError('C001 worker/driver changed')
    runtime_matches(env)
    return env


def seal(directory):
    files = {str(p.relative_to(directory)): digest(p) for p in directory.rglob('*')
             if p.is_file() and p.name not in ['raw_sha256.json', 'summary.json']}
    dump(directory / 'raw_sha256.json', files)


def verify_raw(directory):
    if (directory / 'invalid.json').exists():
        raise ValueError('INSTRUMENT_INVALID: retained failure overrides all metrics')
    manifest = json.loads((directory / 'raw_sha256.json').read_text())
    actual = {str(p.relative_to(directory)) for p in directory.rglob('*')
              if p.is_file() and p.name not in ['raw_sha256.json', 'summary.json']}
    if actual != set(manifest) or not {'environment.json', 'cases.json', 'plan.json', 'complete.json'}.issubset(actual):
        raise ValueError('raw inventory mismatch/incomplete campaign')
    if any(digest(directory / p) != sha for p, sha in manifest.items()):
        raise ValueError('raw hash mismatch')


def analyze(directory, runtime=False):
    verify_raw(directory)
    env = json.loads((directory / 'environment.json').read_text())
    verify_sources(env)
    verify_environment(env)
    if runtime:
        runtime_matches(env)
    cs, seeds = inputs(env['smoke'])
    if json.loads((directory / 'cases.json').read_text()) != cs:
        raise ValueError('archived input mismatch')
    expected = plan(cs, seeds)
    if json.loads((directory / 'plan.json').read_text()) != expected:
        raise ValueError('sequence plan mismatch')
    complete = json.loads((directory / 'complete.json').read_text())
    if complete != dict(commit=env['commit'], cells=len(expected)):
        raise ValueError('completion mismatch')
    required = {f'{i:04d}{ext}' for i in range(len(expected)) for ext in ['.json', '.sol']}
    if {p.name for p in (directory / 'cells').iterdir()} != required:
        raise ValueError('cell inventory mismatch')
    by_id = {c['id']: c for c in cs}
    data, rows = {}, []
    previous_finish = 0
    for item in expected:
        path = directory / 'cells' / f'{item["sequence"]:04d}.json'
        row = json.loads(path.read_text())
        if row['identity'] != item or row['command'] != command(env, item):
            raise ValueError('cell identity/command mismatch')
        if row['clocks']['boot_ns'] < previous_finish:
            raise ValueError('overlapping/out-of-order cells')
        e, bits, receipt = validate(by_id[item['case']], row)
        if path.with_suffix('.sol').read_text() != bits + '\n':
            raise ValueError('final witness mismatch')
        previous_finish = row['clocks']['finish_ns']
        data[item['case'], item['seed'], item['arm']] = e
        rows.append((item, row))
    summary = dict(cells=len(rows), incumbents=sum(x['event']['kind'] == 'inc' for _, r in rows for x in r['events']))
    summary.update(verdict='SMOKE_PASS') if env['smoke'] else summary.update(classify(cs, seeds, data))
    summary['diagnostics'] = {}
    for a in ARMS:
        rr = [r for item, r in rows if item['arm'] == a]
        mm = [x['event'] for r in rr for x in r['events'] if x['event']['kind'] == 'meta']
        summary['diagnostics'][a] = {name: base.distribution([r[name] for r in rr]) for name in ['startup_s', 'stop_s', 'termination_s']}
        summary['diagnostics'][a].update({name: base.distribution([m[name] for m in mm]) for name in ['hot', 'transform_s']})
        summary['diagnostics'][a]['incumbents'] = sum(x['event']['kind'] == 'inc' for r in rr for x in r['events'])
    return summary


def command(env, item):
    return [env['python'], env['worker'], '--arm', item['arm'], '--seed', str(item['seed']), '--engine', env['engine']]


def run(directory, smoke, python, engine):
    if directory.exists():
        raise ValueError('refuse existing output directory')
    env = capture(smoke, python, engine)
    cs, seeds = inputs(smoke)
    schedule = plan(cs, seeds)
    by_id = {c['id']: c for c in cs}
    (directory / 'cells').mkdir(parents=True)
    dump(directory / 'environment.json', env)
    dump(directory / 'cases.json', cs)
    dump(directory / 'plan.json', schedule)
    current = None
    try:
        for item in schedule:
            current = item
            if any(digest(ROOT / p) != sha for p, sha in env['sources'].items()) or digest(engine) != env['engine_sha256']:
                raise ValueError('source/binary changed')
            case = by_id[item['case']]
            row = supervise(command(env, item), case)
            row['identity'] = item
            path = directory / 'cells' / f'{item["sequence"]:04d}.json'
            dump(path, row)  # Preserve failing row before verification; no retry.
            e, bits, _ = validate(case, row)
            path.with_suffix('.sol').write_text(bits + '\n')
            print(f'{item["sequence"] + 1}/{len(schedule)} {item["case"]} {item["seed"]} {item["arm"]} E={e}', flush=True)
        runtime_matches(env)
        dump(directory / 'complete.json', dict(commit=env['commit'], cells=len(schedule)))
        seal(directory)
        dump(directory / 'summary.json', analyze(directory))
    except BaseException as exc:
        dump(directory / 'invalid.json', dict(verdict='INSTRUMENT_INVALID', cell=current, error=repr(exc), unix_ns=time.time_ns()))
        seal(directory)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['run', 'smoke', 'analyze'])
    parser.add_argument('output', type=Path)
    parser.add_argument('--runtime', action='store_true')
    parser.add_argument('--python', type=Path, default=ROOT / 'benchmark-env/bin/python3')
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/release/examples/hubo_compare')
    args = parser.parse_args()
    if args.mode == 'analyze':
        print(json.dumps(analyze(args.output, args.runtime), indent=2))
    else:
        run(args.output, args.mode == 'smoke', args.python, args.engine)


if __name__ == '__main__':
    main()
