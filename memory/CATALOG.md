# Document catalogue

This sidecar catalogue classifies every Markdown document without modifying frozen evidence. Paths are the stable identifiers. Dates come from Git history; they describe chronology but do not establish authority.
Its enforced scope is Git-tracked plus nonignored untracked Markdown (371 files
at this update). The 55 Git-ignored Markdown reports and download READMEs are
classified separately as historical evidence in [FILE_MAP.tsv](FILE_MAP.tsv);
the catalogue gate intentionally excludes ignored paths.

## Field meanings

- **Authority:** A0 governs a named scope; A1 is a primary supporting source; A2 is a reviewed design or analysis; A3 is a pointer/publication/reference; A4 is historical only.
- **Lifecycle:** defined in [AUTHORITY.md](AUTHORITY.md).
- **Immutable:** the file must not be edited, moved, or renamed. Corrections require a new linked artifact.
- **Canonical scope:** the question for which the file may be authoritative. `none` means it cannot decide current work.

## Summary

- `active`: 18
- `binding`: 53
- `closed`: 45
- `historical`: 2
- `proposed`: 43
- `reference`: 206
- `superseded`: 4

## Registry

| Path | Kind | Authority | Lifecycle | Created | Last changed | Immutable | Canonical scope |
|---|---|---:|---|---|---|---|---|
| `research/experiments/entity_resolution/RESULT.md` | research-result | A1 | closed | 2026-09-27 | 2026-09-27 | true | ER-001 |
| `research/experiments/entity_resolution/protocol.md` | research-protocol | A1 | binding | 2026-09-27 | 2026-09-27 | true | ER-001 |
| `research/experiments/labs_q002/RESULT.md` | research-result | A1 | closed | 2026-09-26 | 2026-09-26 | true | LABS-Q002 |
| `research/experiments/hubo_corpus_qualification/RESULT_PHASE_B.md` | research-result | A1 | closed | 2026-09-27 | 2026-09-27 | true | HUBO-Q002-Phase-B |
| `research/experiments/hubo_corpus_qualification/RESULT_PHASE_A.md` | research-result | A1 | closed | 2026-09-27 | 2026-09-27 | true | HUBO-Q002-Phase-A |
| `research/experiments/hubo_corpus_qualification/protocol.md` | preregistration | A0 | binding | 2026-09-27 | 2026-09-27 | true | HUBO-Q002 |
| `research/experiments/hubo_comparison/RESULT.md` | research-result | A1 | closed | 2026-09-27 | 2026-09-27 | true | HUBO-C001 |
| `research/experiments/hubo_comparison/HYPODIVE_BUILDER_HANDOFF.md` | research-handoff | A2 | reference | 2026-09-27 | 2026-09-27 | false | HUBO-C001 |
| `research/experiments/hubo_comparison/protocol.md` | preregistration | A0 | binding | 2026-09-27 | 2026-09-27 | true | HUBO-C001 |
| `research/experiments/hubo_comparison/HYPODIVE_BUILDER_INTAKE.md` | research-intake | A2 | reference | 2026-09-27 | 2026-09-27 | false | HUBO-C001 |
| `research/experiments/hubo_representation_gate/RESULT.md` | research-result | A1 | closed | 2026-09-27 | 2026-09-27 | true | HUBO-RG001 |
| `research/experiments/hubo_representation_gate/protocol.md` | preregistration | A0 | binding | 2026-09-27 | 2026-09-27 | true | HUBO-RG001 |
| `research/experiments/labs_q002/protocol.md` | preregistration | A0 | binding | 2026-09-26 | 2026-09-26 | true | LABS-Q002 |
| `.claude/agents/doc-keeper.md` | agent-role | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/agents/explorer.md` | agent-role | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/agents/implementer.md` | agent-role | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/agents/perf-verifier.md` | agent-role | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/agents/planner.md` | agent-role | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/agents/reviewer.md` | agent-role | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/commands/benchmark.md` | agent-command | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/commands/explore.md` | agent-command | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/commands/implement.md` | agent-command | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/commands/plan.md` | agent-command | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/commands/review.md` | agent-command | A3 | reference | 2026-07-05 | 2026-07-05 | false | agent-tooling |
| `.claude/commands/verify.md` | agent-command | A3 | reference | 2026-07-05 | 2026-07-07 | false | agent-tooling |
| `AGENTS.md` | engineering-rules | A0 | active | 2026-07-05 | 2026-08-26 | false | engineering-workflow |
| `anls_design_spec.md` | design-spec | A2 | reference | 2026-03-12 | 2026-03-12 | false | design |
| `architecture_spec.md` | reference | A3 | reference | 2026-03-12 | 2026-03-12 | false | none |
| `ARCHITECTURE.md` | engineering-reference | A1 | active | 2026-03-11 | 2026-03-11 | false | engineering |
| `bayesian_design_spec.md` | design-spec | A2 | reference | 2026-03-12 | 2026-03-12 | false | design |
| `benchmark_suite/README.md` | reference | A3 | reference | 2026-07-07 | 2026-07-07 | false | none |
| `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/ms_03_050_002/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/ms_03_050_005/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/ms_03_050_007/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/ms_03_050_009/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/ms_03_100_001/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/01-marketsplit/20260924_UltimateSolver_Zhupanov/ms_03_100_012/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/20260924_UltimateSolver_Zhupanov/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/20260924_UltimateSolver_Zhupanov/sloane_1dc_128/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/20260924_UltimateSolver_Zhupanov/sloane_1dc_64/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/20260924_UltimateSolver_Zhupanov/sloane_2dc_128/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/qoblib/submissions/20260924_UltimateSolver_Zhupanov/socfb-haverford76/README.md` | benchmark-submission-guide | A3 | reference | 2026-09-24 | 2026-09-24 | false | QOBLIB-submission |
| `benchmarks/report.md` | reference | A3 | reference | 2026-07-06 | 2026-07-07 | false | none |
| `CLAUDE.md` | agent-operations | A1 | active | 2026-07-05 | 2026-08-26 | false | claude-operation |
| `CONTEXT.md` | historical-summary | A4 | superseded | 2026-07-05 | 2026-08-19 | false | none |
| `design_spec.md` | design-spec | A2 | reference | 2026-03-12 | 2026-03-12 | false | design |
| `gnn_design_spec.md` | design-spec | A2 | reference | 2026-03-12 | 2026-03-12 | false | design |
| `INDEX.md` | navigation | A3 | reference | 2026-07-05 | 2026-09-25 | false | compatibility |
| `MEMORY.md` | historical-summary | A4 | superseded | 2026-07-05 | 2026-08-19 | false | none |
| `memory/ARCHITECTURE.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-08-19 | false | navigation |
| `memory/AUTHORITY.md` | governance-map | A0 | active | 2026-08-26 | 2026-08-26 | false | document-governance |
| `memory/BENCHMARKS.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-08-19 | false | navigation |
| `memory/CATALOG.md` | document-registry | A1 | active | 2026-08-26 | 2026-09-26 | false | document-catalogue |
| `memory/CLAUDE_PRODUCT_COMPARISON_TASK.md` | historical-instruction | A4 | historical | 2026-08-22 | 2026-08-26 | false | none |
| `memory/CURRENT_HANDOFF.md` | historical-handoff | A4 | superseded | 2026-08-22 | 2026-08-26 | false | compatibility |
| `memory/CURRENT_TASK.md` | historical-handoff | A4 | superseded | 2026-08-19 | 2026-08-26 | false | compatibility |
| `memory/DECISIONS.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-09-26 | false | navigation |
| `memory/INDEX.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-08-26 | false | navigation |
| `memory/NEXT_TASK_RC027.md` | task-bundle | A1 | active | 2026-08-30 | 2026-08-30 | false | RC027 |
| `memory/NOW.md` | live-state | A0 | active | 2026-08-26 | 2026-09-27 | false | current-task |
| `memory/OBSIDIAN.md` | workspace-guide | A3 | reference | 2026-08-26 | 2026-08-26 | false | navigation |
| `memory/OPEN_PROBLEMS.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-08-30 | false | navigation |
| `memory/PERFORMANCE.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-08-19 | false | navigation |
| `memory/RESEARCH.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-09-26 | false | navigation |
| `memory/ROADMAP.md` | memory-pointer | A3 | reference | 2026-08-19 | 2026-08-26 | false | navigation |
| `memory/TIMELINE.md` | chronology | A1 | active | 2026-08-26 | 2026-09-27 | false | project-chronology |
| `PERF.md` | engineering-reference | A1 | active | 2026-07-05 | 2026-07-06 | false | engineering |
| `portfolio_design_spec.md` | design-spec | A2 | reference | 2026-03-13 | 2026-03-13 | false | design |
| `PRODUCT_SPEC.md` | product-spec | A0 | active | 2026-08-22 | 2026-08-26 | false | finished-product |
| `PROJECT_PLAN.md` | active-plan | A0 | active | 2026-08-22 | 2026-08-30 | false | ordered-gates |
| `README.md` | public-overview | A2 | active | 2026-03-11 | 2026-09-27 | false | public-overview |
| `research/CANDIDATE_IDEAS_LEDGER.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | research-backlog |
| `research/adversarial_architecture_review.md` | research-reference | A2 | reference | 2026-07-07 | 2026-07-07 | false | research-context |
| `research/architecture/ADR/ADR-0000-adr-system-and-knowledge-graph.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0001-operators-not-algorithms.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0002-two-state-backends.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0003-cache-locality-outranks-flops.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0004-reproducibility-mandatory.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0005-deterministic-certified-lowering.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0006-amendment-presumed-fixed.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0007-four-track-program.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0008-experiment-infrastructure.md` | architecture-decision | A1 | binding | 2026-07-14 | 2026-07-14 | true | architecture |
| `research/architecture/ADR/ADR-0009-addressable-randomness-for-counterfactuals.md` | architecture-decision | A2 | proposed | 2026-08-19 | 2026-08-19 | false | architecture |
| `research/architecture/ADR/ADR-0010-canonical-energy-entropy-v1.md` | architecture-decision | A2 | proposed | 2026-08-19 | 2026-08-19 | false | architecture |
| `research/architecture/ADR/ADR-0011-durable-project-memory.md` | architecture-decision | A1 | binding | 2026-08-26 | 2026-08-26 | true | architecture |
| `research/AUTONOMOUS_SCIENTIST_ARCHITECTURE.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/AXIOMS_OF_OPTIMIZATION.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/CHANNEL_EXHAUSTION.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/CHIEF_SCIENTIST_ASSESSMENT.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/COMPETITIVE_ANALYSIS.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/CONCEPT_DISCOVERY_AND_INTEGRATIONS.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/CONCEPT_EVOLUTION_CONSTITUTION.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/EXTERNAL_COMPARISON_PROTOCOL.md` | research-protocol | A0 | binding | 2026-08-22 | 2026-08-22 | true | external-comparison |
| `research/EXTERNAL_PROJECTS_BACKLOG.md` | research-reference | A2 | reference | 2026-08-20 | 2026-08-20 | false | research-context |
| `research/ISING_ENGINE_CONSTITUTION.md` | constitution | A0 | binding | 2026-07-14 | 2026-07-14 | true | project-direction |
| `research/OPTIMIZATION_ENGINE_BLUEPRINT.md` | research-reference | A2 | reference | 2026-07-14 | 2026-07-14 | false | research-context |
| `research/optimization_plan_rank135.md` | research-reference | A2 | reference | 2026-07-07 | 2026-07-07 | false | research-context |
| `research/PLATFORM_BLUEPRINT.md` | research-reference | A2 | reference | 2026-07-14 | 2026-07-14 | false | research-context |
| `research/PREREG_RC014_AMENDMENT_1.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC014 |
| `research/PREREG_RC014_AMENDMENT_2.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC014 |
| `research/PREREG_RC014.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC014 |
| `research/PREREG_RC015_AMENDMENT_1.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC015 |
| `research/PREREG_RC015.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC015 |
| `research/PREREG_RC016_AMENDMENT_1.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/PREREG_RC016_AMENDMENT_2.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/PREREG_RC016_AMENDMENT_3.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/PREREG_RC016_AMENDMENT_4.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/PREREG_RC016_DESCENDANT.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/PREREG_RC016.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/PREREG_RC017.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC017 |
| `research/PREREG_RC018_COST_IDENTIFICATION.md` | research-protocol | A0 | binding | 2026-08-19 | 2026-08-19 | true | RC018 |
| `research/PREREG_RC020_AMENDMENT_1.md` | research-protocol | A0 | binding | 2026-08-20 | 2026-08-20 | true | RC020 |
| `research/PREREG_RC020_MARGINAL_WALL_COST.md` | research-protocol | A0 | binding | 2026-08-20 | 2026-08-20 | true | RC020 |
| `research/PREREG_RC021_AMENDMENT_1.md` | research-protocol | A0 | binding | 2026-08-23 | 2026-08-23 | true | RC021 |
| `research/PREREG_RC021_AMENDMENT_2.md` | research-protocol | A0 | binding | 2026-08-25 | 2026-08-25 | true | RC021 |
| `research/PREREG_RC021_HOST_INSTRUMENT.md` | research-protocol | A0 | binding | 2026-08-23 | 2026-08-23 | true | RC021 |
| `research/RC001_ENSEMBLE_THERMOSTAT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC001 |
| `research/RC002_INITIALIZATION_ERASURE.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC002 |
| `research/RC003_ARCHITECTURE_AUDIT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC003 |
| `research/RC004_ARCHITECTURE_SPACE.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC004 |
| `research/RC021_C10_DIAGNOSIS.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | RC021-C10 |
| `research/BENCHMARK_DEGENERACY_AUDIT.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | benchmark-capability |
| `research/PERF_INCREMENTAL_RELINK.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | wall-time-protocol |
| `research/RC022_ARCHITECTURE_CENSUS.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | RC022 |
| `research/PREREG_RC023_MEMORY_FORM.md` | preregistration | A1 | binding | 2026-08-30 | 2026-08-30 | true | RC023 |
| `research/PREREG_RC024_PATH_RELINKING.md` | preregistration | A1 | binding | 2026-08-30 | 2026-08-30 | true | RC024 |
| `research/PREREG_RC025_CORPUS_OR_MECHANISM.md` | preregistration | A1 | binding | 2026-08-30 | 2026-08-30 | true | RC025 |
| `research/PREREG_RC025_AMENDMENT_1.md` | preregistration | A1 | binding | 2026-08-30 | 2026-08-30 | true | RC025 |
| `research/PREREG_RC026_EMBEDDING_LADDER.md` | preregistration | A1 | binding | 2026-08-30 | 2026-08-30 | true | RC026 |
| `research/PREREG_RC027_BUDGET_AXIS.md` | preregistration | A1 | binding | 2026-08-30 | 2026-08-30 | true | RC027 |
| `research/RC005_COST_MODEL_BLINDNESS.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC005 |
| `research/RC006_GRADIENT_LEDGER.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC006 |
| `research/RC007_OPERATOR_COMMUTATIVITY.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC007 |
| `research/RC008_MIXING_AND_ENSEMBLE_COLLAPSE.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC008 |
| `research/RC009_BACKEND_PASSPORT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC009 |
| `research/RC010_WORLD_MODEL_AUDIT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC010 |
| `research/RC011_PREDICTOR_METRIC_INVARIANCE.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC011 |
| `research/RC012_DYNAMICS_EARLY_STOP_AUDIT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC012 |
| `research/RC013_KERNEL_FLOORS.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC013 |
| `research/RC014_COUNTERFACTUAL_SUBSTITUTION.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC014 |
| `research/RC014_PHASE0_AUDIT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC014 |
| `research/RC014_PLAN_CAUSAL_INSTRUMENT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC014 |
| `research/RC015_TIE_HANDLING.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC015 |
| `research/RC016_CYCLE_RECORD.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/RC016_NOVELTY_REVIEW.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC016 |
| `research/RC017_ABORT_RECORD.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC017 |
| `research/RC017_ENTROPY_PILOT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC017 |
| `research/RC018_PILOT_ABORT_RECORD.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-19 | true | RC018 |
| `research/RC019_INSTRUMENT_CONFORMANCE_AUDIT.md` | research-record | A1 | closed | 2026-08-19 | 2026-08-20 | true | RC019 |
| `research/RC020_PILOT_ABORT_RECORD.md` | research-record | A1 | closed | 2026-08-22 | 2026-08-22 | true | RC020 |
| `research/RC021_INSTRUMENT_INVALID_RECORD.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | RC021 |
| `research/RELATIONAL_PRIMITIVE.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/RESEARCH_GAPS.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/RESEARCH_HISTORY.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/RESEARCH_INVENTORY.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-26 | false | research-context |
| `research/ROADMAP_AUTONOMOUS_SCIENTIST.md` | research-reference | A2 | reference | 2026-08-19 | 2026-08-19 | false | research-context |
| `research/STAGE_7_RESEARCH_PLATFORM.md` | research-reference | A2 | reference | 2026-07-14 | 2026-07-14 | false | research-context |
| `research/STAGE_8_KNOWLEDGE_OS.md` | research-reference | A2 | reference | 2026-07-14 | 2026-07-14 | false | research-context |
| `research/SYNTHESIS_CD_SOLVER_ARCHITECTURE.md` | design-spec | A4 | historical | 2026-09-19 | 2026-09-25 | false | none |
| `results/rc023/RESULT.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | RC023 |
| `results/rc024/RESULT.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | RC024 |
| `results/rc025/RESULT.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | RC025 |
| `results/rc026/RESULT.md` | research-record | A1 | closed | 2026-08-30 | 2026-08-30 | true | RC026 |
| `ROADMAP.md` | stage-inventory | A1 | active | 2026-07-05 | 2026-09-26 | false | implementation-status |
| `SECURITY.md` | engineering-reference | A1 | active | 2026-03-11 | 2026-03-11 | false | engineering |
| `SKILL.md` | reference | A3 | reference | 2026-03-11 | 2026-03-11 | false | none |
| `SOUL.md` | mission | A0 | active | 2026-07-15 | 2026-07-15 | false | project-mission |
| `START_HERE.md` | navigation | A0 | active | 2026-08-26 | 2026-09-27 | false | startup |
| `VERIFY.md` | engineering-reference | A1 | active | 2026-07-05 | 2026-07-05 | false | engineering |
| `website/AGENTS.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/ARCHITECTURE_REVIEW.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/ARCHITECTURE_V2.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/ARCHITECTURE_V3_ADDENDUM.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/CLAUDE.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/PRODUCT_SPEC.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/README.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/references/analysis.md` | publication | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `website/.claude/skills/run-ising-website/SKILL.md` | publication-tool | A3 | reference | 2026-08-19 | 2026-08-19 | false | website |
| `YC_APPLICATION_FALL_2026.md` | reference | A3 | reference | 2026-08-19 | 2026-08-19 | false | none |

## Maintenance rule

Every added, moved, or lifecycle-changing Markdown file updates this catalogue in the same commit. Automated validation compares this table with `rg --files -g '*.md'`; missing and duplicate paths fail the gate.
| `research/breakthrough/AUDIT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/EXP001_PROTOCOL.md` | research-protocol | A1 | binding | 2026-09-11 | 2026-09-11 | true | breakthrough |
| `research/breakthrough/GAUGE_SCOPE.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/HYPOTHESES.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/RESEARCH_STATE.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h01/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h01/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h01/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h01/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h01/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h02/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h02/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h02/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h02/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h02/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h03/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h03/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h03/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h03/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h03/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h04/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h04/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h04/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h04/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h04/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h05/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h05/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h05/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h05/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h05/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h06/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h06/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h06/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h06/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h06/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h07/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h07/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h07/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h07/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h07/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h08/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h08/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h08/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h08/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h08/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h09/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h09/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h09/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h09/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h09/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h10/HYPOTHESIS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h10/IMPLEMENTATION.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h10/MATH.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h10/NEXT.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/h10/RESULTS.md` | research-protocol | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough |
| `research/breakthrough/HANDOFF.md` | research-handoff | A1 | reference | 2026-09-11 | 2026-09-11 | false | breakthrough-resume |
| `research/HYPODIVE_BUILDER_INTAKE.md` | research-intake | A2 | reference | 2026-09-17 | 2026-09-27 | false | CD001; ER-001 |
| `research/HYPODIVE_BUILDER_HANDOFF.md` | research-handoff | A2 | reference | 2026-09-17 | 2026-09-27 | false | CD001; ER-001 |
| `research/HYPODIVE_TRIAGE.md` | research-triage | A2 | reference | 2026-09-17 | 2026-09-27 | false | CD001; application-discovery; prospective-CD003-correction; GitHub-opportunity-triage; Graft-qualification; scientific-ecosystem-triage; GitHub-publication-correction |
| `research/EXP005_MARKETSPLIT_TRIAGE.md` | research-triage | A2 | reference | 2026-09-25 | 2026-09-25 | false | EXP-005 |
| `research/EXP006A_RAW_RECHECK.md` | prospective-audit | A2 | reference | 2026-09-26 | 2026-09-26 | false | EXP-TEN-006A-R |
| `research/PRIOR_ART_MATRIX.md` | research-reference | A2 | reference | 2026-09-17 | 2026-09-27 | false | CD001; application-discovery-primary-sources |
| `research/breakthrough/EXP001_RESULT.md` | research-result-draft | A2 | proposed | 2026-09-17 | 2026-09-17 | false | breakthrough-CD001 |
| `research/breakthrough/EXP002_PROTOCOL.md` | research-protocol-draft | A2 | proposed | 2026-09-17 | 2026-09-17 | false | breakthrough-CD001 |
| `research/breakthrough/cd002/HYPOTHESIS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD002 |
| `research/breakthrough/cd002/KILL_TEST.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD002 |
| `research/breakthrough/cd002/MATH.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD002 |
| `research/breakthrough/cd002/NEXT.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD002 |
| `research/breakthrough/cd002/PRIOR_ART.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD002 |
| `research/breakthrough/cd002/RESULTS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD002 |
| `research/breakthrough/cd003/HYPOTHESIS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD003 |
| `research/breakthrough/cd003/KILL_TEST.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD003 |
| `research/breakthrough/cd003/MATH.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD003 |
| `research/breakthrough/cd003/NEXT.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD003 |
| `research/breakthrough/cd003/PRIOR_ART.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD003 |
| `research/breakthrough/cd003/RESULTS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD003 |
| `research/breakthrough/cd003/APPLICABILITY_TRIAGE.md` | research-triage | A2 | reference | 2026-09-25 | 2026-09-25 | false | CD003-applicability |
| `research/breakthrough/cd004/HYPOTHESIS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD004 |
| `research/breakthrough/cd004/KILL_TEST.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD004 |
| `research/breakthrough/cd004/MATH.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD004 |
| `research/breakthrough/cd004/NEXT.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD004 |
| `research/breakthrough/cd004/PRIOR_ART.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD004 |
| `research/breakthrough/cd004/RESULTS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD004 |
| `research/breakthrough/cd004_recheck/AMENDMENT_1.md` | research-protocol-amendment | A2 | binding | 2026-09-25 | 2026-09-25 | true | breakthrough-CD004-R |
| `research/breakthrough/cd004_recheck/HYPODIVE_BUILDER_HANDOFF.md` | research-handoff | A2 | reference | 2026-09-25 | 2026-09-25 | false | breakthrough-CD004-R |
| `research/breakthrough/cd004_recheck/HYPODIVE_BUILDER_INTAKE.md` | research-intake | A2 | reference | 2026-09-25 | 2026-09-25 | false | breakthrough-CD004-R |
| `research/breakthrough/cd004_recheck/PROTOCOL.md` | research-protocol | A2 | binding | 2026-09-25 | 2026-09-25 | true | breakthrough-CD004-R |
| `research/breakthrough/cd004_recheck/RESULT.md` | research-record | A2 | closed | 2026-09-25 | 2026-09-25 | true | breakthrough-CD004-R |
| `research/breakthrough/cd005/HYPOTHESIS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD005 |
| `research/breakthrough/cd005/KILL_TEST.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD005 |
| `research/breakthrough/cd005/MATH.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD005 |
| `research/breakthrough/cd005/NEXT.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD005 |
| `research/breakthrough/cd005/PRIOR_ART.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD005 |
| `research/breakthrough/cd005/RESULTS.md` | research-reference | A2 | reference | 2026-09-19 | 2026-09-19 | false | breakthrough-CD005 |
| `research/cd005_equal_time/AMENDMENT_1.md` | research-protocol-amendment | A2 | binding | 2026-09-25 | 2026-09-25 | true | CD005-Q2 |
| `research/cd005_equal_time/HYPODIVE_BUILDER_HANDOFF.md` | research-handoff | A2 | reference | 2026-09-25 | 2026-09-25 | false | CD005-Q2 |
| `research/cd005_equal_time/HYPODIVE_BUILDER_INTAKE.md` | research-intake | A2 | reference | 2026-09-25 | 2026-09-25 | false | CD005-Q |
| `research/cd005_equal_time/PROTOCOL.md` | research-protocol | A2 | binding | 2026-09-25 | 2026-09-25 | true | CD005-Q |
| `research/cd005_equal_time/RESULT.md` | research-result | A1 | closed | 2026-09-25 | 2026-09-25 | true | CD005-Q2 |
| `research/experiments/exp007_weighted_mis/protocol.md` | research-protocol | A1 | binding | 2026-09-25 | 2026-09-25 | true | EXP-007W |
| `research/experiments/cd003_market_residual/protocol.md` | research-protocol | A1 | binding | 2026-09-25 | 2026-09-25 | true | CD003-MR1 |
| `research/experiments/cd003_market_residual/RESULT.md` | research-result | A1 | closed | 2026-09-25 | 2026-09-25 | true | CD003-MR1 |
| `research/experiments/cd003_market_residual/results/run001/README.md` | run-guide | A3 | reference | 2026-09-25 | 2026-09-25 | false | CD003-MR1 |
| `research/experiments/exp007_weighted_mis/HYPODIVE_BUILDER_HANDOFF.md` | research-handoff | A2 | reference | 2026-09-25 | 2026-09-25 | false | EXP-007W |
| `research/experiments/exp007_weighted_mis/RESULT.md` | research-result | A1 | closed | 2026-09-25 | 2026-09-25 | true | EXP-007W |
| `research/experiments/exp007_weighted_mis/results/run001/README.md` | run-guide | A3 | reference | 2026-09-25 | 2026-09-25 | false | EXP-007W |
| `research/breakthrough/h11/HYPOTHESIS.md` | research-reference | A2 | reference | 2026-09-17 | 2026-09-17 | false | breakthrough-CD001 |
| `research/breakthrough/h11/MATH.md` | research-reference | A2 | reference | 2026-09-17 | 2026-09-17 | false | breakthrough-CD001 |
| `research/breakthrough/h11/IMPLEMENTATION.md` | research-reference | A2 | reference | 2026-09-17 | 2026-09-17 | false | breakthrough-CD001 |
| `research/breakthrough/h11/RESULTS.md` | research-reference | A2 | reference | 2026-09-17 | 2026-09-17 | false | breakthrough-CD001 |
| `research/breakthrough/h11/NEXT.md` | research-reference | A2 | reference | 2026-09-17 | 2026-09-17 | false | breakthrough-CD001 |
| `research/laya_semantic/PROTOCOL.md` | research-protocol | A2 | binding | 2026-09-23 | 2026-09-23 | true | LAYA-001 |
| `research/laya_semantic/HYPODIVE_BUILDER_INTAKE.md` | research-intake | A2 | reference | 2026-09-23 | 2026-09-23 | false | LAYA-001 |
| `research/laya_semantic/README.md` | research-guide | A2 | reference | 2026-09-23 | 2026-09-27 | false | LAYA-001 |
| `research/laya_semantic/RESULT.md` | research-result | A2 | reference | 2026-09-23 | 2026-09-23 | false | LAYA-001 |
| `research/laya_semantic/HYPODIVE_BUILDER_HANDOFF.md` | research-handoff | A2 | reference | 2026-09-23 | 2026-09-23 | false | LAYA-001 |
| `research/labs_qualification/PROTOCOL.md` | research-protocol | A2 | binding | 2026-09-23 | 2026-09-23 | true | LABS-Q001 |
| `research/labs_qualification/HYPODIVE_BUILDER_INTAKE.md` | research-intake | A2 | reference | 2026-09-23 | 2026-09-23 | false | LABS-Q001 |
| `research/labs_qualification/README.md` | research-guide | A2 | reference | 2026-09-23 | 2026-09-23 | false | LABS-Q001 |
| `research/labs_qualification/RESULT.md` | research-result | A2 | reference | 2026-09-23 | 2026-09-23 | false | LABS-Q001 |

## Discovered local documents (2026-09-25 audit)

These untracked documents have no Git chronology yet; `—` is intentional.
Their listing records existence, not scientific acceptance.

| Path | Kind | Authority | Lifecycle | Created | Last changed | Immutable | Canonical scope |
|---|---|---:|---|---|---|---|---|
| `.agents/agents/benchmark-engineer.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/agents/falsification-agent.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/agents/literature-researcher.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/agents/paper-reviewer.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/agents/reproducibility-auditor.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/agents/research-lead.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/agents/solver-engineer.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/agents/theory-reviewer.md` | agent-role | A3 | reference | — | — | false | agent-tooling |
| `.agents/skills/claim-audit/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/experimental-methodology/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/paper-writing/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/performance-engineering/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/qoblib-benchmark/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/reproducibility/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/research-literature/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/solver-validation/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `.agents/skills/statistics/SKILL.md` | research-skill | A3 | reference | — | — | false | research-method |
| `DECISIONS.md` | navigation-pointer | A3 | reference | 2026-09-25 | 2026-09-25 | false | navigation |
| `NOW.md` | navigation-pointer | A3 | reference | 2026-09-25 | 2026-09-25 | false | navigation |
| `PROJECTS.md` | project-map | A1 | active | 2026-09-25 | 2026-09-26 | false | project-navigation |
| `RESEARCH.md` | navigation-pointer | A3 | reference | 2026-09-25 | 2026-09-25 | false | navigation |
| `research/CLAIMS.md` | research-local | A2 | proposed | — | 2026-09-27 | false | research-triage; benchmark-and-tuner-corrections |
| `research/FAILED_IDEAS.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/ISING_ENGINE_RESEARCH_AUDIT.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/KNOWLEDGE_BASE.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/LITERATURE_MAP.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/NEXT_10_EXPERIMENTS.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/OPEN_QUESTIONS.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/PROJECT_STATE.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/RESEARCH_LOOP.md` | research-local | A2 | proposed | — | — | false | research-triage |
| `research/fundamental_ai/ANOMALIES.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXPERIMENT_PROTOCOL.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_001_RESULT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_002_PROTOCOL.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_002_RESULT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_003_PROTOCOL.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_003_RESULT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_004_PROTOCOL.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_004_RESULT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_005_PROTOCOL.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_005_RESULT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006B_PROTOCOL.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006B_RESULT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006_NOVELTY_REVIEW.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006_PRIOR_ART_RESET.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006_PROTOCOL.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006_RESULT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006_STABILITY_THEORY.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/EXP_TEN_006_THEORY.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/HYPOTHESES.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/MATH.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/NEGATIVE_RESULTS.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/NEXT.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/NOVELTY_LEDGER.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/PRIOR_ART.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/README.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/RESEARCH_GRAPH.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `research/fundamental_ai/THEORY_SIGNAL_CROSSTALK.md` | research-local | A2 | proposed | — | — | false | fundamental-ai |
| `results/rc027/RESULT.md` | research-result-local | A2 | proposed | — | — | false | RC027 |

| `research/EXTERNAL_COMPARISON_AMENDMENT_1.md` | research-protocol | A0 | binding | 2026-09-27 | 2026-09-27 | true | MQ-QUAL-001; external-comparison-handoff |

| `research/mqlib_qualification/README.md` | adapter-qualification | A2 | reference | 2026-09-27 | 2026-09-27 | false | MQ-QUAL-001 |

| `research/mqlib_qualification/run001/README.md` | run-guide | A3 | reference | 2026-09-27 | 2026-09-27 | false | MQ-QUAL-001-run001 |

| `research/experiments/mqlib_screen/protocol.md` | preregistration | A0 | binding | 2026-09-27 | 2026-09-27 | true | MQ-SCREEN-001; narrow-external-amendment |

| `research/experiments/mqlib_screen/smoke001/README.md` | run-guide | A3 | reference | 2026-09-27 | 2026-09-27 | false | MQ-SCREEN-001-smoke001 |

| `research/experiments/mqlib_screen/run001/README.md` | run-guide | A3 | reference | 2026-09-27 | 2026-09-27 | false | MQ-SCREEN-001-run001 |

| `research/experiments/mqlib_screen/RESULT.md` | research-result | A1 | closed | 2026-09-27 | 2026-09-27 | true | MQ-SCREEN-001 |

| `research/experiments/mqlib_screen/NEXT_DESIGN.md` | research-design | A2 | proposed | 2026-09-27 | 2026-09-27 | false | MQ-DESIGN-002 |

| `research/experiments/mqlib_timing_calibration/protocol.md` | preregistration | A0 | binding | 2026-09-27 | 2026-09-27 | true | MQ-CAL-001; no-search-instrument-only |

| `research/experiments/mqlib_timing_calibration/smoke001/README.md` | run-guide | A3 | reference | 2026-09-27 | 2026-09-27 | false | MQ-CAL-001-smoke001 |

| `research/experiments/mqlib_timing_calibration/run001/README.md` | run-guide | A3 | reference | 2026-09-27 | 2026-09-27 | false | MQ-CAL-001-run001 |

| `research/experiments/mqlib_timing_calibration/RESULT.md` | research-result | A1 | closed | 2026-09-27 | 2026-09-27 | true | MQ-CAL-001 |
