//! `DecisionEngine` — the brain (Constitution §10). Before a run, the lowered
//! IR is analyzed and an execution `Plan` is emitted. This module owns the
//! structural analysis and the (initially rule-based) plan selection; it
//! learns only through the knowledge base (Constitution §12), never through
//! hidden accreted constants.
//!
//! v0.1 scope: compute the structural signals and choose the backend. The
//! operator schedule is a placeholder until the operator library exists
//! (Blueprint v0.1 registers exactly one: GibbsColorSweep).

use super::capability::{Capability, CapabilityQuery, CapabilitySet};
use super::ir::ProblemIR;
use super::knowledge::{InstanceFeatures, KnowledgeBase};
use super::plan::{Backend, Phase, Plan, PlanStep};
use super::registry::OperatorRegistry;

/// Structural fingerprint of an instance (Constitution §10). Extended as
/// operators come to depend on more signals (treewidth, clustering, spectrum).
#[derive(Debug, Clone)]
pub struct InstanceStats {
    pub n: usize,
    pub num_pairs: usize,
    /// 2·pairs / n(n-1) — fraction of possible edges present.
    pub density: f64,
    pub mean_degree: f64,
    /// Coefficient of variation of degree (0 = regular, high = scale-free).
    pub degree_cv: f64,
    /// Average local clustering coefficient ∈ [0, 1]: how often a node's
    /// neighbors are themselves connected (triangle density). A key signal for
    /// which moves help — the scientist reasons about it (e.g. "high clustering
    /// ⇒ greedy after a thermal pass tends to win").
    pub clustering: f64,
    pub integral: bool,
}

impl InstanceStats {
    pub fn analyze(ir: &ProblemIR) -> Self {
        let n = ir.n;
        let num_pairs = ir.num_pairs();
        let degrees: Vec<usize> = (0..n)
            .map(|i| (ir.row_ptr[i + 1] - ir.row_ptr[i]) as usize)
            .collect();
        let sum: usize = degrees.iter().sum();
        let mean_degree = if n > 0 { sum as f64 / n as f64 } else { 0.0 };
        let var = if n > 0 {
            degrees
                .iter()
                .map(|&d| (d as f64 - mean_degree).powi(2))
                .sum::<f64>()
                / n as f64
        } else {
            0.0
        };
        let degree_cv = if mean_degree > 0.0 {
            var.sqrt() / mean_degree
        } else {
            0.0
        };
        let max_edges = if n > 1 {
            n as f64 * (n as f64 - 1.0) / 2.0
        } else {
            1.0
        };
        Self {
            n,
            num_pairs,
            density: num_pairs as f64 / max_edges,
            mean_degree,
            degree_cv,
            clustering: Self::avg_clustering(ir, &degrees),
            integral: ir.is_integral(),
        }
    }

    /// Average local clustering coefficient. For each node with degree ≥ 2, the
    /// fraction of its neighbor-pairs that share an edge; averaged over nodes.
    /// O(Σ deg·mean_deg) — cheap on the sparse instances the engine targets.
    fn avg_clustering(ir: &ProblemIR, degrees: &[usize]) -> f64 {
        let n = ir.n;
        if n == 0 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut counted = 0usize;
        for (i, &deg) in degrees.iter().enumerate() {
            if deg < 2 {
                continue;
            }
            let (nbrs, _) = ir.row(i);
            let nbr_set: std::collections::HashSet<u32> = nbrs.iter().copied().collect();
            let mut links = 0usize;
            for &a in nbrs {
                let (a_nbrs, _) = ir.row(a as usize);
                for &b in a_nbrs {
                    // Count each unordered neighbor-pair edge once (a < b).
                    if a < b && nbr_set.contains(&b) {
                        links += 1;
                    }
                }
            }
            let possible = deg * (deg - 1) / 2;
            total += links as f64 / possible as f64;
            counted += 1;
        }
        if counted > 0 {
            total / counted as f64
        } else {
            0.0
        }
    }

    /// Backend selection rule (ADR-0002, initial form; refined by
    /// measurement). Sparse + integral ⇒ the bit-sliced integer machine;
    /// otherwise the dense float workhorse.
    pub fn select_backend(&self) -> Backend {
        const DENSITY_THRESHOLD: f64 = 0.05;
        if self.density < DENSITY_THRESHOLD && self.integral {
            Backend::SparseBitSlice
        } else {
            Backend::DenseByte
        }
    }
}

/// The decision engine. In v0.1 it is a thin structural analyzer + backend
/// selector; the plan-synthesis logic (feature → tuned schedule, from the
/// knowledge base) arrives with the operator library and Decision Engine
/// milestone (Blueprint Step 6).
pub struct DecisionEngine;

/// A geometric per-replica temperature ladder (hot → cold), the canonical
/// ensemble the thermal operators read from `RuntimeView`.
pub fn geometric_ladder(num_replicas: usize, t_hi: f64, t_lo: f64) -> Vec<f64> {
    let (hi, lo) = (t_hi.max(t_lo), t_hi.min(t_lo).max(1e-6));
    if num_replicas <= 1 {
        return vec![hi];
    }
    let ratio = (lo / hi).powf(1.0 / (num_replicas as f64 - 1.0));
    (0..num_replicas)
        .map(|k| hi * ratio.powi(k as i32))
        .collect()
}

impl DecisionEngine {
    pub fn analyze(ir: &ProblemIR) -> InstanceStats {
        InstanceStats::analyze(ir)
    }

    /// The capability query that describes what is RUNNABLE on this instance:
    /// operators whose passport supports the chosen backend and the instance's
    /// integrality. This is the gate every selection passes through — selection
    /// is a query over PROPERTIES, never a switch over names (Constitution §10).
    pub fn runnable_query(stats: &InstanceStats, backend: Backend) -> CapabilityQuery {
        CapabilityQuery {
            backend: Some(backend),
            integral: Some(stats.integral),
            multi_replica: None,
            ..Default::default()
        }
    }

    /// The operator POOL for an instance: every registered operator compatible
    /// with the instance + backend, selected by CAPABILITY. Sorted, so the pool
    /// (and everything built on it) is deterministic (ADR-0004).
    pub fn operator_pool(
        stats: &InstanceStats,
        backend: Backend,
        registry: &OperatorRegistry,
    ) -> Vec<&'static str> {
        registry.find(&Self::runnable_query(stats, backend))
    }

    /// Select operators that provide a SPECIFIC capability (e.g. "give me a
    /// monotone exploiter"): the required capability, intersected with what is
    /// runnable here. Returns candidates by property; the caller (or the
    /// Evolution Engine) decides among them.
    pub fn select_with(
        stats: &InstanceStats,
        backend: Backend,
        registry: &OperatorRegistry,
        capability: Capability,
    ) -> Vec<&'static str> {
        let query = CapabilityQuery {
            required: CapabilitySet::empty().with(capability),
            ..Self::runnable_query(stats, backend)
        };
        registry.find(&query)
    }

    /// LEARNED operator ranking (Constitution §10, §12) — the Decision Engine's
    /// move from rule-based to learnable. The capability-valid pool is ranked by
    /// each operator's predicted utility on instances SIMILAR to this one, read
    /// from the `KnowledgeBase`. Operators with no evidence keep the deterministic
    /// capability order behind those with evidence, so an empty base degrades
    /// gracefully to the rule-based pool. As the base fills from experiments, the
    /// same instance can get a different, better-informed ranking — it learns.
    pub fn learned_operator_ranking(
        stats: &InstanceStats,
        backend: Backend,
        registry: &OperatorRegistry,
        kb: &KnowledgeBase,
    ) -> Vec<&'static str> {
        let mut pool = Self::operator_pool(stats, backend, registry);
        let feats = InstanceFeatures::from_stats(stats);
        let scores = kb.operator_scores(&feats);
        // Higher learned score first; unseen operators (score None) sort last but
        // keep their capability/name order among themselves (stable, ADR-0004).
        pool.sort_by(|a, b| {
            let sa = scores.get(*a);
            let sb = scores.get(*b);
            match (sa, sb) {
                (Some(x), Some(y)) => y.total_cmp(x).then(a.cmp(b)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.cmp(b),
            }
        });
        pool
    }

    /// Assemble a default rule-based plan BY CAPABILITY: a thermal explorer over
    /// a hot→cold ladder, then a monotone exploiter to quench. The operator
    /// identities are resolved from the registry via capability queries, so this
    /// plan is not wired to any operator name — swap the library and the plan
    /// re-resolves. (The Evolution Engine searches a far larger space than this
    /// hand rule; this is the sensible baseline it must beat.)
    pub fn default_plan(
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        num_replicas: usize,
        sweeps: u32,
        seed: u64,
    ) -> Result<Plan, String> {
        let stats = Self::analyze(ir);
        let backend = stats.select_backend();
        let explore = *Self::select_with(&stats, backend, registry, Capability::Exploration)
            .first()
            .ok_or("no Exploration-capable operator registered for this instance")?;
        let exploit = *Self::select_with(&stats, backend, registry, Capability::Exploitation)
            .first()
            .ok_or("no Exploitation-capable operator registered for this instance")?;
        let mut steps = vec![PlanStep {
            operator: explore.to_string(),
            phase: Phase::Explore,
            sweeps,
            repeat: 1,
        }];
        // Only add a distinct quench step if the exploiter differs from the
        // explorer (otherwise the explore step already covers it).
        if exploit != explore {
            steps.push(PlanStep {
                operator: exploit.to_string(),
                phase: Phase::Exploit,
                sweeps: sweeps.max(1),
                repeat: 1,
            });
        }
        Ok(Plan {
            name: "decision_default".into(),
            backend,
            num_replicas,
            temperatures: geometric_ladder(num_replicas, 4.0, 0.08),
            steps,
            seed,
            rationale: Default::default(),
        })
    }

    /// UTILITY-BASED plan synthesis (Constitution §10) — the Decision Engine's
    /// move from ranking to trainable selection. For every capability-valid
    /// candidate of each phase it computes
    ///
    ///   U = α·q̂ + β·ĉ + γ·m̂ + δ·l̂   (minimized)
    ///
    /// where q̂ is the KB-predicted quality (learned from experiments on
    /// similar instances, §12), ĉ the operator's declared work per step, m̂ the
    /// backend's ensemble-memory estimate, and l̂ the plan-length cost — q̂/ĉ
    /// normalized over the candidate set so the α/β/γ/δ weights express pure
    /// priorities. The winning operator per phase enters the plan, and the FULL
    /// breakdown (with the input features) is recorded in `Plan::rationale`,
    /// which the Runtime copies into every step's decision log — rule 2: a
    /// wrong decision is a reproducible defect, not a mystery. The only learned
    /// inputs come from the KnowledgeBase (rule 3), so the synthesis improves
    /// as experiments accumulate — trainable, not hardcoded.
    pub fn synthesize_plan(
        ir: &ProblemIR,
        registry: &OperatorRegistry,
        kb: &KnowledgeBase,
        weights: &super::evolution::UtilityWeights,
        num_replicas: usize,
        sweeps: u32,
        seed: u64,
    ) -> Result<Plan, String> {
        let stats = Self::analyze(ir);
        let backend = stats.select_backend();
        let feats = InstanceFeatures::from_stats(&stats);
        let learned = kb.operator_scores(&feats);
        let shape = super::operator::InstanceShape {
            n: stats.n,
            num_pairs: stats.num_pairs,
            num_replicas,
        };
        // Coarse but declared memory model per backend (bytes).
        let mem = match backend {
            Backend::DenseByte => {
                (stats.n * stats.n) as f64 * 8.0 + (stats.n * num_replicas) as f64 * 9.0
            }
            Backend::SparseBitSlice => (stats.n * num_replicas) as f64 * 9.0,
        };
        let features_str = format!(
            "n={} density={:.4} clustering={:.3} degree_cv={:.2} integral={}",
            stats.n, stats.density, stats.clustering, stats.degree_cv, stats.integral
        );

        // Pick the minimum-utility candidate for one phase; returns the winner
        // and its recorded rationale.
        let pick = |capability: Capability, phase: &str| -> Option<(String, String)> {
            let cands = Self::select_with(&stats, backend, registry, capability);
            if cands.is_empty() {
                return None;
            }
            // Normalizers over the candidate set (max magnitude per term).
            let costs: Vec<f64> = cands
                .iter()
                .map(|op| {
                    registry
                        .lookup(op)
                        .map(|o| o.cost_model(shape).work_per_sweep * sweeps as f64)
                        .unwrap_or(0.0)
                })
                .collect();
            let max_cost = costs.iter().fold(1e-12f64, |a, &b| a.max(b));
            let max_q = cands
                .iter()
                .filter_map(|op| learned.get(*op))
                .fold(1e-12f64, |a, &b| a.max(b.abs()));

            let mut best: Option<(String, f64, String)> = None;
            for (op, cost) in cands.iter().zip(&costs) {
                let (q_norm, evidence) = match learned.get(*op) {
                    // Higher learned score = better ⇒ negate for minimization.
                    Some(s) => (-s / max_q, format!("KB score {s:+.3}")),
                    None => (0.0, "no KB evidence (neutral)".to_string()),
                };
                let c_norm = cost / max_cost;
                let u = weights.quality * q_norm
                    + weights.cost * c_norm
                    + weights.memory * mem
                    + weights.latency;
                let why = format!(
                    "{phase} ← {op}: U={u:.6} = α({})·q̂({q_norm:+.3}) + β({})·ĉ({c_norm:.3}) + γ({})·m̂({mem:.0}B) + δ({})·1 | {evidence} | features: {features_str}",
                    weights.quality, weights.cost, weights.memory, weights.latency
                );
                match &best {
                    // Strict improvement over sorted candidates ⇒ deterministic ties.
                    Some((_, bu, _)) if *bu <= u => {}
                    _ => best = Some((op.to_string(), u, why)),
                }
            }
            best.map(|(op, _, why)| (op, why))
        };

        let mut steps = Vec::new();
        let mut rationale = std::collections::HashMap::new();
        let (explore, why) = pick(Capability::Exploration, "Explore")
            .ok_or("no Exploration-capable operator registered for this instance")?;
        rationale.insert(explore.clone(), why);
        steps.push(PlanStep {
            operator: explore.clone(),
            phase: Phase::Explore,
            sweeps,
            repeat: 1,
        });
        // A barrier-crossing pass is scheduled when one is runnable; whether it
        // EARNS its slot is learned through the KB, not assumed here.
        if let Some((barrier, why)) = pick(Capability::BarrierCrossing, "Exploit(barrier)") {
            if barrier != explore {
                rationale.insert(barrier.clone(), why);
                steps.push(PlanStep {
                    operator: barrier.clone(),
                    phase: Phase::Exploit,
                    sweeps: sweeps.max(1) / 2 + 1,
                    repeat: 1,
                });
            }
        }
        let (exploit, why) = pick(Capability::Exploitation, "Finish")
            .ok_or("no Exploitation-capable operator registered for this instance")?;
        rationale.insert(exploit.clone(), why);
        steps.push(PlanStep {
            operator: exploit,
            phase: Phase::Finish,
            sweeps: sweeps.max(1),
            repeat: 1,
        });

        Ok(Plan {
            name: "utility_synthesis".into(),
            backend,
            num_replicas,
            temperatures: geometric_ladder(num_replicas, 4.0, 0.08),
            steps,
            seed,
            rationale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clustering_coefficient_is_correct() {
        // Triangle: each node's two neighbors are connected ⇒ clustering 1.0.
        let tri = ProblemIR::from_pairs(
            3,
            0.0,
            vec![0.0; 3],
            &[(0, 1, 1.0), (1, 2, 1.0), (0, 2, 1.0)],
        );
        assert!((InstanceStats::analyze(&tri).clustering - 1.0).abs() < 1e-9);
        // Path 0-1-2-3: node 1's neighbors (0,2) are not connected ⇒ 0.0.
        let path = ProblemIR::from_pairs(
            4,
            0.0,
            vec![0.0; 4],
            &[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0)],
        );
        assert!(InstanceStats::analyze(&path).clustering.abs() < 1e-9);
    }

    #[test]
    fn sparse_integral_selects_bitslice() {
        // A path graph on 100 nodes: very sparse, integral.
        let pairs: Vec<(u32, u32, f64)> = (0..99).map(|i| (i, i + 1, -1.0)).collect();
        let ir = ProblemIR::from_pairs(100, 0.0, vec![0.0; 100], &pairs);
        let stats = InstanceStats::analyze(&ir);
        assert!(stats.density < 0.05);
        assert!(stats.integral);
        assert_eq!(stats.select_backend(), Backend::SparseBitSlice);
    }

    #[test]
    fn selects_operators_by_capability_not_name() {
        use super::super::registry::OperatorRegistry;
        // Sparse integral path graph → SparseBitSlice.
        let pairs: Vec<(u32, u32, f64)> = (0..99).map(|i| (i, i + 1, -1.0)).collect();
        let ir = ProblemIR::from_pairs(100, 0.0, vec![0.0; 100], &pairs);
        let stats = InstanceStats::analyze(&ir);
        let backend = stats.select_backend();
        let reg = OperatorRegistry::standard();

        // Whole standard library is capability-selected (all run on a sparse
        // integral instance).
        let pool = DecisionEngine::operator_pool(&stats, backend, &reg);
        assert_eq!(
            pool.len(),
            reg.len(),
            "pool should be the whole compatible library"
        );
        assert!(pool.len() >= 13);

        // "Give me a monotone exploiter" resolves to greedy WITHOUT naming it.
        let exploiters =
            DecisionEngine::select_with(&stats, backend, &reg, Capability::Exploitation);
        assert!(exploiters.contains(&"greedy_descent"));
        // Barrier-crossing is provided by the cluster / exchange / extremal
        // operators — selected purely by capability, never by name.
        let barrier =
            DecisionEngine::select_with(&stats, backend, &reg, Capability::BarrierCrossing);
        assert!(barrier.contains(&"houdayer_cluster"));
        assert!(barrier.contains(&"replica_exchange"));

        // The default plan is assembled from capability queries and runs.
        let plan = DecisionEngine::default_plan(&ir, &reg, 8, 5, 1).unwrap();
        assert_eq!(plan.backend, Backend::SparseBitSlice);
        assert!(!plan.steps.is_empty());
        assert_eq!(plan.temperatures.len(), 8);
    }

    #[test]
    fn decision_engine_learns_from_the_knowledge_base() {
        use super::super::knowledge::{Experience, InstanceFeatures, KnowledgeBase};
        use super::super::registry::OperatorRegistry;
        let pairs: Vec<(u32, u32, f64)> = (0..99).map(|i| (i, i + 1, -1.0)).collect();
        let ir = ProblemIR::from_pairs(100, 0.0, vec![0.0; 100], &pairs);
        let stats = InstanceStats::analyze(&ir);
        let backend = stats.select_backend();
        let reg = OperatorRegistry::standard();

        // Empty base ⇒ ranking == capability order (rule-based fallback).
        let empty = KnowledgeBase::new();
        let base_rank = DecisionEngine::learned_operator_ranking(&stats, backend, &reg, &empty);
        assert_eq!(base_rank.len(), reg.len());

        // Record that on similar instances, extremal_optimization won big.
        let mut kb = KnowledgeBase::new();
        kb.record(Experience {
            features: InstanceFeatures::from_stats(&stats),
            backend: "SparseBitSlice".into(),
            sequence: vec!["extremal_optimization".into(), "greedy_descent".into()],
            sweeps: vec![10, 10],
            temp_hi: 4.0,
            temp_lo: 0.1,
            best_energy: -999.0,
            best_utility: -999.0,
        });
        let learned = DecisionEngine::learned_operator_ranking(&stats, backend, &reg, &kb);
        // The evidence-backed operator now leads the ranking — learned, not ruled.
        assert_eq!(learned[0], "extremal_optimization");
        assert_ne!(learned, base_rank, "the base changed the decision");
    }

    #[test]
    fn utility_synthesis_records_breakdown_and_learns_from_kb() {
        use super::super::evolution::UtilityWeights;
        use super::super::knowledge::{Experience, InstanceFeatures, KnowledgeBase};
        use super::super::registry::OperatorRegistry;
        let pairs: Vec<(u32, u32, f64)> = (0..99).map(|i| (i, i + 1, -1.0)).collect();
        let ir = ProblemIR::from_pairs(100, 0.0, vec![0.0; 100], &pairs);
        let reg = OperatorRegistry::standard();
        let w = UtilityWeights::default();

        // Empty KB: a valid plan with a full recorded rationale per operator.
        let empty = KnowledgeBase::new();
        let plan = DecisionEngine::synthesize_plan(&ir, &reg, &empty, &w, 8, 20, 7).unwrap();
        assert_eq!(plan.name, "utility_synthesis");
        assert!(!plan.steps.is_empty());
        for step in &plan.steps {
            let why = plan
                .rationale
                .get(&step.operator)
                .unwrap_or_else(|| panic!("no rationale for {}", step.operator));
            for needle in ["U=", "α(", "β(", "γ(", "δ(", "features: n=100"] {
                assert!(why.contains(needle), "rationale missing {needle}: {why}");
            }
            assert!(why.contains("no KB evidence"), "empty KB must be disclosed");
        }
        // Deterministic synthesis.
        let plan2 = DecisionEngine::synthesize_plan(&ir, &reg, &empty, &w, 8, 20, 7).unwrap();
        let ops1: Vec<_> = plan.steps.iter().map(|s| &s.operator).collect();
        let ops2: Vec<_> = plan2.steps.iter().map(|s| &s.operator).collect();
        assert_eq!(ops1, ops2);

        // KB evidence flips the Finish choice: steepest_descent won big on a
        // similar instance, so the utility term now prefers it.
        let stats = InstanceStats::analyze(&ir);
        let mut kb = KnowledgeBase::new();
        kb.record(Experience {
            features: InstanceFeatures::from_stats(&stats),
            backend: "SparseBitSlice".into(),
            sequence: vec!["steepest_descent".into()],
            sweeps: vec![10],
            temp_hi: 4.0,
            temp_lo: 0.1,
            best_energy: -999.0,
            best_utility: -999.0,
        });
        let learned_plan = DecisionEngine::synthesize_plan(&ir, &reg, &kb, &w, 8, 20, 7).unwrap();
        let finish = &learned_plan.steps.last().unwrap().operator;
        assert_eq!(
            finish, "steepest_descent",
            "KB evidence must change the selection: {:?}",
            learned_plan.rationale
        );
        assert!(learned_plan.rationale[finish].contains("KB score"));
        // The runtime executes the synthesized plan and surfaces the rationale
        // in its per-step decision log (no more "scheduled by plan" stub).
        let mut state = super::super::backends::ReferenceState::new(&ir, 8, &[0u8; 100]);
        let mut rt = super::super::runtime::Runtime::new(
            super::super::context::RunContext::new(7),
            &learned_plan,
        );
        let rec = rt.run(&learned_plan, &mut state, &reg, &ir).unwrap();
        assert!(
            rec.events.iter().all(|e| e.decision.contains("U=")),
            "every step must carry the utility rationale"
        );
    }

    #[test]
    fn dense_selects_densebyte() {
        // Complete graph on 20 nodes: dense.
        let mut pairs = Vec::new();
        for i in 0..20u32 {
            for j in (i + 1)..20 {
                pairs.push((i, j, 1.0));
            }
        }
        let ir = ProblemIR::from_pairs(20, 0.0, vec![0.0; 20], &pairs);
        let stats = InstanceStats::analyze(&ir);
        assert!(stats.density > 0.05);
        assert_eq!(stats.select_backend(), Backend::DenseByte);
    }
}
