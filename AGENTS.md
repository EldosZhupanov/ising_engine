# AGENTS.md — Ising Engine engineering rules

Read [`START_HERE.md`](START_HERE.md) and [`memory/NOW.md`](memory/NOW.md)
before doing any work. Run `git status --short` first. Read only the task bundle
named by `NOW.md`; use targeted `rg` rather than scanning the repository.

This file contains stable engineering rules, not current status, test counts, or
roadmap percentages. `memory/AUTHORITY.md` defines which document governs each
question. If instructions conflict, this file wins for engineering workflow
unless the user explicitly overrides it in the current conversation. A binding
research protocol governs its scientific scope; an unresolved same-scope
conflict is a stop condition.

## 1. Decision hierarchy

Choose among valid implementations in this order:

1. correctness;
2. architectural boundaries;
3. measured performance;
4. public API stability;
5. readability and maintainability;
6. code size.

Never trade correctness for speed or readability. Never claim a performance
gain without measurements. Surface conflicts instead of silently choosing the
lower-priority concern.

## 2. Architectural invariants

### Solver families stay independent

| Family | Main files | Data model |
|---|---|---|
| Scalar | `parallel_tempering.rs`, `adaptive.rs`, `cluster.rs`, `tabu.rs` | `QuboModel` + `CsrMatrix`, per-replica state |
| MSC | `ultimate.rs`, `solver/engine.rs`, `solver/types.rs` | `HuboModel`/`FlatHuboModel` → `QuantumField`, replicas packed/vectorized |

- A change in `engine.rs`/`types.rs` must not require a scalar-family change,
  and vice versa.
- A task touching both families is presumed mis-scoped: stop and ask.
- Do not introduce `CsrMatrix` into the MSC family.

### Ownership

- `QuantumField` is the sole long-lived spin-state owner in MSC.
- `HuboModel` is the coefficient source of truth.
- `FlatHuboModel` is a read-only hot-path projection, built once and never
  mutated during a sweep.
- No other structure retains a spin copy longer than one `step()`.

### Module boundaries

- `core/`: pure models/data; no I/O, randomness, or rayon.
- `compiler/`: logic → QUBO penalties; no solver knowledge.
- `solver/`: Monte Carlo/PT/PA mathematics; no HTTP/JSON.
- `bin/`: orchestration only.
- `server_api.rs` calls `UltimateSolver::solve()`; changing that contract is an
  explicit architectural task.
- Every binary under `src/bin/` must continue to compile after public-type
  changes.

### Energy correctness

For Ising/HUBO terms, local flip deltas must satisfy:

`E(after via accumulated delta_e) == E(recomputed from scratch)`

within the documented float tolerance. A solver-math refactor requires a fixed-
seed before/after comparison on a small instance. Systematic divergence is a
bug, not acceptable Monte Carlo noise.

## 3. Context and search discipline

Use the cheapest source that can answer the question:

1. `START_HERE.md` → `memory/NOW.md` → named task bundle;
2. `git status` / `git diff`;
3. exact symbol with `rg`;
4. type/call-site search;
5. filename glob;
6. `cargo metadata`/`cargo tree` for dependency questions;
7. focused file ranges;
8. whole-file reading only when necessary.

Never recursively inspect generated/incidental data (`target/`, `.git/`, raw
criterion reports). If investigation reaches more than eight files, stop direct
reading and perform an explicit read-only dependency-map pass. Do not read every
Markdown file; use `memory/CATALOG.md` and scoped indexes.

Prefer a targeted patch to a rewrite. A full rewrite requires an incompatible
old structure and an explicit plan item. Preserve unrelated dirty-worktree
changes.

## 4. Roles and workflow

Every non-trivial task follows:

**Explore → Plan → Implement → Verify → Document → Report**

- **Explorer (read-only):** declarations, call sites, tests, binaries, file:line
  dependency map. Mandatory for public types used in three or more modules,
  deletions, and broad architectural work.
- **Planner:** atomic steps, edit order, tests, explicit do-not-touch scope.
  Show the plan before architectural work or work touching more than three
  files. More than fifteen steps means split the task.
- **Implementer:** smallest patch per plan; no “while I am here” changes.
- **Perf verifier:** mandatory for speed claims in `engine.rs`/`types.rs`.
- **Independent reviewer:** reads the finished diff from scratch and does not
  edit it. Any failed review dimension means Not Done.
- **Doc keeper:** updates `memory/NOW.md`, catalogue entries, and affected
  canonical documentation when facts change.

Task sizes:

- trivial: one local non-hot-path file;
- standard: two or three files or one-module public type;
- non-trivial: public type across modules, hot path, deletion;
- architectural: new invariant/dependency or §2 boundary change.

One logical step equals one commit. Do not mix rename/move with logic changes.
Do not commit a red tree. Keep the final state green.

## 5. Required quality gates

For any `src/` change:

```bash
cargo check
cargo test --release
cargo build --release --bins
cargo clippy --all-targets -- -D warnings
cargo fmt --check
git diff --check
```

Use workspace/all-target variants when the changed scope includes workspace
members. Changed public types require an Explorer-confirmed call-site update.

Additional gates:

- `core/hubo.rs`: `test_hubo_qpa`, `test_anls`, and energy-equivalence tests for
  new projections.
- `server_api.rs`: keep `num_vars` validation, CSR deduplication, semaphore
  backpressure, and the `UltimateSolver` entry point.
- `solver/`: solver integration tests and fixed-seed numerical compatibility.
- compiler changes: exhaustive gate/logic penalty tests.
- documentation-only: link/memory validation and `git diff --check`; no cargo
  unless executable docs/tooling changed.

Reviewer matrix — all apply:

| Dimension | Required question |
|---|---|
| Correctness | Does result/energy match the baseline? |
| Performance | Are claims backed by measurements? |
| Architecture | Are family, ownership, and module boundaries intact? |
| API | Are public signatures unchanged or explicitly authorized? |
| Memory | No new hot-loop allocation? |
| Safety | Every allowed `unsafe` has a local `// SAFETY:` proof? |
| Regression | Do all applicable tests pass? |
| Tests | Does new behavior have a non-vacuous regression test? |

## 6. Performance changes

No performance claim without all applicable evidence:

```bash
RUSTFLAGS='-C target-cpu=native -C llvm-args=-pass-remarks=loop-vectorize' \
  cargo build --release --lib 2>&1 | rg 'vectorized loop'
objdump -d target/release/libising_engine.rlib | rg -c 'ymm|zmm'
cargo bench --bench benchmark
```

Attach before/after numbers and environment. The scalar family must remain
untouched and green for MSC-only optimization. Update the benchmark baseline
only with real measurements.

Inside `step()`, `calculate_delta_e_local`, or any per-sweep hot loop:

- no `Vec::new`, `vec!`, owning clone, formatting, `String`, or captured heap
  closure per iteration;
- no `HashMap`/`HashSet` lookup;
- reuse constructor-allocated flat buffers;
- parallelize outer populations/cells, not replicas inside a cache line;
- keep energy accumulation in `f64` unless explicitly authorized and measured;
- prefer branchless/vector-friendly code only after profiling identifies the
  branch;
- `unsafe` is allowed only in `solver/engine.rs`, one small operation at a time,
  with a precise `// SAFETY:` invariant.

Common rejected patterns: variable shifts over replica bits, hidden allocation
inside `step()`, bitcasts that block vectorization, cross-family reuse, and
deleting “dead” files without a repository-wide reference check.

## 7. Refactoring and fixes

- Green → green atomicity: intermediate red steps may exist only inside one
  active series; never hand back or publish a red state.
- Rename/move → check → test → logic change. Split mixed commits.
- Before deleting a file, prove it is absent from module declarations, imports,
  binaries, tests, and build scripts. If a roadmap “dead” file is used, the
  roadmap is stale; do not delete it.
- Bug fixes require root cause, the violated invariant, and a test catching that
  cause. Otherwise report the fix as temporary.
- New functionality requires an appropriate unit/integration/system test. If it
  fits no existing level, decide the correct layer before implementation.
- Never weaken public scalar-solver APIs as incidental MSC work.

## 8. Escalation

Stop and present two or three options when:

- both solver families must change;
- `server_api.rs` public behavior must break;
- a math-preserving refactor changes fixed-seed energy/result;
- a new heavy dependency is required;
- a plan exceeds fifteen steps;
- allegedly dead code is referenced;
- a binding protocol, accepted ADR, and engineering rule cannot all be obeyed;
- confidence in a fact driving an irreversible action is Low.

Confidence labels:

- **High:** confirmed by a command/test/grep;
- **Medium:** confirmed by focused code reading;
- **Low:** structural guess. Raise it before acting.

## 9. Git and documentation discipline

- Use non-interactive Git; never discard user work with reset/checkout.
- Stage only the intended paths and inspect the staged diff.
- One logical plan step per commit; commit messages use
  `<area>: <imperative description>`.
- Binding preregistrations, amendments, and research records are immutable.
  Correct them prospectively with a linked amendment/record.
- `memory/NOW.md` is the sole live state. `ROADMAP.md` is inventory, not handoff.
- Added/moved Markdown updates `memory/CATALOG.md`; milestones update
  `memory/TIMELINE.md`.
- Stable instruction files contain no current test counts, branch state, or
  transient priorities.
- Obsidian may view the repository but must not auto-commit or create a second
  source of truth.

## 10. Final report

Keep it short and evidence-based:

```text
Changed: <paths and outcome>
Quality gates: <commands and exact results>
Architecture: intact / exact authorized exception
Remaining: <next item from memory/NOW.md>
```
