import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from campaign import (ARMS, TARGETS, SEEDS, SOURCE_PATHS, analyze, checker_valid, command_for, design, distribution,
                      holm, q001, restart_tts99, valid_endpoint, validate_provenance, wilcoxon_exact)


def make_complete(directory):
    for n, target in TARGETS.items():
        bits = "0" * n
        energy = q001.energy(bits, n)
        for seed in SEEDS:
            for arm in ARMS:
                row = {"arm": arm, "n": n, "target": target, "seed": seed,
                       "command": command_for(arm, arm, n, seed),
                       "budget": 10.0, "stop": "deadline", "stop_seconds": 10.01,
                       "errors": [], "events": [{"energy": energy, "bits": bits,
                           "seconds": .01, "eligible": True}],
                       "checker": {"returncode": 20,
                                   "stdout": f"LABS k={n} E(S)={energy} seq={bits}"}}
                (directory / f"{n}_{seed}_{arm}.json").write_text(json.dumps(row))
    (directory / "complete.json").write_text(json.dumps({"cells": 60, "commit": "a" * 40}))
    (directory / "environment.json").write_text(json.dumps({
        "commit": "a" * 40, "smoke": False, "targets": TARGETS, "seeds": SEEDS, "budget": 10.0,
        "source_sha256": {path: "b" * 64 for path in SOURCE_PATHS},
        "binaries": {arm: arm for arm in (*ARMS, "checker")},
        "binary_sha256": {arm: "c" * 64 for arm in (*ARMS, "checker")}}))


class CampaignTests(unittest.TestCase):
    def test_exact_wilcoxon_ties_zeros_and_holm(self):
        self.assertEqual(wilcoxon_exact([0] * 10), 1)
        self.assertEqual(wilcoxon_exact([1] * 10), 2 / 1024)
        self.assertEqual(wilcoxon_exact([1, -1]), 1)
        self.assertEqual(wilcoxon_exact([1, 2, 3]), .25)
        self.assertEqual(wilcoxon_exact([-1, -2, -3]), .25)
        self.assertEqual(holm({"40": .001, "50": .02, "60": .8}),
                         {"40": .003, "50": .04, "60": .8})

    def test_descriptive_and_restart_metrics(self):
        stats = distribution([1, 2, 3, 4])
        self.assertEqual(stats["mean"], 2.5)
        self.assertEqual(stats["median"], 2.5)
        self.assertEqual((stats["q25"], stats["q75"], stats["iqr"]), (1.75, 3.25, 1.5))
        self.assertAlmostEqual(stats["sample_sd"], (5 / 3)**.5)
        self.assertIsNone(restart_tts99(0))
        self.assertEqual(restart_tts99(10), 10)
        self.assertEqual(restart_tts99(5), 70)

    def test_checker_energy_match_is_exact(self):
        self.assertTrue(checker_valid({"returncode": 20, "stdout": "LABS k=40 E(S)=110 seq="}, 40, 110))
        self.assertFalse(checker_valid({"returncode": 20, "stdout": "LABS k=40 E(S)=110 seq="}, 40, 11))
        self.assertFalse(checker_valid({"returncode": 21, "stdout": "LABS k=40 E(S)=110 seq="}, 40, 110))

    def test_complete_valid_but_unsuccessful_campaign(self):
        with tempfile.TemporaryDirectory() as folder, patch("campaign.git_digest", return_value="b" * 64):
            root = Path(folder)
            make_complete(root)
            result = analyze(root)
        self.assertEqual(result["verdict"], "NOT_QUALIFIED")
        self.assertEqual(result["failures"], [])
        self.assertEqual(result["paired"]["40"]["ties"], 10)
        self.assertFalse(result["paired"]["40"]["local_superiority_gate"])
        self.assertEqual(result["arms"]["memetic"]["40"]["hits"], 0)

    def test_missing_or_invalid_either_arm_invalidates_whole_campaign(self):
        for change in ("missing", "checker", "deadline", "early_deadline", "command", "energy"):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as folder, patch("campaign.git_digest", return_value="b" * 64):
                root = Path(folder)
                make_complete(root)
                path = root / f"40_{SEEDS[0]}_lmats.json"
                row = json.loads(path.read_text())
                if change == "missing":
                    path.unlink()
                else:
                    if change == "checker":
                        row["checker"]["returncode"] = 21
                    elif change == "deadline":
                        row["stop_seconds"] = 10.2
                    elif change == "early_deadline":
                        row["stop_seconds"] = 9.9
                    elif change == "command":
                        row["command"][2] = "1"
                    else:
                        row["events"][0]["energy"] = 108
                    path.write_text(json.dumps(row))
                result = analyze(root)
                self.assertEqual(result["verdict"], "INSTRUMENT_INVALID")
                self.assertEqual(result["arms"]["lmats"]["40"]["runs"], 10)
                self.assertEqual(result["paired"]["40"]["invalid_pairs"], 1)
                self.assertTrue(all(not row["local_superiority_gate"] for row in result["paired"].values()))

    def test_provenance_required_and_commit_consistent(self):
        with tempfile.TemporaryDirectory() as folder, patch("campaign.git_digest", return_value="b" * 64):
            root = Path(folder)
            make_complete(root)
            self.assertEqual(validate_provenance(root)["commit"], "a" * 40)
            (root / "complete.json").write_text(json.dumps({"cells": 60, "commit": "d" * 40}))
            self.assertEqual(analyze(root)["verdict"], "INSTRUMENT_INVALID")
            (root / "environment.json").unlink()
            self.assertEqual(analyze(root)["verdict"], "INSTRUMENT_INVALID")
        with tempfile.TemporaryDirectory() as folder, patch("campaign.git_digest", return_value="e" * 64):
            root = Path(folder)
            make_complete(root)
            self.assertEqual(analyze(root)["verdict"], "INSTRUMENT_INVALID")

    def test_after_stop_event_cannot_be_credited(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            make_complete(root)
            row = json.loads((root / f"40_{SEEDS[0]}_memetic.json").read_text())
            row.update(stop="optimum", stop_seconds=.005)
            with self.assertRaisesRegex(ValueError, "after stop"):
                valid_endpoint(row, "memetic", 40, SEEDS[0])

    def test_smoke_manifest_uses_its_own_design(self):
        self.assertEqual(design("smoke"), ({20: 26}, [820001, 820002], 1.0))
        self.assertEqual(design("run"), (TARGETS, SEEDS, 10.0))


if __name__ == "__main__":
    unittest.main()
