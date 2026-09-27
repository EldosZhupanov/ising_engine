"""Deterministic MQ-DIFFICULTY-001 qualification inputs; never optimizer outcomes."""
import argparse
import hashlib
import json
from pathlib import Path

DOMAIN='ising-engine/MQ-DIFFICULTY-001/qualification/v1'


def encode(obj):
    return (json.dumps(obj,sort_keys=True,separators=(',',':'),allow_nan=False)+'\n').encode()


def make(n,denominator,seed):
    if type(n) is not int or not 1<=n<=512 or type(denominator) is not int or denominator not in (2,16) or type(seed) is not int or seed<0:
        raise ValueError('invalid generator arguments')
    h=[0]*n;pairs=[];offset=0
    for i in range(n):
        for j in range(i+1,n):
            digest=hashlib.sha256(f'{DOMAIN}/{seed}/{n}/{denominator}/{i}/{j}'.encode()).digest()
            if digest[0]>=256//denominator: continue
            v=digest[1]&15
            w=v-8 if v<8 else v-7  # uniform mapping of sixteen labels to {-8..-1,1..8}
            offset+=w;h[i]-=2*w;h[j]-=2*w;pairs.append([i,j,4*w])
    return {'offset':offset,'linear':h,'pairs':pairs}


def configurations():
    k=0
    for n in (256,512):
        for denominator in (16,2):
            for replica in range(6):
                yield {'id':f'q{k:02d}','n':n,'density_denominator':denominator,'replica':replica,'generator_seed':62001+k}
                k+=1


def write_corpus(folder):
    folder.mkdir(parents=True,exist_ok=False)
    entries=[]
    for c in configurations():
        m=make(c['n'],c['density_denominator'],c['generator_seed'])
        data=encode(m);path=folder/(c['id']+'.json');path.write_bytes(data)
        entries.append({**c,'file':path.name,'sha256':hashlib.sha256(data).hexdigest(),
                        'bytes':len(data),'pairs':len(m['pairs']),
                        'normalization_L':sum(abs(v) for v in m['linear'])+sum(abs(v) for _,_,v in m['pairs']),
                        'partition':'qualification','family':DOMAIN})
    return entries


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    print(json.dumps(write_corpus(a.output),indent=2))
