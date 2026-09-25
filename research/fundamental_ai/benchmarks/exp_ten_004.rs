//! Benchmark executable running EXP-TEN-004: Relational Compositional Generalization.
//! Tests inference-time compute scaling (T=1, 2, 4, 8) and trilinear tensor binding
//! on systematically held-out 2-hop compositions: A -> R1 -> ? -> R2 -> C.

#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]

use fundamental_ai::datasets::{generate_relational_world, RelationalFact, TwoHopQuery};
use fundamental_ai::experiment::corrupt_pattern;
use fundamental_ai::types::SpinState;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

struct SingleResult {
    num_entities: usize,
    steps: usize,
    noise: f64,
    model: String,
    seed: u64,
    inter_acc: f64,
    final_acc: f64,
    final_cos: f64,
    wall_time_us: f64,
}

/// Helper: Raw dot product between two spin states sum_i s_i * other_i.
#[inline]
fn spin_dot(a: &SpinState, b: &SpinState) -> f64 {
    let mut d = 0;
    for i in 0..a.len() {
        d += (a.get(i) as i32) * (b.get(i) as i32);
    }
    d as f64
}

/// Linear Trilinear Tensor Energy Chain Solver (Classical Hebbian TPR).
/// Updates variables synchronously to measure parallel propagation depth.
fn solve_linear_trilinear_chain(
    facts_hop1: &[RelationalFact],
    facts_hop2: &[RelationalFact],
    _query: &TwoHopQuery,
    s_a: &SpinState,
    s_r1: &SpinState,
    s_r2: &SpinState,
    steps: usize,
    seed: u64,
) -> (SpinState, SpinState) {
    let n_e = s_a.len();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut s_x = SpinState::from_slice(
        &(0..n_e)
            .map(|_| {
                if rand::Rng::gen::<bool>(&mut rng) {
                    1i8
                } else {
                    -1i8
                }
            })
            .collect::<Vec<i8>>(),
    );
    let mut s_c = SpinState::from_slice(
        &(0..n_e)
            .map(|_| {
                if rand::Rng::gen::<bool>(&mut rng) {
                    1i8
                } else {
                    -1i8
                }
            })
            .collect::<Vec<i8>>(),
    );

    let mut h_x = vec![0.0; n_e];
    let mut h_c = vec![0.0; n_e];

    for _ in 0..steps {
        h_x.fill(0.0);
        h_c.fill(0.0);

        // Forward field on s_X from hop 1
        for f in facts_hop1 {
            let dot_a = spin_dot(s_a, &f.s_subject);
            let dot_r1 = spin_dot(s_r1, &f.s_relation);
            let weight = dot_a * dot_r1;
            for i in 0..n_e {
                h_x[i] += weight * (f.s_object.get(i) as f64);
            }
        }
        // Backward field on s_X from hop 2
        for f in facts_hop2 {
            let dot_c = spin_dot(&s_c, &f.s_object);
            let dot_r2 = spin_dot(s_r2, &f.s_relation);
            let weight = dot_c * dot_r2;
            for i in 0..n_e {
                h_x[i] += weight * (f.s_subject.get(i) as f64);
            }
        }

        // Forward field on s_C from hop 2 (computed from OLD s_x synchronously)
        for f in facts_hop2 {
            let dot_x = spin_dot(&s_x, &f.s_subject);
            let dot_r2 = spin_dot(s_r2, &f.s_relation);
            let weight = dot_x * dot_r2;
            for i in 0..n_e {
                h_c[i] += weight * (f.s_object.get(i) as f64);
            }
        }

        // Synchronous update
        for i in 0..n_e {
            s_x.set(i, if h_x[i] >= 0.0 { 1 } else { -1 });
            s_c.set(i, if h_c[i] >= 0.0 { 1 } else { -1 });
        }
    }

    (s_x, s_c)
}

/// Modern Trilinear Tensor Energy Chain Solver (High-Order Contrast Separation).
/// Synchronous parallel relaxation with beta contrast.
fn solve_modern_trilinear_chain(
    facts_hop1: &[RelationalFact],
    facts_hop2: &[RelationalFact],
    _query: &TwoHopQuery,
    s_a: &SpinState,
    s_r1: &SpinState,
    s_r2: &SpinState,
    steps: usize,
    beta: f64,
    seed: u64,
) -> (SpinState, SpinState) {
    let n_e = s_a.len();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut s_x = SpinState::from_slice(
        &(0..n_e)
            .map(|_| {
                if rand::Rng::gen::<bool>(&mut rng) {
                    1i8
                } else {
                    -1i8
                }
            })
            .collect::<Vec<i8>>(),
    );
    let mut s_c = SpinState::from_slice(
        &(0..n_e)
            .map(|_| {
                if rand::Rng::gen::<bool>(&mut rng) {
                    1i8
                } else {
                    -1i8
                }
            })
            .collect::<Vec<i8>>(),
    );

    let mut h_x = vec![0.0; n_e];
    let mut h_c = vec![0.0; n_e];

    for _ in 0..steps {
        h_x.fill(0.0);
        h_c.fill(0.0);

        // 1. Forward contrast scores for hop 1: (s_A . xi_A) * (s_R1 . xi_R)
        // Normalized overlap is in [-1.0, 1.0]
        let mut scores1 = Vec::with_capacity(facts_hop1.len());
        let mut max_sc1 = f64::NEG_INFINITY;
        for f in facts_hop1 {
            let sc = s_a.overlap(&f.s_subject) * s_r1.overlap(&f.s_relation);
            scores1.push(sc);
            if sc > max_sc1 {
                max_sc1 = sc;
            }
        }
        let mut sum_exp1 = 0.0;
        let mut w1 = Vec::with_capacity(facts_hop1.len());
        for &sc in &scores1 {
            let w = ((sc - max_sc1) * beta * (n_e as f64)).exp();
            w1.push(w);
            sum_exp1 += w;
        }
        for (idx, f) in facts_hop1.iter().enumerate() {
            let weight = w1[idx] / sum_exp1;
            for i in 0..n_e {
                h_x[i] += weight * (f.s_object.get(i) as f64);
            }
        }

        // 2. Forward contrast scores for hop 2: (s_X . xi_A) * (s_R2 . xi_R) computed from OLD s_X
        let mut scores2 = Vec::with_capacity(facts_hop2.len());
        let mut max_sc2 = f64::NEG_INFINITY;
        for f in facts_hop2 {
            let sc = s_x.overlap(&f.s_subject) * s_r2.overlap(&f.s_relation);
            scores2.push(sc);
            if sc > max_sc2 {
                max_sc2 = sc;
            }
        }
        let mut sum_exp2 = 0.0;
        let mut w2 = Vec::with_capacity(facts_hop2.len());
        for &sc in &scores2 {
            let w = ((sc - max_sc2) * beta * (n_e as f64)).exp();
            w2.push(w);
            sum_exp2 += w;
        }
        for (idx, f) in facts_hop2.iter().enumerate() {
            let weight = w2[idx] / sum_exp2;
            for i in 0..n_e {
                h_c[i] += weight * (f.s_object.get(i) as f64);
            }
        }

        // Synchronous update
        for i in 0..n_e {
            s_x.set(i, if h_x[i] >= 0.0 { 1 } else { -1 });
            s_c.set(i, if h_c[i] >= 0.0 { 1 } else { -1 });
        }
    }

    (s_x, s_c)
}

/// Pairwise Hopfield Chain Solver.
fn solve_pairwise_chain(
    facts_hop1: &[RelationalFact],
    facts_hop2: &[RelationalFact],
    _query: &TwoHopQuery,
    s_a: &SpinState,
    s_r1: &SpinState,
    s_r2: &SpinState,
    steps: usize,
    seed: u64,
) -> (SpinState, SpinState) {
    let n_e = s_a.len();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let mut s_x = SpinState::from_slice(
        &(0..n_e)
            .map(|_| {
                if rand::Rng::gen::<bool>(&mut rng) {
                    1i8
                } else {
                    -1i8
                }
            })
            .collect::<Vec<i8>>(),
    );
    let mut s_c = SpinState::from_slice(
        &(0..n_e)
            .map(|_| {
                if rand::Rng::gen::<bool>(&mut rng) {
                    1i8
                } else {
                    -1i8
                }
            })
            .collect::<Vec<i8>>(),
    );

    let mut h_x = vec![0.0; n_e];
    let mut h_c = vec![0.0; n_e];

    for _ in 0..steps {
        h_x.fill(0.0);
        h_c.fill(0.0);

        for f in facts_hop1 {
            let dot_a = spin_dot(s_a, &f.s_subject);
            let dot_r1 = spin_dot(s_r1, &f.s_relation);
            for i in 0..n_e {
                h_x[i] += (dot_a + dot_r1) * (f.s_object.get(i) as f64);
            }
        }

        for f in facts_hop2 {
            let dot_x = spin_dot(&s_x, &f.s_subject);
            let dot_r2 = spin_dot(s_r2, &f.s_relation);
            for i in 0..n_e {
                h_c[i] += (dot_x + dot_r2) * (f.s_object.get(i) as f64);
            }
        }

        for i in 0..n_e {
            s_x.set(i, if h_x[i] >= 0.0 { 1 } else { -1 });
            s_c.set(i, if h_c[i] >= 0.0 { 1 } else { -1 });
        }
    }

    (s_x, s_c)
}

/// Cascaded 1-NN Lookup Baseline.
fn solve_cascaded_1nn(
    facts_hop1: &[RelationalFact],
    facts_hop2: &[RelationalFact],
    s_a: &SpinState,
    s_r1: &SpinState,
    s_r2: &SpinState,
) -> (SpinState, SpinState) {
    let mut best_score1 = f64::NEG_INFINITY;
    let mut best_fact1 = &facts_hop1[0];
    for f in facts_hop1 {
        let score = s_a.overlap(&f.s_subject) * s_r1.overlap(&f.s_relation);
        if score > best_score1 {
            best_score1 = score;
            best_fact1 = f;
        }
    }
    let s_x = best_fact1.s_object.clone();

    let mut best_score2 = f64::NEG_INFINITY;
    let mut best_fact2 = &facts_hop2[0];
    for f in facts_hop2 {
        let score = s_x.overlap(&f.s_subject) * s_r2.overlap(&f.s_relation);
        if score > best_score2 {
            best_score2 = score;
            best_fact2 = f;
        }
    }
    let s_c = best_fact2.s_object.clone();

    (s_x, s_c)
}

/// Cascaded Modern Hopfield Baseline.
fn solve_cascaded_modern_hopfield(
    facts_hop1: &[RelationalFact],
    facts_hop2: &[RelationalFact],
    s_a: &SpinState,
    s_r1: &SpinState,
    s_r2: &SpinState,
    beta: f64,
) -> (SpinState, SpinState) {
    let n_e = s_a.len();

    // Stage 1
    let mut scores1 = Vec::with_capacity(facts_hop1.len());
    let mut max_sc1 = f64::NEG_INFINITY;
    for f in facts_hop1 {
        let sc = s_a.overlap(&f.s_subject) * s_r1.overlap(&f.s_relation);
        scores1.push(sc);
        if sc > max_sc1 {
            max_sc1 = sc;
        }
    }
    let mut weights1 = Vec::with_capacity(facts_hop1.len());
    let mut sum_exp1 = 0.0;
    for &sc in &scores1 {
        let w = ((sc - max_sc1) * beta * (n_e as f64)).exp();
        weights1.push(w);
        sum_exp1 += w;
    }
    let mut out_x = vec![0.0; n_e];
    for (idx, f) in facts_hop1.iter().enumerate() {
        let norm_w = weights1[idx] / sum_exp1;
        for i in 0..n_e {
            out_x[i] += norm_w * (f.s_object.get(i) as f64);
        }
    }
    let s_x = SpinState::from_slice(
        &out_x
            .iter()
            .map(|&v| if v >= 0.0 { 1i8 } else { -1i8 })
            .collect::<Vec<i8>>(),
    );

    // Stage 2
    let mut scores2 = Vec::with_capacity(facts_hop2.len());
    let mut max_sc2 = f64::NEG_INFINITY;
    for f in facts_hop2 {
        let sc = s_x.overlap(&f.s_subject) * s_r2.overlap(&f.s_relation);
        scores2.push(sc);
        if sc > max_sc2 {
            max_sc2 = sc;
        }
    }
    let mut weights2 = Vec::with_capacity(facts_hop2.len());
    let mut sum_exp2 = 0.0;
    for &sc in &scores2 {
        let w = ((sc - max_sc2) * beta * (n_e as f64)).exp();
        weights2.push(w);
        sum_exp2 += w;
    }
    let mut out_c = vec![0.0; n_e];
    for (idx, f) in facts_hop2.iter().enumerate() {
        let norm_w = weights2[idx] / sum_exp2;
        for i in 0..n_e {
            out_c[i] += norm_w * (f.s_object.get(i) as f64);
        }
    }
    let s_c = SpinState::from_slice(
        &out_c
            .iter()
            .map(|&v| if v >= 0.0 { 1i8 } else { -1i8 })
            .collect::<Vec<i8>>(),
    );

    (s_x, s_c)
}

fn main() {
    println!("================================================================================");
    println!("EXP-TEN-004: Relational Compositional Generalization & Inference Scaling");
    println!("Evaluating Multi-Hop Energy Relaxation vs Cascaded and Pairwise Baselines");
    println!("================================================================================\n");

    let total_start = Instant::now();
    let num_entities = 32;
    let num_relations = 4;
    let entity_dim = 64;
    let relation_dim = 32;
    let step_values = [1, 2, 4, 8];
    let noise_levels = [0.0, 0.15, 0.30];
    let seeds: Vec<u64> = (4001..=4020).collect();

    let mut tsv_file = File::create("research/fundamental_ai/EXP_TEN_004_RAW.tsv")
        .expect("Failed to create TSV file");
    writeln!(
        tsv_file,
        "entities\tsteps\tnoise\tmodel\tseed\tinter_acc\tfinal_acc\tfinal_cos\twall_time_us"
    )
    .unwrap();

    for &noise in &noise_levels {
        println!("\n>>> Evaluation under Query Noise = {:.0}%", noise * 100.0);

        for &steps in &step_values {
            print!("  Inference Steps T = {}: ", steps);

            let seed_results: Vec<Vec<SingleResult>> = seeds
                .par_iter()
                .map(|&seed| {
                    let mut local_res = Vec::new();
                    let world = generate_relational_world(
                        num_entities,
                        num_relations,
                        entity_dim,
                        relation_dim,
                        seed,
                    );

                    let facts_hop1: Vec<RelationalFact> = world
                        .train_facts
                        .iter()
                        .filter(|f| f.relation_idx == 0)
                        .cloned()
                        .collect();
                    let facts_hop2: Vec<RelationalFact> = world
                        .train_facts
                        .iter()
                        .filter(|f| f.relation_idx == 1)
                        .cloned()
                        .collect();

                    let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(5));

                    // 1. Candidate: Modern Trilinear Tensor Energy Chain (Beta = 0.2)
                    let t0 = Instant::now();
                    let mut inter_exact = 0;
                    let mut final_exact = 0;
                    let mut total_cos = 0.0;
                    for q in &world.test_queries {
                        let noisy_a = if noise > 0.0 {
                            corrupt_pattern(&q.s_subject, noise, &mut rng)
                        } else {
                            q.s_subject.clone()
                        };
                        let noisy_r1 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation1, noise, &mut rng)
                        } else {
                            q.s_relation1.clone()
                        };
                        let noisy_r2 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation2, noise, &mut rng)
                        } else {
                            q.s_relation2.clone()
                        };

                        let (rec_x, rec_c) = solve_modern_trilinear_chain(
                            &facts_hop1,
                            &facts_hop2,
                            q,
                            &noisy_a,
                            &noisy_r1,
                            &noisy_r2,
                            steps,
                            0.2,
                            seed.wrapping_add(10),
                        );

                        if rec_x.hamming_distance(&q.s_intermediate_target) == 0 {
                            inter_exact += 1;
                        }
                        if rec_c.hamming_distance(&q.s_final_target) == 0 {
                            final_exact += 1;
                        }
                        total_cos += rec_c.overlap(&q.s_final_target);
                    }
                    let time_cand = t0.elapsed().as_micros() as f64;
                    local_res.push(SingleResult {
                        num_entities,
                        steps,
                        noise,
                        model: "Candidate_ModernTrilinearChain".to_string(),
                        seed,
                        inter_acc: (inter_exact as f64) / (world.test_queries.len() as f64),
                        final_acc: (final_exact as f64) / (world.test_queries.len() as f64),
                        final_cos: total_cos / (world.test_queries.len() as f64),
                        wall_time_us: time_cand,
                    });

                    // 2. Classical Linear Trilinear Chain (Beta = 0)
                    let t0 = Instant::now();
                    let mut lin_inter = 0;
                    let mut lin_final = 0;
                    let mut lin_cos = 0.0;
                    for q in &world.test_queries {
                        let noisy_a = if noise > 0.0 {
                            corrupt_pattern(&q.s_subject, noise, &mut rng)
                        } else {
                            q.s_subject.clone()
                        };
                        let noisy_r1 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation1, noise, &mut rng)
                        } else {
                            q.s_relation1.clone()
                        };
                        let noisy_r2 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation2, noise, &mut rng)
                        } else {
                            q.s_relation2.clone()
                        };

                        let (rec_x, rec_c) = solve_linear_trilinear_chain(
                            &facts_hop1,
                            &facts_hop2,
                            q,
                            &noisy_a,
                            &noisy_r1,
                            &noisy_r2,
                            steps,
                            seed.wrapping_add(15),
                        );
                        if rec_x.hamming_distance(&q.s_intermediate_target) == 0 {
                            lin_inter += 1;
                        }
                        if rec_c.hamming_distance(&q.s_final_target) == 0 {
                            lin_final += 1;
                        }
                        lin_cos += rec_c.overlap(&q.s_final_target);
                    }
                    let time_lin = t0.elapsed().as_micros() as f64;
                    local_res.push(SingleResult {
                        num_entities,
                        steps,
                        noise,
                        model: "B1_LinearTrilinearChain".to_string(),
                        seed,
                        inter_acc: (lin_inter as f64) / (world.test_queries.len() as f64),
                        final_acc: (lin_final as f64) / (world.test_queries.len() as f64),
                        final_cos: lin_cos / (world.test_queries.len() as f64),
                        wall_time_us: time_lin,
                    });

                    // 3. Baseline B0: Pairwise Hopfield Chain
                    let t0 = Instant::now();
                    let mut b0_inter = 0;
                    let mut b0_final = 0;
                    let mut b0_cos = 0.0;
                    for q in &world.test_queries {
                        let noisy_a = if noise > 0.0 {
                            corrupt_pattern(&q.s_subject, noise, &mut rng)
                        } else {
                            q.s_subject.clone()
                        };
                        let noisy_r1 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation1, noise, &mut rng)
                        } else {
                            q.s_relation1.clone()
                        };
                        let noisy_r2 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation2, noise, &mut rng)
                        } else {
                            q.s_relation2.clone()
                        };

                        let (rec_x, rec_c) = solve_pairwise_chain(
                            &facts_hop1,
                            &facts_hop2,
                            q,
                            &noisy_a,
                            &noisy_r1,
                            &noisy_r2,
                            steps,
                            seed.wrapping_add(20),
                        );
                        if rec_x.hamming_distance(&q.s_intermediate_target) == 0 {
                            b0_inter += 1;
                        }
                        if rec_c.hamming_distance(&q.s_final_target) == 0 {
                            b0_final += 1;
                        }
                        b0_cos += rec_c.overlap(&q.s_final_target);
                    }
                    let time_b0 = t0.elapsed().as_micros() as f64;
                    local_res.push(SingleResult {
                        num_entities,
                        steps,
                        noise,
                        model: "B0_PairwiseChain".to_string(),
                        seed,
                        inter_acc: (b0_inter as f64) / (world.test_queries.len() as f64),
                        final_acc: (b0_final as f64) / (world.test_queries.len() as f64),
                        final_cos: b0_cos / (world.test_queries.len() as f64),
                        wall_time_us: time_b0,
                    });

                    // 4. Baseline B2: Cascaded 1-NN
                    let t0 = Instant::now();
                    let mut b1_inter = 0;
                    let mut b1_final = 0;
                    let mut b1_cos = 0.0;
                    for q in &world.test_queries {
                        let noisy_a = if noise > 0.0 {
                            corrupt_pattern(&q.s_subject, noise, &mut rng)
                        } else {
                            q.s_subject.clone()
                        };
                        let noisy_r1 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation1, noise, &mut rng)
                        } else {
                            q.s_relation1.clone()
                        };
                        let noisy_r2 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation2, noise, &mut rng)
                        } else {
                            q.s_relation2.clone()
                        };

                        let (rec_x, rec_c) = solve_cascaded_1nn(
                            &facts_hop1,
                            &facts_hop2,
                            &noisy_a,
                            &noisy_r1,
                            &noisy_r2,
                        );
                        if rec_x.hamming_distance(&q.s_intermediate_target) == 0 {
                            b1_inter += 1;
                        }
                        if rec_c.hamming_distance(&q.s_final_target) == 0 {
                            b1_final += 1;
                        }
                        b1_cos += rec_c.overlap(&q.s_final_target);
                    }
                    let time_b1 = t0.elapsed().as_micros() as f64;
                    local_res.push(SingleResult {
                        num_entities,
                        steps,
                        noise,
                        model: "B2_Cascaded1NN".to_string(),
                        seed,
                        inter_acc: (b1_inter as f64) / (world.test_queries.len() as f64),
                        final_acc: (b1_final as f64) / (world.test_queries.len() as f64),
                        final_cos: b1_cos / (world.test_queries.len() as f64),
                        wall_time_us: time_b1,
                    });

                    // 5. Baseline B3: Cascaded Modern Hopfield
                    let t0 = Instant::now();
                    let mut b3_inter = 0;
                    let mut b3_final = 0;
                    let mut b3_cos = 0.0;
                    for q in &world.test_queries {
                        let noisy_a = if noise > 0.0 {
                            corrupt_pattern(&q.s_subject, noise, &mut rng)
                        } else {
                            q.s_subject.clone()
                        };
                        let noisy_r1 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation1, noise, &mut rng)
                        } else {
                            q.s_relation1.clone()
                        };
                        let noisy_r2 = if noise > 0.0 {
                            corrupt_pattern(&q.s_relation2, noise, &mut rng)
                        } else {
                            q.s_relation2.clone()
                        };

                        let (rec_x, rec_c) = solve_cascaded_modern_hopfield(
                            &facts_hop1,
                            &facts_hop2,
                            &noisy_a,
                            &noisy_r1,
                            &noisy_r2,
                            0.2,
                        );
                        if rec_x.hamming_distance(&q.s_intermediate_target) == 0 {
                            b3_inter += 1;
                        }
                        if rec_c.hamming_distance(&q.s_final_target) == 0 {
                            b3_final += 1;
                        }
                        b3_cos += rec_c.overlap(&q.s_final_target);
                    }
                    let time_b3 = t0.elapsed().as_micros() as f64;
                    local_res.push(SingleResult {
                        num_entities,
                        steps,
                        noise,
                        model: "B3_CascadedModernHopfield".to_string(),
                        seed,
                        inter_acc: (b3_inter as f64) / (world.test_queries.len() as f64),
                        final_acc: (b3_final as f64) / (world.test_queries.len() as f64),
                        final_cos: b3_cos / (world.test_queries.len() as f64),
                        wall_time_us: time_b3,
                    });

                    local_res
                })
                .collect();

            // Write to TSV
            for seed_res in &seed_results {
                for r in seed_res {
                    writeln!(
                        tsv_file,
                        "{}\t{}\t{:.2}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.2}",
                        r.num_entities,
                        r.steps,
                        r.noise,
                        r.model,
                        r.seed,
                        r.inter_acc,
                        r.final_acc,
                        r.final_cos,
                        r.wall_time_us
                    )
                    .unwrap();
                }
            }

            // Summary print
            let mut cand_acc = 0.0;
            let mut cand_cos = 0.0;
            let mut lin_acc = 0.0;
            let mut b0_acc = 0.0;
            let mut b1_acc = 0.0;
            let count = seed_results.len() as f64;
            for seed_res in &seed_results {
                for r in seed_res {
                    match r.model.as_str() {
                        "Candidate_ModernTrilinearChain" => {
                            cand_acc += r.final_acc;
                            cand_cos += r.final_cos;
                        }
                        "B1_LinearTrilinearChain" => {
                            lin_acc += r.final_acc;
                        }
                        "B0_PairwiseChain" => {
                            b0_acc += r.final_acc;
                        }
                        "B2_Cascaded1NN" => {
                            b1_acc += r.final_acc;
                        }
                        _ => {}
                    }
                }
            }

            println!(
                "Candidate Modern Chain Acc={:.1}% (Cos={:.3}) | Linear TPR={:.1}% | Pairwise={:.1}% | Cascaded 1NN={:.1}%",
                (cand_acc / count) * 100.0,
                cand_cos / count,
                (lin_acc / count) * 100.0,
                (b0_acc / count) * 100.0,
                (b1_acc / count) * 100.0
            );
        }
    }

    tsv_file.flush().unwrap();
    println!(
        "\nEXP-TEN-004 complete in {:.2}s. Full TSV logged to research/fundamental_ai/EXP_TEN_004_RAW.tsv",
        total_start.elapsed().as_secs_f64()
    );
}
