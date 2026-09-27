"""Strictness and corruption tests; no actual solver-quality experiment."""
import copy
from fractions import Fraction
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('adapter', ROOT/'benchmarks/adapters/run_mqlib.py')
a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a)
from fixtures import cases


def parse_export(text, offset):
    """Independent interpretation of MQLib matrix file back to a polynomial."""
    lines = text.splitlines()
    n, count = map(int, lines[0].split())
    if count != len(lines)-1:
        raise ValueError('bad line count')
    linear, pairs, seen = [0.0]*n, [], set()
    for line in lines[1:]:
        i, j, v = line.split()
        i, j, v = int(i)-1, int(j)-1, float(v)
        if not 0 <= i <= j < n or (i,j) in seen:
            raise ValueError('bad matrix indices')
        seen.add((i,j))
        if i == j:
            linear[i] = -v
        else:
            pairs.append([i,j,-2*v])
    return {'offset':offset, 'linear':linear, 'pairs':pairs}


def verify_export(model, text, offset):
    if parse_export(text, offset) != model:
        raise ValueError('coefficient/offset roundtrip mismatch')


class AdapterTests(unittest.TestCase):
    def test_export_roundtrip_and_all_states(self):
        for model in cases():
            restored = parse_export(a.export_qubo(model),model['offset'])
            self.assertEqual(restored,model)
            for mask in range(1 << len(model['linear'])):
                state = [(mask>>i)&1 for i in range(len(model['linear']))]
                self.assertEqual(a.exact_energy(model,state),a.exact_energy(restored,state))

    def test_sign_factor_offset_mutations_are_detected(self):
        model = cases()[3]
        exported = a.export_qubo(model)
        verify_export(model,exported,model['offset'])
        for mutation in ('sign','factor','offset'):
            lines=exported.splitlines()
            if mutation=='sign':
                lines[1]='1 1 0.5'  # should be -0.5
            if mutation=='factor':
                lines[-2]='1 2 -2.5'  # should be -1.25
            offset=0 if mutation=='offset' else model['offset']
            with self.subTest(mutation=mutation),self.assertRaises(ValueError):
                verify_export(model,'\n'.join(lines)+'\n',offset)

    def test_bad_models_rejected(self):
        bad = [None, {}, {'offset':0,'linear':[],'pairs':[]}]
        for key,value in [('offset',10**1000),('offset',float('nan')),('linear',[True]),('linear',[float('inf')]),
                          ('pairs',[[0,1,1],[0,1,2]]),('pairs',[[1,0,1]]),
                          ('pairs',[[0,0,1]]),('pairs',[[0,4,1]]),
                          ('pairs',[[True,2,1]]),('pairs',[[0,1,5e-324]])]:
            m=copy.deepcopy(cases()[3]);m[key]=value;bad.append(m)
        m=copy.deepcopy(cases()[3]);m['answer']=[0]*4;bad.append(m)
        for model in bad:
            with self.subTest(model=model), self.assertRaises(ValueError): a.validate_model(model)

    def test_duplicate_json_keys_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'input.json';p.write_text('{"offset":0,"offset":1,"linear":[1],"pairs":[]}')
            with self.assertRaises(ValueError): a.load_model(p)

    def test_output_exact_energy_includes_offset(self):
        out='0.02,MERZ2002ONEOPT,"fixture",3,0.02,[0:0;3:0.01]\n\nSolution:\n0 1 0\n'
        result=a.parse_output(out,cases()[1],0.02,'fixture')
        self.assertEqual(result['energy'],-5)
        self.assertEqual(result['energy_exact'],'-5')

    def test_output_corruption_rejected(self):
        valid='0.02,MERZ2002ONEOPT,"fixture",3,0.02,[]\nSolution:\n0 1 0\n'
        bad=[valid.replace('0 1 0','0 1'),valid.replace('0 1 0','0 1 0 0'),
             valid.replace('0 1 0','0 2 0'),valid.replace('0 1 0','0 1.0 0'),
             valid.replace(',3,',',4,'),valid.replace(',3,',',nan,'),
             valid.replace(',0.02,[]',',-1,[]'),valid.replace(',0.02,[]',',inf,[]'),
             valid.replace('fixture','another'),valid.replace('MERZ2002ONEOPT','HH_fake'),
             valid.replace('0.02,MERZ','0.03,MERZ'),valid+'Solution:\n0 1 0\n',
             'Error: invalid history!\n'+valid]
        for out in bad:
            with self.subTest(out=out),self.assertRaises(ValueError): a.parse_output(out,cases()[1],.02,'fixture')

    def test_seed_and_time_validation(self):
        for seed,seconds,hard in [(-1,.02,5),(65536,.02,5),(True,.02,5),(1,0,5),
                                  (1,float('nan'),5),(1,.02,.01),(1,.02,float('inf'))]:
            with self.assertRaises(ValueError): a.run(cases()[0],'/missing',seed,seconds,hard)

    def test_process_failure_preserves_raw_output(self):
        with patch.object(a.subprocess,'run',return_value=subprocess.CompletedProcess([],3,'bad','err')):
            result=a.run(cases()[0],__file__,101,.02)
        self.assertEqual(result['status'],'INVALID')
        self.assertEqual(result['stdout'],'bad')
        self.assertEqual(result['exit_code'],3)

    def test_timeout_cannot_be_success(self):
        with patch.object(a.subprocess,'run',side_effect=subprocess.TimeoutExpired([],5,output=b'partial')):
            result=a.run(cases()[0],__file__,101,.02)
        self.assertEqual(result['status'],'TIMEOUT')
        self.assertEqual(result['stdout'],'partial')

    def test_private_tempfiles_and_explicit_seed(self):
        paths=[]
        def fake(cmd,**kwargs):
            path=cmd[cmd.index('-fQ')+1];paths.append(path)
            self.assertEqual(cmd[cmd.index('-s')+1],'101')
            self.assertTrue(Path(path).exists())
            return subprocess.CompletedProcess(cmd,0,f'0.02,MERZ2002ONEOPT,"{path}",0,0.02,[]\nSolution:\n0\n','')
        with patch.object(a.subprocess,'run',side_effect=fake):
            for _ in range(2): self.assertEqual(a.run(cases()[0],__file__,101,.02)['status'],'VALID')
        self.assertNotEqual(paths[0],paths[1])
        self.assertFalse(any(Path(p).exists() for p in paths))

    def test_cli_failure_is_retained_and_output_is_exclusive(self):
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp)/'failure.json'
            cmd=[sys.executable,str(ROOT/'benchmarks/adapters/run_mqlib.py'),
                 str(Path(tmp)/'missing.json'),'--binary','/missing','--seed','101',
                 '--seconds','0.02','--output',str(out)]
            first=subprocess.run(cmd,capture_output=True)
            self.assertEqual(first.returncode,2)
            saved=out.read_bytes()
            self.assertIn(b'INVALID',saved)
            second=subprocess.run(cmd,capture_output=True)
            self.assertNotEqual(second.returncode,0)
            self.assertEqual(out.read_bytes(),saved)

    def test_oracle_timeout_and_launch_failure_are_records(self):
        from qualify import run_oracle
        with patch('qualify.subprocess.run',side_effect=subprocess.TimeoutExpired(['oracle'],5,output=b'0 1\n',stderr=b'partial')):
            record=run_oracle(Path('/oracle'),Path('/fixture'))
        self.assertEqual(record['status'],'TIMEOUT')
        self.assertEqual(record['stdout'],'0 1\n')
        self.assertEqual(record['stderr'],'partial')
        self.assertIsNone(record['exit_code'])
        self.assertEqual(record['command'],['/oracle','/fixture'])
        with patch('qualify.subprocess.run',side_effect=OSError('missing')):
            record=run_oracle(Path('/oracle'),Path('/fixture'))
        self.assertEqual(record['status'],'INVALID')


if __name__ == '__main__': unittest.main()
