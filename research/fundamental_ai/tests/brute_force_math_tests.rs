//! Rigorous Brute-Force Mathematical Verification Tests.
//!
//! Verifies for all models (Pairwise, DenseTensor3, SparseHyperedge, LowRankCP) that:
//! 1. Delta E_i = 2 * s_i * h_i^eff identically equals E(s^(i)) - E(s) to float precision.
//! 2. Greedy coordinate descent is strictly monotonic (Delta E <= 0).
//! 3. LowRankCP energy and fields identically match DenseTensor3 contraction.

use fundamental_ai::dynamics::greedy_descent;
use fundamental_ai::learning::{
    train_low_rank_cp, train_pairwise_hebbian, train_sparse_hyperedge3_budgeted,
};
use fundamental_ai::models::{
    DenseTensor3Hopfield, EnergyModel, LowRankCPMemory, PairwiseHopfield, SparseHyperedgeMemory,
};
use fundamental_ai::types::{Hyperedge3, Hyperedge4, SpinState};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[test]
fn test_pairwise_flip_delta_brute_force() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let n = 12;
    let mut model = PairwiseHopfield::new(n);

    // Random fields and couplings
    for i in 0..n {
        model.h[i] = rng.gen_range(-2.0..2.0);
        for j in (i + 1)..n {
            model.set_j(i, j, rng.gen_range(-1.5..1.5));
        }
    }

    // Test across 50 random states
    for _ in 0..50 {
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
                "Pairwise delta mismatch at spin {}: brute={}, formula={}",
                i,
                delta_brute,
                delta_formula
            );
        }
    }
}

#[test]
fn test_dense_tensor3_flip_delta_brute_force() {
    let mut rng = ChaCha8Rng::seed_from_u64(137);
    let n = 8;
    let mut model = DenseTensor3Hopfield::new(n);

    for i in 0..n {
        model.pairwise.h[i] = rng.gen_range(-1.0..1.0);
        for j in (i + 1)..n {
            model.pairwise.set_j(i, j, rng.gen_range(-1.0..1.0));
            for k in (j + 1)..n {
                model.set_t(i, j, k, rng.gen_range(-0.5..0.5));
            }
        }
    }

    for _ in 0..30 {
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
                "DenseTensor3 delta mismatch at spin {}: brute={}, formula={}",
                i,
                delta_brute,
                delta_formula
            );
        }
    }
}

#[test]
fn test_sparse_hyperedge_flip_delta_brute_force() {
    let mut rng = ChaCha8Rng::seed_from_u64(256);
    let n = 16;
    let mut model = SparseHyperedgeMemory::new(n);

    for i in 0..n {
        model.h[i] = rng.gen_range(-1.0..1.0);
    }

    // Add 25 random 3-body hyperedges
    for _ in 0..25 {
        let mut idx = [
            rng.gen_range(0..n),
            rng.gen_range(0..n),
            rng.gen_range(0..n),
        ];
        idx.sort_unstable();
        if idx[0] < idx[1] && idx[1] < idx[2] {
            model.add_hyperedge3(Hyperedge3::new(
                idx[0],
                idx[1],
                idx[2],
                rng.gen_range(-1.0..1.0),
            ));
        }
    }

    // Add 15 random 4-body hyperedges
    for _ in 0..15 {
        let mut idx = [
            rng.gen_range(0..n),
            rng.gen_range(0..n),
            rng.gen_range(0..n),
            rng.gen_range(0..n),
        ];
        idx.sort_unstable();
        if idx[0] < idx[1] && idx[1] < idx[2] && idx[2] < idx[3] {
            model.add_hyperedge4(Hyperedge4::new(
                idx[0],
                idx[1],
                idx[2],
                idx[3],
                rng.gen_range(-0.5..0.5),
            ));
        }
    }

    for _ in 0..30 {
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
                "Sparse hyperedge delta mismatch at spin {}: brute={}, formula={}",
                i,
                delta_brute,
                delta_formula
            );
        }
    }
}

#[test]
fn test_low_rank_cp_flip_delta_brute_force() {
    let mut rng = ChaCha8Rng::seed_from_u64(999);
    let n = 10;
    let rank = 4;
    let mut model = LowRankCPMemory::new(n, rank);

    for r in 0..rank {
        model.cp.lambda[r] = rng.gen_range(0.2..1.5);
        for i in 0..n {
            model.cp.set_factor(i, r, rng.gen_range(-1.0..1.0));
        }
    }
    model.cp.update_norm_sq();

    for _ in 0..30 {
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
                "LowRankCP delta mismatch at spin {}: brute={}, formula={}",
                i,
                delta_brute,
                delta_formula
            );
        }
    }
}

#[test]
fn test_low_rank_cp_matches_dense_tensor3() {
    let mut rng = ChaCha8Rng::seed_from_u64(777);
    let n = 7;
    let rank = 3;
    let mut cp_model = LowRankCPMemory::new(n, rank);

    for r in 0..rank {
        cp_model.cp.lambda[r] = rng.gen_range(0.5..2.0);
        for i in 0..n {
            cp_model.cp.set_factor(i, r, rng.gen_range(-1.0..1.0));
        }
    }
    cp_model.cp.update_norm_sq();

    // Construct equivalent dense tensor T_ijk = sum_r lambda_r * a_ir * a_jr * a_kr
    let mut dense_model = DenseTensor3Hopfield::new(n);
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let mut t_val = 0.0;
                for r in 0..rank {
                    t_val += cp_model.cp.lambda[r]
                        * cp_model.cp.get_factor(i, r)
                        * cp_model.cp.get_factor(j, r)
                        * cp_model.cp.get_factor(k, r);
                }
                dense_model.set_t(i, j, k, t_val);
            }
        }
    }

    for _ in 0..20 {
        let spins: Vec<i8> = (0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect();
        let state = SpinState::from_slice(&spins);

        let e_cp = cp_model.energy(&state);
        let e_dense = dense_model.energy(&state);
        assert!(
            (e_cp - e_dense).abs() < 1e-10,
            "Energy mismatch between CP and Dense: cp={}, dense={}",
            e_cp,
            e_dense
        );

        for i in 0..n {
            let f_cp = cp_model.effective_field(&state, i);
            let f_dense = dense_model.effective_field(&state, i);
            assert!(
                (f_cp - f_dense).abs() < 1e-10,
                "Field mismatch at spin {}: cp={}, dense={}",
                i,
                f_cp,
                f_dense
            );
        }
    }
}

#[test]
fn test_greedy_descent_monotonicity() {
    let mut rng = ChaCha8Rng::seed_from_u64(1024);
    let n = 16;
    let pat = SpinState::from_slice(
        &(0..n)
            .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
            .collect::<Vec<i8>>(),
    );
    let patterns = vec![pat];

    let model_a = train_pairwise_hebbian(&patterns);
    let model_b = train_sparse_hyperedge3_budgeted(&patterns, 60, 42);
    let model_c = train_low_rank_cp(&patterns);

    let models: Vec<&dyn EnergyModel> = vec![&model_a, &model_b, &model_c];

    for (idx, m) in models.iter().enumerate() {
        for _ in 0..10 {
            let init_spins: Vec<i8> = (0..n)
                .map(|_| if rng.gen::<bool>() { 1 } else { -1 })
                .collect();
            let init_state = SpinState::from_slice(&init_spins);
            let res = greedy_descent(*m, &init_state, 30);

            // Verify strictly non-increasing energy trajectory
            for w in res.energy_trajectory.windows(2) {
                assert!(
                    w[1] <= w[0] + 1e-12,
                    "Model {} energy increased: before={}, after={}",
                    idx,
                    w[0],
                    w[1]
                );
            }
        }
    }
}
