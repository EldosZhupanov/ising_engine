//! Knowledge Graph (Stage 5+) — structured, queryable memory, not a flat table.
//! Facts are typed triples `(subject) —predicate→ (object)` carrying an evidence
//! WEIGHT (running mean, e.g. mean improvement) and SUPPORT (how many times
//! observed). The Knowledge Manager writes only reproducible, statistically
//! supported patterns here, so the graph is durable knowledge:
//!
//!   greedy_descent —precedes-well→ (nothing)          (weight = mean improvement)
//!   gibbs_color_sweep —precedes-well→ greedy_descent   (weight, support)
//!   houdayer_cluster —effective-on→ sparse
//!   metropolis_sweep —provides→ Exploration
//!
//! Dependency-free text persistence (no serde), so the graph is inspectable.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// A weighted, evidence-counted fact, optionally CONDITIONED on instance
/// structure ("A precedes-well B IF density<0.05&clustering>0.4"). `proof`
/// records where the evidence came from so every fact is auditable.
#[derive(Debug, Clone, PartialEq)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    /// Structural condition under which the fact holds; empty = unconditional.
    pub condition: String,
    /// Running mean of the observed effect (e.g. improvement over baseline).
    pub weight: f64,
    /// Number of observations backing this fact.
    pub support: u32,
    /// Sum of squared deviations of the observed effects (Welford), for the
    /// consistency term of `confidence()`.
    pub m2: f64,
    /// Provenance: which experiments/analysis produced this fact.
    pub proof: String,
}

impl Triple {
    /// Evidence strength in [0,1]: grows with support, shrinks with noise.
    /// confidence = (1 − e^(−support/5)) · 1/(1 + cv), cv = std/|mean|.
    pub fn confidence(&self) -> f64 {
        let sup = 1.0 - (-(self.support as f64) / 5.0).exp();
        if self.support < 2 {
            return sup * 0.5;
        }
        let var = self.m2 / (self.support as f64 - 1.0);
        let cv = var.sqrt() / self.weight.abs().max(1e-12);
        sup / (1.0 + cv)
    }
}

#[derive(Debug, Clone, Default)]
pub struct KnowledgeGraph {
    triples: Vec<Triple>,
    index: HashMap<(String, String, String, String), usize>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.triples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triples.is_empty()
    }

    pub fn triples(&self) -> &[Triple] {
        &self.triples
    }

    /// Observe an unconditional fact with an effect `weight`. Merges into the
    /// existing triple (running mean, incremented support) or inserts a new one.
    pub fn observe(&mut self, subject: &str, predicate: &str, object: &str, weight: f64) {
        self.observe_if(subject, predicate, object, "", weight, "");
    }

    /// Observe a fact holding under `condition` (e.g. "density<0.05"), carrying
    /// `proof` provenance. Facts with different conditions are distinct.
    pub fn observe_if(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        condition: &str,
        weight: f64,
        proof: &str,
    ) {
        let sane = |s: &str| s.replace('|', "/");
        let key = (
            sane(subject),
            sane(predicate),
            sane(object),
            sane(condition),
        );
        if let Some(&i) = self.index.get(&key) {
            let t = &mut self.triples[i];
            // Welford update: running mean + sum of squared deviations.
            let n = t.support as f64 + 1.0;
            let delta = weight - t.weight;
            t.weight += delta / n;
            t.m2 += delta * (weight - t.weight);
            t.support += 1;
            if !proof.is_empty() {
                t.proof = sane(proof);
            }
        } else {
            let i = self.triples.len();
            self.triples.push(Triple {
                subject: key.0.clone(),
                predicate: key.1.clone(),
                object: key.2.clone(),
                condition: key.3.clone(),
                weight,
                support: 1,
                m2: 0.0,
                proof: sane(proof),
            });
            self.index.insert(key, i);
        }
    }

    /// All facts about `subject` with `predicate`, strongest (highest weight)
    /// first. Empty predicate matches any.
    pub fn query(&self, subject: &str, predicate: &str) -> Vec<&Triple> {
        let mut out: Vec<&Triple> = self
            .triples
            .iter()
            .filter(|t| t.subject == subject && (predicate.is_empty() || t.predicate == predicate))
            .collect();
        out.sort_by(|a, b| b.weight.total_cmp(&a.weight).then(a.object.cmp(&b.object)));
        out
    }

    /// The best `object` for `(subject, predicate)` by weight, if any is
    /// supported by at least `min_support` observations.
    pub fn best_object(&self, subject: &str, predicate: &str, min_support: u32) -> Option<&str> {
        self.query(subject, predicate)
            .into_iter()
            .find(|t| t.support >= min_support)
            .map(|t| t.object.as_str())
    }

    /// All facts about `subject` whose condition is satisfied by the given
    /// instance features (unconditional facts always match), strongest first.
    pub fn query_applicable(
        &self,
        subject: &str,
        predicate: &str,
        density: f64,
        clustering: f64,
    ) -> Vec<&Triple> {
        let mut out: Vec<&Triple> = self
            .triples
            .iter()
            .filter(|t| {
                t.subject == subject
                    && (predicate.is_empty() || t.predicate == predicate)
                    && condition_holds(&t.condition, density, clustering)
            })
            .collect();
        out.sort_by(|a, b| b.weight.total_cmp(&a.weight).then(a.object.cmp(&b.object)));
        out
    }

    // ---- persistence ---------------------------------------------------------
    // v1 (5 fields): subject|predicate|object|weight|support
    // v2 (8 fields): subject|predicate|object|condition|weight|support|m2|proof
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut out = String::new();
        for t in &self.triples {
            out.push_str(&format!(
                "{}|{}|{}|{}|{}|{}|{}|{}\n",
                t.subject, t.predicate, t.object, t.condition, t.weight, t.support, t.m2, t.proof
            ));
        }
        fs::write(path, out)
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => return Err(e),
        };
        let mut g = Self::new();
        for line in text.lines() {
            let f: Vec<&str> = line.split('|').collect();
            let (condition, weight, support, m2, proof) = match f.len() {
                5 => (String::new(), f[3], f[4], 0.0, String::new()),
                8 => (
                    f[3].to_string(),
                    f[4],
                    f[5],
                    f[6].parse().unwrap_or(0.0),
                    f[7].to_string(),
                ),
                _ => continue,
            };
            let key = (
                f[0].to_string(),
                f[1].to_string(),
                f[2].to_string(),
                condition.clone(),
            );
            let i = g.triples.len();
            g.triples.push(Triple {
                subject: key.0.clone(),
                predicate: key.1.clone(),
                object: key.2.clone(),
                condition,
                weight: weight.parse().unwrap_or(0.0),
                support: support.parse().unwrap_or(1),
                m2,
                proof,
            });
            g.index.insert(key, i);
        }
        Ok(g)
    }
}

/// Evaluate a conjunctive condition string like "density<0.05&clustering>=0.4"
/// against instance features. Unknown atoms are conservative: they FAIL, so a
/// malformed condition never lets a fact apply where it should not. Empty
/// conditions always hold.
pub fn condition_holds(condition: &str, density: f64, clustering: f64) -> bool {
    if condition.trim().is_empty() {
        return true;
    }
    condition.split('&').all(|atom| {
        let atom = atom.trim();
        let (var, rest) = if let Some(r) = atom.strip_prefix("density") {
            (density, r)
        } else if let Some(r) = atom.strip_prefix("clustering") {
            (clustering, r)
        } else {
            return false;
        };
        let (op, num) = if let Some(n) = rest.strip_prefix(">=") {
            (">=", n)
        } else if let Some(n) = rest.strip_prefix("<=") {
            ("<=", n)
        } else if let Some(n) = rest.strip_prefix('>') {
            (">", n)
        } else if let Some(n) = rest.strip_prefix('<') {
            ("<", n)
        } else {
            return false;
        };
        let Ok(threshold) = num.trim().parse::<f64>() else {
            return false;
        };
        match op {
            ">=" => var >= threshold,
            "<=" => var <= threshold,
            ">" => var > threshold,
            "<" => var < threshold,
            _ => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_merges_running_mean_and_support() {
        let mut g = KnowledgeGraph::new();
        g.observe("gibbs_color_sweep", "precedes-well", "greedy_descent", 10.0);
        g.observe("gibbs_color_sweep", "precedes-well", "greedy_descent", 20.0);
        assert_eq!(g.len(), 1);
        let t = &g.query("gibbs_color_sweep", "precedes-well")[0];
        assert_eq!(t.support, 2);
        assert!((t.weight - 15.0).abs() < 1e-9);
    }

    #[test]
    fn best_object_respects_support_threshold() {
        let mut g = KnowledgeGraph::new();
        g.observe("op", "effective-on", "sparse", 5.0);
        assert_eq!(g.best_object("op", "effective-on", 2), None); // only 1 obs
        g.observe("op", "effective-on", "sparse", 7.0);
        assert_eq!(g.best_object("op", "effective-on", 2), Some("sparse"));
    }

    #[test]
    fn conditional_facts_are_distinct_and_filtered_by_features() {
        let mut g = KnowledgeGraph::new();
        g.observe_if(
            "cluster",
            "effective-on",
            "graph",
            "density<0.05",
            12.0,
            "exp 1-8",
        );
        g.observe_if(
            "cluster",
            "effective-on",
            "graph",
            "density>=0.05",
            -3.0,
            "exp 9-16",
        );
        assert_eq!(g.len(), 2); // same (s,p,o), different condition ⇒ distinct
        let sparse = g.query_applicable("cluster", "effective-on", 0.01, 0.3);
        assert_eq!(sparse.len(), 1);
        assert_eq!(sparse[0].weight, 12.0);
        assert_eq!(sparse[0].proof, "exp 1-8");
        let dense = g.query_applicable("cluster", "effective-on", 0.2, 0.3);
        assert_eq!(dense.len(), 1);
        assert_eq!(dense[0].weight, -3.0);
    }

    #[test]
    fn condition_parser_is_conservative() {
        assert!(condition_holds("", 0.5, 0.5));
        assert!(condition_holds("density<0.05&clustering>0.4", 0.01, 0.5));
        assert!(!condition_holds("density<0.05&clustering>0.4", 0.01, 0.3));
        assert!(condition_holds("density>=0.05", 0.05, 0.0));
        // Unknown variable or malformed atom ⇒ fact does NOT apply.
        assert!(!condition_holds("entropy<1.0", 0.0, 0.0));
        assert!(!condition_holds("density~0.05", 0.0, 0.0));
    }

    #[test]
    fn confidence_grows_with_support_and_penalizes_noise() {
        let mut g = KnowledgeGraph::new();
        for _ in 0..10 {
            g.observe_if("a", "p", "b", "", 10.0, ""); // perfectly consistent
        }
        for (i, w) in [30.0, -10.0, 25.0, -15.0, 20.0, -5.0, 15.0, 0.0, 10.0, 5.0]
            .iter()
            .enumerate()
        {
            g.observe_if("c", "p", "d", "", *w, &format!("exp {i}"));
        }
        let consistent = g.query("a", "p")[0].confidence();
        let noisy = g.query("c", "p")[0].confidence();
        assert!(consistent > 0.8, "consistent fact: {consistent}");
        assert!(
            noisy < consistent,
            "noisy {noisy} < consistent {consistent}"
        );
        // Single observation is weak evidence.
        g.observe("e", "p", "f", 100.0);
        assert!(g.query("e", "p")[0].confidence() < 0.15);
    }

    #[test]
    fn loads_legacy_v1_lines() {
        let path = std::env::temp_dir().join(format!("kg_v1_{}.txt", std::process::id()));
        std::fs::write(&path, "a|precedes-well|b|4.5|3\n").unwrap();
        let g = KnowledgeGraph::load(&path).unwrap();
        assert_eq!(g.len(), 1);
        let t = &g.triples()[0];
        assert_eq!((t.weight, t.support), (4.5, 3));
        assert!(t.condition.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_load_roundtrip() {
        let path = std::env::temp_dir().join(format!("kg_{}.txt", std::process::id()));
        let mut g = KnowledgeGraph::new();
        g.observe("a", "precedes-well", "b", 3.0);
        g.observe("a", "precedes-well", "b", 5.0);
        g.save(&path).unwrap();
        let l = KnowledgeGraph::load(&path).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l.query("a", "precedes-well")[0].support, 2);
        // continued observation merges correctly after load
        let mut l = l;
        l.observe("a", "precedes-well", "b", 4.0);
        assert_eq!(l.query("a", "precedes-well")[0].support, 3);
        let _ = std::fs::remove_file(&path);
    }
}
