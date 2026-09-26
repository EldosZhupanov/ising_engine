"""Deterministic synthetic endpoints and fake processes only; no solver outcomes."""
import copy
import itertools
import json
import math
from pathlib import Path
import shutil
import signal
import sys
import tempfile
import unittest
from unittest.mock import patch
import phase_b as c


class Checks(unittest.TestCase):
    def row(self, offset=0):
        case = c.base.smoke_case()
        incidence = [0] * 6
        for w, vs in case['terms']:
            for v in vs:
                incidence[v] += abs(w)
        start = 1_000_000_000 + offset
        row = dict(errors=[], budget_s=2., startup_s=.1, stop_s=2.001,
                   termination_s=2.01, returncode=-signal.SIGTERM, survivors=[], pid=99999,
                   stdout='', stderr='', clocks=dict(boot_ns=start - 100_000_000,
                   go_ns=start, stop_ns=start + 2_001_000_000, term_ns=start + 2_002_000_000,
                   finish_ns=start + 2_010_000_000, boot_unix_ns=1, go_unix_ns=2, finish_unix_ns=3),
                   events=[dict(received_s=.01, event=dict(kind='inc', bits='000000', energy=c.base.score(case, '000000'), steps=0)),
                           dict(received_s=.02, event=dict(kind='meta', model_n=6, model_terms=len(case['terms']), penalty=0,
                                hot=max(incidence)/math.log(2), transform_s=.001))])
        self.stdout(row)
        return row

    def stdout(self, row):
        row['stdout'] = 'READY\n' + ''.join(json.dumps(x['event']) + '\n' for x in row['events'])

    def test_balanced_order_and_inputs(self):
        cs, seeds = c.inputs()
        plan = c.plan(cs, seeds)
        self.assertEqual(len(plan), 240)
        for ci, case in enumerate(cs):
            rr = [r for r in plan if r['case'] == case['id']]
            self.assertEqual(sum(r['arm'] == 'msc_native' for r in rr[::2]), 5)
            self.assertEqual([r['seed'] for r in rr[::2]], seeds)
        self.assertEqual(c.inputs(True)[1], [980001, 980002])

    def toy(self):
        cs = []
        for n, d in itertools.product([64, 128], [3, 4]):
            for i in range(3):
                case = copy.deepcopy(c.base.smoke_case())
                case.update(id=f'{n}_{d}_{i}', n=n, degree=d)
                cs.append(case)
        data = {(v['id'], s, a): 0 for v in cs for s in c.SEEDS for a in c.ARMS}
        return cs, data

    def test_gate_boundaries_and_symmetry(self):
        cs, data = self.toy()
        self.assertEqual(c.classify(cs, c.SEEDS, data)['verdict'], 'NO_QUALIFIED_VARIATION')
        for count, expected in [(1, 'NO_QUALIFIED_VARIATION'), (2, 'MIXED'), (5, 'MIXED'), (8, 'VARIATION_PRESENT')]:
            cs, data = self.toy()
            for case in cs[:count]:
                for s in c.SEEDS[:2]:
                    data[case['id'], s, c.ARMS[1]] = 2
            out = c.classify(cs, c.SEEDS, data)
            self.assertEqual(out['verdict'], expected)
            swapped = {(i, s, c.ARMS[1-c.ARMS.index(a)]): v for (i, s, a), v in data.items()}
            other = c.classify(cs, c.SEEDS, swapped)
            self.assertEqual(out['verdict'], other['verdict'])
            self.assertEqual(out['strata'], other['strata'])
        cs, data = self.toy()
        for case in cs:
            data[case['id'], c.SEEDS[0], c.ARMS[1]] = 2
        self.assertEqual(c.classify(cs, c.SEEDS, data)['passing_strata'], 0)

    def test_permuted_multisets_not_superiority_and_stable_separation(self):
        cs, data = self.toy()
        for case in cs:
            data[case['id'], c.SEEDS[0], c.ARMS[0]] = 2
            data[case['id'], c.SEEDS[1], c.ARMS[1]] = 2
        out = c.classify(cs, c.SEEDS, data)
        self.assertEqual(out['verdict'], 'VARIATION_PRESENT')
        for value in out['instances'].values():
            self.assertEqual(value['msc_wins'], value['msc_losses'])
        for key in data:
            data[key] = 2 if key[2] == c.ARMS[1] else 0
        out = c.classify(cs, c.SEEDS, data)
        self.assertEqual(out['verdict'], 'NO_QUALIFIED_VARIATION')
        self.assertTrue(all(v['stable_separation'] for v in out['instances'].values()))

    def test_row_mutations(self):
        case = c.base.smoke_case()
        self.assertEqual(c.validate(case, self.row())[1], '000000')
        for key, value in [('budget_s', 3), ('stop_s', 1.9), ('termination_s', 2.3),
                           ('returncode', 0), ('errors', ['bad']), ('survivors', [1]),
                           ('startup_s', float('nan')), ('stdout', 'READY\n')]:
            row = self.row(); row[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                c.validate(case, row)
        for field, value in [('model_n', 7), ('model_terms', 1), ('penalty', 1), ('hot', float('inf')), ('transform_s', 9)]:
            row = self.row(); row['events'][1]['event'][field] = value; self.stdout(row)
            with self.subTest(field=field), self.assertRaises(ValueError):
                c.validate(case, row)
        row = self.row(); row['events'][0]['event']['energy'] += 1; self.stdout(row)
        with self.assertRaises(ValueError): c.validate(case, row)
        row = self.row(); row['clocks']['finish_ns'] += 1
        with self.assertRaises(ValueError): c.validate(case, row)

    def test_late_witness_checked_but_not_credited(self):
        case = c.base.smoke_case(); row = self.row()
        best = min((''.join(x) for x in itertools.product('01', repeat=6)), key=lambda b: c.base.score(case, b))
        row['events'].append(dict(received_s=2.005, event=dict(kind='inc', bits=best, energy=c.base.score(case, best))))
        self.stdout(row)
        self.assertEqual(c.validate(case, row)[1], '000000')
        row['events'][-1]['event']['energy'] += 1; self.stdout(row)
        with self.assertRaises(ValueError): c.validate(case, row)

    def test_deadline_group_cleanup_nonblocking_large_input(self):
        # Child and parent sleep, neither reads stdin: input is larger than pipe capacity.
        code = "import time,sys,subprocess; print('READY',flush=True); subprocess.Popen([sys.executable,'-c','import time; time.sleep(30)']); time.sleep(30)"
        row = c.supervise([sys.executable, '-u', '-c', code], {'payload': 'x' * 1_000_000})
        self.assertFalse(row['errors'], row)
        self.assertGreaterEqual(row['stop_s'], 2)
        self.assertLessEqual(row['termination_s'], 2.2)
        self.assertEqual(c.base.active_group(row['pid']), [])

    def test_early_exit_launch_failure_malformed(self):
        for code in ["print('READY',flush=True); input()", "import time; print('READY',flush=True); input(); print('bad-json',flush=True); time.sleep(30)"]:
            row = c.supervise([sys.executable, '-u', '-c', code], c.base.smoke_case())
            self.assertTrue(row['errors'])
        self.assertTrue(c.supervise(['/no/such/worker'], {})['errors'])

    def test_sigkill_and_partial_or_malformed_tail(self):
        code = "import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); print('READY',flush=True); input(); print('{partial',end='',flush=True); time.sleep(30)"
        row = c.supervise([sys.executable, '-u', '-c', code], {})
        self.assertFalse(row['errors'], row)
        self.assertEqual(row['returncode'], -signal.SIGKILL)
        self.assertIn('kill_ns', row['clocks'])
        self.assertTrue(row['stdout'].endswith('{partial'))
        self.assertEqual(row['events'], [])
        code = "import signal,time; signal.signal(signal.SIGTERM,lambda *_:(print('bad-json',flush=True),exit(0))); print('READY',flush=True); input(); time.sleep(30)"
        row = c.supervise([sys.executable, '-u', '-c', code], {})
        self.assertTrue(any('invalid tail' in e for e in row['errors']))
        self.assertIn('bad-json', row['stdout'])

    def fixture(self, root):
        env = c.reference_env()
        env.update(sources={}, commit='test', smoke=True, python='/missing/python', engine='/missing/engine',
                   root='/old/root', worker='/old/root/research/experiments/hubo_comparison/worker.py', thread_env=c.THREAD_ENV)
        cs, seeds = c.inputs(True); schedule = c.plan(cs, seeds)
        (root/'cells').mkdir(parents=True)
        c.dump(root/'environment.json', env); c.dump(root/'cases.json', cs)
        c.dump(root/'plan.json', schedule); c.dump(root/'complete.json', dict(commit='test', cells=4))
        for item in schedule:
            row = self.row(item['sequence'] * 3_000_000_000)
            row.update(identity=item, command=c.command(env, item))
            path = root/'cells'/f'{item["sequence"]:04d}.json'
            c.dump(path, row); path.with_suffix('.sol').write_text('000000\n')
        c.seal(root)

    def test_portability_inventory_and_sequence(self):
        with tempfile.TemporaryDirectory() as tmp:
            first, moved = Path(tmp)/'a', Path(tmp)/'b'
            self.fixture(first); shutil.copytree(first, moved); shutil.rmtree(first)
            # Stub ONLY Git provenance for fabricated fixture. No installed runtime is consulted.
            with patch.object(c, 'verify_sources'), patch.object(c, 'runtime_matches', side_effect=ValueError('missing runtime')) as runtime:
                self.assertEqual(c.analyze(moved)['verdict'], 'SMOKE_PASS')
                runtime.assert_not_called()
                with self.assertRaises(ValueError): c.analyze(moved, runtime=True)
                p = moved/'cells/0001.json'; row = json.loads(p.read_text()); row['clocks']['boot_ns'] = 1
                c.dump(p, row); c.seal(moved)
                with self.assertRaises(ValueError): c.analyze(moved)
            (moved/'extra').write_text('unexpected')
            with self.assertRaises(ValueError): c.verify_raw(moved)

    def test_portable_environment_mutations(self):
        for key, value in [('packages', {'numpy': 'wrong'}), ('thread_env', {'RAYON_NUM_THREADS': '8'}),
                           ('engine_sha256', '0'*64), ('python_binary_sha256', '0'*64),
                           ('worker', '/wrong/worker.py'), ('rustflags', 'different'), ('baseline_hashes', {})]:
            with self.subTest(key=key), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)/'run'; self.fixture(root)
                env = json.loads((root/'environment.json').read_text()); env[key] = value
                c.dump(root/'environment.json', env); c.seal(root)
                with patch.object(c, 'verify_sources'), self.assertRaises(ValueError):
                    c.analyze(root)

    def test_invalid_marker_overrides_complete_and_hashes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)/'run'; self.fixture(root)
            c.dump(root/'invalid.json', dict(error='late audit failure'))
            c.seal(root)
            with self.assertRaisesRegex(ValueError, 'INSTRUMENT_INVALID'):
                c.verify_raw(root)

    def test_fail_closed_preserves_first_bad_row(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)/'run'
            env = dict(sources={}, engine_sha256='dummy', python='python', worker='worker', engine='engine', commit='test')
            bad = self.row(); bad['errors'] = ['deliberate failure']
            with patch.object(c, 'capture', return_value=env), patch.object(c, 'digest', return_value='dummy'), patch.object(c, 'supervise', return_value=bad) as sup:
                with self.assertRaises(ValueError): c.run(out, True, Path('python'), Path('engine'))
                self.assertEqual(sup.call_count, 1)
            self.assertTrue((out/'cells/0000.json').exists())
            self.assertTrue((out/'invalid.json').exists())
            self.assertFalse((out/'complete.json').exists())
            with self.assertRaises(ValueError): c.verify_raw(out)

    def test_empty_source_manifest_rejected(self):
        with self.assertRaises(ValueError):
            c.verify_sources(dict(commit=c.base.git('rev-parse', 'HEAD'), sources={}))


if __name__ == '__main__':
    unittest.main()
