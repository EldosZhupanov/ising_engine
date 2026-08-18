# memory/CURRENT_TASK.md — what is in flight

*Authored here — no other file in the repo tracks live status. Update this on
every meaningful change.*

Last updated: 2026-07-27

## Branch

`feat/solver-research-upgrades` (main branch for PRs: `master`).
**~37 uncommitted paths** — 9 modified under `src/`, the rest untracked
(`research/RC00*.md`, `research/AXIOMS_OF_OPTIMIZATION.md`,
`research/CHANNEL_EXHAUSTION.md`, `research/RELATIONAL_PRIMITIVE.md`,
`research/COMPETITIVE_ANALYSIS.md`, `src/bin/exp_*.rs`, `src/bin/control_api*`,
`website/`, this `memory/` directory). Nothing is committed yet.

## Active priority

**`../ROADMAP.md` "Next 10", item 1 — the growth campaign.** A long
curiosity-driven `--service` run toward 500k recorded experiments, re-measuring
cross-family transfer as the dataset grows. Everything in Stage 11 (Research
Foundation Model) is gated on data volume, not on architecture, so this is the
unblocking task.

```
cargo run --release --bin research_platform -- \
    --file "$(ls benchmark_suite/data/gset/G* | paste -sd,)" \
    --service --max-experiments N --executive --planner --shared-knowledge
```

## Last verified gate results (2026-07-26)

| Gate | Result |
|---|---|
| `cargo build --release --workspace --all-targets` | clean |
| `cargo test --release --workspace` | **317 passed / 0 failed / 5 ignored** |
| `cargo clippy --release --workspace --all-targets -- -D warnings` | **0 issues** |
| `cargo fmt --all --check` | clean |
| `cargo test --release --test test_regression_golden` | **3 passed / 0 failed** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` | **0 errors** (was 18) |
| test count | **322** passed / 0 failed (was 317; +3 gradient-ledger invariant, +2 passport audit) |
| website `lint` / `typecheck` / `build` | clean; 0 stubs, 0 `planned` nav entries |

## Recently landed (this working session, uncommitted)

- **CI scope widened + docs gate added** (2026-07-27). `cargo metadata` proved
  `workspace_default_members` is the root package alone, so CI's `cargo build
  --all-targets` **never compiled `research/`** — the structural reason the
  E0063 break there survived. Added `--workspace` to build/clippy/test, `--all`
  to fmt, and a new `docs` job with `RUSTDOCFLAGS: -D warnings`. All 7 `run:`
  commands in `.github/workflows/ci.yml` extracted and executed locally: **all
  pass**.
- **Doc pipeline repaired** (2026-07-27), the last broken gate. 18 rustdoc errors
  across 9 files → 0. Bare `[x]` in prose parsed as intra-doc links,
  `Vec<Vec<Edge>>`/`<EXP>` as unclosed HTML, one public→private link. Fixed by
  backticking prose: **15 doc-comment lines, zero code lines** (diff-verified).
  Errors cascade because rustdoc aborts per crate — three passes were needed, so
  a single clean run is not evidence until it is re-run after each fix.
- Fixed the only broken target in the workspace: `research/src/bin/test_engine.rs`
  was missing `energy_offset` on `QuboModel` (E0063). Set to `0.0`, which
  reproduces the binary's behaviour from before that field existed.
- Four new operators registered (registry now **18**, was 14):
  `consensus_freeze`, `consensus_seek` (`operators/ensemble_thermostat.rs`);
  `synth_population`, `synth_dynamics` (`operators/move_synthesis.rs`).
  Both modules carry a null variant that is unit-tested **bit-identical to
  `metropolis_sweep`**, so any measured effect is the new term and not a
  construction error.
- Four experiment binaries: `exp_consensus`, `exp_initialization`,
  `exp_synthesis`, `exp_field_capacity`.
- Research record RC-001…RC-004 plus `AXIOMS_OF_OPTIMIZATION.md` and
  `CHANNEL_EXHAUSTION.md` — see [RESEARCH.md](RESEARCH.md) for what stands and
  what was retracted.

## Research programme

Eleven research cycles complete (RC-001…RC-011). **Master register:
`../research/RESEARCH_INVENTORY.md`** — inventory, revocation register,
validation matrix, reproduction commands, code-integration decisions, CI
coverage, dependency graph, final audit.

Headline: **six of eleven cycles refuted something previously asserted**,
including two of my own instrument errors and one retracted law. Two results
landed as permanent tests (RC-006, RC-009); four are open decisions in
[OPEN_PROBLEMS.md](OPEN_PROBLEMS.md) §0.

Next highest-value experiment: audit **`Dynamics`** — the last unaudited learned
model and the only one whose errors are *silent* (a wrong remaining-improvement
prediction halts a run early and costs quality with no observable failure).

## Immediate decisions pending

1. **Commit or not.** Nothing above is committed. The research record includes a
   retraction, which is worth preserving in history.
2. **`src/core/simd_utils.rs`** — orphaned (declared in no `mod.rs`, never
   compiled), self-described as "a theoretical implementation". Either delete it
   or implement it; leaving it reads as a performance claim the code does not
   support. See [OPEN_PROBLEMS.md](OPEN_PROBLEMS.md).
