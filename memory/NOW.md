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

**RC-021 Step 8 — CLI and end-to-end synthetic tests.**

This is the single active task. The five public modes and synthetic or injected
end-to-end tests are permitted. Running RC-021 is not: no production control,
no qualification session, and no sentinel on G11 may execute, and
`experiments/rc021/` must not be created.

## Working context

- Active branch: `feat/solver-research-upgrades`.
- RC-021 Step 6 is complete, committed as
  `1b855b774654f85a326baf37436262e9f5322e5b`, and an independent read-only
  review of it returned PASS.
- RC-021 Step 7 is complete and committed as
  `3493ed8`; its independent review blockers and adversarial follow-up findings
  are closed, and the full source gate set is green.
- The durable-memory series is integrated on this branch as commits
  `983497d..d5880e3`.
- The integrated memory state at `e824b7f` was independently reviewed and
  returned PASS.
- The working tree contains an unrelated draft change to
  `research/EXTERNAL_PROJECTS_BACKLOG.md`; preserve it and exclude it from every
  RC-021 commit.
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

1. The five public modes and their exact argument/precondition state machine are
   wired through the existing private modules.
2. Every mode exit code is derived from the frozen mode/run-status taxonomy.
3. Synthetic end-to-end tests cover the legal transitions, refusals, terminal
   lock, finalization and read-only verification.
4. No test executes a real control, qualification session, or G11 sentinel.
5. The applicable `cargo` check, test, build, `clippy`, and `fmt` gates pass.
6. An independent read-only review of the complete instrument returns PASS.
7. No `experiments/rc021/` and no RC-021 measurement data exists.

## Next action

Explorer, strictly read-only: map the existing hidden P2 dispatch and the five
mode APIs to Amendment 2's state-machine table and exit codes. Then Planner must
present the single Step-8 source commit and synthetic integration-test plan
before any edit is made.

## Forbidden during this task

- Running RC-021, any real control, any qualification session, or a sentinel on
  G11.
- Creating `experiments/rc021/`.
- Editing or moving any binding preregistration, amendment, or cycle record.
- Fixing the non-blocking F1/F2 test-hardening findings "while here".
- Starting the real six-session execution or writing the instrument-review
  record before the complete-instrument review returns PASS.
- Modifying or staging `research/EXTERNAL_PROJECTS_BACKLOG.md` as part of
  RC-021.
- Changing `solver`, `core`, `engine_v2`, or any Cargo dependency.
- Weakening the durability contract, the terminal evidence lock, or the
  read-only guarantee of `--verify`.
- Treating Obsidian, generated indexes, summaries, chat logs, or model memory as
  a source that outranks Git-tracked canonical documents.
