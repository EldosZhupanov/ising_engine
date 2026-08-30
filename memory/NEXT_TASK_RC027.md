# Task bundle — RC-027, the budget axis

**Written 2026-08-30 for the next session.** Read `START_HERE.md`, then
`memory/NOW.md`, then this file. Do not scan the repository.

---

## 1. Where the project stands, in one screen

The **architecture line is closed.** Six cycles asked whether our operator corpus
is missing something the field has. The answer, measured three different ways, is
that the missing mechanisms do not pay:

| cycle | question | result | record |
|---|---|---|---|
| RC-022 | do human solvers occupy cells we do not? | yes — 8+ cells to our 1 | `research/RC022_ARCHITECTURE_CENSUS.md` |
| RC-023 | does the **form** of memory matter? | no, p = 0.87 | `results/rc023/RESULT.md` |
| RC-024 | does path relinking pay? | +0.105 %, 69 W / **0 L** | `results/rc024/RESULT.md` |
| RC-025 | is **G-Set's degeneracy** the reason? | no — a weight ladder is flat | `results/rc025/RESULT.md` |
| RC-026 | were we measuring it **outside the search**? | partly yes: ρ = +0.31, p = 1.0e-4 — and still immaterial | `results/rc026/RESULT.md` |
| — | the blocker RC-026 named | fixed: **5.72×**, bit-identical | `research/PERF_INCREMENTAL_RELINK.md` |

RC-026 is the only cycle that found a signal, and it found it by changing **the
arrangement of the comparison, not the thing compared**. Embedding path relinking
k times inside one search raises the gain monotonically over five rungs at an
unchanged 50-sweep budget. It is still `gain ∝ k^0.18` — 25× the rounds buys
1.77×, and 1 % would need ~3.6e5 rounds. **The direction that helps is the one
that cannot be afforded.**

**Do not open a seventh cycle on mechanisms.** The three natural questions
(mechanism, corpus, comparison shape) are spent.

## 2. Your task: RC-027 — the budget axis

**Every one of the six cycles ran 50 sweeps.** A mechanism that only pays on a
long run would be invisible to all of them. That is the last cheap question this
project has not asked, and it is the only one left that could overturn the
closure.

> Does the value of a mechanism depend on the run length it is given? Do the
> curves for candidate and control **converge, diverge, or stay parallel** as the
> budget grows?

Three outcomes matter and all are publishable:

- **diverge** (gain grows with budget) — the closure was an artifact of a short
  budget, and the architecture line reopens under a new preregistration;
- **converge** (gain shrinks) — the mechanisms buy *earlier convergence*, not a
  better optimum, which is a different and honest claim about what they are for;
- **parallel** — budget is not the explanation either, and the closure is final.

### Suggested design — you own the final call, but justify any change

- **Pairs:** the same two as RC-025, used **unchanged**: `path_relink_sweep` vs
  `metropolis_sweep`, and `tabu_sweep` vs `history_field`. No retuning.
- **Budget ladder:** sweeps ∈ {50, 200, 800, 3200}. 50 is the rung every prior
  cycle used and is therefore your **mandatory control** — it must reproduce
  RC-024's +0.105 % and RC-023's null, or no reading of the ladder is licensed.
- **Corpus:** all 30 G-Set instances. RC-025 showed the corpus does not change
  these answers, so use the one every prior cycle used, for comparability.
- **Seeds:** 401, 402, 403 — fresh, so this is a new measurement.
- **Primary test:** Spearman of per-instance gain against `log(sweeps)`, per
  pair. State the direction each outcome implies **before** running.
- **Rows:** 30 × 4 × 3 × 2 = 720.

**Cost warning, measured not guessed.** RC-026 at k=25 took over an hour before
the relink optimisation and 375 s after. A 3200-sweep rung is 64× the work of a
50-sweep rung. Estimate the runtime with a **single throwaway instance** before
freezing the ladder, and if 3200 is impractical, freeze a shorter top rung in the
preregistration rather than discovering it mid-run. Do **not** trim the ladder
after seeing results.

## 3. Non-negotiable procedure — this is how every cycle above was run

1. **Preregister before any code.** Commit
   `research/PREREG_RC027_*.md` with: the question, the frozen corpus, budget
   ladder, seeds, harness name, estimand, the primary test, the **mandatory
   controls**, a frozen decision table naming every outcome, the §5 correctness
   obligations, the prohibited claims, and the exact §7 command. Register it in
   `memory/CATALOG.md` and `memory/BINDING_SHA256`. It becomes immutable.
2. **A prospective correction needs a separate amendment commit**, and only
   before any datum exists. See `research/PREREG_RC025_AMENDMENT_1.md` — the
   harness refused before measuring, the family names were wrong, and the fix was
   committed as its own amendment.
3. **Build the harness as a private binary** that refuses any argument differing
   from the frozen command. Copy the shape of
   `src/bin/exp_rc026_embedding_ladder.rs`.
4. **Every guard must be killed by a mutation.** Break the branch the test names
   and require exactly that test to fail; restore the tree and verify the SHA-256
   matches. A guard no mutation can kill is either equivalent or untested — say
   which, in writing.
5. **Run once.** Publish the outcome whatever it is.
6. **Record it**: `results/rc027/RESULT.md` + `raw.tsv` + `run.err`, plus
   `memory/OPEN_PROBLEMS.md`, `memory/NOW.md`, `memory/TIMELINE.md`,
   `memory/CATALOG.md`, `memory/BINDING_SHA256`.

## 4. Gates before the run, and again after the commit

```
cargo check --all-targets
cargo test --release && cargo test          # both profiles
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --release --test test_regression_golden
bash scripts/check_memory_docs.sh
sha256sum -c memory/BINDING_SHA256
```

Current baseline: **719 tests**, clippy clean, 150 Markdown files, 67 immutable.
A test whose outcome flips on `git commit` is testing the tree, not the code.

## 5. Traps that have already cost this project time

- **`parse_rudy` silently drops self-loops and out-of-range edges.** A QUBO file
  handed to it does not fail — it quietly becomes a different model. Guard on the
  raw text and the header's declared edge count, not on the parser's output.
- **A guard that refuses on a *name* is weaker than one that refuses on a
  *count*.** RC-025's family names were wrong and a substring matcher would have
  returned the right *number* of files from the wrong sizes.
- **Python edit scripts write at the end.** An `AssertionError` mid-script
  discards everything silently. Re-check with `grep` after each edit.
- **`cargo fmt` reformats your literals**, so a later `str.replace` against the
  pre-format text fails. Re-read the file after formatting.
- **Check where a "diagnostic" field is consumed.** `Report.work` looked
  diagnostic and reaches `lab.rs`'s `fitness(score, work, …)`.
- **Six unregistered sub-analyses will hand you two contradictory p < 0.001
  results.** RC-025 §"Exploratory" is the worked example. Report them, read them
  as nothing.

## 6. Tools that now exist and did not before

- `src/bin/host_timing_calibration.rs` — what a paired wall-time comparison can
  resolve on this host, with a null arm **and** an injected positive control.
  **Run it before any timing threshold.** This host: ~25 % clean, ~5 % marginal,
  **1 % never**. Do not pass `--pin` here — it leaves the null near zero while
  reporting a known +25 % injection as +48 %.
- The wall-time protocol in `CLAUDE.md` §9, and its worked example
  `research/PERF_INCREMENTAL_RELINK.md`: calibrate the host, keep the previous
  release binary and interleave the arms, prove bit-identity by reproducing a
  SHA-256 published **before** the change, and declare whatever moved.

## 7. Hard prohibitions for this task

- **No wall-time, throughput or equal-cost claim.** RC-021 left this host
  `INSTRUMENT-INVALID` (C10). `research/RC021_C10_DIAGNOSIS.md` shows C10's
  recorded failure reason was never established — its 1 % bound sat at the median
  of its own null — but the closure stands and is not to be reopened, edited or
  re-run. `experiments/rc021/` is terminal evidence: do not touch it.
- No claim about `UltimateSolver`, production routing, or production integration.
- Do not edit any existing preregistration, amendment or result. They are in
  `memory/BINDING_SHA256`.
- Do not change the scalar solver family, `core`, `UltimateSolver`, public API,
  or Cargo dependencies.
- Do not regenerate `.claude/context/` — untracked, ungenerated, and its
  `modules.txt` still describes a project with no `engine_v2`. `CLAUDE.md` §7–§8
  now rank `rg` above it for that reason.
- Do not run the secondary high-diversity extension that RC-025 §2 registered and
  §3 excluded. Adding an arm after seeing a null is what preregistration forbids.

## 8. The one asset worth carrying forward

`path_relink_sweep` has **478 wins and 0 losses in 630 paired trials** across two
corpora, three preregistrations, nine seeds and five embedding depths. It has
never once finished worse than its control. That is the never-worse invariant,
proven on synthetic endpoints *before* any of those runs and enforced by
rewinding each source to its best prefix.

**The template is the asset, not the mechanism.** Whatever RC-027 finds, a
mechanism whose guarantee is proven before it is measured is the thing this
project learned to build.
