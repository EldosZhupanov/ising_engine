# memory/CURRENT_TASK.md — what is in flight

*Authored here — no other file in the repo tracks live status. Update this on
every meaningful change.*

Last updated: 2026-08-19

## Branch

`feat/solver-research-upgrades` (main branch for PRs: `master`).
**Working tree clean.** The RC-001…RC-013 corpus, `memory/`, the `exp_*` binaries,
the two permanent tests and `website/` are committed, as are RC-014 and RC-015
with their pre-registrations. Pre-registration now precedes data in git history,
which is what makes Constitution §13 checkable rather than asserted.

## Active priority — RC-012, and it is an engineering defect, not a study

**`--early-stop` is documented as shipped and silently does nothing** (RC-012:
the deployed bootstrap yields 6 rows against `fit`'s 20-row floor, so it always
skips; and a fitted model predicts ≡0 at every step, making the ε-gate vacuous).
This is the only one of the five open decisions that is a **correctness/honesty
defect** rather than an improvement, so it goes first.

**Decompose it — the two halves have very different risk:**

- **Disable now (zero risk).** The flag currently changes nothing, so gating or
  removing it is **trajectory-neutral by construction** and makes the product
  honest immediately. No A/B needed; nothing to replay.
- **Fix properly (a project).** Layers 2 and 3 must be repaired *together* —
  fixing only the bootstrap ships a silent 50% truncation of every run. That is
  trajectory-changing and needs approval + identical-seed A/B.

## Then, in order (agreed priority for the five open decisions)

1. **RC-012** — fix or disable `--early-stop` (above).
2. **RC-011** — remove the predictor's proven blindness to instance features.
   Without it no sensor-sufficiency result can be turned into *control*: the
   model literally cannot see the instance it would be conditioning on.
3. **RC-005** — correct the `cost_model` (φ, not shape-only), A/B at identical
   seeds. Also a **prerequisite for RC-014 Gate B**, whose `I_replace_cost`
   estimand needs an honest equal-cost budget.
4. **RC-007** — narrow the Evolution Engine's ordering search; stop spending
   compute on pair-level distinctions that do not exist.
5. **RC-002** — diverse initialisation last: the effect is real but 0.09–0.46%,
   and adopting it re-bases the entire historical corpus.

## Next scientific question — sensor sufficiency, not a better operator

RC-014/RC-015 landed the project on it. The load-bearing consequence of RC-015 is
**not** the tie mechanism but this:

> Identical global parameters — `n`, `m`, weights, **and tie count** — determine
> neither the magnitude nor the sign of an operator's value.

G43/G44/G45 are the same `n = 1000, m = 9990`, all weights `+1`, and require
opposite treatment: G43 wants heat-bath (materially, −44), G44 and G45 are
indifferent. That is a candidate counterexample for the frozen `S_1` sensor map,
and it is the concrete, cheap instantiation of the plan's Phase 5.

**The audit, in order:**

1. Finish the **narrow G11/G32 cycle** under its own pre-registration, answering
   only "what fraction of the already-measured *ladder* effect passes through tie
   handling". Flat-`T` conclusions may not be transferred (Amendment 1 A1.5).
2. **Stop RC-015.** Do not hunt for an optimal `q`: D-15 showed `q = 0.75` does
   not beat the shipped heat-bath rule. Neutral-network observables are justified
   only as a **predictor of the Gibbs↔Metropolis choice**, never as a new operator.
3. **Sweep `S_1(instance) → (I_replace, ρ, sign, materiality)` across the whole
   G-Set**, not the six instances used so far. The harness already exists
   (`exp_tie_handling`, `exp_counterfactual`).
   **Caveat that shapes the sweep:** the six measured instances contain **no**
   close pair with opposite *material* signs — G43 is materially negative at
   n = 1000 while G23/G24 are materially positive at n = 2000, so `log_n` already
   separates them and the pair proves nothing. A real impossibility result needs
   **opposite material signs at the same n**, ideally inside one matched triple.
   That is what the sweep must look for; G43 vs G44 is only the weak
   "switch vs indifferent" form.
4. **Only if `S_1` is insufficient**, test the Amendment 1 A1.6 neutral-network
   observables as `S_2` — cheapest first (run lengths, recurrence, first
   non-neutral exit energy, overlap change). **Kill criterion:** if they do not
   separate the nearest conflicting pairs on held-out, do not build a
   neutral-network model.

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
