import itertools
import json
from pathlib import Path
import signal
import sys
import tempfile
import unittest
from unittest.mock import patch
import campaign as c
import worker


class Checks(unittest.TestCase):
    def test_input_expansion(self):
        for case in c.cases()+[c.smoke_case()]:
            for mask in [0,(1<<case['n'])-1,0x55555555,0xAAAAAAAA,0xCAFE,0xF00D]:
                c.score(case, ''.join(str((mask>>v)&1) for v in range(case['n'])))

    def test_quadratic_minimum(self):
        case=c.smoke_case();n,terms,_,_=worker.model(case,True)
        self.assertLessEqual(n,14)
        for x in range(1<<case['n']):
            bits=''.join(str((x>>v)&1) for v in range(case['n']))
            es=[]
            for y in range(1<<(n-case['n'])):
                mask=x|(y<<case['n'])
                es.append(sum(w for w,vs in terms if all(mask&(1<<v) for v in vs)))
            self.assertEqual(min(es),c.score(case,bits))

    def test_seed_and_order_balance(self):
        seeds=[worker.restart_seed(s,i) for s in c.SEEDS for i in range(1000)]
        self.assertEqual(len(seeds),len(set(seeds)))
        with self.assertRaises(ValueError):worker.restart_seed(950001,1_000_000)
        orders=[c.ORDERS[i%4] for i in range(100)]
        for a in range(4):
            self.assertEqual([sum(o[p]==a for o in orders) for p in range(4)],[25]*4)
            for b in range(a+1,4):self.assertEqual(sum(o.index(a)<o.index(b) for o in orders),50)

    def row(self):
        case=c.smoke_case();bits='0'*case['n']
        return {'errors':[],'budget_s':2.0,'stop_s':2.001,'termination_s':2.01,
                'startup_s':0.1,'returncode':-signal.SIGTERM,
                'events':[{'received_s':0.01,'event':{'kind':'inc','energy':c.score(case,bits),'bits':bits}},
                          {'received_s':0.02,'event':{'kind':'meta','model_n':6,'model_terms':4,'penalty':0,'hot':4.,'transform_s':0.01}}]}

    def test_corruption_deadline_missing_early(self):
        for key,value in [('budget_s',3),('stop_s',1.9),('termination_s',2.3),('returncode',0),('errors',['bad'])]:
            row=self.row();row[key]=value
            with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)
        row=self.row();row['events'][0]['event']['energy']+=1
        with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)
        row=self.row();row['events']=[]
        with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)
        row=self.row();row['events'][0]['received_s']=2.01
        with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)

    def test_late_better_witness_not_credited(self):
        row=self.row();case=c.smoke_case()
        best=min((''.join(x) for x in itertools.product('01',repeat=6)),key=lambda b:c.score(case,b))
        row['events'].append({'received_s':2.01,'event':{'kind':'inc','energy':c.score(case,best),'bits':best}})
        self.assertEqual(c.validate(case,row)[1],'000000')

    def test_nonfinite_and_malformed_metadata(self):
        for key in ['budget_s','startup_s','stop_s','termination_s']:
            row=self.row();row[key]=float('nan')
            with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)
        for key,value in [('model_n',1),('hot',float('inf')),('penalty',-1),('transform_s',10)]:
            row=self.row();row['events'][1]['event'][key]=value
            with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)
        row=self.row();del row['events'][1]['event']['model_terms']
        with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)
        row=self.row();row['events'][0]['received_s']=float('nan')
        with self.assertRaises(ValueError):c.validate(c.smoke_case(),row)

    def test_tail_parser_preserves_late_and_rejects_complete_malformed(self):
        with self.assertRaises(ValueError):c.parse_tail(b'not json\n',2.01)
        self.assertEqual(c.parse_tail(b'{"partial":',2.01),[])
        event=self.row()['events'][0]['event']
        parsed=c.parse_tail((json.dumps(event)+'\n').encode(),2.01)
        self.assertEqual(parsed,[{'received_s':2.01,'event':event}])

    def test_signed_rank_holm(self):
        self.assertEqual(c.signed_rank([1]*10),2/1024)
        self.assertEqual(c.signed_rank([0]*10),1.)
        self.assertEqual(c.signed_rank([-1,1]),1.)
        self.assertEqual(c.holm([.03,.001,.002]),[.03,.003,.004])
        self.assertEqual(c.distribution([1,2,3,4])['median'],2.5)

    def test_supervisor_early_exit(self):
        row=c.supervise([sys.executable,'-u','-c',"print('READY'); input(); raise SystemExit(0)"],c.smoke_case())
        self.assertTrue(row['errors'])

    def test_supervisor_actual_deadline(self):
        row=c.supervise([sys.executable,'-u','-c',"import time; print('READY'); input(); time.sleep(10)"],c.smoke_case())
        self.assertFalse(row['errors'],row)
        self.assertGreaterEqual(row['stop_s'],2.0)
        self.assertLessEqual(row['termination_s'],2.2)

    def test_launch_failure_and_group_cleanup(self):
        row=c.supervise(['/no/such/hubo-executable'],c.smoke_case())
        self.assertIn('launch failure',row['errors'])
        code="import subprocess,time,sys; print('READY'); input(); subprocess.Popen([sys.executable,'-c','import time;time.sleep(20)']); time.sleep(20)"
        row=c.supervise([sys.executable,'-u','-c',code],c.smoke_case())
        self.assertFalse(row['errors'],row)

    def test_signal_handler_malformed_tail(self):
        code="import signal,time; signal.signal(signal.SIGTERM,lambda *_: (print('not-json',flush=True),exit(0))); print('READY'); input(); time.sleep(20)"
        row=c.supervise([sys.executable,'-u','-c',code],c.smoke_case())
        self.assertIn('not-json',row['stdout'])
        self.assertTrue(any('invalid tail' in e for e in row['errors']))

    def test_instance_unit_and_primary_decision(self):
        cases=[{'id':str(i),'normalizer':128} for i in range(10)]
        data={(v['id'],s,a):0 for v in cases for s in c.SEEDS for a in c.ARMS}
        self.assertEqual(c.primary(cases,c.SEEDS,data)['verdict'],'INCONCLUSIVE')
        for v in cases:
            for si,s in enumerate(c.SEEDS):
                for a in c.ARMS[1:]:data[v['id'],s,a]=16 if si<9 or v['id']=='9' else -144
        result=c.primary(cases,c.SEEDS,data)
        self.assertEqual(result['comparisons'][0]['instance_gains'][:9],[0]*9)
        self.assertEqual(result['comparisons'][0]['p'],1.)
        self.assertEqual(result['verdict'],'NOT_QUALIFIED_FOR_ADVANTAGE')
        for key in data:data[key]=0 if key[2]=='msc_native' else 8
        self.assertEqual(c.primary(cases,c.SEEDS,data)['verdict'],'CONTINUE')
        for key in data:
            if key[2]=='oj_native':data[key]=0
        self.assertEqual(c.primary(cases,c.SEEDS,data)['verdict'],'NOT_QUALIFIED_FOR_ADVANTAGE')
        for key in data:data[key]=0 if key[2]=='msc_native' else (8 if int(key[0])>=5 else -1)
        with patch.object(c,'signed_rank',return_value=.001):
            self.assertEqual(c.primary(cases,c.SEEDS,data)['verdict'],'NOT_QUALIFIED_FOR_ADVANTAGE')

    def test_analysis_complete_set_and_command(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);env={'sources':{},'commit':'test','smoke':True,'python':'python','engine':'engine'}
            c.dump(root/'environment.json',env);c.dump(root/'complete.json',{'commit':'test','cells':8})
            for seed in [910001,910002]:
                for arm in c.ARMS:
                    row=self.row();row['command']=['python',str(c.HERE/'worker.py'),'--arm',arm,'--seed',str(seed),'--engine','engine']
                    p=root/f'smoke6_{seed}_{arm}.json';c.dump(p,row);p.with_suffix('.sol').write_text('000000\n')
            with patch.object(c,'verify_sources'):
                self.assertEqual(c.analyze(root)['cells'],8)
                p=root/'smoke6_910001_msc_native.json';row=json.loads(p.read_text());row['command'][5]='0';c.dump(p,row)
                with self.assertRaises(ValueError):c.analyze(root)
                p.unlink()
                with self.assertRaises(ValueError):c.analyze(root)
        with self.assertRaises(ValueError):c.verify_sources({'sources':{},'commit':'bad'})


if __name__=='__main__':unittest.main()
