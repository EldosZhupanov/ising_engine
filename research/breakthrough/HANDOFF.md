# Saved checkpoint — 2026-09-11

**Resume event, 2026-09-12:** the thread goal was externally changed from blocked to active. Work resumes from this checkpoint; current execution status is in RESEARCH_STATE and memory/NOW. The text below preserves the state at the stop, not a new prohibition on the resumed task.

The user explicitly stopped work on 2026-09-11 and requested saving everything so the next launch starts here. **No further research or experiments should run until a new explicit resume request.** The research objective is unfinished, not achieved. These files are saved in the working tree; no commit was created in this session.

## Startup

Run `git status --short` first, then read `START_HERE.md`, `memory/NOW.md` and this bundle. Baseline HEAD at stop: `fdec0df` (`fix(rc027): freeze full-budget executor`), branch `feat/solver-research-upgrades`. Expected changes: `memory/NOW.md`, `memory/CATALOG.md`, `memory/BINDING_SHA256`, new `research/breakthrough/`. Pre-existing **unrelated** `results/rc027/` is untracked and must remain untouched/uninspected. Do not stage it. Do not inspect or modify `experiments/rc021/` terminal evidence; do not rerun its controls/sentinels.

## Exact point reached

1. Completed an algorithm-surface repository audit, including `engine_v2` state/backends/operators/Runtime/selector/research platform and independent read-only Explorer examination of historical branches and research binaries. This is not a line-by-line certification of all files. Read [AUDIT.md](AUDIT.md).
2. Wrote ten mathematically specified, prior-art-linked hypotheses and four prioritization scores each: [HYPOTHESES.md](HYPOTHESES.md), directories `h01`..`h10` each with the five requested documents.
3. Derived a valid counterexample to the overbroad marginal-vacuity corollary in the immutable RELATIONAL_PRIMITIVE record. The correct Gibbs-symmetry theorem stands. Read [GAUGE_SCOPE.md](GAUGE_SCOPE.md). No historical evidence was edited.
4. Selected H01 conditional elimination and H07 joint two-flip curvature for first factorial experiment. Wrote [EXP001_PROTOCOL.md](EXP001_PROTOCOL.md), fixed all independent review blockers, obtained **PASS for preregistration design**. No prototype code or candidate result data exists. **Preregistration is NOT committed yet.** Its checksum/catalogue entry are prepared for the commit; do not run candidate data before committing it.
5. Ran only existing baseline build/golden checks and host timing calibration. Calibration is [calibration.tsv](calibration.tsv), not a candidate experiment. No breakthrough, solver improvement or matched-cost advantage has been measured.

## Independent review, already completed

Agent `/root/audit_review` performed read-only Explorer and review; it made no edits. Do not depend on that session agent surviving a restart.

Initial MEDIUM blockers were all repaired before commit/data: frozen primary contrasts and aggregation, handling of censored target times, C timing at every completed solve, common full-variable 1-opt for every arm (production polish touches only free variables), exact inline normative equations instead of mutable governing notes, and exactly two subdivision vertices per cubic-core edge. Final reviewer verdict: **PASS for prereg commit; no HIGH/MEDIUM**, explicitly not a review of future prototype/results.

Key historical findings worth preserving: Bayesian/ML-labelled research binaries already learn per-variable proposal probabilities from downhill/thermal/rejection statistics and neighbor activation. `bayesian_scaling_lab_v3.rs:333–339` appears to compare `te` to the identical just-swapped energy, so the intended branch for memory transfer is ineffective; this is a read-only finding, not fixed or tested. `core/anls.rs` is multiplicative shifted NMF with test call sites, not an integrated spectral optimizer. GNN demo probabilities and advertised Gurobi benchmark time are hardcoded; the “official GSet” demo is not an official-instance oracle. Cached remote HUBO 3-XORSAT family is satisfied by all-ones. These paths cannot establish performance baselines.

## Next actions after explicit resume

1. Review only the saved protocol and working diff; do not restart broad discovery. Re-run the scoped memory check below if needed. Commit **only** the intended research documents, maps, checker and three memory files, with message such as `research: preregister conditional elimination experiment`. Inspect staged diff and confirm no `results/rc027/` staged. Preserve baseline source commit `fdec0df` in the record. If Git writes require environment approval, files are already saved and reviewable; the missing action is the preregistration commit, not research clarification.
2. Implement isolated `research/breakthrough/probe.rs`, linked with rustc to `target/release/libising_engine.rlib`, and `run_exp001.py`; no Cargo/public API/production source edits. No new dependencies. Exact experiment design is frozen in EXP001. All algorithm code must come after prereg commit. Do not silently alter the protocol after outcomes.
3. Use canonical `core::QuboModel` from **`core/hubo.rs` via `core/mod.rs`**, including `energy_offset`; `core/qubo_model.rs` is not the active declaration. Build symmetric CSR, preserve offsets, and verify against an independent original Ising edge scorer. Ultimate physical width is 64 per temperature regardless of the `num_replicas` field.
4. Tests before data: every residual assignment after exact elimination, mixed-sign/field/fractional coefficients, reverse lift, all-pairs delta equivalence, full-variable 1-opt restoration, genuine 2-flip barrier, null identity, no-eligible-vertex control, deterministic replay, gauge counterexample. No new unsafe. Prototype reductions are cold passes; allocate scratch once for refinement loops.
5. Run the two frozen suites: 3600 fixed-sweep rows + 600 matched-wall rows, each in a separate process, sequentially with RAYON_NUM_THREADS=1. `/usr/bin/time` can record process peak RSS and CPU. Exactly record transformations/refinement costs, target censoring and deadline overshoots. Baseline exact internal evaluation counts are unavailable; nominal counts must be labelled upper bounds, never equal-work evidence.
6. Analyze complete factorial, preserve negatives; independent review before any claim. H02 exact tree moves or H04 gauge-conditioned distributions are subsequent distinct directions, each requiring its own new protocol. Do not stop at a cycle-only exact-reduction win: that is a known-method positive control.

## Checks at stop

- `cargo build --release --lib`: PASS, 3.54 s reported.
- `cargo build --release --bin host_timing_calibration`: PASS, 0.67 s.
- `cargo test --release --test test_regression_golden`: **3 passed, 0 failed**, 1.69 s tests.
- `git diff --check`: PASS before final handoff, repeated at stop.
- Ordinary `bash scripts/check_memory_docs.sh HEAD`: fails because pre-existing untracked `results/rc027/RESULT.md` is absent from the catalogue. This was not caused or repaired by this work; do not add a committed catalogue reference to an untracked file.
- Scoped tracked-source snapshot + every intended new/changed file overlay: **PASS**, initially 208 Markdown files / 70 immutable files. Re-run after handoff additions; final output is saved in `memory_validation.log`. Script: `python3 research/breakthrough/check_docs_snapshot.py`. It creates a temporary Git snapshot, checks links/catalogue/checksums and writes `memory_validation_manifest.txt`; it does not alter source Git state or touch unrelated results. Distinguish this PASS from the full-worktree limitation.
- No production source changed. Full cargo/clippy/bin gate set was not rerun and no such claim is made.

The current live research ledger is [RESEARCH_STATE.md](RESEARCH_STATE.md); project handoff authority remains [memory/NOW.md](../../memory/NOW.md). All resumable content is in the repository; `/tmp` paths are expendable.
