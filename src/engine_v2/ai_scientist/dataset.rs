//! Foundation Dataset (Stage 8 — a DATASET, not a model).
//!
//! A Research Foundation Model does not come from architecture; it comes from
//! data — millions of the platform's own studies. That data does not exist yet:
//! there are ~tens of thousands of experiments, and a foundation model needs
//! orders of magnitude more. What CAN exist now, and is the honest precondition,
//! is the *training corpus itself*: a clean, versioned, ML-ready export of every
//! experiment — features in, algorithm, outcome — plus a manifest that states,
//! without spin, exactly how far the data is from foundation scale.
//!
//! This module exports that corpus. It builds no model. The manifest is
//! deliberately blunt about the gap, because the difference between a research
//! platform and a pitch deck is refusing to call a dataset a foundation model.

use super::db::ExperimentDb;
use super::dynamics::Trajectory;
use super::graph::KnowledgeGraph;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

/// Foundation-scale milestones (the honest ladder from today to a real model).
const SCALE_MILESTONES: [usize; 4] = [500_000, 1_000_000, 5_000_000, 20_000_000];

/// One TSV row per experiment. Header + `feature… | algorithm | outcome`.
fn header() -> &'static str {
    "instance\tn\tdensity\tclustering\tmean_degree\tdegree_cv\tbackend\ttemp_hi\ttemp_lo\t\
operators\tsweeps\twork\twall_ms\tscore\tbaseline\trel_improvement\tcampaign\tgeneration\tseed\ttimestamp"
}

/// The training corpus + its honest manifest.
pub struct FoundationDataset;

impl FoundationDataset {
    /// Export every experiment to `dir/foundation_dataset.tsv` (ML-ready, one row
    /// per run) and write `dir/foundation_manifest.md`. Returns the number of
    /// data rows written.
    pub fn export(db: &ExperimentDb, dir: impl AsRef<Path>) -> io::Result<usize> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let mut out = String::from(header());
        out.push('\n');
        for r in db.all() {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                r.instance_id.replace(['\t', '\n'], " "),
                r.n,
                r.density,
                r.clustering,
                r.mean_degree,
                r.degree_cv,
                r.backend,
                r.temp_hi,
                r.temp_lo,
                r.sequence.join(">"),
                r.sweeps
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(">"),
                r.work,
                r.wall_ms,
                r.score,
                r.baseline,
                r.rel_improvement(),
                r.campaign_id,
                r.generation_id,
                r.seed,
                r.timestamp,
            ));
        }
        fs::write(dir.join("foundation_dataset.tsv"), out)?;
        fs::write(dir.join("foundation_manifest.md"), Self::manifest(db))?;
        Ok(db.len())
    }

    /// The honest manifest: how much data there is, how it is distributed, and
    /// exactly how far it is from the scale a foundation model would need.
    pub fn manifest(db: &ExperimentDb) -> String {
        let n = db.len();
        let instances: BTreeSet<&str> = db
            .all()
            .iter()
            .map(|r| r.instance_id.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        let vocab: BTreeSet<&str> = db
            .all()
            .iter()
            .flat_map(|r| r.sequence.iter().map(|s| s.as_str()))
            .collect();
        let with_traj = db.all().iter().filter(|r| r.wall_ms > 0.0).count();

        let mut m = String::from("# Foundation Dataset — Manifest\n\n");
        m.push_str(&format!("- Examples (experiments): **{n}**\n"));
        m.push_str(&format!("- Distinct instances: {}\n", instances.len()));
        m.push_str(&format!("- Operator vocabulary: {}\n", vocab.len()));
        m.push_str(&format!(
            "- Rows with timing/trajectory metadata: {with_traj}\n\n"
        ));
        m.push_str("Each row is `structural features → algorithm (operator sequence + budgets + temperatures) → outcome (score, improvement, work)`. This is the corpus a Research Foundation Model would train on.\n\n");
        m.push_str("## Distance to foundation scale (honest)\n\n");
        m.push_str("A foundation model that predicts algorithm behavior across problem families needs orders of magnitude more data than this. Progress toward the milestones:\n\n");
        m.push_str(
            "| milestone | examples | reached? | multiplier still needed |\n|---|---|---|---|\n",
        );
        for &milestone in &SCALE_MILESTONES {
            let reached = n >= milestone;
            let mult = if reached {
                "—".to_string()
            } else {
                format!("{:.0}×", milestone as f64 / n.max(1) as f64)
            };
            m.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                milestone,
                n,
                if reached { "yes" } else { "no" },
                mult
            ));
        }
        m.push_str(&format!(
            "\n**Verdict:** with {n} examples, this is a training corpus, not a foundation model. \
Building a Transformer / Graph-Transformer / GNN on it now would over-fit; the honest path is to \
GROW this dataset (Curiosity Engine for informative data, Scientific Memory to never lose it) \
until the milestones are met — only then does a large model have something to learn.\n"
        ));
        m
    }

    /// DECISION LOG — the "why" behind the platform's choices. Every fact in the
    /// knowledge graph carries its provenance (which agent/model decided it, on
    /// what evidence) in the `proof` field; this export compiles them into a
    /// decision history: `subject → predicate → object IF condition | reason`.
    /// It is the honest answer to "the reasons for each agent's action
    /// selection", drawn from data already persisted — no re-running.
    pub fn export_decision_log(graph: &KnowledgeGraph, dir: impl AsRef<Path>) -> io::Result<usize> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let mut facts: Vec<_> = graph.triples().iter().collect();
        facts.sort_by(|a, b| b.confidence().total_cmp(&a.confidence()));
        let mut out = String::from(
            "subject\tpredicate\tobject\tcondition\tweight\tsupport\tconfidence\treason\n",
        );
        for t in &facts {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                t.subject,
                t.predicate,
                t.object,
                if t.condition.is_empty() {
                    "-"
                } else {
                    &t.condition
                },
                t.weight,
                t.support,
                t.confidence(),
                t.proof.replace(['\t', '\n'], " "),
            ));
        }
        fs::write(dir.join("decision_log.tsv"), out)?;
        Ok(facts.len())
    }

    /// SEARCH-TRAJECTORY digests — one row per STEP of the captured trajectories,
    /// with the operator that ran and the observable state after it (best/mean
    /// energy, entropy, replica diversity, acceptance, fraction elapsed). This
    /// is the "solution search trajectories" corpus: how the ensemble MOVED, not
    /// just where it ended. Callers capture (via `capture_trajectory`) and pass
    /// `(instance_id, operator-sequence, trajectory)`; nothing is re-run here.
    pub fn export_trajectories(
        dir: impl AsRef<Path>,
        captures: &[(String, Vec<String>, Trajectory)],
    ) -> io::Result<usize> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let mut out = String::from(
            "instance\tstep\toperator\tbest_energy\tmean_energy\tentropy\tdiversity\tacceptance\tfrac_elapsed\n",
        );
        let mut rows = 0usize;
        for (inst, ops, tr) in captures {
            for (i, s) in tr.steps.iter().enumerate() {
                let op = ops.get(i).map(|s| s.as_str()).unwrap_or("?");
                out.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    inst.replace(['\t', '\n'], " "),
                    i,
                    op,
                    s.best_energy,
                    s.mean_energy,
                    s.entropy,
                    s.diversity,
                    s.acceptance,
                    s.frac_elapsed,
                ));
                rows += 1;
            }
        }
        fs::write(dir.join("trajectories.tsv"), out)?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::super::db::ExperimentRecord;
    use super::*;

    fn rec(inst: &str, seq: &[&str], score: f64) -> ExperimentRecord {
        ExperimentRecord {
            instance_id: inst.into(),
            sequence: seq.iter().map(|s| s.to_string()).collect(),
            sweeps: vec![10; seq.len()],
            score,
            baseline: 0.0,
            density: 0.01,
            n: 800,
            backend: "SparseBitSlice".into(),
            ..Default::default()
        }
    }

    #[test]
    fn export_writes_one_row_per_experiment_plus_header() {
        let db = ExperimentDb::from_records(vec![
            rec("G1", &["metropolis_sweep", "greedy_descent"], -0.5),
            rec("G2", &["gibbs_color_sweep"], -0.4),
            rec("G1", &["metropolis_sweep"], -0.3),
        ]);
        let dir = std::env::temp_dir().join(format!("fds_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let n = FoundationDataset::export(&db, &dir).unwrap();
        assert_eq!(n, 3);
        let tsv = fs::read_to_string(dir.join("foundation_dataset.tsv")).unwrap();
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(lines.len(), 4, "header + 3 rows");
        assert!(lines[0].starts_with("instance\tn\t"));
        assert!(lines[1].contains("metropolis_sweep>greedy_descent"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_is_blunt_about_the_scale_gap() {
        let db = ExperimentDb::from_records((0..100).map(|_| rec("g", &["op"], -0.5)).collect());
        let m = FoundationDataset::manifest(&db);
        assert!(m.contains("Examples (experiments): **100**"));
        assert!(m.contains("500000"));
        // 100 examples ⇒ far from every milestone, with an explicit multiplier.
        assert!(m.contains("5000×"), "should state 500000/100 = 5000×: {m}");
        assert!(m.contains("training corpus, not a foundation model"));
    }

    #[test]
    fn decision_log_captures_the_reason_behind_every_fact() {
        let mut g = KnowledgeGraph::new();
        g.observe_if(
            "metropolis_sweep",
            "consensus-prefer",
            "schedule",
            "density<0.05",
            2.5,
            "meta-layer: policy=+3.6, world=+1.5",
        );
        g.observe_if(
            "greedy_descent",
            "theory-explains",
            "exploitation",
            "density<0.05",
            0.9,
            "ablation: removing it worsened best by 40% ⇒ causal",
        );
        let dir = std::env::temp_dir().join(format!("dlog_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let n = FoundationDataset::export_decision_log(&g, &dir).unwrap();
        assert_eq!(n, 2);
        let tsv = fs::read_to_string(dir.join("decision_log.tsv")).unwrap();
        assert!(tsv.starts_with("subject\tpredicate\tobject"));
        assert!(tsv.contains("meta-layer: policy=+3.6"));
        assert!(tsv.contains("ablation: removing it worsened"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn trajectory_export_writes_per_step_search_path() {
        use super::super::dynamics::{StepObs, Trajectory};
        let step = |best: f64, ent: f64, frac: f64| StepObs {
            best_energy: best,
            mean_energy: best + 1.0,
            entropy: ent,
            diversity: 0.1,
            acceptance: 0.3,
            frac_elapsed: frac,
        };
        let tr = Trajectory {
            features: [0.0; 5],
            steps: vec![step(-5.0, 2.0, 0.5), step(-8.0, 1.0, 1.0)],
        };
        let captures = vec![(
            "G11".to_string(),
            vec!["metropolis_sweep".to_string(), "greedy_descent".to_string()],
            tr,
        )];
        let dir = std::env::temp_dir().join(format!("traj_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let rows = FoundationDataset::export_trajectories(&dir, &captures).unwrap();
        assert_eq!(rows, 2);
        let tsv = fs::read_to_string(dir.join("trajectories.tsv")).unwrap();
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(lines.len(), 3, "header + 2 steps");
        assert!(lines[1].contains("metropolis_sweep"));
        assert!(lines[2].contains("greedy_descent"));
        // The search path is visible: best energy improves −5 → −8, entropy falls.
        assert!(lines[1].contains("-5") && lines[2].contains("-8"));
        let _ = fs::remove_dir_all(&dir);
    }
}
