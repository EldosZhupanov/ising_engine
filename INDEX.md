# INDEX.md

## Start here

**`memory/INDEX.md`** — the persistent project memory. A pointer layer over
everything below; read it first in a new session.

## Governance order (`CLAUDE.md` §Governance — higher outranks lower)

1. `research/ISING_ENGINE_CONSTITUTION.md` — direction (single source of truth)
2. `SOUL.md` — why the project exists
3. `research/architecture/ADR/` — architecture decisions
4. `ROADMAP.md` — status: what exists, what is in flight, what is next
5. `CLAUDE.md` — day-to-day working rules

## Documentation Index

CLAUDE.md
Fast session context and engineering rules.

AGENTS.md
Engineering rules.

ARCHITECTURE.md
Architecture overview.

ROADMAP.md
Current implementation roadmap and priorities.

PERF.md
Performance history. Note the ~9% host timing drift caveat.

VERIFY.md
Verification checklist.

## Persistent memory (`memory/`)

Thin pointer files — each names its authoritative source and adds only what that
source lacks, so nothing here can drift.

| File | Purpose |
|---|---|
| `memory/INDEX.md` | map of the memory layer |
| `memory/CURRENT_TASK.md` | what is in flight right now |
| `memory/ARCHITECTURE.md` | the hard boundary and the three backends |
| `memory/DECISIONS.md` | the nine ADRs, one line each |
| `memory/ROADMAP.md` | status pointer + the "Next 10" |
| `memory/OPEN_PROBLEMS.md` | debt + **benchmark degeneracy findings** |
| `memory/PERFORMANCE.md` | measurement rules that reject work by default |
| `memory/BENCHMARKS.md` | honest competitive standing |
| `memory/RESEARCH.md` | research cycles: what stands, what was retracted |

## Superseded

`MEMORY.md` and `CONTEXT.md` — retained for history only. They predate the
current direction and describe a SIMD-first plan the code does not implement.
Corrections are marked inline in both.
