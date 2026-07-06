//! Contracts for the TTS framework (Rønnow et al. 2014; Luby et al. 1993).

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::tts::{
    bootstrap_success_ci, has_converged, luby, success_probability, tts, tts_ci,
};
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[test]
fn tts_formula_edge_cases_and_value() {
    // p = 1 → a single run suffices.
    assert!((tts(1.0, 10.0, 0.99) - 10.0).abs() < 1e-12);
    // p = 0 → infinite.
    assert!(tts(0.0, 10.0, 0.99).is_infinite());
    // Known value: p = 0.5, q = 0.99 → t·ln(0.01)/ln(0.5).
    let expected = 10.0 * (0.01f64).ln() / (0.5f64).ln();
    assert!((tts(0.5, 10.0, 0.99) - expected).abs() < 1e-9);
    // TTS is monotone decreasing in p_s.
    assert!(tts(0.2, 1.0, 0.99) > tts(0.8, 1.0, 0.99));
}

#[test]
fn success_probability_counts() {
    assert!((success_probability(&[true, true, false, false]) - 0.5).abs() < 1e-12);
    assert_eq!(success_probability(&[]), 0.0);
}

#[test]
fn bootstrap_ci_brackets_estimate_and_is_deterministic() {
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    let successes: Vec<bool> = (0..500).map(|_| rng.gen_bool(0.6)).collect();
    let p = success_probability(&successes);
    let (lo, hi) = bootstrap_success_ci(&successes, 2000, 0.95, 42);
    assert!(
        lo <= p && p <= hi,
        "CI [{}, {}] must bracket p={}",
        lo,
        hi,
        p
    );
    assert!(hi - lo < 0.15, "CI unreasonably wide: {}", hi - lo);
    // Deterministic given seed.
    let (lo2, hi2) = bootstrap_success_ci(&successes, 2000, 0.95, 42);
    assert_eq!((lo, hi), (lo2, hi2));
    // TTS CI: lower p bound → upper TTS bound.
    let (tlo, thi) = tts_ci(&successes, 1.0, 0.99, 2000, 0.95, 42);
    assert!(tlo <= thi);
    assert!((tlo - tts(hi, 1.0, 0.99)).abs() < 1e-9);
}

#[test]
fn luby_sequence_matches_reference() {
    // Canonical Luby prefix.
    let expected = [1, 1, 2, 1, 1, 2, 4, 1, 1, 2, 1, 1, 2, 4, 8];
    let got: Vec<usize> = (1..=15).map(luby).collect();
    assert_eq!(got, expected);
}

#[test]
fn convergence_detection() {
    // Flat tail → converged.
    assert!(has_converged(&[10.0, 5.0, 2.0, 2.0, 2.0], 3, 1e-9));
    // Still improving → not converged.
    assert!(!has_converged(&[10.0, 8.0, 6.0, 4.0], 3, 1e-9));
    // Too short → not converged.
    assert!(!has_converged(&[1.0], 3, 1e-9));
}

#[allow(clippy::needless_range_loop)]
fn random_qubo(n: usize, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            upper[i][j] = rng.gen_range(-2.0..2.0);
        }
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let w = if i < j { upper[i][j] } else { upper[j][i] };
                col_indices.push(j);
                values.push(w);
            }
        }
        row_offsets.push(col_indices.len());
    }
    QuboModel {
        num_vars: n,
        linear: (0..n).map(|_| rng.gen_range(-2.0..2.0)).collect(),
        quadratic: CsrMatrix {
            values,
            col_indices,
            row_offsets,
        },
        energy_offset: 0.0,
    }
}

fn brute(model: &QuboModel) -> f64 {
    let n = model.num_vars;
    let mut best = f64::INFINITY;
    for bits in 0..(1u32 << n) {
        let s: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        best = best.min(model.calculate_total_energy(&s));
    }
    best
}

/// Luby restarts must be deterministic and never worse than a single solve
/// (they keep the best over restarts), and reach the optimum on easy cases.
#[test]
fn luby_restarts_deterministic_and_optimal() {
    let mut rng = ChaCha8Rng::seed_from_u64(2);
    for _ in 0..5 {
        let model = random_qubo(16, &mut rng);
        let e_opt = brute(&model);
        let solver = UltimateSolver::new(10.0, 0.05, 3, 3, Some(4242));
        let a = solver.solve_with_luby_restarts(&model, &[], 6);
        let b = solver.solve_with_luby_restarts(&model, &[], 6);
        assert_eq!(a, b, "Luby restarts must be deterministic");
        let e = model.calculate_total_energy(&a);
        assert!(
            (e - e_opt).abs() < 1e-9,
            "restarts returned {} but optimum is {}",
            e,
            e_opt
        );
    }
}
