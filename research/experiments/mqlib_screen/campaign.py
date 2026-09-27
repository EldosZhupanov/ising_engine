#!/usr/bin/env python3
"""Frozen MQ-SCREEN-001 delivered-witness comparison, with exclusive outputs."""
import argparse
import hashlib
import importlib.util
import itertools
import json
import math
import os
from pathlib import Path
import platform
import random
import selectors
import statistics as st
import subprocess
import sys
import tempfile
import time

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('mqlib_adapter',ROOT/'benchmarks/adapters/run_mqlib.py')
mq=importlib.util.module_from_spec(spec);spec.loader.exec_module(mq)
ARMS=['ultimate','v2_default','mqlib']
SOURCES=['research/examples/mqlib_compare.rs','research/experiments/mqlib_screen/stream.cpp',
         'research/experiments/mqlib_screen/build.py',
         'research/experiments/mqlib_screen/campaign.py','research/experiments/mqlib_screen/test_campaign.py',
         'research/experiments/mqlib_screen/protocol.md','research/experiments/mqlib_screen/manifest.json',
         'benchmarks/adapters/run_mqlib.py']


def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def git(*args): return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
def write(p,data):
    with Path(p).open('x') as f: json.dump(data,f,indent=2,allow_nan=False);f.write('\n')


def energy(model,x):
    if len(x)!=len(model['linear']) or any(type(v) is not int or v not in (0,1) for v in x):
        raise ValueError('invalid state')
    return model['offset']+sum(h*v for h,v in zip(model['linear'],x))+sum(w*x[i]*x[j] for i,j,w in model['pairs'])


def verify_event(model,event,arm):
    if not isinstance(event,dict): raise ValueError('event not object')
    if event.get('kind')=='config' and arm=='v2_default':
        ops=event.get('operators');temps=event.get('temperatures')
        if (event.get('replicas')!=32 or event.get('backend') not in ('SparseBitSlice','DenseByte')
            or not isinstance(ops,list) or not 1<=len(ops)<=2
            or any(not isinstance(op,str) or not op for op in ops)
            or not isinstance(temps,list) or len(temps)!=32
            or any(type(v) not in (int,float) or not math.isfinite(v) or abs(v-4.0*(.08/4)**(i/31))>1e-12 for i,v in enumerate(temps))):
            raise ValueError('invalid v2 config')
        return None
    if event.get('kind')!='inc': raise ValueError('unexpected event kind')
    value=energy(model,event['state'])
    report=event['objective'] if arm=='mqlib' else event['energy']
    if type(report) not in (int,float) or not math.isfinite(report): raise ValueError('invalid objective')
    actual=model['offset']-report if arm=='mqlib' else report
    if abs(actual-value)>1e-9: raise ValueError('independent energy mismatch')
    return value


def consume(model,arm,line,received,budget,record):
    item={'received_seconds':received,'raw':line,'eligible':received<budget}
    record['events'].append(item)
    try:
        event=json.loads(line)
        value=verify_event(model,event,arm)
        if value is None:
            if event.get('seed') != (record['seed']*0x9e3779b97f4a7c15)%(2**64):
                raise ValueError('wrong v2 config seed')
            record['configs']=record.get('configs',0)+1
            if record['configs']!=1: raise ValueError('duplicate v2 config')
        if value is not None:
            item['verified_energy']=value
            if received<budget:
                record['witnesses_before_deadline']+=1
                if value<record['best_energy']:
                    record.update(best_energy=value,best_state=event['state'],time_to_best_seconds=received)
    except (ValueError,KeyError,TypeError,OverflowError) as exc:
        # Late complete witnesses are still validated but never score.
        item['error']=str(exc);record['errors'].append(str(exc))


def run_cell(model,arm,seed,budget,cpu,binary_override=None):
    record={'arm':arm,'seed':seed,'budget_seconds':budget,'cpu':cpu,
            'best_state':[0]*len(model['linear']),'best_energy':model['offset'],
            'time_to_best_seconds':0.0,'witnesses_before_deadline':0,'events':[],'errors':[]}
    start=time.perf_counter()
    env={**os.environ,'RAYON_NUM_THREADS':'1','OMP_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1'}
    proc=None
    with tempfile.TemporaryDirectory(prefix='mq-screen-') as tmp:
        path=Path(tmp)/'input'
        if arm=='mqlib':
            path.write_text(mq.export_qubo(model))
            cmd=[str(ROOT/'.cache/mqlib-qualification/screen_stream'),str(path),str(seed)]
        else:
            path.write_text(json.dumps(model))
            cmd=[str(ROOT/'target/release/examples/mqlib_compare'),arm,str(path),str(seed)]
        if binary_override is not None: cmd=binary_override
        cmd=['taskset','-c',str(cpu),*cmd]
        record['command']=cmd
        record['setup_seconds']=time.perf_counter()-start
        pending=b''
        try:
            with (Path(tmp)/'stderr').open('wb') as err:
                proc=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=err,env=env)
                os.set_blocking(proc.stdout.fileno(),False)
                with selectors.DefaultSelector() as sel:
                    sel.register(proc.stdout,selectors.EVENT_READ)
                    eof=False
                    while True:
                        remaining=budget-(time.perf_counter()-start)
                        if remaining<=0: break
                        ready=sel.select(remaining)
                        if not ready: continue
                        chunk=os.read(proc.stdout.fileno(),65536)
                        received=time.perf_counter()-start
                        if not chunk:
                            eof=True
                            break
                        pending+=chunk
                        if len(pending)>1000000: raise ValueError('oversized incomplete output')
                        while b'\n' in pending:
                            line,pending=pending.split(b'\n',1)
                            if line.strip(): consume(model,arm,line.decode(),received,budget,record)
                record['cutoff_seconds']=time.perf_counter()-start
                actual_deadline=record['cutoff_seconds']>=budget
                if eof and not actual_deadline:
                    # A tiny exit race is allowed for process reaping, not search.
                    try: proc.wait(timeout=.02)
                    except subprocess.TimeoutExpired:
                        record['errors'].append('stdout closed while process still running before deadline')
                    if pending: record['errors'].append('incomplete output before deadline')
                record['killed_at_deadline']=actual_deadline and proc.poll() is None
                if proc.poll() is None: proc.kill()
                proc.wait(timeout=5)
                # Drain all post-cutoff output; it is late even if buffered earlier.
                rest=proc.stdout.read() or b''
                received=max(budget,time.perf_counter()-start)
                pending+=rest
                for line in pending.split(b'\n')[:-1]:
                    if line.strip(): consume(model,arm,line.decode(),received,budget,record)
                record['partial_output']=pending.split(b'\n')[-1].decode(errors='replace')
                proc.stdout.close()
                record['exit_code']=proc.returncode
                if proc.returncode not in (0,-9) or (proc.returncode==-9 and not record['killed_at_deadline']):
                    record['errors'].append('unexpected process exit')
        except (OSError,ValueError,subprocess.TimeoutExpired) as exc:
            record['errors'].append(str(exc))
        finally:
            if proc is not None and proc.poll() is None: proc.kill();proc.wait()
        record['stderr']=(Path(tmp)/'stderr').read_text(errors='replace') if (Path(tmp)/'stderr').exists() else ''
        record['total_seconds']=time.perf_counter()-start
    record['status']='INVALID' if record['errors'] else 'VALID'
    record['fallback_only']=record['witnesses_before_deadline']==0
    return record


def describe(values):
    ordered=sorted(values)
    def quantile(q):
        p=q*(len(ordered)-1);lo=int(p);hi=min(lo+1,len(ordered)-1)
        return ordered[lo]*(1-(p-lo))+ordered[hi]*(p-lo)
    return {'n':len(values),'mean':st.mean(values),'median':st.median(values),'std':st.pstdev(values),
            'min':min(values),'max':max(values),'q25':quantile(.25),'q75':quantile(.75)}


def analyze(records,manifest):
    expected={(i['name'],s,a) for i in manifest['instances'] for s in manifest['search_seeds'] for a in ARMS}
    indexed={(r['instance'],r['seed'],r['arm']):r for r in records}
    if len(indexed)!=len(records) or set(indexed)!=expected or any(r['status']!='VALID' for r in records):
        return {'status':'NO_VERDICT','reason':'incomplete, duplicate or invalid cells'}
    instances=[];ds=[]
    for item in manifest['instances']:
        name=item['name'];seeds=manifest['search_seeds'];scale=item['normalizer']
        arm_values={a:[indexed[name,s,a]['best_energy'] for s in seeds] for a in ARMS}
        contrast=[(b-a)/scale for a,b in zip(arm_values['ultimate'],arm_values['mqlib'])]
        ds.append(st.mean(contrast))
        wlt={}
        for a,b in itertools.combinations(ARMS,2):
            delta=[x-y for x,y in zip(arm_values[a],arm_values[b])]
            wlt[a+'_vs_'+b]={'wins':sum(d<0 for d in delta),'ties':sum(d==0 for d in delta),'losses':sum(d>0 for d in delta),
                             'normalized_improvement':describe([-d/scale for d in delta])}
        instances.append({'name':name,'energies':{a:describe(v) for a,v in arm_values.items()},
                          'ultimate_vs_mqlib_normalized':describe(contrast),'pairwise':wlt})
    observed=abs(st.mean(ds))
    p=sum(abs(st.mean(d*s for d,s in zip(ds,signs)))>=observed-1e-15
          for signs in itertools.product((-1,1),repeat=len(ds)))/(2**len(ds))
    rng=random.Random(52001)
    boot=sorted(st.mean(rng.choices(ds,k=len(ds))) for _ in range(10000))
    ci=[boot[249],boot[9749]]
    means={a:st.mean(st.mean(indexed[i['name'],s,a]['best_energy']/i['normalizer'] for s in manifest['search_seeds'])
                     for i in manifest['instances']) for a in ARMS}
    oracle=st.mean(min(st.mean(indexed[i['name'],s,a]['best_energy']/i['normalizer'] for s in manifest['search_seeds']) for a in ARMS)
                   for i in manifest['instances'])
    return {'status':'COMPLETE','cells':len(records),'instances':instances,
            'primary':{'instance_contrasts':ds,'mean':st.mean(ds),'sign_flip_p':p,'bootstrap_95':ci,
                       'distribution':describe(ds),
                       'H1':'SUPPORTED_ON_THIS_SCREEN' if st.mean(ds)>=.001 and p<.05 and ci[0]>0 else 'INCONCLUSIVE'},
            'fallback_only':{a:sum(r['fallback_only'] for r in records if r['arm']==a) for a in ARMS},
            'optimistic_diagnostic':{'normalized_fixed_arm_means':means,'oracle_mean':oracle,
                                     'best_fixed_minus_oracle':min(means.values())-oracle,
                                     'warning':'post-hoc in-sample means; not deployable selection'},
            'limitations':['synthetic screen','configured restarted wrappers','not learned selection','not optimum/TTS evidence']}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--smoke',action='store_true');args=p.parse_args()
    manifest=json.loads((HERE/'manifest.json').read_text())
    head=git('rev-parse','HEAD')
    for path in SOURCES:
        if subprocess.check_output(['git','show',head+':'+path],cwd=ROOT)!=(ROOT/path).read_bytes():
            raise RuntimeError('uncommitted instrument '+path)
    binaries=[ROOT/'target/release/examples/mqlib_compare',ROOT/'.cache/mqlib-qualification/screen_stream']
    from build import verify_build
    build_record=verify_build()
    artifacts={str(p.relative_to(ROOT)):sha(p) for p in binaries}
    artifacts.update({path:sha(ROOT/path) for path in SOURCES})
    if args.smoke:
        corpus=[]
        for i in range(4):
            model=json.loads((ROOT/f'research/mqlib_qualification/run001/case{i:02}/model.json').read_text())
            corpus.append((f'smoke{i}',model))
        seeds=[501,502];budget=.25
    else:
        corpus=[]
        for i in manifest['instances']:
            path=HERE/'instances'/i['name']
            if sha(path)!=i['sha256']: raise RuntimeError('input checksum mismatch')
            corpus.append((i['name'],mq.load_model(path)))
        seeds=manifest['search_seeds'];budget=manifest['budget_seconds']
    args.output.mkdir(parents=True,exist_ok=False)
    metadata={'git_commit':head,'git_status':git('status','--short'),'hashes':artifacts,'build':build_record,
              'upstream_commit':git('-C',str(ROOT/'.cache/mqlib-qualification/upstream'),'rev-parse','HEAD'),
              'environment':{'platform':platform.platform(),'cpuinfo':Path('/proc/cpuinfo').read_text(),
                             'load_start':os.getloadavg(),'python':sys.version,'rustc':subprocess.check_output(['rustc','-V'],text=True),
                             'RUSTFLAGS':os.environ.get('RUSTFLAGS',''),'rayon_threads':1,'cpu':manifest['cpu']},
              'argv':sys.argv,'smoke':args.smoke,'started_unix':time.time()}
    write(args.output/'metadata.json',metadata)
    records=[]
    with (args.output/'raw.jsonl').open('x') as f:
        stop=False
        for i,(name,model) in enumerate(corpus):
            for j,seed in enumerate(seeds):
                order=ARMS[(i+j)%3:]+ARMS[:(i+j)%3]
                for arm in order:
                    record=run_cell(model,arm,seed,budget,manifest['cpu'])
                    record['instance']=name;records.append(record)
                    f.write(json.dumps(record,allow_nan=False)+'\n');f.flush()
                    if record['status']!='VALID': stop=True;break
                print(f'{name} seed={seed} completed_cells={len(records)}',flush=True)
                if stop: break
            if stop: break
    if args.smoke:
        summary={'status':'PASS' if len(records)==24 and all(r['status']=='VALID' and not r['fallback_only'] and (r['arm']!='v2_default' or r.get('configs')==1) for r in records) else 'FAIL',
                 'cells':len(records),'fallback_only':sum(r['fallback_only'] for r in records)}
    else: summary=analyze(records,manifest)
    if any(sha(ROOT/path)!=digest for path,digest in artifacts.items()):
        summary={'status':'NO_VERDICT','reason':'instrument/binary changed during execution'}
    try: verify_build()
    except RuntimeError as exc: summary={'status':'NO_VERDICT','reason':str(exc)}
    summary['load_end']=os.getloadavg();summary['raw_sha256']=sha(args.output/'raw.jsonl')
    write(args.output/'summary.json',summary)
    (args.output/'README.md').write_text('# MQ-SCREEN-001 run\n\nInstrument `'+head+'`.\n\n'
        'See the parent protocol and metadata.json for frozen commands, environment and hashes.\n'
        'Reproduce with campaign.py --output NEW_DIRECTORY'+(' --smoke' if args.smoke else '')+'.\n'
        'Raw received witness timestamps govern budget eligibility; every state requires independent rescore.\n')
    print(json.dumps({k:v for k,v in summary.items() if k!='instances'},indent=2))
    return 0 if summary['status'] in ('PASS','COMPLETE') else 1


if __name__=='__main__': raise SystemExit(main())
