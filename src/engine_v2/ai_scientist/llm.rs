//! LLM scientist (Stage 5++). Plugs a local LLM (Ollama, e.g.
//! `qwen2.5-coder:7b`) into the `Ideator` slot. The LLM plays SCIENTIST, not
//! compute engine: it reads the aggregated results + knowledge graph and
//! proposes 20–100 REASONED ideas (operator sequences with a causal argument).
//! The Evolution Engine then multiplies those into hundreds of thousands of
//! variants and the deterministic core evaluates them.
//!
//! No new crates: we shell out to the `ollama` CLI (prompt on stdin, text on
//! stdout) with a hard timeout. If the model is missing, times out, or returns
//! nothing parseable, we FALL BACK to the deterministic heuristic generator —
//! so the lab always works, and tests never depend on a running LLM.

use super::super::capability::Capability;
use super::lab::{HypothesisGenerator, Ideator, ResearchBrief};
use super::scientist::{Hypothesis, HypothesisStatus};
use rand_chacha::ChaCha8Rng;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Client for the Ollama HTTP API via `curl` (no HTTP crate). We use
/// `/api/generate` with `"stream": false`, which returns ONE clean JSON object —
/// avoiding the terminal-redraw control codes the `ollama run` CLI emits.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    /// The HTTP client command (default `curl`); tests point it at a bogus path.
    pub command: String,
    pub host: String,
    pub model: String,
    pub timeout: Duration,
}

impl OllamaClient {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            command: "curl".into(),
            host: "http://localhost:11434".into(),
            model: model.into(),
            timeout: Duration::from_secs(180),
        }
    }

    /// Override the HTTP client command (for tests: a nonexistent path forces
    /// the deterministic fallback).
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = command.into();
        self
    }

    /// POST `prompt` to `/api/generate` and return the model's completion.
    /// Non-streaming ⇒ the response is clean. Reads/writes on threads (no pipe
    /// deadlock) and kills the child on timeout; any failure is an `Err` the
    /// caller turns into a fallback.
    pub fn generate(&self, prompt: &str) -> Result<String, String> {
        let body = format!(
            "{{\"model\":\"{}\",\"prompt\":\"{}\",\"stream\":false}}",
            json_escape(&self.model),
            json_escape(prompt)
        );
        let url = format!("{}/api/generate", self.host);
        let mut child = Command::new(&self.command)
            .arg("-s")
            .arg("-X")
            .arg("POST")
            .arg(&url)
            .arg("--data-binary")
            .arg("@-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {}: {e}", self.command))?;

        // Feed the request body on a thread so it can't deadlock.
        if let Some(mut stdin) = child.stdin.take() {
            std::thread::spawn(move || {
                let _ = stdin.write_all(body.as_bytes());
            });
        }
        let stdout = child.stdout.take();
        let reader = stdout.map(|mut out| {
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
        let text = extract_json_string_field(&raw, "response")
            .map(|t| strip_ansi(&t))
            .ok_or_else(|| "no response field in reply".to_string())?;
        if text.trim().is_empty() {
            Err("empty completion".into())
        } else {
            Ok(text)
        }
    }
}

/// Escape a string for embedding in a JSON string literal.
pub fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// Extract a top-level JSON string field's value (unescaped). Minimal, tolerant:
/// finds `"<key>":`, then reads the quoted string, handling standard escapes.
pub fn extract_json_string_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    let mut chars = rest.strip_prefix('"')?.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(ch);
                    }
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None // unterminated string
}

/// The `ollama` CLI streams tokens with terminal control codes (cursor moves,
/// erase-line, bracketed-paste) even to a pipe. Strip ANSI/CSI escapes and stray
/// control characters so the parser sees clean text.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // ESC: consume an escape sequence.
            match chars.next() {
                Some('[') => {
                    // CSI: bytes until a final byte in 0x40..=0x7E.
                    for f in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&f) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC: until BEL or ESC\.
                    while let Some(f) = chars.next() {
                        if f == '\u{7}' {
                            break;
                        }
                        if f == '\u{1b}' {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if c == '\n' || c == '\t' || !c.is_control() {
            out.push(c);
        }
        // other control chars (including \r) are dropped
    }
    out
}

/// Build the research prompt from the brief. Gives the LLM the deep structural
/// signals (including the clustering coefficient), the operator "menu" with
/// capabilities, the current best, what has worked, and the knowledge graph —
/// then asks for a strict, parseable list of ideas.
pub fn build_prompt(brief: &ResearchBrief, n: usize) -> String {
    let s = brief.stats;
    let mut p = String::new();
    p.push_str(
        "You are a research scientist designing optimization algorithms for Ising/QUBO problems. \
You compose PHYSICS-INSPIRED OPERATORS into sequences. Reason about the instance structure and \
past results, then propose new operator sequences likely to beat the current best.\n\n",
    );
    p.push_str(&format!(
        "INSTANCE: n={}, density={:.4}, mean_degree={:.2}, degree_cv={:.2}, clustering_coeff={:.3}, {}.\n",
        s.n, s.density, s.mean_degree, s.degree_cv, s.clustering, brief.context
    ));

    p.push_str("\nAVAILABLE OPERATORS (use ONLY these names):\n");
    for name in brief.pool {
        let caps = brief
            .registry
            .metadata(name)
            .map(|d| {
                let mut v = Vec::new();
                for (c, tag) in [
                    (Capability::Exploration, "exploration"),
                    (Capability::Exploitation, "exploitation"),
                    (Capability::BarrierCrossing, "barrier-crossing"),
                ] {
                    if d.capabilities.contains(c) {
                        v.push(tag);
                    }
                }
                v.join(",")
            })
            .unwrap_or_default();
        p.push_str(&format!("  - {name} [{caps}]\n"));
    }

    if let Some(best) = &brief.analysis.best_schedule {
        p.push_str(&format!(
            "\nCURRENT BEST: {:?}  (score {:.1}; baseline {:.1})\n",
            best.ops, brief.analysis.best_score, brief.analysis.baseline
        ));
    }
    if !brief.analysis.operator_scores.is_empty() {
        p.push_str("OPERATOR IMPROVEMENT (mean over baseline, best first):\n");
        for (op, score, cnt) in brief.analysis.operator_scores.iter().take(8) {
            p.push_str(&format!("  {op}: {score:+.2} (n={cnt})\n"));
        }
    }
    let facts = brief.graph.triples();
    if !facts.is_empty() {
        p.push_str("KNOWLEDGE GRAPH (learned patterns):\n");
        let mut f: Vec<_> = facts.iter().collect();
        f.sort_by(|a, b| b.weight.total_cmp(&a.weight));
        for t in f.iter().take(8) {
            p.push_str(&format!(
                "  {} --{}--> {} (w={:+.1}, support={})\n",
                t.subject, t.predicate, t.object, t.weight, t.support
            ));
        }
    }

    p.push_str(&format!(
        "\nPropose {n} DIVERSE operator sequences (2-4 operators each). For each, output EXACTLY one line:\n\
OPS: <op1>, <op2>, ... | WHY: <one-sentence causal reason>\n\
Use only the operator names listed above. Output only these lines, nothing else.\n"
    ));
    p
}

/// Parse the LLM completion into (operators, reasoning) proposals. Tolerant of
/// extra prose: it scans every line for the `OPS: ... | WHY: ...` pattern and
/// keeps only operators that exist in `pool`.
pub fn parse_proposals(text: &str, pool: &[&'static str]) -> Vec<(Vec<String>, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches(['-', '*', '#', ' ']);
        let upper = line.to_ascii_uppercase();
        let Some(ops_pos) = upper.find("OPS:") else {
            continue;
        };
        let rest = &line[ops_pos + 4..];
        let (ops_part, why_part) = match rest.to_ascii_uppercase().find("WHY:") {
            Some(w) => (&rest[..w], rest[w + 4..].trim().to_string()),
            None => (rest, String::new()),
        };
        let ops: Vec<String> = ops_part
            .split([',', '|'])
            .map(|o| o.trim().to_string())
            .filter(|o| pool.iter().any(|p| p.eq_ignore_ascii_case(o)))
            // normalize to the canonical pool spelling
            .filter_map(|o| pool.iter().find(|p| p.eq_ignore_ascii_case(&o)).map(|p| p.to_string()))
            .collect();
        if !ops.is_empty() {
            out.push((ops, why_part));
        }
    }
    out
}

/// An `Ideator` backed by an LLM, with a heuristic fallback.
pub struct LlmHypothesisGenerator {
    client: OllamaClient,
    fallback: HypothesisGenerator,
    next_id: u64,
}

impl LlmHypothesisGenerator {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            client: OllamaClient::new(model),
            fallback: HypothesisGenerator::new(),
            next_id: 100_000, // keep LLM ids clearly distinct from heuristic ids
        }
    }

    /// Use a custom HTTP-client command (for tests: a bogus path forces the
    /// deterministic fallback).
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.client = self.client.with_command(command);
        self
    }

    /// Point at a custom Ollama host (e.g. a remote/cluster inference server).
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.client.host = host.into();
        self
    }

    fn make(&mut self, ops: Vec<String>, reasoning: String) -> Hypothesis {
        let id = self.next_id;
        self.next_id += 1;
        Hypothesis {
            id,
            operators: ops,
            rationale: "LLM proposal".into(),
            reasoning: if reasoning.is_empty() {
                "proposed by the LLM scientist".into()
            } else {
                reasoning
            },
            predicted_improvement: 0.0,
            status: HypothesisStatus::Proposed,
            observed_improvement: 0.0,
            confidence: 0.0,
        }
    }
}

impl Ideator for LlmHypothesisGenerator {
    fn propose(
        &mut self,
        brief: &ResearchBrief,
        n: usize,
        rng: &mut ChaCha8Rng,
    ) -> Vec<Hypothesis> {
        let prompt = build_prompt(brief, n);
        match self.client.generate(&prompt) {
            Ok(text) => {
                let proposals = parse_proposals(&text, brief.pool);
                if proposals.is_empty() {
                    // LLM produced nothing usable — fall back deterministically.
                    return self.fallback.propose(brief, n, rng);
                }
                let mut hyps: Vec<Hypothesis> = proposals
                    .into_iter()
                    .take(n)
                    .map(|(ops, why)| self.make(ops, why))
                    .collect();
                // Top up with heuristic ideas if the LLM was terse.
                if hyps.len() < n {
                    let extra = self.fallback.propose(brief, n - hyps.len(), rng);
                    hyps.extend(extra);
                }
                hyps
            }
            // LLM unavailable → deterministic heuristic. The lab never stalls.
            Err(_) => self.fallback.propose(brief, n, rng),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentDb;
    use super::super::executor::RuntimeExecutor;
    use super::super::lab::{LabConfig, ScientificLab};
    use super::*; // llm module items (build_prompt, parse_proposals, LlmHypothesisGenerator)
    use crate::engine_v2::evolution::Evolver;
    use crate::engine_v2::ir::ProblemIR;
    use crate::engine_v2::knowledge::KnowledgeBase;
    use crate::engine_v2::registry::OperatorRegistry;

    #[test]
    fn json_escape_and_extract_roundtrip() {
        let s = "line1\nquote:\" backslash:\\ tab:\t done";
        let json = format!("{{\"response\":\"{}\",\"done\":true}}", json_escape(s));
        let got = extract_json_string_field(&json, "response").unwrap();
        assert_eq!(got, s);
        // unicode escape from a real API reply
        assert_eq!(
            extract_json_string_field(r#"{"response":"café"}"#, "response").as_deref(),
            Some("café")
        );
        assert_eq!(
            extract_json_string_field(r#"{"other":"x"}"#, "response"),
            None
        );
    }

    #[test]
    fn strip_ansi_removes_control_codes() {
        let dirty = "\u{1b}[?2026h\u{1b}[?25lOPS: gibbs_color_sweep\u{1b}[3D\u{1b}[K | WHY: reset\u{1b}[1D done\r\n";
        let clean = strip_ansi(dirty);
        assert!(!clean.contains('\u{1b}'));
        assert!(!clean.contains('\r'));
        assert!(clean.contains("OPS: gibbs_color_sweep"));
        assert!(clean.contains("WHY: reset done"));
    }

    #[test]
    fn parser_extracts_valid_operators_and_ignores_prose() {
        let pool: Vec<&'static str> =
            vec!["gibbs_color_sweep", "greedy_descent", "metropolis_sweep"];
        let text = "\
Sure! Here are my ideas:\n\
OPS: metropolis_sweep, greedy_descent | WHY: escape then refine\n\
some rambling line without the pattern\n\
- OPS: Gibbs_Color_Sweep, nonexistent_op, GREEDY_DESCENT | WHY: case-insensitive + drop unknowns\n\
OPS: not_a_real_operator | WHY: should be dropped entirely\n";
        let props = parse_proposals(text, &pool);
        assert_eq!(props.len(), 2);
        assert_eq!(props[0].0, vec!["metropolis_sweep", "greedy_descent"]);
        assert_eq!(props[0].1, "escape then refine");
        // case-insensitive match, unknown operator dropped, canonical spelling
        assert_eq!(props[1].0, vec!["gibbs_color_sweep", "greedy_descent"]);
    }

    #[test]
    fn falls_back_to_heuristic_when_llm_unavailable() {
        // Point at a binary that does not exist ⇒ generate() errors ⇒ fallback.
        let ir = ProblemIR::from_pairs(
            6,
            0.0,
            vec![0.0; 6],
            &[(0, 1, 1.0), (1, 2, -1.0), (2, 3, 1.0)],
        );
        let reg = OperatorRegistry::standard();
        let evolver = Evolver::new(Default::default());
        let exec = RuntimeExecutor::new(2);
        let ideator = Box::new(
            LlmHypothesisGenerator::new("qwen2.5-coder:7b")
                .with_command("/nonexistent/curl-binary-xyz"),
        );
        let cfg = LabConfig {
            rounds: 2,
            hypotheses_per_round: 5,
            batch_size: 12,
            seeds_per_hypothesis: 2,
            num_replicas: 8,
            base_seed: 3,
            ..Default::default()
        };
        let mut db = ExperimentDb::new();
        let mut kb = KnowledgeBase::new();
        let mut lab = ScientificLab::with_ideator(3, Default::default(), ideator);
        let report = lab.run(&ir, &reg, &evolver, &exec, &mut db, &mut kb, &cfg);
        // Despite the LLM being unavailable, the lab ran end-to-end.
        assert!(report.experiments_run > 0);
        assert!(report.best_score <= report.baseline + 1e-9);
        assert_eq!(ir.energy(&report.best_state), report.best_score);
    }

    #[test]
    fn prompt_includes_clustering_and_operators() {
        use super::super::graph::KnowledgeGraph;
        use crate::engine_v2::ai_scientist::scientist::AIScientist;
        use crate::engine_v2::decision::DecisionEngine;
        let ir = ProblemIR::from_pairs(
            3,
            0.0,
            vec![0.0; 3],
            &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)],
        );
        let reg = OperatorRegistry::standard();
        let stats = DecisionEngine::analyze(&ir);
        let db = ExperimentDb::new();
        let analysis = AIScientist::analyze(&db);
        let pool = DecisionEngine::operator_pool(&stats, stats.select_backend(), &reg);
        let graph = KnowledgeGraph::new();
        let brief = ResearchBrief {
            stats: &stats,
            analysis: &analysis,
            graph: &graph,
            pool: &pool,
            registry: &reg,
            context: "dense",
        };
        let prompt = build_prompt(&brief, 20);
        assert!(prompt.contains("clustering_coeff"));
        assert!(prompt.contains("gibbs_color_sweep"));
        assert!(prompt.contains("OPS:"));
    }
}
