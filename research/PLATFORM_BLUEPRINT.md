# The Ising Engine Research Platform — 10-Year Blueprint

Role: CTO / Chief Scientist / systems architect. Status: DESIGN (no code
changed by this document). Inspirations audited: everything-claude-code,
LLVM (RFC+regression-suite culture), Linux kernel (maintainer trees,
Documentation/process), Rust Foundation (RFC lifecycle, editions),
DeepMind/Anthropic (eval-first, model cards), university labs (lab
notebooks, pre-registration). Copied: none. The design principle below is
stronger than any of them *for this domain* because this domain has what
they lack: cheap, abundant ground truth.

## 0. The Organizing Principle: CLAIM-ANCHORED ENGINEERING

Every artifact the project produces — commit, experiment, optimization,
memory entry, decision, report table, release note — is a **claim**, and
every claim must be bound to a machine-checkable **anchor**:

| Anchor class | Examples (already in use) |
|---|---|
| Exhaustive oracle | brute force ≤ 2^20 states (QPBO validation: 16,500 instances) |
| Published optimum | bqp50.1=2098, gka1a=−3414, sg3dl051000=110 |
| Official solution file | QPLIB_3565.sol → 282.0000 exactly |
| Bit-identical replay | golden trajectory test; identical-seed A/B energies |
| Content-addressed artifact | SHA256-pinned datasets; baseline binaries |
| Statistical certificate | bootstrap CI, Wilcoxon/sign p, Cliff's δ over fixed seeds |

**System integrity = graph connectivity to anchors.** A claim without an
anchor is a hypothesis; hypotheses are welcome but cannot enter reports,
rules, or memory. This one rule subsumes "scientific integrity
monitoring": integrity violations become *dangling references*, which a
dumb indexer can detect. No judgment required.

The 10-year enemy is **environment decay** (hardware, toolchains, dead
URLs, drifting baselines). The counter-principle: **measure ratios, not
times; address content, not locations; re-anchor on migration.**

---

## 1. Operating System for Research

```
                          ┌──────────────────────────────────────┐
                          │             ANCHOR STORE             │
                          │  oracles · published optima · sol    │
                          │  files · golden trajectories ·       │
                          │  SHA256 datasets · baseline binaries │
                          └───────────────▲──────────────────────┘
                                          │ every edge below must
                                          │ terminate here
   ┌───────────┐   freeze-before-code  ┌──┴─────────┐  ratio records  ┌───────────┐
   │   CODE    │◄──────────────────────│ EXPERIMENTS│◄───────────────►│ BENCHMARKS│
   │ commits + │   Exp:/ADR: trailers  │  EXP-NNN   │                 │ ledger +  │
   │ toolchain │──────────────────────►│  frozen    │                 │ campaigns │
   └─────┬─────┘                       └──────┬─────┘                 └─────┬─────┘
         │                                    │                             │
         │        ┌───────────┐         ┌─────▼─────┐                 ┌─────▼─────┐
         └───────►│ DECISIONS │◄────────│ KNOWLEDGE │◄────────────────│ FINDINGS  │
                  │  ADR-NNN  │  cites  │ graph +   │  auto-drafted,  │ (decision │
                  └─────┬─────┘         │ memory    │  human-ratified │  intel)   │
                        │               └─────┬─────┘                 └───────────┘
                        ▼                     ▼
                 ┌────────────────────────────────────┐
                 │  PUBLICATIONS & RELEASES           │
                 │  compiled from raw + ledger + EXP; │
                 │  never hand-written                │
                 └────────────────────────────────────┘
```

Interaction contracts (the OS "syscalls"):
- **code → experiments**: behavior-changing work must reference a FROZEN
  EXP (git commit order proves freeze-before-implementation).
- **benchmarks → ledger**: every A/B appends a ratio record with env
  fingerprint + binary hashes. Absolute times never compared across days.
- **findings → knowledge**: promotion requires an anchor; the system
  proposes, ratification requires evidence, never vibes.
- **publications**: `raw/*.jsonl → stats → tables/plots → REPORT` is one
  deterministic pipeline; a hand-edited table is a build break.
- **releases**: compiled from ledger + EXP verdicts since last release.
- **memory**: agent-facing distillation of the same graph, budget-capped.

## 2. Knowledge Graph

**Entities** (17): Algorithm, OptimizationChange (OPT-NNN), Experiment
(EXP-NNN), Instance, Dataset, BenchmarkCampaign, Measurement, Claim,
Anchor, Decision (ADR-NNN), Commit, Bug, Regression, Finding, Validation,
Report/Paper, Release, Environment (host×toolchain), Baseline (binary).

**Relations are verification relations, not documentation links:**
`VALIDATED_AGAINST` (claim→anchor), `MEASURED_BY` (opt→measurement),
`FROZEN_BEFORE` (exp→commit; provable from git order), `REJECTED_BECAUSE`
(opt→measurement), `SUPERSEDES` (adr→adr), `REGRESSED_IN`
(measurement→commit), `DEPENDS_ON` (measurement→environment),
`REPRODUCED_BY` (exp→later run), `MOTIVATED_BY` (exp→finding),
`IMPLEMENTED_IN` (opt→commit), `CITES` (report→everything).

**Storage: files ARE the database.** Front-mattered Markdown for
narrative entities (EXP/ADR/OPT/Finding), JSONL for high-volume ones
(measurements, ledger). Git is the temporal dimension (who/when/order —
FROZEN_BEFORE is free). Rationale: plain text + git is the only store
with a demonstrated 30-year survival record, it is Claude-Code-native
(rg-searchable), diffs are reviewable, and the volume (≈10³ nodes/decade
for this project) never justifies a server.

**Auto-update via chokepoint emitters, not crawlers:** the A/B harness
writes Measurement records; `/experiment` scaffolds EXP files; commit
trailers (`Exp: EXP-014`, `ADR: ADR-007`, `Opt: OPT-021`) let a ~200-line
indexer build `kb/graph.json` + generated indexes. The indexer **fails on
dangling references** — that failure is the integrity monitor.

## 3. Research Database

Object = **Record**: one file, one entity, YAML frontmatter + body.

Common fields: `id`, `type`, `title`, `status`, `created`, `anchors:[]`
(mandatory, ≥1 for any status past DRAFT), `links:{relation:[ids]}`,
`env:` (when measured). Type-specific: EXP adds
`hypothesis/panel/seeds/thresholds/verdict`; OPT adds
`bottleneck_evidence/asm_before/asm_after/speedup/verdict`; ADR adds
`options_considered(≥3)/decision/review_by`; Finding adds
`signature/confidence/proposed_direction`.

Indexes (generated, never hand-maintained): `kb/INDEX.md` (one line per
record — the token-budget view for Claude), `by-status`, `by-instance`,
`by-family`, `by-anchor-class`, `graph.json`. Search layers: (1) rg over
frontmatter (instant), (2) indexer queries
(`kb q 'type:opt verdict:rejected touching:engine.rs'` — deterministic
filters), (3) graph traversal ("what depends on this anchor?"). Claude
Code integration: CLAUDE.md already mandates INDEX-first reading; kb/
extends the same discipline — the model reads one generated index line,
then opens exactly one record.

## 4. Architecture Decision Records

Format: `ADR-NNN-slug.md`; statuses PROPOSED → ACCEPTED →
SUPERSEDED-BY/REJECTED; mandatory: context, **≥3 options with pro/con**,
decision, consequences, **evidence anchors**, `review_by` date (decisions
expire into re-validation, never into folklore).

Retroactive backlog (all evidence already exists in-repo):
- ADR-001 Byte-per-replica SIMD layout over multi-spin coding (types.rs
  rationale; revisit trigger: n≥10⁵ ambitions — adversarial review Rank 6).
- ADR-002 NUM_REPLICAS=64 (anchor: golden suite; known cost: Rank-1).
- ADR-003 No FP contraction / no FMA (anchor: bit-identity requirement;
  measured consequence: vmulpd+vaddpd in sweep).
- ADR-004 XOR-SUB i8 product (anchor: OPT record, ×1.07 batch, vpmullw→0).
- ADR-005 Equal-wall-clock benchmark protocol with verified calibration
  (anchor: adherence tables; rejected alternative: equal-sweeps —
  meaningless across 640-chain vs 1-chain architectures).
- ADR-006 Best-found vs best-known reference policy (anchor: REPORT
  threats-to-validity).
- ADR-007 Objective conventions per library (anchor: published optima +
  QPLIB sol file — the 3-bug incident is the *why* documentation).
- ADR-008 Files-as-database for the KB (this blueprint §2, options in §11).

## 5. Experiment Registry

Lifecycle: **DRAFT** (hypothesis, panel, seeds, budgets, thresholds) →
**FROZEN** (committed *before* any implementation commit; git order = the
pre-registration certificate) → **RUN** (pinned toolchain, SHA256-checked
data, seeds from the file, raw output archived) → **VERDICT**
(ACCEPTED/REJECTED/AMENDED — amendments are diffs to a frozen file,
inherently visible) → **ARCHIVED** with a **replay bundle**: exact command
lines, binary SHA256s, dataset manifest hashes, env fingerprint.

Five-year repeatability = replay bundle + content-addressed store +
pinned toolchain + the hub's multi-mirror downloader (URL rot is already
handled: local cache is authoritative after first fetch, hash-verified).
Hardware will differ in five years — therefore verdicts are defined on
**ratios and quality metrics** (gap, success probability, TTS ratio vs a
re-run baseline binary), never on absolute milliseconds.

## 6. Scientific CI/CD

Not test-CI; **claim-CI**. Pipeline (all local scripts; a server is an
optional executor, never a dependency):

- **Stage 0 — fast gate (blocking, per change):** fmt · clippy -D
  warnings · full tests · golden bit-identity · determinism.
- **Stage 1 — sentinel (blocking on identity, advisory on speed):**
  3-instance panel (sparse G22 / dense bqp1000 / clamp-heavy gka), re-run
  the preserved baseline binary *back-to-back* with HEAD (ratio-based ⇒
  immune to the host's ~9% drift), assert energy identity, append ledger.
- **Stage 2 — claim diff (blocking for publication branches):**
  regenerate all tables/plots from raw; `git diff` on generated outputs;
  any changed number without a linked EXP/OPT = failure. This is the
  automatic answer to "did scientific results / tables / plots / optima
  change?"
- **Stage 3 — graph integrity:** indexer run; dangling anchors, orphaned
  claims, unfrozen-but-run experiments = failure.
- **Stage 4 — periodic re-anchor (cron/loop):** re-validate convention
  anchors (gka1a=−3414, QPLIB 282.0000, bqp optima, golden), re-verify
  dataset hashes. Catches silent decay: parser edits, data corruption,
  toolchain effects.

## 7. Optimization Knowledge Base

`kb/opt/OPT-NNN-slug.md` — the unit of performance knowledge. Fields:
problem (bottleneck class from the taxonomy), evidence (asm
before/after, instruction census, spill/branch observations), hypothesis,
minimal change, A/B measurement (ledger ref), statistics, **verdict**,
why. Rejected entries are first-class citizens — OPT-lane-halving
(×0.98, reverted) is worth as much as OPT-xor-sub (×1.07): it prevents
re-derivation. Retroactive seeds already written in this repo's history:
Fixes A–D, XOR-SUB+bounds+skip, extraction de-interleave, lane-halving
rejection, plus the adversarial-review taxonomy as the classification
scheme.

## 8. Decision Intelligence

After every campaign, an analyzer (rules + regressions, zero free
generation) emits **Findings**:
1. **Phase attribution** — time split presolve/sweep/exchange/extraction
   (requires a `--timing-breakdown` flag on the solver bridge: the one
   small code change this blueprint requests, P1).
2. **Scaling fits** — per family, time & gap vs n, nnz; exponents with CIs.
3. **Signature → bottleneck-class mapping** using the audited taxonomy
   (e.g. "sparse family ∧ per-chain sweeps < c·n ∧ gap grows with n" ⇒
   depth-starvation ⇒ cite Rank-1 + OPT precedents).
4. **Ranked directions**, each citing its measured signature and prior
   OPT/EXP records. A direction without a signature is not emitted.
Humans ratify Findings into EXP drafts. The system narrows the search; it
never invents.

## 9. Long-Term Evolution

- **Year 1:** ~50 EXP, ~20 ADR, ~30 OPT, ledger in the hundreds; a new
  contributor reaches productive state from INDEX + ADRs + CLAUDE.md
  alone (measurable onboarding test: "explain why no FMA" answerable
  without asking anyone).
- **Year 3:** first hardware/toolchain migration ⇒ **re-anchoring
  ceremony**: rerun anchor suite on new env, open new ratio-chain in the
  ledger, ADR the migration. Comparability preserved by design.
- **Year 5:** platform extraction — the domain-agnostic layer (registry,
  hub, calibration protocol, ledger, claim-CI) becomes a standalone tool;
  SAT plugs in via the Problem/Oracle interface (§12); the Ising engine
  becomes its first tenant, not its owner.
- **Year 10:** the code may be rewritten entirely; the *graph* — claims,
  anchors, verdicts, data hashes, plain text — remains valid and legible.
  Knowledge-loss prevention is structural: no orphan claims (Stage 3),
  expiry-review on ADRs, provenance on memory, negative results retained.

## 10. Self-Improving Research System

The only admissible loop:

```
measurement → auto-drafted Finding (signature-cited)
    → human/oracle ratification → KB record with anchors
        → IF mechanically checkable: enforcement (hook/CI stage)
            → future measurements
```

Rules have lifecycles: PROPOSED (evidence attached) → ACTIVE (cited by
enforcement) → INVALIDATED (anchor vanished — detected by the sweep, not
by memory). Memories cite sources; recommendations cite experiments;
enforcement cites rules. **No auto-promotion**: the system may write
drafts all day; nothing becomes normative without an anchor. This is the
corrected everything-claude-code learning loop — theirs extracts
patterns heuristically and trusts them; ours extracts proposals and
verifies them.

## 11. Adversarial Review of This Blueprint

**Attack 1 — ceremony kills velocity (solo researcher).** Real. Defense:
scope gates — pre-registration/OPT records only for behavior-changing or
performance-claiming work; bit-identical refactors and docs exempt; EXP
template ≤ 1 page. Residual risk accepted; monitor: if >20% of working
time is ceremony, cut tiers by ADR.
**Attack 2 — files-as-DB won't scale.** At 10³–10⁴ records it does
(rg + index). If exceeded, the indexer's *output* moves to SQLite; the
*sources of truth* stay text. No migration cliff.
**Attack 3 — single-host science.** The ledger records env; ratios are
host-local; cross-host claims require re-anchoring. Publication reports
already disclose this (threats-to-validity). Multi-host is a P2 add,
not a redesign.
**Attack 4 — gaming frozen thresholds via easy panels.** Panels are
fixed by ADR and changed only by ADR; the standard panel spans
sparse/dense/clamp-heavy adversarially (chosen from the engine's known
weak spots, not its strengths).
**Attack 5 — blocking CI flakes on a 9%-drift host.** Only *identity*
blocks; speed is advisory at the sentinel and binding only through
back-to-back ratio A/B, which drift cannot fake.

**Alternative architectures compared (3, per house rules):**
- **A. Industrial stack** (GitHub Actions + Postgres + MLflow-style
  tracker + dashboards): + standard tooling, multi-user ready; − offline-
  hostile, infra to maintain, 10-year durability poor, alien to
  Claude-Code workflows. Score for this project: 2/5.
- **B. Files-as-database + chokepoint emitters + local claim-CI**
  (chosen): + durable, greppable, zero infra, git-native provenance,
  agent-native; − limited query power, discipline-dependent (mitigated by
  Stage-3 enforcement). 4.5/5.
- **C. Notebook-centric lab** (Quarto/Jupyter research narratives):
  + publication-friendly narrative; − unenforceable, diff-hostile,
  binary-artifact rot, invites hidden state. 1.5/5.
Decision: **B**, with A's *runner* adoptable later as a mere executor of
the same scripts. This is ADR-008 material.

## 12. Final Blueprint — the general platform

Strip the Ising specifics and five interfaces remain; any optimization
domain (SAT, MaxSAT, QUBO/Ising, TSP, kernel autotuning, AI eval) plugs
into them:

1. **Problem** — parse bytes → canonical model + energy/score function
   (conventions validated against an Anchor before trust).
2. **Oracle** — ground-truth provider hierarchy: exhaustive | published |
   official-solution | proof-checked (SAT: DRAT-checked certs) |
   bit-identical-replay | statistical certificate. Every domain has at
   least replay + statistics.
3. **Solver adapter** — black-box contract (already proven here):
   `args(instance, budget-knob, seed) → JSON{state, metric, wall}` —
   language-agnostic, version-addressed by binary hash.
4. **Protocol** — budget equalization with verified calibration (<5%),
   fixed seed lists, per-instance references, class-based panels.
5. **Evidence pipeline** — measurements → ledger → stats (CIs,
   paired tests, effect sizes) → compiled reports → claim-diff CI —
   with the knowledge graph (EXP/ADR/OPT/Finding) wrapped around it.

What makes this platform *surpass* its inspirations for algorithm
research: LLVM has the regression suite but no pre-registration; labs
have pre-registration but no bit-identity; everything-claude-code has
the closed loop but no ground truth; the industry trackers have metrics
but no claims. This design has all four, because the domain uniquely
permits it — **optimization research is the rare field where the truth
is computable.** The platform's moat is not code; it is the growing,
anchored, replayable graph of what was tried, what was measured, and
what was proven.

---
*Implementation staging (unchanged from the P0–P3 roadmap): toolchain
pin, ledger, pre-registration first (~2.5 h); sentinel + claim-diff CI +
kb indexer second (~1 day); decision-intelligence analyzer + platform
extraction later. This document changes no code.*
