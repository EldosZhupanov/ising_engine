#!/usr/bin/env python3
"""Independent ER-001 raw-evidence audit; no imports from the pilot, no solver runs.

Reproduce: python3 research/experiments/entity_resolution/independent_audit.py main
Use smoke instead of main for the isolated synthetic smoke. Requires source files
and binary matching the recorded evaluated commit; run without Python -O.
"""

import json,hashlib,itertools,math,statistics,subprocess,sys
from fractions import Fraction
from pathlib import Path
root=Path(__file__).resolve().parents[3]
if not __debug__:
 raise RuntimeError('Independent assertions require Python without -O')
if len(sys.argv)!=2 or sys.argv[1] not in ('smoke','main'):
 raise SystemExit('usage: independent_audit.py {smoke|main}')
d=root/'research/experiments/entity_resolution/run'/sys.argv[1]
read=lambda p:json.loads(p.read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
env=read(d/'environment.json');summary=read(d/'summary.json')
assert env['evaluated_commit']=='96b0c23090d0b82251e8d99c1a8e8a6bf6684a09'
assert sha(Path(env['engine']))==env['engine_sha256']
required={'core.py','prepare.py','pilot.py','test_core.py','test_pilot.py','protocol.md','cases.json'}
scope=['src','Cargo.toml','Cargo.lock','.cargo/config.toml','research/Cargo.toml','research/examples/hubo_compare.rs','research/experiments/hubo_comparison/campaign.py','research/experiments/hubo_corpus_qualification/phase_b.py']
names=set(subprocess.check_output(['git','ls-tree','-r','--name-only',env['evaluated_commit'],'--',*scope],cwd=root,text=True).splitlines())
names|={'research/experiments/entity_resolution/'+n for n in required}
assert names==set(env['sources'])
for name,h in env['sources'].items():
 assert sha(root/name)==h
 assert hashlib.sha256(subprocess.check_output(['git','show',env['evaluated_commit']+':'+name],cwd=root)).hexdigest()==h
assert env['threads']=={'MKL_NUM_THREADS':'1','OMP_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1','PYTHONHASHSEED':'0','RAYON_NUM_THREADS':'1'}
assert Path(env['pilot_path'])==Path(env['repo_root'])/'research/experiments/entity_resolution/pilot.py'
assert env['budget_s']==2.0
cases=read(d/'cases.json');seeds=env['seeds']
canonical=root/'research/experiments/entity_resolution/cases.json'
assert sha(canonical)==env['case_sha256']=='3214b795b55805fe5314830004e0b3a31bb50e21b671a1b5fe503d93e7bfe271'
if env['smoke']:
 assert seeds==[991001] and cases==[{'id':'smoke','record_ids':['a','b','c'],'weights':[2,2,-3],'truth':['a','a','b']}]
else:
 assert seeds==list(range(990001,990011)) and cases==read(canonical)['cases']
expected={'environment.json','cases.json','complete.json'}|{f"{c['id']}_{s}.json" for c in cases for s in ['baselines','model',*seeds]}
manifest=read(d/'raw_manifest.json')
assert set(manifest)==expected
assert {p.name for p in d.iterdir()}==expected|{'raw_manifest.json','summary.json'}
for name,h in manifest.items():assert sha(d/name)==h
assert read(d/'complete.json')=={'cases':len(cases),'cells':len(cases)*len(seeds),'evaluated_commit':env['evaluated_commit']}
def partitions(items):
 if not items:
  yield []
  return
 first,*rest=items
 for ps in partitions(rest):
  yield [[first],*[g[:] for g in ps]]
  for k in range(len(ps)):
   yield [([first]+g if i==k else g[:]) for i,g in enumerate(ps)]
def evaluate(c,bits):
 n=len(c['record_ids']);ee=list(itertools.combinations(range(n),2))
 assert isinstance(bits,str) and len(bits)==len(ee) and set(bits)<=set('01')
 links=dict(zip(ee,map(int,bits)))
 bad=sum(sum(links[e] for e in itertools.combinations(t,2))==2 for t in itertools.combinations(range(n),3))
 score=sum(w*links[e] for w,e in zip(c['weights'],ee))
 return {'score':score,'violations':bad,'energy':-score+(1+sum(map(abs,c['weights'])))*bad}
def bits_for(n,ps):
 return ''.join(str(int(any(i in g and j in g for g in ps))) for i,j in itertools.combinations(range(n),2))
def counts(c,bits):
 out=dict(tp=0,fp=0,fn=0,tn=0)
 for bit,(i,j) in zip(bits,itertools.combinations(range(len(c['truth'])),2)):
  predicted=bit=='1';truth=c['truth'][i]==c['truth'][j]
  out[('tp' if truth else 'fp') if predicted else ('fn' if truth else 'tn')]+=1
 return out
def f1(cs):
 den=2*cs['tp']+cs['fp']+cs['fn']
 return Fraction(2*cs['tp'],den) if den else Fraction(0)
def add(a,b):
 for k,v in b.items():a[k]+=v
totals={a:dict(tp=0,fp=0,fn=0,tn=0) for a in ['pairwise','closure','greedy','exact']}
native={s:dict(tp=0,fp=0,fn=0,tn=0) for s in seeds}
hits=wrapper=fallbacks=incs=conflicted=changed=0;previous=0;gaps=[];per_case=[]
for c in cases:
 n=len(c['record_ids']);ee=list(itertools.combinations(range(n),2));w=dict(zip(ee,c['weights']))
 bitstates=[bits_for(n,ps) for ps in partitions(list(range(n)))]
 assert len(bitstates)==len(set(bitstates))=={3:5,6:203}[n]
 optimum=min((evaluate(c,b)['energy'],b) for b in bitstates)
 threshold=''.join(str(int(x>0)) for x in c['weights'])
 graph=[{i} for i in range(n)]
 for (i,j),weight in w.items():
  if weight>0:
   ia=next(k for k,g in enumerate(graph) if i in g);ib=next(k for k,g in enumerate(graph) if j in g)
   if ia!=ib:graph[ia]|=graph[ib];graph.pop(ib)
 closure=bits_for(n,graph)
 graph=[{i} for i in range(n)]
 while len(graph)>1:
  moves=[(-sum(w[min(i,j),max(i,j)] for i in graph[a] for j in graph[b]),a,b) for a,b in itertools.combinations(range(len(graph)),2)]
  neg,a,b=min(moves)
  if neg>=0:break
  graph[a]|=graph.pop(b)
 greedy=bits_for(n,graph)
 base=read(d/f"{c['id']}_baselines.json")
 for arm,bits in dict(pairwise=threshold,closure=closure,greedy=greedy,exact=optimum[1]).items():
  assert base[arm]['bits']==bits
  assert {k:base[arm][k] for k in ['energy','score','violations']}==evaluate(c,bits)
  assert math.isfinite(base[arm]['wall_s']) and base[arm]['wall_s']>=0
  add(totals[arm],counts(c,bits))
 conflicted+=evaluate(c,threshold)['violations']>0;changed+=optimum[1]!=threshold
 modelrow=read(d/f"{c['id']}_model.json");m=modelrow['model']
 assert set(m)=={'n','terms','spin_terms'} and m['n']==len(ee)
 assert modelrow['construction_s']>=0 and math.isfinite(modelrow['construction_s'])
 outcomes=[]
 for seed in seeds:
  row=read(d/f"{c['id']}_{seed}.json");cl=row['clocks']
  assert row['command']==[env['python_executable'],env['pilot_path'],'worker','--engine',env['engine'],'--seed',str(seed)]
  assert not row['errors'] and not row['survivors'] and row['returncode'] in [-15,-9]
  assert previous<cl['boot_ns']<=cl['go_ns']<=cl['stop_ns']<=cl['term_ns']<=cl['finish_ns'];previous=cl['finish_ns']
  for field,l,r in [('startup_s','boot_ns','go_ns'),('stop_s','go_ns','stop_ns'),('termination_s','go_ns','finish_ns')]:
   assert row[field]==(cl[r]-cl[l])/1e9
  assert row['budget_s']==2 and 0<=row['startup_s']<=30 and 2<=row['stop_s']<=row['termination_s']<=2.2
  events=row['events'];times=[e['received_s'] for e in events]
  assert times==sorted(times) and all(math.isfinite(t) and 0<=t<=row['termination_s'] for t in times)
  lines=row['stdout'].split('\n')[:-1]
  assert lines[0]=='READY' and list(map(json.loads,lines[1:]))==[e['event'] for e in events]
  meta=[e['event'] for e in events if e['event']['kind']=='meta'];assert len(meta)==1
  assert meta[0]['model_n']==len(ee) and meta[0]['model_terms']==len(m['terms']) and meta[0]['penalty']==0
  incidence=[sum(abs(w) for w,vs in m['terms'] if i in vs) for i in range(m['n'])]
  assert math.isclose(meta[0]['hot'],max(1,max(incidence))/math.log(2),rel_tol=1e-14)
  mt=next(e['received_s'] for e in events if e['event']['kind']=='meta')
  assert math.isfinite(meta[0]['transform_s']) and 0<=meta[0]['transform_s']<=mt
  assert type(row['pid']) is int and row['pid']>0
  assert all(type(cl[k]) is int and cl[k]>0 for k in ['boot_unix_ns','go_unix_ns','finish_unix_ns'])
  vals=[]
  for e in events:
   ev=e['event']
   if ev['kind']!='inc':continue
   incs+=1;measured=evaluate(c,ev['bits']);xx=list(map(int,ev['bits']))
   binary=sum(a*math.prod(xx[i] for i in vs) for a,vs in m['terms'])
   spin=sum(a*math.prod(2*xx[i]-1 for i in vs) for a,vs in m['spin_terms'])
   assert ev['energy']==binary==spin==8*measured['energy']
   if e['received_s']<=2:vals.append((measured['energy'],ev['bits'],e['received_s'],ev.get('origin')))
  best=min(vals,key=lambda x:x[0]);energy,bits,t,_=best
  assert evaluate(c,bits)['violations']==0
  gap=energy-optimum[0];assert gap>=0;gaps.append(gap)
  wrapper+=gap==0;hits+=any(x[0]==optimum[0] and x[3]!='fallback' for x in vals)
  fallbacks+=not any(x[1]==bits and x[3]!='fallback' for x in vals)
  add(native[seed],counts(c,bits))
  outcomes.append(dict(seed=seed,bits=bits,**evaluate(c,bits),received_s=t,optimal=gap==0,gap=gap))
 per_case.append(dict(id=c['id'],native=outcomes))
assert summary['per_case']==per_case
for arm,cs in totals.items():
 stored=summary['baseline_pair_metrics'][arm]
 assert all(stored[k]==v for k,v in cs.items()) and stored['f1']==float(f1(cs))
 assert stored['precision']==(cs['tp']/(cs['tp']+cs['fp']) if cs['tp']+cs['fp'] else 0)
 assert stored['recall']==(cs['tp']/(cs['tp']+cs['fn']) if cs['tp']+cs['fn'] else 0)
assert summary['native_f1_by_seed']=={str(s):float(f1(cs)) for s,cs in native.items()}
h1=conflicted>=3;h2=h1 and f1(totals['exact'])-max(f1(totals[a]) for a in ['pairwise','closure','greedy'])>=Fraction(1,50)
checks=dict(cells=len(gaps),all_incumbents_checked=incs,native_optimum_hits=hits,wrapper_optimum_hits=wrapper,native_fallback_endpoints=fallbacks,conflicted_blocks=conflicted,exact_changed_blocks=changed,informative_gate=h1,semantic_signal_gate=h2,native_fidelity_gate=Fraction(hits,len(gaps))>=Fraction(95,100))
assert all(summary[k]==v for k,v in checks.items())
assert summary['native_success_fraction']==hits/len(gaps)
assert summary['native_f1_mean']==statistics.mean(float(f1(cs)) for cs in native.values())
assert summary['exact_minus_closure_f1']==float(f1(totals['exact']))-float(f1(totals['closure']))
assert summary['exact_minus_best_simple_f1']==float(f1(totals['exact']))-max(float(f1(totals[a])) for a in ['pairwise','closure','greedy'])
q=statistics.quantiles(gaps,n=4,method='inclusive') if len(gaps)>1 else gaps*3
assert summary['objective_gap']==dict(mean=statistics.mean(gaps),median=statistics.median(gaps),std=statistics.pstdev(gaps),min=min(gaps),max=max(gaps),q25=q[0],q75=q[2])
print(json.dumps(dict(status='PASS independent audit',mode=sys.argv[1],hashed_raw=len(manifest),hashed_sources=len(names),checks=checks,baseline_counts=totals,f1={k:float(f1(v)) for k,v in totals.items()},native_f1={str(s):float(f1(v)) for s,v in native.items()},gap=summary['objective_gap']),indent=2))
