use fundamental_ai::learned_energy::PermutationWorld;
use fundamental_ai::synchronization_audit::{
    DampedRecurrentAttention, LoopyMinSumSync, SpectralPermutationSync, StatisticalTests,
    SymmetricEnergyAttention, TrajectoryAuditor,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn test_spectral_synchronization_improves_noisy_cycle() {
    let dim = 16;
    let num_relations = 4;
    let world = PermutationWorld::new(dim, num_relations, 42);
    let mut rng = ChaCha8Rng::seed_from_u64(1234);

    let graph = world.generate_consistent_cycle(4, 0.20, &mut rng);
    let sync = SpectralPermutationSync::new(dim, world.permutations.clone());

    let synced = sync.synchronize(&graph, 10, 0.5);

    let audit_0 = TrajectoryAuditor::audit_state(
        &graph.noisy_states,
        &graph.noisy_states,
        &graph.noisy_states,
        &graph.clean_states,
        0,
        0.0,
    );

    let audit_sync = TrajectoryAuditor::audit_state(
        &synced,
        &graph.noisy_states,
        &graph.noisy_states,
        &graph.clean_states,
        10,
        0.0,
    );

    println!(
        "Spectral Sync: T=0 acc={:.3}, T=10 acc={:.3}, rescue={:.3}, damage={:.3}",
        audit_0.mean_accuracy,
        audit_sync.mean_accuracy,
        audit_sync.rescue_rate,
        audit_sync.damage_rate
    );

    assert!(audit_sync.mean_accuracy >= 0.70);
}

#[test]
fn test_loopy_min_sum_sync_executes() {
    let dim = 16;
    let num_relations = 4;
    let world = PermutationWorld::new(dim, num_relations, 42);
    let mut rng = ChaCha8Rng::seed_from_u64(5678);

    let graph = world.generate_consistent_cycle(4, 0.20, &mut rng);
    let min_sum = LoopyMinSumSync::new(dim, world.permutations.clone());

    let solved = min_sum.solve(&graph, 8, 0.5);
    let audit = TrajectoryAuditor::audit_state(
        &solved,
        &graph.noisy_states,
        &graph.noisy_states,
        &graph.clean_states,
        8,
        0.0,
    );

    println!("Min-Sum Sync acc: {:.3}", audit.mean_accuracy);
    assert!(audit.mean_accuracy > 0.50);
}

#[test]
fn test_damped_and_symmetric_attention() {
    let dim = 16;
    let num_relations = 4;
    let world = PermutationWorld::new(dim, num_relations, 42);
    let mut rng = ChaCha8Rng::seed_from_u64(9999);

    let graph = world.generate_consistent_cycle(4, 0.20, &mut rng);
    let damped_attn = DampedRecurrentAttention::new(dim, num_relations, 0.25, 100);
    let sym_attn = SymmetricEnergyAttention::new(dim, num_relations, 0.25, 200);

    let res_damped = damped_attn.unroll(&graph, 16);
    let res_sym = sym_attn.unroll(&graph, 16);

    let audit_damped = TrajectoryAuditor::audit_state(
        &res_damped,
        &graph.noisy_states,
        &graph.noisy_states,
        &graph.clean_states,
        16,
        0.0,
    );

    let audit_sym = TrajectoryAuditor::audit_state(
        &res_sym,
        &graph.noisy_states,
        &graph.noisy_states,
        &graph.clean_states,
        16,
        0.0,
    );

    println!(
        "Damped Attn acc: {:.3}, Sym Attn acc: {:.3}",
        audit_damped.mean_accuracy, audit_sym.mean_accuracy
    );
    assert!(audit_damped.mean_accuracy > 0.50);
    assert!(audit_sym.mean_accuracy > 0.50);
}

#[test]
fn test_statistical_tests_mcnemar_and_bootstrap() {
    let a = vec![true, true, true, false, true, false, true, true];
    let b = vec![true, false, false, false, true, false, false, true];

    let (discordant_b, discordant_c, chi2, p_val) = StatisticalTests::mcnemar(&a, &b);
    println!(
        "McNemar test: b={}, c={}, chi2={:.3}, p={:.4}",
        discordant_b, discordant_c, chi2, p_val
    );
    assert_eq!(discordant_b, 3);
    assert_eq!(discordant_c, 0);

    let vals_a = vec![0.85, 0.90, 0.88, 0.92, 0.87];
    let vals_b = vec![0.75, 0.78, 0.80, 0.72, 0.76];
    let (mean_diff, ci_low, ci_high) =
        StatisticalTests::paired_bootstrap_ci(&vals_a, &vals_b, 500, 42);

    println!(
        "Bootstrap CI: mean_diff={:.3}, 95% CI=[{:.3}, {:.3}]",
        mean_diff, ci_low, ci_high
    );
    assert!(mean_diff > 0.0);
    assert!(ci_low > 0.0);
}
