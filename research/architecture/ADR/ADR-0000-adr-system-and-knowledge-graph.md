---
id: ADR-0000
title: Architecture Decision Records with typed knowledge-graph edges
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [ADR-System, implements, CONSTITUTION#12]
  - [ADR-System, implements, CONSTITUTION#15]
  - [Knowledge-Graph, realized_by, ADR-System]
anchors:
  - practice: LLVM/Rust RFC processes; Nygard ADRs (2011) — decision memory in long-lived projects
---

# ADR-0000 — The ADR system and the knowledge graph

## Context

The project now has a Constitution (fixed direction) and blueprints (fixed
architecture shape). What it lacks is **architectural memory**: in a year,
nobody — human or agent — will remember *why* `SparseBitSlice` exists alongside
`DenseByte`, why operators replaced algorithms, or why a specific pass is
deterministic. Undocumented decisions get re-litigated, and re-litigation is
how projects lose their fixed vector.

A second requirement goes beyond prose: decisions, operators, experiments, and
evidence form a graph ("why Integer Fields?" → ADR → experiment → benchmark →
implementation). Markdown alone cannot answer graph queries.

## Decision

1. Every architecture-level decision is recorded as an ADR in
   `research/architecture/ADR/ADR-NNNN-<slug>.md`, numbered sequentially,
   append-only. Statuses: `proposed | accepted | amended | superseded`.
   Superseded ADRs are never deleted — they are memory of why we changed.

2. Every ADR carries **machine-readable frontmatter** with typed graph edges:

   ```yaml
   edges:
     - [Subject, predicate, Object]
   ```

   Standard predicates: `implements`, `refines`, `requires`, `uses`,
   `validated_by`, `anchored_by`, `motivated_by`, `supersedes`, `amends`,
   `conflicts_with`, `realized_by`. Nodes are stable slugs: constitutional
   sections (`CONSTITUTION#9`), ADRs (`ADR-0002`), components
   (`SparseBitSlice`, `Operator-API`), experiments (`EXP-*`), and evidence
   artifacts (`BENCH-*`).

3. The graph is **files-as-database** (consistent with PLATFORM_BLUEPRINT):
   the frontmatter *is* the graph; `research/architecture/query_graph.py`
   extracts and queries it (`list`, `edges <node>`, `why <node>`). No separate
   graph store to drift out of sync with the documents.

4. Required ADR sections: Context, Decision, Evidence, Consequences,
   Alternatives rejected. Evidence must name its anchors (benchmarks,
   experiments, published results) — an ADR without evidence is `proposed`,
   not `accepted`.

5. Experiments get `EXP-NNNN` identifiers when the Research Track (§13
   pipeline) runs them; ADRs link to them via `validated_by`. Benchmark
   campaigns get `BENCH-*` identifiers.

## Consequences

- "Why X?" becomes a query, not an archaeology project.
- New agents/contributors reconstruct rationale without replaying history.
- Cost: every architecture decision now requires writing ~1 page. This is the
  intended friction — a decision not worth a page is not an architecture
  decision.

## Alternatives rejected

- **Dedicated graph database**: infrastructure to maintain, drifts from the
  documents, violates files-as-database.
- **Prose-only ADRs (no edges)**: preserves rationale but cannot answer
  transitive "why" queries; loses exactly the CERN-style traceability this
  system exists for.
