//! QOBLIB 2026 Problem Class 01: LLL screening on 41 known-feasible instances.
//!
//! Evaluates lattice reduction on 41 Market Split instances whose input comments
//! already publish feasible witnesses.
//!
//! For each instance:
//! 1. Builds extended lattice (n + 1) x (n + 1 + m) with penalty scaling.
//! 2. Executes LLL reduction (delta = 0.75).
//! 3. Extracts individually checked integer null vectors and, when found, a
//!    particular integer solution Ax_0 = b; basis completeness is unverified.
//! 4. Evaluates whether exact boolean solution x in {0, 1}^n is found directly.
//! 5. Evaluates 2-opt swap quench from the LLL-seeded state.
//! 6. Verifies any candidate solution with official ZIB `check_marketsplit`.

#![allow(clippy::needless_range_loop)]

#[path = "common/marketsplit_certificate.rs"]
mod marketsplit_certificate;

use ising_engine::core::lattice::aardal_diophantine_reduction;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone)]
pub struct MarketSplitInstance {
    pub name: String,
    pub path: PathBuf,
    pub num_cons: usize,
    pub num_vars: usize,
    pub matrix: Vec<Vec<i64>>,
    pub rhs: Vec<i64>,
}

pub fn parse_marketsplit_dat<P: AsRef<Path>>(
    path: P,
) -> Result<MarketSplitInstance, Box<dyn std::error::Error>> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);

    let name = path
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let mut num_cons = 0;
    let mut num_vars = 0;
    let mut matrix = Vec::new();
    let mut rhs = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if num_cons == 0 && num_vars == 0 {
            if parts.len() >= 2 {
                num_cons = parts[0].parse::<usize>()?;
                num_vars = parts[1].parse::<usize>()?;
            }
        } else if parts.len() > num_vars {
            let mut row = Vec::with_capacity(num_vars);
            for i in 0..num_vars {
                row.push(parts[i].parse::<i64>()?);
            }
            let b = parts[num_vars].parse::<i64>()?;
            matrix.push(row);
            rhs.push(b);
        }
    }

    Ok(MarketSplitInstance {
        name,
        path: path.as_ref().to_path_buf(),
        num_cons,
        num_vars,
        matrix,
        rhs,
    })
}

pub struct State {
    pub sol: Vec<i8>,
    pub residuals: Vec<i64>,
    pub energy: i64,
}

impl State {
    pub fn new(inst: &MarketSplitInstance, sol: Vec<i8>) -> Self {
        let mut residuals = Vec::with_capacity(inst.num_cons);
        let mut energy = 0i64;
        for k in 0..inst.num_cons {
            let mut sum = 0i64;
            for j in 0..inst.num_vars {
                if sol[j] == 1 {
                    sum += inst.matrix[k][j];
                }
            }
            let r = sum - inst.rhs[k];
            residuals.push(r);
            energy += r * r;
        }
        Self {
            sol,
            residuals,
            energy,
        }
    }

    pub fn quench_swap_2opt(&mut self, inst: &MarketSplitInstance) {
        let n = inst.num_vars;
        let m = inst.num_cons;
        let mut improved = true;

        while improved && self.energy > 0 {
            improved = false;
            let mut best_delta = 0i64;
            let mut best_move = None;

            let ones: Vec<usize> = (0..n).filter(|&p| self.sol[p] == 1).collect();
            let zeros: Vec<usize> = (0..n).filter(|&p| self.sol[p] == 0).collect();

            'scan: for &p in &ones {
                for &q in &zeros {
                    let mut delta = 0i64;
                    for k in 0..m {
                        let dr = inst.matrix[k][q] - inst.matrix[k][p];
                        delta += 2 * self.residuals[k] * dr + dr * dr;
                    }
                    if delta < best_delta {
                        best_delta = delta;
                        best_move = Some((p, q));
                        if delta <= -8 {
                            break 'scan;
                        }
                    }
                }
            }

            if let Some((p, q)) = best_move {
                self.sol[p] = 0;
                self.sol[q] = 1;
                let mut new_e = 0i64;
                for k in 0..m {
                    self.residuals[k] += inst.matrix[k][q] - inst.matrix[k][p];
                    new_e += self.residuals[k] * self.residuals[k];
                }
                self.energy = new_e;
                improved = true;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================================================================");
    println!(
        "     QOBLIB 2026: LLL / AARDAL LATTICE BASIS SCREENING ON 41 INSTANCES                   "
    );
    println!("==========================================================================================");
    println!("Problem Class: 01-marketsplit (Cornuéjols & Dawande 1998, QOBLIB 2026)");
    println!("Analytical Core: LLL Lattice Basis Reduction + Aardal-Hurkens-Lenstra (2000)");
    println!("Target: 41 known-feasible instances (m in [12..15], n in [50..140])");
    println!("------------------------------------------------------------------------------------------\n");

    let unsolved_dir = Path::new("benchmarks/qoblib/marketsplit/unsolved_instances");
    let sol_dir = Path::new("benchmarks/qoblib/marketsplit/unsolved_solutions");
    std::fs::create_dir_all(sol_dir)?;

    let mut entries: Vec<PathBuf> = std::fs::read_dir(unsolved_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "dat"))
        .collect();
    entries.sort();

    println!("Loaded {} benchmark instances from disk.", entries.len());
    println!(
        "\n{:<16} | {:>4} | {:>4} | {:>8} | {:>10} | {:>11} | {:>9} | {:>12}",
        "Instance", "Cons", "Vars", "Kernel", "Exact Ax0=b", "Clamped E", "Quench E", "Time (s)"
    );
    println!(
        "{:-<16}-+-{:-<4}-+-{:-<4}-+-{:-<8}-+-{:-<10}-+-{:-<11}-+-{:-<9}-+-{:-<12}",
        "", "", "", "", "", "", "", ""
    );

    let mut direct_solutions_count = 0usize;

    for path in &entries {
        let inst = parse_marketsplit_dat(path)?;
        let t0 = Instant::now();

        // Run Aardal LLL reduction
        let reduction = aardal_diophantine_reduction(&inst.matrix, &inst.rhs, 10_000, 10);
        let elapsed = t0.elapsed();

        match reduction {
            Ok(res) => {
                let kernel_dim = res.kernel_basis.len();
                let has_exact_ax0_b = res.particular_solution.is_some();

                if let Some(ref exact_bool) = res.exact_boolean_solution {
                    let checker = Path::new("target/release/check_marketsplit");
                    let rejected_dir =
                        Path::new("benchmarks/qoblib/marketsplit/rejected_candidates");
                    let certificate = marketsplit_certificate::certify(
                        path,
                        &inst.name,
                        exact_bool,
                        checker,
                        sol_dir,
                        Path::new("benchmarks/qoblib/marketsplit/unsolved_candidates"),
                        rejected_dir,
                    )?;
                    let checker_status = match certificate {
                        marketsplit_certificate::CertificateStatus::Verified(ref file) => {
                            direct_solutions_count += 1;
                            println!("    Verified certificate: {}", file.display());
                            "VALID (0 viol)"
                        }
                        marketsplit_certificate::CertificateStatus::Rejected(ref file) => {
                            println!("    Rejected candidate retained: {}", file.display());
                            "INVALID"
                        }
                    };

                    println!(
                        "   {:<14} | {:>4} | {:>4} | {:>8} | {:>10} | {:>11} | {:>9} | {:>11.3}s [CHECKER: {}]",
                        inst.name, inst.num_cons, inst.num_vars, kernel_dim, "YES", 0, 0, elapsed.as_secs_f64(), checker_status
                    );
                } else if let Some(ref particular) = res.particular_solution {
                    // Evaluate clamped particular solution
                    let clamped: Vec<i8> = particular
                        .iter()
                        .map(|&v| if v > 0 { 1 } else { 0 })
                        .collect();
                    let clamped_state = State::new(&inst, clamped.clone());
                    let mut quenched_state = State::new(&inst, clamped);
                    quenched_state.quench_swap_2opt(&inst);

                    println!(
                        "   {:<14} | {:>4} | {:>4} | {:>8} | {:>10} | {:>11} | {:>9} | {:>11.3}s",
                        inst.name,
                        inst.num_cons,
                        inst.num_vars,
                        kernel_dim,
                        if has_exact_ax0_b { "YES" } else { "NO" },
                        clamped_state.energy,
                        quenched_state.energy,
                        elapsed.as_secs_f64()
                    );
                } else {
                    println!(
                        "   {:<14} | {:>4} | {:>4} | {:>8} | {:>10} | {:>11} | {:>9} | {:>11.3}s",
                        inst.name,
                        inst.num_cons,
                        inst.num_vars,
                        kernel_dim,
                        "NO",
                        "—",
                        "—",
                        elapsed.as_secs_f64()
                    );
                }
            }
            Err(e) => {
                println!(
                    "   {:<14} | {:>4} | {:>4} | {:>8} | {:>10} | {:>11} | {:>9} | {:>11.3}s [ERR: {}]",
                    inst.name, inst.num_cons, inst.num_vars, "—", "—", "—", "—", elapsed.as_secs_f64(), e
                );
            }
        }
    }

    println!("\n==========================================================================================");
    println!(
        "LLL Screening Complete. Direct Exact Boolean Solutions Found: {} / {}",
        direct_solutions_count,
        entries.len()
    );
    println!("==========================================================================================");

    Ok(())
}
