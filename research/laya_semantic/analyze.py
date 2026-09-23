"""Frozen descriptive analysis; independently rechecks every stored case."""
import argparse
import json
from pathlib import Path
import statistics

from pipeline import corpus, verify_and_score


def analyze(directory):
    groups = json.loads((directory/'corpus.json').read_text())
    if groups != json.loads(json.dumps(corpus())):
        raise ValueError('corpus differs from protocol generator')
    if not (directory/'complete.json').exists():
        raise ValueError('incomplete pilot')
    totals = {}
    fixed = []
    first = []
    residual = []
    counts = []
    brier = []
    raw_seconds = []
    for group in groups:
        gid = group['id']
        case = json.loads((directory/f'case_{gid:02d}.json').read_text())
        raw = json.loads((directory/f'raw_{gid:02d}.json').read_text())
        values = [raw['result']['answers'][f'q{i}']['noul'] for i in range(8)]
        if values != case['probabilities']:
            raise ValueError('cached probability mismatch')
        scores = verify_and_score(group,values,case['bridge'])
        if scores != case['scores']:
            raise ValueError('cached score mismatch')
        for name,s in scores.items():
            t = totals.setdefault(name,dict(correct=0,group_correct=0,feasible_groups=0,
                                            violations=0,recovered=0,damaged=0,max_energy_gap=0.0))
            for key in ('correct','group_correct','violations','recovered','damaged'):
                t[key] += s[key]
            t['feasible_groups'] += s['violations']==0
            t['max_energy_gap'] = max(t['max_energy_gap'],s['energy_gap'])
        reduced = case['bridge']['reduced_exact']
        fixed.append(len(reduced['fixed']))
        first.append(len(reduced['first_order_fixed']))
        residual.append(max(map(len,reduced['components']),default=0))
        counts.append(reduced['enumerated_states'])
        brier.extend((p-y)**2 for p,y in zip(values,group['truth']))
        raw_seconds.append(raw['seconds_observation_only'])
    for t in totals.values():
        t['bit_accuracy'] = t['correct']/192
        t['net_correction_fraction'] = (t['recovered']-t['damaged'])/192
    return {'scope':'24 synthetic groups, descriptive capability pilot only', 'groups':24,'bits':192,
            'gates':'PASS: encoding, returned energies, exact and reduced optima',
            'arms':totals,'brier_raw':statistics.mean(brier),
            'reduction':{'first_order_fixed_per_group':first,'full_fixed_per_group':fixed,
                         'max_residual_component_per_group':residual,
                         'residual_enumerated_states':sum(counts),'full_enumerated_states':24*256,
                         'fully_fixed_groups':sum(n==8 for n in fixed)},
            'inference_seconds_observation_only':sum(raw_seconds),
            'limits':['Shared templates; no real-domain validation','No calibration/training',
                      'No equal-cost or wall-time superiority claim','Ultimate already includes presolve']}


if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('directory',type=Path)
    args=parser.parse_args()
    print(json.dumps(analyze(args.directory),indent=2,allow_nan=False))
