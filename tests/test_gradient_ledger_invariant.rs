//! The gradient-ledger invariant (RC-006).
//!
//! `SpinState` maintains two ledgers that are updated independently on every
//! flip: the per-replica `energies`, and the field ledger read by
//! `delta_e_into`. `audit()` cross-checks them by rebuilding everything from
//! scratch — O(E) per call, which is why it runs in tests and verification
//! passes rather than during a run.
//!
//! They are algebraically linked, and the link is cheaper. For
//! `E(x) = offset + Σᵢ aᵢxᵢ + Σ_{i<j} q_ij xᵢxⱼ` over x ∈ {0,1}, a flip gives
//! `ΔEᵢ = (1−2xᵢ)·hᵢ` with `hᵢ = aᵢ + Σⱼ q_ij xⱼ`. Summing over all sites and
//! solving for the quadratic part yields
//!
//! ```text
//!   E = offset + L(x) + ( A_tot + Σⱼ dⱼxⱼ − 2·L(x) − Σᵢ ΔEᵢ ) / 4
//! ```
//!
//! where `A_tot = Σᵢ aᵢ` and the weighted degrees `dⱼ = Σᵢ q_ij` are
//! per-instance constants. After that one-off O(E) setup the check is **O(n) per
//! replica with no edge traversal**.
//!
//! LIMITATION, stated so the guard is not over-trusted: this is a *scalar
//! aggregate*, hence a **necessary but not sufficient** condition. Two
//! compensating errors in different `ΔEᵢ` cancel in the sum and slip through.
//! It does not replace `audit()`; it is affordable at a frequency `audit()` is
//! not.

use ising_engine::engine_v2::backends::{ReferenceState, SparseBitSlice};
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::state::SpinState;

/// One-off O(E) constants: `A_tot` and the weighted degrees.
fn constants(ir: &ProblemIR) -> (f64, Vec<f64>) {
    let a_tot: f64 = ir.linear.iter().sum();
    let deg: Vec<f64> = (0..ir.n)
        .map(|i| {
            let (lo, hi) = (ir.row_ptr[i] as usize, ir.row_ptr[i + 1] as usize);
            ir.weights[lo..hi].iter().sum()
        })
        .collect();
    (a_tot, deg)
}

/// O(n) energy reconstruction from the gradient ledger.
fn energy_from_gradient(ir: &ProblemIR, a_tot: f64, deg: &[f64], x: &[u8], sum_de: f64) -> f64 {
    let (mut l, mut dx) = (0.0f64, 0.0f64);
    for i in 0..ir.n {
        if x[i] != 0 {
            l += ir.linear[i];
            dx += deg[i];
        }
    }
    ir.offset + l + (a_tot + dx - 2.0 * l - sum_de) / 4.0
}

fn sum_delta_e(state: &dyn SpinState, n: usize, replica: usize) -> f64 {
    let mut de = vec![0.0f64; state.num_replicas()];
    let mut s = 0.0;
    for site in 0..n {
        state.delta_e_into(site, &mut de);
        s += de[replica];
    }
    s
}

/// Deterministic pseudo-random bit state — reproducible without an RNG crate.
fn pseudo_state(n: usize, seed: u64) -> Vec<u8> {
    let mut h = seed;
    (0..n)
        .map(|_| {
            h = h
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((h >> 33) & 1) as u8
        })
        .collect()
}

/// Small hand-built instance with mixed-sign weights, a non-zero offset and
/// non-trivial linear terms — the general case, not just MaxCut's shape.
fn mixed_sign_ir() -> ProblemIR {
    ProblemIR::from_pairs(
        7,
        -2.5,
        vec![1.0, -2.0, 3.0, 0.0, -1.0, 2.0, 1.5],
        &[
            (0, 1, -1.0),
            (1, 2, 2.0),
            (2, 3, -3.0),
            (0, 3, 1.0),
            (3, 4, -1.5),
            (4, 5, 2.0),
            (5, 6, -2.0),
            (2, 6, 1.0),
        ],
    )
}

#[test]
fn invariant_holds_on_both_backends_mixed_sign() {
    let ir = mixed_sign_ir();
    let (a_tot, deg) = constants(&ir);
    for seed in 0..64u64 {
        let x = pseudo_state(ir.n, 0x9E37 + seed);

        let refs = ReferenceState::new(&ir, 1, &x);
        let mut e = vec![0.0; 1];
        refs.energies_into(&mut e);
        let recon = energy_from_gradient(&ir, a_tot, &deg, &x, sum_delta_e(&refs, ir.n, 0));
        assert!(
            (e[0] - recon).abs() < 1e-9,
            "ReferenceState seed {seed}: ledger {} vs reconstructed {recon}",
            e[0]
        );

        if let Ok(bs) = SparseBitSlice::new(&ir, 1, &x) {
            bs.energies_into(&mut e);
            let recon_b = energy_from_gradient(&ir, a_tot, &deg, &x, sum_delta_e(&bs, ir.n, 0));
            assert!(
                (e[0] - recon_b).abs() < 1e-9,
                "SparseBitSlice seed {seed}: ledger {} vs reconstructed {recon_b}",
                e[0]
            );
        }
    }
}

/// The invariant must survive mutation, not just hold at construction — that is
/// the whole point of a ledger guard.
#[test]
fn invariant_survives_flips() {
    use ising_engine::engine_v2::state::ReplicaMask;
    let ir = mixed_sign_ir();
    let (a_tot, deg) = constants(&ir);
    let mut x = pseudo_state(ir.n, 0xBEEF);
    let mut st = ReferenceState::new(&ir, 1, &x);
    let mut mask = ReplicaMask::new(1);

    let mut h = 0xC0FFEEu64;
    for _ in 0..200 {
        h = h
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let site = ((h >> 33) as usize) % ir.n;
        mask.clear();
        mask.set(0);
        st.apply_flips(site, &mask);
        x[site] ^= 1;

        let mut e = vec![0.0; 1];
        st.energies_into(&mut e);
        let recon = energy_from_gradient(&ir, a_tot, &deg, &x, sum_delta_e(&st, ir.n, 0));
        assert!(
            (e[0] - recon).abs() < 1e-9,
            "drift after flipping site {site}: ledger {} vs reconstructed {recon}",
            e[0]
        );
    }
    assert_eq!(st.audit(), 0.0, "audit disagrees with the cheap invariant");
}

/// Real instance, multi-replica, both backends.
#[test]
fn invariant_holds_on_a_real_gset_instance() {
    let Ok(text) = std::fs::read_to_string("benchmark_suite/data/gset/G11") else {
        eprintln!("G11 unavailable — skipping (data is optional in some checkouts)");
        return;
    };
    let ir = rudy_maxcut_ir(&text).expect("G11 parses");
    let (a_tot, deg) = constants(&ir);
    let r = 8;
    let x = pseudo_state(ir.n, 0x5EED);

    let refs = ReferenceState::new(&ir, r, &x);
    let bs = SparseBitSlice::new(&ir, r, &x).expect("integral weights");
    let mut e = vec![0.0; r];

    for (label, st) in [
        ("ReferenceState", &refs as &dyn SpinState),
        ("SparseBitSlice", &bs as &dyn SpinState),
    ] {
        st.energies_into(&mut e);
        // Every replica starts from the same configuration `x`.
        let recon = energy_from_gradient(&ir, a_tot, &deg, &x, sum_delta_e(st, ir.n, 0));
        assert!(
            (e[0] - recon).abs() < 1e-9,
            "{label}: ledger {} vs reconstructed {recon}",
            e[0]
        );
    }
}
