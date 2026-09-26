# HUBO-RG001 — correctness gate PASS, performance untested

Date: 2026-09-27. Scope: deterministic representation checks only.
Protocol frozen at `7542602`, reviewed source frozen at `b9b99fe` before test
execution. No production code, public API, solver family or dependency changed.

## Outcome

`tests/test_hubo_representation.rs`: **6 passed, 0 failed**.

| Exhaustive check | Count | Mismatches |
|---|---:|---:|
| Native MSC lane energies versus original integer polynomial | 832 | 0 |
| Native MSC single-flip deltas versus integer energy differences | 2,880 | 0 |
| Original assignments with min-over-auxiliary identity | 194 | 0 |
| Expanded assignments evaluated for identity/strict penalty gap | 4,506 | 0 |

All thirteen fixtures follow the frozen protocol, including both coefficient
signs, zero coefficients, overlapping cubic/quartic monomials, nonzero offsets
and lower-order/constant-only controls. All 64 native lanes are inspected;
repeated lanes on smaller fixtures are coverage, not independent samples.
There is no statistical generalization claim from these counts.

The known product-substitution penalty has a direct bound: changing auxiliary
values can change the reduced nonpenalty part by at most the sum of absolute
high-order coefficients B. Any inconsistent substitution incurs at least M.
With fresh acyclic auxiliary definitions and M=B+1, every inconsistent extension
is strictly worse than the unique consistent extension at the same original x.
The exhaustive implementation checks found no violation on the frozen fixtures.

All three falsification controls were detected:

- Weak penalty M=1 for -10*x0*x1*x2 permits invalid auxiliary energy -9 at
  x=(0,1,1), while the original energy is 0.
- Omitting the nonzero constant changes full energy even though all local deltas
  are unchanged. The test adapter keeps that constant separately and adds it once.
- Deleting one quartic incidence gives an incorrect zero flip delta instead of -5.

These tests exercise a simple Rosenberg-type reduction, not an optimized Ishikawa
implementation. Passing them establishes neither baseline competitiveness nor
higher-order speed/success advantage.

## Existing evidence corrected prospectively

The completed breakthrough EXP001 studied elimination and pair refinement.
The similarly named HUBO EXP-001 in NEXT_10_EXPERIMENTS is a proposal. Neither
supplies a completed native-HUBO-versus-quadratization benchmark. No older record
was edited or rerun.

The existing HuboModel/FlatHuboModel represents 0/1 monomial products and has no
energy-offset field. The experiment keeps the original constant externally;
changing a production public type was unnecessary for this gate.
UltimateSolver's public solve takes QuboModel. Kernel support for degree four
must not be described as an arbitrary-HUBO production entry point.

## Evidence and reproduction

- [Environment and source hashes](environment.json): source commit, Rust/Cargo,
  platform, build-job limit and hashes of tested dependencies/protocol.
- [Commands and statuses](verification.json).
- [New tests](test_0.log): all six passed.
- [Existing relevant regressions](test_1.log): test_hubo_qpa 2/2 and test_anls 1/1.
- Compile-only source check, targeted Clippy with -D warnings and cargo fmt
  passed before execution. No src changes require a new full-root gate run.
- Independent read-only review accepted protocol, proof, fixture counts and
  implementation before execution. Post-run independent review matched all six
  dependency/protocol hashes to `b9b99fe`, reran all six new and three existing
  tests, and reproduced every count: PASS. No timing or stochastic solve
  campaign was executed.

Reproduce from repository root:

```bash
CARGO_BUILD_JOBS=2 cargo test --release --test test_hubo_representation -- --nocapture
CARGO_BUILD_JOBS=2 cargo test --release --test test_hubo_qpa --test test_anls
CARGO_BUILD_JOBS=2 cargo clippy --test test_hubo_representation -- -D warnings
cargo fmt --check
```

## Decision and next task

**GO to design a matched-cost comparison; advantage remains UNKNOWN.**
A four-week research direction begins with this gate; it is not complete here.
Next preregistration must distinguish representation effects from engine effects:
first use the same MSC search kernel for direct and quadratic representations,
then challenge any signal with a strong independently implemented baseline.
Do not use this deliberately simple reduction alone to claim a competitive win.
Include reduction time, auxiliary count, coefficient range/penalty selection,
original-objective verification, fixed training/tuning and held-out instances,
>=10 stochastic seeds, external deadlines and all censored failures. Set a
practical improvement margin and termination criterion before execution.

No record hunt or Laya training follows from this correctness result. Laya's
separate training hypothesis and existing prior art are assessed in
[its existing guide](../../laya_semantic/README.md).
