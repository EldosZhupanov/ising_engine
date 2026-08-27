---
id: memory-now
kind: live-state
status: active
authority_scope: current-task
updated: 2026-08-27
immutable: false
---

# NOW — the only active project state

This file is the sole repository authority for what is in flight. Always verify
its claims against `git status --short` and the current branch before acting.

## Active task

**RC-021 Step 7 — decision, finalize and verify.**

This is the single active task. Implementation and synthetic or injected tests
are permitted. Running RC-021 is not: no production control, no qualification
session, and no sentinel on G11 may execute, and `experiments/rc021/` must not
be created.

## Working context

- Active branch: `feat/solver-research-upgrades`.
- RC-021 Step 6 is complete, committed as
  `1b855b774654f85a326baf37436262e9f5322e5b`, and an independent read-only
  review of it returned PASS.
- The durable-memory series is integrated on this branch as commits
  `983497d..d5880e3`.
- The integrated memory state at `e824b7f` was independently reviewed and
  returned PASS.
- The working tree is clean.
- RC-021 has never been executed: no controls, no qualification session, no
  sentinel on G11, and no measurement data of any kind.
- Binding RC documents and their paths are immutable.
- `docs/memory-architecture` is a historical backup reference only. It is not
  the active branch and holds no authority.

## Product position

- The production solver exists and remains the comparison arbiter.
- The `engine_v2` selector, capability registry, plan synthesis, Runtime,
  knowledge system, and research orchestration exist.
- Production auto-routing is not yet licensed: the domain-specific selector
  still needs replicated equal-cost evidence and a production A/B win against
  `UltimateSolver`.

## This task is complete only when

1. The §7.2 decision predicate and all terminal status mappings are implemented
   from the binding documents.
2. Finalization writes the closure using the reservation-first durability
   contract.
3. The terminal evidence lock is enforced.
4. `--verify` is strictly read-only and validates the full durable record.
5. Synthetic or injected tests cover the decision, closure, damaged or partial
   closure, and verify paths without running RC-021.
6. The applicable `cargo` check, test, build, `clippy`, and `fmt` gates pass.
7. An independent read-only review returns PASS.
8. No `experiments/rc021/` and no RC-021 measurement data exists.

## Next action

Explorer, strictly read-only: build the Step-7 dependency map from the binding
preregistration and its amendments — the §7.2 predicate, the §8 statuses, the
closure schema, the terminal lock, and the verify contract — together with the
existing journal, manifest, protocol, session, and controls layers. Then Planner
must present an atomic implementation plan before any edit is made.

## Forbidden during this task

- Running RC-021, any real control, any qualification session, or a sentinel on
  G11.
- Creating `experiments/rc021/`.
- Editing or moving any binding preregistration, amendment, or cycle record.
- Starting Step 8 or any public CLI surface.
- Fixing the non-blocking F1/F2 test-hardening findings "while here".
- Changing `solver`, `core`, `engine_v2`, or any Cargo dependency.
- Weakening the durability contract, the terminal evidence lock, or the
  read-only guarantee of `--verify`.
- Treating Obsidian, generated indexes, summaries, chat logs, or model memory as
  a source that outranks Git-tracked canonical documents.
