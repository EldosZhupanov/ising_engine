"""Generate fresh, outcome-blind Q002 inputs from the frozen C001 generator."""
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
GENERATOR = HERE.parent / 'hubo_comparison/generate_cases.py'
GENERATOR_SHA = 'b7279ec51f4c328a618ffe3752d7651e1723e46a36385b434d21502ec3ff864f'
STRATA = [(64, 3, 961001), (64, 4, 962001), (128, 3, 963001), (128, 4, 964001)]


def prepared():
    if hashlib.sha256(GENERATOR.read_bytes()).hexdigest() != GENERATOR_SHA:
        raise ValueError('frozen generator changed')
    spec = importlib.util.spec_from_file_location('c001_input_generator', GENERATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    cases = []
    for n, degree, first in STRATA:
        for seed in range(first, first + 3):
            case = module.make_case(degree, seed, n)
            case['id'] = f'n{n}_d{degree}_{seed}'
            cases.append(case)
    return cases


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


if __name__ == '__main__':
    output = HERE / 'cases.json'
    manifest = HERE / 'inputs.json'
    if output.exists() or manifest.exists():
        raise SystemExit('refuse to overwrite registered inputs')
    cases = prepared()
    data = encoded(cases)
    output.write_bytes(data)
    manifest.write_text(json.dumps({
        'generator_sha256': GENERATOR_SHA,
        'cases_sha256': hashlib.sha256(data).hexdigest(),
        'solver_seeds': list(range(970001, 970011)),
        'scope': 'fresh synthetic qualification data; no optimum or solver outcomes accessed',
        'instances': [{k: case[k] for k in ['id', 'n', 'degree', 'instance_seed', 'normalizer']}
                      | {'sha256': hashlib.sha256(encoded(case)).hexdigest()}
                      for case in cases],
    }, indent=2) + '\n')
