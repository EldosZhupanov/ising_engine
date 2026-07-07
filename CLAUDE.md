# Ising Engine

## Mission

You are working on a production-grade Rust optimization engine implementing Ising/QUBO algorithms.

Primary objective:

- maximize engineering quality
- minimize token usage
- never sacrifice correctness for speed

This repository is large.
Do not read unnecessary files.

---

# Working Strategy

Always solve tasks in this order.

1. Read git diff.
2. Read INDEX.md.
3. Read .claude/context/.
4. Search with rg.
5. Search with fd.
6. Search with ast-grep.
7. Read Cargo metadata.
8. Open only the minimum required source files.

Never scan the whole repository unless explicitly requested.

---

# Token Budget

Always minimize context usage.

Prefer:

- rg
- fd
- ast-grep
- cargo metadata
- git diff
- generated indexes

Never open 20 files if 2 files are enough.

Never summarize large files unless requested.

---

# Repository Layout

Core:

src/core/

Solver:

src/solver/

Compiler:

src/compiler/

Executables:

src/bin/

Tests:

tests/

Benchmarks:

benches/

Research:

research/

Generated indexes:

INDEX.md

.claude/context/

---

# Current Solver Architecture

UltimateSolver is the production solver.

Location:

src/solver/ultimate.rs

Engine:

src/solver/engine.rs

Quantum state:

src/solver/types.rs

Everything else is secondary unless explicitly requested.

---

# Legacy Solvers

The following solvers are independent implementations.

parallel_tempering

adaptive

cluster

tabu

Do NOT merge their implementations into UltimateSolver.

Do NOT rewrite them unless requested.

---

# Dead Code

ultimate_patch.rs

is not part of production.

Never edit it instead of

ultimate.rs

---

# API Rules

server_api.rs

must always call

UltimateSolver

Never bypass it.

---

# Build Rules

Every change must preserve compilation.

Required:

cargo check

Recommended:

cargo test

If benchmarking:

cargo bench

---

# Editing Policy

Prefer surgical edits.

Never perform repository-wide rewrites.

Never rename public APIs unless required.

Never introduce breaking changes without explanation.

---

# Search Policy

Always search before reading.

Examples:

rg UltimateSolver src

rg QuantumField src

rg solve src

fd ultimate

ast-grep

Never guess.

Search first.

---

# Investigation Order

If a bug appears:

1. git diff

2. failing test

3. related module

4. dependencies

5. implementation

Never randomly inspect files.

---

# Performance Rules

Performance has priority.

Avoid:

allocations

temporary Vec

HashMap in hot paths

string cloning

Prefer:

iterators

slices

references

packed layouts

cache locality

---

# Memory Rules

Avoid unnecessary cloning.

Prefer borrowing.

Avoid repeated allocations.

Reuse buffers.

---

# Rust Style

Prefer:

Result

Option

Iterator

match

avoid unwrap()

avoid expect()

unless impossible to fail.

---

# Unsafe

Unsafe requires justification.

Never introduce unsafe for micro-optimizations.

---

# Tests

If code changes:

run affected tests.

If public API changes:

run full tests.

---

# Benchmarks

Never claim performance improvements.

Measure first.

Use:

criterion

hyperfine

cargo bench

ab_engine_compare.py (identical-seed A/B, asserts bit-identical energies)

---

# Performance-Change Safety

Optimizations must be bit-identical (same trajectories, same energies)
or explicitly approved as behavior-changing.

Golden regression (tests/test_regression_golden.rs) must pass unchanged.

Keep the previous release binary; A/B with identical seeds.

Keep only measured >1% wins. Revert regressions immediately.

FP contraction (mul_add), reduced-precision RNG compares, and reordered
float sums CHANGE trajectories — rejected by default.

---

# Diagnostics

Prefer:

cargo check

cargo clippy

cargo test

before making assumptions.

---

# Git

Always inspect:

git status

git diff

before editing.

---

# Generated Context

Prefer using:

INDEX.md

.claude/context/tree.txt

.claude/context/modules.txt

.claude/context/symbols.md

.claude/context/git.md

instead of scanning the repository.

---

# Dependency Rules

Avoid adding crates.

Reuse existing dependencies.

If a new dependency is necessary:

justify it.

---

# Binary Targets

The repository contains many binaries.

Do not break unrelated binaries.

Keep every target compilable.

---

# Refactoring

Small focused refactors.

Never perform cosmetic rewrites.

Never reformat unrelated code.

---

# Documentation

Update documentation only when behavior changes.

Avoid redundant comments.

Explain WHY.

Not WHAT.

---

# Decision Priority

Correctness

↓

Compilation

↓

Tests

↓

Performance

↓

Readability

↓

Style

---

# Before Finishing

Verify:

✓ compiles

✓ tests affected code

✓ no unrelated edits

✓ minimal token usage

✓ minimal file reads

