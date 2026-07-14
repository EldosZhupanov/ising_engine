//! Knowledge system (Constitution §12) — the engine's long-term memory and the
//! substrate for meta-learning. Every solved instance leaves an `Experience`:
//! its structural FEATURES, the operator sequence that worked, and the utility
//! it achieved. Later instances query this base:
//!
//!  - the Evolution Engine SEEDS its search from sequences that worked on
//!    similar instances (knowledge transfer between tasks);
//!  - the Decision Engine RANKS operators by their learned utility on similar
//!    instances (learned, not rule-based).
//!
//! Persistence is a dependency-free line format (no serde), so the base is a
//! plain file the four-track program can inspect and version.

use super::decision::InstanceStats;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Structural fingerprint used for instance similarity. A small, scale-aware
/// vector; extend as more signals prove predictive (Constitution §10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceFeatures {
    pub n: usize,
    pub density: f64,
    pub mean_degree: f64,
    pub degree_cv: f64,
    pub integral: bool,
}

impl InstanceFeatures {
    pub fn from_stats(s: &InstanceStats) -> Self {
        Self {
            n: s.n,
            density: s.density,
            mean_degree: s.mean_degree,
            degree_cv: s.degree_cv,
            integral: s.integral,
        }
    }

    /// Scale-normalized coordinates for distance (each roughly in [0, 1]).
    fn coords(&self) -> [f64; 4] {
        [
            (1.0 + self.n as f64).ln() / 12.0,
            self.density,
            self.degree_cv / 2.0,
            if self.integral { 1.0 } else { 0.0 },
        ]
    }

    /// Euclidean distance in normalized feature space.
    pub fn distance(&self, other: &Self) -> f64 {
        self.coords()
            .iter()
            .zip(other.coords().iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
    }
}

/// One recorded outcome: what worked, on what kind of instance, how well.
#[derive(Debug, Clone)]
pub struct Experience {
    pub features: InstanceFeatures,
    pub backend: String,
    pub sequence: Vec<String>,
    pub sweeps: Vec<u32>,
    pub temp_hi: f64,
    pub temp_lo: f64,
    pub best_energy: f64,
    pub best_utility: f64,
}

/// The learned memory. In-memory `Vec` with a plain-text file backing.
#[derive(Debug, Clone, Default)]
pub struct KnowledgeBase {
    experiences: Vec<Experience>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.experiences.len()
    }

    pub fn is_empty(&self) -> bool {
        self.experiences.is_empty()
    }

    pub fn record(&mut self, exp: Experience) {
        self.experiences.push(exp);
    }

    pub fn all(&self) -> &[Experience] {
        &self.experiences
    }

    /// The `k` experiences most similar to `feats`, nearest first.
    pub fn similar(&self, feats: &InstanceFeatures, k: usize) -> Vec<&Experience> {
        let mut scored: Vec<(f64, &Experience)> = self
            .experiences
            .iter()
            .map(|e| (feats.distance(&e.features), e))
            .collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        scored.into_iter().take(k).map(|(_, e)| e).collect()
    }

    /// Learned per-operator score for an instance like `feats` (HIGHER is
    /// better). Each experience votes for the operators it used, weighted by
    /// 1/(1+distance) and by how good its utility was (lower utility ⇒ stronger
    /// vote). This is the model the learnable Decision Engine consults.
    pub fn operator_scores(&self, feats: &InstanceFeatures) -> HashMap<String, f64> {
        let mut acc: HashMap<String, (f64, f64)> = HashMap::new(); // (weighted sum, weight)
        for e in &self.experiences {
            let dist = feats.distance(&e.features);
            let proximity = 1.0 / (1.0 + dist);
            // Turn utility into a reward: better (more negative) utility ⇒ higher.
            let reward = -e.best_utility * proximity;
            for op in &e.sequence {
                let entry = acc.entry(op.clone()).or_insert((0.0, 0.0));
                entry.0 += reward;
                entry.1 += proximity;
            }
        }
        acc.into_iter()
            .map(|(k, (sum, w))| (k, if w > 0.0 { sum / w } else { 0.0 }))
            .collect()
    }

    // ------------------------------------------------------------ persistence
    // Line format (one experience):
    //   n|density|mean_degree|degree_cv|integral|backend|energy|utility|
    //   op1,op2|sw1,sw2|t_hi|t_lo

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut out = String::new();
        for e in &self.experiences {
            out.push_str(&format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                e.features.n,
                e.features.density,
                e.features.mean_degree,
                e.features.degree_cv,
                e.features.integral as u8,
                e.backend,
                e.best_energy,
                e.best_utility,
                e.sequence.join(","),
                e.sweeps
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                e.temp_hi,
                e.temp_lo,
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
        let mut kb = Self::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('|').collect();
            if f.len() != 12 {
                continue; // skip malformed rows rather than fail the whole base
            }
            let parse_ops = |s: &str| -> Vec<String> {
                if s.is_empty() {
                    Vec::new()
                } else {
                    s.split(',').map(|x| x.to_string()).collect()
                }
            };
            let parse_sw = |s: &str| -> Vec<u32> {
                if s.is_empty() {
                    Vec::new()
                } else {
                    s.split(',').filter_map(|x| x.parse().ok()).collect()
                }
            };
            let exp = Experience {
                features: InstanceFeatures {
                    n: f[0].parse().unwrap_or(0),
                    density: f[1].parse().unwrap_or(0.0),
                    mean_degree: f[2].parse().unwrap_or(0.0),
                    degree_cv: f[3].parse().unwrap_or(0.0),
                    integral: f[4] == "1",
                },
                backend: f[5].to_string(),
                best_energy: f[6].parse().unwrap_or(f64::INFINITY),
                best_utility: f[7].parse().unwrap_or(f64::INFINITY),
                sequence: parse_ops(f[8]),
                sweeps: parse_sw(f[9]),
                temp_hi: f[10].parse().unwrap_or(1.0),
                temp_lo: f[11].parse().unwrap_or(0.1),
            };
            kb.record(exp);
        }
        Ok(kb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feats(n: usize, density: f64) -> InstanceFeatures {
        InstanceFeatures {
            n,
            density,
            mean_degree: 3.0,
            degree_cv: 0.2,
            integral: true,
        }
    }

    fn exp(n: usize, density: f64, seq: &[&str], utility: f64) -> Experience {
        Experience {
            features: feats(n, density),
            backend: "SparseBitSlice".into(),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
            best_energy: utility,
            best_utility: utility,
        }
    }

    #[test]
    fn similar_returns_nearest_first() {
        let mut kb = KnowledgeBase::new();
        kb.record(exp(800, 0.01, &["gibbs_color_sweep"], -100.0));
        kb.record(exp(50, 0.9, &["metropolis_sweep"], -20.0));
        let near = kb.similar(&feats(820, 0.012), 1);
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].features.n, 800);
    }

    #[test]
    fn operator_scores_prefer_winners_on_similar_instances() {
        let mut kb = KnowledgeBase::new();
        // On big sparse instances, greedy+gibbs did very well (low utility).
        kb.record(exp(
            800,
            0.01,
            &["gibbs_color_sweep", "greedy_descent"],
            -500.0,
        ));
        // A dense small instance where random did poorly (high utility).
        kb.record(exp(30, 0.8, &["random_flip_sweep"], 5.0));
        let scores = kb.operator_scores(&feats(790, 0.011));
        let g = scores["gibbs_color_sweep"];
        let r = scores["random_flip_sweep"];
        assert!(
            g > r,
            "operator that won on similar instances must score higher"
        );
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kb_test_{}.txt", std::process::id()));
        let mut kb = KnowledgeBase::new();
        kb.record(exp(
            800,
            0.01,
            &["gibbs_color_sweep", "greedy_descent"],
            -500.0,
        ));
        kb.save(&path).unwrap();
        let loaded = KnowledgeBase::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded.all()[0].sequence,
            vec!["gibbs_color_sweep", "greedy_descent"]
        );
        assert_eq!(loaded.all()[0].features.n, 800);
        let _ = std::fs::remove_file(&path);
        // Loading a missing file yields an empty base, not an error.
        assert!(KnowledgeBase::load(dir.join("kb_does_not_exist_zzz.txt"))
            .unwrap()
            .is_empty());
    }
}
