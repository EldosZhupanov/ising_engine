"""Run only the frozen LAYA-001 pilot; successful raw inference files are never repeated."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from pipeline import corpus, encode, verify_and_score

SNAPSHOT = '5e7b2b1b8ca2ecdd3f2322d94069c9b6ce7e844b'


def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def write_new(path, value):
    with Path(path).open('x') as f:
        json.dump(value, f, indent=2, allow_nan=False)
        f.write('\n')


def bridge_attempt(binary, request, directory, gid):
    """Retain request/stdout/stderr even if execution, parsing or verification fails."""
    attempt = 0
    while (directory / f'bridge_{gid:02d}_attempt_{attempt}.json').exists():
        attempt += 1
    started = time.perf_counter()
    try:
        completed = subprocess.run([str(binary.resolve())], input=json.dumps(request)+'\n',
                                   text=True, capture_output=True, timeout=120, check=False)
        evidence = {'request':request, 'returncode':completed.returncode,
                    'stdout':completed.stdout, 'stderr':completed.stderr, 'timed_out':False}
    except subprocess.TimeoutExpired as exc:
        def decoded(value):
            return value.decode(errors='replace') if isinstance(value, bytes) else (value or '')
        evidence = {'request':request, 'returncode':None, 'stdout':decoded(exc.stdout),
                    'stderr':decoded(exc.stderr), 'timed_out':True}
    except OSError as exc:
        evidence = {'request':request, 'returncode':None, 'stdout':'', 'stderr':str(exc),
                    'timed_out':False, 'launch_error':True}
    evidence['seconds_observation_only'] = time.perf_counter()-started
    write_new(directory/f'bridge_{gid:02d}_attempt_{attempt}.json', evidence)
    if evidence['returncode'] != 0:
        raise RuntimeError('bridge failed; retained attempt evidence')
    return json.loads(evidence['stdout']), evidence


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--bridge', required=True, type=Path)
    parser.add_argument('--snapshot', required=True, type=Path)
    parser.add_argument('--resume', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    commit = subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    dirty = subprocess.check_output(['git','status','--porcelain','--untracked-files=normal'],cwd=root,text=True)
    if dirty:
        raise RuntimeError('run from clean instrument checkout; output must be outside it')
    if not args.resume:
        args.output.mkdir(parents=True, exist_ok=False)
    groups = corpus()
    corpus_path = args.output / 'corpus.json'
    if corpus_path.exists():
        if json.loads(corpus_path.read_text()) != json.loads(json.dumps(groups)):
            raise RuntimeError('resume corpus mismatch')
    else:
        write_new(corpus_path, groups)
    if args.snapshot.name != SNAPSHOT:
        raise RuntimeError('unexpected checkpoint snapshot')
    bridge_hash = digest(args.bridge)
    prior = args.output / 'environment.json'
    if prior.exists():
        previous = json.loads(prior.read_text())
        if previous['commit'] != commit or previous['bridge_sha256'] != bridge_hash:
            raise RuntimeError('resume instrument mismatch')

    os.environ['HF_HUB_OFFLINE'] = '1'
    os.environ['TRANSFORMERS_OFFLINE'] = '1'
    os.environ['TOKENIZERS_PARALLELISM'] = 'false'
    import torch
    import laya

    if laya.__version__ != '0.3.7':
        raise RuntimeError('unexpected Laya version')
    torch.set_num_threads(2)
    torch.manual_seed(23)
    local = args.output / 'checkpoint'
    if not local.exists():
        local.mkdir()
        for name in ('tokenizer','encoder'):
            shutil.copytree(args.snapshot/name, local/name)
        shutil.copy2(args.snapshot/'rl_agent_config.json',local/'rl_agent_config.json')
        (local/'model.safetensors').symlink_to((args.snapshot/'model.safetensors').resolve())
    started = time.perf_counter()
    agent = laya.load(str(local), device='cpu')
    package = Path(laya.__file__).parent
    environment = {
        'commit':commit, 'python':sys.version, 'torch':torch.__version__, 'laya':laya.__version__,
        'device':str(agent.device), 'threads':2, 'snapshot':SNAPSHOT,
        'weights_sha256':digest(local/'model.safetensors'), 'bridge_sha256':bridge_hash,
        'package_sha256':{p.name:digest(p) for p in sorted(package.glob('*.py'))},
        'metadata_sha256':{str(p.relative_to(local)):digest(p) for p in sorted(local.rglob('*.json'))},
        'load_seconds_observation_only':time.perf_counter()-started,
        'protocol_sha256':digest(root/'research/laya_semantic/PROTOCOL.md'),
    }
    if not prior.exists():
        write_new(prior,environment)
    else:
        for key in ('weights_sha256','package_sha256','metadata_sha256','python','torch'):
            if environment[key] != previous[key]:
                raise RuntimeError('resume environment mismatch: '+key)
    for group in groups:
        gid = group['id']
        raw_path = args.output/f'raw_{gid:02d}.json'
        case_path = args.output/f'case_{gid:02d}.json'
        if case_path.exists():
            continue
        if not raw_path.exists():
            questions = {f'q{i}':{'type':'noul','instructions':message+' Is the operation running now?'}
                         for i,message in enumerate(group['messages'])}
            started = time.perf_counter()
            result = agent.predict('Manufacturing status classification.',questions)
            write_new(raw_path, {'group_id':gid,'questions':questions,'result':result,
                                 'seconds_observation_only':time.perf_counter()-started})
        raw = json.loads(raw_path.read_text())
        if raw['group_id'] != gid:
            raise RuntimeError('raw group mismatch')
        values = [raw['result']['answers'][f'q{i}']['noul'] for i in range(8)]
        request, penalty = encode(values,group['rules'],1000+gid)
        started = time.perf_counter()
        bridge, evidence = bridge_attempt(args.bridge, request, args.output, gid)
        scores = verify_and_score(group,values,bridge)
        write_new(case_path, {'group_id':gid,'probabilities':values,'request':request,
                              'penalty':penalty,'bridge':bridge,'scores':scores,
                              'bridge_seconds_observation_only':time.perf_counter()-started,
                              'bridge_stderr':evidence['stderr']})
        print(f'group {gid+1}/24: encoding and optimum checks PASS',flush=True)
    write_new(args.output/'complete.json',{'groups':24,'commit':commit,'status':'complete'})


if __name__ == '__main__': main()
