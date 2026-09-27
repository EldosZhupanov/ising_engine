"""Fabricated streams/processes only; never executes an optimizer."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import qualify as q

MODEL={'offset':3.25,'linear':[.5,-1.25,0.,0.],'pairs':[[0,1,2.5],[1,2,-.75]]}


def callback(i=1,keep=False,elapsed=1):
    return {'kind':'callback','index':i,'state':[0,0,0,0],'objective':0.,'continue':keep,'elapsed_ns':elapsed,'upstream_runtime':.001,'affinity':'0'}


def final(count=1):
    return {'kind':'final','state':[0,0,0,0],'objective':0.,'callbacks':count,'stop_seen':True}


def raw(events): return ''.join(json.dumps(e)+'\n' for e in events).encode()


class QualificationTests(unittest.TestCase):
    def test_failure_has_priority_over_campaign_cap(self):
        self.assertEqual(q.result_status('failure',False,True),'FAIL')
        self.assertEqual(q.result_status('cap',False,True),'INCOMPLETE')
        self.assertEqual(q.result_status(None,True,True),'INCOMPLETE')
        self.assertEqual(q.result_status(None,True,False),'PASS')

    def test_oversized_number_is_validation_failure(self):
        self.assertFalse(q.number(10**1000))
        e=callback();e['objective']=10**1000
        with self.assertRaises(ValueError): q.validate(MODEL,'first',raw([e,final()]),0,False)

    def test_first_latched_allows_second_callback(self):
        data=[callback(),callback(2),final(2)]
        self.assertEqual(len(q.validate(MODEL,'first',raw(data),0,False)['callbacks']),2)

    def test_third_continuation(self):
        data=[callback(1,True),callback(2,True),callback(3),final(3)]
        self.assertEqual(q.validate(MODEL,'third',raw(data),0,False)['best_energy_exact'],'13/4')

    def test_deadline_uses_own_clock(self):
        a=callback(1,True,19999999);a['upstream_runtime']=100.
        b=callback(2,False,20000000)
        self.assertIsNotNone(q.validate(MODEL,'deadline',raw([a,b,final(2)]),0,False)['final'])
        a['continue']=False
        with self.assertRaises(ValueError): q.validate(MODEL,'deadline',raw([a,b,final(2)]),0,False)

    def test_watchdog_retains_partial(self):
        r=q.validate(MODEL,'watchdog',raw([callback(1,True)])+b'{incomplete',-9,True)
        self.assertEqual(bytes.fromhex(r['partial_hex']),b'{incomplete')
        with self.assertRaises(ValueError): q.validate(MODEL,'watchdog',raw([callback(1,True)]),0,False)

    def test_cooperative_timeout_rejected(self):
        with self.assertRaises(ValueError): q.validate(MODEL,'first',raw([callback()]),-9,True)

    def test_bad_state_and_energy(self):
        for key,value in [('state',[0,0]),('state',[True,0,0,0]),('state',[2,0,0,0]),('objective',1.),('objective',float('nan')),('extra',1)]:
            e=callback();e[key]=value
            with self.assertRaises(ValueError): q.validate(MODEL,'first',raw([e,final()]),0,False)

    def test_sign_pair_factor_offset_mutations(self):
        x=[1,1,0,0];e=callback();e['state']=x;e['objective']=-1.75
        f=final();f['state']=x;f['objective']=-1.75
        self.assertIsNotNone(q.validate(MODEL,'first',raw([e,f]),0,False)['final'])
        for mutate in ['sign','pair','offset']:
            m=copy.deepcopy(MODEL)
            if mutate=='sign': m['linear']=[-v for v in m['linear']]
            if mutate=='pair': m['pairs'][0][2]*=2
            if mutate=='offset': m['offset']+=1; e['objective']-=1;f['objective']-=1
            with self.assertRaises(ValueError): q.validate(m,'first',raw([e,f]),0,False)

    def test_latch_and_sequence(self):
        for events in [[callback(),callback(2,True),final(2)],[callback(2),final()],[callback(),final(),final()],[]]:
            with self.assertRaises(ValueError): q.validate(MODEL,'first',raw(events),0,False)

    def test_domain_and_final_mismatch(self):
        for key,value in [('affinity','1'),('elapsed_ns',-1),('continue',0),('upstream_runtime',-1)]:
            e=callback();e[key]=value
            with self.assertRaises(ValueError): q.validate(MODEL,'first',raw([e,final()]),0,False)
        f=final();f['callbacks']=2
        with self.assertRaises(ValueError): q.validate(MODEL,'first',raw([callback(),f]),0,False)
        with self.assertRaises(ValueError): q.unique([('kind','a'),('kind','b')])

    def test_bad_model_seed_no_process(self):
        with patch.object(q.subprocess,'Popen') as popen, tempfile.TemporaryDirectory() as tmp:
            for seed in [-1,True,65536]:
                with self.assertRaises(ValueError): q.run_cell(MODEL,'fake',seed,'first',Path(tmp)/'x',{},10**30)
            popen.assert_not_called()

    def test_mock_forced_kill_retains_raw(self):
        data=raw([callback(1,True)])
        class Process:
            pid=123
            returncode=None
            calls=0
            def communicate(self,timeout):
                self.calls+=1
                if self.calls==1: raise subprocess.TimeoutExpired('fake',timeout)
                return data,b''
            def kill(self): self.returncode=-9
            def poll(self): return self.returncode
        clock=[0]
        def now(): clock[0]+=100000001; return clock[0]
        with tempfile.TemporaryDirectory() as tmp, patch.object(q.os,'sched_getaffinity',return_value={0}), patch.object(q.time,'monotonic_ns',side_effect=now), patch.object(q.subprocess,'Popen',return_value=Process()):
            r=q.run_cell(MODEL,'fake',101,'watchdog',Path(tmp)/'cell',{'forced_watchdog_seconds':.1},10**30)
            self.assertEqual(r['status'],'VALID');self.assertEqual(bytes.fromhex(r['stdout_hex']),data)
            saved=json.loads((Path(tmp)/'cell/record.json').read_text());self.assertEqual(saved,r)


if __name__=='__main__': unittest.main()
