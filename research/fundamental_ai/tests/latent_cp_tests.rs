use fundamental_ai::*;

#[test]
fn test_compositional_grammar_generation() {
    let n = 64;
    let k = 6; // 2^(6-1) = 32 valid patterns
    let p_train = 24;
    let seed = 42;

    let (train, unseen, g) = generate_compositional_grammar_suite(n, k, p_train, seed);
    assert_eq!(train.len(), 24);
    assert_eq!(unseen.len(), 8);
    assert_eq!(g.len(), n * k);

    // Unseen patterns must not overlap with train
    for u in &unseen {
        for t in &train {
            assert_ne!(u.spins, t.spins, "Unseen pattern found in train set!");
        }
    }
}

#[test]
fn test_svd_covariance_monotonic_eigenvalues() {
    let n = 64;
    let k = 5;
    let (train, _, _) = generate_compositional_grammar_suite(n, k, 12, 101);
    let rank = 8;

    let (_, lambdas) = compute_covariance_eigenvectors(&train, rank);
    assert_eq!(lambdas.len(), rank);
    // Eigenvalues should be approximately non-increasing
    for i in 1..lambdas.len() {
        assert!(
            lambdas[i] <= lambdas[i - 1] + 1e-4,
            "Eigenvalues not decreasing: {} > {}",
            lambdas[i],
            lambdas[i - 1]
        );
    }
}

#[test]
fn test_factor_diagnostics_distributed_latents() {
    let n = 128;
    let k = 8;
    let p_train = 100;
    let (train, _, _) = generate_compositional_grammar_suite(n, k, p_train, 202);
    let rank = 8;

    let model = train_low_rank_cp_svd(&train, rank);
    let max_sims = compute_factor_max_similarity(&model, &train);
    let prs = compute_factor_participation_ratios(&model, &train);
    let entropy = compute_factor_entropy(&model, &train);

    assert_eq!(max_sims.len(), rank);
    assert_eq!(prs.len(), rank);

    // For distributed latents, max similarity to any single memory should be well below 1.0
    for &ms in &max_sims {
        assert!(ms < 0.85, "Factor is an exemplar copy, MaxSim = {}", ms);
    }

    // Participation ratio should be well above 1.0 (factors active across multiple memories)
    for &pr in &prs {
        assert!(pr > 2.0, "Factor is localized to < 2 memories, PR = {}", pr);
    }

    // Entropy should be positive and bounded in [0, 1]
    assert!(
        entropy > 0.1 && entropy <= 1.0,
        "Entropy invalid: {}",
        entropy
    );
}

#[test]
fn test_linear_svd_subspace_projection() {
    let n = 64;
    let k = 6;
    let (train, _unseen, _) = generate_compositional_grammar_suite(n, k, 24, 303);
    let proj = LinearSVDSubspace::from_patterns(&train, 6);

    // On uncorrupted train pattern, projection should reconstruct exactly or near exactly
    let p0 = &train[0];
    let rec0 = proj.project(p0);
    let hd = p0.hamming_distance(&rec0);
    assert!(hd <= 2, "Reconstruction Hamming distance too high: {}", hd);
}
