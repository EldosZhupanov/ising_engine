"""Bind the screen executables to committed source and pinned linked MQLib."""
import json
from pathlib import Path
import subprocess
from campaign import ROOT, HERE, SOURCES, sha, git, mq

CACHE=ROOT/'.cache/mqlib-qualification'
UP=CACHE/'upstream'
CPP=['g++','-std=c++11','-O2','-Wall','-Wextra','-Werror','-I'+str(UP/'include'),
     str(HERE/'stream.cpp'),str(UP/'bin/MQLib.a'),'-o',str(CACHE/'screen_stream')]
RUST=['cargo','build','--release','-p','research','--example','mqlib_compare']


def inputs():
    paths=git('ls-files','src','research/src','research/examples','Cargo.toml','Cargo.lock',
              'research/Cargo.toml','.cargo').splitlines()
    paths=sorted(set(paths+SOURCES))
    head=git('rev-parse','HEAD')
    for p in paths:
        if subprocess.check_output(['git','show',head+':'+p],cwd=ROOT)!=(ROOT/p).read_bytes():
            raise RuntimeError('uncommitted build input '+p)
    return {p:sha(ROOT/p) for p in paths}


def upstream():
    if git('-C',str(UP),'rev-parse','HEAD')!=mq.UPSTREAM_SHA or git('-C',str(UP),'status','--porcelain'):
        raise RuntimeError('upstream pin/cleanliness mismatch')
    record=json.loads((CACHE/'build.json').read_text())
    if record['upstream_commit']!=mq.UPSTREAM_SHA: raise RuntimeError('bad upstream build')
    for p,h in record['hashes'].items():
        if sha(ROOT/p)!=h: raise RuntimeError('upstream build hash mismatch '+p)
    return record


def verify_build():
    record=json.loads((CACHE/'screen_build.json').read_text())
    if record['source_hashes']!=inputs() or record['upstream_build']!=upstream():
        raise RuntimeError('build source provenance mismatch')
    for p,h in record['binaries'].items():
        if sha(ROOT/p)!=h: raise RuntimeError('screen binary mismatch')
    return record


def main():
    sources=inputs();up=upstream();logs=[]
    for cmd in (RUST,CPP):
        r=subprocess.run(cmd,cwd=ROOT,capture_output=True,text=True)
        logs.append({'command':cmd,'exit_code':r.returncode,'stdout':r.stdout,'stderr':r.stderr})
        if r.returncode:
            print(r.stderr);raise RuntimeError('build failed')
    record={'git_commit':git('rev-parse','HEAD'),'source_hashes':sources,'upstream_build':up,
            'commands':logs,'compiler':subprocess.check_output(['g++','--version'],text=True),
            'binaries':{str(p.relative_to(ROOT)):sha(p) for p in
                        [ROOT/'target/release/examples/mqlib_compare',CACHE/'screen_stream']}}
    (CACHE/'screen_build.json').write_text(json.dumps(record,indent=2)+'\n')
    verify_build();print('Committed source and executable hashes bound')


if __name__=='__main__': main()
