//! Benchmark executable running EXP-TEN-003: True Latent Compression (P >> R).
//! Investigates whether low-rank CP-3 tensor factors capture a generative grammar,
//! evaluating factor specialization, factor entropy, and unseen pattern reconstruction.

#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]

use fundamental_ai::baselines::{ModernHopfield, NearestNeighborOracle};
use fundamental_ai::datasets::generate_compositional_grammar_suite;
use fundamental_ai::dynamics::greedy_descent;
use fundamental_ai::experiment::corrupt_pattern;
use fundamental_ai::learning::{
    compute_factor_entropy, compute_factor_max_similarity, compute_factor_participation_ratios,
    train_low_rank_cp_svd, train_low_rank_cp_tensor_power, train_pairwise_low_rank_svd,
    LinearSVDSubspace,
};
use fundamental_ai::models::EnergyModel;
use fundamental_ai::types::SpinState;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

struct SingleResult {
    n: usize,
    k: usize,
    p_train: usize,
    p_unseen: usize,
    rank: usize,
    noise: f64,
    model: String,
    seed: u64,
    seen_exact_acc: f64,
    seen_cosine: f64,
    unseen_exact_acc: f64,
    unseen_cosine: f64,
    max_sim: f64,
    part_ratio: f64,
    factor_entropy: f64,
    wall_time_us: f64,
}

fn evaluate_model_reconstruction<M: EnergyModel>(
    model: &M,
    patterns: &[SpinState],
    noise: f64,
    seed: u64,
    max_steps: usize,
) -> (f64, f64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut exact_count = 0;
    let mut total_cosine = 0.0;
    let n = patterns[0].len();

    for pat in patterns {
        let noisy = corrupt_pattern(pat, noise, &mut rng);
        let res = greedy_descent(model, &noisy, max_steps);

        if res.final_state.hamming_distance(pat) == 0 {
            exact_count += 1;
        }

        let mut dot = 0.0;
        for i in 0..n {
            dot += (res.final_state.get(i) as f64) * (pat.get(i) as f64);
        }
        total_cosine += dot / (n as f64);
    }

    let exact_acc = (exact_count as f64) / (patterns.len() as f64);
    let mean_cos = total_cosine / (patterns.len() as f64);
    (exact_acc, mean_cos)
}

fn main() {
    println!("================================================================================");
    println!("EXP-TEN-003: True Latent Factorization (P >> R) & Generative Generalization");
    println!("Investigating Latent CP-3 Factors, Factor Entropy, and Unseen Grammar Recall");
    println!("================================================================================\n");

    let total_start = Instant::now();
    let n = 128;
    let k = 9; // 2^(9-1) = 256 valid grammatical patterns
    let p_train_values = [128, 192];
    let ranks = [4, 8, 16, 32];
    let noise_levels = [0.15, 0.30];
    let seeds: Vec<u64> = (3001..=3020).collect();

    let mut tsv_file = File::create("research/fundamental_ai/EXP_TEN_003_RAW.tsv")
        .expect("Failed to create TSV file");
    writeln!(
        tsv_file,
        "n\tk\tp_train\tp_unseen\trank\tnoise\tmodel\tseed\tseen_exact_acc\tseen_cosine\tunseen_exact_acc\tunseen_cosine\tmax_sim\tpart_ratio\tfactor_entropy\twall_time_us"
    )
    .unwrap();

    let mut all_results = Vec::new();

    for &p_train in &p_train_values {
        let total_valid = 1usize << (k - 1);
        let p_unseen = total_valid - p_train;
        println!(
            "\n>>> Evaluating P_train = {} / {} (Held-out Unseen = {})",
            p_train, total_valid, p_unseen
        );

        for &rank in &ranks {
            println!(
                "\n  --- Latent Rank R = {} (Compression Ratio R/P = {:.3}) ---",
                rank,
                (rank as f64) / (p_train as f64)
            );

            for &noise in &noise_levels {
                print!("    Noise = {:.0}%: ", noise * 100.0);

                let seed_results: Vec<Vec<SingleResult>> = seeds
                    .par_iter()
                    .map(|&seed| {
                        let mut local_res = Vec::new();
                        let (train_pats, unseen_pats, _) =
                            generate_compositional_grammar_suite(n, k, p_train, seed);

                        // 1. Candidate: Latent CP-3 Memory (SVD initialization)
                        let t0 = Instant::now();
                        let model_cp3_svd = train_low_rank_cp_svd(&train_pats, rank);
                        let max_sims = compute_factor_max_similarity(&model_cp3_svd, &train_pats);
                        let mean_max_sim = max_sims.iter().sum::<f64>() / (max_sims.len() as f64);
                        let prs = compute_factor_participation_ratios(&model_cp3_svd, &train_pats);
                        let mean_pr = prs.iter().sum::<f64>() / (prs.len() as f64);
                        let f_entropy = compute_factor_entropy(&model_cp3_svd, &train_pats);

                        let (seen_acc, seen_cos) = evaluate_model_reconstruction(
                            &model_cp3_svd,
                            &train_pats,
                            noise,
                            seed.wrapping_add(1),
                            25,
                        );
                        let (unseen_acc, unseen_cos) = evaluate_model_reconstruction(
                            &model_cp3_svd,
                            &unseen_pats,
                            noise,
                            seed.wrapping_add(2),
                            25,
                        );
                        let time_cp3 = t0.elapsed().as_micros() as f64;
                        local_res.push(SingleResult {
                            n,
                            k,
                            p_train,
                            p_unseen,
                            rank,
                            noise,
                            model: "Candidate_LatentCP3_SVD".to_string(),
                            seed,
                            seen_exact_acc: seen_acc,
                            seen_cosine: seen_cos,
                            unseen_exact_acc: unseen_acc,
                            unseen_cosine: unseen_cos,
                            max_sim: mean_max_sim,
                            part_ratio: mean_pr,
                            factor_entropy: f_entropy,
                            wall_time_us: time_cp3,
                        });

                        // 2. Candidate: Latent CP-3 Memory (Tensor Power Method)
                        let t0 = Instant::now();
                        let model_cp3_tp = train_low_rank_cp_tensor_power(
                            &train_pats,
                            rank,
                            15,
                            seed.wrapping_add(10),
                        );
                        let max_sims_tp = compute_factor_max_similarity(&model_cp3_tp, &train_pats);
                        let mean_max_sim_tp =
                            max_sims_tp.iter().sum::<f64>() / (max_sims_tp.len() as f64);
                        let prs_tp =
                            compute_factor_participation_ratios(&model_cp3_tp, &train_pats);
                        let mean_pr_tp = prs_tp.iter().sum::<f64>() / (prs_tp.len() as f64);
                        let f_entropy_tp = compute_factor_entropy(&model_cp3_tp, &train_pats);

                        let (seen_acc, seen_cos) = evaluate_model_reconstruction(
                            &model_cp3_tp,
                            &train_pats,
                            noise,
                            seed.wrapping_add(1),
                            25,
                        );
                        let (unseen_acc, unseen_cos) = evaluate_model_reconstruction(
                            &model_cp3_tp,
                            &unseen_pats,
                            noise,
                            seed.wrapping_add(2),
                            25,
                        );
                        let time_tp = t0.elapsed().as_micros() as f64;
                        local_res.push(SingleResult {
                            n,
                            k,
                            p_train,
                            p_unseen,
                            rank,
                            noise,
                            model: "Candidate_LatentCP3_TP".to_string(),
                            seed,
                            seen_exact_acc: seen_acc,
                            seen_cosine: seen_cos,
                            unseen_exact_acc: unseen_acc,
                            unseen_cosine: unseen_cos,
                            max_sim: mean_max_sim_tp,
                            part_ratio: mean_pr_tp,
                            factor_entropy: f_entropy_tp,
                            wall_time_us: time_tp,
                        });

                        // 3. Baseline B0: Pairwise Hopfield Low-Rank
                        let t0 = Instant::now();
                        let model_b0 = train_pairwise_low_rank_svd(&train_pats, rank);
                        let (seen_acc, seen_cos) = evaluate_model_reconstruction(
                            &model_b0,
                            &train_pats,
                            noise,
                            seed.wrapping_add(1),
                            25,
                        );
                        let (unseen_acc, unseen_cos) = evaluate_model_reconstruction(
                            &model_b0,
                            &unseen_pats,
                            noise,
                            seed.wrapping_add(2),
                            25,
                        );
                        let time_b0 = t0.elapsed().as_micros() as f64;
                        local_res.push(SingleResult {
                            n,
                            k,
                            p_train,
                            p_unseen,
                            rank,
                            noise,
                            model: "B0_Pairwise_LowRank".to_string(),
                            seed,
                            seen_exact_acc: seen_acc,
                            seen_cosine: seen_cos,
                            unseen_exact_acc: unseen_acc,
                            unseen_cosine: unseen_cos,
                            max_sim: 0.0,
                            part_ratio: 0.0,
                            factor_entropy: 0.0,
                            wall_time_us: time_b0,
                        });

                        // 4. Baseline B1: Linear SVD Subspace feedforward projection
                        let t0 = Instant::now();
                        let proj = LinearSVDSubspace::from_patterns(&train_pats, rank);
                        let mut rng_seen = ChaCha8Rng::seed_from_u64(seed.wrapping_add(1));
                        let mut seen_exact = 0;
                        let mut seen_cos_tot = 0.0;
                        for pat in &train_pats {
                            let noisy = corrupt_pattern(pat, noise, &mut rng_seen);
                            let rec = proj.project(&noisy);
                            if rec.hamming_distance(pat) == 0 {
                                seen_exact += 1;
                            }
                            let mut dot = 0.0;
                            for i in 0..n {
                                dot += (rec.get(i) as f64) * (pat.get(i) as f64);
                            }
                            seen_cos_tot += dot / (n as f64);
                        }
                        let mut rng_unseen = ChaCha8Rng::seed_from_u64(seed.wrapping_add(2));
                        let mut unseen_exact = 0;
                        let mut unseen_cos_tot = 0.0;
                        for pat in &unseen_pats {
                            let noisy = corrupt_pattern(pat, noise, &mut rng_unseen);
                            let rec = proj.project(&noisy);
                            if rec.hamming_distance(pat) == 0 {
                                unseen_exact += 1;
                            }
                            let mut dot = 0.0;
                            for i in 0..n {
                                dot += (rec.get(i) as f64) * (pat.get(i) as f64);
                            }
                            unseen_cos_tot += dot / (n as f64);
                        }
                        let time_b1 = t0.elapsed().as_micros() as f64;
                        local_res.push(SingleResult {
                            n,
                            k,
                            p_train,
                            p_unseen,
                            rank,
                            noise,
                            model: "B1_LinearSVD".to_string(),
                            seed,
                            seen_exact_acc: (seen_exact as f64) / (train_pats.len() as f64),
                            seen_cosine: seen_cos_tot / (train_pats.len() as f64),
                            unseen_exact_acc: (unseen_exact as f64) / (unseen_pats.len() as f64),
                            unseen_cosine: unseen_cos_tot / (unseen_pats.len() as f64),
                            max_sim: 0.0,
                            part_ratio: 0.0,
                            factor_entropy: 0.0,
                            wall_time_us: time_b1,
                        });

                        // 5. Baseline B2: 1-NN Exemplar Oracle (train set only)
                        let t0 = Instant::now();
                        let oracle = NearestNeighborOracle::new(&train_pats);
                        let mut rng_seen = ChaCha8Rng::seed_from_u64(seed.wrapping_add(1));
                        let mut seen_exact = 0;
                        let mut seen_cos_tot = 0.0;
                        for pat in &train_pats {
                            let noisy = corrupt_pattern(pat, noise, &mut rng_seen);
                            let (rec, _, _) = oracle.retrieve(&noisy);
                            if rec.hamming_distance(pat) == 0 {
                                seen_exact += 1;
                            }
                            let mut dot = 0.0;
                            for i in 0..n {
                                dot += (rec.get(i) as f64) * (pat.get(i) as f64);
                            }
                            seen_cos_tot += dot / (n as f64);
                        }
                        let mut rng_unseen = ChaCha8Rng::seed_from_u64(seed.wrapping_add(2));
                        let mut unseen_exact = 0;
                        let mut unseen_cos_tot = 0.0;
                        for pat in &unseen_pats {
                            let noisy = corrupt_pattern(pat, noise, &mut rng_unseen);
                            let (rec, _, _) = oracle.retrieve(&noisy);
                            if rec.hamming_distance(pat) == 0 {
                                unseen_exact += 1;
                            }
                            let mut dot = 0.0;
                            for i in 0..n {
                                dot += (rec.get(i) as f64) * (pat.get(i) as f64);
                            }
                            unseen_cos_tot += dot / (n as f64);
                        }
                        let time_b2 = t0.elapsed().as_micros() as f64;
                        local_res.push(SingleResult {
                            n,
                            k,
                            p_train,
                            p_unseen,
                            rank,
                            noise,
                            model: "B2_1NN_Oracle".to_string(),
                            seed,
                            seen_exact_acc: (seen_exact as f64) / (train_pats.len() as f64),
                            seen_cosine: seen_cos_tot / (train_pats.len() as f64),
                            unseen_exact_acc: (unseen_exact as f64) / (unseen_pats.len() as f64),
                            unseen_cosine: unseen_cos_tot / (unseen_pats.len() as f64),
                            max_sim: 0.0,
                            part_ratio: 0.0,
                            factor_entropy: 0.0,
                            wall_time_us: time_b2,
                        });

                        // 6. Baseline B3: Modern Hopfield Attention (train set only, beta=1.5)
                        let t0 = Instant::now();
                        let m_hop = ModernHopfield::new(&train_pats, 1.5);
                        let mut rng_seen = ChaCha8Rng::seed_from_u64(seed.wrapping_add(1));
                        let mut seen_exact = 0;
                        let mut seen_cos_tot = 0.0;
                        for pat in &train_pats {
                            let noisy = corrupt_pattern(pat, noise, &mut rng_seen);
                            let rec = m_hop.retrieve(&noisy, 10);
                            if rec.hamming_distance(pat) == 0 {
                                seen_exact += 1;
                            }
                            let mut dot = 0.0;
                            for i in 0..n {
                                dot += (rec.get(i) as f64) * (pat.get(i) as f64);
                            }
                            seen_cos_tot += dot / (n as f64);
                        }
                        let mut rng_unseen = ChaCha8Rng::seed_from_u64(seed.wrapping_add(2));
                        let mut unseen_exact = 0;
                        let mut unseen_cos_tot = 0.0;
                        for pat in &unseen_pats {
                            let noisy = corrupt_pattern(pat, noise, &mut rng_unseen);
                            let rec = m_hop.retrieve(&noisy, 10);
                            if rec.hamming_distance(pat) == 0 {
                                unseen_exact += 1;
                            }
                            let mut dot = 0.0;
                            for i in 0..n {
                                dot += (rec.get(i) as f64) * (pat.get(i) as f64);
                            }
                            unseen_cos_tot += dot / (n as f64);
                        }
                        let time_b3 = t0.elapsed().as_micros() as f64;
                        local_res.push(SingleResult {
                            n,
                            k,
                            p_train,
                            p_unseen,
                            rank,
                            noise,
                            model: "B3_ModernHopfield".to_string(),
                            seed,
                            seen_exact_acc: (seen_exact as f64) / (train_pats.len() as f64),
                            seen_cosine: seen_cos_tot / (train_pats.len() as f64),
                            unseen_exact_acc: (unseen_exact as f64) / (unseen_pats.len() as f64),
                            unseen_cosine: unseen_cos_tot / (unseen_pats.len() as f64),
                            max_sim: 0.0,
                            part_ratio: 0.0,
                            factor_entropy: 0.0,
                            wall_time_us: time_b3,
                        });

                        local_res
                    })
                    .collect();

                // Aggregate and write to TSV
                for seed_res in &seed_results {
                    for r in seed_res {
                        writeln!(
                            tsv_file,
                            "{}\t{}\t{}\t{}\t{}\t{:.2}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.2}",
                            r.n,
                            r.k,
                            r.p_train,
                            r.p_unseen,
                            r.rank,
                            r.noise,
                            r.model,
                            r.seed,
                            r.seen_exact_acc,
                            r.seen_cosine,
                            r.unseen_exact_acc,
                            r.unseen_cosine,
                            r.max_sim,
                            r.part_ratio,
                            r.factor_entropy,
                            r.wall_time_us
                        )
                        .unwrap();
                        all_results.push(SingleResult {
                            n: r.n,
                            k: r.k,
                            p_train: r.p_train,
                            p_unseen: r.p_unseen,
                            rank: r.rank,
                            noise: r.noise,
                            model: r.model.clone(),
                            seed: r.seed,
                            seen_exact_acc: r.seen_exact_acc,
                            seen_cosine: r.seen_cosine,
                            unseen_exact_acc: r.unseen_exact_acc,
                            unseen_cosine: r.unseen_cosine,
                            max_sim: r.max_sim,
                            part_ratio: r.part_ratio,
                            factor_entropy: r.factor_entropy,
                            wall_time_us: r.wall_time_us,
                        });
                    }
                }

                // Compute averages for stdout summary
                let mut cp3_seen = 0.0;
                let mut cp3_unseen = 0.0;
                let mut cp3_sim = 0.0;
                let mut b1_seen = 0.0;
                let mut b1_unseen = 0.0;
                let mut b2_seen = 0.0;
                let mut b2_unseen = 0.0;
                let mut b3_unseen = 0.0;
                let count = seed_results.len() as f64;

                for seed_res in &seed_results {
                    for r in seed_res {
                        match r.model.as_str() {
                            "Candidate_LatentCP3_SVD" => {
                                cp3_seen += r.seen_exact_acc;
                                cp3_unseen += r.unseen_exact_acc;
                                cp3_sim += r.max_sim;
                            }
                            "B1_LinearSVD" => {
                                b1_seen += r.seen_exact_acc;
                                b1_unseen += r.unseen_exact_acc;
                            }
                            "B2_1NN_Oracle" => {
                                b2_seen += r.seen_exact_acc;
                                b2_unseen += r.unseen_exact_acc;
                            }
                            "B3_ModernHopfield" => {
                                b3_unseen += r.unseen_exact_acc;
                            }
                            _ => {}
                        }
                    }
                }

                println!(
                    "Seen Acc: 1NN={:.1}%, SVD={:.1}%, CP3={:.1}% | Unseen Acc: 1NN={:.1}%, MHop={:.1}%, SVD={:.1}%, CP3={:.1}% | MaxSim={:.3}",
                    (b2_seen / count) * 100.0,
                    (b1_seen / count) * 100.0,
                    (cp3_seen / count) * 100.0,
                    (b2_unseen / count) * 100.0,
                    (b3_unseen / count) * 100.0,
                    (b1_unseen / count) * 100.0,
                    (cp3_unseen / count) * 100.0,
                    cp3_sim / count
                );
            }
        }
    }

    tsv_file.flush().unwrap();
    println!(
        "\nEXP-TEN-003 complete in {:.2}s. Full TSV logged to research/fundamental_ai/EXP_TEN_003_RAW.tsv",
        total_start.elapsed().as_secs_f64()
    );
}
