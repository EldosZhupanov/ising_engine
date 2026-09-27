import copy
import json
from pathlib import Path
import sys
import unittest

from campaign import consume, energy, run_cell, verify_event, analyze, ARMS


class ScreenTests(unittest.TestCase):
    def model(self): return {'offset':7,'linear':[-2,3],'pairs':[[0,1,-4]]}
    def record(self): return {'events':[],'errors':[],'witnesses_before_deadline':0,'best_energy':7,'best_state':[0,0]}

    def test_cutoff_is_strict_and_late_is_retained(self):
        r=self.record();line=json.dumps({'kind':'inc','state':[1,0],'energy':5})
        consume(self.model(),'ultimate',line,2.0,2.0,r)
        self.assertEqual(r['best_energy'],7);self.assertEqual(len(r['events']),1)
        consume(self.model(),'ultimate',line,1.99,2.0,r)
        self.assertEqual(r['best_energy'],5)
        self.assertEqual(r['witnesses_before_deadline'],1)

    def test_mqlib_sign_offset_and_corruption(self):
        event={'kind':'inc','state':[1,0],'objective':2}
        self.assertEqual(verify_event(self.model(),event,'mqlib'),5)
        for state,objective in [([1],2),([2,0],2),([1,0],3),([1,0],float('nan')),([True,0],2)]:
            with self.assertRaises(ValueError): verify_event(self.model(),{'kind':'inc','state':state,'objective':objective},'mqlib')

    def test_malformed_output_is_failure_not_loss(self):
        r=self.record();consume(self.model(),'ultimate','not json',.1,2,r)
        self.assertTrue(r['errors']);self.assertEqual(r['best_energy'],7)

    def test_real_process_deadline_and_partial_retention(self):
        script='import sys,time; print(\'{"kind":"inc","state":[1,0],"energy":5}\',flush=True); sys.stdout.write("partial"); sys.stdout.flush(); time.sleep(10)'
        r=run_cell(self.model(),'ultimate',501,.15,0,[sys.executable,'-c',script])
        self.assertEqual(r['status'],'VALID');self.assertTrue(r['killed_at_deadline'])
        self.assertEqual(r['best_energy'],5);self.assertEqual(r['partial_output'],'partial')
        self.assertLess(r['total_seconds'],2)

    def test_failed_process_stops_validity(self):
        r=run_cell(self.model(),'ultimate',501,.15,0,[sys.executable,'-c','raise SystemExit(3)'])
        self.assertEqual(r['status'],'INVALID')

    def test_premature_eof_is_not_deadline(self):
        r=run_cell(self.model(),'ultimate',501,.15,0,[sys.executable,'-c','import os,time;os.close(1);time.sleep(10)'])
        self.assertEqual(r['status'],'INVALID');self.assertFalse(r['killed_at_deadline'])
        r=run_cell(self.model(),'ultimate',501,.15,0,[sys.executable,'-c','print("partial",end="")'])
        self.assertEqual(r['status'],'INVALID')

    def test_bad_v2_config_rejected(self):
        with self.assertRaises(ValueError): verify_event(self.model(),{'kind':'config'},'v2_default')

    def fixture_records(self):
        manifest={'instances':[{'name':str(i),'normalizer':10} for i in range(8)],'search_seeds':list(range(10))}
        records=[{'instance':i['name'],'seed':s,'arm':a,'best_energy':-1 if a=='ultimate' else 0,
                  'status':'VALID','fallback_only':False} for i in manifest['instances'] for s in manifest['search_seeds'] for a in ARMS]
        return records,manifest

    def test_statistics_use_instances_and_known_sign_flip(self):
        r,m=self.fixture_records();summary=analyze(r,m)
        self.assertEqual(summary['primary']['sign_flip_p'],2/256)
        self.assertEqual(len(summary['primary']['instance_contrasts']),8)
        self.assertEqual(summary['primary']['H1'],'SUPPORTED_ON_THIS_SCREEN')
        self.assertEqual(summary['optimistic_diagnostic']['best_fixed_minus_oracle'],0)

    def test_missing_duplicate_invalid_cells_no_verdict(self):
        r,m=self.fixture_records()
        self.assertEqual(analyze(r[:-1],m)['status'],'NO_VERDICT')
        self.assertEqual(analyze(r+[r[0]],m)['status'],'NO_VERDICT')
        r[0]['status']='INVALID';self.assertEqual(analyze(r,m)['status'],'NO_VERDICT')

    def test_null_has_no_success(self):
        r,m=self.fixture_records()
        for row in r: row['best_energy']=0
        s=analyze(r,m)
        self.assertEqual(s['primary']['sign_flip_p'],1)
        self.assertEqual(s['primary']['H1'],'INCONCLUSIVE')


if __name__=='__main__': unittest.main()
