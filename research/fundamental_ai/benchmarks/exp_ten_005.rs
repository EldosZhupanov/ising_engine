//! Benchmark executable running EXP-TEN-005:
//! Autonomous Energy Functional Search: Resolving Sign-Erasure Pathology and Cyclic Constraint Dynamics.
//!
//! Part 1: Latent Representation Reconstruction (P >> R) across Energy Laws.
//! Part 2: Cyclic Constraint Satisfaction on Closed Triads: Energy Relaxation vs Directed Attention.

#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]

use fundamental_ai::datasets::generate_compositional_grammar_suite;
use fundamental_ai::experiment::corrupt_pattern;
use fundamental_ai::learning::{compute_covariance_eigenvectors, LinearSVDSubspace};
use fundamental_ai::models::{
    ClosedTriadMemory, DualLatentModel, EnergyLaw, LatentEnergyModel, PairwiseCycleHopfield,
};
use fundamental_ai::types::SpinState;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

struct BenchmarkRow {
    part: String,
    model: String,
    noise: f64,
    steps: usize,
    seed: u64,
    instance_id: usize,
    metric_primary: f64,   // Cosine Sim (Part 1) or Exact Triad Rec (Part 2)
    metric_secondary: f64, // BER (Part 1) or Mean Entity Acc (Part 2)
    metric_exact: f64,     // Exact Recovery (Part 1 & 2)
    wall_time_us: f64,
}

fn main() {
    println!("================================================================================");
    println!("EXP-TEN-005: Autonomous Energy Functional Search & Cyclic Dynamics");
    println!("Part 1: Latent Compression (P >> R) - Resolving Sign-Erasure Pathology");
    println!("Part 2: Cyclic Constraint Dynamics - Energy Relaxation vs Directed Attention");
    println!("================================================================================");

    let output_path = "EXP_TEN_005_RAW.tsv";
    let mut out_file = File::create(output_path).expect("Failed to create EXP_TEN_005_RAW.tsv");
    writeln!(
        out_file,
        "part\tmodel\tnoise\tsteps\tseed\tinstance_id\tmetric_primary\tmetric_secondary\tmetric_exact\twall_time_us"
    )
    .unwrap();

    // -------------------------------------------------------------------------
    // PART 1: Latent Reconstruction (P >> R)
    // -------------------------------------------------------------------------
    println!("\n>>> RUNNING PART 1: Latent Compression & Energy Potential Search...");
    let n = 128;
    let k_latent = 11; // 2^10 = 1024 total patterns
    let p_train = 800;
    let rank = 24;

    let (train_pats, unseen_pats, _) =
        generate_compositional_grammar_suite(n, k_latent, p_train, 42);
    let test_count = unseen_pats.len().min(200);

    // Compute sample covariance eigenvectors
    let (basis, lambdas) = compute_covariance_eigenvectors(&train_pats, rank);

    // Build models
    let svd_baseline = LinearSVDSubspace {
        n,
        rank,
        basis: basis.clone(),
    };

    let model_cubic =
        LatentEnergyModel::new(n, rank, basis.clone(), lambdas.clone(), EnergyLaw::Cubic);
    let model_quartic =
        LatentEnergyModel::new(n, rank, basis.clone(), lambdas.clone(), EnergyLaw::Quartic);
    let model_sextic =
        LatentEnergyModel::new(n, rank, basis.clone(), lambdas.clone(), EnergyLaw::Sextic);
    let model_logcosh1 = LatentEnergyModel::new(
        n,
        rank,
        basis.clone(),
        lambdas.clone(),
        EnergyLaw::LogCosh { beta: 1.0 },
    );
    let model_logcosh3 = LatentEnergyModel::new(
        n,
        rank,
        basis.clone(),
        lambdas.clone(),
        EnergyLaw::LogCosh { beta: 3.0 },
    );
    let model_rat = LatentEnergyModel::new(
        n,
        rank,
        basis.clone(),
        lambdas.clone(),
        EnergyLaw::Rational { gamma: 0.1 },
    );
    let model_dual = DualLatentModel::new(n, rank, basis.clone(), 0.2);

    let noise_levels_part1 = [0.0, 0.15, 0.30];

    let mut part1_rows: Vec<BenchmarkRow> = Vec::new();

    for &noise in &noise_levels_part1 {
        let results: Vec<Vec<BenchmarkRow>> = (0..test_count)
            .into_par_iter()
            .map(|idx| {
                let clean_test = &unseen_pats[idx];
                let mut rng = ChaCha8Rng::seed_from_u64((idx as u64) + 1000);
                let query = if noise > 0.0 {
                    corrupt_pattern(clean_test, noise, &mut rng)
                } else {
                    clean_test.clone()
                };

                let mut rows = Vec::new();

                // 1. Linear SVD Subspace
                let t0 = Instant::now();
                let rec_svd = svd_baseline.project(&query);
                let dt = t0.elapsed().as_secs_f64() * 1e6;
                let cos_svd = clean_test.overlap(&rec_svd);
                let ber_svd = clean_test.bit_error_rate(&rec_svd);
                let exact_svd = if ber_svd == 0.0 { 1.0 } else { 0.0 };
                rows.push(BenchmarkRow {
                    part: "part1_latent".into(),
                    model: "B1_LinearSVD".into(),
                    noise,
                    steps: 1,
                    seed: idx as u64,
                    instance_id: idx,
                    metric_primary: cos_svd,
                    metric_secondary: ber_svd,
                    metric_exact: exact_svd,
                    wall_time_us: dt,
                });

                // 2. 1-NN Oracle in Train Set
                let t0 = Instant::now();
                let mut best_sim = -2.0;
                let mut best_pat = &train_pats[0];
                for tp in &train_pats {
                    let sim = query.overlap(tp);
                    if sim > best_sim {
                        best_sim = sim;
                        best_pat = tp;
                    }
                }
                let dt = t0.elapsed().as_secs_f64() * 1e6;
                let cos_nn = clean_test.overlap(best_pat);
                let ber_nn = clean_test.bit_error_rate(best_pat);
                let exact_nn = if ber_nn == 0.0 { 1.0 } else { 0.0 };
                rows.push(BenchmarkRow {
                    part: "part1_latent".into(),
                    model: "B2_1NN_Oracle".into(),
                    noise,
                    steps: 1,
                    seed: idx as u64,
                    instance_id: idx,
                    metric_primary: cos_nn,
                    metric_secondary: ber_nn,
                    metric_exact: exact_nn,
                    wall_time_us: dt,
                });

                // Energy models to evaluate
                let energy_models: [(&str, &LatentEnergyModel); 6] = [
                    ("Cubic_CP3", &model_cubic),
                    ("Quartic_CP4", &model_quartic),
                    ("Sextic_CP6", &model_sextic),
                    ("LogCosh_beta1", &model_logcosh1),
                    ("LogCosh_beta3", &model_logcosh3),
                    ("Rational_gamma01", &model_rat),
                ];

                for (name, em) in energy_models {
                    let t0 = Instant::now();
                    let (rec, flips) = em.relax_greedy(&query, 50);
                    let dt = t0.elapsed().as_secs_f64() * 1e6;
                    let cos = clean_test.overlap(&rec);
                    let ber = clean_test.bit_error_rate(&rec);
                    let exact = if ber == 0.0 { 1.0 } else { 0.0 };
                    rows.push(BenchmarkRow {
                        part: "part1_latent".into(),
                        model: name.to_string(),
                        noise,
                        steps: flips,
                        seed: idx as u64,
                        instance_id: idx,
                        metric_primary: cos,
                        metric_secondary: ber,
                        metric_exact: exact,
                        wall_time_us: dt,
                    });
                }

                // Dual Latent Model
                let t0 = Instant::now();
                let rec_dual = model_dual.reconstruct(&query, 5);
                let dt = t0.elapsed().as_secs_f64() * 1e6;
                let cos_dual = clean_test.overlap(&rec_dual);
                let ber_dual = clean_test.bit_error_rate(&rec_dual);
                let exact_dual = if ber_dual == 0.0 { 1.0 } else { 0.0 };
                rows.push(BenchmarkRow {
                    part: "part1_latent".into(),
                    model: "DualLatent_alpha02".into(),
                    noise,
                    steps: 5,
                    seed: idx as u64,
                    instance_id: idx,
                    metric_primary: cos_dual,
                    metric_secondary: ber_dual,
                    metric_exact: exact_dual,
                    wall_time_us: dt,
                });

                rows
            })
            .collect();

        for batch in results {
            for r in batch {
                part1_rows.push(r);
            }
        }
    }

    // Write Part 1 rows to TSV
    for r in &part1_rows {
        writeln!(
            out_file,
            "{}\t{}\t{:.2}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.2}",
            r.part,
            r.model,
            r.noise,
            r.steps,
            r.seed,
            r.instance_id,
            r.metric_primary,
            r.metric_secondary,
            r.metric_exact,
            r.wall_time_us
        )
        .unwrap();
    }

    // Print summary table for Part 1
    println!("\n--- PART 1 RESULTS SUMMARY (Mean Cosine Similarity on Held-Out Grammar) ---");
    println!(
        "{:<20} | {:<10} | {:<10} | {:<10}",
        "Model", "Noise=0.0", "Noise=0.15", "Noise=0.30"
    );
    println!("{:-<20}-+-{:-<10}-+-{:-<10}-+-{:-<10}", "", "", "", "");

    let model_names_p1 = [
        "B1_LinearSVD",
        "B2_1NN_Oracle",
        "Cubic_CP3",
        "Quartic_CP4",
        "Sextic_CP6",
        "LogCosh_beta1",
        "LogCosh_beta3",
        "Rational_gamma01",
        "DualLatent_alpha02",
    ];

    for name in model_names_p1 {
        let get_mean_cos = |ns: f64| -> f64 {
            let matched: Vec<f64> = part1_rows
                .iter()
                .filter(|r| r.model == name && (r.noise - ns).abs() < 1e-4)
                .map(|r| r.metric_primary)
                .collect();
            if matched.is_empty() {
                0.0
            } else {
                matched.iter().sum::<f64>() / matched.len() as f64
            }
        };

        println!(
            "{:<20} | {:<10.4} | {:<10.4} | {:<10.4}",
            name,
            get_mean_cos(0.0),
            get_mean_cos(0.15),
            get_mean_cos(0.30)
        );
    }

    // -------------------------------------------------------------------------
    // PART 2: Cyclic Constraint Dynamics (Closed Triads)
    // -------------------------------------------------------------------------
    println!("\n>>> RUNNING PART 2: Cyclic Relational Reasoning (Energy vs Attention)...");
    let n_e = 64;
    let n_r = 32;
    let num_triads = 32;
    let beta = 8.0;

    // Create knowledge base of closed triads
    let mut rng = ChaCha8Rng::seed_from_u64(999);
    let mut triads_a = Vec::with_capacity(num_triads);
    let mut triads_b = Vec::with_capacity(num_triads);
    let mut triads_c = Vec::with_capacity(num_triads);

    for _ in 0..num_triads {
        let mut sa = vec![1i8; n_e];
        let mut sb = vec![1i8; n_e];
        let mut sc = vec![1i8; n_e];
        for i in 0..n_e {
            if rng.gen_bool(0.5) {
                sa[i] = -1;
            }
            if rng.gen_bool(0.5) {
                sb[i] = -1;
            }
            if rng.gen_bool(0.5) {
                sc[i] = -1;
            }
        }
        triads_a.push(SpinState::from_slice(&sa));
        triads_b.push(SpinState::from_slice(&sb));
        triads_c.push(SpinState::from_slice(&sc));
    }

    let mut r1_spins = vec![1i8; n_r];
    let mut r2_spins = vec![1i8; n_r];
    let mut r3_spins = vec![1i8; n_r];
    for i in 0..n_r {
        if rng.gen_bool(0.5) {
            r1_spins[i] = -1;
        }
        if rng.gen_bool(0.5) {
            r2_spins[i] = -1;
        }
        if rng.gen_bool(0.5) {
            r3_spins[i] = -1;
        }
    }
    let r1 = SpinState::from_slice(&r1_spins);
    let r2 = SpinState::from_slice(&r2_spins);
    let r3 = SpinState::from_slice(&r3_spins);

    let triad_memory = ClosedTriadMemory::new(
        n_e,
        n_r,
        triads_a.clone(),
        triads_b.clone(),
        triads_c.clone(),
        r1,
        r2,
        r3,
        beta,
    );

    let pairwise_cycle = PairwiseCycleHopfield::train(&triads_a, &triads_b, &triads_c);

    let noise_levels_part2 = [0.15, 0.30, 0.45];
    let step_options = [1, 2, 4, 8, 16];
    let num_seeds = 10;

    let mut part2_rows: Vec<BenchmarkRow> = Vec::new();

    for &noise in &noise_levels_part2 {
        for &steps in &step_options {
            let results: Vec<Vec<BenchmarkRow>> = (0..num_seeds)
                .into_par_iter()
                .map(|seed_idx| {
                    let seed_base = 5000 + (seed_idx as u64) * 100;
                    let mut rows = Vec::new();

                    for mu in 0..num_triads {
                        let clean_a = &triads_a[mu];
                        let clean_b = &triads_b[mu];
                        let clean_c = &triads_c[mu];

                        // Corrupt ALL THREE entities simultaneously
                        let mut rng_a = ChaCha8Rng::seed_from_u64(seed_base + (mu as u64) * 3);
                        let mut rng_b = ChaCha8Rng::seed_from_u64(seed_base + (mu as u64) * 3 + 1);
                        let mut rng_c = ChaCha8Rng::seed_from_u64(seed_base + (mu as u64) * 3 + 2);

                        let noisy_a = corrupt_pattern(clean_a, noise, &mut rng_a);
                        let noisy_b = corrupt_pattern(clean_b, noise, &mut rng_b);
                        let noisy_c = corrupt_pattern(clean_c, noise, &mut rng_c);

                        // 1. Feedforward Attention 1-Pass
                        if steps == 1 {
                            let t0 = Instant::now();
                            let (ra, rb, rc) =
                                triad_memory.feedforward_1pass(&noisy_a, &noisy_b, &noisy_c);
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            let exact = if ra == *clean_a && rb == *clean_b && rc == *clean_c {
                                1.0
                            } else {
                                0.0
                            };
                            let acc_a = if ra == *clean_a { 1.0 } else { 0.0 };
                            let acc_b = if rb == *clean_b { 1.0 } else { 0.0 };
                            let acc_c = if rc == *clean_c { 1.0 } else { 0.0 };
                            rows.push(BenchmarkRow {
                                part: "part2_cyclic".into(),
                                model: "FF_Attention_1Pass".into(),
                                noise,
                                steps: 1,
                                seed: seed_idx as u64,
                                instance_id: mu,
                                metric_primary: exact,
                                metric_secondary: (acc_a + acc_b + acc_c) / 3.0,
                                metric_exact: exact,
                                wall_time_us: dt,
                            });
                        }

                        // 2. Feedforward Attention 2-Pass
                        if steps == 2 {
                            let t0 = Instant::now();
                            let (ra, rb, rc) =
                                triad_memory.feedforward_2pass(&noisy_a, &noisy_b, &noisy_c);
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            let exact = if ra == *clean_a && rb == *clean_b && rc == *clean_c {
                                1.0
                            } else {
                                0.0
                            };
                            let acc_a = if ra == *clean_a { 1.0 } else { 0.0 };
                            let acc_b = if rb == *clean_b { 1.0 } else { 0.0 };
                            let acc_c = if rc == *clean_c { 1.0 } else { 0.0 };
                            rows.push(BenchmarkRow {
                                part: "part2_cyclic".into(),
                                model: "FF_Attention_2Pass".into(),
                                noise,
                                steps: 2,
                                seed: seed_idx as u64,
                                instance_id: mu,
                                metric_primary: exact,
                                metric_secondary: (acc_a + acc_b + acc_c) / 3.0,
                                metric_exact: exact,
                                wall_time_us: dt,
                            });
                        }

                        // 3. Recurrent Directed Attention
                        {
                            let t0 = Instant::now();
                            let (ra, rb, rc) = triad_memory
                                .recurrent_attention(&noisy_a, &noisy_b, &noisy_c, steps);
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            let exact = if ra == *clean_a && rb == *clean_b && rc == *clean_c {
                                1.0
                            } else {
                                0.0
                            };
                            let acc_a = if ra == *clean_a { 1.0 } else { 0.0 };
                            let acc_b = if rb == *clean_b { 1.0 } else { 0.0 };
                            let acc_c = if rc == *clean_c { 1.0 } else { 0.0 };
                            rows.push(BenchmarkRow {
                                part: "part2_cyclic".into(),
                                model: "Recurrent_Attention".into(),
                                noise,
                                steps,
                                seed: seed_idx as u64,
                                instance_id: mu,
                                metric_primary: exact,
                                metric_secondary: (acc_a + acc_b + acc_c) / 3.0,
                                metric_exact: exact,
                                wall_time_us: dt,
                            });
                        }

                        // 4. Pairwise Hopfield Cycle
                        {
                            let t0 = Instant::now();
                            let (ra, rb, rc) =
                                pairwise_cycle.relax(&noisy_a, &noisy_b, &noisy_c, steps);
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            let exact = if ra == *clean_a && rb == *clean_b && rc == *clean_c {
                                1.0
                            } else {
                                0.0
                            };
                            let acc_a = if ra == *clean_a { 1.0 } else { 0.0 };
                            let acc_b = if rb == *clean_b { 1.0 } else { 0.0 };
                            let acc_c = if rc == *clean_c { 1.0 } else { 0.0 };
                            rows.push(BenchmarkRow {
                                part: "part2_cyclic".into(),
                                model: "Pairwise_Hopfield_Cycle".into(),
                                noise,
                                steps,
                                seed: seed_idx as u64,
                                instance_id: mu,
                                metric_primary: exact,
                                metric_secondary: (acc_a + acc_b + acc_c) / 3.0,
                                metric_exact: exact,
                                wall_time_us: dt,
                            });
                        }

                        // 5. Modern Trilinear Bidirectional Energy Relaxation
                        {
                            let t0 = Instant::now();
                            let (ra, rb, rc) =
                                triad_memory.relax_energy(&noisy_a, &noisy_b, &noisy_c, steps);
                            let dt = t0.elapsed().as_secs_f64() * 1e6;
                            let exact = if ra == *clean_a && rb == *clean_b && rc == *clean_c {
                                1.0
                            } else {
                                0.0
                            };
                            let acc_a = if ra == *clean_a { 1.0 } else { 0.0 };
                            let acc_b = if rb == *clean_b { 1.0 } else { 0.0 };
                            let acc_c = if rc == *clean_c { 1.0 } else { 0.0 };
                            rows.push(BenchmarkRow {
                                part: "part2_cyclic".into(),
                                model: "Modern_Trilinear_Cycle_Energy".into(),
                                noise,
                                steps,
                                seed: seed_idx as u64,
                                instance_id: mu,
                                metric_primary: exact,
                                metric_secondary: (acc_a + acc_b + acc_c) / 3.0,
                                metric_exact: exact,
                                wall_time_us: dt,
                            });
                        }
                    }
                    rows
                })
                .collect();

            for batch in results {
                for r in batch {
                    part2_rows.push(r);
                }
            }
        }
    }

    // Write Part 2 rows to TSV
    for r in &part2_rows {
        writeln!(
            out_file,
            "{}\t{}\t{:.2}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.2}",
            r.part,
            r.model,
            r.noise,
            r.steps,
            r.seed,
            r.instance_id,
            r.metric_primary,
            r.metric_secondary,
            r.metric_exact,
            r.wall_time_us
        )
        .unwrap();
    }

    // Print summary table for Part 2 at noise = 0.30 across steps
    println!("\n--- PART 2 RESULTS SUMMARY (Exact Triad Recovery at Noise = 0.30) ---");
    println!(
        "{:<30} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8}",
        "Model", "T=1", "T=2", "T=4", "T=8", "T=16"
    );
    println!(
        "{:-<30}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}",
        "", "", "", "", "", ""
    );

    let models_p2 = [
        "FF_Attention_1Pass",
        "FF_Attention_2Pass",
        "Recurrent_Attention",
        "Pairwise_Hopfield_Cycle",
        "Modern_Trilinear_Cycle_Energy",
    ];

    for name in models_p2 {
        let get_acc = |s: usize| -> f64 {
            let matched: Vec<f64> = part2_rows
                .iter()
                .filter(|r| r.model == name && (r.noise - 0.30).abs() < 1e-4 && r.steps == s)
                .map(|r| r.metric_primary)
                .collect();
            if matched.is_empty() {
                0.0
            } else {
                matched.iter().sum::<f64>() / matched.len() as f64
            }
        };

        println!(
            "{:<30} | {:<8.1}% | {:<8.1}% | {:<8.1}% | {:<8.1}% | {:<8.1}%",
            name,
            get_acc(1) * 100.0,
            get_acc(2) * 100.0,
            get_acc(4) * 100.0,
            get_acc(8) * 100.0,
            get_acc(16) * 100.0
        );
    }

    println!("\n>>> EXP-TEN-005 BENCHMARK COMPLETE. Data saved to EXP_TEN_005_RAW.tsv");
}
