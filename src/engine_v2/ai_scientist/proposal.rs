//! Operator constructor (Stage 6, Task 7) — turns a mined operator GAP (or an
//! LLM suggestion) into a structured `operator_proposal_<name>.md` DRAFT:
//! idea, mathematical description, pseudocode, expected properties, required
//! capabilities, expected complexity.
//!
//! It deliberately writes NO Rust code. A proposal is a research artifact for a
//! human (or a later, supervised implementation pass) to review; automatic code
//! generation into the verified operator set would bypass the Harness Checker.

use super::super::capability::{Capability, Complexity};
use super::super::registry::OperatorRegistry;
use super::meta_learner::OperatorGap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A structured draft for a NEW operator. All fields are prose/pseudocode.
#[derive(Debug, Clone)]
pub struct OperatorProposal {
    pub name: String,
    pub idea: String,
    pub math: String,
    pub pseudocode: String,
    pub expected_properties: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub expected_complexity: String,
    /// Where the proposal came from (mined gap, LLM, ...) — auditable.
    pub provenance: String,
}

fn capability_tags(reg: &OperatorRegistry, op: &str) -> Vec<String> {
    let Some(d) = reg.metadata(op) else {
        return Vec::new();
    };
    [
        (Capability::Exploration, "Exploration"),
        (Capability::Exploitation, "Exploitation"),
        (Capability::BarrierCrossing, "BarrierCrossing"),
        (Capability::ExactInference, "ExactInference"),
        (Capability::Approximate, "Approximate"),
        (Capability::Warmstart, "Warmstart"),
    ]
    .iter()
    .filter(|(c, _)| d.capabilities.contains(*c))
    .map(|(_, tag)| tag.to_string())
    .collect()
}

fn complexity_bound(reg: &OperatorRegistry, ops: [&str; 2]) -> String {
    let rank = |c: Complexity| match c {
        Complexity::Constant => 0,
        Complexity::LinearVars => 1,
        Complexity::LinearEdges => 2,
        Complexity::Superlinear => 3,
    };
    let worst = ops
        .iter()
        .filter_map(|op| reg.metadata(op).map(|d| d.complexity))
        .max_by_key(|c| rank(*c));
    match worst {
        Some(Complexity::Constant) => "O(1) amortized per touched site".into(),
        Some(Complexity::LinearVars) => "O(n) per sweep (linear in variables)".into(),
        Some(Complexity::LinearEdges) => "O(|E|) per sweep (one pass over all edges)".into(),
        Some(Complexity::Superlinear) => "worse than O(|E|) per sweep (cluster growth)".into(),
        None => "unknown — parents not in the registry".into(),
    }
}

impl OperatorProposal {
    /// Deterministic draft from a mined gap: a fused operator between the two
    /// most frequently adjacent operators in the best solutions.
    pub fn from_gap(gap: &OperatorGap, reg: &OperatorRegistry) -> Self {
        let mut caps = capability_tags(reg, &gap.predecessor);
        for c in capability_tags(reg, &gap.successor) {
            if !caps.contains(&c) {
                caps.push(c);
            }
        }
        Self {
            name: gap.suggested_name.clone(),
            idea: format!(
                "Fuse `{}` and `{}` into one operator. Evidence: {}",
                gap.predecessor, gap.successor, gap.rationale
            ),
            math: format!(
                "Let S be the spin state and ΔE_v the local field of variable v.\n\
Phase 1 applies the acceptance rule of `{}` to propose a move set M;\n\
Phase 2 immediately applies the update rule of `{}` RESTRICTED to the\n\
neighborhood N(M) of variables touched in Phase 1, instead of a full pass.\n\
The fused kernel therefore computes ΔE once per touched edge and reuses it\n\
across both phases, which the two-operator sequence cannot.",
                gap.predecessor, gap.successor
            ),
            pseudocode: format!(
                "for sweep in 0..sweeps:\n\
\u{20}   M ← {}(S, T)              # phase 1: proposal/thermal pass\n\
\u{20}   for v in neighborhood(M):  # phase 2 fused on the touched region\n\
\u{20}       S ← {}_step(S, v)\n\
\u{20}   record observables",
                gap.predecessor, gap.successor
            ),
            expected_properties: vec![
                format!(
                    "strictly cheaper than the sequence `{} → {}` (shared ΔE, one memory pass)",
                    gap.predecessor, gap.successor
                ),
                "same fixed points as the two-operator sequence".into(),
                "deterministic given (seed, sweep order) — required by the Harness Checker".into(),
            ],
            required_capabilities: caps,
            expected_complexity: complexity_bound(reg, [&gap.predecessor, &gap.successor]),
            provenance: format!(
                "meta-learner gap: {} → {} adjacent {}x in top solutions",
                gap.predecessor, gap.successor, gap.support
            ),
        }
    }

    /// Draft from an LLM suggestion: the model supplies the idea text; the
    /// structural sections are templated for a human to complete. Nothing about
    /// the idea is invented beyond what the model wrote.
    pub fn from_llm(name: &str, idea: &str, provenance: &str) -> Self {
        let sane: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        Self {
            name: sane,
            idea: idea.trim().to_string(),
            math: "(to be formalized during review — the LLM idea above is the only source)".into(),
            pseudocode: "(to be written during review)".into(),
            expected_properties: vec![
                "deterministic given (seed, sweep order) — required by the Harness Checker".into(),
            ],
            required_capabilities: Vec::new(),
            expected_complexity: "unknown until formalized".into(),
            provenance: provenance.to_string(),
        }
    }

    /// Write `dir/operator_proposal_<name>.md`. Returns the path.
    pub fn write(&self, dir: impl AsRef<Path>) -> io::Result<PathBuf> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let path = dir.join(format!("operator_proposal_{}.md", self.name));
        let mut out = String::new();
        out.push_str(&format!("# Operator Proposal: `{}`\n\n", self.name));
        out.push_str(&format!("Provenance: {}\n\n", self.provenance));
        out.push_str("## Idea\n\n");
        out.push_str(&self.idea);
        out.push_str("\n\n## Mathematical description\n\n");
        out.push_str(&self.math);
        out.push_str("\n\n## Pseudocode\n\n```text\n");
        out.push_str(&self.pseudocode);
        out.push_str("\n```\n\n## Expected properties\n\n");
        for p in &self.expected_properties {
            out.push_str(&format!("- {p}\n"));
        }
        out.push_str("\n## Required capabilities\n\n");
        if self.required_capabilities.is_empty() {
            out.push_str("- (to be determined during review)\n");
        }
        for c in &self.required_capabilities {
            out.push_str(&format!("- {c}\n"));
        }
        out.push_str(&format!(
            "\n## Expected complexity\n\n{}\n\n\
## Status\n\nDRAFT — no implementation exists. Per Stage 6 policy the platform \
never generates operator code; implementation requires human review and the \
full verification track.\n",
            self.expected_complexity
        ));
        fs::write(&path, out)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap() -> OperatorGap {
        OperatorGap {
            predecessor: "gibbs_color_sweep".into(),
            successor: "greedy_descent".into(),
            suggested_name: "gibbs_greedy".into(),
            rationale: "most frequent transition in the best solutions (9x)".into(),
            support: 9,
        }
    }

    #[test]
    fn gap_proposal_has_all_sections_and_no_rust_code() {
        let reg = OperatorRegistry::standard();
        let prop = OperatorProposal::from_gap(&gap(), &reg);
        assert_eq!(prop.name, "gibbs_greedy");
        assert!(!prop.required_capabilities.is_empty());
        assert!(prop.expected_complexity.contains("O("));

        let dir = std::env::temp_dir().join(format!("prop_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = prop.write(&dir).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        for section in [
            "## Idea",
            "## Mathematical description",
            "## Pseudocode",
            "## Expected properties",
            "## Required capabilities",
            "## Expected complexity",
            "DRAFT — no implementation exists",
        ] {
            assert!(text.contains(section), "missing {section}");
        }
        // The draft must not contain Rust implementation code.
        assert!(!text.contains("fn apply"));
        assert!(!text.contains("impl Operator"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn llm_proposal_sanitizes_name_and_keeps_idea_verbatim() {
        let prop = OperatorProposal::from_llm(
            "Cluster-Greedy Thermal!",
            "  Grow Houdayer clusters at high T, then greedy-quench each cluster. ",
            "cloud analysis over 10000 experiments",
        );
        assert_eq!(prop.name, "cluster_greedy_thermal_");
        assert_eq!(
            prop.idea,
            "Grow Houdayer clusters at high T, then greedy-quench each cluster."
        );
        let dir = std::env::temp_dir().join(format!("prop_llm_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = prop.write(&dir).unwrap();
        assert!(path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("operator_proposal_cluster_greedy_thermal_"));
        let _ = fs::remove_dir_all(&dir);
    }
}
