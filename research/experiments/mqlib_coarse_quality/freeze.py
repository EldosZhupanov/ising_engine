"""Bind new committed orchestration to unchanged, previously built workers."""
import hashlib
import json
from pathlib import Path
import subprocess
from bindings import HERE,ROOT,native_build

PREREG='4d0b3f4944ef4ae5849a824d26acd4c6c197e29c'
FILES=['protocol.md','manifest.json','bindings.py','freeze.py','campaign.py','check.py','test_campaign.py']
CACHE=ROOT/'.cache/mqlib-coarse-quality'


def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
def manifest():return json.loads((HERE/'manifest.json').read_text())


def inputs():
    hashes={}
    for name in FILES:
        p=HERE/name;rel=str(p.relative_to(ROOT))
        raw=subprocess.check_output(['git','show','HEAD:'+rel],cwd=ROOT)
        if raw!=p.read_bytes():raise ValueError('uncommitted source '+rel)
        hashes[rel]=sha(p)
    for name in ['protocol.md','manifest.json']:
        p=HERE/name;rel=str(p.relative_to(ROOT))
        if subprocess.check_output(['git','show',PREREG+':'+rel],cwd=ROOT)!=p.read_bytes():raise ValueError('registration drift')
    return hashes


def prepare():
    record={'git_commit':git('rev-parse','HEAD'),'registration_commit':PREREG,'sources':inputs(),'native_build':native_build.verify()}
    CACHE.mkdir(parents=True,exist_ok=True)
    (CACHE/'freeze.json').write_text(json.dumps(record,indent=2)+'\n')
    verify();print('Committed coordinator and unchanged worker provenance PASS')


def verify():
    r=json.loads((CACHE/'freeze.json').read_text())
    if r['registration_commit']!=PREREG or r['sources']!=inputs() or r['native_build']!=native_build.verify():raise ValueError('instrument drift')
    return r


if __name__=='__main__':prepare()
