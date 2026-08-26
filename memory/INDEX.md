# Persistent project memory

This directory is a governed pointer layer over canonical Git-tracked sources.
It does not override the sources it names.

## Start here

1. [`../START_HERE.md`](../START_HERE.md)
2. [`NOW.md`](NOW.md)
3. [`AUTHORITY.md`](AUTHORITY.md)
4. only the task bundle named by `NOW.md`

## Memory indexes

| File | Purpose |
|---|---|
| [`NOW.md`](NOW.md) | sole live state and next action |
| [`AUTHORITY.md`](AUTHORITY.md) | authority by question and conflict protocol |
| [`CATALOG.md`](CATALOG.md) | complete document registry and lifecycle |
| [`TIMELINE.md`](TIMELINE.md) | milestone and research-cycle chronology |
| [`OBSIDIAN.md`](OBSIDIAN.md) | optional local graph and backlink view |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | pointer to architecture sources |
| [`DECISIONS.md`](DECISIONS.md) | pointer to accepted/proposed ADRs |
| [`ROADMAP.md`](ROADMAP.md) | pointer to the implementation inventory |
| [`OPEN_PROBLEMS.md`](OPEN_PROBLEMS.md) | unresolved questions and debt |
| [`PERFORMANCE.md`](PERFORMANCE.md) | measurement rules and performance evidence |
| [`BENCHMARKS.md`](BENCHMARKS.md) | competitive evidence |
| [`RESEARCH.md`](RESEARCH.md) | research findings and retractions |

## Compatibility files

`CURRENT_TASK.md` and `CURRENT_HANDOFF.md` retain old links but redirect to
`NOW.md`. Root `MEMORY.md` and `CONTEXT.md` remain historical snapshots.

## Update discipline

- Current work changes only in `NOW.md`.
- Added/moved Markdown changes `CATALOG.md` in the same commit.
- Accepted decisions use ADRs.
- Research chronology changes in `TIMELINE.md` only after a durable cycle-state
  transition.
- Binding evidence is never edited or moved; its metadata lives in the catalogue.
