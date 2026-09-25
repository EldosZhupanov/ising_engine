"""Synthetic checks for EXP-007W analysis; no holdout input is opened."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import analyze
import run


class WeightedMisChecks(unittest.TestCase):
    def test_runner_rejects_wrong_input_hash(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "input.json"
            path.write_bytes(b"one\n")
            expected = hashlib.sha256(b"one\n").hexdigest()
            self.assertEqual(run.verify_input(path, expected), expected)
            path.write_bytes(b"two\n")
            with self.assertRaisesRegex(ValueError, "frozen input hash mismatch"):
                run.verify_input(path, expected)

    def test_weighted_witness_and_collision(self):
        graph = {"n": 2, "weights": [1, 2], "edges": [[0, 1]]}
        self.assertEqual(analyze.direct(graph, "10"), (1, 0, -1))
        self.assertEqual(analyze.direct(graph, "01"), (2, 0, -2))
        self.assertEqual(analyze.direct(graph, "11"), (3, 1, 8))
        self.assertEqual(analyze.exact_sign_p(10, 0), 1 / 1024)
        self.assertEqual(analyze.exact_sign_p(0, 0), 1.0)
        self.assertEqual(analyze.exact_sign_p(1, 1), 0.75)

    def test_complete_synthetic_run_and_energy_tamper(self):
        graphs = []
        structural_rows = []
        timed_rows = []
        for g in range(6):
            name = f"er_n2_p100_g{g}"
            graphs.append({"name": name, "n": 2, "edge_percent": 100,
                           "weights": [1, 2], "edges": [[0, 1]]})
            for s in range(10):
                structural_rows.append({
                    "evaluation_id": "EXP-007W", "graph": name,
                    "start_index": s, "seed": 9_000_000 + 1_000 * g + s,
                    "fixed": [], "state": "10", "model_energy": -1,
                    "direct_energy": -1, "min_single_delta": 1,
                    "min_pair_delta": -1,
                    "improving_pair": {"u": 0, "v": 1, "delta": -1},
                    "valid": True,
                })
            for c in range(10):
                seed = 7_000_000 + 100_000 * g + 10_000 * c
                sol = {"seed": seed, "state": "01", "weight": 2,
                       "collisions": 0, "model_energy": -2,
                       "duration_ms": 10, "elapsed_ms": 10, "valid": True}
                arm = {"completed": 1, "late_completions": 0,
                       "elapsed_ms": 2000, "best_weight": 2,
                       "valid": True, "solutions": [sol]}
                timed_rows.append({"evaluation_id": "EXP-007W", "graph": name,
                                   "n": 2, "edges": 1, "campaign": c,
                                   "baseline": arm, "candidate": arm,
                                   "delta": 0, "valid": True})
        dataset = {"experiment_id": "EXP-007W", "generator": "SHA-256-v1",
                   "instances": graphs}
        encoded = (json.dumps(dataset, sort_keys=True, separators=(",", ":")) + "\n").encode()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            data = root / "instances.json"
            structural = root / "structural.jsonl"
            raw = root / "raw.jsonl"
            data.write_bytes(encoded)
            structural.write_text("".join(json.dumps(row) + "\n" for row in structural_rows))
            raw.write_text("".join(json.dumps(row) + "\n" for row in timed_rows))
            with patch.object(analyze, "SPECS", ((2, 100),) * 6), \
                 patch.object(analyze, "INPUT_SHA256", hashlib.sha256(encoded).hexdigest()):
                result = analyze.analyze(structural, raw, data)
                self.assertEqual(result["structural_hits"], 60)
                self.assertEqual(result["ties"], 60)
                self.assertEqual(result["decision_h2"], "NO-GO")
                timed_rows[0]["baseline"]["solutions"] = [dict(sol, model_energy=-3)]
                raw.write_text("".join(json.dumps(row) + "\n" for row in timed_rows))
                result = analyze.analyze(structural, raw, data)
                self.assertEqual(len(result["invalid_cells"]), 1)
                self.assertEqual(result["decision_h2"], "NO-GO")


if __name__ == "__main__":
    unittest.main()
