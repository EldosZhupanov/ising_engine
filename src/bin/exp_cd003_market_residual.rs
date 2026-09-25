//! Isolated CD003-MR1 structural probe. The frozen protocol lives under
//! research/experiments/cd003_market_residual/protocol.md.
use ising_engine::core::{CsrMatrix, QuboModel};
use ising_engine::presolve::full_presolve;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::Instant;

struct MarketSplit {
    rows: Vec<Vec<i64>>,
    rhs: Vec<i64>,
    n: usize,
}

fn parse(path: &Path) -> Result<MarketSplit, Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let mut data = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let header: Vec<_> = data
        .next()
        .ok_or("missing header")?
        .split_whitespace()
        .collect();
    if header.len() != 2 {
        return Err("header must contain exactly m n".into());
    }
    let m: usize = header[0].parse()?;
    let n: usize = header[1].parse()?;
    if m == 0 || n == 0 {
        return Err("m and n must be positive".into());
    }
    let mut rows = Vec::with_capacity(m);
    let mut rhs = Vec::with_capacity(m);
    for _ in 0..m {
        let values: Vec<i64> = data
            .next()
            .ok_or("missing coefficient row")?
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        if values.len() != n + 1 {
            return Err("coefficient row length differs from n + 1".into());
        }
        rows.push(values[..n].to_vec());
        rhs.push(values[n]);
    }
    if data.next().is_some() {
        return Err("extra coefficient row".into());
    }
    Ok(MarketSplit { rows, rhs, n })
}

fn to_qubo(input: &MarketSplit) -> QuboModel {
    let n = input.n;
    let mut linear = vec![0.0; n];
    let mut offset = 0.0;
    for (row, &rhs) in input.rows.iter().zip(&input.rhs) {
        let b = rhs as f64;
        offset += b * b;
        for (j, &coefficient) in row.iter().enumerate() {
            let a = coefficient as f64;
            linear[j] += a * a - 2.0 * b * a;
        }
    }
    let mut adjacency = vec![Vec::<(usize, f64)>::new(); n];
    for i in 0..n {
        for j in i + 1..n {
            let weight: f64 = 2.0
                * input
                    .rows
                    .iter()
                    .map(|row| (row[i] as f64) * (row[j] as f64))
                    .sum::<f64>();
            if weight != 0.0 {
                adjacency[i].push((j, weight));
                adjacency[j].push((i, weight));
            }
        }
    }
    let mut csr = CsrMatrix::empty(n);
    csr.row_offsets.clear();
    csr.row_offsets.push(0);
    for row in adjacency {
        for (j, weight) in row {
            csr.col_indices.push(j);
            csr.values.push(weight);
        }
        csr.row_offsets.push(csr.values.len());
    }
    QuboModel {
        num_vars: n,
        linear,
        quadratic: csr,
        energy_offset: offset,
    }
}

fn direct_energy(input: &MarketSplit, state: &[i8]) -> i64 {
    input
        .rows
        .iter()
        .zip(&input.rhs)
        .map(|(row, &rhs)| {
            let residual: i64 = row
                .iter()
                .zip(state)
                .map(|(&a, &x)| a * i64::from(x))
                .sum::<i64>()
                - rhs;
            residual * residual
        })
        .sum()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("usage: exp_cd003_market_residual INSTANCE.dat".into());
    }
    let path = Path::new(&args[1]);
    let input = parse(path)?;
    let model = to_qubo(&input);
    for mask in 0..8usize {
        let state: Vec<i8> = (0..input.n)
            .map(|i| ((mask >> (i % 3)) & 1) as i8)
            .collect();
        let direct = direct_energy(&input, &state) as f64;
        if (model.calculate_total_energy(&state) - direct).abs() > 1e-8 {
            return Err("QUBO energy mismatch".into());
        }
    }
    let started = Instant::now();
    let mut fixed = full_presolve(&model, &[]);
    let presolve_ms = started.elapsed().as_secs_f64() * 1000.0;
    fixed.sort_unstable_by_key(|&(i, _)| i);
    let mut free = Vec::with_capacity(input.n.saturating_sub(fixed.len()));
    let mut cursor = 0;
    for i in 0..input.n {
        if cursor < fixed.len() && fixed[cursor].0 == i {
            if fixed[cursor].1 != 0 && fixed[cursor].1 != 1 {
                return Err("nonbinary fixing".into());
            }
            cursor += 1;
        } else {
            free.push(i);
        }
    }
    if cursor != fixed.len() {
        return Err("duplicate or out-of-range fixing".into());
    }
    let zero = vec![0; input.n];
    let one = vec![1; input.n];
    println!(
        "{}",
        json!({
            "instance": path.file_name().ok_or("missing filename")?.to_string_lossy(),
            "m": input.rows.len(),
            "n": input.n,
            "fixed": fixed,
            "free": free,
            "presolve_ms": presolve_ms,
            "energy_zero": model.calculate_total_energy(&zero),
            "energy_one": model.calculate_total_energy(&one)
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{direct_energy, to_qubo, MarketSplit};

    #[test]
    fn exact_marketsplit_qubo_energy() {
        let input = MarketSplit {
            rows: vec![vec![1, 2, 3], vec![2, -1, 1]],
            rhs: vec![3, 1],
            n: 3,
        };
        let model = to_qubo(&input);
        for mask in 0..8 {
            let state: Vec<i8> = (0..3).map(|i| ((mask >> i) & 1) as i8).collect();
            assert_eq!(
                model.calculate_total_energy(&state),
                direct_energy(&input, &state) as f64
            );
        }
    }
}
