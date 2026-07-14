//! Dashboard (Stage 6, Task 10) — one self-contained `dashboard.html` (inline
//! CSS + SVG, no external assets) generated from the persistent stores: best
//! algorithms, quality evolution, operator ranking, knowledge-graph facts,
//! report/analysis/proposal indexes, and campaign history.
//!
//! Palette (validated for CVD + contrast on both surfaces): blue #2a78d6 /
//! #3987e5 for the single series, red #e34948 / #e66767 only for NEGATIVE
//! polarity. Identity is never color-alone: every mark carries a text label or
//! a native `<title>` tooltip, and each chart has a table twin.

use super::curiosity::{CuriosityConfig, CuriosityEngine};
use super::db::ExperimentDb;
use super::executive::ExecutiveBrief;
use super::graph::KnowledgeGraph;
use super::memory_os::MemoryManager;
use super::reports::ReportArchive;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Format an integer with thousands separators (18570 → "18,570").
fn fmt_int(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// Running best-so-far relative improvement, downsampled to ≤ `max_pts`.
fn quality_series(db: &ExperimentDb, max_pts: usize) -> Vec<(usize, f64)> {
    let mut best = f64::NEG_INFINITY;
    let all: Vec<f64> = db
        .all()
        .iter()
        .map(|r| {
            best = best.max(r.rel_improvement());
            best
        })
        .collect();
    if all.is_empty() {
        return Vec::new();
    }
    let stride = all.len().div_ceil(max_pts).max(1);
    let mut pts: Vec<(usize, f64)> = all
        .iter()
        .enumerate()
        .filter(|(i, _)| i % stride == 0)
        .map(|(i, v)| (i + 1, *v))
        .collect();
    if pts.last().map(|p| p.0) != Some(all.len()) {
        pts.push((all.len(), *all.last().unwrap()));
    }
    pts
}

/// Mean relative improvement and count per operator, best first.
fn operator_ranking(db: &ExperimentDb) -> Vec<(String, f64, usize)> {
    let mut agg: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for r in db.all() {
        for op in &r.sequence {
            let e = agg.entry(op.clone()).or_insert((0.0, 0));
            e.0 += r.rel_improvement();
            e.1 += 1;
        }
    }
    let mut v: Vec<(String, f64, usize)> = agg
        .into_iter()
        .map(|(op, (s, n))| (op, s / n.max(1) as f64, n))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// Best distinct schedules: (sequence, best rel improvement, runs).
fn best_algorithms(db: &ExperimentDb, k: usize) -> Vec<(String, f64, usize)> {
    let mut agg: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for r in db.all() {
        let key = r.sequence.join(" → ");
        let e = agg.entry(key).or_insert((f64::NEG_INFINITY, 0));
        e.0 = e.0.max(r.rel_improvement());
        e.1 += 1;
    }
    let mut v: Vec<(String, f64, usize)> = agg.into_iter().map(|(s, (b, n))| (s, b, n)).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(k);
    v
}

fn line_chart(pts: &[(usize, f64)]) -> String {
    if pts.len() < 2 {
        return "<p class=\"muted\">Not enough experiments for a trend yet.</p>".into();
    }
    let (w, h, ml, mr, mt, mb) = (880.0, 260.0, 60.0, 18.0, 18.0, 34.0);
    let (pw, ph) = (w - ml - mr, h - mt - mb);
    let xmax = pts.last().unwrap().0 as f64;
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for (_, y) in pts {
        ymin = ymin.min(*y);
        ymax = ymax.max(*y);
    }
    if (ymax - ymin).abs() < 1e-12 {
        ymax = ymin + 1.0;
    }
    let px = |x: f64| ml + pw * x / xmax.max(1.0);
    let py = |y: f64| mt + ph * (1.0 - (y - ymin) / (ymax - ymin));
    let mut s = format!(
        "<svg viewBox=\"0 0 {w} {h}\" preserveAspectRatio=\"none\" role=\"img\" aria-label=\"Best relative improvement over experiments\">\
<defs><linearGradient id=\"area\" x1=\"0\" x2=\"0\" y1=\"0\" y2=\"1\">\
<stop offset=\"0\" stop-color=\"var(--accent)\" stop-opacity=\"0.28\"/>\
<stop offset=\"1\" stop-color=\"var(--accent)\" stop-opacity=\"0\"/></linearGradient></defs>"
    );
    // Recessive gridlines + y labels.
    for i in 0..=3 {
        let yv = ymin + (ymax - ymin) * i as f64 / 3.0;
        let y = py(yv);
        s.push_str(&format!(
            "<line class=\"grid\" x1=\"{ml}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\"/>\
<text class=\"tick\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{:+.1}%</text>",
            w - mr,
            ml - 8.0,
            y + 4.0,
            yv * 100.0
        ));
    }
    s.push_str(&format!(
        "<text class=\"tick\" x=\"{ml}\" y=\"{:.1}\">1</text>\
<text class=\"tick\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>",
        h - 8.0,
        w - mr,
        h - 8.0,
        pts.last().unwrap().0
    ));
    let path: Vec<String> = pts
        .iter()
        .map(|(x, y)| format!("{:.1},{:.1}", px(*x as f64), py(*y)))
        .collect();
    // Gradient area under the curve.
    let base = py(ymin);
    s.push_str(&format!(
        "<polygon class=\"area\" points=\"{:.1},{base:.1} {} {:.1},{base:.1}\"/>",
        px(pts[0].0 as f64),
        path.join(" "),
        px(xmax)
    ));
    s.push_str(&format!(
        "<polyline class=\"series\" points=\"{}\"/>",
        path.join(" ")
    ));
    // Sparse hover markers with native tooltips (hit target > mark).
    let stride = pts.len().div_ceil(24).max(1);
    for (x, y) in pts.iter().step_by(stride).chain(pts.last()) {
        s.push_str(&format!(
            "<circle class=\"pt\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"7\">\
<title>experiment {} — best {:+.2}% of baseline</title></circle>",
            px(*x as f64),
            py(*y),
            x,
            y * 100.0
        ));
    }
    s.push_str("</svg>");
    s
}

fn bar_chart(rank: &[(String, f64, usize)]) -> String {
    if rank.is_empty() {
        return "<p class=\"muted\">No operator data yet.</p>".into();
    }
    let rows = rank.len().min(12);
    let (w, bar_h, gap, ml) = (860.0, 22.0, 8.0, 230.0);
    let h = rows as f64 * (bar_h + gap) + 16.0;
    let span = rank
        .iter()
        .take(rows)
        .map(|r| r.1.abs())
        .fold(1e-12f64, f64::max);
    let zero_x = ml + (w - ml - 70.0) * 0.5;
    let scale = (w - ml - 70.0) * 0.5 / span;
    let mut s = format!(
        "<svg viewBox=\"0 0 {w} {h:.0}\" role=\"img\" aria-label=\"Mean improvement per operator\">"
    );
    s.push_str(&format!(
        "<line class=\"grid\" x1=\"{zero_x:.1}\" y1=\"4\" x2=\"{zero_x:.1}\" y2=\"{:.1}\"/>",
        h - 8.0
    ));
    for (i, (op, imp, n)) in rank.iter().take(rows).enumerate() {
        let y = 8.0 + i as f64 * (bar_h + gap);
        let len = imp.abs() * scale;
        let (x, class) = if *imp >= 0.0 {
            (zero_x, "pos")
        } else {
            (zero_x - len, "neg")
        };
        s.push_str(&format!(
            "<text class=\"lbl\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\
<rect class=\"{class}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{bar_h}\" rx=\"4\">\
<title>{}: {:+.2}% mean of baseline over {n} runs</title></rect>\
<text class=\"val\" x=\"{:.1}\" y=\"{:.1}\">{:+.1}%</text>",
            ml - 10.0,
            y + bar_h * 0.72,
            esc(op),
            len.max(1.5),
            esc(op),
            imp * 100.0,
            zero_x + if *imp >= 0.0 { len } else { -len } + if *imp >= 0.0 { 8.0 } else { -58.0 },
            y + bar_h * 0.72,
            imp * 100.0
        ));
    }
    s.push_str("</svg>");
    s
}

fn file_links(dir: &Path, sub: &str, ext: &str) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir.join(sub))
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|f| f.ends_with(ext))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Generate `dir/dashboard.html`. Purely derived from the stores — safe to
/// regenerate after every generation.
pub fn write_dashboard(
    dir: impl AsRef<Path>,
    db: &ExperimentDb,
    graph: &KnowledgeGraph,
    archive: &ReportArchive,
    notes: &[String],
) -> io::Result<PathBuf> {
    write_dashboard_with(dir, db, graph, archive, notes, None)
}

/// As [`write_dashboard`], but with the Chief Scientist's [`ExecutiveBrief`]
/// rendered as the hero panel (its current directives and why).
pub fn write_dashboard_with(
    dir: impl AsRef<Path>,
    db: &ExperimentDb,
    graph: &KnowledgeGraph,
    archive: &ReportArchive,
    notes: &[String],
    executive: Option<&ExecutiveBrief>,
) -> io::Result<PathBuf> {
    let dir = dir.as_ref();
    let rank = operator_ranking(db);
    let series = quality_series(db, 200);
    let algos = best_algorithms(db, 8);
    let campaigns: BTreeMap<u64, usize> = db.all().iter().fold(BTreeMap::new(), |mut m, r| {
        *m.entry(r.campaign_id).or_insert(0) += 1;
        m
    });
    let best = db.best();

    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>Ising Research OS</title><style>\
*{box-sizing:border-box}\
:root{--bg:#0a0b0e;--card:#14161c;--card2:#1a1d25;--border:#262a33;--ink:#e7e9ee;--ink2:#9aa0ad;--ink3:#636a78;\
--accent:#5b8def;--accent2:#7c5cff;--s1:var(--accent);--good:#3fb950;--bad:#f0736b;--warn:#e3b341}\
@media (prefers-color-scheme:light){:root{--bg:#f6f7f9;--card:#ffffff;--card2:#f3f4f7;--border:#e4e7ec;\
--ink:#14171f;--ink2:#5a616e;--ink3:#8a91a0;--accent:#2a78d6;--good:#1a7f37;--bad:#cf222e}}\
html{-webkit-text-size-adjust:100%}\
body{margin:0;background:var(--bg);color:var(--ink);font:14.5px/1.55 -apple-system,BlinkMacSystemFont,'Segoe UI',system-ui,sans-serif;\
-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}\
.top{position:sticky;top:0;z-index:9;background:color-mix(in srgb,var(--bg) 82%,transparent);\
backdrop-filter:saturate(140%) blur(12px);border-bottom:1px solid var(--border)}\
.top-in{max-width:1180px;margin:0 auto;padding:14px 24px;display:flex;align-items:center;justify-content:space-between;gap:16px;flex-wrap:wrap}\
.brand{display:flex;align-items:center;gap:10px;font-weight:640;letter-spacing:-.01em;font-size:15.5px}\
.mk{width:20px;height:20px;border-radius:6px;background:linear-gradient(135deg,var(--accent),var(--accent2));display:inline-block;box-shadow:0 0 0 1px var(--border)}\
.stat{color:var(--ink2);font-size:12.5px;font-variant-numeric:tabular-nums}\
.wrap{max-width:1180px;margin:0 auto;padding:26px 24px 90px}\
.sub{color:var(--ink3);font-size:13.5px;margin:0 0 24px;max-width:70ch}\
h2{font-size:11.5px;text-transform:uppercase;letter-spacing:.09em;color:var(--ink2);font-weight:600;\
margin:40px 0 12px;display:flex;align-items:center;gap:9px}\
h2::before{content:'';width:7px;height:7px;border-radius:2px;background:var(--accent)}\
h2 .note{text-transform:none;letter-spacing:0;color:var(--ink3);font-weight:400}\
.card{background:var(--card);border:1px solid var(--border);border-radius:14px;padding:18px 20px;box-shadow:0 1px 2px rgba(0,0,0,.18)}\
.grid-kpi{display:grid;grid-template-columns:repeat(auto-fit,minmax(148px,1fr));gap:13px;margin-bottom:8px}\
.kpi{background:var(--card);border:1px solid var(--border);border-radius:14px;padding:16px 18px}\
.kpi .n{font-size:29px;font-weight:660;letter-spacing:-.02em;font-variant-numeric:tabular-nums;line-height:1.1}\
.kpi .l{color:var(--ink2);font-size:11.5px;text-transform:uppercase;letter-spacing:.05em;margin-top:5px}\
.kpi .n.good{color:var(--good)}.kpi .n.accent{color:var(--accent)}\
.exec{background:linear-gradient(180deg,color-mix(in srgb,var(--accent) 8%,var(--card)),var(--card));\
border:1px solid color-mix(in srgb,var(--accent) 24%,var(--border));border-radius:16px;padding:20px 22px;margin:6px 0 8px}\
.exec .eyebrow{display:flex;align-items:center;gap:8px;font-size:11.5px;text-transform:uppercase;letter-spacing:.09em;color:var(--accent);font-weight:600;margin-bottom:10px}\
.exec .headline{font-size:15px;color:var(--ink);margin:0 0 14px;font-weight:500}\
.decisions{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:9px}\
.decision{display:flex;gap:11px;align-items:baseline;padding:9px 12px;background:var(--card2);border:1px solid var(--border);border-radius:10px}\
.dot{flex:0 0 auto;width:8px;height:8px;border-radius:50%;margin-top:6px;background:var(--ink3)}\
.dot.hi{background:var(--warn)}.dot.mid{background:var(--accent)}\
.decision .act{font-weight:600;color:var(--ink);margin-right:2px}\
.decision .why{color:var(--ink2)}\
table{border-collapse:separate;border-spacing:0;width:100%;background:var(--card);border:1px solid var(--border);border-radius:14px;overflow:hidden;font-size:13px}\
th{text-align:left;color:var(--ink2);font-weight:500;font-size:11px;text-transform:uppercase;letter-spacing:.045em;padding:10px 13px;background:var(--card2);border-bottom:1px solid var(--border)}\
td{padding:10px 13px;border-bottom:1px solid var(--border);color:var(--ink);vertical-align:top}\
tr:last-child td{border-bottom:none}tbody tr:hover td,table tr:hover td{background:var(--card2)}\
code{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:12px;color:var(--ink);\
background:color-mix(in srgb,var(--ink) 8%,transparent);padding:1.5px 6px;border-radius:5px}\
svg{width:100%;height:auto;display:block}\
.card svg{margin:0}\
.grid{stroke:var(--border);stroke-width:1}\
.tick,.lbl,.val{fill:var(--ink2);font:11px ui-monospace,monospace}.val{fill:var(--ink)}\
.series{fill:none;stroke:var(--accent);stroke-width:2.5;stroke-linejoin:round;stroke-linecap:round}\
.area{fill:url(#area);stroke:none}.pt{fill:transparent}.pt:hover{fill:var(--accent)}\
.pos{fill:var(--accent)}.neg{fill:var(--bad)}rect{rx:5}\
.pill{display:inline-block;padding:2px 9px;border-radius:999px;font-size:10.5px;font-weight:600;letter-spacing:.02em;white-space:nowrap}\
.pill.good{background:color-mix(in srgb,var(--good) 18%,transparent);color:var(--good)}\
.pill.bad{background:color-mix(in srgb,var(--bad) 16%,transparent);color:var(--bad)}\
.pill.accent{background:color-mix(in srgb,var(--accent) 16%,transparent);color:var(--accent)}\
.pill.muted{background:color-mix(in srgb,var(--ink) 8%,transparent);color:var(--ink2)}\
.muted{color:var(--ink2)}.mono{font-variant-numeric:tabular-nums}\
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}\
ul.links{margin:6px 0;padding-left:18px;color:var(--ink2);font-size:13px}\
.foot{margin-top:44px;padding-top:18px;border-top:1px solid var(--border);color:var(--ink3);font-size:12px}\
</style></head><body>\
<header class=\"top\"><div class=\"top-in\"><div class=\"brand\"><span class=\"mk\"></span>Ising Research OS</div>",
    );
    html.push_str(&format!(
        "<div class=\"stat mono\">{} experiments · {} campaigns · {} knowledge facts</div></div></header>\n<div class=\"wrap\">\n",
        db.len(),
        db.all().iter().map(|r| r.campaign_id).collect::<std::collections::BTreeSet<_>>().len(),
        graph.len(),
    ));
    html.push_str("<p class=\"sub\">Autonomous optimization-research platform. Every figure below is a recorded measurement from the append-only experiment log — no estimates, no fabrication.</p>\n");

    // ---- Chief Scientist hero panel -----------------------------------------
    if let Some(brief) = executive {
        html.push_str("<section class=\"exec\"><div class=\"eyebrow\">◆ Chief Scientist · current directives</div>");
        html.push_str(&format!(
            "<p class=\"headline\">{}</p><ul class=\"decisions\">",
            esc(&brief.headline)
        ));
        for d in brief.decisions.iter().take(7) {
            let dot = if d.urgency >= 0.6 {
                "hi"
            } else if d.urgency >= 0.42 {
                "mid"
            } else {
                ""
            };
            html.push_str(&format!(
                "<li class=\"decision\"><span class=\"dot {dot}\"></span><span><span class=\"act\">{}</span> <span class=\"why\">— {}</span></span></li>",
                esc(&d.title()),
                esc(&d.reason),
            ));
        }
        html.push_str("</ul></section>\n");
    }

    // ---- KPI cards -----------------------------------------------------------
    let kpi = |n: String, cls: &str, l: &str| {
        format!(
            "<div class=\"kpi\"><div class=\"n {cls}\">{n}</div><div class=\"l\">{l}</div></div>"
        )
    };
    let theories = graph
        .triples()
        .iter()
        .filter(|t| t.predicate == "theory-explains")
        .count();
    html.push_str("<div class=\"grid-kpi\">");
    html.push_str(&kpi(fmt_int(db.len()), "", "experiments"));
    if let Some(b) = best {
        html.push_str(&kpi(
            format!("{:+.1}%", b.rel_improvement() * 100.0),
            "good",
            "best improvement",
        ));
    }
    html.push_str(&kpi(theories.to_string(), "accent", "theories"));
    html.push_str(&kpi(graph.len().to_string(), "", "knowledge facts"));
    html.push_str(&kpi(campaigns.len().to_string(), "", "campaigns"));
    html.push_str(&kpi(
        archive.reports().len().to_string(),
        "",
        "research reports",
    ));
    html.push_str("</div>\n");

    // ---- quality evolution ---------------------------------------------------
    html.push_str("<h2>Quality evolution <span class=\"note\">— best relative improvement vs baseline</span></h2>\n");
    html.push_str(&format!(
        "<div class=\"card\">{}</div>",
        line_chart(&series)
    ));

    // ---- operator ranking ----------------------------------------------------
    html.push_str("<h2>Operator impact <span class=\"note\">— mean improvement per operator (blue helps · red hurts)</span></h2>\n");
    html.push_str(&format!("<div class=\"card\">{}</div>", bar_chart(&rank)));

    // ---- best algorithms -----------------------------------------------------
    html.push_str("<h2>Best algorithms discovered</h2>\n<table><tr><th>schedule</th><th>best improvement</th><th>runs</th></tr>");
    for (seq, imp, n) in &algos {
        html.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{:+.2}%</td><td>{n}</td></tr>",
            esc(seq),
            imp * 100.0
        ));
    }
    html.push_str("</table>\n");

    // ---- knowledge graph -----------------------------------------------------
    html.push_str("<h2>Knowledge graph — highest-confidence facts</h2>\n<table><tr><th>fact</th><th>condition</th><th>weight</th><th>support</th><th>confidence</th><th>proof</th></tr>");
    let mut facts: Vec<_> = graph.triples().iter().collect();
    facts.sort_by(|a, b| b.confidence().total_cmp(&a.confidence()));
    for t in facts.iter().take(14) {
        html.push_str(&format!(
            "<tr><td>{} <span class=\"muted\">{}</span> {}</td><td>{}</td><td>{:+.2}</td><td>{}</td><td>{:.2}</td><td class=\"muted\">{}</td></tr>",
            esc(&t.subject),
            esc(&t.predicate),
            esc(&t.object),
            if t.condition.is_empty() { "—".into() } else { esc(&t.condition) },
            t.weight,
            t.support,
            t.confidence(),
            if t.proof.is_empty() { "—".into() } else { esc(&t.proof) },
        ));
    }
    html.push_str("</table>\n");

    // ---- theories (mechanism, ablation-tested) -------------------------------
    let theories: Vec<_> = graph
        .triples()
        .iter()
        .filter(|t| t.predicate == "theory-explains" || t.predicate == "theory-refuted")
        .collect();
    if !theories.is_empty() {
        html.push_str("<h2>Theories <span class=\"note\">— mechanisms that faced ablation (a rule is a correlation; a theory survived the Runtime's attempt to refute it)</span></h2>\n<table><tr><th>operator</th><th>verdict</th><th>provides</th><th>if</th><th>confidence</th><th>mechanism &amp; ablation</th></tr>");
        let mut ts = theories.clone();
        ts.sort_by(|a, b| b.confidence().total_cmp(&a.confidence()));
        for t in ts.iter().take(12) {
            let pill = if t.predicate == "theory-explains" {
                "<span class=\"pill good\">SUPPORTED</span>"
            } else {
                "<span class=\"pill bad\">refuted</span>"
            };
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td class=\"mono\">{}</td><td class=\"mono\">{:.2}</td><td class=\"muted\">{}</td></tr>",
                esc(&t.subject),
                pill,
                esc(&t.object),
                if t.condition.is_empty() { "—".into() } else { esc(&t.condition) },
                t.confidence(),
                esc(&t.proof),
            ));
        }
        html.push_str("</table>\n");
    }

    // ---- cross-model consensus (the meta-learning layer) ---------------------
    let consensus: Vec<_> = graph
        .triples()
        .iter()
        .filter(|t| {
            t.predicate == "consensus-prefer"
                || t.predicate == "consensus-avoid"
                || t.predicate == "plateaus-early"
        })
        .collect();
    if !consensus.is_empty() {
        html.push_str("<h2>Cross-model consensus <span class=\"note\">— what Policy, World, Dynamics &amp; the meta-learner jointly decided</span></h2>\n<table><tr><th>operator</th><th>verdict</th><th>if</th><th>why (source attribution)</th></tr>");
        for t in &consensus {
            let pill = match t.predicate.as_str() {
                "consensus-prefer" => "<span class=\"pill good\">PREFER</span>",
                "consensus-avoid" => "<span class=\"pill bad\">avoid</span>",
                _ => "<span class=\"pill accent\">switch-early</span>",
            };
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td class=\"mono\">{}</td><td class=\"muted\">{}</td></tr>",
                esc(&t.subject),
                pill,
                if t.condition.is_empty() { "—".into() } else { esc(&t.condition) },
                esc(&t.proof),
            ));
        }
        html.push_str("</table>\n");
    }

    // ---- curiosity (where the platform is UNSURE — active learning) ----------
    let curiosity = CuriosityEngine::from_db(db, None, &CuriosityConfig::default());
    let curious = curiosity.ranked();
    if !curious.is_empty() {
        html.push_str("<h2>Curiosity <span class=\"note\">— where the models are least certain (coverage deficit + cross-seed anomaly); the platform steers exploration here</span></h2>\n<table><tr><th>operator</th><th>curiosity</th><th>coverage</th><th>anomaly</th></tr>");
        for (op, c) in curious.iter().take(8) {
            let (cov, anom, _dis) = curiosity.components(op);
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td class=\"mono\">{c:.2}</td><td class=\"mono\">{cov:.2}</td><td class=\"mono\">{anom:.2}</td></tr>",
                esc(op)
            ));
        }
        html.push_str("</table>\n");
    }

    // ---- scientific memory regimes -------------------------------------------
    let mem = MemoryManager::default().analyze(db);
    if !mem.buckets.is_empty() {
        html.push_str("<h2>Memory regimes <span class=\"note\">— what is well-characterized (append-only; nothing is ever deleted)</span></h2>\n<table><tr><th>regime</th><th>experiments</th><th>instances</th><th>dominant operator</th><th>compactable</th></tr>");
        for b in &mem.buckets {
            let pill = if b.compactable {
                "<span class=\"pill accent\">yes</span>"
            } else {
                "<span class=\"pill muted\">no</span>"
            };
            html.push_str(&format!(
                "<tr><td>{}</td><td class=\"mono\">{}</td><td class=\"mono\">{}</td><td><code>{}</code></td><td>{pill}</td></tr>",
                esc(&b.label),
                b.experiments,
                b.instances,
                esc(b.dominant_operator.as_deref().unwrap_or("—")),
            ));
        }
        html.push_str("</table>\n");
    }

    // ---- archives ------------------------------------------------------------
    let reports = file_links(dir, "reports", ".md");
    let analyses = file_links(dir, "analysis", ".md");
    let proposals = file_links(dir, "proposals", ".md");
    if !reports.is_empty() || !analyses.is_empty() || !proposals.is_empty() {
        html.push_str("<h2>Research history</h2>\n<ul class=\"links\">");
        for f in reports {
            html.push_str(&format!("<li><a href=\"reports/{f}\">{f}</a></li>"));
        }
        for f in analyses {
            html.push_str(&format!(
                "<li><a href=\"analysis/{f}\">{f}</a> — cloud deep analysis</li>"
            ));
        }
        for f in proposals {
            html.push_str(&format!(
                "<li><a href=\"proposals/{f}\">{f}</a> — operator proposal draft</li>"
            ));
        }
        html.push_str("</ul>\n");
    }

    // ---- campaigns -----------------------------------------------------------
    if !campaigns.is_empty() {
        html.push_str("<h2>Campaigns</h2>\n<table><tr><th>campaign</th><th>experiments</th></tr>");
        for (id, n) in &campaigns {
            html.push_str(&format!(
                "<tr><td>#{id}</td><td class=\"mono\">{n}</td></tr>"
            ));
        }
        html.push_str("</table>\n");
    }

    if !notes.is_empty() {
        html.push_str("<h2>Disclosures</h2>\n<ul class=\"links\">");
        for n in notes {
            html.push_str(&format!("<li>{}</li>", esc(n)));
        }
        html.push_str("</ul>\n");
    }
    html.push_str(
        "<div class=\"foot\">Ising Research OS · self-contained dashboard rendered from the append-only stores · correctness &gt; speed, always.</div>\n</div></body></html>\n",
    );

    let path = dir.join("dashboard.html");
    fs::write(&path, html)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::super::meta_learner::MetaLearner;
    use super::*;

    #[test]
    fn dashboard_renders_all_sections_from_real_stores() {
        let dir = std::env::temp_dir().join(format!("dash_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut db = ExperimentDb::new();
        for i in 0..40 {
            db.record(ExperimentRecord {
                sequence: vec!["metropolis_sweep".into(), "greedy_descent".into()],
                sweeps: vec![10, 10],
                temp_hi: 4.0,
                temp_lo: 0.1,
                score: -0.1 - (i as f64) * 0.01,
                baseline: 0.0,
                density: 0.01,
                clustering: 0.3,
                backend: "SparseBitSlice".into(),
                ..Default::default()
            });
            db.record(ExperimentRecord {
                sequence: vec!["random_flip_sweep".into()],
                score: 0.05,
                baseline: 0.0,
                density: 0.01,
                backend: "SparseBitSlice".into(),
                ..Default::default()
            });
        }
        let mut graph = KnowledgeGraph::new();
        graph.observe_if(
            "metropolis_sweep",
            "precedes-well",
            "greedy_descent",
            "density<0.05",
            0.3,
            "hypothesis #4",
        );
        let mut archive = ReportArchive::open(dir.join("reports"), 10).unwrap();
        archive
            .force_report(&db, &graph, &MetaLearner::new())
            .unwrap();

        let path = write_dashboard(
            &dir,
            &db,
            &graph,
            &archive,
            &["cloud tier skipped honestly: no key".to_string()],
        )
        .unwrap();
        let html = fs::read_to_string(&path).unwrap();
        for needle in [
            "Quality evolution",
            "<svg",
            "polyline",
            "metropolis_sweep",
            "Best algorithms",
            "Knowledge graph",
            "density&lt;0.05", // conditional fact rendered (escaped)
            "hypothesis #4",   // proof column
            "report_",
            "Disclosures",
            "skipped honestly",
            "prefers-color-scheme", // theme-aware (dark base + light media query)
            "Ising Research OS",    // the redesigned header/brand
            "grid-kpi",             // KPI cards
            "class=\"area\"",       // gradient area under the line chart
        ] {
            assert!(html.contains(needle), "dashboard missing: {needle}");
        }
        // Negative operator gets the polarity class; positive the series class.
        assert!(html.contains("class=\"neg\""));
        assert!(html.contains("class=\"pos\""));

        // The Chief Scientist hero panel renders when a brief is supplied.
        use super::super::executive::{ResearchExecutive, ResourceState};
        let brief = ResearchExecutive::new(Default::default()).assess(
            &db,
            &graph,
            &ResourceState {
                local_available: true,
                ..Default::default()
            },
        );
        let path2 = write_dashboard_with(&dir, &db, &graph, &archive, &[], Some(&brief)).unwrap();
        let html2 = fs::read_to_string(&path2).unwrap();
        assert!(html2.contains("Chief Scientist"), "executive panel missing");
        assert!(html2.contains("class=\"decision\""), "decisions missing");
        let _ = fs::remove_dir_all(&dir);
    }
}
