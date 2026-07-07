#!/usr/bin/env python3
"""Analyze compare_openjij.py results: tables, plots, significance, report.

Reads  benchmark_suite/results/raw/openjij_vs_ultimate.jsonl
Writes benchmark_suite/results/tables/summary.csv
       benchmark_suite/results/tables/per_instance.csv
       benchmark_suite/results/plots/gap_by_instance.png
       benchmark_suite/results/plots/win_matrix.png
       benchmark_suite/results/report_openjij.md

Gap definition: per instance, reference = verified best-known native
objective when available (QPLIB official index), else the best native value
found by ANY run of EITHER solver (marked "best-found" in the tables).
gap = (ref - native) / |ref|  (>= 0; 0 = reached the reference).

Run inside benchmark-env (needs matplotlib):
  benchmark-env/bin/python3 benchmark_suite/scripts/analyze_openjij.py
"""

import json
import math
import statistics
from collections import defaultdict
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

REPO = Path(__file__).resolve().parent.parent.parent
RESULTS = REPO / "benchmark_suite/results"
RAW = RESULTS / "raw/openjij_vs_ultimate.jsonl"


def wilcoxon_two_sided(a, b):
    """Wilcoxon signed-rank (normal approx., tie-corrected). Returns p."""
    diffs = [x - y for x, y in zip(a, b) if x != y]
    n = len(diffs)
    if n < 2:
        return 1.0
    order = sorted(range(n), key=lambda k: abs(diffs[k]))
    ranks = [0.0] * n
    tie = 0.0
    i = 0
    while i < n:
        j = i + 1
        while j < n and abs(abs(diffs[order[j]]) - abs(diffs[order[i]])) < 1e-12:
            j += 1
        g = j - i
        avg = (i + 1 + j) / 2.0
        for k in order[i:j]:
            ranks[k] = avg
        tie += g**3 - g
        i = j
    w_plus = sum(ranks[k] for k in range(n) if diffs[k] > 0)
    mean = n * (n + 1) / 4.0
    var = n * (n + 1) * (2 * n + 1) / 24.0 - tie / 48.0
    if var <= 0:
        return 1.0
    d = w_plus - mean
    z = (d - math.copysign(0.5, d)) / math.sqrt(var)
    p = 2.0 * (1.0 - 0.5 * (1.0 + math.erf(abs(z) / math.sqrt(2))))
    return min(max(p, 0.0), 1.0)


def main():
    rows = [json.loads(l) for l in open(RAW, encoding="utf-8") if l.strip()]
    (RESULTS / "tables").mkdir(parents=True, exist_ok=True)
    (RESULTS / "plots").mkdir(parents=True, exist_ok=True)

    # ---- reference per instance -------------------------------------------
    by_inst = defaultdict(list)
    for r in rows:
        by_inst[r["instance"]].append(r)
    ref, ref_src = {}, {}
    for inst, rs in by_inst.items():
        bk = rs[0]["best_known"]
        maximize = rs[0]["maximize"]
        best_found = (max if maximize else min)(r["native"] for r in rs)
        if bk is not None:
            ref[inst], ref_src[inst] = bk, "best-known"
        else:
            ref[inst], ref_src[inst] = best_found, "best-found"
    for r in rows:
        rf = ref[r["instance"]]
        sign = 1.0 if r["maximize"] else -1.0
        r["gap"] = max(0.0, sign * (rf - r["native"]) / max(abs(rf), 1.0))

    # ---- per (instance, solver) table -------------------------------------
    cells = defaultdict(list)
    for r in rows:
        cells[(r["class"], r["instance"], r["solver"])].append(r)
    per_rows = []
    for (cls, inst, solver), rs in sorted(cells.items()):
        natives = [r["native"] for r in rs]
        gaps = [r["gap"] for r in rs]
        maximize = rs[0]["maximize"]
        per_rows.append(
            {
                "class": cls,
                "instance": inst,
                "n": rs[0]["n"],
                "solver": solver,
                "runs": len(rs),
                "best_native": (max if maximize else min)(natives),
                "mean_native": statistics.mean(natives),
                "std_native": statistics.pstdev(natives),
                "mean_gap": statistics.mean(gaps),
                "best_gap": min(gaps),
                "mean_wall_s": statistics.mean(r["wall_s"] for r in rs),
                "reference": ref[inst],
                "ref_source": ref_src[inst],
            }
        )
    with open(RESULTS / "tables/per_instance.csv", "w", encoding="utf-8") as fh:
        cols = list(per_rows[0].keys())
        fh.write(",".join(cols) + "\n")
        for pr in per_rows:
            fh.write(",".join(f"{pr[c]:.6g}" if isinstance(pr[c], float) else str(pr[c]) for c in cols) + "\n")

    # ---- head-to-head per instance -----------------------------------------
    insts = sorted(by_inst, key=lambda i: (by_inst[i][0]["class"], by_inst[i][0]["n"]))
    wins = {"Ultimate": 0, "OpenJij": 0, "tie": 0}
    h2h = []
    for inst in insts:
        u = next(p for p in per_rows if p["instance"] == inst and p["solver"] == "Ultimate")
        o = next(p for p in per_rows if p["instance"] == inst and p["solver"] == "OpenJij")
        du, do = u["mean_gap"], o["mean_gap"]
        if abs(du - do) < 1e-12:
            verdict = "tie"
        elif du < do:
            verdict = "Ultimate"
        else:
            verdict = "OpenJij"
        wins[verdict] += 1
        h2h.append((inst, u, o, verdict))

    # paired significance across instances on mean gap
    p_wilcoxon = wilcoxon_two_sided(
        [x[1]["mean_gap"] for x in h2h], [x[2]["mean_gap"] for x in h2h]
    )

    # ---- summary csv -------------------------------------------------------
    with open(RESULTS / "tables/summary.csv", "w", encoding="utf-8") as fh:
        fh.write("solver,instances,mean_gap,median_gap,mean_wall_s,wins\n")
        for solver in ("Ultimate", "OpenJij"):
            ps = [p for p in per_rows if p["solver"] == solver]
            fh.write(
                f"{solver},{len(ps)},{statistics.mean(p['mean_gap'] for p in ps):.6f},"
                f"{statistics.median(p['mean_gap'] for p in ps):.6f},"
                f"{statistics.mean(p['mean_wall_s'] for p in ps):.3f},{wins[solver]}\n"
            )

    # ---- plots ---------------------------------------------------------------
    labels = [f"{i}\n(n={by_inst[i][0]['n']})" for i in insts]
    ug = [next(p["mean_gap"] for p in per_rows if p["instance"] == i and p["solver"] == "Ultimate") for i in insts]
    og = [next(p["mean_gap"] for p in per_rows if p["instance"] == i and p["solver"] == "OpenJij") for i in insts]
    x = range(len(insts))
    fig, ax = plt.subplots(figsize=(14, 5.5))
    ax.bar([i - 0.2 for i in x], ug, width=0.4, label="Ising Engine (Ultimate)", color="#4e79a7")
    ax.bar([i + 0.2 for i in x], og, width=0.4, label="OpenJij SASampler", color="#f28e2b")
    ax.set_xticks(list(x))
    ax.set_xticklabels(labels, rotation=45, ha="right", fontsize=7)
    ax.set_ylabel("mean optimality gap (lower is better)")
    ax.set_title("Equal wall-clock budget: mean gap per instance (5 seeds small/medium, 3 large)")
    ax.legend()
    ax.set_yscale("symlog", linthresh=1e-4)
    fig.tight_layout()
    fig.savefig(RESULTS / "plots/gap_by_instance.png", dpi=150)

    fig2, ax2 = plt.subplots(figsize=(8, 5))
    ns = [by_inst[i][0]["n"] for i in insts]
    ax2.scatter(ns, ug, label="Ising Engine", color="#4e79a7", s=45)
    ax2.scatter(ns, og, label="OpenJij", color="#f28e2b", s=45, marker="s")
    ax2.set_xscale("log")
    ax2.set_yscale("symlog", linthresh=1e-4)
    ax2.set_xlabel("problem size n")
    ax2.set_ylabel("mean optimality gap")
    ax2.set_title("Solution quality vs. problem size (equal time budget)")
    ax2.legend()
    fig2.tight_layout()
    fig2.savefig(RESULTS / "plots/quality_vs_size.png", dpi=150)

    # ---- report -------------------------------------------------------------
    rep = []
    rep.append("# Ising Engine vs. OpenJij — objective benchmark\n")
    rep.append(
        "Protocol: identical instances and seeds; both solvers' returned states "
        "re-scored with one canonical energy function; **equal wall-clock budget "
        "per size class** (small 1 s, medium 3 s, large 8 s), calibrated per "
        "instance per solver with excluded probes. Engine: production "
        "`UltimateSolver` at defaults (untouched). OpenJij: `SASampler`, single "
        "deep anneal filling the budget (its quality-optimal configuration). "
        "No parameter tuning for either side.\n"
    )
    rep.append(f"Head-to-head (mean gap per instance): **Ising Engine {wins['Ultimate']}"
               f" — {wins['tie']} tie — {wins['OpenJij']} OpenJij**"
               f" (Wilcoxon two-sided p = {p_wilcoxon:.4f}, n = {len(h2h)} instances)\n")
    rep.append("## Per-instance results\n")
    rep.append("| class | instance | n | ref (src) | Engine best | OpenJij best | Engine mean gap | OpenJij mean gap | winner |")
    rep.append("|---|---|---|---|---|---|---|---|---|")
    for inst, u, o, verdict in h2h:
        rep.append(
            f"| {u['class']} | {inst} | {u['n']} | {u['reference']:g} ({u['ref_source']}) "
            f"| {u['best_native']:g} | {o['best_native']:g} "
            f"| {u['mean_gap']:.5f} | {o['mean_gap']:.5f} | {verdict} |"
        )
    rep.append("\n## Wall-clock check (budget adherence)\n")
    rep.append("| class | budget s | Engine mean s | OpenJij mean s |")
    rep.append("|---|---|---|---|")
    for cls in ("small", "medium", "large"):
        us = [p["mean_wall_s"] for p in per_rows if p["class"] == cls and p["solver"] == "Ultimate"]
        os_ = [p["mean_wall_s"] for p in per_rows if p["class"] == cls and p["solver"] == "OpenJij"]
        if us:
            budget = {"small": 1.0, "medium": 3.0, "large": 8.0}[cls]
            rep.append(f"| {cls} | {budget} | {statistics.mean(us):.2f} | {statistics.mean(os_):.2f} |")
    rep.append("\n![gap by instance](plots/gap_by_instance.png)\n")
    rep.append("![quality vs size](plots/quality_vs_size.png)\n")
    (RESULTS / "report_openjij.md").write_text("\n".join(rep), encoding="utf-8")
    print(f"analyzed {len(rows)} runs, {len(insts)} instances")
    print(f"wins: {wins}  wilcoxon p={p_wilcoxon:.4f}")
    print(f"wrote tables/, plots/, report_openjij.md under {RESULTS}")


if __name__ == "__main__":
    main()
