# AGENTS.md — Ising Engine Engineering Rulebook

This file is the operating manual for agents (Claude Code and subagents) working on this project.
Document hierarchy:
- `CLAUDE.md` — fast context, loaded every session (one-line architecture, commands, invariants).
- `ROADMAP.md` — living work plan, phase priorities, code diffs.
- `AGENTS.md` (this file) — stable rules: roles, workflow, code standards, quality gates. Changes rarely, committed to git.

If an instruction conflicts with `AGENTS.md`, the rule in `AGENTS.md` wins — unless the user has explicitly overridden it in the current conversation.

---

## 0. Engineering Philosophy and Decision Hierarchy

When several ways exist to implement the same thing, priority order is:

1.	Correctness — energy/solution correctness matters more than anything else
2.	Architecture — never violate the boundaries in §2 (scalar/MSC family, ownership)
3.	Performance — but only measured performance (see §5.2), never assumed
4.	API stability — never break public signatures without an explicit task
5.	Readability — clear code, as long as it doesn't cost correctness/architecture
6.	Code size — less code is better, but never at the expense of 1-5

**Never** sacrifice readability for correctness — if that trade-off appears, the decision is wrong at a higher level.
**Never** sacrifice maintainability for speed without benchmark numbers in front of you (see §5.2) — "probably faster" is not an argument.

On conflict (e.g. "the fastest option breaks the scalar/MSC family boundary"), the higher item in the hierarchy wins, and the conflict is explicitly surfaced to the user (see §8) — never silently resolved in favor of performance.

---

## 1. Domain Glossary

So the agent doesn't get lost in terminology while reasoning about the code:

- **QUBO** — Quadratic Unconstrained Binary Optimization: minimize `x^T Q x`, x ∈ {0,1}^n.
- **Ising** — equivalent spin formulation, s ∈ {-1,+1}, energy `H = -Σ J_ij s_i s_j - Σ h_i s_i`.
- **HUBO** — Higher-order Unconstrained Binary Optimization: same variables, but 3rd/4th order interactions (Edge3, Edge4).
- **Replica** — an independent copy of the spin system, simulated in parallel (in MSC, one of 64 copies packed into a machine word or byte-lane).
- **Parallel Tempering (PT)** — replicas at different temperatures periodically exchange states (replica exchange) to escape local minima.
- **Population Annealing (PA)** — an ensemble of replicas resampled by Boltzmann weight between "generations".
- **Trotter slice** — imaginary-time discretization used to simulate quantum tunneling (path-integral Monte Carlo); `num_slices` is the depth of that discretization.
- **MSC (Multi-Spin Coding)** — technique for storing many replicas in one machine word/cache line for SIMD parallelism across replicas rather than variables.
- **delta_e** — energy change from flipping one spin; computed locally (without a full system energy recompute) for performance.
- **Wolff cluster flip** — flipping a connected cluster of strongly correlated spins in one move (speeds up convergence near the critical point).
- **Replica exchange acceptance** — probability of swapping states between neighboring temperatures, `min(1, exp(-(β_i - β_j)(E_j - E_i)))`; target acceptance rate ~20-40%, handled by `adaptive.rs`.
- **Annealing schedule** — sequence of temperatures/β from "hot" (fast exploration) to "cold" (refining the minimum); generated geometrically in `ultimate.rs`.

### 1.1 Energy Function (important for verifying numerical correctness)

Full energy of one replica:
E = Σ_i h_i·s_i (linear term)
•	Σ_(i,j)∈Edge2 J_ij·s_i·s_j (pairwise)
•	Σ_(i,j,k)∈Edge3 J_ijk·s_i·s_j·s_k (3-body)
•	Σ_(i,j,k,l)∈Edge4 J_ijkl·s_i·s_j·s_k·s_l (4-body)
•	Trotter coupling between neighboring slices (only if num_slices > 1)
`calculate_delta_e_local()` does not compute full `E`, only the incremental change from flipping one `s_v` — only terms where index `v` appears. Any refactor of this function must preserve the invariant:
`E(after N steps via delta_e) == E(recomputed from scratch via calculate_replica_energies_local)` within float tolerance.
This is exactly the "re-sync" every 1000 steps in `ultimate.rs` — if the invariant is broken, re-sync will show a divergence (a numerical drift bug). This is the first place to look when a bug is suspected after a hot-path refactor.

---

## 2. Architectural Invariants (extended)

### 2.1 Two independent solver families — do not mix

| Family | Files | Data model | Parallelism |
|---|---|---|---|
| **Scalar family** | `parallel_tempering.rs`, `adaptive.rs`, `cluster.rs`, `tabu.rs` | `QuboModel` + `CsrMatrix` | per-replica via `Vec<Replica>`, sequential/rayon per-replica |
| **MSC family** | `ultimate.rs` + `solver/engine.rs` + `solver/types.rs` | `HuboModel`/`FlatHuboModel` → `QuantumField` (5D) | 64 replicas encoded in one word/cache line, vectorized across replicas |

**Rule:** any change in `engine.rs`/`types.rs` must not require changes in the scalar family, and vice versa. If a task requires touching both families at once — that's a signal to stop and ask the user; the task is likely mis-scoped.

### 2.2 Data Ownership

- `QuantumField` is the sole owner of spin state in the MSC family. No other structure may hold a copy of spins longer than one `step()`.
- `HuboModel` is the source of truth for interaction coefficients. `FlatHuboModel` is a derived read-only projection for the hot path, generated once at solver startup, never mutated during a sweep.
- `CsrMatrix` is used only by the scalar family and the legacy `QuboModel` path. Do not introduce `CsrMatrix` into the MSC family — `FlatHuboModel` already covers that.

### 2.3 Entry Points That Must Not Change Implicitly

- `server_api.rs` → `UltimateSolver::solve()` is the only solver call from the HTTP layer. If `solve()`'s signature changes, `server_api.rs` must be updated in the same commit/patch.
- All binaries under `src/bin/*.rs` (50+) must keep compiling after any change to public types in `core/`, `solver/`. Before considering a task done — `cargo build --release --bins`.

### 2.4 Module Ownership Boundaries

core/ — pure data structures and models. No randomness, no I/O, no rayon. compiler/ — translates logic gates into QUBO penalty functions. Knows nothing about solvers. solver/ — all Monte Carlo/PT/PA math. Knows nothing about HTTP/JSON. bin/ — entry points (CLI/HTTP/benchmarks). Orchestration only, no business logic.
No cross-ownership: if a refactor pulls logic "upward" (e.g. JSON parsing inside `solver/`), that's an architectural regression — revert it.

---

## 2.5 Anti-patterns — concrete examples (from project history)

Real issues that have already occurred in this codebase. The agent checks new code against these patterns before handing it to Reviewer.

**Anti-pattern A — variable shift blocks vectorization:**
```rust
// BAD — LLVM cannot vectorize a variable shift
for bit in 0..64 {
    let state_bit = ((spin_i >> bit) & 1) as f64;
}
// GOOD — direct index access per replica
for r in 0..NUM_REPLICAS {
    let state_bit = spins[base + r] as f64;
}
Anti-pattern B — hidden allocation in a hot loop:
// BAD — Vec::new() on every step() call
fn step(&mut self) {
    let mut deltas = Vec::new();   // allocated every call
    ...
}
// GOOD — buffer allocated once in the solver constructor
struct UltimateSolver { delta_buffer: Vec<f64>, /* reused */ }
Anti-pattern C — bitcast that LLVM cannot vectorize:
// BAD
f64::from_bits((val as u64) << 32)
// GOOD — pure f64 arithmetic (polynomial approximation, see ROADMAP Phase 4)
Anti-pattern D — scope creep "while I'm at it": An engine.rs change quietly changes a public signature in core/hubo.rs "for convenience" — forbidden without its own plan item and without an Explorer pass over every call site.
Anti-pattern E — mixing solver families: Adding a FlatHuboModel-specific method to parallel_tempering.rs "for code reuse" violates §2.1. If the code is genuinely shared, hoist it into core/ — don't pull MSC specifics into the scalar family.
Anti-pattern F — deleting "dead code" without verification: ultimate_patch.rs was deleted without a codebase-wide grep → it turned out one of the investor_proofs_v*.rs binaries imported it. See §6.5 for the mandatory order of operations.
2.6 Context Budget — token economy when reading
Order of context lookup BEFORE reading whole files, cheapest to most expensive:
1. CLAUDE.md / ROADMAP.md   — already gives an architectural map, often enough on its own
2. git diff / git status    — what actually changed, don't read the whole repo "just in case"
3. grep / rg by symbol      — find exact usage sites
4. cargo metadata           — crate dependency graph, if the question is about build/features
5. Read a specific file     — only once it's clear why this exact file, and which line range
Rules:
•	Never open a directory wholesale "to look around" — start with grep/glob on a specific symbol/pattern.
•	Never recursively inspect every subdirectory without a concrete search target.
•	Never read generated/incidental files: target/, Cargo.lock, .git/, benchmark output (criterion/ reports) — if you need a fact from there, get it via a command (cargo tree, objdump | grep), not by reading the raw file.
•	Explorer (§3.1) is the only role allowed broad exploration; the main session delegates to it rather than reading everything itself "to be safe".
2.7 Search Strategy — order for finding a symbol/pattern
1. Exact symbol name via grep/rg  (`rg 'fn calculate_delta_e_local'`)
2. Search by type via grep         (`rg 'QuantumField'`)
3. glob by filename                 (`solver/*.rs`)
4. cargo metadata / cargo tree      — if the question is about deps/features, not code
5. git grep / git log -p           — if you need the symbol's history
6. Read the whole file              — last resort, once the exact line range can't be pinned down otherwise
grep/rg is almost always cheaper than opening a whole file — prefer it whenever "where" and "how many times" is all that's needed, not the logic around it.
2.8 Patch Strategy — small edits over full rewrites
•	Prefer a targeted str_replace/diff over recreating a file wholesale.
•	Don't rewrite an entire file when a 10-20 line patch solves the problem — especially in engine.rs/types.rs, where a full rewrite risks losing unrelated logic (other sweep phases, PT swap, resampling).
•	A full file rewrite is justified only when: (a) the old structure is incompatible with the new architecture (e.g. Phase 1 changes the types.rs layout itself), and (b) the plan explicitly says "rewrite the file" — not something discovered mid-edit.
2.9 Context Explosion Prevention
If a task's investigation starts touching more than 8 files — that's a signal to stop, not to keep reading:
1.	Stop direct reading.
2.	Invoke/delegate to Explorer (§3.1) with the explicit goal of building a dependency map (who references what), not reading every file's contents.
3.	Based on the dependency map, Planner decides whether to split the task (see §6.6) or whether it's a genuinely broad architectural task (see §8, escalation to the user).
________________________________________
3. Agent Roles
Every non-trivial task (>1 file, refactor, perf-critical path) goes through explicit roles. The same Claude Code session can perform roles sequentially, but must explicitly announce the transition between them. For large refactors, use subagents (.claude/agents/*.md) for Explorer/Reviewer roles to avoid bloating the main session's context.
3.1 Explorer (read-only)
•	Task: find every place touched by a change (grep by type/function, all call sites, all tests covering them).
•	Tools: Read, Grep, Glob. Never Write/Edit/side-effecting Bash.
•	Output: a file+line list with a one-line reason each is relevant. Does not write code.
•	Required before: any change touching a public type (QuantumField, HuboModel, Edge2/3/4) or a signature used in 3+ places.
3.2 Planner
•	Task: turn Explorer's output into a step-by-step plan: file edit order, which tests to update/add, which benchmarks to run, which spots are marked "do not touch" (see §2).
•	Output: a numbered plan of atomic steps, each step = one logical commit.
•	Rule: the plan is shown to the user before edits begin, if the task is marked non-trivial (see §6.1). The plan is no longer than 15 items; if it is, the task is too big — split it.
3.3 Implementer
•	Task: applies edits strictly per Planner's plan, one plan step = one edit/one commit.
•	Rule: never expands scope without stopping and flagging it ("fixed X while I was at it" is forbidden — it becomes its own plan item).
•	Must: run cargo check (at minimum) after each step before moving to the next.
3.4 SIMD/Perf Specialist
•	Task: project-specific role — any change in engine.rs/types.rs claiming a speedup must be verified by this role before being called done.
•	Checklist (all items mandatory, see §5.2): LLVM vectorization remarks, objdump for ymm/zmm instructions, cargo bench before/after with numbers, no regressions in the scalar family.
•	Forbidden: claiming "sped up Nx" without cargo bench numbers in the same response.
3.5 Reviewer/Verifier
•	Task: independently verifies a finished change against §2 (invariants) and §5 (quality gates). Doesn't write code, only READ + test commands.
•	Output: Pass/Fail per checklist item, with concrete lines for any Fail.
•	Rule: Reviewer cannot be the same role/pass as the Implementer of the same change — at minimum, re-read the diff "from scratch".
3.6 Benchmark Auditor
•	Task: after merging perf changes, run gset_benchmark and criterion benches, compare against the baseline recorded in ROADMAP.md ("Real Performance" section), update those numbers there.
•	Rule: the baseline is updated only with real measured numbers, with environment noted (Linux/WSL/Windows, CPU, compile flags). Never extrapolate a theoretical speedup as a measured one.
3.7 Doc Keeper
•	Task: keep CLAUDE.md/ROADMAP.md/AGENTS.md in sync after structural changes (new module, deleted file, changed invariant).
•	Rule: if an edit changes a fact recorded in CLAUDE.md (e.g. "22 tests" → now 25) — Doc Keeper updates the number in the same PR/patch, not later.
________________________________________
3.8 Subagent Templates (.claude/agents/*.md)
The roles in §3.1–3.6 can be materialized as real Claude Code subagents — this moves them into a separate context and saves the main session's tokens. Recommended project files (put in .claude/agents/, commit to git so the whole team/future sessions see them):
.claude/agents/explorer.md
---
name: explorer
description: Read-only search for every place touched by a change to a public type or function in ising_engine. Use before any edit in core/ or solver/ that touches 3+ modules.
tools: Read, Grep, Glob
model: sonnet
---
You are a read-only explorer of the ising_engine codebase. You only search and report, never edit files.
For a given type/function, find: all declarations, all call sites, all tests covering them, all binaries in src/bin/ that use them.
Return a file:line list plus one line explaining relevance each. Do not propose a solution — that's Planner's job.
.claude/agents/perf-verifier.md
---
name: perf-verifier
description: Verifies perf changes in solver/engine.rs and solver/types.rs against the SIMD/Perf Specialist checklist (AGENTS.md §5.2). Use after any edit claiming a speedup.
tools: Read, Bash, Grep
model: sonnet
---
You verify perf changes in ising_engine. Run: cargo build with loop-vectorize remarks, objdump grep ymm,
cargo bench --bench benchmark. Compare numbers against the baseline in ROADMAP.md. Return Pass/Fail per §5.2 item in AGENTS.md with concrete numbers, not qualitative claims like "faster".
.claude/agents/reviewer.md
---
name: reviewer
description: Independent review of a diff against the invariants in AGENTS.md §2 and the quality gates in §5. Use before considering any non-trivial task complete.
tools: Read, Grep, Bash
model: opus
---
You are an independent reviewer. You're given a diff and a link to the plan. Check: are any §2 invariants violated (mixing scalar/MSC family, changed entry points), are the §5 gates applicable to this change type met. Don't rewrite code — only Pass/Fail plus concrete line references.
To enable: /agents → Create new agent → Project, or just drop the files above into .claude/agents/.
________________________________________
3.9 Thinking Protocol — how to think before any edit
This is a thinking-level protocol, not an action-level one — it precedes Workflow (§4) and applies even to Trivial tasks mentally, without a formal pass through every role.
1. Understand the task         — restate in your own words what should change
2. Find the minimal change     — the smallest patch that solves exactly this task (see §2.8)
3. Identify affected modules   — which ownership boundaries (§2.4) this touches
4. Check architectural boundaries — is §2.1/§2.2 violated (scalar/MSC family, ownership)
5. Predict regressions         — what else could break beyond the obvious (other call sites, tests)
6. Estimate performance impact — if it's hot-path code, is a §5.2 pass needed
7. Implement the minimal patch — no more than the task requires
8. Verify                      — the §5 gates applicable to this change type
9. Update documentation        — only if an architectural fact changed (Doc Keeper, §3.7), not by default
The point of the protocol is a predictable checklist pass, not an intuitive "creative" edit. If step 3-4 surfaces a conflict with §2 — stop and go to §8 (escalation), don't keep improvising around the conflict.
________________________________________
4. Workflow (Explore → Plan → Implement → Verify → Document)
1.	Classify the task — trivial (1 file, local fix) vs non-trivial (see §6.1).
2.	Explore — a trivial task can skip this role if the file is already open and the context is clear.
3.	Plan — mandatory for non-trivial tasks. Show the plan to the user if the task is architectural (changes a §2 invariant) or touches >3 files.
4.	Implement — per the plan, one step at a time, cargo check after each.
5.	Verify — run the full set of quality gates (§5) applicable to the change type.
6.	Document — Doc Keeper updates the affected .md files.
7.	Report — a short summary: what changed, what numbers came out (tests/benches), what's left.
4.1 Git Discipline
•	One logical plan step = one commit. Don't mix "refactor" and "new feature" in one commit.
•	Commit message format: <area>: <what was done, imperative> — e.g. engine: replace variable-shift with byte-per-replica access.
•	Don't commit if cargo test --release fails — if tests are deliberately broken by an intermediate step, that's reflected in the plan as "step N: update test" as the very next step, not spread across published commits over more than one session.
________________________________________
5. Quality Gates (Definition of Done)
5.1 Any change under src/
•	[ ] cargo check — clean, no new warnings in changed files.
•	[ ] cargo test --release — all tests green (baseline: 22/22; number updated by Doc Keeper when tests are added).
•	[ ] cargo build --release --bins — every binary under src/bin/ compiles.
•	[ ] Public types (QuantumField, HuboModel, Edge2/3/4, FlatHuboModel) — if a signature changed, every call site updated (found via Explorer).
5.2 Additionally, for perf changes (Roadmap Phases 1-5)
•	[ ] LLVM vectorization remarks show vectorized loop for the affected hot loops: 
•	RUSTFLAGS='-C target-cpu=native -C llvm-args=-pass-remarks=loop-vectorize' \  cargo build --release --lib 2>&1 | grep "vectorized loop"
•	[ ] objdump -d target/release/libising_engine.rlib | grep -c ymm — count increased vs baseline (not 0, if the change targets AVX2).
•	[ ] cargo bench --bench benchmark — before/after numbers attached to the report, not just qualitative "it's faster".
•	[ ] Scalar family (parallel_tempering.rs etc.) untouched and its tests still pass — a regression there means §2.1 was violated.
5.3 Additionally, for changes to core/hubo.rs (data model)
•	[ ] test_hubo_qpa.rs and test_anls.rs pass.
•	[ ] If FlatHuboModel was added — there's a round-trip test HuboModel → FlatHuboModel → energies match the original path (numeric comparison with tolerance, not exact float equality).
5.4 Additionally, for server_api.rs
•	[ ] Input validation not weakened (bounds check num_vars ≤ 50000 still there).
•	[ ] CSR edge deduplication still applied before passing to the solver.
•	[ ] Semaphore backpressure not removed.
5.5 Review Matrix — mandatory checklist for Reviewer (§3.5)
Every patch is checked across all columns, not just the one that seems obvious for this task (a perf patch is also checked on Architecture and API, not only Performance):
Dimension	Question
Correctness	Does the energy/result match the baseline on a test seed (§6.3)?
Performance	Are there cargo bench numbers, not an eyeballed estimate?
Architecture	Are §2.1/§2.4 boundaries (scalar/MSC, ownership) intact?
API	Are public signatures unchanged without a dedicated task?
Memory	No new allocations in the hot loop (§7)?
Safety	Does every unsafe block have a // SAFETY: comment?
Regression	Do existing tests (all levels in §7.1) pass?
Tests	Was a test added at the appropriate level for the new behavior?
A Fail in even one column = the patch is not Done, regardless of how well it solves the original task.
________________________________________
6. Refactoring Rules
6.1 What counts as "non-trivial" (requires an explicit plan and often user confirmation)
•	A change to a public type used in 3+ modules.
•	Any edit in engine.rs/types.rs (this is the hot path, mistakes there are expensive).
•	Deleting a file/module (even one marked as dead code — confirm via Explorer that it's truly unimported anywhere, not just by the roadmap's comment).
•	A Cargo.toml dependency change affecting feature flags or the Windows/Linux build simultaneously.
•	Anything touching server_api.rs (the public HTTP API contract).
6.2 Refactor Atomicity Principle
Every refactor moves the codebase from one green state (cargo test passes) to another green state. Intermediate steps within one session can be red, but:
•	Never end the session/report "done" to the user in a red state.
•	If the refactor is multi-step (e.g. Phase 1: types.rs → engine.rs → ultimate.rs), it's acceptable to keep the code non-compiling for 1-2 steps within one series of edits, but the final step of the series must return a green state before handing control back to the user.
6.3 Numerical Result Compatibility
For any solver refactor (not just SIMD) — compare energy/result on the same seed before and after the change on a small test instance (e.g. G1 from gset/). Numeric drift within float precision is fine. Systematic divergence is a bug — revert and investigate, don't write it off as "Monte Carlo randomness".
6.4 Scalar Family Backward Compatibility
ParallelTemperingSolver, AdaptiveTemperingSolver, ClusterSolver, TabuSolver — stable public API for external callers (investor demos, legacy binaries). Don't change their signatures while refactoring the MSC family. If a change is genuinely needed — a separate explicit task with its own plan, not "while I'm at it".
6.5 Dead Code
Before deleting any file marked in ROADMAP.md as "dead code" (ultimate_patch.rs etc.):
1.	Explorer confirms: the file is not referenced in any mod.rs, nor in any use outside itself.
2.	Only after confirmation — delete. If the file is used somewhere after all, it's not dead code — the roadmap is stale, update ROADMAP.md.
6.6 Breaking Down Large Tasks
The roadmap is deliberately split into Phases 0-7 with different priorities — don't try to do Phase 1 (SIMD layout) as one giant patch "combined with" Phase 2 (FlatHuboModel). Order:
1.	Each roadmap phase = at least one separate Explore→Plan→Implement→Verify pass.
2.	Phase 1 is large enough on its own to be broken down by file (types.rs → then engine.rs → then ultimate.rs), with a green cargo check between steps where possible, and one final green cargo test --release at the end of the whole phase.
3.	Don't start Phase N+1 until Verify for Phase N has fully passed (§5) — otherwise risk accumulates and becomes hard to localize on regression.
6.7 Safe Refactor Sequence and Root Cause Analysis
Safe refactor order — never mix rename/move with logic changes in one step:
rename/move  →  compile (cargo check)  →  test (cargo test)  →  then optimize
If one commit both renames a method AND changes its logic — Reviewer (§3.5) must reject the patch: split it into two commits, even if the diff looks small.
For any bug found (not just a perf regression) — don't just patch the symptom:
Bug → Root cause → Which §2/§6 invariant was violated → A test catching this exact cause → §11 (if §5/§7.1 tests didn't catch it immediately)
A fix without an established root cause and without a new test covering it specifically is a temporary patch — mark it explicitly as such in the report (§9.2) and don't close the task as fully resolved.
________________________________________
7. Code Standards (Rust, perf-critical)
•	unsafe: allowed only in the hot path (solver/engine.rs), and only with a // SAFETY: ... comment explaining the invariant that makes the block sound. Every unsafe block is small (one operation) — never wrap a whole function.
•	Allocations in a hot loop are forbidden. step() and anything called inside a sweep must not do Vec::new/push/clone on every iteration — only reusable buffers allocated once at solver initialization.
7.0 Explicit list: what's forbidden inside a hot loop (step(), calculate_delta_e_local, anything invoked on every sweep iteration)
•	No allocations (Vec::new, vec![], .clone() on owning structures).
•	No HashMap/HashSet lookups — the hot path works with flat indices (CsrMatrix/FlatHuboModel), not hash structures.
•	No format!/String — string formatting/building only in diagnostics/logging outside the hot path.
•	No closures capturing Box/Rc created on every iteration.
•	If profiling (cargo bench/perf) doesn't show a bottleneck at a specific spot — don't optimize "by eye"; measure first (see §0, item 3).
•	rayon: parallelism only across outer dimensions (temperature cells, populations), not across replicas within a cache line — replica-level parallelism comes from SIMD/layout, not threads.
•	f64 vs f32: the solver uses f64 for energies (accumulated error over millions of steps). Don't lower precision in intermediate sums without an explicit request and without measuring the impact on solution quality (energy gap on G-Set).
•	Branching in a hot loop: prefer branchless patterns (see Roadmap Phase 3) where profiling shows branch misprediction, but don't introduce branchless code for its own sake without measurement — sometimes a predictable branch is faster.
•	Naming: the *_local suffix is reserved for functions operating on a slice of a single temperature cell (current convention in engine.rs) — don't reuse it for something else.
•	Clippy: cargo clippy --release -- -D warnings must be clean for new/changed files before a step is considered done (pre-existing legacy warnings in untouched files are not a blocker).
7.1 Testing Strategy — what's checked where
Level	File(s)	What it checks	When required
Unit — gates	test_gate_correctness.rs, test_logic.rs	AND/XOR/adder/multiplier produce correct QUBO penalties (exhaustive over all inputs)	compiler/ changes
Unit — energies	test_energy.rs	calculate_delta_e_local agrees with a full energy recompute on random configurations	engine.rs changes
Unit — HUBO/QPA	test_hubo_qpa.rs, test_anls.rs	3/4-body edges enter the energy correctly; ANLS converges on a test matrix	core/hubo.rs, core/anls.rs changes
Integration — solvers	test_solver.rs	Every solver (PT/adaptive/cluster/tabu/ultimate) finds a known optimum on a small instance (e.g. a simple MaxCut on 10-20 vertices)	any file change in solver/
Regression — numerical drift	ad hoc, see §6.3	Energy on a fixed seed matches before/after a refactor within float precision	any refactor of the math (not just SIMD)
System — performance	benches/benchmark.rs, gset_benchmark.rs	Absolute numbers (ms, ymm count) on real G-Set instances	Roadmap Phases 1-5
Principle: new functionality without a test at at least one level of this table is not Done. If the functionality doesn't fit any row — discuss with the user where to test it before writing the code (a signal that either the functionality is misplaced, or the table needs another row).
7.2 Property-based verification (recommended, not mandatory yet)
For calculate_delta_e_local, it's worth adding a property test: "for a random HuboModel and a random initial configuration, the sum of delta_e over a sequence of N random flips equals the difference calculate_replica_energies_local(before) - calculate_replica_energies_local(after)." This catches exactly the class of bugs that quietly corrupt correctness during a layout refactor (Phase 1) but that don't break existing unit tests written on too-small/degenerate instances.
________________________________________
8. Escalation — when the agent must stop and ask
•	The task requires changing both the scalar family and the MSC family at once.
•	The task requires deleting/changing the public contract of server_api.rs.
•	The solver's numeric result (energy/best solution) diverged from the baseline on a test instance after a refactor that shouldn't have affected the math (e.g. a pure layout refactor in Phase 1).
•	A new heavy dependency is required (an ethers-like case) — justify the need first, estimate the number of transitive crates.
•	The refactor plan has grown past 15 steps — the task isn't decomposed, go back to Planner.
•	"Dead code" claimed in the roadmap turns out to be used (see §6.5, step 1 failed).
In all these cases — briefly state the conflict/finding and propose 2-3 courses of action; don't silently proceed down the path of least resistance.
8.1 Confidence Levels
Every non-trivial conclusion (especially "this is dead code", "this doesn't affect other modules", "energy matches to float precision") comes with a confidence level:
High   — confirmed by running something (test/grep gave an unambiguous result)
Medium — confirmed by reading the code, but not by running anything
Low    — a guess based on structure/naming, unverified
Rule: if confidence on a fact driving a decision is below Medium (i.e. Low) — don't act on that fact; raise confidence first (run Explorer, run a grep/test). This is especially critical for §6.5 (deleting "dead code") and §6.3 (numerical compatibility) — Low confidence is not acceptable before an irreversible action (deleting a file, committing a perf change).
________________________________________
9. Quick Command Reference
# Build
cargo check -j 1                                  # if a normal build fails (Windows env issue)
cargo build --release --bins                      # every binary must build

# Tests
cargo test --release                               # 22/22 baseline

# Perf verification (see §5.2)
RUSTFLAGS='-C target-cpu=native -C llvm-args=-pass-remarks=loop-vectorize' \
  cargo build --release --lib 2>&1 | grep "vectorized loop"
objdump -d target/release/libising_engine.rlib | grep -c ymm
cargo bench --bench benchmark

# Benchmarks on real data
cargo run --release --bin gset_benchmark

# Lint
cargo clippy --release -- -D warnings
________________________________________
9.1 Task Size Classification
Size	Signs	Required
Trivial	1 file, not public API, not hot path	cargo check, commit
Standard	2-3 files OR a public type in 1 module	Explorer (opt.) → Implement → §5.1
Non-trivial	public type in 3+ modules, hot path, file deletion	Explorer → Plan (show the user) → Implement → §5 in full
Architectural	new invariant, new dependency, §2 change	All of the above + explicit discussion with the user before Implement
9.2 Final Report Template
Every non-trivial task ends with a report in this format (short, no filler):
Changed: <files>
Quality gates: cargo test 22/22 ✅ | clippy ✅ | bins build ✅ | [if perf] vectorize ✅ bench: 291ms→143.9ms
§2 invariants: intact / <if broken — exactly what and why it was necessary>
Remaining: <next ROADMAP.md item, or "nothing">
________________________________________
10. Maintaining This File
•	AGENTS.md changes rarely: a new role, a new durable refactoring rule, a changed architectural boundary. Don't put current tasks/progress here — that's what ROADMAP.md is for.
•	If the same question/mistake recurs 2+ times across different sessions — that's a candidate for a new rule here, not a one-off chat note.
•	When adding a new rule — state which real problem it prevents (not abstract "best practices just in case").
11. Incident Log (postmortem log)
When a refactor caused a regression that the quality gates didn't catch immediately (e.g. numeric drift only surfaced on a large G-Set instance, not on unit tests) — record a short entry here. Goal: the next session doesn't hit the same rake, and §5/§7.1 grow over time from real gaps, not imagined ones.
Entry format:
### <date> — <short title>
What broke: ...
Why quality gates missed it: <which test/check didn't cover this case>
Fixed: <commit/file>
New rule/test added: <§ reference, if applicable>
(Entries are added as incidents occur — none exist as of this file's creation.)
________________________________________
