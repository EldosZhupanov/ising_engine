"""Bind committed instrument and frozen solver algorithms to actual executables."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CACHE = ROOT / '.cache/mqlib-difficulty'
UP = ROOT / '.cache/mqlib-qualification/upstream'
RUST = ROOT / 'target/release/examples/mqlib_difficulty'
CPP = CACHE / 'worker'


def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*args): return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()
def manifest(): return json.loads((HERE / 'manifest.json').read_text())


def environment():
    keys = ['CXX', 'CC', 'CXXFLAGS', 'CFLAGS', 'LDFLAGS', 'AR', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS']
    return {k: os.environ.get(k) for k in keys}


def inputs(development=False):
    m = manifest()
    paths = git('ls-files', 'src', 'Cargo.toml', 'Cargo.lock', '.cargo', 'research/Cargo.toml').splitlines()
    for p in paths:
        if subprocess.check_output(['git', 'show', m['source_base_commit'] + ':' + p], cwd=ROOT) != (ROOT / p).read_bytes():
            raise ValueError('production/config source drift ' + p)
    for p, h in m['source_hashes'].items():
        if sha(ROOT / p) != h: raise ValueError('baseline source drift ' + p)
    paths += [str(p.relative_to(ROOT)) for p in HERE.iterdir() if p.is_file() and p.suffix in ('.py', '.cpp', '.json', '.md')]
    paths += ['research/examples/mqlib_difficulty.rs', 'benchmarks/adapters/run_mqlib.py']
    paths = sorted(set(paths))
    if not development:
        for p in paths:
            if subprocess.check_output(['git', 'show', 'HEAD:' + p], cwd=ROOT) != (ROOT / p).read_bytes():
                raise ValueError('dirty/uncommitted instrument ' + p)
    return {p: sha(ROOT / p) for p in paths}


def upstream():
    m = manifest()
    if git('-C', str(UP), 'rev-parse', 'HEAD') != m['upstream_commit'] or git('-C', str(UP), 'status', '--porcelain'):
        raise ValueError('upstream pin/clean mismatch')
    r = json.loads((ROOT / '.cache/mqlib-qualification/build.json').read_text())
    if r['upstream_commit'] != m['upstream_commit']: raise ValueError('wrong upstream build')
    for p, h in r['hashes'].items():
        if sha(ROOT / p) != h: raise ValueError('upstream artifact mismatch ' + p)
    return r


def verify():
    r = json.loads((CACHE / 'build.json').read_text())
    if r['development'] or r['source_hashes'] != inputs() or r['upstream_build'] != upstream() or r['environment'] != environment():
        raise ValueError('build source/environment provenance mismatch')
    for p, h in r['binaries'].items():
        if sha(ROOT / p) != h: raise ValueError('binary drift ' + p)
    return r


def main():
    p = argparse.ArgumentParser(); p.add_argument('--development', action='store_true'); a = p.parse_args()
    if os.environ.get('RUSTFLAGS', '') or os.environ.get('CARGO_ENCODED_RUSTFLAGS', ''):
        raise ValueError('unexpected Rust flags')
    if subprocess.check_output(['rustc', '--version'], text=True).strip() != manifest()['environment']['rustc']:
        raise ValueError('rustc changed')
    if subprocess.check_output(['g++', '--version'], text=True).splitlines()[0] != manifest()['environment']['g++']:
        raise ValueError('g++ changed')
    sources = inputs(a.development); up = upstream(); CACHE.mkdir(parents=True, exist_ok=True)
    commands = [
        ['cargo', 'build', '--release', '-p', 'research', '--example', 'mqlib_difficulty'],
        ['g++', '-std=c++11', '-O2', '-Wall', '-Wextra', '-Werror', '-I'+str(UP/'include'), str(HERE/'worker.cpp'), str(UP/'bin/MQLib.a'), '-o', str(CPP)],
    ]
    logs = []
    for cmd in commands:
        proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
        logs.append({'command': cmd, 'exit_code': proc.returncode, 'stdout': proc.stdout, 'stderr': proc.stderr})
        if proc.returncode: print(proc.stderr); raise RuntimeError('build failed')
    r = {'git_commit': git('rev-parse', 'HEAD'), 'development': a.development,
         'source_hashes': sources, 'upstream_build': up, 'environment': environment(), 'commands': logs,
         'binaries': {str(x.relative_to(ROOT)): sha(x) for x in [RUST, CPP]}}
    (CACHE/'build.json').write_text(json.dumps(r, indent=2)+'\n')
    if not a.development: verify()
    print('Build PASS; development=' + str(a.development))


if __name__ == '__main__': main()
