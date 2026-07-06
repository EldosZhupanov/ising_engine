//! Feedback-optimized ladder (Katzgraber-Trebst-Huse-Troyer 2006).

use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::solver::population_annealing::feedback_optimized_alphas;
use ising_engine::solver::ultimate::LadderMode;
use ising_engine::solver::UltimateSolver;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// A uniform flow gradient is already optimal → the geometric ladder is a
/// fixed point of the redistribution (with full damping).
#[test]
fn linear_flow_is_a_fixed_point() {
    let nt = 8;
    let alphas: Vec<f64> = (0..nt).map(|t| t as f64 / (nt - 1) as f64).collect();
    // Linear flow (constant df/dα) with equal spacing ⇒ equal masses ⇒
    // equidistribution reproduces the same positions.
    let flow: Vec<f64> = (0..nt).map(|t| t as f64 / (nt - 1) as f64).collect();
    let out = feedback_optimized_alphas(&alphas, &flow, 1.0);
    for (a, b) in out.iter().zip(&alphas) {
        assert!(
            (a - b).abs() < 1e-9,
            "linear flow must be a fixed point: {:?}",
            out
        );
    }
}

/// Where the flow changes steeply, the ladder must concentrate temperatures
/// (smaller spacings); where flow is flat, spacings widen.
#[test]
fn ladder_concentrates_where_flow_is_steep() {
    let nt = 9;
    let alphas: Vec<f64> = (0..nt).map(|t| t as f64 / (nt - 1) as f64).collect();
    // Flow rises almost entirely in the middle (steep gradient there).
    let flow: Vec<f64> = alphas
        .iter()
        .map(|&a| {
            // Sigmoid centered at 0.5 with a sharp slope.
            1.0 / (1.0 + (-20.0 * (a - 0.5)).exp())
        })
        .collect();
    let out = feedback_optimized_alphas(&alphas, &flow, 1.0);
    // The spacing straddling the center must be smaller than an edge spacing.
    let center = nt / 2;
    let center_gap = out[center] - out[center - 1];
    let edge_gap = out[1] - out[0];
    assert!(
        center_gap < edge_gap,
        "temperatures must concentrate at the steep-flow center: center {} vs edge {}",
        center_gap,
        edge_gap
    );
    // Endpoints stay pinned and order preserved.
    assert!((out[0]).abs() < 1e-9 && (out[nt - 1] - 1.0).abs() < 1e-9);
    for w in out.windows(2) {
        assert!(w[1] > w[0], "ladder must stay strictly ordered");
    }
}

#[allow(clippy::needless_range_loop)]
fn random_sparse(n: usize, rng: &mut ChaCha8Rng) -> QuboModel {
    let mut upper = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if rng.gen_range(0.0..1.0) < 0.5 {
                upper[i][j] = rng.gen_range(-2.0..2.0);
            }
        }
    }
    let mut values = Vec::new();
    let mut col_indices = Vec::new();
    let mut row_offsets = vec![0];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let w = if i < j { upper[i][j] } else { upper[j][i] };
                if w != 0.0 {
                    col_indices.push(j);
                    values.push(w);
                }
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

fn brute(m: &QuboModel) -> f64 {
    let n = m.num_vars;
    let mut best = f64::INFINITY;
    for bits in 0..(1u32 << n) {
        let s: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        best = best.min(m.calculate_total_energy(&s));
    }
    best
}

/// End-to-end: the feedback ladder must be deterministic and still reach the
/// exhaustive optimum (correctness preserved by the mode switch).
#[test]
fn feedback_ladder_solver_deterministic_and_optimal() {
    let mut rng = ChaCha8Rng::seed_from_u64(3);
    for trial in 0..5 {
        let model = random_sparse(14, &mut rng);
        let e_opt = brute(&model);
        let mut s = UltimateSolver::new(10.0, 0.05, 5, 15, Some(700 + trial));
        s.num_pops = 2;
        s.num_temps = 8;
        s.ladder_mode = LadderMode::FeedbackOptimized;
        let out1 = s.solve(&model, &[]);
        let out2 = s.solve(&model, &[]);
        assert_eq!(out1, out2, "feedback-ladder solve must be deterministic");
        let e = model.calculate_total_energy(&out1);
        assert!(
            (e - e_opt).abs() < 1e-9,
            "trial {}: feedback ladder returned {} but optimum is {}",
            trial,
            e,
            e_opt
        );
    }
}
