# Repository index

Start at [`START_HERE.md`](START_HERE.md).

| Need | Entry point | Authority |
|---|---|---|
| What is happening now | [NOW.md](NOW.md) → [memory/NOW.md](memory/NOW.md) | `memory/NOW.md` only |
| Projects, files, forgotten work | [PROJECTS.md](PROJECTS.md), `memory/FILE_MAP.tsv` | Map; verify claims against evidence |
| Next sequence and backlog | [ROADMAP.md](ROADMAP.md), [PROJECT_PLAN.md](PROJECT_PLAN.md) | `PROJECT_PLAN.md` governs product gates |
| Research and retractions | [RESEARCH.md](RESEARCH.md) → [memory/RESEARCH.md](memory/RESEARCH.md) | Individual frozen protocols/results |
| Architecture and decisions | [DECISIONS.md](DECISIONS.md) → [memory/DECISIONS.md](memory/DECISIONS.md) | Accepted ADRs |
| Document authority | [memory/AUTHORITY.md](memory/AUTHORITY.md), [memory/CATALOG.md](memory/CATALOG.md) | Scoped authority matrix |

Generated dependencies under `target/`, `website/node_modules/`, and the
Python environment are counted but not treated as project source. Important
ignored experiment and archive files are identified in `PROJECTS.md` and the
file map. Untracked paths are visible in that map until reviewed and committed.

The durable memory layer is indexed by:

- [`memory/NOW.md`](memory/NOW.md) — the only active task;
- [`memory/AUTHORITY.md`](memory/AUTHORITY.md) — which source governs which question;
- [`memory/CATALOG.md`](memory/CATALOG.md) — every Markdown document by authority and lifecycle;
- [`memory/TIMELINE.md`](memory/TIMELINE.md) — chronological navigation;
- [`memory/INDEX.md`](memory/INDEX.md) — topic pointers.

`MEMORY.md`, `CONTEXT.md`, `memory/CURRENT_TASK.md`, and
`memory/CURRENT_HANDOFF.md` are compatibility/history pointers, not current
project state.
