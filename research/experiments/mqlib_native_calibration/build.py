"""Isolated native probes; no production edits or optimization execution."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
CACHE = ROOT / '.cache/mqlib-native'
UP = ROOT / '.cache/mqlib-qualification/upstream'
BINS = {'rust_model': ROOT / 'target/release/examples/mqlib_native_ready',
        'mqlib_model': CACHE / 'probe'}
LOCAL = ['protocol.md', 'manifest.json', 'echo_n8.json', 'echo_n128.json',
         'probe.cpp', 'build.py', 'calibrate.py', 'test_calibrate.py']


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def inputs(development=False):
    manifest = json.loads((HERE / 'manifest.json').read_text())
    for p, h in manifest['inherited_sources'].items():
        if sha(ROOT / p) != h:
            raise ValueError('inherited source changed: ' + p)
    orig = (ROOT / 'research/examples/mqlib_compare.rs').read_text()
    probe = (ROOT / 'research/examples/mqlib_native_ready.rs').read_text()
    def body(s):
        return s[s.index('#[derive(Deserialize)]'):s.index('fn main()')]
    if body(orig) != body(probe):
        raise ValueError('Rust Model/prepare byte guard failed')
    if '#include "../mqlib_screen/stream.cpp"' not in (HERE / 'probe.cpp').read_text():
        raise ValueError('C++ original serializer include missing')
    paths = git('ls-files', 'src', 'research/src', 'research/examples', 'Cargo.toml',
                'Cargo.lock', 'research/Cargo.toml', '.cargo').splitlines()
    paths += list(manifest['inherited_sources'])
    paths += [str((HERE / p).relative_to(ROOT)) for p in LOCAL]
    paths += ['research/examples/mqlib_native_ready.rs']
    paths = sorted(set(paths))
    if not development:
        tracked = set(git('ls-files').splitlines())
        if set(paths) - tracked:
            raise ValueError('uncommitted new build inputs')
        if subprocess.run(['git', 'diff', '--quiet', 'HEAD', '--', *paths], cwd=ROOT).returncode:
            raise ValueError('dirty build inputs')
    return {p: sha(ROOT / p) for p in paths}


def upstream():
    pin = '585496274af5abb0849d0d47e135496b4688680b'
    if git('-C', str(UP), 'rev-parse', 'HEAD') != pin or git('-C', str(UP), 'status', '--porcelain'):
        raise ValueError('upstream pin/cleanliness mismatch')
    record = json.loads((ROOT / '.cache/mqlib-qualification/build.json').read_text())
    if record['upstream_commit'] != pin:
        raise ValueError('upstream build pin mismatch')
    for p, h in record['hashes'].items():
        if sha(ROOT / p) != h:
            raise ValueError('upstream build mismatch: ' + p)
    return record


def build_environment():
    keys = ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_TARGET',
            'CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS', 'RUSTUP_TOOLCHAIN',
            'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_JOBS',
            'CC', 'CXX', 'CFLAGS', 'CXXFLAGS', 'AR', 'LDFLAGS']
    return {k: os.environ.get(k) for k in keys}


def verify():
    record = json.loads((CACHE / 'build.json').read_text())
    if record['build_environment'] != build_environment():
        raise ValueError('build environment changed')
    if record['development'] or record['source_hashes'] != inputs() or record['upstream_build'] != upstream():
        raise ValueError('build provenance mismatch')
    if record['binaries'] != {k: {'path': str(p.relative_to(ROOT)), 'sha256': sha(p)} for k, p in BINS.items()}:
        raise ValueError('native binary changed')
    return record


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--development', action='store_true')
    args = parser.parse_args()
    sources = inputs(args.development)
    up = upstream()
    CACHE.mkdir(parents=True, exist_ok=True)
    commands = [
        ['cargo', 'build', '--release', '-p', 'research', '--example', 'mqlib_native_ready'],
        ['g++', '-std=c++11', '-O2', '-Wall', '-Wextra', '-Werror', '-I'+str(UP/'include'),
         str(HERE/'probe.cpp'), str(UP/'bin/MQLib.a'), '-pthread', '-o', str(BINS['mqlib_model'])],
    ]
    logs = []
    for cmd in commands:
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
        logs.append({'command': cmd, 'exit_code': r.returncode, 'stdout': r.stdout, 'stderr': r.stderr})
        if r.returncode:
            print(r.stderr)
            raise RuntimeError('native build failed')
    rec = {'git_commit': git('rev-parse', 'HEAD'), 'development': args.development,
           'build_environment': build_environment(),
           'source_hashes': sources, 'upstream_build': up, 'commands': logs,
           'compilers': {c: subprocess.check_output([c, '--version'], text=True) for c in ['rustc', 'cargo', 'g++']},
           'binaries': {k: {'path': str(p.relative_to(ROOT)), 'sha256': sha(p)} for k, p in BINS.items()}}
    (CACHE/'build.json').write_text(json.dumps(rec, indent=2)+'\n')
    if not args.development:
        verify()
    print('Native probes built; development=' + str(args.development))


if __name__ == '__main__':
    main()
