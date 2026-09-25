import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import study
from study import count_fields


class FieldCountTest(unittest.TestCase):
    def test_identical_boundary_columns_collapse(self):
        matrix = [[1, 1, 1, 1]]
        self.assertEqual(count_fields(matrix, [0], [1, 2, 3]), 4)

    def test_independent_boundary_columns_do_not_collapse(self):
        matrix = [[1, 0, 1, 0], [0, 1, 0, 1]]
        self.assertEqual(count_fields(matrix, [0, 1], [2, 3]), 4)

    def test_zero_cross_field_has_one_image(self):
        matrix = [[1, 0, 0]]
        self.assertEqual(count_fields(matrix, [0], [1, 2]), 1)

    def test_analyzer_checks_probe_and_counts_fields(self):
        with TemporaryDirectory() as directory:
            path = Path(directory) / 'sample.dat'
            path.write_text('1 4\n1 1 1 1 2\n')
            checksum = study.digest(path)
            record = {
                'instance': path.name, 'input_sha256': checksum, 'status': 'ok',
                'probe': {'instance': path.name, 'm': 1, 'n': 4, 'fixed': [],
                          'free': [0, 1, 2, 3], 'energy_zero': 4,
                          'energy_one': 4, 'presolve_ms': 1.0},
            }
            with patch.object(study, 'BASE', Path(directory)), \
                 patch.object(study, 'INPUTS', {path.name: checksum}):
                row = study.analyze_row(record)
                self.assertEqual((row['raw_contexts'], row['distinct_fields']), (4, 3))
                record['probe']['fixed'] = [[0, 0], [0, 1]]
                with self.assertRaisesRegex(ValueError, 'duplicate'):
                    study.analyze_row(record)

    def test_incomplete_raw_is_inconclusive(self):
        with TemporaryDirectory() as directory:
            path = Path(directory) / 'raw.jsonl'
            path.write_text('')
            with patch.object(study, 'verify_inputs'), \
                 patch.object(study, 'INPUTS', {'sample.dat': 'sha'}):
                self.assertEqual(study.analyze(path)['decision'], 'INCONCLUSIVE')
            path.write_text('{bad json}\n')
            with patch.object(study, 'verify_inputs'):
                self.assertEqual(study.analyze(path)['decision'], 'INCONCLUSIVE')
            path.write_text('null\n')
            with patch.object(study, 'verify_inputs'):
                self.assertEqual(study.analyze(path)['decision'], 'INCONCLUSIVE')


if __name__ == '__main__':
    unittest.main()
