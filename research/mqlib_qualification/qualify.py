#!/usr/bin/env python3
"""MQ-QUAL-001 retained qualification. Refuses an existing output directory."""
import argparse
from datetime import datetime, timezone
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time

from fixtures import cases
from test_adapter import a, verify_export, ROOT

SOURCES = ['benchmarks/adapters/run_mqlib.py', 'scripts/setup_mqlib.sh',
           'research/mqlib_qualification/oracle.cpp',
           'research/mqlib_qualification/fixtures.py',
           'research/mqlib_qualification/test_adapter.py',
           'research/mqlib_qualification/qualify.py',
           'research/EXTERNAL_COMPARISON_AMENDMENT_1.md']


def command(args, cwd=ROOT):
    return subprocess.check_output(args,cwd=cwd,text=True).strip()


def write(path, data):
    with path.open('x') as f:
        json.dump(data,f,indent=2,allow_nan=False);f.write('\n')


def energy(model, mask):
    # Independent exhaustive polynomial evaluator, never calling adapter energy.
    value = Fraction(model['offset'])
    for i,h in enumerate(model['linear']):
        if mask & (1<<i): value += Fraction(h)
    for i,j,v in model['pairs']:
        if mask & (1<<i) and mask & (1<<j): value += Fraction(v)
    return value


def run_oracle(binary, path):
    cmd=[str(binary),str(path.resolve())]
    record={'command':cmd,'exit_code':None,'stdout':'','stderr':'','status':'INVALID'}
    def decoded(value):
        return value.decode(errors='replace') if isinstance(value,bytes) else value or ''
    try:
        result=subprocess.run(cmd,capture_output=True,text=True,timeout=5)
        record.update(exit_code=result.returncode,stdout=result.stdout,stderr=result.stderr,
                      status='RETURNED' if result.returncode==0 else 'INVALID')
    except subprocess.TimeoutExpired as exc:
        record.update(status='TIMEOUT',stdout=decoded(exc.stdout),stderr=decoded(exc.stderr),error=str(exc))
    except OSError as exc:
        record['error']=str(exc)
    return record


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    cache=ROOT/'.cache/mqlib-qualification'
    upstream=cache/'upstream'
    tip=command(['git','rev-parse','HEAD'])
    for path in SOURCES:
        blob=subprocess.check_output(['git','show',f'{tip}:{path}'],cwd=ROOT)
        if blob != (ROOT/path).read_bytes(): raise RuntimeError('uncommitted instrument: '+path)
    if command(['git','rev-parse','HEAD'],upstream) != a.UPSTREAM_SHA:
        raise RuntimeError('wrong upstream revision')
    if command(['git','status','--porcelain'],upstream):
        raise RuntimeError('modified upstream')
    build=json.loads((cache/'build.json').read_text())
    if build['upstream_commit'] != a.UPSTREAM_SHA:
        raise RuntimeError('wrong build revision')
    for path,digest in build['hashes'].items():
        if a.sha256(ROOT/path) != digest: raise RuntimeError('stale build: '+path)
    source_hashes={p:a.sha256(ROOT/p) for p in SOURCES}
    args.output.mkdir(parents=True,exist_ok=False)
    started=datetime.now(timezone.utc).isoformat()
    metadata={'experiment_id':'MQ-QUAL-001','kind':'correctness qualification, NOT competitive benchmark',
              'git_commit':tip,'git_dirty':bool(command(['git','status','--porcelain','--untracked-files=no'])),
              'source_sha256':source_hashes,'upstream_commit':a.UPSTREAM_SHA,'build':build,
              'environment':{'platform':platform.platform(),'python':sys.version,
                             'cpu':Path('/proc/cpuinfo').read_text(),
                             'ram_bytes':os.sysconf('SC_PAGE_SIZE')*os.sysconf('SC_PHYS_PAGES'),
                             'threads':1,'rust_version':command(['rustc','-V'])},
              'execution':{'command_line':sys.argv,'seeds':[101,102,103],
                           'requested_seconds':.02,'hard_timeout_seconds':5,'start':started}}
    write(args.output/'metadata.json',metadata)
    failures=[]; enumerated=0; valid=0
    for index,model in enumerate(cases()):
        fixture=args.output/f'case{index:02d}'
        fixture.mkdir()
        write(fixture/'model.json',model)
        text=a.export_qubo(model)
        inp=fixture/'input.qubo';inp.write_text(text)
        try:
            verify_export(model,text,model['offset'])
        except ValueError as exc:
            failures.append(f'case{index}: {exc}')
        oracle_record=run_oracle(cache/'oracle',inp)
        write(fixture/'oracle.json',oracle_record)
        expected={m:Fraction(model['offset'])-energy(model,m) for m in range(1<<len(model['linear']))}
        try:
            rows=[line.split() for line in oracle_record['stdout'].splitlines()]
            observed={int(m):Fraction(v) for m,v in rows}
            if oracle_record['exit_code'] != 0 or len(rows)!=len(expected) or observed != expected:
                raise ValueError('upstream full objective differs')
            enumerated += len(rows)
        except (ValueError,OverflowError) as exc:
            failures.append(f'case{index}: {exc}')
        optimum=min(energy(model,m) for m in expected)
        for seed in (101,102,103):
            record=a.run(model,upstream/'bin/MQLib',seed,.02,5)
            record['exact_optimum']=str(optimum)
            if record['status']=='VALID':
                mask=sum(bit<<i for i,bit in enumerate(record['state']))
                exact=energy(model,mask)
                if exact != Fraction(record['energy_exact']):
                    record.update(status='INVALID',error='independent second evaluator mismatch')
                else:
                    record['exact_gap']=str(exact-optimum);valid+=1
            if record['status']!='VALID': failures.append(f'case{index}/seed{seed}: {record.get("error")}')
            write(fixture/f'seed{seed}.json',record)
    if source_hashes != {p:a.sha256(ROOT/p) for p in SOURCES}:
        failures.append('instrument changed during execution')
    if any(a.sha256(ROOT/p)!=digest for p,digest in build['hashes'].items()):
        failures.append('build changed during execution')
    files={str(p.relative_to(args.output)):a.sha256(p) for p in sorted(args.output.rglob('*')) if p.is_file()}
    summary={'status':'PASS' if not failures and valid==36 else 'FAIL',
             'fixtures':len(cases()),'enumerated_states':enumerated,'valid_candidates':valid,
             'failures':failures,'artifact_sha256':files,'finished':datetime.now(timezone.utc).isoformat(),
             'claim':'adapter correctness only; no comparative quality or time claim'}
    write(args.output/'summary.json',summary)
    (args.output/'README.md').write_text(
        '# MQ-QUAL-001 retained run\n\n'
        f'Instrument commit: `{tip}`. Status: **{summary["status"]}**.\n\n'
        'Reproduce from repository root after checkout of the instrument commit:\n\n'
        '```bash\nbash scripts/setup_mqlib.sh\n'
        'python3 -m unittest discover -s research/mqlib_qualification -p "test_*.py" -v\n'
        'python3 research/mqlib_qualification/qualify.py --output .cache/mqlib-reproduction\n```\n\n'
        'Use a new output directory. Wall-limited search can return different equally valid\n'
        'states; exhaustive objectives must match exactly, every candidate must independently\n'
        'verify. Timings, temporary paths and binary hashes vary across builds/hosts.\n'
        'metadata.json retains build output (including upstream warnings), environment and hashes.\n'
        'See ../README.md and ../../EXTERNAL_COMPARISON_AMENDMENT_1.md for scope.\n')
    print(json.dumps({k:v for k,v in summary.items() if k!='artifact_sha256'},indent=2))
    return 0 if summary['status']=='PASS' else 1


if __name__=='__main__': raise SystemExit(main())
