# HUBO-Q002 Phase B — mixed endpoint-variation qualification

2026-09-27. Closed on publication commit. **MIXED: 2/4 strata pass.**
The preregistered overall success threshold was at least three passing strata;
it was not met. This is an operational measurement result, not a superiority,
intrinsic-hardness, optimum, novelty or world-record claim.

Binding [protocol](protocol.md) `9438f15`; independently reviewed instrument and
analysis `5e9ea62`; complete raw campaign `d5a2a63`. [Phase A](RESULT_PHASE_A.md)
remains an unchanged historical record: its statement that Phase B had not run
was true when that record closed. This prospective record reports Phase B.

## Complete result

All 240 registered cells completed and passed instrument checks, with 4,079
original-space incumbents and 240 retained final witnesses. No cell was replaced,
excluded, rerun or granted extra budget. All twelve instances remain in the
[raw summary](phase_b_run/summary.json), including constant endpoints and losses.
The disjoint explicit n=6 smoke passed four cells and eleven incumbents before
any main cell. No main generator was invoked at the smoke size.

An instance qualifies only if at least one arm's ten-seed spread is >=2 energy
units AND at least two paired endpoints differ. A stratum needs >=2/3 qualifying
instances. Equal distributions with permuted seeds can pass; stable different
endpoints can fail. Neither case is a superiority test.

| Stratum | Qualifying instances | Stratum passes | MSC lower / equal / higher energy than OpenJij |
|---|---:|---|---:|
| n64, degree3 | 0/3 | No | 0 / 30 / 0 |
| n64, degree4 | 1/3 | No | 5 / 25 / 0 |
| n128, degree3 | 2/3 | Yes | 1 / 24 / 5 |
| n128, degree4 | 3/3 | Yes | 21 / 3 / 6 |

Six of twelve instances qualify. Six have constant, identical native endpoints;
there are no stable-separation cases. Descriptive totals are 27 lower, 82 equal,
11 higher MSC energies across 120 pairs. Do not interpret these totals as an
independent significance test or select the favorable n128/degree4 group as a
post-hoc confirmatory result. Both arms improved on all-zero energy in every cell.
All endpoint vectors, sample standard deviations, inclusive quartiles, normalized
paired differences and distinct counts are retained per instance in the summary.
There are no certified targets, so optimum hit rates and TTS99 are unavailable.

## Measurement and integrity

Two unchanged native arms: the MSC research driver and OpenJij0.12.0. The public
UltimateSolver remains QUBO-only; this experiment used the direct HUBO kernel.
Worker/driver bytes match C001 source `23d300f`; production `src/`, Cargo files,
solver interfaces, Phase A source and C001 evidence were not changed.

Every cell received a two-second external warm-runtime budget covering input
serialization/transfer/parse, initialization, search, scoring and IPC. Shared
Python imports before READY were timed separately. AB/BA alternation was balanced
five/five within each instance; one worker ran at a time with thread controls=1.
Recorded host: AMD Ryzen7 170, WSL2, rustc1.95.0; package versions and exact binary,
source and package-file hashes match the pinned C001 reference. No affinity was
pinned. These are exploratory warm-time measurements, not cold-latency claims.

Main cell deadlines were reached between 2.00002 and 2.00227 seconds. Process
termination finished by 2.02420 seconds, within the 2.2-second limit. Recorded
process groups had no live survivors. No complete events were received late in
this run; the instrument nevertheless validates late events without crediting
those after two seconds. Raw stdout, stderr, receipt times, sequence identifiers,
monotonic/UTC clocks, return codes and partial tails are retained. Overall main
sequence elapsed time including startup and inter-cell checks was 646.543 seconds;
credited search budgets total 480 seconds. Wall time is not a new speed claim.

Thirteen deterministic/fake-process instrument tests passed, including blocked
large stdin, descendant cleanup, SIGKILL, partial/malformed output, late witnesses,
source/environment mutations, failure preservation and classification boundaries.
The eight unchanged Phase A tests also passed. Independent pre-freeze review found
one portable-environment verification gap; it was corrected and covered by seven
resealed mutation subcases before smoke or main outcomes. Source freeze followed
review PASS. Archived settings verification is separate from optional matching of
currently installed runtime files.

[Independent raw audit](independent_audit/audit_phase_b.json): **PASS**. A separate
standard-library [auditor](independent_audit/audit_phase_b.py), with no project
implementation imports, verified all 4,090 main/smoke incumbents and 244 final
witnesses, all recorded clocks/order/metadata, 202 frozen source files, 500 archive
files and every summary field. It confirms MIXED and the table above. Three
[corruption checks](independent_audit/audit_mutations.json) rejected a wrong late
energy, damaged raw hash and altered summary. Because there were no real late
incumbents, that mutation used an explicitly marked synthetic late event.
Original raw files were unchanged. The audit confirms recorded process evidence;
it does not claim retrospective observation of live processes. A separate
`--runtime` replay matched the current installed binaries/packages and reproduced
the stored summary exactly.

## Reproduce the checks without rerunning solvers

From a repository containing the frozen Git history and committed raw files:

```bash
python3 research/experiments/hubo_corpus_qualification/phase_b.py analyze research/experiments/hubo_corpus_qualification/phase_b_run
python3 research/experiments/hubo_corpus_qualification/phase_b.py analyze research/experiments/hubo_corpus_qualification/phase_b_run --runtime
python3 research/experiments/hubo_corpus_qualification/independent_audit/audit_phase_b.py --output /tmp/q002-independent-audit-new.json
python3 -m unittest discover -s research/experiments/hubo_corpus_qualification -p 'test_phase_b.py' -v
```

The first and third commands do not need the original installed solver/venv or
absolute runtime paths. They do need frozen Git objects for provenance. Only
`--runtime` additionally requires recorded runtime paths and current file hashes.
The independent auditor refuses to overwrite existing output. Recomputing raw
metrics is distinct from repeating the stochastic experiment on another machine.

## Disposition and next question

The overall >=3/4 qualification hypothesis is not supported by this run. The two
n128 strata exhibit the registered variation, while n64 mostly reproduces C001's
constant-endpoint limitation. Q002 has not established statistical power or an
algorithmic advantage, and its opened inputs are not a future holdout.

Next: prospectively design a new independent holdout and a within-kernel
local/global-penalty comparison crossed with representation-derived versus common
temperature endpoints, retaining native controls and OpenJij. Any narrower target
population must be declared before generating new inputs, and its conclusions
must stay within that population. Do not filter or reinterpret Q002 to make it
pass. Local penalties from Phase A have correctness evidence only; their effect
on search is still unmeasured. No additional campaign, Laya training, production
routing or record hunt follows automatically from this result.
