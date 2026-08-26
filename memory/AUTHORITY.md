---
id: memory-authority
kind: governance-map
status: active
authority_scope: document-governance
updated: 2026-08-26
immutable: false
---

# Document authority

Authority is scoped by question. A single total ordering is misleading: an ADR
can govern architecture without overriding a binding research protocol, while a
handoff can describe today's task without changing the project's mission.

## Authority matrix

| Scope | Authority | May be superseded by |
|---|---|---|
| Current explicit user decision | the current conversation | a later explicit user decision |
| Engineering workflow and safety | root `AGENTS.md` | explicit user override or reviewed edit to `AGENTS.md` |
| Project mission | `SOUL.md` | an explicit mission revision |
| Project direction | ratified `research/ISING_ENGINE_CONSTITUTION.md` | its amendment procedure |
| Active research protocol | cycle preregistration plus committed amendments | a new committed amendment written before the data it governs |
| Architecture | accepted ADRs | a later accepted ADR that explicitly supersedes them |
| Finished-product definition | `PRODUCT_SPEC.md` | a reviewed product-spec revision |
| Ordered path and gates | `PROJECT_PLAN.md` | a reviewed plan update consistent with binding records |
| Implementation inventory | `ROADMAP.md` | a reviewed roadmap update backed by code/tests |
| Current task | `memory/NOW.md` | the next reviewed update to `memory/NOW.md` |
| Historical scientific fact | immutable cycle records | never; later records may narrow interpretation but not rewrite history |
| Summary/navigation | `START_HERE.md`, `memory/INDEX.md`, pointer files | their named canonical source |

## Conflict protocol

1. Identify the question's scope.
2. Prefer the authority named for that scope, not the newest or largest file.
3. Apply explicit `supersedes` and amendment links.
4. A summary loses to its source; a design loses to an accepted ADR; a roadmap
   loses to a binding protocol for that protocol's scope.
5. If two same-scope authorities disagree and neither explicitly supersedes the
   other, stop. Record the conflict and repair governance before acting.

## Lifecycle vocabulary

| State | Meaning |
|---|---|
| `proposed` | may inform discussion; not a decision |
| `active` | currently governs its declared scope |
| `binding` | frozen evidence/protocol; change only by explicit amendment |
| `closed` | completed or terminated; remains evidence |
| `superseded` | replaced for current decisions; retained for history |
| `historical` | context only; never current authority |

Chronology and authority are independent. A document can be old and binding, or
new and merely proposed.

## Binding-file rule

Files catalogued as `immutable: true` are never edited, moved, or renamed by a
memory migration. Metadata for them lives in `memory/CATALOG.md` and the frozen
hash manifest. A correction is a new linked amendment or record.
