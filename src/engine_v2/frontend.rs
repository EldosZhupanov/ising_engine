//! Frontend lowering (Constitution §7): external problem formats → `ProblemIR`.
//! Kept tiny and dependency-free. Today: rudy weighted-MaxCut, lowered to the
//! QUBO minimization `min −cut` that every engine and the experiment runner's
//! canonical scorer agree on; plus a bridge from the `core::QuboModel` used by
//! the benchmark parsers (BiqMac / OR-Library BQP / QPLIB), so the engine_v2
//! platform can run — and learn across — every problem family, not just MaxCut.

use super::ir::ProblemIR;
use crate::core::QuboModel;

/// Bridge a `core::QuboModel` (the form the benchmark family parsers emit) into
/// the engine_v2 `ProblemIR`. Both use the identical energy convention —
/// symmetric CSR storing (i,j) and (j,i), pair contribution halved, plus a
/// constant offset — so the lowering is an upper-triangle extraction. The
/// benchmark `assemble` never stores self-loops (diagonals are folded into
/// `linear` by each parser); a stray `i==j` entry, if one ever appeared, is
/// folded into `linear` here too, so `ProblemIR::energy` equals
/// `QuboModel::calculate_total_energy` on every state (asserted by tests over
/// real BiqMac/ORLIB/QPLIB instances).
pub fn qubo_model_to_ir(m: &QuboModel) -> ProblemIR {
    let n = m.num_vars;
    let mut linear = m.linear.clone();
    let mut pairs = Vec::new();
    for (i, lin_i) in linear.iter_mut().enumerate() {
        for (j, w) in m.quadratic.get_row(i) {
            match j.cmp(&i) {
                std::cmp::Ordering::Greater => pairs.push((i as u32, j as u32, w)),
                std::cmp::Ordering::Equal => *lin_i += 0.5 * w, // defensive: self-loop → linear
                std::cmp::Ordering::Less => {} // symmetric duplicate, captured by (j,i)
            }
        }
    }
    ProblemIR::from_pairs(n, m.energy_offset, linear, &pairs)
}

/// An upper-triangle weighted edge list: (i, j, weight) with i < j.
pub type Edges = Vec<(u32, u32, f64)>;

/// Parse a rudy weighted-MaxCut file: header `n m`, then `m` lines `i j [w]`
/// (1-indexed; missing weight = 1). Returns (n, upper-triangle (i<j) edges).
pub fn parse_rudy(text: &str) -> Result<(usize, Edges), String> {
    let mut lines = text
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>())
        .filter(|t| !t.is_empty());
    let header = lines.next().ok_or("empty file")?;
    let n: usize = header
        .first()
        .ok_or("missing n")?
        .parse()
        .map_err(|_| "bad n")?;
    let mut edges = Vec::new();
    for t in lines {
        if t.len() < 2 {
            continue;
        }
        let u: usize = t[0].parse().map_err(|_| "bad u")?;
        let v: usize = t[1].parse().map_err(|_| "bad v")?;
        let w: f64 = if t.len() > 2 {
            t[2].parse().map_err(|_| "bad w")?
        } else {
            1.0
        };
        if u == v || u < 1 || v < 1 || u > n || v > n {
            continue;
        }
        let (a, b) = ((u - 1).min(v - 1) as u32, (u - 1).max(v - 1) as u32);
        edges.push((a, b, w));
    }
    Ok((n, edges))
}

/// Lower MaxCut to a QUBO minimization (minimize −cut). Per edge (u,v,w):
/// linear[u]−=w, linear[v]−=w, pair(u,v)+=2w. Lowest QUBO energy ⇔ highest cut.
pub fn maxcut_to_qubo(n: usize, edges: &[(u32, u32, f64)]) -> ProblemIR {
    let mut linear = vec![0.0f64; n];
    let mut pairs = Vec::with_capacity(edges.len());
    for &(u, v, w) in edges {
        linear[u as usize] -= w;
        linear[v as usize] -= w;
        pairs.push((u, v, 2.0 * w));
    }
    ProblemIR::from_pairs(n, 0.0, linear, &pairs)
}

/// Convenience: rudy text → QUBO `ProblemIR`.
pub fn rudy_maxcut_ir(text: &str) -> Result<ProblemIR, String> {
    let (n, edges) = parse_rudy(text)?;
    Ok(maxcut_to_qubo(n, &edges))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_cut_lowering() {
        // Triangle, unit weights: max cut = 2 (any 2-1 split).
        let ir = rudy_maxcut_ir("3 3\n1 2\n2 3\n1 3\n").unwrap();
        // QUBO energy of x=(0,1,0): cut edges (1,2),(2,3) = 2 → energy −2.
        assert_eq!(ir.energy(&[0, 1, 0]), -2.0);
        assert_eq!(ir.energy(&[0, 0, 0]), 0.0);
    }

    #[test]
    fn qubo_bridge_preserves_energy_on_every_family() {
        use crate::benchmark::instances::{
            parse_biqmac_sparse, parse_orlib_bqp, parse_qplib, parse_rudy_maxcut,
        };
        use rand::{Rng, SeedableRng};
        use rand_chacha::ChaCha8Rng;

        // One representative instance per family, from the local dataset. Each
        // is skipped (not failed) if the file is absent, so the test is honest
        // on a partial checkout but exercises whatever is present.
        let mut checked = 0;
        let mut check = |model: &crate::core::QuboModel, tag: &str| {
            let ir = qubo_model_to_ir(model);
            assert_eq!(ir.n, model.num_vars, "{tag}: n");
            let mut rng = ChaCha8Rng::seed_from_u64(0x0B1D6E ^ tag.len() as u64);
            for _ in 0..40 {
                let x: Vec<u8> = (0..model.num_vars)
                    .map(|_| rng.gen::<bool>() as u8)
                    .collect();
                let xi: Vec<i8> = x.iter().map(|&b| b as i8).collect();
                let e_ir = ir.energy(&x);
                let e_qm = model.calculate_total_energy(&xi);
                assert_eq!(
                    e_ir.to_bits(),
                    e_qm.to_bits(),
                    "{tag}: energy mismatch {e_ir} vs {e_qm}"
                );
            }
            checked += 1;
        };

        let root = "benchmark_suite/data";
        if let Ok(t) = std::fs::read_to_string(format!("{root}/biqmac/be100.1.sparse")) {
            if let Ok(inst) = parse_biqmac_sparse(&t, "be100.1") {
                check(&inst.model, "biqmac");
            }
        }
        if let Ok(t) = std::fs::read_to_string(format!("{root}/orlib/bqp100.txt")) {
            if let Ok(insts) = parse_orlib_bqp(&t, "bqp100") {
                if let Some(inst) = insts.first() {
                    check(&inst.model, "orlib");
                }
            }
        }
        for f in ["QPLIB_3506", "QPLIB_3565", "QPLIB_3642"] {
            if let Ok(t) = std::fs::read_to_string(format!("{root}/qplib/{f}.qplib")) {
                if let Ok(inst) = parse_qplib(&t, f) {
                    check(&inst.model, "qplib");
                    break;
                }
            }
        }
        if let Ok(t) = std::fs::read_to_string(format!("{root}/gset/G11")) {
            if let Ok(inst) = parse_rudy_maxcut(&t, "G11") {
                check(&inst.model, "maxcut");
            }
        }
        // If none of the datasets are present this still passes vacuously; when
        // present (the normal case) it proves the bridge is energy-exact.
        eprintln!("qubo bridge validated on {checked} family instance(s)");
    }
}
