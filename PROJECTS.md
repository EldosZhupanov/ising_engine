# Project and evidence map

The initial sweep inspected the working tree on 2026-09-25 at `11418e3`;
the preservation state below was updated on 2026-09-26.
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
| RESEARCH — LABS | `research/labs_qualification/`, `src/bin/labs_record_hunter.rs`, `benchmarks/qoblib/world_records/` | The frozen PT qualification is REPRODUCED as NOT QUALIFIED versus lMAts at 10 s. Eight checkpoints are now tracked; `check_labs` parsed them and independent autocorrelation sums reproduce E=289, 318, 310, 319, 355, 360, 372, 357 for N=67…74. Its optimum table ends before these sizes, so exit 0 is **not an optimality certificate**. N=74 E=357 remains above the listed 341. Record-reaching probability is UNKNOWN. | Qualify the later memetic hunter against lMAts on an independent equal-time campaign before a record run. |
| RESEARCH — Market Split | `research/EXP005_MARKETSPLIT_TRIAGE.md`, `src/core/lattice.rs`, four `qoblib_*` prototype binaries | The claim of 41 first-known solutions is FALSIFIED: all 41 preserved inputs contain checker-valid Boolean answers. LLL/hybrid code is tracked as EXPERIMENTAL at `cffaee3`; extracted kernel completeness and solver advantage are unproven. Five checker-rejected `.sol` files are preserved under `rejected_candidates/`. Candidate promotion now requires the checker; empty kernels and malformed timeouts have regression guards. An independent review identified architecture debt: lattice preprocessing/QUBO encoding reside in `core/`, and substantial search math resides in `bin/`, contrary to AGENTS.md boundaries. This snapshot is not a production architecture. | Before production integration, separate pure models, compiler lowering and solver math; prove arithmetic/kernel invariants. Then run a blind `(A,b)` comparison with preprocessing inside budget and a specialist baseline. |
| RESEARCH — Laya | `research/laya_semantic/` | REPRODUCED synthetic pilot; all 24 residuals were fully fixed by presolve. General semantic benefit on real annotated data is UNKNOWN. | External annotated corpus with nonempty residual graphs. |
| RESEARCH — fundamental AI | `research/fundamental_ai/` | Separate Rust crate and raw corpus preserved byte-for-byte at `004385e`; tests pass. Several model-novelty and discrete-synchronization hypotheses are recorded as FALSIFIED locally. [EXP-TEN-006A-R recheck](research/EXP006A_RAW_RECHECK.md) reproduces all 3,360 non-timing rows from current source, narrows the classical win to partial bit accuracy, and finds 0/20 exact solves at K=16 for every method. Historical September 12 source freeze is UNKNOWN. | Audit remaining raw/protocol chronology and external baselines before promoting claims or code. |
| RESEARCH — RC/CD cycles | `research/RC*.md`, `research/breakthrough/`, `results/` | Mixed REPRODUCED, FALSIFIED and INCONCLUSIVE. [memory/RESEARCH.md](memory/RESEARCH.md) and the immutable result records govern the claim, not later summaries. | Reopen a failed result only with a new assumption and protocol. |
| MODELS | `src/engine_v2/ai_scientist/`, `research/fundamental_ai/src/`, `research/laya_semantic/` | Model code and pilot results exist; no tracked model-weight directory or supported general foundation model was found. Any installed local Laya weights live outside this repo. | Record model version and dataset provenance before inference claims. |
| EXPERIMENTS | `experiments/`, `research/experiments/`, `research/breakthrough/exp001/` | `experiments/` is Git-ignored generated platform state but includes ~55 MB of reports/data. EXP001 raw and target hashes match its result record; replay from raw reproduced four output files byte-for-byte after its corpus was committed at `31de138`. Binding protocols remain immutable. | Preserve significant ignored runs with manifests before treating them as durable evidence. |
| RESULTS | `results/rc027/`, QOBLIB submissions, `benchmarks/qoblib/world_records/` | RC027 raw and stderr hashes match its now-tracked result at `4435ae9`; the recorded instrument commit contains a transcription error, while the actual commit's source hash matches. QOBLIB checkpoint validity and scientific novelty are separate questions. | Cite the RC027 correction prospectively and verify each submission against its exact instance. |
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

The first REPO-002 raw-data check counted 3,360 data rows plus one header in
`research/fundamental_ai/EXP_TEN_006_AUDIT_RAW.tsv`, while
`EXP_TEN_006_NOVELTY_REVIEW.md` says 4,410 evaluations. For the exact slice
`cycle_k=16`, `noise=0.20`, `step=16`, 20 seeds per method, recomputed means of
`mean_accuracy` are 0.7965 for `Candidate_LearnedEnergy`, 0.8936 for
`SYNC1_Spectral_Sync`, and 0.9510 for `SYNC5_Loopy_MinSum` (rounded to four
decimals). The qualitative ordering survives this check; the reported count
does not. The review Markdown also contains damaged formula/percentage text.
Treat its exact printed statistics as INCONCLUSIVE until independently
recomputed. The [prospective recheck](research/EXP006A_RAW_RECHECK.md) records
the source replay, exact-solve counts and protocol limits. The raw TSV is
tracked at `004385e` and has a SHA-256 row in `FILE_MAP.tsv`.
