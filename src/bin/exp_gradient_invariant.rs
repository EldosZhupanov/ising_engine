//! Vector-1 probe: is the energy ledger reconstructible from the gradient ledger?
//!
//! The engine maintains two independent ledgers — `energies[rep]`, and the field
//! ledger read by `delta_e_into`. `SpinState::audit()` cross-checks them with a
//! full O(E) recompute. But they are algebraically linked, and the link is never
//! declared anywhere in the codebase.
//!
//! For `E(x) = offset + Σᵢ aᵢxᵢ + Σ_{i<j} q_ij xᵢxⱼ` over x ∈ {0,1}, flipping site
//! i changes the energy by `ΔEᵢ = (1−2xᵢ)·hᵢ` where `hᵢ = aᵢ + Σⱼ q_ij xⱼ`.
//! Summing over every site:
//!
//! ```text
//!   Σᵢ ΔEᵢ = A_tot + Σⱼ dⱼxⱼ − 2·L(x) − 4·Q(x)
//! ```
//!
//! with `A_tot = Σᵢ aᵢ`, `dⱼ = Σᵢ q_ij` the weighted degree (a per-INSTANCE
//! constant, computed once), `L(x) = Σᵢ aᵢxᵢ`, and `Q(x)` the quadratic part.
//! Since `E = offset + L + Q`, solving for Q gives an O(n) reconstruction of the
//! energy from the gradient ledger alone:
//!
//! ```text
//!   E = offset + L(x) + ( A_tot + Σⱼ dⱼxⱼ − 2·L(x) − Σᵢ ΔEᵢ ) / 4
//! ```
//!
//! CLAIM UNDER TEST: that identity holds exactly, on both backends, for arbitrary
//! states. If it does, the two ledgers can be cross-checked in O(n) per replica
//! with no edge traversal — an invariant the engine relies on implicitly but
//! never uses.
//!
//! This is a falsification harness, not a demonstration: the derivation was done
//! by hand and a wrong constant would show up as a systematic residual.
//!
//! ```text
//! cargo run --release --bin exp_gradient_invariant -- --file benchmark_suite/data/gset/G1
//! ```

use ising_engine::engine_v2::backends::{ReferenceState, SparseBitSlice};
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::state::SpinState;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == flag)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Per-instance constants: `A_tot = Σ aᵢ` and the weighted degrees `dⱼ = Σᵢ q_ij`.
/// Computed once in O(E); every subsequent check is O(n).
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

/// The O(n) reconstruction. Reads only the gradient ledger and the spins.
fn energy_from_gradient(ir: &ProblemIR, a_tot: f64, deg: &[f64], x: &[u8], sum_de: f64) -> f64 {
    let mut l = 0.0f64; // L(x) = Σ aᵢxᵢ
    let mut dx = 0.0f64; // Σⱼ dⱼxⱼ
    for i in 0..ir.n {
        if x[i] != 0 {
            l += ir.linear[i];
            dx += deg[i];
        }
    }
    ir.offset + l + (a_tot + dx - 2.0 * l - sum_de) / 4.0
}

fn main() {
    let file: String = arg("--file", "benchmark_suite/data/gset/G1".to_string());
    let trials: usize = arg("--trials", 12);

    let Ok(text) = std::fs::read_to_string(&file) else {
        eprintln!("cannot read {file}");
        std::process::exit(1);
    };
    let Ok(ir) = rudy_maxcut_ir(&text) else {
        eprintln!("cannot parse {file}");
        std::process::exit(1);
    };
    let (a_tot, deg) = constants(&ir);

    println!("# gradient-ledger invariant · {} · n={}", file, ir.n);
    println!("# E ?= offset + L + (A_tot + Σdⱼxⱼ − 2L − ΣΔEᵢ)/4\n");
    println!(
        "{:>6} {:>8} {:>20} {:>20} {:>14}",
        "trial", "backend", "ledger E", "reconstructed E", "abs residual"
    );

    // Deterministic pseudo-random states; no RNG crate needed, and reproducible.
    let mut h: u64 = 0x243F_6A88_85A3_08D3;
    let mut worst = 0.0f64;
    let mut checked = 0usize;

    for t in 0..trials {
        let mut x = vec![0u8; ir.n];
        for xi in x.iter_mut() {
            h = h
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *xi = ((h >> 33) & 1) as u8;
        }

        // ReferenceState (f64 oracle) and SparseBitSlice (exact integer) must both
        // satisfy it, or the identity is backend-specific and therefore not an
        // invariant of the model.
        let refs = ReferenceState::new(&ir, 1, &x);
        let mut de = vec![0.0f64; 1];
        let mut sum_de = 0.0f64;
        for site in 0..ir.n {
            refs.delta_e_into(site, &mut de);
            sum_de += de[0];
        }
        let mut e = vec![0.0f64; 1];
        refs.energies_into(&mut e);
        let recon = energy_from_gradient(&ir, a_tot, &deg, &x, sum_de);
        let r1 = (e[0] - recon).abs();
        worst = worst.max(r1);
        checked += 1;
        if t < 4 {
            println!(
                "{t:>6} {:>8} {:>20.6} {:>20.6} {r1:>14.3e}",
                "ref", e[0], recon
            );
        }

        if let Ok(bs) = SparseBitSlice::new(&ir, 1, &x) {
            let mut sum_de_b = 0.0f64;
            for site in 0..ir.n {
                bs.delta_e_into(site, &mut de);
                sum_de_b += de[0];
            }
            bs.energies_into(&mut e);
            let recon_b = energy_from_gradient(&ir, a_tot, &deg, &x, sum_de_b);
            let r2 = (e[0] - recon_b).abs();
            worst = worst.max(r2);
            checked += 1;
            if t < 4 {
                println!(
                    "{t:>6} {:>8} {:>20.6} {:>20.6} {r2:>14.3e}",
                    "bitslice", e[0], recon_b
                );
            }
        }
    }

    println!("\n# checks: {checked}   worst absolute residual: {worst:.6e}");
    if worst < 1e-6 {
        println!("# VERIFIED — identity holds on both backends.");
        println!("# Cost: O(n) per replica after an O(E) one-off, vs audit()'s O(E) per call.");
    } else {
        println!("# REFUTED — the hand derivation carries an error; do not use.");
        std::process::exit(1);
    }
}
