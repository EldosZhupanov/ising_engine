//! Output formats: raw + summary CSV, JSON, LaTeX booktabs tables, and
//! self-contained SVG plots (no plotting dependency). All artifacts land in
//! the configured output directory from a single run.

use super::adapters::Solver;
use super::{RunRecord, Significance, SuiteConfig, Summary};
use std::fmt::Write as _;

/// Write every artifact under `config.out_dir`.
pub fn write_all(
    config: &SuiteConfig,
    records: &[RunRecord],
    summaries: &[Summary],
    significance: &[Significance],
    skipped: &[&Solver],
) {
    let dir = &config.out_dir;
    std::fs::create_dir_all(dir).ok();
    let _ = super::adapters::ensure_bridge_scripts();
    let w = |name: &str, body: String| {
        std::fs::write(format!("{dir}/{name}"), body).ok();
    };
    w("results_raw.csv", raw_csv(records));
    w("summary.csv", summary_csv(summaries));
    w("results.json", json(summaries, significance));
    w("tables.tex", latex_tables(summaries, significance));
    w("plot_success.svg", svg_bars(summaries, Metric::Success));
    w("plot_gap.svg", svg_bars(summaries, Metric::Gap));
    w("report.md", markdown(summaries, significance, skipped));
}

fn raw_csv(records: &[RunRecord]) -> String {
    let mut s =
        String::from("instance,family,n,solver,seed,energy,native,wall_ms,reference,gap,success\n");
    for r in records {
        writeln!(
            s,
            "{},{},{},{},{},{:.6},{:.6},{:.4},{:.6},{:.6},{}",
            r.instance,
            r.family,
            r.n,
            r.solver,
            r.seed,
            r.energy,
            r.native,
            r.wall_ms,
            r.reference,
            r.gap,
            r.success as u8
        )
        .ok();
    }
    s
}

fn summary_csv(summaries: &[Summary]) -> String {
    let mut s = String::from(
        "family,solver,n_instances,n_runs,p_success,tts099,tts_lo,tts_hi,\
         mean_native,best_native,mean_gap,gap_lo,gap_hi,mean_ms\n",
    );
    for r in summaries {
        writeln!(
            s,
            "{},{},{},{},{:.4},{},{},{},{:.4},{:.4},{:.6},{:.6},{:.6},{:.4}",
            r.family,
            r.solver,
            r.n_instances,
            r.n_runs,
            r.p_success,
            fmt_tts(r.tts),
            fmt_tts(r.tts_lo),
            fmt_tts(r.tts_hi),
            r.mean_native,
            r.best_native,
            r.mean_gap,
            r.gap_lo,
            r.gap_hi,
            r.mean_ms
        )
        .ok();
    }
    s
}

fn fmt_tts(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.4}")
    } else {
        "inf".to_string()
    }
}

// --------------------------------------------------------------------------
// JSON
// --------------------------------------------------------------------------

fn json(summaries: &[Summary], significance: &[Significance]) -> String {
    let mut s = String::from("{\n  \"summaries\": [\n");
    for (i, r) in summaries.iter().enumerate() {
        writeln!(
            s,
            "    {{\"family\":\"{}\",\"solver\":\"{}\",\"n_instances\":{},\"n_runs\":{},\
             \"p_success\":{:.6},\"tts099\":{},\"tts_lo\":{},\"tts_hi\":{},\
             \"mean_native\":{:.6},\"best_native\":{:.6},\"mean_gap\":{:.6},\
             \"gap_lo\":{:.6},\"gap_hi\":{:.6},\"mean_ms\":{:.6}}}{}",
            r.family,
            r.solver,
            r.n_instances,
            r.n_runs,
            r.p_success,
            json_num(r.tts),
            json_num(r.tts_lo),
            json_num(r.tts_hi),
            r.mean_native,
            r.best_native,
            r.mean_gap,
            r.gap_lo,
            r.gap_hi,
            r.mean_ms,
            if i + 1 < summaries.len() { "," } else { "" }
        )
        .ok();
    }
    s.push_str("  ],\n  \"significance\": [\n");
    for (i, t) in significance.iter().enumerate() {
        writeln!(
            s,
            "    {{\"family\":\"{}\",\"a\":\"{}\",\"b\":\"{}\",\"metric\":\"{}\",\
             \"wilcoxon_p\":{:.6},\"t_p\":{:.6},\"n_pairs\":{},\"a_better\":{}}}{}",
            t.family,
            t.solver_a,
            t.solver_b,
            t.metric,
            t.wilcoxon_p,
            t.t_p,
            t.n_pairs,
            t.a_better,
            if i + 1 < significance.len() { "," } else { "" }
        )
        .ok();
    }
    s.push_str("  ]\n}\n");
    s
}

fn json_num(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.6}")
    } else {
        "null".to_string()
    }
}

// --------------------------------------------------------------------------
// LaTeX
// --------------------------------------------------------------------------

fn latex_tables(summaries: &[Summary], significance: &[Significance]) -> String {
    let mut s = String::new();
    s.push_str("% Requires \\usepackage{booktabs}\n");
    s.push_str("\\begin{table}[t]\n  \\centering\n  \\caption{Solver comparison: success probability, mean optimality gap (with 95\\% bootstrap CI), and TTS(0.99).}\n");
    s.push_str("  \\begin{tabular}{llrrrr}\n    \\toprule\n");
    s.push_str("    Family & Solver & $p_\\mathrm{succ}$ & Gap & Gap CI$_{95}$ & TTS$_{0.99}$ \\\\\n    \\midrule\n");
    for r in summaries {
        writeln!(
            s,
            "    {} & {} & {:.2} & {:.4} & [{:.4},{:.4}] & {} \\\\",
            latex_escape(&r.family),
            latex_escape(&r.solver),
            r.p_success,
            r.mean_gap,
            r.gap_lo,
            r.gap_hi,
            fmt_tts(r.tts)
        )
        .ok();
    }
    s.push_str("    \\bottomrule\n  \\end{tabular}\n\\end{table}\n\n");

    if !significance.is_empty() {
        s.push_str("\\begin{table}[t]\n  \\centering\n  \\caption{Paired significance of Ultimate vs.\\ each baseline (per family): Wilcoxon signed-rank and paired $t$-test $p$-values on per-instance mean objective.}\n");
        s.push_str("  \\begin{tabular}{lllrrr}\n    \\toprule\n");
        s.push_str(
            "    Family & A & B & Wilcoxon $p$ & $t$-test $p$ & A better \\\\\n    \\midrule\n",
        );
        for t in significance {
            writeln!(
                s,
                "    {} & {} & {} & {:.4} & {:.4} & {} \\\\",
                latex_escape(&t.family),
                latex_escape(&t.solver_a),
                latex_escape(&t.solver_b),
                t.wilcoxon_p,
                t.t_p,
                if t.a_better { "yes" } else { "no" }
            )
            .ok();
        }
        s.push_str("    \\bottomrule\n  \\end{tabular}\n\\end{table}\n");
    }
    s
}

fn latex_escape(s: &str) -> String {
    s.replace('_', "\\_")
}

// --------------------------------------------------------------------------
// SVG plots
// --------------------------------------------------------------------------

enum Metric {
    Success,
    Gap,
}

/// Grouped bar chart: families along x, one bar per solver, height = metric.
/// Self-contained SVG (theme-neutral, embeds no external assets).
fn svg_bars(summaries: &[Summary], metric: Metric) -> String {
    use std::collections::BTreeSet;
    let families: Vec<String> = summaries
        .iter()
        .map(|s| s.family.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let solvers: Vec<String> = summaries
        .iter()
        .map(|s| s.solver.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let palette = [
        "#4e79a7", "#f28e2b", "#59a14f", "#e15759", "#b07aa1", "#76b7b2",
    ];
    let (w, h) = (900.0, 460.0);
    let (ml, mr, mt, mb) = (60.0, 20.0, 50.0, 90.0);
    let pw = w - ml - mr;
    let ph = h - mt - mb;
    let (title, ymax) = match metric {
        Metric::Success => ("Success probability by family", 1.0),
        Metric::Gap => (
            "Mean optimality gap by family (lower is better)",
            summaries.iter().map(|s| s.mean_gap).fold(1e-6, f64::max) * 1.15,
        ),
    };
    let val = |s: &Summary| match metric {
        Metric::Success => s.p_success,
        Metric::Gap => s.mean_gap,
    };
    let get = |fam: &str, sol: &str| -> Option<f64> {
        summaries
            .iter()
            .find(|s| s.family == fam && s.solver == sol)
            .map(val)
    };

    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         font-family=\"sans-serif\" font-size=\"12\">\n\
         <rect width=\"{w}\" height=\"{h}\" fill=\"white\"/>\n\
         <text x=\"{tx}\" y=\"24\" font-size=\"16\" font-weight=\"bold\">{title}</text>\n",
        tx = ml,
    );
    // Axes.
    writeln!(
        svg,
        "<line x1=\"{ml}\" y1=\"{y0}\" x2=\"{x1}\" y2=\"{y0}\" stroke=\"#333\"/>\
         <line x1=\"{ml}\" y1=\"{mt}\" x2=\"{ml}\" y2=\"{y0}\" stroke=\"#333\"/>",
        y0 = mt + ph,
        x1 = ml + pw,
    )
    .ok();
    // Y gridlines + labels.
    for k in 0..=5 {
        let frac = k as f64 / 5.0;
        let y = mt + ph - frac * ph;
        let label = ymax * frac;
        writeln!(
            svg,
            "<line x1=\"{ml}\" y1=\"{y:.1}\" x2=\"{x1}\" y2=\"{y:.1}\" stroke=\"#eee\"/>\
             <text x=\"{lx:.1}\" y=\"{ty:.1}\" text-anchor=\"end\">{label:.3}</text>",
            x1 = ml + pw,
            lx = ml - 6.0,
            ty = y + 4.0,
        )
        .ok();
    }
    let group_w = pw / families.len().max(1) as f64;
    let bar_w = group_w * 0.8 / solvers.len().max(1) as f64;
    for (fi, fam) in families.iter().enumerate() {
        let gx = ml + fi as f64 * group_w;
        for (si, sol) in solvers.iter().enumerate() {
            if let Some(v) = get(fam, sol) {
                let bh = (v / ymax).clamp(0.0, 1.0) * ph;
                let x = gx + group_w * 0.1 + si as f64 * bar_w;
                let y = mt + ph - bh;
                writeln!(
                    svg,
                    "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bw:.1}\" height=\"{bh:.1}\" fill=\"{c}\"/>",
                    bw = bar_w * 0.92,
                    c = palette[si % palette.len()],
                )
                .ok();
            }
        }
        writeln!(
            svg,
            "<text x=\"{tx:.1}\" y=\"{ty:.1}\" text-anchor=\"middle\">{fam}</text>",
            tx = gx + group_w / 2.0,
            ty = mt + ph + 18.0,
        )
        .ok();
    }
    // Legend.
    for (si, sol) in solvers.iter().enumerate() {
        let lx = ml + si as f64 * 150.0;
        let ly = h - 30.0;
        writeln!(
            svg,
            "<rect x=\"{lx:.1}\" y=\"{ly:.1}\" width=\"12\" height=\"12\" fill=\"{c}\"/>\
             <text x=\"{tx:.1}\" y=\"{ty:.1}\">{sol}</text>",
            c = palette[si % palette.len()],
            tx = lx + 16.0,
            ty = ly + 11.0,
        )
        .ok();
    }
    svg.push_str("</svg>\n");
    svg
}

// --------------------------------------------------------------------------
// Markdown + stdout
// --------------------------------------------------------------------------

fn markdown(summaries: &[Summary], significance: &[Significance], skipped: &[&Solver]) -> String {
    let mut s = String::from("# Benchmark Report\n\n");
    s.push_str(
        "Solvers run on identical instances, seeds, and computational budget. \
         Gap is the mean per-run optimality gap vs. the per-instance reference \
         (verified best-known when supplied in `best_known.csv`, else the best \
         objective found by any solver). TTS(0.99) is in per-run wall-clock ms \
         with 95% bootstrap CIs.\n\n",
    );
    if !skipped.is_empty() {
        s.push_str("**Skipped (unavailable in this environment):** ");
        s.push_str(
            &skipped
                .iter()
                .map(|x| x.label())
                .collect::<Vec<_>>()
                .join(", "),
        );
        s.push_str(
            ". Their Python bridge scripts are in `benchmarks/adapters/`; \
                    install the libraries and rerun to include them.\n\n",
        );
    }
    s.push_str("## Summary\n\n");
    s.push_str(
        "| family | solver | inst | p_success | mean gap | gap CI95 | TTS(0.99) ms | mean ms |\n",
    );
    s.push_str("|---|---|---|---|---|---|---|---|\n");
    for r in summaries {
        writeln!(
            s,
            "| {} | {} | {} | {:.2} | {:.4} | [{:.4},{:.4}] | {} | {:.2} |",
            r.family,
            r.solver,
            r.n_instances,
            r.p_success,
            r.mean_gap,
            r.gap_lo,
            r.gap_hi,
            fmt_tts(r.tts),
            r.mean_ms
        )
        .ok();
    }
    if !significance.is_empty() {
        s.push_str("\n## Significance (Ultimate vs. baseline, per family)\n\n");
        s.push_str(
            "| family | vs | Wilcoxon p | t-test p | Ultimate better |\n|---|---|---|---|---|\n",
        );
        for t in significance {
            writeln!(
                s,
                "| {} | {} | {:.4} | {:.4} | {} |",
                t.family,
                t.solver_b,
                t.wilcoxon_p,
                t.t_p,
                if t.a_better { "yes" } else { "no" }
            )
            .ok();
        }
    }
    s.push_str("\n## Artifacts\n\n- `results_raw.csv` — every run.\n- `summary.csv` — per (family,solver) metrics.\n- `results.json` — machine-readable summary + significance.\n- `tables.tex` — LaTeX booktabs tables.\n- `plot_success.svg`, `plot_gap.svg` — figures.\n");
    s
}

/// Short human summary for stdout.
pub fn stdout_summary(
    summaries: &[Summary],
    significance: &[Significance],
    skipped: &[&Solver],
    n_instances: usize,
) -> String {
    let mut s = format!(
        "benchmark complete: {} instances, {} (family,solver) cells, {} pairwise tests\n",
        n_instances,
        summaries.len(),
        significance.len()
    );
    if !skipped.is_empty() {
        let names: Vec<_> = skipped.iter().map(|x| x.label()).collect();
        writeln!(s, "skipped (unavailable): {}", names.join(", ")).ok();
    }
    s.push_str("wrote CSV/JSON/LaTeX/SVG/Markdown to the output directory\n");
    s
}
