# RC-024 — Does endpoint-guided path relinking buy material quality?

**Registered decision: WORKS BUT IMMATERIAL.**

The mechanism moves quality, reliably and with an effect size that is not in
doubt. It does not clear the project's 1 % materiality floor, and not one of the
ninety paired observations came close to clearing it.

## Provenance

| item | value |
|---|---|
| preregistration | `research/PREREG_RC024_PATH_RELINKING.md`, commit `28710dc`, frozen before candidate code |
| preregistration SHA-256 | `b6e7514e63d72049568879333597a2ce979932f1142f10d9e57a3f5d320064a5` |
| candidate commit | `4af912efab20150199cb70220a6139f118119582` |
| registered command | `cargo run --release --bin exp_rc024_path_relink -- --dir benchmark_suite/data/gset --sweeps 50 --replicas 32 --seeds 101,102,103` |
| `raw.tsv` SHA-256 | `7a136e595732b8b3cb48f78aeb0d19ed91b0edefb900483b7abddc98ece49289` |
| `run.err` SHA-256 | `e38ec9c3d0b4bbf0b6de93fc3be0451c6d5c7b1ab3669b84c5ee9733b5c243e1` |
| runs | one, as registered. No pilot, no tuning run, no repeat |

The harness refuses any argument that differs from the frozen values; the test
`only_the_preregistered_command_is_accepted` perturbs every position of the
command line and requires each perturbation to be rejected.

## Result

| quantity | value |
|---|---|
| paired observations | 90 (30 instances × seeds 101, 102, 103) |
| wins / losses / ties at 1e-9 | **69 / 0 / 21** |
| mean relative gain | **+0.1053 %** |
| median relative gain | **+0.0770 %** |
| Wilcoxon signed-rank | n = 69, z = 7.2166, **p = 5.37 × 10⁻¹³** |
| largest single gain | +0.463 % |
| pairs clearing the 1 % floor | **0 / 90** |
| instances with a strictly positive mean gain | 25 / 30 |

Applying the frozen decision table: `p < 0.05` holds by twelve orders of
magnitude and the median is positive, but the mean gain is 0.105 %, an order of
magnitude below 1 %. That is the **WORKS BUT IMMATERIAL** row, and it was
written down before the run.

The zero in the loss column is the strongest single number here. In ninety
paired trials the candidate never once finished worse than its own control. That
is not luck: it is the never-worse invariant, proven on synthetic endpoints
before the run and enforced by rewinding each source to its best prefix.

## What this says, taken with RC-003 and RC-023

RC-022's census found synthesized moves in 9 of 20 external heuristic families —
the largest mechanism gap between our operator corpus and the human one. That
gap is now measured twice, in two independent forms:

| experiment | form of synthesis | effect |
|---|---|---|
| RC-003 | collective subsets mined from population covariance | ≈ +0.06 % |
| RC-024 | endpoint-guided path relinking between population members | +0.105 % |

Two mechanisms designed on different principles, tested years apart in project
time, land within a factor of two of each other and both an order of magnitude
under the materiality floor. Together with RC-023 — where the *form* of memory
turned out not to matter at all — the emerging picture is that the mechanism
inventory is not where our corpus is short. That is a claim about *these*
instances at *this* budget, and it is the hypothesis the next cycle should try
to break, not a conclusion.

**The RC-023 post-hoc stratification does not reproduce here.** RC-023 found the
sign of its effect separating perfectly by whether an instance carries negative
weights. On RC-024 the same split gives +0.126 % (signed, n = 6) against
+0.100 % (unweighted, n = 24) — a difference of no consequence. This does not
refute the RC-023 observation, which was about a different mechanism, but it
does remove one way that observation could have been a general property of the
corpus rather than of tabu memory.

## Prohibited claims (PREREG §6, honoured)

The candidate performs **strictly more logical work** than the control: one
extra relinking round on top of an identical 50-sweep prefix. This experiment
deliberately does not identify that cost, and RC-021 left this host
**INSTRUMENT-INVALID** (C10: diagnostic overhead 0.0108 > 0.01). Therefore:

- **no claim about wall time, throughput, or equal cost.** A +0.105 % quality
  gain bought with unmeasured extra work is not a speed result and must never be
  quoted as one;
- no claim about `UltimateSolver`, production auto-routing, or integration;
- no claim about tuned path relinking — the source fraction `max(1, R/4)`, the
  single round, and the greedy lowest-Δ*E* rule were frozen in advance and never
  varied. A tuned variant might answer differently; that question was not asked;
- no claim about corpora other than these 30 G-Set instances, and none about
  universal optimization or model training.

Under the frozen table, WORKS BUT IMMATERIAL licenses **nothing further on its
own**. It does not earn the cost/replication study that a material signal would
have earned.

## Artifacts

- `raw.tsv` — header plus exactly 90 data rows, energies at full `f64` precision
- `run.err` — stderr of the registered invocation, retained verbatim including
  the build lines that `cargo run` emits
- this file
