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

**Independently review the integrated durable-memory state, then resume RC-021
at Step 7 only after PASS.**

No RC-021 control, qualification session, G11 sentinel, or experiment may run as
part of this task.

## Working context

- Active branch: `feat/solver-research-upgrades`.
- RC-021 Step 6 is complete, committed as
  `1b855b774654f85a326baf37436262e9f5322e5b`, and an independent read-only
  review of it returned PASS.
- The durable-memory series is integrated on this branch as commits
  `983497d..d5880e3`.
- The working tree is clean and `experiments/rc021` does not exist.
- RC-021 has never been executed: no controls, no qualification session, no
  sentinel on G11, and no measurement data of any kind.
- `docs/memory-architecture` is a historical backup reference only. It is not
  the active branch and holds no authority.
- Binding RC documents and their paths are immutable.
- `memory/CURRENT_TASK.md` and `memory/CURRENT_HANDOFF.md` are compatibility
  pointers. Their former snapshots remain available through Git history.

## Product position

- The production solver exists and remains the comparison arbiter.
- The `engine_v2` selector, capability registry, plan synthesis, Runtime,
  knowledge system, and research orchestration exist.
- Production auto-routing is not yet licensed: the domain-specific selector
  still needs replicated equal-cost evidence and a production A/B win against
  `UltimateSolver`.

## This task is complete only when

1. This file and `memory/TIMELINE.md` describe the integrated branch state
   rather than the superseded memory-branch state.
2. `scripts/check_memory_docs.sh` still passes against the Step 6 commit.
3. The document catalogue, its counters, and the frozen-evidence manifest are
   unchanged in substance.
4. An independent read-only reviewer returns PASS on the integrated state.

## Next action

Request an independent read-only review of the combined state — reviewed Step 6
plus the integrated memory series — starting from this live-state commit. Only
after that PASS does work move to **RC-021 Step 7: decision, finalize and
verify**.

## Forbidden during this task

- Running RC-021, its controls, a qualification session, or a sentinel on G11.
- Creating `experiments/rc021/`.
- Editing or moving any binding preregistration, amendment, or cycle record.
- Starting Step 7 before the integrated-memory review returns PASS.
- Fixing the non-blocking F1/F2 test-hardening findings in this documentation
  commit.
- Mixing source-code fixes into a memory commit.
- Treating Obsidian, generated indexes, summaries, chat logs, or model memory as
  a source that outranks Git-tracked canonical documents.
