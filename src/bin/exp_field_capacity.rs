//! Prediction P2 — does a marginal field carry ANY information on a spin glass?
//!
//! The Ax2-destroying architecture (state = distribution held as sufficient
//! statistics) is only worth building if per-site marginals are not pure noise.
//! That is decidable BEFORE writing the architecture, by measurement.
//!
//! For R replicas, m_i = ⟨s_i⟩. If the ensemble carries no per-site information —
//! i.e. the replicas are independent samples of a symmetric multi-modal
//! distribution — then m_i is a sum of R fair ±1 draws and
//!
//!     E|m_i| = sqrt(2 / (π R))          (the NULL)
//!
//! which is 0.1410 at R = 32. A product-form field is then encoding nothing, and
//! the architecture collapses for the same reason belief propagation does, by a
//! different door.
//!
//! Initialization matters and RC-002 says why: from the all-zeros start every
//! replica is identical, so |m_i| ≡ 1 trivially — an artifact, not structure. The
//! ensemble is therefore randomized first (`random_flip_sweep`), exactly as
//! RC-002's winning arm does.
//!
//! Reported per temperature: mean |m_i|, the null, the ratio, and the fraction of
//! sites that are strongly polarised (|m| > 0.5).
//!
//! ```text
//! cargo run --release --bin exp_field_capacity -- --dir benchmark_suite/data/gset
//! ```

use ising_engine::engine_v2::backends::SparseBitSlice;
use ising_engine::engine_v2::frontend::rudy_maxcut_ir;
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::{Budget, Operator};
use ising_engine::engine_v2::operators::{MetropolisSweep, RandomFlipSweep};
use ising_engine::engine_v2::runtime::RuntimeView;
use ising_engine::engine_v2::state::SpinState;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn arg<T: std::str::FromStr>(flag: &str, default: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == flag)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Geometric ladder from `hi` down to `lo`, one temperature per replica.
fn ladder(hi: f64, lo: f64, r: usize) -> Vec<f64> {
    if r <= 1 {
        return vec![lo];
    }
    let ratio = (lo / hi).powf(1.0 / (r - 1) as f64);
    (0..r).map(|k| hi * ratio.powi(k as i32)).collect()
}

fn main() {
    let dir: String = arg("--dir", "benchmark_suite/data/gset".to_string());
    let sweeps: u32 = arg("--sweeps", 100);
    let r: usize = arg("--replicas", 32);
    let seed: u64 = arg("--seed", 1);
    // P5: break the Z2 gauge symmetry with an external field. If m≈0 is a
    // CONSEQUENCE OF SYMMETRY (theorem) rather than of glassiness (phenomenon),
    // even a small field must lift the marginals far above the noise floor.
    let field: f64 = arg("--field", 0.0);

    let mut paths: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(e) => {
            eprintln!("cannot read {dir}: {e}");
            std::process::exit(1);
        }
    };
    paths.sort();
    let mut instances: Vec<(String, ProblemIR)> = Vec::new();
    for p in &paths {
        let (Ok(t), Some(id)) = (
            std::fs::read_to_string(p),
            p.file_name().and_then(|s| s.to_str()),
        ) else {
            continue;
        };
        if let Ok(ir) = rudy_maxcut_ir(&t) {
            instances.push((id.to_string(), ir));
        }
    }
    if instances.is_empty() {
        eprintln!("no parsable instances in {dir}");
        std::process::exit(1);
    }

    // E|m| for R independent fair ±1 draws — the "field is pure noise" null.
    let null = (2.0 / (std::f64::consts::PI * r as f64)).sqrt();

    println!(
        "# P2 field capacity · {} instances · R={} sweeps={} · randomized init (RC-002)",
        instances.len(),
        r,
        sweeps
    );
    println!("# null E|m| for a noise field = sqrt(2/(pi*R)) = {null:.4}");
    println!("# ratio <= ~1.2 means the product-form field encodes nothing\n");
    println!(
        "{:<8} {:>9} {:>7} {:>12} {:>12} | {:>9} {:>7} {:>12}",
        "temp_hi",
        "mean|m|",
        "ratio",
        "frac|m|>.5",
        "frac|m|>.9",
        "mean|cov|",
        "ratio",
        "frac|c|>.5"
    );

    for &temp_hi in &[0.1f64, 0.5, 1.0, 2.0, 4.0] {
        let temps = ladder(temp_hi, 0.1, r);
        let (mut sum_abs_m, mut n_tot, mut strong, mut vstrong) = (0.0f64, 0usize, 0usize, 0usize);
        let (mut sum_abs_c, mut n_pairs, mut strong_c) = (0.0f64, 0usize, 0usize);
        let (mut grp_n, mut grp_s, mut grp_ss) = ([0usize; 2], [0.0f64; 2], [0.0f64; 2]);
        let (mut fr_n, mut fr_s, mut fr_deg) = ([0usize; 3], [0.0f64; 3], [0.0f64; 3]);
        // Q1: covariance on NON-ADJACENT pairs. Every number reported so far was
        // measured on graph EDGES — pairs that are directly coupled, where
        // correlation is expected and carries no new information. Emergent
        // structure, if it exists, must show up between UNCOUPLED variables.
        let (mut na_n, mut na_s) = (0usize, 0.0f64);
        // Q2: two-factor eta^2. Group by (sign J x degree octile). If degree —
        // a trivially known local property — absorbs the residual, the
        // "76% emergent" was heterogeneity in the graph, not emergent structure.
        let (mut g2_n, mut g2_s, mut g2_ss) = ([0usize; 16], [0.0f64; 16], [0.0f64; 16]);

        for (_, ir0) in &instances {
            // MaxCut in spin variables is field-free: the QUBO linear terms
            // exactly cancel the field. Perturbing them re-introduces one.
            let mut irw;
            let ir = if field > 0.0 {
                irw = ir0.clone();
                for (i, l) in irw.linear.iter_mut().enumerate() {
                    let s = if (i * 2654435761) % 2 == 0 { 1.0 } else { -1.0 };
                    *l += field * s;
                }
                &irw
            } else {
                ir0
            };
            let init = vec![0u8; ir.n];
            let Ok(mut st) = SparseBitSlice::new(ir, r, &init) else {
                continue;
            };
            let view = RuntimeView {
                iteration: 0,
                temperatures: &temps,
                num_replicas: r,
                recent_acceptance: 0.0,
                remaining_ms: f64::INFINITY,
            };
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            // Randomize first: from all-zeros every replica is identical and
            // |m| would be 1 by construction (RC-002's artifact).
            RandomFlipSweep::new().apply(&mut st, &view, &mut rng, Budget { sweeps: 1 });
            MetropolisSweep::new().apply(&mut st, &view, &mut rng, Budget { sweeps });

            let mut mag = vec![0.0f64; ir.n];
            for (site, mg) in mag.iter_mut().enumerate() {
                let up = (0..r).filter(|&rep| st.spin(site, rep)).count();
                let m = (2.0 * up as f64 - r as f64) / r as f64;
                *mg = m;
                let a = m.abs();
                sum_abs_m += a;
                n_tot += 1;
                if a > 0.5 {
                    strong += 1;
                }
                if a > 0.9 {
                    vstrong += 1;
                }
            }

            // SECOND moment. In a spin glass ⟨sᵢ⟩ → 0 while ⟨sᵢsⱼ⟩ − ⟨sᵢ⟩⟨sⱼ⟩
            // stays finite, so the marginals dying does not imply the pairwise
            // statistics die. This is the part RC-003's covariance-mined moves
            // actually exploited, so it must be measured separately.
            for i in 0..ir.n {
                let lo = ir.row_ptr[i] as usize;
                let hi = ir.row_ptr[i + 1] as usize;
                for (k, &jc) in ir.col_idx[lo..hi].iter().enumerate() {
                    let j = jc as usize;
                    if j <= i {
                        continue;
                    }
                    let mut prod = 0i64;
                    for rep in 0..r {
                        let a = if st.spin(i, rep) { 1i64 } else { -1 };
                        let b = if st.spin(j, rep) { 1i64 } else { -1 };
                        prod += a * b;
                    }
                    let cov = (prod as f64 / r as f64) - mag[i] * mag[j];
                    sum_abs_c += cov.abs();
                    n_pairs += 1;
                    if cov.abs() > 0.5 {
                        strong_c += 1;
                    }
                    // P3: is this covariance just a restatement of the coupling?
                    // Mean-field says cov(i,j) ≈ −tanh(β·J_ij), which on an
                    // unweighted instance depends ONLY on sign(J). Group by that
                    // sign; if the within-group variance vanishes, the ensemble
                    // told us nothing the instance did not already say.
                    let w = ir.weights[lo + k];
                    let g = usize::from(w > 0.0);
                    let (jl0, jh0) = (ir.row_ptr[j] as usize, ir.row_ptr[j + 1] as usize);
                    grp_n[g] += 1;
                    grp_s[g] += cov;
                    grp_ss[g] += cov * cov;
                    let dsum = (hi - lo) + (jh0 - jl0);
                    let oct = (dsum / 16).min(7);
                    let gg = g * 8 + oct;
                    g2_n[gg] += 1;
                    g2_s[gg] += cov;
                    g2_ss[gg] += cov * cov;
                    // (degree of j, needed above)
                    // P7b: SELF-REFUTATION of Law 2. Edges in triangles have
                    // common neighbours BY DEFINITION, so they sit in denser
                    // neighbourhoods, and covariance falls with local density for
                    // reasons unrelated to frustration. The P7 split therefore
                    // confounds frustration with density. Control: hold "is in a
                    // triangle" fixed and vary only whether it is FRUSTRATED.
                    //   group 0 = in no triangle at all
                    //   group 1 = in triangles, NONE frustrated  <- the control
                    //   group 2 = in >=1 frustrated triangle
                    let (mut tri_all, mut tri_fr) = (0usize, 0usize);
                    let (jl, jh) = (ir.row_ptr[j] as usize, ir.row_ptr[j + 1] as usize);
                    for (k2, &kc) in ir.col_idx[jl..jh].iter().enumerate() {
                        let kk = kc as usize;
                        if kk == i {
                            continue;
                        }
                        let mut w_ik = 0.0;
                        for (k3, &c3) in ir.col_idx[lo..hi].iter().enumerate() {
                            if c3 as usize == kk {
                                w_ik = ir.weights[lo + k3];
                                break;
                            }
                        }
                        if w_ik != 0.0 {
                            tri_all += 1;
                            if w * ir.weights[jl + k2] * w_ik > 0.0 {
                                tri_fr += 1;
                            }
                        }
                    }
                    let fg = if tri_all == 0 {
                        0
                    } else if tri_fr == 0 {
                        1
                    } else {
                        2
                    };
                    fr_n[fg] += 1;
                    fr_s[fg] += cov.abs();
                    fr_deg[fg] += (hi - lo) as f64 + (jh - jl) as f64;
                }
            }
            // Q1: sample non-adjacent pairs on this instance.
            let mut h: u64 = 0x9E3779B97F4A7C15;
            let mut draws = 0;
            while draws < 4000 && ir.n > 8 {
                h = h
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let i = ((h >> 33) as usize) % ir.n;
                h = h
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let j = ((h >> 33) as usize) % ir.n;
                if i == j {
                    continue;
                }
                let (al, ah) = (ir.row_ptr[i] as usize, ir.row_ptr[i + 1] as usize);
                if ir.col_idx[al..ah].iter().any(|&c| c as usize == j) {
                    continue;
                }
                draws += 1;
                let mut prod = 0i64;
                for rep in 0..r {
                    let a = if st.spin(i, rep) { 1i64 } else { -1 };
                    let b = if st.spin(j, rep) { 1i64 } else { -1 };
                    prod += a * b;
                }
                let cov = (prod as f64 / r as f64) - mag[i] * mag[j];
                na_s += cov.abs();
                na_n += 1;
            }
        }

        let mean = sum_abs_m / n_tot.max(1) as f64;
        let meanc = sum_abs_c / n_pairs.max(1) as f64;
        // eta^2 = fraction of covariance variance explained by sign(J) alone.
        // eta^2 -> 1 means the ensemble is redundant with the problem statement.
        let ntot = (grp_n[0] + grp_n[1]).max(1) as f64;
        let grand = (grp_s[0] + grp_s[1]) / ntot;
        let total_ss = (grp_ss[0] + grp_ss[1]) - ntot * grand * grand;
        let mut between = 0.0;
        for g in 0..2 {
            if grp_n[g] > 0 {
                let mg = grp_s[g] / grp_n[g] as f64;
                between += grp_n[g] as f64 * (mg - grand) * (mg - grand);
            }
        }
        let eta2 = if total_ss > 1e-12 {
            between / total_ss
        } else {
            0.0
        };
        let resid = if total_ss > 0.0 {
            ((total_ss - between) / ntot).sqrt()
        } else {
            0.0
        };
        println!(
            "{temp_hi:<8} {mean:>9.4} {:>7.2} {:>11.2}% {:>11.2}% | {meanc:>9.4} {:>7.2} {:>11.2}%",
            mean / null,
            100.0 * strong as f64 / n_tot.max(1) as f64,
            100.0 * vstrong as f64 / n_tot.max(1) as f64,
            meanc / null,
            100.0 * strong_c as f64 / n_pairs.max(1) as f64
        );
        println!(
            "{:<8} {:>9} {:>7} {:>12} {:>12} | P3: eta^2(sign J)={eta2:.4}  residual sd={resid:.4}  resid/null={:.2}",
            "", "", "", "", "",
            resid / null
        );
        {
            // Q2: how much of the covariance is explained once the trivially
            // known local properties — sign(J) AND local degree — are both
            // accounted for. Q1: is there any correlation at all between
            // variables that are NOT directly coupled.
            let tot: usize = g2_n.iter().sum();
            let gs: f64 = g2_s.iter().sum();
            let gss: f64 = g2_ss.iter().sum();
            let grand2 = gs / tot.max(1) as f64;
            let tss = gss - tot as f64 * grand2 * grand2;
            let mut betw = 0.0;
            for g in 0..16 {
                if g2_n[g] > 0 {
                    let m = g2_s[g] / g2_n[g] as f64;
                    betw += g2_n[g] as f64 * (m - grand2) * (m - grand2);
                }
            }
            let eta2_2f = if tss > 1e-12 { betw / tss } else { 0.0 };
            let na_mean = if na_n > 0 {
                na_s / na_n as f64
            } else {
                f64::NAN
            };
            println!(
                "{:<8} Q1 NON-ADJACENT pairs: n={:<7} mean|cov|={na_mean:.4} ratio={:.2}  ||  Q2 eta^2(signJ x degree)={eta2_2f:.4}",
                "",
                na_n,
                na_mean / null
            );
        }
        let mn = |g: usize| {
            if fr_n[g] > 0 {
                fr_s[g] / fr_n[g] as f64
            } else {
                f64::NAN
            }
        };
        let dg = |g: usize| {
            if fr_n[g] > 0 {
                fr_deg[g] / fr_n[g] as f64
            } else {
                f64::NAN
            }
        };
        println!(
            "{:<8} P7b  no-tri: n={:<7} |cov|={:.4} deg={:.1} | tri-UNfrust: n={:<7} |cov|={:.4} deg={:.1} | tri-FRUST: n={:<7} |cov|={:.4} deg={:.1}",
            "",
            fr_n[0], mn(0), dg(0),
            fr_n[1], mn(1), dg(1),
            fr_n[2], mn(2), dg(2)
        );
    }

    println!("\n# P2 REFUTES the distributional architecture if ratio ~ 1 at every temperature.");
    println!("# P2 SUPPORTS it if the field polarises well above the noise floor.");
}
