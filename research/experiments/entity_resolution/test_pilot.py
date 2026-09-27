"""Synthetic process/archive witnesses only: never runs the empirical pilot."""
import copy
import json
from pathlib import Path
import signal
import tempfile
import unittest
from unittest.mock import patch
import core
import pilot as p


class Checks(unittest.TestCase):
    def case(self):return p.design(True)[0][0]

    def row(self):
        model=core.model(3,self.case()['weights'])
        inc=[0]*model['n']
        for w,vs in model['terms']:
            for v in vs:inc[v]+=abs(w)
        events=[{'received_s':.01,'event':{'kind':'inc','bits':'000','energy':0,'steps':0,'origin':'fallback'}},
                {'received_s':.02,'event':{'kind':'meta','model_n':3,'model_terms':len(model['terms']),
                    'penalty':0,'hot':max(inc)/p.math.log(2),'transform_s':.001}},
                {'received_s':.1,'event':{'kind':'inc','bits':'100','energy':-16,'steps':1}}]
        clocks={'boot_ns':900000000,'go_ns':1000000000,'stop_ns':3001000000,
                'term_ns':3002000000,'finish_ns':3010000000,
                'boot_unix_ns':1,'go_unix_ns':2,'finish_unix_ns':3}
        return {'command':p.command_for(self.env(),p.SMOKE_SEED),'budget_s':2.,'startup_s':.1,
                'stop_s':2.001,'termination_s':2.01,'returncode':-signal.SIGTERM,
                'errors':[],'events':events,'clocks':clocks,'pid':9999,'survivors':[],
                'stderr':'','stdout':'READY\n'+''.join(json.dumps(e['event'])+'\n' for e in events)}

    def env(self):
        return {'python_executable':'/usr/bin/python','pilot_path':str(p.HERE/'pilot.py'),'engine':'/bin/engine',
                'repo_root':str(p.ROOT),'engine_sha256':'0'*64,'threads':p.deadline.THREAD_ENV,
                'python':'synthetic','platform':'synthetic','cpu':'synthetic','rustc':'synthetic','rustflags':None,
                'sources':{},'smoke':True,'seeds':[p.SMOKE_SEED],'case_sha256':p.CASE_SHA,
                'evaluated_commit':'0'*40,'budget_s':2.0}

    def archive(self,directory):
        case=self.case();ws=case['weights']
        p.write_new(directory/'environment.json',self.env())
        p.write_new(directory/'cases.json',[case])
        p.write_new(directory/'complete.json',{'cases':1,'cells':1,'evaluated_commit':'0'*40})
        p.write_new(directory/'smoke_baselines.json',{name:{'bits':fn(3,ws),**core.inspect(3,ws,fn(3,ws)),'wall_s':.01}
                                                  for name,fn in p.BASELINES.items()})
        p.write_new(directory/'smoke_model.json',{'model':core.model(3,ws),'construction_s':.001})
        p.write_new(directory/f'smoke_{p.SMOKE_SEED}.json',self.row())
        p.seal_raw(directory,[case],[p.SMOKE_SEED])

    def test_native_endpoint_and_energy_corruption(self):
        self.assertEqual(p.checked_endpoint(self.case(),self.row())['bits'],'100')
        for mutation in ['energy','bits','clock','late_energy']:
            row=self.row()
            if mutation=='energy':row['events'][2]['event']['energy']=-8
            elif mutation=='bits':row['events'][2]['event']['bits']='110'
            elif mutation=='clock':row['clocks']['stop_ns']+=1
            else:row['events'][2]['received_s']=2.005;row['events'][2]['event']['energy']=5
            row['stdout']='READY\n'+''.join(json.dumps(e['event'])+'\n' for e in row['events'])
            with self.subTest(mutation=mutation),self.assertRaises(ValueError):p.checked_endpoint(self.case(),row)

    def test_late_correct_witness_not_credited(self):
        row=self.row();row['events'][2]['received_s']=2.005
        self.assertEqual(p.checked_endpoint(self.case(),row)['bits'],'000')

    def test_seed_duplicate_and_overlap_rejected(self):
        row=self.row()
        finish=p.verify_row_identity(row,self.env(),p.SMOKE_SEED,0)
        with self.assertRaises(ValueError):p.verify_row_identity(row,self.env(),p.SMOKE_SEED+1,0)
        with self.assertRaises(ValueError):p.verify_row_identity(row,self.env(),p.SMOKE_SEED,finish)

    def test_synthetic_archive_and_tampering(self):
        for mutation in [None,'truth','seed','subset','extra','hash','source','model','threads','engine_hash']:
            with self.subTest(mutation=mutation),tempfile.TemporaryDirectory() as tmp:
                d=Path(tmp);self.archive(d)
                if mutation in ['truth','subset']:
                    cases=json.loads((d/'cases.json').read_text())
                    if mutation=='truth':cases[0]['truth'][0]='wrong'
                    else:cases=[]
                    (d/'cases.json').write_text(json.dumps(cases))
                elif mutation in ['seed','source','threads','engine_hash']:
                    env=json.loads((d/'environment.json').read_text())
                    if mutation=='seed':env['seeds']=[1]
                    elif mutation=='source':env['sources']={'missing':'bad'}
                    elif mutation=='threads':env['threads']={**env['threads'],'RAYON_NUM_THREADS':'8'}
                    else:del env['engine_sha256']
                    (d/'environment.json').write_text(json.dumps(env))
                    manifest=json.loads((d/'raw_manifest.json').read_text());manifest['environment.json']=p.sha(d/'environment.json')
                    (d/'raw_manifest.json').write_text(json.dumps(manifest))
                elif mutation=='extra':(d/'unexpected.json').write_text('{}')
                elif mutation=='hash':(d/'smoke_baselines.json').write_text('{}')
                elif mutation=='model':
                    obj=json.loads((d/'smoke_model.json').read_text());obj['model']['n']=4
                    (d/'smoke_model.json').write_text(json.dumps(obj))
                    manifest=json.loads((d/'raw_manifest.json').read_text());manifest['smoke_model.json']=p.sha(d/'smoke_model.json')
                    (d/'raw_manifest.json').write_text(json.dumps(manifest))
                with patch.object(p,'source_files',return_value={}):
                    if mutation:
                        with self.assertRaises(ValueError):p.analyze(d)
                    else:
                        self.assertEqual(p.analyze(d)['native_optimum_hits'],1)

    def test_source_inventory_and_exclusive_output(self):
        sources=p.source_files()
        self.assertTrue(all(str((p.HERE/f).relative_to(p.ROOT)) in sources for f in p.REQUIRED))
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'result.json';p.write_new(path,{'x':1})
            with self.assertRaises(FileExistsError):p.write_new(path,{'x':2})

    def test_exact_f1_gate_boundary(self):
        totals={a:{'tp':7,'fp':43,'fn':43,'tn':87} for a in ['pairwise','closure','greedy']}
        totals['exact']={'tp':8,'fp':42,'fn':42,'tn':88}
        self.assertTrue(p.semantic_gate(totals,True))
        self.assertFalse(p.semantic_gate(totals,False))
        totals['exact']['fp']+=1
        self.assertFalse(p.semantic_gate(totals,True))

    def test_fallback_not_native_success(self):
        row=self.row();row['events']=row['events'][:2]
        self.assertFalse(p.native_hit(row,0))
        row['events'].append({'received_s':.1,'event':{'kind':'inc','bits':'000','energy':0,'steps':1}})
        self.assertTrue(p.native_hit(row,0))
        row['events'][-1]['received_s']=2.001
        self.assertFalse(p.native_hit(row,0))


if __name__=='__main__':unittest.main()
