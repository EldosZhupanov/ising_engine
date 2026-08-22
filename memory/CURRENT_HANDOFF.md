# CURRENT HANDOFF

Read `PROJECT_PLAN.md` first, then this file, then only the files it names. Do
not scan the repo. Updated 2026-08-22.

## State

- **HEAD** — see `git rev-parse --short HEAD`. Branch `feat/solver-research-upgrades`.
  Updated 2026-08-22.
- Tree clean. `experiments/rc020/` **does not exist** and never did — neither
  RC-020 attempt wrote an artifact.
- Coordinator: Claude Code, manual. Codex is no longer coordinator.
- Automations **empty**. No DAG. No orchestration workers.

## Current result

**RC-020 is CLOSED WITHOUT A SCIENTIFIC VERDICT.** Record:
`research/RC020_PILOT_ABORT_RECORD.md`.

Two pilot attempts, no artifact from either, **Gate A = NO VERDICT**:

- **attempt 1** — refused at the control gate (K1, P3 clean half), **exit 2**;
  the pilot body never ran and the seeds were not executed. Its log lived in
  `/tmp` (tmpfs) and was **destroyed by the later `wsl --shutdown`**; the record
  is now the only committed source for its figures, and they carry that caveat.
- **attempt 2** — after host remediation (agents closed, `memory=4GB → 10GB`,
  `wsl --shutdown`; WSL total 9947 MiB, load 0.08). In-process controls passed
  but at poor margin (`N1` paired spread `0.0755`, 84% of the `0.09` bound; `P3`
  clean `0.0397`) versus a standalone check two minutes earlier (`0.0012`,
  `0.0000`) — note the control phase is itself CPU-loaded.
  Repetition 0 completed; repetition 1 exceeded the sentinel bound **twice**, so
  §5 discarded the session. **exit 5**, no artifact.

**`K3` did NOT fire** — **failed executions = 2; unique discarded repetition IDs = 1; internal discard counter = 2.** `K3` cannot fire under any reading, because no value exceeds 2; both
failures were of the *same* repetition, and `K3` requires more than 2. Do not
report it as `HOST-UNSTABLE`, and do not write "2 of 9 repetitions".

Seeds `10001–10008` are **burned**, conservatively: the body executed them, but
nothing was observed — no artifact, and zero per-seed lines in the log. The
record states this is a *choice*, not a deduction. Held-in `11001–11008` and
held-out `12001–12008` are **untouched**; §9 descendant is **still blank**.

**Cause is NOT established and the hypotheses are NOT distinguished.** Elevated
paired spread existed already in the control phase, so warming from repetition 0
alone is insufficient — but the control phase itself loads the CPU, so thermal,
bursty-host and CPU-contention explanations are not separated. See §4.

**Instrument defect recorded:** `run_pilot` appends discard rows to an in-memory
buffer, but both abort paths `return Err` before the single `fs::write`, so every
discard row dies with the process — violating §5's "never silently dropped".
Underlying this is a **pre-registration contradiction**: §5 requires the discard
written to the artifact, §10.1 forbids writing it unless the session completes.

## Next atomic task

**A fresh pre-registered RC-021 host/instrument successor** — see
`PROJECT_PLAN.md` Stage S1b. Its subject is the *host and the instrument*, not
marginal cost: characterise the host's timing distribution with a pass criterion
fixed in advance, resolve the §5/§10.1 contradiction, close the §8 taxonomy gap,
and name fresh disjoint seed blocks.

**Binding method rule:** do **not** retry until a run happens to pass. Repeating
until a drift guard admits the run selects an atypically quiet moment and biases
the measured quantity. The instrument must work reliably, not pass once by luck.

## Gates

Gate A (RC-020 §7) is calibration only — *is `b` identifiable?* — on pilot block
`10001–10008`. Branches: **INSTRUMENT INVALID** (K1–K6, Class I, no result of any
kind) · **NOT IDENTIFIED** (K7, published null) · **IDENTIFIED + NOT FEASIBLE**
(K8, published null) · **IDENTIFIED + FEASIBLE** — the only one opening a
descendant, which must be **committed before any held-in seed opens**.
`FEASIBLE AT REDUCED RESOLUTION` is **not** that branch.

## Forbidden

- Retrying the RC-020 pilot at all — its single authorised attempt is spent;
  repairing the RC-020 instrument in place; reusing burned `5001–5008`,
  `7001–7008` or `10001–10008`; opening reserved `6001–6008`, `11001–11008` or
  `12001–12008`; changing `CONTROL_SEED = 20001`; lowering the 9% drift bound.
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
