use crate::core::hubo::{FlatHuboModel, HuboModel};
use crate::solver::engine::{calculate_replica_energies, step, StepScratch};
use crate::solver::types::{QuantumField, NUM_REPLICAS};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Temperature-ladder tuning strategy for PA-PT mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LadderMode {
    /// Acceptance uniformization (Rathore-Chopra-de Pablo 2005 / Kofke 2002).
    #[default]
    AcceptanceUniform,
    /// Round-trip feedback optimization (Katzgraber-Trebst-Huse-Troyer 2006).
    FeedbackOptimized,
}

#[derive(Clone)]
pub struct UltimateSolver {
    pub num_replicas: usize,
    pub temp_max: f64,
    pub temp_min: f64,
    pub sweeps_per_exchange: usize,
    pub total_exchanges: usize,
    pub seed: Option<u64>,
    pub num_slices: usize,
    pub num_temps: usize,
    pub num_pops: usize,
    pub gnn_heuristic_probs: Option<Vec<f64>>,
    /// Population Annealing mode (num_pops > 1): resample when the effective
    /// sample size falls below this fraction of the population (0 < θ ≤ 1).
    pub ess_threshold: f64,
    /// PA-PT mode: iterations of the pre-run adaptive-ladder tuning phase
    /// (acceptance-uniformization; 0 disables tuning → geometric ladder).
    /// The tuned shape is FROZEN before the production run.
    pub ladder_tuning_iters: usize,
    /// PA-PT mode: PT steps per tuning iteration used to measure swap
    /// acceptance rates.
    pub ladder_tuning_steps: usize,
    /// PA-PT mode: extra full-PT relaxation steps run immediately after
    /// every resampling event, decorrelating cloned members before the next
    /// Boltzmann reweighting (Wang, Machta & Katzgraber, PRE 92, 063307
    /// (2015): equilibrate after resampling).
    pub post_resample_relaxation: usize,
    /// Isoenergetic Cluster Moves (Houdayer/Zhu-Ochoa-Katzgraber). Applied
    /// once per exchange/stage when the model is pairwise and num_slices==1
    /// (auto-gated). Off by default to preserve legacy trajectories.
    pub use_icm: bool,
    /// Path-relinking finisher: after annealing, relink diverse elite
    /// replicas (Wang-Lü-Glover-Hao 2012) and keep the best. Off by default.
    pub use_path_relinking: bool,
    /// Ladder-tuning strategy (PA-PT mode). Defaults to acceptance
    /// uniformization; FeedbackOptimized uses the KTHT round-trip method.
    pub ladder_mode: LadderMode,
}

impl UltimateSolver {
    pub fn new(
        temp_max: f64,
        temp_min: f64,
        sweeps: usize,
        exchanges: usize,
        seed: Option<u64>,
    ) -> Self {
        Self {
            num_replicas: NUM_REPLICAS,
            temp_max,
            temp_min,
            sweeps_per_exchange: sweeps,
            total_exchanges: exchanges,
            seed,
            num_slices: 1,
            num_temps: 10,
            num_pops: 1,
            gnn_heuristic_probs: None,
            ess_threshold: 0.5,
            ladder_tuning_iters: 8,
            ladder_tuning_steps: 10,
            post_resample_relaxation: 5,
            use_icm: false,
            use_path_relinking: false,
            ladder_mode: LadderMode::AcceptanceUniform,
        }
    }

    /// Runs `num_restarts` independent solves with Luby-scaled budgets
    /// (restart i uses `total_exchanges · luby(i+1)` exchanges) and distinct
    /// derived seeds, returning the best solution by true model energy.
    /// Deterministic given `seed`; exploits heavy-tailed runtime
    /// distributions (Luby, Sinclair & Zuckerman 1993). The base budget and
    /// all other settings are inherited.
    pub fn solve_with_luby_restarts(
        &self,
        model: &crate::core::QuboModel,
        clamped: &[(usize, i8)],
        num_restarts: usize,
    ) -> Vec<i8> {
        let base_seed = self.seed.unwrap_or_else(rand::random);
        let mut best: Option<(f64, Vec<i8>)> = None;
        for i in 0..num_restarts.max(1) {
            let scale = crate::solver::tts::luby(i + 1);
            let restart = UltimateSolver {
                total_exchanges: self.total_exchanges * scale,
                seed: Some(
                    base_seed.wrapping_add(0x9e3779b97f4a7c15u64.wrapping_mul(i as u64 + 1)),
                ),
                ..self.clone()
            };
            let s = restart.solve(model, clamped);
            let e = model.calculate_total_energy(&s);
            if best.as_ref().map(|(be, _)| e < *be).unwrap_or(true) {
                best = Some((e, s));
            }
        }
        best.map(|(_, s)| s).unwrap_or_default()
    }

    pub fn with_gnn_heuristic(mut self, probs: Vec<f64>) -> Self {
        self.gnn_heuristic_probs = Some(probs);
        self
    }

    pub fn with_quantum_dims(mut self, slices: usize, temps: usize, pops: usize) -> Self {
        self.num_slices = slices;
        self.num_temps = temps;
        self.num_pops = pops;
        self
    }

    pub fn solve(&self, model: &crate::core::QuboModel, clamped: &[(usize, i8)]) -> Vec<i8> {
        // Convert QuboModel → HuboModel → FlatHuboModel
        let mut hubo_model = HuboModel::new(model.num_vars);
        hubo_model.linear = model.linear.clone();
        for i in 0..model.num_vars {
            for (j, weight) in model.quadratic.get_row(i) {
                hubo_model.edges2[i].push(crate::core::hubo::Edge2 { j, weight });
            }
        }
        // Flatten to CSR layout for SIMD-optimized traversal
        let flat_model = FlatHuboModel::from_hubo(&hubo_model);

        let n = model.num_vars;
        let base_seed = self.seed.unwrap_or_else(rand::random);
        let mut rng = ChaCha8Rng::seed_from_u64(base_seed);

        let mut is_clamped = vec![false; n];
        let mut clamped_val = vec![0i8; n];
        for &(idx, val) in clamped {
            is_clamped[idx] = true;
            clamped_val[idx] = val;
        }

        // Presolve: exact probing persistency (strictly dominates
        // first-order; provably preserves a global optimum; never touches
        // user-clamped indices). See crate::presolve.
        let derived = crate::presolve::fix_persistent_variables_probing(model, clamped);
        for &(idx, val) in &derived {
            is_clamped[idx] = true;
            clamped_val[idx] = val;
        }

        // Decomposition: if the free-variable interaction graph is
        // disconnected, energies are additive across components, so solving
        // each component independently and merging the results is exact.
        // Fixed variables do not transmit coupling; their =1 contributions
        // are folded into component fields by extract_component.
        let free: Vec<bool> = is_clamped.iter().map(|&c| !c).collect();
        let components = crate::presolve::connected_components(model, &free);
        if components.len() > 1 {
            let mut fixed_one = vec![false; n];
            let mut result = vec![0i8; n];
            for v in 0..n {
                if is_clamped[v] {
                    result[v] = clamped_val[v];
                    fixed_one[v] = clamped_val[v] == 1;
                }
            }
            for (c_idx, comp) in components.iter().enumerate() {
                let (sub_model, map) = crate::presolve::extract_component(model, comp, &fixed_one);
                let sub_solver = UltimateSolver {
                    num_replicas: self.num_replicas,
                    temp_max: self.temp_max,
                    temp_min: self.temp_min,
                    sweeps_per_exchange: self.sweeps_per_exchange,
                    total_exchanges: self.total_exchanges,
                    // Deterministic per-component seed derived from the run seed.
                    seed: Some(base_seed.wrapping_add(c_idx as u64 + 1)),
                    num_slices: self.num_slices,
                    num_temps: self.num_temps,
                    num_pops: self.num_pops,
                    gnn_heuristic_probs: self
                        .gnn_heuristic_probs
                        .as_ref()
                        .map(|p| map.iter().map(|&g| p[g]).collect()),
                    ess_threshold: self.ess_threshold,
                    ladder_tuning_iters: self.ladder_tuning_iters,
                    ladder_tuning_steps: self.ladder_tuning_steps,
                    post_resample_relaxation: self.post_resample_relaxation,
                    use_icm: self.use_icm,
                    use_path_relinking: self.use_path_relinking,
                    ladder_mode: self.ladder_mode,
                };
                let sub_state = sub_solver.solve(&sub_model, &[]);
                for (li, &gi) in map.iter().enumerate() {
                    result[gi] = sub_state[li];
                }
            }
            return result;
        }

        // Hybrid PA-PT mode: num_pops > 1 runs Population Annealing with
        // Parallel Tempering equilibration at every annealing stage
        // (see solve_population_annealing) instead of the static-ladder PT
        // path below.
        if self.num_pops > 1 {
            return self.solve_population_annealing(
                model,
                &flat_model,
                &is_clamped,
                &clamped_val,
                base_seed,
            );
        }

        // Initialize 5D Field with byte-per-replica layout
        let mut field = QuantumField::new(n, self.num_slices, self.num_temps, self.num_pops);

        // Generate temperature schedule (geometric)
        let mut temps = Vec::with_capacity(self.num_temps);
        let temp_factor = if self.num_temps > 1 {
            (self.temp_min / self.temp_max).powf(1.0 / (self.num_temps - 1) as f64)
        } else {
            1.0
        };
        for i in 0..self.num_temps {
            temps.push(self.temp_max * temp_factor.powi(i as i32));
        }

        // Initialize States — byte-per-replica layout
        for p in 0..self.num_pops {
            for t in 0..self.num_temps {
                for s in 0..self.num_slices {
                    for v in 0..n {
                        let base = field.var_base(v, s, t, p);
                        if is_clamped[v] {
                            // Set all replicas to clamped value
                            let val = if clamped_val[v] == 1 { 1i8 } else { 0i8 };
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = val;
                            }
                        } else if let Some(ref probs) = self.gnn_heuristic_probs {
                            // GNN heuristic: each replica independently sampled
                            let prob_one = probs[v];
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = if rng.gen_range(0.0..1.0) < prob_one {
                                    1
                                } else {
                                    0
                                };
                            }
                        } else {
                            // Random initialization: extract bits from random u64
                            let random_word: u64 = rng.gen();
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = ((random_word >> r) & 1) as i8;
                            }
                        }
                    }
                }
            }
        }

        let j_tau = 1.0;

        // Compute initial energies for incremental tracking
        for pp in 0..self.num_pops {
            for tt in 0..self.num_temps {
                field.energies[tt + self.num_temps * pp] =
                    calculate_replica_energies(&flat_model, &field, tt, pp, j_tau);
            }
        }

        // ICM is applicable only for pairwise models on a single slice.
        let icm_enabled = self.use_icm
            && self.num_slices == 1
            && crate::solver::icm::is_icm_applicable(&flat_model);

        // Main solver loop with periodic energy re-sync.
        // Scratch is allocated once and reused across every step.
        let mut scratch = StepScratch::for_field(&field);
        let mut incumbent_energy = f64::INFINITY;
        let mut incumbent = vec![0i8; n];
        let mut incumbent_buf = vec![0i8; n];
        let mut step_counter = 0u64;
        for _ in 0..self.total_exchanges {
            for _ in 0..self.sweeps_per_exchange {
                step(
                    &mut field,
                    &flat_model,
                    &temps,
                    j_tau,
                    &is_clamped,
                    &mut scratch,
                    &mut rng,
                );
                step_counter += 1;
                // Re-sync every 1000 steps to prevent floating-point drift
                if step_counter.is_multiple_of(1000) {
                    for pp in 0..self.num_pops {
                        for tt in 0..self.num_temps {
                            field.energies[tt + self.num_temps * pp] =
                                calculate_replica_energies(&flat_model, &field, tt, pp, j_tau);
                        }
                    }
                }
            }
            if icm_enabled {
                crate::solver::icm::icm_sweep(&mut field, &flat_model, &mut rng);
            }
            Self::update_incumbent(
                model,
                &field,
                &mut incumbent_energy,
                &mut incumbent,
                &mut incumbent_buf,
            );
        }

        // Extract best replica — direct byte read (no bit extraction!)
        let mut best_state = vec![0i8; n];
        let mut best_energy = f64::INFINITY;
        let mut state_buf = vec![0i8; n];

        for p in 0..self.num_pops {
            for t in 0..self.num_temps {
                for s in 0..self.num_slices {
                    for r in 0..NUM_REPLICAS {
                        for (v, slot) in state_buf.iter_mut().enumerate() {
                            // Direct byte read — no shifting, no masking
                            *slot = field.get_replica(v, s, t, p, r);
                        }
                        let energy = model.calculate_total_energy(&state_buf);
                        if energy < best_energy {
                            best_energy = energy;
                            best_state.copy_from_slice(&state_buf);
                        }
                    }
                }
            }
        }
        // Anytime guarantee: never return worse than the best state seen
        // during the run; then polish to a 1-opt local minimum.
        if incumbent_energy < best_energy {
            best_state.copy_from_slice(&incumbent);
        }
        crate::solver::local_search::steepest_descent_1opt(model, &mut best_state, &is_clamped);
        if self.use_path_relinking {
            let e = model.calculate_total_energy(&best_state);
            best_state = Self::path_relink_finish(model, &field, &is_clamped, best_state, e);
        }
        best_state
    }

    /// Updates the best-seen (incumbent) solution from the current field.
    ///
    /// Tracked energies are only the TRIGGER (cheap argmin scan; they may
    /// carry bounded drift and, for num_slices > 1, the Trotter term): the
    /// candidate replica is re-scored with the true model energy before the
    /// incumbent is replaced, so the incumbent can only improve. Without
    /// this, a configuration visited mid-run and destroyed by later sweeps
    /// or resampling would be unrecoverable (anytime-correctness).
    fn update_incumbent(
        model: &crate::core::QuboModel,
        field: &QuantumField,
        best_energy: &mut f64,
        best_state: &mut [i8],
        buf: &mut [i8],
    ) {
        let mut arg = (0usize, 0usize);
        let mut min_tracked = f64::INFINITY;
        for (cell, es) in field.energies.iter().enumerate() {
            for (r, &e) in es.iter().enumerate() {
                if e < min_tracked {
                    min_tracked = e;
                    arg = (cell, r);
                }
            }
        }
        if !min_tracked.is_finite() {
            return;
        }
        let (cell, r) = arg;
        let t = cell % field.num_temps;
        let p = cell / field.num_temps;
        for s in 0..field.num_slices {
            for (v, slot) in buf.iter_mut().enumerate() {
                *slot = field.get_replica(v, s, t, p, r);
            }
            let e = model.calculate_total_energy(buf);
            if e < *best_energy {
                *best_energy = e;
                best_state.copy_from_slice(buf);
            }
        }
    }

    /// Path-relinking finisher: harvests diverse elite replicas from the
    /// field into an archive, relinks all elite pairs, polishes each result
    /// to a 1-opt minimum, and returns the best state found (never worse
    /// than `current`). Deterministic. See crate::solver::elite.
    fn path_relink_finish(
        model: &crate::core::QuboModel,
        field: &QuantumField,
        is_clamped: &[bool],
        current: Vec<i8>,
        current_energy: f64,
    ) -> Vec<i8> {
        use crate::solver::elite::{path_relink, EliteArchive};
        let n = model.num_vars;
        let min_h = (n / 10).max(1);
        let mut archive = EliteArchive::new(8, min_h);
        archive.insert(&current, current_energy);
        let mut buf = vec![0i8; n];
        for cell in 0..field.energies.len() {
            let t = cell % field.num_temps;
            let p = cell / field.num_temps;
            for r in 0..NUM_REPLICAS {
                for s in 0..field.num_slices {
                    for (v, slot) in buf.iter_mut().enumerate() {
                        *slot = field.get_replica(v, s, t, p, r);
                    }
                    let e = model.calculate_total_energy(&buf);
                    archive.insert(&buf, e);
                }
            }
        }
        let elites: Vec<Vec<i8>> = archive.entries().iter().map(|(_, s)| s.clone()).collect();
        let mut best = current;
        let mut best_e = current_energy;
        for i in 0..elites.len() {
            for j in (i + 1)..elites.len() {
                let (mut s, _) = path_relink(model, &elites[i], &elites[j]);
                crate::solver::local_search::steepest_descent_1opt(model, &mut s, is_clamped);
                let e = model.calculate_total_energy(&s);
                if e < best_e {
                    best_e = e;
                    best = s;
                }
            }
        }
        best
    }

    /// Initializes population states (clamped / GNN warm-start / random)
    /// with the same policy as the PT path, for a field of any geometry.
    fn init_field_states(
        &self,
        field: &mut QuantumField,
        is_clamped: &[bool],
        clamped_val: &[i8],
        rng: &mut ChaCha8Rng,
    ) {
        for p in 0..field.num_pops {
            for t in 0..field.num_temps {
                for s in 0..field.num_slices {
                    for v in 0..field.num_vars {
                        let base = field.var_base(v, s, t, p);
                        if is_clamped[v] {
                            let val = if clamped_val[v] == 1 { 1i8 } else { 0i8 };
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = val;
                            }
                        } else if let Some(ref probs) = self.gnn_heuristic_probs {
                            let prob_one = probs[v];
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = if rng.gen_range(0.0..1.0) < prob_one {
                                    1
                                } else {
                                    0
                                };
                            }
                        } else {
                            let random_word: u64 = rng.gen();
                            for r in 0..NUM_REPLICAS {
                                field.spins[base + r] = ((random_word >> r) & 1) as i8;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Hybrid Population Annealing + Parallel Tempering (PA-PT).
    ///
    /// Population Annealing per Machta, PRE 82, 026704 (2010) and Wang,
    /// Machta & Katzgraber, PRE 92, 063307 (2015), with the equilibration
    /// step of every annealing stage performed by full Parallel Tempering
    /// (Hukushima & Nemoto, J. Phys. Soc. Jpn. 65, 1604 (1996)):
    ///
    /// - Population: R = num_pops × NUM_REPLICAS members. A MEMBER is lane r
    ///   of population p across the ENTIRE temperature ladder.
    /// - Stages k = 0..total_exchanges: the ladder spans [T_cold(k), temp_max]
    ///   geometrically with `num_temps` levels; T_cold anneals geometrically
    ///   from temp_max to temp_min across stages. The hot end never cools,
    ///   preserving PT's ergodicity anchor. Each stage runs
    ///   `sweeps_per_exchange` PT steps (sweeps at every level + replica
    ///   exchange — PT is active at all levels throughout).
    /// - After each completed stage, member i accumulates the Boltzmann
    ///   log-weight −Δβ_cold·E_cold,i from its coldest-level energy (the
    ///   level whose β changed). Weights are normalized by log-sum-exp;
    ///   when ESS = 1/Σw̃² falls below ess_threshold·R the population is
    ///   systematically resampled (single random offset, randomly shuffled
    ///   offspring, whole-ladder configuration and all tracked energies
    ///   copied exactly) and the log-weights reset.
    ///
    /// Two refinements on top of the textbook scheme:
    ///
    /// - ADAPTIVE LADDER: before the production run, an acceptance-
    ///   uniformization tuning phase (`ladder_tuning_iters` ×
    ///   `ladder_tuning_steps` PT steps on a throwaway field at the full
    ///   span) adapts the interior log-temperature positions toward uniform
    ///   pair swap acceptance (Rathore-Chopra-de Pablo 2005; Kofke 2002;
    ///   round-trip refinement: Katzgraber-Trebst-Huse-Troyer, JSTAT P03018
    ///   (2006)). The tuned shape is then FROZEN, preserving detailed
    ///   balance during production; every stage ladder reuses it.
    /// - POST-RESAMPLING RELAXATION: `post_resample_relaxation` extra
    ///   full-ladder PT steps run immediately after every resampling event
    ///   to decorrelate cloned members before the next reweighting.
    ///
    /// num_temps = 1 degenerates to textbook single-chain PA; num_pops = 1
    /// (the default solve path) is plain PT and is unaffected.
    /// Deterministic given `seed`.
    fn solve_population_annealing(
        &self,
        model: &crate::core::QuboModel,
        flat_model: &FlatHuboModel,
        is_clamped: &[bool],
        clamped_val: &[i8],
        base_seed: u64,
    ) -> Vec<i8> {
        self.solve_population_annealing_diag(model, flat_model, is_clamped, clamped_val, base_seed)
            .0
    }

    /// Population Annealing driver returning solution and diagnostics
    /// (free energy, ESS evolution, family entropy — Machta 2010;
    /// Wang-Machta-Katzgraber 2015). The plain `solve` path discards the
    /// diagnostics; `solve_with_pa_diagnostics` exposes them.
    #[allow(clippy::type_complexity)]
    fn solve_population_annealing_diag(
        &self,
        model: &crate::core::QuboModel,
        flat_model: &FlatHuboModel,
        is_clamped: &[bool],
        clamped_val: &[i8],
        base_seed: u64,
    ) -> (Vec<i8>, crate::solver::population_annealing::PaDiagnostics) {
        use crate::solver::population_annealing as pa;

        let n = model.num_vars;
        let nt = self.num_temps.max(1);
        let r_total = self.num_pops * NUM_REPLICAS;
        let mut rng = ChaCha8Rng::seed_from_u64(base_seed);
        let j_tau = 1.0;
        let mut family_ids: Vec<usize> = (0..r_total).collect();
        let mut diagnostics = pa::PaDiagnostics::default();
        let pa_icm_enabled = self.use_icm
            && self.num_slices == 1
            && crate::solver::icm::is_icm_applicable(flat_model);

        // ------------------------------------------------------------------
        // Adaptive-ladder tuning phase (frozen before the production run).
        //
        // The tunable object is the ladder SHAPE: interior positions
        // α_t ∈ (0,1) in log-temperature space, T_t = T_max·(T_c/T_max)^α_t
        // (uniform α ≡ geometric ladder). Tuning runs on a THROWAWAY field at
        // the full production span so the production population's hot start —
        // required for an unbiased PA weight chain — is not contaminated.
        // Each iteration measures per-pair swap acceptance and applies the
        // acceptance-uniformization update (see pa::adapt_spacings; Rathore-
        // Chopra-de Pablo 2005 / Kofke 2002; round-trip refinement:
        // Katzgraber-Trebst-Huse-Troyer, JSTAT P03018 (2006)). Endpoints are
        // never adapted — they belong to the PA annealing schedule.
        // ------------------------------------------------------------------
        let denom = (nt - 1).max(1) as f64;
        let mut alphas: Vec<f64> = (0..nt).map(|t| t as f64 / denom).collect();
        if nt >= 3 && self.ladder_tuning_iters > 0 && self.ladder_tuning_steps > 0 {
            let mut tune_field = QuantumField::new(n, self.num_slices, nt, self.num_pops);
            self.init_field_states(&mut tune_field, is_clamped, clamped_val, &mut rng);
            for p in 0..self.num_pops {
                for t in 0..nt {
                    tune_field.energies[t + nt * p] =
                        calculate_replica_energies(flat_model, &tune_field, t, p, j_tau);
                }
            }
            let mut tune_scratch = StepScratch::for_field(&tune_field);
            match self.ladder_mode {
                LadderMode::AcceptanceUniform => tune_scratch.enable_swap_stats(),
                LadderMode::FeedbackOptimized => tune_scratch.enable_roundtrip_tracking(),
            }
            let mut temps = vec![0.0f64; nt];
            for _ in 0..self.ladder_tuning_iters {
                for (t, slot) in temps.iter_mut().enumerate() {
                    *slot = self.temp_max * (self.temp_min / self.temp_max).powf(alphas[t]);
                }
                match self.ladder_mode {
                    LadderMode::AcceptanceUniform => tune_scratch.reset_swap_stats(),
                    LadderMode::FeedbackOptimized => tune_scratch.reset_roundtrip_stats(),
                }
                for _ in 0..self.ladder_tuning_steps {
                    step(
                        &mut tune_field,
                        flat_model,
                        &temps,
                        j_tau,
                        is_clamped,
                        &mut tune_scratch,
                        &mut rng,
                    );
                }
                match self.ladder_mode {
                    LadderMode::AcceptanceUniform => {
                        // Acceptance uniformization (Rathore/Kofke class).
                        let rates = tune_scratch.swap_acceptance_rates();
                        let spacings: Vec<f64> = alphas.windows(2).map(|w| w[1] - w[0]).collect();
                        let new_spacings =
                            crate::solver::population_annealing::adapt_spacings(&spacings, &rates);
                        let mut acc = 0.0;
                        for (t, d) in new_spacings.iter().enumerate() {
                            acc += d;
                            alphas[t + 1] = acc;
                        }
                        alphas[nt - 1] = 1.0;
                    }
                    LadderMode::FeedbackOptimized => {
                        // True KTHT feedback optimization on replica flow f(T).
                        // Safety guard: the flow estimate needs enough round
                        // trips to be reliable — with too few, the histogram
                        // is a near-step-function and redistribution would
                        // pathologically collapse the ladder (verified:
                        // ~10 tuning steps → −99% round-trip rate, ~300 →
                        // +43%). Skip the update when the sample is too thin;
                        // the ladder then stays at its previous (worst-case
                        // geometric) profile rather than degrading.
                        if tune_scratch.roundtrip_count() >= (nt as u64) {
                            let flow = tune_scratch.roundtrip_flow();
                            alphas = crate::solver::population_annealing::feedback_optimized_alphas(
                                &alphas, &flow, 0.5,
                            );
                            alphas[0] = 0.0;
                            alphas[nt - 1] = 1.0;
                        }
                    }
                }
            }
            // The tuned α-profile is now FROZEN for the production run.
        }

        let mut field = QuantumField::new(n, self.num_slices, nt, self.num_pops);
        self.init_field_states(&mut field, is_clamped, clamped_val, &mut rng);
        for p in 0..self.num_pops {
            for t in 0..nt {
                field.energies[t + nt * p] =
                    calculate_replica_energies(flat_model, &field, t, p, j_tau);
            }
        }

        let k_stages = self.total_exchanges.max(1);
        let cold_factor = if k_stages > 1 {
            (self.temp_min / self.temp_max).powf(1.0 / (k_stages - 1) as f64)
        } else {
            1.0
        };

        let mut scratch = StepScratch::for_field(&field);
        let mut log_w = vec![0.0f64; r_total];
        let mut temps = vec![0.0f64; nt];
        let mut beta_cold_prev: Option<f64> = None;
        let mut incumbent_energy = f64::INFINITY;
        let mut incumbent = vec![0i8; n];
        let mut incumbent_buf = vec![0i8; n];

        for k in 0..k_stages {
            let t_cold = if k_stages > 1 {
                self.temp_max * cold_factor.powi(k as i32)
            } else {
                self.temp_min
            };
            // Stage ladder from the frozen α-profile over [t_cold, temp_max]:
            // T_t = temp_max · (t_cold/temp_max)^α_t. Uniform α reproduces
            // the geometric ladder exactly; the tuned profile equalizes
            // per-pair swap acceptance across every stage's span.
            let span = t_cold / self.temp_max;
            for (t, slot) in temps.iter_mut().enumerate() {
                *slot = self.temp_max * span.powf(alphas[t]);
            }
            let beta_cold = 1.0 / t_cold;

            // PA reweighting for the completed β_cold step (skipped at the
            // first stage: there is no previous ensemble to reweight from).
            if let Some(beta_prev) = beta_cold_prev {
                let delta_beta = beta_cold - beta_prev;
                let cold = nt - 1;
                for p in 0..self.num_pops {
                    for (r, lw) in log_w[p * NUM_REPLICAS..(p + 1) * NUM_REPLICAS]
                        .iter_mut()
                        .enumerate()
                    {
                        *lw -= delta_beta * field.energies[cold + nt * p][r];
                    }
                }
                let w = pa::normalized_weights(&log_w);
                let ess = pa::effective_sample_size(&w);
                diagnostics.ess_history.push(ess);
                if ess < self.ess_threshold * r_total as f64 {
                    // Free-energy increment must be accumulated from the
                    // log-weights BEFORE they are reset (Machta 2010).
                    diagnostics.free_energy += pa::free_energy_increment(&log_w);
                    let u: f64 = rng.gen_range(0.0..1.0);
                    let counts = pa::systematic_resample(&w, u);
                    let parents = pa::apply_resample(&mut field, &counts, &mut rng);
                    // Propagate ancestral family IDs through the resampling.
                    let old_family = family_ids.clone();
                    for (dst, &src) in parents.iter().enumerate() {
                        family_ids[dst] = old_family[src];
                    }
                    diagnostics
                        .family_entropy_history
                        .push(pa::family_entropy(&family_ids));
                    diagnostics.resample_events += 1;
                    for lw in log_w.iter_mut() {
                        *lw = 0.0;
                    }
                    // Post-resampling PT relaxation: extra full-ladder PT
                    // steps decorrelate the freshly cloned members and
                    // restore equilibrium before the next reweighting
                    // (Wang, Machta & Katzgraber, PRE 92, 063307 (2015)).
                    for _ in 0..self.post_resample_relaxation {
                        step(
                            &mut field,
                            flat_model,
                            &temps,
                            j_tau,
                            is_clamped,
                            &mut scratch,
                            &mut rng,
                        );
                    }
                }
            }
            beta_cold_prev = Some(beta_cold);

            // Stage equilibration: full PT over the stage ladder.
            for _ in 0..self.sweeps_per_exchange {
                step(
                    &mut field,
                    flat_model,
                    &temps,
                    j_tau,
                    is_clamped,
                    &mut scratch,
                    &mut rng,
                );
            }
            // Exact re-sync of every level: the next reweighting reads the
            // cold level, and this also bounds float drift (replacing the
            // PT path's periodic re-sync).
            for p in 0..self.num_pops {
                for t in 0..nt {
                    field.energies[t + nt * p] =
                        calculate_replica_energies(flat_model, &field, t, p, j_tau);
                }
            }
            if pa_icm_enabled {
                crate::solver::icm::icm_sweep(&mut field, flat_model, &mut rng);
                for p in 0..self.num_pops {
                    for t in 0..nt {
                        field.energies[t + nt * p] =
                            calculate_replica_energies(flat_model, &field, t, p, j_tau);
                    }
                }
            }
            // Anytime hook: capture the stage's best BEFORE the next stage's
            // resampling can destroy it.
            Self::update_incumbent(
                model,
                &field,
                &mut incumbent_energy,
                &mut incumbent,
                &mut incumbent_buf,
            );
        }

        // Extract the best member state by true model energy.
        let mut best_state = vec![0i8; n];
        let mut best_energy = f64::INFINITY;
        let mut state_buf = vec![0i8; n];
        for p in 0..self.num_pops {
            for t in 0..nt {
                for s in 0..self.num_slices {
                    for r in 0..NUM_REPLICAS {
                        for (v, slot) in state_buf.iter_mut().enumerate() {
                            *slot = field.get_replica(v, s, t, p, r);
                        }
                        let energy = model.calculate_total_energy(&state_buf);
                        if energy < best_energy {
                            best_energy = energy;
                            best_state.copy_from_slice(&state_buf);
                        }
                    }
                }
            }
        }
        if incumbent_energy < best_energy {
            best_state.copy_from_slice(&incumbent);
        }
        crate::solver::local_search::steepest_descent_1opt(model, &mut best_state, is_clamped);
        (best_state, diagnostics)
    }

    /// Runs the solver in PA-PT mode (forces num_pops ≥ 2) and returns the
    /// solution together with Population-Annealing diagnostics. Deterministic
    /// given a seed. Presolve/decomposition are bypassed so the diagnostics
    /// describe the full-model annealing run.
    pub fn solve_with_pa_diagnostics(
        &self,
        model: &crate::core::QuboModel,
        clamped: &[(usize, i8)],
    ) -> (Vec<i8>, crate::solver::population_annealing::PaDiagnostics) {
        let mut hubo_model = HuboModel::new(model.num_vars);
        hubo_model.linear = model.linear.clone();
        for i in 0..model.num_vars {
            for (j, weight) in model.quadratic.get_row(i) {
                hubo_model.edges2[i].push(crate::core::hubo::Edge2 { j, weight });
            }
        }
        let flat_model = FlatHuboModel::from_hubo(&hubo_model);
        let base_seed = self.seed.unwrap_or_else(rand::random);
        let n = model.num_vars;
        let mut is_clamped = vec![false; n];
        let mut clamped_val = vec![0i8; n];
        for &(idx, val) in clamped {
            is_clamped[idx] = true;
            clamped_val[idx] = val;
        }
        let pops = self.num_pops.max(2);
        let solver = UltimateSolver {
            num_pops: pops,
            ..self.clone()
        };
        solver.solve_population_annealing_diag(
            model,
            &flat_model,
            &is_clamped,
            &clamped_val,
            base_seed,
        )
    }
}
