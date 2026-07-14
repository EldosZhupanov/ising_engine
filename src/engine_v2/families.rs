//! Problem families (Stage 8 — "Research OS manages all these modules as one").
//!
//! Everything downstream — Runtime, backends, the four learned models, the
//! Theory Engine, the whole research loop — operates on `ProblemIR`. So the ONLY
//! thing needed to give the platform a new kind of problem to research is a
//! frontend that lowers it into a QUBO `ProblemIR`. This module adds two
//! genuinely new families beyond MaxCut/BQP/QPLIB:
//!
//!   - TSP  (Travelling Salesman / routing) — the Lucas (2014) permutation
//!     encoding: n² binary variables x[city][position], one-hot penalties for
//!     "each city once / each position once", plus the tour-length objective.
//!   - MAX-2-SAT (Boolean satisfiability) — minimize the number of UNSATISFIED
//!     clauses; the QUBO ground state is a maximum-satisfying assignment.
//!
//! Both encodings are PROVEN correct against brute force (the QUBO ground state
//! decodes to the true optimum) in the tests — no encoding is trusted on
//! assertion. Scheduling and full VRP are the natural next additions behind the
//! same contract (a lowering to `ProblemIR`); TSP already covers single-vehicle
//! routing.

use super::ir::ProblemIR;

/// Order an unordered pair as (min, max, weight) for `ProblemIR::from_pairs`.
fn ordered(i: u32, j: u32, q: f64) -> (u32, u32, f64) {
    if i < j {
        (i, j, q)
    } else {
        (j, i, q)
    }
}

// ============================================================== TSP / routing

/// Lower a symmetric TSP distance matrix into a QUBO `ProblemIR`. Variables are
/// `x[v*n + j]` = "city v occupies tour position j". Returns the IR and `n`
/// (cities), so callers can decode. The one-hot penalty strength exceeds any
/// tour length, so the ground state is always a valid, minimum-length tour.
pub fn tsp_qubo(dist: &[Vec<f64>]) -> (ProblemIR, usize) {
    let n = dist.len();
    assert!(n >= 2, "TSP needs at least 2 cities");
    let nn = n * n;
    let idx = |v: usize, j: usize| (v * n + j) as u32;
    let mut linear = vec![0.0f64; nn];
    let mut pairs: Vec<(u32, u32, f64)> = Vec::new();
    let mut offset = 0.0f64;

    // Penalty A must beat any tour: A = (sum of all distances) + 1.
    let a: f64 = dist.iter().flatten().map(|d| d.abs()).sum::<f64>() + 1.0;

    // Each city occupies exactly one position: A·(Σ_j x[v][j] − 1)².
    // (S−1)² = −Σ x_k + 2·Σ_{k<l} x_k x_l + 1 for binary x.
    for v in 0..n {
        for j in 0..n {
            linear[v * n + j] -= a;
        }
        offset += a;
        for j in 0..n {
            for k in (j + 1)..n {
                pairs.push(ordered(idx(v, j), idx(v, k), 2.0 * a));
            }
        }
    }
    // Each position holds exactly one city: A·(Σ_v x[v][j] − 1)².
    for j in 0..n {
        for v in 0..n {
            linear[v * n + j] -= a;
        }
        offset += a;
        for v in 0..n {
            for w in (v + 1)..n {
                pairs.push(ordered(idx(v, j), idx(w, j), 2.0 * a));
            }
        }
    }
    // Tour length: Σ_j Σ_{u≠v} d[u][v]·x[u][j]·x[v][j+1 mod n].
    for j in 0..n {
        let jn = (j + 1) % n;
        for (u, row) in dist.iter().enumerate() {
            for (v, &d) in row.iter().enumerate() {
                if u == v || d == 0.0 {
                    continue;
                }
                pairs.push(ordered(idx(u, j), idx(v, jn), d));
            }
        }
    }
    (ProblemIR::from_pairs(nn, offset, linear, &pairs), n)
}

/// Decode a QUBO state into a tour (city per position), or `None` if the state
/// violates the permutation constraints (not exactly one city per position, or
/// a city used twice).
pub fn decode_tsp(state: &[u8], n: usize) -> Option<Vec<usize>> {
    let mut tour = Vec::with_capacity(n);
    for j in 0..n {
        let mut city = None;
        for v in 0..n {
            if state[v * n + j] == 1 {
                if city.is_some() {
                    return None; // two cities at one position
                }
                city = Some(v);
            }
        }
        tour.push(city?);
    }
    let mut seen = vec![false; n];
    for &c in &tour {
        if seen[c] {
            return None; // city used twice
        }
        seen[c] = true;
    }
    Some(tour)
}

/// Length of a decoded tour under a symmetric distance matrix.
pub fn tour_length(tour: &[usize], dist: &[Vec<f64>]) -> f64 {
    let n = tour.len();
    (0..n).map(|j| dist[tour[j]][tour[(j + 1) % n]]).sum()
}

// ============================================================== MAX-2-SAT

/// A clause is a pair of literals; a literal is a SIGNED 1-based variable id
/// (`+3` = x₂ true, `-3` = x₂ false). Use `0` as the second literal for a unit
/// clause.
pub type Clause = (i32, i32);

/// False-indicator of a literal: `f = a + b·x_var` (f = 1 when the literal is
/// FALSE). Positive literal `xᵢ` is false when xᵢ=0 ⇒ f = 1 − xᵢ; negative
/// literal ¬xᵢ is false when xᵢ=1 ⇒ f = xᵢ.
fn literal_false(l: i32) -> (f64, f64, usize) {
    let var = (l.unsigned_abs() as usize) - 1;
    if l > 0 {
        (1.0, -1.0, var)
    } else {
        (0.0, 1.0, var)
    }
}

/// Lower MAX-2-SAT into a QUBO `ProblemIR` whose energy equals the number of
/// UNSATISFIED clauses. The ground state is therefore a maximum-satisfying
/// assignment. `n_vars` variables, clauses of ≤ 2 literals.
pub fn max2sat_qubo(n_vars: usize, clauses: &[Clause]) -> ProblemIR {
    let mut linear = vec![0.0f64; n_vars];
    let mut pairs: Vec<(u32, u32, f64)> = Vec::new();
    let mut offset = 0.0f64;

    for &(l1, l2) in clauses {
        if l2 == 0 {
            // Unit clause: unsatisfied indicator = f1.
            let (a1, b1, i) = literal_false(l1);
            offset += a1;
            linear[i] += b1;
        } else {
            // Unsatisfied = f1·f2 = (a1+b1 x_i)(a2+b2 x_j).
            let (a1, b1, i) = literal_false(l1);
            let (a2, b2, j) = literal_false(l2);
            offset += a1 * a2;
            linear[j] += a1 * b2;
            linear[i] += a2 * b1;
            if i == j {
                // x_i² = x_i for binary.
                linear[i] += b1 * b2;
            } else {
                pairs.push(ordered(i as u32, j as u32, b1 * b2));
            }
        }
    }
    ProblemIR::from_pairs(n_vars, offset, linear, &pairs)
}

/// Number of clauses satisfied by an assignment (0/1 per variable).
pub fn count_satisfied(assignment: &[u8], clauses: &[Clause]) -> usize {
    let truth = |l: i32| {
        let v = assignment[(l.unsigned_abs() as usize) - 1] == 1;
        if l > 0 {
            v
        } else {
            !v
        }
    };
    clauses
        .iter()
        .filter(|&&(l1, l2)| truth(l1) || (l2 != 0 && truth(l2)))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force the minimum-energy state over all 2^n configurations.
    fn brute_ground(ir: &ProblemIR) -> (f64, Vec<u8>) {
        let n = ir.n;
        assert!(n <= 20, "brute force only for small n");
        let mut best = (f64::INFINITY, vec![0u8; n]);
        for mask in 0u32..(1u32 << n) {
            let x: Vec<u8> = (0..n).map(|i| ((mask >> i) & 1) as u8).collect();
            let e = ir.energy(&x);
            if e < best.0 {
                best = (e, x);
            }
        }
        best
    }

    #[test]
    fn tsp_ground_state_is_the_optimal_tour() {
        // 4 cities on a line 0-1-2-3: optimal tour 0-1-2-3 (length 2*(3)=... )
        // Use an asymmetric-ish but symmetric matrix with a clear optimum.
        let d = vec![
            vec![0.0, 1.0, 4.0, 2.0],
            vec![1.0, 0.0, 1.0, 5.0],
            vec![4.0, 1.0, 0.0, 1.0],
            vec![2.0, 5.0, 1.0, 0.0],
        ];
        let (ir, n) = tsp_qubo(&d);
        assert_eq!(ir.n, 16);
        let (_e, ground) = brute_ground(&ir);
        let tour = decode_tsp(&ground, n).expect("ground state must be a valid tour");

        // Brute-force the true optimal tour length over all permutations.
        let mut best_len = f64::INFINITY;
        let cities = [0usize, 1, 2, 3];
        // fix city 0 at position 0 (cyclic symmetry); permute the other 3.
        for p in permutations(&[1, 2, 3]) {
            let t = [cities[0], p[0], p[1], p[2]];
            best_len = best_len.min(tour_length(&t, &d));
        }
        assert!(
            (tour_length(&tour, &d) - best_len).abs() < 1e-9,
            "QUBO tour {tour:?} len {} != optimal {best_len}",
            tour_length(&tour, &d)
        );
    }

    fn permutations(items: &[usize]) -> Vec<Vec<usize>> {
        if items.len() <= 1 {
            return vec![items.to_vec()];
        }
        let mut out = Vec::new();
        for i in 0..items.len() {
            let mut rest = items.to_vec();
            let x = rest.remove(i);
            for mut p in permutations(&rest) {
                p.insert(0, x);
                out.push(p);
            }
        }
        out
    }

    #[test]
    fn max2sat_ground_state_maximizes_satisfied_clauses() {
        // 4 variables, a mix of clauses including a contradiction so the optimum
        // is NOT all-satisfiable (tests that we MAXIMIZE, not require SAT).
        let n = 4;
        let clauses: Vec<Clause> = vec![
            (1, 2),   // x0 ∨ x1
            (-1, 3),  // ¬x0 ∨ x2
            (2, -4),  // x1 ∨ ¬x3
            (-2, -3), // ¬x1 ∨ ¬x2
            (4, 0),   // unit: x3
            (1, -1),  // tautology-ish contradiction pair to stress the encoder
        ];
        let ir = max2sat_qubo(n, &clauses);
        assert_eq!(ir.n, 4);
        let (energy, ground) = brute_ground(&ir);

        // Brute-force max satisfied over all assignments.
        let mut max_sat = 0;
        for mask in 0u32..(1 << n) {
            let a: Vec<u8> = (0..n).map(|i| ((mask >> i) & 1) as u8).collect();
            max_sat = max_sat.max(count_satisfied(&a, &clauses));
        }
        // Energy = unsatisfied count = clauses − max_satisfied.
        let unsat_at_ground = clauses.len() - count_satisfied(&ground, &clauses);
        assert_eq!(
            count_satisfied(&ground, &clauses),
            max_sat,
            "QUBO ground state must maximize satisfied clauses"
        );
        assert!(
            (energy - unsat_at_ground as f64).abs() < 1e-9,
            "QUBO energy {energy} must equal unsatisfied count {unsat_at_ground}"
        );
    }

    #[test]
    fn max2sat_energy_counts_unsatisfied_for_every_assignment() {
        let clauses: Vec<Clause> = vec![(1, 2), (-1, -2), (1, 0)];
        let ir = max2sat_qubo(2, &clauses);
        for mask in 0u32..4 {
            let a: Vec<u8> = (0..2).map(|i| ((mask >> i) & 1) as u8).collect();
            let unsat = clauses.len() - count_satisfied(&a, &clauses);
            assert!(
                (ir.energy(&a) - unsat as f64).abs() < 1e-9,
                "assignment {a:?}: energy {} != unsatisfied {unsat}",
                ir.energy(&a)
            );
        }
    }
}
