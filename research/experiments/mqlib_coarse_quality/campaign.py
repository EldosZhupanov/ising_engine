"""MQ-QUALITY-002: one admission and one main; old failure stays closed."""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import platform
import statistics as st
import sys
import time
from bindings import HERE,ROOT,worker,old_analysis
import freeze

ARMS=['ultimate','v2_default','mqlib_merz','mqlib_mst2']


def schedule(m,phase):
    if phase=='admission':
        for name in m['preflight']['probe_instances']:
            for arm in ARMS:yield dict(instance=name,arm=arm,seed=0,mode='null_a',budget=2.,block='probe')
        for arm in ARMS:yield dict(instance='smoke',arm=arm,seed=501,mode='search',budget=2.,block='smoke')
    else:
        for i,item in enumerate(m['instances']):
            for j,s in enumerate(m['search_seeds']):
                k=(i+j)%4
                for arm in ARMS[k:]+ARMS[:k]:yield dict(instance=item['id'],arm=arm,seed=s,mode='search',budget=2.,block='main')


def score(r):
    values=[e['verified_energy'] for e in r['events'] if 'verified_energy' in e and max(e['received_seconds'],e['validated_seconds'])<2]
    return min([r['fallback_energy']]+values),len(values)


def valid_records(rs,m,phase):
    cells=list(schedule(m,phase))
    return len(rs)==len(cells) and all(r['cell']==c and r['status']=='VALID' and r['kill_seconds']<=2.+.100 for r,c in zip(rs,cells))


def analyze(rs,m,phase):
    if not valid_records(rs,m,phase):return {'status':'NO_VERDICT','reason':'incomplete/invalid schedule or kill bound'}
    if phase=='admission':
        ok=all(score(r)[1]>=1 and (r['cell']['block']!='probe' or (score(r)[1]==1 and all(e.get('verified_energy',r['fallback_energy'])==r['fallback_energy'] for e in r['events']))) for r in rs)
        return {'status':'PASS' if ok else 'FAIL','cells':20,'eligible_witness_cells':sum(score(r)[1]>0 for r in rs),'max_kill_overshoot':max(r['kill_seconds']-2 for r in rs)}
    result=[];wins={a:0 for a in ARMS};informative=0;contrasts={a+'_vs_'+b:[] for a,b in itertools.combinations(ARMS,2)}
    for item in m['instances']:
        group=[r for r in rs if r['cell']['instance']==item['id']];L=item['normalization_L']
        values={a:[score(r)[0] for r in group if r['cell']['arm']==a] for a in ARMS}
        means={a:st.mean(v) for a,v in values.items()};rank=sorted(ARMS,key=means.get)
        spread=(means[rank[-1]]-means[rank[0]])/L;top=(means[rank[1]]-means[rank[0]])/L
        informative+=spread>=.001
        if top>=.001:wins[rank[0]]+=1
        paired={}
        for a,b in itertools.combinations(ARMS,2):
            d=[y-x for x,y in zip(values[a],values[b])];key=a+'_vs_'+b
            contrasts[key].append(st.mean(d)/L)
            paired[key]={'wins':sum(x>0 for x in d),'ties':sum(x==0 for x in d),'losses':sum(x<0 for x in d),'mean_normalized_improvement':st.mean(d)/L}
        result.append({'instance':item['id'],'energies':{a:old_analysis.summary(v) for a,v in values.items()},'fallback_only':{a:sum(score(r)[1]==0 for r in group if r['cell']['arm']==a) for a in ARMS},'paired':paired,'spread':spread,'top_two_spread':top})
    draws=list(itertools.islice(old_analysis.bootstrap_indices(),10000));intervals={}
    for key,v in contrasts.items():
        boot=sorted(st.mean(v[i] for i in row) for row in draws)
        def q(p):
            x=p*9999;lo=int(x);return boot[lo]+(x-lo)*(boot[lo+1]-boot[lo])
        intervals[key]={'mean':st.mean(v),'descriptive_95':[q(.025),q(.975)]}
    return {'status':'COMPLETE','instances':result,'informative_instances':informative,'unique_wins':wins,'H1':'SUPPORTED_ON_QUALIFICATION' if informative>=8 else 'INCONCLUSIVE','H2':'SUPPORTED_ON_QUALIFICATION' if sum(v>=4 for v in wins.values())>=2 else 'NOT_ESTABLISHED','descriptive_instance_bootstrap':intervals}


def admitted(folder,provenance):
    read=lambda p:json.loads(p.read_text())
    audit=read(folder/'audit.json');review=read(folder/'review.json');meta=read(folder/'metadata.json');terminal=read(folder/'terminal.json')
    if audit['status']!='PASS' or read(folder/'analysis.json')['status']!='PASS' or terminal['status']!='COMPLETE':raise ValueError('admission failed')
    if review['status']!='PASS' or review['audit_sha256']!=freeze.sha(folder/'audit.json'):raise ValueError('review missing or stale')
    if meta['provenance']!=provenance:raise ValueError('different admission instrument')
    for name,h in audit['artifact_hashes'].items():
        if freeze.sha(folder/name)!=h:raise ValueError('admission evidence drift')
    return terminal['elapsed_seconds']


def main():
    start=time.perf_counter()
    p=argparse.ArgumentParser();p.add_argument('--phase',choices=['admission','main'],required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--admission',type=Path);a=p.parse_args()
    m=freeze.manifest();provenance=freeze.verify();prior=0.
    if platform.platform()!=m['environment']['os'] or m['environment']['cpu'] not in Path('/proc/cpuinfo').read_text():raise ValueError('host changed')
    if a.phase=='main':
        if a.admission is None:raise ValueError('admission required')
        prior=admitted(a.admission,provenance)
    models={}
    for item in m['instances']+[dict(id='smoke',**m['preflight']['smoke_fixture'])]:
        path=ROOT/item['file']
        if freeze.sha(path)!=item['sha256']:raise ValueError('input hash mismatch')
        models[item['id']]=json.loads(path.read_text())
    os.sched_setaffinity(0,{0})
    with (HERE/('.'+a.phase+'_started')).open('x') as f:f.write(str(a.output.resolve())+'\n')
    a.output.mkdir(parents=True,exist_ok=False)
    worker.write(a.output/'metadata.json',{'experiment_id':m['id'],'phase':a.phase,'git_commit':freeze.git('rev-parse','HEAD'),'git_dirty':bool(freeze.git('status','--porcelain','--untracked-files=no')),'provenance':provenance,'manifest':m,'argv':sys.argv,'prior_elapsed_seconds':prior,'parent_affinity':sorted(os.sched_getaffinity(0)),'started_unix':time.time(),'environment':{'os':platform.platform(),'cpu':Path('/proc/cpuinfo').read_text(),'ram':Path('/proc/meminfo').read_text(),'python':sys.version,'load':os.getloadavg()},'admission':str(a.admission) if a.admission else None})
    (a.output/'README.md').write_text('MQ-QUALITY-002 '+a.phase+' retained evidence.\n\nExact command/source/binary provenance: metadata.json. Raw streams and records: cellNNNN/.\nOffline audit: `python3 research/experiments/mqlib_coarse_quality/check.py '+str(a.output)+'`.\nUse frozen instrument on clean checkout for a separately labelled replication; do not overwrite or delete guards.\n')
    rs=[];status='COMPLETE';reason='';result={'status':'NO_VERDICT'}
    try:
        for i,cell in enumerate(schedule(m,a.phase)):
            if prior+time.perf_counter()-start+7>m['wall_cap_seconds']:status='ABORT';reason='cumulative cap';break
            r=worker.run_cell(models[cell['instance']],cell,a.output/f'cell{i:04}');rs.append(r)
            if r['status']!='VALID' or r.get('kill_seconds',float('inf'))>2.+.1:status='INVALID';reason=f'cell{i:04}';break
            if i%20==0:print(a.phase,i+1,flush=True)
        result=analyze(rs,m,a.phase)
    except BaseException as exc:status='ABORT';reason=repr(exc);raise
    finally:
        elapsed=time.perf_counter()-start
        if prior+elapsed>m['wall_cap_seconds']:status='ABORT';reason='cumulative cap exceeded'
        if status!='COMPLETE':result={'status':'NO_VERDICT','reason':reason}
        worker.write(a.output/'analysis.json',result)
        worker.write(a.output/'terminal.json',{'status':status,'reason':reason,'cells':len(rs),'elapsed_seconds':elapsed,'ended_unix':time.time()})
    print(json.dumps({'status':status,'analysis':result['status'],'cells':len(rs)}))


if __name__=='__main__':main()
