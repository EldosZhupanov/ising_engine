# External comparison amendment 1 — MQLib adapter qualification

Date: 2026-09-27. Status: binding prospective procedure.
Parent: [external comparison protocol](EXTERNAL_COMPARISON_PROTOCOL.md).
No comparative MQLib outcomes have been collected for this amendment. Source
inspection and compilation are preparation, not solver-quality evidence.

## Scope and authority

Replace only the parent's section 7 handoff path with
[`memory/NOW.md`](../memory/NOW.md). All comparison-wave prerequisites, S3,
X1–X6, holdout protections, and independent pre/post review remain in force.

Permit a separate engineering qualification, MQ-QUAL-001, before those waves:
synthetic inputs of 1–8 variables, exhaustive objective checks and short process
smokes. It is NOT a comparison wave, selector training, complementarity screen,
performance experiment or evidence of competitive advantage. No Gset, BiqMac,
QOBLIB, reserved research seeds or held-out data are opened.

## Frozen qualification design

- Upstream: https://github.com/MQLib/MQLib, MIT, commit
  `585496274af5abb0849d0d47e135496b4688680b`. Build in ignored local cache,
  unmodified source; record compiler, flags, source and executable hashes.
- Isolated Python adapter outside `src/`; no Rust API/kernel changes, no reuse
  of the legacy external serializer (which omits offsets).
- Input is an explicit binary polynomial to MINIMIZE:
  `E(x)=c+sum(h_i*x_i)+sum(i<j,J_ij*x_i*x_j)`. One pair per unordered edge;
  strict dimensions, finite coefficients and binary state. Duplicate/reversed
  pairs, unknown fields and invalid values are rejected, never repaired.
- MQLib maximizes `F(x)=sum(Q_ii*x_i)+2*sum(i<j,Q_ij*x_i*x_j)`.
  Export one-based diagonal `-h_i`, off-diagonal `-J_ij/2`; retain c in
  provenance. Verify `E(x)=c-F(x)` for every enumerated state and exact
  coefficient round trip. No generic CSR converter is qualified by this work.
- Fixtures: zero singleton with offset 7; linear-only 3-variable instance
  with `[2,-3,0]`, offset -2; complete unit triangle Max-Cut encoded with
  `h=[-2,-2,-2]`, each J=2, c=0; a 4-variable mixed case with
  `h=[0.5,-1.25,0,0]`, pairs `(0,1,2.5),(1,2,-0.75)`, c=3.25;
  plus n=1..8 deterministic dense fixtures with
  `h_i=(((7*i+3*n)%17)-8)/4`,
  `J_ij=(((11*i+5*j+n)%19)-9)/4`, `c=(n-4)/4`.
- Enumerate all states in Python exact rational arithmetic and independently
  through MQLib's actual QUBO loader and recomputation API, not a second copy
  of the exporter formula. Preserve per-state output.
- External smokes: only native QUBO heuristic `MERZ2002ONEOPT`; seeds
  101,102,103; requested runtime 0.02 seconds per fixture, external hard
  process timeout 5 seconds. 12 fixtures × 3 seeds = 36 calls. No retries
  selected for nicer quality. Explicit seed passed; wall-budget nondeterminism
  is disclosed. Requested runtime is not claimed to be a hard end-to-end cap.
- Store every raw response, command, input, binary/source hashes, state, exact
  recomputed energy, upstream reported objective and timing. Time measurements
  describe execution only. Nonzero exit, missing/malformed state, nonfinite
  metric, objective mismatch or timeout is a failure, never a valid candidate.
- Tests must reject missing/extra/nonbinary states, bad metrics, altered energy,
  invalid model/seed/budget, and process errors; explicitly test that sign,
  pair-factor and offset corruptions are detected. Preserve failing attempts.

## Acceptance and limits

PASS requires all exhaustive identities, strict-parser tests and 36 independently
verified candidate objectives. Candidate optimality is NOT required: record the
exhaustive optimum and gap separately without claiming a solver benchmark.
Failure is published; corrections require source provenance and a fresh output
directory, never overwriting evidence. No optimum-hunting retry.

Commit this amendment before qualification execution; commit reviewed adapter
and fixture/checker sources before retained external runs. An independent
reviewer checks the plan before execution and finished evidence afterwards.
Retain metadata.json and reproduction README. Update NOW and catalogue.

Next research decision, not authorized by qualification alone: define a
prospective matched-budget complementarity study and reconcile it with the
parent's still-binding comparison prerequisites. No automatic campaign follows.
