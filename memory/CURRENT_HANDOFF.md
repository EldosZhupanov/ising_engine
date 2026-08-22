# CURRENT HANDOFF

Read `PROJECT_PLAN.md` first, then this file, then only the files it names. Do
not scan the repo. Updated 2026-08-22.

## State

- **HEAD `838e7ec`** — *add PROJECT_PLAN.md and CURRENT_HANDOFF.md as the
  canonical plan* (docs only). Branch `feat/solver-research-upgrades`.
- Tree clean except untracked `memory/CLAUDE_PRODUCT_COMPARISON_TASK.md`, the
  user's task authority for actions 1–6 below.
- `experiments/rc020/` **does not exist**; no RC-020 artifact of any kind.
- Coordinator: Claude Code, manual. Codex is no longer coordinator.
- Automations **empty**. No DAG. No orchestration workers.

## Current result

RC-020 controls **ALL PASS**. The pilot was invoked once; the frozen **P3 host
gate rejected it**. Seeds `10001–10008` did **not** execute; nothing was written.
A host-condition refusal — not a result, not an instrument failure; Gate A has
**no verdict**. **The "16.1% host drift" figure is terminal-observed, pending
record** — sole source orchestration message `msg_c57fccbbc13f`; never cite it.

## Next atomic task

Ordered actions of `memory/CLAUDE_PRODUCT_COMPARISON_TASK.md`. **(1) (2) (3) —
DONE**, reviewed CONSISTENT. **(4) close RC-020 — BLOCKED: host not quiet.**
Measured 2026-08-22 15:33: 135 MiB free of a 4 GB WSL cap; WSL relay
`UtilAcceptVsock` errors every 60 s for ~3 h; Windows-interop process creation
failing 7/25 (28%); `agy`, two `claude` and one `codex` live. RC-020 times
ms/sweep — this host cannot support it, and only **one** attempt is authorised.
**Do not spend it here.** Retry needs a quiet host: agents closed,
`wsl --shutdown`, ideally `memory=6GB`; re-verify before running.

## Gates

Gate A (RC-020 §7) is calibration only — *is `b` identifiable?* — on pilot block
`10001–10008`. Branches: **INSTRUMENT INVALID** (K1–K6, Class I, no result of any
kind) · **NOT IDENTIFIED** (K7, published null) · **IDENTIFIED + NOT FEASIBLE**
(K8, published null) · **IDENTIFIED + FEASIBLE** — the only one opening a
descendant, which must be **committed before any held-in seed opens**.
`FEASIBLE AT REDUCED RESOLUTION` is **not** that branch.

## Forbidden

- Running the pilot outside action 4's five preconditions; opening held-in/out;
  reusing burned `5001–5008`; inspecting reserved `6001–6008`; changing
  `CONTROL_SEED = 20001`; lowering the 9% drift bound.
- Writing a Class I outcome up as a Class II null.
- Foundry, DSL, new operators, growth campaigns; DAG, automations, `worker_done`.
- Any universal-optimizer or efficacy claim; any product build before the
  `PRODUCT_SPEC.md` §12 gate.

## Files

Under `research/` unless noted. Root — `PROJECT_PLAN.md`, `PRODUCT_SPEC.md`;
authority — `memory/CLAUDE_PRODUCT_COMPARISON_TASK.md`. Prereg —
`PREREG_RC020_MARGINAL_WALL_COST.md`, `PREREG_RC020_AMENDMENT_1.md`; instrument —
`src/bin/exp_marginal_cost.rs`. RC-016 — `RC016_CYCLE_RECORD.md`; closed —
`RC018_PILOT_ABORT_RECORD.md`, `RC017_ABORT_RECORD.md`. Comparisons —
`EXTERNAL_COMPARISON_PROTOCOL.md`, `EXTERNAL_PROJECTS_BACKLOG.md`.

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
