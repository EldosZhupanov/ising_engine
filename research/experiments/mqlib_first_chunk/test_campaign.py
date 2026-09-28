"""Fabricated tests: no optimizer or qualification input is searched here."""
import base64
import json
import math
from pathlib import Path
import tempfile
import hashlib
import unittest
from unittest.mock import patch
import campaign
import audit


def record(cell, t):
    return {'cell': cell, 'status': 'VALID', 'ready': 1, 'kill_seconds': 10.01,
            'events': [{'message': {'kind': 'ready'}, 'received_seconds': .05, 'validated_seconds': .06}]
            + ([] if t is None else [{'verified_energy': -1, 'received_seconds': t-.001,
                                       'validated_seconds': t}])}


class FirstChunkTests(unittest.TestCase):
    def setUp(self):
        self.m = campaign.manifest()

    def test_exact_schedule_independent_auditor(self):
        plan = campaign.schedule(self.m)
        self.assertEqual(plan, audit.expected(self.m))
        self.assertEqual(len(plan), 12)
        self.assertEqual(len({tuple(c.items()) for c in plan}), 12)
        self.assertEqual((plan[0]['instance'], plan[-1]['instance']), ('q12', 'q18'))

    def test_strict_first_witness_deadlines(self):
        for t, result in [(1.999, 1.999), (2., 2.), (9.999, 9.999), (10., None), (10.001, None), (None, None)]:
            self.assertEqual(campaign.first_inc(record({}, t), 10), result)
        r = record({}, 9.999)
        r['events'][-1]['validated_seconds'] = 10.
        self.assertIsNone(campaign.first_inc(r, 10))

    def test_registered_checks_and_censoring(self):
        rs = [record(c, .3 if c['instance'] == 'q12' or c['arm'] == 'v2_default' else 4.)
              for c in campaign.schedule(self.m)]
        out = campaign.analyze(rs, self.m)
        self.assertEqual(out['status'], 'COMPLETE')
        self.assertTrue(out['C1'] and out['C2'] and out['CONTROL'])
        self.assertEqual(out['cells'][0]['first_inc_receipt_seconds'], .299)
        self.assertEqual(out['cells'][0]['first_inc_validated_seconds'], .3)
        self.assertEqual(out['before_2s'], {'ultimate': 3, 'v2_default': 6})
        self.assertEqual(out['readiness_seconds']['q18/ultimate'],
                         {'median': .06, 'min': .06, 'max': .06})
        self.assertEqual(audit.readiness_summary(self.m, out['cells']), out['readiness_seconds'])
        rs[-2] = record(rs[-2]['cell'], None)  # q18 Ultimate, censored
        out = campaign.analyze(rs, self.m)
        self.assertTrue(out['C2'])
        self.assertEqual(out['cells'][-2]['right_censored_at_seconds'], 10.)
        for r in rs:
            if r['cell']['instance'] == 'q18' and r['cell']['arm'] == 'ultimate':r['events'] = r['events'][:1]
        self.assertFalse(campaign.analyze(rs, self.m)['C2'])
        rs[0] = record(rs[0]['cell'], 10.001)
        late = campaign.analyze(rs, self.m)['cells'][0]
        self.assertIsNone(late['first_verified_inc_seconds'])
        self.assertEqual(late['first_inc_validated_seconds'], 10.001)

    def test_missing_duplicate_invalid_and_kill_boundary(self):
        rs = [record(c, .3) for c in campaign.schedule(self.m)]
        self.assertEqual(campaign.analyze(rs[:-1], self.m)['status'], 'INVALID')
        self.assertEqual(campaign.analyze([rs[0]] + rs[:-1], self.m)['status'], 'INVALID')
        rs[0]['status'] = 'INVALID'
        self.assertEqual(campaign.analyze(rs, self.m)['status'], 'INVALID')
        rs[0]['status'] = 'VALID'
        rs[0]['kill_seconds'] = 10.1
        self.assertEqual(campaign.analyze(rs, self.m)['status'], 'COMPLETE')
        rs[0]['kill_seconds'] = math.nextafter(10.1, math.inf)
        self.assertEqual(campaign.analyze(rs, self.m)['status'], 'INVALID')

    def test_raw_energy_and_duplicate_json(self):
        model = {'offset': 2, 'linear': [3, -1], 'pairs': [[0, 1, 4]]}
        self.assertEqual(audit.old.energy(model, [1, 1]), 8)
        with self.assertRaises(AssertionError):audit.old.energy(model, [1, 2])
        with self.assertRaises(AssertionError):audit.old.verify_reported_energy(8, {'energy': 9}, False, 2)
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / 'duplicate.json';path.write_text('{"x":1,"x":2}')
            with self.assertRaisesRegex(AssertionError, 'duplicate JSON key'):audit.read(path)

    def test_source_drift_rejected(self):
        with patch.object(campaign.subprocess, 'check_output', return_value=b'incorrect committed bytes'):
            with self.assertRaisesRegex(ValueError, 'uncommitted instrument'):
                campaign.source_record()

    def test_binary_hash_drift_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td);(root / 'worker').write_bytes(b'frozen')
            good = {'binaries': {'worker': hashlib.sha256(b'frozen').hexdigest()}}
            audit.verify_binary_hashes(good, root)
            (root / 'worker').write_bytes(b'changed')
            with self.assertRaisesRegex(AssertionError, 'native binary drift'):
                audit.verify_binary_hashes(good, root)

    def test_fabricated_full_audit_and_bad_exit(self):
        with tempfile.TemporaryDirectory() as td:
            folder = Path(td)
            m = self.m;records = []
            for i, cell in enumerate(campaign.schedule(m)):
                model = json.loads((campaign.ROOT / next(x['file'] for x in m['instances'] if x['id'] == cell['instance'])).read_text())
                zero = [0] * len(model['linear']);events=[];raw=[]
                messages = [{'kind': 'ready', 'affinity': '0'}]
                if cell['arm'] == 'v2_default':
                    messages.append({'kind': 'config', 'backend': 'DenseByte', 'operators': ['consensus_freeze'],
                        'temperatures': [4 * (.08 / 4) ** (j / 31) for j in range(32)],
                        'replicas': 32, 'seed': (cell['seed'] * 0x9e3779b97f4a7c15) % 2**64})
                messages.append({'kind': 'inc', 'state': zero, 'energy': model['offset'], 'chunk': 0})
                for j, message in enumerate(messages):
                    line = json.dumps(message, separators=(',', ':')).encode();raw.append(line + b'\n')
                    e={'raw_base64':base64.b64encode(line).decode(), 'message':message,
                       'received_seconds':.1 + j*.1,'validated_seconds':.11 + j*.1}
                    if message['kind']=='inc':e['verified_energy']=model['offset']
                    events.append(e)
                record={'cell':cell,'status':'VALID','errors':[],'events':events,'fallback_energy':model['offset'],
                    'parent_affinity':[0],'partial_base64':'','stdout_bytes':sum(map(len,raw)),
                    'ready':1,'configs':int(cell['arm']=='v2_default'),'cutoff_seconds':10.001,
                    'kill_seconds':10.002,'reap_seconds':10.003,'total_seconds':10.004,
                    'exit_code':-9,'observations':[]}
                cell_dir=folder/f'cell{i:02}';cell_dir.mkdir()
                (cell_dir/'stdout.bin').write_bytes(b''.join(raw));(cell_dir/'stderr.bin').write_bytes(b'')
                (cell_dir/'record.json').write_text(json.dumps(record))
                records.append(record)
            (folder/'metadata.json').write_text(json.dumps({'manifest':m}))
            (folder/'analysis.json').write_text(json.dumps(campaign.analyze(records,m)))
            (folder/'terminal.json').write_text(json.dumps({'status':'COMPLETE','cells':12,'elapsed_seconds':123}))
            with patch.object(audit,'provenance'):
                self.assertEqual(audit.audit(folder)['registered_status'],'COMPLETE')
                first=folder/'cell00/record.json';bad=json.loads(first.read_text());bad['exit_code']=1
                first.write_text(json.dumps(bad))
                with self.assertRaises(AssertionError):audit.audit(folder)


if __name__ == '__main__':unittest.main()
