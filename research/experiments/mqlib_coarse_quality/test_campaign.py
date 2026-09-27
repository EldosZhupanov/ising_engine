"""Fabricated regressions only: no qualification optimizer invocation."""
import copy
import json
import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import campaign as c
import check
import freeze


def record(cell,energy=0,receipt=.2,validated=.3):
    return dict(cell=cell,status='VALID',kill_seconds=2.001,fallback_energy=0,
                events=[dict(verified_energy=energy,received_seconds=receipt,validated_seconds=validated)])


class InstrumentTests(unittest.TestCase):
    def setUp(self):self.m=freeze.manifest()

    def test_schedule_matches_independent(self):
        for phase,n in [('admission',20),('main',960)]:
            cells=list(c.schedule(self.m,phase));self.assertEqual(len(cells),n)
            self.assertEqual(cells,check.expected(self.m,phase))
            self.assertEqual(len({tuple(x.items()) for x in cells}),n)
        main=list(c.schedule(self.m,'main'))
        self.assertEqual(main[4]['arm'],'v2_default')
        self.assertEqual(main[40]['arm'],'v2_default')

    def test_strict_deadline_and_validation(self):
        for received,validated,count in [(1.9,1.99,1),(1.99,2.,0),(2.,2.,0),(2.1,2.2,0)]:
            r=record({},-8,received,validated)
            self.assertEqual(c.score(r),(-8 if count else 0,count))
            self.assertEqual(c.score(r),check.endpoint(r))

    def test_absent_partial_fallback(self):
        r=record({});r['events']=[];r['partial_base64']='e30='
        self.assertEqual(c.score(r),(0,0))

    def test_admission_and_failed_witness(self):
        rs=[record(x) for x in c.schedule(self.m,'admission')]
        self.assertEqual(c.analyze(rs,self.m,'admission')['status'],'PASS')
        rs[0]['events'][0]['validated_seconds']=2
        self.assertEqual(c.analyze(rs,self.m,'admission')['status'],'FAIL')
        rs[0]['events']=[]
        self.assertEqual(c.analyze(rs,self.m,'admission')['status'],'FAIL')

    def test_duplicate_probe_witness_rejected(self):
        rs=[record(x) for x in c.schedule(self.m,'admission')]
        rs[0]['events']*=2
        self.assertEqual(c.analyze(rs,self.m,'admission')['status'],'FAIL')

    def test_schedule_invalid_duplicate_missing(self):
        rs=[record(x) for x in c.schedule(self.m,'admission')]
        for bad in [rs[:-1],[rs[0]]+rs[:-1]]:
            self.assertEqual(c.analyze(bad,self.m,'admission')['status'],'NO_VERDICT')
        rs[0]['status']='INVALID'
        self.assertEqual(c.analyze(rs,self.m,'admission')['status'],'NO_VERDICT')

    def test_kill_not_cutoff_bound(self):
        rs=[record(x) for x in c.schedule(self.m,'admission')]
        rs[0]['cutoff_seconds']=2.0001;rs[0]['kill_seconds']=2.10001
        self.assertFalse(c.valid_records(rs,self.m,'admission'))
        rs[0]['kill_seconds']=math.nextafter(2.1,math.inf)
        self.assertFalse(c.valid_records(rs,self.m,'admission'))
        rs[0]['kill_seconds']=2.1
        self.assertTrue(c.valid_records(rs,self.m,'admission'))

    def test_main_gates_at_registered_counts(self):
        rs=[record(x) for x in c.schedule(self.m,'main')]
        # Eight informative cases, four unique wins per arm, exactly .001L.
        for r in rs:
            i=int(r['cell']['instance'][1:]);arm=r['cell']['arm']
            if i<8 and arm==c.ARMS[i//4]:
                r['events'][0]['verified_energy']=-.001*self.m['instances'][i]['normalization_L']
        out=c.analyze(rs,self.m,'main')
        self.assertEqual(out['informative_instances'],8)
        self.assertEqual(out['H1'],'SUPPORTED_ON_QUALIFICATION')
        self.assertEqual(out['H2'],'SUPPORTED_ON_QUALIFICATION')
        self.assertEqual(out['unique_wins'],dict(ultimate=4,v2_default=4,mqlib_merz=0,mqlib_mst2=0))
        for r in rs:
            if r['cell']['instance']=='q07':r['events'][0]['verified_energy']=0
        out=c.analyze(rs,self.m,'main')
        self.assertEqual(out['informative_instances'],7)
        self.assertEqual(out['H1'],'INCONCLUSIVE');self.assertEqual(out['H2'],'NOT_ESTABLISHED')

    def test_admission_refusal_and_stale_review(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)
            values={'audit':dict(status='PASS',artifact_hashes={}), 'review':dict(status='PASS'),
                    'metadata':dict(provenance={'x':1}), 'terminal':dict(status='COMPLETE',elapsed_seconds=41),
                    'analysis':dict(status='PASS')}
            for k,v in values.items():(p/(k+'.json')).write_text(json.dumps(v))
            def review(h): (p/'review.json').write_text(json.dumps(dict(status='PASS',audit_sha256=h)))
            review(freeze.sha(p/'audit.json'));self.assertEqual(c.admitted(p,{'x':1}),41)
            with self.assertRaises(ValueError):c.admitted(p,{'x':2})
            review('stale')
            with self.assertRaises(ValueError):c.admitted(p,{'x':1})
            review(freeze.sha(p/'audit.json'));(p/'analysis.json').write_text('{"status":"FAIL"}')
            with self.assertRaises(ValueError):c.admitted(p,{'x':1})

    def test_cap_abort_retains_guard_and_artifacts(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td);out=root/'run';m=copy.deepcopy(self.m);m['wall_cap_seconds']=0
            with patch.object(c,'HERE',root),patch.object(freeze,'manifest',return_value=m),patch.object(freeze,'verify',return_value={}),patch.object(freeze,'git',return_value=''),patch.object(c.os,'sched_setaffinity'),patch.object(c.os,'sched_getaffinity',return_value={0}),patch.object(c.sys,'argv',['campaign','--phase','admission','--output',str(out)]):
                c.main()
                self.assertTrue((root/'.admission_started').exists())
                self.assertEqual(json.loads((out/'terminal.json').read_text())['status'],'ABORT')
                self.assertEqual(json.loads((out/'analysis.json').read_text())['status'],'NO_VERDICT')
                self.assertFalse(list(out.glob('cell*')))
                with self.assertRaises(FileExistsError):c.main()

    def test_registration_and_source_drift(self):
        with tempfile.TemporaryDirectory() as td:
            cache=Path(td);(cache/'freeze.json').write_text(json.dumps(dict(registration_commit='bad',sources={},native_build={})))
            with patch.object(freeze,'CACHE',cache):
                with self.assertRaises(ValueError):freeze.verify()
            (cache/'freeze.json').write_text(json.dumps(dict(registration_commit=freeze.PREREG,sources={},native_build={})))
            with patch.object(freeze,'CACHE',cache),patch.object(freeze,'inputs',return_value={'changed':'hash'}):
                with self.assertRaises(ValueError):freeze.verify()

    def test_independent_duplicate_json_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/'x.json';p.write_text('{"energy":1,"energy":2}')
            with self.assertRaisesRegex(AssertionError,'duplicate JSON key'):check.read(p)


if __name__=='__main__':unittest.main()
