# memory/ARCHITECTURE.md — pointer

**Authoritative sources**
- `../ARCHITECTURE.md` — system overview
- `../CLAUDE.md` §1–2 — module map, current and accurate
- `../research/architecture/ADR/` — why it is shaped this way ([DECISIONS.md](DECISIONS.md))
- `../CONTEXT.md` — **superseded**, historical only (see [INDEX.md](INDEX.md))

This file records only the two things a newcomer gets wrong, which are not
stated plainly in any single source above.

## The hard boundary

Two systems in one repository. The arrow is one-way.

```
PRODUCTION  src/core · src/solver (UltimateSolver) · src/presolve · src/benchmark
              ▲  read-only, energy-exact bridge (frontend::qubo_model_to_ir)
              │
RESEARCH    src/engine_v2 · src/engine_v2/ai_scientist
```

`src/server_api.rs` must always call `UltimateSolver` and never bypass it.
Legacy solvers (`parallel_tempering`, `adaptive`, `cluster`, `tabu`,
`autopilot`) are **frozen** — never merged, never rewritten. The golden
regression `tests/test_regression_golden.rs` protects the production path.

## The read-only core

`Runtime`, `Scheduler`, `SpinState`, the three backends, the Operator API, the
Experiment Runner and the canonical scorer are read-only. Extend *above* them by
adding operators, agents or analysis; never edit them to suit a caller. All
computation is delegated to a `BatchExecutor` that owns the Runtime — never
reach around it.

## Three backends, one interface

`ProblemIR` (symmetric CSR: `row_ptr`, `col_idx`, `weights`) → `SpinState` trait:

| Backend | Role | Property |
|---|---|---|
| `ReferenceState` | f64 oracle | ground truth for cross-checks |
| `SparseBitSlice` | exact integer | verified bit-identical vs oracle at >100k spins |
| `DenseByte` | production-shaped | **4.8× vs oracle**, bit-identical |

Every operator is written against `SpinState` only and is cross-validated
bit-identical on **all** backends. That firewall is what makes ADR-0004's replay
guarantee hold. 18 operators are registered as of 2026-07-27 (was 14).

Note `SparseBitSlice` **rejects non-integral weights** — a real trap. A silent
`let Ok(..) else { continue }` around its constructor will skip every instance
and quietly measure the empty set.
