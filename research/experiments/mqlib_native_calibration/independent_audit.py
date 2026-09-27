"""Offline raw-evidence audit. No imports from analyzer, build, adapter or solver."""
import argparse
from collections import defaultdict
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import statistics as st
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
ARMS=('null_a','null_b','delay')
WORKERS=('rust_model','mqlib_model')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def need(condition, message):
    if not condition: raise ValueError(message)


def quant95(xs):
    return sorted(xs)[math.ceil(len(xs)*.95)-1] if xs else None


def distribution(xs):
    if not xs: return {'n':0}
    s=sorted(xs)
    def linear(p):
        pos=(len(s)-1)*p
        i=int(pos)
        return s[i]*(1-pos+i)+s[min(i+1,len(s)-1)]*(pos-i)
    return dict(n=len(xs),mean=st.mean(xs),median=st.median(xs),std=st.pstdev(xs),
                min=min(xs),max=max(xs),q25=linear(.25),q75=linear(.75),q95_nearest_rank=quant95(xs))


def equal(a,b):
    if isinstance(a,dict): return isinstance(b,dict) and a.keys()==b.keys() and all(equal(a[k],b[k]) for k in a)
    if isinstance(a,list): return isinstance(b,list) and len(a)==len(b) and all(equal(x,y) for x,y in zip(a,b))
    if type(a) is float: return type(b) in (int,float) and math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-6)
    return a==b


def audit(folder):
    meta=json.loads((folder/'metadata.json').read_text())
    summary=json.loads((folder/'summary.json').read_text())
    manifest=json.loads((HERE/'manifest.json').read_text())
    need(meta['manifest']==manifest,'manifest drift')
    need(meta['affinity']==[0],'parent metadata affinity')
    need(meta['thread_environment']==dict.fromkeys(['RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS'],'1'),'threads')
    need(summary['raw_sha256']==digest(folder/'raw.jsonl'),'raw digest')
    for name,h in meta['hashes'].items():
        need(digest(ROOT/name)==h,'current hash '+name)
        if not Path(name).is_absolute() and not name.startswith(('.cache/','target/')):
            data=subprocess.check_output(['git','show',meta['git_commit']+':'+name],cwd=ROOT)
            need(hashlib.sha256(data).hexdigest()==h,'instrument Git hash '+name)
    build=meta['build']
    need(not build['development'],'development binary')
    need(build['source_hashes'].items()<=meta['hashes'].items(),'build inputs differ')
    for spec in build['binaries'].values():
        need(meta['hashes'][spec['path']]==spec['sha256'],'build binary hash')
    need(build['upstream_build']['upstream_commit']=='585496274af5abb0849d0d47e135496b4688680b','upstream pin')
    for name,h in build['upstream_build']['hashes'].items(): need(digest(ROOT/name)==h,'upstream artifact '+name)
    need(all(c['exit_code']==0 for c in build['commands']),'build exit')
    models={}
    for f in manifest['fixtures']:
        need(digest(HERE/f['file'])==f['sha256'],'fixture hash')
        models[f['file']]=json.loads((HERE/f['file']).read_text())
    rows=[json.loads(l) for l in (folder/'raw.jsonl').read_text().splitlines()]
    expected=[]
    if meta['smoke']:
        expected=[(w,manifest['fixtures'][0]['file'],0,'null_a',200000000) for w in WORKERS]
    else:
        for wi,w in enumerate(WORKERS):
            for fi,f in enumerate(manifest['fixtures']):
                for bi,b in enumerate(manifest['budgets_seconds']):
                    for repeat in range(20):
                        rotate=(wi+fi+bi+repeat)%3
                        expected.extend((w,f['file'],repeat,a,round(b*1e9)) for a in ARMS[rotate:]+ARMS[:rotate])
    need([(r['worker'],r['fixture'],r['repeat'],r['arm'],r['budget_ns']) for r in rows]==expected,'cell order/cardinality')
    count=0
    checked=[]
    for row in rows:
        need(row['status']=='VALID' and row['errors']==[],'invalid row')
        need(row['parent_affinity']==row['child_affinity']==[0] and row['cpu']==0,'cell affinity')
        need(row['exit_code']==-9 and row['killed_at_deadline'] and not row['eof'] and not row['campaign_interrupted'],'termination')
        need(row['budget_ns']<=row['cutoff_ns']<=row['kill_sent_ns']-row['start_ns']<=row['reaped_ns']-row['start_ns']<=row['total_ns'],'phase order')
        need(row['kill_reap_ns']==row['reaped_ns']-row['kill_sent_ns'],'teardown time')
        need(row['delay_ns']==(row['budget_ns']//4 if row['arm']=='delay' else 0),'delay contract')
        need(row['command'][:3]==['taskset','-c','0'],'child command affinity')
        need(row['command'][3]==str(ROOT/build['binaries'][row['worker']]['path']),'worker executable')
        need(row['command'][-1]==str(row['delay_ns']),'command delay')
        raw=bytes.fromhex(row['raw_hex'])
        pieces=raw.split(b'\n')
        need(pieces[-1]==bytes.fromhex(row['partial_hex']),'partial bytes')
        need([p.decode() for p in pieces[:-1]]==[e['raw'] for e in row['events']],'raw reconstruction')
        model=models[row['fixture']];n=len(model['linear'])
        states=[[0]*n,[1]*n,[i%2 for i in range(n)],[int(i%3==0) for i in range(n)]]
        energies=[]
        for state in states:
            e=Fraction(model['offset'])+sum(Fraction(h)*v for h,v in zip(model['linear'],state))+sum(Fraction(w)*state[i]*state[j] for i,j,w in model['pairs'])
            energies.append(float(Fraction(model['offset'])-e if row['worker']=='mqlib_model' else e))
        kinds=[];times=[];diag=None
        last=row['start_ns']
        for ev in row['events']:
            count+=1
            d=json.loads(ev['raw']);k=d.get('kind');kinds.append(k)
            need(ev['decoded']==d and ev['kind']==k,'decoded raw mismatch')
            latency=ev['receipt_ns']-row['start_ns']
            need(last<=ev['receipt_ns']<=row['start_ns']+row['total_ns'],'receipt order')
            last=ev['receipt_ns']
            need(ev['latency_ns']==latency and ev['eligible']==(latency<row['budget_ns']),'receipt eligibility')
            times.append(latency)
            if k=='inc':
                keys={'kind','state','objective'} if row['worker']=='mqlib_model' else {'kind','state','energy','chunk'}
                need(set(d)==keys and d['state']==[0]*n and all(type(v) is int for v in d['state']),'zero state/schema')
                value=d.get('objective') if row['worker']=='mqlib_model' else d.get('energy')
                need(type(value) in (int,float) and math.isfinite(value) and value==energies[0],'zero energy')
                if row['worker']=='rust_model': need(d['chunk']==0 and type(d['chunk']) is int,'chunk')
            elif k=='diagnostic':
                need(set(d)=={'kind','energies','ready_ns','sleep_ns','affinity'},'diagnostic schema')
                need(d['energies']==energies and len(d['energies'])==4 and all(type(v) in (int,float) and math.isfinite(v) for v in d['energies']),'four state energies')
                need(all(type(d[t]) is int and d[t]>=0 for t in ('ready_ns','sleep_ns')),'relative times')
                need(d['ready_ns']+d['sleep_ns']<=latency and d['affinity']=='0','ready/affinity')
                diag=d
            else: raise ValueError('unknown event')
        need(len(kinds)==len(set(kinds)) and len(kinds)<=2,'duplicate events')
        done=max(times) if len(kinds)==2 and max(times)<row['budget_ns'] else None
        checked.append((row,done,diag))
    if meta['smoke']:
        need(all(t is not None for _,t,_ in checked),'smoke completion')
        need(summary['status']=='SMOKE_PASS','smoke verdict')
    else:
        groups=[]
        for w in WORKERS:
            for f in manifest['fixtures']:
                for b in manifest['budgets_seconds']:
                    ns=round(b*1e9);d=ns//4
                    batch=[v for v in checked if v[0]['worker']==w and v[0]['fixture']==f['file'] and v[0]['budget_ns']==ns]
                    need(len(batch)==60,'group cardinality')
                    on={(r['repeat'],r['arm']):t for r,t,_ in batch if t is not None}
                    lat={a:[t for (i,arm),t in on.items() if arm==a] for a in ARMS}
                    ab=[on[i,'null_b']-on[i,'null_a'] for i in range(20) if (i,'null_a') in on and (i,'null_b') in on]
                    da=[on[i,'delay']-on[i,'null_a'] for i in range(20) if (i,'null_a') in on and (i,'delay') in on]
                    over=[r['cutoff_ns']-ns for r,_,_ in batch]
                    sleeps=[v['sleep_ns'] for r,_,v in batch if r['arm']=='delay' and v is not None]
                    osleep=[s-d for s in sleeps]
                    gates=dict(completion=all(len(lat[a])>=19 for a in ARMS) and len(ab)>=19 and len(da)>=19,
                               deadline=min(over)>=0 and quant95(over)<=max(1000000,.05*ns),
                               null=bool(ab) and abs(st.median(ab))<=max(2000000,.1*ns),
                               positive=bool(da) and abs(st.median(da)-d)<=max(2000000,.2*d) and sum(v>0 for v in da)>=18,
                               sleep=len(lat['delay'])>=19 and bool(osleep) and min(osleep)>=0 and quant95(osleep)<=max(1000000,.1*d),
                               affinity=True)
                    g=dict(worker=w,fixture=f['file'],budget_ns=ns,status='PASS' if all(gates.values()) else 'FAIL',gates=gates,
                           on_time_completions={a:len(lat[a]) for a in ARMS},latency_ns_by_arm={a:distribution(lat[a]) for a in ARMS},
                           null_difference_ns=distribution(ab),positive_difference_ns=distribution(da),deadline_overshoot_ns=distribution(over),
                           actual_delay_sleep_ns=distribution(sleeps),oversleep_ns=distribution(osleep),
                           model_ready_ns=distribution([v['ready_ns'] for _,_,v in batch if v is not None]),
                           kill_reap_ns=distribution([r['kill_reap_ns'] for r,_,_ in batch]),total_ns=distribution([r['total_ns'] for r,_,_ in batch]),
                           complete_but_late=sum(len(r['events'])==2 and t is None for r,t,_ in batch),
                           without_both_lines=sum(len(r['events'])<2 for r,_,_ in batch))
                    groups.append(g)
        need(equal(groups,summary['groups']),'independent statistics/gates differ')
        status='PASS' if all(g['status']=='PASS' for g in groups) else 'FAIL'
        need(summary['status']==status,'registered status')
        qualified=[round(b*1e9) for b in manifest['budgets_seconds'] if all(g['status']=='PASS' for g in groups if g['budget_ns']==round(b*1e9))]
        need(summary['qualified_model_budget_ns']==qualified,'admitted budgets')
    need(summary['cells']==len(rows) and summary['elapsed_seconds']<manifest['hard_campaign_cap_seconds'],'count/cap')
    return {'audit':'PASS','registered_status':summary['status'],'cells':len(rows),
            'messages_checked':count,'source_hashes_checked':len(meta['hashes']),
            'on_time_completions':sum(t is not None for _,t,_ in checked),
            'raw_sha256':summary['raw_sha256'],'auditor_sha256':digest(__file__),
            'scope':'Independent arithmetic/provenance replay; no analyzer/solver imports or optimizer execution'}


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('folder',type=Path)
    args=p.parse_args();print(json.dumps(audit(args.folder),indent=2))
