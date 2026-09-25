//! Mathematical unit tests for parameterized energy laws and cyclic networks.

use fundamental_ai::models::{
    ClosedTriadMemory, EnergyLaw, EnergyModel, LatentEnergyModel, PairwiseCycleHopfield,
};
use fundamental_ai::types::SpinState;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[test]
fn test_latent_energy_flip_delta_brute_force_all_laws() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let n = 16;
    let rank = 4;

    let mut basis = vec![0.0; n * rank];
    for val in basis.iter_mut() {
        *val = rng.gen_range(-1.0..1.0);
    }
    let lambdas = vec![1.0, 0.8, 0.5, 0.3];

    let laws = [
        EnergyLaw::Cubic,
        EnergyLaw::Quartic,
        EnergyLaw::Sextic,
        EnergyLaw::LogCosh { beta: 1.5 },
        EnergyLaw::Rational { gamma: 0.5 },
    ];

    let mut spins = vec![1i8; n];
    for s in spins.iter_mut() {
        if rng.gen_bool(0.5) {
            *s = -1;
        }
    }
    let state = SpinState::from_slice(&spins);

    for law in laws {
        let model = LatentEnergyModel::new(n, rank, basis.clone(), lambdas.clone(), law);
        let e_init = model.energy(&state);

        for i in 0..n {
            let delta_e_formula = model.flip_delta(&state, i);

            let mut flipped_state = state.clone();
            flipped_state.flip(i);
            let e_flipped = model.energy(&flipped_state);
            let delta_e_brute = e_flipped - e_init;

            assert!(
                (delta_e_formula - delta_e_brute).abs() < 1e-9,
                "Failed on law {:?}, spin {}: formula={}, brute={}",
                law,
                i,
                delta_e_formula,
                delta_e_brute
            );
        }
    }
}

#[test]
fn test_energy_monotonicity_greedy_descent() {
    let mut rng = ChaCha8Rng::seed_from_u64(1234);
    let n = 32;
    let rank = 6;

    let mut basis = vec![0.0; n * rank];
    for val in basis.iter_mut() {
        *val = rng.gen_range(-1.0..1.0);
    }
    let lambdas = vec![1.0; rank];

    let laws = [
        EnergyLaw::Quartic,
        EnergyLaw::LogCosh { beta: 2.0 },
        EnergyLaw::Rational { gamma: 0.1 },
    ];

    for law in laws {
        let model = LatentEnergyModel::new(n, rank, basis.clone(), lambdas.clone(), law);
        let mut spins = vec![1i8; n];
        for s in spins.iter_mut() {
            if rng.gen_bool(0.5) {
                *s = -1;
            }
        }
        let state = SpinState::from_slice(&spins);

        let e_before = model.energy(&state);
        let (relaxed, flips) = model.relax_greedy(&state, 100);
        let e_after = model.energy(&relaxed);

        assert!(
            e_after <= e_before + 1e-9,
            "Greedy relaxation violated monotonicity for {:?}: e_before={}, e_after={}",
            law,
            e_before,
            e_after
        );
        if flips > 0 {
            assert!(e_after < e_before);
        }
    }
}

#[test]
fn test_sign_preservation_vs_sign_erasure() {
    // 1D test factor: u = [1.0, 1.0, 1.0, 1.0]
    let n = 4;
    let rank = 1;
    let basis = vec![1.0; n * rank];
    let lambdas = vec![1.0];

    // Negative state: s = [-1, -1, -1, -1] -> overlap u . s = -4.0
    let neg_state = SpinState::new(n, -1);

    // 1. Cubic (p=3): derivative is x^2 = (-4)^2 = +16. Field is POSITIVE!
    let model_cubic =
        LatentEnergyModel::new(n, rank, basis.clone(), lambdas.clone(), EnergyLaw::Cubic);
    let h_cubic = model_cubic.effective_field(&neg_state, 0);
    assert!(
        h_cubic > 0.0,
        "Cubic law should produce positive field due to sign erasure: found {}",
        h_cubic
    );

    // 2. Quartic (p=4): derivative is x^3 = (-4)^3 = -64. Field is NEGATIVE!
    let model_quartic =
        LatentEnergyModel::new(n, rank, basis.clone(), lambdas.clone(), EnergyLaw::Quartic);
    let h_quartic = model_quartic.effective_field(&neg_state, 0);
    assert!(
        h_quartic < 0.0,
        "Quartic law must preserve negative sign: found {}",
        h_quartic
    );

    // 3. LogCosh: derivative is tanh(beta * x). For x = -4, tanh is negative!
    let model_logcosh = LatentEnergyModel::new(
        n,
        rank,
        basis.clone(),
        lambdas.clone(),
        EnergyLaw::LogCosh { beta: 1.0 },
    );
    let h_logcosh = model_logcosh.effective_field(&neg_state, 0);
    assert!(
        h_logcosh < 0.0,
        "LogCosh law must preserve negative sign: found {}",
        h_logcosh
    );

    // 4. Rational: derivative is 2x / (1 + gamma x^2)^2. For x = -4, negative!
    let model_rat =
        LatentEnergyModel::new(n, rank, basis, lambdas, EnergyLaw::Rational { gamma: 0.1 });
    let h_rat = model_rat.effective_field(&neg_state, 0);
    assert!(
        h_rat < 0.0,
        "Rational law must preserve negative sign: found {}",
        h_rat
    );
}

#[test]
fn test_closed_triad_clean_fixed_point() {
    let n_e = 32;
    let n_r = 16;
    let m = 4;

    let mut entities_a = Vec::with_capacity(m);
    let mut entities_b = Vec::with_capacity(m);
    let mut entities_c = Vec::with_capacity(m);

    // Create strictly orthogonal and zero-mean balanced entities (Walsh-Hadamard basis)
    for mu in 0..m {
        let block_size = 1 << (4 - mu);
        let mut sa = vec![1i8; n_e];
        let mut sb = vec![1i8; n_e];
        let mut sc = vec![1i8; n_e];
        for i in 0..n_e {
            if (i / block_size) % 2 == 1 {
                sa[i] = -1;
                sb[i] = -1;
                sc[i] = -1;
            }
        }
        entities_a.push(SpinState::from_slice(&sa));
        entities_b.push(SpinState::from_slice(&sb));
        entities_c.push(SpinState::from_slice(&sc));
    }

    let r1 = SpinState::new(n_r, 1);
    let r2 = SpinState::new(n_r, 1);
    let r3 = SpinState::new(n_r, 1);

    let triad_mem = ClosedTriadMemory::new(
        n_e,
        n_r,
        entities_a.clone(),
        entities_b.clone(),
        entities_c.clone(),
        r1,
        r2,
        r3,
        10.0,
    );

    // Test that for target triad 0, clean inputs remain a stable fixed point
    let target_a = &entities_a[0];
    let target_b = &entities_b[0];
    let target_c = &entities_c[0];

    let (rel_a, rel_b, rel_c) = triad_mem.relax_energy(target_a, target_b, target_c, 3);
    assert_eq!(rel_a, *target_a);
    assert_eq!(rel_b, *target_b);
    assert_eq!(rel_c, *target_c);

    // Check PairwiseCycleHopfield clean fixed point
    let pairwise = PairwiseCycleHopfield::train(&entities_a, &entities_b, &entities_c);
    let (p_a, p_b, p_c) = pairwise.relax(target_a, target_b, target_c, 3);
    assert_eq!(p_a, *target_a);
    assert_eq!(p_b, *target_b);
    assert_eq!(p_c, *target_c);
}
