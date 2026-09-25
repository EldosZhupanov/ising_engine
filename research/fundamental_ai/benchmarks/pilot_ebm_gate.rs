//! Pilot EBM Gate Evaluation for EXP-TEN-006B.
//!
//! Tests whether trained AnalyticBilinearEnergyNet (via Analytic BPTT and EqProp)
//! achieves Net Rescue > +10% at T=8, and compares directly with classical
//! Spectral Synchronization and Loopy Min-Sum.

#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    unused_variables,
    dead_code
)]

use fundamental_ai::analytic_ebm::AnalyticBilinearEnergyNet;
use fundamental_ai::learned_energy::PermutationWorld;
use fundamental_ai::synchronization_audit::{LoopyMinSumSync, SpectralPermutationSync};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn main() {
    println!("===============================================================================");
    println!("PILOT GATE EVALUATION: EXP-TEN-006B");
    println!("Target: Net Rescue > +10% (Reasoning vs Inertia Criterion)");
    println!("===============================================================================");

    let dim = 16;
    let num_relations = 4;
    let noise_rate = 0.20;
    let world = PermutationWorld::new(dim, num_relations, 1001);

    let mut rng_train = ChaCha8Rng::seed_from_u64(2002);
    let mut rng_test = ChaCha8Rng::seed_from_u64(3003);

    let train_graphs: Vec<_> = (0..50)
        .map(|i| {
            let k = 3 + (i % 4); // lengths 3, 4, 5, 6
            world.generate_consistent_cycle(k, noise_rate, &mut rng_train)
        })
        .collect();

    let test_graphs: Vec<_> = (0..50)
        .map(|i| {
            let k = 3 + (i % 4);
            world.generate_consistent_cycle(k, noise_rate, &mut rng_test)
        })
        .collect();

    let eval_net = AnalyticBilinearEnergyNet::new(dim, num_relations, 1.0, 0.2, 999);
    let n_test = test_graphs.len() as f64;

    println!(
        "Dataset: 50 train graphs, 50 test graphs (K in [3, 6], d={}, noise={:.2})",
        dim, noise_rate
    );

    // 1. Classical Baselines on Test Set
    println!("\n--- Classical Synchronization Baselines (Test Set) ---");
    let spectral = SpectralPermutationSync::new(dim, world.permutations.clone());
    let min_sum = LoopyMinSumSync::new(dim, world.permutations.clone());

    let mut spectral_acc = 0.0;
    let mut spectral_rescue = 0.0;
    let mut spectral_damage = 0.0;

    let mut min_sum_acc = 0.0;
    let mut min_sum_rescue = 0.0;
    let mut min_sum_damage = 0.0;

    for g in &test_graphs {
        let s_spec = spectral.synchronize(g, 10, 0.5);
        let m_spec = eval_net.evaluate_metrics(g, &s_spec);
        spectral_acc += m_spec.accuracy;
        spectral_rescue += m_spec.rescue_rate;
        spectral_damage += m_spec.damage_rate;

        let s_ms = min_sum.solve(g, 10, 0.5);
        let m_ms = eval_net.evaluate_metrics(g, &s_ms);
        min_sum_acc += m_ms.accuracy;
        min_sum_rescue += m_ms.rescue_rate;
        min_sum_damage += m_ms.damage_rate;
    }
    spectral_acc /= n_test;
    spectral_rescue /= n_test;
    spectral_damage /= n_test;
    min_sum_acc /= n_test;
    min_sum_rescue /= n_test;
    min_sum_damage /= n_test;

    println!("Spectral Permutation Sync (T=10) : Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
        spectral_acc * 100.0, spectral_rescue * 100.0, spectral_damage * 100.0, (spectral_rescue - spectral_damage) * 100.0);
    println!("Loopy Min-Sum BP (T=10)         : Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
        min_sum_acc * 100.0, min_sum_rescue * 100.0, min_sum_damage * 100.0, (min_sum_rescue - min_sum_damage) * 100.0);

    // 2. Untrained EBM (Random Initial Weights)
    println!("\n--- Untrained Energy Net (Random Weights) ---");
    let untrained_ebm = AnalyticBilinearEnergyNet::new(dim, num_relations, 1.0, 0.2, 42);
    for &t in &[0, 1, 2, 4, 8, 16] {
        let mut m_acc = 0.0;
        let mut m_rescue = 0.0;
        let mut m_damage = 0.0;
        for g in &test_graphs {
            let s = untrained_ebm.relax(g, t, 0.05);
            let m = untrained_ebm.evaluate_metrics(g, &s);
            m_acc += m.accuracy;
            m_rescue += m.rescue_rate;
            m_damage += m.damage_rate;
        }
        m_acc /= n_test;
        m_rescue /= n_test;
        m_damage /= n_test;
        println!(
            "Untrained T={:2} : Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%",
            t,
            m_acc * 100.0,
            m_rescue * 100.0,
            m_damage * 100.0,
            (m_rescue - m_damage) * 100.0
        );
    }

    // 3. Train with Analytical BPTT (TRAIN-A)
    println!("\n--- Training Analytic Energy Net with Analytical BPTT (TRAIN-A) ---");
    let mut bptt_ebm = AnalyticBilinearEnergyNet::new(dim, num_relations, 1.0, 0.2, 42);
    let epochs = 80;
    let unroll_steps = 4;
    let relax_lr = 0.05;
    let train_lr = 0.05;

    for ep in 1..=epochs {
        let loss =
            bptt_ebm.train_step_analytic_bptt(&train_graphs, train_lr, unroll_steps, relax_lr);
        if ep % 20 == 0 || ep == 1 {
            println!("Epoch {:3}/{}: Training Loss = {:.6}", ep, epochs, loss);
        }
    }

    println!("\nEvaluation on Held-Out Test Graphs (BPTT-Trained Model):");
    let mut bptt_net_rescue_t8 = 0.0;
    for &t in &[0, 1, 2, 4, 8, 16] {
        let mut m_acc = 0.0;
        let mut m_rescue = 0.0;
        let mut m_damage = 0.0;
        let mut m_cst = 0.0;
        for g in &test_graphs {
            let s = bptt_ebm.relax(g, t, relax_lr);
            let m = bptt_ebm.evaluate_metrics(g, &s);
            m_acc += m.accuracy;
            m_rescue += m.rescue_rate;
            m_damage += m.damage_rate;
            m_cst += m.constraint_satisfaction;
        }
        m_acc /= n_test;
        m_rescue /= n_test;
        m_damage /= n_test;
        m_cst /= n_test;
        let net = (m_rescue - m_damage) * 100.0;
        if t == 8 {
            bptt_net_rescue_t8 = net;
        }
        println!("BPTT-Model T={:2} : Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%, CstSat={:.1}%",
            t, m_acc * 100.0, m_rescue * 100.0, m_damage * 100.0, net, m_cst * 100.0);
    }

    // 4. Train with Equilibrium Propagation (TRAIN-C)
    println!("\n--- Training Analytic Energy Net with Equilibrium Propagation (TRAIN-C) ---");
    let mut eqprop_ebm = AnalyticBilinearEnergyNet::new(dim, num_relations, 1.0, 0.2, 42);
    let eq_epochs = 80;
    let eq_train_lr = 0.02;
    let beta_nudge = 0.2;

    for ep in 1..=eq_epochs {
        let loss =
            eqprop_ebm.train_step_eq_prop(&train_graphs, eq_train_lr, 8, 8, 0.05, beta_nudge);
        if ep % 20 == 0 || ep == 1 {
            println!(
                "Epoch {:3}/{}: EqProp Training Loss = {:.6}",
                ep, eq_epochs, loss
            );
        }
    }

    println!("\nEvaluation on Held-Out Test Graphs (EqProp-Trained Model):");
    let mut eqprop_net_rescue_t8 = 0.0;
    for &t in &[0, 1, 2, 4, 8, 16] {
        let mut m_acc = 0.0;
        let mut m_rescue = 0.0;
        let mut m_damage = 0.0;
        let mut m_cst = 0.0;
        for g in &test_graphs {
            let s = eqprop_ebm.relax(g, t, 0.05);
            let m = eqprop_ebm.evaluate_metrics(g, &s);
            m_acc += m.accuracy;
            m_rescue += m.rescue_rate;
            m_damage += m.damage_rate;
            m_cst += m.constraint_satisfaction;
        }
        m_acc /= n_test;
        m_rescue /= n_test;
        m_damage /= n_test;
        m_cst /= n_test;
        let net = (m_rescue - m_damage) * 100.0;
        if t == 8 {
            eqprop_net_rescue_t8 = net;
        }
        println!("EqProp-Model T={:2} : Acc={:.1}%, Rescue={:.1}%, Damage={:.1}%, NetRescue={:+.1}%, CstSat={:.1}%",
            t, m_acc * 100.0, m_rescue * 100.0, m_damage * 100.0, net, m_cst * 100.0);
    }

    // 5. Decision Gate Evaluation
    println!("\n===============================================================================");
    println!("PILOT DECISION GATE SUMMARY:");
    println!("BPTT Model Net Rescue (T=8)  : {:+.1}%", bptt_net_rescue_t8);
    println!(
        "EqProp Model Net Rescue (T=8): {:+.1}%",
        eqprop_net_rescue_t8
    );
    println!("Pilot Gate Threshold         : > +10.0%");

    if bptt_net_rescue_t8 > 10.0 || eqprop_net_rescue_t8 > 10.0 {
        println!(
            "DECISION: PASS PILOT GATE. Active reasoning detected. Proceed to full benchmark."
        );
    } else {
        println!("DECISION: FAIL PILOT GATE. Net Rescue <= +10%. Diagnosing dynamics.");
    }
    println!("===============================================================================");
}
