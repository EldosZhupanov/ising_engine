"""Frozen EXP001 descriptive ablation and explicitly scoped continuation screens."""
from collections import defaultdict
import csv
import hashlib
import json
from pathlib import Path
import statistics as st
import sys
import unittest

from run_exp001 import FAMILIES, INSTANCE_SEEDS, SOLVER_SEEDS, design


def mean(xs):
    return st.mean(xs) if xs else None


def restricted_target(row):
    value = row["time_target_ms"]
    return 100.0 if value == "NA" else min(100.0, float(value))


def family_screen(rows, family, candidate):
    blocks = defaultdict(dict)
    for row in rows:
        if row["mode"] == "wall" and row["family"] == family:
            blocks[int(row["instance_seed"]), int(row["solver_seed"])][int(row["arm"])] = row
    valid = {key: arms for key, arms in blocks.items()
             if set(arms) == set(range(8)) and all(95 <= float(r["elapsed_ms"]) <= 105 for r in arms.values())}
    per_instance = []
    for instance in INSTANCE_SEEDS:
        pairs = [(arms[0], arms[candidate]) for (i, _), arms in valid.items() if i == instance]
        if len(pairs) < 3:
            return {"family": family, "candidate": candidate, "verdict": "INSTRUMENT_INCONCLUSIVE", "valid_blocks": len(valid), "valid_per_instance": {str(i): sum(k[0] == i for k in valid) for i in INSTANCE_SEEDS}}
        advantages = [(float(a["energy"]) - float(b["energy"])) / max(1, abs(float(a["target"]))) for a, b in pairs]
        per_instance.append({"instance_seed": instance, "n_pairs": len(pairs), "quality_advantage": mean(advantages),
                             "baseline_rmst_ms": mean([restricted_target(a) for a, _ in pairs]),
                             "candidate_rmst_ms": mean([restricted_target(b) for _, b in pairs]),
                             "energy_difference": mean([float(b["energy"]) - float(a["energy"]) for a, b in pairs]),
                             "reduced_every_seed": all(int(b["residual_n"]) < int(b["n"]) for _, b in pairs)})
    quality = mean([i["quality_advantage"] for i in per_instance])
    base = mean([i["baseline_rmst_ms"] for i in per_instance])
    cand = mean([i["candidate_rmst_ms"] for i in per_instance])
    ratio = cand / base if base > 0 else None
    quality_pass = quality >= 0.01 and all(i["quality_advantage"] > 0 for i in per_instance)
    time_pass = ratio is not None and ratio <= 0.75 and all(i["energy_difference"] <= 0 for i in per_instance)
    reduction_pass = candidate != 3 or all(i["reduced_every_seed"] for i in per_instance)
    passed = reduction_pass and (quality_pass or time_pass)
    return {"family": family, "candidate": candidate, "valid_blocks": len(valid), "instances": per_instance,
            "quality_advantage": quality, "baseline_rmst_ms": base, "candidate_rmst_ms": cand, "rmst_ratio": ratio,
            "quality_pass": quality_pass, "time_pass": time_pass, "reduction_pass": reduction_pass,
            "verdict": "POSITIVE_CONTROL_ONLY" if passed and family == "cycle" else "CONTINUE_SCREEN" if passed else "ARCHIVE_ON_THIS_FAMILY"}


def analyze(out):
    closure = json.loads((out / "complete.json").read_text())
    for name, key in (("raw.tsv", "raw_sha256"), ("targets.tsv", "targets_sha256")):
        if hashlib.sha256((out / name).read_bytes()).hexdigest() != closure[key]:
            raise RuntimeError(f"closed artifact changed: {name}")
    with (out / "raw.tsv").open() as f:
        rows = list(csv.DictReader(f, delimiter="\t"))
    keys = [(r["mode"], r["family"], int(r["n"]), int(r["instance_seed"]), int(r["solver_seed"]), int(r["arm"]), int(r["exchanges"])) for r in rows]
    if len(rows) != 4200 or len(set(keys)) != 4200 or set(keys) != set(design()):
        raise RuntimeError("missing, duplicate or unexpected rows")
    groups = defaultdict(list)
    for r in rows:
        n = int(r["n"])
        if len(r["state"]) != n or not set(r["state"]) <= {"0", "1"}:
            raise RuntimeError("invalid binary assignment")
        if int(r["success"]) != (float(r["energy"]) <= float(r["target"])):
            raise RuntimeError("inconsistent success indicator")
        if float(r["time_best_ms"]) > float(r["elapsed_ms"]) + 1e-5:
            raise RuntimeError("best observed after algorithm end")
        if r["time_target_ms"] != "NA" and float(r["time_target_ms"]) > float(r["elapsed_ms"]) + 1e-5:
            raise RuntimeError("target observed after algorithm end")
        groups[r["mode"], r["family"], n, int(r["exchanges"]), int(r["arm"])].append(r)
    header = "mode family n exchanges arm runs energy_mean gap_to_reference_mean success_probability within_instance_seed_variance elapsed_ms_median elapsed_ms_mean time_best_ms_median target_rmst100_ms process_cpu_s_mean peak_rss_kib_median residual_n_mean nominal_proposals_mean table_evals_mean degree_scans_mean pair_queries_mean pair_moves_mean common_improvement_sum_mean pair_improvement_sum_mean common_gain_scans_mean pair_stage_gain_scans_mean".split()
    with (out / "summary.tsv").open("w") as f:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n"); writer.writerow(header)
        for key, rs in sorted(groups.items()):
            values = lambda field: [float(r[field]) for r in rs]
            per_seed = defaultdict(list)
            for r in rs:
                per_seed[r["instance_seed"]].append(float(r["energy"]))
            seed_var = mean([st.variance(v) for v in per_seed.values()])
            row = [*key, len(rs), mean(values("energy")), mean([(float(r["energy"])-float(r["target"]))/max(1, abs(float(r["target"]))) for r in rs]),
                   mean(values("success")), seed_var, st.median(values("elapsed_ms")), mean(values("elapsed_ms")), st.median(values("time_best_ms")),
                   mean([restricted_target(r) for r in rs]) if key[0] == "wall" else "NA",
                   mean([float(r["process_user_s"])+float(r["process_system_s"]) for r in rs]), st.median(values("peak_rss_kib")),
                   mean(values("residual_n")), mean(values("nominal_spin_proposals_upper")), mean(values("table_evals")), mean(values("degree_scans")),
                   mean(values("pair_queries")), mean(values("pair_moves")), mean(values("common_improvement_sum")), mean(values("pair_improvement_sum")),
                   mean(values("common_gain_scans")), mean([float(r["gain_scans"])-float(r["common_gain_scans"]) for r in rs])]
            writer.writerow(row)
    by_key = dict(zip(keys, rows))
    contrasts = defaultdict(list)
    for key, r in by_key.items():
        mode, family, n, iseed, seed, arm, exchanges = key
        if arm == 0:
            continue
        base = by_key[mode, family, n, iseed, seed, 0, exchanges]
        contrasts[mode, family, n, exchanges, arm].append((base, r))
    with (out / "paired_ablations.tsv").open("w") as f:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n")
        writer.writerow("mode family n exchanges candidate wins ties losses mean_quality_advantage mean_elapsed_ratio".split())
        for key, pairs in sorted(contrasts.items()):
            d = [float(a["energy"])-float(b["energy"]) for a, b in pairs]
            writer.writerow([*key, sum(v>0 for v in d), sum(v==0 for v in d), sum(v<0 for v in d),
                             mean([v/max(1,abs(float(a["target"]))) for v,(a,_) in zip(d,pairs)]),
                             mean([float(b["elapsed_ms"])/float(a["elapsed_ms"]) for a,b in pairs])])
    screens = [family_screen(rows, family, candidate) for family in FAMILIES for candidate in (3, 4)]
    (out / "primary_screens.json").write_text(json.dumps(screens, indent=2))
    verdicts = {}
    for hypothesis, candidate in (("H01", 3), ("H07", 4)):
        eligible = [s for s in screens if s["candidate"] == candidate and s["family"] != "cycle"]
        verdicts[hypothesis] = "CONTINUE_SCREEN" if any(s["verdict"] == "CONTINUE_SCREEN" for s in eligible) else "INSTRUMENT_INCONCLUSIVE" if all(s["verdict"] == "INSTRUMENT_INCONCLUSIVE" for s in eligible) else "ARCHIVE_WITH_UNRESOLVED_FAMILIES" if any(s["verdict"] == "INSTRUMENT_INCONCLUSIVE" for s in eligible) else "ARCHIVE"
    (out / "verdict.json").write_text(json.dumps(verdicts, indent=2))
    print(json.dumps(verdicts), flush=True)
    for s in screens:
        print(s["family"], s["candidate"], s["verdict"], "valid=", s["valid_blocks"], "quality=", s.get("quality_advantage"), "rmst_ratio=", s.get("rmst_ratio"), flush=True)


class ScreenTests(unittest.TestCase):
    @staticmethod
    def fixture():
        return [{"mode":"wall", "family":"field", "instance_seed":str(i), "solver_seed":str(s), "arm":str(a),
                 "elapsed_ms":"100", "energy":"-102" if a==4 else "-100", "target":"-100", "time_target_ms":"50", "residual_n":"128", "n":"128"}
                for i in INSTANCE_SEEDS for s in SOLVER_SEEDS for a in range(8)]

    def test_quality_screen_uses_all_instances(self):
        rows = self.fixture()
        self.assertEqual(family_screen(rows,"field",4)["verdict"],"CONTINUE_SCREEN")
        for r in rows:
            if r["instance_seed"] == "1103" and r["arm"] == "4":
                r["energy"] = "-99"
        self.assertEqual(family_screen(rows,"field",4)["verdict"],"ARCHIVE_ON_THIS_FAMILY")

    def test_any_arm_overrun_excludes_entire_block(self):
        rows = self.fixture()
        for r in rows:
            if r["instance_seed"] == "1101" and r["solver_seed"] in {"2101","2102","2103"} and r["arm"] == "7":
                r["elapsed_ms"] = "106"
        self.assertEqual(family_screen(rows,"field",4)["verdict"],"INSTRUMENT_INCONCLUSIVE")

    def test_failures_are_not_discarded_from_target_time(self):
        self.assertEqual(restricted_target({"time_target_ms":"NA"}),100)
        self.assertEqual(restricted_target({"time_target_ms":"120"}),100)
        rows = self.fixture()
        for r in rows:
            r["energy"]="-99"
            r["time_target_ms"]="NA" if r["solver_seed"] != "2101" else "1"
        s=family_screen(rows,"field",4)
        self.assertEqual(s["candidate_rmst_ms"],80.2)
        self.assertEqual(s["verdict"],"ARCHIVE_ON_THIS_FAMILY")


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        unittest.main(argv=[sys.argv[0]])
    elif len(sys.argv) == 2:
        analyze(Path(sys.argv[1]))
    else:
        raise SystemExit("usage: analyze_exp001.py OUTPUT | --selftest")
