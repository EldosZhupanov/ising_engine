//! Model Registry (Stage 7) — versioned snapshots of the four learned models
//! (Predictor / Policy / Dynamics / World) with lineage, so the platform can
//! answer "which weights were in force when, trained on what, descended from
//! which prior version?". This is the model-side analogue of the append-only
//! [`ExperimentDb`](super::db): history is never overwritten.
//!
//! Each snapshot carries a per-kind monotonic `version`, its `parent` version
//! (the previous version of the same kind — a lineage chain), the
//! campaign/generation/instance provenance that produced it, the size of the
//! corpus it was `trained_on`, a wall-clock `timestamp`, and an opaque
//! `payload` = the model's own `to_weights_text()` serialization. A snapshot's
//! weights are therefore fully reconstructable (`Model::from_weights_text`).
//!
//! Persistence copies the `ExperimentDb` idiom exactly: dependency-free,
//! pipe-delimited, one snapshot per line, append-only via a `persisted` cursor,
//! missing-file → empty. The `payload` uses only `,` and `;` internally, never
//! `|` or `\n`, so the pipe split stays unambiguous.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Which learned model a snapshot belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Predictor,
    Policy,
    Dynamics,
    World,
}

impl ModelKind {
    pub const ALL: [ModelKind; 4] = [
        ModelKind::Predictor,
        ModelKind::Policy,
        ModelKind::Dynamics,
        ModelKind::World,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            ModelKind::Predictor => "predictor",
            ModelKind::Policy => "policy",
            ModelKind::Dynamics => "dynamics",
            ModelKind::World => "world",
        }
    }

    fn from_tag(s: &str) -> Option<Self> {
        match s {
            "predictor" => Some(ModelKind::Predictor),
            "policy" => Some(ModelKind::Policy),
            "dynamics" => Some(ModelKind::Dynamics),
            "world" => Some(ModelKind::World),
            _ => None,
        }
    }

    fn index(self) -> usize {
        match self {
            ModelKind::Predictor => 0,
            ModelKind::Policy => 1,
            ModelKind::Dynamics => 2,
            ModelKind::World => 3,
        }
    }
}

/// One versioned model snapshot with lineage + provenance.
#[derive(Debug, Clone)]
pub struct ModelSnapshot {
    pub kind: ModelKind,
    pub version: u64,
    /// Previous version of the same kind (`None` for the first).
    pub parent: Option<u64>,
    pub campaign_id: u64,
    pub generation_id: u64,
    pub instance_id: String,
    /// Size of the corpus this version was trained on (records / transitions).
    pub trained_on: usize,
    /// Unix seconds; informational only — never fed back into any model input.
    pub timestamp: u64,
    /// The model's own `to_weights_text()` serialization (no `|` or `\n`).
    pub payload: String,
}

/// Append-only registry of model snapshots, one persistent file.
#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    snaps: Vec<ModelSnapshot>,
    /// Next version number per kind (indexed by `ModelKind::index`).
    next_version: [u64; 4],
    /// How many leading snapshots are already on disk (append-mode cursor).
    persisted: usize,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.snaps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snaps.is_empty()
    }

    pub fn snapshots(&self) -> &[ModelSnapshot] {
        &self.snaps
    }

    /// Number of versions recorded for a kind.
    pub fn versions(&self, kind: ModelKind) -> usize {
        self.snaps.iter().filter(|s| s.kind == kind).count()
    }

    /// The most recent snapshot of a kind (highest version), if any.
    pub fn latest(&self, kind: ModelKind) -> Option<&ModelSnapshot> {
        self.snaps
            .iter()
            .filter(|s| s.kind == kind)
            .max_by_key(|s| s.version)
    }

    /// Version numbers recorded for a kind, oldest → newest.
    pub fn lineage(&self, kind: ModelKind) -> Vec<u64> {
        let mut v: Vec<u64> = self
            .snaps
            .iter()
            .filter(|s| s.kind == kind)
            .map(|s| s.version)
            .collect();
        v.sort_unstable();
        v
    }

    /// Record a new versioned snapshot. Assigns the next per-kind version, sets
    /// `parent` to the kind's previous version, and stamps the wall clock.
    /// Returns the assigned version. Never overwrites.
    pub fn record(
        &mut self,
        kind: ModelKind,
        payload: String,
        trained_on: usize,
        campaign_id: u64,
        generation_id: u64,
        instance_id: &str,
    ) -> u64 {
        let version = self.next_version[kind.index()];
        self.next_version[kind.index()] += 1;
        let parent = version.checked_sub(1);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.snaps.push(ModelSnapshot {
            kind,
            version,
            parent,
            campaign_id,
            generation_id,
            instance_id: instance_id.replace('|', "/"),
            trained_on,
            timestamp,
            payload,
        });
        version
    }

    fn line(s: &ModelSnapshot) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            s.kind.tag(),
            s.version,
            s.parent.map(|p| p as i64).unwrap_or(-1),
            s.campaign_id,
            s.generation_id,
            s.instance_id,
            s.trained_on,
            s.timestamp,
            s.payload,
        )
    }

    pub fn save(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut out = String::new();
        for s in &self.snaps {
            out.push_str(&Self::line(s));
        }
        fs::write(path, out)?;
        self.persisted = self.snaps.len();
        Ok(())
    }

    /// Append ONLY snapshots not yet on disk (history is never rewritten).
    pub fn flush_append(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        if self.persisted >= self.snaps.len() {
            return Ok(());
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let mut out = String::new();
        for s in &self.snaps[self.persisted..] {
            out.push_str(&Self::line(s));
        }
        file.write_all(out.as_bytes())?;
        self.persisted = self.snaps.len();
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => return Err(e),
        };
        let mut reg = Self::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // splitn keeps the payload (possibly containing ',' and ';') whole.
            let f: Vec<&str> = line.splitn(9, '|').collect();
            if f.len() != 9 {
                continue;
            }
            let Some(kind) = ModelKind::from_tag(f[0]) else {
                continue;
            };
            let version: u64 = f[1].parse().unwrap_or(0);
            let parent = match f[2].parse::<i64>().unwrap_or(-1) {
                x if x < 0 => None,
                x => Some(x as u64),
            };
            reg.snaps.push(ModelSnapshot {
                kind,
                version,
                parent,
                campaign_id: f[3].parse().unwrap_or(0),
                generation_id: f[4].parse().unwrap_or(0),
                instance_id: f[5].to_string(),
                trained_on: f[6].parse().unwrap_or(0),
                timestamp: f[7].parse().unwrap_or(0),
                payload: f[8].to_string(),
            });
            let i = kind.index();
            reg.next_version[i] = reg.next_version[i].max(version + 1);
        }
        reg.persisted = reg.snaps.len();
        Ok(reg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_lineage_and_parent_chain() {
        let mut reg = ModelRegistry::new();
        let v0 = reg.record(ModelKind::Predictor, "a".into(), 30, 1, 0, "G11");
        let v1 = reg.record(ModelKind::Predictor, "b".into(), 45, 1, 1, "G11");
        let w0 = reg.record(ModelKind::World, "w".into(), 200, 1, 0, "G11");
        assert_eq!((v0, v1, w0), (0, 1, 0));
        assert_eq!(reg.versions(ModelKind::Predictor), 2);
        assert_eq!(reg.versions(ModelKind::World), 1);
        assert_eq!(reg.versions(ModelKind::Policy), 0);
        assert_eq!(reg.lineage(ModelKind::Predictor), vec![0, 1]);
        // The lineage chain: v1's parent is v0; v0 has no parent.
        let latest = reg.latest(ModelKind::Predictor).unwrap();
        assert_eq!(latest.version, 1);
        assert_eq!(latest.parent, Some(0));
        assert_eq!(reg.snapshots()[0].parent, None);
    }

    #[test]
    fn append_only_persistence_round_trips() {
        let dir = std::env::temp_dir().join(format!("mreg_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("model_registry.txt");
        let _ = fs::remove_file(&path);

        let mut reg = ModelRegistry::new();
        // Payload carries commas + semicolons (the model serialization format).
        reg.record(
            ModelKind::Predictor,
            "op_a,op_b;1.5,2.5;30".into(),
            30,
            1,
            0,
            "G11",
        );
        reg.save(&path).unwrap();
        // A second version appended after the first was already persisted.
        reg.record(
            ModelKind::Predictor,
            "op_a,op_b;1.6,2.4;45".into(),
            45,
            1,
            1,
            "G11",
        );
        reg.flush_append(&path).unwrap();

        let reloaded = ModelRegistry::load(&path).unwrap();
        assert_eq!(reloaded.len(), 2);
        assert_eq!(reloaded.versions(ModelKind::Predictor), 2);
        let latest = reloaded.latest(ModelKind::Predictor).unwrap();
        assert_eq!(latest.version, 1);
        assert_eq!(latest.parent, Some(0));
        assert_eq!(latest.payload, "op_a,op_b;1.6,2.4;45");
        assert_eq!(latest.trained_on, 45);
        // A further record continues the version counter, not restarts it.
        let mut reloaded = reloaded;
        let v2 = reloaded.record(ModelKind::Predictor, "x;y;1".into(), 50, 2, 0, "G11");
        assert_eq!(v2, 2);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_loads_empty() {
        let reg = ModelRegistry::load("/nonexistent/model_registry_xyz.txt").unwrap();
        assert!(reg.is_empty());
    }
}
