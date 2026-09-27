"""One frozen, exclusively created qualification run; no silent retries."""
import argparse
import base64
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
import tempfile
import time
import build
from analysis import ARMS, schedule, preflight, main_analysis

HERE, ROOT = build.HERE, build.ROOT
spec=importlib.util.spec_from_file_location('mq_export', ROOT/'benchmarks/adapters/run_mqlib.py')
mq=importlib.util.module_from_spec(spec); spec.loader.exec_module(mq)


def write(p, obj):
    with Path(p).open('x') as f: json.dump(obj, f, indent=2, allow_nan=False); f.write('\n')


def unique(pairs):
    d={}
    for k,v in pairs:
        if k in d: raise ValueError('duplicate JSON key')
        d[k]=v
    return d


def energy(m,x):
    if not isinstance(x,list) or len(x)!=len(m['linear']) or any(type(v) is not int or v not in (0,1) for v in x):
        raise ValueError('invalid binary state')
    return m['offset']+sum(v*h for v,h in zip(x,m['linear']))+sum(w*x[i]*x[j] for i,j,w in m['pairs'])


def number(v):
    if type(v) not in (int,float) or abs(v)>1e15 or not math.isfinite(v): raise ValueError('invalid finite number')
    return v


def validate(m, event, record):
    if not isinstance(event,dict): raise ValueError('event not object')
    kind=event.get('kind');cell=record['cell']
    schemas={'ready':{'kind','affinity'}, 'config':{'kind','backend','operators','temperatures','replicas','seed'}}
    if kind=='inc':
        key='objective' if cell['arm'].startswith('mqlib_') else 'energy'
        schemas['inc']={'kind','state',key}
        if cell['mode']!='search':schemas['inc'].add('sleep_ns')
        elif not cell['arm'].startswith('mqlib_'):schemas['inc'].add('chunk')
    if kind not in schemas or set(event)!=schemas[kind]:raise ValueError('unexpected message schema')
    if kind=='ready':
        record['ready']+=1
        if record['ready']!=1 or event.get('affinity')!='0': raise ValueError('ready/affinity mismatch')
        return None
    if record['ready']!=1: raise ValueError('message before readiness')
    if kind=='config':
        if cell['arm']!='v2_default' or cell['mode']!='search': raise ValueError('unexpected config')
        record['configs']+=1
        if record['configs']!=1 or event.get('seed')!=(cell['seed']*0x9e3779b97f4a7c15)%(2**64): raise ValueError('config count/seed')
        if event.get('backend') not in ('DenseByte','SparseBitSlice') or event.get('replicas')!=32: raise ValueError('config backend/replicas')
        ts=event.get('temperatures');ops=event.get('operators')
        if not isinstance(ops,list) or not 1<=len(ops)<=2 or any(not isinstance(x,str) or not x for x in ops): raise ValueError('config operators')
        if not isinstance(ts,list) or len(ts)!=32 or any(abs(number(v)-4*(.08/4)**(i/31))>1e-12 for i,v in enumerate(ts)): raise ValueError('config temperatures')
        return None
    if kind!='inc': raise ValueError('unexpected message')
    if cell['arm']=='v2_default' and cell['mode']=='search' and record['configs']!=1: raise ValueError('missing config')
    if cell['mode']=='search' and not cell['arm'].startswith('mqlib_'):
        chunk=event['chunk']
        if type(chunk) is not int or chunk<0 or chunk<=record.get('last_chunk',-1):raise ValueError('invalid chunk progression')
        record['last_chunk']=chunk
    v=energy(m,event.get('state'))
    claimed=m['offset']-number(event.get('objective')) if cell['arm'].startswith('mqlib_') else number(event.get('energy'))
    if abs(claimed-v)>1e-9: raise ValueError('raw energy mismatch')
    if cell['mode']=='search':
        if v>=record.get('previous_energy',float('inf')):raise ValueError('non-improving emitted witness')
        record['previous_energy']=v
    if cell['mode']!='search':
        if any(event['state']): raise ValueError('probe state not zero')
        if type(event.get('sleep_ns')) is not int or event['sleep_ns']<0: raise ValueError('invalid sleep observation')
    return v


def consume(m,line,received,r,start,clock=time.perf_counter):
    e={'raw_base64':base64.b64encode(line).decode(), 'received_seconds':received}
    r['events'].append(e)
    try:
        if len(line)>2**20: raise ValueError('oversized line')
        event=json.loads(line.decode('utf8'),object_pairs_hook=unique)
        v=validate(m,event,r); e['message']=event
        if v is not None:e['verified_energy']=v
    except (ValueError,TypeError,KeyError,OverflowError,UnicodeError,RecursionError) as exc:
        e['error']=str(exc);r['errors'].append(str(exc))
    e['validated_seconds']=clock()-start


def observed_tasks(pid):
    tasks=[]
    try: paths=list(Path(f'/proc/{pid}/task').iterdir())
    except FileNotFoundError: return tasks
    for p in paths:
        try:
            lines=(p/'status').read_text().splitlines()
            info={line.split(':',1)[0]:line.split(':',1)[1].strip() for line in lines if ':' in line}
            tasks.append({'tid':int(p.name),'name':info['Name'],'state':info['State'],'affinity':info['Cpus_allowed_list']})
        except FileNotFoundError: pass
    return tasks


def run_cell(m, cell, folder, override=None):
    r={'cell':cell,'events':[],'errors':[],'fallback_energy':m['offset'],'ready':0,'configs':0,'observations':[]}
    start=time.perf_counter();budget=cell['budget'];proc=None;pending=b'';receipt=0.;total=0
    r['parent_affinity']=sorted(os.sched_getaffinity(0))
    if r['parent_affinity']!=[0]:r['errors'].append('parent affinity mismatch')
    folder.mkdir(exist_ok=False)
    with tempfile.TemporaryDirectory(prefix='mq-difficulty-') as tmp:
        path=Path(tmp)/'model'
        try:
            if cell['arm'].startswith('mqlib_'):path.write_text(mq.export_qubo(m));binary=build.CPP
            else:path.write_text(json.dumps(m));binary=build.RUST
            cmd=[str(binary),cell['arm'],str(path),str(cell['seed']),cell['mode']]
            if override is not None:cmd=override
            env={**os.environ,**build.manifest()['thread_env']};r['command']=['taskset','-c','0',*cmd]
            r['setup_seconds']=time.perf_counter()-start
            if r['setup_seconds']>=budget: raise ValueError('setup exhausted budget before worker launch')
            with (folder/'stderr.bin').open('wb') as err,(folder/'stdout.bin').open('wb') as raw:
                proc=subprocess.Popen(r['command'],stdout=subprocess.PIPE,stderr=err,env=env,start_new_session=True)
                os.set_blocking(proc.stdout.fileno(),False)
                r['pid']=proc.pid;r['launch_seconds']=time.perf_counter()-start
                next_observation=0.
                with selectors.DefaultSelector() as sel:
                    sel.register(proc.stdout,selectors.EVENT_READ)
                    while not r['errors']:
                        now=time.perf_counter()-start
                        if now>=budget:break
                        if now>=next_observation:
                            tasks=observed_tasks(proc.pid);next_observation=now+.05
                            r['observations'].append({'seconds':now,'tasks':tasks})
                            # taskset may not have executed yet; readiness establishes the child mask.
                            if r['ready'] and any(t['affinity']!='0' for t in tasks): raise ValueError('thread affinity mismatch')
                            allowed=2 if cell['arm'] in ('ultimate','v2_default') and cell['mode']=='search' else 1
                            if len(tasks)>allowed:raise ValueError('unexpected worker thread count')
                        if not sel.select(min(.05,max(0,budget-(time.perf_counter()-start)))):continue
                        chunk=os.read(proc.stdout.fileno(),65536);receipt=time.perf_counter()-start
                        if not chunk:
                            r['errors'].append('unexpected early EOF');break
                        raw.write(chunk);total+=len(chunk);pending+=chunk
                        if total>16*2**20:raise ValueError('oversized total output')
                        while b'\n' in pending and time.perf_counter()-start<budget:
                            line,pending=pending.split(b'\n',1)
                            consume(m,line,receipt,r,start)
                            if r['errors']:break
                        if len(pending)>2**20 and b'\n' not in pending:raise ValueError('oversized partial output')
        except (OSError,ValueError,subprocess.TimeoutExpired) as exc:r['errors'].append(str(exc))
        finally:
            r['cutoff_seconds']=time.perf_counter()-start
            if proc is not None:
                r['kill_seconds']=time.perf_counter()-start
                try:os.killpg(proc.pid,signal.SIGKILL)
                except ProcessLookupError:pass
                try:proc.wait(timeout=5)
                except subprocess.TimeoutExpired:r['errors'].append('reap timeout')
                r['reap_seconds']=time.perf_counter()-start;r['exit_code']=proc.returncode
                if proc.returncode!=-9:r['errors'].append('unexpected exit code')
                while b'\n' in pending:
                    line,pending=pending.split(b'\n',1);consume(m,line,receipt,r,start)
                rest=proc.stdout.read() or b'';total+=len(rest)
                with (folder/'stdout.bin').open('ab') as raw:raw.write(rest)
                pending+=rest;receipt=time.perf_counter()-start
                while b'\n' in pending:
                    line,pending=pending.split(b'\n',1);consume(m,line,receipt,r,start)
                proc.stdout.close()
            else:
                (folder/'stdout.bin').touch(exist_ok=True);(folder/'stderr.bin').touch(exist_ok=True)
            if total>16*2**20:r['errors'].append('oversized total output')
            r['partial_base64']=base64.b64encode(pending).decode()
    r['total_seconds']=time.perf_counter()-start;r['stdout_bytes']=total
    if r['ready']!=1:r['errors'].append('readiness not observed')
    if r.get('cutoff_seconds',budget)<budget and not r['errors']:r['errors'].append('early cutoff')
    r['status']='INVALID' if r['errors'] else 'VALID'
    write(folder/'record.json',r)
    return r


def corpus(m):
    items={}
    for x in m['instances']:
        p=HERE/'instances'/x['file']
        if build.sha(p)!=x['sha256']:raise ValueError('input hash mismatch')
        items[x['id']]=json.loads(p.read_text())
    for i,x in enumerate(m['preflight']['smoke_fixtures']):
        p=ROOT/x['file']
        if build.sha(p)!=x['sha256']:raise ValueError('smoke hash mismatch')
        items['smoke'+str(i)]=json.loads(p.read_text())
    return items


def main():
    phase_start=time.perf_counter()
    p=argparse.ArgumentParser();p.add_argument('--phase',choices=['preflight','main'],required=True)
    p.add_argument('--output',type=Path,required=True);p.add_argument('--preflight',type=Path);a=p.parse_args()
    m=build.manifest();build_record=build.verify();items=corpus(m)
    if platform.platform()!=m['environment']['os']:raise ValueError('OS contract changed')
    if m['environment']['cpu'] not in Path('/proc/cpuinfo').read_text():raise ValueError('CPU contract changed')
    prior=0.;preflight_link=None
    if a.phase=='main':
        if a.preflight is None:raise ValueError('preflight required')
        audit=json.loads((a.preflight/'audit.json').read_text());result=json.loads((a.preflight/'analysis.json').read_text())
        old=json.loads((a.preflight/'metadata.json').read_text());terminal=json.loads((a.preflight/'terminal.json').read_text())
        if audit['status']!='PASS' or result['status']!='PASS' or terminal['status']!='COMPLETE':raise ValueError('preflight not admitted')
        if old['build']!=build_record:raise ValueError('different instrument from preflight')
        # Fail closed if the audited evidence was edited after its review.
        for path,h in audit['artifact_hashes'].items():
            if build.sha(a.preflight/path)!=h:raise ValueError('preflight artifact changed')
        if not (a.preflight/'review.json').is_file() or json.loads((a.preflight/'review.json').read_text()).get('status')!='PASS':raise ValueError('independent preflight review absent')
        prior=terminal['elapsed_seconds'];preflight_link=str(a.preflight.resolve())
    os.sched_setaffinity(0,{0})
    # Registration-wide one-shot guard, retained even for an interrupted campaign.
    guard=HERE/('.'+a.phase+'_started')
    with guard.open('x') as f:f.write(str(a.output.resolve())+'\n')
    a.output.mkdir(parents=True,exist_ok=False)
    meta={'experiment':'MQ-DIFFICULTY-001','phase':a.phase,'git_commit':build.git('rev-parse','HEAD'),
          'git_dirty':bool(build.git('status','--porcelain','--untracked-files=no')),'build':build_record,
          'manifest':m,'started_unix':time.time(),'argv':sys.argv,'preflight':preflight_link,
          'parent_affinity':sorted(os.sched_getaffinity(0)),'environment':{'os':platform.platform(),'python':sys.version,
          'cpu':Path('/proc/cpuinfo').read_text(),'memory':Path('/proc/meminfo').read_text(),'load':os.getloadavg()},'prior_elapsed_seconds':prior}
    write(a.output/'metadata.json',meta)
    (a.output/'README.md').write_text('MQ-DIFFICULTY-001 '+a.phase+' retained run.\n\nExact command and build/source provenance: metadata.json.\nRaw streams and records: cellNNNN directories.\nAudit: `python3 research/experiments/mqlib_difficulty_qualification/independent_audit.py '+str(a.output)+'`.\nOne-shot guard prevents replacement; reproduction on a clean checkout requires an explicitly labelled replication.\n')
    records=[];start=phase_start;status='COMPLETE';reason='';result={'status':'NO_VERDICT','reason':'not completed'}
    try:
        for i,cell in enumerate(schedule(m,a.phase)):
            if prior+time.perf_counter()-start+cell['budget']+5>m['wall_cap_seconds']:
                status='ABORT';reason='cumulative cap';break
            r=run_cell(items[cell['instance']],cell,a.output/f'cell{i:04}');records.append(r)
            if r['status']!='VALID':status='INVALID';reason=f'cell{i:04}';break
            if i%20==0:print(a.phase,i+1,flush=True)
        result=preflight(records,m) if a.phase=='preflight' else main_analysis(records,m)
    except BaseException as exc:
        status='ABORT';reason=repr(exc);raise
    finally:
        elapsed=time.perf_counter()-start
        if prior+elapsed>m['wall_cap_seconds']:status='ABORT';reason='cumulative cap exceeded'
        if status!='COMPLETE':result={'status':'NO_VERDICT','reason':reason}
        write(a.output/'analysis.json',result)
        write(a.output/'terminal.json',{'status':status,'reason':reason,'cells':len(records),'elapsed_seconds':elapsed,'ended_unix':time.time()})
    print(json.dumps({'status':status,'analysis':result['status'],'cells':len(records)}))


if __name__=='__main__':main()
