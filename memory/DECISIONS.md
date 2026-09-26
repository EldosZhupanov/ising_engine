# memory/DECISIONS.md — pointer

**Authoritative source:** `../research/architecture/ADR/` — read the ADR itself
before any architectural work. Query the graph with
`python ../research/architecture/query_graph.py`.

Record a new ADR when you change structure, a boundary, or a cross-cutting
guarantee. An ADR states: context, the decision, alternatives considered,
consequences, and links to superseded/related ADRs.

## Current research dispositions (not architecture ADRs)

| Decision | Why | Evidence |
|---|---|---|
| Treat the selected 41 Market Split files as known-feasible benchmarks, never first-known-solution targets. | Official `.dat` comments contain 41 checker-valid binary witnesses. | [EXP-005 triage](../research/EXP005_MARKETSPLIT_TRIAGE.md) |
| Keep lattice and `fundamental_ai` work experimental after preservation commits. | Tracking and passing tests do not establish kernel completeness, algorithmic advantage, or the historical source freeze. | [Project map](../PROJECTS.md), [current state](NOW.md) |
| Do not promote the retained lattice prototype to production architecture. | Independent review found LLL/preprocessing and QUBO encoding in `core/`, plus search math in `bin/`, contrary to AGENTS.md module boundaries. Preservation does not authorize this structure as canonical. A scoped refactor and correctness proof are required before integration. | `src/core/lattice.rs`, four `src/bin/qoblib_*` prototypes, [roadmap](../ROADMAP.md) |
| Preserve checker-rejected candidates away from submission directories. | A `.sol` filename alone is not a certificate; five local candidates fail the checker, including `ms_13_050_003` with 13/13 violated rows. | `benchmarks/qoblib/marketsplit/rejected_candidates/verification.txt`, [project map](../PROJECTS.md) |
| Require a successful local QOBLIB checker before an experimental Market Split binary promotes or announces a solution. | The prior four binaries could write to `unsolved_solutions/` and print `SOLVED` without checking the verifier exit status. | `src/bin/common/marketsplit_certificate.rs`, its pass/fail/missing-checker tests |
| Preserve original research and public-input bytes, including old whitespace, with narrowly scoped Git attributes. | Changing imported text would invalidate retained SHA-256 evidence and obscure the damaged formulas that the audit records. | `research/fundamental_ai/.gitattributes`, `research/breakthrough/exp001/.gitattributes`, `benchmarks/qoblib/marketsplit/unsolved_instances/.gitattributes` |
| Treat LABS checker exit 0 above N=66 as an energy and format check, not proof of optimality. | Its optimum table ends before the eight preserved N=67…74 checkpoints; independent autocorrelation energy agrees, including N=74 E=357. | [Project map](../PROJECTS.md), `benchmarks/qoblib/check_labs.rs` |
| Give each LABS process its own output directory and publish a checkpoint before announcing a new incumbent. | The old atomic-energy update could race with the separately locked sequence/file write. A process-local mutex now covers verified atomic checkpoint publication; it does not synchronize independent processes. | `66e9934`, `src/bin/labs_record_hunter.rs` regression tests |
| Keep HUBO-C001 as a scoped representation result, not a solver-superiority claim. | MSC native won both quadratic contrasts but tied native OpenJij in all 100 paired cells; the registered conjunction failed. A new measurement must discriminate native methods and control penalties/schedules before further algorithm work. | [HUBO-C001 result](../research/experiments/hubo_comparison/RESULT.md), raw `a196745` |
| Separate HUBO representation correctness from search advantage, and Laya feasibility from training novelty. | User selected this research direction on 2026-09-27; prior EXP001 naming conflated two tasks. An exact transform or trainable model is not evidence of competitive improvement. | [HUBO-RG001](../research/experiments/hubo_representation_gate/RESULT.md), [Laya assessment](../research/laya_semantic/README.md) |
| Do not recommend a long LABS record campaign from the current memetic hunter's evidence. | The registered Q002 screen failed at all three lengths; 0 paired wins, 4 ties and 26 losses against the retained specialist at ten seconds. A new diagnostic question must precede more operators or larger budgets. | [LABS-Q002 result](../research/experiments/labs_q002/RESULT.md), raw commit `7449724` |
| Correct the RC027 instrument identifier prospectively. | The literal hash in the result cannot be resolved, but the actual commit's source SHA-256 matches. | [RC027 result](../results/rc027/RESULT.md), `fdec0dfdf245c32925d71ffd745a34b7357cbaa4` |

## The twelve, in one line each

| ADR | Decision |
|---|---|
| [0000](../research/architecture/ADR/ADR-0000-adr-system-and-knowledge-graph.md) | Architecture Decision Records with typed knowledge-graph edges |
| [0001](../research/architecture/ADR/ADR-0001-operators-not-algorithms.md) | The engine is built from **Operators, not Algorithms** |
| [0002](../research/architecture/ADR/ADR-0002-two-state-backends.md) | Two state backends (DenseByte, SparseBitSlice) behind one interface |
| [0003](../research/architecture/ADR/ADR-0003-cache-locality-outranks-flops.md) | **Cache locality outranks FLOPS** in all layout and kernel decisions |
| [0004](../research/architecture/ADR/ADR-0004-reproducibility-mandatory.md) | **Bit-identical reproducibility is mandatory, not optional** |
| [0005](../research/architecture/ADR/ADR-0005-deterministic-certified-lowering.md) | Lowering passes are deterministic and emit energy-preservation certificates |
| [0006](../research/architecture/ADR/ADR-0006-amendment-presumed-fixed.md) | Amendment I: the architectural vector is presumed fixed, not dogmatically fixed |
| [0007](../research/architecture/ADR/ADR-0007-four-track-program.md) | Four parallel tracks: Engine, Research, Benchmark, Verification |
| [0008](../research/architecture/ADR/ADR-0008-experiment-infrastructure.md) | Experiment infrastructure — one command, full report, black-box engines |
| [0009](../research/architecture/ADR/ADR-0009-addressable-randomness-for-counterfactuals.md) | **Proposed:** addressable randomness for counterfactual execution |
| [0010](../research/architecture/ADR/ADR-0010-canonical-energy-entropy-v1.md) | **Proposed:** canonical energy-entropy representation v1 |
| [0011](../research/architecture/ADR/ADR-0011-durable-project-memory.md) | Git-backed durable memory with one live state and scoped authority |

## The two that bite most often

**ADR-0004 (reproducibility).** Same seed ⇒ same trajectory ⇒ same energy, and
every adaptive feature is opt-in and bit-identical on replay. This is why
FP contraction (`mul_add`), reduced-precision RNG comparisons, and reordered
float sums are **rejected by default** — they change trajectories, so they are
behaviour-changing and need explicit approval, no matter how much faster they are.

**ADR-0001 (operators, not algorithms).** Operators are selected through their
capability passport, never by name, so a new operator is usable the moment it is
registered. Its corollary is easy to miss: a capability with **no implementing
operator can never be selected**, and will stay empty silently. Three of six
capabilities were in that state for the entire history of the project —
`ExactInference`, `Approximate` and `Warmstart` still have zero implementations.
