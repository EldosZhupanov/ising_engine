"""Pure preregistered analysis. No subprocesses, datasets or optimizer invocation."""
import hashlib
import itertools
import math
import statistics as st

ARMS = ['ultimate', 'v2_default', 'mqlib_merz', 'mqlib_mst2']
CHECKPOINTS = [.25, .5, 1., 2.]


def summary(x):
    return dict(n=len(x), values=x, mean=st.mean(x), median=st.median(x), std=st.pstdev(x), min=min(x), max=max(x))


def checkpoint(record, budget):
    best = record['fallback_energy']; count = 0
    for e in record['events']:
        if 'verified_energy' in e and e['received_seconds'] < budget and e['validated_seconds'] < budget:
            best = min(best, e['verified_energy']); count += 1
    return best, count


def schedule(m, phase):
    if phase == 'main':
        for i, item in enumerate(m['instances']):
            for j, seed in enumerate(m['search_seeds']):
                k = (i+j) % 4
                for arm in ARMS[k:] + ARMS[:k]:
                    yield dict(instance=item['id'], arm=arm, seed=seed, mode='search', budget=2., block='main')
    else:
        for i, name in enumerate(m['preflight']['instances']):
            for j in range(10):
                k = (i+j) % 4
                for arm in ARMS[k:] + ARMS[:k]:
                    modes = ['null_a', 'null_b', 'delay']; k2 = (i+j+ARMS.index(arm)) % 3
                    for mode in modes[k2:] + modes[:k2]:
                        yield dict(instance=name, arm=arm, seed=0, mode=mode, budget=.25, block='control', repetition=j)
        for i in range(4):
            for j, seed in enumerate([501, 502]):
                k = (i+j) % 4
                for arm in ARMS[k:] + ARMS[:k]:
                    yield dict(instance='smoke'+str(i), arm=arm, seed=seed, mode='search', budget=.25, block='smoke')


def verify_schedule(records, m, phase):
    expected = list(schedule(m, phase))
    return len(records) == len(expected) and all(r['cell'] == c and r['status'] == 'VALID' for r, c in zip(records, expected))


def preflight(records, m):
    if not verify_schedule(records, m, 'preflight'): return dict(status='FAIL', reason='missing/invalid/out-of-order cells')
    groups = []
    for name in m['preflight']['instances']:
        for arm in ARMS:
            rs = [r for r in records if r['cell']['instance'] == name and r['cell']['arm'] == arm]
            times = {mode: [] for mode in ['null_a', 'null_b', 'delay']}; sleep = []; valid = True
            for r in rs:
                inc = [e for e in r['events'] if 'verified_energy' in e]
                valid &= len(inc) == 1 and checkpoint(r, .25)[1] == 1 and inc[0]['verified_energy'] == r['fallback_energy']
                if inc:
                    times[r['cell']['mode']].append(inc[0]['validated_seconds'])
                    if r['cell']['mode'] == 'delay': sleep.append(inc[0]['message']['sleep_ns']/1e9)
            if any(len(x)!=10 for x in times.values()) or len(sleep)!=10:
                groups.append(dict(instance=name, arm=arm, pass_gate=False, reason='missing probe witness')); continue
            null = abs(st.median(times['null_a'])-st.median(times['null_b']))
            delta = st.median([a-b for a,b in zip(times['delay'], times['null_a'])])
            slept = st.median(sleep)
            ok = bool(valid and null <= .010 and .025 <= delta <= .075 and .045 <= slept <= .075)
            groups.append(dict(instance=name, arm=arm, pass_gate=ok, null_median_difference=null, paired_delay_median=delta, sleep_median=slept))
    overshoots = sorted(max(0, r['cutoff_seconds']-.25) for r in records if r['cell']['block']=='control')
    kill_ok = overshoots[math.ceil(.95*len(overshoots))-1] <= .025 and max(overshoots) <= .100
    smoke_ok = all(checkpoint(r, .25)[1] > 0 and (r['cell']['arm'] != 'v2_default' or r['configs']==1)
                   for r in records if r['cell']['block']=='smoke')
    return dict(status='PASS' if all(g['pass_gate'] for g in groups) and kill_ok and smoke_ok else 'FAIL',
                groups=groups, cutoff_gate=kill_ok, smoke_gate=smoke_ok,
                p95_overshoot=overshoots[math.ceil(.95*len(overshoots))-1], max_overshoot=max(overshoots))


def bootstrap_indices():
    pending=[]; counter=0; limit=(2**64//24)*24
    while True:
        digest=hashlib.sha256(('MQ-DIFFICULTY-001/bootstrap/v1/'+str(counter)).encode()).digest(); counter+=1
        for off in range(0,32,8):
            n=int.from_bytes(digest[off:off+8], 'big')
            if n<limit:
                pending.append(n%24)
                if len(pending)==24: yield pending; pending=[]


def main_analysis(records, m):
    if not verify_schedule(records, m, 'main'): return dict(status='NO_VERDICT', reason='missing/invalid/out-of-order cells')
    result=[]; wins={a:0 for a in ARMS}; informative=0
    contrasts={a+'_vs_'+b:[] for a,b in itertools.combinations(ARMS,2)}
    for item in m['instances']:
        rs=[r for r in records if r['cell']['instance']==item['id']]; scale=item['normalization_L']; curves=[]
        for t in CHECKPOINTS:
            values={a:[checkpoint(r,t)[0] for r in rs if r['cell']['arm']==a] for a in ARMS}
            curves.append(dict(seconds=t, arms={a:summary(x) for a,x in values.items()},
                fallback_only={a:sum(checkpoint(r,t)[1]==0 for r in rs if r['cell']['arm']==a) for a in ARMS},
                paired={a+'_vs_'+b:dict(wins=sum(x<y for x,y in zip(values[a],values[b])), ties=sum(x==y for x,y in zip(values[a],values[b])), losses=sum(x>y for x,y in zip(values[a],values[b]))) for a,b in itertools.combinations(ARMS,2)}))
        means={a:st.mean(x) for a,x in values.items()}; ranked=sorted(means,key=means.get)
        spread=(means[ranked[-1]]-means[ranked[0]])/scale
        top=(means[ranked[1]]-means[ranked[0]])/scale
        informative+=spread>=.001
        if top>=.001: wins[ranked[0]]+=1
        for a,b in itertools.combinations(ARMS,2): contrasts[a+'_vs_'+b].append((means[b]-means[a])/scale)
        result.append(dict(instance=item['id'], curves=curves, spread=spread, top_two_spread=top))
    resamples=list(itertools.islice(bootstrap_indices(),10000)); ci={}
    for name,x in contrasts.items():
        boot=sorted(st.mean(x[i] for i in indices) for indices in resamples)
        def q(p):
            j=p*9999;lo=int(j);return boot[lo]+(j-lo)*(boot[min(lo+1,9999)]-boot[lo])
        ci[name]=dict(mean=st.mean(x), descriptive_95=[q(.025),q(.975)])
    return dict(status='COMPLETE', instances=result, informative_instances=informative, unique_wins=wins,
                H1='SUPPORTED_ON_QUALIFICATION' if informative>=8 else 'INCONCLUSIVE',
                H2='SUPPORTED_ON_QUALIFICATION' if sum(v>=4 for v in wins.values())>=2 else 'NOT_ESTABLISHED',
                descriptive_instance_bootstrap=ci)
