//! Benchmark executable running EXP-TEN-006A:
//! Autonomous Learning of Dual-Variable Energy Networks on Cyclic Constraint Factor Graphs.
//!
//! Evaluates whether a parameter-shared local energy interaction can be learned from data
//! on short cycles (K=3..5) and transferred to solve unseen larger cyclic topologies (K=8, 16, 32)
//! through iterative relaxation, compared against Recurrent Attention and GNN Message Passing.

#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]

use fundamental_ai::learned_energy::{
    GNNMessagePassing, LearnedDualEnergyNetwork, PermutationWorld, RecurrentAttentionGraph,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

struct BenchmarkRow {
    split: String,
    cycle_k: usize,
    model: String,
    noise: f64,
    steps: usize,
    seed: u64,
    instance_id: usize,
    exact_solve: f64,
    mean_node_acc: f64,
    limit_cycle: f64,
    wall_time_us: f64,
}

fn main() {
    println!("================================================================================");
    println!("EXP-TEN-006A: Learned Dual Energy Networks for Cyclic Constraint Consistency");
    println!("Testing Structural OOD Generalization (Train: K=3..5 -> Test: K=3..32)");
    println!("================================================================================");

    let output_path = "EXP_TEN_006_RAW.tsv";
    let mut out_file = File::create(output_path).expect("Failed to create EXP_TEN_006_RAW.tsv");
    writeln!(
        out_file,
        "split\tcycle_k\tmodel\tnoise\tsteps\tseed\tinstance_id\texact_solve\tmean_node_acc\tlimit_cycle\twall_time_us"
    )
    .unwrap();

    let dim = 16;
    let latent_dim = 12;
    let num_relations = 4;
    let beta = 2.0;

    let world = PermutationWorld::new(dim, num_relations, 42);

    // -------------------------------------------------------------------------
    // 1. Model Initialization & Parameter Matching Check
    // -------------------------------------------------------------------------
    let mut energy_net = LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 100);
    let mut attn_net = RecurrentAttentionGraph::new(dim, num_relations, 200);
    let mut gnn_net = GNNMessagePassing::new(dim, num_relations, 300);

    println!("\n>>> MODEL PARAMETER BUDGETS:");
    println!(
        "  Candidate Learned Dual Energy Net: {} parameters",
        energy_net.num_parameters()
    );
    println!(
        "  Baseline Recurrent Attention Net:   {} parameters",
        attn_net.num_parameters()
    );
    println!(
        "  Baseline GNN Message Passing Net:   {} parameters",
        gnn_net.num_parameters()
    );

    // -------------------------------------------------------------------------
    // 2. Training Phase: Train on Short Cycles (K in {3, 4, 5})
    // -------------------------------------------------------------------------
    println!("\n>>> TRAINING MODELS ON SHORT CYCLES (K in 3..5)...");
    let mut train_rng = ChaCha8Rng::seed_from_u64(1000);
    let num_train_graphs = 60;
    let mut train_graphs = Vec::with_capacity(num_train_graphs);

    for idx in 0..num_train_graphs {
        let k = 3 + (idx % 3); // alternating K=3, 4, 5
        let noise = 0.20;
        train_graphs.push(world.generate_consistent_cycle(k, noise, &mut train_rng));
    }

    let num_epochs = 20;
    println!(
        "Training for {} epochs on {} graphs...",
        num_epochs, num_train_graphs
    );

    for epoch in 1..=num_epochs {
        let t0 = Instant::now();
        // Train Energy Net (BPTT 4 steps)
        let loss_energy = energy_net.train_step_bptt(&train_graphs, 0.015, 4, 0.05);
        // Train Recurrent Attention (4 steps)
        let loss_attn = attn_net.train_step(&train_graphs, 0.02, 4);
        // Train GNN (4 steps)
        let loss_gnn = gnn_net.train_step(&train_graphs, 0.02, 4);

        if epoch % 5 == 0 || epoch == 1 {
            println!(
                "  Epoch {:02}/{} [{:.1}s] -> Loss: Energy={:.4}, Attn={:.4}, GNN={:.4}",
                epoch,
                num_epochs,
                t0.elapsed().as_secs_f64(),
                loss_energy,
                loss_attn,
                loss_gnn
            );
        }
    }
    println!(">>> Training Complete!");

    // -------------------------------------------------------------------------
    // 3. Evaluation Phase: IID and Structural OOD Sizes
    // -------------------------------------------------------------------------
    let test_cycle_sizes = [3, 4, 5, 8, 16, 32];
    let noise_levels = [0.10, 0.20, 0.30];
    let step_options = [1, 2, 4, 8, 16, 32];
    let num_seeds = 20;

    let mut total_rows: Vec<BenchmarkRow> = Vec::new();

    for &cycle_k in &test_cycle_sizes {
        let split_name = if cycle_k <= 5 {
            "IID_Short"
        } else {
            "OOD_Size"
        };
        println!(
            "\n>>> EVALUATING ON CYCLE LENGTH K = {} ({}) across {} seeds...",
            cycle_k, split_name, num_seeds
        );

        for &noise in &noise_levels {
            for &steps in &step_options {
                let batch_results: Vec<Vec<BenchmarkRow>> = (0..num_seeds)
                    .into_par_iter()
                    .map(|seed_idx| {
                        let mut test_rng = ChaCha8Rng::seed_from_u64(
                            5000 + (seed_idx as u64) * 1000 + (cycle_k as u64) * 10,
                        );
                        let test_graph =
                            world.generate_consistent_cycle(cycle_k, noise, &mut test_rng);
                        let mut rows = Vec::new();

                        // 1. Learned Dual Energy Net
                        {
                            let t0 = Instant::now();
                            let res = energy_net.relax(&test_graph, steps, 0.05);
                            let final_s = &res.states;
                            let lim_cycle = res.limit_cycle;
                            let dt = t0.elapsed().as_secs_f64() * 1e6;

                            let mut exact_nodes = 0;
                            let mut total_acc = 0.0;
                            for v in 0..cycle_k {
                                let mut correct_bits = 0;
                                for i in 0..dim {
                                    let pred = if final_s[v][i] >= 0.0 { 1.0 } else { -1.0 };
                                    if (pred - test_graph.clean_states[v][i]).abs() < 1e-3 {
                                        correct_bits += 1;
                                    }
                                }
                                if correct_bits == dim {
                                    exact_nodes += 1;
                                }
                                total_acc += correct_bits as f64 / dim as f64;
                            }
                            let exact_solve = if exact_nodes == cycle_k { 1.0 } else { 0.0 };
                            let mean_acc = total_acc / cycle_k as f64;

                            rows.push(BenchmarkRow {
                                split: split_name.into(),
                                cycle_k,
                                model: "LearnedDualEnergy".into(),
                                noise,
                                steps,
                                seed: seed_idx as u64,
                                instance_id: seed_idx,
                                exact_solve,
                                mean_node_acc: mean_acc,
                                limit_cycle: if lim_cycle { 1.0 } else { 0.0 },
                                wall_time_us: dt,
                            });
                        }

                        // 2. Recurrent Attention
                        {
                            let t0 = Instant::now();
                            let (final_s, lim_cycle) = attn_net.unroll(&test_graph, steps);
                            let dt = t0.elapsed().as_secs_f64() * 1e6;

                            let mut exact_nodes = 0;
                            let mut total_acc = 0.0;
                            for v in 0..cycle_k {
                                let mut correct_bits = 0;
                                for i in 0..dim {
                                    let pred = if final_s[v][i] >= 0.0 { 1.0 } else { -1.0 };
                                    if (pred - test_graph.clean_states[v][i]).abs() < 1e-3 {
                                        correct_bits += 1;
                                    }
                                }
                                if correct_bits == dim {
                                    exact_nodes += 1;
                                }
                                total_acc += correct_bits as f64 / dim as f64;
                            }
                            let exact_solve = if exact_nodes == cycle_k { 1.0 } else { 0.0 };
                            let mean_acc = total_acc / cycle_k as f64;

                            rows.push(BenchmarkRow {
                                split: split_name.into(),
                                cycle_k,
                                model: "RecurrentAttention".into(),
                                noise,
                                steps,
                                seed: seed_idx as u64,
                                instance_id: seed_idx,
                                exact_solve,
                                mean_node_acc: mean_acc,
                                limit_cycle: if lim_cycle { 1.0 } else { 0.0 },
                                wall_time_us: dt,
                            });
                        }

                        // 3. GNN Message Passing
                        {
                            let t0 = Instant::now();
                            let (final_s, lim_cycle) = gnn_net.unroll(&test_graph, steps);
                            let dt = t0.elapsed().as_secs_f64() * 1e6;

                            let mut exact_nodes = 0;
                            let mut total_acc = 0.0;
                            for v in 0..cycle_k {
                                let mut correct_bits = 0;
                                for i in 0..dim {
                                    let pred = if final_s[v][i] >= 0.0 { 1.0 } else { -1.0 };
                                    if (pred - test_graph.clean_states[v][i]).abs() < 1e-3 {
                                        correct_bits += 1;
                                    }
                                }
                                if correct_bits == dim {
                                    exact_nodes += 1;
                                }
                                total_acc += correct_bits as f64 / dim as f64;
                            }
                            let exact_solve = if exact_nodes == cycle_k { 1.0 } else { 0.0 };
                            let mean_acc = total_acc / cycle_k as f64;

                            rows.push(BenchmarkRow {
                                split: split_name.into(),
                                cycle_k,
                                model: "GNN_MessagePassing".into(),
                                noise,
                                steps,
                                seed: seed_idx as u64,
                                instance_id: seed_idx,
                                exact_solve,
                                mean_node_acc: mean_acc,
                                limit_cycle: if lim_cycle { 1.0 } else { 0.0 },
                                wall_time_us: dt,
                            });
                        }

                        rows
                    })
                    .collect();

                for batch in batch_results {
                    for r in batch {
                        total_rows.push(r);
                    }
                }
            }
        }
    }

    // Write all rows to TSV
    for r in &total_rows {
        writeln!(
            out_file,
            "{}\t{}\t{}\t{:.2}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.2}",
            r.split,
            r.cycle_k,
            r.model,
            r.noise,
            r.steps,
            r.seed,
            r.instance_id,
            r.exact_solve,
            r.mean_node_acc,
            r.limit_cycle,
            r.wall_time_us
        )
        .unwrap();
    }

    // -------------------------------------------------------------------------
    // 4. Output Summary Tables
    // -------------------------------------------------------------------------
    println!("\n================================================================================");
    println!("SUMMARY: EXACT GRAPH SOLVE RATE (%) AT NOISE = 0.20 ACROSS INFERENCE STEPS T");
    println!("================================================================================");

    let models = [
        "LearnedDualEnergy",
        "RecurrentAttention",
        "GNN_MessagePassing",
    ];

    for &cycle_k in &test_cycle_sizes {
        println!(
            "\n--- CYCLE LENGTH K = {} ({}) ---",
            cycle_k,
            if cycle_k <= 5 { "IID" } else { "OOD" }
        );
        println!(
            "{:<22} | {:<7} | {:<7} | {:<7} | {:<7} | {:<7} | {:<7}",
            "Model", "T=1", "T=2", "T=4", "T=8", "T=16", "T=32"
        );
        println!(
            "{:-<22}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<7}-+-{:-<7}",
            "", "", "", "", "", "", ""
        );

        for m in models {
            let mut row_str = format!("{:<22}", m);
            for &s in &step_options {
                let matched: Vec<f64> = total_rows
                    .iter()
                    .filter(|r| {
                        r.model == m
                            && r.cycle_k == cycle_k
                            && (r.noise - 0.20).abs() < 1e-4
                            && r.steps == s
                    })
                    .map(|r| r.exact_solve)
                    .collect();
                if matched.is_empty() {
                    row_str.push_str(" |        ");
                } else {
                    let mean_val = (matched.iter().sum::<f64>() / matched.len() as f64) * 100.0;
                    row_str.push_str(&format!(" | {:5.1}%", mean_val));
                }
            }
            println!("{}", row_str);
        }
    }

    println!("\n================================================================================");
    println!("SUMMARY: LIMIT CYCLE OSCILLATION FREQUENCY (%) AT K = 16, NOISE = 0.20");
    println!("================================================================================");
    for m in models {
        let matched: Vec<f64> = total_rows
            .iter()
            .filter(|r| {
                r.model == m && r.cycle_k == 16 && (r.noise - 0.20).abs() < 1e-4 && r.steps == 16
            })
            .map(|r| r.limit_cycle)
            .collect();
        if !matched.is_empty() {
            let mean_val = (matched.iter().sum::<f64>() / matched.len() as f64) * 100.0;
            println!("  {:<24} : {:5.1}% limit cycles detected", m, mean_val);
        }
    }

    println!("\n>>> EXP-TEN-006A COMPLETE. Data streamed to EXP_TEN_006_RAW.tsv");
}
