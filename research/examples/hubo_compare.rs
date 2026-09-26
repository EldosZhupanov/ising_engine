//! HUBO-C001 driver: unchanged MSC step, externally enforced deadline.
use ising_engine::compiler::LogicBuilder;
use ising_engine::core::hubo::FlatHuboModel;
use ising_engine::solver::{engine, types::QuantumField};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::Deserialize;
use serde_json::json;
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    n: usize,
    original_n: usize,
    terms: Vec<(i64, Vec<usize>)>,
    spin_terms: Vec<(i64, Vec<usize>)>,
    hot: f64,
    seed: u64,
}

struct State {
    model: FlatHuboModel,
    field: QuantumField,
    scratch: engine::StepScratch,
    temps: Vec<f64>,
    clamped: Vec<bool>,
    rng: StdRng,
}

fn prepare(req: &Request) -> Result<State, &'static str> {
    if req.original_n == 0
        || req.original_n > req.n
        || req.n > 4096
        || !req.hot.is_finite()
        || req.hot < 0.1
    {
        return Err("invalid dimensions/temperature");
    }
    let mut b = LogicBuilder::new();
    for _ in 0..req.n {
        b.add_var();
    }
    for (w, v) in &req.terms {
        if v.iter().any(|&i| i >= req.n) || v.windows(2).any(|p| p[0] >= p[1]) {
            return Err("invalid model term");
        }
        match v.as_slice() {
            [] => b.constant += *w as f64,
            [a] => b.linear[*a] += *w as f64,
            [a, c] => b.quadratic.push((*a, *c, *w as f64)),
            [a, c, d] => b.add_edge3(*a, *c, *d, *w as f64),
            [a, c, d, e] => b.add_edge4(*a, *c, *d, *e, *w as f64),
            _ => return Err("degree above four"),
        }
    }
    for (_, v) in &req.spin_terms {
        if v.iter().any(|&i| i >= req.original_n) {
            return Err("invalid oracle term");
        }
    }
    let model = FlatHuboModel::from_hubo(&b.build_hubo());
    let mut field = QuantumField::new(req.n, 1, 8, 1);
    let mut rng = StdRng::seed_from_u64(req.seed);
    for t in 0..8 {
        for v in 0..req.n {
            let word = rng.gen::<u64>();
            for r in 0..64 {
                field.set_replica(v, 0, t, 0, r, ((word >> r) & 1) as i8);
            }
        }
        field.energies[t] = engine::calculate_replica_energies(&model, &field, t, 0, 0.0);
    }
    let temps = (0..8)
        .map(|i| req.hot * (0.1 / req.hot).powf(i as f64 / 7.0))
        .collect();
    let scratch = engine::StepScratch::for_field(&field);
    Ok(State {
        model,
        field,
        scratch,
        temps,
        clamped: vec![false; req.n],
        rng,
    })
}

fn advance(s: &mut State) {
    engine::step(
        &mut s.field,
        &s.model,
        &s.temps,
        0.0,
        &s.clamped,
        &mut s.scratch,
        &mut s.rng,
    );
}

fn original_energy(req: &Request, field: &QuantumField, t: usize, r: usize) -> i64 {
    req.spin_terms
        .iter()
        .map(|(w, vars)| {
            w * vars
                .iter()
                .map(|&v| 2 * i64::from(field.get_replica(v, 0, t, 0, r)) - 1)
                .product::<i64>()
        })
        .sum()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let line = io::stdin().lock().lines().next().ok_or("missing input")??;
    let req: Request = serde_json::from_str(&line)?;
    let mut state = prepare(&req)?;
    let mut best = i64::MAX;
    let mut steps = 0_u64;
    let mut out = io::stdout().lock();
    loop {
        advance(&mut state);
        steps += 1;
        for t in 0..8 {
            for lane in 0..64 {
                let e = original_energy(&req, &state.field, t, lane);
                if e < best {
                    best = e;
                    let bits: String = (0..req.original_n)
                        .map(|v| {
                            if state.field.get_replica(v, 0, t, 0, lane) == 1 {
                                '1'
                            } else {
                                '0'
                            }
                        })
                        .collect();
                    writeln!(
                        out,
                        "{}",
                        json!({"kind":"inc","energy":e,"bits":bits,"steps":steps})
                    )?;
                    out.flush()?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Request {
        Request {
            n: 4,
            original_n: 4,
            terms: vec![
                (7, vec![]),
                (-2, vec![0]),
                (3, vec![0, 1, 2]),
                (-5, vec![0, 1, 2, 3]),
            ],
            spin_terms: vec![(1, vec![0, 1, 2, 3])],
            hot: 10.0,
            seed: 910001,
        }
    }
    #[test]
    fn replay_and_incremental_energy_match_full_integer_polynomial() {
        let req = fixture();
        let mut a = prepare(&req).unwrap();
        let mut b = prepare(&req).unwrap();
        for _ in 0..9 {
            assert_eq!(a.field.spins, b.field.spins);
            for t in 0..8 {
                for r in 0..64 {
                    let e: i64 = req
                        .terms
                        .iter()
                        .filter(|(_, v)| v.iter().all(|&i| a.field.get_replica(i, 0, t, 0, r) == 1))
                        .map(|(w, _)| w)
                        .sum();
                    assert_eq!(a.field.energies[t][r] + 7.0, e as f64);
                }
            }
            advance(&mut a);
            advance(&mut b);
        }
    }
    #[test]
    fn rejects_invalid_request() {
        let mut req = fixture();
        req.terms.push((1, vec![1, 1, 2]));
        assert!(prepare(&req).is_err());
        let mut req = fixture();
        req.hot = f64::NAN;
        assert!(prepare(&req).is_err());
    }
}
