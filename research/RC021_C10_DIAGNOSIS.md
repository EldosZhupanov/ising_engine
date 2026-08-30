# RC-021 control C10 — the failure was real, its recorded reason was not

**Date:** 2026-08-30. **Status:** research record. **Compute:** measurement only;
`experiments/rc021/` was not read, written, repaired or supplemented, and no
qualification session was started.

## What the terminal record says

RC-021 closed Class I `INSTRUMENT-INVALID` on a single failing control:

```
C10 FAILED: diagnostic overhead 0.010802221586322025 exceeds 0.01;
            diagnostics are removed, not tolerated
```

C10 runs the frozen sentinel 30 interleaved pairs per arm — diagnostics on
against diagnostics off — takes the median of each arm, and requires their
relative difference to be at most 1 %. It measured 1.08 % and refused the host.

The recorded reading is that reading `/proc/self/stat` and `/proc/self/status`
around the timed window costs about 1 % of the sentinel's duration. That reading
was never tested. **This record tests it.**

## The measurement

The quantity C10 compares was reproduced outside the instrument, on the same
host, the same instance (G11), the same window (8 sweeps, 32 replicas), the same
untimed 4-sweep prefix and the same 30-pairs-per-arm design — and then run with
**both arms identical**. Two arms of byte-identical work must differ by zero.
What they actually differ by is the noise floor of the host, and it is the number
C10 never observed.

At C10's own sample size, with diagnostics disabled in **both** arms:

| replications | median | p90 | max | exceeded C10's 1 % bound |
|---:|---:|---:|---:|---:|
| 40 | **1.006 %** | 4.94 % | 8.48 % | **20 / 40** |

**The median difference between two identical arms is 1.006 %, and C10's bound is
1 %.** The control fails a perfect host about half the time. The 1.08 % it
measured is indistinguishable from that null.

Larger samples do not rescue it, because the noise is drift rather than sampling
error — more pairs give the scheduler more opportunity, not less:

| pairs per arm | median null | exceeded 1 % |
|---:|---:|---:|
| 30 | 1.006 % | 20 / 40 |
| 60 | 2.12 % | 18 / 25 |
| 120 | 0.54 % | 6 / 15 |
| 240 | 1.32 % | 5 / 8 |

Neither does a longer measurement window, and CPU pinning does not help either
— a pinned thread on WSL2 loses the scheduler's freedom to move away from an
interfering neighbour, and the calibration below shows it does something worse
than that:

| sweeps | ms/run | median null, free | median null, pinned |
|---:|---:|---:|---:|
| 8 | 6.6 | 0.61 % | 1.81 % |
| 32 | 26.3 | 1.85 % | 3.40 % |
| 128 | 105.0 | 7.02 % | 1.39 % |
| 512 | 434.4 | 1.23 % | 1.56 % |
| 2048 | 1435.5 | 1.71 % | — |

## What this establishes, and what it does not

**Established.** C10 cannot resolve the effect it is asked to bound. Its 1 %
threshold sits at the median of its own null distribution at its own sample
size, so its verdict on this host carries close to no information about the
diagnostics. The instrument was invalid — the terminal record's *outcome* stands
— but not for the reason recorded.

**Not established.** That the diagnostics are free. They may well cost something;
this measurement says only that C10 could not have detected it either way. The
honest statement is that the diagnostic overhead on this host is **unmeasured**,
not that it is zero.

**Not a defect of the closure.** The RC-021 instrument reported what its frozen
control returned and refused to proceed. That is exactly what it was built to do.
The defect is one level up, in the preregistration: **a bound was frozen without
ever asking whether the host could resolve it.**

This is the same failure class the RC-021 review rounds kept finding, recorded in
`memory/OPEN_PROBLEMS.md` §0b — *an instrument asserting something it had not
observed*. Here the unobserved thing was the control's own null.

## What was built instead of a repair

C10's threshold and procedure are frozen by an immutable preregistration and were
not touched. Lowering a bound after watching it fail is the precise act this
whole architecture exists to prevent.

What was missing was an instrument to ask the question *before* freezing a
threshold. `src/bin/host_timing_calibration.rs` is that instrument. It runs the
paired comparison in two modes, and needs both:

- a **null arm** — the two arms run identical work, so everything it reports is
  noise. This is what RC-021 never ran;
- an **injection arm** — one side is given a known extra fraction of work. A tool
  that has only ever measured "no difference" has not been shown capable of
  seeing one. The injection is the positive control that earns the null its
  meaning.

Measured on this host, G11, 30 pairs per arm, 20 replications
(`results/host_calibration/calibration_free.tsv`):

| arm | window | expected | measured median | p10 | p90 |
|---|---:|---:|---:|---:|---:|
| null | 8 | 0 | **+0.009 %** | −1.61 % | +3.02 % |
| inject | 8 | +25 % | **+25.24 %** | +24.50 % | +26.64 % |
| inject | 8 | +50 % | **+51.96 %** | +48.30 % | +57.30 % |
| null | 128 | 0 | **−0.144 %** | −4.18 % | +3.45 % |
| inject | 128 | +4.69 % | **+6.33 %** | +1.60 % | +9.46 % |

The estimator is **unbiased** — both null medians sit within 0.15 % of zero — and
the positive controls are recovered accurately. The problem is dispersion, not
bias: a single replication carries roughly a ±3 % error bar.

### Pinning the CPU makes it worse, and the positive control is how we know

Conventional benchmarking advice is to pin the measuring thread to one core. On
this host that advice is wrong, and only the injection arm reveals it. Repeating
the same calibration with `--pin`
(`results/host_calibration/calibration_pinned.tsv`):

| arm | window | expected | free run | **pinned run** |
|---|---:|---:|---:|---:|
| null | 8 | 0 | +0.009 % | +0.149 % |
| inject | 8 | **+25 %** | **+25.24 %** | **+48.07 %** |
| inject | 8 | +50 % | +51.96 % | +51.73 % |
| null | 128 | 0 | −0.144 % | −0.885 % |
| inject | 128 | +4.69 % | +6.33 % | +4.64 % |

The pinned null still lands near zero, so a null-only tool would have reported
pinning as harmless or even an improvement. The 25 % injection comes back as
**48 %** — nearly double. Pinning does not merely add noise here; it **biases the
estimate**, plausibly because a single pinned core carrying the longer arm
throttles in a way a freely scheduled thread does not.

This is the clearest argument in this record for why a null arm alone is not
enough. A calibration that only ever measures "no difference" cannot tell a good
configuration from one that doubles the effect it is trying to measure.

## The number this project needed and did not have

> **On this host, a paired wall-time difference of ~25 % is resolved cleanly
> (20/20, with the treated arm's p10 of +24.5 % far above the null's p90 of
> +3.0 %). A ~5 % difference is detectable but overlaps the null tail. A 1 %
> difference cannot be resolved at all.**

Three consequences, in increasing order of how much they matter:

1. **RC-021's C10 was unachievable on this host from the day it was written**,
   whatever the diagnostics cost. A fresh preregistration wanting a wall-time
   claim must set its bound at or above 5 %, and must run a null arm alongside.

2. **`CLAUDE.md` §9's rule — "keep only measured >1 % wins" — is not verifiable
   here.** The rule is sound; this host cannot execute it. Either the threshold
   rises to what the host can resolve, or performance claims move to a machine
   that has been calibrated. Until one of those happens, a "1.4 % win" measured
   on this host is noise wearing a decimal point.

3. **The direction that needed wall time is open after all.** Compiler/LLM code
   optimisation (`EXTERNAL_PROJECTS_BACKLOG.md` §14.2) reports speedups up to
   1.25×. A 25 % effect is exactly what this host resolves cleanly. The block was
   never "this machine cannot time anything"; it was "this machine cannot time
   1 %". Nobody had measured which.

## Reproducing this

```
cargo run --release --bin host_timing_calibration -- \
  --file benchmark_suite/data/gset/G11 --windows 8,128 \
  --pairs 30 --reps 20 --inject 0.05,0.25,0.50 [--pin]
```

Timings are not reproducible — their variability is the subject — but the work
behind them is fixed by seed, and the null arm is reproducible in the only sense
that matters: it must keep landing on zero.

## What is deliberately not claimed

No claim that any solver is faster or slower than any other. No claim about
`UltimateSolver`, production routing, or any RC-021 scientific quantity. No
repair of, or amendment to, the RC-021 preregistration, whose C10 remains exactly
as frozen. No claim that this host is fit for a wall-time experiment — only a
measurement of what it could resolve if one were preregistered.
