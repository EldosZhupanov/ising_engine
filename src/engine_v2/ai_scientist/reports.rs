//! Research reports (Stage 6, Tasks 4+5) — the platform's CUMULATIVE written
//! memory. Every `every` experiments the archive generates a structured
//! Markdown report (new discoveries, confirmed rules, proven hypotheses,
//! dominant/useless operators, best results, proposed operators) and appends it
//! to `reports/`. Reports are never rewritten.
//!
//! `ResearchMemory` is the ONLY thing the LLM scientist reads: distilled
//! reports + the knowledge graph — never the raw experiment database. The model
//! works with existing knowledge, so its context stays small as history grows
//! into the millions.

use super::db::ExperimentDb;
use super::graph::KnowledgeGraph;
use super::meta_learner::{MetaLearner, Rule, RuleKind};
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Append-only archive of numbered research reports plus the state needed to
/// know WHEN to write the next one and WHICH findings are new.
#[derive(Debug, Clone)]
pub struct ReportArchive {
    dir: PathBuf,
    /// Generate a report every this many experiments.
    pub every: usize,
    /// Experiment count when the last report was written.
    last_report_at: usize,
    /// Rule statements already published (for "what's new" diffing).
    known: BTreeSet<String>,
}

impl ReportArchive {
    /// Open (or create) the archive under `dir`. Resumes the counter and the
    /// known-findings set from `state.txt`, so "new discoveries" stay accurate
    /// across process restarts.
    pub fn open(dir: impl Into<PathBuf>, every: usize) -> io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let mut last_report_at = 0;
        let mut known = BTreeSet::new();
        if let Ok(text) = fs::read_to_string(dir.join("state.txt")) {
            let mut lines = text.lines();
            last_report_at = lines.next().and_then(|l| l.parse().ok()).unwrap_or(0);
            known = lines.map(|l| l.to_string()).collect();
        }
        Ok(Self {
            dir,
            every: every.max(1),
            last_report_at,
            known,
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Whether the database has grown enough for the next report.
    pub fn due(&self, db: &ExperimentDb) -> bool {
        db.len() >= self.last_report_at + self.every
    }

    /// Write the next report if due. Returns its path when one was written.
    pub fn maybe_report(
        &mut self,
        db: &ExperimentDb,
        graph: &KnowledgeGraph,
        meta: &MetaLearner,
    ) -> io::Result<Option<PathBuf>> {
        if !self.due(db) {
            return Ok(None);
        }
        Ok(Some(self.force_report(db, graph, meta)?))
    }

    /// Write a report unconditionally (campaign end, user request).
    pub fn force_report(
        &mut self,
        db: &ExperimentDb,
        graph: &KnowledgeGraph,
        meta: &MetaLearner,
    ) -> io::Result<PathBuf> {
        let rules = meta.mine(db);
        let body = compose_report(db, graph, meta, &rules, &self.known);
        let path = self.dir.join(format!("report_{:08}.md", db.len()));
        fs::write(&path, &body)?;
        self.last_report_at = db.len();
        for r in &rules {
            self.known.insert(r.statement.clone());
        }
        // Persist state (counter + known findings) and refresh the index.
        let mut state = format!("{}\n", self.last_report_at);
        for s in &self.known {
            state.push_str(s);
            state.push('\n');
        }
        fs::write(self.dir.join("state.txt"), state)?;
        self.write_index()?;
        Ok(path)
    }

    /// All report paths, oldest first.
    pub fn reports(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = fs::read_dir(&self.dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .and_then(|f| f.to_str())
                            .is_some_and(|f| f.starts_with("report_") && f.ends_with(".md"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    fn write_index(&self) -> io::Result<()> {
        let mut out = String::from("# Research Report Index\n\n");
        for p in self.reports() {
            if let Some(name) = p.file_name().and_then(|f| f.to_str()) {
                out.push_str(&format!("- [{name}]({name})\n"));
            }
        }
        fs::write(self.dir.join("INDEX.md"), out)
    }
}

fn section(rules: &[Rule], kind: RuleKind, n: usize) -> Vec<&Rule> {
    rules.iter().filter(|r| r.kind == kind).take(n).collect()
}

fn push_rules(out: &mut String, rules: &[&Rule], empty_msg: &str) {
    if rules.is_empty() {
        out.push_str(&format!("- ({empty_msg})\n"));
    }
    for r in rules {
        out.push_str(&format!(
            "- {} (support {}, confidence {:.2})\n",
            r.statement, r.support, r.confidence
        ));
    }
}

/// The Stage 6 report structure: seven fixed questions, answered only with
/// statistically supported findings.
fn compose_report(
    db: &ExperimentDb,
    graph: &KnowledgeGraph,
    meta: &MetaLearner,
    rules: &[Rule],
    known: &BTreeSet<String>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Research Report — {} experiments\n\n", db.len()));
    if let Some(best) = db.best() {
        out.push_str(&format!(
            "Best result so far: `{:?}` — score {:.2} (baseline {:.2}, instance `{}`)\n\n",
            best.sequence,
            best.score,
            best.baseline,
            if best.instance_id.is_empty() {
                "?"
            } else {
                &best.instance_id
            }
        ));
    }

    out.push_str("## 1. New discoveries\n");
    let fresh: Vec<&Rule> = rules
        .iter()
        .filter(|r| !known.contains(&r.statement))
        .collect();
    push_rules(&mut out, &fresh, "nothing new since the previous report");

    out.push_str("\n## 2. Confirmed rules\n");
    let confirmed: Vec<&Rule> = rules
        .iter()
        .filter(|r| known.contains(&r.statement))
        .collect();
    push_rules(
        &mut out,
        &confirmed,
        "no previously known rule re-confirmed",
    );

    out.push_str("\n## 3. Proven hypotheses (knowledge graph, highest confidence)\n");
    let mut facts: Vec<_> = graph.triples().iter().collect();
    facts.sort_by(|a, b| b.confidence().total_cmp(&a.confidence()));
    if facts.is_empty() {
        out.push_str("- (the knowledge graph is still empty)\n");
    }
    for t in facts.iter().take(8) {
        let cond = if t.condition.is_empty() {
            String::new()
        } else {
            format!(" IF {}", t.condition)
        };
        out.push_str(&format!(
            "- {} --{}--> {}{cond} (weight {:+.2}, support {}, confidence {:.2}; proof: {})\n",
            t.subject,
            t.predicate,
            t.object,
            t.weight,
            t.support,
            t.confidence(),
            if t.proof.is_empty() { "-" } else { &t.proof },
        ));
    }

    out.push_str("\n## 4. Dominant operators\n");
    push_rules(
        &mut out,
        &section(rules, RuleKind::Dominance, 6),
        "no operator dominates yet",
    );

    out.push_str("\n## 5. Useless operators\n");
    let mut weak = section(rules, RuleKind::Useless, 6);
    weak.extend(section(rules, RuleKind::Antipattern, 6));
    push_rules(&mut out, &weak, "no operator is measurably useless");

    out.push_str("\n## 6. What produces the best results\n");
    let mut ctx: Vec<&Rule> = Vec::new();
    ctx.extend(section(rules, RuleKind::Ordering, 4));
    ctx.extend(section(rules, RuleKind::Conditional, 4));
    ctx.extend(section(rules, RuleKind::Temperature, 3));
    ctx.extend(section(rules, RuleKind::Budget, 3));
    ctx.extend(section(rules, RuleKind::Backend, 2));
    push_rules(&mut out, &ctx, "not enough data for context rules yet");

    out.push_str("\n## 7. New operators worth creating\n");
    match meta.suggest_operator_gap(db) {
        Some(gap) => out.push_str(&format!(
            "- `{}` bridging {} → {}: {} (support {})\n",
            gap.suggested_name, gap.predecessor, gap.successor, gap.rationale, gap.support
        )),
        None => out.push_str("- (no frequent transition suggests a fused operator yet)\n"),
    }
    out
}

/// Task 5: the LLM reads DISTILLED knowledge, never the raw database. Builds a
/// bounded context from the most recent reports (newest first) plus the
/// highest-confidence knowledge-graph facts.
pub struct ResearchMemory;

impl ResearchMemory {
    pub fn context(archive: &ReportArchive, graph: &KnowledgeGraph, max_chars: usize) -> String {
        let mut out = String::new();
        let mut facts: Vec<_> = graph.triples().iter().collect();
        facts.sort_by(|a, b| b.confidence().total_cmp(&a.confidence()));
        if !facts.is_empty() {
            out.push_str("## Established knowledge (graph, highest confidence first)\n");
            for t in facts.iter().take(15) {
                let cond = if t.condition.is_empty() {
                    String::new()
                } else {
                    format!(" IF {}", t.condition)
                };
                out.push_str(&format!(
                    "- {} --{}--> {}{cond} (w {:+.2}, conf {:.2})\n",
                    t.subject,
                    t.predicate,
                    t.object,
                    t.weight,
                    t.confidence()
                ));
            }
            out.push('\n');
        }
        for path in archive.reports().iter().rev() {
            if out.len() >= max_chars {
                break;
            }
            if let Ok(text) = fs::read_to_string(path) {
                let remaining = max_chars - out.len();
                if text.len() <= remaining {
                    out.push_str(&text);
                    out.push('\n');
                } else {
                    // Keep the head of the report — it carries the new findings.
                    let mut cut = remaining.min(text.len());
                    while cut > 0 && !text.is_char_boundary(cut) {
                        cut -= 1;
                    }
                    out.push_str(&text[..cut]);
                    out.push_str("\n[truncated]\n");
                    break;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("reports_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn fill(db: &mut ExperimentDb, n: usize, seq: &[&str], score: f64) {
        for _ in 0..n {
            db.record(ExperimentRecord {
                sequence: seq.iter().map(|s| s.to_string()).collect(),
                sweeps: vec![10; seq.len()],
                temp_hi: 4.0,
                temp_lo: 0.1,
                num_replicas: 8,
                backend: "SparseBitSlice".into(),
                score,
                density: 0.01,
                clustering: 0.3,
                ..Default::default()
            });
        }
    }

    #[test]
    fn reports_fire_every_n_and_diff_new_findings() {
        let dir = scratch("cadence");
        let mut arch = ReportArchive::open(&dir, 20).unwrap();
        let mut db = ExperimentDb::new();
        let graph = KnowledgeGraph::new();
        let meta = MetaLearner::new();

        fill(&mut db, 10, &["metropolis_sweep"], -0.5);
        assert!(arch.maybe_report(&db, &graph, &meta).unwrap().is_none());

        fill(&mut db, 12, &["metropolis_sweep", "greedy_descent"], -0.9);
        let first = arch.maybe_report(&db, &graph, &meta).unwrap().unwrap();
        let text = fs::read_to_string(&first).unwrap();
        assert!(text.contains("## 1. New discoveries"));
        assert!(text.contains("## 7. New operators worth creating"));
        // Not due again until 20 more.
        assert!(arch.maybe_report(&db, &graph, &meta).unwrap().is_none());

        // Second report: previously seen rules move to "Confirmed".
        fill(&mut db, 20, &["metropolis_sweep", "greedy_descent"], -1.2);
        let second = arch.maybe_report(&db, &graph, &meta).unwrap().unwrap();
        let text2 = fs::read_to_string(&second).unwrap();
        let confirmed = text2.split("## 2. Confirmed rules").nth(1).unwrap();
        assert!(
            confirmed.contains("(support") || confirmed.contains("re-confirmed"),
            "confirmed section present: {confirmed}"
        );
        assert_eq!(arch.reports().len(), 2);
        assert!(dir.join("INDEX.md").exists());

        // Reopen: resumes state, not due immediately.
        let arch2 = ReportArchive::open(&dir, 20).unwrap();
        assert!(!arch2.due(&db));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn research_memory_reads_reports_and_graph_only() {
        let dir = scratch("memory");
        let mut arch = ReportArchive::open(&dir, 5).unwrap();
        let mut db = ExperimentDb::new();
        let mut graph = KnowledgeGraph::new();
        graph.observe_if(
            "houdayer_cluster",
            "effective-on",
            "graph",
            "density<0.05",
            0.4,
            "exp 1-16",
        );
        fill(&mut db, 6, &["gibbs_color_sweep"], -0.7);
        arch.maybe_report(&db, &graph, &MetaLearner::new())
            .unwrap()
            .unwrap();

        let ctx = ResearchMemory::context(&arch, &graph, 8000);
        assert!(ctx.contains("Established knowledge"));
        assert!(ctx.contains("IF density<0.05"));
        assert!(ctx.contains("# Research Report — 6 experiments"));
        // Bounded output.
        let small = ResearchMemory::context(&arch, &graph, 300);
        assert!(small.len() <= 320);
        let _ = fs::remove_dir_all(&dir);
    }
}
