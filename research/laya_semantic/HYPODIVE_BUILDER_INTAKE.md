# LAYA-001 Builder intake

Mode B: capability with a fixed exploratory pilot; no confirmatory science claim.
User authorized starting semantic Laya decisions plus verified discrete
reconciliation, followed by investigation of residual conflict graph reduction.
Done: executable offline pipeline using the real installed model and real
UltimateSolver, independent exact/constraint checks, retained raw data and review.
Stop after this pilot and handoff; real-domain validation is a separate study.

## Explorer map (read-only, 2026-09-23)

- `src/lib.rs:46` exports presolve; `src/core/hubo.rs:165` defines QuboModel.
- `src/core/hubo.rs:176` scores symmetric CSR with a 1/2 factor: adapter must
  insert each upper-triangular pair in both rows with its full coefficient.
- `src/solver/ultimate.rs:138` is the unchanged public solve entry point.
- `src/solver/ultimate.rs:165` invokes full_presolve already; line 176 decomposes.
- `src/presolve/mod.rs:38,194,228,283`: first-order fixing, components, extraction,
  full presolve. Reuse them; never call Ultimate an unreduced comparison.
- `tests/test_presolve.rs`, `test_decomposition.rs`, `test_qpbo.rs` cover reductions.
- `research/Cargo.toml` already depends on ising_engine, serde, serde_json.
  A new research example can bridge JSON without dependency/source/API changes.
- Installed Laya agent.py loads safetensors, may edit tokenizer metadata;
  common.py constructs separate question sequences. Freeze package hashes.

## Atomic edit order

1. Freeze protocol/intake and current-task pointer.
2. Add isolated research Rust example, Python orchestration/analysis and invariant
   tests. Use existing presolve and solver; no copied solver implementation.
3. Test/check instrument and freeze; clean checkout before any pilot inference.
4. Run one fixed pilot, commit raw outputs and analysis separately.
5. Independent read-only review, final handoff and memory update.

## Invariants and constraints

Finite probabilities, bounded n, valid unique constraints, exact pair convention,
feasibility checked independently, objective equality over all pilot states,
presolve optimum equality, deterministic fixed seeds, same probabilities/rules for
all arms. Domain correctness and globally optimal energy are separate properties.
Existing binding records, src, root dependencies, public APIs, scalar/MSC families,
unrelated dirty files and earlier experiments are out of scope.

No known evidence supports accuracy/speed superiority here. Existing presolve
means residual reduction is reuse of an established capability, not a novel method.
The smallest competing explanation is that greedy repair supplies every benefit.
