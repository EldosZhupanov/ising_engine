//! Cloud scientist (Stage 6, Task 6) — the SECOND LLM tier. A frontier model
//! (Claude API) is consulted RARELY (default: every 10 000 experiments). It
//! receives only distilled knowledge — research reports + knowledge graph +
//! aggregate statistics, never the raw database — and writes an in-depth
//! scientific analysis to `deep_analysis_*.md`.
//!
//! Like every external dependency in this codebase, it degrades honestly: no
//! `ANTHROPIC_API_KEY` ⇒ the platform reports the tier as skipped and continues
//! with the local/heuristic scientist. Nothing is fabricated.

use super::db::ExperimentDb;
use super::graph::KnowledgeGraph;
use super::llm::{extract_json_string_field, json_escape};
use super::reports::{ReportArchive, ResearchMemory};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Client for the Anthropic Messages API via `curl` (no HTTP crate).
#[derive(Debug, Clone)]
pub struct CloudScientist {
    /// HTTP client command (default `curl`; tests point it at a bogus path).
    pub command: String,
    pub api_url: String,
    pub model: String,
    /// Consult the cloud model every this many experiments.
    pub every: usize,
    pub max_tokens: usize,
    pub timeout: Duration,
    api_key: Option<String>,
    last_run_at: usize,
}

impl CloudScientist {
    /// Reads `ANTHROPIC_API_KEY` from the environment; absent ⇒ `available()`
    /// is false and `maybe_analyze` skips without error.
    pub fn new(model: impl Into<String>, every: usize) -> Self {
        Self {
            command: "curl".into(),
            api_url: "https://api.anthropic.com/v1/messages".into(),
            model: model.into(),
            every: every.max(1),
            max_tokens: 2000,
            timeout: Duration::from_secs(120),
            api_key: std::env::var("ANTHROPIC_API_KEY")
                .ok()
                .filter(|k| !k.trim().is_empty()),
            last_run_at: 0,
        }
    }

    /// Inject a key directly (tests; alternative key sources).
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = command.into();
        self
    }

    pub fn available(&self) -> bool {
        self.api_key.is_some()
    }

    /// Resume the "last consulted at" counter from `dir/cloud_state.txt`.
    pub fn resume(&mut self, dir: &std::path::Path) {
        if let Ok(text) = fs::read_to_string(dir.join("cloud_state.txt")) {
            self.last_run_at = text.trim().parse().unwrap_or(0);
        }
    }

    pub fn due(&self, experiments: usize) -> bool {
        experiments >= self.last_run_at + self.every
    }

    /// Consult the cloud model if due AND available. Writes the analysis to
    /// `dir/deep_analysis_<count>.md` and returns its path. `Ok(None)` means
    /// "not due" or "honestly skipped (no key)"; `Err` is a real request
    /// failure (the campaign continues either way).
    pub fn maybe_analyze(
        &mut self,
        dir: &std::path::Path,
        archive: &ReportArchive,
        graph: &KnowledgeGraph,
        db: &ExperimentDb,
    ) -> Result<Option<PathBuf>, String> {
        if !self.due(db.len()) || !self.available() {
            return Ok(None);
        }
        let prompt = deep_analysis_prompt(archive, graph, db);
        let text = self.message(&prompt)?;
        let path = dir.join(format!("deep_analysis_{:08}.md", db.len()));
        let body = format!(
            "# Deep Scientific Analysis — {} experiments\n\nModel: {}\n\n{}\n",
            db.len(),
            self.model,
            text.trim()
        );
        fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
        self.last_run_at = db.len();
        let _ = fs::write(
            dir.join("cloud_state.txt"),
            format!("{}\n", self.last_run_at),
        );
        Ok(Some(path))
    }

    /// One Messages-API call; returns the completion text.
    pub fn message(&self, prompt: &str) -> Result<String, String> {
        let key = self.api_key.as_deref().ok_or("no ANTHROPIC_API_KEY")?;
        let body = format!(
            "{{\"model\":\"{}\",\"max_tokens\":{},\"messages\":[{{\"role\":\"user\",\"content\":\"{}\"}}]}}",
            json_escape(&self.model),
            self.max_tokens,
            json_escape(prompt)
        );
        let mut child = Command::new(&self.command)
            .arg("-s")
            .arg("-X")
            .arg("POST")
            .arg(&self.api_url)
            .arg("-H")
            .arg(format!("x-api-key: {key}"))
            .arg("-H")
            .arg("anthropic-version: 2023-06-01")
            .arg("-H")
            .arg("content-type: application/json")
            .arg("--data-binary")
            .arg("@-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {}: {e}", self.command))?;
        if let Some(mut stdin) = child.stdin.take() {
            std::thread::spawn(move || {
                let _ = stdin.write_all(body.as_bytes());
            });
        }
        let reader = child.stdout.take().map(|mut out| {
            std::thread::spawn(move || {
                let mut s = String::new();
                let _ = out.read_to_string(&mut s);
                s
            })
        });
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if start.elapsed() > self.timeout {
                        let _ = child.kill();
                        return Err("request timed out".into());
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(format!("wait {}: {e}", self.command)),
            }
        }
        let raw = reader.and_then(|h| h.join().ok()).unwrap_or_default();
        // Messages API: {"content":[{"type":"text","text":"..."}], ...}
        let text =
            extract_json_string_field(&raw, "text").ok_or_else(
                || match extract_json_string_field(&raw, "message") {
                    Some(msg) => format!("API error: {msg}"),
                    None => "no text in API reply".to_string(),
                },
            )?;
        if text.trim().is_empty() {
            Err("empty completion".into())
        } else {
            Ok(text)
        }
    }
}

/// The cloud model reads ONLY distilled knowledge + aggregates (Task 5).
pub fn deep_analysis_prompt(
    archive: &ReportArchive,
    graph: &KnowledgeGraph,
    db: &ExperimentDb,
) -> String {
    let mut p = String::new();
    p.push_str(
        "You are a senior research scientist reviewing an autonomous optimization-research \
platform for Ising/QUBO problems. Below are its cumulative research reports, its knowledge \
graph, and aggregate statistics. Write an in-depth scientific analysis: which findings are \
solid, which are likely artifacts, what physical mechanisms explain them, and which NEW \
hybrid operators or research directions the platform should try next. Be specific and \
skeptical; every recommendation needs a mechanism.\n\n",
    );
    p.push_str(&format!(
        "AGGREGATES: {} experiments; best score {:.2}; {} knowledge-graph facts.\n\n",
        db.len(),
        db.best().map(|b| b.score).unwrap_or(f64::NAN),
        graph.len()
    ));
    p.push_str(&ResearchMemory::context(archive, graph, 24_000));
    p
}

#[cfg(test)]
mod tests {
    use super::super::meta_learner::MetaLearner;
    use super::*;

    #[test]
    fn skips_honestly_without_key_and_respects_cadence() {
        let dir = std::env::temp_dir().join(format!("cloud_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let archive = ReportArchive::open(dir.join("reports"), 100).unwrap();
        let graph = KnowledgeGraph::new();
        let db = ExperimentDb::new();
        let mut cloud = CloudScientist {
            api_key: None, // no key regardless of the test environment
            ..CloudScientist::new("claude-sonnet-5", 10)
        };
        assert!(!cloud.available());
        // Not due AND unavailable ⇒ Ok(None), never an error, nothing written.
        assert_eq!(
            cloud.maybe_analyze(&dir, &archive, &graph, &db).unwrap(),
            None
        );
        assert!(cloud.due(10));
        assert!(!cloud.due(9));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn request_failure_is_an_error_not_a_fabrication() {
        let dir = std::env::temp_dir().join(format!("cloud_err_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let archive = ReportArchive::open(dir.join("reports"), 100).unwrap();
        let graph = KnowledgeGraph::new();
        let mut db = ExperimentDb::new();
        for _ in 0..12 {
            db.record(Default::default());
        }
        let mut cloud = CloudScientist::new("claude-sonnet-5", 10)
            .with_api_key("test-key")
            .with_command("/nonexistent/curl-xyz");
        let res = cloud.maybe_analyze(&dir, &archive, &graph, &db);
        assert!(res.is_err(), "unreachable API must surface as Err: {res:?}");
        // No analysis file was fabricated.
        assert!(!dir.join("deep_analysis_00000012.md").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prompt_contains_only_distilled_knowledge() {
        let dir = std::env::temp_dir().join(format!("cloud_prompt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut archive = ReportArchive::open(dir.join("reports"), 1).unwrap();
        let mut graph = KnowledgeGraph::new();
        graph.observe_if("a", "precedes-well", "b", "density<0.05", 3.0, "exp 1-9");
        let mut db = ExperimentDb::new();
        db.record(Default::default());
        archive
            .force_report(&db, &graph, &MetaLearner::new())
            .unwrap();
        let p = deep_analysis_prompt(&archive, &graph, &db);
        assert!(p.contains("AGGREGATES: 1 experiments"));
        assert!(p.contains("IF density<0.05"));
        assert!(p.contains("# Research Report"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resume_restores_cadence_counter() {
        let dir = std::env::temp_dir().join(format!("cloud_resume_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("cloud_state.txt"), "500\n").unwrap();
        let mut cloud = CloudScientist::new("claude-sonnet-5", 100);
        cloud.resume(&dir);
        assert!(!cloud.due(599));
        assert!(cloud.due(600));
        let _ = fs::remove_dir_all(&dir);
    }
}
