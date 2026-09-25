#!/usr/bin/env python3
"""Frozen runner and independent integer-field analysis for CD003-MR1."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shlex
import subprocess
import sys
from datetime import datetime, timezone


ROOT = Path(__file__).resolve().parents[3]
BASE = ROOT / 'benchmarks/qoblib/marketsplit/instances'
PROTOCOL = Path(__file__).with_name('protocol.md')
PROTOCOL_SHA = '2129e7de5e091149efbfd3be0862de9f3b68bf9d17066b940b6c86b925afd46f'
INPUTS = {
    'ms_03_050_002.dat': '63c20e7146a3807f935c4be2478c4028ddd0a9c190b6e485c5376375f0ae0d2e',
    'ms_03_050_005.dat': 'eb466cc3d49a385bff22f9920a8fc108f9885ef503209e50e0509380ad472d54',
    'ms_03_050_007.dat': 'ec24fe50b1bca3c170126ef807835b7b5090fc913dfc2f8d778c38216f26df5c',
    'ms_03_050_009.dat': '72bad55ffce20187ec1290abfafee407bb6f2733d8a7e1e54b1c7b429c5d930e',
    'ms_03_100_001.dat': 'a003697b274970998f1f986c1d71ae339b489c19e5344875cc346ac13d383924',
    'ms_03_100_012.dat': 'f8aee6493d86f8a37fc25b471755eeee5ae1ae65463bdc23e15d8c9bdb954f93',
    'ms_04_050_001.dat': 'a6884b557d99f3292faeae2bcdf0073ebf6f9f707f30bae5d16845ee4b52e9cc',
    'ms_04_050_003.dat': '899fa884ceac5f233e6dfae49b33a2027db7cd478294453ac7515ac360392646',
    'ms_04_050_004.dat': 'dc0289a1603d0ca0a714cbd76a87f06b14568330ffe5af1b988f907ddda7609f',
    'ms_04_050_005.dat': 'cb6264c9e00135dae941558d9100117e856c7362047d1279d6b5b4963a168405',
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_inputs():
    if digest(PROTOCOL) != PROTOCOL_SHA:
        raise ValueError('binding protocol hash mismatch')
    for name, expected in INPUTS.items():
        if digest(BASE / name) != expected:
            raise ValueError(f'input hash mismatch: {name}')


def parse_dat(path):
    lines = [line.split() for line in path.read_text().splitlines()
             if line.strip() and not line.lstrip().startswith('#')]
    if len(lines[0]) != 2:
        raise ValueError('bad header')
    m, n = map(int, lines[0])
    if len(lines) != m + 1 or any(len(line) != n + 1 for line in lines[1:]):
        raise ValueError('bad matrix shape')
    values = [[int(x) for x in line] for line in lines[1:]]
    return m, n, [line[:-1] for line in values], [line[-1] for line in values]


def count_fields(matrix, internal, boundary):
    cross = [
        [2 * sum(row[i] * row[j] for row in matrix) for j in boundary]
        for i in internal
    ]
    return len({
        tuple(sum(weight for j, weight in enumerate(row) if mask >> j & 1)
              for row in cross)
        for mask in range(1 << len(boundary))
    })


def analyze_row(record):
    name = record['instance']
    if record.get('status') != 'ok':
        raise ValueError(f'probe status {record.get("status")}')
    if record.get('input_sha256') != INPUTS[name]:
        raise ValueError('record input hash mismatch')
    probe = record['probe']
    m, n, matrix, rhs = parse_dat(BASE / name)
    if probe['instance'] != name or probe['m'] != m or probe['n'] != n:
        raise ValueError('probe instance or dimensions mismatch')
    fixed = probe['fixed']
    if any(not isinstance(pair, list) or len(pair) != 2 or
           type(pair[0]) is not int or type(pair[1]) is not int or
           pair[1] not in (0, 1) for pair in fixed):
        raise ValueError('invalid fixing encoding')
    indices = [pair[0] for pair in fixed]
    if indices != sorted(set(indices)) or any(i < 0 or i >= n for i in indices):
        raise ValueError('duplicate, unordered or out-of-range fixing')
    free = [i for i in range(n) if i not in set(indices)]
    if probe['free'] != free:
        raise ValueError('free variables mismatch')
    zero = sum(b * b for b in rhs)
    one = sum((sum(row) - b) ** 2 for row, b in zip(matrix, rhs))
    if probe['energy_zero'] != zero or probe['energy_one'] != one:
        raise ValueError('cross-language QUBO endpoint energy mismatch')
    milliseconds = probe['presolve_ms']
    if not isinstance(milliseconds, (int, float)) or not math.isfinite(milliseconds) or milliseconds < 0:
        raise ValueError('invalid presolve time')
    b = min(12, len(free) // 2)
    boundary = free[-b:] if b else []
    internal = free[:-b] if b else free
    raw = 1 << b
    fields = count_fields(matrix, internal, boundary)
    if not 1 <= fields <= raw:
        raise ValueError('invalid field count')
    eligible = len(free) >= 8
    compressed = eligible and fields * 2 <= raw
    return {
        'instance': name, 'm': m, 'n': n, 'fixed_count': len(fixed),
        'free_count': len(free), 'boundary_count': b,
        'raw_contexts': raw, 'distinct_fields': fields,
        'field_fraction': fields / raw, 'eligible': eligible,
        'compressed': compressed, 'presolve_ms': milliseconds,
    }


def analyze(raw_path):
    try:
        verify_inputs()
        records = [json.loads(line) for line in raw_path.read_text().splitlines()]
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return {'decision': 'INCONCLUSIVE', 'compression_hits': 0,
                'valid_rows': 0,
                'invalid_rows': [{'instance': '*', 'reason': str(exc)}], 'rows': []}
    names = list(INPUTS)
    if any(not isinstance(row, dict) for row in records):
        return {'decision': 'INCONCLUSIVE', 'compression_hits': 0,
                'valid_rows': 0,
                'invalid_rows': [{'instance': '*', 'reason': 'raw row is not an object'}],
                'rows': []}
    if [row.get('instance') for row in records] != names:
        return {'decision': 'INCONCLUSIVE', 'compression_hits': 0,
                'valid_rows': 0,
                'invalid_rows': [{'instance': '*', 'reason': 'missing, duplicated or reordered raw row'}],
                'rows': []}
    rows = []
    invalid = []
    for record in records:
        try:
            rows.append(analyze_row(record))
        except (KeyError, TypeError, ValueError) as exc:
            invalid.append({'instance': record['instance'], 'reason': str(exc)})
    hits = sum(row['compressed'] for row in rows)
    decision = 'INCONCLUSIVE' if invalid else ('GO-TO-EXACT-TEST' if hits >= 3 else 'NO-GO')
    return {'decision': decision, 'compression_hits': hits,
            'valid_rows': len(rows), 'invalid_rows': invalid, 'rows': rows}


def git_value(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()


def host_value(path, prefix):
    for line in Path(path).read_text().splitlines():
        if line.startswith(prefix):
            return line.split(':', 1)[1].strip()
    raise ValueError(f'missing host field: {prefix}')


def run(binary, out_dir):
    verify_inputs()
    if not binary.is_file():
        raise ValueError('compiled probe binary missing')
    if subprocess.run(['git', 'diff', '--quiet', 'HEAD', '--',
                       'src/bin/exp_cd003_market_residual.rs', str(Path(__file__).relative_to(ROOT))],
                      cwd=ROOT, check=False).returncode != 0:
        raise ValueError('instrument differs from committed source')
    out_dir.mkdir(parents=True, exist_ok=False)
    build_command = 'cargo build --release --bin exp_cd003_market_residual'
    run_command = ' '.join(shlex.quote(part) for part in
                           ('python3', str(Path(__file__).relative_to(ROOT)), 'run',
                            str(binary), str(out_dir)))
    (out_dir / 'commands.sh').write_text(
        '#!/usr/bin/env bash\nset -euo pipefail\n'
        f'cd {shlex.quote(str(ROOT))}\n{build_command}\n{run_command}\n')
    (out_dir / 'README.md').write_text(
        '# CD003-MR1 run001\n\n'
        'Frozen [protocol](../../protocol.md); raw output is `raw.jsonl`, '
        'independent projection counts are in `analysis.json`. '
        'The evaluated commit, hashes and host are in `metadata.json`. '
        '`commands.sh` records the exact build and invocation. '
        'Replaying it requires a new output directory because completed runs '
        'are never overwritten.\n')
    metadata = {
        'source_commit': git_value('rev-parse', 'HEAD'),
        'protocol_sha256': PROTOCOL_SHA,
        'binary_sha256': digest(binary),
        'input_sha256': INPUTS,
        'utc_started': datetime.now(timezone.utc).isoformat(),
        'build_command': build_command,
        'run_command': run_command,
        'timeout_seconds_per_instance': 60,
        'platform': platform.platform(),
        'cpu_model': host_value('/proc/cpuinfo', 'model name'),
        'mem_total': host_value('/proc/meminfo', 'MemTotal'),
        'python': sys.version,
        'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
        'rayon_num_threads': 1,
        'git_dirty': bool(git_value('status', '--porcelain')),
        'source_paths_clean': True,
    }
    (out_dir / 'metadata.json').write_text(json.dumps(metadata, indent=2, sort_keys=True) + '\n')
    with (out_dir / 'raw.jsonl').open('w') as handle:
        for name, input_hash in INPUTS.items():
            row = {'instance': name, 'input_sha256': input_hash}
            try:
                completed = subprocess.run(
                    [str(binary), str(BASE / name)], text=True, capture_output=True,
                    timeout=60, env={**os.environ, 'RAYON_NUM_THREADS': '1'}, check=False)
                row['stderr'] = completed.stderr
                if completed.returncode == 0:
                    row['status'] = 'ok'
                    row['probe'] = json.loads(completed.stdout)
                else:
                    row['status'] = 'error'
                    row['exit_code'] = completed.returncode
                    row['stdout'] = completed.stdout
            except subprocess.TimeoutExpired as exc:
                row['status'] = 'timeout'
                row['stderr'] = (exc.stderr or b'').decode(errors='replace') if isinstance(exc.stderr, bytes) else (exc.stderr or '')
            handle.write(json.dumps(row, sort_keys=True) + '\n')
            handle.flush()
            os.fsync(handle.fileno())
            print(name, row['status'], flush=True)
    (out_dir / 'analysis.json').write_text(json.dumps(analyze(out_dir / 'raw.jsonl'), indent=2, sort_keys=True) + '\n')
    metadata['utc_completed'] = datetime.now(timezone.utc).isoformat()
    (out_dir / 'metadata.json').write_text(json.dumps(metadata, indent=2, sort_keys=True) + '\n')


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest='command', required=True)
    run_parser = sub.add_parser('run')
    run_parser.add_argument('binary', type=Path)
    run_parser.add_argument('output', type=Path)
    analyze_parser = sub.add_parser('analyze')
    analyze_parser.add_argument('raw', type=Path)
    args = parser.parse_args()
    if args.command == 'run':
        run(args.binary.resolve(), args.output.resolve())
    else:
        print(json.dumps(analyze(args.raw), indent=2, sort_keys=True))


if __name__ == '__main__':
    main()
