# RC-023 — Does the FORM of memory matter?

Registered in `research/PREREG_RC023_MEMORY_FORM.md`, committed `74e425e`,
**before** any code was written. Operator committed `34eab7c`.

**Question.** `history_field` makes a recently-flipped site temporarily
*costly* (a decaying penalty on ΔE). Seven of MQLib's twenty families make it
temporarily *forbidden*, and buy the prohibition back with an aspiration
criterion. Both are memory beyond the configuration — RC-004 axis Σ. Does the
form of that memory change what the operator can do?

## The comparison

`tabu_sweep` was built as `history_field`'s twin: same sweep frame, same
Metropolis acceptance on the real ΔE, same unconditional RNG draw on both
branches (test `the_rng_stream_stays_aligned_with_the_twin` pins this — the two
operators consume identical random streams), same `Report` accounting, same
untouched canonical ledger. **Only the form of the memory differs.** Tenure = 10,
fixed by the preregistration and never tuned.

Harness: `--op-benchmark --sweeps 50 --replicas 32`, seeds 1/2/3, all 30 G-Set
instances, unchanged from the preregistration.

## Preregistered primary result — NO

Two-sided Wilcoxon signed-rank on 30 paired per-instance normalised scores:

| analysis | W | p | ties dropped |
|---|---|---|---|
| as preregistered (harness prints 3 dp) | 144.5 | **0.8746** | 6 |
| same runs, print precision raised to 9 dp | 182.0 | **0.6325** | 2 |

The second row is a **post-hoc robustness check**, not the registered analysis:
the runs are byte-identical (deterministic seeds; max deviation 0.0010 = the
rounding itself), only the printed resolution changed. It exists because
3-decimal printing manufactured six ties that the signed-rank test discards. It
does not rescue the result and was not meant to.

**Outcome (a) of the three registered outcomes: the two forms of memory are
indistinguishable at this budget on this corpus.** Hard prohibition with an
aspiration criterion does not, in aggregate, do anything a soft decaying bias
cannot. Mean difference −0.0018 on a [0,1] scale; median +0.0010 — the two
disagree in sign, which is itself a statement that there is no effect to find.

## What the data does say — POST-HOC, not a finding

The *sign* of the per-instance difference is perfectly predicted by one property
of the instance: whether its edges carry negative weights.

| stratum | n | tabu lower on | mean diff |
|---|---|---|---|
| signed (±1) — G11 G12 G13 G32 G33 G34 | 6 | **6/6** | −0.0138 |
| unweighted | 24 | 3/24 | +0.0012 |

Mann–Whitney U = 0 (complete separation), p = 0.0001. The six largest
differences in the entire table are exactly the six signed instances.

**Density is excluded as the explanation.** G48/G49/G50 are toroidal grids at
m/n = 2.00, the same topology and the same density as G11/G12/G13 — and they
show a difference of ~0.

**But the matched control is saturated, so the confound is only partly
excluded.** On G48/G49/G50 nine of the nineteen registered operators sit exactly
at the instance optimum: the control cannot discriminate between *any* two
operators, so its null difference is a ceiling, not evidence of no effect. What
survives is that sign and frustration co-vary with the effect and that density
alone does not — not that frustration is the cause.

RC-004 retracted Law 2 for exactly this shape of error (a perfect separation
that turned out to track a confound). This stratification is therefore recorded
as **a hypothesis requiring its own preregistration**, with the ceiling problem
as its first design obstacle: it needs a control corpus where unweighted
instances are not solved to optimality by half the operator pool.

## Prohibited claims (PREREG §5, honoured)

No wall-time claim of any kind — RC-021 left this host **unqualified** (C10
failed: diagnostic overhead 0.0108 > 0.01). Nothing here is a claim about
`UltimateSolver`, about generalisation beyond unweighted/±1 G-Set, or about
tuned tabu search: tenure 10 was frozen in advance and a tuned tenure might
answer differently. That question was not asked here.

## Artifacts

- `op_benchmark_raw.tsv` — the preregistered output (3 dp)
- `op_benchmark_precision9.tsv` — the robustness re-print (9 dp)
- `op_benchmark.err` — stderr of the registered run (empty)
