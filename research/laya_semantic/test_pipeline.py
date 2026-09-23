import math
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from pipeline import direct_energy, encode, greedy, probabilities, qubo_energy, states, verify_and_score, violations
from run_pilot import bridge_attempt


class EncodingTests(unittest.TestCase):
    def test_exact_encoding_and_penalty_feasibility(self):
        for p in ([0, 1, .5], [.9, .8, .1], [.2, .3, .7]):
            rules = [('exclude', 0, 1), ('implies', 1, 2)]
            req, penalty = encode(p, rules, 1)
            clipped = probabilities(p)
            scores = []
            for x in states(3):
                direct = direct_energy(x, clipped, rules, penalty)
                self.assertAlmostEqual(direct, qubo_energy(x, req), places=10)
                scores.append((direct,x))
            self.assertEqual(violations(min(scores)[1], rules), 0)

    def test_invalid_probabilities_and_rules_rejected(self):
        for p in ([], [math.nan], [math.inf], [-.1], [1.1], [True]):
            with self.assertRaises(ValueError): probabilities(p)
        for rules in ([('bad',0,1)], [('exclude',0,0)], [('implies',0,2)], [('exclude',0,1)]*2):
            with self.assertRaises(ValueError): encode([.1,.2], rules, 1)

    def test_greedy_can_lose_to_exact_without_violating_rules(self):
        p = [.9,.95]
        rules = [('exclude',0,1)]
        self.assertEqual(greedy(p,rules),[0,1])
        # Descending order tries dependent first, then prerequisite; one-pass stays suboptimal.
        p = [.95,.9]
        rules = [('implies',0,1)]
        self.assertEqual(greedy(p,rules),[0,1])
        req, _ = encode(p,rules,1)
        self.assertLess(qubo_energy([1,1],req),qubo_energy(greedy(p,rules),req))

    def test_independent_checker_catches_energy_corruption(self):
        g = {'id':99,'truth':[1,0],'rules':[('exclude',0,1)]}
        p = [.9,.2]
        req,m = encode(p,g['rules'],1099)
        spectrum = [direct_energy(x,p,g['rules'],m) for x in states(2)]
        best = states(2)[min(range(4),key=spectrum.__getitem__)]
        b = {'energy_spectrum':spectrum, **{a:{'state':best,'energy':min(spectrum)} for a in ('exact','reduced_exact','ultimate')}}
        self.assertEqual(verify_and_score(g,p,b)['exact']['correct'],2)
        for corrupt in (math.nan, math.inf, True, '1'):
            bad = copy.deepcopy(b)
            bad['ultimate']['energy'] = corrupt
            with self.assertRaises(ValueError): verify_and_score(g,p,bad)
            bad = copy.deepcopy(b)
            bad['energy_spectrum'][0] = corrupt
            with self.assertRaises(ValueError): verify_and_score(g,p,bad)
        b['ultimate']['energy'] += 1
        with self.assertRaisesRegex(ValueError,'energy mismatch'): verify_and_score(g,p,b)

    def test_failed_bridge_evidence_survives_parsing_and_process_failures(self):
        cases = [subprocess.CompletedProcess([],0,'not JSON','diagnostic'),
                 subprocess.CompletedProcess([],3,'partial','failed')]
        with tempfile.TemporaryDirectory() as temp:
            directory=Path(temp)
            for index,result in enumerate(cases):
                with patch('run_pilot.subprocess.run',return_value=result):
                    with self.assertRaises((RuntimeError,json.JSONDecodeError)):
                        bridge_attempt(Path('/unused'),{'linear':[1]},directory,0)
                saved=json.loads((directory/f'bridge_00_attempt_{index}.json').read_text())
                self.assertEqual(saved['stdout'],result.stdout)
                self.assertEqual(saved['stderr'],result.stderr)
                self.assertEqual(saved['request'],{'linear':[1]})
            with patch('run_pilot.subprocess.run',side_effect=subprocess.TimeoutExpired('bridge',120,output=b'partial')):
                with self.assertRaises(RuntimeError): bridge_attempt(Path('/unused'),{},directory,0)
            self.assertTrue(json.loads((directory/'bridge_00_attempt_2.json').read_text())['timed_out'])


if __name__ == '__main__': unittest.main()
