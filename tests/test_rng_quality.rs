//! Sanity contract for the 8-lane interleaved Xoshiro256++ generator.
//!
//! Not a statistical test battery (the underlying generator is
//! Blackman & Vigna's xoshiro256++, already validated in ACM TOMS 2021);
//! these tests pin the properties the Metropolis kernel relies on:
//! range, determinism, lane independence, and basic uniformity.

use ising_engine::solver::engine::Xoshiro256PlusPlusX8;

#[test]
fn outputs_are_uniform_in_unit_interval() {
    let mut rng = Xoshiro256PlusPlusX8::new(12345);
    let mut buf = vec![0.0f64; 100_000];
    rng.fill_f64(&mut buf);

    let mut sum = 0.0;
    for &x in &buf {
        assert!((0.0..1.0).contains(&x), "out of range: {}", x);
        sum += x;
    }
    let mean = sum / buf.len() as f64;
    // Std. error of the mean for U[0,1) at n=1e5 is ~0.0009; 5σ band.
    assert!(
        (mean - 0.5).abs() < 0.005,
        "mean {} deviates from 0.5 beyond 5σ",
        mean
    );
}

#[test]
fn generator_is_deterministic() {
    let mut a = Xoshiro256PlusPlusX8::new(777);
    let mut b = Xoshiro256PlusPlusX8::new(777);
    let mut buf_a = vec![0.0f64; 1024];
    let mut buf_b = vec![0.0f64; 1024];
    a.fill_f64(&mut buf_a);
    b.fill_f64(&mut buf_b);
    assert_eq!(buf_a, buf_b);
}

#[test]
fn different_seeds_give_different_streams() {
    let mut a = Xoshiro256PlusPlusX8::new(1);
    let mut b = Xoshiro256PlusPlusX8::new(2);
    let mut buf_a = vec![0.0f64; 64];
    let mut buf_b = vec![0.0f64; 64];
    a.fill_f64(&mut buf_a);
    b.fill_f64(&mut buf_b);
    assert_ne!(buf_a, buf_b);
}

#[test]
fn lanes_are_independent_streams() {
    // Consecutive outputs land in different lanes (index i uses lane i mod 8);
    // within one fill, the 8 lane-subsequences must all differ.
    let mut rng = Xoshiro256PlusPlusX8::new(99);
    let mut buf = vec![0.0f64; 8 * 32];
    rng.fill_f64(&mut buf);
    for a in 0..8 {
        for b in (a + 1)..8 {
            let lane_a: Vec<f64> = buf.iter().skip(a).step_by(8).copied().collect();
            let lane_b: Vec<f64> = buf.iter().skip(b).step_by(8).copied().collect();
            assert_ne!(
                lane_a, lane_b,
                "lanes {} and {} emitted identical streams",
                a, b
            );
        }
    }
}
