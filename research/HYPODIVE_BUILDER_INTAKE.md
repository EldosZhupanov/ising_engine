# HYPODIVE intake: Ising as a cross-domain falsification instrument

Date 2026-09-17. Mode C — research evidence. User explicitly requested the
five-step map/boundary/transfer/novelty/minimal-experiment workflow and both skills.
Scope is this new cycle CD001/H11, not all science or the unrelated fundamental_ai
project. Current-task authority remains memory/NOW.md.

Objective: prevent another large implementation of an already known or false
mechanism. Map seven mechanism families, distinguish exact equivalence from
analogy, and run the cheapest decisive test of a concrete transfer assumption.

Strongest proposed claim under attack: a rank-one interaction between an Ising
block and its boundary entails only polynomially many distinct conditional optima.
The algebraic counterexample is known to us BEFORE execution; this is a deductive
witness check, not held-out empirical confirmation or a performance benchmark.

Canonical path: breakthrough/h11/witness.py. Exact Python integer coefficients,
QUBO-to-Ising conversion, exhaustive minimization, and small synthetic tests.
No production source, dependency, API, solver-family or public signature changes.
Invariants: square energy equals expanded QUBO equals native Ising scaled by four;
exact rank-one rectangular interface; conditional minima are unique in
binary_square and separable, while unit_square deliberately permits ties;
coefficient scale and normalized gap exposed. Baselines: exhaustive search and
closed-form identity; table response count is not a running-time claim.

Evidence to produce: mechanism/prior-art map, symbolic proof, machine-readable
rows and source/config hash, independent review, scoped GO/NARROW/NO-GO verdict.
Success: a correct decision about H11, even NO-GO. Stop after the witness decides
it; do not rescue the claim by changing its precision assumptions afterward.
Non-goals: a new general solver, AI usefulness claim, universal novelty, speedup,
rerun of EXP001, execution of the draft EXP002 or modification of old evidence.

---

## ER-001 entity reconciliation — 2026-09-27

Mode C. User selected application APP-01 after the Hypodive triage. Build a small
real-data reconciliation capability and test its formulation, independently from
the future semantic scorer. Strongest intended claim: verified complete-graph
partitioning on twelve label-blind six-record WDC development blocks; whether
consistency improves annotation quality is an open measured gate, not an assumption.

Canonical implementation: `research/experiments/entity_resolution/core.py`;
data intake `prepare.py`, instrument/analysis `pilot.py`, unchanged native MSC
`research/examples/hubo_compare.rs` through the Q002 deadline supervisor.
Dependencies are Python standard library and the existing Rust build only.
Inputs contain all within-block scores; cross-block pairs are unresolved.

Invariants: no label enters blocking/scoring/search; exact set partitions agree
with the cubic penalty optimum; both submitted forms equal 8x the application
energy; every native incumbent is verified; deadlines, source/binary identity,
case/seed design and complete raw inventory are checked. Preserve failed runs.

Existing evidence: five application-triage witnesses; synthetic LAYA-001 cannot
establish real-data value because it was fully presolved. WDC training-large
offers groups of 3..11 records; twelve blocks selected without outcome access.
The [binding pilot protocol](experiments/entity_resolution/protocol.md) fixes
H1 informative conflicts, H2 development F1 signal, and H3 native fidelity.
Fourteen synthetic tests and the unchanged Rust example tests pass before freeze.

Missing pieces deliberately excluded: working Laya environment, fine-tuning,
probability calibration, source-disjoint real holdout, production deployment,
scaling/latency claims, presolve benefit. Public UltimateSolver remains unchanged.
No migration or irreversible action. Stop after preserved results and independent
handoff; failed qualification cannot be rescued by retuning on these blocks.
