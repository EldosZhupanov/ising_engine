---
id: ADR-0006
title: "Amendment I: the architectural vector is presumed fixed, not dogmatically fixed"
status: accepted
date: 2026-07-08
tier: constitution
edges:
  - [ADR-0006, amends, CONSTITUTION#Preamble]
  - [ADR-0006, amends, CONSTITUTION#13]
  - [ADR-0006, uses, CONSTITUTION#15]
---

# ADR-0006 — Amendment I: "presumed fixed"

*This is the first amendment to the Constitution, and deliberately exercises
the §15 procedure so that the procedure itself is tested while the stakes are
low.*

## Context

The ratified Constitution stated flatly: **"The architectural vector is
fixed."** The intent — end monthly direction changes, stop re-litigating the
operator architecture — is correct and retained. But the flat wording has a
failure mode the history of computing documents repeatedly: architectural
commitments held past the evidence (Itanium's compiler-will-solve-it bet,
network protocols ossified against measured needs). A constitution that cannot
lose to evidence eventually forces its holders to choose between the document
and the truth — and either choice destroys the document's authority.

The Constitution already contained the cure in §15 (amendments on evidence of
constitutional weight), but the preamble's absolute phrasing contradicted it.

## Decision

The direction clause is reworded, in the preamble and wherever restated:

> **The architectural vector is presumed fixed. It changes only if there is
> reproducible experimental evidence or a demonstrable mathematical
> impossibility that invalidates its assumptions — and only through the
> Amendment Procedure (§15).**

Semantics, made precise:

1. **Presumption, with the burden on the challenger.** In all day-to-day work
   the vector is treated as fixed: research clarifies details, agents align
   before acting, convenience and fashion have standing zero. Nothing about
   daily discipline changes.
2. **Exactly two admissible challenges**: (a) reproducible experimental
   evidence — measured, replayed, adversarially reviewed — that a core
   architectural assumption is false; (b) a demonstrable mathematical
   impossibility. Nothing else opens the question.
3. **Procedure unchanged**: challenges go through §15 (written ADR, evidence
   of constitutional weight, tracked edit). §15's guidance that proposals
   against §1–§3 are presumptively wrong is retained — this amendment sets the
   *standard of proof*, it does not lower it.

## Consequences

- The Constitution remains stable but cannot become dogma: it is now
  falsifiable in exactly the way the project's own scientific philosophy (§5)
  demands of every other claim.
- Applied edits: preamble direction sentence; §13 rule 4 restated with
  "presumed fixed"; the `CLAUDE.md` pointer updated to match.

## Alternatives rejected

- **Keep the absolute wording**: internally inconsistent with §15, and
  historically the losing pattern.
- **Soften further** ("the vector may evolve as we learn"): re-opens monthly
  re-litigation — the disease the Constitution exists to cure.
