//! Monitor (Stage 7) — continuous, cheap health gates over the platform state.
//! The autonomous loop can run for a very long time; the Monitor is what lets it
//! notice when something has gone wrong (a corrupted append, a model that has
//! stopped tracking reality, knowledge that has stopped being extracted) and
//! PAUSE instead of compounding the fault.
//!
//! Every check is O(experiments) at worst and reads only the in-memory stores,
//! so it is safe to call each tick. The gate is simple: [`HealthReport::healthy`]
//! is false iff any check is [`HealthStatus::Fail`] — the caller then stops.
//! Warnings are surfaced but never halt the loop (honesty over alarmism).

use super::db::ExperimentDb;
use super::evaluation::evaluate_predictor;
use super::graph::KnowledgeGraph;
use super::model_registry::{ModelKind, ModelRegistry};

/// Severity of a single check. Ordered so `Fail` is the maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthStatus {
    Ok,
    Warn,
    Fail,
}

impl HealthStatus {
    pub fn label(self) -> &'static str {
        match self {
            HealthStatus::Ok => "OK",
            HealthStatus::Warn => "WARN",
            HealthStatus::Fail => "FAIL",
        }
    }
}

/// One named gate result with a measured detail line.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub detail: String,
}

/// The full result of a monitor pass.
#[derive(Debug, Clone, Default)]
pub struct HealthReport {
    pub checks: Vec<HealthCheck>,
}

impl HealthReport {
    /// The gate: healthy iff nothing FAILed. Warnings do not stop the loop.
    pub fn healthy(&self) -> bool {
        !self.checks.iter().any(|c| c.status == HealthStatus::Fail)
    }

    /// The most severe status observed (Ok if there were no checks).
    pub fn worst(&self) -> HealthStatus {
        self.checks
            .iter()
            .map(|c| c.status)
            .max()
            .unwrap_or(HealthStatus::Ok)
    }

    /// One-line human summary, e.g. `HEALTH FAIL: 1 fail, 2 warn (5 checks)`.
    pub fn summary(&self) -> String {
        let fails = self
            .checks
            .iter()
            .filter(|c| c.status == HealthStatus::Fail)
            .count();
        let warns = self
            .checks
            .iter()
            .filter(|c| c.status == HealthStatus::Warn)
            .count();
        format!(
            "HEALTH {}: {fails} fail, {warns} warn ({} checks)",
            self.worst().label(),
            self.checks.len()
        )
    }

    fn push(&mut self, name: &str, status: HealthStatus, detail: String) {
        self.checks.push(HealthCheck {
            name: name.to_string(),
            status,
            detail,
        });
    }
}

/// Thresholds for the health gates.
#[derive(Debug, Clone, Copy)]
pub struct MonitorConfig {
    /// Below this leave-one-instance-out mean Spearman, the predictor is warned
    /// as untrustworthy (it is filtering on noise, not signal).
    pub min_predictor_spearman: f64,
    /// Below this fraction of records carrying a non-empty `instance_id`, the
    /// provenance gate warns (results that cannot be attributed).
    pub min_provenance_frac: f64,
    /// At/above this many experiments, an EMPTY knowledge graph is a warning
    /// (data is accumulating but nothing is being learned from it).
    pub extraction_floor: usize,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            min_predictor_spearman: 0.0,
            min_provenance_frac: 0.99,
            extraction_floor: 50,
        }
    }
}

/// Runs the health gates over the persistent stores.
#[derive(Debug, Clone, Default)]
pub struct Monitor {
    cfg: MonitorConfig,
}

impl Monitor {
    pub fn new(cfg: MonitorConfig) -> Self {
        Self { cfg }
    }

    /// Full health pass over the experiment DB + knowledge graph.
    pub fn check(&self, db: &ExperimentDb, graph: &KnowledgeGraph) -> HealthReport {
        let mut report = HealthReport::default();

        // 1. DB integrity — the append-only invariant made observable: ids must
        // be strictly increasing and unique. A regression here means the store
        // was corrupted or a record was overwritten (forbidden).
        let mut prev: Option<u64> = None;
        let mut violation: Option<u64> = None;
        for r in db.all() {
            if let Some(p) = prev {
                if r.id <= p {
                    violation = Some(r.id);
                    break;
                }
            }
            prev = Some(r.id);
        }
        match violation {
            Some(id) => report.push(
                "db-integrity",
                HealthStatus::Fail,
                format!(
                    "id {id} is not strictly greater than its predecessor — append-only broken"
                ),
            ),
            None => report.push(
                "db-integrity",
                HealthStatus::Ok,
                format!("{} records, ids strictly monotonic", db.len()),
            ),
        }

        // 2. DB non-empty — nothing to monitor if there is no data yet.
        if db.is_empty() {
            report.push(
                "db-nonempty",
                HealthStatus::Warn,
                "no experiments recorded yet".into(),
            );
            return report; // the remaining checks need data
        }

        // 3. Provenance — every result should be attributable to an instance.
        let with_prov = db
            .all()
            .iter()
            .filter(|r| !r.instance_id.is_empty())
            .count();
        let frac = with_prov as f64 / db.len() as f64;
        if frac + 1e-9 < self.cfg.min_provenance_frac {
            report.push(
                "provenance",
                HealthStatus::Warn,
                format!(
                    "{:.1}% of records carry an instance id (< {:.0}% target)",
                    frac * 100.0,
                    self.cfg.min_provenance_frac * 100.0
                ),
            );
        } else {
            report.push(
                "provenance",
                HealthStatus::Ok,
                format!("{:.1}% of records provenance-stamped", frac * 100.0),
            );
        }

        // 4. Predictor health — does the filter still track reality? Leave-one-
        // instance-out Spearman; an honest skip when there is too little data.
        match evaluate_predictor(db, 1e-3) {
            None => report.push(
                "predictor-health",
                HealthStatus::Ok,
                "insufficient data to evaluate the predictor yet (honest skip)".into(),
            ),
            Some(accs) if accs.is_empty() => report.push(
                "predictor-health",
                HealthStatus::Ok,
                "no eligible instances to evaluate the predictor".into(),
            ),
            Some(accs) => {
                let mean = accs.iter().map(|a| a.spearman).sum::<f64>() / accs.len() as f64;
                if mean < self.cfg.min_predictor_spearman {
                    report.push(
                        "predictor-health",
                        HealthStatus::Warn,
                        format!(
                            "mean leave-one-out Spearman {mean:.3} < {:.3} — filter is not tracking reality",
                            self.cfg.min_predictor_spearman
                        ),
                    );
                } else {
                    report.push(
                        "predictor-health",
                        HealthStatus::Ok,
                        format!(
                            "mean leave-one-out Spearman {mean:.3} over {} instances",
                            accs.len()
                        ),
                    );
                }
            }
        }

        // 5. Knowledge extraction — data without learning is a stall.
        if db.len() >= self.cfg.extraction_floor && graph.is_empty() {
            report.push(
                "knowledge-extraction",
                HealthStatus::Warn,
                format!(
                    "{} experiments but the knowledge graph is empty — nothing is being learned",
                    db.len()
                ),
            );
        } else {
            report.push(
                "knowledge-extraction",
                HealthStatus::Ok,
                format!(
                    "{} knowledge facts from {} experiments",
                    graph.len(),
                    db.len()
                ),
            );
        }

        report
    }

    /// Optional model-registry gate: warns for any of the four model kinds that
    /// has never been snapshotted (its lineage is empty) once shared knowledge
    /// should have produced one. Kept separate because the registry is optional
    /// state the base loop does not always populate.
    pub fn check_models(&self, models: &ModelRegistry) -> HealthReport {
        let mut report = HealthReport::default();
        for kind in ModelKind::ALL {
            let n = models.versions(kind);
            let status = if n == 0 {
                HealthStatus::Warn
            } else {
                HealthStatus::Ok
            };
            report.push(
                "model-lineage",
                status,
                format!("{}: {n} version(s) recorded", kind.tag()),
            );
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(id: u64, inst: &str) -> ExperimentRecord {
        ExperimentRecord {
            id,
            instance_id: inst.into(),
            sequence: vec!["metropolis_sweep".into()],
            sweeps: vec![10],
            score: -0.5,
            baseline: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn healthy_on_a_clean_store() {
        let db = ExperimentDb::from_records((0..60).map(|i| rec(i, "G11")).collect());
        let mut graph = KnowledgeGraph::default();
        graph.observe_if("a", "precedes-well", "b", "", 1.0, "test");
        let report = Monitor::new(MonitorConfig::default()).check(&db, &graph);
        assert!(report.healthy(), "{}", report.summary());
        assert_eq!(report.worst(), HealthStatus::Ok);
    }

    #[test]
    fn fails_the_gate_on_a_broken_append_only_invariant() {
        // A duplicate id violates strict monotonicity → corruption / overwrite.
        let db = ExperimentDb::from_records(vec![rec(0, "G11"), rec(5, "G11"), rec(5, "G11")]);
        let graph = KnowledgeGraph::default();
        let report = Monitor::new(MonitorConfig::default()).check(&db, &graph);
        assert!(!report.healthy(), "{}", report.summary());
        assert_eq!(report.worst(), HealthStatus::Fail);
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "db-integrity" && c.status == HealthStatus::Fail));
    }

    #[test]
    fn warns_but_does_not_halt_on_missing_provenance() {
        let db = ExperimentDb::from_records((0..60).map(|i| rec(i, "")).collect());
        let graph = KnowledgeGraph::default();
        let report = Monitor::new(MonitorConfig::default()).check(&db, &graph);
        assert!(report.healthy(), "warnings must not fail the gate");
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "provenance" && c.status == HealthStatus::Warn));
        // Data with no extracted knowledge is a warning, not a failure.
        assert!(report
            .checks
            .iter()
            .any(|c| c.name == "knowledge-extraction" && c.status == HealthStatus::Warn));
    }

    #[test]
    fn empty_db_short_circuits_to_a_warning() {
        let db = ExperimentDb::new();
        let graph = KnowledgeGraph::default();
        let report = Monitor::new(MonitorConfig::default()).check(&db, &graph);
        assert!(report.healthy());
        assert!(report.checks.iter().any(|c| c.name == "db-nonempty"));
    }
}
