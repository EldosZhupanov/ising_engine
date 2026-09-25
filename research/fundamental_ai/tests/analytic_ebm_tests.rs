//! Mathematical tests verifying analytical gradients and learning dynamics for AnalyticBilinearEnergyNet.

#![allow(clippy::needless_range_loop)]

use fundamental_ai::analytic_ebm::AnalyticBilinearEnergyNet;
use fundamental_ai::learned_energy::PermutationWorld;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn test_analytic_state_gradient_matches_finite_differences() {
    let dim = 4;
    let num_relations = 2;
    let beta = 1.0;
    let lambda_obs = 0.5;
    let model = AnalyticBilinearEnergyNet::new(dim, num_relations, beta, lambda_obs, 42);

    let world = PermutationWorld::new(dim, num_relations, 123);
    let mut rng = ChaCha8Rng::seed_from_u64(456);
    let graph = world.generate_consistent_cycle(4, 0.20, &mut rng);

    let n = graph.num_nodes;
    let mut s = graph.noisy_states.clone();

    // Compute analytical state gradient
    let mut grad_s = vec![vec![0.0; dim]; n];
    model.compute_state_gradient(&s, &graph, &mut grad_s);

    let eps = 1e-6;
    for v in 0..n {
        for i in 0..dim {
            let orig = s[v][i];
            s[v][i] = orig + eps;
            let e_plus = model.energy(&s, &graph);
            s[v][i] = orig - eps;
            let e_minus = model.energy(&s, &graph);
            s[v][i] = orig;

            let num_grad = (e_plus - e_minus) / (2.0 * eps);
            let ana_grad = grad_s[v][i];

            assert!(
                (num_grad - ana_grad).abs() < 1e-4,
                "State gradient mismatch at v={}, i={}: ana={}, num={}",
                v,
                i,
                ana_grad,
                num_grad
            );
        }
    }
}

#[test]
fn test_analytic_param_gradient_matches_finite_differences() {
    let dim = 4;
    let num_relations = 2;
    let beta = 1.0;
    let lambda_obs = 0.5;
    let mut model = AnalyticBilinearEnergyNet::new(dim, num_relations, beta, lambda_obs, 42);

    let world = PermutationWorld::new(dim, num_relations, 123);
    let mut rng = ChaCha8Rng::seed_from_u64(456);
    let graph = world.generate_consistent_cycle(4, 0.20, &mut rng);

    let s = graph.noisy_states.clone();

    let mut grad_w = vec![0.0; model.w.len()];
    model.compute_param_gradient(&s, &graph, &mut grad_w);

    let eps = 1e-6;
    for idx in 0..model.w.len() {
        let orig = model.w[idx];
        model.w[idx] = orig + eps;
        let e_plus = model.energy(&s, &graph);
        model.w[idx] = orig - eps;
        let e_minus = model.energy(&s, &graph);
        model.w[idx] = orig;

        let num_grad = (e_plus - e_minus) / (2.0 * eps);
        let ana_grad = grad_w[idx];

        assert!(
            (num_grad - ana_grad).abs() < 1e-4,
            "Param gradient mismatch at idx={}: ana={}, num={}",
            idx,
            ana_grad,
            num_grad
        );
    }
}

#[test]
fn test_analytic_bptt_gradient_matches_finite_differences() {
    let dim = 4;
    let num_relations = 2;
    let beta = 1.0;
    let lambda_obs = 0.5;
    let mut model = AnalyticBilinearEnergyNet::new(dim, num_relations, beta, lambda_obs, 42);

    let world = PermutationWorld::new(dim, num_relations, 123);
    let mut rng = ChaCha8Rng::seed_from_u64(456);
    let graph = world.generate_consistent_cycle(3, 0.20, &mut rng);

    let unroll_steps = 3;
    let relax_lr = 0.05;

    let mut ana_grad_w = vec![0.0; model.w.len()];
    let _loss = model.compute_bptt_grad_single(&graph, unroll_steps, relax_lr, &mut ana_grad_w);

    let eps = 1e-5;
    for idx in 0..model.w.len() {
        let orig = model.w[idx];

        model.w[idx] = orig + eps;
        let s_plus = model.relax(&graph, unroll_steps, relax_lr);
        let mut loss_plus = 0.0;
        let nd_f = (graph.num_nodes * dim) as f64;
        for v in 0..graph.num_nodes {
            for i in 0..dim {
                let diff = s_plus[v][i] - graph.clean_states[v][i];
                loss_plus += diff * diff / nd_f;
            }
        }

        model.w[idx] = orig - eps;
        let s_minus = model.relax(&graph, unroll_steps, relax_lr);
        let mut loss_minus = 0.0;
        for v in 0..graph.num_nodes {
            for i in 0..dim {
                let diff = s_minus[v][i] - graph.clean_states[v][i];
                loss_minus += diff * diff / nd_f;
            }
        }

        model.w[idx] = orig;

        let num_grad = (loss_plus - loss_minus) / (2.0 * eps);
        let ana_grad = ana_grad_w[idx];

        assert!(
            (num_grad - ana_grad).abs() < 1e-3,
            "BPTT gradient mismatch at idx={}: ana={}, num={}, diff={}",
            idx,
            ana_grad,
            num_grad,
            (num_grad - ana_grad).abs()
        );
    }
}

#[test]
fn test_analytic_bptt_loss_decrease() {
    let dim = 4;
    let num_relations = 2;
    let beta = 1.0;
    let lambda_obs = 0.5;
    let mut model = AnalyticBilinearEnergyNet::new(dim, num_relations, beta, lambda_obs, 42);

    let world = PermutationWorld::new(dim, num_relations, 123);
    let mut rng = ChaCha8Rng::seed_from_u64(456);
    let graphs: Vec<_> = (0..10)
        .map(|_| world.generate_consistent_cycle(4, 0.20, &mut rng))
        .collect();

    let initial_loss = model.train_step_analytic_bptt(&graphs, 0.0, 4, 0.05);

    let mut loss = initial_loss;
    for _ in 0..20 {
        loss = model.train_step_analytic_bptt(&graphs, 0.1, 4, 0.05);
    }

    assert!(
        loss < initial_loss,
        "Analytical BPTT failed to reduce loss: init={}, final={}",
        initial_loss,
        loss
    );
}

#[test]
fn test_analytic_eq_prop_loss_decrease() {
    let dim = 4;
    let num_relations = 2;
    let beta = 1.0;
    let lambda_obs = 0.5;
    let mut model = AnalyticBilinearEnergyNet::new(dim, num_relations, beta, lambda_obs, 42);

    let world = PermutationWorld::new(dim, num_relations, 123);
    let mut rng = ChaCha8Rng::seed_from_u64(456);
    let graphs: Vec<_> = (0..10)
        .map(|_| world.generate_consistent_cycle(4, 0.20, &mut rng))
        .collect();

    let initial_loss = model.train_step_eq_prop(&graphs, 0.0, 8, 8, 0.05, 0.1);

    let mut loss = initial_loss;
    for _ in 0..20 {
        loss = model.train_step_eq_prop(&graphs, 0.05, 8, 8, 0.05, 0.1);
    }

    assert!(
        loss < initial_loss,
        "EqProp failed to reduce loss: init={}, final={}",
        initial_loss,
        loss
    );
}
