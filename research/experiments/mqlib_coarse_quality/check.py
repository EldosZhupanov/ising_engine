"""Offline independent audit; no calls to coordinator scoring or search."""
import argparse
import base64
import hashlib
import importlib.util
import itertools
import json
import math
from pathlib import Path
import statistics as st
import subprocess

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
OLD=HERE.parent/'mqlib_difficulty_qualification'
spec=importlib.util.spec_from_file_location('immutable_raw_checker',OLD/'independent_audit.py')
rawcheck=importlib.util.module_from_spec(spec);spec.loader.exec_module(rawcheck)
ARMS=['ultimate','v2_default','mqlib_merz','mqlib_mst2']
PREREG='4d0b3f4944ef4ae5849a824d26acd4c6c197e29c'


def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def read(p):return json.loads(Path(p).read_text(),object_pairs_hook=rawcheck.unique)
def close(a,b):assert math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-12),(a,b)


def expected(m,phase):
    out=[]
    if phase=='admission':
        for name in ['q05','q11','q17','q23']:
            for arm in ARMS:out.append(dict(instance=name,arm=arm,seed=0,mode='null_a',budget=2.,block='probe'))
        for arm in ARMS:out.append(dict(instance='smoke',arm=arm,seed=501,mode='search',budget=2.,block='smoke'))
    else:
        for i,item in enumerate(m['instances']):
            for j,seed in enumerate(m['search_seeds']):
                for k in range(4):out.append(dict(instance=item['id'],arm=ARMS[(i+j+k)%4],seed=seed,mode='search',budget=2.,block='main'))
    return out


def endpoint(r):
    eligible=[e['verified_energy'] for e in r['events'] if 'verified_energy' in e and e['received_seconds']<2 and e['validated_seconds']<2]
    return min([r['fallback_energy']]+eligible),len(eligible)


def provenance(meta):
    m=meta['manifest'];p=meta['provenance']
    for name in ['manifest.json','protocol.md']:
        path=HERE/name;rel=str(path.relative_to(ROOT))
        assert subprocess.check_output(['git','show',PREREG+':'+rel],cwd=ROOT)==path.read_bytes(),'registration drift'
    assert m==read(HERE/'manifest.json') and p['registration_commit']==PREREG
    required={str((HERE/name).relative_to(ROOT)) for name in ['protocol.md','manifest.json','bindings.py','freeze.py','campaign.py','check.py','test_campaign.py']}
    assert set(p['sources'])==required,'missing source coverage'
    for name,h in p['sources'].items():
        assert sha(ROOT/name)==h
        assert hashlib.sha256(subprocess.check_output(['git','show',p['git_commit']+':'+name],cwd=ROOT)).hexdigest()==h
    native=p['native_build'];old_manifest=read(OLD/'manifest.json')
    rawcheck.provenance({'manifest':old_manifest,'build':native})
    assert sha(OLD/'manifest.json')==m['inherited_manifest_sha256']
    for name,h in native['source_hashes'].items():
        assert sha(ROOT/name)==h
        assert hashlib.sha256(subprocess.check_output(['git','show',native['git_commit']+':'+name],cwd=ROOT)).hexdigest()==h
    for name,h in native['binaries'].items():assert sha(ROOT/name)==h
    for name,h in native['upstream_build']['hashes'].items():assert sha(ROOT/name)==h


def audit(folder):
    meta=read(folder/'metadata.json');m=meta['manifest'];provenance(meta)
    assert not meta['git_dirty'] and meta['parent_affinity']==[0]
    terminal=read(folder/'terminal.json');result=read(folder/'analysis.json');rs=[];witnesses=late=0
    assert meta['phase'] in ['admission','main']
    if meta['phase']=='admission':
        assert meta['prior_elapsed_seconds']==0 and meta['admission'] is None
    else:
        admission=ROOT/meta['admission']
        prior=read(admission/'metadata.json');review=read(admission/'review.json');saved=read(admission/'audit.json')
        verified=audit(admission)
        assert verified==saved and saved['status']=='PASS' and saved['registered_status']=='PASS'
        assert review['status']=='PASS' and review['audit_sha256']==sha(admission/'audit.json')
        assert prior['phase']=='admission' and prior['provenance']==meta['provenance']
        close(meta['prior_elapsed_seconds'],read(admission/'terminal.json')['elapsed_seconds'])
    models={}
    for item in m['instances']+[dict(id='smoke',**m['preflight']['smoke_fixture'])]:
        assert sha(ROOT/item['file'])==item['sha256'];models[item['id']]=read(ROOT/item['file'])
    plan=expected(m,meta['phase']);paths=sorted(folder.glob('cell*/record.json'))
    assert len(paths)==terminal['cells'] and len(paths)<=len(plan)
    for i,path in enumerate(paths):
        assert path.parent.name==f'cell{i:04}'
        r=read(path);c=r['cell'];assert c==plan[i];model=models[c['instance']]
        assert r['fallback_energy']==model['offset'] and r['parent_affinity']==[0]
        assert r['status']==('INVALID' if r['errors'] else 'VALID')
        stream=(path.parent/'stdout.bin').read_bytes()
        assert stream==b''.join(base64.b64decode(e['raw_base64'])+b'\n' for e in r['events'])+base64.b64decode(r['partial_base64'])
        assert len(stream)==r['stdout_bytes']
        ready=config=0;last_chunk=-1;last_energy=float('inf');previous=0
        for e in r['events']:
            assert e['received_seconds']>=0 and e['validated_seconds']>=max(previous,e['received_seconds']);previous=e['validated_seconds']
            if 'error' in e:assert r['status']=='INVALID';continue
            event=json.loads(base64.b64decode(e['raw_base64']),object_pairs_hook=rawcheck.unique);assert event==e['message']
            rawcheck.message_schema(event,c)
            if event['kind']=='ready':ready+=1;assert ready==1 and event['affinity']=='0'
            elif event['kind']=='config':
                config+=1;assert ready==1 and config==1 and c['arm']=='v2_default' and c['mode']=='search'
                assert event['seed']==(c['seed']*0x9e3779b97f4a7c15)%(2**64) and event['replicas']==32
                assert event['backend'] in ['SparseBitSlice','DenseByte'] and 1<=len(event['operators'])<=2 and len(event['temperatures'])==32
                for j,t in enumerate(event['temperatures']):close(t,4*(.08/4)**(j/31))
            else:
                assert ready==1
                if c['arm']=='v2_default' and c['mode']=='search':assert config==1
                v=rawcheck.energy(model,event['state']);assert abs(v-e['verified_energy'])<=1e-9
                rawcheck.verify_reported_energy(v,event,c['arm'].startswith('mqlib_'),model['offset'])
                if c['mode']=='search':
                    assert v<last_energy;last_energy=v
                    if not c['arm'].startswith('mqlib_'):
                        assert type(event['chunk']) is int and event['chunk']>=0 and event['chunk']>last_chunk;last_chunk=event['chunk']
                else:assert not any(event['state']) and type(event['sleep_ns']) is int and event['sleep_ns']>=0
                witnesses+=1;late+=max(e['received_seconds'],e['validated_seconds'])>=2
        if r['status']=='VALID':
            assert ready==r['ready']==1 and config==r['configs']
            assert 2<=r['cutoff_seconds']<=r['kill_seconds']<=r['reap_seconds']<=r['total_seconds']
            assert r['exit_code']==-9 and r['reap_seconds']-r['kill_seconds']<=5.1
            ready_at=next(e['validated_seconds'] for e in r['events'] if e.get('message',{}).get('kind')=='ready')
            for obs in r['observations']:
                assert len(obs['tasks'])<=(2 if c['arm'] in ['ultimate','v2_default'] and c['mode']=='search' else 1)
                if obs['seconds']>=ready_at:assert all(t['affinity']=='0' for t in obs['tasks'])
        rs.append(r)
    complete=len(rs)==len(plan) and terminal['status']=='COMPLETE' and all(r['status']=='VALID' and r['kill_seconds']<=2.+.100 for r in rs)
    if not complete:assert result['status']=='NO_VERDICT'
    elif meta['phase']=='admission':
        eligible=sum(endpoint(r)[1]>0 for r in rs)
        probe_ok=all(endpoint(r)[1]==1 and all(e.get('verified_energy',r['fallback_energy'])==r['fallback_energy'] for e in r['events']) for r in rs if r['cell']['block']=='probe')
        assert result['status']==('PASS' if eligible==20 and probe_ok else 'FAIL')
        assert result['cells']==20 and result['eligible_witness_cells']==eligible
        close(max(r['kill_seconds']-2 for r in rs),result['max_kill_overshoot'])
    else:
        informative=0;wins={a:0 for a in ARMS};contrasts={a+'_vs_'+b:[] for a,b in itertools.combinations(ARMS,2)}
        for i,item in enumerate(m['instances']):
            out=result['instances'][i];assert out['instance']==item['id'];L=item['normalization_L']
            assert L==sum(abs(h) for h in models[item['id']]['linear'])+sum(abs(w) for _,_,w in models[item['id']]['pairs'])
            group=[r for r in rs if r['cell']['instance']==item['id']];values={a:[endpoint(r)[0] for r in group if r['cell']['arm']==a] for a in ARMS}
            means={a:st.mean(v) for a,v in values.items()};rank=sorted(ARMS,key=means.get)
            spread=(means[rank[-1]]-means[rank[0]])/L;top=(means[rank[1]]-means[rank[0]])/L
            close(spread,out['spread']);close(top,out['top_two_spread']);informative+=spread>=.001
            if top>=.001:wins[rank[0]]+=1
            for a,v in values.items():
                summary=out['energies'][a];assert summary['values']==v and summary['n']==10
                for key,value in dict(mean=st.mean(v),median=st.median(v),std=st.pstdev(v),min=min(v),max=max(v)).items():close(value,summary[key])
                assert out['fallback_only'][a]==sum(endpoint(r)[1]==0 for r in group if r['cell']['arm']==a)
            for a,b in itertools.combinations(ARMS,2):
                key=a+'_vs_'+b;delta=[y-x for x,y in zip(values[a],values[b])];d=st.mean(delta)/L;contrasts[key].append(d)
                assert out['paired'][key]==dict(wins=sum(x>0 for x in delta),ties=sum(x==0 for x in delta),losses=sum(x<0 for x in delta),mean_normalized_improvement=d)
        assert result['informative_instances']==informative and result['unique_wins']==wins
        assert result['H1']==('SUPPORTED_ON_QUALIFICATION' if informative>=8 else 'INCONCLUSIVE')
        assert result['H2']==('SUPPORTED_ON_QUALIFICATION' if sum(v>=4 for v in wins.values())>=2 else 'NOT_ESTABLISHED')
        indices=[];counter=0
        while len(indices)<240000:
            h=hashlib.sha256(f'MQ-DIFFICULTY-001/bootstrap/v1/{counter}'.encode()).digest();counter+=1
            for offset in [0,8,16,24]:
                v=int.from_bytes(h[offset:offset+8],'big')
                if v<(2**64//24)*24:indices.append(v%24)
        for key,v in contrasts.items():
            samples=sorted(st.mean(v[indices[k+j]] for j in range(24)) for k in range(0,240000,24))
            out=result['descriptive_instance_bootstrap'][key];close(st.mean(v),out['mean'])
            for i,q in enumerate([.025,.975]):
                x=q*9999;lo=int(x);close(samples[lo]+(x-lo)*(samples[lo+1]-samples[lo]),out['descriptive_95'][i])
    if terminal['status']=='COMPLETE':assert meta['prior_elapsed_seconds']+terminal['elapsed_seconds']<=3000
    return {'status':'PASS','scope':'artifact integrity, not optimizer replication','cells':len(rs),'witnesses':witnesses,'late_witnesses':late,'registered_status':result['status'],'artifact_hashes':{str(p.relative_to(folder)):sha(p) for p in sorted(folder.rglob('*')) if p.is_file() and p.name not in ['audit.json','review.json']}}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('folder',type=Path);a=p.parse_args();r=audit(a.folder)
    with (a.folder/'audit.json').open('x') as f:json.dump(r,f,indent=2);f.write('\n')
    print({k:v for k,v in r.items() if k!='artifact_hashes'})
