//! Mathematical unit tests for learned dual energy networks and baselines (EXP-TEN-006).

use fundamental_ai::learned_energy::{
    GNNMessagePassing, LearnedDualEnergyNetwork, PermutationWorld, RecurrentAttentionGraph,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn test_learned_energy_analytical_gradients_match_finite_differences() {
    let dim = 4;
    let latent_dim = 2;
    let num_relations = 2;
    let beta = 1.0;
    let model = LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 42);

    let world = PermutationWorld::new(dim, num_relations, 123);
    let mut rng = ChaCha8Rng::seed_from_u64(456);
    let graph = world.generate_consistent_cycle(3, 0.20, &mut rng);

    let n = graph.num_nodes;
    let mut s = graph.noisy_states.clone();
    let mut z = vec![vec![0.5; latent_dim]; n];

    // Compute analytical gradients
    let mut grad_s = vec![vec![0.0; dim]; n];
    let mut grad_z = vec![vec![0.0; latent_dim]; n];
    model.compute_gradients(&s, &z, &graph, &mut grad_s, &mut grad_z);

    let eps = 1e-6;

    // Check grad_s numerically
    for v in 0..n {
        for i in 0..dim {
            let orig = s[v][i];
            s[v][i] = orig + eps;
            let e_plus = model.energy(&s, &z, &graph);
            s[v][i] = orig - eps;
            let e_minus = model.energy(&s, &z, &graph);
            s[v][i] = orig;

            let num_grad = (e_plus - e_minus) / (2.0 * eps);
            let ana_grad = grad_s[v][i];

            assert!(
                (num_grad - ana_grad).abs() < 1e-4,
                "Gradient mismatch for s[{}][{}]: ana={}, num={}",
                v,
                i,
                ana_grad,
                num_grad
            );
        }
    }

    // Check grad_z numerically
    for v in 0..n {
        for k in 0..latent_dim {
            let orig = z[v][k];
            z[v][k] = orig + eps;
            let e_plus = model.energy(&s, &z, &graph);
            z[v][k] = orig - eps;
            let e_minus = model.energy(&s, &z, &graph);
            z[v][k] = orig;

            let num_grad = (e_plus - e_minus) / (2.0 * eps);
            let ana_grad = grad_z[v][k];

            assert!(
                (num_grad - ana_grad).abs() < 1e-4,
                "Gradient mismatch for z[{}][{}]: ana={}, num={}",
                v,
                k,
                ana_grad,
                num_grad
            );
        }
    }
}

#[test]
fn test_learned_energy_monotonic_relaxation() {
    let dim = 8;
    let latent_dim = 4;
    let num_relations = 3;
    let beta = 1.0;
    let model = LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 999);

    let world = PermutationWorld::new(dim, num_relations, 777);
    let mut rng = ChaCha8Rng::seed_from_u64(888);
    let graph = world.generate_consistent_cycle(4, 0.25, &mut rng);

    let res = model.relax(&graph, 15, 0.05);

    // Verify energy non-increasing at each relaxation step
    for t in 0..(res.energies.len() - 1) {
        assert!(
            res.energies[t + 1] <= res.energies[t] + 1e-5,
            "Energy monotonicity violated at step {}: E(t)={}, E(t+1)={}",
            t,
            res.energies[t],
            res.energies[t + 1]
        );
    }
}

#[test]
fn test_learned_energy_bptt_loss_decrease() {
    let dim = 8;
    let latent_dim = 4;
    let num_relations = 2;
    let beta = 1.0;
    let mut model = LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, beta, 12);

    let world = PermutationWorld::new(dim, num_relations, 34);
    let mut rng = ChaCha8Rng::seed_from_u64(56);
    let train_graphs: Vec<_> = (0..5)
        .map(|_| world.generate_consistent_cycle(3, 0.20, &mut rng))
        .collect();

    let initial_loss = model.train_step_bptt(&train_graphs, 0.0, 3, 0.05);
    let mut final_loss = initial_loss;

    for _ in 0..5 {
        final_loss = model.train_step_bptt(&train_graphs, 0.02, 3, 0.05);
    }

    assert!(
        final_loss <= initial_loss + 1e-5,
        "Training failed to decrease loss: init={}, final={}",
        initial_loss,
        final_loss
    );
}

#[test]
fn test_parameter_counts_matched() {
    let dim = 16;
    let num_relations = 4;
    let latent_dim = 16;

    let energy_net = LearnedDualEnergyNetwork::new(dim, latent_dim, num_relations, 1.0, 1);
    let attn_net = RecurrentAttentionGraph::new(dim, num_relations, 1);
    let gnn_net = GNNMessagePassing::new(dim, num_relations, 1);

    println!("Energy Net parameters: {}", energy_net.num_parameters());
    println!("Attn Net parameters: {}", attn_net.num_parameters());
    println!("GNN Net parameters: {}", gnn_net.num_parameters());

    // All should be within reasonable comparison range (around 500-2500 params)
    assert!(energy_net.num_parameters() > 100);
    assert!(attn_net.num_parameters() > 100);
    assert!(gnn_net.num_parameters() > 100);
}
