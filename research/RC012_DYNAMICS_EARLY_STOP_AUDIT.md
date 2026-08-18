# RC-012 — The Dynamics early-stop controller: three defects, one per layer

**Status: pre-registered prediction CONFIRMED, then superseded by two deeper
findings discovered while verifying it.** The last of the four learned models to
be audited, and the only one that *acts* — `EarlyStopController::decide` converts
`predict_remaining < ε` directly into `RunControl::Stop`, so a wrong prediction
truncates a run and silently costs quality.

Deployed defaults (`executor.rs::EarlyStopConfig`): **ε = 0.005, min_frac = 0.5**.

## Method

Offline decision-rule evaluation: trajectories captured **without** any
controller, then the exact deployed rule replayed over each trajectory's step
stream. Because the full trajectory is known, the ACTUAL remaining improvement at
the stop point is known, using the training target's own scale
`(best_k − best_final)/max(|best_k|,1)`. A **false stop** = firing with
actual ≥ ε. Zero new runtime machinery; nothing can contaminate the measurement.

Training mirrored the module's own test (4 frustrated rings, greedy quench,
144 rows). Conditions: A ring+quench (in-distribution), B ring+thermal
(schedule shift), C G11+quench (instance shift), D G11+thermal (both shifts).
5 seeds each.

## Layer 1 — pre-registered prediction: CONFIRMED

> False stops will concentrate on thermal / shifted conditions; refuted if every
> firing's actual remaining < ε.

**20 firings, 7 false stops — all 7 on thermal conditions.** Worst: D discards
**1.86%** of reachable improvement (≈ 3.7 × ε) for a 50% budget saving. A and C
(quench): 10/10 firings clean.

## Layer 2 — the ε-test is vacuous: the controller is "stop at min_frac"

Every firing in every condition landed at exactly `frac_elapsed = 0.50` with
`predicted = 0.00000`. Profiling predictions along whole trajectories:

```
A ring+quench   pred: 0.0000 ×12        real: 0.0043 0 0 0 ...
B ring+thermal  pred: 0.0000 ×12        real: 0.0584 0.0094 ×6 ...
D G11+thermal   pred: 0.0000 ×12        real: 0.1617 0.0460 0.0302 ...
```

**The model predicts (post-clamp) exactly zero at every step of every
trajectory — including step 0 of condition D, where 16.2% of improvement
remains.** The ε-gate therefore never gates anything; `min_frac` does all the
work. The "learned" early stop is behaviourally the fixed rule *truncate at 50%*.

Mechanism: on a greedy quench the best energy converges within 1–2 of 12 steps,
so ~90% of training targets are exactly 0. Ridge fits a near-zero function; the
`.max(0.0)` clamp in `predict_remaining` finishes the job. The module's own test
only asserts late-step < early-step prediction, which near-zero values satisfy.

## Layer 3 — at the deployed configuration, the controller cannot activate at all

`research_platform --early-stop` bootstraps from **instance[0]** with the 2-step
schedule `[metropolis(24), greedy(24)]` and seeds `[1,2,3]`. Reproduced verbatim:

```
steps per trajectory = 2
fit returned NONE (rows < 20)
```

**3 seeds × 2 steps = 6 rows < the 20 `fit` requires.** `train_on_instances`
returns `None` and the flag prints "skipping honestly" — always, on every
instance set. The replica count does not enter the row count, so no configuration
of the current bootstrap can reach 20 rows.

> The `--early-stop` feature, as shipped, is **unreachable code at its own
> deployed configuration.** ROADMAP's prior-Next-10 item 2 ("✅ Dynamics
> early-stop on the executor's every-run path") ships a controller that never
> turns on.

The honest-skip design deserves credit: the platform *says* it is skipping. But
nothing surfaced that it skips **every** time.

## Practical consequences

- ROADMAP item annotated (see Truth Maintenance below).
- Fix options, all behaviour-changing and therefore recorded rather than applied:
  (a) bootstrap with more steps/seeds (e.g. 12-step schedule × 3 seeds = 36 rows);
  (b) lower `fit`'s row floor; (c) train on richer campaign trajectories.
  Note that fixing Layer 3 merely *exposes* Layer 2: an activatable controller
  whose ε-test is vacuous is a silent 50% truncation of every run past min_frac —
  arguably worse than the dead flag. **Layers 2 and 3 must be fixed together.**
- Layer 2's training-target pathology (plateau-dominated targets → degenerate
  ridge) needs either target reweighting or plateau-stratified training.

## Limitations

- Layer 1/2 measurements used the module-test training distribution, because the
  deployed path cannot produce a model at all (Layer 3). If a future richer
  training corpus fixes Layer 3, Layer 2 must be re-measured on it.
- One instance family per shift axis; 5 seeds; step granularity = schedule steps.

## Reproduction

```bash
cargo run --release --bin exp_dynamics_audit
```
Expected: 20 firings / 7 false stops (all thermal); Q2 profile all-zero
predictions; Q3 `fit returned NONE (rows < 20)`.
