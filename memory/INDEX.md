# memory/INDEX.md — persistent project memory

**This directory is a pointer layer, not a content store.** Each file names its
authoritative source and carries only what that source does not. Nothing here
duplicates `ROADMAP.md` or the ADRs, so nothing here can drift out of sync with
them.

Read in this order at the start of a session:

| File | Purpose | Authoritative source |
|---|---|---|
| [INDEX.md](INDEX.md) | this map | — |
| [CURRENT_TASK.md](CURRENT_TASK.md) | what is in flight right now | *authored here* |
| [ARCHITECTURE.md](ARCHITECTURE.md) | system shape and boundaries | `../ARCHITECTURE.md`, `../CONTEXT.md`, `../research/architecture/ADR/` |
| [DECISIONS.md](DECISIONS.md) | why the architecture is shaped this way | `../research/architecture/ADR/` |
| [ROADMAP.md](ROADMAP.md) | status and priorities | `../ROADMAP.md` |
| [OPEN_PROBLEMS.md](OPEN_PROBLEMS.md) | unresolved questions and debt | `../research/RESEARCH_GAPS.md`, `../ROADMAP.md` §Technical debt |
| [PERFORMANCE.md](PERFORMANCE.md) | perf history and measurement rules | `../PERF.md` |
| [BENCHMARKS.md](BENCHMARKS.md) | competitive standing | `../experiments/results/index.json` |
| [RESEARCH.md](RESEARCH.md) | research cycles, results, retractions | **`../research/RESEARCH_INVENTORY.md`** (master register), `../research/RC0*.md` |

## Governance order (unchanged — from `../CLAUDE.md`)

Higher outranks lower. This directory does **not** insert itself into that chain;
it indexes it.

1. `../research/ISING_ENGINE_CONSTITUTION.md` — direction (single source of truth)
2. `../SOUL.md` — why the project exists
3. `../research/architecture/ADR/` — architecture decisions
4. `../ROADMAP.md` — status
5. `../CLAUDE.md` — day-to-day rules

## Superseded documents

`../MEMORY.md` and `../CONTEXT.md` are **retained for history only**. They
predate the current direction and describe a SIMD-first plan that the code does
not implement. Use `../ROADMAP.md` for direction and
[CURRENT_TASK.md](CURRENT_TASK.md) for status.

## A second, separate store

`~/.claude/projects/-home-eldos-ising-engine/memory/` holds session-scoped
assistant memory (16 topic files + its own `MEMORY.md`). It is auto-loaded per
session and is **not** part of the repository. Findings that matter to the
project must be written *here*, in the repo, to survive.
