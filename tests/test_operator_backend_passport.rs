//! RC-009 — does the capability passport match reality?
//!
//! ADR-0001 makes the passport load-bearing: operators are selected by
//! capability, never by name, so a wrong `Constraints` field silently changes
//! every selection decision. Two claims are declared by **all** registered
//! operators and were verified for none of them:
//!
//!   supports_sparse: true      supports_dense: true
//!
//! `tests/test_dense_byte_verification.rs` verifies the DenseByte *backend*
//! primitives against the f64 oracle over thousands of checks — but exercises
//! **zero operators**. So "every operator runs correctly on DenseByte" was an
//! unverified universal claim about state representation.
//!
//! This suite closes that gap. For every operator in the standard registry, on
//! every backend its passport claims to support:
//!
//!   1. it does not panic,
//!   2. `audit()` reports zero ledger drift,
//!   3. the resulting energies are **identical** to the reference oracle
//!      (the ADR-0004 bit-identity firewall, extended to DenseByte).
//!
//! A failure here is not a style issue: it means the Decision Engine can route
//! an operator onto a backend where it misbehaves.

use ising_engine::engine_v2::backends::{DenseByteState, ReferenceState, SparseBitSlice};
use ising_engine::engine_v2::ir::ProblemIR;
use ising_engine::engine_v2::operator::Budget;
use ising_engine::engine_v2::registry::OperatorRegistry;
use ising_engine::engine_v2::runtime::RuntimeView;
use ising_engine::engine_v2::state::SpinState;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Small, integral, mixed-sign, connected instance: accepted by every backend
/// (SparseBitSlice requires integral weights) and frustrated enough to exercise
/// cluster and extremal operators rather than trivially converging.
fn probe_ir() -> ProblemIR {
    ProblemIR::from_pairs(
        12,
        -1.0,
        vec![
            1.0, -2.0, 3.0, 0.0, -1.0, 2.0, 1.0, -3.0, 2.0, 0.0, -1.0, 1.0,
        ],
        &[
            (0, 1, -1.0),
            (1, 2, 2.0),
            (2, 3, -3.0),
            (0, 3, 1.0),
            (3, 4, -1.0),
            (4, 5, 2.0),
            (5, 6, -2.0),
            (2, 6, 1.0),
            (6, 7, 1.0),
            (7, 8, -2.0),
            (8, 9, 3.0),
            (9, 10, -1.0),
            (10, 11, 2.0),
            (0, 11, -1.0),
            (4, 8, 1.0),
            (5, 9, -2.0),
        ],
    )
}

/// A diverse starting ensemble. From the all-zeros init every replica is
/// identical (RC-002), which makes `replica_exchange` and the cluster operators
/// structurally inert — they would pass this audit while doing nothing at all.
fn diverse_init(n: usize) -> Vec<u8> {
    // Per-site alternation gives a non-uniform configuration; per-replica
    // diversity is then created by the randomising warm-up in `run_on`.
    (0..n).map(|i| (i % 2) as u8).collect()
}

fn temps(r: usize) -> Vec<f64> {
    (0..r).map(|k| 2.0 * 0.9f64.powi(k as i32)).collect()
}

/// Run `name` on a freshly built state of each backend and return the per-replica
/// energies plus the audit drift.
fn run_on<S: SpinState>(
    state: &mut S,
    name: &str,
    r: usize,
    seed: u64,
) -> (Vec<f64>, f64, Vec<f64>) {
    let reg = OperatorRegistry::standard();
    let mut op = reg.lookup(name).expect("registered operator");
    let t = temps(r);
    let view = RuntimeView {
        iteration: 0,
        temperatures: &t,
        num_replicas: r,
        recent_acceptance: 0.25,
        remaining_ms: f64::INFINITY,
    };
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // A randomising warm-up first, so replica-coupled operators have genuine
    // diversity to act on rather than being silently inert.
    let mut warm = reg.lookup("random_flip_sweep").expect("warm-up operator");
    warm.apply(state, &view, &mut rng, Budget { sweeps: 1 });

    // Baseline AFTER the warm-up: lets the caller detect an operator that did
    // nothing at all, in which case "all backends agree" is vacuous.
    let mut base = vec![0.0; r];
    state.energies_into(&mut base);

    let mut rng2 = ChaCha8Rng::seed_from_u64(seed);
    op.apply(state, &view, &mut rng2, Budget { sweeps: 8 });

    let mut e = vec![0.0; r];
    state.energies_into(&mut e);
    (e, state.audit(), base)
}

#[test]
fn every_operator_honours_its_backend_passport() {
    let ir = probe_ir();
    let reg = OperatorRegistry::standard();
    let mut names: Vec<String> = reg.names().map(|s| s.to_string()).collect();
    names.sort();
    assert!(
        names.len() >= 18,
        "expected the full library, got {}",
        names.len()
    );

    let r = 8;
    let init = diverse_init(ir.n);
    let mut failures: Vec<String> = Vec::new();
    let mut inert: Vec<String> = Vec::new();

    for name in &names {
        let meta = reg.metadata(name).expect("descriptor");
        let c = meta.constraints;

        // Reference oracle is the ground truth for this operator.
        let mut refs = ReferenceState::new(&ir, r, &init);
        let (e_ref, drift_ref, base_ref) = run_on(&mut refs, name, r, 4242);
        if e_ref == base_ref {
            inert.push(name.clone());
        }
        if drift_ref != 0.0 {
            failures.push(format!("{name}: ReferenceState ledger drift {drift_ref}"));
        }

        if c.supports_sparse {
            match SparseBitSlice::new(&ir, r, &init) {
                Ok(mut bs) => {
                    let (e, drift, _) = run_on(&mut bs, name, r, 4242);
                    if drift != 0.0 {
                        failures.push(format!("{name}: SparseBitSlice ledger drift {drift}"));
                    }
                    if e != e_ref {
                        failures.push(format!(
                            "{name}: SparseBitSlice energies differ from oracle\n     oracle {e_ref:?}\n     sparse {e:?}"
                        ));
                    }
                }
                Err(err) => failures.push(format!(
                    "{name}: declares supports_sparse but SparseBitSlice rejected the instance: {err}"
                )),
            }
        }

        if c.supports_dense {
            let mut db = DenseByteState::new(&ir, r, &init);
            let (e, drift, _) = run_on(&mut db, name, r, 4242);
            if drift != 0.0 {
                failures.push(format!("{name}: DenseByte ledger drift {drift}"));
            }
            if e != e_ref {
                failures.push(format!(
                    "{name}: DenseByte energies differ from oracle\n     oracle {e_ref:?}\n     dense  {e:?}"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "capability passport does not match reality ({} issue(s)):\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    // Cross-backend agreement is only evidence if the operators actually acted.
    // Anything listed here passed vacuously and its passport remains unverified.
    eprintln!(
        "audited {} operators on all declared backends; {} left the energy unchanged{}",
        names.len(),
        inert.len(),
        if inert.is_empty() {
            String::new()
        } else {
            format!(
                " (VACUOUS, passport still unverified): {}",
                inert.join(", ")
            )
        }
    );
    assert!(
        inert.len() * 2 < names.len(),
        "{} of {} operators were inert — this audit would be mostly vacuous: {}",
        inert.len(),
        names.len(),
        inert.join(", ")
    );
}

/// `needs_replicas: false` is also a claim. An operator asserting it must
/// actually function with a single replica, or the passport is over-permissive
/// and the Decision Engine can route it into a configuration it cannot handle.
#[test]
fn operators_not_needing_replicas_work_at_r_equals_one() {
    let ir = probe_ir();
    let reg = OperatorRegistry::standard();
    let mut names: Vec<String> = reg.names().map(|s| s.to_string()).collect();
    names.sort();

    let init = diverse_init(ir.n);
    let mut failures: Vec<String> = Vec::new();

    for name in &names {
        let meta = reg.metadata(name).expect("descriptor");
        if meta.constraints.needs_replicas {
            continue; // honestly declared as requiring an ensemble
        }
        let mut refs = ReferenceState::new(&ir, 1, &init);
        let (_e, drift, _) = run_on(&mut refs, name, 1, 7);
        if drift != 0.0 {
            failures.push(format!(
                "{name}: declares needs_replicas=false but drifts at R=1 ({drift})"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "needs_replicas=false is inaccurate for {} operator(s):\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
