---
id: ADR-0011
status: accepted
date: 2026-08-26
supersedes: []
related: [ADR-0000, ADR-0004]
---

# ADR-0011 — Git-backed durable project memory

## Context

The repository contains more than one hundred Markdown files. Git preserved
every version, but several documents simultaneously claimed to describe the
current task. `CURRENT_TASK.md`, `CURRENT_HANDOFF.md`, `PROJECT_PLAN.md`, and
parts of `ROADMAP.md` drifted to different dates. Agents were instructed to read
one of those stale files at startup, so durable storage did not produce durable
context.

The root `AGENTS.md` also exceeded Codex's default project-instruction budget.
Adding more prose to startup files would worsen the failure.

## Decision

1. Git-tracked Markdown remains the source of truth.
2. `START_HERE.md` is the single startup page and `memory/NOW.md` is the sole
   live-state document.
3. Authority is scoped by question in `memory/AUTHORITY.md`; there is no false
   global ordering among unrelated scopes.
4. Every document is represented in a catalogue with lifecycle and chronology.
5. Binding research evidence remains byte- and path-stable; sidecar metadata
   describes it without editing it.
6. Obsidian may open the repository root as a human navigation layer, but it is
   not a second store and may not auto-commit.
7. Automated checks enforce startup size, catalogue coverage, links, unique live
   authority, and binding-file integrity.
8. Retrieval or graph-RAG may be added only after the canonical layer is clean;
   retrieved text never outranks its source.

## Alternatives considered

### Git alone, unchanged

Rejected. It versions bytes but does not identify which conflicting document a
new session should trust.

### A separate Obsidian vault

Rejected. It creates a second truth store and a synchronization problem.

### Replace Markdown with a database or hosted wiki

Rejected. It weakens reviewable chronology, portability, and the binding
pre-registration record.

### GraphRAG/MemGPT as primary memory

Rejected for the canonical layer. Retrieval over stale documents makes stale
answers easier to produce; it does not resolve authority.

## Consequences

- Agents receive a small deterministic bootstrap instead of scanning the repo.
- Live status has one writer and one canonical location.
- Historical and binding documents remain visible without masquerading as
  current instructions.
- Updating `memory/NOW.md` and the catalogue becomes part of the relevant task's
  documentation duty.
- The repository gains validation tooling, but no runtime dependency.
