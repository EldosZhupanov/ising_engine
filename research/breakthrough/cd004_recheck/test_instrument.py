"""Calibration checks excluded from the untouched seed block."""

import random
import unittest

import instrument


class CoreLearningChecks(unittest.TestCase):
    def test_legacy_zero_field_counterexample_and_repair(self):
        n = 8
        J, h = instrument.legacy.generate_frustrated_instance(n, "complete_SK", 1)
        random.seed(10001)
        start, initial_energy = instrument.legacy.find_heuristic_incumbent(
            J, h, n, num_restarts=1)
        optimum, _ = instrument.exact_oracle(J, h)
        old = instrument.legacy.solve_conflict_driven_bnb(J, h, n, initial_energy)
        self.assertEqual((initial_energy, optimum, old[4]), (-10, -12, -10))
        self.assertEqual(instrument.solve(J, h, start, False)["energy"], optimum)
        corrected = instrument.solve(J, h, start, True)
        self.assertEqual(corrected["energy"], optimum)
        self.assertGreater(corrected["cores"], 0)
        self.assertGreater(corrected["cache_hits"], 0)
        self.assertEqual(instrument.direct_energy(J, h, corrected["state"]), optimum)

    def test_small_random_fields_against_independent_oracle(self):
        for n in range(4, 8):
            for seed in range(20):
                rng = random.Random(1000 * n + seed)
                J = [[0.0] * n for _ in range(n)]
                h = [float(rng.choice((-2, -1, 0, 1, 2))) for _ in range(n)]
                for i in range(n):
                    for j in range(i + 1, n):
                        J[i][j] = J[j][i] = float(rng.choice((-1, 0, 0, 1)))
                start = [rng.choice((-1, 1)) for _ in range(n)]
                optimum, _ = instrument.exact_oracle(J, h)
                for learn in (False, True):
                    result = instrument.solve(J, h, start, learn)
                    self.assertEqual(result["energy"], optimum, (n, seed, learn))
                    self.assertEqual(instrument.direct_energy(J, h, result["state"]),
                                     optimum, (n, seed, learn))


if __name__ == "__main__":
    unittest.main()
