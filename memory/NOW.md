---
id: memory-now
kind: live-state
status: active
authority_scope: current-task
updated: 2026-08-26
immutable: false
---

# NOW — the only active project state

This file is the sole repository authority for what is in flight. Always verify
its claims against `git status --short` and the current branch before acting.

## Active task

**Build and independently review the durable project-memory architecture.**

RC-021 implementation and execution are paused by explicit user direction while
the memory layer is repaired. No RC-021 control, qualification session, G11
sentinel, or experiment may run as part of this task.

## Working context

- Memory work branch: `docs/memory-architecture`, based on `cad5b87`.
- The primary worktree contains uncommitted Step 6 review corrections. This
  memory branch must not modify, stage, reset, or discard them.
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

1. Every Markdown document is catalogued by purpose, authority, lifecycle, and
   chronology without editing binding evidence.
2. `START_HERE.md` and this file are the deterministic startup path.
3. Competing live-state files no longer claim authority.
4. `PROJECT_PLAN.md`, `ROADMAP.md`, and the research register no longer contradict
   the current repository state.
5. agent instruction files fit inside their loading budget and contain no stale
   dynamic counters.
6. automated checks reject broken links, duplicate authorities, uncatalogued RC
   files, stale active markers, and changes to frozen evidence.
7. an independent read-only reviewer returns PASS.

## Next action

The six-commit memory series is built and its memory-integrity gate passes.
Request an independent read-only review. Only after PASS and an explicit
integration decision may work return to the paused RC-021 Step 6 review/amend
sequence.

## Forbidden during this task

- Running RC-021 or creating `experiments/rc021/`.
- Editing or moving any binding preregistration, amendment, or cycle record.
- Mixing source-code fixes into a memory commit.
- Treating Obsidian, generated indexes, summaries, chat logs, or model memory as
  a source that outranks Git-tracked canonical documents.
