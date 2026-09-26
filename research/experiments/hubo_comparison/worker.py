"""HUBO-C001 common warm wrapper. Parent owns deadline and independent scoring."""
import argparse
import json
import math
import subprocess
import sys
import time
import dimod
import openjij


def energy(case, bits):
    return sum(w * math.prod(2 * int(bits[i])-1 for i in v)
               for w, v in case['spin_terms'])


def restart_seed(seed, index):
    if not 0 <= index < 1_000_000:
        raise ValueError('restart limit exceeded')
    return seed + 10*index


def model(case, quadratic):
    terms = case['terms']
    n = case['n']
    penalty = 0
    if quadratic:
        poly = {tuple(v): w for w, v in terms}
        penalty = 1 + sum(abs(w) for w, v in terms if len(v)>2)
        bqm = dimod.make_quadratic(poly, penalty, dimod.BINARY)
        for v in range(n):
            bqm.add_variable(v,0)
        aux = sorted([v for v in bqm.variables if not isinstance(v,int)],key=str)
        labels = {v:v for v in range(n)}
        labels.update({v:n+i for i,v in enumerate(aux)})
        n += len(aux)
        terms = [[int(bqm.offset),[]]]
        terms += [[int(w),[labels[v]]] for v,w in bqm.linear.items() if w]
        terms += [[int(w),sorted([labels[u],labels[v]])] for (u,v),w in bqm.quadratic.items() if w]
    incident = [0]*n
    for w, variables in terms:
        for v in variables:
            incident[v] += abs(w)
    hot = max(1,max(incident))/math.log(2)
    return n, terms, hot, penalty


def emit(value):
    print(json.dumps(value,separators=(',',':')),flush=True)


def run():
    p=argparse.ArgumentParser()
    p.add_argument('--arm',choices=['msc_native','msc_quad','oj_native','oj_quad'],required=True)
    p.add_argument('--seed',type=int,required=True)
    p.add_argument('--engine',required=True)
    a=p.parse_args()
    print('READY',flush=True)
    case=json.loads(sys.stdin.readline())
    bits='0'*case['n']
    best=energy(case,bits)
    emit({'kind':'inc','energy':best,'bits':bits,'steps':0})
    start=time.perf_counter()
    n,terms,hot,penalty=model(case,a.arm.endswith('_quad'))
    emit({'kind':'meta','model_n':n,'model_terms':len(terms),'penalty':penalty,
          'hot':hot,'transform_s':time.perf_counter()-start})
    if a.arm.startswith('msc_'):
        request={'n':n,'original_n':case['n'],'terms':terms,'spin_terms':case['spin_terms'],
                 'hot':hot,'seed':a.seed}
        proc=subprocess.run([a.engine],input=json.dumps(request)+'\n',text=True,check=False)
        raise SystemExit(proc.returncode or 3) # normal early return is never success
    poly={tuple(v):w for w,v in terms if v}
    for v in range(n):poly.setdefault((v,),0)
    offset=sum(w for w,v in terms if not v)
    sampler=openjij.SASampler()
    index=0
    while True:
        result=sampler.sample_hubo(poly,'BINARY',num_sweeps=1000,num_reads=1,num_threads=1,
                                  beta_min=1/hot,beta_max=10,updater='METROPOLIS',
                                  random_number_engine='XORSHIFT',seed=restart_seed(a.seed,index),
                                  temperature_schedule='GEOMETRIC')
        sample=result.first.sample
        full=[int(sample[v]) for v in range(n)]
        if any(v not in (0,1) for v in full):raise ValueError('nonbinary response')
        actual=sum(w*math.prod(full[v] for v in variables) for w,variables in terms)
        if actual != result.first.energy + offset:raise ValueError('external model energy mismatch')
        bits=''.join(str(v) for v in full[:case['n']])
        e=energy(case,bits)
        index+=1
        if e<best:
            best=e
            emit({'kind':'inc','energy':e,'bits':bits,'steps':index*1000})


if __name__=='__main__':run()
