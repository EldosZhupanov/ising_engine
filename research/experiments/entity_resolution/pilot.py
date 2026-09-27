"""ER-001 development pilot. Frozen cases, audited existing deadline supervisor."""
import argparse
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time
import re
from fractions import Fraction
import core

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
spec=importlib.util.spec_from_file_location('q002_deadline',HERE.parent/'hubo_corpus_qualification/phase_b.py')
deadline=importlib.util.module_from_spec(spec)
spec.loader.exec_module(deadline)
SEEDS=list(range(990001,990011))
SMOKE_SEED=991001
CASE_SHA='3214b795b55805fe5314830004e0b3a31bb50e21b671a1b5fe503d93e7bfe271'
BASELINES={'pairwise':core.threshold,'closure':core.closure,'greedy':core.greedy,'exact':core.exact}
REQUIRED=['core.py','prepare.py','pilot.py','test_core.py','test_pilot.py','protocol.md','cases.json']


def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def write_new(path,obj):
    with Path(path).open('x') as f:f.write(json.dumps(obj,indent=2,sort_keys=True)+'\n')
def emit(obj):print(json.dumps(obj,separators=(',',':')),flush=True)


def worker(engine,seed):
    print('READY',flush=True)
    case=json.loads(sys.stdin.readline())
    start=time.perf_counter()
    bits='0'*case['n']
    emit({'kind':'inc','bits':bits,'energy':deadline.base.score(case,bits),'steps':0,'origin':'fallback'})
    incidence=[0]*case['n']
    for w,vs in case['terms']:
        for v in vs:incidence[v]+=abs(w)
    hot=max(1,max(incidence))/math.log(2)
    emit({'kind':'meta','model_n':case['n'],'model_terms':len(case['terms']),
          'penalty':0,'hot':hot,'transform_s':time.perf_counter()-start})
    request=dict(case,original_n=case['n'],hot=hot,seed=seed)
    completed=subprocess.run([str(engine)],input=json.dumps(request)+'\n',text=True,check=False)
    raise SystemExit(completed.returncode or 3)


def checked_endpoint(case,row):
    n=len(case['record_ids']);model=core.model(n,case['weights'])
    energy,bits,received=deadline.validate(model,row)
    for item in row['events']:
        ev=item['event']
        if ev['kind']=='inc':
            measured=core.inspect(n,case['weights'],ev['bits'])
            if 8*measured['energy']!=ev['energy']:raise ValueError('application/solver energy mismatch')
    report=core.inspect(n,case['weights'],bits)
    if report['violations']:raise ValueError('infeasible endpoint despite feasible fallback')
    return {'bits':bits,**report,'received_s':received}


def source_files():
    names=subprocess.check_output(['git','ls-files','src','Cargo.toml','Cargo.lock','.cargo/config.toml',
           'research/Cargo.toml','research/examples/hubo_compare.rs',str(HERE.relative_to(ROOT)),
           'research/experiments/hubo_comparison/campaign.py',
           'research/experiments/hubo_corpus_qualification/phase_b.py'],cwd=ROOT,text=True).splitlines()
    names=[name for name in names if not name.startswith(str(HERE.relative_to(ROOT))+'/')]
    names += [str((HERE/name).relative_to(ROOT)) for name in REQUIRED]
    return {name:sha(ROOT/name) for name in sorted(set(names))}


def design(smoke):
    if type(smoke) is not bool:raise ValueError('invalid smoke flag')
    if smoke:return [{'id':'smoke','record_ids':['a','b','c'],'weights':[2,2,-3],'truth':['a','a','b']}],[SMOKE_SEED]
    if sha(HERE/'cases.json')!=CASE_SHA:raise ValueError('frozen input hash mismatch')
    cases=json.loads((HERE/'cases.json').read_text())['cases']
    if len(cases)!=12:raise ValueError('case count mismatch')
    return cases,SEEDS


def command_for(env,seed):
    return [env['python_executable'],env['pilot_path'],'worker','--engine',env['engine'],'--seed',str(seed)]


def verify_environment(env):
    for key in ['evaluated_commit','engine_sha256','python','python_executable','pilot_path',
                'repo_root','engine','platform','cpu','rustc']:
        if not isinstance(env.get(key),str) or not env[key]:raise ValueError('missing environment '+key)
    if not re.fullmatch('[0-9a-f]{64}',env['engine_sha256']):raise ValueError('invalid engine hash')
    if not re.fullmatch('[0-9a-f]{40}',env['evaluated_commit']):raise ValueError('invalid evaluated commit')
    if env.get('threads')!=deadline.THREAD_ENV:raise ValueError('thread configuration mismatch')
    if 'rustflags' not in env or env['rustflags'] is not None and not isinstance(env['rustflags'],str):
        raise ValueError('missing/invalid rustflags')
    if any(not Path(env[k]).is_absolute() for k in ['engine','python_executable','pilot_path','repo_root']):
        raise ValueError('nonabsolute runtime provenance')
    if Path(env['pilot_path'])!=Path(env['repo_root'])/Path(__file__).resolve().relative_to(ROOT):
        raise ValueError('worker path outside recorded source')


def native_hit(row,optimal_energy):
    return any(e['received_s']<=2 and e['event'].get('kind')=='inc'
               and e['event'].get('origin')!='fallback' and e['event']['energy']==8*optimal_energy
               for e in row['events'])


def f1_fraction(counts):
    tp,fp,fn=counts['tp'],counts['fp'],counts['fn']
    return Fraction(2*tp,2*tp+fp+fn) if 2*tp+fp+fn else Fraction(0)


def semantic_gate(totals,informative):
    delta=f1_fraction(totals['exact'])-max(f1_fraction(totals[a]) for a in ['pairwise','closure','greedy'])
    return informative and delta>=Fraction(1,50)


def verify_row_identity(row,env,seed,previous_finish):
    if row['command']!=command_for(env,seed):raise ValueError('row seed/command mismatch')
    if row['clocks']['boot_ns']<=previous_finish:raise ValueError('overlapping/reordered cells')
    return row['clocks']['finish_ns']


def raw_names(cases,seeds):
    return {'environment.json','cases.json','complete.json'} | {
        f"{c['id']}_{suffix}.json" for c in cases for suffix in ['baselines','model']+list(map(str,seeds))}


def seal_raw(directory,cases,seeds):
    write_new(directory/'raw_manifest.json',{name:sha(directory/name) for name in sorted(raw_names(cases,seeds))})


def verify_raw(directory,cases,seeds):
    names=raw_names(cases,seeds)
    inventory={p.name for p in directory.iterdir()}
    if inventory not in [names|{'raw_manifest.json'},names|{'raw_manifest.json','summary.json'}]:
        raise ValueError('missing/extra raw evidence')
    manifest=json.loads((directory/'raw_manifest.json').read_text())
    if set(manifest)!=names or any(sha(directory/name)!=h for name,h in manifest.items()):
        raise ValueError('raw evidence hash/inventory mismatch')


def run(args):
    args.output.mkdir(parents=True,exist_ok=False)
    try:
        run_inner(args)
    except BaseException as exc:
        write_new(args.output/'invalid.json',{'error':str(exc),'type':type(exc).__name__})
        raise


def run_inner(args):
    source=source_files()
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    for name,h in source.items():
        tracked=subprocess.check_output(['git','show',f'{commit}:{name}'],cwd=ROOT)
        if hashlib.sha256(tracked).hexdigest()!=h:raise ValueError('source differs from evaluated commit')
    cases,seeds=design(args.smoke)
    cpu=next((s.split(':',1)[1].strip() for s in Path('/proc/cpuinfo').read_text().splitlines() if s.startswith('model name')),'unknown')
    env={'evaluated_commit':commit,'sources':source,'python':sys.version,
              'repo_root':str(ROOT),
              'python_executable':sys.executable,'pilot_path':str(Path(__file__).resolve()),
              'platform':platform.platform(),'cpu':cpu,'engine':str(args.engine),'engine_sha256':sha(args.engine),
              'case_sha256':sha(HERE/'cases.json'),'seeds':seeds,'smoke':args.smoke,'threads':deadline.THREAD_ENV,
              'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
              'rustflags':os.environ.get('RUSTFLAGS'),'budget_s':2.0}
    write_new(args.output/'environment.json',env)
    write_new(args.output/'cases.json',cases)
    for case in cases:
        n=len(case['record_ids']);ws=case['weights'];baselines={}
        for name,method in BASELINES.items():
            start=time.perf_counter();bits=method(n,ws);elapsed=time.perf_counter()-start
            baselines[name]={'bits':bits,**core.inspect(n,ws,bits),'wall_s':elapsed}
        write_new(args.output/f"{case['id']}_baselines.json",baselines)
        start=time.perf_counter();request=core.model(n,ws);build_s=time.perf_counter()-start
        write_new(args.output/f"{case['id']}_model.json",{'model':request,'construction_s':build_s})
        for seed in seeds:
            if sha(args.engine)!=env['engine_sha256']:raise ValueError('engine changed')
            row=deadline.supervise(command_for(env,seed),request)
            # Persist before verification so failures remain reviewable.
            path=args.output/f"{case['id']}_{seed}.json"
            write_new(path,row)
            checked_endpoint(case,row)
        print(case['id'],'completed',len(seeds),'cells',flush=True)
    if source_files()!=source or sha(args.engine)!=env['engine_sha256']:raise ValueError('source/engine changed during run')
    write_new(args.output/'complete.json',{'cases':len(cases),'cells':len(cases)*len(seeds),'evaluated_commit':commit})
    seal_raw(args.output,cases,seeds)
    result=analyze(args.output)
    write_new(args.output/'summary.json',result)
    print(json.dumps(result,indent=2,sort_keys=True))


def analyze(directory):
    directory=Path(directory)
    env=json.loads((directory/'environment.json').read_text())
    verify_environment(env)
    if set(env['sources'])!=set(source_files()):raise ValueError('missing/extra source inventory')
    for name,h in env['sources'].items():
        if sha(ROOT/name)!=h:raise ValueError('analysis code/input changed')
        frozen=subprocess.check_output(['git','show',f"{env['evaluated_commit']}:{name}"],cwd=ROOT)
        if hashlib.sha256(frozen).hexdigest()!=h:raise ValueError('evaluated Git source mismatch')
    cases=json.loads((directory/'cases.json').read_text())
    complete=json.loads((directory/'complete.json').read_text())
    seeds=env['seeds']
    expected_cases,expected_seeds=design(env['smoke'])
    if cases!=expected_cases or seeds!=expected_seeds or env['case_sha256']!=CASE_SHA or env['budget_s']!=2.0:
        raise ValueError('archive design differs from frozen inputs')
    verify_raw(directory,cases,seeds)
    if complete!={'cases':len(cases),'cells':len(cases)*len(seeds),'evaluated_commit':env['evaluated_commit']}:
        raise ValueError('incomplete design')
    per_case=[];totals={arm:{k:0 for k in ['tp','fp','fn','tn']} for arm in BASELINES}
    native_totals={seed:{k:0 for k in ['tp','fp','fn','tn']} for seed in seeds}
    hits=0;wrapper_hits=0;fallbacks=0;gaps=[];cells=0;incumbents=0;conflicted=0;changed=0;previous_finish=0
    for case in cases:
        n=len(case['record_ids']);ws=case['weights'];truth=case['truth']
        baseline=json.loads((directory/f"{case['id']}_baselines.json").read_text())
        model_row=json.loads((directory/f"{case['id']}_model.json").read_text())
        if model_row['model']!=core.model(n,ws) or not deadline.base.finite_number(model_row['construction_s']) or model_row['construction_s']<0:
            raise ValueError('model evidence mismatch')
        if set(baseline)!=set(BASELINES):raise ValueError('baseline inventory')
        for arm,fn in BASELINES.items():
            row=baseline[arm]
            if not deadline.base.finite_number(row['wall_s']) or row['wall_s']<0:raise ValueError('invalid baseline time')
            if row['bits']!=fn(n,ws) or core.inspect(n,ws,row['bits'])!={k:row[k] for k in ['score','energy','violations']}:
                raise ValueError('baseline witness mismatch')
            for k,v in core.confusion(n,row['bits'],truth).items():totals[arm][k]+=v
        conflicted+=baseline['pairwise']['violations']>0
        changed+=baseline['exact']['bits']!=baseline['pairwise']['bits']
        outcomes=[]
        for seed in seeds:
            row=json.loads((directory/f"{case['id']}_{seed}.json").read_text())
            previous_finish=verify_row_identity(row,env,seed,previous_finish)
            incs=[e for e in row['events'] if e['event']['kind']=='inc'];incumbents+=len(incs)
            out=checked_endpoint(case,row)
            gap=out['energy']-baseline['exact']['energy']
            if gap<0:raise ValueError('native below independent exact oracle')
            cells+=1;wrapper_hits+=gap==0;hits+=native_hit(row,baseline['exact']['energy']);gaps.append(gap)
            fallbacks+=not any(e['received_s']<=2 and e['event'].get('origin')!='fallback' and e['event']['bits']==out['bits'] for e in incs)
            for k,v in core.confusion(n,out['bits'],truth).items():native_totals[seed][k]+=v
            outcomes.append({'seed':seed,**out,'optimal':gap==0,'gap':gap})
        per_case.append({'id':case['id'],'native':outcomes})
    aggregate={arm:core.metrics(v) for arm,v in totals.items()}
    f1=[core.metrics(v)['f1'] for v in native_totals.values()]
    q=statistics.quantiles(gaps,n=4,method='inclusive') if len(gaps)>1 else gaps*3
    delta=aggregate['exact']['f1']-aggregate['closure']['f1']
    best_simple=max(aggregate[a]['f1'] for a in ['pairwise','closure','greedy'])
    informative=conflicted>=3
    return {'scope':'development capability gate; no superiority/holdout claim','cells':cells,
            'all_incumbents_checked':incumbents,'native_optimum_hits':hits,'native_fallback_endpoints':fallbacks,
            'wrapper_optimum_hits':wrapper_hits,
            'conflicted_blocks':conflicted,'exact_changed_blocks':changed,'baseline_pair_metrics':aggregate,
            'native_f1_by_seed':dict(zip(map(str,seeds),f1)),
            'objective_gap':{'mean':statistics.mean(gaps),'median':statistics.median(gaps),
             'std':statistics.pstdev(gaps),'min':min(gaps),'max':max(gaps),'q25':q[0],'q75':q[2]},
            'exact_minus_closure_f1':delta,'informative_gate':informative,
            'exact_minus_best_simple_f1':aggregate['exact']['f1']-best_simple,
            'semantic_signal_gate':semantic_gate(totals,informative),
            'native_fidelity_gate':hits/cells>=.95,'native_success_fraction':hits/cells,
            'native_f1_mean':statistics.mean(f1),'per_case':per_case}


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['run','worker','analyze'])
    p.add_argument('--engine',type=Path);p.add_argument('--output',type=Path);p.add_argument('--seed',type=int)
    p.add_argument('--smoke',action='store_true');args=p.parse_args()
    if args.mode=='worker':worker(args.engine,args.seed)
    elif args.mode=='run':
        if not args.engine or not args.output:p.error('--engine and --output required')
        args.engine=args.engine.resolve();run(args)
    else:print(json.dumps(analyze(args.output),indent=2,sort_keys=True))


if __name__=='__main__':main()
