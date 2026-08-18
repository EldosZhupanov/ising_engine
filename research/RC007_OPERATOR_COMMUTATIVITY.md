# RC-007 — The operator monoid is effectively commutative

**Vector 3 (axiomatic inversion): invert commutativity.** The Runtime executes
operator *sequences* and the Evolution Engine runs a genetic search over their
*ordering* (`evolution.rs`). That search is only worth its cost if order carries
information. It had never been measured.

## Method

These are stochastic maps, so literal `A∘B = B∘A` is the wrong question — the two
orders consume the shared RNG stream differently even at one seed. The decidable
question is whether order matters *more than the seed does*:

    d_order = mean over seeds of |E([A,B],s) − E([B,A],s)|      (scale-normalised)
    d_seed  = mean over seed pairs of |E([A,B],sᵢ) − E([A,B],sⱼ)|   ← the null
    rho     = d_order / d_seed

8 operators spanning every family, all 28 unordered pairs, 6 G-Set instances ×
6 seeds, 30 sweeps each, ladder 2.0→0.1. Harness `src/bin/exp_commutator.rs`.

## A confound in the first run, and its correction

The first pass reported 57% commuting with **8 `inf` rows** and 5 pairs at exactly
(0, 0). Cause: from the all-zeros init every replica is **identical** (RC-002), so
`replica_exchange` has nothing to swap and `houdayer_cluster` has no overlap
domains — both are **structurally inert**, and deterministic operators then show
*zero seed variance*, making rho divide by zero. The instrument had inherited the
very defect RC-002 documented.

Re-run with one randomising sweep prepended (`--diverse`) so the ensemble is
genuinely diverse:

| | all-zeros (confounded) | diverse |
|---|---|---|
| effectively commuting (rho<2) | 57% | **100% (28/28)** |
| `inf` rows | 8 | **0** |
| max rho | 109.63 | **1.32** |

## Result — prediction refuted

Pre-registered: rho ≈ 1 for most pairs but **large** for asymmetric pairs such as
greedy vs random-flip, on the theory that a schedule is dominated by which
operator runs *last*.

Observed: `greedy_descent × random_flip_sweep` tops the table at **rho = 1.32**,
and **no pair exceeds 1.4**. All 28 pairs perturb the outcome no more than
re-seeding does.

> **Swapping two operators changes the result no more than changing the seed.**

Two pairs show `d_order` **exactly 0.000000** against a non-zero null —
`greedy_descent × replica_exchange` and `steepest_descent × replica_exchange`.
This is not the earlier vacuity: deterministic descent acts independently and
identically on each replica, so permuting replicas and descending genuinely
commute. A provable relation, confirmed to exact zero.

## Consequence

The Evolution Engine's ordering search explores distinctions that largely do not
exist at the pair level. This supplies a *mechanism* for a result already in
`ROADMAP.md` — evolved plans lose **5/5** to `UltimateSolver` at equal budget:
a substantial dimension of the search space it optimises over is degenerate.

## Scope — what is NOT shown

- **Pairs only.** Order effects could compound in longer schedules; untested.
- Equal budget per operator (30/30); unequal splits untested.
- One ladder (2.0→0.1), final energy only, no trajectory comparison.
- 6×6 gives modest power. The honest claim is "no pair shows order effects
  substantially exceeding seed noise", **not** "rho = 1 exactly". Separating
  rho=1.0 from rho=1.3 needs more data than this.
- No fix applied. Narrowing the Evolution search space is behaviour-changing
  (ADR-0004) and needs approval plus an A/B at identical seeds.

## Reproduction

```bash
cargo run --release --bin exp_commutator -- \
    --dir benchmark_suite/data/gset --sweeps 30 --seeds 6 --instances 6 --diverse
```
