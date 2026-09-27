"""Small complete-graph entity reconciliation. No labels enter the optimizer."""
import itertools
import math
import re


def edges(n):
    if type(n) is not int or not 1 <= n <= 8:
        raise ValueError('oracle supports 1..8 records')
    return list(itertools.combinations(range(n), 2))


def validate_weights(n, weights):
    if len(weights) != len(edges(n)) or any(type(w) is not int or abs(w)>1_000_000 for w in weights):
        raise ValueError('one integer score per complete-graph edge required')


def tokens(title):
    if not isinstance(title, str):
        raise ValueError('title must be text')
    return frozenset(re.findall(r'[^\W_]+', title.casefold()))


def similarity(a, b):
    return len(a & b) / len(a | b) if a | b else 0.0


def weights_for(records):
    ts = [tokens(r['title']) for r in records]
    # Integer utility, not calibrated probability; exact half-up rounding.
    result = []
    for i, j in edges(len(records)):
        common, total = len(ts[i] & ts[j]), len(ts[i] | ts[j])
        result.append((400*common + total)//(2*total)-100 if total else -100)
    return result


def blocks(records, maximum=6):
    """Label-blind disjoint greedy blocking; all rows included, no quality filter."""
    if not 2 <= maximum <= 8:
        raise ValueError('invalid block size')
    ids = [str(r['id']) for r in records]
    if len(set(ids)) != len(ids):
        raise ValueError('duplicate record id')
    by_id = {str(r['id']): {'id': str(r['id']), 'title': r['title']} for r in records}
    ts = {i: tokens(r['title']) for i, r in by_id.items()}
    left = set(ids)
    result = []
    for anchor in sorted(ids):
        if anchor not in left:
            continue
        candidates = sorted(left - {anchor}, key=lambda i: (-similarity(ts[anchor], ts[i]), i))
        group = [anchor] + [i for i in candidates if similarity(ts[anchor], ts[i]) >= .1][:maximum-1]
        left.difference_update(group)
        result.append([by_id[i] for i in group])
    return result


def partition_bits(labels):
    return ''.join(str(int(labels[i] == labels[j])) for i, j in edges(len(labels)))


def partitions(n):
    edges(n)
    def rec(prefix):
        if len(prefix) == n:
            yield tuple(prefix)
        else:
            for label in range(max(prefix)+2):
                yield from rec(prefix+[label])
    yield from rec([0])


def inspect(n, weights, bits):
    validate_weights(n, weights)
    ee = edges(n)
    if not isinstance(bits, str) or len(bits) != len(ee) or set(bits)-{'0', '1'}:
        raise ValueError('invalid edge witness')
    pair = dict(zip(ee, map(int, bits)))
    violations = sum(sum(pair[min(i,j), max(i,j)] for i,j in itertools.combinations(t,2)) == 2
                     for t in itertools.combinations(range(n),3))
    score = sum(w*int(bit) for w,bit in zip(weights,bits))
    penalty = 1 + sum(abs(w) for w in weights)
    return {'score': score, 'violations': violations, 'energy': -score+penalty*violations}


def exact(n, weights):
    validate_weights(n, weights)
    # Enumerates set partitions directly, not triangle-penalized bit vectors.
    candidates = ((sum(w*int(b) for w,b in zip(weights, partition_bits(p))), partition_bits(p))
                  for p in partitions(n))
    score, bits = min(candidates, key=lambda row: (-row[0],row[1]))
    return bits


def threshold(n, weights):
    validate_weights(n, weights)
    return ''.join('1' if w > 0 else '0' for w in weights)


def closure(n, weights):
    labels = list(range(n))
    for (a,b),bit in zip(edges(n),threshold(n,weights)):
        if bit == '1':
            old, new = labels[b], labels[a]
            labels = [new if c == old else c for c in labels]
    return partition_bits(labels)


def greedy(n, weights):
    validate_weights(n,weights)
    groups = [{i} for i in range(n)]
    scores = dict(zip(edges(n),weights))
    while len(groups)>1:
        choices = [(sum(scores[min(i,j),max(i,j)] for i in a for j in b),u,v)
                   for u,a in enumerate(groups) for v,b in enumerate(groups) if u<v]
        gain,u,v = min(choices,key=lambda row:(-row[0],row[1],row[2]))
        if gain<=0:
            break
        groups[u] |= groups.pop(v)
    labels=[next(k for k,g in enumerate(groups) if i in g) for i in range(n)]
    return partition_bits(labels)


def model(n, weights):
    validate_weights(n, weights)
    ee = edges(n)
    index = {p:k for k,p in enumerate(ee)}
    poly = {(k,):-w for k,w in enumerate(weights) if w}
    penalty = 1+sum(abs(w) for w in weights)
    for a,b,c in itertools.combinations(range(n),3):
        tri = sorted([index[a,b],index[a,c],index[b,c]])
        for pair in itertools.combinations(tri,2):
            poly[pair]=poly.get(pair,0)+penalty
        poly[tuple(tri)]=poly.get(tuple(tri),0)-3*penalty
    # Both integer representations equal 8 times the original energy.
    spin = {}
    for vs,w in poly.items():
        for k in range(len(vs)+1):
            for sub in itertools.combinations(vs,k):
                spin[sub]=spin.get(sub,0)+(8//(2**len(vs)))*w
    return {'n':len(ee),'terms':[[8*w,list(v)] for v,w in sorted(poly.items()) if w],
            'spin_terms':[[w,list(v)] for v,w in sorted(spin.items()) if w]}


def confusion(n, bits, truth):
    # Evaluation only: truth must never be handed to a scorer/search function.
    if len(truth)!=n:
        raise ValueError('truth size')
    inspect(n,[0]*len(edges(n)),bits)
    counts={'tp':0,'fp':0,'fn':0,'tn':0}
    for (i,j),b in zip(edges(n),bits):
        actual=truth[i]==truth[j]
        counts['tp' if b=='1' and actual else 'fp' if b=='1' else 'fn' if actual else 'tn']+=1
    return counts


def metrics(counts):
    tp,fp,fn=counts['tp'],counts['fp'],counts['fn']
    return dict(counts, precision=tp/(tp+fp) if tp+fp else 0.,
                recall=tp/(tp+fn) if tp+fn else 0.,
                f1=2*tp/(2*tp+fp+fn) if 2*tp+fp+fn else 0.)


def reconcile(request):
    """Public research boundary: complete scores in lexicographic index-pair order."""
    if not isinstance(request,dict) or set(request)!={'record_ids','weights'}:
        raise ValueError('expected only record_ids and weights; labels forbidden')
    ids=request['record_ids']
    if not isinstance(ids,list) or any(not isinstance(i,str) or not i for i in ids) or len(set(ids))!=len(ids):
        raise ValueError('unique nonempty string IDs required')
    n=len(ids);weights=request['weights'];bits=exact(n,weights)
    labels=list(range(n))
    for (i,j),bit in zip(edges(n),bits):
        if bit=='1':
            old,new=labels[j],labels[i]
            labels=[new if k==old else k for k in labels]
    groups=[[ids[i] for i in range(n) if labels[i]==k] for k in sorted(set(labels))]
    return {'clusters':groups,'bits':bits,**inspect(n,weights,bits),
            'status':'exact objective optimum for this block; semantic truth not certified'}


if __name__=='__main__':
    import json
    import sys
    print(json.dumps(reconcile(json.load(sys.stdin)),indent=2))
