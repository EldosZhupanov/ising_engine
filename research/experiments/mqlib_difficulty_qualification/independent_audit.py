"""Independent raw-data auditor: no imports from runner, scoring, or solver code."""
import argparse
import base64
import hashlib
import itertools
import json
import math
from pathlib import Path
import statistics as st
import subprocess

ROOT=Path(__file__).resolve().parents[3]
ARMS=['ultimate','v2_default','mqlib_merz','mqlib_mst2']


def unique(pairs):
    d={}
    for k,v in pairs:
        assert k not in d,'duplicate JSON key'
        d[k]=v
    return d


def read(p):return json.loads(Path(p).read_text(),object_pairs_hook=unique)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def close(x,y):assert math.isclose(x,y,rel_tol=1e-12,abs_tol=1e-12),(x,y)


def energy(m,x):
    assert len(x)==len(m['linear']) and all(type(b) is int and b in [0,1] for b in x)
    total=m['offset']
    for i in range(len(x)):total+=m['linear'][i]*x[i]
    for i,j,w in m['pairs']:total+=w*x[i]*x[j]
    return total


def best(r,t):
    eligible=[e['verified_energy'] for e in r['events'] if 'verified_energy' in e and max(e['received_seconds'],e['validated_seconds'])<t]
    return min([r['fallback_energy']]+eligible),len(eligible)


def expected(m,phase):
    out=[]
    if phase=='main':
        for i,x in enumerate(m['instances']):
            for j,s in enumerate(m['search_seeds']):
                for k in range(4):out.append(dict(instance=x['id'],arm=ARMS[(i+j+k)%4],seed=s,mode='search',budget=2.,block='main'))
    else:
        for i,x in enumerate(m['preflight']['instances']):
            for rep in range(10):
                for k in range(4):
                    ai=(i+rep+k)%4
                    for z in range(3):out.append(dict(instance=x,arm=ARMS[ai],seed=0,mode=['null_a','null_b','delay'][(i+rep+ai+z)%3],budget=.25,block='control',repetition=rep))
        for i in range(4):
            for j,s in enumerate([501,502]):
                for k in range(4):out.append(dict(instance='smoke'+str(i),arm=ARMS[(i+j+k)%4],seed=s,mode='search',budget=.25,block='smoke'))
    return out


def provenance(meta):
    here=Path(__file__).resolve().parent
    rel=str(here.relative_to(ROOT))
    frozen=json.loads(subprocess.check_output(['git','show','253d94a:'+rel+'/manifest.json'],cwd=ROOT))
    assert meta['manifest']==frozen==read(here/'manifest.json'),'unregistered manifest'
    assert not meta['build']['development'],'development build'
    registered_protocol=subprocess.check_output(['git','show','253d94a:'+rel+'/protocol.md'],cwd=ROOT)
    assert registered_protocol==(here/'protocol.md').read_bytes(),'registration drift'
    assert sha(here/'manifest.json') in registered_protocol.decode(),'registration manifest hash'
    production=subprocess.check_output(['git','ls-files','src','Cargo.toml','Cargo.lock','.cargo','research/Cargo.toml'],cwd=ROOT,text=True).splitlines()
    required=set(production)|{str(p.relative_to(ROOT)) for p in here.iterdir() if p.is_file() and p.suffix in ['.py','.cpp','.json','.md']}|{'research/examples/mqlib_difficulty.rs','benchmarks/adapters/run_mqlib.py'}
    assert set(meta['build']['source_hashes'])==required,'incomplete source coverage'
    for p in production:
        raw=subprocess.check_output(['git','show',frozen['source_base_commit']+':'+p],cwd=ROOT)
        assert raw==(ROOT/p).read_bytes(),'production source drift'
    up=ROOT/'.cache/mqlib-qualification/upstream'
    assert subprocess.check_output(['git','-C',str(up),'rev-parse','HEAD'],text=True).strip()==frozen['upstream_commit']
    assert not subprocess.check_output(['git','-C',str(up),'status','--porcelain'],text=True).strip()


def message_schema(event,cell):
    kind=event.get('kind')
    if kind=='ready':required={'kind','affinity'}
    elif kind=='config':required={'kind','backend','operators','temperatures','replicas','seed'}
    else:
        assert kind=='inc'
        required={'kind','state','objective' if cell['arm'].startswith('mqlib_') else 'energy'}
        if cell['mode']!='search':required.add('sleep_ns')
        elif not cell['arm'].startswith('mqlib_'):required.add('chunk')
    assert set(event)==required,'message schema'


def verify_reported_energy(v,event,mqlib,offset):
    value=event['objective'] if mqlib else event['energy']
    assert type(value) in [int,float] and abs(value)<=1e15 and math.isfinite(value)
    claimed=offset-value if mqlib else value
    assert abs(v-claimed)<=1e-9,'absolute energy tolerance'


def audit(folder):
    meta=read(folder/'metadata.json');m=meta['manifest'];terminal=read(folder/'terminal.json');result=read(folder/'analysis.json')
    provenance(meta)
    assert meta['parent_affinity']==[0] and not meta['git_dirty']
    for p,h in meta['build']['source_hashes'].items():
        raw=subprocess.check_output(['git','show',meta['build']['git_commit']+':'+p],cwd=ROOT)
        assert hashlib.sha256(raw).hexdigest()==h,p
        assert sha(ROOT/p)==h,p
    for p,h in meta['build']['binaries'].items():assert sha(ROOT/p)==h,p
    for p,h in meta['build']['upstream_build']['hashes'].items():assert sha(ROOT/p)==h,p
    assert meta['build']['upstream_build']['upstream_commit']==m['upstream_commit']
    models={}
    for item in m['instances']:
        p=Path(__file__).parent/'instances'/item['file'];assert sha(p)==item['sha256']
        models[item['id']]=read(p)
    for i,item in enumerate(m['preflight']['smoke_fixtures']):
        p=ROOT/item['file'];assert sha(p)==item['sha256'];models['smoke'+str(i)]=read(p)
    paths=sorted(folder.glob('cell*/record.json'));rs=[];witnesses=0;late=0
    schedule=expected(m,meta['phase']);assert len(paths)==terminal['cells']
    for idx,p in enumerate(paths):
        assert p.parent.name==f'cell{idx:04}'
        r=read(p);assert r['cell']==schedule[idx];c=r['cell'];model=models[c['instance']]
        assert r['fallback_energy']==model['offset'] and r['parent_affinity']==[0]
        assert r['status']==('INVALID' if r['errors'] else 'VALID')
        stream=(p.parent/'stdout.bin').read_bytes()
        recovered=b''.join(base64.b64decode(e['raw_base64'])+b'\n' for e in r['events'])+base64.b64decode(r.get('partial_base64',''))
        assert stream==recovered,'stream/line mismatch'
        assert len(stream)==r['stdout_bytes']
        previous=0;ready=0;config=0;last_chunk=-1;last_energy=float('inf')
        for e in r['events']:
            assert e['received_seconds']>=0 and e['validated_seconds']>=e['received_seconds']
            assert e['validated_seconds']>=previous;previous=e['validated_seconds']
            if 'error' in e:
                assert r['status']=='INVALID';continue
            event=json.loads(base64.b64decode(e['raw_base64']),object_pairs_hook=unique);assert event==e['message']
            message_schema(event,c)
            if event['kind']=='ready':ready+=1;assert event['affinity']=='0' and ready==1
            elif event['kind']=='config':
                config+=1;assert ready==1 and c['arm']=='v2_default' and c['mode']=='search'
                assert event['seed']==(c['seed']*0x9e3779b97f4a7c15)%(2**64)
                assert event['replicas']==32 and event['backend'] in ['DenseByte','SparseBitSlice']
                assert 1<=len(event['operators'])<=2 and len(event['temperatures'])==32
                for i,t in enumerate(event['temperatures']):close(t,4*(.08/4)**(i/31))
            else:
                assert ready==1 and event['kind']=='inc'
                if c['arm']=='v2_default' and c['mode']=='search':assert config==1
                v=energy(model,event['state']);assert abs(v-e['verified_energy'])<=1e-9
                verify_reported_energy(v,event,c['arm'].startswith('mqlib_'),model['offset'])
                if c['mode']=='search':
                    assert v<last_energy;last_energy=v
                    if not c['arm'].startswith('mqlib_'):
                        assert type(event['chunk']) is int and event['chunk']>=0 and event['chunk']>last_chunk
                        last_chunk=event['chunk']
                if c['mode']!='search':assert not any(event['state']) and type(event['sleep_ns']) is int and event['sleep_ns']>=0
                witnesses+=1;late+=max(e['received_seconds'],e['validated_seconds'])>=c['budget']
        if r['status']=='VALID':
            assert ready==1 and r['ready']==ready and r['configs']==config
            assert r['cutoff_seconds']>=c['budget'] and r['cutoff_seconds']<=r['kill_seconds']<=r['reap_seconds']<=r['total_seconds']
            assert r['exit_code']==-9 and r['reap_seconds']-r['kill_seconds']<=5.1
            for obs in r['observations']:
                maxthreads=2 if c['arm'] in ['ultimate','v2_default'] and c['mode']=='search' else 1
                assert len(obs['tasks'])<=maxthreads
                # Earliest observation can precede taskset; check samples after ready validation.
                ready_times=[e['validated_seconds'] for e in r['events'] if e.get('message',{}).get('kind')=='ready']
                if ready_times and obs['seconds']>=ready_times[0]:assert all(t['affinity']=='0' for t in obs['tasks'])
        rs.append(r)
    complete=len(rs)==len(schedule) and all(r['status']=='VALID' for r in rs) and terminal['status']=='COMPLETE'
    if not complete:
        assert result['status'] in ['FAIL','NO_VERDICT']
    elif meta['phase']=='preflight':
        groups=[]
        for name in m['preflight']['instances']:
            for arm in ARMS:
                group=[r for r in rs if r['cell']['instance']==name and r['cell']['arm']==arm]
                by={mode:[r for r in group if r['cell']['mode']==mode] for mode in ['null_a','null_b','delay']}
                inc=lambda r:[e for e in r['events'] if 'verified_energy' in e]
                valid=all(len(inc(r))==1 and best(r,.25)[1]==1 and inc(r)[0]['verified_energy']==r['fallback_energy'] for r in group)
                if not all(len(inc(r))==1 for r in group):groups.append(False);continue
                times={mode:[inc(r)[0]['validated_seconds'] for r in x] for mode,x in by.items()}
                null=abs(st.median(times['null_a'])-st.median(times['null_b']))
                diff=st.median([x-y for x,y in zip(times['delay'],times['null_a'])])
                sleep=st.median(inc(r)[0]['message']['sleep_ns']/1e9 for r in by['delay'])
                g=result['groups'][len(groups)];close(null,g['null_median_difference']);close(diff,g['paired_delay_median']);close(sleep,g['sleep_median'])
                groups.append(valid and null<=.010 and .025<=diff<=.075 and .045<=sleep<=.075)
                assert groups[-1]==g['pass_gate']
        over=sorted(max(0,r['cutoff_seconds']-.25) for r in rs if r['cell']['block']=='control')
        kill=over[455]<=.025 and over[-1]<=.1
        smoke=all(best(r,.25)[1]>0 and (r['cell']['arm']!='v2_default' or r['configs']==1) for r in rs if r['cell']['block']=='smoke')
        assert kill==result['cutoff_gate'] and smoke==result['smoke_gate']
        close(over[455],result['p95_overshoot']);close(over[-1],result['max_overshoot'])
        assert result['status']==('PASS' if all(groups) and kill and smoke else 'FAIL')
    else:
        informative=0;wins={a:0 for a in ARMS};contrasts={a+'_vs_'+b:[] for a,b in itertools.combinations(ARMS,2)}
        for i,item in enumerate(m['instances']):
            out=result['instances'][i];assert out['instance']==item['id'];L=item['normalization_L']
            assert L==sum(abs(h) for h in models[item['id']]['linear'])+sum(abs(w) for _,_,w in models[item['id']]['pairs'])
            for j,t in enumerate([.25,.5,1.,2.]):
                values={a:[best(r,t)[0] for r in rs if r['cell']['instance']==item['id'] and r['cell']['arm']==a] for a in ARMS}
                curve=out['curves'][j];assert curve['seconds']==t
                for a,x in values.items():
                    actual=curve['arms'][a];assert actual['values']==x and actual['n']==10
                    for key,v in dict(mean=st.mean(x),median=st.median(x),std=st.pstdev(x),min=min(x),max=max(x)).items():close(v,actual[key])
                    assert curve['fallback_only'][a]==sum(best(r,t)[1]==0 for r in rs if r['cell']['instance']==item['id'] and r['cell']['arm']==a)
                for a,b in itertools.combinations(ARMS,2):
                    d=[x-y for x,y in zip(values[a],values[b])];assert curve['paired'][a+'_vs_'+b]==dict(wins=sum(x<0 for x in d),ties=sum(x==0 for x in d),losses=sum(x>0 for x in d))
            means={a:st.mean(x) for a,x in values.items()};ranked=sorted(means,key=means.get)
            spread=(max(means.values())-min(means.values()))/L;top=(means[ranked[1]]-means[ranked[0]])/L
            close(spread,out['spread']);close(top,out['top_two_spread']);informative+=spread>=.001
            if top>=.001:wins[ranked[0]]+=1
            for a,b in itertools.combinations(ARMS,2):contrasts[a+'_vs_'+b].append((means[b]-means[a])/L)
        assert result['informative_instances']==informative and result['unique_wins']==wins
        assert result['H1']==('SUPPORTED_ON_QUALIFICATION' if informative>=8 else 'INCONCLUSIVE')
        assert result['H2']==('SUPPORTED_ON_QUALIFICATION' if sum(v>=4 for v in wins.values())>=2 else 'NOT_ESTABLISHED')
        # Independent construction of exactly 240000 bootstrap draws.
        indices=[];counter=0;limit=2**64-(2**64%24)
        while len(indices)<240000:
            digest=hashlib.sha256(f'MQ-DIFFICULTY-001/bootstrap/v1/{counter}'.encode()).digest();counter+=1
            for off in [0,8,16,24]:
                k=int.from_bytes(digest[off:off+8],'big')
                if k<limit:indices.append(k%24)
        for name,x in contrasts.items():
            means=sorted(st.mean(x[indices[k+j]] for j in range(24)) for k in range(0,240000,24))
            target=result['descriptive_instance_bootstrap'][name];close(st.mean(x),target['mean'])
            for j,q in enumerate([.025,.975]):
                pos=9999*q;lo=math.floor(pos);v=means[lo]+(means[lo+1]-means[lo])*(pos-lo);close(v,target['descriptive_95'][j])
    if terminal['status']=='COMPLETE':assert meta['prior_elapsed_seconds']+terminal['elapsed_seconds']<=3000
    return {'status':'PASS','scope':'artifact integrity and registered analysis; not independent optimizer replication',
            'cells':len(rs),'witnesses':witnesses,'late_witnesses':late,'registered_status':result['status'],
            'artifact_hashes':{str(p.relative_to(folder)):sha(p) for p in sorted(folder.rglob('*')) if p.is_file() and p.name not in ['audit.json','review.json']}}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);args=p.parse_args()
    r=audit(args.directory)
    with (args.directory/'audit.json').open('x') as f:json.dump(r,f,indent=2);f.write('\n')
    print(json.dumps({k:v for k,v in r.items() if k!='artifact_hashes'}))
