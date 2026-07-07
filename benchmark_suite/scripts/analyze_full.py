#!/usr/bin/env python3
"""Publication-grade analysis of run_full_benchmark.py results.

Reads  benchmark_suite/results/raw/full_benchmark.jsonl
Writes benchmark_suite/results/tables/full_summary.csv
       benchmark_suite/results/tables/full_per_instance.csv
       benchmark_suite/results/tables/full_per_dataset.csv
       benchmark_suite/results/plots/full_*.{png,svg}   (6 figures)
       benchmark_suite/results/REPORT.md                (paper-style report)

Statistics: per-instance mean/median/best/std gap with bootstrap 95% CIs;
cross-instance win/loss/tie, Wilcoxon signed-rank, exact sign test
(binomial), and Cliff's delta effect size — overall and per dataset.

Run inside benchmark-env:
  benchmark-env/bin/python3 benchmark_suite/scripts/analyze_full.py
"""

import json
import math
import platform
import random
import statistics as st
import subprocess
from collections import defaultdict
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

REPO = Path(__file__).resolve().parent.parent.parent
RESULTS = REPO / "benchmark_suite/results"
RAW = RESULTS / "raw/full_benchmark.jsonl"
SOLVERS = ("Ultimate", "OpenJij")
COLORS = {"Ultimate": "#4e79a7", "OpenJij": "#f28e2b"}
LABELS = {"Ultimate": "Ising Engine (UltimateSolver)", "OpenJij": "OpenJij SASampler"}


# ---------------------------------------------------------------------------
# Statistics helpers (stdlib-only)
# ---------------------------------------------------------------------------


def wilcoxon_two_sided(a, b):
    diffs = [x - y for x, y in zip(a, b) if x != y]
    n = len(diffs)
    if n < 2:
        return 1.0, n
    order = sorted(range(n), key=lambda k: abs(diffs[k]))
    ranks = [0.0] * n
    tie = 0.0
    i = 0
    while i < n:
        j = i + 1
        while j < n and abs(abs(diffs[order[j]]) - abs(diffs[order[i]])) < 1e-15:
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
        return 1.0, n
    d = w_plus - mean
    z = (d - math.copysign(0.5, d)) / math.sqrt(var)
    p = 2.0 * (1.0 - 0.5 * (1.0 + math.erf(abs(z) / math.sqrt(2))))
    return min(max(p, 0.0), 1.0), n


def sign_test_two_sided(a, b):
    """Exact binomial sign test on paired samples (ties dropped)."""
    wins_a = sum(1 for x, y in zip(a, b) if x < y)  # a better (lower gap)
    wins_b = sum(1 for x, y in zip(a, b) if x > y)
    n = wins_a + wins_b
    if n == 0:
        return 1.0, 0
    k = min(wins_a, wins_b)
    tail = sum(math.comb(n, i) for i in range(k + 1)) / 2.0**n
    return min(1.0, 2.0 * tail), n


def cliffs_delta(a, b):
    """Cliff's delta between two samples: P(a<b) - P(a>b) over all pairs
    (negative = a stochastically smaller = better for gaps)."""
    gt = lt = 0
    for x in a:
        for y in b:
            if x > y:
                gt += 1
            elif x < y:
                lt += 1
    n = len(a) * len(b)
    return (gt - lt) / n if n else 0.0


def delta_magnitude(d):
    d = abs(d)
    if d < 0.147:
        return "negligible"
    if d < 0.33:
        return "small"
    if d < 0.474:
        return "medium"
    return "large"


def bootstrap_ci(data, n_boot=2000, conf=0.95, seed=7):
    if not data:
        return (float("nan"), float("nan"))
    rng = random.Random(seed)
    means = sorted(
        st.mean(rng.choices(data, k=len(data))) for _ in range(n_boot)
    )
    lo = means[int((1 - conf) / 2 * (n_boot - 1))]
    hi = means[int((1 + conf) / 2 * (n_boot - 1))]
    return lo, hi


# ---------------------------------------------------------------------------
# Load + score
# ---------------------------------------------------------------------------


def load_rows():
    """Load raw runs, dedupe by (instance, solver, seed) keeping the LAST
    occurrence (resume may re-run partially complete instances)."""
    seen = {}
    for line in open(RAW, encoding="utf-8"):
        if not line.strip():
            continue
        r = json.loads(line)
        seen[(r["instance"], r["solver"], r["seed"])] = r
    rows = list(seen.values())
    by_inst = defaultdict(list)
    for r in rows:
        by_inst[r["instance"]].append(r)
    ref, ref_src = {}, {}
    for inst, rs in by_inst.items():
        bk, mx = rs[0]["best_known"], rs[0]["maximize"]
        bf = (max if mx else min)(r["native"] for r in rs)
        if bk is not None:
            ref[inst], ref_src[inst] = bk, "best-known"
        else:
            ref[inst], ref_src[inst] = bf, "best-found"
    for r in rows:
        s = 1.0 if r["maximize"] else -1.0
        r["gap"] = max(0.0, s * (ref[r["instance"]] - r["native"]) / max(abs(ref[r["instance"]]), 1.0))
    return rows, by_inst, ref, ref_src


def per_instance(rows):
    cells = defaultdict(list)
    for r in rows:
        cells[(r["instance"], r["solver"])].append(r)
    out = {}
    for (inst, solver), rs in cells.items():
        gaps = [r["gap"] for r in rs]
        lo, hi = bootstrap_ci(gaps)
        out[(inst, solver)] = {
            "dataset": rs[0]["dataset"], "class": rs[0]["class"], "n": rs[0]["n"],
            "runs": len(rs), "mean_gap": st.mean(gaps), "median_gap": st.median(gaps),
            "best_gap": min(gaps), "std_gap": st.pstdev(gaps),
            "ci_lo": lo, "ci_hi": hi,
            "mean_wall": st.mean(r["wall_s"] for r in rs),
            "budget": rs[0]["budget_s"],
            "best_native": (max if rs[0]["maximize"] else min)(r["native"] for r in rs),
        }
    return out


def h2h(per, insts):
    wins = {"Ultimate": 0, "OpenJij": 0, "tie": 0}
    verdicts = {}
    for i in insts:
        du, do = per[(i, "Ultimate")]["mean_gap"], per[(i, "OpenJij")]["mean_gap"]
        v = "tie" if abs(du - do) < 1e-12 else ("Ultimate" if du < do else "OpenJij")
        wins[v] += 1
        verdicts[i] = v
    return wins, verdicts


# ---------------------------------------------------------------------------
# Plots (each saved as PNG and SVG)
# ---------------------------------------------------------------------------


def save(fig, name):
    for ext in ("png", "svg"):
        fig.savefig(RESULTS / f"plots/full_{name}.{ext}", dpi=150, bbox_inches="tight")
    plt.close(fig)


def make_plots(rows, per, insts):
    ns = {i: per[(i, "Ultimate")]["n"] for i in insts}
    order = sorted(insts, key=lambda i: (ns[i], i))
    ug = [per[(i, "Ultimate")]["mean_gap"] for i in order]
    og = [per[(i, "OpenJij")]["mean_gap"] for i in order]

    # 1. gap vs instance (ordered by n)
    fig, ax = plt.subplots(figsize=(16, 5))
    x = range(len(order))
    ax.plot(x, ug, ".", ms=5, color=COLORS["Ultimate"], label=LABELS["Ultimate"])
    ax.plot(x, og, ".", ms=5, color=COLORS["OpenJij"], label=LABELS["OpenJij"])
    ax.set_yscale("symlog", linthresh=1e-5)
    ax.set_xlabel(f"instance (ordered by n; {len(order)} instances)")
    ax.set_ylabel("mean optimality gap")
    ax.set_title("Mean gap per instance under equal wall-clock budget")
    ax.legend()
    save(fig, "gap_by_instance")

    # 2. runtime vs size
    fig, ax = plt.subplots(figsize=(8, 5))
    for s in SOLVERS:
        xs = [per[(i, s)]["n"] for i in order]
        ys = [per[(i, s)]["mean_wall"] for i in order]
        ax.scatter(xs, ys, s=14, alpha=0.6, color=COLORS[s], label=LABELS[s])
    bs = sorted({per[(i, "Ultimate")]["budget"] for i in order})
    for b in bs:
        ax.axhline(b, color="#999", lw=0.7, ls="--")
    ax.set_xscale("log")
    ax.set_xlabel("problem size n")
    ax.set_ylabel("mean wall time per run (s)")
    ax.set_title("Budget adherence: wall time vs size (dashed = requested budgets)")
    ax.legend()
    save(fig, "runtime_vs_size")

    # 3. gap distribution
    fig, ax = plt.subplots(figsize=(8, 5))
    import numpy as np

    bins = np.logspace(-6, 0, 40)
    for s, g in (("Ultimate", ug), ("OpenJij", og)):
        clipped = [max(v, 1e-6) for v in g]
        ax.hist(clipped, bins=bins, alpha=0.55, color=COLORS[s], label=LABELS[s])
    ax.set_xscale("log")
    ax.set_xlabel("mean optimality gap (clipped at 1e-6)")
    ax.set_ylabel("instances")
    ax.set_title("Distribution of per-instance mean gaps")
    ax.legend()
    save(fig, "gap_distribution")

    # 4. cumulative wins by size
    fig, ax = plt.subplots(figsize=(9, 5))
    cu = co = 0
    cus, cos = [], []
    for i in order:
        du, do = per[(i, "Ultimate")]["mean_gap"], per[(i, "OpenJij")]["mean_gap"]
        if abs(du - do) >= 1e-12:
            if du < do:
                cu += 1
            else:
                co += 1
        cus.append(cu)
        cos.append(co)
    ax.plot(range(len(order)), cus, color=COLORS["Ultimate"], label=LABELS["Ultimate"])
    ax.plot(range(len(order)), cos, color=COLORS["OpenJij"], label=LABELS["OpenJij"])
    ax.set_xlabel("instance rank (ordered by n)")
    ax.set_ylabel("cumulative wins")
    ax.set_title("Cumulative wins as problem size grows")
    ax.legend()
    save(fig, "cumulative_wins")

    # 5. boxplots per dataset
    datasets = sorted({per[(i, "Ultimate")]["dataset"] for i in order})
    fig, ax = plt.subplots(figsize=(12, 5.5))
    data, positions, colors = [], [], []
    for di, d in enumerate(datasets):
        dinsts = [i for i in order if per[(i, "Ultimate")]["dataset"] == d]
        for si, s in enumerate(SOLVERS):
            data.append([max(per[(i, s)]["mean_gap"], 1e-6) for i in dinsts])
            positions.append(di * 2.5 + si * 0.9)
            colors.append(COLORS[s])
    bp = ax.boxplot(data, positions=positions, widths=0.7, patch_artist=True,
                    showfliers=True, flierprops=dict(ms=2.5))
    for patch, c in zip(bp["boxes"], colors):
        patch.set_facecolor(c)
        patch.set_alpha(0.7)
    ax.set_yscale("log")
    ax.set_xticks([di * 2.5 + 0.45 for di in range(len(datasets))])
    ax.set_xticklabels(datasets, rotation=15)
    ax.set_ylabel("mean gap (log, clipped 1e-6)")
    ax.set_title("Gap distribution per dataset (left: Engine, right: OpenJij)")
    save(fig, "boxplot_per_dataset")

    # 6. scatter engine vs openjij
    fig, ax = plt.subplots(figsize=(7, 7))
    xs = [max(per[(i, "Ultimate")]["mean_gap"], 1e-6) for i in order]
    ys = [max(per[(i, "OpenJij")]["mean_gap"], 1e-6) for i in order]
    cmap = {"gset": "#e15759", "orlib": "#4e79a7", "biqmac": "#59a14f",
            "qplib": "#b07aa1", "dimacs_maxcut": "#f28e2b", "snap": "#76b7b2"}
    for d in sorted(cmap):
        sel = [k for k, i in enumerate(order) if per[(i, "Ultimate")]["dataset"] == d]
        if sel:
            ax.scatter([xs[k] for k in sel], [ys[k] for k in sel], s=16,
                       color=cmap[d], label=d, alpha=0.75)
    lims = [1e-6, max(max(xs), max(ys)) * 1.5]
    ax.plot(lims, lims, "k--", lw=0.8)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlim(lims)
    ax.set_ylim(lims)
    ax.set_xlabel("Ising Engine mean gap")
    ax.set_ylabel("OpenJij mean gap")
    ax.set_title("Per-instance mean gap: points above the diagonal favor the Engine")
    ax.legend(fontsize=8)
    save(fig, "scatter_engine_vs_openjij")


# ---------------------------------------------------------------------------
# Tables + report
# ---------------------------------------------------------------------------


def stats_block(per, insts):
    """All cross-instance statistics for a set of instances."""
    a = [per[(i, "Ultimate")]["mean_gap"] for i in insts]
    b = [per[(i, "OpenJij")]["mean_gap"] for i in insts]
    wins, _ = h2h(per, insts)
    wp, wn = wilcoxon_two_sided(a, b)
    sp, sn = sign_test_two_sided(a, b)
    d = cliffs_delta(a, b)
    return {
        "n_instances": len(insts), "wins": wins,
        "mean_gap": {"Ultimate": st.mean(a), "OpenJij": st.mean(b)},
        "median_gap": {"Ultimate": st.median(a), "OpenJij": st.median(b)},
        "ci": {"Ultimate": bootstrap_ci(a), "OpenJij": bootstrap_ci(b)},
        "wilcoxon_p": wp, "wilcoxon_n": wn, "sign_p": sp, "sign_n": sn,
        "cliffs_delta": d, "delta_mag": delta_magnitude(d),
    }


def hardware_software():
    def sh(cmd):
        try:
            return subprocess.run(cmd, shell=True, capture_output=True, text=True,
                                  timeout=10).stdout.strip()
        except Exception:  # noqa: BLE001
            return "?"

    cpu = sh("lscpu | grep 'Model name' | sed 's/.*: *//'") or platform.processor()
    cores = sh("nproc")
    mem = sh("free -g | awk '/Mem:/{print $2}'")
    rustc = sh("rustc --version")
    import openjij as oj
    import numpy

    return {
        "cpu": cpu, "cores": cores, "mem_gb": mem,
        "os": f"{platform.system()} {platform.release()}",
        "python": platform.python_version(), "rustc": rustc,
        "openjij": getattr(oj, "__version__", "unknown"),
        "numpy": numpy.__version__,
        "engine_commit": sh("git -C " + str(REPO) + " rev-parse --short HEAD"),
    }


def fmt_p(p):
    return f"{p:.2e}" if p < 1e-3 else f"{p:.4f}"


def main():
    rows, by_inst, ref, ref_src = load_rows()
    per = per_instance(rows)
    insts = sorted(
        {i for (i, s) in per if (i, "Ultimate") in per and (i, "OpenJij") in per},
        key=lambda i: (per[(i, "Ultimate")]["n"], i),
    )
    (RESULTS / "tables").mkdir(parents=True, exist_ok=True)
    (RESULTS / "plots").mkdir(parents=True, exist_ok=True)

    overall = stats_block(per, insts)
    datasets = sorted({per[(i, "Ultimate")]["dataset"] for i in insts})
    per_ds = {d: stats_block(per, [i for i in insts if per[(i, "Ultimate")]["dataset"] == d])
              for d in datasets}
    classes = ("small", "medium", "large")
    per_cls = {c: stats_block(per, [i for i in insts if per[(i, "Ultimate")]["class"] == c])
               for c in classes if any(per[(i, "Ultimate")]["class"] == c for i in insts)}

    # ---- tables ----
    with open(RESULTS / "tables/full_per_instance.csv", "w", encoding="utf-8") as fh:
        fh.write("instance,dataset,class,n,solver,runs,mean_gap,median_gap,best_gap,"
                 "std_gap,ci_lo,ci_hi,mean_wall_s,budget_s,best_native,reference,ref_source\n")
        for i in insts:
            for s in SOLVERS:
                p = per[(i, s)]
                fh.write(f"{i},{p['dataset']},{p['class']},{p['n']},{s},{p['runs']},"
                         f"{p['mean_gap']:.6g},{p['median_gap']:.6g},{p['best_gap']:.6g},"
                         f"{p['std_gap']:.6g},{p['ci_lo']:.6g},{p['ci_hi']:.6g},"
                         f"{p['mean_wall']:.3f},{p['budget']},{p['best_native']:.6g},"
                         f"{ref[i]:.6g},{ref_src[i]}\n")

    def block_csv_row(name, blk):
        w = blk["wins"]
        return (f"{name},{blk['n_instances']},{w['Ultimate']},{w['OpenJij']},{w['tie']},"
                f"{blk['mean_gap']['Ultimate']:.6f},{blk['mean_gap']['OpenJij']:.6f},"
                f"{blk['median_gap']['Ultimate']:.6f},{blk['median_gap']['OpenJij']:.6f},"
                f"{fmt_p(blk['wilcoxon_p'])},{fmt_p(blk['sign_p'])},"
                f"{blk['cliffs_delta']:.3f},{blk['delta_mag']}\n")

    with open(RESULTS / "tables/full_per_dataset.csv", "w", encoding="utf-8") as fh:
        fh.write("scope,instances,engine_wins,openjij_wins,ties,engine_mean_gap,"
                 "openjij_mean_gap,engine_median_gap,openjij_median_gap,"
                 "wilcoxon_p,sign_p,cliffs_delta,effect\n")
        fh.write(block_csv_row("overall", overall))
        for c, blk in per_cls.items():
            fh.write(block_csv_row(f"class:{c}", blk))
        for d, blk in per_ds.items():
            fh.write(block_csv_row(d, blk))

    with open(RESULTS / "tables/full_summary.csv", "w", encoding="utf-8") as fh:
        fh.write("solver,instances,mean_gap,ci_lo,ci_hi,median_gap,wins\n")
        for s in SOLVERS:
            ci = overall["ci"][s]
            fh.write(f"{s},{overall['n_instances']},{overall['mean_gap'][s]:.6f},"
                     f"{ci[0]:.6f},{ci[1]:.6f},{overall['median_gap'][s]:.6f},"
                     f"{overall['wins'][s]}\n")

    make_plots(rows, per, insts)

    # ---- budget adherence ----
    adherence = []
    for c in classes:
        ci = [i for i in insts if per[(i, "Ultimate")]["class"] == c]
        if not ci:
            continue
        b = per[(ci[0], "Ultimate")]["budget"]
        mu = st.mean(per[(i, "Ultimate")]["mean_wall"] for i in ci)
        mo = st.mean(per[(i, "OpenJij")]["mean_wall"] for i in ci)
        adherence.append((c, b, mu, mo, 100 * abs(mu - mo) / b,
                          100 * abs(mu - b) / b, 100 * abs(mo - b) / b))

    # ---- report ----
    hw = hardware_software()
    seeds_used = {c: sorted({r["seed"] for r in rows if r["class"] == c}) for c in classes}
    unsupported = (RESULTS / "unsupported.txt").read_text(encoding="utf-8").strip()
    total_runs = len(rows)

    rep = []
    A = rep.append
    A("# Benchmarking the Ising Engine against OpenJij on 6 Public Libraries\n")
    A("## 1. Dataset summary\n")
    A("| dataset | instances | n range | kind | source |")
    A("|---|---|---|---|---|")
    for d in datasets:
        di = [i for i in insts if per[(i, "Ultimate")]["dataset"] == d]
        nmin = min(per[(i, "Ultimate")]["n"] for i in di)
        nmax = max(per[(i, "Ultimate")]["n"] for i in di)
        kind = "MaxCut" if d in ("gset", "dimacs_maxcut", "snap") else (
            "mixed" if d == "biqmac" else "QUBO/BQP")
        A(f"| {d} | {len(di)} | {nmin}–{nmax} | {kind} | see registry |")
    A(f"\nTotal: **{len(insts)} instances, {total_runs} solver runs**. "
      "Instance acquisition, SHA256 verification, and parser validation are in "
      "`benchmark_suite/` (registry: `configs/benchmark_registry.yaml`).\n")
    A(f"Unsupported/skipped instances: {unsupported if unsupported != '(none)' else 'none'}.\n")
    A("## 2. Hardware and software\n")
    A(f"- CPU: {hw['cpu']} ({hw['cores']} logical cores), {hw['mem_gb']} GB RAM, {hw['os']} (WSL2)")
    A(f"- Ising Engine: Rust, `{hw['rustc']}`, release build, commit `{hw['engine_commit']}`")
    A(f"- OpenJij {hw['openjij']}, Python {hw['python']}, NumPy {hw['numpy']}\n")
    A("## 3. Benchmark protocol\n")
    A("- **Solvers**: production `UltimateSolver` at library defaults "
      "(QPBO/probing presolve + SIMD parallel tempering, untouched) vs. "
      "`openjij.SASampler` (single deep anneal at default schedule — its "
      "quality-optimal configuration). No parameter tuning for either side.")
    A("- **Objective identity**: one canonical energy function "
      "(`ising_bench.Qubo.energy`) re-scores the raw states returned by both "
      "solvers. Parse conventions validated against published optima "
      "(OR-Library bqp50/100/250/500, Biq Mac gka1a = −3414, DIMACS torus "
      "sg3dl051000 = 110) and the official QPLIB solution file (282.0000).")
    A("- **Equal time budget** per size class: small (n<200) 1 s, medium "
      "(n<1000) 3 s, large 8 s per run. Per (instance, solver): adaptive "
      "two-point affine calibration plus measured verification runs, "
      "corrected until within 5% of budget (probes/verifications excluded "
      "from results).")
    A(f"- **Seeds**: identical lists for both solvers — small {seeds_used.get('small', [])[:3]}…"
      f"{len(seeds_used.get('small', []))} seeds, medium {len(seeds_used.get('medium', []))}, "
      f"large {len(seeds_used.get('large', []))}.")
    A("- **Reference for gaps**: verified best-known objective when available "
      "(QPLIB official index), else best value found by any run of either "
      "solver; gap = |ref − value| / |ref|, direction-aware, clamped at 0.\n")
    A("## 4. Statistical methodology\n")
    A("Per instance: mean/median/best/std of the gap over seeds with "
      "percentile-bootstrap 95% CIs (2000 resamples). Cross-instance "
      "(paired on per-instance mean gap): Wilcoxon signed-rank "
      "(normal approximation, tie-corrected, continuity-corrected), exact "
      "two-sided binomial sign test, and Cliff's delta with the standard "
      "|d| thresholds (0.147/0.33/0.474).\n")
    A("## 5. Results\n")
    A("### Overall\n")
    w = overall["wins"]
    A(f"- Head-to-head: **Engine {w['Ultimate']} — {w['tie']} ties — {w['OpenJij']} OpenJij** "
      f"over {overall['n_instances']} instances")
    for s in SOLVERS:
        ci = overall["ci"][s]
        A(f"- {LABELS[s]}: mean gap {overall['mean_gap'][s]:.5f} "
          f"[95% CI {ci[0]:.5f}, {ci[1]:.5f}], median {overall['median_gap'][s]:.5f}")
    A(f"- Wilcoxon signed-rank: p = {fmt_p(overall['wilcoxon_p'])} "
      f"(n = {overall['wilcoxon_n']} non-tied pairs)")
    A(f"- Sign test: p = {fmt_p(overall['sign_p'])} (n = {overall['sign_n']})")
    A(f"- Cliff's delta: {overall['cliffs_delta']:.3f} ({overall['delta_mag']}); "
      "negative favors the Engine\n")
    A("### By size class and dataset\n")
    A("| scope | inst | E wins | OJ wins | ties | E mean gap | OJ mean gap | Wilcoxon p | sign p | Cliff's δ |")
    A("|---|---|---|---|---|---|---|---|---|---|")

    def row(name, blk):
        w = blk["wins"]
        return (f"| {name} | {blk['n_instances']} | {w['Ultimate']} | {w['OpenJij']} | {w['tie']} "
                f"| {blk['mean_gap']['Ultimate']:.5f} | {blk['mean_gap']['OpenJij']:.5f} "
                f"| {fmt_p(blk['wilcoxon_p'])} | {fmt_p(blk['sign_p'])} "
                f"| {blk['cliffs_delta']:.2f} ({blk['delta_mag']}) |")

    for c, blk in per_cls.items():
        A(row(f"class {c}", blk))
    for d, blk in per_ds.items():
        A(row(d, blk))
    A("\n### Budget adherence (fairness check)\n")
    A("| class | requested s | Engine mean s | OpenJij mean s | E-vs-OJ diff % | E dev % | OJ dev % |")
    A("|---|---|---|---|---|---|---|")
    for (c, b, mu, mo, dd, du_, do_) in adherence:
        A(f"| {c} | {b:.0f} | {mu:.2f} | {mo:.2f} | {dd:.1f}% | {du_:.1f}% | {do_:.1f}% |")
    A("\n### Figures\n")
    for name, cap in (
        ("gap_by_instance", "Mean gap per instance (ordered by n)"),
        ("runtime_vs_size", "Budget adherence: wall time vs size"),
        ("gap_distribution", "Distribution of per-instance mean gaps"),
        ("cumulative_wins", "Cumulative wins as size grows"),
        ("boxplot_per_dataset", "Gap boxplots per dataset"),
        ("scatter_engine_vs_openjij", "Per-instance scatter (above diagonal = Engine better)"),
    ):
        A(f"![{cap}](plots/full_{name}.png)\n")
    A("## 6. Discussion\n")
    A("(Numbers above; qualitative reading:) The Engine's advantage "
      "concentrates on QUBO/BQP instances — dense couplings and strong "
      "roof-duality persistencies play to its presolve + wide replica "
      "population. OpenJij's advantage concentrates on large sparse MaxCut, "
      "where at a fixed wall budget a single deep annealing schedule beats "
      "a wide population of shorter schedules, and the Engine's "
      "dense-oriented SIMD layout cannot exploit degree-bounded sparsity.\n")
    A("## 7. Threats to validity\n")
    A("- Wall-clock equality on shared hardware compares whole solvers, not "
      "algorithms: the Engine uses multiple cores + SIMD, OpenJij is "
      "single-core. Per-core normalization would shift sparse-MaxCut "
      "further toward OpenJij.")
    A("- Best-found references (all non-QPLIB datasets) can flatter whichever "
      "solver found them; gaps there are relative, not absolute.")
    A("- The OR-Library gka problems and Biq Mac gka*.sparse are the same "
      "underlying instances in two libraries (different conventions), so "
      "they carry double weight in cross-instance tests.")
    A("- One host, one run per seed; no CPU-frequency pinning (WSL2). "
      "Same-host criterion drift up to ~9% has been observed on this machine.")
    A("- Calibration verification targets 5%; residual budget mismatch is "
      "reported per class above.\n")
    A("## 8. Reproducibility\n")
    A("```bash")
    A("python3 benchmark_suite/scripts/download_all_benchmarks.py   # data + SHA256")
    A("python3 benchmark_suite/scripts/selftest.py                  # convention checks")
    A("cargo build --release --bin solve_instance")
    A("benchmark-env/bin/python3 benchmark_suite/scripts/run_full_benchmark.py")
    A("benchmark-env/bin/python3 benchmark_suite/scripts/analyze_full.py")
    A("```")
    A("Seeds are fixed (1..k per class); instance files are SHA256-pinned in "
      "each dataset's metadata.json; raw per-run records are in "
      "`results/raw/full_benchmark.jsonl`.\n")
    A("## 9. Appendix: every tested instance\n")
    A("| instance | dataset | n | class | ref (src) | E mean gap | OJ mean gap |")
    A("|---|---|---|---|---|---|---|")
    for i in insts:
        pu, po = per[(i, "Ultimate")], per[(i, "OpenJij")]
        A(f"| {i} | {pu['dataset']} | {pu['n']} | {pu['class']} | {ref[i]:.6g} ({ref_src[i]}) "
          f"| {pu['mean_gap']:.5f} | {po['mean_gap']:.5f} |")

    (RESULTS / "REPORT.md").write_text("\n".join(rep), encoding="utf-8")
    print(f"analyzed {total_runs} runs over {len(insts)} instances")
    print(f"overall wins: {overall['wins']}, wilcoxon p={fmt_p(overall['wilcoxon_p'])}, "
          f"sign p={fmt_p(overall['sign_p'])}, cliffs d={overall['cliffs_delta']:.3f}")
    print(f"wrote REPORT.md, tables/full_*.csv, plots/full_*.png+svg under {RESULTS}")


if __name__ == "__main__":
    main()
