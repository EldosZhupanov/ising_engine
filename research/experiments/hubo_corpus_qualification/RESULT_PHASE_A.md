# HUBO-Q002 Phase A — certified local penalties prepared

2026-09-27. Closed Phase A record on commit. **Phase B: NOT RUN.**
No annealing, solver endpoint comparison, optimum search or timing benchmark
was performed. This record does not change HUBO-C001's failed advantage gate.

Protocol `9438f15`, compiler/checker/test source `2017661`, full static
certificates `cd2fc6e`. [Protocol](protocol.md) defines all twelve new inputs,
the local/global reduction, and the future 240-cell native-only variation gate.
[Input manifest](inputs.json) and [preparation script](prepare_inputs.py) reproduce
the fresh n=64/128, degree3/4 corpus without accessing solver outcomes.

## Checked result

Eight unit tests passed. They exhaustively compare minima on eight small
fixtures: 256 original assignments and 1,184 expanded assignments counting both
penalty modes. Tests cover mixed signs, constants, quadratic-only and cancelled
terms, shared/nested auxiliaries, conditional minima at every step, fixed pair
order, arbitrary precision, malformed input and corruption detection.

All twelve new inputs compiled successfully in both modes. The 24 complete
certificates retain original-to-product variable maps, every affected monomial,
penalty and final quadratic polynomial in [phase_a.json](phase_a.json).
[SHA manifest](phase_a_sha256.json) binds this record's raw JSON/stdout/stderr.
The static checker replayed every step, checked local<=global bounds, identical
pair plans, canonical terms, safe integer magnitude, and four predetermined
consistent extensions per certificate. These large-instance checks are symbolic
certificates plus sampled extensions, not exhaustive 2^n enumeration.

| Stratum | Expanded variable range | Global M range | Local per-product M range |
|---|---:|---:|---:|
| n64, degree3 | 199–208 | 1,537 | 9–25 |
| n64, degree4 | 330–339 | 9,137–9,217 | 9–97 |
| n128, degree3 | 446–456 | 3,073 | 9–25 |
| n128, degree4 | 734–743 | 18,369–18,417 | 17–65 |

These ranges describe coefficient magnitudes, **not speedups**. The two modes
use the same pair plan and auxiliary count. Smaller certified penalties do not
by themselves show that a stochastic search improves or that a schedule is fair.

## Why the certificate is sufficient

For an affected polynomial uv*g, let P be the sum of positive coefficients and
N the sum of absolute negative coefficients. On Boolean inputs −N<=g<=P.
Using M=1+max(P,N) in M*(uv−2uz−2vz+3z) makes every incorrect z cost at least one
more than z=uv for every fixed assignment of all old variables. Thus minimizing
over the fresh z exactly restores the old polynomial. Sequential composition
proves preservation for shared and nested products; previous quadratic penalties
are never substituted. [Protocol](protocol.md) gives the complete two-case proof.

This is an explicit sufficient bound for an established substitution technique,
not a novelty claim or proof that these are the tightest possible penalties.
The implementation uses exact Python integers and does not convert to floating
point. Integration with a floating-point solver remains a separate checked step.

## Defects found before source freeze

Independent review found and prompted three repairs: one-shot variable iterators
could be consumed twice and miscompiled; the draft static recorder omitted full
certificates; the checker could overwrite duplicate emitted monomials when
constructing a dictionary. Final code rejects unsupported variable containers,
retains full certificates and rejects duplicate/zero/unsorted/noninteger emitted
terms. Regression tests include the reviewer's exact false-certificate example.
Final pre-execution independent implementation review passed with 8/8 tests.

[Post-execution independent audit](independent_audit/audit_phase_a.json): **PASS**.
Without importing the compiler or solver, the auditor verified all 24 models,
8,076 substitution steps including the pair-selection rule, 288 consistent
extensions and all twelve exact spin-to-binary expansions. It independently
reproduced the 256/1,184 fixture counts, verified source/generator/raw hashes,
and rejected three artificially corrupted certificates. Raw records stayed
byte-identical. The [auditor](independent_audit/audit_phase_a.py) is preserved.
Production `src/`, solver families, public APIs, Cargo dependencies and every
frozen C001 artifact remain unchanged. No full production Cargo suite was run
for these isolated standard-library Python files.

## Reproduction and next action

```bash
python3 -m unittest discover -s research/experiments/hubo_corpus_qualification -p 'test_*.py'
```

`verify_phase_a.py` refuses to overwrite its record and requires a committed
source. To reproduce its construction, use a separate checkout of `2017661`,
then run that script there; retain the new record separately. The independent
auditor checks saved certificates without invoking the compiler and also refuses
to overwrite its audit record. To repeat that audit, copy the published auditor
into the same relative directory in the separate reproduction checkout (where
its output does not yet exist), then run it there. Neither command is intended
to overwrite evidence in this checkout.

Next: implement and freeze Phase B's new controller/analysis, with portable raw
verification, explicit execution order/timestamps, symmetric variation criteria,
and disjoint smoke. Only then run the registered 240 native-arm cells once.
The qualification can return variation despite identical performance distributions;
it is not a superiority test. A future native/local/global penalty and schedule
comparison needs its own protocol/holdout. No extra campaign is silently added.
