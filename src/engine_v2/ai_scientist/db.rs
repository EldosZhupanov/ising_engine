//! Experiment database (Stage 5) — the append-only, reproducible record of every
//! experiment the AI Scientist ever ran. NO row is ever overwritten; each gets a
//! fresh monotonic id. Every field needed to replay an experiment bit-for-bit is
//! stored: id, operator sequence, parameters, seed, backend, cost, score, and
//! the baseline it is measured against (ADR-0004, Constitution §12).

use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

/// One immutable experiment result. Fully specifies a reproducible run.
#[derive(Debug, Clone, Default)]
pub struct ExperimentRecord {
    pub id: u64,
    /// The hypothesis that motivated this experiment, if any.
    pub hypothesis_id: Option<u64>,
    pub sequence: Vec<String>,
    pub sweeps: Vec<u32>,
    pub temp_hi: f64,
    pub temp_lo: f64,
    pub num_replicas: usize,
    pub seed: u64,
    pub backend: String,
    /// Computational cost (spin-update operation count) — deterministic.
    pub work: f64,
    /// Canonical score (best energy; lower is better).
    pub score: f64,
    /// Baseline energy this experiment is compared against.
    pub baseline: f64,
    /// Instance CONTEXT — recorded so the meta-learner can mine rules
    /// conditioned on graph structure ACROSS campaigns ("X wins on sparse").
    pub density: f64,
    pub clustering: f64,
    // ---- Stage 6 provenance (stamped by `record()` from the RunContext) -----
    /// Which instance the experiment ran on (name; '|' is sanitized to '/').
    pub instance_id: String,
    pub n: usize,
    pub mean_degree: f64,
    pub degree_cv: f64,
    /// Approximate wall-clock time of this run in milliseconds (informational;
    /// `work` is the deterministic cost measure).
    pub wall_ms: f64,
    pub campaign_id: u64,
    pub generation_id: u64,
    /// Unix epoch seconds when the record was appended.
    pub timestamp: u64,
}

impl ExperimentRecord {
    /// Improvement over baseline (positive ⇒ better, i.e. lower energy).
    pub fn improvement(&self) -> f64 {
        self.baseline - self.score
    }

    /// Relative improvement (fraction of |baseline|), size-comparable.
    pub fn rel_improvement(&self) -> f64 {
        self.improvement() / self.baseline.abs().max(1.0)
    }
}

/// Provenance stamped onto every record appended while it is set: which
/// instance/campaign/generation produced the result, plus instance features.
#[derive(Debug, Clone, Default)]
pub struct RunContext {
    pub instance_id: String,
    pub n: usize,
    pub mean_degree: f64,
    pub degree_cv: f64,
    pub campaign_id: u64,
    pub generation_id: u64,
}

/// Append-only store with a monotonic id counter and plain-text persistence.
#[derive(Debug, Clone, Default)]
pub struct ExperimentDb {
    records: Vec<ExperimentRecord>,
    next_id: u64,
    context: RunContext,
    /// How many leading records are already persisted (for append-mode flush).
    persisted: usize,
}

impl ExperimentDb {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a db VIEW from existing records verbatim (ids, provenance, and
    /// timestamps untouched). For analysis subsets (per-instance mining,
    /// leave-one-instance-out predictor training) — not a persistence path.
    pub fn from_records(records: Vec<ExperimentRecord>) -> Self {
        let next_id = records.iter().map(|r| r.id + 1).max().unwrap_or(0);
        Self {
            records,
            next_id,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn all(&self) -> &[ExperimentRecord] {
        &self.records
    }

    /// Set the provenance stamped onto every subsequently appended record.
    pub fn set_context(&mut self, ctx: RunContext) {
        self.context = ctx;
    }

    pub fn context(&self) -> &RunContext {
        &self.context
    }

    /// Append a result. Assigns and returns a fresh id; never overwrites.
    /// `rec.id` on input is ignored and replaced with the next id. The current
    /// [`RunContext`] and the wall clock stamp provenance onto the record.
    pub fn record(&mut self, mut rec: ExperimentRecord) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        rec.id = id;
        rec.instance_id = self.context.instance_id.replace('|', "/");
        rec.n = self.context.n;
        rec.mean_degree = self.context.mean_degree;
        rec.degree_cv = self.context.degree_cv;
        rec.campaign_id = self.context.campaign_id;
        rec.generation_id = self.context.generation_id;
        rec.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.records.push(rec);
        id
    }

    /// The record with the lowest score (best), if any.
    pub fn best(&self) -> Option<&ExperimentRecord> {
        self.records
            .iter()
            .min_by(|a, b| a.score.total_cmp(&b.score))
    }

    /// All scores recorded for an exact operator sequence (for statistics).
    pub fn scores_for(&self, sequence: &[String]) -> Vec<f64> {
        self.records
            .iter()
            .filter(|r| r.sequence == sequence)
            .map(|r| r.score)
            .collect()
    }

    // ---- persistence: one record per line, pipe-delimited (no serde) --------
    // v1 (14 fields): id|hyp|seq|sweeps|t_hi|t_lo|replicas|seed|backend|work|
    //                 score|baseline|density|clustering
    // v2 (22 fields): v1 + instance|n|mean_degree|degree_cv|wall_ms|campaign|
    //                 generation|timestamp
    // `load` accepts both, so old databases keep their history.

    fn line(r: &ExperimentRecord) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            r.id,
            r.hypothesis_id.map(|h| h as i64).unwrap_or(-1),
            r.sequence.join(","),
            r.sweeps
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(","),
            r.temp_hi,
            r.temp_lo,
            r.num_replicas,
            r.seed,
            r.backend,
            r.work,
            r.score,
            r.baseline,
            r.density,
            r.clustering,
            r.instance_id,
            r.n,
            r.mean_degree,
            r.degree_cv,
            r.wall_ms,
            r.campaign_id,
            r.generation_id,
            r.timestamp,
        )
    }

    pub fn save(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut out = String::new();
        for r in &self.records {
            out.push_str(&Self::line(r));
        }
        fs::write(path, out)?;
        self.persisted = self.records.len();
        Ok(())
    }

    /// History is never deleted: append ONLY records not yet on disk. Cheap
    /// enough to call after every batch of a million-experiment campaign.
    pub fn flush_append(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        if self.persisted >= self.records.len() {
            return Ok(());
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let mut out = String::new();
        for r in &self.records[self.persisted..] {
            out.push_str(&Self::line(r));
        }
        file.write_all(out.as_bytes())?;
        self.persisted = self.records.len();
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => return Err(e),
        };
        let mut db = Self::new();
        let mut max_id: i64 = -1;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('|').collect();
            if f.len() != 14 && f.len() != 22 {
                continue;
            }
            let seq = if f[2].is_empty() {
                Vec::new()
            } else {
                f[2].split(',').map(|x| x.to_string()).collect()
            };
            let sweeps = if f[3].is_empty() {
                Vec::new()
            } else {
                f[3].split(',').filter_map(|x| x.parse().ok()).collect()
            };
            let id: u64 = f[0].parse().unwrap_or(0);
            max_id = max_id.max(id as i64);
            let hyp = match f[1].parse::<i64>().unwrap_or(-1) {
                x if x < 0 => None,
                x => Some(x as u64),
            };
            let mut rec = ExperimentRecord {
                id,
                hypothesis_id: hyp,
                sequence: seq,
                sweeps,
                temp_hi: f[4].parse().unwrap_or(1.0),
                temp_lo: f[5].parse().unwrap_or(0.1),
                num_replicas: f[6].parse().unwrap_or(1),
                seed: f[7].parse().unwrap_or(0),
                backend: f[8].to_string(),
                work: f[9].parse().unwrap_or(0.0),
                score: f[10].parse().unwrap_or(f64::INFINITY),
                baseline: f[11].parse().unwrap_or(f64::INFINITY),
                density: f[12].parse().unwrap_or(0.0),
                clustering: f[13].parse().unwrap_or(0.0),
                ..Default::default()
            };
            if f.len() == 22 {
                rec.instance_id = f[14].to_string();
                rec.n = f[15].parse().unwrap_or(0);
                rec.mean_degree = f[16].parse().unwrap_or(0.0);
                rec.degree_cv = f[17].parse().unwrap_or(0.0);
                rec.wall_ms = f[18].parse().unwrap_or(0.0);
                rec.campaign_id = f[19].parse().unwrap_or(0);
                rec.generation_id = f[20].parse().unwrap_or(0);
                rec.timestamp = f[21].parse().unwrap_or(0);
            }
            db.records.push(rec);
        }
        db.next_id = (max_id + 1) as u64;
        db.persisted = db.records.len();
        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(seq: &[&str], score: f64) -> ExperimentRecord {
        ExperimentRecord {
            id: 0,
            hypothesis_id: Some(1),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            temp_hi: 4.0,
            temp_lo: 0.1,
            num_replicas: 8,
            seed: 42,
            backend: "SparseBitSlice".into(),
            work: 1000.0,
            score,
            baseline: 0.0,
            density: 0.01,
            clustering: 0.3,
            ..Default::default()
        }
    }

    #[test]
    fn append_only_assigns_ids_and_never_overwrites() {
        let mut db = ExperimentDb::new();
        let a = db.record(rec(&["gibbs_color_sweep"], -10.0));
        let b = db.record(rec(&["gibbs_color_sweep"], -12.0));
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(db.len(), 2);
        assert_eq!(db.best().unwrap().score, -12.0);
        assert_eq!(db.scores_for(&["gibbs_color_sweep".to_string()]).len(), 2);
    }

    #[test]
    fn context_is_stamped_and_survives_roundtrip_with_append() {
        let path = std::env::temp_dir().join(format!("expdb_ctx_{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut db = ExperimentDb::new();
        db.set_context(RunContext {
            instance_id: "G11|test".into(), // '|' must be sanitized
            n: 800,
            mean_degree: 4.0,
            degree_cv: 0.1,
            campaign_id: 3,
            generation_id: 7,
        });
        db.record(rec(&["a"], -1.0));
        db.save(&path).unwrap();
        // Append-only flush writes just the new tail.
        db.set_context(RunContext {
            generation_id: 8,
            ..db.context().clone()
        });
        db.record(rec(&["b"], -2.0));
        db.flush_append(&path).unwrap();
        let loaded = ExperimentDb::load(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        let r0 = &loaded.all()[0];
        assert_eq!(r0.instance_id, "G11/test");
        assert_eq!((r0.n, r0.campaign_id, r0.generation_id), (800, 3, 7));
        assert!(r0.timestamp > 0);
        assert_eq!(loaded.all()[1].generation_id, 8);
        // Idempotent: nothing new ⇒ nothing appended.
        let mut loaded = loaded;
        loaded.flush_append(&path).unwrap();
        assert_eq!(ExperimentDb::load(&path).unwrap().len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn loads_legacy_v1_lines() {
        let path = std::env::temp_dir().join(format!("expdb_v1_{}.txt", std::process::id()));
        std::fs::write(
            &path,
            "5|-1|a,b|10,10|4|0.1|8|42|SparseBitSlice|1000|-9|0|0.01|0.3\n",
        )
        .unwrap();
        let loaded = ExperimentDb::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        let r = &loaded.all()[0];
        assert_eq!(r.id, 5);
        assert_eq!(r.sequence, vec!["a", "b"]);
        assert_eq!(r.score, -9.0);
        assert_eq!(r.instance_id, ""); // v1 has no provenance — defaults
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_load_preserves_ids_and_continues_counter() {
        let path = std::env::temp_dir().join(format!("expdb_{}.txt", std::process::id()));
        let mut db = ExperimentDb::new();
        db.record(rec(&["a", "b"], -5.0));
        db.record(rec(&["c"], -7.0));
        db.save(&path).unwrap();
        let mut loaded = ExperimentDb::load(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        // New id continues after the max loaded id (no collision / overwrite).
        let id = loaded.record(rec(&["d"], -1.0));
        assert_eq!(id, 2);
        let _ = std::fs::remove_file(&path);
    }
}
