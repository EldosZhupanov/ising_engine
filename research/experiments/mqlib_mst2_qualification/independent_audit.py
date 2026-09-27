"""Offline qualification audit; no adapter/build/validator imports or solver runs."""
import argparse
from fractions import Fraction as F
import hashlib
import json
import math
from pathlib import Path
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]


def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def require(ok,message):
    if not ok: raise ValueError(message)


def energy(m,x):
    require(isinstance(x,list) and len(x)==len(m['linear']) and all(type(v) is int and v in (0,1) for v in x),'binary state')
    return F(m['offset'])+sum(F(h)*v for h,v in zip(m['linear'],x))+sum(F(w)*x[i]*x[j] for i,j,w in m['pairs'])


def unique(pairs):
    out={}
    for k,v in pairs:
        require(k not in out,'duplicate JSON field');out[k]=v
    return out


def audit(folder):
    meta=json.loads((folder/'metadata.json').read_text());summary=json.loads((folder/'summary.json').read_text())
    manifest=json.loads((HERE/'manifest.json').read_text());fixtures=json.loads((HERE/'fixtures.json').read_text())
    require(meta['manifest']==manifest and sha(HERE/'fixtures.json')==manifest['fixture_sha256'],'manifest/fixture provenance')
    require(meta['affinity']==[0] and meta['thread_environment']==dict.fromkeys(['RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS'],'1'),'metadata placement')
    for name,h in summary['raw_hashes'].items(): require(sha(folder/name)==h,'raw hash '+name)
    br=meta['build'];require(not br['development'] and br['exit_code']==0,'unqualified build')
    for name,h in br['source_hashes'].items():
        require(sha(ROOT/name)==h,'source hash '+name)
        blob=subprocess.check_output(['git','show',meta['git_commit']+':'+name],cwd=ROOT)
        require(hashlib.sha256(blob).hexdigest()==h,'committed source hash '+name)
    require(br['upstream_build']['upstream_commit']==manifest['upstream_commit'],'upstream pin')
    for name,h in br['upstream_build']['hashes'].items(): require(sha(ROOT/name)==h,'upstream artifact '+name)
    require(sha(ROOT/br['binary'])==br['binary_sha256'] and sha(ROOT/br['oracle'])==br['oracle_sha256'],'executables')
    require(sha(Path(meta['python_executable']).resolve())==meta['python_sha256'],'Python executable')
    require(subprocess.run(['git','merge-base','--is-ancestor',meta['protocol_commit'],meta['git_commit']],cwd=ROOT).returncode==0,'protocol chronology')
    callbacks=0;finals=0;kills=0;identities=0;hits=0;rows=[];previous_start=-1;all_hash_files={'metadata.json'}
    mode_counts={mode:0 for mode in manifest['modes']}
    for c,m in zip(manifest['cases'],fixtures):
        base=folder/c['id'];n=len(m['linear'])
        require(hashlib.sha256(json.dumps(m,sort_keys=True,separators=(',',':')).encode()).hexdigest()==c['model_sha256'],'model hash')
        require(json.loads((base/'model.json').read_text())==m and n==c['n'],'model bytes')
        oracle=json.loads((base/'oracle.json').read_text());inp=base/'oracle_input.qubo'
        require(oracle['command']==[str(ROOT/br['oracle']),str(inp.resolve())] and oracle['returncode']==0,'oracle command/exit')
        entries=inp.read_text().splitlines();header=entries.pop(0).split()
        expected=[(i+1,i+1,-F(h)) for i,h in enumerate(m['linear'])]+[(i+1,j+1,-F(w)/2) for i,j,w in m['pairs']]
        require(list(map(int,header))==[n,len(expected)],'matrix header')
        parsed=[(int(i),int(j),F(v)) for i,j,v in (line.split() for line in entries)]
        require(parsed==expected,'exported coefficients/sign/pair factor')
        spectrum=bytes.fromhex(oracle['stdout_hex']).decode().splitlines()
        require(len(spectrum)==1<<n,'oracle state count')
        exact=[]
        for mask,line in enumerate(spectrum):
            index,value=line.split();e=energy(m,[(mask>>j)&1 for j in range(n)])
            require(int(index)==mask and F(value)==F(m['offset'])-e,'oracle full recomputation identity')
            exact.append(e);identities+=1
        for filename in ['model.json','oracle_input.qubo','oracle.json']: all_hash_files.add(f"{c['id']}/{filename}")
        for seed in manifest['seeds']:
            for mode in manifest['modes']:
                cell=base/f'{seed}_{mode}';r=json.loads((cell/'record.json').read_text());rows.append(r);mode_counts[mode]+=1
                require((r['case'],r['seed'],r['mode'])==(c['id'],seed,mode),'cell identity/order')
                require(r['status']=='VALID' and not r['errors'],'invalid cell')
                require(r['parent_affinity']==r['child_affinity']==[0],'process placement')
                require(r['start_ns']>previous_start,'run start order');previous_start=r['start_ns']
                limit=manifest['forced_watchdog_seconds'] if mode=='watchdog' else manifest['cooperative_watchdog_seconds']
                require(r['watchdog_seconds']==limit,'watchdog contract')
                deadline=r['start_ns']+round(limit*1e9)
                require(r['reaped_ns']>=r['start_ns'] and r['elapsed_seconds']*1e9>=r['reaped_ns']-r['start_ns'],'parent timing order')
                path=cell/'input.qubo'
                require(path.read_bytes()==inp.read_bytes() and sha(path)==r['input_sha256'],'cell input identity')
                require(r['command']==['taskset','-c','0',str(ROOT/br['binary']),str(path.resolve()),str(seed),mode],'cell command')
                for filename in ['input.qubo','record.json']: all_hash_files.add(f"{c['id']}/{seed}_{mode}/{filename}")
                raw=bytes.fromhex(r['stdout_hex']);require(len(raw)<=2*1024*1024,'output size')
                parts=raw.split(b'\n');partial=parts.pop()
                events=[json.loads(line,object_pairs_hook=unique) for line in parts]
                cb=[];terminal=None;stopped=False;last_time=-1;last_obj=-math.inf;energies=[]
                for ev in events:
                    require(isinstance(ev,dict),'object schema')
                    kind=ev.get('kind')
                    keys={'kind','state','objective'}|({'index','continue','elapsed_ns','upstream_runtime','affinity'} if kind=='callback' else {'callbacks','stop_seen'})
                    require(kind in ('callback','final') and set(ev)==keys and terminal is None,'event schema/order')
                    e=energy(m,ev['state']);obj=ev['objective']
                    require(type(obj) in (int,float) and math.isfinite(obj) and abs(F(obj)-(F(m['offset'])-e))<=F(1,10**9),'witness energy')
                    require(obj>=last_obj,'degraded incumbent');last_obj=obj
                    checked={**ev,'energy_exact':str(e)}
                    if kind=='callback':
                        require(type(ev['index']) is int and ev['index']==len(cb)+1,'callback counter')
                        t=ev['elapsed_ns'];require(type(t) is int and t>=0 and t>=last_time and t<=r['reaped_ns']-r['start_ns'],'callback monotonic phase')
                        last_time=t
                        require(ev['affinity']=='0' and type(ev['upstream_runtime']) in (int,float) and math.isfinite(ev['upstream_runtime']) and ev['upstream_runtime']>=0,'callback diagnostics')
                        if mode=='first' or (mode=='third' and ev['index']>=3) or (mode=='deadline' and t>=20000000): stopped=True
                        require(type(ev['continue']) is bool and ev['continue']==(not stopped),'latched callback decision')
                        cb.append(checked);energies.append(e);callbacks+=1
                    else:
                        require(cb and type(ev['callbacks']) is int and ev['callbacks']==len(cb) and type(ev['stop_seen']) is bool and ev['stop_seen']==stopped,'final counter/latch')
                        require(ev['state']==cb[-1]['state'] and ev['objective']==cb[-1]['objective'],'final matches last callback')
                        terminal=checked;finals+=1
                require(cb,'no callback witness')
                if mode=='watchdog':
                    require(r['timed_out'] and r['returncode']==-9 and terminal is None and r['kill_sent_ns']>=deadline,'watchdog kill semantics');kills+=1
                else:
                    require(not partial and not r['timed_out'] and r['returncode']==0 and terminal is not None and stopped,'cooperative stop semantics')
                    require(r['kill_sent_ns'] is None and r['reaped_ns']<deadline,'cooperative watchdog bound')
                    if mode=='third': require(len(cb)>=3,'third callback absent')
                require(r['verified']=={'callbacks':cb,'final':terminal,'partial_hex':partial.hex(),'best_energy_exact':str(energies[-1])},'saved validator differs')
                hits+=energies[-1]==min(exact)
    require(set(summary['raw_hashes'])==all_hash_files,'raw inventory not exhaustive')
    require(len(rows)==manifest['expected_cells']==144 and identities==manifest['expected_state_identities']==544,'sample size')
    for k,v in [('cells',len(rows)),('valid_cells',len(rows)),('oracle_states',identities),('callbacks',callbacks),('cooperative_finals',finals),('expected_forced_kills',kills),('tiny_optimum_hits',hits)]: require(summary[k]==v,'summary counter '+k)
    require(summary['status']=='PASS' and summary['failure'] is None and summary['elapsed_seconds']<manifest['hard_campaign_cap_seconds'],'overall gate')
    return {'audit':'PASS','registered_status':'PASS','cells':len(rows),'oracle_states':identities,
            'callbacks':callbacks,'cooperative_finals':finals,'expected_forced_kills':kills,
            'tiny_optimum_hits_descriptive_only':hits,'mode_counts':mode_counts,
            'raw_hashes_checked':len(summary['raw_hashes']),'source_hashes_checked':len(br['source_hashes']),
            'auditor_sha256':sha(__file__),'scope':'Raw evidence replay; no optimizer execution or analyzer imports'}


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('folder',type=Path);a=p.parse_args()
    print(json.dumps(audit(a.folder),indent=2))
