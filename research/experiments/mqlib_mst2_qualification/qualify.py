"""MQ-MST2-001: exact-state and callback/process qualification; no comparison."""
import argparse
from fractions import Fraction
import importlib.util
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
import build

HERE=build.HERE
ROOT=build.ROOT
spec=importlib.util.spec_from_file_location('qualified_exporter',ROOT/'benchmarks/adapters/run_mqlib.py')
mq=importlib.util.module_from_spec(spec);spec.loader.exec_module(mq)
MODES=['first','third','deadline','watchdog']


def write(path,obj): Path(path).write_text(json.dumps(obj,indent=2,allow_nan=False)+'\n')


def unique(pairs):
    d={}
    for k,v in pairs:
        if k in d: raise ValueError('duplicate JSON key')
        d[k]=v
    return d


def number(x):
    if type(x) not in (int,float): return False
    try: return math.isfinite(x)
    except OverflowError: return False


def result_status(failure_kind, complete, cap_exceeded):
    if failure_kind == 'failure': return 'FAIL'
    if failure_kind == 'cap' or cap_exceeded or not complete: return 'INCOMPLETE'
    return 'PASS'


def validate(model,mode,raw,returncode,timed_out):
    if mode not in MODES: raise ValueError('unknown mode')
    if len(raw)>2*1024*1024: raise ValueError('oversize stdout')
    lines=raw.split(b'\n');partial=lines.pop()
    if partial and mode!='watchdog': raise ValueError('incomplete cooperative output')
    callbacks=[];final=None;stopped=False;last_weight=-math.inf;last_ns=-1
    for line in lines:
        e=json.loads(line,object_pairs_hook=unique)
        if not isinstance(e,dict): raise ValueError('record must be object')
        kind=e.get('kind')
        common={'kind','state','objective'}
        keys=common|({'index','continue','elapsed_ns','upstream_runtime','affinity'} if kind=='callback' else {'callbacks','stop_seen'})
        if kind not in ('callback','final') or set(e)!=keys or final is not None: raise ValueError('wrong record schema/order')
        if not isinstance(e['state'],list): raise ValueError('state must be list')
        energy=mq.exact_energy(model,e['state'])
        if not number(e['objective']) or abs(Fraction(e['objective'])-(Fraction(model['offset'])-energy))>Fraction(1,10**9):
            raise ValueError('objective mismatch')
        if e['objective']<last_weight: raise ValueError('incumbent degraded')
        last_weight=e['objective']
        checked={**e,'energy_exact':str(energy)}
        if kind=='callback':
            if type(e['index']) is not int or e['index']!=len(callbacks)+1: raise ValueError('callback sequence')
            if type(e['continue']) is not bool: raise ValueError('nonboolean continuation')
            if type(e['elapsed_ns']) is not int or e['elapsed_ns']<0 or e['elapsed_ns']<last_ns: raise ValueError('monotonic timestamp')
            if not number(e['upstream_runtime']) or e['upstream_runtime']<0 or e['affinity']!='0': raise ValueError('runtime/affinity')
            last_ns=e['elapsed_ns']
            if mode=='first' or (mode=='third' and e['index']>=3) or (mode=='deadline' and last_ns>=20000000): stopped=True
            expected=not stopped
            if e['continue']!=expected: raise ValueError('callback mode/latch mismatch')
            callbacks.append(checked)
        else:
            if not callbacks or type(e['callbacks']) is not int or e['callbacks']!=len(callbacks) or type(e['stop_seen']) is not bool or e['stop_seen']!=stopped:
                raise ValueError('final callback/stop mismatch')
            if e['state']!=callbacks[-1]['state'] or e['objective']!=callbacks[-1]['objective']: raise ValueError('final incumbent differs')
            final=checked
    if not callbacks: raise ValueError('no complete callback witness')
    if mode=='watchdog':
        if returncode!=-9 or not timed_out or final is not None: raise ValueError('expected forced watchdog termination')
    else:
        if returncode!=0 or timed_out or final is None or not stopped: raise ValueError('cooperative stop failed')
        if mode=='third' and len(callbacks)<3: raise ValueError('third callback absent')
    return {'callbacks':callbacks,'final':final,'partial_hex':partial.hex(),
            'best_energy_exact':callbacks[-1]['energy_exact']}


def run_cell(model,case,seed,mode,folder,manifest,campaign_deadline):
    if type(seed) is not int or not 0<=seed<=65535 or mode not in MODES: raise ValueError('bad seed/mode')
    mq.validate_model(model)
    start=time.monotonic_ns()
    limit=manifest['forced_watchdog_seconds'] if mode=='watchdog' else manifest['cooperative_watchdog_seconds']
    deadline=min(start+round(limit*1e9),campaign_deadline)
    row={'case':case,'seed':seed,'mode':mode,'start_ns':start,'watchdog_seconds':limit,
         'parent_affinity':sorted(os.sched_getaffinity(0)),'status':'FAIL','errors':[]}
    proc=None;stdout=b'';stderr=b'';drained=False
    folder.mkdir(parents=True,exist_ok=False)
    try:
        path=folder/'input.qubo';path.write_text(mq.export_qubo(model));row['input_sha256']=build.sha(path)
        cmd=['taskset','-c','0',str(build.BINARY),str(path.resolve()),str(seed),mode];row['command']=cmd
        env={**os.environ,'RAYON_NUM_THREADS':'1','OMP_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1'}
        proc=subprocess.Popen(cmd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
        row['child_affinity']=sorted(os.sched_getaffinity(proc.pid))
        if row['parent_affinity']!=[0] or row['child_affinity']!=[0]: raise ValueError('observed affinity mismatch')
        row['timed_out']=False;row['kill_sent_ns']=None
        try:
            stdout,stderr=proc.communicate(timeout=max(0,(deadline-time.monotonic_ns())/1e9))
        except subprocess.TimeoutExpired:
            row['timed_out']=True;row['kill_sent_ns']=time.monotonic_ns();proc.kill()
            stdout,stderr=proc.communicate(timeout=3)
        drained=True
        row['reaped_ns']=time.monotonic_ns();row['returncode']=proc.returncode
        if row['kill_sent_ns'] is not None and row['kill_sent_ns']<start+round(limit*1e9): raise ValueError('campaign interrupted watchdog')
        if not row['timed_out'] and row['reaped_ns']>=start+round(limit*1e9): raise ValueError('cooperative exit not before watchdog')
        row['verified']=validate(model,mode,stdout,proc.returncode,row['timed_out'])
        row['status']='VALID'
    except (ValueError,OSError,subprocess.TimeoutExpired) as exc:
        row['errors'].append(str(exc))
    finally:
        if proc is not None and not drained:
            if proc.poll() is None: proc.kill()
            stdout,stderr=proc.communicate(timeout=3)
            row['returncode']=proc.returncode
    row['stdout_hex']=stdout.hex();row['stderr_hex']=stderr.hex()
    row['elapsed_seconds']=(time.monotonic_ns()-start)/1e9
    write(folder/'record.json',row)
    return row


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    manifest=json.loads((HERE/'manifest.json').read_text());provenance=build.verify()
    if platform.platform()!=manifest['environment']['platform'] or sys.version!=manifest['environment']['python']: raise ValueError('environment mismatch')
    cpu=next(l.split(':',1)[1].strip() for l in Path('/proc/cpuinfo').read_text().splitlines() if l.startswith('model name'))
    if cpu!=manifest['environment']['cpu_model']: raise ValueError('CPU mismatch')
    if build.sha(HERE/'fixtures.json')!=manifest['fixture_sha256']: raise ValueError('fixture hash')
    models=json.loads((HERE/'fixtures.json').read_text());os.sched_setaffinity(0,{0})
    a.output.mkdir(parents=True,exist_ok=False)
    meta={'experiment_id':manifest['id'],'git_commit':build.git('rev-parse','HEAD'),
          'protocol_commit':'f6fa9e9','git_status':build.git('status','--short'),'manifest':manifest,
          'build':provenance,'affinity':sorted(os.sched_getaffinity(0)),
          'platform':platform.platform(),'python':sys.version,'python_executable':sys.executable,
          'python_sha256':build.sha(Path(sys.executable).resolve()),
          'cpuinfo':Path('/proc/cpuinfo').read_text(),'meminfo':Path('/proc/meminfo').read_text(),
          'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
          'thread_environment':dict.fromkeys(['RAYON_NUM_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS'],'1'),
          'argv':sys.argv,'started_unix':time.time(),'load_start':os.getloadavg()}
    write(a.output/'metadata.json',meta)
    start=time.monotonic_ns();deadline=start+round(manifest['hard_campaign_cap_seconds']*1e9)
    rows=[];oracles=[];failure=None;failure_kind=None
    try:
        for i,model in enumerate(models):
            case=manifest['cases'][i];folder=a.output/case['id'];folder.mkdir()
            if build.hashlib.sha256(json.dumps(model,sort_keys=True,separators=(',',':')).encode()).hexdigest()!=case['model_sha256']: raise ValueError('case hash mismatch')
            write(folder/'model.json',model);path=folder/'oracle_input.qubo';path.write_text(mq.export_qubo(model))
            cmd=[str(build.ORACLE),str(path.resolve())]
            try:
                proc=subprocess.run(cmd,capture_output=True,timeout=min(5,max(.001,(deadline-time.monotonic_ns())/1e9)))
            except subprocess.TimeoutExpired as exc:
                write(folder/'oracle.json',{'case':case['id'],'command':cmd,'timed_out':True,'stdout_hex':(exc.stdout or b'').hex(),'stderr_hex':(exc.stderr or b'').hex()})
                raise
            oracle={'case':case['id'],'command':cmd,'returncode':proc.returncode,'stdout_hex':proc.stdout.hex(),'stderr_hex':proc.stderr.hex()}
            write(folder/'oracle.json',oracle)
            if proc.returncode: raise ValueError('oracle exit')
            lines=proc.stdout.decode().splitlines();n=len(model['linear'])
            if len(lines)!=1<<n: raise ValueError('oracle cardinality')
            energies=[]
            for mask,line in enumerate(lines):
                idx,obj=line.split();x=[(mask>>j)&1 for j in range(n)];energy=mq.exact_energy(model,x)
                if int(idx)!=mask or Fraction(obj)!=Fraction(model['offset'])-energy: raise ValueError('oracle energy identity')
                energies.append(energy)
            oracle['states_checked']=len(lines);oracle['minimum_exact']=str(min(energies));oracles.append(oracle)
            for seed in manifest['seeds']:
                for mode in MODES:
                    if time.monotonic_ns()>=deadline: raise TimeoutError('campaign cap')
                    row=run_cell(model,case['id'],seed,mode,folder/f'{seed}_{mode}',manifest,deadline)
                    rows.append(row)
                    if row['status']!='VALID': raise ValueError('cell failure '+str(row['errors']))
                    row['gap_exact']=str(Fraction(row['verified']['best_energy_exact'])-min(energies))
            print('completed '+case['id']+': '+str(len(rows))+'/144',flush=True)
    except TimeoutError as exc:
        failure=str(exc);failure_kind='cap'
    except (ValueError,OSError,subprocess.TimeoutExpired) as exc:
        failure=str(exc);failure_kind='failure'
    try:
        if build.verify()!=provenance: raise ValueError('build/source drift during run')
    except (ValueError,OSError,subprocess.CalledProcessError) as exc: failure=str(exc);failure_kind='failure'
    summary={'status':result_status(failure_kind,len(rows)==144 and sum(o['states_checked'] for o in oracles)==544,False),
             'failure':failure,'cells':len(rows),'valid_cells':sum(r['status']=='VALID' for r in rows),
             'oracle_states':sum(o['states_checked'] for o in oracles),
             'callbacks':sum(len(r.get('verified',{}).get('callbacks',[])) for r in rows),
             'cooperative_finals':sum(r.get('verified',{}).get('final') is not None for r in rows),
             'expected_forced_kills':sum(r['mode']=='watchdog' and r['status']=='VALID' for r in rows),
             'tiny_optimum_hits':sum(r.get('gap_exact')=='0' for r in rows),
             'elapsed_seconds':(time.monotonic_ns()-start)/1e9,'load_end':os.getloadavg()}
    summary['status']=result_status(failure_kind,len(rows)==144 and summary['oracle_states']==544,summary['elapsed_seconds']>=manifest['hard_campaign_cap_seconds'])
    summary['raw_hashes']={str(f.relative_to(a.output)):build.sha(f) for f in sorted(a.output.rglob('*')) if f.is_file()}
    write(a.output/'summary.json',summary)
    write(a.output/'reproduction.json',{'argv':sys.argv,'git_commit':meta['git_commit'],'fresh_output_required':True})
    (a.output/'README.md').write_text('# MQ-MST2-001 retained run\n\nInstrument `'+meta['git_commit']+'`.\nSee ../protocol.md and reproduction.json.\n\n'
        'Original invocation is historical; never overwrite evidence or rescue a failed run.\n'
        'Raw artifact audit is not an independent solver rerun. No comparative claim.\n')
    print(json.dumps({k:v for k,v in summary.items() if k!='raw_hashes'},indent=2))
    return 0 if summary['status']=='PASS' else 1


if __name__=='__main__': raise SystemExit(main())
