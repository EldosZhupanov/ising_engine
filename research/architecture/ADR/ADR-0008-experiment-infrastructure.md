---
id: ADR-0008
title: Experiment infrastructure — one command, full report, black-box engines
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [Experiment-Runner, implements, CONSTITUTION#7]
  - [Experiment-Runner, implements, CONSTITUTION#12]
  - [Experiment-Runner, realized_by, engine_v2]
  - [Experiment-Runner, requires, Single-Scorer]
  - [Experiment-Runner, requires, Determinism-Check]
  - [Determinism-Check, implements, ADR-0004]
  - [Single-Scorer, implements, ADR-0005]
  - [Experiment-Runner, validated_by, EXP-0000]
  - [EXP-0001, refines, BLUEPRINT#Roadmap]
---

# ADR-0008 — Experiment infrastructure

## Context

Blueprint Step 2: "any new operator can be tested in a single evening." Without
this, every operator in the coming library (Blueprint Step 5) costs weeks of
manual runs, ad-hoc plots, and un-replayable comparisons — the MQLib failure
mode at the level of our own process. The Research Track (ADR-0007) cannot
enforce its pre-registered pipeline (§13) if running an experiment is bespoke
each time.

## Decision

A single command produces a complete, claim-anchored report:

```
cargo run --release --bin experiment -- --exp EXP-XXXX
```

Design commitments:

1. **Engines are black boxes that emit a spin STATE**, not an energy. The
   runner scores every engine with ONE canonical scorer (Single-Scorer,
   ADR-0005) — so cross-engine and cross-language convention mismatches (the
   parser-trap failure class) cannot bias a comparison. `solve_instance` today;
   `solve_v2` (SparseBitSlice) plugs in unchanged as another `kind:"binary"`.
2. **Specs are pre-registered JSON** (`experiments/specs/EXP-*.json`) carrying
   hypothesis, kill criterion, instances, seeds, budgets, engines, baseline —
   fixed before code (§13). EXP-0001 is the v0.1 gate, pre-registered while its
   engine is still unbuilt.
3. **Append-only result database** (`results.jsonl`) with resume: re-running
   skips completed cells. A run is a reproducible experiment, not a one-shot.
4. **Determinism (golden) check built in**: a deterministic binary engine is
   re-run at a fixed seed and its canonical score asserted identical
   (ADR-0004). The pipeline reports FAIL loudly.
5. **Full artifact tree every run**: `json/ tables/ graphs/ html/ report.md
   meta.json`, plus a top-level `index.json` that flags regressions vs the
   previous run of the same experiment.
6. **The Rust `experiment` bin is a thin shim**; the runner lives in Python
   (`experiments/run_experiment.py`) where the statistics/visualization stack
   (numpy/scipy/matplotlib) already lives. No experiment logic in the shim.

## Evidence

- EXP-0000 (this session): production engine vs OpenJij vs neal, 3 G-Set
  instances × 3 seeds, one command, 98.6 s cold / 18 s resumed. Determinism
  check PASS. Full artifact tree emitted; Wilcoxon + sign test computed;
  regression tracking active. The pipeline is proven end-to-end.

## Consequences

- New operator → new spec + `solve_v2` path → evening-scale evaluation with
  stats, plots, and a determinism proof, automatically.
- Cost: two languages in the loop. Accepted — the shim keeps the boundary
  clean and the stats stack is Python-native.
- The runner is engine-agnostic; it does not know what SparseBitSlice is,
  which is exactly why it will still work when the engine is replaced.

## Alternatives rejected

- **Trust each engine's self-reported energy**: reintroduces the convention
  traps ADR-0005 exists to kill; cross-engine numbers become incomparable.
- **All-Rust runner**: would re-implement scipy/matplotlib; the baseline
  solvers (OpenJij/neal) are Python anyway.
- **No pre-registration**: lets success thresholds drift after seeing results
  — the exact anti-pattern §13 forbids.
