"""Fabricated timing tests and a tiny exhaustive native energy oracle (no search)."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import calibrate as c

MODEL={'offset':3.25,'linear':[.5,-1.25,0.,0.], 'pairs':[[0,1,2.5],[1,2,-.75]]}


def event(kind, worker='rust_model', receipt=20):
    if kind=='inc':
        obj={'kind':'inc','state':[0]*4,'energy':3.25,'chunk':0} if worker=='rust_model' else {'kind':'inc','state':[0]*4,'objective':0.}
    else:
        energies=[float(c.mq.exact_energy(MODEL,x)) for x in c.probe_states(4)]
        if worker=='mqlib_model': energies=[3.25-e for e in energies]
        obj={'kind':'diagnostic','energies':energies,'ready_ns':2,'sleep_ns':3,'affinity':'0'}
    return c.validate_event(MODEL,worker,json.dumps(obj),receipt,0,100)


def rows():
    result=[]
    for i in range(20):
        for a in c.ARMS:
            delay=25000000 if a=='delay' else 0
            e=[event('inc'),event('diagnostic')]
            for v in e:
                v['latency_ns']=10000000+delay
                v['receipt_ns']=v['latency_ns']
            e[1]['decoded']['sleep_ns']=delay
            result.append({'repeat':i,'arm':a,'status':'VALID','events':e,
                           'parent_affinity':[0],'child_affinity':[0],
                           'cutoff_ns':100000000,'kill_reap_ns':1000,'total_ns':100001000})
    return result


class NativeTests(unittest.TestCase):
    def test_exact_deadline_is_late(self):
        e=event('inc',receipt=100)
        self.assertFalse(e['eligible'])

    def test_both_lines_required(self):
        self.assertIsNone(c.completion({'events':[event('inc')]}))
        self.assertEqual(c.completion({'events':[event('inc'),event('diagnostic',receipt=21)]}),21)

    def test_late_second_line_censors_completion(self):
        self.assertIsNone(c.completion({'events':[event('inc'),event('diagnostic',receipt=100)]}))

    def test_wrong_energy_sign_and_state(self):
        for worker in c.WORKERS:
            obj=event('diagnostic',worker)['decoded'];obj['energies'][1]+=1
            with self.assertRaises(ValueError): c.validate_event(MODEL,worker,json.dumps(obj),20,0,100)
        obj=event('inc')['decoded'];obj['state'][0]=True
        with self.assertRaises(ValueError): c.validate_event(MODEL,'rust_model',json.dumps(obj),20,0,100)

    def test_schema_affinity_and_duration(self):
        for key,value in [('affinity','1'),('sleep_ns',-1),('ready_ns',100),('extra',1)]:
            obj=event('diagnostic')['decoded'];obj[key]=value
            with self.assertRaises(ValueError): c.validate_event(MODEL,'rust_model',json.dumps(obj),20,0,100)
        with self.assertRaises(ValueError): c.unique([('x',1),('x',2)])

    def test_group_pass_and_quantile(self):
        self.assertEqual(c.evaluate_group(rows(),100000000)['status'],'PASS')
        self.assertEqual(c.q95(list(range(20))),18)

    def test_missing_pairs_fail(self):
        rs=rows()
        for r in rs:
            if r['repeat']<2 and r['arm']=='delay': r['events']=[]
        g=c.evaluate_group(rs,100000000)
        self.assertFalse(g['gates']['completion'])
        self.assertFalse(g['gates']['sleep'])

    def test_positive_control_detects_missing_delay(self):
        rs=rows()
        for r in rs:
            for e in r['events']: e['latency_ns']=10000000
        self.assertFalse(c.evaluate_group(rs,100000000)['gates']['positive'])

    def test_late_diagnostic_sleep_still_checked(self):
        rs=rows()
        r=next(r for r in rs if r['arm']=='delay')
        r['events'][1]['eligible']=False
        r['events'][1]['decoded']['sleep_ns']=1
        g=c.evaluate_group(rs,100000000)
        self.assertTrue(g['gates']['completion'])
        self.assertFalse(g['gates']['sleep'])

    def test_masks_and_overshoot(self):
        rs=rows();rs[0]['child_affinity']=[1]
        self.assertFalse(c.evaluate_group(rs,100000000)['gates']['affinity'])
        for r in rs: r['cutoff_ns']=110000000
        self.assertFalse(c.evaluate_group(rs,100000000)['gates']['deadline'])

    def test_unexpected_exit(self):
        self.assertTrue(c.close_errors(0,False,True,5,100,b''))
        self.assertFalse(c.close_errors(-9,True,False,101,100,b'partial'))

    def test_manifest_grid(self):
        m=json.loads((c.HERE/'manifest.json').read_text())
        jobs=c.jobs(m)
        self.assertEqual(len(jobs),960)
        self.assertEqual(len(set(jobs)),960)
        self.assertAlmostEqual(sum(j[-1] for j in jobs),44.4)
        self.assertEqual(c.analyze([],m)['status'],'INCOMPLETE')

    def test_parent_validation_precedes_kill_and_counts_toward_cutoff(self):
        clock=[0]
        class Pipe:
            def fileno(self): return 42
            def read(self): return b''
            def close(self): pass
        class Process:
            pid=123
            stdout=Pipe()
            returncode=None
            def poll(self): return self.returncode
            def kill(self): self.returncode=-9
            def wait(self,timeout): return self.returncode
        class Selector:
            def __enter__(self): return self
            def __exit__(self,*args): pass
            def register(self,*args): pass
            def select(self,timeout): return [True]
        proc=Process()
        payload=(event('inc')['raw']+'\n'+event('diagnostic')['raw']+'\n').encode()
        original=c.validate_event
        def costly(*args):
            self.assertIsNone(proc.poll(), 'validation was deferred until after kill')
            answer=original(*args)
            clock[0]+=30000000
            return answer
        def read(*args):
            clock[0]=100
            return payload
        with patch.object(c.time,'monotonic_ns',side_effect=lambda:clock[0]), \
             patch.object(c.os,'sched_getaffinity',return_value={0}), \
             patch.object(c.os,'set_blocking'), patch.object(c.os,'read',side_effect=read), \
             patch.object(c.subprocess,'Popen',return_value=proc), \
             patch.object(c.selectors,'DefaultSelector',return_value=Selector()), \
             patch.object(c,'validate_event',side_effect=costly):
            r=c.run_cell(MODEL,'rust_model','fake',0,'null_a',.05,0,1000000000)
        self.assertEqual(r['status'],'VALID')
        self.assertGreater(r['cutoff_ns'],50000000)
        self.assertEqual(len(r['events']),2)

    def test_cpp_exhaustive_sign_offset(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'tiny.qubo';p.write_text(c.mq.export_qubo(MODEL))
            r=subprocess.run([str(c.build.BINS['mqlib_model']),str(p),'spectrum'],capture_output=True,text=True,check=True)
        got=[float(x) for x in r.stdout.splitlines()]
        expected=[MODEL['offset']-float(c.mq.exact_energy(MODEL,[(mask>>i)&1 for i in range(4)])) for mask in range(16)]
        self.assertEqual(got,expected)


if __name__=='__main__': unittest.main()
