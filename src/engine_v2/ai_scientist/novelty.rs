//! Novelty search (Stage 5+). Without it, evolution collapses onto tiny
//! variations of one winner — `greedy, greedy, greedy` a hundred thousand times.
//! We keep an ARCHIVE of the operator architectures already explored and measure
//! how DIFFERENT a candidate is from everything seen. The lab rewards novelty in
//! its fitness and drops near-duplicate candidates, keeping the search spread
//! across genuinely different algorithm designs.

use super::super::evolution::Schedule;
use std::collections::HashSet;

/// Structural distance between two operator sequences, in [0, 1]. Combines the
/// set of operators used (Jaccard) with the adjacent-pair structure (bigram
/// Jaccard), so both COMPOSITION and ORDER matter. 0 = identical architecture,
/// 1 = no operators or transitions in common.
pub fn schedule_distance(a: &[String], b: &[String]) -> f64 {
    let set_a: HashSet<&String> = a.iter().collect();
    let set_b: HashSet<&String> = b.iter().collect();
    let set_sim = jaccard(&set_a, &set_b);

    let big_a: HashSet<(&String, &String)> = a.windows(2).map(|w| (&w[0], &w[1])).collect();
    let big_b: HashSet<(&String, &String)> = b.windows(2).map(|w| (&w[0], &w[1])).collect();
    let bigram_sim = if big_a.is_empty() && big_b.is_empty() {
        set_sim // length-1 sequences: fall back to the set similarity
    } else {
        jaccard(&big_a, &big_b)
    };

    1.0 - 0.5 * (set_sim + bigram_sim)
}

fn jaccard<T: std::hash::Hash + Eq>(a: &HashSet<T>, b: &HashSet<T>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

/// Archive of explored operator architectures for novelty scoring.
#[derive(Debug, Clone, Default)]
pub struct NoveltyArchive {
    seen: Vec<Vec<String>>,
}

impl NoveltyArchive {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    pub fn add(&mut self, sched: &Schedule) {
        self.seen.push(sched.ops.clone());
    }

    /// Novelty of `sched` = its minimum distance to anything in the archive
    /// (1.0 if the archive is empty). High ⇒ a genuinely new architecture.
    pub fn novelty(&self, sched: &Schedule) -> f64 {
        if self.seen.is_empty() {
            return 1.0;
        }
        self.seen
            .iter()
            .map(|s| schedule_distance(&sched.ops, s))
            .fold(f64::INFINITY, f64::min)
            .clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sched(ops: &[&str]) -> Schedule {
        Schedule {
            ops: ops.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; ops.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
        }
    }

    #[test]
    fn identical_sequences_have_zero_distance() {
        let s = vec!["gibbs".to_string(), "greedy".to_string()];
        assert!(schedule_distance(&s, &s) < 1e-9);
    }

    #[test]
    fn different_architectures_are_distant() {
        let a = vec!["gibbs".to_string(), "greedy".to_string()];
        let b = vec!["metropolis".to_string(), "houdayer".to_string()];
        assert!(schedule_distance(&a, &b) > 0.9);
    }

    #[test]
    fn archive_penalizes_repeats() {
        let mut arc = NoveltyArchive::new();
        assert_eq!(arc.novelty(&sched(&["gibbs", "greedy"])), 1.0); // empty archive
        arc.add(&sched(&["gibbs", "greedy"]));
        // A near-duplicate scores low novelty...
        assert!(arc.novelty(&sched(&["gibbs", "greedy"])) < 1e-9);
        // ...a genuinely different architecture scores high.
        assert!(arc.novelty(&sched(&["metropolis", "houdayer", "icm"])) > 0.9);
    }
}
