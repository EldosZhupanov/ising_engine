"""Read-only independent retained-witness/provenance audit; never runs solvers."""
from pathlib import Path
import sys,json,hashlib,subprocess,itertools,random,statistics as st,math
from fractions import Fraction as F

p=Path(sys.argv[1]);m=json.loads((p/'metadata.json').read_text());s=json.loads((p/'summary.json').read_text())
rows=[json.loads(t)for t in(p/'raw.jsonl').read_text().splitlines()]
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert h(p/'raw.jsonl')==s['raw_sha256'];sha=m['git_commit']
for path,d in m['hashes'].items():assert h(path)==d,path
for path,d in m['build']['source_hashes'].items():
 assert h(path)==d,path
 assert hashlib.sha256(subprocess.check_output(['git','show',sha+':'+path])).hexdigest()==d,path
for path,d in m['build']['binaries'].items():assert h(path)==d,path
assert m['build']==json.loads(Path('.cache/mqlib-qualification/screen_build.json').read_text())
for path,d in m['build']['upstream_build']['hashes'].items():assert h(path)==d,path
assert m['upstream_commit']==m['build']['upstream_build']['upstream_commit']=='585496274af5abb0849d0d47e135496b4688680b'
assert all(c['exit_code']==0 for c in m['build']['commands'])
subprocess.run(['git','merge-base','--is-ancestor','2d6d08d',sha],check=True)
arms=['ultimate','v2_default','mqlib'];order=[];nev=0;nlate=0;cfg=0;fallback={a:0 for a in arms}
manifest=json.loads(Path('research/experiments/mqlib_screen/manifest.json').read_text())
if m['smoke']:
 names=[f'smoke{i}'for i in range(4)];seeds=[501,502];budget=.25
 models={name:json.loads(Path(f'research/mqlib_qualification/run001/case{i:02d}/model.json').read_text())for i,name in enumerate(names)}
else:
 names=[x['name']for x in manifest['instances']];seeds=manifest['search_seeds'];budget=2.0;models={}
 for item in manifest['instances']:
  path=Path('research/experiments/mqlib_screen/instances')/item['name'];assert h(path)==item['sha256']
  q=json.loads(path.read_text());assert sum(map(abs,q['linear']))+sum(abs(v)for i,j,v in q['pairs'])==item['normalizer'];models[item['name']]=q
for i,name in enumerate(names):
 for j,seed in enumerate(seeds):order.extend((name,seed,a)for a in arms[(i+j)%3:]+arms[:(i+j)%3])
assert [(r['instance'],r['seed'],r['arm'])for r in rows]==order
for r in rows:
 q=models[r['instance']];best=F(q['offset']);state=[0]*len(q['linear']);tb=0;count=0;prev=-1;localcfg=0
 def energy(x):
  assert len(x)==len(q['linear'])and all(type(v)==int and v in (0,1)for v in x)
  return F(q['offset'])+sum(F(v)*x[i]for i,v in enumerate(q['linear']))+sum(F(v)*x[i]*x[j]for i,j,v in q['pairs'])
 assert r['status']=='VALID'and not r['errors']and r['budget_seconds']==budget and r['cpu']==0
 assert r['command'][:3]==['taskset','-c','0']and r['command'][-1]==str(r['seed'])
 for item in r['events']:
  t=item['received_seconds'];assert math.isfinite(t)and t>=prev;prev=t;assert item['eligible']==(t<budget)
  ev=json.loads(item['raw'])
  if ev['kind']=='config':
   assert r['arm']=='v2_default'and ev['replicas']==32 and ev['backend']in ['SparseBitSlice','DenseByte'];assert len(ev['temperatures'])==32
   assert all(abs(v-4*(.08/4)**(i/31))<1e-12 for i,v in enumerate(ev['temperatures']))
   assert ev['seed']==r['seed']*0x9e3779b97f4a7c15%(2**64);assert len(ev['operators'])in [1,2];cfg+=1;localcfg+=1;continue
  assert ev['kind']=='inc';e=energy(ev['state']);assert F(item['verified_energy'])==e
  assert e==(F(q['offset'])-F(ev['objective'])if r['arm']=='mqlib'else F(ev['energy']))
  nev+=1;nlate+=not item['eligible']
  if t<budget:
   count+=1
   if e<best:best=e;state=ev['state'];tb=t
 assert best==F(r['best_energy'])and state==r['best_state']and tb==r['time_to_best_seconds']and count==r['witnesses_before_deadline']
 assert r['fallback_only']==(count==0);fallback[r['arm']]+=count==0
 if m['smoke']:assert count>0
 if r['arm']=='v2_default':assert localcfg==1
 assert r['cutoff_seconds']>=budget and r['killed_at_deadline']and r['exit_code']==-9 and r['total_seconds']>=r['cutoff_seconds']
def distribution(values):
 ordered=sorted(values);n=len(values)
 def percentile(q):
  pos=(n-1)*q;lo=math.floor(pos);hi=math.ceil(pos)
  return ordered[lo]+(ordered[hi]-ordered[lo])*(pos-lo)
 return {'n':n,'mean':st.mean(values),'median':st.median(values),'std':st.pstdev(values),'min':ordered[0],'max':ordered[-1],'q25':percentile(.25),'q75':percentile(.75)}
def same(actual,expected):
 if isinstance(expected,dict):
  assert actual.keys()==expected.keys(),(actual.keys(),expected.keys())
  for key in expected:same(actual[key],expected[key])
 elif isinstance(expected,list):
  assert len(actual)==len(expected)
  for av,ev in zip(actual,expected):same(av,ev)
 elif isinstance(expected,(int,float)):
  assert math.isclose(actual,expected,rel_tol=1e-12,abs_tol=1e-14),(actual,expected)
 else:assert actual==expected,(actual,expected)
primary=None;wlts={};descriptive_instances=0
if not m['smoke']:
 index={(r['instance'],r['seed'],r['arm']):r['best_energy']for r in rows};ds=[]
 for i in manifest['instances']:
  ds.append(st.mean((index[i['name'],seed,'mqlib']-index[i['name'],seed,'ultimate'])/i['normalizer']for seed in seeds))
 obs=abs(st.mean(ds));prob=sum(abs(st.mean(d*z for d,z in zip(ds,ss)))>=obs-1e-15 for ss in itertools.product([-1,1],repeat=8))/256
 rng=random.Random(52001);boot=sorted(st.mean(rng.choices(ds,k=8))for _ in range(10000));ci=[boot[249],boot[9749]]
 primary={'instance_contrasts':ds,'mean':st.mean(ds),'sign_flip_p':prob,'bootstrap_95':ci,'distribution':distribution(ds),'H1':'SUPPORTED_ON_THIS_SCREEN'if st.mean(ds)>=.001 and prob<.05 and ci[0]>0 else 'INCONCLUSIVE'}
 same(s['primary'],primary);assert fallback==s['fallback_only']
 assert s['status']=='COMPLETE'and s['cells']==len(rows)and len(s['instances'])==len(names)
 normalized_means={a:[]for a in arms};oracle=[]
 for item,summary in zip(manifest['instances'],s['instances']):
  name=item['name'];scale=item['normalizer'];assert summary['name']==name
  values={a:[index[name,seed,a]for seed in seeds]for a in arms}
  same(summary['energies'],{a:distribution(vals)for a,vals in values.items()})
  same(summary['ultimate_vs_mqlib_normalized'],distribution([(index[name,seed,'mqlib']-index[name,seed,'ultimate'])/scale for seed in seeds]))
  for a,b in itertools.combinations(arms,2):
   delta=[index[name,seed,a]-index[name,seed,b]for seed in seeds]
   expected={'wins':sum(d<0 for d in delta),'ties':sum(d==0 for d in delta),'losses':sum(d>0 for d in delta),'normalized_improvement':distribution([-d/scale for d in delta])}
   same(summary['pairwise'][a+'_vs_'+b],expected)
  for a in arms:normalized_means[a].append(st.mean(v/scale for v in values[a]))
  oracle.append(min(normalized_means[a][-1]for a in arms));descriptive_instances+=1
 arm_means={a:st.mean(vals)for a,vals in normalized_means.items()}
 same(s['optimistic_diagnostic'],{'normalized_fixed_arm_means':arm_means,'oracle_mean':st.mean(oracle),'best_fixed_minus_oracle':min(arm_means.values())-st.mean(oracle),'warning':'post-hoc in-sample means; not deployable selection'})
 for a,b in itertools.combinations(arms,2):
  delta=[index[name,seed,a]-index[name,seed,b]for name in names for seed in seeds]
  wlts[a+'_vs_'+b]={'wins':sum(d<0 for d in delta),'ties':sum(d==0 for d in delta),'losses':sum(d>0 for d in delta)}
else:assert s['status']=='PASS'and s['fallback_only']==0
report={'audit':'PASS','raw_sha256':h(p/'raw.jsonl'),'cells':len(rows),'witnesses':nev,'late_witnesses':nlate,'v2_configs':cfg,'source_hashes':len(m['build']['source_hashes']),'extra_hashes':len(m['hashes']),'fallbacks':fallback,'primary':primary,'pairwise':wlts,'descriptive_instances_checked':descriptive_instances}
print(json.dumps(report,indent=2))
