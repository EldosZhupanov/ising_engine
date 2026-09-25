# CD003 applicability triage — 2026-09-25

Decision: **NARROW**. The conditional-response bound in
[`MATH.md`](MATH.md) is valid when a
cross-coupling admits the stated bounded-integer factorization and ties are
broken consistently. It bounds the number of *different boundary fields and
responses*. It does not bound the cost of computing one response or the cost
of optimizing the remaining boundary variables. The broad algorithmic reading
in [`RESULTS.md`](RESULTS.md) §2 and
[`PRIOR_ART.md`](PRIOR_ART.md) §3 is not
established. These historical files are preserved unchanged.

## Cheapest decisive counterexample

For an arbitrary graph G, let internal variables y encode independent set with
`E_G(y) = -sum(y_v) + 2 sum_(u,v in E(G)) y_u y_v`; its minimum is
`-alpha(G)`. Add one internal dummy bit d and b boundary bits z with the
rank-one, unit-precision interaction `d * sum(z_j)`. There are at most b+1
boundary fields, and a deterministic minimizer chooses d=0 for all z, so the
response count is one. Computing the response still solves maximum independent
set on G. Boundary-only terms can independently make the remaining problem
hard. Thus a polynomial response-table *size* is not a polynomial-time
elimination algorithm.

The asserted “if and only if” precision condition is also too strong as
written: large coefficient precision can coexist with one response, and a
rank r that grows with b makes `(2bK+1)^r` superpolynomial even when K is
constant. The sufficient bound remains useful for fixed/small r and K.

## Practical claim and simplest baseline

The next falsifiable question is narrower: after the existing exact presolve,
do official Market Split QUBOs retain nonempty blocks whose boundary
assignments collapse to substantially fewer distinct fields? The simple
baseline is direct enumeration of all `2^b` boundary assignments. A field
cache is safe because equal cross-fields induce identical conditional
internal objectives; this diagnostic alone cannot establish a speedup or a
new solver capability.

The selected corpus, split, threshold, timeout, and integrity rules are frozen
separately in
[`protocol.md`](../../experiments/cd003_market_residual/protocol.md)
before any probe run. The corpus is a benchmark, not real annotated Laya
data. The LAYA-001 pilot fixed every variable and cannot test this question.

## Risks, novelty, and stop condition

Top risks are presolve saturation, distinct fields despite low algebraic rank,
and a costly internal optimization for each retained field. The conditional
field identity is elementary; no novelty claim is made. Stop this Market Split
field-cache line if the frozen structural gate fails. A passed gate permits a
separate, preregistered equal-cost solver comparison; it is not itself GO for
production.
