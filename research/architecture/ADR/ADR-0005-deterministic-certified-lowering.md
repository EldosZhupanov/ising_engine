---
id: ADR-0005
title: Lowering passes are deterministic and emit energy-preservation certificates
status: accepted
date: 2026-07-08
tier: architecture
edges:
  - [Lowering-Pipeline, implements, CONSTITUTION#7]
  - [Lowering-Pipeline, requires, Bit-Identical-Determinism]
  - [Quantization-Pass, implements, Lowering-Pipeline]
  - [Reorder-Pass, implements, Lowering-Pipeline]
  - [Persistency-Pass, implements, Lowering-Pipeline]
  - [Lowering-Pipeline, motivated_by, Parser-Convention-Traps]
  - [Parser-Convention-Traps, anchored_by, Published-Optima-Validation]
---

# ADR-0005 — Why deterministic, certified lowering

## Context

Between the user's problem and the executed instance sits a chain of
transformations: normalization, persistency fixing (QPBO), decomposition,
quantization, reordering, coloring. Each is an opportunity to silently corrupt
the objective — and this project has *already been burned* three times at the
even-simpler parsing stage (Parser-Convention-Traps): OR-Library's symmetric
double-count MAXIMIZE, Biq Mac's same-count MINIMIZE, QPLIB's ½-factor with
adaptive layout. All three produced plausible-looking wrong objectives; all
three were caught **only** by validating against published optima — cross-
solver agreement could not catch them because both solvers consumed the same
wrong model.

The lesson generalizes: any transformation whose correctness is not
machine-checkable will eventually be wrong, and the error will be invisible
from inside the system.

Determinism is a separate necessity: if a pass's output depends on hash
ordering, thread timing, or pointer values, then (problem, plan, seeds) no
longer determines the trajectory, and the entire replay discipline (ADR-0004)
collapses at the front door.

## Decision

1. **Every lowering pass is a pure, deterministic function** of the IR and its
   parameters: fixed iteration orders, no address-dependent or time-dependent
   behavior, seeded randomness only (and recorded in the plan).
2. **Every pass emits a machine-checkable certificate** that the objective is
   preserved under a recorded bijection:
   - normalize/symmetrize: coefficient-preservation identity;
   - persistency: QPBO persistency proofs for each fixed variable;
   - decomposition: component partition + offset bookkeeping;
   - quantization: exactness certificate when input is integral (G-Set,
     ORLIB, most Biq Mac), or an explicit typed ε with bound otherwise —
     ε-quantization is behavior-changing and marked as such;
   - reorder/color: stored permutation / class partition, invertible.
3. **Certificates are checked, not trusted**: spot-verified in tests by
   evaluating random configurations through the bijection on both sides, and
   fully verified for every instance whose final answer becomes a claim.
4. The final answer is always mapped back through the recorded bijections and
   **re-scored by the canonical scorer against the original bare model** —
   the last line of defense that catches any pass bug end-to-end.

## Evidence

- Parser-Convention-Traps: three real, externally-caught objective corruptions
  in this project's history — the base rate of transformation bugs is not
  hypothetical.
- Published-Optima-Validation: bqp50#1 = 2098, gka1a = −3414,
  QPLIB_3565 = 282.0000 — the external anchors that caught them, now
  institutionalized (Constitution §4.12).
- LLVM precedent: pass verifiers and `-verify` infrastructure exist because
  mature pipelines assume passes *will* have bugs.

## Consequences

- Writing a pass costs more (certificate + verifier alongside the transform).
  Intended friction.
- End-to-end re-scoring makes the certificate system defense-in-depth: a
  wrong certificate and a wrong pass must *coincide* to corrupt a claim.
- Pass ordering is part of the plan and thus part of the reproducibility
  contract — reordering passes is a recorded, versioned change.

## Alternatives rejected

- **Trust + tests**: unit tests sample the input space; the parser incidents
  prove convention-level bugs survive sampling and cross-checks.
- **Certify only "risky" passes**: every one of the three historical bugs was
  in a step considered trivial. There is no reliable notion of "risky enough."
