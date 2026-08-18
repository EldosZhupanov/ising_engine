# RC-011 — The predictor's leave-one-instance-out Spearman cannot measure transfer

**A proof, not a statistical result.** `ROADMAP.md` listed under *Confirmed:*
"cross-family transfer is real (portability 9/12, predictor
leave-one-instance-out **Spearman +0.747**)". The second half of that evidence is
mathematically incapable of supporting the claim.

## What is sound

The fold structure is correct. `leave_one_instance_out` excludes the entire
instance, so there is **no row-level leakage** — a real risk given ~800 rows per
instance, and the code avoids it.

## The theorem

`predictor::feature_row` builds

    [ bias | instance features (feature_registry v0) | schedule features | op indicators ]

with **no interaction terms**. The predictor is therefore purely additive:

    ŷ(inst, sched) = w·x_instance(inst) + w·x_schedule(sched)

`evaluate_predictor` scores each fold with `spearman(preds, actual)` over the rows
of the **single** held-out instance. All those rows share that instance's
signature, so `w·x_instance` is *one constant* across the entire test set.
Spearman is rank-based, and **a constant offset cannot change a ranking**.

> **The reported 0.747 is invariant to the instance features and to every weight
> attached to them.** A predictor that ignored the instance entirely would score
> identically.

It measures **schedule-quality ranking**. It cannot detect transfer of
instance-conditional knowledge, because it is blind to the instance by
construction.

## Empirical confirmation (real 18,570-row DB, 23 instances)

The argument does not rest on reading code. Both load-bearing facts were measured:

| fact | prediction | measured |
|---|---|---|
| no instance×schedule interaction: `predict(sigA,s) − predict(sigB,s)` constant over wildly different `s` | exactly constant | **max deviation 1.110e-16** (machine ε) over 7 schedules |
| instance signature constant within each instance | 0 violations | **0 violations**, 23 instances, 18,570 rows |
| reported metric reproduces | ≈0.747 | **0.7467** over 23 folds |

1 + 2 ⇒ the offset is constant within every fold ⇒ Spearman unchanged. ∎

## Scope — what is NOT claimed

- The predictor is **not** broken. Ranking schedules well *is* useful, and it is
  what the model is used for (`filter`).
- **The portability 9/12 result is a separate claim** and is untouched here. It
  may well evidence transfer; this cycle says nothing about it.
- The fix is to the *interpretation*, not the code: report 0.747 as
  schedule-ranking skill, not as transfer evidence.

## How the metric could be made transfer-sensitive

Either add instance×schedule interaction terms (then instance features affect
within-fold ranking, and the metric becomes informative about transfer), or score
transfer *across* folds — e.g. rank-correlate predicted-vs-actual *instance mean*
improvement over held-out instances, where the instance features do vary. Both are
behaviour-changing and are recorded rather than applied.

## Reproduction

```bash
cargo run --release --bin exp_predictor_audit -- \
    --db experiments/platform_gset/ai_experiments.txt
```
