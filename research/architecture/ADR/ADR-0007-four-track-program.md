---
id: ADR-0007
title: "Program structure: four parallel tracks (Engine, Research, Benchmark, Verification)"
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [Engine-Track, implements, BLUEPRINT#Roadmap]
  - [Research-Track, implements, CONSTITUTION#13]
  - [Benchmark-Track, implements, CONSTITUTION#7]
  - [Verification-Track, implements, ADR-0005]
  - [Engine-Track, requires, Verification-Track]
  - [Engine-Track, requires, Benchmark-Track]
  - [Research-Track, requires, Benchmark-Track]
---

# ADR-0007 — The four-track program

## Context

The project is leaving the *idea exploration* stage (blueprints, validation
program, constitution) and entering *platform construction*. Serialized work
("finish the engine, then benchmark it, then verify it") is how the coupling
between architecture, evidence, and correctness — the project's actual
strength — decays: benchmarks go stale while code changes, verification is
bolted on after design freezes, research queues behind engineering.

## Decision

Work proceeds on **four parallel tracks**, each with a standing charter and
explicit interfaces to the others. No track may be paused "until another
finishes."

**1. Engine Track** — implements the architecture: SparseBitSlice substrate,
`Operator`/`SpinState` contracts, lowering passes, scheduler, adaptive
control — in the Blueprint's gated order (v0.1 → v1.0). Consumes: verification
gates, benchmark verdicts, IMPLEMENT-verdict research outputs. Produces:
capability.

**2. Research Track** — runs new physics operators and methods through the
constitutional pipeline (§13: idea → … → verdict), pre-registered, isolated
from production until IMPLEMENT. Consumes: benchmark protocol, engine
substrate for prototypes. Produces: operators, kill-reports, knowledge.

**3. Benchmark Track** — maintains the ongoing comparison against OpenJij,
MQLib heuristics, dwave-neal, and published hardware-annealer results on the
fixed 499-instance base: calibrated equal-wall-clock, fixed seeds, exactly-once
accounting, statistical certificates. Re-runs on every engine release.
Consumes: release binaries. Produces: the evidence base every other track's
claims anchor to — and the gates (e.g. v0.1's "beat OpenJij on ≥½ of G-Set
n≤2000").

**4. Verification Track** — owns proof of correctness: golden regressions,
identical-seed A/B harness, pass certificates and their verifiers, canonical
end-to-end re-scoring, external ground-truth validation for every new format
or transformation. Consumes: everything. Produces: the right of anything to
merge.

Interface rules:

- Engine work merges only through Verification gates; performance and quality
  claims exist only as Benchmark artifacts (Constitution §4.6–4.7).
- Research reaches production only through its verdict pipeline; its
  experiments run under Benchmark protocol so results are comparable.
- A track's backlog is visible to all tracks; cross-track blockers outrank
  in-track features.

## Consequences

- Every engine milestone lands with its evidence and its proofs *already
  current* — no "we'll re-benchmark later" debt.
- Cost: context-switching across tracks in a small team / agent setting.
  Mitigation: tracks are *charters*, not headcount — one session advances one
  track at a time, but no track is ever declared dormant.

## Alternatives rejected

- **Serialized phases**: produces stale benchmarks and post-hoc verification —
  the exact failure modes the project's history already caught (parser bugs
  found only when validation ran; geometry flaw found only when the full
  campaign ran).
- **Engine-only focus until v1.0**: the Blueprint's gates are benchmark
  verdicts; without the Benchmark and Verification tracks running, the gates
  cannot even be evaluated.
