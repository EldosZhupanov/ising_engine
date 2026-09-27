"""Build one pinned isolated bridge; never execute optimization."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
CACHE=ROOT/'.cache/mqlib-mst2'
UP=ROOT/'.cache/mqlib-qualification/upstream'
BINARY=CACHE/'stream'
ORACLE=ROOT/'.cache/mqlib-qualification/oracle'
FILES=['protocol.md','manifest.json','fixtures.json','bridge.cpp','build.py','qualify.py','test_qualify.py']


def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()


def inputs(development=False):
    m=json.loads((HERE/'manifest.json').read_text())
    for p,h in m['inherited_sources'].items():
        if sha(ROOT/p)!=h: raise ValueError('inherited source drift '+p)
    paths=list(m['inherited_sources'])+[str((HERE/f).relative_to(ROOT)) for f in FILES]
    if not development:
        if set(paths)-set(git('ls-files').splitlines()): raise ValueError('untracked instrument')
        if subprocess.run(['git','diff','--quiet','HEAD','--',*paths],cwd=ROOT).returncode: raise ValueError('dirty instrument')
    return {p:sha(ROOT/p) for p in paths}


def upstream():
    m=json.loads((HERE/'manifest.json').read_text())
    if git('-C',str(UP),'rev-parse','HEAD')!=m['upstream_commit'] or git('-C',str(UP),'status','--porcelain'):
        raise ValueError('upstream pin/clean mismatch')
    r=json.loads((ROOT/'.cache/mqlib-qualification/build.json').read_text())
    if r['upstream_commit']!=m['upstream_commit']: raise ValueError('wrong upstream build')
    for p,h in r['hashes'].items():
        if sha(ROOT/p)!=h: raise ValueError('upstream artifact mismatch '+p)
    return r


def env():
    return {k:os.environ.get(k) for k in ['CXX','CC','CXXFLAGS','CFLAGS','LDFLAGS','AR','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS']}


def verify():
    r=json.loads((CACHE/'build.json').read_text())
    if r['development'] or r['source_hashes']!=inputs() or r['upstream_build']!=upstream() or r['environment']!=env():
        raise ValueError('build provenance mismatch')
    if r['binary_sha256']!=sha(BINARY) or r['oracle_sha256']!=sha(ORACLE): raise ValueError('executable drift')
    return r


def main():
    p=argparse.ArgumentParser();p.add_argument('--development',action='store_true');a=p.parse_args()
    sources=inputs(a.development);up=upstream();CACHE.mkdir(parents=True,exist_ok=True)
    cmd=['g++','-std=c++11','-O2','-Wall','-Wextra','-Werror','-I'+str(UP/'include'),str(HERE/'bridge.cpp'),str(UP/'bin/MQLib.a'),'-o',str(BINARY)]
    proc=subprocess.run(cmd,cwd=ROOT,capture_output=True,text=True)
    if proc.returncode: print(proc.stderr);raise RuntimeError('bridge build failed')
    r={'git_commit':git('rev-parse','HEAD'),'development':a.development,'source_hashes':sources,
       'upstream_build':up,'environment':env(),'command':cmd,'exit_code':proc.returncode,
       'stdout':proc.stdout,'stderr':proc.stderr,'compiler':subprocess.check_output(['g++','--version'],text=True),
       'binary':str(BINARY.relative_to(ROOT)),'binary_sha256':sha(BINARY),
       'oracle':str(ORACLE.relative_to(ROOT)),'oracle_sha256':sha(ORACLE)}
    (CACHE/'build.json').write_text(json.dumps(r,indent=2)+'\n')
    if not a.development: verify()
    print('MST2 bridge built; development='+str(a.development))


if __name__=='__main__': main()
