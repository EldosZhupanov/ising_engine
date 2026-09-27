import copy
import json
import unittest

import calibrate as c


def group(budget=100000000):
    rows = []
    for i in range(20):
        for arm in c.ARMS:
            delay = budget // 4 if arm == 'delay' else 0
            rows.append({'repeat': i, 'arm': arm, 'status': 'VALID', 'cutoff_ns': budget + 100000,
                         'events': [{'eligible': True, 'latency_ns': 3000000 + delay,
                                     'delivery_lag_ns': 100000, 'sleep_ns': delay + 10000}]})
    return rows


class CalibrationTests(unittest.TestCase):
    def test_exact_boundary_and_late_are_censored(self):
        m = {'offset': 0, 'linear': [3], 'pairs': []}
        raw = json.dumps({'state': [0], 'energy': 0, 'sleep_start_ns': 101, 'sleep_end_ns': 102, 'pre_emit_ns': 103})
        self.assertTrue(c.validate_event(m, raw, 109, 100, 10)['eligible'])
        self.assertFalse(c.validate_event(m, raw, 110, 100, 10)['eligible'])
        self.assertFalse(c.validate_event(m, raw, 120, 100, 10)['eligible'])

    def test_wrong_state_energy_schema_and_clock(self):
        m = {'offset': 0, 'linear': [3], 'pairs': []}
        event = {'state': [0], 'energy': 0, 'sleep_start_ns': 101, 'sleep_end_ns': 102, 'pre_emit_ns': 103}
        for key, value in [('state', [1]), ('state', [False]), ('energy', 3), ('energy', float('nan')),
                           ('sleep_start_ns', 99), ('sleep_end_ns', 104), ('pre_emit_ns', 111), ('pre_emit_ns', 103.0)]:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                c.validate_event(m, json.dumps({**event, key: value}), 110, 100, 20)
        with self.assertRaises(ValueError):
            c.validate_event(m, json.dumps({**event, 'extra': 1}), 110, 100, 20)

    def test_early_eof_exit_and_partial_are_not_timeouts(self):
        self.assertTrue(c.close_errors(0, False, True, 8, 10, b''))
        self.assertTrue(c.close_errors(-9, False, True, 8, 10, b'{'))
        self.assertFalse(c.close_errors(-9, True, False, 11, 10, b'{'))

    def test_malformed_packet_is_validation_failure(self):
        model = {'offset': 0, 'linear': [1], 'pairs': []}
        for raw in ['null', '[]', '[[]]', 'true', '42', '{']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                c.validate_event(model, raw, 110, 100, 20)

    def test_quantile_and_even_median(self):
        self.assertEqual(c.q95(list(range(1, 21))), 19)
        self.assertEqual(c.q95(list(range(1, 61))), 57)
        self.assertEqual(c.describe([1, 2, 3, 4])['median'], 2.5)
        self.assertEqual(c.describe([1, 2, 3, 4])['q25'], 1.75)

    def test_all_seven_gates_pass_known_control(self):
        s = c.evaluate_group(group(), 100000000)
        self.assertEqual(s['status'], 'PASS')
        self.assertEqual(len(s['gates']), 7)
        self.assertEqual(s['positive_difference_ns']['median'], 25000000)

    def test_missing_duplicate_invalid_rows(self):
        base = group()
        for rows in [base[:-1], base[:-1] + [base[0]], [{**base[0], 'status': 'INVALID'}] + base[1:]]:
            self.assertEqual(c.evaluate_group(rows, 100000000)['status'], 'INVALID')

    def test_misses_not_imputed_and_pairs_required(self):
        rows = group()
        for r in rows:
            if r['arm'] == 'delay' and r['repeat'] < 2:
                r['events'] = []
        s = c.evaluate_group(rows, 100000000)
        self.assertFalse(s['gates']['completion'])
        self.assertFalse(s['gates']['pairs'])
        self.assertEqual(s['positive_difference_ns']['n'], 18)
        self.assertEqual(s['latency_ns_by_arm']['delay']['n'], 18)
        self.assertEqual(s['absent_messages'], 2)

    def test_each_numeric_gate_can_fail(self):
        for gate in ['deadline', 'delivery', 'null', 'positive', 'sleep']:
            rows = group()
            for r in rows:
                if gate == 'deadline':
                    r['cutoff_ns'] += 10000000
                elif gate == 'delivery':
                    r['events'][0]['delivery_lag_ns'] = 10000000
                elif gate == 'null' and r['arm'] == 'null_b':
                    r['events'][0]['latency_ns'] += 20000000
                elif gate == 'positive' and r['arm'] == 'delay':
                    r['events'][0]['latency_ns'] = 3000000
                elif gate == 'sleep' and r['arm'] == 'delay':
                    r['events'][0]['sleep_ns'] = 24999999
            with self.subTest(gate=gate):
                self.assertFalse(c.evaluate_group(rows, 100000000)['gates'][gate])

    def test_late_sleep_undershoot_is_not_hidden(self):
        rows = group()
        r = next(r for r in rows if r['arm'] == 'delay')
        r['events'][0].update(eligible=False, sleep_ns=1)
        s = c.evaluate_group(rows, 100000000)
        self.assertEqual(s['on_time_delay_sleep_count'], 19)
        self.assertFalse(s['gates']['sleep'])

    def test_positive_sign_count_and_null_border(self):
        rows = group()
        for r in rows:
            if r['arm'] == 'delay' and r['repeat'] < 3:
                r['events'][0]['latency_ns'] = 2000000
        self.assertFalse(c.evaluate_group(rows, 100000000)['gates']['positive'])
        rows = group()
        for r in rows:
            if r['arm'] == 'null_b':
                r['events'][0]['latency_ns'] += 10000000
        self.assertTrue(c.evaluate_group(rows, 100000000)['gates']['null'])
        for r in rows:
            if r['arm'] == 'null_b':
                r['events'][0]['latency_ns'] += 1
        self.assertFalse(c.evaluate_group(rows, 100000000)['gates']['null'])

    def test_campaign_completeness(self):
        manifest = {'fixtures': [{'file': 'a'}], 'budgets_seconds': [.1]}
        rows = [{**r, 'fixture': 'a', 'budget_ns': 100000000} for r in group()]
        self.assertEqual(c.analyze(rows, manifest)['status'], 'PASS')
        self.assertEqual(c.analyze(rows[:-1], manifest)['status'], 'INCOMPLETE')
        duplicate = copy.deepcopy(rows)
        duplicate[-1] = duplicate[0]
        self.assertEqual(c.analyze(duplicate, manifest)['status'], 'INCOMPLETE')
        broken = [{**rows[0], 'status': 'INVALID'}]
        self.assertEqual(c.analyze(broken, manifest)['status'], 'INVALID')


if __name__ == '__main__':
    unittest.main()
