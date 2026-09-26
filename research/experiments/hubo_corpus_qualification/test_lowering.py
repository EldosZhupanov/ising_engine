import hashlib
import itertools
import json
import math
from pathlib import Path
import unittest

from lowering import canonical, lower
from prepare_inputs import encoded, prepared

HERE = Path(__file__).resolve().parent
FIXTURES = [
    (4, [[2, [0, 1, 2]], [-3, [0, 1, 3]], [2, [1, 2, 3]]]),
    (5, [[7, [0, 1, 2, 3]], [-5, [0, 1, 2, 4]], [3, [0, 2]], [-2, []]]),
    (5, [[5, [0, 1, 2, 3]], [-7, [0, 1, 2, 4]], [3, [1, 2, 3, 4]]]),
    (4, [[-9, [0, 1, 2, 3]], [2, [0]], [7, []]]),
    (4, [[9, [0, 1, 2, 3]], [-2, [0]], [-7, []]]),
    (3, [[3, [0, 1, 2]], [-3, [2, 0, 1]], [-4, [0, 1]], [1, []]]),
    (2, [[-5, []], [4, [0]], [-2, [1]], [3, [0, 1]]]),
    (2, []),
]


def energy(terms, bits):
    # Independent integer evaluation; no incremental/compiler energy helper.
    return sum(w for w, variables in terms if all(bits[v] for v in variables))


class Checks(unittest.TestCase):
    def test_serialized_certificate_rejects_duplicate_corruption(self):
        from verify_phase_a import check_certificate
        original = [[1, [0]], [1, [2]]]
        result = json.loads(json.dumps(lower(3, original)))
        check_certificate(3, original, result)
        result['terms'] = [[1, [0]], [-1, [2]]] + result['terms']
        self.assertNotEqual(energy(result['terms'], (1, 0, 0)), energy(original, (1, 0, 0)))
        with self.assertRaises(AssertionError):
            check_certificate(3, original, result)
        original = [[-5, [0, 1, 2]]]
        result = lower(3, original)
        result['steps'][0]['affected'] *= 2
        with self.assertRaises(AssertionError):
            check_certificate(3, original, result)

    def test_one_shot_variable_iterators_are_rejected(self):
        for variables in (iter([0, 1, 2]), (v for v in range(3)), {0, 1, 2}):
            with self.assertRaises(ValueError):
                lower(3, [(5, variables)])
        self.assertEqual(canonical(3, [(5, (0, 1, 2))]), {(0, 1, 2): 5})

    def test_exhaustive_original_and_auxiliary_minima(self):
        for n, terms in FIXTURES:
            for mode in ('local', 'global'):
                result = lower(n, terms, mode)
                self.assertLessEqual(result['n'], 12)
                for original in itertools.product((0, 1), repeat=n):
                    candidates = [energy(result['terms'], original + extra)
                                  for extra in itertools.product((0, 1), repeat=result['n']-n)]
                    self.assertEqual(min(candidates), energy(terms, original))
                    self.assertEqual(candidates.count(min(candidates)), 1)

    def test_conditional_minimum_at_every_nested_step(self):
        for n, terms in FIXTURES:
            for mode in ('local', 'global'):
                result = lower(n, terms, mode)
                for step in result['steps']:
                    u, v, z, m = [step[k] for k in ('u', 'v', 'z', 'penalty')]
                    self.assertLess(max(u, v), z)
                    for old in itertools.product((0, 1), repeat=z):
                        before = energy(step['affected'], old)
                        alternatives = []
                        for bit in (0, 1):
                            affected_new = sum(w*bit for w, vs in step['affected']
                                               if all(old[i] for i in vs if i not in (u, v)))
                            alternatives.append(affected_new + m*(old[u]*old[v]-2*old[u]*bit-2*old[v]*bit+3*bit))
                        self.assertEqual(min(alternatives), before)
                        self.assertGreaterEqual(alternatives[1-old[u]*old[v]] - before, 1)

    def test_pair_plan_bounds_and_actual_nesting(self):
        for n, terms in FIXTURES:
            local, glob = lower(n, terms), lower(n, terms, 'global')
            plans = lambda r: [(s['u'], s['v'], s['z'], s['affected']) for s in r['steps']]
            self.assertEqual(plans(local), plans(glob))
            self.assertEqual(local['n'], glob['n'])
            self.assertTrue(all(s['penalty'] <= local['global_penalty'] for s in local['steps']))
        result = lower(*FIXTURES[1])
        self.assertTrue(any(s['u'] >= 5 or s['v'] >= 5 for s in result['steps']))
        self.assertEqual([(s['u'], s['v'], s['z']) for s in result['steps']],
                         [(0, 1, 5), (2, 5, 6)])

    def test_insufficient_penalty_mutation_is_detected(self):
        terms = [[-5, [0, 1, 2]]]
        correct = lower(3, terms)
        # Remove the product penalty: z=1 now falsely improves x=(0,0,1).
        broken = [[-5, [2, 3]]]
        original = (0, 0, 1)
        self.assertEqual(min(energy(correct['terms'], original+(z,)) for z in (0, 1)), 0)
        self.assertNotEqual(min(energy(broken, original+(z,)) for z in (0, 1)), 0)

    def test_canonicalization_invalid_inputs_and_arbitrary_precision(self):
        self.assertEqual(canonical(3, [[2, [2, 0]], [-1, [0, 2]], [0, [1]]]), {(0, 2): 1})
        for n, terms in [(0, []), (True, []), (2, [[1.5, [0]]]), (2, [[True, [0]]]),
                         (2, [[1, [-1]]]), (2, [[1, [2]]]), (2, [[1, [True]]]),
                         (2, [[1, [0, 0]]]), (5, [[1, [0, 1, 2, 3, 4]]])]:
            with self.assertRaises(ValueError): lower(n, terms)
        with self.assertRaises(ValueError): lower(2, [], 'unregistered')
        big = 2**80
        result = lower(3, [[big, [0, 1, 2]]])
        self.assertEqual(result['steps'][0]['penalty'], big+1)

    def test_input_manifest_determinism_and_structure(self):
        manifest = json.loads((HERE/'inputs.json').read_text())
        cases = prepared()
        self.assertEqual(encoded(cases), (HERE/'cases.json').read_bytes())
        self.assertEqual(hashlib.sha256(encoded(cases)).hexdigest(), manifest['cases_sha256'])
        self.assertEqual(len(cases), 12)
        for case, entry in zip(cases, manifest['instances']):
            self.assertEqual(hashlib.sha256(encoded(case)).hexdigest(), entry['sha256'])
            n, degree = case['n'], case['degree']
            self.assertIn(n, (64, 128))
            self.assertEqual(len(case['spin_terms']), 4*n)
            self.assertEqual(sum(len(vs)==degree for _, vs in case['spin_terms']), 3*n)
            self.assertTrue(all(abs(w)==1 for w, _ in case['spin_terms']))
            self.assertEqual(len({tuple(vs) for _, vs in case['spin_terms']}), 4*n)
            for mask in (0, (1<<n)-1, 0x5555555555555555, 0xDEADBEEF):
                bits = [(mask>>i)&1 for i in range(n)]
                spin = sum(w * math.prod(2*bits[i]-1 for i in vs)
                           for w, vs in case['spin_terms'])
                self.assertEqual(spin, energy(case['terms'], bits))


if __name__ == '__main__':
    unittest.main()
