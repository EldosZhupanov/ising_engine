# Ising Engine — start here

This is the single entry point for humans and agents. It is deliberately short:
Git stores the record, while this page tells a new session which part of that
record is authoritative now.

## Mandatory startup

1. Run `git status --short` and `git log -1 --oneline`.
2. Read [`memory/NOW.md`](memory/NOW.md) for the only active task.
3. Read [`memory/AUTHORITY.md`](memory/AUTHORITY.md) before resolving conflicting
   documents.
4. Read only the task bundle named by `memory/NOW.md`; use targeted `rg` for
   anything else.
5. For code work, follow [`AGENTS.md`](AGENTS.md). For Claude Code operation,
   also follow [`CLAUDE.md`](CLAUDE.md).

## Project in one paragraph

Ising Engine contains a stable production optimizer (`UltimateSolver`) and an
autonomous research platform (`engine_v2`) that analyzes a problem, selects
operators by capability, synthesizes a plan, executes it reproducibly, and
learns from evidence. The selector platform exists; what has not yet been earned
is a production claim that its selected plans beat `UltimateSolver` at equal
cost on an independently replicated domain.

## Where each answer lives

| Question | Canonical source |
|---|---|
| Why the project exists | [`SOUL.md`](SOUL.md) |
| Direction | [`research/ISING_ENGINE_CONSTITUTION.md`](research/ISING_ENGINE_CONSTITUTION.md) |
| Definition of a finished product | [`PRODUCT_SPEC.md`](PRODUCT_SPEC.md) |
| Projects and untracked work | [`PROJECTS.md`](PROJECTS.md) and [`memory/FILE_MAP.tsv`](memory/FILE_MAP.tsv) |
| Engineering rules | [`AGENTS.md`](AGENTS.md) |
| Architecture decisions | [`research/architecture/ADR/`](research/architecture/ADR/) |
| Active scientific/product sequence | [`PROJECT_PLAN.md`](PROJECT_PLAN.md) |
| Stage inventory | [`ROADMAP.md`](ROADMAP.md) |
| Current task and next action | [`memory/NOW.md`](memory/NOW.md) |
| Research evidence | binding preregistrations, amendments, and cycle records under [`research/`](research/) |
| Documentation catalogue | [`memory/CATALOG.md`](memory/CATALOG.md) |
| Chronology | [`memory/TIMELINE.md`](memory/TIMELINE.md) |

## Non-negotiable distinctions

- A fresh document is not automatically authoritative.
- A closed or superseded document remains evidence of what happened, but cannot
  define current work.
- A summary never overrides its named source.
- Binding research documents are immutable. Corrections are new amendments or
  records, never silent edits.
- Git history proves chronology; dates are descriptive only.
- If two documents claim authority over the same question and disagree, stop
  and repair the authority map before continuing implementation or research.

## Optional code navigation

Use the pinned local [Graft adapter](scripts/graft.sh) for symbol and relationship
navigation after the mandatory startup. Install/rebuild with
`bash scripts/graft.sh setup` (Node/npm, Python, Git, curl and flock required).

```bash
bash scripts/graft.sh ask "UltimateSolver" --in src/solver/ultimate.rs --source
bash scripts/graft.sh skeleton src/core/csr_matrix.rs
bash scripts/graft.sh callers "roof_duality_persistencies" --in src
```

The adapter indexes visible supported source files, including dirty/untracked
code, under `.cache/graft-index`; it excludes solver `.sol` artifacts. Scope
ambiguous symbols with `--in`: legacy prototypes can outrank production code.
Use the catalogue and explicit-path `rg` for Markdown/evidence. The generated
graph is navigation, not authority or proof of a call edge. See the
[qualification record](research/HYPODIVE_TRIAGE.md#graft-local-integration--2026-09-27).
