import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import subprocess

from campaign import analyze, best_event, check_witness, energy, incumbent, supervise, wilson


class QualificationTests(unittest.TestCase):
    def test_checker_failures_preserve_evidence(self):
        for error in [FileNotFoundError("absent"),
                      subprocess.TimeoutExpired(["checker"], 5, output=b"partial", stderr=b"error")]:
            with patch("campaign.subprocess.run", side_effect=error):
                result = check_witness(["checker"])
            self.assertIsNone(result["returncode"])
            self.assertIn("error", result)
        self.assertEqual(result["stdout"], "partial")

    def test_energy_and_input_validation(self):
        self.assertEqual(energy("0000", 4), 14)
        self.assertEqual(energy("0001", 4), 2)
        for bits in ["000", "00000", "000x", [0, 0, 0, 0]]:
            with self.assertRaises(ValueError):
                energy(bits, 4)

    def test_claimed_energy_must_match(self):
        with self.assertRaises(ValueError):
            incumbent("INC 0 0000", 4, 0.01, 1)
        with self.assertRaises(ValueError):
            incumbent("INC NaN 0000", 4, 0.01, 1)
        self.assertFalse(incumbent("INC 14 0000", 4, 1.01, 1)["eligible"])

    def test_optimum_receipt_stops_early(self):
        row = supervise([sys.executable, "-c",
            "import time; print('INC 2 0001',flush=True); time.sleep(2)"], 4, 2, 1)
        self.assertEqual(row["stop"], "optimum")
        self.assertEqual(best_event(row)["energy"], 2)
        self.assertEqual(row["errors"], [])

    def test_partial_line_is_not_a_witness(self):
        row = supervise([sys.executable, "-c",
            "import sys,time; print('INC 14 0000',flush=True); sys.stdout.write('INC 2 0001'); sys.stdout.flush(); time.sleep(2)"], 4, 2, 0.2)
        self.assertEqual(row["stop"], "deadline")
        self.assertEqual(best_event(row)["energy"], 14)
        self.assertIn("INC 2 0001", row["stdout"])

    def test_bad_energy_and_process_errors_persist(self):
        row = supervise([sys.executable, "-c",
            "import time; print('INC 0 0000',flush=True); time.sleep(2)"], 4, 2, 1)
        self.assertEqual(row["stop"], "invalid_witness")
        self.assertIn("INC 0 0000", row["stdout"])
        self.assertTrue(row["errors"])
        row = supervise(["/no/such/labs/solver"], 4, 2, 1)
        self.assertEqual(row["stop"], "launch_failure")
        self.assertTrue(row["errors"])
        row = supervise([sys.executable, "-c", "raise SystemExit(7)"], 4, 2, 1)
        self.assertEqual(row["returncode"], 7)
        self.assertTrue(row["errors"])

    def test_late_witness_and_tampered_timestamp(self):
        row = {"n": 4, "budget": 1, "events": [
            incumbent("INC 14 0000", 4, 0.1, 1),
            incumbent("INC 2 0001", 4, 1.1, 1)]}
        self.assertEqual(best_event(row)["energy"], 14)
        row["events"][1]["eligible"] = True
        with self.assertRaises(AssertionError):
            best_event(row)

    def test_missing_cells_fail_not_excluded(self):
        with tempfile.TemporaryDirectory() as folder:
            result = analyze(Path(folder))
        self.assertFalse(result["pt_qualified"])
        self.assertEqual(len(result["arms"]["pt"]["40"]["failures"]), 10)
        self.assertEqual(result["paired"]["40"]["invalid_pairs"], 10)
        self.assertAlmostEqual(wilson(0, 10)[1], 0.2775327998628892)


if __name__ == "__main__":
    unittest.main()
