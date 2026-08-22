# CURRENT HANDOFF

Read `PROJECT_PLAN.md` first, then this file, then only the files it names. Do
not scan the repo. Updated 2026-08-22.

## State

- **Baseline HEAD `567dd7e`** (clean at that commit), branch
  `feat/solver-research-upgrades` — *align RC-020 instrument with Amendment 1*.
- **Working tree currently DIRTY**: four uncommitted docs — `PROJECT_PLAN.md`,
  `memory/CURRENT_HANDOFF.md` (both new), added blocks in `CLAUDE.md` and
  `ROADMAP.md`. Docs-only; no source file touched.
- `experiments/rc020/` **does not exist**; no RC-020 artifact of any kind.
- Coordinator: Claude Code, manual. Codex is no longer coordinator.
- Automations **empty**. No DAG. No orchestration workers.

## Current result

RC-020 controls **ALL PASS**. The pilot was invoked once; the frozen **P3 host
gate rejected it**. Seeds `10001–10008` did **not** execute; nothing was written.
A **host-condition refusal** — not a scientific result, not an instrument
failure. Gate A has returned **no verdict**.
**The "16.1% host drift" figure is terminal-observed, pending record.** Its only
source is orchestration message `msg_c57fccbbc13f`; no `RC020_*_RECORD.md`
exists. Do not cite it as evidence or feed it into any threshold.

## Next atomic task

**Await an explicit user decision on retrying the RC-020 pilot.** Do not run it
otherwise; the only precondition is a quiet host holding P3 spread under `0.09`.
Nothing else may start — stages S2–S7 of `PROJECT_PLAN.md` are all downstream of
this one verdict.

## Gates

Gate A (RC-020 §7) is calibration only — *is `b` identifiable?* — on pilot block
`10001–10008`. Four branches: **INSTRUMENT INVALID** (K1–K6, Class I, no result
of any kind) · **NOT IDENTIFIED** (K7, published null) · **IDENTIFIED + NOT
FEASIBLE** (K8, published null) · **IDENTIFIED + FEASIBLE** — the only branch
opening a descendant — which must be **committed before any held-in seed opens**.
`FEASIBLE AT REDUCED RESOLUTION` is **not** that branch.

## Forbidden

- Running the pilot without an explicit user decision.
- Opening held-in / held-out; reusing burned `5001–5008`; inspecting reserved
  `6001–6008`.
- Changing `CONTROL_SEED = 20001`; lowering the 9% drift bound.
- Writing a Class I outcome up as a Class II null.
- Foundry, DSL, new operators, growth campaigns; DAG, automations,
  `orchestration worker_done`.
- Any universal-optimizer or efficacy claim — RC-020 measures marginal wall cost
  and nothing else.

## Files

All under `research/` unless noted. Sequence — `PROJECT_PLAN.md` (root). Prereg —
`PREREG_RC020_MARGINAL_WALL_COST.md`, `PREREG_RC020_AMENDMENT_1.md`. Instrument —
`src/bin/exp_marginal_cost.rs`. RC-016 verdict — `RC016_CYCLE_RECORD.md`. Closed
cycles — `RC018_PILOT_ABORT_RECORD.md`, `RC017_ABORT_RECORD.md`. External
baselines — `EXTERNAL_PROJECTS_BACKLOG.md`. Open — `memory/OPEN_PROBLEMS.md`.

## Commands

```
git status --porcelain && git rev-parse --short HEAD
git diff --check                              # docs-only: no cargo needed
cargo check && cargo test                     # code changes
cargo clippy --all-targets -- -D warnings && cargo fmt --check

# RC-020 — ONLY on explicit user decision, on a quiet host
cargo run --release --bin exp_marginal_cost -- --controls
cargo run --release --bin exp_marginal_cost -- --pilot --dir experiments/rc020
```

## Token economy

Targeted `rg`/`sed` (`AGENTS.md` §2.6–2.7) · one implementer + one independent
reviewer (§3.5) · one atomic task · prompts cite paths, never paste logs · fresh
session or `/compact` between tasks · cheap work to a small model.
