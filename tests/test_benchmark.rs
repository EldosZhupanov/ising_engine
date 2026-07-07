//! Validation of the benchmark framework: instance parsers map to the correct
//! minimization energy (checked by brute force), and the statistics match
//! known ground truth.

use ising_engine::benchmark::instances::{
    parse_biqmac_sparse, parse_orlib_bqp, parse_qplib, parse_rudy_maxcut, Instance,
};
use ising_engine::benchmark::stats;
use ising_engine::core::QuboModel;

/// Brute-force minimum energy and the native optimum for a small instance.
fn brute(inst: &Instance) -> (f64, f64) {
    let n = inst.n();
    assert!(n <= 20);
    let mut best = f64::INFINITY;
    for bits in 0..(1u32 << n) {
        let s: Vec<i8> = (0..n).map(|v| ((bits >> v) & 1) as i8).collect();
        best = best.min(inst.model.calculate_total_energy(&s));
    }
    (best, inst.native_objective(best))
}

#[test]
fn rudy_maxcut_maps_to_negative_cut() {
    // Triangle: max cut = 2 (any single-vertex side splits 2 of 3 edges).
    let text = "3 3\n1 2 1\n2 3 1\n1 3 1\n";
    let inst = parse_rudy_maxcut(text, "tri").unwrap();
    assert!(inst.maximize);
    let (min_e, native) = brute(&inst);
    assert!(
        (min_e + 2.0).abs() < 1e-9,
        "min energy should be -2, got {min_e}"
    );
    assert!(
        (native - 2.0).abs() < 1e-9,
        "max cut should be 2, got {native}"
    );
}

#[test]
fn rudy_maxcut_weighted_square() {
    // 4-cycle with unit weights: max cut = 4 (bipartite, split all edges).
    let text = "4 4\n1 2 1\n2 3 1\n3 4 1\n4 1 1\n";
    let inst = parse_rudy_maxcut(text, "c4").unwrap();
    let (_min_e, native) = brute(&inst);
    assert!(
        (native - 4.0).abs() < 1e-9,
        "max cut should be 4, got {native}"
    );
}

#[test]
fn orlib_bqp_maximizes_objective() {
    // Q symmetric, one triangle listed -> off-diagonals count TWICE:
    // max x'Qx = x1 + x2 + 2*3*x1x2 = 8 at (1,1).
    let text = "1\n2 3\n1 1 1\n2 2 1\n1 2 3\n";
    let v = parse_orlib_bqp(text, "toy").unwrap();
    assert_eq!(v.len(), 1);
    let (min_e, native) = brute(&v[0]);
    assert!(
        (min_e + 8.0).abs() < 1e-9,
        "min energy should be -8 (double-count), got {min_e}"
    );
    assert!(
        (native - 8.0).abs() < 1e-9,
        "max objective should be 8, got {native}"
    );
}

#[test]
fn biqmac_sparse_minimizes_with_double_count() {
    // MINIMIZE x'Qx (Biq Mac convention, unlike OR-Library), symmetric Q
    // one triangle listed: E = x1 + x2 - 6 x1x2, min -4 at (1,1).
    let inst = parse_biqmac_sparse("2 3\n1 1 1\n2 2 1\n1 2 -3\n", "bm").unwrap();
    assert!(!inst.maximize);
    let (min_e, native) = brute(&inst);
    assert!(
        (min_e + 4.0).abs() < 1e-9,
        "min energy should be -4 (minimize + double count), got {min_e}"
    );
    assert!((native + 4.0).abs() < 1e-9);
}

#[test]
fn qplib_unconstrained_binary_minimizes() {
    // Listed quad entries carry a ½ factor (verified vs. official QPLIB
    // solution files), so pair coeff = -6/2 = -3:
    // obj = x1 + x2 - 3 x1 x2, minimize → -1 at (1,1).
    // Real QBN layout: NO constraint-count line (adaptive format).
    let text = "toy            name\n\
                QBN            type\n\
                minimize       sense\n\
                2              n\n\
                1              nq\n\
                1 2 -6         q12\n\
                0              b_default\n\
                2              nb\n\
                1 1            b1\n\
                2 1            b2\n\
                0              c\n";
    let inst = parse_qplib(text, "toy").unwrap();
    assert!(!inst.maximize);
    let (min_e, native) = brute(&inst);
    assert!(
        (min_e + 1.0).abs() < 1e-9,
        "min energy should be -1, got {min_e}"
    );
    assert!((native + 1.0).abs() < 1e-9);
}

#[test]
fn qplib_refuses_constrained() {
    // QBL: linear CONSTRAINTS present -> refuse on the type code alone.
    let text = "c              name\nQBL            type\nminimize       sense\n2              n\n";
    assert!(parse_qplib(text, "c").is_err());
}

#[test]
fn qplib_refuses_nonbinary() {
    // QCN: quadratic objective, CONTINUOUS variables, no constraints.
    let text = "c              name\nQCN            type\nminimize       sense\n2              n\n";
    assert!(parse_qplib(text, "c").is_err());
}

// --------------------------------------------------------------------------
// Statistics
// --------------------------------------------------------------------------

#[test]
fn normal_and_t_sanity() {
    assert!((stats::normal_cdf(0.0) - 0.5).abs() < 1e-6);
    assert!(stats::normal_cdf(3.0) > 0.99);
    // t=0 ⇒ two-sided p = 1.
    assert!((stats::student_t_sf_two_sided(0.0, 10.0) - 1.0).abs() < 1e-6);
    // Large t ⇒ tiny p.
    assert!(stats::student_t_sf_two_sided(10.0, 10.0) < 1e-3);
}

#[test]
fn paired_tests_detect_a_clear_difference() {
    // b is uniformly worse than a by ~1 with small noise.
    let a: Vec<f64> = (0..12).map(|i| i as f64 * 0.1).collect();
    let b: Vec<f64> = a.iter().map(|x| x + 1.0).collect();
    let t = stats::paired_t_test(&a, &b);
    let w = stats::wilcoxon_signed_rank(&a, &b);
    assert!(t.p_value < 0.01, "t p={}", t.p_value);
    assert!(w.p_value < 0.05, "wilcoxon p={}", w.p_value);
}

#[test]
fn paired_tests_null_is_not_significant() {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let b = [1.1, 1.9, 3.05, 3.9, 5.1, 5.95];
    let t = stats::paired_t_test(&a, &b);
    assert!(
        t.p_value > 0.2,
        "near-identical samples flagged significant: p={}",
        t.p_value
    );
}

#[test]
fn bootstrap_ci_brackets_the_mean() {
    let data: Vec<f64> = (0..50).map(|i| (i % 7) as f64).collect();
    let mean = data.iter().sum::<f64>() / data.len() as f64;
    let (lo, hi) = stats::bootstrap_mean_ci(&data, 2000, 0.95, 1);
    assert!(lo <= mean && mean <= hi, "mean {mean} not in [{lo},{hi}]");
    assert!(lo < hi);
}

#[test]
fn native_objective_direction() {
    let m = QuboModel {
        num_vars: 1,
        linear: vec![0.0],
        quadratic: ising_engine::core::CsrMatrix::empty(1),
        energy_offset: 0.0,
    };
    let maxi = Instance {
        name: "m".into(),
        family: "f".into(),
        model: m,
        maximize: true,
        best_known: None,
    };
    assert_eq!(maxi.native_objective(-3.0), 3.0);
    assert!(maxi.is_better(5.0, 2.0));
}
