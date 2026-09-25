//! Benchmark executable running EXP-TEN-001 across the full preregistered matrix.

use fundamental_ai::experiment::{
    evaluate_model_on_patterns, generate_random_patterns, ModelConditionResult,
};
use fundamental_ai::learning::{
    train_low_rank_cp, train_pairwise_hebbian, train_sparse_hyperedge3_budgeted,
    train_sparse_hyperedge4_budgeted,
};
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

fn main() {
    println!("================================================================================");
    println!("EXP-TEN-001: Associative Binary Pattern Completion Under Noise Corruption");
    println!("Fundamental AI Research: Higher-Order Tensor Energy Dynamics vs Pairwise Hopfield");
    println!("================================================================================\n");

    let total_start = Instant::now();
    let n_values = [128, 256, 512];
    let p_ratios = [0.10, 0.25, 0.50, 1.00];
    let noise_levels = [0.10, 0.20, 0.30, 0.40];
    let seeds: Vec<u64> = (1001..=1020).collect();

    // Open TSV file for streaming raw results
    let mut tsv_file = File::create("research/fundamental_ai/EXP_TEN_001_RAW.tsv")
        .expect("Failed to create raw TSV file");
    writeln!(
        tsv_file,
        "n\tp\tnoise\tmodel\tseed\tparameters\texact_recovery_rate\tmean_ber\tmean_sweeps\tmean_flips\tmean_wall_time_us\tcapacity_per_parameter"
    )
    .unwrap();

    for &n in &n_values {
        let pairwise_budget = n * (n - 1) / 2;
        println!(
            ">>> Running System Size N = {} (Pairwise Budget: {} parameters)",
            n, pairwise_budget
        );

        for &p_ratio in &p_ratios {
            let p = ((n as f64) * p_ratio).round() as usize;
            println!("  > Pattern Load P/N = {:.2} (P = {} patterns)", p_ratio, p);

            for &noise in &noise_levels {
                print!("    Noise = {:.0}%: ", noise * 100.0);

                // Run seeds in parallel
                let seed_results: Vec<Vec<ModelConditionResult>> = seeds
                    .par_iter()
                    .map(|&seed| {
                        let patterns = generate_random_patterns(n, p, seed);
                        let mut local_res = Vec::new();

                        // 1. Model A: Pairwise Hopfield
                        let model_a = train_pairwise_hebbian(&patterns);
                        let res_a = evaluate_model_on_patterns(
                            &model_a,
                            "Model_A_Pairwise",
                            &patterns,
                            noise,
                            seed,
                            50,
                        );
                        local_res.push(res_a);

                        // 2. Model B: 3-Body Sparse Tensor (budget matched)
                        let model_b = train_sparse_hyperedge3_budgeted(
                            &patterns,
                            pairwise_budget,
                            seed.wrapping_add(100),
                        );
                        let res_b = evaluate_model_on_patterns(
                            &model_b,
                            "Model_B_Sparse_3Body",
                            &patterns,
                            noise,
                            seed,
                            50,
                        );
                        local_res.push(res_b);

                        // 3. Model C: Low-Rank CP 3-Body Tensor (rank = P)
                        let model_c = train_low_rank_cp(&patterns);
                        let res_c = evaluate_model_on_patterns(
                            &model_c,
                            "Model_C_LowRank_CP",
                            &patterns,
                            noise,
                            seed,
                            50,
                        );
                        local_res.push(res_c);

                        // 4. Model D: 4-Body Sparse Tensor (budget matched)
                        let model_d = train_sparse_hyperedge4_budgeted(
                            &patterns,
                            pairwise_budget,
                            seed.wrapping_add(200),
                        );
                        let res_d = evaluate_model_on_patterns(
                            &model_d,
                            "Model_D_Sparse_4Body",
                            &patterns,
                            noise,
                            seed,
                            50,
                        );
                        local_res.push(res_d);

                        local_res
                    })
                    .collect();

                // Aggregate across seeds
                let mut sum_a = (0.0, 0.0, 0.0); // exact_rate, ber, capacity_per_param
                let mut sum_b = (0.0, 0.0, 0.0);
                let mut sum_c = (0.0, 0.0, 0.0);
                let mut sum_d = (0.0, 0.0, 0.0);
                let num_seeds = seeds.len() as f64;

                for (idx, seed_group) in seed_results.iter().enumerate() {
                    let seed = seeds[idx];
                    for res in seed_group {
                        writeln!(
                            tsv_file,
                            "{}\t{}\t{:.2}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.2}\t{:.2}\t{:.2}\t{:.6}",
                            res.n,
                            res.p,
                            res.noise,
                            res.model_name,
                            seed,
                            res.num_parameters,
                            res.exact_recovery_rate,
                            res.mean_ber,
                            res.mean_sweeps,
                            res.mean_flips,
                            res.mean_wall_time_us,
                            res.capacity_per_parameter
                        )
                        .unwrap();

                        match res.model_name.as_str() {
                            "Model_A_Pairwise" => {
                                sum_a.0 += res.exact_recovery_rate;
                                sum_a.1 += res.mean_ber;
                                sum_a.2 += res.capacity_per_parameter;
                            }
                            "Model_B_Sparse_3Body" => {
                                sum_b.0 += res.exact_recovery_rate;
                                sum_b.1 += res.mean_ber;
                                sum_b.2 += res.capacity_per_parameter;
                            }
                            "Model_C_LowRank_CP" => {
                                sum_c.0 += res.exact_recovery_rate;
                                sum_c.1 += res.mean_ber;
                                sum_c.2 += res.capacity_per_parameter;
                            }
                            "Model_D_Sparse_4Body" => {
                                sum_d.0 += res.exact_recovery_rate;
                                sum_d.1 += res.mean_ber;
                                sum_d.2 += res.capacity_per_parameter;
                            }
                            _ => {}
                        }
                    }
                }

                println!(
                    "Exact Acc: M_A={:.1}%, M_B={:.1}%, M_C={:.1}%, M_D={:.1}% | Cap/Param: M_A={:.3}, M_B={:.3}, M_C={:.3}, M_D={:.3}",
                    sum_a.0 / num_seeds * 100.0,
                    sum_b.0 / num_seeds * 100.0,
                    sum_c.0 / num_seeds * 100.0,
                    sum_d.0 / num_seeds * 100.0,
                    sum_a.2 / num_seeds,
                    sum_b.2 / num_seeds,
                    sum_c.2 / num_seeds,
                    sum_d.2 / num_seeds,
                );
            }
        }
        println!();
    }

    let elapsed = total_start.elapsed();
    println!("================================================================================");
    println!(
        "EXP-TEN-001 Completed in {:.2} seconds.",
        elapsed.as_secs_f64()
    );
    println!("Raw results saved to: research/fundamental_ai/EXP_TEN_001_RAW.tsv");
    println!("================================================================================");
}
