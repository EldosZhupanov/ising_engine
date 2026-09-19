# CD001 HYPODIVE falsification triage

Date 2026-09-17. Mode A: cheapest decisive checks before a new solver.
The user explicitly authorized this investigation after an earlier audit stop.

Strongest tested claim H11: low interface rank alone bounds the number of unique
conditional ground-state responses polynomially. Exact symbolic counterexample:
E(x,z)=(sum 2^i(x_i-z_i))^2, rectangular interface -2aa^T, unique response x=z.
There are 2^b answers, yet the response program simply copies b bits.

Falsifier: direct proof plus three-way objective check and exhaustive minimization
of every context for b1..6. [Result](breakthrough/h11/RESULTS.md). No holdout,
random worlds, seed independence or significance claims; the family was expressly
constructed to attack the assumption. Baselines are exhaustive minimization and
the strongest possible simple rule for this family, identity. There is no timing
race against a weak generic solver. All responses are returned, no abstention.

Decision **NO-GO** for H11. **NOT NOVEL** for finite-state exponential energy
reweighting, exactly Hedge. **NARROW/OVERLAPPING** for parameter-cell memory:
known lower-envelope geometry, analogous to explicit control but different from
continuous convex QP. No candidate survives as a scientifically new method.
This is a successful cheap rejection, not evidence all response compression fails.

Top risks: (1) conflating interface rank with zero-diagonal Ising matrix rank;
(2) hiding coefficient precision/normalized margin; (3) conflating response count
with program complexity. Explicitly addressed in the derivation/result.
Novelty search: [source matrix](PRIOR_ART_MATRIX.md), inspected primary equations
and source sections. The exact witness's historical priority remains UNKNOWN;
its known squared-penalty ingredients do not establish a new algorithm.

Stop condition met by counterexample. No automatic rescue under new assumptions.
Independent finished-code review passed on retry; Git freeze completed in this
CD001 commit. A materially different, precisely scoped hypothesis requires its own
selection and design. Broad research can continue under the user instruction; this
particular claim is terminated.
