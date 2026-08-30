# PREREG RC-023 — Does the form of memory matter?

**Written before any code and before any run.** Nothing below may be changed
after the first datum; a correction requires a new linked artifact stating what
changed and why, as RC-022 §0 did.

## §1 The question

RC-022 established that memory beyond the configuration is an occupied
architectural cell in both corpora, and that we entered it once — from physics.

- **Ours:** `history_field`, a metadynamics **soft** bias. A per-(site, replica)
  penalty grows by 1.0 on each flip and decays by 0.98 per sweep; it is added to
  ΔE at acceptance and never enters the reported energy.
- **The human corpus:** seven of twenty MQLib families use tabu memory — **hard**
  prohibition for a fixed tenure, with an **aspiration criterion** that overrides
  the prohibition when the forbidden move would beat the incumbent.

> **Does the form of the memory matter? Does hard prohibition with aspiration do
> something a soft decaying bias cannot?**

## §2 What is being compared, and what is held fixed

One new operator, `tabu_sweep`, built as `history_field`'s **twin**: the same
sweep frame, the same Metropolis acceptance on the **real** ΔE, the same
unconditional RNG draw on both acceptance branches so the stream stays aligned,
the same `Report` accounting, the same untouched canonical ledger.

**The only difference is the form of the memory.**

| | `history_field` | `tabu_sweep` |
|---|---|---|
| memory | continuous penalty added to ΔE | integer sweep index until which the site is forbidden |
| effect | slows a return | forbids a return |
| release | exponential decay | tenure expiry |
| override | none | aspiration: allowed if the move would beat that replica's best-so-far |

## §3 Frozen design

- **Instances:** all 30 G-Set files in `benchmark_suite/data/gset/`. No subset,
  no selection after seeing results.
- **Seeds:** `1, 2, 3` — the harness's own fixed seeds, unmodified.
- **Budget:** `--sweeps 50`, `--replicas 32` — the harness defaults, unmodified.
- **Harness:** `research_platform --op-benchmark`, used **unchanged**. It runs
  every registered operator solo at identical seeds and budget and normalises
  each instance's mean best energy to [0,1] across operators (0 = best on that
  instance).
- **Metric:** the normalised score of the two operators, paired by instance.
  Because the per-instance normalisation is a monotone affine map, the **sign**
  of the difference between two operators is preserved; this design therefore
  tests **direction**, not effect size in energy units, and no energy-magnitude
  claim may be made from it.
- **Tenure:** 10 sweeps, fixed here. It is the one free parameter and it is
  **not** tuned: a tenure chosen after seeing results would make this a search
  for a winning configuration rather than a test of a mechanism.

## §4 Hypotheses and the decision rule, fixed in advance

- **H0 (null):** the two forms are indistinguishable in direction.
- **H1:** hard prohibition with aspiration reaches lower normalised energy than
  the soft bias.

**Test:** two-sided Wilcoxon signed-rank over the 30 paired instance scores.
**Decision, fixed now:**

| outcome | reading |
|---|---|
| p < 0.05 and `tabu_sweep` lower | the form matters; hard prohibition does something the soft bias does not |
| p < 0.05 and `history_field` lower | the form matters, **in our favour**; the physics form is the stronger one |
| p ≥ 0.05 | **no evidence that the form matters.** The axis is what counts, not its spelling — and we were already in it |

All three are results and all three will be published. The third is the outcome
that most changes the programme, because it would say the census's headline gap
is cosmetic.

## §5 What this run may not claim

- **No wall-time claim of any kind.** RC-021 left this host **unqualified** as a
  measuring station (C10 failed: diagnostic overhead 0.0108 ms against a 0.01 ms
  bound). This experiment counts sweeps, which is exact and clock-independent, so
  it is legitimate here — but no statement about speed may be derived from it.
- No claim about `UltimateSolver`. This compares two `engine_v2` operators.
- No claim that either form generalises beyond unweighted G-Set. G-Set is
  degenerate on the guide axis (`E = 2V − |E|`, RC-004) and carries a
  density/ruggedness confound; a result here is about this corpus.
- No claim about tuned tabu. Tenure is fixed at 10 and untuned by construction.

## §6 Correctness obligations before the run

1. `tabu_sweep` must be bit-identical between `ReferenceState` and
   `SparseBitSlice`, as every operator is.
2. The reported energy must remain the canonical ledger energy; the memory must
   never enter the score.
3. `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` clean in both
   profiles, and the golden regression unchanged.
