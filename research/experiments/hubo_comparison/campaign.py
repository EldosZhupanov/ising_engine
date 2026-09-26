"""Strict external warm-runtime deadline and independent HUBO-C001 analysis."""
import argparse
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import platform
import selectors
import signal
import statistics
import subprocess
import sys
import time

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
ARMS=['msc_native','msc_quad','oj_native','oj_quad']
SEEDS=list(range(950001,950011))
ORDERS=[[0,1,2,3],[3,2,1,0],[2,3,0,1],[1,0,3,2]]
BUDGET=2.0
CASE_HASH='79026f9af3fe1846129da52f28e214b77be12fac7548e17ef96bed7330cde1a9'


def digest(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
def dump(path,obj):Path(path).write_text(json.dumps(obj,indent=2,sort_keys=True)+'\n')


def cases():
    if digest(HERE/'cases.json')!=CASE_HASH:raise ValueError('case hash mismatch')
    return json.loads((HERE/'cases.json').read_text())


def smoke_case():
    # Explicit, finite fixture; no use of the n=32 generator at an invalid size.
    spin=[[1,[0,1,2]],[-1,[1,2,3,4]],[1,[5]],[-1,[0]]]
    poly={}
    for w,vs in spin:
        for k in range(len(vs)+1):
            for sub in itertools.combinations(vs,k):
                poly[sub]=poly.get(sub,0)+w*2**k*(-1)**(len(vs)-k)
    return {'id':'smoke6','n':6,'spin_terms':spin,'terms':[[w,list(v)] for v,w in sorted(poly.items()) if w], 'normalizer':4}


def score(case,bits):
    if not isinstance(bits,str) or len(bits)!=case['n'] or set(bits)-{'0','1'}:
        raise ValueError('invalid witness')
    x=[int(b) for b in bits]
    spin=sum(w*math.prod(2*x[v]-1 for v in vs) for w,vs in case['spin_terms'])
    binary=sum(w*math.prod(x[v] for v in vs) for w,vs in case['terms'])
    if spin!=binary:raise ValueError('original expansion mismatch')
    return spin


def event_energy(case,event):
    if type(event.get('energy')) is not int:raise ValueError('noninteger energy')
    actual=score(case,event.get('bits'))
    if actual!=event['energy']:raise ValueError('reported energy mismatch')
    return actual


def active_group(pgid):
    live=[]
    for p in Path('/proc').iterdir():
        if not p.name.isdigit():continue
        try:
            fields=(p/'stat').read_text().rsplit(')',1)[1].split()
            if int(fields[2])==pgid and fields[0]!='Z':live.append(int(p.name))
        except (OSError,ValueError,IndexError):pass
    return live


def parse_tail(data,elapsed):
    # Only complete lines are events. A partial line truncated by kill is raw evidence.
    return [{'received_s':elapsed,'event':json.loads(line)}
            for line in data.split(b'\n')[:-1]]


def supervise(command,case,budget=BUDGET):
    env=dict(os.environ,RAYON_NUM_THREADS='1',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',
             MKL_NUM_THREADS='1',PYTHONHASHSEED='0')
    boot=time.monotonic()
    try:
        p=subprocess.Popen(command,cwd=ROOT,stdin=subprocess.PIPE,stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE,start_new_session=True,env=env)
    except OSError as exc:
        return {'command':command,'budget_s':budget,'startup_s':None,'stop_s':None,
                'termination_s':None,'events':[],'stdout':'','stderr':str(exc),
                'returncode':None,'errors':['launch failure'],'load1':os.getloadavg()[0]}

    sel=selectors.DefaultSelector()
    sel.register(p.stdout,selectors.EVENT_READ,'stdout')
    sel.register(p.stderr,selectors.EVENT_READ,'stderr')
    buffers={'stdout':b'','stderr':b''}; outputs={'stdout':[],'stderr':[]}
    events=[]; errors=[]; ready=False; start=None; stop=None
    try:
        while True:
            limit=(start+budget) if ready else (boot+30)
            remain=limit-time.monotonic()
            if remain<=0:
                if not ready:errors.append('READY timeout')
                stop=time.monotonic();break
            if p.poll() is not None:
                errors.append('unexpected early worker exit');stop=time.monotonic();break
            for key,_ in sel.select(min(remain,0.05)):
                data=os.read(key.fileobj.fileno(),65536)
                received_at=time.monotonic()
                if not data:
                    sel.unregister(key.fileobj);continue
                name=key.data;outputs[name].append(data.decode(errors='replace'))
                if name=='stderr':continue
                buffers[name]+=data
                while b'\n' in buffers[name]:
                    line,buffers[name]=buffers[name].split(b'\n',1)
                    if not ready:
                        if line!=b'READY':raise ValueError('unexpected stdout before READY')
                        ready=True;start=time.monotonic()
                        p.stdin.write((json.dumps(case)+'\n').encode());p.stdin.flush()
                        p.stdin.close();p.stdin=None
                    else:
                        elapsed=received_at-start
                        event=json.loads(line)
                        events.append({'received_s':elapsed,'event':event})
        if ready and stop-start<budget:errors.append('deadline ended early')
    except Exception as exc:
        errors.append(str(exc));stop=time.monotonic()
    finally:
        sel.close()
        try:os.killpg(p.pid,signal.SIGTERM)
        except ProcessLookupError:pass
        try:tail_out,tail_err=p.communicate(timeout=0.1)
        except subprocess.TimeoutExpired:
            try:os.killpg(p.pid,signal.SIGKILL)
            except ProcessLookupError:pass
            try:tail_out,tail_err=p.communicate(timeout=0.1)
            except subprocess.TimeoutExpired as exc:
                tail_out,tail_err=exc.output or b'',exc.stderr or b''
                errors.append('final drain timeout')
                try:p.wait(timeout=1)
                except subprocess.TimeoutExpired:errors.append('unreaped worker')
        tail_received_at=time.monotonic()
        if ready:
            try:events.extend(parse_tail(buffers['stdout']+tail_out,tail_received_at-start))
            except Exception as exc:errors.append('invalid tail event: '+str(exc))
        outputs['stdout'].append(tail_out.decode(errors='replace'))
        outputs['stderr'].append(tail_err.decode(errors='replace'))
        survivors=active_group(p.pid)
        if survivors:
            try:os.killpg(p.pid,signal.SIGKILL)
            except ProcessLookupError:pass
            errors.append('live process-group survivors')
    end=time.monotonic()
    if ready and end-start>budget+0.2:errors.append('termination exceeded tolerance')
    return {'command':command,'budget_s':budget,'startup_s':None if start is None else start-boot,
            'stop_s':None if start is None else stop-start,'termination_s':None if start is None else end-start,
            'events':events,'stdout':''.join(outputs['stdout']),'stderr':''.join(outputs['stderr']),
            'returncode':p.returncode,'errors':errors,'load1':os.getloadavg()[0]}


def finite_number(value):
    return type(value) in (int,float) and math.isfinite(value)


def validate(case,row):
    if row['errors']:raise ValueError(str(row['errors']))
    for name in ['budget_s','startup_s','stop_s','termination_s']:
        if not finite_number(row.get(name)) or row[name]<0:raise ValueError('invalid timing field')
    if row['budget_s']!=BUDGET or not BUDGET<=row['stop_s']<=row['termination_s']<=BUDGET+0.2:
        raise ValueError('invalid deadline')
    valid=[]; meta=[]
    for item in row['events']:
        event=item['event'];received=item['received_s']
        if not finite_number(received) or not 0<=received<=row['termination_s']:
            raise ValueError('invalid receipt time')
        if not isinstance(event,dict):raise ValueError('invalid event object')
        if event.get('kind')=='inc':
            e=event_energy(case,event)
            if received<=BUDGET:valid.append((e,event['bits'],received))
        elif event.get('kind')=='meta':
            for name in ['model_n','model_terms','penalty']:
                if type(event.get(name)) is not int or event[name]<0:raise ValueError('invalid metadata integer')
            for name in ['hot','transform_s']:
                if not finite_number(event.get(name)) or event[name]<0:raise ValueError('invalid metadata timing')
            if event['model_n']<case['n'] or event['model_terms']<1 or event['hot']<0.1 or event['transform_s']>received:
                raise ValueError('inconsistent metadata')
            meta.append(event)
        else:raise ValueError('invalid event kind')
    if len(meta)!=1 or not valid:raise ValueError('missing metadata/witness')
    if row['returncode'] not in (-signal.SIGTERM,-signal.SIGKILL):raise ValueError('unexpected status')
    return min(valid,key=lambda r:r[0])


def signed_rank(values):
    vals=sorted(abs(x) for x in values if x)
    if not vals:return 1.0
    ranks={v:(vals.index(v)+1+len(vals)-list(reversed(vals)).index(v))/2 for v in set(vals)}
    rs=[int(2*ranks[abs(x)]) for x in values if x]
    observed=abs(sum(r*(1 if x>0 else -1) for r,x in zip(rs,[x for x in values if x])))
    count=sum(abs(sum(s*r for s,r in zip(signs,rs)))>=observed
              for signs in itertools.product((-1,1),repeat=len(rs)))
    return count/(2**len(rs))


def holm(ps):
    out=[0.0]*len(ps);running=0.0
    for k,i in enumerate(sorted(range(len(ps)),key=ps.__getitem__)):
        running=max(running,min(1.0,(len(ps)-k)*ps[i]));out[i]=running
    return out


def distribution(values):
    q=statistics.quantiles(values,n=4,method='inclusive')
    return dict(mean=statistics.mean(values),median=statistics.median(values),sd=statistics.stdev(values),
                minimum=min(values),maximum=max(values),q25=q[0],q75=q[2],iqr=q[2]-q[0])


def source_hashes():
    paths=git('ls-files','src','Cargo.toml','Cargo.lock','.cargo/config.toml',
              'research/Cargo.toml','research/examples/hubo_compare.rs',
              'research/experiments/hubo_comparison').splitlines()
    return {p:digest(ROOT/p) for p in paths if not '/run/' in p and not '/smoke/' in p}


def verify_sources(environment):
    required={'Cargo.toml','Cargo.lock','.cargo/config.toml','research/Cargo.toml',
              'research/examples/hubo_compare.rs','src/solver/engine.rs','src/solver/types.rs',
              'src/core/hubo.rs','src/compiler/logic_builder.rs'}
    required.update(str((HERE/f).relative_to(ROOT)) for f in
                    ['campaign.py','worker.py','generate_cases.py','cases.json','protocol.md','test_campaign.py'])
    if not required.issubset(environment['sources']):raise ValueError('incomplete source manifest')
    frozen_paths=subprocess.check_output(['git','ls-tree','-r','--name-only',environment['commit'],'--',
                 'src','Cargo.toml','Cargo.lock','.cargo/config.toml','research/Cargo.toml',
                 'research/examples/hubo_compare.rs','research/experiments/hubo_comparison'],cwd=ROOT,text=True).splitlines()
    expected_paths={p for p in frozen_paths if '/run/' not in p and '/smoke/' not in p}
    if set(environment['sources'])!=expected_paths:raise ValueError('source manifest does not cover freeze')
    for path,expected in environment['sources'].items():
        blob=subprocess.check_output(['git','show',environment['commit']+':'+path],cwd=ROOT)
        if hashlib.sha256(blob).hexdigest()!=expected:raise ValueError('source not in freeze: '+path)


def primary(cs,seeds,data):
    comparisons=[]
    for a in ARMS[1:]:
        gains=[statistics.mean([data[c['id'],s,a]-data[c['id'],s,'msc_native'] for s in seeds])/c['normalizer'] for c in cs]
        diffs=[data[c['id'],s,a]-data[c['id'],s,'msc_native'] for c in cs for s in seeds]
        comparisons.append({'other':a,'instance_gains':gains,'median':statistics.median(gains),'mean':statistics.mean(gains),
                            'family_means':[statistics.mean(gains[:5]),statistics.mean(gains[5:])],
                            'p':signed_rank(gains),'wins':sum(d>0 for d in diffs),'ties':diffs.count(0),'losses':sum(d<0 for d in diffs)})
    for c,p in zip(comparisons,holm([c['p'] for c in comparisons])):c['holm_p']=p
    go=all(c['median']>=0.01 and c['holm_p']<0.05 and min(c['family_means'])>0 for c in comparisons)
    return {'comparisons':comparisons,'verdict':'CONTINUE' if go else
            ('INCONCLUSIVE' if all(c['ties']==len(cs)*len(seeds) for c in comparisons) else 'NOT_QUALIFIED_FOR_ADVANTAGE')}


def analyze(directory):
    env=json.loads((directory/'environment.json').read_text())
    verify_sources(env)
    complete=json.loads((directory/'complete.json').read_text())
    if complete['commit']!=env['commit']:raise ValueError('completion/source mismatch')
    cs=[smoke_case()] if env['smoke'] else cases()
    seeds=[910001,910002] if env['smoke'] else SEEDS
    expected={f'{c["id"]}_{s}_{a}.json' for c in cs for s in seeds for a in ARMS}
    actual={p.name for p in directory.glob('d*.json')} if not env['smoke'] else {p.name for p in directory.glob('smoke6_*.json')}
    if actual!=expected or complete['cells']!=len(expected):raise ValueError('incomplete/unexpected cells')
    data={};counts={};errors=[]; rows=[]
    for c in cs:
        for s in seeds:
            for a in ARMS:
                path=directory/f'{c["id"]}_{s}_{a}.json';row=json.loads(path.read_text())
                command=[env['python'],str(HERE/'worker.py'),'--arm',a,'--seed',str(s),'--engine',env['engine']]
                if row['command']!=command:raise ValueError('command mismatch')
                e,bits,t=validate(c,row)
                rows.append((c,s,a,row))
                if path.with_suffix('.sol').read_text().strip()!=bits:raise ValueError('witness mismatch')
                data[c['id'],s,a]=e
                counts[c['id'],s,a]=len([v for v in row['events'] if v['event']['kind']=='inc'])
    stats={}
    for a in ARMS:
        stats[a]=distribution([data[c['id'],s,a] for c in cs for s in seeds])
    diagnostics={}
    for arm in ARMS:
        rr=[r for _,_,a,r in rows if a==arm]
        mm=[x['event'] for r in rr for x in r['events'] if x['event']['kind']=='meta']
        diagnostics[arm]={name:distribution([r[name] for r in rr]) for name in ['startup_s','stop_s','termination_s']}
        diagnostics[arm].update({name:distribution([m[name] for m in mm]) for name in ['model_n','model_terms','penalty','hot','transform_s']})
    if env['smoke']:return {'cells':len(expected),'errors':errors,'stats':stats}
    decision=primary(cs,seeds,data)
    return {'cells':len(expected),'errors':errors,**decision,'stats':stats,'incumbents':sum(counts.values()),'diagnostics':diagnostics,
            'family_stats':{f'd{d}_{a}':distribution([data[c['id'],s,a] for c in cs if c['degree']==d for s in seeds]) for d in [3,4] for a in ARMS},
            'improved_over_zero':{a:sum(data[c['id'],s,a]<score(c,'0'*c['n']) for c in cs for s in seeds)/100 for a in ARMS},
            'energies':{c['id']:{a:[data[c['id'],s,a] for s in seeds] for a in ARMS} for c in cs}}


def main():
    p=argparse.ArgumentParser();p.add_argument('mode',choices=['run','smoke','analyze']);p.add_argument('output',type=Path)
    p.add_argument('--engine',default=str(ROOT/'target/release/examples/hubo_compare'))
    p.add_argument('--python',default=str(ROOT/'benchmark-env/bin/python3'));a=p.parse_args()
    if a.mode=='analyze':print(json.dumps(analyze(a.output),indent=2));return
    if a.output.exists():raise ValueError('refuse existing output directory')
    sources=source_hashes();commit=git('rev-parse','HEAD')
    packagecode="import importlib.metadata as m,json; print(json.dumps({k:m.version(k) for k in ['numpy','openjij','dimod']}))"
    versions=json.loads(subprocess.check_output([a.python,'-c',packagecode],text=True))
    baselinecode="""import openjij,dimod,numpy,hashlib,json
from pathlib import Path
paths=set()
for module in [openjij,dimod,numpy]:
 root=Path(module.__file__).parent
 paths.update(p for p in root.rglob('*') if p.suffix in ('.py','.so'))
print(json.dumps({str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}))"""
    baseline_hashes=json.loads(subprocess.check_output([a.python,'-c',baselinecode],text=True))

    if versions!={'numpy':'2.5.1','openjij':'0.12.0','dimod':'0.12.22'}:raise ValueError('baseline version mismatch')
    env={'commit':commit,'sources':sources,'smoke':a.mode=='smoke','python':str(Path(a.python).absolute()),
         'engine':str(Path(a.engine).absolute()),'engine_sha256':digest(a.engine),'packages':versions,
         'baseline_hashes':baseline_hashes,'python_binary_sha256':digest(Path(a.python).resolve()),
         'platform':platform.platform(),'cpu':Path('/proc/cpuinfo').read_text(),'memory':Path('/proc/meminfo').read_text(),
         'load1':os.getloadavg()[0],'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
         'rustflags':os.getenv('RUSTFLAGS'),'config':(ROOT/'.cargo/config.toml').read_text()}
    verify_sources(env);a.output.mkdir(parents=True);dump(a.output/'environment.json',env)
    cs=[smoke_case()] if env['smoke'] else cases();seeds=[910001,910002] if env['smoke'] else SEEDS
    count=0
    for ci,c in enumerate(cs):
        for si,s in enumerate(seeds):
            for idx in ORDERS[(ci*10+si)%4]:
                arm=ARMS[idx]
                if source_hashes()!=sources or digest(a.engine)!=env['engine_sha256']:raise ValueError('source/binary changed')
                command=[env['python'],str(HERE/'worker.py'),'--arm',arm,'--seed',str(s),'--engine',env['engine']]
                row=supervise(command,c);path=a.output/f'{c["id"]}_{s}_{arm}.json';dump(path,row)
                try:e,bits,_=validate(c,row)
                except Exception:
                    dump(a.output/'invalid.json',{'cell':path.name,'commit':commit});raise
                path.with_suffix('.sol').write_text(bits+'\n');count+=1
                print(f'{count}: {c["id"]} {s} {arm} E={e}',flush=True)
    if any(digest(p)!=h for p,h in baseline_hashes.items()):raise ValueError('baseline changed')
    dump(a.output/'complete.json',{'commit':commit,'cells':count})
    dump(a.output/'summary.json',analyze(a.output))


if __name__=='__main__':main()
