//! Benchmark executable running EXP-TEN-002: Hard Falsification of Model C.

use fundamental_ai::baselines::{
    ModernHopfield, NearestNeighborOracle, PolynomialDAM, PseudoinverseHopfield,
};
use fundamental_ai::datasets::{
    generate_adversarial_pairs_suite, generate_biased_suite, generate_clustered_suite,
    generate_correlated_suite, generate_low_rank_suite, generate_random_suite,
};
use fundamental_ai::dynamics::greedy_descent;
use fundamental_ai::experiment::corrupt_pattern;
use fundamental_ai::learning::{train_low_rank_cp, train_pairwise_hebbian};
use fundamental_ai::models::EnergyModel;
use fundamental_ai::types::SpinState;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

struct SingleResult {
    suite: String,
    n: usize,
    p: usize,
    noise: f64,
    model: String,
    seed: u64,
    exact_acc: f64,
    mean_ber: f64,
    wall_time_us: f64,
}

fn main() {
    println!("================================================================================");
    println!("EXP-TEN-002: Hard Adversarial Falsification of Model C vs Modern Baselines");
    println!("Suites: Random, Correlated, Clustered, Low-Rank, Adversarial Pairs, Biased");
    println!("================================================================================\n");

    let total_start = Instant::now();
    let n = 128;
    let p_values = [32, 64, 128];
    let noise_levels = [0.10, 0.20, 0.30, 0.40];
    let seeds: Vec<u64> = (2001..=2020).collect();

    let suites = [
        "Suite_A_Random",
        "Suite_B_Correlated",
        "Suite_C_Clustered",
        "Suite_D_LowRank",
        "Suite_E_Adversarial",
        "Suite_F_Biased",
    ];

    let mut tsv_file = File::create("research/fundamental_ai/EXP_TEN_002_RAW.tsv")
        .expect("Failed to create TSV file");
    writeln!(
        tsv_file,
        "suite\tn\tp\tnoise\tmodel\tseed\texact_acc\tmean_ber\twall_time_us"
    )
    .unwrap();

    for &suite in &suites {
        println!(">>> Running Data Suite: {}", suite);

        for &p in &p_values {
            println!("  > Memory Capacity P = {} (N = {})", p, n);

            for &noise in &noise_levels {
                print!("    Noise = {:.0}%: ", noise * 100.0);

                let seed_results: Vec<Vec<SingleResult>> = seeds
                    .par_iter()
                    .map(|&seed| {
                        let patterns: Vec<SpinState> = match suite {
                            "Suite_A_Random" => generate_random_suite(n, p, seed),
                            "Suite_B_Correlated" => generate_correlated_suite(n, p, 0.6, seed),
                            "Suite_C_Clustered" => generate_clustered_suite(n, p, 4, 0.15, seed),
                            "Suite_D_LowRank" => generate_low_rank_suite(n, p, 6, seed),
                            "Suite_E_Adversarial" => generate_adversarial_pairs_suite(n, p, seed),
                            "Suite_F_Biased" => generate_biased_suite(n, p, 0.4, seed),
                            _ => unreachable!(),
                        };

                        let mut results = Vec::new();
                        let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(8888));

                        // 1. Baseline B0: Pairwise Hopfield
                        let b0 = train_pairwise_hebbian(&patterns);
                        let (acc0, ber0, time0) =
                            eval_energy_model(&b0, &patterns, noise, &mut rng);
                        results.push(SingleResult {
                            suite: suite.to_string(),
                            n,
                            p,
                            noise,
                            model: "B0_Hebbian".to_string(),
                            seed,
                            exact_acc: acc0,
                            mean_ber: ber0,
                            wall_time_us: time0,
                        });

                        // 2. Baseline B1: Pseudoinverse Hopfield
                        if let Ok(b1) = PseudoinverseHopfield::train(&patterns) {
                            let (acc1, ber1, time1) =
                                eval_energy_model(&b1, &patterns, noise, &mut rng);
                            results.push(SingleResult {
                                suite: suite.to_string(),
                                n,
                                p,
                                noise,
                                model: "B1_Pseudoinverse".to_string(),
                                seed,
                                exact_acc: acc1,
                                mean_ber: ber1,
                                wall_time_us: time1,
                            });
                        }

                        // 3. Baseline B2: 1-NN Hamming Oracle
                        let b2 = NearestNeighborOracle::new(&patterns);
                        let mut matches2 = 0;
                        let mut total_ber2 = 0.0;
                        let t2_start = Instant::now();
                        for pat in &patterns {
                            let probe = corrupt_pattern(pat, noise, &mut rng);
                            let (retrieved, _, _) = b2.retrieve(&probe);
                            if retrieved == *pat {
                                matches2 += 1;
                            }
                            total_ber2 += retrieved.bit_error_rate(pat);
                        }
                        let time2 = (t2_start.elapsed().as_micros() as f64) / (p as f64);
                        results.push(SingleResult {
                            suite: suite.to_string(),
                            n,
                            p,
                            noise,
                            model: "B2_1NN_Oracle".to_string(),
                            seed,
                            exact_acc: (matches2 as f64) / (p as f64),
                            mean_ber: total_ber2 / (p as f64),
                            wall_time_us: time2,
                        });

                        // 4. Baseline B3: Exact Polynomial DAM (n=3)
                        let b3 = PolynomialDAM::new(&patterns);
                        let (acc3, ber3, time3) =
                            eval_energy_model(&b3, &patterns, noise, &mut rng);
                        results.push(SingleResult {
                            suite: suite.to_string(),
                            n,
                            p,
                            noise,
                            model: "B3_PolynomialDAM".to_string(),
                            seed,
                            exact_acc: acc3,
                            mean_ber: ber3,
                            wall_time_us: time3,
                        });

                        // 5. Baseline B4: Modern Hopfield (Softmax Attention)
                        let b4 = ModernHopfield::new(&patterns, 1.5);
                        let mut matches4 = 0;
                        let mut total_ber4 = 0.0;
                        let t4_start = Instant::now();
                        for pat in &patterns {
                            let probe = corrupt_pattern(pat, noise, &mut rng);
                            let retrieved = b4.retrieve(&probe, 15);
                            if retrieved == *pat {
                                matches4 += 1;
                            }
                            total_ber4 += retrieved.bit_error_rate(pat);
                        }
                        let time4 = (t4_start.elapsed().as_micros() as f64) / (p as f64);
                        results.push(SingleResult {
                            suite: suite.to_string(),
                            n,
                            p,
                            noise,
                            model: "B4_ModernHopfield".to_string(),
                            seed,
                            exact_acc: (matches4 as f64) / (p as f64),
                            mean_ber: total_ber4 / (p as f64),
                            wall_time_us: time4,
                        });

                        // 6. Candidate: Model C (LowRankCP 3-Body Tensor Memory)
                        let cand_c = train_low_rank_cp(&patterns);
                        let (acc_c, ber_c, time_c) =
                            eval_energy_model(&cand_c, &patterns, noise, &mut rng);
                        results.push(SingleResult {
                            suite: suite.to_string(),
                            n,
                            p,
                            noise,
                            model: "Candidate_ModelC".to_string(),
                            seed,
                            exact_acc: acc_c,
                            mean_ber: ber_c,
                            wall_time_us: time_c,
                        });

                        results
                    })
                    .collect();

                // Aggregate across seeds
                let mut acc_map: std::collections::BTreeMap<String, f64> =
                    std::collections::BTreeMap::new();
                let num_seeds = seeds.len() as f64;

                for seed_res in seed_results {
                    for r in seed_res {
                        writeln!(
                            tsv_file,
                            "{}\t{}\t{}\t{:.2}\t{}\t{}\t{:.6}\t{:.6}\t{:.2}",
                            r.suite,
                            r.n,
                            r.p,
                            r.noise,
                            r.model,
                            r.seed,
                            r.exact_acc,
                            r.mean_ber,
                            r.wall_time_us
                        )
                        .unwrap();

                        *acc_map.entry(r.model).or_insert(0.0) += r.exact_acc;
                    }
                }

                let mut summary_strs = Vec::new();
                for (model, sum_acc) in acc_map {
                    summary_strs.push(format!("{}:{:.1}%", model, sum_acc / num_seeds * 100.0));
                }
                println!("{}", summary_strs.join(" | "));
            }
        }
        println!();
    }

    let elapsed = total_start.elapsed();
    println!("================================================================================");
    println!(
        "EXP-TEN-002 Completed in {:.2} seconds.",
        elapsed.as_secs_f64()
    );
    println!("Raw results saved to: research/fundamental_ai/EXP_TEN_002_RAW.tsv");
    println!("================================================================================");
}

fn eval_energy_model(
    model: &dyn EnergyModel,
    patterns: &[SpinState],
    noise: f64,
    rng: &mut ChaCha8Rng,
) -> (f64, f64, f64) {
    let p = patterns.len();
    let mut exact_matches = 0;
    let mut total_ber = 0.0;
    let mut total_time_us = 0;

    for pat in patterns {
        let probe = corrupt_pattern(pat, noise, rng);
        let res = greedy_descent(model, &probe, 30);
        if res.final_state == *pat {
            exact_matches += 1;
        }
        total_ber += res.final_state.bit_error_rate(pat);
        total_time_us += res.wall_time_us;
    }

    (
        (exact_matches as f64) / (p as f64),
        total_ber / (p as f64),
        (total_time_us as f64) / (p as f64),
    )
}
