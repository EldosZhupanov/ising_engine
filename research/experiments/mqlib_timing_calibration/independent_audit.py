"""Independent MQ-CAL-001 raw-event audit; never executes a worker."""
import hashlib,json,math,statistics,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
HERE=ROOT/'research/experiments/mqlib_timing_calibration'
ARMS=['null_a','null_b','delay']
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def check(v, label):
    if not v: raise AssertionError(label)
def q95(v): return sorted(v)[math.ceil(.95*len(v))-1]
def audit(folder):
    folder=Path(folder); meta=json.loads((folder/'metadata.json').read_text()); summary=json.loads((folder/'summary.json').read_text()); manifest=meta['manifest']
    check(sha(folder/'raw.jsonl')==summary['raw_sha256'],'raw hash')
    check(meta['affinity']==[0],'parent affinity')
    check(manifest==json.loads((HERE/'manifest.json').read_text()),'manifest agreement')
    for name,h in meta['hashes'].items():
        p=Path(name) if name.startswith('/') else ROOT/name
        check(sha(p)==h,'disk source '+name)
        if not name.startswith('/'):
            blob=subprocess.check_output(['git','show',meta['git_commit']+':'+name],cwd=ROOT)
            check(hashlib.sha256(blob).hexdigest()==h,'committed source '+name)
    models={}
    for f in manifest['fixtures']:
        check(sha(HERE/f['file'])==f['sha256'],'fixture '+f['file'])
        models[f['file']]=json.loads((HERE/f['file']).read_text())
    rows=[json.loads(line) for line in (folder/'raw.jsonl').read_text().splitlines()]
    schedule=[]
    if meta['smoke']: schedule=[(manifest['fixtures'][0]['file'],200000000,0,'null_a')]
    else:
        for fi,f in enumerate(manifest['fixtures']):
            for bi,b in enumerate(manifest['budgets_seconds']):
                for i in range(20):
                    rot=(fi+bi+i)%3
                    schedule.extend((f['file'],round(b*1e9),i,a) for a in ARMS[rot:]+ARMS[:rot])
    check(len(rows)==len(schedule),'complete cell count')
    messages=0; all_on={}
    for ix,(r,expected) in enumerate(zip(rows,schedule)):
        label='row '+str(ix)
        check((r['fixture'],r['budget_ns'],r['repeat'],r['arm'])==expected,label+' schedule')
        check(r['status']=='VALID' and r['errors']==[],label+' validity')
        check(r['exit_code']==-9 and r['killed_at_deadline'],label+' terminal state')
        check(r['kill_sent_ns']>=r['start_ns']+r['budget_ns'],label+' no early kill')
        check(r['reaped_ns']>=r['kill_sent_ns'] and r['kill_reap_ns']==r['reaped_ns']-r['kill_sent_ns'],label+' reap')
        check(r['cpu']==0 and r['command'][:3]==['taskset','-c','0'],label+' worker affinity')
        check(r['cutoff_ns']>=r['budget_ns'],label+' cutoff')
        check(len(r['events'])<=1,label+' one message')
        raw=bytes.fromhex(r['raw_hex']); parts=raw.split(b'\n')
        check(parts[-1].hex()==r['partial_hex'],label+' raw partial')
        check([v.decode() for v in parts[:-1]]==[e['raw'] for e in r['events']],label+' packet raw conservation')
        model=models[r['fixture']]
        for e in r['events']:
            v=json.loads(e['raw']); x=v['state']; messages+=1
            check(set(v)=={'state','energy','sleep_start_ns','sleep_end_ns','pre_emit_ns'},label+' schema')
            check(len(x)==len(model['linear']) and all(type(n) is int and n==0 for n in x),label+' zero witness')
            energy=model['offset']+sum(h*x[i] for i,h in enumerate(model['linear']))+sum(w*x[i]*x[j] for i,j,w in model['pairs'])
            check(v['energy']==energy==e['verified_energy'],label+' energy')
            check(r['start_ns']<=v['sleep_start_ns']<=v['sleep_end_ns']<=v['pre_emit_ns']<=e['receipt_ns'],label+' clock ordering')
            check(e['sleep_ns']==v['sleep_end_ns']-v['sleep_start_ns'],label+' sleep')
            check(e['delivery_lag_ns']==e['receipt_ns']-v['pre_emit_ns'],label+' lag')
            check(e['latency_ns']==e['receipt_ns']-r['start_ns'],label+' latency')
            check(e['eligible']==(e['receipt_ns']<r['start_ns']+r['budget_ns']),label+' strict deadline')
            if e['eligible']: all_on[(r['fixture'],r['budget_ns'],r['repeat'],r['arm'])]=e
    groups=[]
    if meta['smoke']:
        check(summary['status']=='SMOKE_PASS' and messages==1,'smoke validity')
    else:
        for f in manifest['fixtures']:
            for b in manifest['budgets_seconds']:
                B=round(b*1e9); d=B//4
                rs=[r for r in rows if r['fixture']==f['file'] and r['budget_ns']==B]
                on={(i,a):e for (name,budget,i,a),e in all_on.items() if name==f['file'] and budget==B}
                counts={a:sum(aa==a for i,aa in on) for a in ARMS}
                null=[on[i,'null_b']['latency_ns']-on[i,'null_a']['latency_ns'] for i in range(20) if (i,'null_b') in on and (i,'null_a') in on]
                pos=[on[i,'delay']['latency_ns']-on[i,'null_a']['latency_ns'] for i in range(20) if (i,'delay') in on and (i,'null_a') in on]
                over=[r['cutoff_ns']-B for r in rs]; lag=[e['delivery_lag_ns'] for e in on.values()]
                sl=[r['events'][0]['sleep_ns']-d for r in rs if r['arm']=='delay' and r['events']]
                gates={'completion':all(n>=19 for n in counts.values()),'pairs':len(null)>=19 and len(pos)>=19,
                    'deadline':min(over)>=0 and q95(over)<=max(1000000,.05*B),
                    'delivery':bool(lag) and min(lag)>=0 and q95(lag)<=max(1000000,.05*B),
                    'null':bool(null) and abs(statistics.median(null))<=max(2000000,.1*B),
                    'positive':bool(pos) and abs(statistics.median(pos)-d)<=max(2000000,.2*d) and sum(v>0 for v in pos)>=18,
                    'sleep':counts['delay']>=19 and min(sl)>=0 and q95(sl)<=max(1000000,.1*d)}
                stored=next(g for g in summary['groups'] if g['fixture']==f['file'] and g['budget_ns']==B)
                check(stored['gates']==gates,f['file']+' gates '+str(B))
                verdict='PASS' if all(gates.values()) else 'FAIL'
                check(stored['status']==verdict,'group verdict')
                check(stored['completion_by_arm']=={a:n/20 for a,n in counts.items()},'completion fractions')
                check(stored['null_difference_ns']['n']==len(null) and stored['positive_difference_ns']['n']==len(pos),'pair counts')
                check(stored['on_time_delay_sleep_count']==counts['delay'],'on time delay sleep count')
                groups.append({'fixture':f['file'],'budget_ns':B,'status':verdict,'counts':counts,'gates':gates})
        expected_status='PASS' if all(g['status']=='PASS' for g in groups) else 'FAIL'
        check(summary['status']==expected_status,'overall verdict')
        admitted=[round(b*1e9) for b in manifest['budgets_seconds'] if all(g['status']=='PASS' for g in groups if g['budget_ns']==round(b*1e9))]
        check(summary['qualified_echo_budget_ns']==admitted,'admitted budgets')
    check(summary['elapsed_seconds']<manifest['hard_campaign_cap_seconds'],'campaign cap')
    return {'audit':'PASS','run':str(folder),'cells':len(rows),'messages_rescored':messages,'source_hashes':len(meta['hashes']),'registered_status':summary['status'],'groups':groups}
if __name__=='__main__': print(json.dumps(audit(sys.argv[1]),indent=2))
