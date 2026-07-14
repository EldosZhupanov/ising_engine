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
    let (w, h, ml, mr, mt, mb) = (860.0, 240.0, 56.0, 16.0, 12.0, 30.0);
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
        "<svg viewBox=\"0 0 {w} {h}\" role=\"img\" aria-label=\"Best relative improvement over experiments\">"
    );
    // Recessive gridlines + y labels.
    for i in 0..=3 {
        let yv = ymin + (ymax - ymin) * i as f64 / 3.0;
        let y = py(yv);
        s.push_str(&format!(
            "<line class=\"grid\" x1=\"{ml}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\"/>\
<text class=\"tick\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{:+.1}%</text>",
            w - mr,
            ml - 6.0,
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
    s.push_str(&format!(
        "<polyline class=\"series\" points=\"{}\"/>",
        path.join(" ")
    ));
    // Sparse hover markers with native tooltips (hit target > mark).
    let stride = pts.len().div_ceil(24).max(1);
    for (x, y) in pts.iter().step_by(stride).chain(pts.last()) {
        s.push_str(&format!(
            "<circle class=\"pt\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"8\">\
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
<title>Autonomous Research Platform</title><style>\
:root{--surface:#fcfcfb;--ink:#0b0b0b;--ink2:#52514e;--grid:#e5e4e0;--s1:#2a78d6;--neg:#e34948;--card:#ffffff}\
@media (prefers-color-scheme:dark){:root{--surface:#1a1a19;--ink:#ffffff;--ink2:#c3c2b7;--grid:#383835;--s1:#3987e5;--neg:#e66767;--card:#242423}}\
body{margin:0;background:var(--surface);color:var(--ink);font:15px/1.5 system-ui,sans-serif;padding:24px}\
h1{font-size:22px;margin:0 0 4px}h2{font-size:16px;margin:28px 0 10px}\
.muted{color:var(--ink2)}.tiles{display:flex;flex-wrap:wrap;gap:12px;margin:16px 0}\
.tile{background:var(--card);border:1px solid var(--grid);border-radius:8px;padding:12px 18px;min-width:120px}\
.tile b{display:block;font-size:24px}.tile span{color:var(--ink2);font-size:13px}\
svg{width:100%;height:auto;background:var(--card);border:1px solid var(--grid);border-radius:8px}\
.grid{stroke:var(--grid);stroke-width:1}.tick,.lbl,.val{fill:var(--ink2);font:12px system-ui,sans-serif}\
.val{fill:var(--ink)}.series{fill:none;stroke:var(--s1);stroke-width:2}\
.pt{fill:transparent}.pt:hover{fill:var(--s1)}\
.pos{fill:var(--s1)}.neg{fill:var(--neg)}rect.pos:hover,rect.neg:hover{opacity:.8}\
table{border-collapse:collapse;width:100%;background:var(--card);border:1px solid var(--grid);border-radius:8px}\
th,td{text-align:left;padding:6px 10px;border-top:1px solid var(--grid);font-size:13px}\
th{color:var(--ink2);font-weight:600;border-top:none}code{font-size:12px}\
a{color:var(--s1)}ul{margin:6px 0}</style></head><body>\n<h1>Autonomous Research Platform</h1>\n",
    );
    html.push_str("<p class=\"muted\">Generated from the append-only experiment database; every number is a recorded measurement.</p>\n");

    // ---- stat tiles ----------------------------------------------------------
    html.push_str("<div class=\"tiles\">");
    let tile = |v: String, l: &str| format!("<div class=\"tile\"><b>{v}</b><span>{l}</span></div>");
    html.push_str(&tile(db.len().to_string(), "experiments"));
    html.push_str(&tile(campaigns.len().to_string(), "campaigns"));
    html.push_str(&tile(graph.len().to_string(), "knowledge facts"));
    html.push_str(&tile(
        archive.reports().len().to_string(),
        "research reports",
    ));
    if let Some(b) = best {
        html.push_str(&tile(
            format!("{:+.1}%", b.rel_improvement() * 100.0),
            "best improvement",
        ));
    }
    html.push_str("</div>\n");

    // ---- quality evolution ---------------------------------------------------
    html.push_str("<h2>Quality evolution — best relative improvement over baseline</h2>\n");
    html.push_str(&line_chart(&series));

    // ---- operator ranking ----------------------------------------------------
    html.push_str("<h2>Operators — mean improvement (blue = helps, red = hurts)</h2>\n");
    html.push_str(&bar_chart(&rank));

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
        html.push_str("<h2>Theories — mechanisms that faced ablation</h2>\n<p class=\"muted\">A rule is a correlation; a theory is a mechanism that survived the Runtime's attempt to refute it.</p>\n<table><tr><th>operator</th><th>verdict</th><th>provides</th><th>if</th><th>confidence</th><th>mechanism &amp; ablation</th></tr>");
        let mut ts = theories.clone();
        ts.sort_by(|a, b| b.confidence().total_cmp(&a.confidence()));
        for t in ts.iter().take(12) {
            let verdict = if t.predicate == "theory-explains" {
                "SUPPORTED"
            } else {
                "refuted"
            };
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td><td class=\"muted\">{}</td></tr>",
                esc(&t.subject),
                verdict,
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
        html.push_str("<h2>Cross-model consensus — what the models jointly decided</h2>\n<table><tr><th>operator</th><th>verdict</th><th>if</th><th>why (source attribution)</th></tr>");
        for t in &consensus {
            let verdict = match t.predicate.as_str() {
                "consensus-prefer" => "PREFER",
                "consensus-avoid" => "avoid",
                _ => "switch-early",
            };
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td class=\"muted\">{}</td></tr>",
                esc(&t.subject),
                verdict,
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
        html.push_str("<h2>Curiosity — where the models are least certain</h2>\n<p class=\"muted\">Surprise = coverage deficit + cross-seed anomaly. The platform steers exploration here.</p>\n<table><tr><th>operator</th><th>curiosity</th><th>coverage</th><th>anomaly</th></tr>");
        for (op, c) in curious.iter().take(8) {
            let (cov, anom, _dis) = curiosity.components(op);
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{c:.2}</td><td>{cov:.2}</td><td>{anom:.2}</td></tr>",
                esc(op)
            ));
        }
        html.push_str("</table>\n");
    }

    // ---- scientific memory regimes -------------------------------------------
    let mem = MemoryManager::default().analyze(db);
    if !mem.buckets.is_empty() {
        html.push_str("<h2>Memory regimes — what is well-characterized</h2>\n<table><tr><th>regime</th><th>experiments</th><th>instances</th><th>dominant operator</th><th>compactable</th></tr>");
        for b in &mem.buckets {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
                esc(&b.label),
                b.experiments,
                b.instances,
                esc(b.dominant_operator.as_deref().unwrap_or("—")),
                if b.compactable { "yes" } else { "no" },
            ));
        }
        html.push_str(&format!(
            "</table>\n<p class=\"muted\">{}</p>\n",
            esc(&mem.note)
        ));
    }

    // ---- operator table twin (accessibility) ----------------------------------
    html.push_str("<h2>Operator data (table view)</h2>\n<table><tr><th>operator</th><th>mean improvement</th><th>runs</th></tr>");
    for (op, imp, n) in rank.iter().take(12) {
        html.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{:+.2}%</td><td>{n}</td></tr>",
            esc(op),
            imp * 100.0
        ));
    }
    html.push_str("</table>\n");

    // ---- archives ------------------------------------------------------------
    html.push_str("<h2>Research history</h2>\n<ul>");
    for f in file_links(dir, "reports", ".md") {
        html.push_str(&format!("<li><a href=\"reports/{f}\">{f}</a></li>"));
    }
    for f in file_links(dir, "analysis", ".md") {
        html.push_str(&format!(
            "<li><a href=\"analysis/{f}\">{f}</a> (cloud deep analysis)</li>"
        ));
    }
    for f in file_links(dir, "proposals", ".md") {
        html.push_str(&format!(
            "<li><a href=\"proposals/{f}\">{f}</a> (operator proposal draft)</li>"
        ));
    }
    html.push_str("</ul>\n");

    // ---- campaigns -----------------------------------------------------------
    html.push_str("<h2>Campaigns</h2>\n<table><tr><th>campaign</th><th>experiments</th></tr>");
    for (id, n) in &campaigns {
        html.push_str(&format!("<tr><td>#{id}</td><td>{n}</td></tr>"));
    }
    html.push_str("</table>\n");

    if !notes.is_empty() {
        html.push_str("<h2>Disclosures</h2>\n<ul>");
        for n in notes {
            html.push_str(&format!("<li class=\"muted\">{}</li>", esc(n)));
        }
        html.push_str("</ul>\n");
    }
    html.push_str("</body></html>\n");

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
            "prefers-color-scheme:dark",
        ] {
            assert!(html.contains(needle), "dashboard missing: {needle}");
        }
        // Negative operator gets the polarity class; positive the series class.
        assert!(html.contains("class=\"neg\""));
        assert!(html.contains("class=\"pos\""));
        let _ = fs::remove_dir_all(&dir);
    }
}
