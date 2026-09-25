//! Mathematical unit tests for strong baseline implementations (EXP-TEN-002).

use fundamental_ai::baselines::{
    ModernHopfield, NearestNeighborOracle, PolynomialDAM, PseudoinverseHopfield,
};
use fundamental_ai::learning::train_low_rank_cp;
use fundamental_ai::models::EnergyModel;
use fundamental_ai::types::SpinState;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[test]
fn test_pseudoinverse_stores_patterns_as_attractors() {
    let mut rng = ChaCha8Rng::seed_from_u64(101);
    let n = 20;
    let p = 6;
    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }

    let model = PseudoinverseHopfield::train(&patterns).expect("Training failed");

    // Verify all stored patterns are local minima
    for (idx, pat) in patterns.iter().enumerate() {
        for i in 0..n {
            let field = model.effective_field(pat, i);
            let si = pat.get(i) as f64;
            assert!(
                si * field > 0.0,
                "Pattern {} spin {} is unstable under pseudoinverse: field={}, spin={}",
                idx,
                i,
                field,
                si
            );
        }
    }
}

#[test]
fn test_nearest_neighbor_oracle_retrieval() {
    let mut rng = ChaCha8Rng::seed_from_u64(202);
    let n = 32;
    let p = 5;
    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }

    let oracle = NearestNeighborOracle::new(&patterns);

    // Corrupt pattern 2 by 3 bit flips
    let mut probe = patterns[2].clone();
    probe.flip(0);
    probe.flip(5);
    probe.flip(12);

    let (retrieved, retrieved_idx, _) = oracle.retrieve(&probe);
    assert_eq!(retrieved_idx, 2);
    assert_eq!(retrieved, patterns[2]);
}

#[test]
fn test_polynomial_dam_flip_delta_brute_force() {
    let mut rng = ChaCha8Rng::seed_from_u64(303);
    let n = 10;
    let p = 4;
    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }

    let model = PolynomialDAM::new(&patterns);

    for _ in 0..20 {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        let state = SpinState::from_slice(&spins);

        for i in 0..n {
            let e_before = model.energy(&state);
            let mut flipped = state.clone();
            flipped.flip(i);
            let e_after = model.energy(&flipped);
            let delta_brute = e_after - e_before;

            let delta_formula = model.flip_delta(&state, i);
            assert!(
                (delta_brute - delta_formula).abs() < 1e-10,
                "PolynomialDAM delta mismatch at spin {}: brute={}, formula={}",
                i,
                delta_brute,
                delta_formula
            );
        }
    }
}

#[test]
fn test_modern_hopfield_softmax_convergence() {
    let mut rng = ChaCha8Rng::seed_from_u64(404);
    let n = 32;
    let p = 5;
    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }

    let model = ModernHopfield::new(&patterns, 2.0);

    let mut probe = patterns[1].clone();
    probe.flip(2);
    probe.flip(7);

    let retrieved = model.retrieve(&probe, 10);
    assert_eq!(retrieved, patterns[1]);
}

#[test]
fn test_polynomial_dam_matches_low_rank_cp_dynamics() {
    let mut rng = ChaCha8Rng::seed_from_u64(505);
    let n = 12;
    let p = 3;
    let mut patterns = Vec::with_capacity(p);
    for _ in 0..p {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        patterns.push(SpinState::from_slice(&spins));
    }

    let dam_model = PolynomialDAM::new(&patterns);
    let cp_model = train_low_rank_cp(&patterns);

    // Test that the effective field direction sign(h_i) matches between DAM and CP
    for _ in 0..20 {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        let state = SpinState::from_slice(&spins);

        for i in 0..n {
            let f_dam = dam_model.effective_field(&state, i);
            let f_cp = cp_model.effective_field(&state, i);
            let sign_dam = if f_dam >= 0.0 { 1 } else { -1 };
            let sign_cp = if f_cp >= 0.0 { 1 } else { -1 };

            // When fields are non-negligible, signs must be identical
            if f_dam.abs() > 1e-4 && f_cp.abs() > 1e-4 {
                assert_eq!(
                    sign_dam, sign_cp,
                    "Field sign mismatch at spin {}: dam={}, cp={}",
                    i, f_dam, f_cp
                );
            }
        }
    }
}
