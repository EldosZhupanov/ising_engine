"""Local instrument checks; no held-out graph is opened."""

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import analyze
from analyze import read_graph
from fetch import git_blob_sha1


class FrozenToolChecks(unittest.TestCase):
    def test_git_blob_hash_matches_known_git_object(self):
        self.assertEqual(git_blob_sha1(b"hello\n"),
                         "ce013625030ba8dba906f756967f9e9ca394464a")

    def test_independent_dimacs_parser_rejects_duplicate(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "small.gph"
            path.write_text("p edge 3 2\ne 1 2\ne 2 3\n")
            self.assertEqual(read_graph(path), (3, {(0, 1), (1, 2)}))
            path.write_text("p edge 3 2\ne 1 2\ne 2 1\n")
            with self.assertRaises(ValueError):
                read_graph(path)

    def test_empty_arm_is_reported_as_no_go_without_dropping_rows(self):
        graph_bytes = b"p edge 3 2\ne 1 2\ne 2 3\n"
        files = {f"g{i}.gph": git_blob_sha1(graph_bytes) for i in range(6)}
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            for name in files:
                (folder / name).write_bytes(graph_bytes)
            rows = []
            for index, name in enumerate(files):
                for campaign in range(3):
                    seed = 5_000_000 + 100_000 * index + 10_000 * campaign
                    solution = {"seed": seed, "state": "101", "size": 2,
                                "collisions": 0, "model_energy": -2.0,
                                "elapsed_ms": 100.0, "valid": True}
                    arm = {"completed": 1, "late_completions": 0,
                           "best_size": 2, "valid": True, "solutions": [solution]}
                    baseline = dict(arm)
                    candidate = dict(arm)
                    invalid = index == 0 and campaign == 0
                    if invalid:
                        baseline = {"completed": 0, "late_completions": 1,
                                    "best_size": None, "valid": False, "solutions": []}
                    rows.append({"evaluation_id": "CD005-Q2", "graph": name,
                                 "campaign": campaign, "n": 3,
                                 "edges": 2, "baseline": baseline,
                                 "candidate": candidate,
                                 "delta": None if invalid else 0,
                                 "valid": not invalid})
            raw = folder / "raw.jsonl"
            raw.write_text("".join(json.dumps(row) + "\n" for row in rows))
            with patch.object(analyze, "FILES", files):
                result = analyze.analyze(raw, folder)
            self.assertEqual(result["cells"], 18)
            self.assertEqual(result["ties"], 17)
            self.assertEqual(len(result["invalid_cells"]), 1)
            self.assertEqual(result["decision"], "NO-GO")


if __name__ == "__main__":
    unittest.main()
