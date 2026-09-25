# Project and evidence map

This map covers the working tree as inspected on 2026-09-25 at `11418e3`.
`CURRENT` means present and usable in this checkout; it does not mean superior
performance. `UNKNOWN` means the evidence named here is insufficient. The
per-file Markdown authority/lifecycle is in [memory/CATALOG.md](memory/CATALOG.md);
the tracked, untracked, and selected ignored-evidence file inventory is
`memory/FILE_MAP.tsv` (`python3 scripts/build_file_map.py > memory/FILE_MAP.tsv`).
Git remains the chronology; [memory/NOW.md](memory/NOW.md) is the only live task
authority.

| Area | Main paths | Status and evidence | Next gate |
|---|---|---|---|
| PROJECTS — production optimizer | `src/core/`, `src/solver/`, `src/presolve/`, `tests/` | CURRENT code: `cargo check` and `cargo test --release` pass in this checkout. Scalar and MSC families have separate state models. A passing test suite is not a performance claim. | Preserve API and energy invariants; compare changes at equal cost. |
| PROJECTS — research platform | `src/engine_v2/`, `research/RESEARCH_INVENTORY.md` | CURRENT research code; historical experiments include falsified mechanisms. No independently replicated equal-cost production win over `UltimateSolver` is established in [the research map](memory/RESEARCH.md). | Follow [PROJECT_PLAN.md](PROJECT_PLAN.md) gates before product integration. |
| PROJECTS — website | `website/src/`, `website/package.json` | CURRENT tracked Next.js source; runtime/product claims UNKNOWN in this audit. `website/node_modules/` is generated. | Run its own build/typecheck before any website change. |
| BENCHMARKS — public corpus hub | `benchmark_suite/`, `benchmarks/`, `gset/` | CURRENT acquisition/parsers and QOBLIB checker code. `benchmark_suite/data/` and `external/` are ignored downloads/clones, not source-of-truth commits. | Pin source hashes and run independent checkers for every reported solution. |
| RESEARCH — LABS | `research/labs_qualification/`, `src/bin/labs_record_hunter.rs`, `benchmarks/qoblib/world_records/` | The frozen PT qualification is REPRODUCED as NOT QUALIFIED versus lMAts at 10 s. Eight local checkpoint files pass `check_labs`; N=74 has E=357 versus the listed best-known 341. The hunter/checkpoints are a separate untracked campaign, so record-reaching probability is UNKNOWN. | Qualify the later memetic hunter against lMAts on an independent equal-time campaign before a record run. |
| RESEARCH — Market Split | `research/EXP005_MARKETSPLIT_TRIAGE.md`, `src/core/lattice.rs`, four untracked `qoblib_*` binaries | The claim of 41 first-known solutions is FALSIFIED: official input files contain validated Boolean answers. LLL/hybrid code is EXPERIMENTAL and uncommitted. Five checker-rejected `.sol` files are preserved under `rejected_candidates/`; see the table below. | Blind `(A,b)` comparison with preprocessing inside budget, strong specialized baseline and ablation. |
| RESEARCH — Laya | `research/laya_semantic/` | REPRODUCED synthetic pilot; all 24 residuals were fully fixed by presolve. General semantic benefit on real annotated data is UNKNOWN. | External annotated corpus with nonempty residual graphs. |
| RESEARCH — fundamental AI | `research/fundamental_ai/` | Separate untracked Rust crate; its tests pass in this checkout. Several model-novelty and discrete-synchronization hypotheses are recorded as FALSIFIED locally. The track is IN PROGRESS, not part of the main Cargo workspace. | Audit raw/protocol chronology and external baselines before promoting claims or code. |
| RESEARCH — RC/CD cycles | `research/RC*.md`, `research/breakthrough/`, `results/` | Mixed REPRODUCED, FALSIFIED and INCONCLUSIVE. [memory/RESEARCH.md](memory/RESEARCH.md) and the immutable result records govern the claim, not later summaries. | Reopen a failed result only with a new assumption and protocol. |
| MODELS | `src/engine_v2/ai_scientist/`, `research/fundamental_ai/src/`, `research/laya_semantic/` | Model code and pilot results exist; no tracked model-weight directory or supported general foundation model was found. Any installed local Laya weights live outside this repo. | Record model version and dataset provenance before inference claims. |
| EXPERIMENTS | `experiments/`, `research/experiments/`, `research/breakthrough/exp001/` | `experiments/` is Git-ignored generated platform state but includes ~55 MB of reports/data. EXP001 raw and target hashes match its result record. Binding protocols remain immutable. | Preserve significant ignored runs with manifests before treating them as durable evidence. |
| RESULTS | `results/rc027/`, QOBLIB submissions, `benchmarks/qoblib/world_records/` | RC027 raw and stderr hashes match its result; the recorded instrument commit contains a transcription error, while the actual commit's source hash matches. QOBLIB checkpoint validity and scientific novelty are separate questions. | Record the RC027 correction prospectively and verify each submission against its exact instance. |
| ARCHIVE | `archive/`, historical pointers in `memory/`, earlier design specs at root | HISTORICAL. `archive/` is ignored and contains four text inventories plus one ELF binary; no contents were deleted. Old pointers are SUPERSEDED where [memory/CATALOG.md](memory/CATALOG.md) says so. | Keep for provenance; migrate only after a reference check. |
| UNKNOWN / NEEDS REVIEW | root scripts (`autopilot.py`, `dex_arbitrage_bridge.py`, `parser.py`, etc.), `config/`, incomplete `benchmarks/qoblib/upstream/` clone | UNKNOWN or LEGACY until execution path and provenance are checked. The upstream clone has a `.git/shallow.lock` and no verified working tree; it is not a benchmark source. | Inspect exact callers and provenance before deletion or promotion. |

The non-source bulk was counted, not interpreted as project code: 81,906 files
under `target/`, 49,036 under `website/node_modules/`, 13,537 in the Python
`benchmark-env/lib/`, and 9,479 under `benchmark_suite/external/`. All are
rebuildable or downloaded categories; important ignored research under
`experiments/` and `archive/` is called out above instead of being silently
discarded.

The file map marks 33 exact byte duplicates, including shared platform reports,
three duplicate EXP001 scripts, two benchmark adapter scripts, and copied G-Set
instances. Byte identity does not establish semantic redundancy, so no copy was
removed. Ignored `experiments/` reports and `archive/` files remain HISTORICAL
until their run provenance is verified.

The 2026-09-25 official-checker pass over the eleven files originally in
`benchmarks/qoblib/marketsplit/solutions/` accepted seven and rejected four.
The rejected files were moved intact to `rejected_candidates/` after a Git
reference check; they are **near-miss candidates, not solutions**.
Independent integer `Ax-b` calculations agree with the checker:

| Preserved candidate in `rejected_candidates/` | Residual vector `Ax-b` | Sum of squares |
|---|---|---:|
| `ms_04_050_001.sol` | `[0, 0, 0, -1]` | 1 |
| `ms_05_050_001.sol` | `[0, 0, 0, 1, 0]` | 1 |
| `ms_06_050_001.sol` | `[-2, 1, 1, -2, 1, -1]` | 12 |
| `ms_13_050_000.sol` | `[4, 3, -5, -8, -6, -4, 6, 4, -1, -1, -10, 2, -3]` | 333 |

Reproduce each rejection with
`target/release/check_marketsplit benchmarks/qoblib/marketsplit/instances/NAME.dat benchmarks/qoblib/marketsplit/rejected_candidates/NAME.sol`.
The four commands exit 21; the seven files left in `solutions/` exit 0. A `.sol`
suffix is never an evidence status.
