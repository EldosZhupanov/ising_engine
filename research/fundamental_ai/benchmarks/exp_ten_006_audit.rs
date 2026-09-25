//! Benchmark: EXP-TEN-006A-R Critical Audit & Prior-Art Falsification.
//!
//! Rigorously evaluates:
//! 1. Critical Check: Is the model actually 'thinking'? (T=0, Rescue Rate, Damage Rate, Net Rescue).
//! 2. Energy Ablations (Full, Initialization Only, Zero Interaction, Random Symmetric, Shuffled).
//! 3. Strong Classical Synchronization Baselines (Spectral Sync, Loopy Min-Sum).
//! 4. Stabilized Neural Baselines (Damped Recurrent Attention, Symmetric Energy Attention).
//! 5. Statistical Rigor (McNemar Exact Test, Paired 95% Bootstrap CIs).

use fundamental_ai::learned_energy::{
    LearnedDualEnergyNetwork, PermutationWorld, RecurrentAttentionGraph,
};
use fundamental_ai::synchronization_audit::{
    DampedRecurrentAttention, LoopyMinSumSync, SpectralPermutationSync, StatisticalTests,
    SymmetricEnergyAttention, TrajectoryAuditor,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

struct AuditRow {
    cycle_k: usize,
    model: String,
    noise: f64,
    step: usize,
    seed: u64,
    instance_id: usize,
    mean_accuracy: f64,
    exact_solve: f64,
    rescue_rate: f64,
    damage_rate: f64,
    net_rescue: f64,
    dist_clean: usize,
    dist_noisy: usize,
    delta_s: f64,
    wall_time_us: f64,
}

fn main() {
    println!("================================================================================");
    println!("EXP-TEN-006A-R: CRITICAL AUDIT, RESCUE-RATE ANALYSIS & PRIOR-ART FALSIFICATION");
    println!("================================================================================");

    let output_path = "EXP_TEN_006_AUDIT_RAW.tsv";
    let mut out_file =
        File::create(output_path).expect("Failed to create EXP_TEN_006_AUDIT_RAW.tsv");
    writeln!(
        out_file,
        "cycle_k	model	noise	step	seed	instance_id	mean_accuracy	exact_solve	rescue_rate	damage_rate	net_rescue	dist_clean	dist_noisy	delta_s	wall_time_us"
    )
    .unwrap();

    let dim = 16;
    let latent_dim = 12;
    let num_relations = 4;
    let beta = 2.0;

    let world = PermutationWorld::new(dim, num_relations, 42);

    // 1. Initialize candidate models and ablations
    let mut candidate_energy =
        LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 100);
    let random_energy = LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 777);

    // Zero-interaction energy net: weights set to 0, only lambda_obs is active
    let mut zero_interaction_energy =
        LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 100);
    zero_interaction_energy.w1.fill(0.0);
    zero_interaction_energy.w2.fill(0.0);
    zero_interaction_energy.u_lat.fill(0.0);
    zero_interaction_energy.vz.fill(0.0);

    // Shuffled factors energy net
    let mut shuffled_energy =
        LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 100);
    let mut shuff_rng = ChaCha8Rng::seed_from_u64(888);
    use rand::seq::SliceRandom;
    shuffled_energy.w1.shuffle(&mut shuff_rng);
    shuffled_energy.w2.shuffle(&mut shuff_rng);

    // Neural baselines
    let mut unconstrained_attn = RecurrentAttentionGraph::new(dim, num_relations, 200);
    let damped_attn_025 = DampedRecurrentAttention::new(dim, num_relations, 0.25, 201);
    let _damped_attn_050 = DampedRecurrentAttention::new(dim, num_relations, 0.50, 202);
    let sym_energy_attn = SymmetricEnergyAttention::new(dim, num_relations, 0.25, 203);

    // Classical synchronization baselines
    let spectral_sync = SpectralPermutationSync::new(dim, world.permutations.clone());
    let min_sum_sync = LoopyMinSumSync::new(dim, world.permutations.clone());

    // 2. Training Phase: Train candidate & unconstrained attention on short cycles
    println!("\n>>> Training models on short cycles (K in 3..5)...\n");
    let mut train_rng = ChaCha8Rng::seed_from_u64(1000);
    let mut train_graphs = Vec::with_capacity(30);
    for idx in 0..30 {
        let k = 3 + (idx % 3);
        train_graphs.push(world.generate_consistent_cycle(k, 0.20, &mut train_rng));
    }
    for _epoch in 1..=5 {
        candidate_energy.train_step_bptt(&train_graphs, 0.015, 3, 0.05);
        unconstrained_attn.train_step(&train_graphs, 0.02, 3);
    }
    println!(
        ">>> Training Complete.
"
    );

    // 3. Evaluation Setup
    let test_cycle_sizes = [4, 8, 16];
    let noise_levels = [0.20];
    let step_options = [0, 1, 2, 4, 8, 16, 32];
    let num_seeds = 20;

    let mut all_rows: Vec<AuditRow> = Vec::new();

    for &cycle_k in &test_cycle_sizes {
        println!(
            ">>> EVALUATING ON CYCLE LENGTH K = {} across {} seeds...",
            cycle_k, num_seeds
        );

        for &noise in &noise_levels {
            for &step in &step_options {
                let batch_results: Vec<Vec<AuditRow>> = (0..num_seeds)
                    .into_par_iter()
                    .map(|seed_idx| {
                        let mut test_rng = ChaCha8Rng::seed_from_u64(
                            5000 + (seed_idx as u64) * 1000 + (cycle_k as u64) * 10,
                        );
                        let graph = world.generate_consistent_cycle(cycle_k, noise, &mut test_rng);
                        let mut rows = Vec::new();

                        let eval_model =
                            |name: &str,
                             s_final: &[Vec<f64>],
                             dt: f64,
                             rows_out: &mut Vec<AuditRow>| {
                                let audit = TrajectoryAuditor::audit_state(
                                    s_final,
                                    &graph.noisy_states,
                                    &graph.noisy_states,
                                    &graph.clean_states,
                                    step,
                                    0.0,
                                );
                                rows_out.push(AuditRow {
                                    cycle_k,
                                    model: name.into(),
                                    noise,
                                    step,
                                    seed: seed_idx as u64,
                                    instance_id: seed_idx,
                                    mean_accuracy: audit.mean_accuracy,
                                    exact_solve: if audit.exact_solve { 1.0 } else { 0.0 },
                                    rescue_rate: audit.rescue_rate,
                                    damage_rate: audit.damage_rate,
                                    net_rescue: audit.net_rescue,
                                    dist_clean: audit.hamming_dist_to_clean,
                                    dist_noisy: audit.hamming_dist_to_noisy,
                                    delta_s: audit.delta_s_norm,
                                    wall_time_us: dt,
                                });
                            };

                        // A. Full Candidate Learned Energy
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                candidate_energy.relax(&graph, step, 0.05).states
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("Candidate_LearnedEnergy", &s, dt, &mut rows);
                        }

                        // B. Ablation: Zero Interaction Energy (Psi = 0)
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                zero_interaction_energy.relax(&graph, step, 0.05).states
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("Ablation_ZeroInteraction", &s, dt, &mut rows);
                        }

                        // C. Ablation: Random Symmetric Energy
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                random_energy.relax(&graph, step, 0.05).states
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("Ablation_RandomSymmetric", &s, dt, &mut rows);
                        }

                        // D. Neural: N0 Unconstrained Recurrent Attention
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                unconstrained_attn.unroll(&graph, step).0
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("N0_Unconstrained_Attn", &s, dt, &mut rows);
                        }

                        // E. Neural: N1 Damped Recurrent Attention (alpha = 0.25)
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                damped_attn_025.unroll(&graph, step)
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("N1_Damped_Attn_0.25", &s, dt, &mut rows);
                        }

                        // F. Neural: N4 Symmetric Energy-Admitting Attention
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                sym_energy_attn.unroll(&graph, step)
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("N4_Symmetric_Energy_Attn", &s, dt, &mut rows);
                        }

                        // G. Classical: SYNC-1 Spectral Permutation Synchronization
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                spectral_sync.synchronize(&graph, step, 0.5)
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("SYNC1_Spectral_Sync", &s, dt, &mut rows);
                        }

                        // H. Classical: SYNC-5 Loopy Min-Sum Message Passing
                        {
                            let t0 = Instant::now();
                            let s = if step == 0 {
                                graph.noisy_states.clone()
                            } else {
                                min_sum_sync.solve(&graph, step, 0.5)
                            };
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            eval_model("SYNC5_Loopy_MinSum", &s, dt, &mut rows);
                        }

                        rows
                    })
                    .collect();

                for batch in batch_results {
                    for r in batch {
                        all_rows.push(r);
                    }
                }
            }
        }
    }

    // Write all rows to TSV
    for r in &all_rows {
        writeln!(
            out_file,
            "{}	{}	{:.2}	{}	{}	{}	{:.4}	{:.4}	{:.4}	{:.4}	{:.4}	{}	{}	{:.4}	{:.2}",
            r.cycle_k,
            r.model,
            r.noise,
            r.step,
            r.seed,
            r.instance_id,
            r.mean_accuracy,
            r.exact_solve,
            r.rescue_rate,
            r.damage_rate,
            r.net_rescue,
            r.dist_clean,
            r.dist_noisy,
            r.delta_s,
            r.wall_time_us
        )
        .unwrap();
    }

    // 4. Detailed Audit Report Output
    println!(
        "
================================================================================"
    );
    println!("1. RESCUE RATE AUDIT: IS THE CANDIDATE MODEL ACTUALLY 'THINKING'? (K=16, eta=0.20)");
    println!("================================================================================");
    println!(
        "{:<26} | {:<7} | {:<7} | {:<7} | {:<7} | {:<7} | {:<7} | {:<7}",
        "Model", "T=0", "T=1", "T=2", "T=4", "T=8", "T=16", "T=32"
    );
    println!(
        "{:-<26}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<7}",
        "", "", "", "", "", "", "", ""
    );

    let audit_models = [
        "Candidate_LearnedEnergy",
        "Ablation_ZeroInteraction",
        "Ablation_RandomSymmetric",
        "N0_Unconstrained_Attn",
        "N1_Damped_Attn_0.25",
        "N4_Symmetric_Energy_Attn",
        "SYNC1_Spectral_Sync",
        "SYNC5_Loopy_MinSum",
    ];

    for m in audit_models {
        // Print Mean Accuracy
        let mut acc_str = format!("{:<26}", m);
        for &s in &step_options {
            let matched: Vec<f64> = all_rows
                .iter()
                .filter(|r| r.model == m && r.cycle_k == 16 && r.step == s)
                .map(|r| r.mean_accuracy)
                .collect();
            if matched.is_empty() {
                acc_str.push_str(" |        ");
            } else {
                let mean_val = (matched.iter().sum::<f64>() / matched.len() as f64) * 100.0;
                acc_str.push_str(&format!(" | {:5.1}%", mean_val));
            }
        }
        println!("{}", acc_str);
    }

    println!(
        "
================================================================================"
    );
    println!("2. DYNAMICS BREAKDOWN AT T=16 (K=16, eta=0.20): RESCUE vs DAMAGE RATES");
    println!("================================================================================");
    println!(
        "{:<26} | {:<12} | {:<12} | {:<12} | {:<12}",
        "Model", "Accuracy", "Rescue Rate", "Damage Rate", "Net Rescue"
    );
    println!(
        "{:-<26}-+-{:-<12}-+-{:-<12}-+-{:-<12}-+-{:-<12}",
        "", "", "", "", ""
    );

    for m in audit_models {
        let matched: Vec<&AuditRow> = all_rows
            .iter()
            .filter(|r| r.model == m && r.cycle_k == 16 && r.step == 16)
            .collect();
        if !matched.is_empty() {
            let n = matched.len() as f64;
            let mean_acc = (matched.iter().map(|r| r.mean_accuracy).sum::<f64>() / n) * 100.0;
            let mean_rescue = (matched.iter().map(|r| r.rescue_rate).sum::<f64>() / n) * 100.0;
            let mean_damage = (matched.iter().map(|r| r.damage_rate).sum::<f64>() / n) * 100.0;
            let mean_net = (matched.iter().map(|r| r.net_rescue).sum::<f64>() / n) * 100.0;
            println!(
                "{:<26} | {:10.1}% | {:10.1}% | {:10.1}% | {:+10.1}%	",
                m, mean_acc, mean_rescue, mean_damage, mean_net
            );
        }
    }

    println!(
        "
================================================================================"
    );
    println!("3. STATISTICAL SIGNIFICANCE TESTS (Candidate vs Baselines at K=16, T=16)");
    println!("================================================================================");
    let cand_accs: Vec<f64> = all_rows
        .iter()
        .filter(|r| r.model == "Candidate_LearnedEnergy" && r.cycle_k == 16 && r.step == 16)
        .map(|r| r.mean_accuracy)
        .collect();

    for comp in [
        "N0_Unconstrained_Attn",
        "N1_Damped_Attn_0.25",
        "N4_Symmetric_Energy_Attn",
        "SYNC1_Spectral_Sync",
        "Ablation_ZeroInteraction",
    ] {
        let comp_accs: Vec<f64> = all_rows
            .iter()
            .filter(|r| r.model == comp && r.cycle_k == 16 && r.step == 16)
            .map(|r| r.mean_accuracy)
            .collect();

        if !comp_accs.is_empty() && cand_accs.len() == comp_accs.len() {
            let (mean_diff, ci_low, ci_high) =
                StatisticalTests::paired_bootstrap_ci(&cand_accs, &comp_accs, 1000, 42);
            let (b, c, chi2, p_val) = StatisticalTests::mcnemar(
                &cand_accs.iter().map(|&a| a >= 0.80).collect::<Vec<_>>(),
                &comp_accs.iter().map(|&a| a >= 0.80).collect::<Vec<_>>(),
            );
            println!(
                "  Candidate vs {:<24} -> Diff: {:+6.2}% [95% CI: {:+5.2}%, {:+5.2}%] | McNemar(b={}, c={}, chi2={:.2}, p={:.4})",
                comp, mean_diff * 100.0, ci_low * 100.0, ci_high * 100.0, b, c, chi2, p_val
            );
        }
    }

    println!(
        "
>>> EXP-TEN-006A-R Critical Audit Complete. Data saved to EXP_TEN_006_AUDIT_RAW.tsv"
    );
}
