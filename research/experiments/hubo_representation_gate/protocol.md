# HUBO-RG001 — representation correctness gate

Date: 2026-09-27. Binding on commit before implementation/execution.
Scope: deterministic correctness qualification, not a stochastic benchmark.

## Question and success criterion

Can a test-local adapter feed the existing product-basis MSC kernel and a simple
quadratic reduction with exactly the same original objective? Require zero
mismatches for every enumerated original assignment and every single-flip delta
in the fixtures below. A failure blocks performance measurement; it does not
refute the possible utility of higher-order optimization.

No TTS, speed, novelty or superiority conclusion is authorized. Seeds, statistical
sample sizes and significance tests are N/A for exhaustive deterministic tests.

## Existing evidence and dependency map

The completed `research/breakthrough/EXP001_PROTOCOL.md` / `EXP001_RESULT.md`
study degree-0/1/2 elimination and pair refinement, not HUBO quadratization.
`research/NEXT_10_EXPERIMENTS.md` separately uses EXP-001 for an unexecuted
HUBO proposal. This gate gets a distinct identifier; neither record is edited.

- `src/compiler/logic_builder.rs`: public `LogicBuilder`, `build_hubo()` and
  degree-3/4 insertion. `build_hubo()` does not carry the builder's constant.
- `src/core/hubo.rs`: product-basis HuboModel, FlatHuboModel, Edge2/3/4; no offset.
- `src/solver/engine.rs`: `calculate_replica_energies_local`, `calculate_delta_e`.
- `src/solver/types.rs`: QuantumField owns 64 byte lanes.
- `src/solver/ultimate.rs`: public `solve` takes QuboModel, not arbitrary HUBO.
- `tests/test_hubo_qpa.rs`: existing basic higher-order and multi-population tests.
- New test-only file: `tests/test_hubo_representation.rs`.

Keep the offset explicitly outside the MSC model in the test adapter, add it once
for full objective reporting, and never include it in a flip delta. This documents
an existing integration limitation, not a production bug fix or new public API.
No production source, solver family, public type, dependency or frozen evidence
changes. No inference from engine capability to UltimateSolver HUBO support.

## Exact reference and reduction

Binary polynomial f(x) = c + sum a_S product(x_i, i in S), with unique indices
per monomial, integer coefficients and maximum degree four. Oracle evaluates the
original term list directly in integer arithmetic; it never calls the engine or
reduction to obtain expected energies.

The test-local competitor uses fresh auxiliaries per term: repeatedly substitute
z=uv until degree <=2, adding M(uv-2uz-2vz+3z) for each substitution.
Use M=1+sum(abs(a_S), degree(S)>2). For a fixed x, a consistent auxiliary
assignment has zero penalties and yields f(x). An inconsistent assignment has
at least one unit penalty; changing the reduced nonpenalty high-order objective
relative to its consistent value changes it by at most sum(abs(a_S), degree(S)>2). Hence
all inconsistent assignments have strictly greater energy. Exhaustively require
f(x)=min_z Q(x,z) for every x, with no inconsistent minimizer. This is a simple
Rosenberg-type correctness baseline, not an optimized Ishikawa implementation
and not sufficient as the only baseline for a future performance claim.

The penalty truth table and minimum-over-auxiliary definition are standard;
see https://docs.dwavequantum.com/en/latest/quantum_research/reformulating.html
and https://arxiv.org/abs/1404.6538 (2014 preprint; no novelty claim).

## Frozen fixtures and falsification checks

1. Single cubic x0*x1*x2 at n=3 and quartic x0*x1*x2*x3 at n=4, coefficients
   {-2,-1,0,1,2} (ten fixtures), plus offset 7 and linear coefficient -1 on variable zero.
2. Mixed n=6 fixture: offset -7; linear +3*x0 -2*x5; pair +4*x0*x3;
   cubic -5*x0*x1*x2 +2*x1*x3*x5; quartic +7*x0*x2*x4*x5
   -3*x1*x2*x3*x4. Includes overlapping terms and both signs.
3. No-high-order control n=3: offset 11, linear -2*x0, pair +3*x0*x2;
   and constant-only n=1, offset -4.
4. Use one slice, one temperature, one population and j_tau=0. Populate every
   lane of the 64-lane QuantumField with enumerated assignments
   (repeat cyclically for small n); inspect every lane, all variables and flips.
   Compare integer oracle to engine f64 exactly (small integers exactly representable).
5. Test the substitution truth table independently for all eight u,v,z values.
6. Deliberately insufficient penalty: cubic -10*x0*x1*x2 with M=1 must violate
   the minimum-over-auxiliaries identity at some fixed original assignment.
7. Dropping a nonzero offset must fail full-energy equivalence, while local
   deltas remain identical. Missing incident high-order edges must fail a delta
   check. Controls pass only if those injected defects are detected.

## Execution and publication

Commit this protocol first. Record implementation commit and environment before
running `cargo test --release --test test_hubo_representation -- --nocapture`.
Use `CARGO_BUILD_JOBS=2` to bound compilation load. Expected test enumeration is
well below a million states; no search campaign, wall-time comparison or GPU.
Run relevant existing `test_hubo_qpa` and `test_anls`, formatting, Clippy on the
new target, memory checks and diff checks. Independent read-only review must
validate the finished code, proof, non-vacuous controls and result claims.

Preserve raw test output and metadata; publish PASS/FAIL and counts. PASS permits
design of a separately preregistered matched-cost experiment; it does not launch
one automatically. That design must include strong quadratization/specialist
baselines, original-objective witnesses, penalties, startup/preprocessing costs,
new held-out instances/seeds, budget allocation and stopping criteria.
