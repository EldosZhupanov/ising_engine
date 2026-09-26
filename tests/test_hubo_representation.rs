//! HUBO-RG001: exhaustive test-local adapters; no solver-performance claim.
//! Frozen protocol: research/experiments/hubo_representation_gate/protocol.md.
use ising_engine::compiler::LogicBuilder;
use ising_engine::core::hubo::{FlatHuboModel, HuboModel};
use ising_engine::solver::{engine, types::QuantumField};

#[derive(Clone)]
struct Polynomial {
    n: usize,
    offset: i64,
    terms: Vec<(i64, Vec<usize>)>,
}

impl Polynomial {
    // Integer oracle over the original monomials; no compiler/engine call.
    fn energy(&self, mask: usize) -> i64 {
        self.offset
            + self
                .terms
                .iter()
                .filter(|(_, vars)| vars.iter().all(|&v| mask & (1 << v) != 0))
                .map(|(coefficient, _)| coefficient)
                .sum::<i64>()
    }

    fn native(&self) -> (HuboModel, i64) {
        let mut builder = LogicBuilder::new();
        for _ in 0..self.n {
            builder.add_var();
        }
        builder.constant = self.offset as f64;
        for (coefficient, vars) in &self.terms {
            let w = *coefficient as f64;
            match vars.as_slice() {
                [a] => builder.linear[*a] += w,
                [a, b] => builder.quadratic.push((*a, *b, w)),
                [a, b, c] => builder.add_edge3(*a, *b, *c, w),
                [a, b, c, d] => builder.add_edge4(*a, *b, *c, *d, w),
                _ => panic!("fixture violates degree 1..4 contract"),
            }
        }
        // build_hubo has no offset field: keep this explicitly in the adapter.
        (builder.build_hubo(), self.offset)
    }
}

struct Reduction {
    q: Polynomial,
    substitutions: Vec<(usize, usize, usize)>,
}

fn quadratize(original: &Polynomial, penalty: i64) -> Reduction {
    assert!(penalty > 0);
    let mut reduced = Reduction {
        q: Polynomial {
            n: original.n,
            offset: original.offset,
            terms: Vec::new(),
        },
        substitutions: Vec::new(),
    };
    for (coefficient, vars) in &original.terms {
        let mut remaining = vars.clone();
        while remaining.len() > 2 {
            let u = remaining.remove(0);
            let v = remaining.remove(0);
            let z = reduced.q.n;
            reduced.q.n += 1;
            reduced.substitutions.push((u, v, z));
            reduced.q.terms.extend([
                (penalty, vec![u, v]),
                (-2 * penalty, vec![u, z]),
                (-2 * penalty, vec![v, z]),
                (3 * penalty, vec![z]),
            ]);
            remaining.push(z);
        }
        reduced.q.terms.push((*coefficient, remaining));
    }
    reduced
}

fn consistent(mask: usize, substitutions: &[(usize, usize, usize)]) -> bool {
    substitutions
        .iter()
        .all(|&(u, v, z)| ((mask >> z) & 1) == (((mask >> u) & 1) * ((mask >> v) & 1)))
}

fn fixtures() -> Vec<Polynomial> {
    let mut cases = Vec::new();
    for degree in [3, 4] {
        for coefficient in [-2, -1, 0, 1, 2] {
            cases.push(Polynomial {
                n: degree,
                offset: 7,
                terms: vec![(coefficient, (0..degree).collect()), (-1, vec![0])],
            });
        }
    }
    cases.push(Polynomial {
        n: 6,
        offset: -7,
        terms: vec![
            (3, vec![0]),
            (-2, vec![5]),
            (4, vec![0, 3]),
            (-5, vec![0, 1, 2]),
            (2, vec![1, 3, 5]),
            (7, vec![0, 2, 4, 5]),
            (-3, vec![1, 2, 3, 4]),
        ],
    });
    cases.push(Polynomial {
        n: 3,
        offset: 11,
        terms: vec![(-2, vec![0]), (3, vec![0, 2])],
    });
    cases.push(Polynomial {
        n: 1,
        offset: -4,
        terms: Vec::new(),
    });
    cases
}

fn populated(n: usize) -> QuantumField {
    let mut field = QuantumField::new(n, 1, 1, 1);
    for lane in 0..64 {
        let mask = lane % (1 << n);
        for v in 0..n {
            field.set_replica(v, 0, 0, 0, lane, ((mask >> v) & 1) as i8);
        }
    }
    field
}

#[test]
fn native_energy_and_deltas_match_integer_oracle_in_every_lane() {
    let mut energies_checked = 0;
    let mut deltas_checked = 0;
    for (id, original) in fixtures().iter().enumerate() {
        let (native, offset) = original.native();
        let flat = FlatHuboModel::from_hubo(&native);
        let field = populated(original.n);
        let energies =
            engine::calculate_replica_energies_local(&flat, &field.spins, original.n, 1, 0.0);
        for (lane, energy) in energies.iter().enumerate() {
            assert_eq!(
                *energy + offset as f64,
                original.energy(lane % (1 << original.n)) as f64,
                "fixture {id}, lane {lane}"
            );
            energies_checked += 1;
        }
        for v in 0..original.n {
            let deltas = engine::calculate_delta_e(&flat, &field, v, 0, 0, 0);
            for (lane, delta) in deltas.iter().enumerate() {
                let mask = lane % (1 << original.n);
                assert_eq!(
                    *delta,
                    (original.energy(mask ^ (1 << v)) - original.energy(mask)) as f64,
                    "fixture {id}, lane {lane}, variable {v}"
                );
                deltas_checked += 1;
            }
        }
    }
    assert_eq!((energies_checked, deltas_checked), (832, 2880));
    println!("13 fixtures: {energies_checked} lane energies, {deltas_checked} lane deltas");
}

#[test]
fn quadratization_minimum_and_all_minimizers_are_exact() {
    let mut original_states = 0;
    let mut expanded_states = 0;
    for original in fixtures() {
        let penalty = 1 + original
            .terms
            .iter()
            .filter(|(_, vars)| vars.len() > 2)
            .map(|(coefficient, _)| coefficient.abs())
            .sum::<i64>();
        let reduction = quadratize(&original, penalty);
        assert!(reduction.q.terms.iter().all(|(_, vars)| vars.len() <= 2));
        for x in 0..1 << original.n {
            let expected = original.energy(x);
            let mut minimum = i64::MAX;
            let mut valid_extensions = 0;
            for auxiliary in 0..1 << (reduction.q.n - original.n) {
                let mask = x | (auxiliary << original.n);
                let energy = reduction.q.energy(mask);
                if consistent(mask, &reduction.substitutions) {
                    assert_eq!(energy, expected);
                    valid_extensions += 1;
                } else {
                    assert!(
                        energy > expected,
                        "inconsistent auxiliary minimized objective"
                    );
                }
                minimum = minimum.min(energy);
                expanded_states += 1;
            }
            assert_eq!(minimum, expected);
            assert_eq!(valid_extensions, 1);
            original_states += 1;
        }
    }
    assert_eq!((original_states, expanded_states), (194, 4506));
    println!("{original_states} fixed-x minima, {expanded_states} expanded states");
}

#[test]
fn substitution_truth_table_is_zero_exactly_for_and() {
    for u in 0..=1 {
        for v in 0..=1 {
            for z in 0..=1 {
                let penalty = u * v - 2 * u * z - 2 * v * z + 3 * z;
                assert_eq!(penalty == 0, z == u * v);
                assert!(penalty >= 0);
            }
        }
    }
}

#[test]
fn insufficient_penalty_is_detected() {
    let original = Polynomial {
        n: 3,
        offset: 0,
        terms: vec![(-10, vec![0, 1, 2])],
    };
    let reduction = quadratize(&original, 1);
    let x = 0b110;
    assert_eq!(original.energy(x), 0);
    assert_eq!(reduction.q.energy(x | (1 << 3)), -9);
    assert!(!consistent(x | (1 << 3), &reduction.substitutions));
}

#[test]
fn omitted_offset_is_detected_without_changing_deltas() {
    let original = fixtures().remove(10);
    let (native, offset) = original.native();
    assert_ne!(offset, 0);
    let flat = FlatHuboModel::from_hubo(&native);
    let field = populated(original.n);
    let energies =
        engine::calculate_replica_energies_local(&flat, &field.spins, original.n, 1, 0.0);
    let mut shifted = original.clone();
    shifted.offset = 0;
    for (lane, &energy) in energies.iter().enumerate() {
        assert_ne!(energy, original.energy(lane) as f64);
        assert_eq!(energy, shifted.energy(lane) as f64);
        for v in 0..original.n {
            assert_eq!(
                original.energy(lane ^ (1 << v)) - original.energy(lane),
                shifted.energy(lane ^ (1 << v)) - shifted.energy(lane)
            );
        }
    }
}

#[test]
fn missing_incident_edge_is_detected() {
    let original = Polynomial {
        n: 4,
        offset: 0,
        terms: vec![(5, vec![0, 1, 2, 3])],
    };
    let (mut native, _) = original.native();
    assert_eq!(native.edges4[1].len(), 1);
    native.edges4[1].clear(); // Inject asymmetric incidence, preserving the min-index copy.
    let flat = FlatHuboModel::from_hubo(&native);
    let field = populated(original.n);
    let delta = engine::calculate_delta_e(&flat, &field, 1, 0, 0, 0)[15];
    assert_eq!(delta, 0.0);
    assert_eq!(original.energy(0b1101) - original.energy(0b1111), -5);
    assert_ne!(delta, -5.0);
}
