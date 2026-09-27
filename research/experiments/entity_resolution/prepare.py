"""Pinned WDC development-only intake. Never opens official validation/test members."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import time
import urllib.request
import zipfile
import core

URL='https://data.dws.informatik.uni-mannheim.de/largescaleproductcorpus/data/wdc-products/80multi.zip'
ARCHIVE_SHA='5385e2491e82c7f6c208d8eac7ec777a4a44180769f3a339c99803ff2b068981'
MEMBER='wdcproductsmulti80cc20rnd000un_train_large.json.gz'
MEMBER_SHA='152ee17e30c2488bb6b87bd3e9e6fdd4e3f47a5f1181449f47c0c60ff5d24dde'
MAX_BLOCK=6
BLOCK_COUNT=12


def sha(data):return hashlib.sha256(data).hexdigest()


def prepare(archive):
    start=time.perf_counter()
    content=Path(archive).read_bytes()
    if sha(content)!=ARCHIVE_SHA:raise ValueError('archive hash mismatch')
    with zipfile.ZipFile(archive) as z:
        raw=z.read(MEMBER)
    if sha(raw)!=MEMBER_SHA:raise ValueError('member hash mismatch')
    rows=[json.loads(line) for line in gzip.decompress(raw).splitlines()]
    if len(rows)!=2841 or len({str(r['id']) for r in rows})!=2841:
        raise ValueError('unexpected row count/duplicate ids')
    if any(r['label']!=r['cluster_id'] or not isinstance(r['title'],str) for r in rows):
        raise ValueError('invalid source schema')
    # Drop label, cluster_id, unseen, URL and all other columns at input boundary.
    inputs=[{'id':str(r['id']),'title':r['title']} for r in rows]
    groups=core.blocks(inputs,MAX_BLOCK)
    selected=[g for g in groups if len(g)>=3][:BLOCK_COUNT]
    if len(selected)!=BLOCK_COUNT:raise ValueError('insufficient blocks; no silent resizing')
    truth={str(r['id']):str(r['cluster_id']) for r in rows}
    cases=[{'id':f'wdc-{i:02d}','record_ids':[r['id'] for r in group],
            'weights':core.weights_for(group),'truth':[truth[r['id']] for r in group]}
           for i,group in enumerate(selected)]
    selected_ids={i for c in cases for i in c['record_ids']}
    size_counts=Counter(truth.values())
    all_positives=sum(n*(n-1)//2 for n in size_counts.values())
    observed_positives=sum(sum(a==b for a,b in __import__('itertools').combinations(c['truth'],2)) for c in cases)
    record={'schema':1,'data_scope':'development-only, not official benchmark score',
            'archive_url':URL,'archive_sha256':ARCHIVE_SHA,'member':MEMBER,'member_sha256':MEMBER_SHA,
            'source_rows':len(rows),'source_products':len(size_counts),
            'all_block_sizes':dict(sorted(Counter(map(len,groups)).items())),
            'selected_records':len(selected_ids),'selected_blocks':len(cases),
            'source_positive_pairs':all_positives,'selected_within_block_positive_pairs':observed_positives,
            'cases':cases}
    # Timing is separate: cases.json remains byte-reproducible across runs.
    return record,{'intake_and_blocking_s':time.perf_counter()-start}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--archive',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--download',action='store_true')
    a=p.parse_args()
    if a.download and not a.archive.exists():
        a.archive.parent.mkdir(parents=True,exist_ok=True)
        with urllib.request.urlopen(URL,timeout=45) as r:
            content=r.read(10_000_001)
        if len(content)>10_000_000 or sha(content)!=ARCHIVE_SHA:
            raise ValueError('download size/hash mismatch')
        with a.archive.open('xb') as f:f.write(content)
    result,timing=prepare(a.archive)
    with a.output.open('x') as f:f.write(json.dumps(result,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'cases_sha256':sha(a.output.read_bytes()),**timing}))


if __name__=='__main__':main()
