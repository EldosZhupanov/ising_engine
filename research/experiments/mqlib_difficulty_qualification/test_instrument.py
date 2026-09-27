import base64
import hashlib
import json
from pathlib import Path
import tempfile
import os
import sys
import time
import unittest
from unittest.mock import patch
import analysis as a
import independent_audit as auditor
import build
import run

MODEL={'offset':3,'linear':[2,-3],'pairs':[[0,1,-4]]}


def record(arm='ultimate',mode='search'):
    return {'cell':{'arm':arm,'mode':mode,'seed':61101},'events':[],'errors':[], 'ready':1,'configs':0,'fallback_energy':3}


class InstrumentTests(unittest.TestCase):
    def test_raw_energy_exhaustive_and_export(self):
        text=run.mq.export_qubo(MODEL).splitlines()[1:]
        coeff=[(int(i)-1,int(j)-1,float(w)) for i,j,w in (x.split() for x in text)]
        for mask in range(4):
            x=[mask&1,(mask>>1)&1]
            native=sum(w*x[i]*x[j]*(1 if i==j else 2) for i,j,w in coeff)
            self.assertEqual(MODEL['offset']-native, run.energy(MODEL,x))

    def test_invalid_states_and_energy_rejected(self):
        for event in [dict(kind='inc',state=[True,0],energy=3),dict(kind='inc',state=[0],energy=3),dict(kind='inc',state=[0,0],energy=4),dict(kind='inc',state=[0,0],energy=float('inf'))]:
            with self.assertRaises(ValueError):run.validate(MODEL,event,record())

    def test_validation_cost_and_exact_cutoff(self):
        for receipt,completed,want in [(.01,.1,3),(.01,.099,-2),(.1,.1,3),(.11,.12,3)]:
            r=record();run.consume(MODEL,b'{"kind":"inc","state":[1,1],"energy":-2,"chunk":0}',receipt,r,0,clock=lambda:completed)
            self.assertFalse(r['errors']);self.assertEqual(a.checkpoint(r,.1)[0],want)

    def test_duplicate_nonfinite_and_oversized(self):
        for line in [b'{"kind":"inc","kind":"inc"}',b'{"kind":"inc","state":[0,0],"energy":NaN}',b'x'*(2**20+1),b'\xff',b'[]']:
            r=record();run.consume(MODEL,line,.01,r,0,clock=lambda:.02)
            self.assertTrue(r['errors']);json.dumps(r,allow_nan=False)

    def test_ready_and_config_invariants(self):
        for ev in [dict(kind='ready',affinity='0-3'),dict(kind='inc',state=[0,0],energy=3)]:
            r=record();r['ready']=0
            if ev['kind']=='inc' or ev['affinity']!='0':
                with self.assertRaises(ValueError):run.validate(MODEL,ev,r)
        r=record('v2_default')
        with self.assertRaises(ValueError):run.validate(MODEL,dict(kind='inc',state=[0,0],energy=3,chunk=0),r)

    def test_schedule_counts_balance(self):
        m=build.manifest();main=list(a.schedule(m,'main'));pre=list(a.schedule(m,'preflight'))
        self.assertEqual(len(main),960);self.assertEqual(len(pre),512)
        for arm in a.ARMS:self.assertEqual(sum(c['arm']==arm for c in main),240)
        self.assertEqual(len({json.dumps(c,sort_keys=True) for c in main}),960)

    def test_missing_duplicate_and_corrupt_schedule(self):
        m=build.manifest();r=[dict(cell=c,status='VALID') for c in a.schedule(m,'main')]
        self.assertTrue(a.verify_schedule(r,m,'main'))
        self.assertFalse(a.verify_schedule(r[:-1],m,'main'))
        self.assertFalse(a.verify_schedule([r[0]]+r[:-1],m,'main'))
        r[0]['status']='INVALID';self.assertFalse(a.verify_schedule(r,m,'main'))

    def test_bootstrap_deterministic_indices(self):
        one=next(a.bootstrap_indices());self.assertEqual(one,next(a.bootstrap_indices()));self.assertEqual(len(one),24)
        digest=hashlib.sha256(b'MQ-DIFFICULTY-001/bootstrap/v1/0').digest()
        self.assertEqual(one[0],int.from_bytes(digest[:8],'big')%24)

    def test_worker_policy_preservation(self):
        original=(build.ROOT/'research/examples/mqlib_compare.rs').read_text()
        new=(build.ROOT/'research/examples/mqlib_difficulty.rs').read_text()
        # rustfmt-only whitespace aside, model preparation and the full search loop stay identical.
        compact=lambda s:''.join(s.split())
        self.assertEqual(compact(original[original.index('fn prepare'):original.index('fn main')]),compact(new[new.index('fn prepare'):new.index('fn main')]))
        self.assertEqual(compact(original[original.index('    for chunk in'):original.index('\n#[cfg(test)]')]),compact(new[new.index('    for chunk in'):new.index('\n#[cfg(test)]')]))

    def mock_cell(self, source, budget=.25, extra_patch=None):
        old=os.sched_getaffinity(0)
        try:
            os.sched_setaffinity(0,{0})
            with tempfile.TemporaryDirectory() as tmp:
                cell=dict(arm='ultimate',mode='search',seed=61101,budget=budget,instance='fake',block='test')
                if extra_patch:
                    with extra_patch:r=run.run_cell(MODEL,cell,Path(tmp)/'cell',[sys.executable,'-c',source])
                else:r=run.run_cell(MODEL,cell,Path(tmp)/'cell',[sys.executable,'-c',source])
                raw=(Path(tmp)/'cell/stdout.bin').read_bytes()
                reconstructed=b''.join(base64.b64decode(e['raw_base64'])+b'\n' for e in r['events'])+base64.b64decode(r['partial_base64'])
                self.assertEqual(raw,reconstructed)
                return r
        finally:os.sched_setaffinity(0,old)

    def test_mock_late_validation_and_killed_partial_retained(self):
        original=run.consume
        def slow(*args,**kwargs):
            original(*args,**kwargs)
            if args[1].startswith(b'{"kind":"ready"'):time.sleep(.30)
        source='import sys,time;sys.stdout.write(\'{"kind":"ready","affinity":"0"}\\n{"kind":"inc","state":[1,1],"energy":-2,"chunk":0}\\npartial\');sys.stdout.flush();time.sleep(10)'
        r=self.mock_cell(source,extra_patch=patch.object(run,'consume',side_effect=slow))
        self.assertEqual(r['status'],'VALID');self.assertEqual(a.checkpoint(r,.25)[1],0)
        self.assertEqual(base64.b64decode(r['partial_base64']),b'partial');self.assertIn('reap_seconds',r)

    def test_mock_early_exit_and_overflow_preserve_evidence(self):
        sources=['print(\'{"kind":"ready","affinity":"0"}\',flush=True)',
                 'import sys,time;sys.stdout.write("x"*(2**20+65536));sys.stdout.flush();time.sleep(10)']
        for source in sources:
            r=self.mock_cell(source, budget=1)
            self.assertEqual(r['status'],'INVALID');self.assertIn('kill_seconds',r);self.assertIn('exit_code',r)
            self.assertGreater(r['stdout_bytes'],0)

    def test_mock_affinity_error(self):
        r=self.mock_cell('import time;time.sleep(10)',extra_patch=patch.object(run.os,'sched_getaffinity',return_value={1}))
        self.assertEqual(r['status'],'INVALID');self.assertIn('parent affinity mismatch',r['errors'])

    def test_cap_abort_retains_terminal_and_cannot_restart(self):
        with tempfile.TemporaryDirectory() as tmp:
            here=Path(tmp);out=here/'output';m=build.manifest();m['wall_cap_seconds']=0
            with patch.object(run,'HERE',here),patch.object(build,'manifest',return_value=m),patch.object(build,'verify',return_value={}),patch.object(run,'corpus',return_value={}),patch.object(build,'git',return_value=''),patch.object(sys,'argv',['run.py','--phase','preflight','--output',str(out)]):
                old=os.sched_getaffinity(0)
                try:
                    run.main()
                    with self.assertRaises(FileExistsError):run.main()
                finally:os.sched_setaffinity(0,old)
            self.assertEqual(json.loads((out/'terminal.json').read_text())['status'],'ABORT')
            self.assertEqual(json.loads((out/'analysis.json').read_text())['status'],'NO_VERDICT')

    def test_auditor_rejects_unregistered_manifest_and_dev_build(self):
        m=build.manifest()
        with self.assertRaises(AssertionError):auditor.provenance({'manifest':{**m,'budget_seconds':20},'build':{'development':False}})
        with self.assertRaises(AssertionError):auditor.provenance({'manifest':m,'build':{'development':True}})
        with self.assertRaises(AssertionError):auditor.provenance({'manifest':m,'build':{'development':False,'source_hashes':{}}})

    def test_auditor_schemas_duplicates_and_absolute_tolerance(self):
        with self.assertRaises(AssertionError):json.loads('{"kind":1,"kind":2}',object_pairs_hook=auditor.unique)
        with self.assertRaises(AssertionError):auditor.message_schema({'kind':'ready','affinity':'0','extra':0},{'arm':'ultimate','mode':'search'})
        with self.assertRaises(AssertionError):auditor.verify_reported_energy(100000,{'energy':100000.00000001},False,0)
        auditor.verify_reported_energy(100000,{'energy':100000},False,0)

    def test_one_shot_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'r';run.write(p,{'old':1})
            with self.assertRaises(FileExistsError):run.write(p,{'new':2})
            self.assertEqual(json.loads(p.read_text()),{'old':1})

    def test_inherited_source_drift(self):
        with patch.object(build,'sha',return_value='wrong'):
            with self.assertRaises(ValueError):build.inputs(development=True)

    def test_preflight_missing_output_fails(self):
        m=build.manifest();rs=[]
        for c in a.schedule(m,'preflight'):
            rs.append(dict(cell=c,status='VALID',events=[],fallback_energy=0,cutoff_seconds=.25,configs=1))
        self.assertEqual(a.preflight(rs,m)['status'],'FAIL')

    def test_analysis_ties_and_complementarity(self):
        m=build.manifest();rs=[]
        for c in a.schedule(m,'main'):
            rs.append(dict(cell=c,status='VALID',events=[],fallback_energy=0))
        with patch.object(a,'bootstrap_indices',return_value=iter([list(range(24))]*10000)):
            result=a.main_analysis(rs,m)
        self.assertEqual(result['informative_instances'],0);self.assertEqual(result['H2'],'NOT_ESTABLISHED')
        # Eight discriminating instances, four wins for each of two arms, exact threshold.
        for r in rs:
            idx=int(r['cell']['instance'][1:]);arm='ultimate' if idx<4 else 'mqlib_mst2'
            if idx<8 and r['cell']['arm']==arm:
                scale=m['instances'][idx]['normalization_L']
                r['events']=[dict(received_seconds=.01,validated_seconds=.02,verified_energy=-scale/1000)]
        with patch.object(a,'bootstrap_indices',return_value=iter([list(range(24))]*10000)):
            result=a.main_analysis(rs,m)
        self.assertEqual(result['informative_instances'],8)
        self.assertEqual(result['H1'],'SUPPORTED_ON_QUALIFICATION');self.assertEqual(result['H2'],'SUPPORTED_ON_QUALIFICATION')


if __name__=='__main__':unittest.main()
